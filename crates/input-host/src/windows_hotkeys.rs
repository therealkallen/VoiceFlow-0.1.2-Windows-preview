#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use std::sync::Mutex;

#[cfg(windows)]
use shared_protocol::{KeyModifier, RuntimeSettings, Shortcut, TriggerMode};
#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
    UnregisterHotKey,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE, PeekMessageW, QS_HOTKEY,
    WM_HOTKEY,
};

use crate::host::ShortcutAdapter;
use crate::windows_shortcut::{collect_release_virtual_keys, parse_virtual_key};

const FIRST_HOTKEY_ID: i32 = 0x1200;

#[derive(Debug)]
pub struct WindowsHotkeyAdapter {
    registration_state: Mutex<RegistrationState>,
}

impl Default for WindowsHotkeyAdapter {
    fn default() -> Self {
        Self {
            registration_state: Mutex::new(RegistrationState::default()),
        }
    }
}

#[derive(Debug)]
struct RegistrationState {
    bindings: Vec<RegisteredBinding>,
    next_hotkey_id: i32,
}

impl Default for RegistrationState {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
            next_hotkey_id: FIRST_HOTKEY_ID,
        }
    }
}

#[derive(Clone, Debug)]
struct RegisteredBinding {
    id: i32,
    shortcut: ParsedShortcut,
    trigger: TriggerMode,
}

#[derive(Clone, Debug)]
struct DesiredBinding {
    shortcut: ParsedShortcut,
    trigger: TriggerMode,
}

impl ShortcutAdapter for WindowsHotkeyAdapter {
    fn register(&self, settings: &RuntimeSettings) -> Result<(), String> {
        let mut state = self
            .registration_state
            .lock()
            .map_err(|_| "failed to lock hotkey registration state".to_string())?;
        if !state.bindings.is_empty() {
            return Err("hotkeys are already registered".to_string());
        }
        install_bindings(&mut state, desired_bindings(settings)?)
    }

    fn rebind(&self, _current: &RuntimeSettings, updated: &RuntimeSettings) -> Result<(), String> {
        let desired = desired_bindings(updated)?;
        let mut state = self
            .registration_state
            .lock()
            .map_err(|_| "failed to lock hotkey registration state".to_string())?;
        install_bindings(&mut state, desired)
    }

    fn wait_for_trigger(&self) -> Result<TriggerMode, String> {
        wait_for_hotkey_message(&self.registration_state, None)?
            .ok_or_else(|| "message loop ended before a hotkey was received".to_string())
    }

    fn wait_for_trigger_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Option<TriggerMode>, String> {
        wait_for_hotkey_message(&self.registration_state, Some(timeout))
    }

    fn wait_for_trigger_release(
        &self,
        trigger_mode: &TriggerMode,
        timeout: Option<std::time::Duration>,
    ) -> Result<bool, String> {
        let shortcut = {
            let state = self
                .registration_state
                .lock()
                .map_err(|_| "failed to lock hotkey registration state".to_string())?;
            state
                .bindings
                .iter()
                .find(|binding| &binding.trigger == trigger_mode)
                .map(|binding| binding.shortcut.clone())
        }
        .ok_or_else(|| {
            format!(
                "no registered shortcut metadata was available for {:?}",
                trigger_mode
            )
        })?;

        wait_for_shortcut_release(&shortcut, timeout)
    }
}

impl Drop for WindowsHotkeyAdapter {
    fn drop(&mut self) {
        if let Ok(state) = self.registration_state.lock() {
            for binding in &state.bindings {
                unregister_hotkey(binding.id);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedShortcut {
    modifiers: u32,
    virtual_key: u32,
    release_virtual_keys: Vec<i32>,
}

fn desired_bindings(settings: &RuntimeSettings) -> Result<Vec<DesiredBinding>, String> {
    let dictation = parse_shortcut(&settings.dictation_shortcut)?;
    let mut desired = vec![DesiredBinding {
        shortcut: dictation.clone(),
        trigger: TriggerMode::GlobalShortcut,
    }];
    if settings.dictation_shortcut != settings.edit_shortcut {
        desired.push(DesiredBinding {
            shortcut: parse_shortcut(&settings.edit_shortcut)?,
            trigger: TriggerMode::EditShortcut,
        });
    }
    Ok(desired)
}

fn install_bindings(
    state: &mut RegistrationState,
    desired: Vec<DesiredBinding>,
) -> Result<(), String> {
    let mut next_bindings = Vec::with_capacity(desired.len());
    let mut reused_ids = Vec::new();
    let mut newly_registered_ids = Vec::new();

    for desired_binding in desired {
        if let Some(existing) = state.bindings.iter().find(|binding| {
            binding.shortcut == desired_binding.shortcut && !reused_ids.contains(&binding.id)
        }) {
            reused_ids.push(existing.id);
            next_bindings.push(RegisteredBinding {
                id: existing.id,
                shortcut: desired_binding.shortcut,
                trigger: desired_binding.trigger,
            });
            continue;
        }

        let id = state.next_hotkey_id;
        state.next_hotkey_id = state.next_hotkey_id.saturating_add(1);
        if let Err(error) = register_hotkey(id, &desired_binding.shortcut) {
            for registered_id in newly_registered_ids {
                unregister_hotkey(registered_id);
            }
            return Err(error);
        }
        newly_registered_ids.push(id);
        next_bindings.push(RegisteredBinding {
            id,
            shortcut: desired_binding.shortcut,
            trigger: desired_binding.trigger,
        });
    }

    for existing in &state.bindings {
        if !reused_ids.contains(&existing.id) {
            unregister_hotkey(existing.id);
        }
    }
    state.bindings = next_bindings;
    Ok(())
}

fn parse_shortcut(shortcut: &Shortcut) -> Result<ParsedShortcut, String> {
    let mut modifiers = MOD_NOREPEAT;
    for modifier in &shortcut.modifiers {
        modifiers |= match modifier {
            KeyModifier::Control => MOD_CONTROL,
            KeyModifier::Alt => MOD_ALT,
            KeyModifier::Shift => MOD_SHIFT,
            KeyModifier::Meta => MOD_WIN,
        };
    }

    Ok(ParsedShortcut {
        modifiers,
        virtual_key: parse_virtual_key(&shortcut.key)?,
        release_virtual_keys: collect_release_virtual_keys(shortcut)?,
    })
}

fn register_hotkey(id: i32, shortcut: &ParsedShortcut) -> Result<(), String> {
    let result =
        unsafe { RegisterHotKey(null_mut(), id, shortcut.modifiers, shortcut.virtual_key) };

    if result == 0 {
        Err(last_os_error("RegisterHotKey failed"))
    } else {
        Ok(())
    }
}

fn unregister_hotkey(id: i32) {
    unsafe {
        UnregisterHotKey(null_mut(), id);
    }
}

fn wait_for_hotkey_message(
    registration_state: &Mutex<RegistrationState>,
    timeout: Option<std::time::Duration>,
) -> Result<Option<TriggerMode>, String> {
    let deadline = timeout.map(|duration| std::time::Instant::now() + duration);

    loop {
        let timeout_ms = if let Some(deadline) = deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            remaining.as_millis().min(u32::MAX as u128) as u32
        } else {
            u32::MAX
        };

        let wait_result = unsafe {
            MsgWaitForMultipleObjectsEx(0, null_mut(), timeout_ms, QS_HOTKEY, MWMO_INPUTAVAILABLE)
        };

        const WAIT_OBJECT_0: u32 = 0x0000_0000;
        const WAIT_TIMEOUT: u32 = 0x0000_0102;
        const WAIT_FAILED: u32 = 0xFFFF_FFFF;

        if wait_result == WAIT_TIMEOUT {
            return Ok(None);
        }

        if wait_result == WAIT_FAILED {
            return Err(last_os_error("MsgWaitForMultipleObjectsEx failed"));
        }

        if wait_result != WAIT_OBJECT_0 {
            continue;
        }

        let mut message = unsafe { std::mem::zeroed::<MSG>() };
        loop {
            let has_message = unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) };
            if has_message == 0 {
                break;
            }

            if message.message == WM_HOTKEY {
                let trigger = registration_state
                    .lock()
                    .map_err(|_| "failed to lock hotkey registration state".to_string())?
                    .bindings
                    .iter()
                    .find(|binding| binding.id == message.wParam as i32)
                    .map(|binding| binding.trigger.clone());

                if let Some(trigger) = trigger {
                    return Ok(Some(trigger));
                }
            }
        }
    }
}

fn wait_for_shortcut_release(
    shortcut: &ParsedShortcut,
    timeout: Option<std::time::Duration>,
) -> Result<bool, String> {
    let deadline = timeout.map(|duration| std::time::Instant::now() + duration);

    loop {
        if shortcut
            .release_virtual_keys
            .iter()
            .all(|virtual_key| !is_virtual_key_pressed(*virtual_key))
        {
            return Ok(true);
        }

        if let Some(deadline) = deadline {
            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn is_virtual_key_pressed(virtual_key: i32) -> bool {
    unsafe { (GetAsyncKeyState(virtual_key) as u16 & 0x8000) != 0 }
}

fn last_os_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}
