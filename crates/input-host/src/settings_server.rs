use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::app_paths::resolve_repo_root;
use crate::history_ledger::clear_history_for_settings_path;
use crate::provider_key_store::{
    ProviderCredentialError, ProviderCredentialStore, resolve_provider_config_with_credentials,
};
use crate::settings_bridge::{
    LastSettingsApplySummary, build_last_settings_apply_summary,
    encode_settings_runtime_state_json_with_credentials,
    write_settings_runtime_state_with_credentials,
};
use crate::settings_store::{
    LoadedRuntimeSettings, RuntimeSettingsUpdate, load_runtime_settings, update_runtime_settings,
};
use shared_protocol::ProviderPreset;
use speech_engine::{ProviderConnectionTestResult, ResolvedProvider};

const SETTINGS_PORT_SEARCH_LIMIT: u16 = 12;
const SETTINGS_CONTROL_TOKEN_HEADER: &str = "X-VoiceFlow-Settings-Token";
const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 64 * 1024;

pub struct SettingsControlBinding {
    listener: TcpListener,
    port: u16,
}

impl SettingsControlBinding {
    pub fn port(&self) -> u16 {
        self.port
    }

    #[cfg(test)]
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, String> {
        self.listener
            .local_addr()
            .map_err(|error| format!("failed to read settings control listener address: {error}"))
    }
}

pub fn bind_settings_control(preferred_port: u16) -> Result<SettingsControlBinding, String> {
    for offset in 0..=SETTINGS_PORT_SEARCH_LIMIT {
        let candidate_port = preferred_port.saturating_add(offset);
        match TcpListener::bind(("127.0.0.1", candidate_port)) {
            Ok(listener) => {
                listener.set_nonblocking(true).map_err(|error| {
                    format!("failed to configure settings control listener: {error}")
                })?;
                return Ok(SettingsControlBinding {
                    listener,
                    port: candidate_port,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => continue,
            Err(error) => {
                return Err(format!(
                    "failed to bind settings control server on 127.0.0.1:{candidate_port}: {error}"
                ));
            }
        }
    }

    Err(format!(
        "failed to bind settings control server on 127.0.0.1:{preferred_port} or the next {} port(s) because they were already in use",
        SETTINGS_PORT_SEARCH_LIMIT
    ))
}

pub fn serve_settings_control(
    binding: SettingsControlBinding,
    provider_credential_store: Arc<dyn ProviderCredentialStore>,
) -> Result<(), String> {
    let SettingsControlBinding { listener, port } = binding;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("failed to configure settings control listener: {error}"))?;

    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_requested_for_handler = Arc::clone(&stop_requested);
    let last_settings_apply = Arc::new(Mutex::new(None::<LastSettingsApplySummary>));
    let context = SettingsControlContext {
        port,
        control_token: generate_control_token()?,
        settings_ui_root: settings_ui_root()?,
        last_settings_apply,
        provider_credential_store,
        provider_connection_tester: Arc::new(LiveProviderConnectionTester),
    };
    ctrlc::set_handler(move || {
        stop_requested_for_handler.store(true, Ordering::SeqCst);
    })
    .map_err(|error| format!("failed to install Ctrl+C handler for settings control: {error}"))?;

    println!(
        "Settings control server is running at http://127.0.0.1:{port}. Press Ctrl+C to exit."
    );
    println!("Open Settings at http://127.0.0.1:{port}/ to use the same-origin control bridge.");

    while !stop_requested.load(Ordering::SeqCst) && !crate::desktop_lifecycle::stop_requested() {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(error) = handle_stream(stream, &context) {
                    eprintln!("settings control request failed: {error}");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                return Err(format!(
                    "settings control listener failed while accepting connections: {error}"
                ));
            }
        }
    }

    println!("Settings control server received Ctrl+C and is shutting down cleanly.");
    Ok(())
}

struct SettingsControlContext {
    port: u16,
    control_token: String,
    settings_ui_root: PathBuf,
    last_settings_apply: Arc<Mutex<Option<LastSettingsApplySummary>>>,
    provider_credential_store: Arc<dyn ProviderCredentialStore>,
    provider_connection_tester: Arc<dyn ProviderConnectionTester>,
}

trait ProviderConnectionTester: Send + Sync {
    fn test(&self, resolved: &ResolvedProvider) -> ProviderConnectionTestResult;
}

#[derive(Debug)]
struct LiveProviderConnectionTester;

impl ProviderConnectionTester for LiveProviderConnectionTester {
    fn test(&self, resolved: &ResolvedProvider) -> ProviderConnectionTestResult {
        speech_engine::test_provider_connection(resolved)
    }
}

fn handle_stream(mut stream: TcpStream, context: &SettingsControlContext) -> Result<(), String> {
    stream.set_nonblocking(false).map_err(|error| {
        format!("failed to configure accepted settings control stream: {error}")
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to set settings control read timeout: {error}"))?;
    let request = match read_http_request(&mut stream) {
        Ok(request) => request,
        Err(error) if is_ignorable_read_error(&error) => return Ok(()),
        Err(error) => return Err(error),
    };
    let response = handle_request(request, context)?;

    write_http_response(&mut stream, response)
}

fn handle_request(
    request: HttpRequest,
    context: &SettingsControlContext,
) -> Result<HttpResponse, String> {
    let response = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => HttpResponse::json(200, r#"{"ok":true}"#.to_string()),
        ("GET", "/") | ("GET", "/index.html") => serve_settings_index(context),
        ("GET", "/favicon.ico") => HttpResponse::no_content(),
        ("GET", "/src/runtime-state.js") => {
            let loaded = load_runtime_settings()?;
            let encoded = encode_settings_runtime_state_json_with_credentials(
                &loaded, Some(&format!("http://127.0.0.1:{}", context.port)), None,
                context.last_settings_apply.lock().map_err(|_| "settings state lock failed")?.as_ref(),
                context.provider_credential_store.as_ref(),
            )?;
            HttpResponse {
                status_code: 200, reason_phrase: "OK",
                body: format!("window.__VOICEFLOW_SETTINGS_RUNTIME__ = {encoded};").into_bytes(),
                content_type: "text/javascript; charset=utf-8", cache_control_no_store: true,
            }
        }
        ("GET", "/runtime-state") => {
            let loaded = load_runtime_settings()?;
            let last_apply = context
                .last_settings_apply
                .lock()
                .map_err(|_| "settings apply state mutex should not be poisoned".to_string())?
                .clone();
            let encoded = encode_settings_runtime_state_json_with_credentials(
                &loaded,
                Some(&format!("http://127.0.0.1:{}", context.port)),
                None,
                last_apply.as_ref(),
                context.provider_credential_store.as_ref(),
            )?;
            HttpResponse::json(200, encoded)
        }
        ("POST", "/update-settings") => {
            if let Some(response) = reject_invalid_mutation_token(&request, context) {
                return Ok(response);
            }
            let update = serde_json::from_slice::<RuntimeSettingsUpdate>(&request.body).map_err(
                |error| format!("failed to parse settings update request body as JSON: {error}"),
            )?;
            let apply_summary = build_last_settings_apply_summary(&update, "Rust host bridge");
            let loaded = update_runtime_settings(update)?;
            {
                let mut last_apply = context
                    .last_settings_apply
                    .lock()
                    .map_err(|_| "settings apply state mutex should not be poisoned".to_string())?;
                *last_apply = Some(apply_summary.clone());
            }
            let settings_control_url = format!("http://127.0.0.1:{}", context.port);
            write_settings_runtime_state_with_credentials(
                &loaded,
                Some(&settings_control_url),
                None,
                Some(&apply_summary),
                context.provider_credential_store.as_ref(),
            )?;
            let encoded = encode_settings_runtime_state_json_with_credentials(
                &loaded,
                Some(&settings_control_url),
                None,
                Some(&apply_summary),
                context.provider_credential_store.as_ref(),
            )?;
            HttpResponse::json(200, encoded)
        }
        ("POST", "/prompt-settings") => {
            // Reading prompt contents is also privileged; never include them in public snapshots.
            if let Some(response) = reject_invalid_mutation_token(&request, context) {
                return Ok(response);
            }
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct PromptRequest {
                slot_id: String,
                action: String,
                text: Option<String>,
            }
            let result = serde_json::from_slice::<PromptRequest>(&request.body)
                .map_err(|_| "Invalid prompt request.".to_string())
                .and_then(|input| match input.action.as_str() {
                    "read" => speech_engine::prompt_store::read(&input.slot_id),
                    "save" => input
                        .text
                        .as_deref()
                        .ok_or_else(|| "Prompt text is required.".to_string())
                        .and_then(|text| {
                            speech_engine::prompt_store::save(&input.slot_id, Some(text))
                        }),
                    "reset" => speech_engine::prompt_store::save(&input.slot_id, None),
                    _ => Err("Unknown prompt action.".into()),
                });
            let mut response = match result {
                Ok(state) => HttpResponse::json(
                    200,
                    serde_json::to_string(&state)
                        .map_err(|_| "Could not encode prompt response.")?,
                ),
                Err(error) => {
                    HttpResponse::json(400, serde_json::json!({"error": error}).to_string())
                }
            };
            response.cache_control_no_store = true;
            response
        }
        ("PUT", "/provider-credential") => {
            if let Some(response) = reject_invalid_mutation_token(&request, context) {
                return Ok(response);
            }
            let mutation = serde_json::from_slice::<ProviderCredentialPutRequest>(&request.body)
                .map_err(|error| {
                    format!("failed to parse provider credential request body as JSON: {error}")
                })?;
            let api_key = mutation.api_key.trim();
            if api_key.is_empty() {
                return Ok(HttpResponse::json(
                    400,
                    r#"{"ok":false,"error":"provider API key cannot be empty"}"#.to_string(),
                ));
            }
            if api_key.len() > MAX_PROVIDER_API_KEY_BYTES {
                return Ok(HttpResponse::json(
                    413,
                    r#"{"ok":false,"error":"provider API key is too large"}"#.to_string(),
                ));
            }
            if let Err(error) = context
                .provider_credential_store
                .set(&mutation.provider, api_key)
            {
                return Ok(provider_credential_error_response(error));
            }
            if let Err(error) =
                verify_provider_credential_read_after_write(context, &mutation.provider)
            {
                return Ok(provider_credential_error_response(error));
            }
            provider_credential_runtime_state_response(context)?
        }
        ("DELETE", "/provider-credential") => {
            if let Some(response) = reject_invalid_mutation_token(&request, context) {
                return Ok(response);
            }
            let mutation = serde_json::from_slice::<ProviderCredentialDeleteRequest>(&request.body)
                .map_err(|error| {
                    format!("failed to parse provider credential request body as JSON: {error}")
                })?;
            if let Err(error) = context.provider_credential_store.delete(&mutation.provider) {
                return Ok(provider_credential_error_response(error));
            }
            provider_credential_runtime_state_response(context)?
        }
        ("POST", "/test-provider") => {
            if let Some(response) = reject_invalid_mutation_token(&request, context) {
                return Ok(response);
            }
            if !request.body.is_empty() {
                return Ok(HttpResponse::json(
                    400,
                    r#"{"ok":false,"error":"test provider request body is not supported"}"#
                        .to_string(),
                ));
            }
            let loaded = load_runtime_settings()?;
            let resolved = resolve_provider_config_with_credentials(
                &loaded.settings,
                context.provider_credential_store.as_ref(),
            );
            let result = context.provider_connection_tester.test(&resolved);
            HttpResponse::json(
                200,
                serde_json::to_string(&result)
                    .map_err(|error| format!("failed to encode provider test result: {error}"))?,
            )
        }
        ("POST", "/clear-history") => {
            if let Some(response) = reject_invalid_mutation_token(&request, context) {
                return Ok(response);
            }
            let loaded = load_runtime_settings()?;
            let last_apply = context
                .last_settings_apply
                .lock()
                .map_err(|_| "settings apply state mutex should not be poisoned".to_string())?
                .clone();
            let settings_control_url = format!("http://127.0.0.1:{}", context.port);
            let encoded = clear_history_runtime_state_json(
                &loaded,
                &settings_control_url,
                last_apply.as_ref(),
                true,
                context.provider_credential_store.as_ref(),
            )?;
            HttpResponse::json(200, encoded)
        }
        ("GET", _) => serve_settings_asset(context, &request.path),
        _ => route_not_found(),
    };

    Ok(response)
}

fn clear_history_runtime_state_json(
    loaded: &LoadedRuntimeSettings,
    settings_control_url: &str,
    last_settings_apply: Option<&LastSettingsApplySummary>,
    write_snapshot: bool,
    provider_credential_store: &dyn ProviderCredentialStore,
) -> Result<String, String> {
    clear_history_for_settings_path(&loaded.path)?;
    if write_snapshot {
        write_settings_runtime_state_with_credentials(
            loaded,
            Some(settings_control_url),
            None,
            last_settings_apply,
            provider_credential_store,
        )?;
    }
    encode_settings_runtime_state_json_with_credentials(
        loaded,
        Some(settings_control_url),
        None,
        last_settings_apply,
        provider_credential_store,
    )
}

const MAX_PROVIDER_API_KEY_BYTES: usize = 8 * 1024;

#[derive(serde::Deserialize)]
struct ProviderCredentialPutRequest {
    provider: ProviderPreset,
    api_key: String,
}

impl std::fmt::Debug for ProviderCredentialPutRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderCredentialPutRequest")
            .field("provider", &self.provider)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, serde::Deserialize)]
struct ProviderCredentialDeleteRequest {
    provider: ProviderPreset,
}

fn provider_credential_runtime_state_response(
    context: &SettingsControlContext,
) -> Result<HttpResponse, String> {
    let loaded = load_runtime_settings()?;
    let last_apply = context
        .last_settings_apply
        .lock()
        .map_err(|_| "settings apply state mutex should not be poisoned".to_string())?
        .clone();
    let settings_control_url = format!("http://127.0.0.1:{}", context.port);
    let encoded = encode_settings_runtime_state_json_with_credentials(
        &loaded,
        Some(&settings_control_url),
        None,
        last_apply.as_ref(),
        context.provider_credential_store.as_ref(),
    )?;
    Ok(HttpResponse::json(200, encoded))
}

fn verify_provider_credential_read_after_write(
    context: &SettingsControlContext,
    provider: &ProviderPreset,
) -> Result<(), ProviderCredentialError> {
    match context.provider_credential_store.get(provider) {
        speech_engine::ProviderCredentialLookup::Found(value) if !value.trim().is_empty() => Ok(()),
        speech_engine::ProviderCredentialLookup::Found(_)
        | speech_engine::ProviderCredentialLookup::Missing
        | speech_engine::ProviderCredentialLookup::StoreError => {
            Err(ProviderCredentialError::read_after_write_failed())
        }
    }
}

fn provider_credential_error_response(error: ProviderCredentialError) -> HttpResponse {
    let status = match error.kind() {
        crate::provider_key_store::ProviderCredentialErrorKind::Rejected => 400,
        crate::provider_key_store::ProviderCredentialErrorKind::Unavailable
        | crate::provider_key_store::ProviderCredentialErrorKind::EntryCreationFailed
        | crate::provider_key_store::ProviderCredentialErrorKind::WriteFailed
        | crate::provider_key_store::ProviderCredentialErrorKind::DeleteFailed
        | crate::provider_key_store::ProviderCredentialErrorKind::ReadAfterWriteFailed => 503,
    };
    eprintln!("provider_credential_error={}", error.safe_category());
    HttpResponse::json(
        status,
        format!(
            r#"{{"ok":false,"error":"{}"}}"#,
            json_escape(error.safe_message())
        ),
    )
}

struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

struct HttpResponse {
    status_code: u16,
    reason_phrase: &'static str,
    body: Vec<u8>,
    content_type: &'static str,
    cache_control_no_store: bool,
}

impl HttpResponse {
    fn json(status_code: u16, body: String) -> Self {
        Self {
            status_code,
            reason_phrase: reason_phrase_for_status(status_code),
            body: body.into_bytes(),
            content_type: "application/json; charset=utf-8",
            cache_control_no_store: false,
        }
    }

    fn html(status_code: u16, body: String) -> Self {
        Self {
            status_code,
            reason_phrase: reason_phrase_for_status(status_code),
            body: body.into_bytes(),
            content_type: "text/html; charset=utf-8",
            cache_control_no_store: false,
        }
    }

    fn no_content() -> Self {
        Self {
            status_code: 204,
            reason_phrase: "No Content",
            body: Vec::new(),
            content_type: "text/plain; charset=utf-8",
            cache_control_no_store: false,
        }
    }

    fn with_no_store(mut self) -> Self {
        self.cache_control_no_store = true;
        self
    }
}

fn route_not_found() -> HttpResponse {
    HttpResponse::json(
        404,
        r#"{"ok":false,"error":"settings control route not found"}"#.to_string(),
    )
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 1024];
    let mut header_length = None;
    let mut content_length = 0_usize;

    loop {
        let bytes_read = stream
            .read(&mut temp)
            .map_err(|error| format!("failed to read settings control request: {error}"))?;
        if bytes_read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..bytes_read]);

        if header_length.is_none() {
            if buffer.len() > MAX_HTTP_HEADER_BYTES {
                return Err(format!(
                    "settings control request header exceeded {MAX_HTTP_HEADER_BYTES} bytes"
                ));
            }
            if let Some(end) = find_header_end(&buffer) {
                header_length = Some(end);
                let header_text = String::from_utf8_lossy(&buffer[..end]);
                content_length = parse_content_length(&header_text)?;
                if content_length > MAX_HTTP_BODY_BYTES {
                    return Err(format!(
                        "settings control request body exceeded {MAX_HTTP_BODY_BYTES} bytes"
                    ));
                }
            }
        }

        if let Some(header_end) = header_length {
            if buffer.len() >= header_end + content_length {
                break;
            }
        }
    }

    let header_end = header_length.ok_or_else(|| {
        "settings control request did not contain a complete HTTP header".to_string()
    })?;
    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "settings control request was missing the request line".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "settings control request was missing the HTTP method".to_string())?;
    let path = request_parts
        .next()
        .ok_or_else(|| "settings control request was missing the path".to_string())?;
    let headers = parse_headers(&header_text);
    let body = buffer
        .get(header_end..header_end + content_length)
        .ok_or_else(|| "settings control request body was incomplete".to_string())?
        .to_vec();

    Ok(HttpRequest {
        method: method.to_string(),
        path: normalize_request_path(path),
        headers,
        body,
    })
}

fn normalize_request_path(path: &str) -> String {
    path.split('?').next().unwrap_or(path).to_string()
}

fn is_ignorable_read_error(error: &str) -> bool {
    error.contains("os error 10060")
        || error.contains("os error 10054")
        || error.contains("os error 10053")
        || error.contains("timed out")
}

fn parse_content_length(header_text: &str) -> Result<usize, String> {
    for line in header_text.lines() {
        if let Some(value) = line.strip_prefix("Content-Length:") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|error| format!("invalid Content-Length header: {error}"));
        }
        if let Some(value) = line.strip_prefix("content-length:") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|error| format!("invalid content-length header: {error}"));
        }
    }

    Ok(0)
}

fn parse_headers(header_text: &str) -> Vec<(String, String)> {
    header_text
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn header_value<'a>(request: &'a HttpRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

fn reject_invalid_mutation_token(
    request: &HttpRequest,
    context: &SettingsControlContext,
) -> Option<HttpResponse> {
    let Some(presented) = header_value(request, SETTINGS_CONTROL_TOKEN_HEADER) else {
        return Some(invalid_mutation_token_response());
    };
    if constant_time_eq(presented.as_bytes(), context.control_token.as_bytes()) {
        None
    } else {
        Some(invalid_mutation_token_response())
    }
}

fn invalid_mutation_token_response() -> HttpResponse {
    HttpResponse::json(
        403,
        r#"{"ok":false,"error":"missing or invalid settings control token"}"#.to_string(),
    )
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0_u8;
    for (left_byte, right_byte) in left.iter().zip(right.iter()) {
        diff |= left_byte ^ right_byte;
    }
    diff == 0
}

fn settings_ui_root() -> Result<PathBuf, String> {
    Ok(resolve_repo_root()?.join("apps").join("settings-ui"))
}

fn serve_settings_index(context: &SettingsControlContext) -> HttpResponse {
    let index_path = context.settings_ui_root.join("index.html");
    match fs::read_to_string(&index_path) {
        Ok(contents) => {
            let bootstrap = settings_bootstrap_script(&context.control_token);
            HttpResponse::html(200, inject_settings_bootstrap(&contents, &bootstrap))
                .with_no_store()
        }
        Err(error) => HttpResponse::json(
            500,
            format!(
                r#"{{"ok":false,"error":"failed to read Settings UI index: {}"}}"#,
                json_escape(&error.to_string())
            ),
        ),
    }
}

fn settings_bootstrap_script(control_token: &str) -> String {
    let payload = serde_json::json!({ "controlToken": control_token });
    format!("<script>window.__VOICEFLOW_SETTINGS_BRIDGE__ = {payload};</script>")
}

fn inject_settings_bootstrap(index_html: &str, bootstrap_script: &str) -> String {
    let main_script = r#"<script src="./src/main.js"></script>"#;
    if let Some(position) = index_html.find(main_script) {
        let mut output = String::with_capacity(index_html.len() + bootstrap_script.len() + 12);
        output.push_str(&index_html[..position]);
        output.push_str("    ");
        output.push_str(bootstrap_script);
        output.push('\n');
        output.push_str(&index_html[position..]);
        return output;
    }

    if let Some(position) = index_html.rfind("</body>") {
        let mut output = String::with_capacity(index_html.len() + bootstrap_script.len() + 12);
        output.push_str(&index_html[..position]);
        output.push_str("    ");
        output.push_str(bootstrap_script);
        output.push('\n');
        output.push_str(&index_html[position..]);
        return output;
    }

    format!("{index_html}\n{bootstrap_script}\n")
}

fn serve_settings_asset(context: &SettingsControlContext, request_path: &str) -> HttpResponse {
    let Some(asset_path) = settings_asset_path(&context.settings_ui_root, request_path) else {
        return route_not_found();
    };
    match fs::read(&asset_path) {
        Ok(body) => HttpResponse {
            status_code: 200,
            reason_phrase: reason_phrase_for_status(200),
            body,
            content_type: content_type_for_path(&asset_path),
            cache_control_no_store: false,
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => route_not_found(),
        Err(error) => HttpResponse::json(
            500,
            format!(
                r#"{{"ok":false,"error":"failed to read Settings UI asset: {}"}}"#,
                json_escape(&error.to_string())
            ),
        ),
    }
}

fn settings_asset_path(ui_root: &Path, request_path: &str) -> Option<PathBuf> {
    let relative = request_path.trim_start_matches('/');
    if relative.is_empty() || relative == "index.html" {
        return Some(ui_root.join("index.html"));
    }

    let mut path = PathBuf::from(ui_root);
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(segment) => path.push(segment),
            _ => return None,
        }
    }
    Some(path)
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("ico") => "image/x-icon",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn generate_control_token() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes)
        .map_err(|error| format!("failed to generate settings control token: {error}"))?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> Result<(), String> {
    let headers = http_response_headers(&response);
    stream
        .write_all(headers.as_bytes())
        .map_err(|error| format!("failed to write settings control response headers: {error}"))?;
    if !response.body.is_empty() {
        stream
            .write_all(&response.body)
            .map_err(|error| format!("failed to write settings control response body: {error}"))?;
    }
    stream
        .flush()
        .map_err(|error| format!("failed to flush settings control response: {error}"))
}

fn http_response_headers(response: &HttpResponse) -> String {
    let cache_headers = if response.cache_control_no_store {
        "Cache-Control: no-store\r\nPragma: no-cache\r\n"
    } else {
        ""
    };
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n",
        response.status_code,
        response.reason_phrase,
        response.content_type,
        response.body.len(),
        cache_headers
    )
}

fn reason_phrase_for_status(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_ledger::history_ledger_path_for_settings_path;
    use crate::provider_key_store::{
        InMemoryProviderCredentialStore, MissingProviderCredentialStore, ProviderCredentialStore,
    };
    use crate::settings_store::{LoadedRuntimeSettings, RuntimeSettingsSource};
    use shared_protocol::{HistoryRetention, ProviderPreset, ProviderSettings, RuntimeSettings};
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::net::TcpListener;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    static SETTINGS_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = env::var_os(key);
            unsafe {
                env::set_var(key, value);
            }
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = env::var_os(key);
            unsafe {
                env::remove_var(key);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(previous) = &self.previous {
                    env::set_var(self.key, previous);
                } else {
                    env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn provider_credential_put_request_debug_redacts_api_key() {
        let request = ProviderCredentialPutRequest {
            provider: ProviderPreset::Bailian,
            api_key: "sentinel-secret-key".to_string(),
        };

        let debug = format!("{request:?}");

        assert!(debug.contains("Bailian"));
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("sentinel-secret-key"));
    }

    struct SettingsPathEnvGuard {
        previous: Option<OsString>,
    }

    impl SettingsPathEnvGuard {
        fn set(path: &Path) -> Self {
            let previous = env::var_os("VOICEFLOW_SETTINGS_PATH");
            unsafe {
                env::set_var("VOICEFLOW_SETTINGS_PATH", path);
            }
            Self { previous }
        }
    }

    impl Drop for SettingsPathEnvGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(previous) = &self.previous {
                    env::set_var("VOICEFLOW_SETTINGS_PATH", previous);
                } else {
                    env::remove_var("VOICEFLOW_SETTINGS_PATH");
                }
            }
        }
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        env::temp_dir().join(format!("voiceflow-{label}-{unique_id}"))
    }

    fn write_test_settings_ui(temp_dir: &Path) -> PathBuf {
        let ui_root = temp_dir.join("settings-ui");
        fs::create_dir_all(ui_root.join("src")).expect("settings UI temp dir should be created");
        fs::write(
            ui_root.join("index.html"),
            r#"<!doctype html><html><body><script src="./src/runtime-state.js"></script><script src="./src/main.js"></script></body></html>"#,
        )
        .expect("index should write");
        fs::write(
            ui_root.join("src").join("main.js"),
            "console.log('settings');\n",
        )
        .expect("main script should write");
        fs::write(
            ui_root.join("src").join("runtime-state.js"),
            "window.__VOICEFLOW_SETTINGS_RUNTIME__ = {};\n",
        )
        .expect("runtime script should write");
        ui_root
    }

    fn test_context(ui_root: PathBuf, token: &str) -> SettingsControlContext {
        test_context_with_store(ui_root, token, Arc::new(MissingProviderCredentialStore))
    }

    fn test_context_with_store(
        ui_root: PathBuf,
        token: &str,
        provider_credential_store: Arc<dyn ProviderCredentialStore>,
    ) -> SettingsControlContext {
        test_context_with_store_and_tester(
            ui_root,
            token,
            provider_credential_store,
            Arc::new(MockProviderConnectionTester::default()),
        )
    }

    fn test_context_with_store_and_tester(
        ui_root: PathBuf,
        token: &str,
        provider_credential_store: Arc<dyn ProviderCredentialStore>,
        provider_connection_tester: Arc<dyn ProviderConnectionTester>,
    ) -> SettingsControlContext {
        SettingsControlContext {
            port: 8765,
            control_token: token.to_string(),
            settings_ui_root: ui_root,
            last_settings_apply: Arc::new(Mutex::new(None)),
            provider_credential_store,
            provider_connection_tester,
        }
    }

    #[derive(Default)]
    struct MockProviderConnectionTester {
        calls: Mutex<Vec<(String, String, String)>>,
    }

    impl MockProviderConnectionTester {
        fn calls(&self) -> Vec<(String, String, String)> {
            self.calls
                .lock()
                .expect("calls lock should not be poisoned")
                .clone()
        }
    }

    impl ProviderConnectionTester for MockProviderConnectionTester {
        fn test(&self, resolved: &ResolvedProvider) -> ProviderConnectionTestResult {
            let provider_preset =
                speech_engine::provider_preset_label(&resolved.metadata.preset).to_string();
            let key_source =
                speech_engine::provider_key_source_label(resolved.metadata.key_source).to_string();
            self.calls
                .lock()
                .expect("calls lock should not be poisoned")
                .push((
                    provider_preset.clone(),
                    resolved.metadata.model_code.clone(),
                    key_source.clone(),
                ));
            ProviderConnectionTestResult {
                success: resolved.config.is_some(),
                status: if resolved.config.is_some() {
                    speech_engine::ProviderConnectionTestStatus::Success
                } else if !resolved.metadata.key_present {
                    speech_engine::ProviderConnectionTestStatus::MissingKey
                } else {
                    speech_engine::ProviderConnectionTestStatus::InvalidConfiguration
                },
                provider_preset,
                provider_label: resolved.metadata.display_label.clone(),
                model_code: resolved.metadata.model_code.clone(),
                effective_key_source: key_source,
                latency_ms: Some(3),
                http_status: resolved.config.as_ref().map(|_| 200),
                message: "mock provider test result".to_string(),
            }
        }
    }

    fn get_request(path: &str) -> HttpRequest {
        HttpRequest {
            method: "GET".to_string(),
            path: path.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn post_request(path: &str, token: Option<&str>, body: &[u8]) -> HttpRequest {
        json_request("POST", path, token, body)
    }

    fn json_request(method: &str, path: &str, token: Option<&str>, body: &[u8]) -> HttpRequest {
        let mut headers = Vec::new();
        if let Some(token) = token {
            headers.push((SETTINGS_CONTROL_TOKEN_HEADER.to_string(), token.to_string()));
        }
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            headers,
            body: body.to_vec(),
        }
    }

    #[test]
    fn parses_content_length_case_insensitively() {
        let header_text = "POST /update-settings HTTP/1.1\r\ncontent-length: 17\r\n\r\n";

        let content_length =
            parse_content_length(header_text).expect("content-length should parse");

        assert_eq!(content_length, 17);
    }

    #[test]
    fn finds_http_header_end() {
        let request = b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\nbody";

        let end = find_header_end(request).expect("header end should be found");

        assert_eq!(end, 41);
    }

    #[test]
    fn incomplete_http_body_returns_error_instead_of_panicking() {
        for body in ["", "{}"] {
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
            let (mut server, _) = listener.accept().unwrap();
            client.write_all(format!(
                "POST /update-settings HTTP/1.1\r\nContent-Length: 20\r\n\r\n{body}"
            ).as_bytes()).unwrap();
            client.shutdown(std::net::Shutdown::Write).unwrap();
            let error = read_http_request(&mut server).err().expect("truncated body must fail");
            assert!(error.contains("body was incomplete"));
        }
    }

    #[test]
    fn complete_http_body_is_preserved() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client.write_all(b"POST /update-settings HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}").unwrap();
        let request = read_http_request(&mut server).unwrap();
        assert_eq!(request.body, b"{}");
    }

    #[test]
    fn falls_forward_to_a_nearby_free_port_when_preferred_port_is_busy() {
        let occupied = TcpListener::bind(("127.0.0.1", 0)).expect("ephemeral port should bind");
        let occupied_port = occupied
            .local_addr()
            .expect("local addr should be available")
            .port();

        let binding = bind_settings_control(occupied_port).expect("binding should fall forward");

        assert_ne!(binding.port(), occupied_port);
    }

    #[test]
    fn binds_settings_control_to_loopback_only() {
        let binding = bind_settings_control(0).expect("settings control should bind");
        let address = binding.local_addr().expect("local address should resolve");

        assert_eq!(address.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn served_settings_ui_injects_same_origin_control_token() {
        let temp_dir = unique_temp_dir("settings-ui-bootstrap");
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context(ui_root, "bootstrap-token");

        let response = handle_request(get_request("/"), &context).expect("request should handle");
        assert_eq!(response.status_code, 200);
        let headers = http_response_headers(&response);
        assert!(headers.contains("Cache-Control: no-store"));
        assert!(headers.contains("Pragma: no-cache"));
        let body = String::from_utf8(response.body).expect("body should be utf8");
        assert!(body.contains("window.__VOICEFLOW_SETTINGS_BRIDGE__"));
        assert!(body.contains(r#""controlToken":"bootstrap-token""#));
        assert!(
            body.find("__VOICEFLOW_SETTINGS_BRIDGE__").unwrap()
                < body.find("./src/main.js").unwrap()
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn valid_token_permits_update_settings() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let temp_dir = unique_temp_dir("settings-update-token");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context(ui_root, "valid-token");

        let response = handle_request(
            post_request(
                "/update-settings",
                Some("valid-token"),
                br#"{"silence_gate_level":3}"#,
            ),
            &context,
        )
        .expect("request should handle");
        let body: serde_json::Value =
            serde_json::from_slice(&response.body).expect("response should decode");

        assert_eq!(response.status_code, 200);
        assert_eq!(body["silence_gate_level"], 3);
        let saved: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&settings_path).expect("settings should read"),
        )
        .expect("settings should decode");
        assert_eq!(saved["silence_gate_level"], 3);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn missing_token_returns_403_for_update_settings() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let temp_dir = unique_temp_dir("settings-update-missing-token");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context(ui_root, "valid-token");

        let response = handle_request(
            post_request("/update-settings", None, br#"{"silence_gate_level":3}"#),
            &context,
        )
        .expect("request should handle");

        assert_eq!(response.status_code, 403);
        assert!(!settings_path.exists());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn invalid_token_returns_403_for_update_settings() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let temp_dir = unique_temp_dir("settings-update-invalid-token");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context(ui_root, "valid-token");

        let response = handle_request(
            post_request(
                "/update-settings",
                Some("wrong-token"),
                br#"{"silence_gate_level":3}"#,
            ),
            &context,
        )
        .expect("request should handle");

        assert_eq!(response.status_code, 403);
        assert!(!settings_path.exists());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn prompt_editor_requires_token_and_saves_resets_separate_slots() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().unwrap();
        let dir = unique_temp_dir("prompt-editor");
        fs::create_dir_all(&dir).unwrap();
        let settings = dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings);
        let context = test_context(write_test_settings_ui(&dir), "valid-token");
        let read = br#"{"slot_id":"selected_text_edit","action":"read"}"#;
        assert_eq!(
            handle_request(post_request("/prompt-settings", None, read), &context)
                .unwrap()
                .status_code,
            403
        );
        assert_eq!(
            handle_request(
                post_request("/prompt-settings", Some("wrong"), read),
                &context
            )
            .unwrap()
            .status_code,
            403
        );
        let call = |body: &[u8]| {
            handle_request(
                post_request("/prompt-settings", Some("valid-token"), body),
                &context,
            )
            .unwrap()
        };
        let initial = call(read);
        assert_eq!(initial.status_code, 200);
        let initial: serde_json::Value = serde_json::from_slice(&initial.body).unwrap();
        assert!(!initial["builtin_text"].as_str().unwrap().is_empty());
        let saved =
            call(br#"{"slot_id":"selected_text_edit","action":"save","text":"Keep all facts."}"#);
        assert_eq!(saved.status_code, 200);
        assert_eq!(
            speech_engine::prompt_store::read("selected_text_edit")
                .unwrap()
                .text,
            "Keep all facts."
        );
        assert_eq!(
            call(br#"{"slot_id":"../escape","action":"save","text":"bad"}"#).status_code,
            400
        );
        assert_eq!(
            call(br#"{"slot_id":"selected_text_edit","action":"save","text":" "}"#).status_code,
            400
        );
        let reset = call(br#"{"slot_id":"selected_text_edit","action":"reset"}"#);
        assert_eq!(reset.status_code, 200);
        let reset: serde_json::Value = serde_json::from_slice(&reset.body).unwrap();
        assert_eq!(reset["text"], initial["builtin_text"]);
        assert_eq!(reset["source"], "builtin");
        assert!(!settings.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn provider_credential_route_requires_control_token() {
        let temp_dir = unique_temp_dir("provider-credential-token");
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context_with_store(
            ui_root,
            "valid-token",
            Arc::new(InMemoryProviderCredentialStore::new()),
        );

        let response = handle_request(
            json_request(
                "PUT",
                "/provider-credential",
                None,
                br#"{"provider":"bailian","api_key":"secret"}"#,
            ),
            &context,
        )
        .expect("request should handle");

        assert_eq!(response.status_code, 403);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn provider_credential_save_and_delete_return_safe_runtime_state() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let _provider_key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let temp_dir = unique_temp_dir("provider-credential-save-delete");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&RuntimeSettings {
                provider: shared_protocol::ProviderSettings {
                    preset: ProviderPreset::VolcengineArk,
                    ..shared_protocol::ProviderSettings::default()
                },
                ..RuntimeSettings::default()
            })
            .expect("settings should encode"),
        )
        .expect("settings should write");
        let ui_root = write_test_settings_ui(&temp_dir);
        let store = InMemoryProviderCredentialStore::new();
        let context = test_context_with_store(ui_root, "valid-token", Arc::new(store.clone()));

        let save_response = handle_request(
            json_request(
                "PUT",
                "/provider-credential",
                Some("valid-token"),
                br#"{"provider":"volcengine_ark","api_key":" stored-secret-sentinel "}"#,
            ),
            &context,
        )
        .expect("request should handle");
        assert_eq!(save_response.status_code, 200);
        let save_body = String::from_utf8(save_response.body).expect("body should be utf8");
        assert!(!save_body.contains("stored-secret-sentinel"));
        let save_json: serde_json::Value =
            serde_json::from_str(&save_body).expect("response should decode");
        assert_eq!(
            save_json["provider"]["stored_credential_present"],
            serde_json::json!(true)
        );
        assert_eq!(
            save_json["provider"]["credential_store_status"],
            serde_json::json!("available")
        );
        assert_eq!(
            save_json["provider"]["effective_key_present"],
            serde_json::json!(true)
        );
        assert_eq!(
            save_json["provider"]["effective_key_source"],
            serde_json::json!("credential_store")
        );
        assert_eq!(
            save_json["provider"]["credential_status_by_preset"]["volcengine_ark"]["stored_credential_present"],
            serde_json::json!(true)
        );
        assert_eq!(
            save_json["provider"]["credential_status_by_preset"]["volcengine_ark"]["credential_store_status"],
            serde_json::json!("available")
        );
        assert_eq!(
            store.get(&ProviderPreset::VolcengineArk),
            speech_engine::ProviderCredentialLookup::Found("stored-secret-sentinel".to_string())
        );

        let delete_response = handle_request(
            json_request(
                "DELETE",
                "/provider-credential",
                Some("valid-token"),
                br#"{"provider":"volcengine_ark"}"#,
            ),
            &context,
        )
        .expect("request should handle");
        assert_eq!(delete_response.status_code, 200);
        let delete_body = String::from_utf8(delete_response.body).expect("body should be utf8");
        assert!(!delete_body.contains("stored-secret-sentinel"));
        let delete_json: serde_json::Value =
            serde_json::from_str(&delete_body).expect("response should decode");
        assert_eq!(
            delete_json["provider"]["credential_store_status"],
            serde_json::json!("missing")
        );
        assert_eq!(
            delete_json["provider"]["stored_credential_present"],
            serde_json::json!(false)
        );
        assert_eq!(
            delete_json["provider"]["effective_key_present"],
            serde_json::json!(false)
        );
        assert_eq!(
            delete_json["provider"]["effective_key_source"],
            serde_json::json!("missing")
        );
        assert_eq!(
            delete_json["provider"]["credential_status_by_preset"]["volcengine_ark"]["stored_credential_present"],
            serde_json::json!(false)
        );
        assert_eq!(
            store.get(&ProviderPreset::VolcengineArk),
            speech_engine::ProviderCredentialLookup::Missing
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn provider_credential_store_errors_are_reported_without_secret_material() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let temp_dir = unique_temp_dir("provider-credential-store-error");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&RuntimeSettings {
                provider: shared_protocol::ProviderSettings {
                    preset: ProviderPreset::VolcengineArk,
                    ..shared_protocol::ProviderSettings::default()
                },
                ..RuntimeSettings::default()
            })
            .expect("settings should encode"),
        )
        .expect("settings should write");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        let ui_root = write_test_settings_ui(&temp_dir);
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::VolcengineArk, "stored-secret-sentinel")
            .expect("mock credential should store");
        store.set_read_error(true);
        let context = test_context_with_store(ui_root, "valid-token", Arc::new(store));

        let response =
            handle_request(get_request("/runtime-state"), &context).expect("request should handle");
        assert_eq!(response.status_code, 200);
        let body = String::from_utf8(response.body).expect("body should be utf8");
        assert!(!body.contains("stored-secret-sentinel"));
        let json: serde_json::Value = serde_json::from_str(&body).expect("response should decode");
        assert_eq!(
            json["provider"]["credential_store_status"],
            serde_json::json!("error")
        );
        assert_eq!(
            json["provider"]["key_source"],
            serde_json::json!("store_error_last_known_good")
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn provider_credential_delete_keeps_env_key_effective_without_stored_key() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let _provider_key_guard =
            EnvVarGuard::set("VOICEFLOW_PROVIDER_API_KEY", "env-secret-sentinel");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let temp_dir = unique_temp_dir("provider-credential-delete-env");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&RuntimeSettings {
                provider: shared_protocol::ProviderSettings {
                    preset: ProviderPreset::VolcengineArk,
                    ..shared_protocol::ProviderSettings::default()
                },
                ..RuntimeSettings::default()
            })
            .expect("settings should encode"),
        )
        .expect("settings should write");
        let ui_root = write_test_settings_ui(&temp_dir);
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::VolcengineArk, "stored-secret-sentinel")
            .expect("test store should accept key");
        let context = test_context_with_store(ui_root, "valid-token", Arc::new(store.clone()));

        let delete_response = handle_request(
            json_request(
                "DELETE",
                "/provider-credential",
                Some("valid-token"),
                br#"{"provider":"volcengine_ark"}"#,
            ),
            &context,
        )
        .expect("request should handle");

        assert_eq!(delete_response.status_code, 200);
        let body = String::from_utf8(delete_response.body).expect("body should be utf8");
        assert!(!body.contains("stored-secret-sentinel"));
        assert!(!body.contains("env-secret-sentinel"));
        let json: serde_json::Value = serde_json::from_str(&body).expect("response should decode");
        assert_eq!(json["provider"]["stored_credential_present"], false);
        assert_eq!(json["provider"]["credential_store_status"], "missing");
        assert_eq!(json["provider"]["effective_key_present"], true);
        assert_eq!(json["provider"]["effective_key_source"], "provider_env");
        assert_eq!(
            store.get(&ProviderPreset::VolcengineArk),
            speech_engine::ProviderCredentialLookup::Missing
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn provider_credential_save_with_env_key_reports_selected_stored_credential() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let _provider_key_guard =
            EnvVarGuard::set("VOICEFLOW_PROVIDER_API_KEY", "env-secret-sentinel");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let temp_dir = unique_temp_dir("provider-credential-save-env");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&RuntimeSettings::default())
                .expect("settings should encode"),
        )
        .expect("settings should write");
        let ui_root = write_test_settings_ui(&temp_dir);
        let store = InMemoryProviderCredentialStore::new();
        let context = test_context_with_store(ui_root, "valid-token", Arc::new(store.clone()));

        let save_response = handle_request(
            json_request(
                "PUT",
                "/provider-credential",
                Some("valid-token"),
                br#"{"provider":"bailian","api_key":" stored-secret-sentinel "}"#,
            ),
            &context,
        )
        .expect("request should handle");

        assert_eq!(save_response.status_code, 200);
        let body = String::from_utf8(save_response.body).expect("body should be utf8");
        assert!(!body.contains("stored-secret-sentinel"));
        assert!(!body.contains("env-secret-sentinel"));
        let json: serde_json::Value = serde_json::from_str(&body).expect("response should decode");
        assert_eq!(json["provider"]["effective_key_present"], true);
        assert_eq!(json["provider"]["effective_key_source"], "provider_env");
        assert_eq!(
            json["provider"]["credential_status_by_preset"]["bailian"]["stored_credential_present"],
            true
        );
        assert_eq!(
            json["provider"]["credential_status_by_preset"]["bailian"]["credential_store_status"],
            "available"
        );
        assert_eq!(
            store.get(&ProviderPreset::Bailian),
            speech_engine::ProviderCredentialLookup::Found("stored-secret-sentinel".to_string())
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn provider_credential_save_readback_failure_returns_sanitized_error() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let _provider_key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let temp_dir = unique_temp_dir("provider-credential-readback-failure");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&RuntimeSettings::default())
                .expect("settings should encode"),
        )
        .expect("settings should write");
        let ui_root = write_test_settings_ui(&temp_dir);
        let store = InMemoryProviderCredentialStore::new();
        store.set_read_error(true);
        let context = test_context_with_store(ui_root, "valid-token", Arc::new(store));

        let save_response = handle_request(
            json_request(
                "PUT",
                "/provider-credential",
                Some("valid-token"),
                br#"{"provider":"bailian","api_key":" stored-secret-sentinel "}"#,
            ),
            &context,
        )
        .expect("request should handle");

        assert_eq!(save_response.status_code, 503);
        let body = String::from_utf8(save_response.body).expect("body should be utf8");
        assert!(body.contains("credential store did not confirm"));
        assert!(!body.contains("stored-secret-sentinel"));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn provider_credential_write_error_is_sanitized() {
        let temp_dir = unique_temp_dir("provider-credential-write-error");
        let ui_root = write_test_settings_ui(&temp_dir);
        let store = InMemoryProviderCredentialStore::new();
        store.set_write_error(true);
        let context = test_context_with_store(ui_root, "valid-token", Arc::new(store));

        let response = handle_request(
            json_request(
                "PUT",
                "/provider-credential",
                Some("valid-token"),
                br#"{"provider":"bailian","api_key":"stored-secret-sentinel"}"#,
            ),
            &context,
        )
        .expect("request should handle");

        assert_eq!(response.status_code, 503);
        let body = String::from_utf8(response.body).expect("body should be utf8");
        assert!(!body.contains("stored-secret-sentinel"));
        assert!(body.contains("credential store write failed"));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_provider_route_requires_control_token() {
        let temp_dir = unique_temp_dir("test-provider-token");
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context_with_store(
            ui_root,
            "valid-token",
            Arc::new(InMemoryProviderCredentialStore::new()),
        );

        let response = handle_request(post_request("/test-provider", None, b""), &context)
            .expect("request should handle");

        assert_eq!(response.status_code, 403);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_provider_route_rejects_arbitrary_request_body() {
        let temp_dir = unique_temp_dir("test-provider-body");
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context_with_store(
            ui_root,
            "valid-token",
            Arc::new(InMemoryProviderCredentialStore::new()),
        );

        let response = handle_request(
            post_request(
                "/test-provider",
                Some("valid-token"),
                br#"{"base_url":"https://attacker.example/v1","api_key":"sentinel-secret"}"#,
            ),
            &context,
        )
        .expect("request should handle");

        assert_eq!(response.status_code, 400);
        let body = String::from_utf8(response.body).expect("body should be utf8");
        assert!(!body.contains("attacker.example"));
        assert!(!body.contains("sentinel-secret"));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_provider_route_uses_effective_settings_and_credentials_safely() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let _provider_key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let temp_dir = unique_temp_dir("test-provider-success");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&RuntimeSettings {
                provider: ProviderSettings {
                    preset: ProviderPreset::VolcengineArk,
                    active_model: Some("ark-test-model".to_string()),
                    ..ProviderSettings::default()
                },
                ..RuntimeSettings::default()
            })
            .expect("settings should encode"),
        )
        .expect("settings should write");
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::VolcengineArk, "stored-secret-sentinel")
            .expect("test store should accept key");
        let tester = Arc::new(MockProviderConnectionTester::default());
        let context = test_context_with_store_and_tester(
            write_test_settings_ui(&temp_dir),
            "valid-token",
            Arc::new(store),
            tester.clone(),
        );

        let response = handle_request(
            post_request("/test-provider", Some("valid-token"), b""),
            &context,
        )
        .expect("request should handle");

        assert_eq!(response.status_code, 200);
        let body = String::from_utf8(response.body).expect("body should be utf8");
        assert!(!body.contains("stored-secret-sentinel"));
        let json: serde_json::Value = serde_json::from_str(&body).expect("body should decode");
        assert_eq!(json["success"], true);
        assert_eq!(json["status"], "success");
        assert_eq!(json["provider_preset"], "volcengine_ark");
        assert_eq!(json["model_code"], "ark-test-model");
        assert_eq!(json["effective_key_source"], "credential_store");
        assert_eq!(
            tester.calls(),
            vec![(
                "volcengine_ark".to_string(),
                "ark-test-model".to_string(),
                "credential_store".to_string()
            )]
        );
        let ledger_path = history_ledger_path_for_settings_path(&settings_path);
        assert!(!ledger_path.exists());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn valid_token_permits_clear_history() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let temp_dir = unique_temp_dir("settings-clear-history-token");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&RuntimeSettings::default())
                .expect("settings should encode"),
        )
        .expect("settings should write");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        let ledger_path = history_ledger_path_for_settings_path(&settings_path);
        fs::write(
            &ledger_path,
            serde_json::json!({
                "schema_version": 1,
                "updated_at_epoch_ms": 10,
                "applied_session_keys": ["report:session:1"],
                "entries": [{
                    "entry_id": "report:session:1",
                    "source_run_id": "report",
                    "session_id": 1,
                    "timestamp_epoch_ms": 10,
                    "mode": "Dictation",
                    "summary": "Dictation inserted",
                    "text": "stored text",
                    "redacted": false,
                    "word_count": 2,
                    "char_count": 11,
                    "refinement_profile_label": null,
                    "model_name": null,
                    "model_code": null,
                    "provider_attempted": null,
                    "provider_succeeded": null,
                    "deterministic_fallback_used": null,
                    "fallback_reason": null,
                    "selected_text_action": null,
                    "selected_text_action_label": null,
                    "selected_text_char_count": null,
                    "output_char_count": null
                }]
            })
            .to_string(),
        )
        .expect("ledger should write");
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context(ui_root, "valid-token");

        let response = handle_request(
            post_request("/clear-history", Some("valid-token"), b"{}"),
            &context,
        )
        .expect("request should handle");
        let body: serde_json::Value =
            serde_json::from_slice(&response.body).expect("response should decode");

        assert_eq!(response.status_code, 200);
        assert_eq!(
            body["history_summary"]["entries"].as_array().unwrap().len(),
            0
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn get_runtime_state_remains_readable_without_token() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let temp_dir = unique_temp_dir("settings-runtime-no-token");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        let ui_root = write_test_settings_ui(&temp_dir);
        let context = test_context(ui_root, "runtime-token");

        let response =
            handle_request(get_request("/runtime-state"), &context).expect("request should handle");
        let body: serde_json::Value =
            serde_json::from_slice(&response.body).expect("response should decode");

        assert_eq!(response.status_code, 200);
        assert_eq!(body["settings_control_url"], "http://127.0.0.1:8765");
        assert!(body.get("controlToken").is_none());
        assert!(
            !response
                .body
                .windows(b"runtime-token".len())
                .any(|window| window == b"runtime-token")
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn live_responses_do_not_return_wildcard_cors() {
        let response = HttpResponse::json(200, r#"{"ok":true}"#.to_string());
        let headers = http_response_headers(&response);

        assert!(!headers.contains("Access-Control-Allow-Origin"));
        assert!(!headers.contains("*"));
    }

    #[test]
    fn control_token_is_not_written_to_runtime_state_mirror() {
        let _settings_env_guard = SETTINGS_ENV_LOCK.lock().expect("settings env lock");
        let temp_dir = unique_temp_dir("settings-token-not-mirrored");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let _path_guard = SettingsPathEnvGuard::set(&settings_path);
        let ui_root = write_test_settings_ui(&temp_dir);
        let token = "sentinel-control-token-never-mirror";
        let context = test_context(ui_root, token);

        let response = handle_request(
            post_request(
                "/update-settings",
                Some(token),
                br#"{"silence_gate_level":4}"#,
            ),
            &context,
        )
        .expect("request should handle");
        assert_eq!(response.status_code, 200);
        assert!(
            !response
                .body
                .windows(token.len())
                .any(|window| window == token.as_bytes())
        );

        let settings_mirror = crate::app_paths::settings_runtime_script_path()
            .expect("settings runtime path should resolve");
        let mirror_contents =
            fs::read_to_string(settings_mirror).expect("settings mirror should read");
        assert!(!mirror_contents.contains(token));

        let overlay_mirror = crate::app_paths::overlay_runtime_script_path()
            .expect("overlay runtime path should resolve");
        let overlay_contents = fs::read_to_string(overlay_mirror).unwrap_or_default();
        assert!(!overlay_contents.contains(token));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn clear_history_response_preserves_settings_and_returns_empty_history_summary() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let temp_dir =
            std::env::temp_dir().join(format!("voiceflow-clear-history-server-{unique_id}"));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let ledger_path = history_ledger_path_for_settings_path(&settings_path);
        let ledger = serde_json::json!({
            "schema_version": 1,
            "updated_at_epoch_ms": 10,
            "applied_session_keys": ["report:session:1"],
            "entries": [
                {
                    "entry_id": "report:session:1",
                    "source_run_id": "report",
                    "session_id": 1,
                    "timestamp_epoch_ms": 10,
                    "mode": "Dictation",
                    "summary": "Dictation inserted",
                    "text": "stored text",
                    "redacted": false,
                    "word_count": 2,
                    "char_count": 11,
                    "refinement_profile_label": null,
                    "model_name": null,
                    "model_code": null,
                    "provider_attempted": null,
                    "provider_succeeded": null,
                    "deterministic_fallback_used": null,
                    "fallback_reason": null,
                    "selected_text_action": null,
                    "selected_text_action_label": null,
                    "selected_text_char_count": null,
                    "output_char_count": null
                }
            ]
        });
        fs::write(
            &ledger_path,
            serde_json::to_string_pretty(&ledger).expect("ledger should encode"),
        )
        .expect("ledger should write");
        let mut settings = RuntimeSettings::default();
        settings.history_retention = HistoryRetention::Last30Days;
        let loaded = LoadedRuntimeSettings {
            settings,
            path: settings_path,
            source: RuntimeSettingsSource::File,
            warnings: Vec::new(),
        };

        let encoded = clear_history_runtime_state_json(
            &loaded,
            "http://127.0.0.1:8765",
            None,
            false,
            &MissingProviderCredentialStore,
        )
        .expect("history clear response should encode");

        let response: serde_json::Value =
            serde_json::from_str(&encoded).expect("response should decode");
        assert_eq!(response["history_retention"], "last_30_days");
        assert_eq!(
            response["history_summary"]["entries"]
                .as_array()
                .expect("entries should be an array")
                .len(),
            0
        );
        let persisted: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&ledger_path).expect("ledger should read"))
                .expect("ledger should decode");
        assert_eq!(
            persisted["entries"]
                .as_array()
                .expect("entries should be an array")
                .len(),
            0
        );
        assert_eq!(
            persisted["applied_session_keys"]
                .as_array()
                .expect("keys should be an array")
                .len(),
            0
        );
        assert!(persisted["cleared_at_epoch_ms"].as_u64().is_some());
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn runtime_state_response_includes_prompt_override_status() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let temp_dir =
            std::env::temp_dir().join(format!("voiceflow-prompt-status-server-{unique_id}"));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: temp_dir.join("settings.json"),
            source: RuntimeSettingsSource::Defaults,
            warnings: Vec::new(),
        };

        let encoded = encode_settings_runtime_state_json_with_credentials(
            &loaded,
            Some("http://127.0.0.1:8765"),
            None,
            None,
            &MissingProviderCredentialStore,
        )
        .expect("runtime state should encode");
        let response: serde_json::Value =
            serde_json::from_str(&encoded).expect("response should decode");
        let prompt_overrides = response["prompt_overrides"]
            .as_array()
            .expect("prompt overrides should be an array");

        assert_eq!(prompt_overrides.len(), 4);
        assert!(prompt_overrides.iter().any(|status| {
            status["slot_id"] == "dictation_light_cleanup"
                && status["env_var"] == "VOICEFLOW_DICTATION_LIGHT_PROMPT_FILE"
                && status.get("configured_path").is_some()
        }));
        assert!(prompt_overrides.iter().any(|status| {
            status["slot_id"] == "dictation_structured_cleanup"
                && status["env_var"] == "VOICEFLOW_DICTATION_STRUCTURED_PROMPT_FILE"
                && status.get("configured_path").is_some()
        }));
        assert!(prompt_overrides.iter().any(|status| {
            status["slot_id"] == "selected_text_edit"
                && status["env_var"] == "VOICEFLOW_SELECTED_TEXT_PROMPT_FILE"
        }));
        assert!(prompt_overrides.iter().any(|status| {
            status["slot_id"] == "instructed_dictation"
                && status["env_var"] == "VOICEFLOW_INSTRUCTED_DICTATION_PROMPT_FILE"
        }));
        let _ = fs::remove_dir_all(temp_dir);
    }
}
