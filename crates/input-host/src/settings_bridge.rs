use crate::app_paths::runtime_script_path_for_settings;
use crate::diagnostics_profile::{
    ActionMixEntry, derive_commit_path_summary, derive_failure_guidance, derive_verification_plan,
    derive_workload_focus, selected_text_action_label, wake_phrase_action_label,
};
use crate::history_ledger::{
    HistoryDashboardSummary, load_history_dashboard_summary_for_settings_path,
};
use crate::live_host_reports::{
    discover_latest_live_host_report_path, discover_latest_live_host_report_path_in,
    discover_live_host_report_paths_in,
};
use crate::provider_key_store::{
    MissingProviderCredentialStore, ProviderCredentialStore,
    resolve_provider_config_with_credentials,
};
use crate::selected_text_compatibility::SelectedTextCompatibilityIssue;
use crate::settings_store::{LoadedRuntimeSettings, RuntimeSettingsSource, RuntimeSettingsUpdate};
use crate::usage_ledger::{UsageDashboardSummary, load_usage_dashboard_summary_for_settings_path};
use serde::{Deserialize, Serialize};
use shared_protocol::{
    DiagnosticsVerbosity, HistoryRetention, InstructedDictationExecutionSummary, KeyModifier,
    ProviderPreset, RefinementModelProfile, RefinementQuality, RuntimeSettings,
    SelectedTextExecutionSummary, ShortcutMode, SystemLanguage, UiStyle,
    WakePhraseIntentExecutionSummary, shortcut_to_string,
};
use speech_engine::{
    PromptOverrideStatus, ProviderConfigSource, ProviderCredentialStoreStatus, ProviderKeySource,
    prompt_override_statuses, resolve_provider_config,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
pub struct SettingsUiSnapshot {
    generated_at_epoch_ms: u128,
    settings_path: String,
    settings_source: String,
    settings_control_url: Option<String>,
    settings_warnings: Vec<String>,
    last_settings_apply: Option<LastSettingsApplySummary>,
    latest_live_host_report_path: Option<String>,
    latest_live_host_report_warning: Option<String>,
    latest_live_host_summary: Option<LatestLiveHostSummary>,
    usage_summary: Option<UsageDashboardSummary>,
    history_summary: Option<HistoryDashboardSummary>,
    prompt_overrides: Vec<PromptOverrideStatus>,
    primary_shortcut: String,
    primary_shortcut_modifiers: Vec<KeyModifier>,
    primary_shortcut_key: String,
    shortcut_mode: ShortcutMode,
    refinement_quality: RefinementQuality,
    refinement_profile: RefinementModelProfile,
    provider: ProviderRuntimeState,
    silence_gate_level: u8,
    diagnostics_verbosity: DiagnosticsVerbosity,
    system_language: SystemLanguage,
    ui_style: UiStyle,
    audio_feedback_enabled: bool,
    wake_phrase_enabled: bool,
    wake_phrase_text: String,
    history_retention: HistoryRetention,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderRuntimeState {
    saved_preset: String,
    saved_base_url: Option<String>,
    saved_active_model: Option<String>,
    saved_request_timeout_ms: Option<u64>,
    preset: String,
    preset_source: String,
    display_label: String,
    configured: bool,
    key_present: bool,
    key_source: String,
    stored_credential_present: bool,
    effective_key_present: bool,
    effective_key_source: String,
    credential_store_status: String,
    credential_store_present: bool,
    credential_status_by_preset: BTreeMap<String, ProviderCredentialRuntimeState>,
    env_key_override: bool,
    base_url: String,
    base_url_source: String,
    model_code: String,
    model_source: String,
    request_timeout_ms: u64,
    request_timeout_source: String,
    enable_thinking: Option<bool>,
    enable_thinking_source: String,
    supports_dashscope_enable_thinking: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderCredentialRuntimeState {
    stored_credential_present: bool,
    credential_store_status: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LastSettingsApplySummary {
    pub applied_at_epoch_ms: u128,
    pub source: String,
    pub applied_fields: Vec<String>,
}

pub fn build_last_settings_apply_summary(
    update: &RuntimeSettingsUpdate,
    source: &str,
) -> LastSettingsApplySummary {
    let mut applied_fields = Vec::new();
    if update.shortcut_mode.is_some() {
        applied_fields.push("Shortcut mode".to_string());
    }
    if update.primary_shortcut_modifiers.is_some() || update.primary_shortcut_key.is_some() {
        applied_fields.push("Primary shortcut".to_string());
    }
    if update.refinement_quality.is_some() {
        applied_fields.push("Refine quality".to_string());
    }
    if update.silence_gate_level.is_some() {
        applied_fields.push("Silence gate".to_string());
    }
    if update.diagnostics_verbosity.is_some() {
        applied_fields.push("Diagnostics verbosity".to_string());
    }
    if update.system_language.is_some() {
        applied_fields.push("System language".to_string());
    }
    if update.ui_style.is_some() {
        applied_fields.push("UI style".to_string());
    }
    if update.audio_feedback_enabled.is_some() {
        applied_fields.push("Audio cues".to_string());
    }
    if update.wake_phrase_enabled.is_some() {
        applied_fields.push("Wake phrase enabled".to_string());
    }
    if update.wake_phrase_phrase.is_some() {
        applied_fields.push("Wake phrase text".to_string());
    }
    if update.history_retention.is_some() {
        applied_fields.push("History retention".to_string());
    }
    if let Some(provider) = update.provider.as_ref() {
        if provider.preset.is_some() {
            applied_fields.push("Provider preset".to_string());
        }
        if provider.base_url.is_some() {
            applied_fields.push("Provider base URL".to_string());
        }
        if provider.active_model.is_some() {
            applied_fields.push("Provider active model".to_string());
        }
        if provider.request_timeout_ms.is_some() {
            applied_fields.push("Provider timeout".to_string());
        }
    }

    LastSettingsApplySummary {
        applied_at_epoch_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_millis(),
        source: source.to_string(),
        applied_fields,
    }
}

#[derive(Debug, Serialize)]
struct LatestLiveHostSummary {
    report_generated_at_epoch_ms: u128,
    effective_primary_shortcut: String,
    effective_primary_shortcut_modifiers: Vec<KeyModifier>,
    effective_primary_shortcut_key: String,
    effective_shortcut_mode: ShortcutMode,
    effective_refinement_quality: RefinementQuality,
    effective_refinement_profile: RefinementModelProfile,
    effective_silence_gate_level: u8,
    effective_diagnostics_verbosity: DiagnosticsVerbosity,
    effective_system_language: SystemLanguage,
    effective_ui_style: UiStyle,
    effective_audio_feedback_enabled: bool,
    effective_wake_phrase_enabled: bool,
    effective_wake_phrase_text: String,
    matches_saved_settings: bool,
    drift_fields: Vec<String>,
    attempted_sessions: u64,
    successful_sessions: u64,
    failed_sessions: u64,
    dictation_sessions: u64,
    selected_text_sessions: u64,
    wake_phrase_sessions: u64,
    #[serde(default)]
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
    selected_text_compatibility_signal: Option<String>,
    selected_text_compatibility_guidance: Option<String>,
    local_asr_only_sessions: u64,
    local_asr_with_refine_sessions: u64,
    cloud_asr_with_refine_sessions: u64,
    avg_total_session_latency_ms: Option<u64>,
    avg_recording_start_latency_ms: Option<u64>,
    avg_audio_duration_ms: Option<u64>,
    avg_audio_rms_level: Option<f32>,
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
    tuning_recommendations: Vec<TuningRecommendation>,
    last_failure: Option<String>,
    last_failure_summary: Option<LatestLiveHostFailureSummary>,
    latest_successful_session: Option<LatestSuccessfulSessionSummary>,
    recent_failures: Vec<RecentFailureSummary>,
}

#[derive(Debug, Serialize)]
struct TuningRecommendation {
    panel: String,
    setting_key: String,
    live_value: String,
    saved_value: String,
    target_value: String,
    saved_already_matches_target: bool,
    restart_required: bool,
    label: String,
    strength: String,
    evidence_count: u64,
    evidence_summary: String,
    rationale: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LatestLiveHostFailureSummary {
    session_id: u64,
    session_kind: String,
    failure_phase: String,
    audio_duration_ms: Option<u64>,
    audio_peak_level: Option<f32>,
    audio_rms_level: Option<f32>,
    error: String,
    selected_text_compatibility: Option<SelectedTextCompatibilityIssue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LatestSuccessfulSessionSummary {
    session_id: u64,
    session_kind: String,
    route_decision: LatestSuccessfulRouteDecision,
    recognized_text: String,
    committed_text: String,
    #[serde(default)]
    degraded_to_asr: bool,
    fallback_reason: Option<String>,
    commit_transport: String,
    selected_text_execution: Option<SelectedTextExecutionSummary>,
    wake_phrase_execution: Option<WakePhraseIntentExecutionSummary>,
    #[serde(default)]
    instructed_dictation_execution: Option<InstructedDictationExecutionSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LatestSuccessfulRouteDecision {
    route_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecentFailureSummary {
    attempted_session_index: u64,
    session_id: u64,
    session_kind: String,
    failure_phase: String,
    error: String,
    selected_text_compatibility: Option<SelectedTextCompatibilityIssue>,
}

#[derive(Debug, Deserialize)]
struct LiveHostReportSummarySnapshot {
    generated_at_epoch_ms: u128,
    effective_settings: RuntimeSettings,
    attempted_sessions: u64,
    successful_sessions: u64,
    failed_sessions: u64,
    dictation_sessions: u64,
    selected_text_sessions: u64,
    wake_phrase_sessions: u64,
    #[serde(default)]
    instructed_dictation_sessions: u64,
    selected_text_action_mix: Option<Vec<ActionMixEntry>>,
    wake_phrase_action_mix: Option<Vec<ActionMixEntry>>,
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
    last_failure_summary: Option<LatestLiveHostFailureSummary>,
    sessions: Vec<LiveHostSessionSnapshot>,
    failure_history: Vec<LiveHostFailureHistorySnapshot>,
}

#[derive(Debug, Deserialize)]
struct LiveHostSessionSnapshot {
    session_id: u64,
    session_kind: String,
    final_state: String,
    recording_start_latency_ms: Option<u64>,
    total_session_latency_ms: u64,
    audio_duration_ms: u64,
    audio_rms_level: f32,
    route_decision: LiveHostSessionRouteDecision,
    recognized_text: String,
    committed_text: String,
    #[serde(default)]
    degraded_to_asr: bool,
    fallback_reason: Option<String>,
    commit_transport: String,
    selected_text_execution: Option<SelectedTextExecutionSummary>,
    wake_phrase_execution: Option<WakePhraseIntentExecutionSummary>,
    #[serde(default)]
    instructed_dictation_execution: Option<InstructedDictationExecutionSummary>,
}

#[derive(Debug, Deserialize)]
struct LiveHostSessionRouteDecision {
    route_name: String,
}

#[derive(Debug, Deserialize)]
struct LiveHostFailureHistorySnapshot {
    attempted_session_index: u64,
    session_id: u64,
    session_kind: String,
    failure_phase: String,
    error: String,
    selected_text_compatibility: Option<SelectedTextCompatibilityIssue>,
}

pub fn write_settings_runtime_state(
    loaded_settings: &LoadedRuntimeSettings,
    settings_control_url: Option<&str>,
    latest_live_host_report_path: Option<&Path>,
    last_settings_apply: Option<&LastSettingsApplySummary>,
) -> Result<PathBuf, String> {
    write_settings_runtime_state_with_credentials(
        loaded_settings,
        settings_control_url,
        latest_live_host_report_path,
        last_settings_apply,
        &MissingProviderCredentialStore,
    )
}

pub fn write_settings_runtime_state_with_credentials(
    loaded_settings: &LoadedRuntimeSettings,
    settings_control_url: Option<&str>,
    latest_live_host_report_path: Option<&Path>,
    last_settings_apply: Option<&LastSettingsApplySummary>,
    provider_credential_store: &dyn ProviderCredentialStore,
) -> Result<PathBuf, String> {
    let script_path = runtime_script_path_for_settings(&loaded_settings.path, "settings-state.js");
    write_settings_runtime_state_to_path(
        &script_path,
        loaded_settings,
        settings_control_url,
        latest_live_host_report_path,
        last_settings_apply,
        provider_credential_store,
    )
}

fn write_settings_runtime_state_to_path(
    script_path: &Path,
    loaded_settings: &LoadedRuntimeSettings,
    settings_control_url: Option<&str>,
    latest_live_host_report_path: Option<&Path>,
    last_settings_apply: Option<&LastSettingsApplySummary>,
    provider_credential_store: &dyn ProviderCredentialStore,
) -> Result<PathBuf, String> {
    let encoded = encode_settings_runtime_state_json_with_credentials(
        loaded_settings,
        settings_control_url,
        latest_live_host_report_path,
        last_settings_apply,
        provider_credential_store,
    )?;
    let script_body = format!("window.__VOICEFLOW_SETTINGS_RUNTIME__ = {};\n", encoded);

    if let Some(parent) = script_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create settings runtime state directory {}: {}",
                parent.display(),
                error
            )
        })?;
    }

    fs::write(&script_path, script_body).map_err(|error| {
        format!(
            "failed to write settings runtime state script {}: {}",
            script_path.display(),
            error
        )
    })?;

    Ok(script_path.to_path_buf())
}

#[cfg(test)]
pub fn encode_settings_runtime_state_json(
    loaded_settings: &LoadedRuntimeSettings,
    settings_control_url: Option<&str>,
    latest_live_host_report_path: Option<&Path>,
    last_settings_apply: Option<&LastSettingsApplySummary>,
) -> Result<String, String> {
    encode_settings_runtime_state_json_with_credentials(
        loaded_settings,
        settings_control_url,
        latest_live_host_report_path,
        last_settings_apply,
        &MissingProviderCredentialStore,
    )
}

pub fn encode_settings_runtime_state_json_with_credentials(
    loaded_settings: &LoadedRuntimeSettings,
    settings_control_url: Option<&str>,
    latest_live_host_report_path: Option<&Path>,
    last_settings_apply: Option<&LastSettingsApplySummary>,
    provider_credential_store: &dyn ProviderCredentialStore,
) -> Result<String, String> {
    let snapshot = build_settings_ui_snapshot_with_credentials(
        loaded_settings,
        settings_control_url,
        latest_live_host_report_path,
        last_settings_apply,
        provider_credential_store,
    )?;
    serde_json::to_string_pretty(&snapshot)
        .map_err(|error| format!("failed to encode settings UI state: {error}"))
}

#[cfg(test)]
pub fn build_settings_ui_snapshot(
    loaded_settings: &LoadedRuntimeSettings,
    settings_control_url: Option<&str>,
    latest_live_host_report_path: Option<&Path>,
    last_settings_apply: Option<&LastSettingsApplySummary>,
) -> Result<SettingsUiSnapshot, String> {
    build_settings_ui_snapshot_with_credentials(
        loaded_settings,
        settings_control_url,
        latest_live_host_report_path,
        last_settings_apply,
        &MissingProviderCredentialStore,
    )
}

pub fn build_settings_ui_snapshot_with_credentials(
    loaded_settings: &LoadedRuntimeSettings,
    settings_control_url: Option<&str>,
    latest_live_host_report_path: Option<&Path>,
    last_settings_apply: Option<&LastSettingsApplySummary>,
    provider_credential_store: &dyn ProviderCredentialStore,
) -> Result<SettingsUiSnapshot, String> {
    let resolved_live_host_report_path =
        resolve_latest_live_host_report_path(latest_live_host_report_path)?;
    let (usable_live_host_report_path, latest_live_host_summary, latest_live_host_report_warning) =
        read_latest_live_host_summary_with_warning(
            resolved_live_host_report_path.as_deref(),
            &loaded_settings.settings,
        );

    let resolved_provider = resolve_provider_config_with_credentials(
        &loaded_settings.settings,
        provider_credential_store,
    );
    let provider_runtime_state = provider_runtime_state_from_metadata(
        &resolved_provider.metadata,
        &loaded_settings.settings.provider,
        provider_credential_store,
    );

    Ok(SettingsUiSnapshot {
        generated_at_epoch_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("failed to compute settings UI state timestamp: {error}"))?
            .as_millis(),
        settings_path: loaded_settings.path.display().to_string(),
        settings_source: settings_source_label(&loaded_settings.source).to_string(),
        settings_control_url: settings_control_url.map(str::to_string),
        settings_warnings: loaded_settings.warnings.clone(),
        last_settings_apply: last_settings_apply.cloned(),
        latest_live_host_report_path: usable_live_host_report_path
            .as_deref()
            .map(|path| path.display().to_string()),
        latest_live_host_report_warning,
        latest_live_host_summary,
        usage_summary: load_usage_dashboard_summary_for_settings_path(&loaded_settings.path)
            .unwrap_or(None),
        history_summary: load_history_dashboard_summary_for_settings_path(
            &loaded_settings.path,
            &loaded_settings.settings.history_retention,
        )
        .unwrap_or(None),
        prompt_overrides: prompt_override_statuses(),
        primary_shortcut: shortcut_to_string(&loaded_settings.settings.dictation_shortcut),
        primary_shortcut_modifiers: loaded_settings
            .settings
            .dictation_shortcut
            .modifiers
            .clone(),
        primary_shortcut_key: loaded_settings.settings.dictation_shortcut.key.clone(),
        shortcut_mode: loaded_settings.settings.shortcut_mode.clone(),
        refinement_quality: loaded_settings.settings.refinement_quality.clone(),
        refinement_profile: resolved_provider.profile,
        provider: provider_runtime_state,
        silence_gate_level: loaded_settings.settings.silence_gate_level,
        diagnostics_verbosity: loaded_settings.settings.diagnostics_verbosity.clone(),
        system_language: loaded_settings.settings.system_language.clone(),
        ui_style: loaded_settings.settings.ui_style.clone(),
        audio_feedback_enabled: loaded_settings.settings.audio_feedback_enabled,
        wake_phrase_enabled: loaded_settings.settings.wake_phrase.enabled,
        wake_phrase_text: loaded_settings.settings.wake_phrase.phrase.clone(),
        history_retention: loaded_settings.settings.history_retention.clone(),
    })
}

fn provider_runtime_state_from_metadata(
    metadata: &speech_engine::ProviderMetadata,
    saved_provider: &shared_protocol::ProviderSettings,
    provider_credential_store: &dyn ProviderCredentialStore,
) -> ProviderRuntimeState {
    ProviderRuntimeState {
        saved_preset: provider_preset_label(&saved_provider.preset).to_string(),
        saved_base_url: saved_provider.base_url.clone(),
        saved_active_model: saved_provider.active_model.clone(),
        saved_request_timeout_ms: saved_provider.request_timeout_ms,
        preset: provider_preset_label(&metadata.preset).to_string(),
        preset_source: provider_source_label(metadata.preset_source).to_string(),
        display_label: metadata.display_label.clone(),
        configured: metadata.configured,
        key_present: metadata.key_present,
        key_source: provider_key_source_label(metadata.key_source).to_string(),
        stored_credential_present: metadata.credential_store_status
            == ProviderCredentialStoreStatus::Present,
        effective_key_present: metadata.key_present,
        effective_key_source: effective_provider_key_source_label(
            metadata.key_source,
            metadata.key_present,
        )
        .to_string(),
        credential_store_status: credential_store_status_label(metadata.credential_store_status)
            .to_string(),
        credential_store_present: metadata.credential_store_status
            == ProviderCredentialStoreStatus::Present,
        credential_status_by_preset: credential_status_by_preset(provider_credential_store),
        env_key_override: matches!(
            metadata.key_source,
            ProviderKeySource::ProviderEnv | ProviderKeySource::LegacyDashscopeEnv
        ),
        base_url: metadata.base_url.clone().unwrap_or_default(),
        base_url_source: provider_source_label(metadata.base_url_source).to_string(),
        model_code: metadata.model_code.clone(),
        model_source: provider_source_label(metadata.model_source).to_string(),
        request_timeout_ms: metadata.request_timeout_ms,
        request_timeout_source: provider_source_label(metadata.request_timeout_source).to_string(),
        enable_thinking: metadata.enable_thinking,
        enable_thinking_source: provider_source_label(metadata.enable_thinking_source).to_string(),
        supports_dashscope_enable_thinking: metadata.supports_dashscope_enable_thinking,
    }
}

fn credential_status_by_preset(
    provider_credential_store: &dyn ProviderCredentialStore,
) -> BTreeMap<String, ProviderCredentialRuntimeState> {
    [
        ProviderPreset::Bailian,
        ProviderPreset::VolcengineArk,
        ProviderPreset::TencentHunyuan,
        ProviderPreset::CustomOpenAiCompatible,
    ]
    .into_iter()
    .map(|preset| {
        let status = credential_store_status_from_lookup(provider_credential_store.get(&preset));
        (
            provider_preset_label(&preset).to_string(),
            ProviderCredentialRuntimeState {
                stored_credential_present: status == ProviderCredentialStoreStatus::Present,
                credential_store_status: credential_store_status_label(status).to_string(),
            },
        )
    })
    .collect()
}

fn credential_store_status_from_lookup(
    lookup: speech_engine::ProviderCredentialLookup,
) -> ProviderCredentialStoreStatus {
    match lookup {
        speech_engine::ProviderCredentialLookup::Found(value) if !value.trim().is_empty() => {
            ProviderCredentialStoreStatus::Present
        }
        speech_engine::ProviderCredentialLookup::Found(_)
        | speech_engine::ProviderCredentialLookup::Missing => {
            ProviderCredentialStoreStatus::Missing
        }
        speech_engine::ProviderCredentialLookup::StoreError => ProviderCredentialStoreStatus::Error,
    }
}

fn provider_preset_label(preset: &ProviderPreset) -> &'static str {
    match preset {
        ProviderPreset::Bailian => "bailian",
        ProviderPreset::VolcengineArk => "volcengine_ark",
        ProviderPreset::TencentHunyuan => "tencent_hunyuan",
        ProviderPreset::CustomOpenAiCompatible => "custom_openai_compatible",
    }
}

fn provider_source_label(source: ProviderConfigSource) -> &'static str {
    match source {
        ProviderConfigSource::Env => "env",
        ProviderConfigSource::Settings => "settings",
        ProviderConfigSource::BuiltIn => "built_in",
        ProviderConfigSource::Missing => "missing",
    }
}

fn provider_key_source_label(source: ProviderKeySource) -> &'static str {
    match source {
        ProviderKeySource::ProviderEnv => "provider_env",
        ProviderKeySource::LegacyDashscopeEnv => "legacy_dashscope_env",
        ProviderKeySource::CredentialStore => "credential_store",
        ProviderKeySource::CredentialStoreError => "store_error_last_known_good",
        ProviderKeySource::Missing => "missing",
    }
}

fn effective_provider_key_source_label(
    source: ProviderKeySource,
    key_present: bool,
) -> &'static str {
    match source {
        ProviderKeySource::CredentialStoreError if !key_present => "missing",
        _ => provider_key_source_label(source),
    }
}

fn credential_store_status_label(source: ProviderCredentialStoreStatus) -> &'static str {
    match source {
        ProviderCredentialStoreStatus::Present => "available",
        ProviderCredentialStoreStatus::Missing => "missing",
        ProviderCredentialStoreStatus::Error => "error",
    }
}

fn read_latest_live_host_summary_with_warning(
    report_path: Option<&Path>,
    saved_settings: &shared_protocol::RuntimeSettings,
) -> (
    Option<PathBuf>,
    Option<LatestLiveHostSummary>,
    Option<String>,
) {
    let Some(report_path) = report_path else {
        return (None, None, None);
    };

    let candidates = live_host_report_candidates(report_path);
    let mut parse_failures = Vec::new();

    for candidate in candidates {
        match read_live_host_summary(candidate.as_path(), saved_settings) {
            Ok(Some(summary)) => {
                let warning = if parse_failures.is_empty() {
                    None
                } else {
                    Some(format!(
                        "Skipped {} newer mirrored diagnostic report{} before using {}.",
                        parse_failures.len(),
                        if parse_failures.len() == 1 { "" } else { "s" },
                        candidate.display()
                    ))
                };
                return (Some(candidate), Some(summary), warning);
            }
            Ok(None) => {}
            Err(error) => parse_failures.push(format!("{} ({error})", candidate.display())),
        }
    }

    if parse_failures.is_empty() {
        (Some(report_path.to_path_buf()), None, None)
    } else {
        (
            Some(report_path.to_path_buf()),
            None,
            Some(format!(
                "Skipped mirrored diagnostics because no parseable live-host report was available. Tried: {}",
                parse_failures.join(" | ")
            )),
        )
    }
}

fn live_host_report_candidates(report_path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![report_path.to_path_buf()];
    if !looks_like_live_host_report_path(report_path) {
        return candidates;
    }

    let Some(report_dir) = report_path.parent() else {
        return candidates;
    };

    if let Ok(discovered) = discover_live_host_report_paths_in(report_dir) {
        for candidate in discovered {
            if candidates.iter().all(|existing| existing != &candidate) {
                candidates.push(candidate);
            }
        }
    }

    candidates
}

fn read_live_host_summary(
    report_path: &Path,
    saved_settings: &shared_protocol::RuntimeSettings,
) -> Result<Option<LatestLiveHostSummary>, String> {
    if !report_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(report_path).map_err(|error| {
        format!(
            "failed to read live-host report {} for settings UI state: {}",
            report_path.display(),
            error
        )
    })?;
    let decoded: LiveHostReportSummarySnapshot = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse live-host report {} for settings UI state: {}",
            report_path.display(),
            error
        )
    })?;

    let local_asr_only_sessions = decoded
        .sessions
        .iter()
        .filter(|session| session.route_decision.route_name == "LocalAsrOnly")
        .count() as u64;
    let local_asr_with_refine_sessions = decoded
        .sessions
        .iter()
        .filter(|session| session.route_decision.route_name == "LocalAsrWithRefine")
        .count() as u64;
    let cloud_asr_with_refine_sessions = decoded
        .sessions
        .iter()
        .filter(|session| session.route_decision.route_name == "CloudAsrWithRefine")
        .count() as u64;
    let selected_text_action_mix = decoded
        .selected_text_action_mix
        .clone()
        .unwrap_or_else(|| derive_selected_text_action_mix(&decoded.sessions));
    let wake_phrase_action_mix = decoded
        .wake_phrase_action_mix
        .clone()
        .unwrap_or_else(|| derive_wake_phrase_action_mix(&decoded.sessions));

    let avg_total_session_latency_ms = if decoded.sessions.is_empty() {
        None
    } else {
        Some(
            decoded
                .sessions
                .iter()
                .map(|session| session.total_session_latency_ms as u128)
                .sum::<u128>()
                .checked_div(decoded.sessions.len() as u128)
                .unwrap_or(0) as u64,
        )
    };
    let recording_latency_values = decoded
        .sessions
        .iter()
        .filter_map(|session| session.recording_start_latency_ms)
        .collect::<Vec<_>>();
    let avg_recording_start_latency_ms = if recording_latency_values.is_empty() {
        None
    } else {
        Some(
            recording_latency_values
                .iter()
                .map(|latency| *latency as u128)
                .sum::<u128>()
                .checked_div(recording_latency_values.len() as u128)
                .unwrap_or(0) as u64,
        )
    };
    let avg_audio_duration_ms = if decoded.sessions.is_empty() {
        None
    } else {
        Some(
            decoded
                .sessions
                .iter()
                .map(|session| session.audio_duration_ms as u128)
                .sum::<u128>()
                .checked_div(decoded.sessions.len() as u128)
                .unwrap_or(0) as u64,
        )
    };
    let avg_audio_rms_level = if decoded.sessions.is_empty() {
        None
    } else {
        Some(
            decoded
                .sessions
                .iter()
                .map(|session| session.audio_rms_level)
                .sum::<f32>()
                / decoded.sessions.len() as f32,
        )
    };
    let derived_failure_guidance = derive_failure_guidance(
        decoded.recording_failures,
        decoded.recognizing_failures,
        decoded.executing_failures,
        decoded.committing_failures,
        decoded.silence_gate_failures,
        decoded.no_speech_failures,
    );
    let derived_commit_path_summary = derive_commit_path_summary(
        decoded.direct_unicode_commits,
        decoded.clipboard_fallback_commits,
        decoded.selection_replace_commits,
        decoded.committing_failures,
    );
    let dominant_failure_mode = decoded.dominant_failure_mode.clone().or_else(|| {
        derived_failure_guidance
            .as_ref()
            .map(|item| item.mode.clone())
    });
    let dominant_failure_guidance = decoded.dominant_failure_guidance.clone().or_else(|| {
        derived_failure_guidance
            .as_ref()
            .map(|item| item.guidance.clone())
    });
    let commit_path_outlook = decoded.commit_path_outlook.clone().or_else(|| {
        derived_commit_path_summary
            .as_ref()
            .map(|item| item.outlook.clone())
    });
    let commit_path_signal = decoded.commit_path_signal.clone().or_else(|| {
        derived_commit_path_summary
            .as_ref()
            .map(|item| item.signal.clone())
    });
    let commit_path_guidance = decoded.commit_path_guidance.clone().or_else(|| {
        derived_commit_path_summary
            .as_ref()
            .map(|item| item.guidance.clone())
    });
    let derived_workload_focus = derive_workload_focus(
        decoded.dictation_sessions,
        decoded.selected_text_sessions,
        decoded.wake_phrase_sessions,
        &selected_text_action_mix,
        &wake_phrase_action_mix,
    );
    let workload_focus = decoded.workload_focus.clone().or_else(|| {
        derived_workload_focus
            .as_ref()
            .map(|item| item.focus.clone())
    });
    let workload_focus_guidance = decoded.workload_focus_guidance.clone().or_else(|| {
        derived_workload_focus
            .as_ref()
            .map(|item| item.guidance.clone())
    });
    let derived_verification_plan = derive_verification_plan(
        dominant_failure_mode.as_deref(),
        commit_path_outlook.as_deref(),
        workload_focus.as_deref(),
        &decoded.effective_settings,
    );
    let verification_focus = decoded.verification_focus.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.focus.clone())
    });
    let verification_focus_guidance = decoded.verification_focus_guidance.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.guidance.clone())
    });
    let verification_title = decoded.verification_title.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.title.clone())
    });
    let verification_scenario = decoded.verification_scenario.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.scenario.clone())
    });
    let verification_command = decoded.verification_command.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.command.clone())
    });
    let verification_gesture = decoded.verification_gesture.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.gesture.clone())
    });
    let verification_example = decoded.verification_example.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.example.clone())
    });
    let verification_note = decoded.verification_note.clone().or_else(|| {
        derived_verification_plan
            .as_ref()
            .map(|item| item.note.clone())
    });
    let tuning_recommendations = derive_tuning_recommendations(
        &decoded.effective_settings,
        saved_settings,
        decoded.recording_failures,
        decoded.recognizing_failures,
        decoded.executing_failures,
        decoded.committing_failures,
        decoded.silence_gate_failures,
        decoded.no_speech_failures,
        decoded.direct_unicode_commits,
        decoded.clipboard_fallback_commits,
    );

    let mut drift_fields = Vec::new();
    if decoded.effective_settings.dictation_shortcut != saved_settings.dictation_shortcut {
        drift_fields.push("Primary shortcut".to_string());
    }
    if decoded.effective_settings.shortcut_mode != saved_settings.shortcut_mode {
        drift_fields.push("Shortcut mode".to_string());
    }
    if decoded.effective_settings.refinement_quality != saved_settings.refinement_quality {
        drift_fields.push("Refine quality".to_string());
    }
    if decoded.effective_settings.silence_gate_level != saved_settings.silence_gate_level {
        drift_fields.push("Silence gate".to_string());
    }
    if decoded.effective_settings.diagnostics_verbosity != saved_settings.diagnostics_verbosity {
        drift_fields.push("Diagnostics verbosity".to_string());
    }
    if decoded.effective_settings.system_language != saved_settings.system_language {
        drift_fields.push("System language".to_string());
    }
    if decoded.effective_settings.ui_style != saved_settings.ui_style {
        drift_fields.push("UI style".to_string());
    }
    if decoded.effective_settings.audio_feedback_enabled != saved_settings.audio_feedback_enabled {
        drift_fields.push("Audio cues".to_string());
    }
    if decoded.effective_settings.wake_phrase.enabled != saved_settings.wake_phrase.enabled {
        drift_fields.push("Wake phrase enabled".to_string());
    }
    if decoded.effective_settings.wake_phrase.phrase != saved_settings.wake_phrase.phrase {
        drift_fields.push("Wake phrase text".to_string());
    }

    Ok(Some(LatestLiveHostSummary {
        report_generated_at_epoch_ms: decoded.generated_at_epoch_ms,
        effective_primary_shortcut: shortcut_to_string(
            &decoded.effective_settings.dictation_shortcut,
        ),
        effective_primary_shortcut_modifiers: decoded
            .effective_settings
            .dictation_shortcut
            .modifiers
            .clone(),
        effective_primary_shortcut_key: decoded.effective_settings.dictation_shortcut.key.clone(),
        effective_shortcut_mode: decoded.effective_settings.shortcut_mode.clone(),
        effective_refinement_quality: decoded.effective_settings.refinement_quality.clone(),
        effective_refinement_profile: resolve_provider_config(&decoded.effective_settings).profile,
        effective_silence_gate_level: decoded.effective_settings.silence_gate_level,
        effective_diagnostics_verbosity: decoded.effective_settings.diagnostics_verbosity.clone(),
        effective_system_language: decoded.effective_settings.system_language.clone(),
        effective_ui_style: decoded.effective_settings.ui_style.clone(),
        effective_audio_feedback_enabled: decoded.effective_settings.audio_feedback_enabled,
        effective_wake_phrase_enabled: decoded.effective_settings.wake_phrase.enabled,
        effective_wake_phrase_text: decoded.effective_settings.wake_phrase.phrase.clone(),
        matches_saved_settings: drift_fields.is_empty(),
        drift_fields,
        attempted_sessions: decoded.attempted_sessions,
        successful_sessions: decoded.successful_sessions,
        failed_sessions: decoded.failed_sessions,
        dictation_sessions: decoded.dictation_sessions,
        selected_text_sessions: decoded.selected_text_sessions,
        wake_phrase_sessions: decoded.wake_phrase_sessions,
        instructed_dictation_sessions: decoded.instructed_dictation_sessions,
        selected_text_action_mix,
        wake_phrase_action_mix,
        failed_with_summary_sessions: decoded.failed_with_summary_sessions,
        failed_before_summary_sessions: decoded.failed_before_summary_sessions,
        recording_failures: decoded.recording_failures,
        recognizing_failures: decoded.recognizing_failures,
        executing_failures: decoded.executing_failures,
        committing_failures: decoded.committing_failures,
        silence_gate_failures: decoded.silence_gate_failures,
        no_speech_failures: decoded.no_speech_failures,
        direct_unicode_commits: decoded.direct_unicode_commits,
        clipboard_fallback_commits: decoded.clipboard_fallback_commits,
        selection_replace_commits: decoded.selection_replace_commits,
        selected_text_compatibility_signal: decoded.selected_text_compatibility_signal,
        selected_text_compatibility_guidance: decoded.selected_text_compatibility_guidance,
        local_asr_only_sessions,
        local_asr_with_refine_sessions,
        cloud_asr_with_refine_sessions,
        avg_total_session_latency_ms,
        avg_recording_start_latency_ms,
        avg_audio_duration_ms,
        avg_audio_rms_level,
        dominant_failure_mode,
        dominant_failure_guidance,
        commit_path_outlook,
        commit_path_signal,
        commit_path_guidance,
        verification_focus,
        verification_focus_guidance,
        verification_title,
        verification_scenario,
        verification_command,
        verification_gesture,
        verification_example,
        verification_note,
        workload_focus,
        workload_focus_guidance,
        tuning_recommendations,
        last_failure: decoded.last_failure,
        last_failure_summary: decoded.last_failure_summary,
        latest_successful_session: decoded
            .sessions
            .into_iter()
            .rev()
            .find(|session| session.final_state == "Committed")
            .map(|session| LatestSuccessfulSessionSummary {
                session_id: session.session_id,
                session_kind: session.session_kind,
                route_decision: LatestSuccessfulRouteDecision {
                    route_name: session.route_decision.route_name,
                },
                recognized_text: session.recognized_text,
                committed_text: session.committed_text,
                degraded_to_asr: session.degraded_to_asr,
                fallback_reason: session.fallback_reason,
                commit_transport: session.commit_transport,
                selected_text_execution: session.selected_text_execution,
                wake_phrase_execution: session.wake_phrase_execution,
                instructed_dictation_execution: session.instructed_dictation_execution,
            }),
        recent_failures: decoded
            .failure_history
            .into_iter()
            .rev()
            .take(3)
            .map(|failure| RecentFailureSummary {
                attempted_session_index: failure.attempted_session_index,
                session_id: failure.session_id,
                session_kind: failure.session_kind,
                failure_phase: failure.failure_phase,
                error: failure.error,
                selected_text_compatibility: failure.selected_text_compatibility,
            })
            .collect(),
    }))
}

fn derive_selected_text_action_mix(sessions: &[LiveHostSessionSnapshot]) -> Vec<ActionMixEntry> {
    let mut counts = std::collections::BTreeMap::new();
    for session in sessions
        .iter()
        .filter(|session| session.final_state == "Committed")
    {
        let Some(execution) = &session.selected_text_execution else {
            continue;
        };
        let label = selected_text_action_label(&execution.action).to_string();
        *counts.entry(label).or_insert(0_u64) += 1;
    }

    summarize_action_mix(counts)
}

fn derive_wake_phrase_action_mix(sessions: &[LiveHostSessionSnapshot]) -> Vec<ActionMixEntry> {
    let mut counts = std::collections::BTreeMap::new();
    for session in sessions
        .iter()
        .filter(|session| session.final_state == "Committed")
    {
        let Some(execution) = &session.wake_phrase_execution else {
            continue;
        };
        let label = wake_phrase_action_label(&execution.action).to_string();
        *counts.entry(label).or_insert(0_u64) += 1;
    }

    summarize_action_mix(counts)
}

fn summarize_action_mix(counts: std::collections::BTreeMap<String, u64>) -> Vec<ActionMixEntry> {
    let mut items = counts
        .into_iter()
        .map(|(label, count)| ActionMixEntry { label, count })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(3);
    items
}

fn derive_tuning_recommendations(
    effective_settings: &RuntimeSettings,
    saved_settings: &RuntimeSettings,
    recording_failures: u64,
    recognizing_failures: u64,
    _executing_failures: u64,
    committing_failures: u64,
    silence_gate_failures: u64,
    no_speech_failures: u64,
    direct_unicode_commits: u64,
    clipboard_fallback_commits: u64,
) -> Vec<TuningRecommendation> {
    let mut recommendations = Vec::new();
    let capture_side_signal_count = recording_failures + silence_gate_failures + no_speech_failures;

    if silence_gate_failures > 0 && effective_settings.silence_gate_level > 1 {
        let target_level = effective_settings.silence_gate_level.saturating_sub(1);
        let saved_value = saved_settings.silence_gate_level.to_string();
        let live_value = effective_settings.silence_gate_level.to_string();
        let saved_already_matches_target = saved_settings.silence_gate_level == target_level;
        recommendations.push(TuningRecommendation {
            panel: "Feedback".to_string(),
            setting_key: "silence_gate_level".to_string(),
            live_value,
            saved_value,
            target_value: target_level.to_string(),
            saved_already_matches_target,
            restart_required: saved_already_matches_target
                && effective_settings.silence_gate_level != target_level,
            label: format!(
                "Lower silence gate to {} ({})",
                target_level,
                silence_gate_label(target_level)
            ),
            strength: recommendation_strength_label(silence_gate_failures).to_string(),
            evidence_count: silence_gate_failures,
            evidence_summary: format!(
                "{} silence-gate failure{} in the mirrored live host",
                silence_gate_failures,
                if silence_gate_failures == 1 { "" } else { "s" }
            ),
            rationale:
                "Recent sessions are being rejected before ASR. A lighter silence gate should admit quieter captures."
                    .to_string(),
        });
    }

    if (no_speech_failures > 0 || recognizing_failures > 0)
        && next_refinement_quality(&effective_settings.refinement_quality)
            != effective_settings.refinement_quality
    {
        let target_quality = next_refinement_quality(&effective_settings.refinement_quality);
        let evidence_count = no_speech_failures + recognizing_failures;
        let saved_value = refinement_quality_value(&saved_settings.refinement_quality).to_string();
        let live_value =
            refinement_quality_value(&effective_settings.refinement_quality).to_string();
        let target_value = refinement_quality_value(&target_quality).to_string();
        let saved_already_matches_target = saved_value == target_value;
        recommendations.push(TuningRecommendation {
            panel: "Speech pipeline".to_string(),
            setting_key: "refinement_quality".to_string(),
            live_value,
            saved_value,
            target_value: target_value.clone(),
            saved_already_matches_target,
            restart_required: saved_already_matches_target
                && effective_settings.refinement_quality != target_quality,
            label: format!(
                "Raise refine quality to {}",
                refinement_quality_label(&target_quality)
            ),
            strength: recommendation_strength_label(evidence_count).to_string(),
            evidence_count,
            evidence_summary: format!(
                "{} recognition-side failure{} in the mirrored live host",
                evidence_count,
                if evidence_count == 1 { "" } else { "s" }
            ),
            rationale:
                "Recognition is failing after capture. A stronger refine tier should give the speech pipeline more room to recover quieter or less clear takes."
                    .to_string(),
        });
    }

    if capture_side_signal_count > 0 && !effective_settings.audio_feedback_enabled {
        let saved_already_matches_target = saved_settings.audio_feedback_enabled;
        recommendations.push(TuningRecommendation {
            panel: "Feedback".to_string(),
            setting_key: "audio_feedback_enabled".to_string(),
            live_value: effective_settings.audio_feedback_enabled.to_string(),
            saved_value: saved_settings.audio_feedback_enabled.to_string(),
            target_value: "true".to_string(),
            saved_already_matches_target,
            restart_required: saved_already_matches_target
                && !effective_settings.audio_feedback_enabled,
            label: "Turn audio cues back on".to_string(),
            strength: recommendation_strength_label(capture_side_signal_count).to_string(),
            evidence_count: capture_side_signal_count,
            evidence_summary: format!(
                "{} capture-side signal{} in the mirrored live host",
                capture_side_signal_count,
                if capture_side_signal_count == 1 { "" } else { "s" }
            ),
            rationale:
                "Capture-side issues are easier to avoid when the start and stop cues are audible, because they make timing and phrase boundaries clearer during the next test pass."
                    .to_string(),
        });
    }

    let clipboard_fallback_dominant =
        clipboard_fallback_commits > direct_unicode_commits && clipboard_fallback_commits > 0;
    if effective_settings.diagnostics_verbosity != DiagnosticsVerbosity::Verbose
        && (committing_failures > 0 || clipboard_fallback_dominant)
    {
        let saved_value =
            diagnostics_verbosity_value(&saved_settings.diagnostics_verbosity).to_string();
        let live_value =
            diagnostics_verbosity_value(&effective_settings.diagnostics_verbosity).to_string();
        let saved_already_matches_target =
            saved_settings.diagnostics_verbosity == DiagnosticsVerbosity::Verbose;
        let evidence_count = if committing_failures > 0 {
            committing_failures
        } else {
            clipboard_fallback_commits
        };
        let evidence_summary = if committing_failures > 0 {
            format!(
                "{} commit failure{} in the mirrored live host",
                evidence_count,
                if evidence_count == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "{} clipboard-fallback commit{} in the mirrored live host",
                evidence_count,
                if evidence_count == 1 { "" } else { "s" }
            )
        };
        let rationale = if committing_failures > 0 {
            "Late commit failures are happening after recognition succeeds. Verbose diagnostics will preserve the fuller event trail for the next troubleshooting pass."
                .to_string()
        } else {
            "Successful sessions are leaning on the clipboard fallback path more than direct caret insertion. Verbose diagnostics will preserve a fuller trail for the next cross-app compatibility pass."
                .to_string()
        };
        recommendations.push(TuningRecommendation {
            panel: "Diagnostics".to_string(),
            setting_key: "diagnostics_verbosity".to_string(),
            live_value,
            saved_value,
            target_value: "Verbose".to_string(),
            saved_already_matches_target,
            restart_required: saved_already_matches_target
                && effective_settings.diagnostics_verbosity != DiagnosticsVerbosity::Verbose,
            label: "Switch diagnostics to Verbose".to_string(),
            strength: recommendation_strength_label(evidence_count).to_string(),
            evidence_count,
            evidence_summary,
            rationale,
        });
    }

    recommendations.truncate(3);
    recommendations
}

fn silence_gate_label(level: u8) -> &'static str {
    match level {
        1 => "Very light",
        2 => "Conservative",
        3 => "Balanced",
        4 => "Firm",
        5 => "Strict",
        _ => "Custom",
    }
}

fn next_refinement_quality(value: &RefinementQuality) -> RefinementQuality {
    match value {
        RefinementQuality::Fast => RefinementQuality::Balanced,
        RefinementQuality::FastPlus => RefinementQuality::Balanced,
        RefinementQuality::Balanced => RefinementQuality::BestQuality,
        RefinementQuality::BestQuality => RefinementQuality::BestQuality,
    }
}

fn refinement_quality_value(value: &RefinementQuality) -> &'static str {
    match value {
        RefinementQuality::Fast => "Fast",
        RefinementQuality::FastPlus => "FastPlus",
        RefinementQuality::Balanced => "Balanced",
        RefinementQuality::BestQuality => "BestQuality",
    }
}

fn refinement_quality_label(value: &RefinementQuality) -> &'static str {
    match value {
        RefinementQuality::Fast => "Fast",
        RefinementQuality::FastPlus => "Balanced",
        RefinementQuality::Balanced => "Balanced",
        RefinementQuality::BestQuality => "Best Quality",
    }
}

fn recommendation_strength_label(evidence_count: u64) -> &'static str {
    if evidence_count >= 3 {
        "Strong signal"
    } else if evidence_count == 2 {
        "Moderate signal"
    } else {
        "Emerging signal"
    }
}

fn diagnostics_verbosity_value(value: &DiagnosticsVerbosity) -> &'static str {
    match value {
        DiagnosticsVerbosity::Standard => "Standard",
        DiagnosticsVerbosity::Verbose => "Verbose",
    }
}

fn settings_source_label(source: &RuntimeSettingsSource) -> &'static str {
    match source {
        RuntimeSettingsSource::Defaults => "defaults",
        RuntimeSettingsSource::File => "file",
    }
}

fn resolve_latest_live_host_report_path(
    explicit_path: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    match explicit_path {
        Some(path) => resolve_latest_live_host_report_path_in(
            Some(path),
            &crate::live_host_reports::default_live_host_report_dir(),
        ),
        None => discover_latest_live_host_report_path(),
    }
}

fn resolve_latest_live_host_report_path_in(
    explicit_path: Option<&Path>,
    report_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    match explicit_path {
        Some(path) if path.exists() => Ok(Some(path.to_path_buf())),
        Some(path) if looks_like_live_host_report_path(path) => {
            discover_latest_live_host_report_path_in(report_dir)
        }
        Some(path) => Ok(Some(path.to_path_buf())),
        None => discover_latest_live_host_report_path_in(report_dir),
    }
}

fn looks_like_live_host_report_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    file_name.starts_with("live-host-report-") && file_name.ends_with(".json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_key_store::{InMemoryProviderCredentialStore, ProviderCredentialStore};
    use shared_protocol::{ProviderPreset, ProviderSettings, RuntimeSettings};
    use std::env;
    use std::ffi::OsString;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
    fn writes_settings_runtime_state_file() {
        let script_path = env::temp_dir().join(format!(
            "voiceflow-settings-runtime-state-{}-{}.js",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos()
        ));
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings {
                system_language: SystemLanguage::Chinese,
                ui_style: UiStyle::Light,
                ..RuntimeSettings::default()
            },
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec!["test warning".to_string()],
        };

        let written_path = write_settings_runtime_state_to_path(
            &script_path,
            &loaded,
            Some("http://127.0.0.1:49000"),
            Some(Path::new(r"C:\Temp\report.json")),
            None,
            &MissingProviderCredentialStore,
        )
        .expect("settings runtime state should write");
        let contents =
            fs::read_to_string(&script_path).expect("settings runtime state should be readable");
        let _ = fs::remove_file(&script_path);

        assert_eq!(written_path, script_path);
        assert!(contents.contains("__VOICEFLOW_SETTINGS_RUNTIME__"));
        assert!(contents.contains("Ctrl+Space"));
        assert!(contents.contains("test warning"));
        assert!(contents.contains("http://127.0.0.1:49000"));
        assert!(contents.contains(r"C:\\Temp\\report.json"));
        assert!(contents.contains(r#""system_language": "Chinese""#));
        assert!(contents.contains(r#""ui_style": "Light""#));
    }

    #[test]
    fn settings_snapshot_includes_language_ui_style_and_history_retention() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings {
                system_language: SystemLanguage::Chinese,
                ui_style: UiStyle::Light,
                history_retention: HistoryRetention::Last7Days,
                ..RuntimeSettings::default()
            },
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };

        let snapshot = build_settings_ui_snapshot(&loaded, None, None, None)
            .expect("settings snapshot should build");

        assert_eq!(snapshot.system_language, SystemLanguage::Chinese);
        assert_eq!(snapshot.ui_style, UiStyle::Light);
        assert_eq!(snapshot.history_retention, HistoryRetention::Last7Days);
        assert_eq!(snapshot.prompt_overrides.len(), 4);
        assert!(snapshot.prompt_overrides.iter().any(|status| {
            status.slot_id == "dictation_light_cleanup"
                && status.env_var == "VOICEFLOW_DICTATION_LIGHT_PROMPT_FILE"
        }));
        assert!(snapshot.prompt_overrides.iter().any(|status| {
            status.slot_id == "dictation_structured_cleanup"
                && status.env_var == "VOICEFLOW_DICTATION_STRUCTURED_PROMPT_FILE"
        }));
        assert!(snapshot.prompt_overrides.iter().any(|status| {
            status.slot_id == "selected_text_edit"
                && status.env_var == "VOICEFLOW_SELECTED_TEXT_PROMPT_FILE"
        }));
        assert!(snapshot.prompt_overrides.iter().any(|status| {
            status.slot_id == "instructed_dictation"
                && status.env_var == "VOICEFLOW_INSTRUCTED_DICTATION_PROMPT_FILE"
        }));
    }

    #[test]
    fn settings_runtime_state_reports_provider_key_source_without_raw_key() {
        let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _api_key_guard = EnvVarGuard::set("DASHSCOPE_API_KEY", "runtime-secret-sentinel");
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };

        let encoded = encode_settings_runtime_state_json(&loaded, None, None, None)
            .expect("settings runtime state should encode");

        assert!(encoded.contains(r#""key_source": "legacy_dashscope_env""#));
        assert!(!encoded.contains("runtime-secret-sentinel"));
        assert!(!encoded.contains("DASHSCOPE_API_KEY"));
    }

    #[test]
    fn settings_runtime_state_separates_stored_credential_from_effective_key_source() {
        let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _provider_key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::Bailian, "stored-secret-sentinel")
            .expect("test store should accept key");
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from("settings.json"),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };

        let encoded =
            encode_settings_runtime_state_json_with_credentials(&loaded, None, None, None, &store)
                .expect("settings runtime state should encode");
        let json: serde_json::Value =
            serde_json::from_str(&encoded).expect("runtime state should decode");

        assert_eq!(json["provider"]["stored_credential_present"], true);
        assert_eq!(json["provider"]["credential_store_status"], "available");
        assert_eq!(json["provider"]["effective_key_present"], true);
        assert_eq!(json["provider"]["effective_key_source"], "credential_store");
        assert!(!encoded.contains("stored-secret-sentinel"));
    }

    #[test]
    fn settings_runtime_state_reports_env_key_when_store_is_missing() {
        let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _provider_key_guard =
            EnvVarGuard::set("VOICEFLOW_PROVIDER_API_KEY", "env-secret-sentinel");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let store = InMemoryProviderCredentialStore::new();
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from("settings.json"),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };

        let encoded =
            encode_settings_runtime_state_json_with_credentials(&loaded, None, None, None, &store)
                .expect("settings runtime state should encode");
        let json: serde_json::Value =
            serde_json::from_str(&encoded).expect("runtime state should decode");

        assert_eq!(json["provider"]["stored_credential_present"], false);
        assert_eq!(json["provider"]["credential_store_status"], "missing");
        assert_eq!(json["provider"]["effective_key_present"], true);
        assert_eq!(json["provider"]["effective_key_source"], "provider_env");
        assert!(!encoded.contains("env-secret-sentinel"));
    }

    #[test]
    fn settings_runtime_state_reports_selected_provider_credential_when_effective_provider_differs()
    {
        let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _provider_type_guard = EnvVarGuard::set("VOICEFLOW_PROVIDER_TYPE", "volcengine_ark");
        let _provider_key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::Bailian, "stored-secret-sentinel")
            .expect("test store should accept key");
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings {
                provider: ProviderSettings {
                    preset: ProviderPreset::Bailian,
                    ..ProviderSettings::default()
                },
                ..RuntimeSettings::default()
            },
            path: PathBuf::from("settings.json"),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };

        let encoded =
            encode_settings_runtime_state_json_with_credentials(&loaded, None, None, None, &store)
                .expect("settings runtime state should encode");
        let json: serde_json::Value =
            serde_json::from_str(&encoded).expect("runtime state should decode");

        assert_eq!(json["provider"]["preset"], "volcengine_ark");
        assert_eq!(json["provider"]["saved_preset"], "bailian");
        assert_eq!(
            json["provider"]["credential_status_by_preset"]["bailian"]["stored_credential_present"],
            true
        );
        assert_eq!(
            json["provider"]["credential_status_by_preset"]["bailian"]["credential_store_status"],
            "available"
        );
        assert_eq!(
            json["provider"]["credential_status_by_preset"]["volcengine_ark"]["stored_credential_present"],
            false
        );
        assert_eq!(
            json["provider"]["credential_status_by_preset"]["tencent_hunyuan"]["stored_credential_present"],
            false
        );
        assert!(!encoded.contains("stored-secret-sentinel"));
    }

    #[test]
    fn settings_runtime_state_reports_built_in_bailian_preset_source() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };

        let encoded = encode_settings_runtime_state_json(&loaded, None, None, None)
            .expect("settings runtime state should encode");

        assert!(encoded.contains(r#""preset": "bailian""#));
        assert!(encoded.contains(r#""preset_source": "built_in""#));
        assert!(!encoded.contains(r#""preset_source": "missing""#));
    }

    #[test]
    fn settings_runtime_state_includes_saved_provider_fields_and_effective_sources() {
        let _env_lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let _api_key_guard =
            EnvVarGuard::set("VOICEFLOW_PROVIDER_API_KEY", "runtime-secret-sentinel");
        let _model_guard = EnvVarGuard::set("VOICEFLOW_MODEL_BEST", "env-model");
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings {
                provider: ProviderSettings {
                    preset: ProviderPreset::VolcengineArk,
                    base_url: Some("https://saved.example/api/v3".to_string()),
                    active_model: Some("saved-model".to_string()),
                    request_timeout_ms: Some(15_000),
                },
                ..RuntimeSettings::default()
            },
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };

        let encoded = encode_settings_runtime_state_json(&loaded, None, None, None)
            .expect("settings runtime state should encode");

        assert!(encoded.contains(r#""saved_preset": "volcengine_ark""#));
        assert!(encoded.contains(r#""saved_base_url": "https://saved.example/api/v3""#));
        assert!(encoded.contains(r#""saved_active_model": "saved-model""#));
        assert!(encoded.contains(r#""saved_request_timeout_ms": 15000"#));
        assert!(encoded.contains(r#""model_code": "env-model""#));
        assert!(encoded.contains(r#""model_source": "env""#));
        assert!(!encoded.contains("runtime-secret-sentinel"));
    }

    #[test]
    fn apply_summary_names_language_ui_style_and_history_retention_updates() {
        let summary = build_last_settings_apply_summary(
            &RuntimeSettingsUpdate {
                system_language: Some(SystemLanguage::Chinese),
                ui_style: Some(UiStyle::Light),
                history_retention: Some(HistoryRetention::Latest500),
                ..RuntimeSettingsUpdate::default()
            },
            "test",
        );

        assert!(
            summary
                .applied_fields
                .contains(&"System language".to_string())
        );
        assert!(summary.applied_fields.contains(&"UI style".to_string()));
        assert!(
            summary
                .applied_fields
                .contains(&"History retention".to_string())
        );
    }

    #[test]
    fn tolerates_unparseable_live_host_report_by_emitting_warning() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-invalid-report-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let invalid_report = temp_root.join("live-host-report-invalid.json");
        fs::write(&invalid_report, "{not-json").expect("invalid report should write");

        let snapshot = build_settings_ui_snapshot(
            &loaded,
            Some("http://127.0.0.1:49000"),
            Some(invalid_report.as_path()),
            None,
        )
        .expect("settings snapshot should still build");

        assert!(snapshot.latest_live_host_summary.is_none());
        assert!(
            snapshot
                .latest_live_host_report_warning
                .as_deref()
                .expect("warning should be present")
                .contains("no parseable live-host report was available")
        );

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn recommends_lowering_silence_gate_when_quiet_sessions_dominate() {
        let mut effective_settings = RuntimeSettings::default();
        effective_settings.silence_gate_level = 4;
        let mut settings = RuntimeSettings::default();
        settings.silence_gate_level = 4;

        let recommendations =
            derive_tuning_recommendations(&effective_settings, &settings, 0, 0, 0, 0, 3, 0, 0, 0);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].panel, "Feedback");
        assert_eq!(recommendations[0].setting_key, "silence_gate_level");
        assert_eq!(recommendations[0].target_value, "3");
        assert_eq!(recommendations[0].strength, "Strong signal");
        assert_eq!(recommendations[0].evidence_count, 3);
    }

    #[test]
    fn recommends_verbose_diagnostics_for_commit_failures() {
        let settings = RuntimeSettings::default();
        let effective_settings = RuntimeSettings::default();

        let recommendations =
            derive_tuning_recommendations(&effective_settings, &settings, 0, 0, 0, 2, 0, 0, 0, 0);

        assert!(recommendations.iter().any(|recommendation| {
            recommendation.setting_key == "diagnostics_verbosity"
                && recommendation.target_value == "Verbose"
        }));
    }

    #[test]
    fn marks_recommendation_as_restart_only_when_saved_settings_already_match_target() {
        let mut effective_settings = RuntimeSettings::default();
        effective_settings.silence_gate_level = 4;
        let mut saved_settings = RuntimeSettings::default();
        saved_settings.silence_gate_level = 3;

        let recommendations = derive_tuning_recommendations(
            &effective_settings,
            &saved_settings,
            0,
            0,
            0,
            0,
            2,
            0,
            0,
            0,
        );

        assert!(recommendations.iter().any(|recommendation| {
            recommendation.setting_key == "silence_gate_level"
                && recommendation.saved_already_matches_target
                && recommendation.restart_required
        }));
    }

    #[test]
    fn recommends_stronger_refine_for_no_speech_failures() {
        let mut settings = RuntimeSettings::default();
        settings.refinement_quality = RefinementQuality::Balanced;
        let mut effective_settings = RuntimeSettings::default();
        effective_settings.refinement_quality = RefinementQuality::Balanced;

        let recommendations =
            derive_tuning_recommendations(&effective_settings, &settings, 0, 0, 0, 0, 0, 2, 0, 0);

        assert!(recommendations.iter().any(|recommendation| {
            recommendation.setting_key == "refinement_quality"
                && recommendation.target_value == "BestQuality"
        }));
    }

    #[test]
    fn does_not_recommend_refine_upgrade_when_best_quality_is_active() {
        let settings = RuntimeSettings::default();
        let effective_settings = RuntimeSettings::default();

        let recommendations =
            derive_tuning_recommendations(&effective_settings, &settings, 0, 0, 0, 0, 0, 2, 0, 0);

        assert!(
            recommendations
                .iter()
                .all(|recommendation| recommendation.setting_key != "refinement_quality")
        );
    }

    #[test]
    fn recommends_audio_cues_for_capture_side_trouble_when_feedback_is_off() {
        let mut effective_settings = RuntimeSettings::default();
        effective_settings.audio_feedback_enabled = false;
        let mut settings = RuntimeSettings::default();
        settings.audio_feedback_enabled = false;

        let recommendations =
            derive_tuning_recommendations(&effective_settings, &settings, 0, 0, 0, 0, 1, 2, 0, 0);

        assert!(recommendations.iter().any(|recommendation| {
            recommendation.setting_key == "audio_feedback_enabled"
                && recommendation.target_value == "true"
                && recommendation.evidence_count == 3
        }));
    }

    #[test]
    fn recommends_verbose_diagnostics_when_clipboard_fallback_dominates() {
        let settings = RuntimeSettings::default();
        let effective_settings = RuntimeSettings::default();

        let recommendations =
            derive_tuning_recommendations(&effective_settings, &settings, 0, 0, 0, 0, 0, 0, 1, 3);

        assert!(recommendations.iter().any(|recommendation| {
            recommendation.setting_key == "diagnostics_verbosity"
                && recommendation.target_value == "Verbose"
                && recommendation
                    .evidence_summary
                    .contains("clipboard-fallback commit")
        }));
    }

    #[test]
    fn reports_clipboard_fallback_when_it_dominates_successful_commits() {
        let summary = derive_commit_path_summary(1, 3, 0, 0).expect("summary should be present");

        assert_eq!(summary.outlook, "Fallback-heavy");
        assert_eq!(summary.signal, "Clipboard fallback dominant (3)");
        assert!(summary.guidance.contains("clipboard-backed fallback"));
    }

    #[test]
    fn discovers_latest_live_host_report_from_directory() {
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let older = temp_root.join("live-host-report-100.json");
        let newer = temp_root.join("live-host-report-200.json");
        fs::write(&older, "{}").expect("older report should write");
        std::thread::sleep(std::time::Duration::from_millis(5));
        fs::write(&newer, "{}").expect("newer report should write");

        let discovered = discover_latest_live_host_report_path_in(&temp_root)
            .expect("report discovery should succeed")
            .expect("latest report should be discovered");

        assert_eq!(discovered, newer);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn falls_back_to_latest_report_when_explicit_path_is_stale() {
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-stale-report-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let stale = temp_root.join("live-host-report-100.json");
        let latest = temp_root.join("live-host-report-200.json");
        fs::write(&latest, "{}").expect("latest report should write");

        let resolved = resolve_latest_live_host_report_path_in(Some(stale.as_path()), &temp_root)
            .expect("report resolution should succeed")
            .expect("latest report should be discovered");

        assert_eq!(resolved, latest);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn falls_back_to_next_parseable_report_when_latest_report_is_malformed() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-parseable-report-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let valid_report = temp_root.join("live-host-report-100.json");
        let invalid_report = temp_root.join("live-host-report-200.json");
        fs::write(
            &valid_report,
            r#"{
  "generated_at_epoch_ms": 100,
  "effective_settings": {
    "dictation_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "edit_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "shortcut_mode": "PushToTalk",
    "refinement_quality": "Balanced",
    "silence_gate_level": 2,
    "diagnostics_verbosity": "Standard",
    "audio_feedback_enabled": true,
    "wake_phrase": { "enabled": true, "phrase": "Hey VoiceFlow" }
  },
  "attempted_sessions": 1,
  "successful_sessions": 1,
  "failed_sessions": 0,
  "dictation_sessions": 1,
  "selected_text_sessions": 0,
  "wake_phrase_sessions": 0,
  "failed_with_summary_sessions": 0,
  "failed_before_summary_sessions": 0,
  "recording_failures": 0,
  "recognizing_failures": 0,
  "executing_failures": 0,
  "committing_failures": 0,
  "silence_gate_failures": 0,
  "no_speech_failures": 0,
  "direct_unicode_commits": 1,
  "clipboard_fallback_commits": 0,
  "selection_replace_commits": 0,
  "dominant_failure_mode": null,
  "dominant_failure_guidance": null,
  "commit_path_outlook": "Healthy",
  "commit_path_signal": "Direct Unicode input stable (1)",
  "commit_path_guidance": "Direct Unicode input has been landing cleanly.",
  "verification_focus": "Real-app confidence",
  "verification_focus_guidance": "Run another confidence check.",
  "verification_title": "Verify a normal live session",
  "verification_scenario": "Run the live host and verify normal dictation.",
  "verification_command": "cargo run -p input-host -- --serve-live",
  "verification_gesture": "Hold Ctrl+Space, speak, release.",
  "verification_example": "Example: say \"all right, testing the speech capture path\" and confirm it lands cleanly.",
  "verification_note": "Run another confidence check. Run the live host and verify normal dictation. Example: say \"all right, testing the speech capture path\" and confirm it lands cleanly. For the next check, Hold Ctrl+Space, speak, release. If text is selected, the runtime should route into edit mode instead of plain dictation.",
  "last_failure": null,
  "last_failure_summary": null,
  "sessions": [
    {
      "session_id": 1,
      "session_kind": "Dictation",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrOnly" },
      "recognized_text": "hello",
      "committed_text": "Hello.",
      "commit_transport": "DirectUnicodeSendInput"
    }
  ],
  "failure_history": []
}"#,
        )
        .expect("valid report should write");
        std::thread::sleep(std::time::Duration::from_millis(5));
        fs::write(&invalid_report, "{not-json").expect("invalid report should write");

        let snapshot =
            build_settings_ui_snapshot(&loaded, None, Some(invalid_report.as_path()), None)
                .expect("settings snapshot should still build");
        let expected_path = valid_report.display().to_string();

        assert_eq!(
            snapshot.latest_live_host_report_path.as_deref(),
            Some(expected_path.as_str())
        );
        let summary = snapshot
            .latest_live_host_summary
            .expect("summary should be mirrored");
        assert_eq!(
            summary.verification_example.as_deref(),
            Some(
                "Example: say \"all right, testing the speech capture path\" and confirm it lands cleanly."
            )
        );
        assert!(
            summary
                .verification_note
                .as_deref()
                .expect("verification note should be mirrored")
                .contains("route into edit mode instead of plain dictation")
        );
        assert!(
            snapshot
                .latest_live_host_report_warning
                .as_deref()
                .expect("warning should be present")
                .contains("Skipped 1 newer mirrored diagnostic report")
        );

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn latest_successful_session_preserves_selected_text_execution_summary() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-selected-text-summary-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let report_path = temp_root.join("live-host-report-selected-text.json");
        fs::write(
            &report_path,
            r#"{
  "generated_at_epoch_ms": 100,
  "effective_settings": {
    "dictation_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "edit_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "shortcut_mode": "PushToTalk",
    "refinement_quality": "Balanced",
    "silence_gate_level": 2,
    "diagnostics_verbosity": "Standard",
    "audio_feedback_enabled": true,
    "wake_phrase": { "enabled": true, "phrase": "Hey VoiceFlow" }
  },
  "attempted_sessions": 1,
  "successful_sessions": 1,
  "failed_sessions": 0,
  "dictation_sessions": 0,
  "selected_text_sessions": 1,
  "wake_phrase_sessions": 0,
  "failed_with_summary_sessions": 0,
  "failed_before_summary_sessions": 0,
  "recording_failures": 0,
  "recognizing_failures": 0,
  "executing_failures": 0,
  "committing_failures": 0,
  "silence_gate_failures": 0,
  "no_speech_failures": 0,
  "direct_unicode_commits": 0,
  "clipboard_fallback_commits": 0,
  "selection_replace_commits": 1,
  "dominant_failure_mode": null,
  "dominant_failure_guidance": null,
  "commit_path_outlook": "Edit-heavy",
  "commit_path_signal": "Selected-text replacement dominant (1)",
  "commit_path_guidance": "Recent success is flowing through selected-text replacement.",
  "verification_focus": "Edit execution",
  "verification_focus_guidance": "Run another selected-text edit case.",
  "verification_title": "Verify selected-text edit routing",
  "verification_scenario": "Highlight a short phrase and speak an explicit edit instruction.",
  "verification_command": "cargo run -p input-host -- --serve-live",
  "verification_gesture": "Hold Ctrl+Space, say uppercase, release.",
  "last_failure": null,
  "last_failure_summary": null,
  "sessions": [
    {
      "session_id": 4,
      "session_kind": "SelectedTextEdit",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrOnly" },
      "recognized_text": "numbered list",
      "committed_text": "1. first\n2. second",
      "commit_transport": "ClipboardSelectionReplace",
      "selected_text_execution": {
        "action": "NumberedList",
        "strategy": "temporary executor reformatted the selection as a numbered list"
      },
      "wake_phrase_execution": null
    }
  ],
  "failure_history": []
}"#,
        )
        .expect("report should write");

        let snapshot = build_settings_ui_snapshot(&loaded, None, Some(report_path.as_path()), None)
            .expect("settings snapshot should build");
        let success = snapshot
            .latest_live_host_summary
            .and_then(|summary| summary.latest_successful_session)
            .expect("successful session should be mirrored");

        assert_eq!(
            success
                .selected_text_execution
                .map(|execution| execution.action),
            Some(shared_protocol::SelectedTextExecutionAction::NumberedList)
        );
        assert_eq!(success.commit_transport, "ClipboardSelectionReplace");

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn latest_successful_session_preserves_wake_phrase_execution_summary() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-wake-phrase-summary-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let report_path = temp_root.join("live-host-report-wake-phrase.json");
        fs::write(
            &report_path,
            r#"{
  "generated_at_epoch_ms": 100,
  "effective_settings": {
    "dictation_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "edit_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "shortcut_mode": "PushToTalk",
    "refinement_quality": "Balanced",
    "silence_gate_level": 2,
    "diagnostics_verbosity": "Standard",
    "audio_feedback_enabled": true,
    "wake_phrase": { "enabled": true, "phrase": "Hey VoiceFlow" }
  },
  "attempted_sessions": 1,
  "successful_sessions": 1,
  "failed_sessions": 0,
  "dictation_sessions": 0,
  "selected_text_sessions": 0,
  "wake_phrase_sessions": 1,
  "failed_with_summary_sessions": 0,
  "failed_before_summary_sessions": 0,
  "recording_failures": 0,
  "recognizing_failures": 0,
  "executing_failures": 0,
  "committing_failures": 0,
  "silence_gate_failures": 0,
  "no_speech_failures": 0,
  "direct_unicode_commits": 1,
  "clipboard_fallback_commits": 0,
  "selection_replace_commits": 0,
  "dominant_failure_mode": null,
  "dominant_failure_guidance": null,
  "commit_path_outlook": "Healthy",
  "commit_path_signal": "Direct Unicode input stable (1)",
  "commit_path_guidance": "Direct Unicode input has been landing cleanly.",
  "verification_focus": "Real-app confidence",
  "verification_focus_guidance": "Run another confidence check.",
  "verification_title": "Verify a normal intent session",
  "verification_scenario": "Trigger a wake-phrase request and verify the drafted output.",
  "verification_command": "cargo run -p input-host -- --serve-live",
  "verification_gesture": "Hold Ctrl+Space, speak the request, release.",
  "last_failure": null,
  "last_failure_summary": null,
  "sessions": [
    {
      "session_id": 5,
      "session_kind": "WakePhraseIntent",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrWithRefine" },
      "recognized_text": "draft an email to finance",
      "committed_text": "Subject: Follow-Up For Finance",
      "commit_transport": "DirectUnicodeSendInput",
      "selected_text_execution": null,
      "wake_phrase_execution": {
        "action": "DraftEmail",
        "strategy": "temporary intent executor expanded the request into an email draft"
      }
    }
  ],
  "failure_history": []
}"#,
        )
        .expect("report should write");

        let snapshot = build_settings_ui_snapshot(&loaded, None, Some(report_path.as_path()), None)
            .expect("settings snapshot should build");
        let success = snapshot
            .latest_live_host_summary
            .and_then(|summary| summary.latest_successful_session)
            .expect("successful session should be mirrored");

        assert_eq!(
            success
                .wake_phrase_execution
                .map(|execution| execution.action),
            Some(shared_protocol::WakePhraseIntentAction::DraftEmail)
        );
        assert_eq!(success.commit_transport, "DirectUnicodeSendInput");

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn latest_successful_session_preserves_fallback_metadata() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-fallback-summary-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let report_path = temp_root.join("live-host-report-fallback.json");
        fs::write(
            &report_path,
            r#"{
  "generated_at_epoch_ms": 100,
  "effective_settings": {
    "dictation_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "edit_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "shortcut_mode": "PushToTalk",
    "refinement_quality": "Balanced",
    "silence_gate_level": 2,
    "diagnostics_verbosity": "Standard",
    "audio_feedback_enabled": true,
    "wake_phrase": { "enabled": true, "phrase": "Hey VoiceFlow" }
  },
  "attempted_sessions": 1,
  "successful_sessions": 1,
  "failed_sessions": 0,
  "dictation_sessions": 1,
  "selected_text_sessions": 0,
  "wake_phrase_sessions": 0,
  "failed_with_summary_sessions": 0,
  "failed_before_summary_sessions": 0,
  "recording_failures": 0,
  "recognizing_failures": 0,
  "executing_failures": 0,
  "committing_failures": 0,
  "silence_gate_failures": 0,
  "no_speech_failures": 0,
  "direct_unicode_commits": 1,
  "clipboard_fallback_commits": 0,
  "selection_replace_commits": 0,
  "dominant_failure_mode": null,
  "dominant_failure_guidance": null,
  "commit_path_outlook": "Healthy",
  "commit_path_signal": "Direct Unicode input stable (1)",
  "commit_path_guidance": "Direct Unicode input has been landing cleanly.",
  "verification_focus": "Real-app confidence",
  "verification_focus_guidance": "Run another confidence check.",
  "verification_title": "Verify a normal dictation session",
  "verification_scenario": "Run the live host and confirm the fallback path stays usable.",
  "verification_command": "cargo run -p input-host -- --serve-live",
  "verification_gesture": "Hold Ctrl+Space, speak, release.",
  "last_failure": null,
  "last_failure_summary": null,
  "sessions": [
    {
      "session_id": 11,
      "session_kind": "Dictation",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrWithRefine" },
      "recognized_text": "this transcript should stay usable",
      "committed_text": "this transcript should stay usable",
      "degraded_to_asr": true,
      "fallback_reason": "local refinement failed and the engine degraded to the recognized transcript: test refiner failed",
      "commit_transport": "DirectUnicodeSendInput",
      "selected_text_execution": null,
      "wake_phrase_execution": null
    }
  ],
  "failure_history": []
}"#,
        )
        .expect("report should write");

        let snapshot = build_settings_ui_snapshot(&loaded, None, Some(report_path.as_path()), None)
            .expect("settings snapshot should build");
        let success = snapshot
            .latest_live_host_summary
            .and_then(|summary| summary.latest_successful_session)
            .expect("successful session should be mirrored");

        assert!(success.degraded_to_asr);
        assert!(
            success
                .fallback_reason
                .expect("fallback reason should be mirrored")
                .contains("test refiner failed")
        );

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn latest_live_host_summary_includes_action_mix_counts() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-action-mix-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let report_path = temp_root.join("live-host-report-action-mix.json");
        fs::write(
            &report_path,
            r#"{
  "generated_at_epoch_ms": 100,
  "effective_settings": {
    "dictation_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "edit_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "shortcut_mode": "PushToTalk",
    "refinement_quality": "Balanced",
    "silence_gate_level": 2,
    "diagnostics_verbosity": "Standard",
    "audio_feedback_enabled": true,
    "wake_phrase": { "enabled": true, "phrase": "Hey VoiceFlow" }
  },
  "attempted_sessions": 5,
  "successful_sessions": 5,
  "failed_sessions": 0,
  "dictation_sessions": 1,
  "selected_text_sessions": 3,
  "wake_phrase_sessions": 2,
  "failed_with_summary_sessions": 0,
  "failed_before_summary_sessions": 0,
  "recording_failures": 0,
  "recognizing_failures": 0,
  "executing_failures": 0,
  "committing_failures": 0,
  "silence_gate_failures": 0,
  "no_speech_failures": 0,
  "direct_unicode_commits": 3,
  "clipboard_fallback_commits": 0,
  "selection_replace_commits": 2,
  "dominant_failure_mode": null,
  "dominant_failure_guidance": null,
  "commit_path_outlook": "Healthy",
  "commit_path_signal": "Direct Unicode input stable (3)",
  "commit_path_guidance": "Direct Unicode input has been landing cleanly.",
  "verification_focus": "Real-app confidence",
  "verification_focus_guidance": "Run another confidence check.",
  "verification_title": "Verify a normal live session",
  "verification_scenario": "Run the live host and verify normal dictation.",
  "verification_command": "cargo run -p input-host -- --serve-live",
  "verification_gesture": "Hold Ctrl+Space, speak, release.",
  "last_failure": null,
  "last_failure_summary": null,
  "sessions": [
    {
      "session_id": 1,
      "session_kind": "Dictation",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrOnly" },
      "recognized_text": "hello",
      "committed_text": "Hello.",
      "commit_transport": "DirectUnicodeSendInput",
      "selected_text_execution": null,
      "wake_phrase_execution": null
    },
    {
      "session_id": 2,
      "session_kind": "SelectedTextEdit",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrOnly" },
      "recognized_text": "uppercase",
      "committed_text": "HELLO",
      "commit_transport": "ClipboardSelectionReplace",
      "selected_text_execution": {
        "action": "Uppercase",
        "strategy": "temporary executor applied uppercase formatting"
      },
      "wake_phrase_execution": null
    },
    {
      "session_id": 3,
      "session_kind": "SelectedTextEdit",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrOnly" },
      "recognized_text": "numbered list",
      "committed_text": "1. first",
      "commit_transport": "ClipboardSelectionReplace",
      "selected_text_execution": {
        "action": "NumberedList",
        "strategy": "temporary executor reformatted the selection as a numbered list"
      },
      "wake_phrase_execution": null
    },
    {
      "session_id": 4,
      "session_kind": "SelectedTextEdit",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrOnly" },
      "recognized_text": "uppercase",
      "committed_text": "WORLD",
      "commit_transport": "ClipboardSelectionReplace",
      "selected_text_execution": {
        "action": "Uppercase",
        "strategy": "temporary executor applied uppercase formatting"
      },
      "wake_phrase_execution": null
    },
    {
      "session_id": 5,
      "session_kind": "WakePhraseIntent",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrWithRefine" },
      "recognized_text": "draft an email",
      "committed_text": "Subject: Draft",
      "commit_transport": "DirectUnicodeSendInput",
      "selected_text_execution": null,
      "wake_phrase_execution": {
        "action": "DraftEmail",
        "strategy": "temporary intent executor expanded the request into an email draft"
      }
    },
    {
      "session_id": 6,
      "session_kind": "WakePhraseIntent",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrWithRefine" },
      "recognized_text": "summarize this",
      "committed_text": "Summary",
      "commit_transport": "DirectUnicodeSendInput",
      "selected_text_execution": null,
      "wake_phrase_execution": {
        "action": "Summarize",
        "strategy": "temporary intent executor condensed the request into a summary"
      }
    }
  ],
  "failure_history": []
}"#,
        )
        .expect("report should write");

        let snapshot = build_settings_ui_snapshot(&loaded, None, Some(report_path.as_path()), None)
            .expect("settings snapshot should build");
        let summary = snapshot
            .latest_live_host_summary
            .expect("live host summary should be mirrored");

        let selected_labels = summary
            .selected_text_action_mix
            .iter()
            .map(|entry| (entry.label.as_str(), entry.count))
            .collect::<Vec<_>>();
        let wake_labels = summary
            .wake_phrase_action_mix
            .iter()
            .map(|entry| (entry.label.as_str(), entry.count))
            .collect::<Vec<_>>();

        assert_eq!(
            selected_labels,
            vec![("Uppercase", 2), ("Numbered list", 1)]
        );
        assert_eq!(wake_labels, vec![("Draft email", 1), ("Summarize", 1)]);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn latest_live_host_summary_prefers_host_owned_action_mix_when_present() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-host-owned-action-mix-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let report_path = temp_root.join("live-host-report-host-owned-action-mix.json");
        fs::write(
            &report_path,
            r#"{
  "generated_at_epoch_ms": 100,
  "effective_settings": {
    "dictation_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "edit_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "shortcut_mode": "PushToTalk",
    "refinement_quality": "Balanced",
    "silence_gate_level": 2,
    "diagnostics_verbosity": "Standard",
    "audio_feedback_enabled": true,
    "wake_phrase": { "enabled": true, "phrase": "Hey VoiceFlow" }
  },
  "attempted_sessions": 1,
  "successful_sessions": 1,
  "failed_sessions": 0,
  "dictation_sessions": 0,
  "selected_text_sessions": 1,
  "wake_phrase_sessions": 0,
  "selected_text_action_mix": [
    { "label": "Host owned edit", "count": 4 }
  ],
  "wake_phrase_action_mix": [
    { "label": "Host owned intent", "count": 2 }
  ],
  "failed_with_summary_sessions": 0,
  "failed_before_summary_sessions": 0,
  "recording_failures": 0,
  "recognizing_failures": 0,
  "executing_failures": 0,
  "committing_failures": 0,
  "silence_gate_failures": 0,
  "no_speech_failures": 0,
  "direct_unicode_commits": 0,
  "clipboard_fallback_commits": 0,
  "selection_replace_commits": 1,
  "dominant_failure_mode": null,
  "dominant_failure_guidance": null,
  "commit_path_outlook": "Edit-heavy",
  "commit_path_signal": "Selected-text replacement dominant (1)",
  "commit_path_guidance": "Recent success is flowing through selected-text replacement.",
  "verification_focus": "Edit execution",
  "verification_focus_guidance": "Run another selected-text edit case.",
  "verification_title": "Verify selected-text edit routing",
  "verification_scenario": "Highlight a short phrase and speak an explicit edit instruction.",
  "verification_command": "cargo run -p input-host -- --serve-live",
  "verification_gesture": "Hold Ctrl+Space, say uppercase, release.",
  "last_failure": null,
  "last_failure_summary": null,
  "sessions": [
    {
      "session_id": 4,
      "session_kind": "SelectedTextEdit",
      "final_state": "Committed",
      "recording_start_latency_ms": 250,
      "total_session_latency_ms": 1200,
      "audio_duration_ms": 900,
      "audio_rms_level": 0.01,
      "route_decision": { "route_name": "LocalAsrOnly" },
      "recognized_text": "uppercase",
      "committed_text": "HELLO",
      "commit_transport": "ClipboardSelectionReplace",
      "selected_text_execution": {
        "action": "Uppercase",
        "strategy": "temporary executor applied uppercase formatting"
      },
      "wake_phrase_execution": null
    }
  ],
  "failure_history": []
}"#,
        )
        .expect("report should write");

        let snapshot = build_settings_ui_snapshot(&loaded, None, Some(report_path.as_path()), None)
            .expect("settings snapshot should build");
        let summary = snapshot
            .latest_live_host_summary
            .expect("live host summary should be mirrored");

        assert_eq!(summary.selected_text_action_mix[0].label, "Host owned edit");
        assert_eq!(summary.selected_text_action_mix[0].count, 4);
        assert_eq!(summary.wake_phrase_action_mix[0].label, "Host owned intent");
        assert_eq!(summary.wake_phrase_action_mix[0].count, 2);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn mirrors_selected_text_compatibility_details_from_live_host_report() {
        let loaded = LoadedRuntimeSettings {
            settings: RuntimeSettings::default(),
            path: PathBuf::from(
                r"C:\Users\Alice\AppData\Local\VoiceFlow Speech Input\settings.json",
            ),
            source: RuntimeSettingsSource::File,
            warnings: vec![],
        };
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-settings-bridge-selected-text-compatibility-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let report_path = temp_root.join("live-host-report-selected-text-compatibility.json");
        fs::write(
            &report_path,
            r#"{
  "generated_at_epoch_ms": 100,
  "effective_settings": {
    "dictation_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "edit_shortcut": { "modifiers": ["Control"], "key": "Space" },
    "shortcut_mode": "PushToTalk",
    "refinement_quality": "Balanced",
    "silence_gate_level": 2,
    "diagnostics_verbosity": "Verbose",
    "audio_feedback_enabled": true,
    "wake_phrase": { "enabled": true, "phrase": "Hey VoiceFlow" }
  },
  "attempted_sessions": 2,
  "successful_sessions": 0,
  "failed_sessions": 2,
  "dictation_sessions": 0,
  "selected_text_sessions": 2,
  "wake_phrase_sessions": 0,
  "selected_text_action_mix": [],
  "wake_phrase_action_mix": [],
  "failed_with_summary_sessions": 1,
  "failed_before_summary_sessions": 1,
  "recording_failures": 0,
  "recognizing_failures": 0,
  "executing_failures": 0,
  "committing_failures": 2,
  "silence_gate_failures": 0,
  "no_speech_failures": 0,
  "direct_unicode_commits": 0,
  "clipboard_fallback_commits": 0,
  "selection_replace_commits": 0,
  "selected_text_compatibility_signal": "Ctrl+C capture timed out (2)",
  "selected_text_compatibility_guidance": "The focused app did not expose selected text through synthetic Ctrl+C within the prototype timeout window.",
  "dominant_failure_mode": "Committing",
  "dominant_failure_guidance": "Speech is making it through recognition, but the final insertion path is failing in the target app. This points to app-compatibility or commit-transport issues.",
  "commit_path_outlook": null,
  "commit_path_signal": null,
  "commit_path_guidance": null,
  "verification_focus": "Edit execution",
  "verification_focus_guidance": "Run another selected-text edit case.",
  "verification_title": "Verify selected-text edit routing",
  "verification_scenario": "Highlight a short phrase and speak an explicit edit instruction.",
  "verification_command": "cargo run -p input-host -- --serve-live",
  "verification_gesture": "Hold Ctrl+Space, say uppercase, release.",
  "verification_example": "Try the same selected-text case in the real target app before trusting the prototype.",
  "verification_note": "Selected-text remains temporary and app-dependent.",
  "workload_focus": null,
  "workload_focus_guidance": null,
  "last_failure": "selected-text compatibility [SelectionCopy/CopyTimedOut]: capture attempt 2 of 2 timed out after 1500 ms",
  "last_failure_summary": {
    "session_id": 7,
    "session_kind": "SelectedTextEdit",
    "failure_phase": "Committing",
    "audio_duration_ms": 900,
    "audio_peak_level": 0.02,
    "audio_rms_level": 0.01,
    "error": "selected-text compatibility [SelectionCopy/CopyTimedOut]: capture attempt 2 of 2 timed out after 1500 ms",
    "selected_text_compatibility": {
      "stage": "SelectionCopy",
      "reason": "Ctrl+C capture timed out",
      "guidance": "The focused app did not expose selected text through synthetic Ctrl+C within the prototype timeout window.",
      "detail": "capture attempt 2 of 2 timed out after 1500 ms"
    }
  },
  "sessions": [],
  "failure_history": [
    {
      "attempted_session_index": 2,
      "session_id": 7,
      "session_kind": "SelectedTextEdit",
      "failure_phase": "Committing",
      "error": "selected-text compatibility [SelectionCopy/CopyTimedOut]: capture attempt 2 of 2 timed out after 1500 ms",
      "selected_text_compatibility": {
        "stage": "SelectionCopy",
        "reason": "Ctrl+C capture timed out",
        "guidance": "The focused app did not expose selected text through synthetic Ctrl+C within the prototype timeout window.",
        "detail": "capture attempt 2 of 2 timed out after 1500 ms"
      }
    }
  ]
}"#,
        )
        .expect("report should write");

        let snapshot = build_settings_ui_snapshot(&loaded, None, Some(report_path.as_path()), None)
            .expect("settings snapshot should build");
        let summary = snapshot
            .latest_live_host_summary
            .expect("live host summary should be mirrored");
        let failure = summary
            .last_failure_summary
            .expect("failure summary should be mirrored");

        assert_eq!(
            summary.selected_text_compatibility_signal.as_deref(),
            Some("Ctrl+C capture timed out (2)")
        );
        assert_eq!(
            summary.selected_text_compatibility_guidance.as_deref(),
            Some(
                "The focused app did not expose selected text through synthetic Ctrl+C within the prototype timeout window."
            )
        );
        assert_eq!(
            failure
                .selected_text_compatibility
                .as_ref()
                .map(|item| item.reason.as_str()),
            Some("Ctrl+C capture timed out")
        );
        assert_eq!(
            summary
                .recent_failures
                .first()
                .and_then(|item| item.selected_text_compatibility.as_ref())
                .map(|item| item.stage.as_str()),
            Some("SelectionCopy")
        );

        let _ = fs::remove_dir_all(&temp_root);
    }
}
