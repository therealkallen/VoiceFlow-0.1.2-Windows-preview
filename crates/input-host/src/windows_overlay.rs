use crate::app_paths::resolve_repo_root;
use crate::host::OverlayPublishSummary;
use serde::Serialize;
use shared_protocol::{
    FailedSessionSummary, OverlayStatus, SessionState, SessionSummary, SystemLanguage,
};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::platform::windows::{
    EventLoopBuilderExtWindows, WindowBuilderExtWindows, WindowExtWindows,
};
use tao::window::{Window, WindowBuilder};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW, HWND_TOPMOST, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};
use wry::http::{Request, Response, header::CONTENT_TYPE};
use wry::{PageLoadEvent, WebContext, WebView, WebViewBuilder};

const OVERLAY_URL: &str = "voiceflow://overlay/index.html";
const OVERLAY_WIDTH_LOGICAL: f64 = 244.0;
const OVERLAY_HEIGHT_LOGICAL: f64 = 68.0;
const OVERLAY_BOTTOM_MARGIN_LOGICAL: f64 = 48.0;

#[derive(Debug)]
pub struct WindowsOverlayAdapter {
    locale: Mutex<&'static str>,
    window: Mutex<Option<OverlayWindowHandle>>,
}

#[derive(Debug)]
struct OverlayWindowHandle {
    proxy: EventLoopProxy<OverlayWindowEvent>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Debug)]
struct OverlayWindowReady {
    proxy: EventLoopProxy<OverlayWindowEvent>,
    window_create_ms: u64,
    webview_create_ms: u64,
}

struct OverlayEnsureResult {
    proxy: EventLoopProxy<OverlayWindowEvent>,
    window_create_ms: u64,
    webview_create_ms: u64,
}

#[derive(Clone, Debug)]
enum OverlayWindowEvent {
    Runtime {
        json: String,
        raw_state: &'static str,
        should_show: bool,
    },
    Ipc(String),
    PageLoaded,
    Shutdown,
}

#[derive(Debug, Serialize)]
struct OverlayRuntimeView {
    generated_at_epoch_ms: u128,
    locale: &'static str,
    latest_status: Option<OverlayStatus>,
    last_session_summary: Option<OverlaySessionOutcome>,
    last_failure_summary: Option<OverlayFailureOutcome>,
}

#[derive(Debug, Serialize)]
struct OverlaySessionOutcome {
    session_id: u64,
    session_kind: shared_protocol::SessionKind,
    final_state: SessionState,
}

#[derive(Debug, Serialize)]
struct OverlayFailureOutcome {
    session_id: u64,
    session_kind: shared_protocol::SessionKind,
    failure_phase: shared_protocol::SessionFailurePhase,
    message: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OverlayBounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl WindowsOverlayAdapter {
    pub fn new(system_language: &SystemLanguage) -> Self {
        Self {
            locale: Mutex::new(overlay_locale(system_language)),
            window: Mutex::new(None),
        }
    }

    pub fn publish_status(&self, status: &OverlayStatus) -> OverlayPublishSummary {
        let runtime = OverlayRuntimeView {
            generated_at_epoch_ms: epoch_ms(),
            locale: self.current_locale(),
            latest_status: Some(status.clone()),
            last_session_summary: None,
            last_failure_summary: None,
        };
        self.publish_runtime(
            session_state_name(&status.state),
            should_show_state(&status.state),
            runtime,
        )
    }

    pub fn publish_summary(&self, summary: &SessionSummary) {
        let runtime = OverlayRuntimeView {
            generated_at_epoch_ms: epoch_ms(),
            locale: self.current_locale(),
            latest_status: Some(OverlayStatus {
                session_id: summary.session_id,
                session_kind: summary.session_kind.clone(),
                state: summary.final_state.clone(),
                detail: String::new(),
            }),
            last_session_summary: Some(OverlaySessionOutcome {
                session_id: summary.session_id,
                session_kind: summary.session_kind.clone(),
                final_state: summary.final_state.clone(),
            }),
            last_failure_summary: None,
        };
        let _ = self.publish_runtime(
            session_state_name(&summary.final_state),
            should_show_state(&summary.final_state),
            runtime,
        );
    }

    pub fn publish_failure(&self, failure: &FailedSessionSummary) {
        let runtime = OverlayRuntimeView {
            generated_at_epoch_ms: epoch_ms(),
            locale: self.current_locale(),
            latest_status: Some(OverlayStatus {
                session_id: failure.session_id,
                session_kind: failure.session_kind.clone(),
                state: SessionState::Failed,
                detail: String::new(),
            }),
            last_session_summary: None,
            last_failure_summary: Some(OverlayFailureOutcome {
                session_id: failure.session_id,
                session_kind: failure.session_kind.clone(),
                failure_phase: failure.failure_phase.clone(),
                message: failure.message.clone(),
            }),
        };
        let _ = self.publish_runtime("Failed", true, runtime);
    }

    pub fn update_settings(&self, settings: &shared_protocol::RuntimeSettings) {
        if let Ok(mut locale) = self.locale.lock() {
            *locale = overlay_locale(&settings.system_language);
        }
    }

    fn current_locale(&self) -> &'static str {
        self.locale.lock().map(|locale| *locale).unwrap_or("en")
    }

    fn publish_runtime(
        &self,
        raw_state: &'static str,
        should_show: bool,
        runtime: OverlayRuntimeView,
    ) -> OverlayPublishSummary {
        let json = match serde_json::to_string(&runtime) {
            Ok(json) => json,
            Err(error) => {
                eprintln!(
                    "[voiceflow-overlay] overlay_raw_state={raw_state} overlay_window_visible=false error=encode_runtime:{error}"
                );
                return OverlayPublishSummary::default();
            }
        };

        match self.ensure_window() {
            Ok(result) => {
                if let Err(error) = result.proxy.send_event(OverlayWindowEvent::Runtime {
                    json,
                    raw_state,
                    should_show,
                }) {
                    eprintln!(
                        "[voiceflow-overlay] overlay_raw_state={raw_state} overlay_window_visible=false error=send_event:{error}"
                    );
                }
                OverlayPublishSummary {
                    window_create_ms: result.window_create_ms,
                    webview_create_ms: result.webview_create_ms,
                }
            }
            Err(error) => {
                eprintln!(
                    "[voiceflow-overlay] overlay_raw_state={raw_state} overlay_window_visible=false error={error}"
                );
                OverlayPublishSummary::default()
            }
        }
    }

    fn ensure_window(&self) -> Result<OverlayEnsureResult, String> {
        let mut handle = self
            .window
            .lock()
            .expect("web overlay mutex should not be poisoned");
        if let Some(window) = handle.as_ref() {
            return Ok(OverlayEnsureResult {
                proxy: window.proxy.clone(),
                window_create_ms: 0,
                webview_create_ms: 0,
            });
        }

        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("voiceflow-overlay-webview".to_string())
            .spawn(move || run_overlay_thread(ready_tx))
            .map_err(|error| format!("failed to spawn overlay webview thread: {error}"))?;
        let ready = match ready_rx.recv() {
            Ok(Ok(ready)) => ready,
            Ok(Err(error)) => {
                let _ = thread.join();
                return Err(error);
            }
            Err(error) => {
                let _ = thread.join();
                return Err(format!(
                    "overlay webview thread did not initialize: {error}"
                ));
            }
        };

        let result = OverlayEnsureResult {
            proxy: ready.proxy.clone(),
            window_create_ms: ready.window_create_ms,
            webview_create_ms: ready.webview_create_ms,
        };
        *handle = Some(OverlayWindowHandle {
            proxy: ready.proxy.clone(),
            thread: Some(thread),
        });
        Ok(result)
    }
}

impl Drop for WindowsOverlayAdapter {
    fn drop(&mut self) {
        let handle = self
            .window
            .get_mut()
            .expect("web overlay mutex should not be poisoned")
            .take();
        if let Some(mut handle) = handle {
            let _ = handle.proxy.send_event(OverlayWindowEvent::Shutdown);
            if let Some(thread) = handle.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

fn run_overlay_thread(ready_tx: mpsc::Sender<Result<OverlayWindowReady, String>>) {
    let mut event_loop_builder = EventLoopBuilder::<OverlayWindowEvent>::with_user_event();
    event_loop_builder.with_any_thread(true);
    event_loop_builder.with_dpi_aware(true);
    let mut event_loop = event_loop_builder.build();
    let proxy = event_loop.create_proxy();
    let repo_root = match resolve_repo_root() {
        Ok(root) => root,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };

    let window_create_started_at = std::time::Instant::now();
    eprintln!("[voiceflow-overlay] overlay_create_phase=window_create_started");
    let window = match build_window(&event_loop) {
        Ok(window) => window,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    let window_create_ms = elapsed_ms(window_create_started_at);
    eprintln!(
        "[voiceflow-overlay] overlay_create_phase=window_create_finished overlay_window_create_ms={window_create_ms} overlay_window_visible=false"
    );
    let hwnd = window.hwnd() as HWND;
    configure_overlay_window(hwnd);

    let webview_create_started_at = std::time::Instant::now();
    eprintln!("[voiceflow-overlay] overlay_create_phase=webview_create_started");
    let data_path = match crate::app_paths::overlay_runtime_script_path() {
        Ok(path) => path.with_file_name("webview2"),
        Err(error) => { let _ = ready_tx.send(Err(error)); return; }
    };
    let mut web_context = WebContext::new(Some(data_path));
    let webview = match build_webview(&window, repo_root, proxy.clone(), &mut web_context) {
        Ok(webview) => webview,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    let webview_create_ms = elapsed_ms(webview_create_started_at);
    eprintln!(
        "[voiceflow-overlay] overlay_create_phase=webview_create_finished overlay_webview_create_ms={webview_create_ms} overlay_window_visible=false"
    );
    hide_overlay_window(&window, &webview);
    if ready_tx
        .send(Ok(OverlayWindowReady {
            proxy,
            window_create_ms,
            webview_create_ms,
        }))
        .is_err()
    {
        return;
    }

    let mut latest_runtime: Option<String> = None;
    let mut latest_raw_state = "none";

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(OverlayWindowEvent::Runtime {
                json,
                raw_state,
                should_show,
            }) => {
                latest_runtime = Some(json);
                latest_raw_state = raw_state;
                if let Some(runtime_json) = latest_runtime.as_deref() {
                    inject_runtime(&webview, runtime_json);
                }
                if should_show {
                    let bounds = position_overlay_window(&window);
                    let _ = webview.set_visible(true);
                    eprintln!(
                        "[voiceflow-overlay] overlay_raw_state={raw_state} overlay_renderer=apps/overlay-ui overlay_window_visible=true overlay_window_bounds={},{},{},{}",
                        bounds.x, bounds.y, bounds.width, bounds.height
                    );
                } else {
                    hide_overlay_window(&window, &webview);
                    eprintln!(
                        "[voiceflow-overlay] overlay_raw_state={raw_state} overlay_renderer=apps/overlay-ui overlay_window_visible=false overlay_window_bounds=hidden"
                    );
                }
            }
            Event::UserEvent(OverlayWindowEvent::PageLoaded) => {
                if let Some(runtime_json) = latest_runtime.as_deref() {
                    inject_runtime(&webview, runtime_json);
                }
            }
            Event::UserEvent(OverlayWindowEvent::Ipc(message)) => {
                if message.contains("\"type\":\"overlay-hidden\"") {
                    hide_overlay_window(&window, &webview);
                    eprintln!(
                        "[voiceflow-overlay] overlay_raw_state={latest_raw_state} overlay_renderer=apps/overlay-ui overlay_window_visible=false overlay_window_bounds=hidden"
                    );
                }
            }
            Event::UserEvent(OverlayWindowEvent::Shutdown) => {
                hide_overlay_window(&window, &webview);
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

fn build_window(
    event_loop: &tao::event_loop::EventLoop<OverlayWindowEvent>,
) -> Result<Window, String> {
    WindowBuilder::new()
        .with_title("VoiceFlow Overlay")
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_visible(false)
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(
            OVERLAY_WIDTH_LOGICAL,
            OVERLAY_HEIGHT_LOGICAL,
        ))
        .with_skip_taskbar(true)
        .with_undecorated_shadow(false)
        .with_drag_and_drop(false)
        .build(event_loop)
        .map_err(|error| format!("failed to create overlay window: {error}"))
}

fn build_webview(
    window: &Window,
    repo_root: PathBuf,
    proxy: EventLoopProxy<OverlayWindowEvent>,
    web_context: &mut WebContext,
) -> Result<WebView, String> {
    WebViewBuilder::new_with_web_context(web_context)
        .with_transparent(true)
        .with_focused(false)
        .with_initialization_script(
            r#"
window.__VOICEFLOW_NATIVE_OVERLAY__ = true;
window.__VOICEFLOW_OVERLAY_NATIVE_SET_RUNTIME__ = function(runtime) {
  window.__VOICEFLOW_OVERLAY_RUNTIME__ = runtime;
  if (window.__VOICEFLOW_OVERLAY_SET_RUNTIME__) {
    window.__VOICEFLOW_OVERLAY_SET_RUNTIME__(runtime);
  } else {
    window.__VOICEFLOW_OVERLAY_PENDING_RUNTIME__ = runtime;
  }
};
"#,
        )
        .with_custom_protocol("voiceflow".into(), move |_webview_id, request| {
            match overlay_protocol_response(&repo_root, request) {
                Ok(response) => response.map(Into::into),
                Err(error) => Response::builder()
                    .status(500)
                    .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                    .body(error.into_bytes())
                    .expect("static overlay error response should build")
                    .map(Into::into),
            }
        })
        .with_on_page_load_handler({
            let proxy = proxy.clone();
            move |event, _url| {
                if matches!(event, PageLoadEvent::Finished) {
                    let _ = proxy.send_event(OverlayWindowEvent::PageLoaded);
                }
            }
        })
        .with_ipc_handler({
            let proxy = proxy.clone();
            move |request: Request<String>| {
                let _ = proxy.send_event(OverlayWindowEvent::Ipc(request.body().clone()));
            }
        })
        .with_url(OVERLAY_URL)
        .build(window)
        .map_err(|error| format!("failed to create overlay webview: {error}"))
}

fn overlay_protocol_response(
    repo_root: &Path,
    request: Request<Vec<u8>>,
) -> Result<Response<Vec<u8>>, String> {
    let path = request.uri().path();
    let relative = if path == "/" || path.is_empty() {
        PathBuf::from("index.html")
    } else {
        PathBuf::from(path.trim_start_matches('/'))
    };
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Response::builder()
            .status(403)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(b"forbidden".to_vec())
            .map_err(|error| format!("failed to build forbidden overlay response: {error}"));
    }

    let full_path = if relative == Path::new("src/runtime-state.js") {
        crate::app_paths::overlay_runtime_script_path()?
    } else {
        repo_root.join("apps").join("overlay-ui").join(&relative)
    };
    let content = std::fs::read(&full_path).map_err(|error| {
        format!(
            "failed to read overlay asset {}: {error}",
            full_path.display()
        )
    })?;
    Response::builder()
        .status(200)
        .header(CONTENT_TYPE, content_type_for_path(&relative))
        .body(content)
        .map_err(|error| format!("failed to build overlay asset response: {error}"))
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

fn inject_runtime(webview: &WebView, runtime_json: &str) {
    let script = format!(
        "window.__VOICEFLOW_OVERLAY_NATIVE_SET_RUNTIME__ && window.__VOICEFLOW_OVERLAY_NATIVE_SET_RUNTIME__({runtime_json});"
    );
    if let Err(error) = webview.evaluate_script(&script) {
        eprintln!("[voiceflow-overlay] overlay_state_delivery=failed error={error}");
    }
}

fn configure_overlay_window(hwnd: HWND) {
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let next_style =
            ex_style | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style as isize);
        ShowWindow(hwnd, SW_HIDE);
    }
}

fn position_overlay_window(window: &Window) -> OverlayBounds {
    let hwnd = window.hwnd() as HWND;
    let scale = active_monitor_scale(hwnd);
    let work_area = active_monitor_work_area(hwnd);
    let bounds = overlay_bounds_for_work_area(work_area, scale);
    unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
        );
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    bounds
}

fn hide_overlay_window(window: &Window, webview: &WebView) {
    let _ = webview.set_visible(false);
    unsafe {
        ShowWindow(window.hwnd() as HWND, SW_HIDE);
    }
}

fn active_monitor_work_area(hwnd: HWND) -> WorkArea {
    let foreground = unsafe { GetForegroundWindow() };
    let target = if foreground.is_null() {
        hwnd
    } else {
        foreground
    };
    let monitor = unsafe { MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        },
        rcWork: RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        },
        dwFlags: 0,
    };
    if !monitor.is_null() {
        unsafe {
            GetMonitorInfoW(monitor, &mut info);
        }
    }
    WorkArea {
        left: info.rcWork.left,
        top: info.rcWork.top,
        right: info.rcWork.right,
        bottom: info.rcWork.bottom,
    }
}

fn active_monitor_scale(hwnd: HWND) -> f64 {
    let foreground = unsafe { GetForegroundWindow() };
    let target = if foreground.is_null() {
        hwnd
    } else {
        foreground
    };
    let monitor = unsafe { MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST) };
    if !monitor.is_null() {
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        let result =
            unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        if result >= 0 && dpi_x > 0 {
            return dpi_x as f64 / 96.0;
        }
    }

    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 }
}

fn overlay_bounds_for_work_area(work_area: WorkArea, scale: f64) -> OverlayBounds {
    let width = scale_logical_px(OVERLAY_WIDTH_LOGICAL, scale);
    let height = scale_logical_px(OVERLAY_HEIGHT_LOGICAL, scale);
    let bottom_margin = scale_logical_px(OVERLAY_BOTTOM_MARGIN_LOGICAL, scale);
    let work_width = work_area.right - work_area.left;
    let x = work_area.left + ((work_width - width) / 2);
    let y = work_area.bottom - height - bottom_margin;
    OverlayBounds {
        x,
        y,
        width,
        height,
    }
}

fn scale_logical_px(value: f64, scale: f64) -> i32 {
    (value * scale).round().max(1.0) as i32
}

fn should_show_state(state: &SessionState) -> bool {
    !matches!(state, SessionState::Idle | SessionState::Cancelled)
}

fn overlay_locale(system_language: &SystemLanguage) -> &'static str {
    match system_language {
        SystemLanguage::Chinese => "zh",
        SystemLanguage::English => "en",
    }
}

fn session_state_name(state: &SessionState) -> &'static str {
    match state {
        SessionState::Idle => "Idle",
        SessionState::Arming => "Arming",
        SessionState::Recording => "Recording",
        SessionState::Recognizing => "Recognizing",
        SessionState::ModeRouting => "ModeRouting",
        SessionState::Executing => "Executing",
        SessionState::ReadyToCommit => "ReadyToCommit",
        SessionState::Committing => "Committing",
        SessionState::Committed => "Committed",
        SessionState::Cancelled => "Cancelled",
        SessionState::Failed => "Failed",
    }
}

fn epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn elapsed_ms(started_at: std::time::Instant) -> u64 {
    started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_state_visibility_keeps_outcomes_visible_for_ui_timers() {
        assert!(!should_show_state(&SessionState::Idle));
        assert!(should_show_state(&SessionState::Arming));
        assert!(should_show_state(&SessionState::Recording));
        assert!(should_show_state(&SessionState::Recognizing));
        assert!(should_show_state(&SessionState::ModeRouting));
        assert!(should_show_state(&SessionState::Executing));
        assert!(should_show_state(&SessionState::ReadyToCommit));
        assert!(should_show_state(&SessionState::Committing));
        assert!(should_show_state(&SessionState::Committed));
        assert!(should_show_state(&SessionState::Failed));
        assert!(!should_show_state(&SessionState::Cancelled));
    }

    #[test]
    fn overlay_locale_uses_effective_system_language() {
        assert_eq!(overlay_locale(&SystemLanguage::English), "en");
        assert_eq!(overlay_locale(&SystemLanguage::Chinese), "zh");
    }

    #[test]
    fn overlay_adapter_uses_reloaded_language_for_the_next_payload() {
        let adapter = WindowsOverlayAdapter::new(&SystemLanguage::English);
        assert_eq!(adapter.current_locale(), "en");
        let mut settings = shared_protocol::RuntimeSettings::default();
        settings.system_language = SystemLanguage::Chinese;
        adapter.update_settings(&settings);
        assert_eq!(adapter.current_locale(), "zh");
    }

    #[test]
    fn native_runtime_payload_includes_locale() {
        let runtime = OverlayRuntimeView {
            generated_at_epoch_ms: 1,
            locale: overlay_locale(&SystemLanguage::English),
            latest_status: None,
            last_session_summary: None,
            last_failure_summary: None,
        };

        let encoded = serde_json::to_string(&runtime).expect("runtime payload should serialize");
        assert!(encoded.contains(r#""locale":"en""#));
    }

    #[test]
    fn overlay_bounds_bottom_center_with_negative_monitor_coordinates() {
        let bounds = overlay_bounds_for_work_area(
            WorkArea {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1040,
            },
            1.0,
        );

        assert_eq!(bounds.width, 244);
        assert_eq!(bounds.height, 68);
        assert_eq!(bounds.x, -1082);
        assert_eq!(bounds.y, 924);
    }

    #[test]
    fn overlay_bounds_scale_for_high_dpi() {
        let bounds = overlay_bounds_for_work_area(
            WorkArea {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2080,
            },
            2.0,
        );

        assert_eq!(bounds.width, 488);
        assert_eq!(bounds.height, 136);
        assert_eq!(bounds.x, 1676);
        assert_eq!(bounds.y, 1848);
    }

    #[test]
    fn protocol_rejects_parent_directory_paths() {
        let request = Request::builder()
            .uri("voiceflow://overlay/../settings-ui/index.html")
            .body(Vec::new())
            .expect("request should build");
        let response = overlay_protocol_response(Path::new("."), request)
            .expect("forbidden response should build");

        assert_eq!(response.status(), 403);
    }
}
