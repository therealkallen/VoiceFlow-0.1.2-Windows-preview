mod app_paths;
mod cpal_audio;
mod desktop_lifecycle;
mod diagnostics_profile;
mod history_ledger;
mod host;
mod insertion_text;
mod live_host_reports;
mod overlay_bridge;
mod provider_key_store;
mod selected_text_compatibility;
mod selected_text_validation;
mod settings_bridge;
mod settings_reload;
mod settings_server;
mod settings_store;
mod text_count;
mod usage_ledger;
#[cfg(windows)]
mod windows_feedback;
mod windows_foreground;
#[cfg(windows)]
mod windows_hotkeys;
#[cfg(windows)]
mod windows_insertion;
#[cfg(windows)]
mod windows_overlay;
#[cfg(windows)]
mod windows_shortcut;

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use cpal_audio::CpalAudioCaptureAdapter;
use diagnostics_profile::{
    ActionMixEntry, derive_commit_path_summary, derive_failure_guidance,
    derive_selected_text_action_mix, derive_verification_plan, derive_wake_phrase_action_mix,
    derive_workload_focus,
};
use history_ledger::append_live_host_history;
use host::{
    HostRuntime, NoopAudioCaptureAdapter, NoopAudioFeedbackAdapter, NoopShortcutAdapter,
    StdoutOverlayAdapter, StubTextInsertionAdapter,
};
use intent_edit_executor::{
    InstructedDictationExecutor, ProviderBackedSelectedTextExecutor,
    ProviderSelectedTextEditRequest, SelectedTextEditProvider, TemporarySelectedTextExecutor,
};
use live_host_reports::{
    ensure_live_host_report_dir, live_host_report_path_for_timestamp, prune_old_live_host_reports,
};
use overlay_bridge::MirroringOverlayAdapter;
use provider_key_store::{
    ProviderCredentialStore, production_provider_credential_store,
    resolve_provider_config_with_credentials,
};
use selected_text_compatibility::{
    SelectedTextCompatibilityIssue, derive_selected_text_compatibility_summary,
    parse_selected_text_failure,
};
use selected_text_validation::{
    SelectedTextProbeKind, SelectedTextProbeOutcome, SelectedTextValidationBaselineReset,
    SelectedTextValidationHistoryScope, SelectedTextValidationMatrixEntry,
    SelectedTextValidationPersistence, SelectedTextValidationRecord,
    SelectedTextValidationRecordInput, append_selected_text_validation_record,
    load_selected_text_validation_report, reset_selected_text_validation_baseline,
    selected_text_validation_archive_count,
};
use serde::Serialize;
use settings_bridge::{
    build_last_settings_apply_summary, write_settings_runtime_state,
    write_settings_runtime_state_with_credentials,
};
use settings_reload::{RuntimeSettingsReloader, SettingsReloadPoll};
use settings_server::{bind_settings_control, serve_settings_control};
use settings_store::{
    ProviderSettingsUpdate, RuntimeSettingsSource, RuntimeSettingsUpdate, load_runtime_settings,
    update_runtime_settings, write_default_settings_template,
};
use shared_protocol::{
    CommitTransport, DiagnosticEvent, DiagnosticsVerbosity, FailedSessionSummary, HistoryRetention,
    InstructedDictationProviderDiagnostics, InstructedDictationTransformRequest,
    InstructedDictationTransformResponse, ProviderPreset, RefinementModelProfile,
    RefinementQuality, RuntimeSettings, SessionSummary, ShortcutMode, SystemLanguage, UiStyle,
};
use speech_engine::{
    ChatCompletionTransport, ProviderKeySource, ResolvedProvider, SpeechEngine,
    UreqChatCompletionTransport, build_provider_instructed_dictation_request,
    build_provider_selected_text_edit_request, provider_key_source_label, provider_preset_label,
    resolve_provider_config,
};
use usage_ledger::{UsageFailureInput, append_live_host_usage};

#[cfg(windows)]
use windows_feedback::WindowsAudioFeedbackAdapter;
use windows_foreground::{ForegroundAppMetadata, capture_foreground_app_metadata};
#[cfg(windows)]
use windows_hotkeys::WindowsHotkeyAdapter;
#[cfg(windows)]
use windows_insertion::{
    ProcessIntegrityProbeResult, SendInputProbeResult, WindowsTextInsertionAdapter,
};

#[derive(Clone, Copy, Debug)]
struct ProcessStartupContext {
    process_started_at: Instant,
    env_load_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveHostStartupTimings {
    env_load_ms: u64,
    settings_load_ms: u64,
    settings_reloader_init_ms: u64,
    settings_runtime_state_init_ms: u64,
    overlay_adapter_init_ms: u64,
    insertion_adapter_init_ms: u64,
    host_runtime_init_ms: u64,
    hotkey_register_ms: u64,
    audio_prepare_ms: u64,
    audio_prepare_succeeded: bool,
    audio_backend_create_ms: u64,
    audio_device_discovery_ms: u64,
    speech_backend_prepare_ms: u64,
    speech_backend_prepare_succeeded: bool,
    speech_model_load_ms: Option<u32>,
    speech_model_warmup_ms: Option<u32>,
    speech_model_warmup_succeeded: Option<bool>,
    host_ready_ms: u64,
    total_process_to_ready_ms: u64,
}

fn main() {
    if let Err(error) = desktop_lifecycle::initialize() {
        eprintln!("{error}");
        std::process::exit(1);
    }
    let process_started_at = Instant::now();
    let env_load_started_at = Instant::now();
    let local_env_load_report = load_local_env_files();
    let startup_context = ProcessStartupContext {
        process_started_at,
        env_load_ms: elapsed_millis(env_load_started_at),
    };
    print_local_env_load_report(&local_env_load_report);

    let result = match parse_command(env::args().skip(1)) {
        Ok(Command::Demo) => run_demo(),
        Ok(Command::ListenOnce { capture_ms }) => run_listen_once(capture_ms),
        Ok(Command::ListenOnceLive { max_capture_ms }) => run_listen_once_live(max_capture_ms),
        Ok(Command::ServeLive {
            max_capture_ms,
            session_limit,
        }) => run_serve_live(max_capture_ms, session_limit, startup_context),
        Ok(Command::CaptureProbe { capture_ms }) => run_capture_probe(capture_ms),
        Ok(Command::TranscribeProbe { capture_ms }) => run_transcribe_probe(capture_ms),
        Ok(Command::PrintSettings) => run_print_settings(),
        Ok(Command::WriteDefaultSettings) => run_write_default_settings(),
        Ok(Command::UpdateSettings { update }) => run_update_settings(update),
        Ok(Command::ServeSettings { port }) => run_serve_settings(port),
        Ok(Command::InsertProbe { text }) => run_insert_probe(&text),
        Ok(Command::SendInputKeyProbe { delay_ms }) => run_sendinput_key_probe(delay_ms),
        Ok(Command::ClipboardOnlyProbe { text }) => run_clipboard_only_probe(&text),
        Ok(Command::CtrlVProbe { delay_ms }) => run_ctrl_v_probe(delay_ms),
        Ok(Command::EditProbe {
            instruction,
            app_label,
            probe_note,
        }) => run_edit_probe(&instruction, app_label.as_deref(), probe_note.as_deref()),
        Ok(Command::EditTranscribeProbe { capture_ms }) => run_edit_transcribe_probe(capture_ms),
        Ok(Command::IntentProbe { request }) => run_intent_probe(&request),
        Ok(Command::ReplaceSelectionProbe {
            text,
            app_label,
            probe_note,
        }) => run_replace_selection_probe(&text, app_label.as_deref(), probe_note.as_deref()),
        Ok(Command::PrintSelectedTextValidationReport { app_label }) => {
            run_print_selected_text_validation_report(app_label.as_deref())
        }
        Ok(Command::ResetSelectedTextValidationBaseline { label }) => {
            run_reset_selected_text_validation_baseline(label.as_deref())
        }
        Err(error) => Err(error),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

enum Command {
    Demo,
    ListenOnce {
        capture_ms: u64,
    },
    ListenOnceLive {
        max_capture_ms: Option<u64>,
    },
    ServeLive {
        max_capture_ms: Option<u64>,
        session_limit: Option<u64>,
    },
    CaptureProbe {
        capture_ms: u64,
    },
    TranscribeProbe {
        capture_ms: u64,
    },
    PrintSettings,
    WriteDefaultSettings,
    UpdateSettings {
        update: RuntimeSettingsUpdate,
    },
    ServeSettings {
        port: u16,
    },
    InsertProbe {
        text: String,
    },
    SendInputKeyProbe {
        delay_ms: u64,
    },
    ClipboardOnlyProbe {
        text: String,
    },
    CtrlVProbe {
        delay_ms: u64,
    },
    EditProbe {
        instruction: String,
        app_label: Option<String>,
        probe_note: Option<String>,
    },
    EditTranscribeProbe {
        capture_ms: u64,
    },
    IntentProbe {
        request: String,
    },
    ReplaceSelectionProbe {
        text: String,
        app_label: Option<String>,
        probe_note: Option<String>,
    },
    PrintSelectedTextValidationReport {
        app_label: Option<String>,
    },
    ResetSelectedTextValidationBaseline {
        label: Option<String>,
    },
}

type RuntimeSelectedTextExecutor = ProviderBackedSelectedTextExecutor<
    TemporarySelectedTextExecutor,
    DashScopeSelectedTextEditProvider,
>;

fn provider_resolution_can_use_last_known_good(resolved: &ResolvedProvider) -> bool {
    resolved.config.is_none() && resolved.metadata.key_source != ProviderKeySource::Missing
}

#[derive(Clone)]
struct DashScopeSelectedTextEditProvider {
    fallback_provider: ResolvedProvider,
    fallback_profile: RefinementModelProfile,
    last_known_good_provider: Arc<Mutex<Option<ResolvedProvider>>>,
    last_provider_diagnostic_context: Arc<Mutex<Option<(String, String)>>>,
    provider_credential_store: Arc<dyn ProviderCredentialStore>,
    transport: UreqChatCompletionTransport,
}

impl DashScopeSelectedTextEditProvider {
    fn from_settings(settings: &RuntimeSettings) -> Self {
        Self::from_settings_with_credentials(settings, production_provider_credential_store())
    }

    fn from_settings_with_credentials(
        settings: &RuntimeSettings,
        provider_credential_store: Arc<dyn ProviderCredentialStore>,
    ) -> Self {
        let fallback_provider = resolve_provider_config(settings);
        let fallback_profile = fallback_provider.profile.clone();
        let last_known_good_provider = fallback_provider
            .config
            .is_some()
            .then_some(fallback_provider.clone());
        Self {
            fallback_provider,
            fallback_profile,
            last_known_good_provider: Arc::new(Mutex::new(last_known_good_provider)),
            last_provider_diagnostic_context: Arc::new(Mutex::new(None)),
            provider_credential_store,
            transport: UreqChatCompletionTransport,
        }
    }

    fn remember_provider_diagnostic_context(&self, resolved: &ResolvedProvider) {
        if let Ok(mut context) = self.last_provider_diagnostic_context.lock() {
            *context = Some((
                provider_preset_label(&resolved.metadata.preset).to_string(),
                provider_key_source_label(resolved.metadata.key_source).to_string(),
            ));
        }
    }

    fn current_provider(&self) -> ResolvedProvider {
        self.current_provider_from_loader(load_runtime_settings)
    }

    fn current_provider_from_loader<F>(&self, loader: F) -> ResolvedProvider
    where
        F: FnOnce() -> Result<crate::settings_store::LoadedRuntimeSettings, String>,
    {
        match loader() {
            Ok(loaded) => {
                let resolved = resolve_provider_config_with_credentials(
                    &loaded.settings,
                    self.provider_credential_store.as_ref(),
                );
                if resolved.config.is_some() {
                    if let Ok(mut last_known_good) = self.last_known_good_provider.lock() {
                        *last_known_good = Some(resolved.clone());
                    }
                    resolved
                } else if provider_resolution_can_use_last_known_good(&resolved) {
                    self.last_known_good_provider
                        .lock()
                        .ok()
                        .and_then(|provider| provider.clone())
                        .unwrap_or(resolved)
                } else {
                    if let Ok(mut last_known_good) = self.last_known_good_provider.lock() {
                        *last_known_good = None;
                    }
                    resolved
                }
            }
            Err(_) => self
                .last_known_good_provider
                .lock()
                .ok()
                .and_then(|provider| provider.clone())
                .unwrap_or_else(|| self.fallback_provider.clone()),
        }
    }
}

impl SelectedTextEditProvider for DashScopeSelectedTextEditProvider {
    fn profile_label(&self) -> &str {
        &self.fallback_profile.display_label
    }

    fn model_code(&self) -> &str {
        &self.fallback_profile.model_code
    }

    fn provider_preset(&self) -> Option<String> {
        self.last_provider_diagnostic_context
            .lock()
            .ok()
            .and_then(|context| context.as_ref().map(|item| item.0.clone()))
    }

    fn provider_key_source(&self) -> Option<String> {
        self.last_provider_diagnostic_context
            .lock()
            .ok()
            .and_then(|context| context.as_ref().map(|item| item.1.clone()))
    }

    fn is_configured(&self) -> bool {
        let resolved = self.current_provider();
        self.remember_provider_diagnostic_context(&resolved);
        resolved.metadata.configured
    }

    fn edit(&self, request: &ProviderSelectedTextEditRequest<'_>) -> Result<String, String> {
        let resolved = self.current_provider();
        self.remember_provider_diagnostic_context(&resolved);
        let Some(config) = &resolved.config else {
            return Err(format!(
                "{} selected-text provider config missing",
                resolved.metadata.display_label
            ));
        };
        let payload = build_provider_selected_text_edit_request(
            request.selected_text,
            request.instruction_text,
            request.action,
            &resolved.profile,
            &config.capabilities,
        );
        self.transport.complete(config, &payload)
    }
}

fn selected_text_executor_for_settings(settings: &RuntimeSettings) -> RuntimeSelectedTextExecutor {
    ProviderBackedSelectedTextExecutor::new(
        TemporarySelectedTextExecutor,
        DashScopeSelectedTextEditProvider::from_settings(settings),
    )
}

#[derive(Clone)]
struct DashScopeInstructedDictationProvider {
    fallback_provider: ResolvedProvider,
    last_known_good_provider: Arc<Mutex<Option<ResolvedProvider>>>,
    provider_credential_store: Arc<dyn ProviderCredentialStore>,
    transport: UreqChatCompletionTransport,
}

impl DashScopeInstructedDictationProvider {
    fn from_settings(settings: &RuntimeSettings) -> Self {
        Self::from_settings_with_credentials(settings, production_provider_credential_store())
    }

    fn from_settings_with_credentials(
        settings: &RuntimeSettings,
        provider_credential_store: Arc<dyn ProviderCredentialStore>,
    ) -> Self {
        let fallback_provider = resolve_provider_config(settings);
        let last_known_good_provider = fallback_provider
            .config
            .is_some()
            .then_some(fallback_provider.clone());
        Self {
            fallback_provider,
            last_known_good_provider: Arc::new(Mutex::new(last_known_good_provider)),
            provider_credential_store,
            transport: UreqChatCompletionTransport,
        }
    }

    fn current_provider(&self) -> ResolvedProvider {
        self.current_provider_from_loader(load_runtime_settings)
    }

    fn current_provider_from_loader<F>(&self, loader: F) -> ResolvedProvider
    where
        F: FnOnce() -> Result<crate::settings_store::LoadedRuntimeSettings, String>,
    {
        match loader() {
            Ok(loaded) => {
                let resolved = resolve_provider_config_with_credentials(
                    &loaded.settings,
                    self.provider_credential_store.as_ref(),
                );
                if resolved.config.is_some() {
                    if let Ok(mut last_known_good) = self.last_known_good_provider.lock() {
                        *last_known_good = Some(resolved.clone());
                    }
                    resolved
                } else if provider_resolution_can_use_last_known_good(&resolved) {
                    self.last_known_good_provider
                        .lock()
                        .ok()
                        .and_then(|provider| provider.clone())
                        .unwrap_or(resolved)
                } else {
                    if let Ok(mut last_known_good) = self.last_known_good_provider.lock() {
                        *last_known_good = None;
                    }
                    resolved
                }
            }
            Err(_) => self
                .last_known_good_provider
                .lock()
                .ok()
                .and_then(|provider| provider.clone())
                .unwrap_or_else(|| self.fallback_provider.clone()),
        }
    }
}

impl InstructedDictationExecutor for DashScopeInstructedDictationProvider {
    fn transform(
        &self,
        request: &InstructedDictationTransformRequest,
    ) -> Result<InstructedDictationTransformResponse, String> {
        let instruction_char_count = request.instruction_text.chars().count();
        let content_char_count = request.content_text.chars().count();
        let resolved = self.current_provider();
        let Some(config) = &resolved.config else {
            return Err(format!(
                "{} instructed dictation provider config missing; profile={} model={} provider_attempted=false provider_succeeded=false deterministic_fallback_used=false",
                resolved.metadata.display_label,
                resolved.profile.display_label,
                resolved.profile.model_code
            ));
        };
        let payload = build_provider_instructed_dictation_request(
            request,
            &resolved.profile,
            &config.capabilities,
        );
        match self.transport.complete(config, &payload) {
            Ok(output_text) => {
                let output_char_count = output_text.chars().count();
                Ok(InstructedDictationTransformResponse {
                    output_text,
                    strategy: "provider transformed instructed dictation content".to_string(),
                    provider_diagnostics: InstructedDictationProviderDiagnostics {
                        profile_label: resolved.profile.display_label.clone(),
                        model_code: resolved.profile.model_code.clone(),
                        provider_preset: Some(
                            provider_preset_label(&resolved.metadata.preset).to_string(),
                        ),
                        provider_key_source: Some(
                            provider_key_source_label(resolved.metadata.key_source).to_string(),
                        ),
                        provider_attempted: true,
                        provider_succeeded: true,
                        deterministic_fallback_used: false,
                        fallback_reason: None,
                        instruction_char_count,
                        content_char_count,
                        output_char_count,
                    },
                })
            }
            Err(error) => Err(format!(
                "{} instructed dictation provider transform failed; profile={} model={} provider_attempted=true provider_succeeded=false deterministic_fallback_used=false: {error}",
                resolved.metadata.display_label,
                resolved.profile.display_label,
                resolved.profile.model_code
            )),
        }
    }
}

fn instructed_dictation_executor_for_settings(
    settings: &RuntimeSettings,
) -> DashScopeInstructedDictationProvider {
    DashScopeInstructedDictationProvider::from_settings(settings)
}

#[derive(Debug, Serialize)]
struct LiveHostReport {
    generated_at_epoch_ms: u128,
    settings_path: PathBuf,
    settings_source: RuntimeSettingsSource,
    effective_settings: RuntimeSettings,
    settings_warnings: Vec<String>,
    max_capture_ms: Option<u64>,
    session_limit: Option<u64>,
    attempted_sessions: u64,
    successful_sessions: u64,
    failed_sessions: u64,
    dictation_sessions: u64,
    selected_text_sessions: u64,
    wake_phrase_sessions: u64,
    instructed_dictation_sessions: u64,
    selected_text_action_mix: Vec<ActionMixEntry>,
    wake_phrase_action_mix: Vec<ActionMixEntry>,
    failed_with_summary_sessions: u64,
    failed_before_summary_sessions: u64,
    recording_failures: u64,
    recognizing_failures: u64,
    executing_failures: u64,
    committing_failures: u64,
    silence_gate_failures: u64,
    no_speech_failures: u64,
    direct_unicode_commits: u64,
    clipboard_fallback_commits: u64,
    selection_replace_commits: u64,
    temporary_stub_commits: u64,
    unknown_transport_commits: u64,
    selected_text_compatibility_signal: Option<String>,
    selected_text_compatibility_guidance: Option<String>,
    dominant_failure_mode: Option<String>,
    dominant_failure_guidance: Option<String>,
    commit_path_outlook: Option<String>,
    commit_path_signal: Option<String>,
    commit_path_guidance: Option<String>,
    verification_focus: Option<String>,
    verification_focus_guidance: Option<String>,
    verification_title: Option<String>,
    verification_scenario: Option<String>,
    verification_command: Option<String>,
    verification_gesture: Option<String>,
    verification_example: Option<String>,
    verification_note: Option<String>,
    workload_focus: Option<String>,
    workload_focus_guidance: Option<String>,
    last_failure: Option<String>,
    last_failure_summary: Option<LiveHostFailureSummary>,
    failure_history: Vec<LiveHostFailureRecord>,
    sessions: Vec<SessionSummary>,
    diagnostics: Vec<DiagnosticEvent>,
}

#[derive(Clone, Debug, Serialize)]
struct LiveHostFailureSummary {
    session_id: u64,
    session_kind: shared_protocol::SessionKind,
    failure_phase: shared_protocol::SessionFailurePhase,
    audio_duration_ms: Option<u64>,
    audio_peak_level: Option<f32>,
    audio_rms_level: Option<f32>,
    error: String,
    selected_text_compatibility: Option<SelectedTextCompatibilityIssue>,
    foreground_app: Option<ForegroundAppMetadata>,
}

#[derive(Clone, Debug, Serialize)]
struct LiveHostFailureRecord {
    attempted_session_index: u64,
    occurred_at_epoch_ms: u128,
    session_id: u64,
    session_kind: shared_protocol::SessionKind,
    failure_phase: shared_protocol::SessionFailurePhase,
    audio_duration_ms: Option<u64>,
    audio_peak_level: Option<f32>,
    audio_rms_level: Option<f32>,
    error: String,
    selected_text_compatibility: Option<SelectedTextCompatibilityIssue>,
    foreground_app: Option<ForegroundAppMetadata>,
}

#[derive(Clone, Copy, Debug, Default)]
struct CommitTransportCounts {
    direct_unicode_commits: u64,
    clipboard_fallback_commits: u64,
    selection_replace_commits: u64,
    temporary_stub_commits: u64,
    unknown_transport_commits: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct SessionOutcomeCounts {
    successful_sessions: u64,
    failed_with_summary_sessions: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct FailureProfileCounts {
    recording_failures: u64,
    recognizing_failures: u64,
    executing_failures: u64,
    committing_failures: u64,
    silence_gate_failures: u64,
    no_speech_failures: u64,
}

const MIN_PUSH_TO_TALK_SAFETY_TIMEOUT_MS: u64 = 5_000;
const DASHSCOPE_API_KEY_ENV: &str = "DASHSCOPE_API_KEY";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LocalEnvLoadReport {
    diagnostics: Vec<String>,
    warnings: Vec<String>,
    loaded_sources: HashMap<String, String>,
    dashscope_api_key_source: String,
}

fn load_local_env_files() -> LocalEnvLoadReport {
    let mut report = LocalEnvLoadReport {
        dashscope_api_key_source: if env::var_os(DASHSCOPE_API_KEY_ENV).is_some() {
            "process env".to_string()
        } else {
            "missing".to_string()
        },
        ..LocalEnvLoadReport::default()
    };

    let workspace_root = match resolve_env_workspace_root() {
        Ok(path) => path,
        Err(error) => {
            report
                .warnings
                .push(format!("failed to resolve env workspace root: {error}"));
            return report;
        }
    };

    report.diagnostics.push(format!(
        "workspace_root={} current_dir={}",
        workspace_root.display(),
        env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|error| format!("<unavailable: {error}>"))
    ));

    for (file_name, label) in [(".env.local", ".env.local"), (".env", ".env")] {
        let path = workspace_root.join(file_name);
        load_env_file_if_present(label, &path, &mut report);
    }

    if let Some(source) = report.loaded_sources.get(DASHSCOPE_API_KEY_ENV) {
        report.dashscope_api_key_source = source.clone();
    } else if env::var_os(DASHSCOPE_API_KEY_ENV).is_none() {
        report.dashscope_api_key_source = "missing".to_string();
    }

    report.diagnostics.push(format!(
        "{DASHSCOPE_API_KEY_ENV} source={}",
        report.dashscope_api_key_source
    ));
    report
}

fn print_local_env_load_report(report: &LocalEnvLoadReport) {
    for diagnostic in &report.diagnostics {
        eprintln!("voiceflow env: {diagnostic}");
    }
    for warning in &report.warnings {
        eprintln!("voiceflow env warning: {warning}");
    }
}

fn resolve_env_workspace_root() -> Result<PathBuf, String> {
    let asset_root = app_paths::resolve_repo_root()?;
    if !asset_root.join("Cargo.toml").is_file() {
        // A portable bundle must not inherit a parent checkout's .env.local.
        return Ok(asset_root);
    }
    let current_dir = env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    if let Some(root) = find_env_workspace_root(&current_dir) {
        return Ok(root);
    }

    Ok(asset_root)
}

fn find_env_workspace_root(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|path| {
        if path.join("Cargo.toml").exists()
            && path.join("apps").exists()
            && path.join("crates").exists()
        {
            Some(path.to_path_buf())
        } else {
            None
        }
    })
}

fn load_env_file_if_present(label: &str, path: &Path, report: &mut LocalEnvLoadReport) {
    report.diagnostics.push(format!(
        "considered {label} path={} exists={}",
        path.display(),
        path.exists()
    ));
    if !path.exists() {
        return;
    }

    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            report.warnings.push(format!(
                "failed to read {label} at {}: {error}",
                path.display()
            ));
            return;
        }
    };

    let mut seen_in_file = HashSet::new();
    for (line_index, line) in raw.lines().enumerate() {
        let Some((key, value)) = parse_env_file_assignment(line) else {
            continue;
        };
        if !seen_in_file.insert(key.clone()) {
            report.warnings.push(format!(
                "duplicate key {key} in {label} at {} line {}; first value wins",
                path.display(),
                line_index + 1
            ));
        }

        if env::var_os(&key).is_none() {
            // SAFETY: input-host loads local env files at process startup before it
            // spawns worker/background threads that may concurrently access env.
            unsafe {
                env::set_var(&key, value);
            }
            report.loaded_sources.insert(key.clone(), label.to_string());
            report.diagnostics.push(format!(
                "loaded key={key} from {label} path={}",
                path.display()
            ));
        } else if report.loaded_sources.contains_key(&key) {
            report.diagnostics.push(format!(
                "skipped key={key} from {label} path={} because a higher-priority env file already set it",
                path.display()
            ));
        } else {
            report.diagnostics.push(format!(
                "skipped key={key} from {label} path={} because process env already existed",
                path.display(),
            ));
        }
    }

    report
        .diagnostics
        .push(format!("loaded {label} path={}", path.display()));
}

fn parse_env_file_assignment(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (raw_key, raw_value) = assignment.split_once('=')?;
    let key = raw_key.trim();
    if !is_valid_env_key(key) {
        return None;
    }

    Some((key.to_string(), parse_env_file_value(raw_value)))
}

fn parse_env_file_value(raw_value: &str) -> String {
    let without_comment = strip_env_inline_comment(raw_value);
    let trimmed = without_comment.trim();
    if trimmed.len() >= 2 {
        let mut chars = trimmed.chars();
        let first = chars.next();
        let last = trimmed.chars().last();
        if matches!(
            (first, last),
            (Some('"'), Some('"')) | (Some('\''), Some('\''))
        ) {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }

    trimmed.to_string()
}

fn strip_env_inline_comment(raw_value: &str) -> &str {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for (index, ch) in raw_value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double_quote && ch == '\\' {
            escaped = true;
            continue;
        }
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '#' if !in_single_quote && !in_double_quote => {
                let before_comment = &raw_value[..index];
                if before_comment.trim().is_empty()
                    || before_comment
                        .chars()
                        .last()
                        .map(char::is_whitespace)
                        .unwrap_or(false)
                {
                    return before_comment;
                }
            }
            _ => {}
        }
    }

    raw_value
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn parse_command<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let collected: Vec<String> = args.into_iter().collect();
    if collected.is_empty() {
        return Ok(Command::Demo);
    }

    match collected[0].as_str() {
        "--listen-once" => Ok(Command::ListenOnce {
            capture_ms: parse_capture_ms(&collected[1..])?,
        }),
        "--listen-once-live" => Ok(Command::ListenOnceLive {
            max_capture_ms: parse_optional_capture_ms(&collected[1..])?,
        }),
        "--serve-live" => {
            let (max_capture_ms, session_limit) = parse_live_loop_args(&collected[1..])?;
            Ok(Command::ServeLive {
                max_capture_ms,
                session_limit,
            })
        }
        "--capture-probe" => Ok(Command::CaptureProbe {
            capture_ms: parse_capture_ms(&collected[1..])?,
        }),
        "--transcribe-probe" => Ok(Command::TranscribeProbe {
            capture_ms: parse_capture_ms(&collected[1..])?,
        }),
        "--print-settings" => {
            ensure_no_extra_args("--print-settings", &collected[1..])?;
            Ok(Command::PrintSettings)
        }
        "--write-default-settings" => {
            ensure_no_extra_args("--write-default-settings", &collected[1..])?;
            Ok(Command::WriteDefaultSettings)
        }
        "--update-settings" => Ok(Command::UpdateSettings {
            update: parse_settings_update(&collected[1..])?,
        }),
        "--serve-settings" => Ok(Command::ServeSettings {
            port: parse_settings_port(&collected[1..])?,
        }),
        "--insert-probe" => Ok(Command::InsertProbe {
            text: parse_probe_text(&collected[1..]),
        }),
        "--sendinput-key-probe" => Ok(Command::SendInputKeyProbe {
            delay_ms: parse_delay_ms_probe_args("--sendinput-key-probe", &collected[1..])?,
        }),
        "--clipboard-only-probe" => Ok(Command::ClipboardOnlyProbe {
            text: parse_required_probe_text("--clipboard-only-probe", &collected[1..])?,
        }),
        "--ctrl-v-probe" => Ok(Command::CtrlVProbe {
            delay_ms: parse_delay_ms_probe_args("--ctrl-v-probe", &collected[1..])?,
        }),
        "--edit-probe" => {
            let parsed = parse_selected_text_probe_args(&collected[1..], "uppercase")?;
            Ok(Command::EditProbe {
                instruction: parsed.text,
                app_label: parsed.app_label,
                probe_note: parsed.probe_note,
            })
        }
        "--edit-transcribe-probe" => Ok(Command::EditTranscribeProbe {
            capture_ms: parse_capture_ms(&collected[1..])?,
        }),
        "--intent-probe" => Ok(Command::IntentProbe {
            request: parse_intent_request(&collected[1..]),
        }),
        "--replace-selection-probe" => {
            let parsed = parse_selected_text_probe_args(
                &collected[1..],
                "VoiceFlow native selection replacement probe.",
            )?;
            Ok(Command::ReplaceSelectionProbe {
                text: parsed.text,
                app_label: parsed.app_label,
                probe_note: parsed.probe_note,
            })
        }
        "--print-selected-text-validation-report" => {
            Ok(Command::PrintSelectedTextValidationReport {
                app_label: parse_optional_filter_label(&collected[1..]),
            })
        }
        "--reset-selected-text-validation-baseline" => {
            Ok(Command::ResetSelectedTextValidationBaseline {
                label: parse_optional_filter_label(&collected[1..]),
            })
        }
        other => Err(format!(
            "unknown argument `{other}`; supported flags are --listen-once, --listen-once-live, --serve-live, --capture-probe, --transcribe-probe, --print-settings, --write-default-settings, --update-settings, --serve-settings, --insert-probe, --sendinput-key-probe, --clipboard-only-probe, --ctrl-v-probe, --edit-probe, --edit-transcribe-probe, --intent-probe, --replace-selection-probe, --print-selected-text-validation-report, and --reset-selected-text-validation-baseline"
        )),
    }
}

fn parse_capture_ms(args: &[String]) -> Result<u64, String> {
    if args.is_empty() {
        return Ok(1_500);
    }

    if args.len() != 1 {
        return Err("expected at most one capture duration argument in milliseconds".to_string());
    }

    args[0]
        .parse::<u64>()
        .map_err(|_| format!("invalid capture duration `{}`", args[0]))
}

fn parse_optional_capture_ms(args: &[String]) -> Result<Option<u64>, String> {
    if args.is_empty() {
        return Ok(None);
    }

    if args.len() != 1 {
        return Err(
            "expected at most one optional safety timeout argument in milliseconds".to_string(),
        );
    }

    args[0]
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("invalid capture duration `{}`", args[0]))
}

fn parse_live_loop_args(args: &[String]) -> Result<(Option<u64>, Option<u64>), String> {
    match args {
        [] => Ok((None, None)),
        [max_capture_ms] => Ok((
            Some(
                max_capture_ms
                    .parse::<u64>()
                    .map_err(|_| format!("invalid capture duration `{max_capture_ms}`"))?,
            ),
            None,
        )),
        [max_capture_ms, session_limit] => Ok((
            Some(
                max_capture_ms
                    .parse::<u64>()
                    .map_err(|_| format!("invalid capture duration `{max_capture_ms}`"))?,
            ),
            Some(
                session_limit
                    .parse::<u64>()
                    .map_err(|_| format!("invalid session limit `{session_limit}`"))?,
            ),
        )),
        _ => Err(
            "expected at most two arguments for --serve-live: [optional safety timeout ms] [optional session limit]"
                .to_string(),
        ),
    }
}

fn parse_probe_text(args: &[String]) -> String {
    if args.is_empty() {
        return "VoiceFlow native insertion probe.".to_string();
    }

    args.join(" ")
}

fn parse_required_probe_text(flag: &str, args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err(format!("{flag} expects text to place on the clipboard"));
    }
    Ok(args.join(" "))
}

fn parse_delay_ms_probe_args(flag: &str, args: &[String]) -> Result<u64, String> {
    match args {
        [] => Ok(3_000),
        [option, value] if option == "--delay-ms" => value
            .parse::<u64>()
            .map_err(|_| format!("invalid --delay-ms value `{value}` for {flag}")),
        [option, _] if option.starts_with("--") => Err(format!(
            "unsupported {flag} option `{option}`; supported option is --delay-ms"
        )),
        _ => Err(format!(
            "{flag} expects optional syntax: --delay-ms <milliseconds>"
        )),
    }
}

struct ParsedSelectedTextProbeArgs {
    text: String,
    app_label: Option<String>,
    probe_note: Option<String>,
}

fn parse_selected_text_probe_args(
    args: &[String],
    default_text: &str,
) -> Result<ParsedSelectedTextProbeArgs, String> {
    let mut app_label = None;
    let mut probe_note = None;
    let mut text_parts = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--app-label" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--app-label expects a value".to_string())?;
                if app_label.is_some() {
                    return Err("--app-label was provided more than once".to_string());
                }
                app_label = Some(value.clone());
                index += 2;
            }
            "--probe-note" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--probe-note expects a value".to_string())?;
                if probe_note.is_some() {
                    return Err("--probe-note was provided more than once".to_string());
                }
                probe_note = Some(value.clone());
                index += 2;
            }
            token if token.starts_with("--") => {
                return Err(format!(
                    "unsupported selected-text probe option `{token}`; supported options are --app-label and --probe-note"
                ));
            }
            token => {
                text_parts.push(token.to_string());
                index += 1;
            }
        }
    }

    Ok(ParsedSelectedTextProbeArgs {
        text: if text_parts.is_empty() {
            default_text.to_string()
        } else {
            text_parts.join(" ")
        },
        app_label,
        probe_note,
    })
}

fn parse_optional_filter_label(args: &[String]) -> Option<String> {
    if args.is_empty() {
        None
    } else {
        Some(args.join(" "))
    }
}

fn parse_settings_update(args: &[String]) -> Result<RuntimeSettingsUpdate, String> {
    if args.is_empty() {
        return Err("--update-settings expects one or more key=value assignments".to_string());
    }

    let mut update = RuntimeSettingsUpdate::default();
    for assignment in args {
        let (raw_key, raw_value) = assignment.split_once('=').ok_or_else(|| {
            format!("invalid settings update `{assignment}`; expected key=value assignments")
        })?;
        let key = raw_key.trim();
        let normalized_key = key.to_ascii_lowercase();
        let value = raw_value.trim();

        match normalized_key.as_str() {
            "shortcut_mode" => assign_once(
                &mut update.shortcut_mode,
                parse_shortcut_mode_setting(value)?,
                key,
            )?,
            "primary_shortcut_key" => assign_once(
                &mut update.primary_shortcut_key,
                parse_shortcut_key_setting(value)?,
                key,
            )?,
            "primary_shortcut_modifiers" => assign_once(
                &mut update.primary_shortcut_modifiers,
                parse_key_modifiers_setting(value)?,
                key,
            )?,
            "refinement_quality" => assign_once(
                &mut update.refinement_quality,
                parse_refinement_quality_setting(value)?,
                key,
            )?,
            "silence_gate_level" => assign_once(
                &mut update.silence_gate_level,
                parse_silence_gate_level_setting(value)?,
                key,
            )?,
            "diagnostics_verbosity" => assign_once(
                &mut update.diagnostics_verbosity,
                parse_diagnostics_verbosity_setting(value)?,
                key,
            )?,
            "system_language" => assign_once(
                &mut update.system_language,
                parse_system_language_setting(value)?,
                key,
            )?,
            "ui_style" => assign_once(&mut update.ui_style, parse_ui_style_setting(value)?, key)?,
            "audio_feedback_enabled" => assign_once(
                &mut update.audio_feedback_enabled,
                parse_bool_setting(value, key)?,
                key,
            )?,
            "wake_phrase_enabled" | "wake_phrase.enabled" => assign_once(
                &mut update.wake_phrase_enabled,
                parse_bool_setting(value, key)?,
                key,
            )?,
            "wake_phrase_phrase" | "wake_phrase.phrase" => {
                assign_once(&mut update.wake_phrase_phrase, value.to_string(), key)?
            }
            "history_retention" => assign_once(
                &mut update.history_retention,
                parse_history_retention_setting(value)?,
                key,
            )?,
            "provider.preset" | "provider_preset" => assign_once(
                &mut provider_update_mut(&mut update).preset,
                parse_provider_preset_setting(value)?,
                key,
            )?,
            "provider.base_url" | "provider_base_url" => assign_once(
                &mut provider_update_mut(&mut update).base_url,
                parse_optional_string_setting(value),
                key,
            )?,
            "provider.active_model" | "provider_active_model" => assign_once(
                &mut provider_update_mut(&mut update).active_model,
                parse_optional_string_setting(value),
                key,
            )?,
            "provider.request_timeout_ms" | "provider_request_timeout_ms" => assign_once(
                &mut provider_update_mut(&mut update).request_timeout_ms,
                parse_optional_u64_setting(value, key)?,
                key,
            )?,
            _ => {
                return Err(format!(
                    "unsupported settings field `{key}`; supported fields are primary_shortcut_key, primary_shortcut_modifiers, shortcut_mode, refinement_quality, silence_gate_level, diagnostics_verbosity, system_language, ui_style, audio_feedback_enabled, wake_phrase_enabled, wake_phrase_phrase, history_retention, provider.preset, provider.base_url, provider.active_model, and provider.request_timeout_ms"
                ));
            }
        }
    }

    if update.is_empty() {
        return Err("at least one supported runtime setting update must be provided".to_string());
    }

    Ok(update)
}

fn provider_update_mut(update: &mut RuntimeSettingsUpdate) -> &mut ProviderSettingsUpdate {
    update
        .provider
        .get_or_insert_with(ProviderSettingsUpdate::default)
}

fn parse_optional_string_setting(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_optional_u64_setting(value: &str, field_name: &str) -> Result<Option<u64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("invalid {field_name} `{value}`; expected an integer"))
}

fn parse_settings_port(args: &[String]) -> Result<u16, String> {
    if args.is_empty() {
        return Ok(47_831);
    }

    if args.len() != 1 {
        return Err("expected at most one optional port argument for --serve-settings".to_string());
    }

    args[0]
        .parse::<u16>()
        .map_err(|_| format!("invalid settings control port `{}`", args[0]))
}

fn assign_once<T>(slot: &mut Option<T>, value: T, field_name: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!(
            "settings field `{field_name}` was provided more than once"
        ));
    }

    *slot = Some(value);
    Ok(())
}

fn parse_shortcut_mode_setting(value: &str) -> Result<ShortcutMode, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "toggle" => Ok(ShortcutMode::Toggle),
        "pushtotalk" | "push-to-talk" | "push_to_talk" => Ok(ShortcutMode::PushToTalk),
        _ => Err(format!(
            "invalid shortcut_mode `{value}`; expected Toggle or PushToTalk"
        )),
    }
}

fn parse_key_modifiers_setting(value: &str) -> Result<Vec<shared_protocol::KeyModifier>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(
            "invalid primary_shortcut_modifiers ``; expected one or more modifier names"
                .to_string(),
        );
    }

    let mut modifiers = Vec::new();
    for raw in trimmed.split(['+', ',', '|']) {
        let normalized = raw.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        let modifier = match normalized.as_str() {
            "control" | "ctrl" => shared_protocol::KeyModifier::Control,
            "alt" => shared_protocol::KeyModifier::Alt,
            "shift" => shared_protocol::KeyModifier::Shift,
            "meta" | "win" | "windows" => shared_protocol::KeyModifier::Meta,
            _ => {
                return Err(format!(
                    "invalid primary_shortcut_modifiers entry `{raw}`; expected Control, Alt, Shift, or Meta"
                ));
            }
        };
        if !modifiers.contains(&modifier) {
            modifiers.push(modifier);
        }
    }

    if modifiers.is_empty() {
        return Err(
            "invalid primary_shortcut_modifiers; expected one or more modifier names".to_string(),
        );
    }

    modifiers.sort_by_key(shortcut_modifier_sort_key);

    Ok(modifiers)
}

fn parse_shortcut_key_setting(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(
            "invalid primary_shortcut_key ``; expected Space or a single alphanumeric key"
                .to_string(),
        );
    }

    if trimmed.eq_ignore_ascii_case("space") {
        return Ok("Space".to_string());
    }

    let mut chars = trimmed.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_ascii_alphanumeric() => Ok(ch.to_ascii_uppercase().to_string()),
        _ => Err(format!(
            "invalid primary_shortcut_key `{value}`; expected Space or a single alphanumeric key"
        )),
    }
}

fn shortcut_modifier_sort_key(modifier: &shared_protocol::KeyModifier) -> u8 {
    match modifier {
        shared_protocol::KeyModifier::Control => 0,
        shared_protocol::KeyModifier::Alt => 1,
        shared_protocol::KeyModifier::Shift => 2,
        shared_protocol::KeyModifier::Meta => 3,
    }
}

fn parse_refinement_quality_setting(value: &str) -> Result<RefinementQuality, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "fast" => Ok(RefinementQuality::Fast),
        "fastplus" | "fast+" | "fast-plus" | "fast_plus" => Ok(RefinementQuality::FastPlus),
        "balanced" => Ok(RefinementQuality::Balanced),
        "bestquality" | "best-quality" | "best_quality" | "best" => {
            Ok(RefinementQuality::BestQuality)
        }
        _ => Err(format!(
            "invalid refinement_quality `{value}`; expected Fast, Balanced, or BestQuality"
        )),
    }
}

fn parse_silence_gate_level_setting(value: &str) -> Result<u8, String> {
    let parsed = value.trim().parse::<u8>().map_err(|_| {
        format!("invalid silence_gate_level `{value}`; expected an integer from 1 to 5")
    })?;
    if !(1..=5).contains(&parsed) {
        return Err(format!(
            "invalid silence_gate_level `{value}`; expected an integer from 1 to 5"
        ));
    }

    Ok(parsed)
}

fn parse_diagnostics_verbosity_setting(value: &str) -> Result<DiagnosticsVerbosity, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "standard" => Ok(DiagnosticsVerbosity::Standard),
        "verbose" => Ok(DiagnosticsVerbosity::Verbose),
        _ => Err(format!(
            "invalid diagnostics_verbosity `{value}`; expected Standard or Verbose"
        )),
    }
}

fn parse_system_language_setting(value: &str) -> Result<SystemLanguage, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "english" | "en" => Ok(SystemLanguage::English),
        "chinese" | "zh" | "zh-cn" | "中文" => Ok(SystemLanguage::Chinese),
        _ => Err(format!(
            "invalid system_language `{value}`; expected English or Chinese"
        )),
    }
}

fn parse_ui_style_setting(value: &str) -> Result<UiStyle, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dark" => Ok(UiStyle::Dark),
        "light" => Ok(UiStyle::Light),
        _ => Err(format!(
            "invalid ui_style `{value}`; expected Dark or Light"
        )),
    }
}

fn parse_history_retention_setting(value: &str) -> Result<HistoryRetention, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "latest_100" => Ok(HistoryRetention::Latest100),
        "latest_500" => Ok(HistoryRetention::Latest500),
        "latest_1000" => Ok(HistoryRetention::Latest1000),
        "last_7_days" => Ok(HistoryRetention::Last7Days),
        "last_30_days" => Ok(HistoryRetention::Last30Days),
        "unlimited" => Ok(HistoryRetention::Unlimited),
        _ => Err(format!(
            "invalid history_retention `{value}`; expected latest_100, latest_500, latest_1000, last_7_days, last_30_days, or unlimited"
        )),
    }
}

fn parse_provider_preset_setting(value: &str) -> Result<ProviderPreset, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "bailian" | "dashscope" | "aliyun" | "alibaba" | "alibaba_cloud_bailian" => {
            Ok(ProviderPreset::Bailian)
        }
        "volcengine_ark" | "volcengine" | "ark" => Ok(ProviderPreset::VolcengineArk),
        "tencent_hunyuan" | "tencent" | "hunyuan" => Ok(ProviderPreset::TencentHunyuan),
        "custom_openai_compatible" | "custom" | "openai_compatible" => {
            Ok(ProviderPreset::CustomOpenAiCompatible)
        }
        _ => Err(format!(
            "invalid provider preset `{value}`; expected bailian, volcengine_ark, tencent_hunyuan, or custom_openai_compatible"
        )),
    }
}

fn parse_bool_setting(value: &str, field_name: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!(
            "invalid {field_name} `{value}`; expected true/false"
        )),
    }
}

fn ensure_no_extra_args(flag: &str, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{flag} does not accept additional arguments; received `{}`",
            args.join(" ")
        ))
    }
}

fn parse_intent_request(args: &[String]) -> String {
    if args.is_empty() {
        return "Hey VoiceFlow, draft an email to my manager about taking Friday off.".to_string();
    }

    let joined = args.join(" ");
    if joined.to_ascii_lowercase().starts_with("hey voiceflow") {
        joined
    } else {
        format!("Hey VoiceFlow, {joined}")
    }
}

fn selected_text_action_label(
    action: &shared_protocol::SelectedTextExecutionAction,
) -> &'static str {
    match action {
        shared_protocol::SelectedTextExecutionAction::Uppercase => "Uppercase",
        shared_protocol::SelectedTextExecutionAction::Lowercase => "Lowercase",
        shared_protocol::SelectedTextExecutionAction::TitleCase => "Title case",
        shared_protocol::SelectedTextExecutionAction::SentenceCase => "Sentence case",
        shared_protocol::SelectedTextExecutionAction::SnakeCase => "Snake case",
        shared_protocol::SelectedTextExecutionAction::KebabCase => "Kebab case",
        shared_protocol::SelectedTextExecutionAction::CamelCase => "Camel case",
        shared_protocol::SelectedTextExecutionAction::PascalCase => "Pascal case",
        shared_protocol::SelectedTextExecutionAction::ConstantCase => "Constant case",
        shared_protocol::SelectedTextExecutionAction::InlineCode => "Inline code",
        shared_protocol::SelectedTextExecutionAction::CodeBlock => "Code block",
        shared_protocol::SelectedTextExecutionAction::StripCodeFence => "Strip code fence",
        shared_protocol::SelectedTextExecutionAction::MarkdownBold => "Markdown bold",
        shared_protocol::SelectedTextExecutionAction::MarkdownItalic => "Markdown italic",
        shared_protocol::SelectedTextExecutionAction::StripMarkdownEmphasis => {
            "Strip markdown emphasis"
        }
        shared_protocol::SelectedTextExecutionAction::WrapInQuotes => "Wrap in quotes",
        shared_protocol::SelectedTextExecutionAction::BulletList => "Bullet list",
        shared_protocol::SelectedTextExecutionAction::Checklist => "Checklist",
        shared_protocol::SelectedTextExecutionAction::QuoteBlock => "Quote block",
        shared_protocol::SelectedTextExecutionAction::NumberedList => "Numbered list",
        shared_protocol::SelectedTextExecutionAction::SortLines => "Sort lines",
        shared_protocol::SelectedTextExecutionAction::DeduplicateLines => "Deduplicate lines",
        shared_protocol::SelectedTextExecutionAction::RemoveEmptyLines => "Remove empty lines",
        shared_protocol::SelectedTextExecutionAction::CommaSeparated => "Comma-separated",
        shared_protocol::SelectedTextExecutionAction::PipeSeparated => "Pipe-separated",
        shared_protocol::SelectedTextExecutionAction::TabSeparated => "Tab-separated",
        shared_protocol::SelectedTextExecutionAction::SemicolonSeparated => "Semicolon-separated",
        shared_protocol::SelectedTextExecutionAction::JsonArray => "JSON array",
        shared_protocol::SelectedTextExecutionAction::QuotedCsv => "Quoted CSV",
        shared_protocol::SelectedTextExecutionAction::SqlInList => "SQL IN list",
        shared_protocol::SelectedTextExecutionAction::YamlList => "YAML list",
        shared_protocol::SelectedTextExecutionAction::YamlMapping => "YAML mapping",
        shared_protocol::SelectedTextExecutionAction::MarkdownTable => "Markdown table",
        shared_protocol::SelectedTextExecutionAction::HeaderBlock => "Header block",
        shared_protocol::SelectedTextExecutionAction::JsonObject => "JSON object",
        shared_protocol::SelectedTextExecutionAction::EnvBlock => "Env block",
        shared_protocol::SelectedTextExecutionAction::QueryString => "Query string",
        shared_protocol::SelectedTextExecutionAction::TomlTable => "TOML table",
        shared_protocol::SelectedTextExecutionAction::ShellExports => "Shell exports",
        shared_protocol::SelectedTextExecutionAction::PowershellEnv => "PowerShell env",
        shared_protocol::SelectedTextExecutionAction::CurlHeaders => "curl headers",
        shared_protocol::SelectedTextExecutionAction::PythonDict => "Python dict",
        shared_protocol::SelectedTextExecutionAction::JavascriptObject => "JavaScript object",
        shared_protocol::SelectedTextExecutionAction::RubyHash => "Ruby hash",
        shared_protocol::SelectedTextExecutionAction::SqlValuesRows => "SQL VALUES rows",
        shared_protocol::SelectedTextExecutionAction::StripListMarkers => "Strip list markers",
        shared_protocol::SelectedTextExecutionAction::SentencePerLine => "Sentence per line",
        shared_protocol::SelectedTextExecutionAction::MarkdownHeading => "Markdown heading",
        shared_protocol::SelectedTextExecutionAction::SingleParagraph => "Single paragraph",
        shared_protocol::SelectedTextExecutionAction::CleanupSpacing => "Spacing cleanup",
        shared_protocol::SelectedTextExecutionAction::PolishWriting => "Polish writing",
        shared_protocol::SelectedTextExecutionAction::ConciseRewrite => "Concise rewrite",
        shared_protocol::SelectedTextExecutionAction::FormalRewrite => "Formal rewrite",
        shared_protocol::SelectedTextExecutionAction::BulletSummary => "Bullet summary",
        shared_protocol::SelectedTextExecutionAction::GeneralProviderEdit => "General edit",
        shared_protocol::SelectedTextExecutionAction::PromptScaffold => "Prompt scaffold",
    }
}

fn wake_phrase_action_label(action: &shared_protocol::WakePhraseIntentAction) -> &'static str {
    match action {
        shared_protocol::WakePhraseIntentAction::DraftEmail => "Draft email",
        shared_protocol::WakePhraseIntentAction::Summarize => "Summarize",
        shared_protocol::WakePhraseIntentAction::BulletPlan => "Bullet plan",
        shared_protocol::WakePhraseIntentAction::Rewrite => "Rewrite",
        shared_protocol::WakePhraseIntentAction::Checklist => "Checklist",
        shared_protocol::WakePhraseIntentAction::ReplyMessage => "Reply draft",
        shared_protocol::WakePhraseIntentAction::GeneralDraft => "General draft",
    }
}

fn run_demo() -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let primary_shortcut = primary_shortcut_label(&settings);
    let settings_runtime_state_path =
        write_settings_runtime_state(&loaded_settings, None, None, None)?;
    let overlay_adapter = MirroringOverlayAdapter::new(&settings)?;
    let overlay_state_script_path = overlay_adapter.script_path().to_path_buf();
    let mut runtime = HostRuntime::with_speech_engine(
        settings,
        SpeechEngine::default(),
        NoopShortcutAdapter,
        NoopAudioCaptureAdapter,
        StubTextInsertionAdapter,
        overlay_adapter,
        NoopAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    print_settings_warnings(&loaded_settings.warnings);
    let summary = runtime.run_milestone_one_demo()?;
    print_summary(&summary, runtime.diagnostics_len());
    println!(
        "overlay_runtime_state: {}",
        overlay_state_script_path.display()
    );
    println!(
        "settings_runtime_state: {}",
        settings_runtime_state_path.display()
    );
    println!("Run with --capture-probe [ms] to test microphone capture.");
    println!(
        "Run with --transcribe-probe [ms] to record and transcribe through the local ASR worker."
    );
    println!(
        "Run with --print-settings to inspect the effective runtime settings and config path."
    );
    println!("Run with --write-default-settings to write a starter config file for the runtime.");
    println!(
        "Run with --update-settings key=value [...] to persist supported runtime settings through the host."
    );
    println!(
        "Run with --serve-settings [port] to expose a localhost settings control bridge for the static companion UI."
    );
    println!(
        "Run with --listen-once [ms] on Windows to wait for a real global hotkey trigger using the current primary shortcut (currently {}).",
        primary_shortcut
    );
    println!(
        "Run with --listen-once-live [max_ms] on Windows to wait for one real hotkey trigger using the current primary shortcut (currently {}).",
        primary_shortcut
    );
    println!(
        "Run with --serve-live [max_ms] [session_limit] on Windows to keep the live host running across multiple sessions."
    );
    println!(
        "Run with --insert-probe [text] on Windows to intentionally type into the focused app."
    );
    println!(
        "Run with --sendinput-key-probe [--delay-ms ms] on Windows to send a minimal A key down/up sequence into the focused app."
    );
    println!(
        "Run with --clipboard-only-probe \"text\" on Windows to write text to the clipboard without sending Ctrl+V."
    );
    println!(
        "Run with --ctrl-v-probe [--delay-ms ms] on Windows to send only Ctrl+V into the focused app."
    );
    println!(
        "Run with --edit-probe [instruction] [--app-label name] [--probe-note note] on Windows to run the real selected-text edit session flow with a typed instruction."
    );
    println!(
        "Run with --edit-transcribe-probe [ms] on Windows to run the selected-text edit session flow with live microphone transcription."
    );
    println!(
        "Run with --intent-probe [request] to run a deterministic wake-phrase intent session."
    );
    println!(
        "Run with --replace-selection-probe [text] [--app-label name] [--probe-note note] on Windows to capture and replace highlighted text in the focused app."
    );
    println!(
        "Run with --print-selected-text-validation-report [optional app label] to inspect the accumulated real-app selected-text compatibility matrix."
    );
    println!(
        "Run with --reset-selected-text-validation-baseline [optional label] to archive the active selected-text validation ledger and start a fresh post-fix baseline."
    );
    Ok(())
}

#[cfg(windows)]
fn run_listen_once(capture_ms: u64) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let primary_shortcut = primary_shortcut_label(&settings);
    let settings_runtime_state_path =
        write_settings_runtime_state(&loaded_settings, None, None, None)?;
    let overlay_adapter = MirroringOverlayAdapter::new(&settings)?;
    let overlay_state_script_path = overlay_adapter.script_path().to_path_buf();
    let mut runtime = HostRuntime::new(
        settings,
        WindowsHotkeyAdapter::default(),
        CpalAudioCaptureAdapter::default(),
        StubTextInsertionAdapter,
        overlay_adapter,
        WindowsAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    runtime.prepare_speech_engine();
    print_settings_warnings(&loaded_settings.warnings);
    println!(
        "Listening for one hotkey trigger. Press {}. If text is selected, the session will route to edit mode automatically. Recording window: {} ms.",
        primary_shortcut, capture_ms
    );
    println!("Overlay mirror: {}", overlay_state_script_path.display());
    println!("Settings mirror: {}", settings_runtime_state_path.display());
    let trigger = runtime.wait_for_trigger()?;
    let summary = runtime
        .run_session_for_trigger_with_capture_window(
            trigger,
            Some(Duration::from_millis(capture_ms)),
        )
        .map_err(|error| error.to_string())?;
    print_summary(&summary, runtime.diagnostics_len());
    Ok(())
}

#[cfg(not(windows))]
fn run_listen_once(_capture_ms: u64) -> Result<(), String> {
    Err("--listen-once is only available on Windows".to_string())
}

#[cfg(windows)]
fn run_listen_once_live(max_capture_ms: Option<u64>) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let primary_shortcut = primary_shortcut_label(&settings);
    let start_stop_description = live_start_stop_description(&settings);
    let push_to_talk_enabled = matches!(
        settings.shortcut_mode,
        shared_protocol::ShortcutMode::PushToTalk
    );
    let effective_max_capture_ms = normalize_live_capture_timeout(&settings, max_capture_ms);
    let settings_runtime_state_path =
        write_settings_runtime_state(&loaded_settings, None, None, None)?;
    let overlay_adapter = MirroringOverlayAdapter::new(&settings)?;
    let overlay_state_script_path = overlay_adapter.script_path().to_path_buf();
    let insertion_adapter =
        WindowsTextInsertionAdapter::from_shortcut(&settings.dictation_shortcut)?;
    let mut runtime = HostRuntime::new(
        settings,
        WindowsHotkeyAdapter::default(),
        CpalAudioCaptureAdapter::default(),
        insertion_adapter,
        overlay_adapter,
        WindowsAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    runtime.prepare_speech_engine();
    print_settings_warnings(&loaded_settings.warnings);
    match effective_max_capture_ms {
        Some(timeout_ms) => println!(
            "Listening for one live hotkey trigger. {} If text is selected, the session will route to edit mode automatically when the current shortcut mode allows it. Safety timeout: {} ms.",
            start_stop_description, timeout_ms
        ),
        None => println!(
            "Listening for one live hotkey trigger. {} If text is selected, the session will route to edit mode automatically when the current shortcut mode allows it.",
            start_stop_description
        ),
    }
    if push_to_talk_enabled {
        println!(
            "Current prototype note: when push-to-talk shares the primary shortcut ({}), selected-text capture is deferred until release so the host can still try edit-mode inference after recording stops.",
            primary_shortcut
        );
        if let (Some(requested_timeout_ms), Some(effective_timeout_ms)) =
            (max_capture_ms, effective_max_capture_ms)
        {
            if requested_timeout_ms != effective_timeout_ms {
                println!(
                    "Current prototype note: requested push-to-talk safety timeout {} ms was raised to {} ms so normal speech is less likely to be cut off.",
                    requested_timeout_ms, effective_timeout_ms
                );
            }
        }
    }
    println!("Overlay mirror: {}", overlay_state_script_path.display());
    println!("Settings mirror: {}", settings_runtime_state_path.display());
    println!(
        "Temporary edit note: selected-text edit currently supports a limited executor command set."
    );
    let trigger = runtime.wait_for_trigger()?;
    let summary = runtime
        .run_live_session_for_trigger(trigger, effective_max_capture_ms.map(Duration::from_millis))
        .map_err(|error| error.to_string())?;
    print_summary(&summary, runtime.diagnostics_len());
    Ok(())
}

#[cfg(not(windows))]
fn run_listen_once_live(_max_capture_ms: Option<u64>) -> Result<(), String> {
    Err("--listen-once-live is only available on Windows".to_string())
}

#[cfg(windows)]
fn run_serve_live(
    max_capture_ms: Option<u64>,
    session_limit: Option<u64>,
    startup_context: ProcessStartupContext,
) -> Result<(), String> {
    let live_host_started_at = Instant::now();
    let settings_load_started_at = Instant::now();
    let mut loaded_settings = load_runtime_settings()?;
    let settings_load_ms = elapsed_millis(settings_load_started_at);
    let settings_reloader_init_started_at = Instant::now();
    let mut settings_reloader = RuntimeSettingsReloader::new(&loaded_settings)?;
    let settings_reloader_init_ms = elapsed_millis(settings_reloader_init_started_at);
    let settings = loaded_settings.settings.clone();
    let start_stop_description = live_start_stop_description(&settings);
    let push_to_talk_enabled = matches!(
        settings.shortcut_mode,
        shared_protocol::ShortcutMode::PushToTalk
    );
    let effective_max_capture_ms = normalize_live_capture_timeout(&settings, max_capture_ms);
    let settings_runtime_state_init_started_at = Instant::now();
    let settings_runtime_state_path =
        write_settings_runtime_state(&loaded_settings, None, None, None)?;
    let settings_runtime_state_init_ms = elapsed_millis(settings_runtime_state_init_started_at);
    let overlay_adapter_init_started_at = Instant::now();
    let overlay_adapter = MirroringOverlayAdapter::new(&settings)?;
    let overlay_adapter_init_ms = elapsed_millis(overlay_adapter_init_started_at);
    let overlay_state_script_path = overlay_adapter.script_path().to_path_buf();
    let insertion_adapter_init_started_at = Instant::now();
    let insertion_adapter =
        WindowsTextInsertionAdapter::from_shortcut(&settings.dictation_shortcut)?;
    let insertion_adapter_init_ms = elapsed_millis(insertion_adapter_init_started_at);
    let host_runtime_init_started_at = Instant::now();
    let mut runtime = HostRuntime::new(
        settings,
        WindowsHotkeyAdapter::default(),
        CpalAudioCaptureAdapter::default(),
        insertion_adapter,
        overlay_adapter,
        WindowsAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );
    let host_runtime_init_ms = elapsed_millis(host_runtime_init_started_at);

    let hotkey_register_started_at = Instant::now();
    runtime.bootstrap()?;
    let hotkey_register_ms = elapsed_millis(hotkey_register_started_at);
    print_settings_warnings(&loaded_settings.warnings);
    println!("Overlay mirror: {}", overlay_state_script_path.display());
    println!("Settings mirror: {}", settings_runtime_state_path.display());
    if push_to_talk_enabled {
        println!(
            "Current prototype note: when push-to-talk shares the primary shortcut, selected-text capture is deferred until release so the host can still try edit-mode inference after recording stops."
        );
        if let (Some(requested_timeout_ms), Some(effective_timeout_ms)) =
            (max_capture_ms, effective_max_capture_ms)
        {
            if requested_timeout_ms != effective_timeout_ms {
                println!(
                    "Current prototype note: requested push-to-talk safety timeout {} ms was raised to {} ms so normal speech is less likely to be cut off.",
                    requested_timeout_ms, effective_timeout_ms
                );
            }
        }
    }

    let live_history_source_run_id = live_history_source_run_id();

    if matches!(session_limit, Some(0)) {
        println!("Live host exited immediately because the requested session limit was 0.");
        print_live_host_summary(
            &loaded_settings.settings,
            runtime.session_history(),
            0,
            &[],
            None,
        );
        let live_host_report_path = write_live_host_report(
            &loaded_settings.path,
            loaded_settings.source.clone(),
            loaded_settings.settings.clone(),
            &loaded_settings.warnings,
            effective_max_capture_ms,
            session_limit,
            0,
            &[],
            runtime.session_history(),
            runtime.diagnostics_snapshot(),
            None,
        );
        if let Ok(path) = &live_host_report_path {
            let _ = append_live_host_usage(
                &loaded_settings.path,
                &live_history_source_run_id,
                runtime.session_history(),
                &[],
            )
            .map_err(|error| eprintln!("failed to update local usage ledger: {error}"));
            let _ = append_live_host_history(
                &loaded_settings.path,
                &live_history_source_run_id,
                runtime.session_history(),
                &loaded_settings.settings.history_retention,
            )
            .map_err(|error| eprintln!("failed to update local history ledger: {error}"));
            let _ =
                write_settings_runtime_state(&loaded_settings, None, Some(path.as_path()), None);
        }
        print_live_host_report_location(live_host_report_path);
        return Ok(());
    }

    let audio_preparation = runtime.prepare_audio_capture();
    let speech_preparation = runtime.prepare_speech_engine();

    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_requested_for_handler = Arc::clone(&stop_requested);
    ctrlc::set_handler(move || {
        stop_requested_for_handler.store(true, Ordering::SeqCst);
    })
    .map_err(|error| format!("failed to install Ctrl+C handler for live host: {error}"))?;

    let startup_timings = LiveHostStartupTimings {
        env_load_ms: startup_context.env_load_ms,
        settings_load_ms,
        settings_reloader_init_ms,
        settings_runtime_state_init_ms,
        overlay_adapter_init_ms,
        insertion_adapter_init_ms,
        host_runtime_init_ms,
        hotkey_register_ms,
        audio_prepare_ms: audio_preparation.prepare_ms,
        audio_prepare_succeeded: audio_preparation.prepare_succeeded,
        audio_backend_create_ms: audio_preparation.backend_create_ms,
        audio_device_discovery_ms: audio_preparation.device_discovery_ms,
        speech_backend_prepare_ms: speech_preparation.prepare_ms,
        speech_backend_prepare_succeeded: speech_preparation.prepare_succeeded,
        speech_model_load_ms: speech_preparation.model_load_ms,
        speech_model_warmup_ms: speech_preparation.model_warmup_ms,
        speech_model_warmup_succeeded: speech_preparation.model_warmup_succeeded,
        host_ready_ms: elapsed_millis(live_host_started_at),
        total_process_to_ready_ms: elapsed_millis(startup_context.process_started_at),
    };
    eprintln!(
        "[voiceflow-startup] {}",
        format_live_host_startup_summary(&startup_timings)
    );
    match (effective_max_capture_ms, session_limit) {
        (Some(timeout_ms), Some(limit)) => println!(
            "Live host is ready for the first dictation. {} Safety timeout: {} ms. Session limit: {}.",
            start_stop_description, timeout_ms, limit
        ),
        (Some(timeout_ms), None) => println!(
            "Live host is ready for the first dictation. {} Safety timeout: {} ms. Press Ctrl+C to exit.",
            start_stop_description, timeout_ms
        ),
        (None, Some(limit)) => println!(
            "Live host is ready for the first dictation. {} Session limit: {}.",
            start_stop_description, limit
        ),
        (None, None) => println!(
            "Live host is ready for the first dictation. {} Press Ctrl+C to exit.",
            start_stop_description
        ),
    }

    let mut attempted_sessions = 0_u64;
    let mut failed_sessions = 0_u64;
    let mut last_failure: Option<FailedSessionSummary> = None;
    let mut failure_history = Vec::new();
    loop {
        if stop_requested.load(Ordering::SeqCst) || desktop_lifecycle::stop_requested() {
            println!("Live host received a shutdown request and is shutting down cleanly.");
            break;
        }

        println!("Waiting for the next live session trigger...");
        let trigger = loop {
            if stop_requested.load(Ordering::SeqCst) || desktop_lifecycle::stop_requested() {
                break None;
            }

            let pending_trigger = runtime.wait_for_trigger_timeout(Duration::from_millis(250))?;
            let mut shortcut_rebound = false;
            match settings_reloader.poll() {
                SettingsReloadPoll::Unchanged => {}
                SettingsReloadPoll::Loaded(reloaded) => {
                    eprintln!("[input-host][settings] settings_reload_detected");
                    match runtime.apply_runtime_settings(reloaded.settings.clone()) {
                        Ok(changed_groups) => {
                            shortcut_rebound = changed_groups.contains(&"shortcut");
                            eprintln!(
                                "[input-host][settings] settings_reload_applied changed_setting_groups={}",
                                changed_groups.join(",")
                            );
                            print_settings_warnings(&reloaded.warnings);
                            loaded_settings = reloaded;
                        }
                        Err(error) => {
                            eprintln!(
                                "[input-host][settings] settings_reload_failed reason={error}"
                            );
                        }
                    }
                }
                SettingsReloadPoll::Failed(error) => {
                    eprintln!(
                        "[input-host][settings] settings_reload_detected settings_reload_failed reason={error}"
                    );
                }
            }

            if shortcut_rebound && pending_trigger.is_some() {
                eprintln!(
                    "[input-host][settings] discarded_trigger_from_previous_shortcut_binding"
                );
                continue;
            }
            if let Some(trigger) = pending_trigger {
                break Some(trigger);
            }
        };
        let Some(trigger) = trigger else {
            println!("Live host received Ctrl+C and is shutting down cleanly.");
            break;
        };
        attempted_sessions += 1;
        let session_capture_timeout_ms =
            normalize_live_capture_timeout(runtime.settings(), max_capture_ms);
        match runtime.run_live_session_for_trigger(
            trigger,
            session_capture_timeout_ms.map(Duration::from_millis),
        ) {
            Ok(summary) => {
                if let Some(failure) = failed_session_summary_from_session(&summary) {
                    last_failure = Some(failure.clone());
                    let selected_text_compatibility = parse_selected_text_failure(&failure.message);
                    failure_history.push(LiveHostFailureRecord {
                        attempted_session_index: attempted_sessions,
                        occurred_at_epoch_ms: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|duration| duration.as_millis())
                            .unwrap_or_default(),
                        session_id: failure.session_id,
                        session_kind: failure.session_kind.clone(),
                        failure_phase: failure.failure_phase.clone(),
                        audio_duration_ms: failure.audio_duration_ms,
                        audio_peak_level: failure.audio_peak_level,
                        audio_rms_level: failure.audio_rms_level,
                        error: failure.message.clone(),
                        selected_text_compatibility: selected_text_compatibility,
                        foreground_app: capture_selected_text_failure_foreground_app(
                            &failure.session_kind,
                        ),
                    });
                }
                print_summary(&summary, runtime.diagnostics_len());
                let _ = append_live_host_usage(
                    &loaded_settings.path,
                    &live_history_source_run_id,
                    runtime.session_history(),
                    &[],
                )
                .map_err(|error| {
                    eprintln!("failed to persist usage for completed live session: {error}")
                });
                if matches!(
                    summary.final_state,
                    shared_protocol::SessionState::Committed
                ) {
                    let _ = append_live_host_history(
                        &loaded_settings.path,
                        &live_history_source_run_id,
                        runtime.session_history(),
                        &loaded_settings.settings.history_retention,
                    )
                    .map_err(|error| {
                        eprintln!(
                            "failed to persist local history for completed live session: {error}"
                        )
                    });
                }
            }
            Err(error) => {
                failed_sessions += 1;
                last_failure = Some(error.clone());
                let selected_text_compatibility = parse_selected_text_failure(&error.message);
                failure_history.push(LiveHostFailureRecord {
                    attempted_session_index: attempted_sessions,
                    occurred_at_epoch_ms: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|duration| duration.as_millis())
                        .unwrap_or_default(),
                    session_id: error.session_id,
                    session_kind: error.session_kind.clone(),
                    failure_phase: error.failure_phase.clone(),
                    audio_duration_ms: error.audio_duration_ms,
                    audio_peak_level: error.audio_peak_level,
                    audio_rms_level: error.audio_rms_level,
                    error: error.message.clone(),
                    selected_text_compatibility,
                    foreground_app: capture_selected_text_failure_foreground_app(
                        &error.session_kind,
                    ),
                });
                eprintln!("Live session failed but the host is still running: {error}");
            }
        }

        if let Some(limit) = session_limit {
            if attempted_sessions >= limit {
                println!("Live host reached the requested session limit.");
                break;
            }
        }
    }

    print_live_host_summary(
        &loaded_settings.settings,
        runtime.session_history(),
        failed_sessions,
        &failure_history,
        last_failure.as_ref(),
    );
    let live_host_report_path = write_live_host_report(
        &loaded_settings.path,
        loaded_settings.source.clone(),
        loaded_settings.settings.clone(),
        &loaded_settings.warnings,
        effective_max_capture_ms,
        session_limit,
        failed_sessions,
        &failure_history,
        runtime.session_history(),
        runtime.diagnostics_snapshot(),
        last_failure.as_ref(),
    );
    if let Ok(path) = &live_host_report_path {
        let _ = append_live_host_usage(
            &loaded_settings.path,
            &live_history_source_run_id,
            runtime.session_history(),
            &usage_failures_from_live_host_failures(&failure_history, runtime.session_history()),
        )
        .map_err(|error| eprintln!("failed to update local usage ledger: {error}"));
        let _ = append_live_host_history(
            &loaded_settings.path,
            &live_history_source_run_id,
            runtime.session_history(),
            &loaded_settings.settings.history_retention,
        )
        .map_err(|error| eprintln!("failed to update local history ledger: {error}"));
        let _ = write_settings_runtime_state(&loaded_settings, None, Some(path.as_path()), None);
    }
    print_live_host_report_location(live_host_report_path);

    Ok(())
}

fn format_live_host_startup_summary(timings: &LiveHostStartupTimings) -> String {
    let optional_ms = |value: Option<u32>| {
        value
            .map(|milliseconds| milliseconds.to_string())
            .unwrap_or_else(|| "not_reported".to_string())
    };
    let optional_bool = |value: Option<bool>| {
        value
            .map(|flag| flag.to_string())
            .unwrap_or_else(|| "not_run".to_string())
    };

    format!(
        "live_host_ready=true readiness_event=ready_for_first_dictation env_load_ms={} settings_load_ms={} settings_reloader_init_ms={} settings_runtime_state_init_ms={} overlay_adapter_init_ms={} overlay_webview_init=deferred_to_first_overlay_state overlay_window_visible=false insertion_adapter_init_ms={} host_runtime_init_ms={} provider_credential_init=deferred_to_provider_request hotkey_register_ms={} audio_prepare_ms={} audio_prepare_succeeded={} audio_backend_create_ms={} audio_device_discovery_ms={} microphone_capture_started=false speech_backend_prepare_ms={} speech_backend_prepare_succeeded={} speech_model_load_ms={} speech_model_warmup_ms={} speech_model_warmup_succeeded={} host_ready_ms={} total_process_to_ready_ms={}",
        timings.env_load_ms,
        timings.settings_load_ms,
        timings.settings_reloader_init_ms,
        timings.settings_runtime_state_init_ms,
        timings.overlay_adapter_init_ms,
        timings.insertion_adapter_init_ms,
        timings.host_runtime_init_ms,
        timings.hotkey_register_ms,
        timings.audio_prepare_ms,
        timings.audio_prepare_succeeded,
        timings.audio_backend_create_ms,
        timings.audio_device_discovery_ms,
        timings.speech_backend_prepare_ms,
        timings.speech_backend_prepare_succeeded,
        optional_ms(timings.speech_model_load_ms),
        optional_ms(timings.speech_model_warmup_ms),
        optional_bool(timings.speech_model_warmup_succeeded),
        timings.host_ready_ms,
        timings.total_process_to_ready_ms,
    )
}

fn elapsed_millis(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn live_history_source_run_id() -> String {
    let started_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("live-host-{started_at_epoch_ms}-{}", std::process::id())
}

fn failed_session_summary_from_session(summary: &SessionSummary) -> Option<FailedSessionSummary> {
    if !matches!(summary.final_state, shared_protocol::SessionState::Failed) {
        return None;
    }

    let message = summary
        .commit_failure_reason
        .clone()
        .unwrap_or_else(|| format!("session ended {:?} during commit", summary.final_state));

    Some(FailedSessionSummary {
        session_id: summary.session_id,
        session_kind: summary.session_kind.clone(),
        failure_phase: shared_protocol::SessionFailurePhase::Committing,
        message,
        audio_duration_ms: Some(summary.audio_duration_ms),
        audio_peak_level: Some(summary.audio_peak_level),
        audio_rms_level: Some(summary.audio_rms_level),
    })
}

fn normalize_live_capture_timeout(
    settings: &RuntimeSettings,
    requested_timeout_ms: Option<u64>,
) -> Option<u64> {
    match (settings.shortcut_mode.clone(), requested_timeout_ms) {
        (shared_protocol::ShortcutMode::PushToTalk, Some(timeout_ms)) => {
            Some(timeout_ms.max(MIN_PUSH_TO_TALK_SAFETY_TIMEOUT_MS))
        }
        (_, timeout_ms) => timeout_ms,
    }
}

#[cfg(not(windows))]
fn run_serve_live(
    _max_capture_ms: Option<u64>,
    _session_limit: Option<u64>,
    _startup_context: ProcessStartupContext,
) -> Result<(), String> {
    Err("--serve-live is only available on Windows".to_string())
}

fn run_capture_probe(capture_ms: u64) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let mut runtime = HostRuntime::new(
        settings,
        NoopShortcutAdapter,
        CpalAudioCaptureAdapter::default(),
        StubTextInsertionAdapter,
        StdoutOverlayAdapter,
        NoopAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    print_settings_warnings(&loaded_settings.warnings);
    let captured_audio = runtime.run_audio_capture_probe(Duration::from_millis(capture_ms))?;
    println!();
    println!("Audio capture probe completed.");
    println!("sample_rate_hz: {}", captured_audio.sample_rate_hz);
    println!("channels: {}", captured_audio.channels);
    println!("sample_count: {}", captured_audio.sample_count);
    println!("duration_ms: {}", captured_audio.duration_ms);
    println!("peak_level: {:.3}", captured_audio.peak_level);
    println!("rms_level: {:.3}", captured_audio.rms_level);
    println!("diagnostic_events: {}", runtime.diagnostics_len());
    Ok(())
}

fn run_transcribe_probe(capture_ms: u64) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let mut runtime = HostRuntime::new(
        settings,
        NoopShortcutAdapter,
        CpalAudioCaptureAdapter::default(),
        StubTextInsertionAdapter,
        StdoutOverlayAdapter,
        NoopAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    runtime.prepare_speech_engine();
    print_settings_warnings(&loaded_settings.warnings);
    let summary = runtime
        .run_session_for_trigger_with_capture_window(
            shared_protocol::TriggerMode::GlobalShortcut,
            Some(Duration::from_millis(capture_ms)),
        )
        .map_err(|error| error.to_string())?;
    print_summary(&summary, runtime.diagnostics_len());
    Ok(())
}

fn run_print_settings() -> Result<(), String> {
    let loaded = load_runtime_settings()?;
    let settings_runtime_state_path = write_settings_runtime_state(&loaded, None, None, None)?;
    println!(
        "settings_source: {}",
        match loaded.source {
            RuntimeSettingsSource::Defaults => "defaults",
            RuntimeSettingsSource::File => "file",
        }
    );
    println!("settings_path: {}", loaded.path.display());
    println!("settings_warnings: {}", loaded.warnings.len());
    for warning in &loaded.warnings {
        println!("settings_warning: {}", warning);
    }
    println!();
    let encoded = serde_json::to_string_pretty(&loaded.settings)
        .map_err(|error| format!("failed to encode effective runtime settings: {error}"))?;
    println!("{encoded}");
    println!();
    println!(
        "settings_runtime_state: {}",
        settings_runtime_state_path.display()
    );
    Ok(())
}

fn run_write_default_settings() -> Result<(), String> {
    let path = write_default_settings_template()?;
    let loaded = load_runtime_settings()?;
    let settings_runtime_state_path = write_settings_runtime_state(&loaded, None, None, None)?;
    println!("wrote_default_settings: {}", path.display());
    println!(
        "settings_runtime_state: {}",
        settings_runtime_state_path.display()
    );
    Ok(())
}

fn run_update_settings(update: RuntimeSettingsUpdate) -> Result<(), String> {
    let apply_summary = build_last_settings_apply_summary(&update, "CLI update command");
    let loaded = update_runtime_settings(update)?;
    let settings_runtime_state_path =
        write_settings_runtime_state(&loaded, None, None, Some(&apply_summary))?;
    println!("updated_settings_path: {}", loaded.path.display());
    println!(
        "settings_source: {}",
        match loaded.source {
            RuntimeSettingsSource::Defaults => "defaults",
            RuntimeSettingsSource::File => "file",
        }
    );
    println!("settings_warnings: {}", loaded.warnings.len());
    for warning in &loaded.warnings {
        println!("settings_warning: {}", warning);
    }
    println!();
    let encoded = serde_json::to_string_pretty(&loaded.settings)
        .map_err(|error| format!("failed to encode updated runtime settings: {error}"))?;
    println!("{encoded}");
    println!();
    println!(
        "settings_runtime_state: {}",
        settings_runtime_state_path.display()
    );
    Ok(())
}

fn run_serve_settings(port: u16) -> Result<(), String> {
    let loaded = load_runtime_settings()?;
    let binding = bind_settings_control(port)?;
    let provider_credential_store = production_provider_credential_store();
    let settings_control_url = format!("http://127.0.0.1:{}", binding.port());
    let settings_runtime_state_path = write_settings_runtime_state_with_credentials(
        &loaded,
        Some(&settings_control_url),
        None,
        None,
        provider_credential_store.as_ref(),
    )?;
    print_settings_warnings(&loaded.warnings);
    println!(
        "settings_runtime_state: {}",
        settings_runtime_state_path.display()
    );
    println!("settings_control_url: {settings_control_url}");
    serve_settings_control(binding, provider_credential_store)
}

#[cfg(windows)]
fn run_insert_probe(text: &str) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let insertion_adapter =
        WindowsTextInsertionAdapter::from_shortcut(&settings.dictation_shortcut)?;
    let mut runtime = HostRuntime::with_speech_engine(
        settings,
        SpeechEngine::default(),
        NoopShortcutAdapter,
        NoopAudioCaptureAdapter,
        insertion_adapter,
        StdoutOverlayAdapter,
        WindowsAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    print_settings_warnings(&loaded_settings.warnings);
    println!("Focus the target text field now. Probe text will be inserted in 3 seconds.");
    for remaining in (1..=3).rev() {
        println!("{remaining}...");
        thread::sleep(Duration::from_secs(1));
    }

    let foreground_app = capture_foreground_app_metadata();
    println!(
        "insertion_probe_app_fingerprint: {}",
        foreground_app.app_fingerprint
    );
    if let Some(process_name) = &foreground_app.process_name {
        println!("insertion_probe_process: {}", process_name);
    }
    if let Some(window_class) = &foreground_app.window_class {
        println!("insertion_probe_window_class: {}", window_class);
    }
    if let Some(window_title) = &foreground_app.window_title {
        println!("insertion_probe_window_title: {}", window_title);
    }
    println!(
        "insertion_probe_capture_state: {}",
        foreground_app.capture_state
    );

    let commit_status = runtime.run_text_insertion_probe(text)?;
    println!();
    println!("Text insertion probe completed.");
    println!("commit_status: {:?}", commit_status.status);
    println!("commit_transport: {:?}", commit_status.transport);
    println!("committed_text: {}", text);
    println!("diagnostic_events: {}", runtime.diagnostics_len());
    Ok(())
}

#[cfg(not(windows))]
fn run_insert_probe(_text: &str) -> Result<(), String> {
    Err("--insert-probe is only available on Windows".to_string())
}

#[cfg(windows)]
fn run_sendinput_key_probe(delay_ms: u64) -> Result<(), String> {
    println!("Focus the target text field now. A key down/up will be sent in {delay_ms} ms.");
    thread::sleep(Duration::from_millis(delay_ms));

    let foreground_app = capture_foreground_app_metadata();
    print_insertion_environment_probe_context("sendinput_key_probe", &foreground_app);
    print_process_integrity_probe_context(&foreground_app);

    let result = windows_insertion::sendinput_key_probe();
    print_sendinput_probe_result("sendinput_key_probe", &result);
    Ok(())
}

#[cfg(not(windows))]
fn run_sendinput_key_probe(_delay_ms: u64) -> Result<(), String> {
    Err("--sendinput-key-probe is only available on Windows".to_string())
}

#[cfg(windows)]
fn run_clipboard_only_probe(text: &str) -> Result<(), String> {
    let result = windows_insertion::clipboard_only_probe(text)?;
    println!("clipboard_only_probe_write: success");
    match result.readback_text {
        Some(readback) => {
            println!("clipboard_only_probe_readback: success");
            println!(
                "clipboard_only_probe_readback_matches: {}",
                readback == text
            );
            println!("clipboard_only_probe_readback_text: {readback}");
        }
        None => {
            println!("clipboard_only_probe_readback: no-readable-text");
            println!("clipboard_only_probe_readback_matches: false");
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn run_clipboard_only_probe(_text: &str) -> Result<(), String> {
    Err("--clipboard-only-probe is only available on Windows".to_string())
}

#[cfg(windows)]
fn run_ctrl_v_probe(delay_ms: u64) -> Result<(), String> {
    println!("Focus the target text field now. Ctrl+V will be sent in {delay_ms} ms.");
    thread::sleep(Duration::from_millis(delay_ms));

    let foreground_app = capture_foreground_app_metadata();
    print_insertion_environment_probe_context("ctrl_v_probe", &foreground_app);
    print_process_integrity_probe_context(&foreground_app);

    let result = windows_insertion::sendinput_ctrl_v_probe();
    print_sendinput_probe_result("ctrl_v_probe", &result);
    Ok(())
}

#[cfg(not(windows))]
fn run_ctrl_v_probe(_delay_ms: u64) -> Result<(), String> {
    Err("--ctrl-v-probe is only available on Windows".to_string())
}

fn capture_selected_text_failure_foreground_app(
    session_kind: &shared_protocol::SessionKind,
) -> Option<ForegroundAppMetadata> {
    if matches!(session_kind, shared_protocol::SessionKind::SelectedTextEdit) {
        Some(capture_foreground_app_metadata())
    } else {
        None
    }
}

fn usage_failures_from_live_host_failures(
    failure_history: &[LiveHostFailureRecord],
    _session_history: &[SessionSummary],
) -> Vec<UsageFailureInput> {
    failure_history
        .iter()
        .map(|failure| UsageFailureInput {
            attempted_session_index: failure.attempted_session_index,
            session_id: failure.session_id,
            session_kind: failure.session_kind.clone(),
            foreground_app: failure.foreground_app.clone(),
        })
        .collect()
}

#[cfg(windows)]
fn run_edit_probe(
    instruction: &str,
    app_label: Option<&str>,
    probe_note: Option<&str>,
) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let insertion_adapter =
        WindowsTextInsertionAdapter::from_shortcut(&settings.dictation_shortcut)?;
    let mut runtime = HostRuntime::new(
        settings,
        NoopShortcutAdapter,
        NoopAudioCaptureAdapter,
        insertion_adapter,
        StdoutOverlayAdapter,
        WindowsAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        TemporarySelectedTextExecutor,
    );

    runtime.bootstrap()?;
    print_settings_warnings(&loaded_settings.warnings);
    println!(
        "Focus a supported app, highlight the target text, and keep it selected. The text-seeded edit probe will start in 3 seconds."
    );
    if let Some(app_label) = app_label {
        println!("Compatibility app label: {}", app_label.trim());
    } else {
        println!("Compatibility app label: Unlabeled app");
    }
    if let Some(note) = probe_note {
        println!("Compatibility probe note: {}", note.trim());
    }
    println!(
        "Current executor note: this probe is text-seeded and does not need speech. Supported instructions include rewrite this, make this more concise, make this more professional, summarize this, plus the earlier deterministic formatting and export transforms."
    );
    for remaining in (1..=3).rev() {
        println!("{remaining}...");
        thread::sleep(Duration::from_secs(1));
    }
    let foreground_app = capture_foreground_app_metadata();
    print_foreground_app_probe_context(&foreground_app);

    match runtime.run_text_seeded_selected_text_edit_probe(instruction) {
        Ok(summary) => {
            print_summary(&summary, runtime.diagnostics_len());
            let persistence =
                append_selected_text_validation_record(SelectedTextValidationRecordInput {
                    app_label: app_label.map(|value| value.to_string()),
                    foreground_app: Some(foreground_app.clone()),
                    probe_kind: SelectedTextProbeKind::EditProbe,
                    probe_input: instruction.to_string(),
                    probe_note: probe_note.map(|value| value.to_string()),
                    outcome: if matches!(
                        summary.final_state,
                        shared_protocol::SessionState::Committed
                    ) {
                        SelectedTextProbeOutcome::Passed
                    } else {
                        SelectedTextProbeOutcome::Failed
                    },
                    commit_transport: Some(format!("{:?}", summary.commit_transport)),
                    selected_text_action: summary
                        .selected_text_execution
                        .as_ref()
                        .map(|execution| selected_text_action_label(&execution.action).to_string()),
                    failure_reason: summary.commit_failure_reason.clone(),
                    selected_text_compatibility: summary
                        .commit_failure_reason
                        .as_deref()
                        .and_then(parse_selected_text_failure),
                })?;
            print_selected_text_validation_capture(&persistence, SelectedTextProbeKind::EditProbe);
            Ok(())
        }
        Err(error) => {
            let persistence =
                append_selected_text_validation_record(SelectedTextValidationRecordInput {
                    app_label: app_label.map(|value| value.to_string()),
                    foreground_app: Some(foreground_app),
                    probe_kind: SelectedTextProbeKind::EditProbe,
                    probe_input: instruction.to_string(),
                    probe_note: probe_note.map(|value| value.to_string()),
                    outcome: SelectedTextProbeOutcome::Failed,
                    commit_transport: None,
                    selected_text_action: None,
                    failure_reason: Some(error.message.clone()),
                    selected_text_compatibility: parse_selected_text_failure(&error.message),
                })?;
            print_selected_text_validation_capture(&persistence, SelectedTextProbeKind::EditProbe);
            Err(error.to_string())
        }
    }
}

#[cfg(not(windows))]
fn run_edit_probe(
    _instruction: &str,
    _app_label: Option<&str>,
    _probe_note: Option<&str>,
) -> Result<(), String> {
    Err("--edit-probe is only available on Windows".to_string())
}

#[cfg(windows)]
fn run_edit_transcribe_probe(capture_ms: u64) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let insertion_adapter =
        WindowsTextInsertionAdapter::from_shortcut(&settings.dictation_shortcut)?;
    let mut runtime = HostRuntime::new(
        settings,
        NoopShortcutAdapter,
        CpalAudioCaptureAdapter::default(),
        insertion_adapter,
        StdoutOverlayAdapter,
        WindowsAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    runtime.prepare_speech_engine();
    print_settings_warnings(&loaded_settings.warnings);
    println!(
        "Focus a supported app, highlight the target text, and keep it selected. The live edit session will start in 3 seconds."
    );
    println!(
        "Speak one supported instruction during the recording window, such as uppercase, lowercase, title case, sentence case, snake case, kebab case, camel case, pascal case, constant case, inline code, code block, strip code fence, markdown bold, markdown italic, strip markdown emphasis, wrap in quotes, bullet list, checklist, quote block, numbered list, sort lines, remove duplicate lines, remove empty lines, comma separated, pipe separated, tab separated, semicolon separated, json array, quoted csv, sql in list, yaml list, yaml mapping, markdown table, header block, json object, env block, query string, toml table, shell exports, powershell env, curl headers, python dict, javascript object, ruby hash, sql values rows, strip list markers, sentence per line, markdown heading, remove line breaks, clean up spacing, or make this a prompt."
    );
    println!("Recording window: {} ms.", capture_ms);
    for remaining in (1..=3).rev() {
        println!("{remaining}...");
        thread::sleep(Duration::from_secs(1));
    }

    let summary = runtime
        .run_session_for_trigger_with_capture_window(
            shared_protocol::TriggerMode::EditShortcut,
            Some(Duration::from_millis(capture_ms)),
        )
        .map_err(|error| error.to_string())?;
    print_summary(&summary, runtime.diagnostics_len());
    Ok(())
}

#[cfg(not(windows))]
fn run_edit_transcribe_probe(_capture_ms: u64) -> Result<(), String> {
    Err("--edit-transcribe-probe is only available on Windows".to_string())
}

fn run_intent_probe(request: &str) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let mut runtime = HostRuntime::with_speech_engine(
        settings,
        SpeechEngine::default(),
        NoopShortcutAdapter,
        NoopAudioCaptureAdapter,
        StubTextInsertionAdapter,
        StdoutOverlayAdapter,
        NoopAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    print_settings_warnings(&loaded_settings.warnings);
    let summary = runtime
        .run_session_for_trigger_with_hint(
            shared_protocol::TriggerMode::GlobalShortcut,
            request.to_string(),
        )
        .map_err(|error| error.to_string())?;
    print_summary(&summary, runtime.diagnostics_len());
    Ok(())
}

#[cfg(windows)]
fn run_replace_selection_probe(
    text: &str,
    app_label: Option<&str>,
    probe_note: Option<&str>,
) -> Result<(), String> {
    let loaded_settings = load_effective_settings()?;
    let settings = loaded_settings.settings.clone();
    let insertion_adapter =
        WindowsTextInsertionAdapter::from_shortcut(&settings.dictation_shortcut)?;
    let mut runtime = HostRuntime::new(
        settings,
        NoopShortcutAdapter,
        NoopAudioCaptureAdapter,
        insertion_adapter,
        StdoutOverlayAdapter,
        WindowsAudioFeedbackAdapter,
        selected_text_executor_for_settings(&loaded_settings.settings),
        TemporarySelectedTextExecutor,
        instructed_dictation_executor_for_settings(&loaded_settings.settings),
    );

    runtime.bootstrap()?;
    print_settings_warnings(&loaded_settings.warnings);
    println!(
        "Focus a supported app and highlight the target text now. Replacement will be attempted in 3 seconds."
    );
    if let Some(app_label) = app_label {
        println!("Compatibility app label: {}", app_label.trim());
    } else {
        println!("Compatibility app label: Unlabeled app");
    }
    if let Some(note) = probe_note {
        println!("Compatibility probe note: {}", note.trim());
    }
    println!(
        "Temporary prototype note: this path currently needs the clipboard to be empty or to expose Unicode or ANSI text so the probe can restore your clipboard text afterward."
    );
    for remaining in (1..=3).rev() {
        println!("{remaining}...");
        thread::sleep(Duration::from_secs(1));
    }
    let foreground_app = capture_foreground_app_metadata();
    print_foreground_app_probe_context(&foreground_app);

    match runtime.run_selection_replace_probe(text) {
        Ok(result) => {
            println!();
            println!("Selection replacement probe completed.");
            println!("selected_text: {}", result.selected_text);
            println!("replacement_text: {}", result.replacement_text);
            println!("commit_status: {:?}", result.commit_status);
            println!("commit_transport: {:?}", result.commit_transport);
            println!("diagnostic_events: {}", runtime.diagnostics_len());
            let commit_failure_reason = match &result.commit_status {
                shared_protocol::CommitStatus::Failed(error) => Some(error.clone()),
                shared_protocol::CommitStatus::NotAttempted => {
                    Some("selection replacement was not attempted".to_string())
                }
                shared_protocol::CommitStatus::Success
                | shared_protocol::CommitStatus::TemporaryStub => None,
            };
            let persistence =
                append_selected_text_validation_record(SelectedTextValidationRecordInput {
                    app_label: app_label.map(|value| value.to_string()),
                    foreground_app: Some(foreground_app.clone()),
                    probe_kind: SelectedTextProbeKind::ReplaceSelectionProbe,
                    probe_input: text.to_string(),
                    probe_note: probe_note.map(|value| value.to_string()),
                    outcome: if matches!(
                        result.commit_status,
                        shared_protocol::CommitStatus::Success
                            | shared_protocol::CommitStatus::TemporaryStub
                    ) {
                        SelectedTextProbeOutcome::Passed
                    } else {
                        SelectedTextProbeOutcome::Failed
                    },
                    commit_transport: Some(format!("{:?}", result.commit_transport)),
                    selected_text_action: None,
                    failure_reason: commit_failure_reason.clone(),
                    selected_text_compatibility: commit_failure_reason
                        .as_deref()
                        .and_then(parse_selected_text_failure),
                })?;
            print_selected_text_validation_capture(
                &persistence,
                SelectedTextProbeKind::ReplaceSelectionProbe,
            );
            Ok(())
        }
        Err(error) => {
            let persistence =
                append_selected_text_validation_record(SelectedTextValidationRecordInput {
                    app_label: app_label.map(|value| value.to_string()),
                    foreground_app: Some(foreground_app),
                    probe_kind: SelectedTextProbeKind::ReplaceSelectionProbe,
                    probe_input: text.to_string(),
                    probe_note: probe_note.map(|value| value.to_string()),
                    outcome: SelectedTextProbeOutcome::Failed,
                    commit_transport: None,
                    selected_text_action: None,
                    failure_reason: Some(error.clone()),
                    selected_text_compatibility: parse_selected_text_failure(&error),
                })?;
            print_selected_text_validation_capture(
                &persistence,
                SelectedTextProbeKind::ReplaceSelectionProbe,
            );
            Err(error)
        }
    }
}

#[cfg(not(windows))]
fn run_replace_selection_probe(
    _text: &str,
    _app_label: Option<&str>,
    _probe_note: Option<&str>,
) -> Result<(), String> {
    Err("--replace-selection-probe is only available on Windows".to_string())
}

fn run_print_selected_text_validation_report(app_label: Option<&str>) -> Result<(), String> {
    let Some(persistence) = load_selected_text_validation_report()? else {
        println!("No selected-text validation report has been recorded yet.");
        println!(
            "Run --edit-probe or --replace-selection-probe to start building the compatibility matrix, or --reset-selected-text-validation-baseline to begin a fresh post-fix baseline."
        );
        return Ok(());
    };

    print_selected_text_validation_report(&persistence, app_label);
    Ok(())
}

fn run_reset_selected_text_validation_baseline(label: Option<&str>) -> Result<(), String> {
    let reset = reset_selected_text_validation_baseline(label.map(|value| value.to_string()))?;
    print_selected_text_validation_baseline_reset(&reset);
    Ok(())
}

fn print_selected_text_validation_baseline_reset(reset: &SelectedTextValidationBaselineReset) {
    println!();
    println!("Selected-text validation baseline reset:");
    println!("selected_text_validation_report: {}", reset.path.display());
    println!(
        "selected_text_validation_history_scope: {}",
        reset.report.history_scope.label()
    );
    if let Some(label) = &reset.report.baseline_label {
        println!("selected_text_validation_baseline_label: {}", label);
    }
    if let Some(started_at) = reset.report.baseline_started_at_epoch_ms {
        println!(
            "selected_text_validation_baseline_started_at_epoch_ms: {}",
            started_at
        );
    }
    if let Some(archived_path) = &reset.archived_path {
        println!(
            "selected_text_validation_archived_report: {}",
            archived_path.display()
        );
    } else {
        println!("selected_text_validation_archived_report: none");
    }
}

fn print_foreground_app_probe_context(foreground_app: &ForegroundAppMetadata) {
    println!(
        "selected_text_probe_fingerprint: {}",
        foreground_app.app_fingerprint
    );
    if let Some(process_name) = &foreground_app.process_name {
        println!("selected_text_probe_process: {}", process_name);
    }
    if let Some(window_class) = &foreground_app.window_class {
        println!("selected_text_probe_window_class: {}", window_class);
    }
    if let Some(window_title) = &foreground_app.window_title {
        println!("selected_text_probe_window_title: {}", window_title);
    }
    println!(
        "selected_text_probe_capture_state: {}",
        foreground_app.capture_state
    );
}

fn print_insertion_environment_probe_context(prefix: &str, foreground_app: &ForegroundAppMetadata) {
    println!(
        "{prefix}_app_fingerprint: {}",
        foreground_app.app_fingerprint
    );
    if let Some(process_name) = &foreground_app.process_name {
        println!("{prefix}_process: {}", process_name);
    }
    if let Some(process_id) = foreground_app.process_id {
        println!("{prefix}_process_id: {}", process_id);
    }
    if let Some(window_class) = &foreground_app.window_class {
        println!("{prefix}_window_class: {}", window_class);
    }
    if let Some(window_title) = &foreground_app.window_title {
        println!("{prefix}_window_title: {}", window_title);
    }
    println!("{prefix}_capture_state: {}", foreground_app.capture_state);
}

#[cfg(windows)]
fn print_process_integrity_probe_context(foreground_app: &ForegroundAppMetadata) {
    let current = windows_insertion::current_process_integrity_probe();
    print_process_integrity_probe_result("current_process", &current);

    let Some(process_id) = foreground_app.process_id else {
        println!("target_process_integrity_detail: no foreground process id captured");
        println!("uipi_sendinput_risk: unknown");
        return;
    };

    let target = windows_insertion::target_process_integrity_probe(process_id);
    print_process_integrity_probe_result("target_process", &target);

    match (current.integrity_rid, target.integrity_rid) {
        (Some(current_rid), Some(target_rid)) if target_rid > current_rid => {
            println!("uipi_sendinput_risk: possible-target-integrity-higher-than-current");
        }
        (Some(_), Some(_)) => {
            println!("uipi_sendinput_risk: not-indicated-by-integrity-level");
        }
        _ => {
            println!("uipi_sendinput_risk: unknown");
        }
    }
}

#[cfg(windows)]
fn print_process_integrity_probe_result(prefix: &str, result: &ProcessIntegrityProbeResult) {
    match &result.integrity_label {
        Some(label) => println!("{prefix}_integrity_level: {label}"),
        None => println!("{prefix}_integrity_level: unknown"),
    }
    if let Some(rid) = result.integrity_rid {
        println!("{prefix}_integrity_rid: {rid}");
    }
    match result.elevated {
        Some(elevated) => println!("{prefix}_elevated: {elevated}"),
        None => println!("{prefix}_elevated: unknown"),
    }
    if let Some(detail) = &result.detail {
        println!("{prefix}_integrity_detail: {detail}");
    }
}

#[cfg(windows)]
fn print_sendinput_probe_result(prefix: &str, result: &SendInputProbeResult) {
    println!(
        "{prefix}_sendinput_accepted_events: {}",
        result.accepted_events
    );
    println!(
        "{prefix}_sendinput_expected_events: {}",
        result.expected_events
    );
    println!("{prefix}_sendinput_last_error: {}", result.last_error);
    match &result.last_error_message {
        Some(message) => println!("{prefix}_sendinput_last_error_message: {message}"),
        None => println!("{prefix}_sendinput_last_error_message: none"),
    }
}

fn print_selected_text_validation_capture(
    persistence: &SelectedTextValidationPersistence,
    probe_kind: SelectedTextProbeKind,
) {
    let Some(record) = persistence.report.records.last() else {
        return;
    };
    println!(
        "selected_text_validation_report: {}",
        persistence.path.display()
    );
    if let Some(app_label) = &record.app_label {
        println!("selected_text_validation_app_label: {}", app_label);
    }
    println!(
        "selected_text_validation_fingerprint: {}",
        record.app_fingerprint
    );
    if let Some(process_name) = &record.foreground_app.process_name {
        println!("selected_text_validation_process: {}", process_name);
    }
    println!("selected_text_validation_flow: {}", probe_kind.label());
    if let Some(entry) = find_validation_matrix_entry(
        &persistence.report.matrix,
        &record.app_fingerprint,
        probe_kind,
    ) {
        println!(
            "selected_text_validation_matrix: {} run(s) | {} passed | {} failed | last={} | verdict={}",
            entry.attempted_runs,
            entry.successful_runs,
            entry.failed_runs,
            entry.last_outcome.label(),
            entry.verdict.label()
        );
        if let Some(compatibility) = &entry.dominant_selected_text_compatibility {
            println!("selected_text_validation_compatibility: {}", compatibility);
        }
        if let Some(transport) = &entry.dominant_commit_transport {
            println!("selected_text_validation_transport: {}", transport);
        }
        if let Some(window_class) = &entry.last_window_class {
            println!("selected_text_validation_window_class: {}", window_class);
        }
        if let Some(window_title) = &entry.last_window_title {
            println!("selected_text_validation_window_title: {}", window_title);
        }
        println!(
            "selected_text_validation_capture_state: {}",
            entry.last_capture_state
        );
    }
}

fn print_selected_text_validation_report(
    persistence: &SelectedTextValidationPersistence,
    app_label: Option<&str>,
) {
    let normalized_filter = normalize_validation_filter(app_label);
    let matrix = persistence
        .report
        .matrix
        .iter()
        .filter(|entry| {
            normalized_filter
                .as_ref()
                .map(|filter| validation_entry_matches_filter(entry, filter))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let filtered_records = persistence
        .report
        .records
        .iter()
        .filter(|record| {
            normalized_filter
                .as_ref()
                .map(|filter| validation_record_matches_filter(record, filter))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let recent_records = persistence
        .report
        .records
        .iter()
        .rev()
        .filter(|record| {
            normalized_filter
                .as_ref()
                .map(|filter| validation_record_matches_filter(record, filter))
                .unwrap_or(true)
        })
        .take(8)
        .collect::<Vec<_>>();

    println!();
    println!("Selected-text validation report:");
    println!(
        "selected_text_validation_report: {}",
        persistence.path.display()
    );
    println!(
        "selected_text_validation_note: {}",
        persistence.report.prototype_boundary_note
    );
    println!(
        "selected_text_validation_history_scope: {}",
        persistence.report.history_scope.label()
    );
    if let Some(started_at) = persistence.report.baseline_started_at_epoch_ms {
        println!(
            "selected_text_validation_baseline_started_at_epoch_ms: {}",
            started_at
        );
    }
    if let Some(label) = &persistence.report.baseline_label {
        println!("selected_text_validation_baseline_label: {}", label);
    }
    println!(
        "selected_text_validation_archive_count: {}",
        selected_text_validation_archive_count().unwrap_or_default()
    );
    println!(
        "selected_text_validation_total_runs: {}",
        if normalized_filter.is_some() {
            filtered_records.len() as u64
        } else {
            persistence.report.total_runs
        }
    );
    println!(
        "selected_text_validation_total_apps: {}",
        if normalized_filter.is_some() {
            matrix
                .iter()
                .map(|entry| entry.app_fingerprint.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len() as u64
        } else {
            persistence.report.total_apps
        }
    );
    println!(
        "selected_text_validation_total_fingerprints: {}",
        if normalized_filter.is_some() {
            matrix
                .iter()
                .map(|entry| entry.app_fingerprint.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len() as u64
        } else {
            persistence.report.total_fingerprints
        }
    );
    if let Some(filter) = &normalized_filter {
        println!("selected_text_validation_filter: {}", filter);
    }
    match persistence.report.history_scope {
        SelectedTextValidationHistoryScope::CumulativeHistory => println!(
            "selected_text_validation_scope_note: showing cumulative history; reset the baseline to start a clean post-fix validation cycle."
        ),
        SelectedTextValidationHistoryScope::CurrentBaseline => println!(
            "selected_text_validation_scope_note: showing the current baseline only; earlier runs should live in archived ledgers."
        ),
    }

    if matrix.is_empty() {
        println!("selected_text_validation_matrix: no matching compatibility entries yet.");
    } else {
        for entry in matrix {
            println!(
                "matrix_entry: {} | {} | {} | {} run(s) | {} passed | {} failed | last={}",
                entry.app_fingerprint,
                entry.probe_kind.label(),
                entry.verdict.label(),
                entry.attempted_runs,
                entry.successful_runs,
                entry.failed_runs,
                entry.last_outcome.label()
            );
            if let Some(process_name) = &entry.process_name {
                println!("matrix_process: {}", process_name);
            }
            if let Some(app_label) = &entry.latest_app_label {
                println!("matrix_app_label: {}", app_label);
            }
            if let Some(compatibility) = &entry.dominant_selected_text_compatibility {
                println!("matrix_compatibility: {}", compatibility);
            }
            if let Some(transport) = &entry.dominant_commit_transport {
                println!("matrix_transport: {}", transport);
            }
            if let Some(action) = &entry.last_selected_text_action {
                println!("matrix_last_action: {}", action);
            }
            if let Some(note) = &entry.last_probe_note {
                println!("matrix_last_note: {}", note);
            }
            if let Some(reason) = &entry.last_failure_reason {
                println!("matrix_last_failure: {}", reason);
            }
            if let Some(window_class) = &entry.last_window_class {
                println!("matrix_last_window_class: {}", window_class);
            }
            if let Some(window_title) = &entry.last_window_title {
                println!("matrix_last_window_title: {}", window_title);
            }
            println!("matrix_capture_state: {}", entry.last_capture_state);
        }
    }

    if recent_records.is_empty() {
        println!("selected_text_validation_recent_runs: none");
    } else {
        for record in recent_records {
            println!(
                "recent_run: {} | {} | {} | {}",
                record.app_fingerprint,
                record.probe_kind.label(),
                record.outcome.label(),
                record.probe_input
            );
            if let Some(process_name) = &record.foreground_app.process_name {
                println!("recent_run_process: {}", process_name);
            }
            if let Some(app_label) = &record.app_label {
                println!("recent_run_app_label: {}", app_label);
            }
            if let Some(window_class) = &record.foreground_app.window_class {
                println!("recent_run_window_class: {}", window_class);
            }
            if let Some(window_title) = &record.foreground_app.window_title {
                println!("recent_run_window_title: {}", window_title);
            }
            println!(
                "recent_run_capture_state: {}",
                record.foreground_app.capture_state
            );
            if let Some(compatibility) = &record.selected_text_compatibility {
                println!(
                    "recent_run_compatibility: {}: {}",
                    compatibility.stage, compatibility.reason
                );
            }
            if let Some(reason) = &record.failure_reason {
                println!("recent_run_failure: {}", reason);
            }
        }
    }
}

fn find_validation_matrix_entry<'a>(
    entries: &'a [SelectedTextValidationMatrixEntry],
    app_fingerprint: &str,
    probe_kind: SelectedTextProbeKind,
) -> Option<&'a SelectedTextValidationMatrixEntry> {
    entries.iter().find(|entry| {
        entry.app_fingerprint.eq_ignore_ascii_case(app_fingerprint)
            && entry.probe_kind == probe_kind
    })
}

fn normalize_validation_filter(filter: Option<&str>) -> Option<String> {
    filter.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn validation_entry_matches_filter(
    entry: &SelectedTextValidationMatrixEntry,
    filter: &str,
) -> bool {
    entry.app_fingerprint.eq_ignore_ascii_case(filter)
        || entry
            .process_name
            .as_deref()
            .map(|process_name| process_name.eq_ignore_ascii_case(filter))
            .unwrap_or(false)
        || entry
            .latest_app_label
            .as_deref()
            .map(|app_label| app_label.eq_ignore_ascii_case(filter))
            .unwrap_or(false)
}

fn validation_record_matches_filter(record: &SelectedTextValidationRecord, filter: &str) -> bool {
    record.app_fingerprint.eq_ignore_ascii_case(filter)
        || record
            .foreground_app
            .process_name
            .as_deref()
            .map(|process_name| process_name.eq_ignore_ascii_case(filter))
            .unwrap_or(false)
        || record
            .app_label
            .as_deref()
            .map(|app_label| app_label.eq_ignore_ascii_case(filter))
            .unwrap_or(false)
}

fn print_summary(summary: &SessionSummary, diagnostic_events: usize) {
    println!();
    println!("VoiceFlow milestone 1 scaffold booted successfully.");
    println!("session_id: {}", summary.session_id);
    println!("session_kind: {:?}", summary.session_kind);
    println!("final_state: {:?}", summary.final_state);
    if let Some(latency_ms) = summary.start_feedback_latency_ms {
        println!("start_feedback_latency_ms: {}", latency_ms);
    }
    if let Some(latency_ms) = summary.recording_start_latency_ms {
        println!("recording_start_latency_ms: {}", latency_ms);
    }
    println!(
        "total_session_latency_ms: {}",
        summary.total_session_latency_ms
    );
    println!("audio_duration_ms: {}", summary.audio_duration_ms);
    println!("audio_peak_level: {:.3}", summary.audio_peak_level);
    println!("audio_rms_level: {:.3}", summary.audio_rms_level);
    println!("route: {:?}", summary.route_decision.route_name);
    println!(
        "refine_fast_path_used: {}",
        summary.route_decision.refine_fast_path_used
    );
    println!(
        "refine_fast_path_reason: {}",
        summary.route_decision.refine_fast_path_reason
    );
    println!(
        "cloud_refine_skipped: {}",
        summary.route_decision.cloud_refine_skipped
    );
    if let Some(asr_diagnostics) = &summary.asr_diagnostics {
        println!("asr_backend: {}", asr_diagnostics.backend);
        if let Some(request_id) = asr_diagnostics.worker_request_id {
            println!("asr_worker_request_id: {}", request_id);
        }
        if let Some(model_load_ms) = asr_diagnostics.worker_model_load_ms {
            println!("asr_worker_model_load_ms: {}", model_load_ms);
        }
        println!("asr_worker_restarted: {}", asr_diagnostics.worker_restarted);
        if !asr_diagnostics.metrics.is_empty() {
            let metrics = asr_diagnostics
                .metrics
                .iter()
                .map(|metric| format!("{}={}ms", metric.name, metric.value_ms))
                .collect::<Vec<_>>()
                .join(", ");
            println!("asr_metrics: {}", metrics);
        }
    }
    if let Some(refine_diagnostics) = &summary.refine_diagnostics {
        println!("refine_profile: {}", refine_diagnostics.profile_label);
        println!("refine_model: {}", refine_diagnostics.model_code);
        println!(
            "refine_provider_configured: {}",
            refine_diagnostics.provider_configured
        );
        println!(
            "refine_provider_attempted: {}",
            refine_diagnostics.provider_attempted
        );
        println!(
            "refine_provider_succeeded: {}",
            refine_diagnostics.provider_succeeded
        );
        println!(
            "refine_deterministic_fallback_used: {}",
            refine_diagnostics.deterministic_fallback_used
        );
        if let Some(reason) = &refine_diagnostics.fallback_reason {
            println!("refine_fallback_reason: {}", reason);
        }
    }
    println!("recognized_text: {}", summary.recognized_text);
    println!("committed_text: {}", summary.committed_text);
    println!("degraded_to_asr: {}", summary.degraded_to_asr);
    if let Some(reason) = &summary.fallback_reason {
        println!("fallback_reason: {}", reason);
    }
    println!("commit_transport: {:?}", summary.commit_transport);
    if let Some(reason) = &summary.commit_failure_reason {
        println!("commit_failure_reason: {}", reason);
        if let Some(issue) = parse_selected_text_failure(reason) {
            println!("selected_text_compatibility_stage: {}", issue.stage);
            println!("selected_text_compatibility_reason: {}", issue.reason);
            println!("selected_text_compatibility_guidance: {}", issue.guidance);
        }
    }
    if let Some(execution) = &summary.selected_text_execution {
        println!("selected_text_action: {:?}", execution.action);
        println!(
            "selected_text_action_label: {}",
            selected_text_action_label(&execution.action)
        );
        println!("selected_text_strategy: {}", execution.strategy);
    }
    if let Some(execution) = &summary.wake_phrase_execution {
        println!("wake_phrase_action: {:?}", execution.action);
        println!(
            "wake_phrase_action_label: {}",
            wake_phrase_action_label(&execution.action)
        );
        println!("wake_phrase_strategy: {}", execution.strategy);
    }
    println!("diagnostic_events: {}", diagnostic_events);
}

fn load_effective_settings() -> Result<settings_store::LoadedRuntimeSettings, String> {
    load_runtime_settings()
}

fn print_settings_warnings(warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }

    println!(
        "Runtime settings note: {} setting warning(s) were applied while loading the config.",
        warnings.len()
    );
    for warning in warnings {
        println!("settings_warning: {}", warning);
    }
}

fn primary_shortcut_label(settings: &RuntimeSettings) -> String {
    shared_protocol::shortcut_to_string(&settings.dictation_shortcut)
}

fn live_start_stop_description(settings: &RuntimeSettings) -> String {
    let primary_shortcut = primary_shortcut_label(settings);
    match settings.shortcut_mode {
        shared_protocol::ShortcutMode::Toggle => format!(
            "Press {} to start each voice session and press {} again to stop it.",
            primary_shortcut, primary_shortcut
        ),
        shared_protocol::ShortcutMode::PushToTalk => format!(
            "Hold {} while speaking and release it to stop the voice session.",
            primary_shortcut
        ),
    }
}

fn print_live_host_summary(
    effective_settings: &RuntimeSettings,
    history: &[SessionSummary],
    failed_before_summary_sessions: u64,
    failure_history: &[LiveHostFailureRecord],
    last_failure: Option<&FailedSessionSummary>,
) {
    let outcome_counts = aggregate_session_outcome_counts(history);
    let failed_sessions =
        outcome_counts.failed_with_summary_sessions + failed_before_summary_sessions;
    let attempted_sessions = outcome_counts.successful_sessions + failed_sessions;

    println!();
    println!("Live host summary:");
    println!(
        "successful_sessions: {}",
        outcome_counts.successful_sessions
    );
    println!("failed_sessions: {}", failed_sessions);
    println!(
        "failed_with_summary_sessions: {}",
        outcome_counts.failed_with_summary_sessions
    );
    println!(
        "failed_before_summary_sessions: {}",
        failed_before_summary_sessions
    );
    println!("attempted_sessions: {}", attempted_sessions);

    if history.is_empty() {
        if let Some(error) = last_failure {
            println!("last_failure: {}", error.message);
            println!("last_failure_phase: {:?}", error.failure_phase);
            println!("last_failure_session_id: {}", error.session_id);
            println!("last_failure_session_kind: {:?}", error.session_kind);
            if let Some(duration_ms) = error.audio_duration_ms {
                println!("last_failure_audio_duration_ms: {}", duration_ms);
            }
            if let Some(peak_level) = error.audio_peak_level {
                println!("last_failure_audio_peak_level: {:.3}", peak_level);
            }
            if let Some(rms_level) = error.audio_rms_level {
                println!("last_failure_audio_rms_level: {:.3}", rms_level);
            }
        }
        return;
    }

    let mut dictation_sessions = 0_u64;
    let mut edit_sessions = 0_u64;
    let mut intent_sessions = 0_u64;
    let mut instructed_dictation_sessions = 0_u64;
    let mut total_session_latency_ms = 0_u128;
    let mut total_recording_start_latency_ms = 0_u128;
    let mut recording_start_latency_count = 0_u128;
    let mut total_audio_duration_ms = 0_u128;
    let mut total_audio_rms_milli = 0_u128;
    let commit_transport_counts = aggregate_commit_transport_counts(history);
    let failure_profile_counts = aggregate_failure_profile_counts(failure_history);
    let selected_text_action_mix = derive_selected_text_action_mix(history);
    let wake_phrase_action_mix = derive_wake_phrase_action_mix(history);
    let failure_guidance = derive_failure_guidance(
        failure_profile_counts.recording_failures,
        failure_profile_counts.recognizing_failures,
        failure_profile_counts.executing_failures,
        failure_profile_counts.committing_failures,
        failure_profile_counts.silence_gate_failures,
        failure_profile_counts.no_speech_failures,
    );
    let selected_text_compatibility_summary = derive_selected_text_compatibility_summary(
        failure_history
            .iter()
            .map(|failure| failure.error.as_str())
            .chain(last_failure.iter().map(|failure| failure.message.as_str())),
    );
    let commit_path_summary = derive_commit_path_summary(
        commit_transport_counts.direct_unicode_commits,
        commit_transport_counts.clipboard_fallback_commits,
        commit_transport_counts.selection_replace_commits,
        failure_profile_counts.committing_failures,
    );

    for summary in history {
        match summary.session_kind {
            shared_protocol::SessionKind::Dictation => dictation_sessions += 1,
            shared_protocol::SessionKind::SelectedTextEdit => edit_sessions += 1,
            shared_protocol::SessionKind::WakePhraseIntent => intent_sessions += 1,
            shared_protocol::SessionKind::InstructedDictation => instructed_dictation_sessions += 1,
        }
        total_session_latency_ms += summary.total_session_latency_ms as u128;
        total_audio_duration_ms += summary.audio_duration_ms as u128;
        total_audio_rms_milli += (summary.audio_rms_level * 1000.0).round() as u128;
        if let Some(latency_ms) = summary.recording_start_latency_ms {
            total_recording_start_latency_ms += latency_ms as u128;
            recording_start_latency_count += 1;
        }
    }

    let workload_focus = derive_workload_focus(
        dictation_sessions,
        edit_sessions,
        intent_sessions,
        &selected_text_action_mix,
        &wake_phrase_action_mix,
    );
    let verification_plan = derive_verification_plan(
        failure_guidance.as_ref().map(|item| item.mode.as_str()),
        commit_path_summary
            .as_ref()
            .map(|item| item.outlook.as_str()),
        workload_focus.as_ref().map(|item| item.focus.as_str()),
        effective_settings,
    );

    println!("dictation_sessions: {}", dictation_sessions);
    println!("selected_text_sessions: {}", edit_sessions);
    println!("wake_phrase_sessions: {}", intent_sessions);
    println!(
        "instructed_dictation_sessions: {}",
        instructed_dictation_sessions
    );
    if !selected_text_action_mix.is_empty() {
        println!(
            "selected_text_action_mix: {}",
            format_action_mix_for_terminal(&selected_text_action_mix)
        );
    }
    if !wake_phrase_action_mix.is_empty() {
        println!(
            "wake_phrase_action_mix: {}",
            format_action_mix_for_terminal(&wake_phrase_action_mix)
        );
    }
    println!(
        "direct_unicode_commits: {}",
        commit_transport_counts.direct_unicode_commits
    );
    println!(
        "clipboard_fallback_commits: {}",
        commit_transport_counts.clipboard_fallback_commits
    );
    println!(
        "selection_replace_commits: {}",
        commit_transport_counts.selection_replace_commits
    );
    println!(
        "temporary_stub_commits: {}",
        commit_transport_counts.temporary_stub_commits
    );
    println!(
        "unknown_transport_commits: {}",
        commit_transport_counts.unknown_transport_commits
    );
    if let Some(summary) = &commit_path_summary {
        println!("commit_path_outlook: {}", summary.outlook);
        println!("commit_path_signal: {}", summary.signal);
    }
    if let Some(summary) = &selected_text_compatibility_summary {
        println!("selected_text_compatibility_signal: {}", summary.signal);
        println!("selected_text_compatibility_guidance: {}", summary.guidance);
    }
    if let Some(item) = &failure_guidance {
        println!("dominant_failure_mode: {}", item.mode);
        println!("dominant_failure_guidance: {}", item.guidance);
    }
    if let Some(plan) = &verification_plan {
        println!("verification_focus: {}", plan.focus);
        println!("verification_focus_guidance: {}", plan.guidance);
        println!("verification_title: {}", plan.title);
        println!("verification_scenario: {}", plan.scenario);
        println!("verification_command: {}", plan.command);
        println!("verification_gesture: {}", plan.gesture);
        println!("verification_example: {}", plan.example);
        println!("verification_note: {}", plan.note);
    }
    if let Some(summary) = &workload_focus {
        println!("workload_focus: {}", summary.focus);
        println!("workload_focus_guidance: {}", summary.guidance);
    }
    println!(
        "avg_total_session_latency_ms: {}",
        total_session_latency_ms / history.len() as u128
    );
    println!(
        "avg_audio_duration_ms: {}",
        total_audio_duration_ms / history.len() as u128
    );
    println!(
        "avg_audio_rms_level: {:.3}",
        (total_audio_rms_milli as f32 / history.len() as f32) / 1000.0
    );
    if recording_start_latency_count > 0 {
        println!(
            "avg_recording_start_latency_ms: {}",
            total_recording_start_latency_ms / recording_start_latency_count
        );
    }

    if let Some(last) = history.last() {
        println!("last_session_id: {}", last.session_id);
        println!("last_session_kind: {:?}", last.session_kind);
        println!("last_route: {:?}", last.route_decision.route_name);
        println!("last_commit_status: {:?}", last.commit_status);
        println!("last_commit_transport: {:?}", last.commit_transport);
    }
    if let Some(error) = last_failure {
        println!("last_failure: {}", error.message);
        println!("last_failure_phase: {:?}", error.failure_phase);
        println!("last_failure_session_id: {}", error.session_id);
        println!("last_failure_session_kind: {:?}", error.session_kind);
        if let Some(duration_ms) = error.audio_duration_ms {
            println!("last_failure_audio_duration_ms: {}", duration_ms);
        }
        if let Some(peak_level) = error.audio_peak_level {
            println!("last_failure_audio_peak_level: {:.3}", peak_level);
        }
        if let Some(rms_level) = error.audio_rms_level {
            println!("last_failure_audio_rms_level: {:.3}", rms_level);
        }
        if let Some(issue) = parse_selected_text_failure(&error.message) {
            println!("last_failure_selected_text_stage: {}", issue.stage);
            println!("last_failure_selected_text_reason: {}", issue.reason);
            println!("last_failure_selected_text_guidance: {}", issue.guidance);
        }
        if let Some(foreground_app) = failure_history
            .last()
            .and_then(|failure| failure.foreground_app.as_ref())
        {
            println!(
                "last_failure_app_fingerprint: {}",
                foreground_app.app_fingerprint
            );
            if let Some(process_name) = &foreground_app.process_name {
                println!("last_failure_process: {}", process_name);
            }
            if let Some(window_class) = &foreground_app.window_class {
                println!("last_failure_window_class: {}", window_class);
            }
            if let Some(window_title) = &foreground_app.window_title {
                println!("last_failure_window_title: {}", window_title);
            }
            println!(
                "last_failure_capture_state: {}",
                foreground_app.capture_state
            );
        }
    }
}

fn aggregate_commit_transport_counts(history: &[SessionSummary]) -> CommitTransportCounts {
    let mut counts = CommitTransportCounts::default();
    for summary in history {
        match summary.commit_transport {
            CommitTransport::DirectUnicodeSendInput => counts.direct_unicode_commits += 1,
            CommitTransport::ClipboardPasteFallback => counts.clipboard_fallback_commits += 1,
            CommitTransport::ClipboardSelectionReplace => counts.selection_replace_commits += 1,
            CommitTransport::TemporaryStub => counts.temporary_stub_commits += 1,
            CommitTransport::Unknown => counts.unknown_transport_commits += 1,
        }
    }
    counts
}

fn format_action_mix_for_terminal(items: &[ActionMixEntry]) -> String {
    items
        .iter()
        .map(|item| format!("{} {}", item.label, item.count))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn aggregate_session_outcome_counts(history: &[SessionSummary]) -> SessionOutcomeCounts {
    let mut counts = SessionOutcomeCounts::default();
    for summary in history {
        if matches!(
            summary.final_state,
            shared_protocol::SessionState::Committed
        ) {
            counts.successful_sessions += 1;
        } else {
            counts.failed_with_summary_sessions += 1;
        }
    }
    counts
}

fn aggregate_failure_profile_counts(
    failure_history: &[LiveHostFailureRecord],
) -> FailureProfileCounts {
    let mut counts = FailureProfileCounts::default();
    for failure in failure_history {
        match failure.failure_phase {
            shared_protocol::SessionFailurePhase::Recording => counts.recording_failures += 1,
            shared_protocol::SessionFailurePhase::Recognizing => counts.recognizing_failures += 1,
            shared_protocol::SessionFailurePhase::Executing => counts.executing_failures += 1,
            shared_protocol::SessionFailurePhase::Committing => counts.committing_failures += 1,
            shared_protocol::SessionFailurePhase::Arming => {}
        }

        let normalized_error = failure.error.to_ascii_lowercase();
        if normalized_error.contains("minimum speech activity threshold")
            || normalized_error.contains("looked like silence")
        {
            counts.silence_gate_failures += 1;
        }
        if normalized_error.contains("no speech was recognized") {
            counts.no_speech_failures += 1;
        }
    }

    counts
}

fn write_live_host_report(
    settings_path: &PathBuf,
    settings_source: RuntimeSettingsSource,
    effective_settings: RuntimeSettings,
    settings_warnings: &[String],
    max_capture_ms: Option<u64>,
    session_limit: Option<u64>,
    failed_before_summary_sessions: u64,
    failure_history: &[LiveHostFailureRecord],
    history: &[SessionSummary],
    diagnostics: &[DiagnosticEvent],
    last_failure: Option<&FailedSessionSummary>,
) -> Result<PathBuf, String> {
    let generated_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to compute report timestamp: {error}"))?
        .as_millis();
    let outcome_counts = aggregate_session_outcome_counts(history);
    let failed_sessions =
        outcome_counts.failed_with_summary_sessions + failed_before_summary_sessions;
    let attempted_sessions = outcome_counts.successful_sessions + failed_sessions;
    let commit_transport_counts = aggregate_commit_transport_counts(history);
    let failure_profile_counts = aggregate_failure_profile_counts(failure_history);
    let selected_text_action_mix = derive_selected_text_action_mix(history);
    let wake_phrase_action_mix = derive_wake_phrase_action_mix(history);
    let failure_guidance = derive_failure_guidance(
        failure_profile_counts.recording_failures,
        failure_profile_counts.recognizing_failures,
        failure_profile_counts.executing_failures,
        failure_profile_counts.committing_failures,
        failure_profile_counts.silence_gate_failures,
        failure_profile_counts.no_speech_failures,
    );
    let selected_text_compatibility_summary = derive_selected_text_compatibility_summary(
        failure_history
            .iter()
            .map(|failure| failure.error.as_str())
            .chain(last_failure.iter().map(|failure| failure.message.as_str())),
    );
    let commit_path_summary = derive_commit_path_summary(
        commit_transport_counts.direct_unicode_commits,
        commit_transport_counts.clipboard_fallback_commits,
        commit_transport_counts.selection_replace_commits,
        failure_profile_counts.committing_failures,
    );
    let dictation_sessions = history
        .iter()
        .filter(|summary| {
            matches!(
                summary.session_kind,
                shared_protocol::SessionKind::Dictation
            )
        })
        .count() as u64;
    let selected_text_sessions = history
        .iter()
        .filter(|summary| {
            matches!(
                summary.session_kind,
                shared_protocol::SessionKind::SelectedTextEdit
            )
        })
        .count() as u64;
    let wake_phrase_sessions = history
        .iter()
        .filter(|summary| {
            matches!(
                summary.session_kind,
                shared_protocol::SessionKind::WakePhraseIntent
            )
        })
        .count() as u64;
    let instructed_dictation_sessions = history
        .iter()
        .filter(|summary| {
            matches!(
                summary.session_kind,
                shared_protocol::SessionKind::InstructedDictation
            )
        })
        .count() as u64;
    let workload_focus = derive_workload_focus(
        dictation_sessions,
        selected_text_sessions,
        wake_phrase_sessions,
        &selected_text_action_mix,
        &wake_phrase_action_mix,
    );
    let verification_plan = derive_verification_plan(
        failure_guidance.as_ref().map(|item| item.mode.as_str()),
        commit_path_summary
            .as_ref()
            .map(|item| item.outlook.as_str()),
        workload_focus.as_ref().map(|item| item.focus.as_str()),
        &effective_settings,
    );
    let report = LiveHostReport {
        generated_at_epoch_ms,
        settings_path: settings_path.clone(),
        settings_source,
        effective_settings,
        settings_warnings: settings_warnings.to_vec(),
        max_capture_ms,
        session_limit,
        attempted_sessions,
        successful_sessions: outcome_counts.successful_sessions,
        failed_sessions,
        dictation_sessions,
        selected_text_sessions,
        wake_phrase_sessions,
        instructed_dictation_sessions,
        selected_text_action_mix,
        wake_phrase_action_mix,
        failed_with_summary_sessions: outcome_counts.failed_with_summary_sessions,
        failed_before_summary_sessions,
        recording_failures: failure_profile_counts.recording_failures,
        recognizing_failures: failure_profile_counts.recognizing_failures,
        executing_failures: failure_profile_counts.executing_failures,
        committing_failures: failure_profile_counts.committing_failures,
        silence_gate_failures: failure_profile_counts.silence_gate_failures,
        no_speech_failures: failure_profile_counts.no_speech_failures,
        direct_unicode_commits: commit_transport_counts.direct_unicode_commits,
        clipboard_fallback_commits: commit_transport_counts.clipboard_fallback_commits,
        selection_replace_commits: commit_transport_counts.selection_replace_commits,
        temporary_stub_commits: commit_transport_counts.temporary_stub_commits,
        unknown_transport_commits: commit_transport_counts.unknown_transport_commits,
        selected_text_compatibility_signal: selected_text_compatibility_summary
            .as_ref()
            .map(|item| item.signal.clone()),
        selected_text_compatibility_guidance: selected_text_compatibility_summary
            .as_ref()
            .map(|item| item.guidance.clone()),
        dominant_failure_mode: failure_guidance.as_ref().map(|item| item.mode.clone()),
        dominant_failure_guidance: failure_guidance.as_ref().map(|item| item.guidance.clone()),
        commit_path_outlook: commit_path_summary
            .as_ref()
            .map(|item| item.outlook.clone()),
        commit_path_signal: commit_path_summary.as_ref().map(|item| item.signal.clone()),
        commit_path_guidance: commit_path_summary
            .as_ref()
            .map(|item| item.guidance.clone()),
        verification_focus: verification_plan.as_ref().map(|item| item.focus.clone()),
        verification_focus_guidance: verification_plan.as_ref().map(|item| item.guidance.clone()),
        verification_title: verification_plan.as_ref().map(|item| item.title.clone()),
        verification_scenario: verification_plan.as_ref().map(|item| item.scenario.clone()),
        verification_command: verification_plan.as_ref().map(|item| item.command.clone()),
        verification_gesture: verification_plan.as_ref().map(|item| item.gesture.clone()),
        verification_example: verification_plan.as_ref().map(|item| item.example.clone()),
        verification_note: verification_plan.as_ref().map(|item| item.note.clone()),
        workload_focus: workload_focus.as_ref().map(|item| item.focus.clone()),
        workload_focus_guidance: workload_focus.as_ref().map(|item| item.guidance.clone()),
        last_failure: last_failure.map(|failure| failure.message.clone()),
        last_failure_summary: last_failure.map(|failure| LiveHostFailureSummary {
            session_id: failure.session_id,
            session_kind: failure.session_kind.clone(),
            failure_phase: failure.failure_phase.clone(),
            audio_duration_ms: failure.audio_duration_ms,
            audio_peak_level: failure.audio_peak_level,
            audio_rms_level: failure.audio_rms_level,
            error: failure.message.clone(),
            selected_text_compatibility: parse_selected_text_failure(&failure.message),
            foreground_app: failure_history
                .last()
                .and_then(|record| record.foreground_app.clone()),
        }),
        failure_history: failure_history
            .iter()
            .map(|failure| LiveHostFailureRecord {
                attempted_session_index: failure.attempted_session_index,
                occurred_at_epoch_ms: failure.occurred_at_epoch_ms,
                session_id: failure.session_id,
                session_kind: failure.session_kind.clone(),
                failure_phase: failure.failure_phase.clone(),
                audio_duration_ms: failure.audio_duration_ms,
                audio_peak_level: failure.audio_peak_level,
                audio_rms_level: failure.audio_rms_level,
                error: failure.error.clone(),
                selected_text_compatibility: failure.selected_text_compatibility.clone(),
                foreground_app: failure.foreground_app.clone(),
            })
            .collect(),
        sessions: history.to_vec(),
        diagnostics: diagnostics.to_vec(),
    };

    let _report_dir = ensure_live_host_report_dir()?;
    let report_path = live_host_report_path_for_timestamp(generated_at_epoch_ms);
    let report_json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode live-host report: {error}"))?;
    fs::write(&report_path, report_json).map_err(|error| {
        format!(
            "failed to write live-host report {}: {}",
            report_path.display(),
            error
        )
    })?;
    if let Err(error) = prune_old_live_host_reports() {
        eprintln!("failed to prune stale live host reports: {error}");
    }

    Ok(report_path)
}

fn print_live_host_report_location(result: Result<PathBuf, String>) {
    match result {
        Ok(path) => println!("live_host_report: {}", path.display()),
        Err(error) => eprintln!("failed to write live host report: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_key_store::InMemoryProviderCredentialStore;
    use shared_protocol::{
        AsrProvider, CommitStatus, KeyModifier, ProviderPreset, ProviderSettings, RouteDecision,
        RouteName, SessionFailurePhase, SessionKind, SessionState,
    };

    static PROVIDER_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn startup_summary_marks_exact_readiness_and_contains_only_safe_timing_fields() {
        let summary = format_live_host_startup_summary(&LiveHostStartupTimings {
            env_load_ms: 1,
            settings_load_ms: 2,
            settings_reloader_init_ms: 3,
            settings_runtime_state_init_ms: 4,
            overlay_adapter_init_ms: 5,
            insertion_adapter_init_ms: 6,
            host_runtime_init_ms: 7,
            hotkey_register_ms: 8,
            audio_prepare_ms: 9,
            audio_prepare_succeeded: true,
            audio_backend_create_ms: 10,
            audio_device_discovery_ms: 11,
            speech_backend_prepare_ms: 12,
            speech_backend_prepare_succeeded: true,
            speech_model_load_ms: Some(13),
            speech_model_warmup_ms: Some(14),
            speech_model_warmup_succeeded: Some(true),
            host_ready_ms: 15,
            total_process_to_ready_ms: 16,
        });

        assert!(summary.contains("live_host_ready=true"));
        assert!(summary.contains("readiness_event=ready_for_first_dictation"));
        assert!(summary.contains("overlay_window_visible=false"));
        assert!(summary.contains("microphone_capture_started=false"));
        assert!(summary.contains("speech_model_warmup_succeeded=true"));
        for forbidden in [
            "api_key",
            "authorization",
            "transcript",
            "prompt_text",
            "provider_response",
        ] {
            assert!(
                !summary.to_ascii_lowercase().contains(forbidden),
                "startup summary must not contain {forbidden}"
            );
        }
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = env::var(key).ok();
            unsafe {
                env::set_var(key, value);
            }
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = env::var(key).ok();
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

    fn set_env(key: &str, value: &str) {
        unsafe {
            env::set_var(key, value);
        }
    }

    fn assert_resolved_provider(
        resolved: &ResolvedProvider,
        preset: ProviderPreset,
        base_url: &str,
        model_code: &str,
        timeout_ms: u64,
        api_key: &str,
        supports_dashscope_enable_thinking: bool,
    ) {
        let config = resolved
            .config
            .as_ref()
            .expect("provider should be configured");
        assert_eq!(config.preset, preset);
        assert_eq!(config.base_url, base_url);
        assert_eq!(config.timeout, Duration::from_millis(timeout_ms));
        assert_eq!(config.api_key, api_key);
        assert_eq!(
            config.capabilities.supports_dashscope_enable_thinking,
            supports_dashscope_enable_thinking
        );
        assert_eq!(resolved.profile.model_code, model_code);
        assert_eq!(resolved.metadata.preset, preset);
        assert_eq!(resolved.metadata.base_url.as_deref(), Some(base_url));
        assert_eq!(resolved.metadata.request_timeout_ms, timeout_ms);
        assert_eq!(resolved.metadata.model_code, model_code);
        assert_eq!(
            resolved.metadata.supports_dashscope_enable_thinking,
            supports_dashscope_enable_thinking
        );
        assert_eq!(
            resolved.metadata.key_source,
            speech_engine::ProviderKeySource::ProviderEnv
        );
    }

    fn build_session_summary(final_state: SessionState) -> SessionSummary {
        SessionSummary {
            session_id: 7,
            session_kind: SessionKind::Dictation,
            final_state: final_state.clone(),
            completed_at_epoch_ms: None,
            start_feedback_latency_ms: Some(0),
            recording_start_latency_ms: Some(250),
            total_session_latency_ms: 1_200,
            audio_duration_ms: 900,
            audio_peak_level: 0.125,
            audio_rms_level: 0.01,
            asr_diagnostics: None,
            refine_diagnostics: None,
            recognized_text: "hello world".to_string(),
            committed_text: "Hello world.".to_string(),
            committed_text_count: None,
            route_decision: RouteDecision {
                route_name: RouteName::LocalAsrWithRefine,
                asr_provider: AsrProvider::Local,
                refine_provider: None,
                refinement_applied: true,
                reason: "test route".to_string(),
                refine_fast_path_used: false,
                refine_fast_path_reason: "not_evaluated".to_string(),
                cloud_refine_skipped: false,
                dictation_routing: None,
            },
            degraded_to_asr: false,
            fallback_reason: None,
            commit_status: match final_state {
                SessionState::Committed => CommitStatus::Success,
                SessionState::Failed => {
                    CommitStatus::Failed("target app rejected simulated input".to_string())
                }
                _ => CommitStatus::NotAttempted,
            },
            commit_transport: CommitTransport::DirectUnicodeSendInput,
            commit_failure_reason: match final_state {
                SessionState::Failed => Some("target app rejected simulated input".to_string()),
                _ => None,
            },
            mode_reason: "test mode".to_string(),
            selected_text_execution: None,
            wake_phrase_execution: None,
            instructed_dictation_execution: None,
        }
    }

    #[test]
    fn provider_wrappers_keep_last_known_good_effective_provider_after_reload_failure() {
        let _env_lock = PROVIDER_ENV_LOCK
            .lock()
            .expect("provider env lock should not be poisoned");
        let _key_guard = EnvVarGuard::set("VOICEFLOW_PROVIDER_API_KEY", "key-a");
        let _provider_type_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_TYPE");
        let _provider_base_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_BASE_URL");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let _legacy_base_guard = EnvVarGuard::remove("DASHSCOPE_BASE_URL");
        let _timeout_guard = EnvVarGuard::remove("VOICEFLOW_LLM_REFINE_TIMEOUT_MS");
        let _best_model_guard = EnvVarGuard::remove("VOICEFLOW_MODEL_BEST");

        let startup = RuntimeSettings {
            provider: ProviderSettings {
                preset: ProviderPreset::Bailian,
                base_url: Some("https://startup.example/v1".to_string()),
                active_model: Some("startup-model".to_string()),
                request_timeout_ms: Some(2_000),
            },
            ..RuntimeSettings::default()
        };
        let updated = RuntimeSettings {
            provider: ProviderSettings {
                preset: ProviderPreset::VolcengineArk,
                base_url: Some("https://updated.example/v1".to_string()),
                active_model: Some("updated-model".to_string()),
                request_timeout_ms: Some(3_000),
            },
            ..RuntimeSettings::default()
        };
        let reload_failed = || Err("simulated reload failure".to_string());

        let store = InMemoryProviderCredentialStore::new();
        let fresh_selected = DashScopeSelectedTextEditProvider::from_settings_with_credentials(
            &startup,
            Arc::new(store.clone()),
        );
        let selected = DashScopeSelectedTextEditProvider::from_settings_with_credentials(
            &startup,
            Arc::new(store.clone()),
        );
        let instructed = DashScopeInstructedDictationProvider::from_settings_with_credentials(
            &startup,
            Arc::new(store.clone()),
        );

        assert_eq!(store.lookup_count(), 0);

        set_env("VOICEFLOW_PROVIDER_API_KEY", "key-b");
        assert_resolved_provider(
            &selected.current_provider_from_loader(|| {
                Ok(crate::settings_store::LoadedRuntimeSettings {
                    settings: updated.clone(),
                    path: PathBuf::from("settings.json"),
                    source: crate::settings_store::RuntimeSettingsSource::File,
                    warnings: vec![],
                })
            }),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );
        assert!(store.lookup_count() >= 1);
        assert_resolved_provider(
            &instructed.current_provider_from_loader(|| {
                Ok(crate::settings_store::LoadedRuntimeSettings {
                    settings: updated.clone(),
                    path: PathBuf::from("settings.json"),
                    source: crate::settings_store::RuntimeSettingsSource::File,
                    warnings: vec![],
                })
            }),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );
        assert!(store.lookup_count() >= 2);

        set_env("VOICEFLOW_PROVIDER_API_KEY", "key-c");
        set_env("VOICEFLOW_PROVIDER_TYPE", "custom_openai_compatible");
        set_env("VOICEFLOW_MODEL_BEST", "env-drift-model");
        set_env("VOICEFLOW_LLM_REFINE_TIMEOUT_MS", "4444");

        assert_resolved_provider(
            &selected.current_provider_from_loader(reload_failed),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );
        assert_resolved_provider(
            &instructed
                .current_provider_from_loader(|| Err("simulated reload failure".to_string())),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );

        assert_resolved_provider(
            &fresh_selected
                .current_provider_from_loader(|| Err("simulated reload failure".to_string())),
            ProviderPreset::Bailian,
            "https://startup.example/v1",
            "startup-model",
            2_000,
            "key-a",
            true,
        );

        let invalid_settings = RuntimeSettings {
            provider: ProviderSettings {
                preset: ProviderPreset::CustomOpenAiCompatible,
                base_url: None,
                active_model: Some("invalid-model".to_string()),
                request_timeout_ms: Some(5_000),
            },
            ..RuntimeSettings::default()
        };
        assert_resolved_provider(
            &selected.current_provider_from_loader(|| {
                Ok(crate::settings_store::LoadedRuntimeSettings {
                    settings: invalid_settings.clone(),
                    path: PathBuf::from("settings.json"),
                    source: crate::settings_store::RuntimeSettingsSource::File,
                    warnings: vec![],
                })
            }),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );
        assert_resolved_provider(
            &instructed.current_provider_from_loader(|| {
                Ok(crate::settings_store::LoadedRuntimeSettings {
                    settings: invalid_settings,
                    path: PathBuf::from("settings.json"),
                    source: crate::settings_store::RuntimeSettingsSource::File,
                    warnings: vec![],
                })
            }),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );
    }

    #[test]
    fn failed_summary_sessions_are_promoted_into_structured_failures() {
        let summary = build_session_summary(SessionState::Failed);

        let failure =
            failed_session_summary_from_session(&summary).expect("failed summary should map");

        assert_eq!(failure.session_id, summary.session_id);
        assert_eq!(failure.session_kind, summary.session_kind);
        assert_eq!(failure.failure_phase, SessionFailurePhase::Committing);
        assert_eq!(
            failure.message,
            "target app rejected simulated input".to_string()
        );
        assert_eq!(failure.audio_duration_ms, Some(summary.audio_duration_ms));
    }

    #[test]
    fn aggregate_outcome_counts_distinguish_committed_and_failed_summaries() {
        let history = vec![
            build_session_summary(SessionState::Committed),
            build_session_summary(SessionState::Failed),
        ];

        let counts = aggregate_session_outcome_counts(&history);

        assert_eq!(counts.successful_sessions, 1);
        assert_eq!(counts.failed_with_summary_sessions, 1);
    }

    #[test]
    fn parse_key_modifiers_setting_normalizes_modifier_order() {
        let parsed = parse_key_modifiers_setting("shift+ctrl+alt").expect("modifiers should parse");

        assert_eq!(
            parsed,
            vec![KeyModifier::Control, KeyModifier::Alt, KeyModifier::Shift]
        );
    }

    #[test]
    fn parse_shortcut_key_setting_normalizes_case_and_space_name() {
        assert_eq!(
            parse_shortcut_key_setting("  r ").expect("letter key should parse"),
            "R"
        );
        assert_eq!(
            parse_shortcut_key_setting("space").expect("space should parse"),
            "Space"
        );
    }

    #[test]
    fn parse_settings_update_accepts_language_and_ui_style() {
        let parsed = parse_settings_update(&[
            "system_language=english".to_string(),
            "ui_style=dark".to_string(),
        ])
        .expect("language and UI style update should parse");

        assert_eq!(parsed.system_language, Some(SystemLanguage::English));
        assert_eq!(parsed.ui_style, Some(UiStyle::Dark));

        let parsed = parse_settings_update(&[
            "system_language=chinese".to_string(),
            "ui_style=light".to_string(),
        ])
        .expect("Chinese and light UI style update should parse");

        assert_eq!(parsed.system_language, Some(SystemLanguage::Chinese));
        assert_eq!(parsed.ui_style, Some(UiStyle::Light));
    }

    #[test]
    fn parse_settings_update_rejects_invalid_language_and_ui_style() {
        let language_error = parse_settings_update(&["system_language=klingon".to_string()])
            .expect_err("invalid language should fail");
        assert!(language_error.contains("invalid system_language"));

        let style_error = parse_settings_update(&["ui_style=solarized".to_string()])
            .expect_err("invalid UI style should fail");
        assert!(style_error.contains("invalid ui_style"));
    }

    #[test]
    fn parse_settings_update_accepts_supported_history_retention_values() {
        for (value, expected) in [
            ("latest_100", HistoryRetention::Latest100),
            ("latest_500", HistoryRetention::Latest500),
            ("latest_1000", HistoryRetention::Latest1000),
            ("last_7_days", HistoryRetention::Last7Days),
            ("last_30_days", HistoryRetention::Last30Days),
            ("unlimited", HistoryRetention::Unlimited),
        ] {
            let parsed = parse_settings_update(&[format!("history_retention={value}")])
                .expect("supported history retention should parse");
            assert_eq!(parsed.history_retention, Some(expected));
        }
    }

    #[test]
    fn parse_settings_update_rejects_invalid_history_retention() {
        let error = parse_settings_update(&["history_retention=forever".to_string()])
            .expect_err("unsupported history retention should fail");

        assert!(error.contains("invalid history_retention"));
    }

    #[test]
    fn parse_settings_update_accepts_provider_fields() {
        let parsed = parse_settings_update(&[
            "provider.preset=volcengine_ark".to_string(),
            "provider.base_url=https://ark.example/api/v3".to_string(),
            "provider.active_model=company-model".to_string(),
            "provider.request_timeout_ms=15000".to_string(),
        ])
        .expect("provider update should parse");

        let provider = parsed.provider.expect("provider update should be present");
        assert_eq!(provider.preset, Some(ProviderPreset::VolcengineArk));
        assert_eq!(
            provider.base_url,
            Some(Some("https://ark.example/api/v3".to_string()))
        );
        assert_eq!(
            provider.active_model,
            Some(Some("company-model".to_string()))
        );
        assert_eq!(provider.request_timeout_ms, Some(Some(15_000)));
    }

    #[test]
    fn parse_settings_update_accepts_tencent_hunyuan_provider() {
        for value in ["tencent_hunyuan", "tencent", "hunyuan"] {
            let parsed = parse_settings_update(&[format!("provider.preset={value}")])
                .expect("Tencent Hunyuan provider should parse");
            assert_eq!(
                parsed.provider.and_then(|provider| provider.preset),
                Some(ProviderPreset::TencentHunyuan)
            );
        }
    }

    #[test]
    fn parse_settings_update_normalizes_blank_provider_optionals() {
        let parsed = parse_settings_update(&[
            "provider_base_url= ".to_string(),
            "provider_active_model=".to_string(),
            "provider_request_timeout_ms=".to_string(),
        ])
        .expect("blank provider optionals should parse");

        let provider = parsed.provider.expect("provider update should be present");
        assert_eq!(provider.base_url, Some(None));
        assert_eq!(provider.active_model, Some(None));
        assert_eq!(provider.request_timeout_ms, Some(None));
    }

    #[test]
    fn provider_settings_ui_no_longer_exposes_old_model_profile_placeholder() {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let html_path = crate_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("crate should live under crates/input-host")
            .join("apps")
            .join("settings-ui")
            .join("index.html");
        let html = fs::read_to_string(html_path).expect("settings UI html should be readable");
        let script_path = crate_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("crate should live under crates/input-host")
            .join("apps")
            .join("settings-ui")
            .join("src")
            .join("main.js");
        let script =
            fs::read_to_string(script_path).expect("settings UI script should be readable");

        assert!(!html.contains("Layout preview only"));
        assert!(!html.contains("Qwen Turbo"));
        assert!(!html.contains("Qwen 3.5 Flash"));
        assert!(!html.contains("Fast model"));
        assert!(!html.contains("Balanced model"));
        assert!(!html.contains("Best model"));
        assert!(html.contains("volcengine_ark"));
        assert!(html.contains("provider-active-model"));
        assert!(html.contains("test-provider-connection"));
        assert!(script.contains("/test-provider"));
        assert!(script.contains("settings.testConnectionApplyFirst"));
        assert!(script.contains("dirtyFields.has(\"provider\")"));
    }

    #[test]
    fn parse_selected_text_probe_args_supports_app_label_and_note() {
        let parsed = parse_selected_text_probe_args(
            &[
                "--app-label".to_string(),
                "Notepad".to_string(),
                "--probe-note".to_string(),
                "plain text".to_string(),
                "make".to_string(),
                "this".to_string(),
                "more".to_string(),
                "concise".to_string(),
            ],
            "uppercase",
        )
        .expect("selected-text probe args should parse");

        assert_eq!(parsed.app_label.as_deref(), Some("Notepad"));
        assert_eq!(parsed.probe_note.as_deref(), Some("plain text"));
        assert_eq!(parsed.text, "make this more concise");
    }

    #[test]
    fn parse_delay_ms_probe_args_defaults_and_accepts_explicit_delay() {
        assert_eq!(
            parse_delay_ms_probe_args("--sendinput-key-probe", &[]).expect("default delay"),
            3_000
        );
        assert_eq!(
            parse_delay_ms_probe_args(
                "--sendinput-key-probe",
                &["--delay-ms".to_string(), "125".to_string()],
            )
            .expect("explicit delay"),
            125
        );
    }

    #[test]
    fn parse_required_probe_text_rejects_missing_text() {
        assert!(parse_required_probe_text("--clipboard-only-probe", &[]).is_err());
        assert_eq!(
            parse_required_probe_text(
                "--clipboard-only-probe",
                &["hello".to_string(), "clipboard".to_string()],
            )
            .expect("probe text"),
            "hello clipboard"
        );
    }

    #[test]
    fn parses_local_env_assignments_without_outer_quotes() {
        assert_eq!(
            parse_env_file_assignment(" export DASHSCOPE_API_KEY=\"sk-test\" "),
            Some(("DASHSCOPE_API_KEY".to_string(), "sk-test".to_string()))
        );
        assert_eq!(
            parse_env_file_assignment("DASHSCOPE_API_KEY=sk-test # local secret"),
            Some(("DASHSCOPE_API_KEY".to_string(), "sk-test".to_string()))
        );
        assert_eq!(
            parse_env_file_assignment("DASHSCOPE_API_KEY=\"sk-#-test\" # local secret"),
            Some(("DASHSCOPE_API_KEY".to_string(), "sk-#-test".to_string()))
        );
        assert_eq!(
            parse_env_file_assignment("DASHSCOPE_API_KEY=sk-#-test"),
            Some(("DASHSCOPE_API_KEY".to_string(), "sk-#-test".to_string()))
        );
        assert_eq!(parse_env_file_assignment("# comment"), None);
        assert_eq!(parse_env_file_assignment("1BAD=value"), None);
    }

    #[test]
    fn finds_workspace_root_from_crate_subdirectory() {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = find_env_workspace_root(&crate_dir).expect("workspace root should resolve");

        assert!(root.join("Cargo.toml").exists());
        assert!(root.join("apps").exists());
        assert!(root.join("crates").exists());
    }

    #[test]
    fn local_env_files_do_not_override_existing_values() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after unix epoch")
            .as_nanos();
        let key = format!("VOICEFLOW_TEST_ENV_{unique}");
        let temp_dir = env::temp_dir().join(format!("voiceflow-env-load-test-{unique}"));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let local_path = temp_dir.join(".env.local");
        let fallback_path = temp_dir.join(".env");
        fs::write(&local_path, format!("{key}=local\n")).expect("local env should write");
        fs::write(&fallback_path, format!("{key}=fallback\n")).expect("fallback env should write");

        unsafe {
            env::remove_var(&key);
        }
        let mut report = LocalEnvLoadReport::default();
        load_env_file_if_present(".env.local", &local_path, &mut report);
        load_env_file_if_present(".env", &fallback_path, &mut report);
        assert_eq!(env::var(&key).as_deref(), Ok("local"));
        assert_eq!(
            report.loaded_sources.get(&key).map(String::as_str),
            Some(".env.local")
        );

        unsafe {
            env::set_var(&key, "process");
        }
        let mut report = LocalEnvLoadReport::default();
        load_env_file_if_present(".env.local", &local_path, &mut report);
        assert_eq!(env::var(&key).as_deref(), Ok("process"));
        assert!(report.loaded_sources.get(&key).is_none());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|line| line.contains("because process env already existed"))
        );

        unsafe {
            env::remove_var(&key);
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn local_env_loader_warns_on_duplicate_keys_without_values() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after unix epoch")
            .as_nanos();
        let key = format!("VOICEFLOW_TEST_DUPLICATE_ENV_{unique}");
        let temp_dir = env::temp_dir().join(format!("voiceflow-env-duplicate-test-{unique}"));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let local_path = temp_dir.join(".env.local");
        fs::write(&local_path, format!("{key}=secret-one\n{key}=secret-two\n"))
            .expect("local env should write");

        unsafe {
            env::remove_var(&key);
        }
        let mut report = LocalEnvLoadReport::default();
        load_env_file_if_present(".env.local", &local_path, &mut report);

        assert_eq!(env::var(&key).as_deref(), Ok("secret-one"));
        assert!(
            report
                .warnings
                .iter()
                .any(|line| line.contains(&format!("duplicate key {key}")))
        );
        assert!(
            !report
                .warnings
                .iter()
                .any(|line| line.contains("secret-one") || line.contains("secret-two"))
        );

        unsafe {
            env::remove_var(&key);
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
