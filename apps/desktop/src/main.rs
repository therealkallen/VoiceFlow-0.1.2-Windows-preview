#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod service;

use service::Services;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_error(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "VoiceFlow".encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn main() {
    let smoke = std::env::args().any(|arg| arg == "--smoke-test");
    let settings_only = smoke || std::env::args().any(|arg| arg == "--settings-only");
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_settings(app)
        }))
        .setup(move |app| {
            let services = Services::start(!settings_only).map_err(std::io::Error::other)?;
            let url: tauri::Url = services.settings_url.parse()?;
            let allowed_origin = url.origin();
            let log_path = services.data_dir.join("desktop.log");
            let window = WebviewWindowBuilder::new(app, "settings", WebviewUrl::External(url))
                .title("VoiceFlow")
                .inner_size(1180.0, 820.0)
                .min_inner_size(860.0, 600.0)
                .data_directory(services.data_dir.join("desktop-webview2"))
                .on_navigation(move |url| url.origin() == allowed_origin)
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .on_page_load(move |_, payload| {
                    if payload.event() == tauri::webview::PageLoadEvent::Finished {
                        let _ =
                            std::fs::write(&log_path, "Tauri settings page loaded successfully.\n");
                    }
                })
                .build()?;
            let close_window = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = close_window.hide();
                }
            });
            let open = MenuItem::with_id(
                app,
                "settings",
                "Open VoiceFlow / 打开设置",
                true,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit VoiceFlow / 退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .ok_or("Application icon missing")?
                        .clone(),
                )
                .tooltip("VoiceFlow — close window to keep dictation running")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => show_settings(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_settings(tray.app_handle());
                    }
                })
                .build(app)?;
            app.manage(Mutex::new(services));
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for _ in 0..if smoke { 8 } else { usize::MAX } {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let state = handle.state::<Mutex<Services>>();
                    let failure = state.lock().unwrap().failure();
                    if let Some(error) = failure {
                        show_error(&error);
                        handle.exit(1);
                        return;
                    }
                }
                if smoke {
                    handle.exit(0);
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<Mutex<Services>>() {
                    state.lock().unwrap().stop();
                }
            }
        }),
        Err(error) => show_error(&format!(
            "VoiceFlow could not start / 无法启动：\n{error}\n\nKeep the complete portable folder together.\nIf WebView2 is missing, install Microsoft Edge WebView2 Evergreen Runtime through your IT team:\nhttps://developer.microsoft.com/microsoft-edge/webview2/\n\nLogs: %LOCALAPPDATA%\\VoiceFlow Speech Input"
        )),
    }
}
