use std::mem::size_of;
use std::ptr::copy_nonoverlapping;
use std::ptr::null_mut;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use shared_protocol::{CommitResult, CommitStatus, CommitTransport, RuntimeSettings, Shortcut};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, GlobalFree, HANDLE, HWND, SetLastError,
};
use windows_sys::Win32::Globalization::{CP_ACP, MultiByteToWideChar};
use windows_sys::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_ELEVATION,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenElevation, TokenIntegrityLevel,
};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::SystemServices::{
    SECURITY_MANDATORY_HIGH_RID, SECURITY_MANDATORY_LOW_RID, SECURITY_MANDATORY_MEDIUM_PLUS_RID,
    SECURITY_MANDATORY_MEDIUM_RID, SECURITY_MANDATORY_PROTECTED_PROCESS_RID,
    SECURITY_MANDATORY_SYSTEM_RID, SECURITY_MANDATORY_UNTRUSTED_RID,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, SendInput, VK_CONTROL, VK_LWIN, VK_MENU, VK_RETURN, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

use crate::host::TextInsertionAdapter;
use crate::selected_text_compatibility::{
    SelectedTextFailureReason, SelectedTextFailureStage, format_selected_text_failure,
};
use crate::windows_shortcut::collect_release_virtual_keys;

const HOTKEY_RELEASE_TIMEOUT: Duration = Duration::from_millis(1_500);
const FAST_HOTKEY_RELEASE_TIMEOUT: Duration = Duration::from_millis(180);
const HOTKEY_SELECTION_SETTLE_DELAY: Duration = Duration::from_millis(120);
const FAST_HOTKEY_SELECTION_SETTLE_DELAY: Duration = Duration::from_millis(35);
const STRICT_SELECTION_CAPTURE_ATTEMPTS: u32 = 2;
const STRICT_SELECTION_CAPTURE_RETRY_DELAY: Duration = Duration::from_millis(90);
const CLIPBOARD_OPEN_TIMEOUT: Duration = Duration::from_millis(250);
const CLIPBOARD_OPEN_RETRY_DELAY: Duration = Duration::from_millis(10);
const CLIPBOARD_WAIT_TIMEOUT: Duration = Duration::from_millis(1_500);
const FAST_CLIPBOARD_WAIT_TIMEOUT: Duration = Duration::from_millis(180);
const CLIPBOARD_PASTE_SETTLE_DELAY: Duration = Duration::from_millis(120);
const CF_TEXT: u32 = 1;
const CF_UNICODETEXT: u32 = 13;

#[derive(Debug)]
pub struct WindowsTextInsertionAdapter {
    release_virtual_keys: Mutex<Vec<i32>>,
}

impl Default for WindowsTextInsertionAdapter {
    fn default() -> Self {
        Self {
            release_virtual_keys: Mutex::new(hotkey_modifier_virtual_keys().to_vec()),
        }
    }
}

impl WindowsTextInsertionAdapter {
    pub fn from_shortcut(shortcut: &Shortcut) -> Result<Self, String> {
        Ok(Self {
            release_virtual_keys: Mutex::new(collect_release_virtual_keys(shortcut)?),
        })
    }
}

impl TextInsertionAdapter for WindowsTextInsertionAdapter {
    fn commit_text(&self, _session_id: u64, text: &str) -> CommitResult {
        commit_text_with_transports(
            text,
            send_unicode_text,
            paste_text_at_caret_with_clipboard,
            report_transport_diagnostic,
        )
    }

    fn capture_selected_text(&self, _session_id: u64) -> Result<Option<String>, String> {
        let release_virtual_keys = self
            .release_virtual_keys
            .lock()
            .map_err(|_| "failed to lock insertion shortcut state".to_string())?
            .clone();
        capture_selected_text_with_mode(SelectionCaptureMode::Strict, &release_virtual_keys)
    }

    fn capture_selected_text_fast(&self, _session_id: u64) -> Result<Option<String>, String> {
        let release_virtual_keys = self
            .release_virtual_keys
            .lock()
            .map_err(|_| "failed to lock insertion shortcut state".to_string())?
            .clone();
        capture_selected_text_with_mode(SelectionCaptureMode::BestEffort, &release_virtual_keys)
    }

    fn replace_selection(&self, _session_id: u64, text: &str) -> CommitResult {
        match replace_selected_text_with_clipboard(text) {
            Ok(()) => CommitResult {
                status: CommitStatus::Success,
                transport: CommitTransport::ClipboardSelectionReplace,
            },
            Err(error) => CommitResult {
                status: CommitStatus::Failed(error),
                transport: CommitTransport::ClipboardSelectionReplace,
            },
        }
    }

    fn update_settings(&self, settings: &RuntimeSettings) -> Result<(), String> {
        let release_virtual_keys = collect_release_virtual_keys(&settings.dictation_shortcut)?;
        *self
            .release_virtual_keys
            .lock()
            .map_err(|_| "failed to lock insertion shortcut state".to_string())? =
            release_virtual_keys;
        Ok(())
    }
}

pub(crate) fn commit_text_with_transports<Direct, Clipboard, Report>(
    text: &str,
    mut direct: Direct,
    mut clipboard: Clipboard,
    mut report: Report,
) -> CommitResult
where
    Direct: FnMut(&str) -> Result<(), String>,
    Clipboard: FnMut(&str) -> Result<(), String>,
    Report: FnMut(&str),
{
    if let Some(formatted_reason) = formatted_multiline_reason(text) {
        match clipboard(text) {
            Ok(()) => {
                report(&format!(
                    "selected ClipboardPasteFallback first because committed text {formatted_reason}; paste succeeded (clipboard restoration warnings are reported separately)"
                ));
                CommitResult {
                    status: CommitStatus::Success,
                    transport: CommitTransport::ClipboardPasteFallback,
                }
            }
            Err(clipboard_error) => {
                // A newline becomes VK_RETURN in the direct transport. In chat
                // and terminal apps that can submit text instead of inserting it.
                report("multiline clipboard paste failed; direct input disabled to avoid sending Enter; fallback_allowed=false");
                CommitResult {
                    status: CommitStatus::Failed(clipboard_error),
                    transport: CommitTransport::ClipboardPasteFallback,
                }
            }
        }
    } else {
        match direct(text) {
            Ok(()) => {
                report(
                    "selected DirectUnicodeSendInput because committed text is normal single-line text",
                );
                CommitResult {
                    status: CommitStatus::Success,
                    transport: CommitTransport::DirectUnicodeSendInput,
                }
            }
            Err(unicode_error) => match clipboard(text) {
                Ok(()) => {
                    report(&format!(
                        "selected DirectUnicodeSendInput for normal single-line text, but it failed ({unicode_error}); ClipboardPasteFallback succeeded"
                    ));
                    CommitResult {
                        status: CommitStatus::Success,
                        transport: CommitTransport::ClipboardPasteFallback,
                    }
                }
                Err(clipboard_error) => {
                    report(
                        "DirectUnicodeSendInput was preferred for normal single-line text, but it and ClipboardPasteFallback both failed",
                    );
                    CommitResult {
                        status: CommitStatus::Failed(format!(
                            "{unicode_error}; clipboard paste fallback also failed: {clipboard_error}"
                        )),
                        transport: CommitTransport::ClipboardPasteFallback,
                    }
                }
            },
        }
    }
}

fn report_transport_diagnostic(detail: &str) {
    eprintln!("[input-host][commit-transport] {detail}");
}

fn formatted_multiline_reason(text: &str) -> Option<&'static str> {
    if text.contains(['\r', '\n']) {
        return Some("contains a newline");
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SendInputProbeResult {
    pub accepted_events: u32,
    pub expected_events: u32,
    pub last_error: u32,
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipboardOnlyProbeResult {
    pub readback_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessIntegrityProbeResult {
    pub integrity_label: Option<String>,
    pub integrity_rid: Option<u32>,
    pub elevated: Option<bool>,
    pub detail: Option<String>,
}

pub(crate) fn sendinput_key_probe() -> SendInputProbeResult {
    let inputs = vec![
        build_virtual_key_input(b'A' as u16, 0),
        build_virtual_key_input(b'A' as u16, KEYEVENTF_KEYUP),
    ];
    send_inputs_raw(&inputs)
}

pub(crate) fn sendinput_ctrl_v_probe() -> SendInputProbeResult {
    let inputs = vec![
        build_virtual_key_input(VK_CONTROL as u16, 0),
        build_virtual_key_input(b'V' as u16, 0),
        build_virtual_key_input(b'V' as u16, KEYEVENTF_KEYUP),
        build_virtual_key_input(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ];
    send_inputs_raw(&inputs)
}

pub(crate) fn clipboard_only_probe(text: &str) -> Result<ClipboardOnlyProbeResult, String> {
    let owner = unsafe { GetForegroundWindow() };
    write_clipboard_unicode_text(owner, text)?;
    let readback_text = read_clipboard_text(owner)?;
    Ok(ClipboardOnlyProbeResult { readback_text })
}

pub(crate) fn current_process_integrity_probe() -> ProcessIntegrityProbeResult {
    let process = unsafe { GetCurrentProcess() };
    process_integrity_probe_from_handle(process, false)
}

pub(crate) fn target_process_integrity_probe(process_id: u32) -> ProcessIntegrityProbeResult {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return ProcessIntegrityProbeResult {
            integrity_label: None,
            integrity_rid: None,
            elevated: None,
            detail: Some(last_os_error(
                "OpenProcess failed while querying target integrity",
            )),
        };
    }

    process_integrity_probe_from_handle(process, true)
}

fn send_unicode_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    require_foreground_window()?;

    let inputs = build_unicode_inputs(text);
    if inputs.is_empty() {
        return Ok(());
    }

    send_inputs(&inputs, "SendInput failed")
}

fn build_unicode_inputs(text: &str) -> Vec<INPUT> {
    let units = collect_text_input_units(text);
    let mut inputs = Vec::with_capacity(units.len() * 2);

    for unit in units {
        match unit {
            TextInputUnit::Unicode(unit) => {
                inputs.push(build_keyboard_input(unit, KEYEVENTF_UNICODE));
                inputs.push(build_keyboard_input(
                    unit,
                    KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                ));
            }
            TextInputUnit::Enter => {
                inputs.push(build_virtual_key_input(VK_RETURN as u16, 0));
                inputs.push(build_virtual_key_input(VK_RETURN as u16, KEYEVENTF_KEYUP));
            }
        }
    }

    inputs
}

fn replace_selected_text_with_clipboard(text: &str) -> Result<(), String> {
    let foreground_window = require_foreground_window().map_err(|detail| {
        format_selected_text_failure(
            SelectedTextFailureStage::SelectionReplace,
            SelectedTextFailureReason::NoForegroundWindow,
            detail,
        )
    })?;
    paste_text_via_clipboard(
        foreground_window,
        text,
        "strict clipboard replacement could not snapshot the current clipboard contents",
    )
}

fn paste_text_at_caret_with_clipboard(text: &str) -> Result<(), String> {
    let foreground_window = require_foreground_window()?;
    paste_text_via_clipboard(
        foreground_window,
        text,
        "temporary clipboard paste fallback could not snapshot the current clipboard contents",
    )
}

fn require_foreground_window() -> Result<HWND, String> {
    let foreground_window = unsafe { GetForegroundWindow() };
    if foreground_window.is_null() {
        Err("no foreground window was available for text insertion".to_string())
    } else {
        Ok(foreground_window)
    }
}

fn capture_selected_text_with_mode(
    mode: SelectionCaptureMode,
    release_virtual_keys: &[i32],
) -> Result<Option<String>, String> {
    let foreground_window = require_foreground_window().map_err(|detail| {
        format_selected_text_failure(
            SelectedTextFailureStage::ClipboardSnapshot,
            SelectedTextFailureReason::NoForegroundWindow,
            detail,
        )
    })?;
    let Some(clipboard_backup) = snapshot_clipboard(foreground_window, mode)? else {
        return Ok(None);
    };

    capture_and_restore_clipboard(
        || capture_selected_text_after_snapshot(foreground_window, mode, release_virtual_keys),
        || restore_clipboard(foreground_window, &clipboard_backup),
    )
}

fn capture_selected_text_after_snapshot(
    foreground_window: HWND,
    mode: SelectionCaptureMode,
    release_virtual_keys: &[i32],
) -> Result<Option<String>, String> {
    if !wait_for_hotkey_release(mode, release_virtual_keys)? {
        return Ok(None);
    }

    thread::sleep(mode.selection_settle_delay());
    let capture_attempts = mode.capture_attempts();
    for attempt in 1..=capture_attempts {
        clear_clipboard(foreground_window).map_err(|detail| {
            format_selected_text_failure(
                SelectedTextFailureStage::ClipboardSnapshot,
                classify_clipboard_error_reason(&detail),
                format!(
                    "capture attempt {attempt} of {capture_attempts} could not clear the clipboard before Ctrl+C: {detail}"
                ),
            )
        })?;
        let initial_sequence = clipboard_sequence_number();
        send_ctrl_shortcut(b'C').map_err(|detail| {
            format_selected_text_failure(
                SelectedTextFailureStage::SelectionCopy,
                SelectedTextFailureReason::CopyShortcutFailed,
                format!(
                    "capture attempt {attempt} of {capture_attempts} could not send the temporary Ctrl+C gesture: {detail}"
                ),
            )
        })?;

        match wait_for_clipboard_text(initial_sequence, mode)? {
            ClipboardWaitOutcome::TextReady => {
                let selected_text = read_clipboard_text(foreground_window).map_err(|detail| {
                    format_selected_text_failure(
                        SelectedTextFailureStage::SelectionRead,
                        classify_clipboard_error_reason(&detail),
                        detail,
                    )
                })?;
                let result = match selected_text {
                    Some(text) if !text.trim().is_empty() => Ok(Some(text)),
                    Some(_) => Err(format_selected_text_failure(
                        SelectedTextFailureStage::SelectionRead,
                        SelectedTextFailureReason::EmptySelection,
                        format!(
                            "capture attempt {attempt} of {capture_attempts} copied empty or whitespace-only text"
                        ),
                    )),
                    None => Err(format_selected_text_failure(
                        SelectedTextFailureStage::SelectionRead,
                        SelectedTextFailureReason::ClipboardChangedWithoutText,
                        format!(
                            "capture attempt {attempt} of {capture_attempts} changed the clipboard without leaving readable Unicode or ANSI text"
                        ),
                    )),
                };
                return result;
            }
            ClipboardWaitOutcome::ClipboardChangedWithoutText => {
                return Err(format_selected_text_failure(
                        SelectedTextFailureStage::SelectionCopy,
                        SelectedTextFailureReason::ClipboardChangedWithoutText,
                        format!(
                            "capture attempt {attempt} of {capture_attempts} changed the clipboard but did not expose Unicode or ANSI text; the focused app may be copying rich content only in this prototype"
                        ),
                    ));
            }
            ClipboardWaitOutcome::NoResult if matches!(mode, SelectionCaptureMode::BestEffort) => {
                return Ok(None);
            }
            ClipboardWaitOutcome::NoResult if attempt < capture_attempts => {
                thread::sleep(STRICT_SELECTION_CAPTURE_RETRY_DELAY);
            }
            ClipboardWaitOutcome::NoResult => {
                return Err(format_selected_text_failure(
                        SelectedTextFailureStage::SelectionCopy,
                        SelectedTextFailureReason::CopyTimedOut,
                        format!(
                            "capture attempt {attempt} of {capture_attempts} timed out after {} ms; the focused app may not support synthetic Ctrl+C selection capture reliably in this prototype",
                            mode.clipboard_wait_timeout().as_millis()
                        ),
                    ));
            }
        }
    }

    Ok(None)
}

fn clipboard_sequence_number() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

fn wait_for_hotkey_release(
    mode: SelectionCaptureMode,
    release_virtual_keys: &[i32],
) -> Result<bool, String> {
    let timeout = mode.hotkey_release_timeout();
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if all_release_virtual_keys_released(release_virtual_keys) {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(20));
    }

    match mode {
        SelectionCaptureMode::Strict => Err(format_selected_text_failure(
            SelectedTextFailureStage::HotkeyRelease,
            SelectedTextFailureReason::HotkeyStillPressed,
            format!(
                "selected-text capture waited {} ms for the hotkey modifiers to be released before attempting Ctrl+C",
                timeout.as_millis()
            ),
        )),
        SelectionCaptureMode::BestEffort => Ok(false),
    }
}

fn is_key_pressed(virtual_key: i32) -> bool {
    unsafe { (GetAsyncKeyState(virtual_key) as u16 & 0x8000) != 0 }
}

fn all_release_virtual_keys_released(release_virtual_keys: &[i32]) -> bool {
    all_release_virtual_keys_released_with(release_virtual_keys, is_key_pressed)
}

fn all_release_virtual_keys_released_with<F>(release_virtual_keys: &[i32], is_pressed: F) -> bool
where
    F: Fn(i32) -> bool,
{
    release_virtual_keys
        .iter()
        .all(|virtual_key| !is_pressed(*virtual_key))
}

fn hotkey_modifier_virtual_keys() -> &'static [i32] {
    &[
        VK_CONTROL as i32,
        VK_MENU as i32,
        VK_SHIFT as i32,
        VK_LWIN as i32,
        VK_RWIN as i32,
    ]
}

fn wait_for_clipboard_text(
    initial_sequence: u32,
    mode: SelectionCaptureMode,
) -> Result<ClipboardWaitOutcome, String> {
    let timeout = mode.clipboard_wait_timeout();
    let deadline = Instant::now() + timeout;
    let mut changed_without_text = false;

    while Instant::now() < deadline {
        if clipboard_sequence_number() != initial_sequence {
            if clipboard_has_readable_text() {
                return Ok(ClipboardWaitOutcome::TextReady);
            }
            changed_without_text = true;
        }
        thread::sleep(Duration::from_millis(20));
    }

    match mode {
        SelectionCaptureMode::Strict if changed_without_text => {
            Ok(ClipboardWaitOutcome::ClipboardChangedWithoutText)
        }
        SelectionCaptureMode::Strict | SelectionCaptureMode::BestEffort => {
            Ok(ClipboardWaitOutcome::NoResult)
        }
    }
}

fn clipboard_has_unicode_text() -> bool {
    unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 }
}

fn clipboard_has_ansi_text() -> bool {
    unsafe { IsClipboardFormatAvailable(CF_TEXT) != 0 }
}

fn clipboard_has_readable_text() -> bool {
    clipboard_has_unicode_text() || clipboard_has_ansi_text()
}

fn snapshot_clipboard(
    owner: HWND,
    mode: SelectionCaptureMode,
) -> Result<Option<ClipboardBackup>, String> {
    let _guard = ClipboardGuard::open(owner).map_err(|detail| {
        format_selected_text_failure(
            SelectedTextFailureStage::ClipboardSnapshot,
            SelectedTextFailureReason::ClipboardBusy,
            detail,
        )
    })?;
    let format_count = unsafe { CountClipboardFormats() };

    if format_count == 0 {
        return Ok(Some(ClipboardBackup::Empty));
    }

    if clipboard_has_unicode_text() {
        return Ok(Some(ClipboardBackup::UnicodeText(
            read_unicode_text_from_open_clipboard()
                .map_err(|detail| {
                    format_selected_text_failure(
                        SelectedTextFailureStage::ClipboardSnapshot,
                        SelectedTextFailureReason::ClipboardReadFailed,
                        detail,
                    )
                })?
                .unwrap_or_default(),
        )));
    }

    if clipboard_has_ansi_text() {
        return Ok(Some(ClipboardBackup::AnsiText(
            read_ansi_text_bytes_from_open_clipboard()
                .map_err(|detail| {
                    format_selected_text_failure(
                        SelectedTextFailureStage::ClipboardSnapshot,
                        SelectedTextFailureReason::ClipboardReadFailed,
                        detail,
                    )
                })?
                .unwrap_or_default(),
        )));
    }

    if !clipboard_has_readable_text() {
        return match mode {
            SelectionCaptureMode::Strict => Err(format_selected_text_failure(
                SelectedTextFailureStage::ClipboardSnapshot,
                SelectedTextFailureReason::UnsupportedClipboardContents,
                "temporary selected-text capture currently requires the clipboard to be empty or to expose Unicode or ANSI text so the probe can restore your clipboard text safely",
            )),
            SelectionCaptureMode::BestEffort => Ok(None),
        };
    }

    Ok(None)
}

fn clear_clipboard(owner: HWND) -> Result<(), String> {
    let _guard = ClipboardGuard::open(owner)?;
    let emptied = unsafe { EmptyClipboard() };
    if emptied == 0 {
        return Err(last_os_error("EmptyClipboard failed"));
    }
    Ok(())
}

fn read_clipboard_text(owner: HWND) -> Result<Option<String>, String> {
    let _guard = ClipboardGuard::open(owner)?;
    read_text_from_open_clipboard()
}

fn read_text_from_open_clipboard() -> Result<Option<String>, String> {
    if clipboard_has_unicode_text() {
        return read_unicode_text_from_open_clipboard();
    }

    if clipboard_has_ansi_text() {
        return read_ansi_text_from_open_clipboard();
    }

    Ok(None)
}

fn read_unicode_text_from_open_clipboard() -> Result<Option<String>, String> {
    if !clipboard_has_unicode_text() {
        return Ok(None);
    }

    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
    if handle.is_null() {
        return Err(last_os_error("GetClipboardData failed"));
    }

    let locked = unsafe { GlobalLock(handle) } as *const u16;
    if locked.is_null() {
        return Err(last_os_error("GlobalLock failed"));
    }

    let mut length = 0usize;
    unsafe {
        while *locked.add(length) != 0 {
            length += 1;
        }
    }

    let text = unsafe {
        let slice = std::slice::from_raw_parts(locked, length);
        String::from_utf16(slice)
            .map_err(|error| format!("clipboard text was not valid UTF-16: {error}"))?
    };

    unsafe {
        GlobalUnlock(handle);
    }

    Ok(Some(text))
}

fn read_ansi_text_from_open_clipboard() -> Result<Option<String>, String> {
    let Some(bytes) = read_ansi_text_bytes_from_open_clipboard()? else {
        return Ok(None);
    };
    let decoded = decode_ansi_text_bytes(&bytes)?;
    Ok(Some(decoded))
}

fn read_ansi_text_bytes_from_open_clipboard() -> Result<Option<Vec<u8>>, String> {
    if !clipboard_has_ansi_text() {
        return Ok(None);
    }

    let handle = unsafe { GetClipboardData(CF_TEXT) };
    if handle.is_null() {
        return Err(last_os_error("GetClipboardData failed"));
    }

    let locked = unsafe { GlobalLock(handle) } as *const u8;
    if locked.is_null() {
        return Err(last_os_error("GlobalLock failed"));
    }

    let mut length = 0usize;
    unsafe {
        while *locked.add(length) != 0 {
            length += 1;
        }
    }

    let bytes = unsafe { std::slice::from_raw_parts(locked, length).to_vec() };

    unsafe {
        GlobalUnlock(handle);
    }

    Ok(Some(bytes))
}

fn decode_ansi_text_bytes(bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() {
        return Ok(String::new());
    }

    let required_units = unsafe {
        MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, null_mut(), 0)
    };
    if required_units == 0 {
        return Err(last_os_error("MultiByteToWideChar failed"));
    }

    let mut wide = vec![0u16; required_units as usize];
    let converted_units = unsafe {
        MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            wide.len() as i32,
        )
    };
    if converted_units == 0 {
        return Err(last_os_error("MultiByteToWideChar failed"));
    }

    String::from_utf16(&wide[..converted_units as usize]).map_err(|error| {
        format!("clipboard ANSI text was not valid UTF-16 after conversion: {error}")
    })
}

fn write_clipboard_unicode_text(owner: HWND, text: &str) -> Result<(), String> {
    let mut wide_text = canonicalize_clipboard_newlines(text)
        .chars()
        .filter(|ch| *ch != '\0')
        .flat_map(|ch| {
            let mut buffer = [0_u16; 2];
            ch.encode_utf16(&mut buffer).to_vec()
        })
        .collect::<Vec<_>>();
    wide_text.push(0);
    let bytes_len = wide_text.len() * size_of::<u16>();

    let _guard = ClipboardGuard::open(owner)?;
    let emptied = unsafe { EmptyClipboard() };
    if emptied == 0 {
        return Err(last_os_error("EmptyClipboard failed"));
    }

    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes_len) };
    if handle.is_null() {
        return Err(last_os_error("GlobalAlloc failed"));
    }

    let locked = unsafe { GlobalLock(handle) } as *mut u16;
    if locked.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err(last_os_error("GlobalLock failed"));
    }

    unsafe {
        copy_nonoverlapping(wide_text.as_ptr(), locked, wide_text.len());
        GlobalUnlock(handle);
    }

    let set_result = unsafe { SetClipboardData(CF_UNICODETEXT, handle) };
    if set_result.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err(last_os_error("SetClipboardData failed"));
    }

    Ok(())
}

fn canonicalize_clipboard_newlines(text: &str) -> String {
    let mut canonical = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                canonical.push_str("\r\n");
            }
            '\n' => canonical.push_str("\r\n"),
            _ => canonical.push(ch),
        }
    }
    canonical
}

fn write_clipboard_ansi_text_bytes(owner: HWND, bytes: &[u8]) -> Result<(), String> {
    let encoded = normalize_ansi_clipboard_bytes(bytes);
    let bytes_len = encoded.len();

    let _guard = ClipboardGuard::open(owner)?;
    let emptied = unsafe { EmptyClipboard() };
    if emptied == 0 {
        return Err(last_os_error("EmptyClipboard failed"));
    }

    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes_len) };
    if handle.is_null() {
        return Err(last_os_error("GlobalAlloc failed"));
    }

    let locked = unsafe { GlobalLock(handle) } as *mut u8;
    if locked.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err(last_os_error("GlobalLock failed"));
    }

    unsafe {
        copy_nonoverlapping(encoded.as_ptr(), locked, encoded.len());
        GlobalUnlock(handle);
    }

    let set_result = unsafe { SetClipboardData(CF_TEXT, handle) };
    if set_result.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err(last_os_error("SetClipboardData failed"));
    }

    Ok(())
}

fn normalize_ansi_clipboard_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut encoded = bytes.to_vec();
    if encoded.last().copied() != Some(0) {
        encoded.push(0);
    }
    encoded
}

fn restore_clipboard(owner: HWND, backup: &ClipboardBackup) -> Result<(), String> {
    match backup {
        ClipboardBackup::Empty => clear_clipboard(owner),
        ClipboardBackup::UnicodeText(text) => write_clipboard_unicode_text(owner, text),
        ClipboardBackup::AnsiText(bytes) => write_clipboard_ansi_text_bytes(owner, bytes),
    }
}

fn send_ctrl_shortcut(key: u8) -> Result<(), String> {
    let inputs = vec![
        build_virtual_key_input(VK_CONTROL as u16, 0),
        build_virtual_key_input(key as u16, 0),
        build_virtual_key_input(key as u16, KEYEVENTF_KEYUP),
        build_virtual_key_input(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ];
    send_inputs(&inputs, "SendInput failed while sending Ctrl shortcut")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextInputUnit {
    Unicode(u16),
    Enter,
}

fn collect_text_input_units(text: &str) -> Vec<TextInputUnit> {
    let mut units = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\0' => {}
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                units.push(TextInputUnit::Enter);
            }
            '\n' => units.push(TextInputUnit::Enter),
            _ => {
                let mut buffer = [0_u16; 2];
                units.extend(
                    ch.encode_utf16(&mut buffer)
                        .iter()
                        .copied()
                        .map(TextInputUnit::Unicode),
                );
            }
        }
    }
    units
}

fn build_keyboard_input(scan_code: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: scan_code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn build_virtual_key_input(virtual_key: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_inputs(inputs: &[INPUT], context: &str) -> Result<(), String> {
    let result = send_inputs_raw(inputs);
    if result.accepted_events != result.expected_events {
        return Err(format_send_input_error_with_code(
            context,
            result.accepted_events,
            result.expected_events,
            result.last_error,
        ));
    }
    Ok(())
}

fn send_inputs_raw(inputs: &[INPUT]) -> SendInputProbeResult {
    let (accepted_events, last_error) = unsafe {
        SetLastError(0);
        let sent = SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        );
        (sent, GetLastError())
    };
    SendInputProbeResult {
        accepted_events,
        expected_events: inputs.len() as u32,
        last_error,
        last_error_message: if last_error == 0 {
            None
        } else {
            Some(std::io::Error::from_raw_os_error(last_error as i32).to_string())
        },
    }
}

fn last_os_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}

fn format_send_input_error_with_code(
    context: &str,
    sent: u32,
    expected: u32,
    last_error: u32,
) -> String {
    if last_error != 0 {
        let os_error = std::io::Error::from_raw_os_error(last_error as i32);
        return format!(
            "{context}: SendInput accepted {sent} of {expected} events; Win32 error {last_error}: {os_error}"
        );
    }

    format!(
        "{context}: SendInput accepted {sent} of {expected} events without exposing a Win32 error code"
    )
}

fn paste_text_via_clipboard(
    owner: HWND,
    text: &str,
    backup_error_context: &str,
) -> Result<(), String> {
    let clipboard_backup = snapshot_clipboard(owner, SelectionCaptureMode::Strict)?
        .ok_or_else(|| backup_error_context.to_string())?;
    let paste_result: Result<(), String> = (|| {
        write_clipboard_unicode_text(owner, text).map_err(|detail| {
            format_selected_text_failure(
                SelectedTextFailureStage::SelectionReplace,
                SelectedTextFailureReason::ClipboardWriteFailed,
                detail,
            )
        })?;
        send_ctrl_shortcut(b'V').map_err(|detail| {
            format_selected_text_failure(
                SelectedTextFailureStage::SelectionReplace,
                SelectedTextFailureReason::PasteShortcutFailed,
                detail,
            )
        })?;
        thread::sleep(CLIPBOARD_PASTE_SETTLE_DELAY);
        Ok(())
    })();

    let restore_result = restore_clipboard(owner, &clipboard_backup).map_err(|detail| {
        format_selected_text_failure(
            SelectedTextFailureStage::ClipboardRestore,
            SelectedTextFailureReason::ClipboardRestoreFailed,
            detail,
        )
    });
    combine_primary_and_restore_results(paste_result, restore_result, report_transport_diagnostic)
}

pub(crate) fn combine_primary_and_restore_results(
    primary_result: Result<(), String>,
    restore_result: Result<(), String>,
    mut report: impl FnMut(&str),
) -> Result<(), String> {
    match (primary_result, restore_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary_error), Ok(())) => Err(primary_error),
        (Ok(()), Err(_restore_error)) => {
            // Ctrl+V already succeeded. Returning an error here would invite a second
            // insertion through the fallback transport and duplicate the user's text.
            report(
                "paste_succeeded=true clipboard_restore_succeeded=false error_category=clipboard_restore_failed nonfatal=true fallback_allowed=false",
            );
            Ok(())
        }
        (Err(primary_error), Err(restore_error)) => Err(format!(
            "{primary_error}; restoring the previous clipboard contents also failed: {restore_error}"
        )),
    }
}

fn process_integrity_probe_from_handle(
    process: HANDLE,
    close_process: bool,
) -> ProcessIntegrityProbeResult {
    let mut token = null_mut();
    let opened = unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) };
    let result = if opened == 0 {
        ProcessIntegrityProbeResult {
            integrity_label: None,
            integrity_rid: None,
            elevated: None,
            detail: Some(last_os_error("OpenProcessToken failed")),
        }
    } else {
        let integrity = query_token_integrity(token);
        let elevated = query_token_elevation(token);
        unsafe {
            CloseHandle(token);
        }

        match (integrity, elevated) {
            (Ok((rid, label)), Ok(elevated)) => ProcessIntegrityProbeResult {
                integrity_label: Some(label),
                integrity_rid: Some(rid),
                elevated: Some(elevated),
                detail: None,
            },
            (Ok((rid, label)), Err(detail)) => ProcessIntegrityProbeResult {
                integrity_label: Some(label),
                integrity_rid: Some(rid),
                elevated: None,
                detail: Some(detail),
            },
            (Err(detail), Ok(elevated)) => ProcessIntegrityProbeResult {
                integrity_label: None,
                integrity_rid: None,
                elevated: Some(elevated),
                detail: Some(detail),
            },
            (Err(integrity_detail), Err(elevation_detail)) => ProcessIntegrityProbeResult {
                integrity_label: None,
                integrity_rid: None,
                elevated: None,
                detail: Some(format!("{integrity_detail}; {elevation_detail}")),
            },
        }
    };

    if close_process {
        unsafe {
            CloseHandle(process);
        }
    }

    result
}

fn query_token_integrity(token: HANDLE) -> Result<(u32, String), String> {
    let mut length = 0_u32;
    unsafe {
        GetTokenInformation(token, TokenIntegrityLevel, null_mut(), 0, &mut length);
    }
    if length == 0 {
        return Err(last_os_error(
            "GetTokenInformation(TokenIntegrityLevel) length query failed",
        ));
    }

    let mut buffer = vec![0_u8; length as usize];
    let queried = unsafe {
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            buffer.len() as u32,
            &mut length,
        )
    };
    if queried == 0 {
        return Err(last_os_error(
            "GetTokenInformation(TokenIntegrityLevel) failed",
        ));
    }

    let label = unsafe { &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL) };
    let sid = label.Label.Sid;
    if sid.is_null() {
        return Err("token integrity SID was null".to_string());
    }

    let count = unsafe { GetSidSubAuthorityCount(sid) };
    if count.is_null() || unsafe { *count } == 0 {
        return Err("token integrity SID had no subauthorities".to_string());
    }

    let subauthority = unsafe { GetSidSubAuthority(sid, (*count - 1) as u32) };
    if subauthority.is_null() {
        return Err("token integrity subauthority was null".to_string());
    }

    let rid = unsafe { *subauthority };
    Ok((rid, integrity_label_for_rid(rid).to_string()))
}

fn query_token_elevation(token: HANDLE) -> Result<bool, String> {
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut length = size_of::<TOKEN_ELEVATION>() as u32;
    let queried = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            length,
            &mut length,
        )
    };
    if queried == 0 {
        return Err(last_os_error("GetTokenInformation(TokenElevation) failed"));
    }

    Ok(elevation.TokenIsElevated != 0)
}

fn integrity_label_for_rid(rid: u32) -> &'static str {
    if rid >= SECURITY_MANDATORY_PROTECTED_PROCESS_RID as u32 {
        "protected-process"
    } else if rid >= SECURITY_MANDATORY_SYSTEM_RID as u32 {
        "system"
    } else if rid >= SECURITY_MANDATORY_HIGH_RID as u32 {
        "high"
    } else if rid >= SECURITY_MANDATORY_MEDIUM_PLUS_RID {
        "medium-plus"
    } else if rid >= SECURITY_MANDATORY_MEDIUM_RID as u32 {
        "medium"
    } else if rid >= SECURITY_MANDATORY_LOW_RID as u32 {
        "low"
    } else if rid >= SECURITY_MANDATORY_UNTRUSTED_RID as u32 {
        "untrusted"
    } else {
        "unknown"
    }
}

enum ClipboardBackup {
    Empty,
    UnicodeText(String),
    AnsiText(Vec<u8>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionCaptureMode {
    Strict,
    BestEffort,
}

impl SelectionCaptureMode {
    fn capture_attempts(self) -> u32 {
        match self {
            SelectionCaptureMode::Strict => STRICT_SELECTION_CAPTURE_ATTEMPTS,
            SelectionCaptureMode::BestEffort => 1,
        }
    }

    fn hotkey_release_timeout(self) -> Duration {
        match self {
            SelectionCaptureMode::Strict => HOTKEY_RELEASE_TIMEOUT,
            SelectionCaptureMode::BestEffort => FAST_HOTKEY_RELEASE_TIMEOUT,
        }
    }

    fn selection_settle_delay(self) -> Duration {
        match self {
            SelectionCaptureMode::Strict => HOTKEY_SELECTION_SETTLE_DELAY,
            SelectionCaptureMode::BestEffort => FAST_HOTKEY_SELECTION_SETTLE_DELAY,
        }
    }

    fn clipboard_wait_timeout(self) -> Duration {
        match self {
            SelectionCaptureMode::Strict => CLIPBOARD_WAIT_TIMEOUT,
            SelectionCaptureMode::BestEffort => FAST_CLIPBOARD_WAIT_TIMEOUT,
        }
    }
}

enum ClipboardWaitOutcome {
    TextReady,
    ClipboardChangedWithoutText,
    NoResult,
}

fn capture_and_restore_clipboard(
    capture: impl FnOnce() -> Result<Option<String>, String>,
    restore: impl FnOnce() -> Result<(), String>,
) -> Result<Option<String>, String> {
    let primary_result = capture();
    let restore_result = restore().map_err(|detail| {
        format_selected_text_failure(
            SelectedTextFailureStage::ClipboardRestore,
            SelectedTextFailureReason::ClipboardRestoreFailed,
            detail,
        )
    });

    match (primary_result, restore_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(primary_error), Ok(())) => Err(primary_error),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(primary_error), Err(restore_error)) => Err(format!(
            "{primary_error}; restoring the previous clipboard contents also failed: {restore_error}"
        )),
    }
}

fn classify_clipboard_error_reason(detail: &str) -> SelectedTextFailureReason {
    if detail.contains("OpenClipboard failed") {
        SelectedTextFailureReason::ClipboardBusy
    } else if detail.contains("GetClipboardData failed")
        || detail.contains("GlobalLock failed")
        || detail.contains("MultiByteToWideChar failed")
        || detail.contains("clipboard text was not valid UTF-16")
        || detail.contains("clipboard ANSI text was not valid UTF-16")
    {
        SelectedTextFailureReason::ClipboardReadFailed
    } else if detail.contains("SetClipboardData failed")
        || detail.contains("GlobalAlloc failed")
        || detail.contains("EmptyClipboard failed")
    {
        SelectedTextFailureReason::ClipboardWriteFailed
    } else {
        SelectedTextFailureReason::ClipboardBusy
    }
}

struct ClipboardGuard;

impl ClipboardGuard {
    fn open(owner: HWND) -> Result<Self, String> {
        let deadline = Instant::now() + CLIPBOARD_OPEN_TIMEOUT;
        let mut attempts = 0_u32;

        loop {
            attempts += 1;
            let opened = unsafe { OpenClipboard(owner) };
            if opened != 0 {
                return Ok(Self);
            }

            let last_error = std::io::Error::last_os_error();
            if Instant::now() >= deadline {
                return Err(format!(
                    "OpenClipboard failed after {attempts} attempt(s) across {} ms: {}",
                    CLIPBOARD_OPEN_TIMEOUT.as_millis(),
                    last_error
                ));
            }

            thread::sleep(CLIPBOARD_OPEN_RETRY_DELAY);
        }
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TextInputUnit, all_release_virtual_keys_released_with, canonicalize_clipboard_newlines,
        collect_text_input_units, combine_primary_and_restore_results, commit_text_with_transports,
        decode_ansi_text_bytes, format_send_input_error_with_code, formatted_multiline_reason,
        normalize_ansi_clipboard_bytes,
        capture_and_restore_clipboard,
    };
    use shared_protocol::{CommitStatus, CommitTransport};
    use std::cell::RefCell;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LWIN, VK_RWIN, VK_SHIFT};

    #[test]
    fn selection_failure_always_restores_clipboard_once() {
        for failure in ["copy failed", "clipboard read failed", "wait failed"] {
            for restore_fails in [false, true] {
                let restores = std::cell::Cell::new(0);
                let result = capture_and_restore_clipboard(
                    || Err(failure.to_string()),
                    || {
                        restores.set(restores.get() + 1);
                        if restore_fails { Err("restore busy".into()) } else { Ok(()) }
                    },
                );
                assert_eq!(restores.get(), 1);
                let error = result.unwrap_err();
                assert!(error.contains(failure));
                assert_eq!(error.contains("restore busy"), restore_fails);
            }
        }
    }

    #[test]
    fn multiline_bullet_text_chooses_clipboard_first_when_safe() {
        let attempts = RefCell::new(Vec::new());
        let diagnostics = RefCell::new(Vec::new());
        let result = commit_text_with_transports(
            "- first item\n- second item\n- third item",
            |_| {
                attempts.borrow_mut().push("direct");
                Ok(())
            },
            |_| {
                attempts.borrow_mut().push("clipboard");
                Ok(())
            },
            |detail| diagnostics.borrow_mut().push(detail.to_string()),
        );

        assert_eq!(attempts.into_inner(), vec!["clipboard"]);
        assert_eq!(result.status, CommitStatus::Success);
        assert_eq!(result.transport, CommitTransport::ClipboardPasteFallback);
        assert!(diagnostics.into_inner()[0].contains("contains a newline"));
    }

    #[test]
    fn normal_single_line_text_still_uses_direct_unicode_send_input() {
        let attempts = RefCell::new(Vec::new());
        let result = commit_text_with_transports(
            "ordinary dictation",
            |_| {
                attempts.borrow_mut().push("direct");
                Ok(())
            },
            |_| {
                attempts.borrow_mut().push("clipboard");
                Ok(())
            },
            |_| {},
        );

        assert_eq!(attempts.into_inner(), vec!["direct"]);
        assert_eq!(result.status, CommitStatus::Success);
        assert_eq!(result.transport, CommitTransport::DirectUnicodeSendInput);
    }

    #[test]
    fn failed_multiline_paste_never_sends_enter_through_direct_input() {
        for text in ["first\nsecond", "first\rsecond", "first\r\nsecond"] {
            for error in ["clipboard contains unsupported formats", "clipboard busy", "paste failed"] {
                let attempts = RefCell::new(Vec::new());
                let diagnostics = RefCell::new(Vec::new());
                let result = commit_text_with_transports(
                    text,
                    |_| panic!("multiline failure must never invoke direct input"),
                    |received| {
                        assert_eq!(received, text);
                        attempts.borrow_mut().push("clipboard");
                        Err(error.to_string())
                    },
                    |detail| diagnostics.borrow_mut().push(detail.to_string()),
                );
                assert_eq!(attempts.into_inner(), vec!["clipboard"]);
                assert_eq!(result.status, CommitStatus::Failed(error.to_string()));
                assert_eq!(result.transport, CommitTransport::ClipboardPasteFallback);
                assert!(diagnostics.into_inner()[0].contains("fallback_allowed=false"));
            }
        }
    }

    #[test]
    fn all_single_line_text_remains_direct_unicode_eligible() {
        for text in ["- item", "• item", "1. item", "ordinary dictation"] {
            assert!(formatted_multiline_reason(text).is_none());
        }
    }

    #[test]
    fn clipboard_newlines_are_canonicalized_to_crlf() {
        assert_eq!(
            canonicalize_clipboard_newlines("one\ntwo\rthree\r\nfour"),
            "one\r\ntwo\r\nthree\r\nfour"
        );
    }

    #[test]
    fn transport_diagnostics_are_reported() {
        let diagnostics = RefCell::new(Vec::new());
        let result = commit_text_with_transports(
            "plain text",
            |_| Ok(()),
            |_| panic!("clipboard should not be attempted"),
            |detail| diagnostics.borrow_mut().push(detail.to_string()),
        );

        assert_eq!(result.transport, CommitTransport::DirectUnicodeSendInput);
        let diagnostics = diagnostics.into_inner();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("selected DirectUnicodeSendInput"));
        assert!(diagnostics[0].contains("normal single-line text"));
    }

    #[test]
    fn converts_all_newline_forms_into_enter_key_events_for_send_input() {
        assert_eq!(
            collect_text_input_units("a\nb\r\nc\rd"),
            vec![
                TextInputUnit::Unicode('a' as u16),
                TextInputUnit::Enter,
                TextInputUnit::Unicode('b' as u16),
                TextInputUnit::Enter,
                TextInputUnit::Unicode('c' as u16),
                TextInputUnit::Enter,
                TextInputUnit::Unicode('d' as u16),
            ]
        );
    }

    #[test]
    fn preserves_surrogate_pairs_for_unicode_input() {
        assert_eq!(
            collect_text_input_units("A\u{1F642}"),
            vec![
                TextInputUnit::Unicode(65_u16),
                TextInputUnit::Unicode(55357_u16),
                TextInputUnit::Unicode(56898_u16),
            ]
        );
    }

    #[test]
    fn send_input_error_message_handles_missing_win32_error_code() {
        let message = format_send_input_error_with_code("SendInput failed", 0, 6, 0);
        assert!(message.contains("accepted 0 of 6 events"));
    }

    #[test]
    fn clipboard_error_combines_primary_and_restore_failures() {
        let error = combine_primary_and_restore_results(
            Err("paste failed".to_string()),
            Err("restore failed".to_string()),
            |_| {},
        )
        .expect_err("combined clipboard failure should be returned");

        assert!(error.contains("paste failed"));
        assert!(error.contains("restore failed"));
    }

    #[test]
    fn successful_paste_never_retries_insertion_when_restoration_fails() {
        for restore_fails in [false, true] {
            let mut paste_calls = 0;
            let mut warnings = Vec::new();
            let result = commit_text_with_transports(
                "Hello\n\n• Team: update\n• Second item",
                |_| panic!("successful paste must not trigger direct fallback"),
                |text| {
                    paste_calls += 1;
                    assert_eq!(
                        canonicalize_clipboard_newlines(text),
                        "Hello\r\n\r\n• Team: update\r\n• Second item"
                    );
                    combine_primary_and_restore_results(
                        Ok(()),
                        if restore_fails {
                            Err("private error sentinel".to_string())
                        } else {
                            Ok(())
                        },
                        |warning| warnings.push(warning.to_string()),
                    )
                },
                |_| {},
            );
            assert_eq!(result.status, CommitStatus::Success);
            assert_eq!(result.transport, CommitTransport::ClipboardPasteFallback);
            assert_eq!(paste_calls, 1);
            assert_eq!(warnings.len(), usize::from(restore_fails));
            if restore_fails {
                assert!(warnings[0].contains("error_category=clipboard_restore_failed"));
                assert!(!warnings[0].contains("private error sentinel"));
            }
        }
    }

    #[test]
    fn blank_lines_emit_two_real_enter_key_pairs() {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_KEYUP, VK_RETURN};
        for text in ["a\n\nb", "a\r\n\r\nb", "a\r\rb"] {
            let inputs = super::build_unicode_inputs(text);
            assert_eq!(inputs.len(), 8);
            for offset in [2, 4] {
                let down = unsafe { inputs[offset].Anonymous.ki };
                let up = unsafe { inputs[offset + 1].Anonymous.ki };
                assert_eq!(down.wVk, VK_RETURN as u16);
                assert_eq!(down.dwFlags, 0);
                assert_eq!(up.wVk, VK_RETURN as u16);
                assert_eq!(up.dwFlags, KEYEVENTF_KEYUP);
            }
        }
    }

    #[test]
    fn normalizes_ansi_clipboard_bytes_with_single_trailing_nul() {
        assert_eq!(normalize_ansi_clipboard_bytes(b"hello"), b"hello\0");
        assert_eq!(normalize_ansi_clipboard_bytes(b"hello\0"), b"hello\0");
    }

    #[test]
    fn decodes_ascii_ansi_clipboard_text() {
        let decoded = decode_ansi_text_bytes(b"plain text").expect("ansi text should decode");

        assert_eq!(decoded, "plain text");
    }

    #[test]
    fn waits_for_shift_modifier_release_during_selection_capture() {
        let all_released =
            all_release_virtual_keys_released_with(&[VK_SHIFT as i32], |virtual_key| {
                virtual_key == VK_SHIFT as i32
            });

        assert!(!all_released);
    }

    #[test]
    fn waits_for_meta_modifier_release_during_selection_capture() {
        let all_released = all_release_virtual_keys_released_with(
            &[VK_LWIN as i32, VK_RWIN as i32],
            |virtual_key| virtual_key == VK_LWIN as i32 || virtual_key == VK_RWIN as i32,
        );

        assert!(!all_released);
    }
}
