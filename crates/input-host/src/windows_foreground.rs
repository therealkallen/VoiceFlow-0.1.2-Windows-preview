use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ForegroundAppMetadata {
    pub app_fingerprint: String,
    pub process_name: Option<String>,
    pub process_id: Option<u32>,
    pub window_class: Option<String>,
    pub window_title: Option<String>,
    pub capture_state: String,
}

impl Default for ForegroundAppMetadata {
    fn default() -> Self {
        Self::unknown("unknown")
    }
}

impl ForegroundAppMetadata {
    pub(crate) fn unknown(capture_state: &str) -> Self {
        Self {
            app_fingerprint: derive_app_fingerprint(None, None),
            process_name: None,
            process_id: None,
            window_class: None,
            window_title: None,
            capture_state: capture_state.to_string(),
        }
    }
}

pub(crate) fn derive_app_fingerprint(
    process_name: Option<&str>,
    window_class: Option<&str>,
) -> String {
    if let Some(process_name) = normalize_process_name(process_name) {
        return format!("process:{process_name}");
    }

    if let Some(window_class) = normalize_window_class(window_class) {
        return format!("process:unknown|class:{window_class}");
    }

    "process:unknown".to_string()
}

fn normalize_process_name(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }

    let basename = Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed)
        .trim();
    if basename.is_empty() {
        None
    } else {
        Some(basename.to_ascii_lowercase())
    }
}

fn normalize_window_class(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_ascii_lowercase())
    }
}

fn normalize_window_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(windows)]
pub(crate) fn capture_foreground_app_metadata() -> ForegroundAppMetadata {
    use windows_sys::Win32::Foundation::{CloseHandle, HWND};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };

    fn read_window_class(hwnd: HWND) -> Option<String> {
        let mut buffer = [0_u16; 256];
        let length = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if length <= 0 {
            None
        } else {
            normalize_window_text(Some(String::from_utf16_lossy(&buffer[..length as usize])))
        }
    }

    fn read_window_title(hwnd: HWND) -> Option<String> {
        let mut buffer = [0_u16; 512];
        let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if length <= 0 {
            None
        } else {
            normalize_window_text(Some(String::from_utf16_lossy(&buffer[..length as usize])))
        }
    }

    fn read_process_name(process_id: u32) -> Option<String> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return None;
        }

        let mut buffer = [0_u16; 1024];
        let mut buffer_length = buffer.len() as u32;
        let query_ok = unsafe {
            QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut buffer_length)
        } != 0;
        unsafe {
            CloseHandle(process);
        }
        if !query_ok || buffer_length == 0 {
            return None;
        }

        normalize_process_name(Some(&String::from_utf16_lossy(
            &buffer[..buffer_length as usize],
        )))
    }

    let foreground_window = unsafe { GetForegroundWindow() };
    if foreground_window.is_null() {
        return ForegroundAppMetadata::unknown("no-foreground-window");
    }

    let window_class = read_window_class(foreground_window);
    let window_title = read_window_title(foreground_window);
    let mut process_id = 0_u32;
    unsafe {
        GetWindowThreadProcessId(foreground_window, &mut process_id);
    }

    let process_id = if process_id == 0 {
        None
    } else {
        Some(process_id)
    };
    let process_name = process_id.and_then(read_process_name);
    let capture_state = if process_name.is_some() {
        "captured"
    } else if process_id.is_some() {
        "process-query-failed"
    } else if window_class.is_some() || window_title.is_some() {
        "window-only"
    } else {
        "foreground-window-empty"
    };

    ForegroundAppMetadata {
        app_fingerprint: derive_app_fingerprint(process_name.as_deref(), window_class.as_deref()),
        process_name,
        process_id,
        window_class,
        window_title,
        capture_state: capture_state.to_string(),
    }
}

#[cfg(not(windows))]
pub(crate) fn capture_foreground_app_metadata() -> ForegroundAppMetadata {
    ForegroundAppMetadata::unknown("unsupported-platform")
}

#[cfg(test)]
mod tests {
    use super::{derive_app_fingerprint, normalize_process_name};

    #[test]
    fn fingerprint_prefers_normalized_process_name() {
        assert_eq!(
            derive_app_fingerprint(
                Some("C:\\Program Files\\Microsoft Office\\root\\Office16\\WINWORD.EXE"),
                Some("OpusApp"),
            ),
            "process:winword.exe"
        );
    }

    #[test]
    fn fingerprint_falls_back_to_window_class_without_title_fragmentation() {
        assert_eq!(
            derive_app_fingerprint(None, Some("Chrome_WidgetWin_1")),
            "process:unknown|class:chrome_widgetwin_1"
        );
    }

    #[test]
    fn normalize_process_name_extracts_basename() {
        assert_eq!(
            normalize_process_name(Some("C:\\Windows\\System32\\NOTEPAD.EXE")),
            Some("notepad.exe".to_string())
        );
    }
}
