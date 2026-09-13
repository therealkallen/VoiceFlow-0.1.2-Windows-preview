use diagnostics::DiagnosticsStore;
use intent_edit_executor::{
    InstructedDictationExecutor, SelectedTextExecutor, WakePhraseIntentExecutor,
};
use shared_protocol::{
    CapturedAudio, CommitResult, CommitStatus, CommitTransport, DICTATION_LOCAL_THRESHOLD,
    DICTATION_STRUCTURED_THRESHOLD, DiagnosticCategory, DiagnosticEvent, DiagnosticsVerbosity,
    DictationRefinementMode, EngineRequest, FailedSessionSummary,
    InstructedDictationExecutionSummary, InstructedDictationParseOutcome,
    InstructedDictationTransformRequest, ModeOutcome, OverlayStatus, ProviderSettings,
    RefineProvider, RouteDecision, RouteName, RuntimeSettings, SelectedTextExecutionRequest,
    SelectedTextExecutionSummary, SessionFailurePhase, SessionKind, SessionState, SessionSummary,
    ShortcutMode, TriggerMode, WakePhraseIntentExecutionRequest, WakePhraseIntentExecutionSummary,
    parse_instructed_dictation, refinement_model_profile_for_quality, shortcut_to_string,
};
use speech_engine::{
    ProviderKeySource, ResolvedProvider, SpeechEngine, resolve_provider_config,
    runtime_provider_config_from_resolved,
};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::insertion_text::prepare_plain_text_for_insertion;
use crate::provider_key_store::{
    MissingProviderCredentialStore, ProviderCredentialStore, production_provider_credential_store,
    resolve_provider_config_with_credentials,
};
use crate::selected_text_compatibility::{
    SelectedTextFailureReason, SelectedTextFailureStage, format_selected_text_failure,
};
use crate::settings_store::load_runtime_settings;
use crate::text_count::count_words_entered;

const REDACTED_SELECTED_TEXT_PROVIDER_EDIT: &str = "[redacted selected-text provider edit]";
const REDACTED_INSTRUCTED_DICTATION_SOURCE: &str = "[redacted instructed dictation source]";

fn resolved_provider_cache_candidate(resolved: ResolvedProvider) -> Option<ResolvedProvider> {
    resolved.config.is_some().then_some(resolved)
}

fn provider_resolution_can_use_last_known_good(resolved: &ResolvedProvider) -> bool {
    resolved.config.is_none() && resolved.metadata.key_source != ProviderKeySource::Missing
}

fn provider_settings_from_resolved(resolved: &ResolvedProvider) -> ProviderSettings {
    ProviderSettings {
        preset: resolved.metadata.preset.clone(),
        base_url: resolved.metadata.base_url.clone(),
        active_model: Some(resolved.profile.model_code.clone()),
        request_timeout_ms: Some(resolved.metadata.request_timeout_ms),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SilenceGateProfile {
    min_activity_duration_ms: u64,
    max_silence_peak_level: f32,
    max_silence_rms_level: f32,
}

pub trait ShortcutAdapter {
    fn register(&self, settings: &RuntimeSettings) -> Result<(), String>;

    fn rebind(&self, _current: &RuntimeSettings, updated: &RuntimeSettings) -> Result<(), String> {
        self.register(updated)
    }

    fn wait_for_trigger(&self) -> Result<TriggerMode, String> {
        Ok(TriggerMode::GlobalShortcut)
    }

    fn wait_for_trigger_timeout(&self, _timeout: Duration) -> Result<Option<TriggerMode>, String> {
        self.wait_for_trigger().map(Some)
    }

    fn wait_for_trigger_release(
        &self,
        _trigger_mode: &TriggerMode,
        _timeout: Option<Duration>,
    ) -> Result<bool, String> {
        Ok(true)
    }
}

pub trait AudioCaptureAdapter {
    fn snapshot_source(&self) -> Option<speech_engine::AudioSnapshot> {
        None
    }
    fn prepare(&self) -> AudioPreparationSummary {
        AudioPreparationSummary {
            prepare_succeeded: true,
            ..AudioPreparationSummary::default()
        }
    }

    fn start(&self, session_id: u64) -> Result<AudioStartSummary, String>;
    fn stop(&self, session_id: u64) -> Result<CapturedAudio, String>;
}

pub trait TextInsertionAdapter {
    fn commit_text(&self, session_id: u64, text: &str) -> CommitResult;

    fn capture_selected_text(&self, _session_id: u64) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn capture_selected_text_fast(&self, session_id: u64) -> Result<Option<String>, String> {
        self.capture_selected_text(session_id)
    }

    fn replace_selection(&self, session_id: u64, text: &str) -> CommitResult {
        self.commit_text(session_id, text)
    }

    fn update_settings(&self, _settings: &RuntimeSettings) -> Result<(), String> {
        Ok(())
    }
}

pub trait OverlayAdapter {
    fn publish(&self, status: OverlayStatus) -> OverlayPublishSummary;

    fn publish_summary(&self, _summary: &SessionSummary) {}

    fn publish_failure(&self, _failure: &FailedSessionSummary) {}

    fn update_settings(&self, _settings: &RuntimeSettings) {}
}

pub trait AudioFeedbackAdapter {
    fn session_armed(&self, _session_id: u64, _session_kind: &SessionKind) {}

    fn recording_stopped(&self, _session_id: u64, _session_kind: &SessionKind) {}

    fn session_failed(&self, _session_id: u64, _session_kind: &SessionKind) {}
}

#[derive(Debug, Default)]
pub struct NoopShortcutAdapter;

impl ShortcutAdapter for NoopShortcutAdapter {
    fn register(&self, _settings: &RuntimeSettings) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NoopAudioCaptureAdapter;

impl AudioCaptureAdapter for NoopAudioCaptureAdapter {
    fn start(&self, _session_id: u64) -> Result<AudioStartSummary, String> {
        Ok(AudioStartSummary::default())
    }

    fn stop(&self, _session_id: u64) -> Result<CapturedAudio, String> {
        let sample_rate_hz = 16_000;
        let duration_ms = 1_200_u64;
        let sample_count = ((sample_rate_hz as u64) * duration_ms / 1000) as usize;
        let samples = (0..sample_count)
            .map(|index| {
                let cycle = index % 32;
                if cycle < 16 { 0.06 } else { -0.06 }
            })
            .collect();
        Ok(CapturedAudio::from_samples(sample_rate_hz, 1, samples))
    }
}

#[derive(Debug, Default)]
pub struct StubTextInsertionAdapter;

impl TextInsertionAdapter for StubTextInsertionAdapter {
    fn commit_text(&self, _session_id: u64, _text: &str) -> CommitResult {
        CommitResult {
            status: CommitStatus::TemporaryStub,
            transport: CommitTransport::TemporaryStub,
        }
    }
}

#[derive(Debug, Default)]
pub struct StdoutOverlayAdapter;

impl OverlayAdapter for StdoutOverlayAdapter {
    fn publish(&self, status: OverlayStatus) -> OverlayPublishSummary {
        println!(
            "[overlay] session={} kind={:?} state={:?} detail={}",
            status.session_id, status.session_kind, status.state, status.detail
        );
        OverlayPublishSummary::default()
    }
}

#[derive(Debug, Default)]
pub struct NoopAudioFeedbackAdapter;

impl AudioFeedbackAdapter for NoopAudioFeedbackAdapter {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeechPreparationSummary {
    pub prepare_ms: u64,
    pub prepare_succeeded: bool,
    pub backend: Option<String>,
    pub cold_start: Option<bool>,
    pub model_load_ms: Option<u32>,
    pub model_warmup_ms: Option<u32>,
    pub model_warmup_succeeded: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OverlayPublishSummary {
    pub window_create_ms: u64,
    pub webview_create_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioPreparationSummary {
    pub prepare_ms: u64,
    pub prepare_succeeded: bool,
    pub backend_create_ms: u64,
    pub device_discovery_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioStartSummary {
    pub total_ms: u64,
    pub backend_create_ms: u64,
    pub device_discovery_ms: u64,
    pub capture_start_ms: u64,
}

pub struct HostRuntime<S, A, T, O, F, E, W, I>
where
    S: ShortcutAdapter,
    A: AudioCaptureAdapter,
    T: TextInsertionAdapter,
    O: OverlayAdapter,
    F: AudioFeedbackAdapter,
    E: SelectedTextExecutor,
    W: WakePhraseIntentExecutor,
    I: InstructedDictationExecutor,
{
    settings: RuntimeSettings,
    last_known_good_provider: Option<ResolvedProvider>,
    provider_credential_store: Arc<dyn ProviderCredentialStore>,
    diagnostics: DiagnosticsStore,
    speech_engine: SpeechEngine,
    shortcut_adapter: S,
    audio_adapter: A,
    insertion_adapter: T,
    overlay_adapter: O,
    audio_feedback_adapter: F,
    selected_text_executor: E,
    wake_phrase_intent_executor: W,
    instructed_dictation_executor: I,
    session_history: Vec<SessionSummary>,
    next_session_id: u64,
    wake_phrase_revision: u64,
}

impl<S, A, T, O, F, E, W, I> HostRuntime<S, A, T, O, F, E, W, I>
where
    S: ShortcutAdapter,
    A: AudioCaptureAdapter,
    T: TextInsertionAdapter,
    O: OverlayAdapter,
    F: AudioFeedbackAdapter,
    E: SelectedTextExecutor,
    W: WakePhraseIntentExecutor,
    I: InstructedDictationExecutor,
{
    pub fn new(
        settings: RuntimeSettings,
        shortcut_adapter: S,
        audio_adapter: A,
        insertion_adapter: T,
        overlay_adapter: O,
        audio_feedback_adapter: F,
        selected_text_executor: E,
        wake_phrase_intent_executor: W,
        instructed_dictation_executor: I,
    ) -> Self {
        Self::with_speech_engine_and_credentials(
            settings,
            SpeechEngine::with_local_worker(),
            production_provider_credential_store(),
            shortcut_adapter,
            audio_adapter,
            insertion_adapter,
            overlay_adapter,
            audio_feedback_adapter,
            selected_text_executor,
            wake_phrase_intent_executor,
            instructed_dictation_executor,
        )
    }

    pub fn with_speech_engine(
        settings: RuntimeSettings,
        speech_engine: SpeechEngine,
        shortcut_adapter: S,
        audio_adapter: A,
        insertion_adapter: T,
        overlay_adapter: O,
        audio_feedback_adapter: F,
        selected_text_executor: E,
        wake_phrase_intent_executor: W,
        instructed_dictation_executor: I,
    ) -> Self {
        Self::with_speech_engine_and_credentials(
            settings,
            speech_engine,
            Arc::new(MissingProviderCredentialStore),
            shortcut_adapter,
            audio_adapter,
            insertion_adapter,
            overlay_adapter,
            audio_feedback_adapter,
            selected_text_executor,
            wake_phrase_intent_executor,
            instructed_dictation_executor,
        )
    }

    pub fn with_speech_engine_and_credentials(
        settings: RuntimeSettings,
        speech_engine: SpeechEngine,
        provider_credential_store: Arc<dyn ProviderCredentialStore>,
        shortcut_adapter: S,
        audio_adapter: A,
        insertion_adapter: T,
        overlay_adapter: O,
        audio_feedback_adapter: F,
        selected_text_executor: E,
        wake_phrase_intent_executor: W,
        instructed_dictation_executor: I,
    ) -> Self {
        let initial_provider = resolve_provider_config(&settings);
        Self {
            last_known_good_provider: resolved_provider_cache_candidate(initial_provider),
            provider_credential_store,
            settings,
            diagnostics: DiagnosticsStore::new(),
            speech_engine,
            shortcut_adapter,
            audio_adapter,
            insertion_adapter,
            overlay_adapter,
            audio_feedback_adapter,
            selected_text_executor,
            wake_phrase_intent_executor,
            instructed_dictation_executor,
            session_history: Vec::new(),
            next_session_id: 1,
            wake_phrase_revision: 0,
        }
    }

    pub fn bootstrap(&mut self) -> Result<(), String> {
        self.shortcut_adapter.register(&self.settings)?;
        self.record(
            DiagnosticCategory::Bootstrap,
            None,
            format!(
                "registered dictation shortcut {} and edit shortcut {}",
                shortcut_to_string(&self.settings.dictation_shortcut),
                shortcut_to_string(&self.settings.edit_shortcut)
            ),
        );
        Ok(())
    }

    pub fn apply_runtime_settings(
        &mut self,
        updated: RuntimeSettings,
    ) -> Result<Vec<&'static str>, String> {
        let changed_groups = changed_setting_groups(&self.settings, &updated);
        if changed_groups.is_empty() {
            return Ok(changed_groups);
        }

        self.insertion_adapter.update_settings(&updated)?;
        let shortcut_changed = self.settings.dictation_shortcut != updated.dictation_shortcut
            || self.settings.edit_shortcut != updated.edit_shortcut;
        if shortcut_changed {
            if let Err(error) = self.shortcut_adapter.rebind(&self.settings, &updated) {
                let _ = self.insertion_adapter.update_settings(&self.settings);
                self.record(
                    DiagnosticCategory::Bootstrap,
                    None,
                    "settings_reload_failed changed_setting_groups=shortcut shortcut_rebind_failed"
                        .to_string(),
                );
                return Err(format!(
                    "shortcut_rebind_failed; previous shortcut remains active: {error}"
                ));
            }
        }

        let wake_phrase_changed = self.settings.wake_phrase != updated.wake_phrase;
        self.overlay_adapter.update_settings(&updated);
        self.settings = updated;
        if wake_phrase_changed {
            self.wake_phrase_revision = self.wake_phrase_revision.saturating_add(1);
        }
        self.record(
            DiagnosticCategory::Bootstrap,
            None,
            format!(
                "settings_reload_applied changed_setting_groups={}",
                changed_groups.join(",")
            ),
        );
        Ok(changed_groups)
    }

    pub fn settings(&self) -> &RuntimeSettings {
        &self.settings
    }

    pub fn prepare_audio_capture(&mut self) -> AudioPreparationSummary {
        let summary = self.audio_adapter.prepare();
        self.record(
            DiagnosticCategory::Bootstrap,
            None,
            format!(
                "audio_prepare_ms={} audio_prepare_succeeded={} audio_backend_create_ms={} audio_device_discovery_ms={} microphone_capture_started=false",
                summary.prepare_ms,
                summary.prepare_succeeded,
                summary.backend_create_ms,
                summary.device_discovery_ms,
            ),
        );
        summary
    }

    pub fn prepare_speech_engine(&mut self) -> SpeechPreparationSummary {
        let speech_engine_ready_started_at = Instant::now();
        match self.speech_engine.prepare() {
            Ok(Some(preparation)) => {
                let prepare_latency_ms = speech_engine_ready_started_at
                    .elapsed()
                    .as_millis()
                    .min(u64::MAX as u128) as u64;
                self.record(
                    DiagnosticCategory::Bootstrap,
                    None,
                    format!(
                        "prepared ASR backend {} in {}ms (cold_start={}, worker_model_load_ms={:?}, worker_model_warmup_ms={:?}, worker_model_warmup_succeeded={:?})",
                        preparation.backend,
                        prepare_latency_ms,
                        preparation.cold_start,
                        preparation.worker_model_load_ms,
                        preparation.worker_model_warmup_ms,
                        preparation.worker_model_warmup_succeeded
                    ),
                );
                SpeechPreparationSummary {
                    prepare_ms: prepare_latency_ms,
                    prepare_succeeded: true,
                    backend: Some(preparation.backend),
                    cold_start: Some(preparation.cold_start),
                    model_load_ms: preparation.worker_model_load_ms,
                    model_warmup_ms: preparation.worker_model_warmup_ms,
                    model_warmup_succeeded: preparation.worker_model_warmup_succeeded,
                }
            }
            Ok(None) => {
                let prepare_latency_ms = speech_engine_ready_started_at
                    .elapsed()
                    .as_millis()
                    .min(u64::MAX as u128) as u64;
                self.record(
                    DiagnosticCategory::Bootstrap,
                    None,
                    "speech engine reported no bootstrap preparation work".to_string(),
                );
                SpeechPreparationSummary {
                    prepare_ms: prepare_latency_ms,
                    prepare_succeeded: true,
                    backend: None,
                    cold_start: None,
                    model_load_ms: None,
                    model_warmup_ms: None,
                    model_warmup_succeeded: None,
                }
            }
            Err(error) => {
                let prepare_latency_ms = speech_engine_ready_started_at
                    .elapsed()
                    .as_millis()
                    .min(u64::MAX as u128) as u64;
                self.record(
                    DiagnosticCategory::Bootstrap,
                    None,
                    format!(
                        "speech engine preparation failed after {}ms; live sessions will retry on demand: {}",
                        prepare_latency_ms, error
                    ),
                );
                SpeechPreparationSummary {
                    prepare_ms: prepare_latency_ms,
                    prepare_succeeded: false,
                    backend: None,
                    cold_start: None,
                    model_load_ms: None,
                    model_warmup_ms: None,
                    model_warmup_succeeded: None,
                }
            }
        }
    }

    fn current_provider_for_request<Loader>(
        &mut self,
        session_id: u64,
        loader: Loader,
    ) -> ResolvedProvider
    where
        Loader: FnOnce() -> Result<crate::settings_store::LoadedRuntimeSettings, String>,
    {
        match loader() {
            Ok(loaded) => {
                let resolved = resolve_provider_config_with_credentials(
                    &loaded.settings,
                    self.provider_credential_store.as_ref(),
                );
                if resolved.config.is_some() {
                    self.last_known_good_provider = Some(resolved.clone());
                    resolved
                } else if provider_resolution_can_use_last_known_good(&resolved) {
                    self.last_known_good_provider.clone().unwrap_or(resolved)
                } else {
                    self.last_known_good_provider = None;
                    resolved
                }
            }
            Err(error) => {
                self.record(
                    DiagnosticCategory::Routing,
                    Some(session_id),
                    format!(
                        "provider settings reload failed; using last-known-good settings for this session: {error}"
                    ),
                );
                self.last_known_good_provider
                    .clone()
                    .unwrap_or_else(|| resolve_provider_config(&self.settings))
            }
        }
    }

    fn engine_request_for_recognition_result(
        &mut self,
        recognition_request: &EngineRequest,
        recognition: &speech_engine::RecognitionOutput,
        load_runtime_settings: impl FnOnce() -> Result<
            crate::settings_store::LoadedRuntimeSettings,
            String,
        >,
    ) -> EngineRequest {
        let mut routed_request = recognition_request.clone();
        routed_request.dictation_routing = self
            .speech_engine
            .dictation_routing_decision(recognition_request, recognition);
        if routed_request
            .dictation_routing
            .as_ref()
            .is_none_or(|routing| routing.refinement_mode == DictationRefinementMode::LocalOnly)
        {
            return routed_request;
        }

        let resolved_provider = self
            .current_provider_for_request(recognition_request.session_id, load_runtime_settings);
        EngineRequest {
            session_id: recognition_request.session_id,
            requested_kind: recognition_request.requested_kind.clone(),
            captured_audio: recognition_request.captured_audio.clone(),
            transcript_hint: recognition_request.transcript_hint.clone(),
            refinement_profile: resolved_provider.profile.clone(),
            provider_settings: provider_settings_from_resolved(&resolved_provider),
            provider_runtime_config: runtime_provider_config_from_resolved(&resolved_provider),
            dictation_routing: routed_request.dictation_routing,
        }
    }

    pub fn wait_for_trigger(&self) -> Result<TriggerMode, String> {
        self.shortcut_adapter.wait_for_trigger()
    }

    pub fn wait_for_trigger_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<TriggerMode>, String> {
        self.shortcut_adapter.wait_for_trigger_timeout(timeout)
    }

    pub fn run_milestone_one_demo(&mut self) -> Result<SessionSummary, String> {
        self.run_session_for_trigger_with_hint(
            TriggerMode::GlobalShortcut,
            default_transcript_for_trigger(&TriggerMode::GlobalShortcut),
        )
        .map_err(|error| error.to_string())
    }

    #[cfg(test)]
    pub fn run_session_for_trigger(
        &mut self,
        trigger_mode: TriggerMode,
    ) -> Result<SessionSummary, FailedSessionSummary> {
        self.run_session_for_trigger_with_capture_window_and_hint(trigger_mode, None, None)
    }

    pub fn run_session_for_trigger_with_capture_window(
        &mut self,
        trigger_mode: TriggerMode,
        capture_window: Option<Duration>,
    ) -> Result<SessionSummary, FailedSessionSummary> {
        self.run_session_for_trigger_with_capture_window_and_hint(
            trigger_mode,
            capture_window,
            None,
        )
    }

    pub fn run_session_for_trigger_with_hint(
        &mut self,
        trigger_mode: TriggerMode,
        transcript_hint: String,
    ) -> Result<SessionSummary, FailedSessionSummary> {
        self.run_session_for_trigger_with_stop_strategy_and_hint(
            trigger_mode,
            RecordingStopStrategy::Immediate,
            Some(transcript_hint),
        )
    }

    pub fn run_live_session_for_trigger(
        &mut self,
        trigger_mode: TriggerMode,
        max_capture_window: Option<Duration>,
    ) -> Result<SessionSummary, FailedSessionSummary> {
        let stop_strategy = if let Some(window) = max_capture_window {
            RecordingStopStrategy::MatchingTriggerOrTimeout {
                expected_trigger: trigger_mode.clone(),
                timeout: window,
            }
        } else {
            RecordingStopStrategy::MatchingTrigger(trigger_mode.clone())
        };

        self.run_session_for_trigger_with_stop_strategy_and_hint(trigger_mode, stop_strategy, None)
    }

    fn run_session_for_trigger_with_capture_window_and_hint(
        &mut self,
        trigger_mode: TriggerMode,
        capture_window: Option<Duration>,
        transcript_hint: Option<String>,
    ) -> Result<SessionSummary, FailedSessionSummary> {
        let stop_strategy = match capture_window {
            Some(window) => RecordingStopStrategy::Timed(window),
            None => RecordingStopStrategy::Immediate,
        };

        self.run_session_for_trigger_with_stop_strategy_and_hint(
            trigger_mode,
            stop_strategy,
            transcript_hint,
        )
    }

    fn run_session_for_trigger_with_stop_strategy_and_hint(
        &mut self,
        trigger_mode: TriggerMode,
        stop_strategy: RecordingStopStrategy,
        transcript_hint: Option<String>,
    ) -> Result<SessionSummary, FailedSessionSummary> {
        let session_started_at = Instant::now();
        let session_id = self.allocate_session_id();
        let wake_phrase_snapshot = self.settings.wake_phrase.clone();
        let wake_phrase_revision = self.wake_phrase_revision;
        let wake_phrase_source = wake_phrase_source(&wake_phrase_snapshot);
        let wake_phrase_normalized_chars = normalized_wake_phrase_char_count(&wake_phrase_snapshot);
        let mut requested_kind = match trigger_mode {
            TriggerMode::GlobalShortcut => SessionKind::Dictation,
            TriggerMode::EditShortcut => SessionKind::SelectedTextEdit,
        };
        let delay_start_feedback_until_recording =
            matches!(self.settings.shortcut_mode, ShortcutMode::PushToTalk);

        let overlay_publish_started_at = Instant::now();
        let overlay_publish = self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::Arming,
            "Session armed",
        );
        let overlay_state_publish_ms = elapsed_millis(overlay_publish_started_at);
        let mut start_feedback_latency_ms = None;
        let mut recording_start_feedback_ms = 0_u64;
        if !delay_start_feedback_until_recording {
            let feedback_started_at = Instant::now();
            start_feedback_latency_ms =
                self.emit_session_armed_feedback(session_id, &requested_kind, session_started_at);
            recording_start_feedback_ms = elapsed_millis(feedback_started_at);
        }
        self.record(
            DiagnosticCategory::SessionLifecycle,
            Some(session_id),
            format!("session created via {:?}", trigger_mode),
        );
        if let Some(latency_ms) = start_feedback_latency_ms {
            self.record(
                DiagnosticCategory::SessionLifecycle,
                Some(session_id),
                format!("start feedback latency={}ms", latency_ms),
            );
        }

        let recording_context_prepare_started_at = Instant::now();
        let mut selected_text_context = if matches!(trigger_mode, TriggerMode::EditShortcut) {
            self.publish(
                session_id,
                requested_kind.clone(),
                SessionState::Arming,
                "Capturing selected-text context",
            );

            let probe_started_at = Instant::now();
            let capture_result = self.insertion_adapter.capture_selected_text(session_id);
            self.record(
                DiagnosticCategory::Commit,
                Some(session_id),
                selected_text_probe_diagnostic(
                    "strict",
                    elapsed_millis(probe_started_at),
                    &capture_result,
                    "explicit selected-text edit shortcut requires reliable selection capture",
                ),
            );
            match capture_result.map_err(|error| {
                self.build_and_publish_failure(
                    session_id,
                    &requested_kind,
                    SessionFailurePhase::Arming,
                    format!("selected-text context capture failed: {error}"),
                    None,
                )
            })? {
                Some(text) if !text.trim().is_empty() => {
                    self.record(
                        DiagnosticCategory::Commit,
                        Some(session_id),
                        format!(
                            "captured selected-text context with {} characters",
                            text.chars().count()
                        ),
                    );
                    Some(text)
                }
                _ => {
                    self.record(
                        DiagnosticCategory::Commit,
                        Some(session_id),
                        "selected-text edit session did not capture usable text".to_string(),
                    );
                    self.publish(
                        session_id,
                        requested_kind.clone(),
                        SessionState::Failed,
                        "No selected-text context was captured",
                    );
                    self.emit_failure_feedback(session_id, &requested_kind);
                    return Err(self.build_and_publish_failure(
                        session_id,
                        &requested_kind,
                        SessionFailurePhase::Arming,
                        "no selected-text context was captured from the focused app".to_string(),
                        None,
                    ));
                }
            }
        } else if matches!(trigger_mode, TriggerMode::GlobalShortcut) {
            if matches!(self.settings.shortcut_mode, ShortcutMode::PushToTalk) {
                self.record(
                    DiagnosticCategory::SessionLifecycle,
                    Some(session_id),
                    selected_text_probe_diagnostic(
                        "skipped",
                        0,
                        &Ok(None),
                        "push-to-talk skips selection probing before recording and allows only a fast probe after release",
                    ),
                );
                None
            } else {
                let probe_started_at = Instant::now();
                let capture_result = self
                    .insertion_adapter
                    .capture_selected_text_fast(session_id);
                self.record(
                    DiagnosticCategory::Commit,
                    Some(session_id),
                    selected_text_probe_diagnostic(
                        "fast",
                        elapsed_millis(probe_started_at),
                        &capture_result,
                        "ordinary primary-shortcut session uses a best-effort selection probe",
                    ),
                );
                match capture_result {
                    Ok(Some(text)) if !text.trim().is_empty() => {
                        requested_kind = SessionKind::SelectedTextEdit;
                        self.record(
                        DiagnosticCategory::SessionLifecycle,
                        Some(session_id),
                        format!(
                            "selected-text context detected via the primary shortcut ({} characters)",
                            text.chars().count()
                        ),
                    );
                        Some(text)
                    }
                    Ok(_) => None,
                    Err(error) => {
                        self.record(
                        DiagnosticCategory::SessionLifecycle,
                        Some(session_id),
                        format!(
                            "primary shortcut could not capture selected-text context; continuing as dictation: {}",
                            error
                        ),
                    );
                        None
                    }
                }
            }
        } else {
            None
        };
        let recording_context_prepare_ms = elapsed_millis(recording_context_prepare_started_at);

        let audio_start = self.audio_adapter.start(session_id).map_err(|error| {
            self.build_and_publish_failure(
                session_id,
                &requested_kind,
                SessionFailurePhase::Arming,
                format!("microphone capture could not start: {error}"),
                None,
            )
        })?;
        let recording_start_latency_ms = Some(
            session_started_at
                .elapsed()
                .as_millis()
                .min(u64::MAX as u128) as u64,
        );
        let recording_start_accounted_ms = overlay_state_publish_ms
            .saturating_add(recording_start_feedback_ms)
            .saturating_add(recording_context_prepare_ms)
            .saturating_add(audio_start.total_ms);
        let remaining_recording_start_ms = recording_start_latency_ms
            .unwrap_or_default()
            .saturating_sub(recording_start_accounted_ms);
        let recording_start_summary = format!(
            "recording_start_latency_ms={} overlay_state_publish_ms={} overlay_window_create_ms={} overlay_webview_create_ms={} recording_start_feedback_ms={} recording_context_prepare_ms={} audio_start_total_ms={} audio_backend_create_ms={} audio_device_discovery_ms={} audio_capture_start_ms={} remaining_recording_start_ms={}",
            recording_start_latency_ms.unwrap_or_default(),
            overlay_state_publish_ms,
            overlay_publish.window_create_ms,
            overlay_publish.webview_create_ms,
            recording_start_feedback_ms,
            recording_context_prepare_ms,
            audio_start.total_ms,
            audio_start.backend_create_ms,
            audio_start.device_discovery_ms,
            audio_start.capture_start_ms,
            remaining_recording_start_ms,
        );
        eprintln!("[voiceflow-latency] {recording_start_summary}");
        self.record(
            DiagnosticCategory::SessionLifecycle,
            Some(session_id),
            recording_start_summary,
        );
        if delay_start_feedback_until_recording {
            start_feedback_latency_ms =
                self.emit_session_armed_feedback(session_id, &requested_kind, session_started_at);
            if let Some(latency_ms) = start_feedback_latency_ms {
                self.record(
                    DiagnosticCategory::SessionLifecycle,
                    Some(session_id),
                    format!("start feedback latency={}ms", latency_ms),
                );
            }
        }
        self.record(
            DiagnosticCategory::SessionLifecycle,
            Some(session_id),
            format!(
                "recording started {}ms after the session trigger path began",
                recording_start_latency_ms.unwrap_or_default()
            ),
        );
        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::Recording,
            "Microphone capture started",
        );

        let recording_recognition = self
            .audio_adapter
            .snapshot_source()
            .and_then(|snapshot| self.speech_engine.start_recording(session_id, snapshot));
        if let Err(mut error) = self.await_recording_stop(session_id, &requested_kind, &stop_strategy) {
            // The capture stream is owned by the adapter, not this stack frame.
            // Returning without stopping it would keep recording after failure.
            if let Err(cleanup_error) = self.audio_adapter.stop(session_id) {
                error.push_str(&format!("; microphone cleanup failed: {cleanup_error}"));
            }
            return Err(self.build_and_publish_failure(
                session_id,
                &requested_kind,
                SessionFailurePhase::Recording,
                error,
                None,
            ));
        }

        let post_recording_started_at = Instant::now();
        let audio_stop_started_at = Instant::now();
        let captured_audio = self.audio_adapter.stop(session_id).map_err(|error| {
            self.build_and_publish_failure(
                session_id,
                &requested_kind,
                SessionFailurePhase::Recording,
                format!("microphone capture could not stop cleanly: {error}"),
                None,
            )
        })?;
        let audio_stop_finalize_ms = elapsed_millis(audio_stop_started_at);
        let post_stop_prepare_started_at = Instant::now();
        let recording_feedback_started_at = Instant::now();
        self.emit_recording_stopped_feedback(session_id, &requested_kind);
        let recording_feedback_ms = elapsed_millis(recording_feedback_started_at);
        self.record(
            DiagnosticCategory::Audio,
            Some(session_id),
            format!(
                "captured {} samples at {} Hz across {} channel(s) in {} ms with peak {:.3} and rms {:.3}",
                captured_audio.sample_count,
                captured_audio.sample_rate_hz,
                captured_audio.channels,
                captured_audio.duration_ms,
                captured_audio.peak_level
                ,
                captured_audio.rms_level
            ),
        );
        let mut selected_text_probe_ms = 0_u64;
        if matches!(trigger_mode, TriggerMode::GlobalShortcut)
            && matches!(self.settings.shortcut_mode, ShortcutMode::PushToTalk)
            && selected_text_context.is_none()
        {
            self.record(
                DiagnosticCategory::SessionLifecycle,
                Some(session_id),
                "attempting fast selected-text probe after push-to-talk recording stopped"
                    .to_string(),
            );
            let probe_started_at = Instant::now();
            let capture_result = self
                .insertion_adapter
                .capture_selected_text_fast(session_id);
            selected_text_probe_ms = elapsed_millis(probe_started_at);
            self.record(
                DiagnosticCategory::Commit,
                Some(session_id),
                selected_text_probe_diagnostic(
                    "fast",
                    elapsed_millis(probe_started_at),
                    &capture_result,
                    "push-to-talk preserves opportunistic selected-text routing without a strict timeout",
                ),
            );
            match capture_result {
                Ok(Some(text)) if !text.trim().is_empty() => {
                    requested_kind = SessionKind::SelectedTextEdit;
                    self.record(
                        DiagnosticCategory::Commit,
                        Some(session_id),
                        format!(
                            "captured selected-text context after push-to-talk release ({} characters)",
                            text.chars().count()
                        ),
                    );
                    selected_text_context = Some(text);
                }
                Ok(_) => {
                    self.record(
                        DiagnosticCategory::Commit,
                        Some(session_id),
                        "no selected-text context was available after push-to-talk release; continuing as dictation".to_string(),
                    );
                }
                Err(error) => {
                    self.record(
                        DiagnosticCategory::Commit,
                        Some(session_id),
                        format!(
                            "deferred selected-text capture failed after push-to-talk release; continuing as dictation: {}",
                            error
                        ),
                    );
                }
            }
        }
        if looks_like_silence(&captured_audio, self.settings.silence_gate_level) {
            let silence_error = format!(
                "captured audio did not cross the minimum speech activity threshold at silence gate level {} (duration={}ms, peak={:.3}, rms={:.3})",
                self.settings.silence_gate_level,
                captured_audio.duration_ms,
                captured_audio.peak_level,
                captured_audio.rms_level
            );
            self.record(
                DiagnosticCategory::Audio,
                Some(session_id),
                format!(
                    "captured audio looked like silence before ASR at gate level {} (duration={}ms, peak={:.3}, rms={:.3})",
                    self.settings.silence_gate_level,
                    captured_audio.duration_ms,
                    captured_audio.peak_level,
                    captured_audio.rms_level
                ),
            );
            self.publish(
                session_id,
                requested_kind.clone(),
                SessionState::Failed,
                "Captured audio looked like silence before ASR",
            );
            return Err(self.build_and_publish_failure(
                session_id,
                &requested_kind,
                SessionFailurePhase::Recording,
                silence_error,
                Some(&captured_audio),
            ));
        }
        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::Recognizing,
            "Routing audio to speech engine",
        );

        let recognition_request = EngineRequest {
            session_id,
            requested_kind: requested_kind.clone(),
            captured_audio: captured_audio.clone(),
            transcript_hint,
            refinement_profile: refinement_model_profile_for_quality(
                &self.settings.refinement_quality,
            ),
            provider_settings: self.settings.provider.clone(),
            provider_runtime_config: None,
            dictation_routing: None,
        };
        let post_stop_prepare_ms = elapsed_millis(post_stop_prepare_started_at);
        let speech_engine_recognize_started_at = Instant::now();
        let prefetch_join_started_at = Instant::now();
        drop(recording_recognition);
        eprintln!(
            "[voiceflow-asr] prefetch_join_ms={}",
            elapsed_millis(prefetch_join_started_at)
        );
        let recognition = match self.speech_engine.recognize(&recognition_request) {
            Ok(response) => response,
            Err(error) => {
                self.record(
                    DiagnosticCategory::Routing,
                    Some(session_id),
                    format!("speech engine failed: {error}"),
                );
                self.publish(
                    session_id,
                    requested_kind.clone(),
                    SessionState::Failed,
                    "Speech engine failed",
                );
                self.emit_failure_feedback(session_id, &requested_kind);
                return Err(self.build_and_publish_failure(
                    session_id,
                    &requested_kind,
                    SessionFailurePhase::Recognizing,
                    format!("speech engine failed: {error}"),
                    Some(&captured_audio),
                ));
            }
        };
        let speech_engine_recognize_ms = speech_engine_recognize_started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let asr_total_ms = recognition
            .diagnostics
            .as_ref()
            .and_then(|diagnostics| {
                diagnostics
                    .metrics
                    .iter()
                    .find(|metric| metric.name == "total_ms")
            })
            .map(|metric| metric.value_ms as u64)
            .unwrap_or(speech_engine_recognize_ms);
        let post_asr_processing_started_at = Instant::now();
        let mut provider_config_prepare_ms = 0_u64;

        if recognition.transcript.trim().is_empty() {
            let no_speech_error = format!(
                "no speech was recognized from the captured audio (duration={}ms, peak={:.3}, rms={:.3})",
                captured_audio.duration_ms, captured_audio.peak_level, captured_audio.rms_level
            );
            self.record(
                DiagnosticCategory::Routing,
                Some(session_id),
                format!(
                    "speech engine returned an empty transcript after {} ms of audio with peak {:.3}",
                    captured_audio.duration_ms, captured_audio.peak_level
                ),
            );
            self.publish(
                session_id,
                requested_kind.clone(),
                SessionState::Failed,
                "No speech was recognized from the captured audio",
            );
            return Err(self.build_and_publish_failure(
                session_id,
                &requested_kind,
                SessionFailurePhase::Recognizing,
                no_speech_error,
                Some(&captured_audio),
            ));
        }

        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::ModeRouting,
            "Evaluating dictation versus explicit intent mode",
        );

        let mut instructed_dictation_parts = None;
        let (engine_response, mode_outcome) = if selected_text_context.is_some() {
            let mut response = self.speech_engine.recognition_without_refinement(
                recognition,
                "recognized text routed to selected-text edit before dictation refinement",
            );
            response.route_decision.refine_fast_path_reason = "selected_text_edit".to_string();
            let mode_outcome =
                self.resolve_mode(&trigger_mode, &requested_kind, &response.final_text);
            (response, mode_outcome)
        } else if matches!(trigger_mode, TriggerMode::GlobalShortcut) {
            match parse_instructed_dictation(&recognition.transcript, &wake_phrase_snapshot) {
                InstructedDictationParseOutcome::NoTrigger => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!(
                            "wake_phrase_source={wake_phrase_source} wake_phrase_revision={wake_phrase_revision} wake_phrase_normalized_chars={wake_phrase_normalized_chars} instructed_trigger_matched=false instructed_trigger_alias=none instructed_leading_filler_removed=false instructed_task_present=false effective_session_kind=dictation"
                        ),
                    );
                    let response = if matches!(requested_kind, SessionKind::Dictation) {
                        let provider_config_prepare_started_at = Instant::now();
                        let engine_request = self.engine_request_for_recognition_result(
                            &recognition_request,
                            &recognition,
                            load_runtime_settings,
                        );
                        provider_config_prepare_ms =
                            elapsed_millis(provider_config_prepare_started_at);
                        self.speech_engine
                            .process_recognition(&engine_request, recognition)
                    } else {
                        self.speech_engine.recognition_without_refinement(
                            recognition,
                            "recognized text routed to explicit non-dictation mode before dictation refinement",
                        )
                    };
                    let mode_outcome =
                        self.resolve_mode(&trigger_mode, &requested_kind, &response.final_text);
                    (response, mode_outcome)
                }
                InstructedDictationParseOutcome::Parsed(parts) => {
                    let reason = format!(
                        "Instructed dictation trigger matched on raw transcript; wake_phrase_source={wake_phrase_source} wake_phrase_revision={wake_phrase_revision} wake_phrase_normalized_chars={wake_phrase_normalized_chars} parser_result={}; task_chars={}; instructed_trigger_matched=true instructed_trigger_alias={} instructed_leading_filler_removed={} instructed_task_present=true effective_session_kind=instructed_dictation",
                        parts.parser_result,
                        parts.task_text.chars().count(),
                        parts.trigger_alias.as_str(),
                        parts.leading_filler_removed,
                    );
                    instructed_dictation_parts = Some(parts);
                    let mut response = self.speech_engine.recognition_without_refinement(
                        recognition,
                        "recognized text routed to instructed dictation before dictation refinement",
                    );
                    response.route_decision.refine_fast_path_reason =
                        "instructed_dictation".to_string();
                    (
                        response,
                        ModeOutcome {
                            session_kind: SessionKind::InstructedDictation,
                            text_for_commit: String::new(),
                            reason,
                        },
                    )
                }
                InstructedDictationParseOutcome::MissingContent(failure) => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!(
                            "instructed dictation trigger matched but parse failed: wake_phrase_source={wake_phrase_source} wake_phrase_revision={wake_phrase_revision} wake_phrase_normalized_chars={wake_phrase_normalized_chars} parser_result={}; task_chars={}; instructed_trigger_matched=true instructed_trigger_alias={} instructed_leading_filler_removed={} instructed_task_present=false effective_session_kind=instructed_dictation",
                            failure.parser_result,
                            failure.task_char_count,
                            failure.trigger_alias.as_str(),
                            failure.leading_filler_removed,
                        ),
                    );
                    self.publish(
                        session_id,
                        SessionKind::InstructedDictation,
                        SessionState::Failed,
                        &failure.reason,
                    );
                    self.emit_failure_feedback(session_id, &SessionKind::InstructedDictation);
                    return Err(self.build_and_publish_failure(
                        session_id,
                        &SessionKind::InstructedDictation,
                        SessionFailurePhase::Executing,
                        failure.reason,
                        Some(&captured_audio),
                    ));
                }
            }
        } else {
            let response = if matches!(requested_kind, SessionKind::Dictation) {
                let provider_config_prepare_started_at = Instant::now();
                let engine_request = self.engine_request_for_recognition_result(
                    &recognition_request,
                    &recognition,
                    load_runtime_settings,
                );
                provider_config_prepare_ms = elapsed_millis(provider_config_prepare_started_at);
                self.speech_engine
                    .process_recognition(&engine_request, recognition)
            } else {
                self.speech_engine.recognition_without_refinement(
                    recognition,
                    "recognized text routed to explicit non-dictation mode before dictation refinement",
                )
            };
            let mode_outcome =
                self.resolve_mode(&trigger_mode, &requested_kind, &response.final_text);
            (response, mode_outcome)
        };
        self.record(
            DiagnosticCategory::Routing,
            Some(session_id),
            format!(
                "route={:?} asr={:?} refinement_applied={} degraded_to_asr={} refine_fast_path_used={} refine_fast_path_reason={} cloud_refine_skipped={} dictation_text_count={} dictation_refinement_mode={} dictation_routing_reason={} self_correction_detected={} dictation_local_threshold={} dictation_structured_threshold={}",
                engine_response.route_decision.route_name,
                engine_response.route_decision.asr_provider,
                engine_response.route_decision.refinement_applied,
                engine_response.degraded_to_asr,
                engine_response.route_decision.refine_fast_path_used,
                engine_response.route_decision.refine_fast_path_reason,
                engine_response.route_decision.cloud_refine_skipped,
                engine_response
                    .route_decision
                    .dictation_routing
                    .as_ref()
                    .map(|routing| routing.text_count.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                engine_response
                    .route_decision
                    .dictation_routing
                    .as_ref()
                    .map(|routing| routing.refinement_mode.as_str())
                    .unwrap_or("none"),
                engine_response
                    .route_decision
                    .dictation_routing
                    .as_ref()
                    .map(|routing| routing.routing_reason.as_str())
                    .unwrap_or("none"),
                engine_response
                    .route_decision
                    .dictation_routing
                    .as_ref()
                    .map(|routing| routing.self_correction_detected.to_string())
                    .unwrap_or_else(|| "false".to_string()),
                DICTATION_LOCAL_THRESHOLD,
                DICTATION_STRUCTURED_THRESHOLD,
            ),
        );
        if let Some(fallback_reason) = &engine_response.fallback_reason {
            self.record(
                DiagnosticCategory::Routing,
                Some(session_id),
                format!("speech fallback: {fallback_reason}"),
            );
        }
        if let Some(asr_diagnostics) = &engine_response.asr_diagnostics {
            let metrics = if asr_diagnostics.metrics.is_empty() {
                "none".to_string()
            } else {
                asr_diagnostics
                    .metrics
                    .iter()
                    .map(|metric| format!("{}={}ms", metric.name, metric.value_ms))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            self.record(
                DiagnosticCategory::Routing,
                Some(session_id),
                format!(
                    "asr backend={} request_id={:?} model_load_ms={:?} restarted={} metrics=[{}]",
                    asr_diagnostics.backend,
                    asr_diagnostics.worker_request_id,
                    asr_diagnostics.worker_model_load_ms,
                    asr_diagnostics.worker_restarted,
                    metrics
                ),
            );
        }
        if let Some(refine_diagnostics) = &engine_response.refine_diagnostics {
            self.record(
                DiagnosticCategory::Routing,
                Some(session_id),
                format!(
                    "refine profile={} model={} provider_preset={} provider_key_source={} provider_configured={} provider_attempted={} provider_succeeded={} deterministic_fallback_used={} prompt_profile={} prompt_source={}",
                    refine_diagnostics.profile_label,
                    refine_diagnostics.model_code,
                    refine_diagnostics
                        .provider_preset
                        .as_deref()
                        .unwrap_or("none"),
                    refine_diagnostics
                        .provider_key_source
                        .as_deref()
                        .unwrap_or("none"),
                    refine_diagnostics.provider_configured,
                    refine_diagnostics.provider_attempted,
                    refine_diagnostics.provider_succeeded,
                    refine_diagnostics.deterministic_fallback_used,
                    refine_diagnostics
                        .prompt_profile
                        .map(|profile| profile.as_str())
                        .unwrap_or("none"),
                    refine_diagnostics
                        .prompt_source
                        .map(|source| source.as_str())
                        .unwrap_or("none"),
                ),
            );
            if let Some(reason) = &refine_diagnostics.fallback_reason {
                self.record(
                    DiagnosticCategory::Routing,
                    Some(session_id),
                    format!("refine fallback: {reason}"),
                );
            }
        }
        let (
            commit_payload,
            selected_text_execution,
            wake_phrase_execution,
            instructed_dictation_execution,
        ) = if let Some(selected_text) = selected_text_context.as_deref() {
            match self
                .selected_text_executor
                .execute(&SelectedTextExecutionRequest {
                    session_id,
                    selected_text: selected_text.to_string(),
                    instruction_text: engine_response.transcript.clone(),
                }) {
                Ok(edit_execution) => {
                    let provider_diagnostics = edit_execution.provider_diagnostics.clone();
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!(
                            "{}; executor_action={:?}; executor_strategy={}; normalized_instruction={}",
                            mode_outcome.reason,
                            edit_execution.action,
                            edit_execution.strategy,
                            edit_execution.normalized_instruction
                        ),
                    );
                    if let Some(provider_diagnostics) = &provider_diagnostics {
                        self.record(
                            DiagnosticCategory::ModeDecision,
                            Some(session_id),
                            format!(
                                "selected-text provider action={:?} profile={} model={} provider_preset={} provider_key_source={} attempted={} succeeded={} deterministic_fallback_used={} selected_text_chars={} output_chars={}",
                                edit_execution.action,
                                provider_diagnostics.profile_label,
                                provider_diagnostics.model_code,
                                provider_diagnostics
                                    .provider_preset
                                    .as_deref()
                                    .unwrap_or("none"),
                                provider_diagnostics
                                    .provider_key_source
                                    .as_deref()
                                    .unwrap_or("none"),
                                provider_diagnostics.provider_attempted,
                                provider_diagnostics.provider_succeeded,
                                provider_diagnostics.deterministic_fallback_used,
                                provider_diagnostics.selected_text_char_count,
                                provider_diagnostics.output_char_count
                            ),
                        );
                        if let Some(reason) = &provider_diagnostics.fallback_reason {
                            self.record(
                                DiagnosticCategory::ModeDecision,
                                Some(session_id),
                                format!("selected-text provider fallback: {reason}"),
                            );
                        }
                    }
                    (
                        edit_execution.output_text,
                        Some(SelectedTextExecutionSummary {
                            action: edit_execution.action,
                            strategy: edit_execution.strategy,
                            provider_diagnostics,
                        }),
                        None,
                        None,
                    )
                }
                Err(error) => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!("{}; executor failed: {}", mode_outcome.reason, error),
                    );
                    self.publish(
                        session_id,
                        mode_outcome.session_kind.clone(),
                        SessionState::Failed,
                        "Selected-text executor could not apply the instruction",
                    );
                    self.emit_failure_feedback(session_id, &mode_outcome.session_kind);
                    return Err(self.build_and_publish_failure(
                        session_id,
                        &mode_outcome.session_kind,
                        SessionFailurePhase::Executing,
                        error,
                        Some(&captured_audio),
                    ));
                }
            }
        } else if matches!(mode_outcome.session_kind, SessionKind::InstructedDictation) {
            let parts = instructed_dictation_parts
                .as_ref()
                .expect("instructed dictation mode should include parsed parts");
            match self.instructed_dictation_executor.transform(
                &InstructedDictationTransformRequest {
                    session_id,
                    instruction_text: parts.task_text.clone(),
                    content_text: String::new(),
                },
            ) {
                Ok(transform) => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!(
                            "{}; provider profile={} model={} provider_preset={} provider_key_source={} attempted={} succeeded={} deterministic_fallback_used={} instruction_chars={} content_chars={} output_chars={}",
                            mode_outcome.reason,
                            transform.provider_diagnostics.profile_label,
                            transform.provider_diagnostics.model_code,
                            transform
                                .provider_diagnostics
                                .provider_preset
                                .as_deref()
                                .unwrap_or("none"),
                            transform
                                .provider_diagnostics
                                .provider_key_source
                                .as_deref()
                                .unwrap_or("none"),
                            transform.provider_diagnostics.provider_attempted,
                            transform.provider_diagnostics.provider_succeeded,
                            transform.provider_diagnostics.deterministic_fallback_used,
                            transform.provider_diagnostics.instruction_char_count,
                            transform.provider_diagnostics.content_char_count,
                            transform.provider_diagnostics.output_char_count
                        ),
                    );
                    if let Some(reason) = &transform.provider_diagnostics.fallback_reason {
                        self.record(
                            DiagnosticCategory::ModeDecision,
                            Some(session_id),
                            format!("instructed dictation provider fallback: {reason}"),
                        );
                    }
                    (
                        transform.output_text,
                        None,
                        None,
                        Some(InstructedDictationExecutionSummary {
                            strategy: transform.strategy,
                            trigger_phrase_matched: true,
                            parser_result: parts.parser_result.clone(),
                            provider_diagnostics: transform.provider_diagnostics,
                        }),
                    )
                }
                Err(error) => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!(
                            "{}; provider transform failed: {}",
                            mode_outcome.reason, error
                        ),
                    );
                    self.publish(
                        session_id,
                        mode_outcome.session_kind.clone(),
                        SessionState::Failed,
                        "Instructed dictation provider could not apply the instruction",
                    );
                    self.emit_failure_feedback(session_id, &mode_outcome.session_kind);
                    return Err(self.build_and_publish_failure(
                        session_id,
                        &mode_outcome.session_kind,
                        SessionFailurePhase::Executing,
                        error,
                        Some(&captured_audio),
                    ));
                }
            }
        } else if matches!(mode_outcome.session_kind, SessionKind::WakePhraseIntent) {
            match self.wake_phrase_intent_executor.execute_intent(
                &WakePhraseIntentExecutionRequest {
                    session_id,
                    command_text: mode_outcome.text_for_commit.clone(),
                },
            ) {
                Ok(intent_execution) => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!(
                            "{}; intent_action={:?}; intent_strategy={}; normalized_command={}",
                            mode_outcome.reason,
                            intent_execution.action,
                            intent_execution.strategy,
                            intent_execution.normalized_command
                        ),
                    );
                    (
                        intent_execution.output_text,
                        None,
                        Some(WakePhraseIntentExecutionSummary {
                            action: intent_execution.action,
                            strategy: intent_execution.strategy,
                        }),
                        None,
                    )
                }
                Err(error) => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!("{}; intent executor failed: {}", mode_outcome.reason, error),
                    );
                    self.publish(
                        session_id,
                        mode_outcome.session_kind.clone(),
                        SessionState::Failed,
                        "Wake-phrase intent executor could not apply the request",
                    );
                    self.emit_failure_feedback(session_id, &mode_outcome.session_kind);
                    return Err(self.build_and_publish_failure(
                        session_id,
                        &mode_outcome.session_kind,
                        SessionFailurePhase::Executing,
                        error,
                        Some(&captured_audio),
                    ));
                }
            }
        } else {
            self.record(
                DiagnosticCategory::ModeDecision,
                Some(session_id),
                mode_outcome.reason.clone(),
            );
            (mode_outcome.text_for_commit.clone(), None, None, None)
        };

        let execution_state = if matches!(
            mode_outcome.session_kind,
            SessionKind::WakePhraseIntent
                | SessionKind::SelectedTextEdit
                | SessionKind::InstructedDictation
        ) {
            SessionState::Executing
        } else {
            SessionState::ReadyToCommit
        };
        self.publish(
            session_id,
            mode_outcome.session_kind.clone(),
            execution_state,
            if matches!(mode_outcome.session_kind, SessionKind::SelectedTextEdit) {
                "Prepared transformed text for replacement"
            } else if matches!(mode_outcome.session_kind, SessionKind::WakePhraseIntent) {
                "Prepared wake-phrase intent result for commit"
            } else if matches!(mode_outcome.session_kind, SessionKind::InstructedDictation) {
                "Prepared instructed dictation result for commit"
            } else {
                "Prepared final text for commit"
            },
        );

        self.publish(
            session_id,
            mode_outcome.session_kind.clone(),
            SessionState::Committing,
            if matches!(mode_outcome.session_kind, SessionKind::SelectedTextEdit) {
                "Replacing selected text in the insertion layer"
            } else if matches!(mode_outcome.session_kind, SessionKind::WakePhraseIntent) {
                "Sending wake-phrase intent result into insertion layer"
            } else if matches!(mode_outcome.session_kind, SessionKind::InstructedDictation) {
                "Sending instructed dictation result into insertion layer"
            } else {
                "Sending final text into insertion layer"
            },
        );

        let insertion_text = prepare_plain_text_for_insertion(&commit_payload);
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "insertion_text_multiline={} insertion_format_normalized={} insertion_line_count={}",
                insertion_text.multiline,
                insertion_text.format_normalized,
                insertion_text.line_count
            ),
        );
        let post_asr_processing_ms = elapsed_millis(post_asr_processing_started_at);
        let insertion_started_at = Instant::now();
        let commit_result = if matches!(mode_outcome.session_kind, SessionKind::SelectedTextEdit) {
            self.insertion_adapter
                .replace_selection(session_id, &insertion_text.text)
        } else {
            self.insertion_adapter
                .commit_text(session_id, &insertion_text.text)
        };
        let insertion_ms = insertion_started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let post_recording_latency_ms = post_recording_started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let refine_diagnostics = engine_response.refine_diagnostics.as_ref();
        let asr_metric = |name: &str| {
            engine_response
                .asr_diagnostics
                .as_ref()
                .and_then(|diagnostics| {
                    diagnostics
                        .metrics
                        .iter()
                        .find(|metric| metric.name == name)
                })
                .map(|metric| u64::from(metric.value_ms))
                .unwrap_or(0)
        };
        let accounted_sequential_ms = audio_stop_finalize_ms
            .saturating_add(post_stop_prepare_ms)
            .saturating_add(speech_engine_recognize_ms)
            .saturating_add(post_asr_processing_ms)
            .saturating_add(insertion_ms);
        let post_recording_unattributed_ms =
            post_recording_latency_ms.saturating_sub(accounted_sequential_ms);
        let provider_latency_fields = refine_diagnostics
            .and_then(|diagnostics| {
                diagnostics
                    .provider_preset
                    .as_deref()
                    .zip(diagnostics.provider_key_source.as_deref())
            })
            .map(|(preset, key_source)| {
                format!(" provider_preset={preset} provider_key_source={key_source}")
            })
            .unwrap_or_default();
        let latency_summary = format!(
            "post_recording_latency_ms={} audio_stop_finalize_ms={} post_stop_prepare_ms={} recording_feedback_ms={} selected_text_probe_ms={} speech_engine_recognize_ms={} asr_total_ms={} asr_host_total_ms={} asr_host_audio_prepare_ms={} asr_host_wav_write_ms={} asr_host_worker_roundtrip_ms={} asr_host_temp_cleanup_ms={} post_asr_processing_ms={} provider_config_prepare_ms={} refine_total_ms={} prompt_load_ms={} payload_build_ms={} provider_request_ms={} provider_output_validation_ms={} insertion_ms={} post_recording_unattributed_ms={} dictation_text_count={} dictation_refinement_mode={} dictation_routing_reason={} self_correction_detected={} dictation_local_threshold={} dictation_structured_threshold={} prompt_profile={} prompt_source={} prompt_char_count={} transcript_char_count={} refine_model={}{} guard_detected={} provider_output_rejected={} fallback_used={}",
            post_recording_latency_ms,
            audio_stop_finalize_ms,
            post_stop_prepare_ms,
            recording_feedback_ms,
            selected_text_probe_ms,
            speech_engine_recognize_ms,
            asr_total_ms,
            asr_metric("host_total_ms"),
            asr_metric("host_audio_prepare_ms"),
            asr_metric("host_wav_write_ms"),
            asr_metric("host_worker_roundtrip_ms"),
            asr_metric("host_temp_cleanup_ms"),
            post_asr_processing_ms,
            provider_config_prepare_ms,
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.refine_total_ms)
                .unwrap_or(0),
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.prompt_load_ms)
                .unwrap_or(0),
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.payload_build_ms)
                .unwrap_or(0),
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.provider_request_ms)
                .unwrap_or(0),
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.provider_output_validation_ms)
                .unwrap_or(0),
            insertion_ms,
            post_recording_unattributed_ms,
            engine_response
                .route_decision
                .dictation_routing
                .as_ref()
                .map(|routing| routing.text_count.to_string())
                .unwrap_or_else(|| "none".to_string()),
            engine_response
                .route_decision
                .dictation_routing
                .as_ref()
                .map(|routing| routing.refinement_mode.as_str())
                .unwrap_or("none"),
            engine_response
                .route_decision
                .dictation_routing
                .as_ref()
                .map(|routing| routing.routing_reason.as_str())
                .unwrap_or("none"),
            engine_response
                .route_decision
                .dictation_routing
                .as_ref()
                .map(|routing| routing.self_correction_detected)
                .unwrap_or(false),
            DICTATION_LOCAL_THRESHOLD,
            DICTATION_STRUCTURED_THRESHOLD,
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.prompt_profile)
                .map(|profile| profile.as_str())
                .unwrap_or("none"),
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.prompt_source)
                .map(|source| source.as_str())
                .unwrap_or("none"),
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.prompt_char_count)
                .unwrap_or(0),
            refine_diagnostics
                .and_then(|diagnostics| diagnostics.transcript_char_count)
                .unwrap_or_else(|| engine_response.transcript.chars().count() as u64),
            refine_diagnostics
                .map(|diagnostics| diagnostics.model_code.as_str())
                .unwrap_or("none"),
            provider_latency_fields,
            refine_diagnostics
                .map(|diagnostics| diagnostics.guard_detected)
                .unwrap_or(false),
            refine_diagnostics
                .map(|diagnostics| diagnostics.provider_output_rejected)
                .unwrap_or(false),
            refine_diagnostics
                .map(|diagnostics| diagnostics.deterministic_fallback_used)
                .unwrap_or(engine_response.degraded_to_asr),
        );
        eprintln!("[voiceflow-latency] {latency_summary}");
        self.record(
            DiagnosticCategory::SessionLifecycle,
            Some(session_id),
            latency_summary,
        );
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "commit result = {:?} via {:?}; commit_transport={:?}",
                commit_result.status, commit_result.transport, commit_result.transport
            ),
        );
        let commit_failure_reason = match &commit_result.status {
            CommitStatus::Failed(error) => Some(error.clone()),
            CommitStatus::NotAttempted => Some("commit was not attempted".to_string()),
            CommitStatus::Success | CommitStatus::TemporaryStub => None,
        };

        let final_state = match commit_result.status {
            CommitStatus::Success | CommitStatus::TemporaryStub => SessionState::Committed,
            CommitStatus::NotAttempted | CommitStatus::Failed(_) => SessionState::Failed,
        };

        let insertion_outcome = format!(
            "session_id={session_id} session_kind={:?} final_state={final_state:?} insertion_text_multiline={} insertion_format_normalized={} insertion_line_count={} commit_transport={:?}",
            mode_outcome.session_kind,
            insertion_text.multiline,
            insertion_text.format_normalized,
            insertion_text.line_count,
            commit_result.transport,
        );
        eprintln!("[input-host][insertion-outcome] {insertion_outcome}");
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            insertion_outcome,
        );

        self.publish(
            session_id,
            mode_outcome.session_kind.clone(),
            final_state.clone(),
            "Session finished",
        );
        if matches!(final_state, SessionState::Failed) {
            self.emit_failure_feedback(session_id, &mode_outcome.session_kind);
        }

        let total_session_latency_ms = session_started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        self.record(
            DiagnosticCategory::SessionLifecycle,
            Some(session_id),
            format!("session finished in {}ms", total_session_latency_ms),
        );
        let committed_text_for_summary = if selected_text_execution
            .as_ref()
            .and_then(|execution| execution.provider_diagnostics.as_ref())
            .is_some()
        {
            REDACTED_SELECTED_TEXT_PROVIDER_EDIT.to_string()
        } else {
            commit_payload.clone()
        };
        let recognized_text_for_summary = if instructed_dictation_execution.is_some() {
            REDACTED_INSTRUCTED_DICTATION_SOURCE.to_string()
        } else {
            engine_response.transcript
        };
        let route_decision_for_summary = if instructed_dictation_execution.is_some() {
            RouteDecision {
                route_name: RouteName::InstructedDictation,
                asr_provider: engine_response.route_decision.asr_provider,
                refine_provider: Some(RefineProvider::Llm),
                refinement_applied: true,
                reason: mode_outcome.reason.clone(),
                refine_fast_path_used: false,
                refine_fast_path_reason: "instructed_dictation".to_string(),
                cloud_refine_skipped: false,
                dictation_routing: None,
            }
        } else {
            engine_response.route_decision
        };

        let summary = SessionSummary {
            session_id,
            session_kind: mode_outcome.session_kind.clone(),
            final_state,
            completed_at_epoch_ms: current_epoch_ms(),
            start_feedback_latency_ms,
            recording_start_latency_ms,
            total_session_latency_ms,
            audio_duration_ms: captured_audio.duration_ms,
            audio_peak_level: captured_audio.peak_level,
            audio_rms_level: captured_audio.rms_level,
            asr_diagnostics: engine_response.asr_diagnostics,
            refine_diagnostics: engine_response.refine_diagnostics,
            recognized_text: recognized_text_for_summary,
            committed_text: committed_text_for_summary,
            committed_text_count: Some(count_words_entered(&commit_payload)),
            route_decision: route_decision_for_summary,
            degraded_to_asr: engine_response.degraded_to_asr,
            fallback_reason: engine_response.fallback_reason,
            commit_status: commit_result.status,
            commit_transport: commit_result.transport,
            commit_failure_reason,
            mode_reason: mode_outcome.reason,
            selected_text_execution,
            wake_phrase_execution,
            instructed_dictation_execution,
        };
        self.session_history.push(summary.clone());
        self.overlay_adapter.publish_summary(&summary);

        Ok(summary)
    }

    pub fn run_audio_capture_probe(
        &mut self,
        capture_window: Duration,
    ) -> Result<CapturedAudio, String> {
        let session_id = self.allocate_session_id();
        self.audio_adapter.start(session_id)?;
        self.publish(
            session_id,
            SessionKind::Dictation,
            SessionState::Recording,
            "Audio capture probe started",
        );
        thread::sleep(capture_window);
        let captured_audio = self.audio_adapter.stop(session_id)?;
        self.record(
            DiagnosticCategory::Audio,
            Some(session_id),
            format!(
                "probe captured {} samples at {} Hz across {} channel(s) in {} ms with peak {:.3}",
                captured_audio.sample_count,
                captured_audio.sample_rate_hz,
                captured_audio.channels,
                captured_audio.duration_ms,
                captured_audio.peak_level
            ),
        );
        self.publish(
            session_id,
            SessionKind::Dictation,
            SessionState::Committed,
            "Audio capture probe finished",
        );
        Ok(captured_audio)
    }

    pub fn run_text_insertion_probe(&mut self, text: &str) -> Result<CommitResult, String> {
        let session_id = self.allocate_session_id();
        self.publish(
            session_id,
            SessionKind::Dictation,
            SessionState::ReadyToCommit,
            "Text insertion probe armed",
        );
        self.publish(
            session_id,
            SessionKind::Dictation,
            SessionState::Committing,
            "Sending probe text into insertion layer",
        );

        let insertion_text = prepare_plain_text_for_insertion(text);
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "insertion_text_multiline={} insertion_format_normalized={} insertion_line_count={}",
                insertion_text.multiline,
                insertion_text.format_normalized,
                insertion_text.line_count
            ),
        );
        let commit_result = self
            .insertion_adapter
            .commit_text(session_id, &insertion_text.text);
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "commit result = {:?} via {:?}; commit_transport={:?}",
                commit_result.status, commit_result.transport, commit_result.transport
            ),
        );

        let final_state = match commit_result.status {
            CommitStatus::Success | CommitStatus::TemporaryStub => SessionState::Committed,
            CommitStatus::NotAttempted | CommitStatus::Failed(_) => SessionState::Failed,
        };
        self.publish(
            session_id,
            SessionKind::Dictation,
            final_state,
            "Text insertion probe finished",
        );

        Ok(commit_result)
    }

    pub fn run_selection_replace_probe(
        &mut self,
        replacement_text: &str,
    ) -> Result<SelectionReplaceProbeResult, String> {
        let session_id = self.allocate_session_id();
        self.publish(
            session_id,
            SessionKind::SelectedTextEdit,
            SessionState::Arming,
            "Selected-text replacement probe armed",
        );

        let selected_text = match self
            .insertion_adapter
            .capture_selected_text(session_id)
            .map_err(|error| {
                self.record(
                    DiagnosticCategory::Commit,
                    Some(session_id),
                    format!(
                        "selected-text capture failed before replacement probe commit: {error}"
                    ),
                );
                self.publish(
                    session_id,
                    SessionKind::SelectedTextEdit,
                    SessionState::Failed,
                    "Selected-text capture failed",
                );
                self.emit_failure_feedback(session_id, &SessionKind::SelectedTextEdit);
                self.build_and_publish_failure(
                    session_id,
                    &SessionKind::SelectedTextEdit,
                    SessionFailurePhase::Arming,
                    format!("selected-text replacement probe capture failed: {error}"),
                    None,
                )
                .to_string()
            })? {
            Some(text) if !text.trim().is_empty() => text,
            _ => {
                let message = format_selected_text_failure(
                    SelectedTextFailureStage::SelectionRead,
                    SelectedTextFailureReason::EmptySelection,
                    "selected-text replacement probe could not capture non-empty text from the focused app",
                );
                self.record(
                    DiagnosticCategory::Commit,
                    Some(session_id),
                    "selected-text replacement probe did not capture usable text".to_string(),
                );
                self.publish(
                    session_id,
                    SessionKind::SelectedTextEdit,
                    SessionState::Failed,
                    "No selected-text context was captured",
                );
                self.emit_failure_feedback(session_id, &SessionKind::SelectedTextEdit);
                let failure = self.build_and_publish_failure(
                    session_id,
                    &SessionKind::SelectedTextEdit,
                    SessionFailurePhase::Arming,
                    message,
                    None,
                );
                return Err(failure.to_string());
            }
        };

        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "captured selected-text context with {} characters",
                selected_text.chars().count()
            ),
        );
        self.publish(
            session_id,
            SessionKind::SelectedTextEdit,
            SessionState::Executing,
            "Selected-text context captured",
        );
        self.publish(
            session_id,
            SessionKind::SelectedTextEdit,
            SessionState::Committing,
            "Replacing the active selection",
        );

        let insertion_text = prepare_plain_text_for_insertion(replacement_text);
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "insertion_text_multiline={} insertion_format_normalized={} insertion_line_count={}",
                insertion_text.multiline,
                insertion_text.format_normalized,
                insertion_text.line_count
            ),
        );
        let commit_result = self
            .insertion_adapter
            .replace_selection(session_id, &insertion_text.text);
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "selection replace result = {:?} via {:?}; commit_transport={:?}",
                commit_result.status, commit_result.transport, commit_result.transport
            ),
        );

        let final_state = match commit_result.status {
            CommitStatus::Success | CommitStatus::TemporaryStub => SessionState::Committed,
            CommitStatus::NotAttempted | CommitStatus::Failed(_) => SessionState::Failed,
        };
        self.publish(
            session_id,
            SessionKind::SelectedTextEdit,
            final_state,
            "Selected-text replacement probe finished",
        );

        Ok(SelectionReplaceProbeResult {
            session_id,
            selected_text,
            replacement_text: replacement_text.to_string(),
            commit_status: commit_result.status,
            commit_transport: commit_result.transport,
        })
    }

    pub fn run_text_seeded_selected_text_edit_probe(
        &mut self,
        instruction_text: &str,
    ) -> Result<SessionSummary, FailedSessionSummary> {
        let session_started_at = Instant::now();
        let session_id = self.allocate_session_id();
        let requested_kind = SessionKind::SelectedTextEdit;
        let normalized_instruction = instruction_text.trim();

        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::Arming,
            "Session armed",
        );
        let start_feedback_latency_ms =
            self.emit_session_armed_feedback(session_id, &requested_kind, session_started_at);
        self.record(
            DiagnosticCategory::SessionLifecycle,
            Some(session_id),
            "session created via text-seeded selected-text probe".to_string(),
        );
        if let Some(latency_ms) = start_feedback_latency_ms {
            self.record(
                DiagnosticCategory::SessionLifecycle,
                Some(session_id),
                format!("start feedback latency={}ms", latency_ms),
            );
        }
        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::Arming,
            "Capturing selected-text context",
        );

        let selected_text = match self
            .insertion_adapter
            .capture_selected_text(session_id)
            .map_err(|error| {
                self.build_and_publish_failure(
                    session_id,
                    &requested_kind,
                    SessionFailurePhase::Arming,
                    format!("selected-text context capture failed: {error}"),
                    None,
                )
            })? {
            Some(text) if !text.trim().is_empty() => {
                self.record(
                    DiagnosticCategory::Commit,
                    Some(session_id),
                    format!(
                        "captured selected-text context with {} characters for the text-seeded probe",
                        text.chars().count()
                    ),
                );
                text
            }
            _ => {
                self.record(
                    DiagnosticCategory::Commit,
                    Some(session_id),
                    "text-seeded selected-text probe did not capture usable text".to_string(),
                );
                self.publish(
                    session_id,
                    requested_kind.clone(),
                    SessionState::Failed,
                    "No selected-text context was captured",
                );
                self.emit_failure_feedback(session_id, &requested_kind);
                return Err(self.build_and_publish_failure(
                    session_id,
                    &requested_kind,
                    SessionFailurePhase::Arming,
                    format_selected_text_failure(
                        SelectedTextFailureStage::SelectionRead,
                        SelectedTextFailureReason::EmptySelection,
                        "text-seeded selected-text probe could not capture non-empty text from the focused app",
                    ),
                    None,
                ));
            }
        };

        let mode_outcome = self.resolve_mode(
            &TriggerMode::EditShortcut,
            &requested_kind,
            normalized_instruction,
        );
        self.record(
            DiagnosticCategory::Routing,
            Some(session_id),
            format!(
                "text-seeded selected-text probe bypassed microphone capture and ASR; instruction={normalized_instruction}"
            ),
        );
        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::ModeRouting,
            "Using text-seeded selected-text probe instruction",
        );

        let edit_execution =
            match self
                .selected_text_executor
                .execute(&SelectedTextExecutionRequest {
                    session_id,
                    selected_text: selected_text.clone(),
                    instruction_text: normalized_instruction.to_string(),
                }) {
                Ok(edit_execution) => {
                    self.record(
                    DiagnosticCategory::ModeDecision,
                    Some(session_id),
                    format!(
                        "{}; executor_action={:?}; executor_strategy={}; normalized_instruction={}",
                        mode_outcome.reason,
                        edit_execution.action,
                        edit_execution.strategy,
                        edit_execution.normalized_instruction
                    ),
                );
                    edit_execution
                }
                Err(error) => {
                    self.record(
                        DiagnosticCategory::ModeDecision,
                        Some(session_id),
                        format!("{}; executor failed: {}", mode_outcome.reason, error),
                    );
                    self.publish(
                        session_id,
                        requested_kind.clone(),
                        SessionState::Failed,
                        "Selected-text executor could not apply the instruction",
                    );
                    self.emit_failure_feedback(session_id, &requested_kind);
                    return Err(self.build_and_publish_failure(
                        session_id,
                        &requested_kind,
                        SessionFailurePhase::Executing,
                        error,
                        None,
                    ));
                }
            };

        let provider_diagnostics = edit_execution.provider_diagnostics.clone();
        if let Some(provider_diagnostics) = &provider_diagnostics {
            self.record(
                DiagnosticCategory::ModeDecision,
                Some(session_id),
                format!(
                    "selected-text provider action={:?} profile={} model={} provider_preset={} provider_key_source={} attempted={} succeeded={} deterministic_fallback_used={} selected_text_chars={} output_chars={}",
                    edit_execution.action,
                    provider_diagnostics.profile_label,
                    provider_diagnostics.model_code,
                    provider_diagnostics
                        .provider_preset
                        .as_deref()
                        .unwrap_or("none"),
                    provider_diagnostics
                        .provider_key_source
                        .as_deref()
                        .unwrap_or("none"),
                    provider_diagnostics.provider_attempted,
                    provider_diagnostics.provider_succeeded,
                    provider_diagnostics.deterministic_fallback_used,
                    provider_diagnostics.selected_text_char_count,
                    provider_diagnostics.output_char_count
                ),
            );
            if let Some(reason) = &provider_diagnostics.fallback_reason {
                self.record(
                    DiagnosticCategory::ModeDecision,
                    Some(session_id),
                    format!("selected-text provider fallback: {reason}"),
                );
            }
        }

        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::Executing,
            "Prepared transformed text for replacement",
        );
        self.publish(
            session_id,
            requested_kind.clone(),
            SessionState::Committing,
            "Replacing selected text in the insertion layer",
        );

        let commit_payload = edit_execution.output_text;
        let insertion_text = prepare_plain_text_for_insertion(&commit_payload);
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "insertion_text_multiline={} insertion_format_normalized={} insertion_line_count={}",
                insertion_text.multiline,
                insertion_text.format_normalized,
                insertion_text.line_count
            ),
        );
        let commit_result = self
            .insertion_adapter
            .replace_selection(session_id, &insertion_text.text);
        self.record(
            DiagnosticCategory::Commit,
            Some(session_id),
            format!(
                "text-seeded selected-text probe commit result = {:?} via {:?}; commit_transport={:?}",
                commit_result.status, commit_result.transport, commit_result.transport
            ),
        );

        let commit_failure_reason = match &commit_result.status {
            CommitStatus::Failed(error) => Some(error.clone()),
            CommitStatus::NotAttempted => Some("commit was not attempted".to_string()),
            CommitStatus::Success | CommitStatus::TemporaryStub => None,
        };
        let final_state = match commit_result.status {
            CommitStatus::Success | CommitStatus::TemporaryStub => SessionState::Committed,
            CommitStatus::NotAttempted | CommitStatus::Failed(_) => SessionState::Failed,
        };

        self.publish(
            session_id,
            requested_kind.clone(),
            final_state.clone(),
            "Session finished",
        );
        if matches!(final_state, SessionState::Failed) {
            self.emit_failure_feedback(session_id, &requested_kind);
        }

        let total_session_latency_ms = session_started_at
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        self.record(
            DiagnosticCategory::SessionLifecycle,
            Some(session_id),
            format!("session finished in {}ms", total_session_latency_ms),
        );
        let committed_text_for_summary = if provider_diagnostics.is_some() {
            REDACTED_SELECTED_TEXT_PROVIDER_EDIT.to_string()
        } else {
            commit_payload.clone()
        };

        let summary = SessionSummary {
            session_id,
            session_kind: requested_kind,
            final_state,
            completed_at_epoch_ms: current_epoch_ms(),
            start_feedback_latency_ms,
            recording_start_latency_ms: None,
            total_session_latency_ms,
            audio_duration_ms: 0,
            audio_peak_level: 0.0,
            audio_rms_level: 0.0,
            asr_diagnostics: None,
            refine_diagnostics: None,
            recognized_text: normalized_instruction.to_string(),
            committed_text: committed_text_for_summary,
            committed_text_count: Some(count_words_entered(&commit_payload)),
            route_decision: shared_protocol::RouteDecision {
                route_name: shared_protocol::RouteName::LocalAsrOnly,
                asr_provider: shared_protocol::AsrProvider::Local,
                refine_provider: None,
                refinement_applied: false,
                reason: "Text-seeded selected-text probe bypassed microphone capture and ASR."
                    .to_string(),
                refine_fast_path_used: false,
                refine_fast_path_reason: "selected_text_edit".to_string(),
                cloud_refine_skipped: false,
                dictation_routing: None,
            },
            degraded_to_asr: false,
            fallback_reason: None,
            commit_status: commit_result.status,
            commit_transport: commit_result.transport,
            commit_failure_reason,
            mode_reason: mode_outcome.reason,
            selected_text_execution: Some(SelectedTextExecutionSummary {
                action: edit_execution.action,
                strategy: edit_execution.strategy,
                provider_diagnostics,
            }),
            wake_phrase_execution: None,
            instructed_dictation_execution: None,
        };
        self.session_history.push(summary.clone());
        self.overlay_adapter.publish_summary(&summary);

        Ok(summary)
    }

    pub fn diagnostics_len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn diagnostics_snapshot(&self) -> &[DiagnosticEvent] {
        self.diagnostics.snapshot()
    }

    pub fn session_history(&self) -> &[SessionSummary] {
        &self.session_history
    }

    fn allocate_session_id(&mut self) -> u64 {
        let next = self.next_session_id;
        self.next_session_id += 1;
        next
    }

    fn await_recording_stop(
        &mut self,
        session_id: u64,
        session_kind: &SessionKind,
        stop_strategy: &RecordingStopStrategy,
    ) -> Result<(), String> {
        let use_push_to_talk_release =
            matches!(self.settings.shortcut_mode, ShortcutMode::PushToTalk)
                && matches!(session_kind, SessionKind::Dictation);
        match stop_strategy {
            RecordingStopStrategy::Immediate => Ok(()),
            RecordingStopStrategy::Timed(window) => {
                thread::sleep(*window);
                Ok(())
            }
            RecordingStopStrategy::MatchingTrigger(expected_trigger) => {
                if use_push_to_talk_release {
                    self.record(
                        DiagnosticCategory::SessionLifecycle,
                        Some(session_id),
                        format!("waiting for push-to-talk release on {:?}", expected_trigger),
                    );
                    self.publish(
                        session_id,
                        session_kind.clone(),
                        SessionState::Recording,
                        "Recording while the shortcut is held down",
                    );
                    return self.wait_for_trigger_release(session_id, expected_trigger, None);
                }
                self.record(
                    DiagnosticCategory::SessionLifecycle,
                    Some(session_id),
                    format!(
                        "waiting for matching stop trigger via {:?}",
                        expected_trigger
                    ),
                );
                self.publish(
                    session_id,
                    session_kind.clone(),
                    SessionState::Recording,
                    "Recording until the same hotkey is pressed again",
                );
                self.wait_for_matching_trigger(session_id, expected_trigger)
            }
            RecordingStopStrategy::MatchingTriggerOrTimeout {
                expected_trigger,
                timeout,
            } => {
                if use_push_to_talk_release {
                    self.record(
                        DiagnosticCategory::SessionLifecycle,
                        Some(session_id),
                        format!(
                            "waiting for push-to-talk release on {:?} or timeout after {} ms",
                            expected_trigger,
                            timeout.as_millis()
                        ),
                    );
                    self.publish(
                        session_id,
                        session_kind.clone(),
                        SessionState::Recording,
                        "Recording while the shortcut is held down or until the safety timeout expires",
                    );
                    return self.wait_for_trigger_release(
                        session_id,
                        expected_trigger,
                        Some(*timeout),
                    );
                }
                self.record(
                    DiagnosticCategory::SessionLifecycle,
                    Some(session_id),
                    format!(
                        "waiting for matching stop trigger via {:?} or timeout after {} ms",
                        expected_trigger,
                        timeout.as_millis()
                    ),
                );
                self.publish(
                    session_id,
                    session_kind.clone(),
                    SessionState::Recording,
                    "Recording until the same hotkey is pressed again or the safety timeout expires",
                );
                self.wait_for_matching_trigger_with_timeout(session_id, expected_trigger, *timeout)
            }
        }
    }

    fn wait_for_matching_trigger(
        &mut self,
        session_id: u64,
        expected_trigger: &TriggerMode,
    ) -> Result<(), String> {
        loop {
            let observed = self.shortcut_adapter.wait_for_trigger()?;
            if &observed == expected_trigger {
                self.record(
                    DiagnosticCategory::SessionLifecycle,
                    Some(session_id),
                    format!("received matching stop trigger via {:?}", observed),
                );
                return Ok(());
            }

            self.record(
                DiagnosticCategory::SessionLifecycle,
                Some(session_id),
                format!(
                    "ignored non-matching trigger {:?} while waiting for {:?}",
                    observed, expected_trigger
                ),
            );
        }
    }

    fn wait_for_matching_trigger_with_timeout(
        &mut self,
        session_id: u64,
        expected_trigger: &TriggerMode,
        timeout: Duration,
    ) -> Result<(), String> {
        let deadline = std::time::Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                self.record(
                    DiagnosticCategory::SessionLifecycle,
                    Some(session_id),
                    format!(
                        "timed out after {} ms waiting for stop trigger {:?}",
                        timeout.as_millis(),
                        expected_trigger
                    ),
                );
                return Ok(());
            }

            let observed = self.shortcut_adapter.wait_for_trigger_timeout(remaining)?;
            match observed {
                Some(trigger) if &trigger == expected_trigger => {
                    self.record(
                        DiagnosticCategory::SessionLifecycle,
                        Some(session_id),
                        format!("received matching stop trigger via {:?}", trigger),
                    );
                    return Ok(());
                }
                Some(trigger) => {
                    self.record(
                        DiagnosticCategory::SessionLifecycle,
                        Some(session_id),
                        format!(
                            "ignored non-matching trigger {:?} while waiting for {:?}",
                            trigger, expected_trigger
                        ),
                    );
                }
                None => {
                    self.record(
                        DiagnosticCategory::SessionLifecycle,
                        Some(session_id),
                        format!(
                            "timed out after {} ms waiting for stop trigger {:?}",
                            timeout.as_millis(),
                            expected_trigger
                        ),
                    );
                    return Ok(());
                }
            }
        }
    }

    fn resolve_mode(
        &self,
        trigger_mode: &TriggerMode,
        requested_kind: &SessionKind,
        transcript: &str,
    ) -> ModeOutcome {
        match requested_kind {
            SessionKind::SelectedTextEdit => ModeOutcome {
                session_kind: SessionKind::SelectedTextEdit,
                text_for_commit: transcript.to_string(),
                reason: match trigger_mode {
                    TriggerMode::EditShortcut => {
                        "Explicit edit shortcut selected selected-text edit mode".to_string()
                    }
                    TriggerMode::GlobalShortcut => {
                        "Selected-text context was present, so the primary shortcut routed into selected-text edit mode".to_string()
                    }
                },
            },
            SessionKind::WakePhraseIntent => ModeOutcome {
                session_kind: SessionKind::WakePhraseIntent,
                text_for_commit: transcript.to_string(),
                reason: "Session was explicitly created as wake-phrase intent".to_string(),
            },
            SessionKind::InstructedDictation => ModeOutcome {
                session_kind: SessionKind::InstructedDictation,
                text_for_commit: transcript.to_string(),
                reason: "Session was explicitly created as instructed dictation".to_string(),
            },
            SessionKind::Dictation => ModeOutcome {
                session_kind: SessionKind::Dictation,
                text_for_commit: transcript.to_string(),
                reason: "No instructed dictation trigger detected; kept transcript as literal dictation"
                    .to_string(),
            },
        }
    }

    fn publish(
        &mut self,
        session_id: u64,
        session_kind: SessionKind,
        state: SessionState,
        detail: &str,
    ) -> OverlayPublishSummary {
        let message = detail.to_string();
        let summary = self.overlay_adapter.publish(OverlayStatus {
            session_id,
            session_kind,
            state,
            detail: message.clone(),
        });
        self.record(DiagnosticCategory::Overlay, Some(session_id), message);
        summary
    }

    fn record(&mut self, category: DiagnosticCategory, session_id: Option<u64>, message: String) {
        if !self.should_record_diagnostic(&category) {
            return;
        }
        self.diagnostics.record(DiagnosticEvent {
            category,
            session_id,
            message,
        });
    }

    fn should_record_diagnostic(&self, category: &DiagnosticCategory) -> bool {
        match self.settings.diagnostics_verbosity {
            DiagnosticsVerbosity::Verbose => true,
            DiagnosticsVerbosity::Standard => !matches!(category, DiagnosticCategory::Overlay),
        }
    }

    fn build_failure(
        &self,
        session_id: u64,
        session_kind: &SessionKind,
        failure_phase: SessionFailurePhase,
        message: String,
        captured_audio: Option<&CapturedAudio>,
    ) -> FailedSessionSummary {
        FailedSessionSummary {
            session_id,
            session_kind: session_kind.clone(),
            failure_phase,
            message,
            audio_duration_ms: captured_audio.map(|audio| audio.duration_ms),
            audio_peak_level: captured_audio.map(|audio| audio.peak_level),
            audio_rms_level: captured_audio.map(|audio| audio.rms_level),
        }
    }

    fn build_and_publish_failure(
        &self,
        session_id: u64,
        session_kind: &SessionKind,
        failure_phase: SessionFailurePhase,
        message: String,
        captured_audio: Option<&CapturedAudio>,
    ) -> FailedSessionSummary {
        let failure = self.build_failure(
            session_id,
            session_kind,
            failure_phase,
            message,
            captured_audio,
        );
        self.overlay_adapter.publish_failure(&failure);
        failure
    }

    fn emit_session_armed_feedback(
        &self,
        session_id: u64,
        session_kind: &SessionKind,
        session_started_at: Instant,
    ) -> Option<u64> {
        if self.settings.audio_feedback_enabled {
            self.audio_feedback_adapter
                .session_armed(session_id, session_kind);
            return Some(
                session_started_at
                    .elapsed()
                    .as_millis()
                    .min(u64::MAX as u128) as u64,
            );
        }

        None
    }

    fn emit_recording_stopped_feedback(&self, session_id: u64, session_kind: &SessionKind) {
        if self.settings.audio_feedback_enabled {
            self.audio_feedback_adapter
                .recording_stopped(session_id, session_kind);
        }
    }

    fn emit_failure_feedback(&self, session_id: u64, session_kind: &SessionKind) {
        if self.settings.audio_feedback_enabled {
            self.audio_feedback_adapter
                .session_failed(session_id, session_kind);
        }
    }

    fn wait_for_trigger_release(
        &mut self,
        session_id: u64,
        expected_trigger: &TriggerMode,
        timeout: Option<Duration>,
    ) -> Result<(), String> {
        let released = self
            .shortcut_adapter
            .wait_for_trigger_release(expected_trigger, timeout)?;

        if released {
            self.record(
                DiagnosticCategory::SessionLifecycle,
                Some(session_id),
                format!(
                    "recording stopped after {:?} was released",
                    expected_trigger
                ),
            );
        } else if let Some(timeout) = timeout {
            self.record(
                DiagnosticCategory::SessionLifecycle,
                Some(session_id),
                format!(
                    "timed out after {} ms waiting for {:?} to be released",
                    timeout.as_millis(),
                    expected_trigger
                ),
            );
        } else {
            self.record(
                DiagnosticCategory::SessionLifecycle,
                Some(session_id),
                format!(
                    "recording release wait for {:?} ended without an explicit release signal",
                    expected_trigger
                ),
            );
        }

        Ok(())
    }
}

fn changed_setting_groups(
    current: &RuntimeSettings,
    updated: &RuntimeSettings,
) -> Vec<&'static str> {
    let mut groups = Vec::new();
    if current.dictation_shortcut != updated.dictation_shortcut
        || current.edit_shortcut != updated.edit_shortcut
    {
        groups.push("shortcut");
    }
    if current.shortcut_mode != updated.shortcut_mode {
        groups.push("shortcut_mode");
    }
    if current.system_language != updated.system_language {
        groups.push("ui_language");
    }
    if current.ui_style != updated.ui_style {
        groups.push("ui_style");
    }
    if current.audio_feedback_enabled != updated.audio_feedback_enabled {
        groups.push("audio_feedback");
    }
    if current.silence_gate_level != updated.silence_gate_level {
        groups.push("silence_gate");
    }
    if current.wake_phrase != updated.wake_phrase {
        groups.push("instructed_dictation");
    }
    if current.diagnostics_verbosity != updated.diagnostics_verbosity {
        groups.push("diagnostics");
    }
    if current.history_retention != updated.history_retention {
        groups.push("history_retention");
    }
    if current.provider != updated.provider
        || current.refinement_quality != updated.refinement_quality
    {
        groups.push("provider");
    }
    groups
}

fn wake_phrase_source(wake_phrase: &shared_protocol::WakePhraseConfig) -> &'static str {
    if wake_phrase == &RuntimeSettings::default().wake_phrase {
        "default"
    } else {
        "settings"
    }
}

fn normalized_wake_phrase_char_count(wake_phrase: &shared_protocol::WakePhraseConfig) -> usize {
    wake_phrase
        .phrase
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .count()
}

fn current_epoch_ms() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

enum RecordingStopStrategy {
    Immediate,
    Timed(Duration),
    MatchingTrigger(TriggerMode),
    MatchingTriggerOrTimeout {
        expected_trigger: TriggerMode,
        timeout: Duration,
    },
}

fn elapsed_millis(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn selected_text_probe_diagnostic(
    mode: &str,
    elapsed_ms: u64,
    result: &Result<Option<String>, String>,
    reason: &str,
) -> String {
    let result_label = match result {
        Ok(Some(text)) if !text.trim().is_empty() => "selected",
        Ok(_) => "no_selection",
        Err(error) if error.to_ascii_lowercase().contains("timed out") => "timeout",
        Err(_) => "error",
    };
    format!(
        "selected_text_probe_mode={mode} selected_text_probe_ms={elapsed_ms} selected_text_probe_result={result_label} selected_text_probe_reason={reason}"
    )
}

fn default_transcript_for_trigger(trigger_mode: &TriggerMode) -> String {
    match trigger_mode {
        TriggerMode::GlobalShortcut => {
            "hello voiceflow this is the milestone one scaffold".to_string()
        }
        TriggerMode::EditShortcut => "translate this into English".to_string(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionReplaceProbeResult {
    pub session_id: u64,
    pub selected_text: String,
    pub replacement_text: String,
    pub commit_status: CommitStatus,
    pub commit_transport: CommitTransport,
}

fn looks_like_silence(captured_audio: &CapturedAudio, silence_gate_level: u8) -> bool {
    let profile = silence_gate_profile(silence_gate_level);
    captured_audio.duration_ms >= profile.min_activity_duration_ms
        && captured_audio.peak_level <= profile.max_silence_peak_level
        && captured_audio.rms_level <= profile.max_silence_rms_level
}

fn silence_gate_profile(level: u8) -> SilenceGateProfile {
    match level {
        1 => SilenceGateProfile {
            min_activity_duration_ms: 500,
            max_silence_peak_level: 0.010,
            max_silence_rms_level: 0.0025,
        },
        2 => SilenceGateProfile {
            min_activity_duration_ms: 250,
            max_silence_peak_level: 0.015,
            max_silence_rms_level: 0.004,
        },
        3 => SilenceGateProfile {
            min_activity_duration_ms: 200,
            max_silence_peak_level: 0.020,
            max_silence_rms_level: 0.005,
        },
        4 => SilenceGateProfile {
            min_activity_duration_ms: 150,
            max_silence_peak_level: 0.028,
            max_silence_rms_level: 0.007,
        },
        _ => SilenceGateProfile {
            min_activity_duration_ms: 100,
            max_silence_peak_level: 0.040,
            max_silence_rms_level: 0.010,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_key_store::InMemoryProviderCredentialStore;
    use crate::usage_ledger::append_live_host_usage;
    use intent_edit_executor::TemporarySelectedTextExecutor;
    use shared_protocol::{
        AsrDiagnostics, AsrLatencyMetric, CapturedAudio, EngineRequest,
        InstructedDictationProviderDiagnostics, InstructedDictationTransformResponse,
        ProviderPreset, SelectedTextExecutionAction, SelectedTextExecutionRequest,
        SelectedTextExecutionResponse, SelectedTextProviderDiagnostics,
    };
    use speech_engine::{
        ChatCompletionTransport, DashScopeChatCompletionRequest, DashScopeRefinerConfig,
        DeterministicRefiner, ProviderBackedRefiner, Refiner, RoutingPolicy, Transcriber,
        TranscriberPreparation, TranscriptionOutput,
    };
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::{env, ffi::OsString};

    static PROVIDER_ENV_LOCK: Mutex<()> = Mutex::new(());
    const CLOUD_DICTATION_TEXT: &str = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen";

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

    fn set_env(key: &str, value: &str) {
        unsafe {
            env::set_var(key, value);
        }
    }

    struct FixedTranscriber {
        transcript: String,
        diagnostics: Option<AsrDiagnostics>,
        calls: AtomicUsize,
    }

    struct SequenceTranscriber {
        transcripts: Mutex<VecDeque<String>>,
    }

    struct PreparingTranscriber {
        warmup_succeeded: bool,
    }

    impl Transcriber for PreparingTranscriber {
        fn prepare(&self) -> Result<Option<TranscriberPreparation>, String> {
            Ok(Some(TranscriberPreparation {
                backend: "test-worker".to_string(),
                cold_start: true,
                worker_model_load_ms: Some(40),
                worker_model_warmup_ms: Some(15),
                worker_model_warmup_succeeded: Some(self.warmup_succeeded),
            }))
        }

        fn transcribe(&self, _request: &EngineRequest) -> Result<TranscriptionOutput, String> {
            panic!("host preparation must not transcribe a user session")
        }
    }

    impl Transcriber for FixedTranscriber {
        fn transcribe(&self, _request: &EngineRequest) -> Result<TranscriptionOutput, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(TranscriptionOutput {
                transcript: self.transcript.clone(),
                diagnostics: self.diagnostics.clone(),
            })
        }
    }

    impl Transcriber for SequenceTranscriber {
        fn transcribe(&self, _request: &EngineRequest) -> Result<TranscriptionOutput, String> {
            let transcript = self
                .transcripts
                .lock()
                .map_err(|_| "sequence transcriber lock was poisoned".to_string())?
                .pop_front()
                .ok_or_else(|| "sequence transcriber ran out of transcripts".to_string())?;
            Ok(TranscriptionOutput {
                transcript,
                diagnostics: None,
            })
        }
    }

    struct FailingRefiner;

    impl Refiner for FailingRefiner {
        fn refine(&self, _input: &str, _request: &EngineRequest) -> Result<String, String> {
            Err("test refiner failed".to_string())
        }
    }

    #[derive(Debug)]
    struct SucceedingChatTransport;

    impl ChatCompletionTransport for SucceedingChatTransport {
        fn complete(
            &self,
            _config: &DashScopeRefinerConfig,
            _payload: &DashScopeChatCompletionRequest,
        ) -> Result<String, String> {
            Ok("provider refined text".to_string())
        }
    }

    #[derive(Default)]
    struct TestShortcutAdapter {
        release_calls: Arc<AtomicUsize>,
    }

    impl ShortcutAdapter for TestShortcutAdapter {
        fn register(&self, _settings: &RuntimeSettings) -> Result<(), String> {
            Ok(())
        }

        fn wait_for_trigger_release(
            &self,
            _trigger_mode: &TriggerMode,
            _timeout: Option<Duration>,
        ) -> Result<bool, String> {
            self.release_calls.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        }
    }

    #[derive(Default)]
    struct ReloadTrackingShortcutAdapter {
        register_calls: Arc<AtomicUsize>,
        rebind_calls: Arc<AtomicUsize>,
        fail_rebind: Arc<AtomicBool>,
    }

    impl ShortcutAdapter for ReloadTrackingShortcutAdapter {
        fn register(&self, _settings: &RuntimeSettings) -> Result<(), String> {
            self.register_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn rebind(
            &self,
            _current: &RuntimeSettings,
            _updated: &RuntimeSettings,
        ) -> Result<(), String> {
            self.rebind_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_rebind.load(Ordering::SeqCst) {
                Err("simulated registration conflict".to_string())
            } else {
                Ok(())
            }
        }
    }

    struct FixedAudioAdapter {
        captured_audio: CapturedAudio,
    }

    impl AudioCaptureAdapter for FixedAudioAdapter {
        fn start(&self, _session_id: u64) -> Result<AudioStartSummary, String> {
            Ok(AudioStartSummary::default())
        }

        fn stop(&self, _session_id: u64) -> Result<CapturedAudio, String> {
            Ok(self.captured_audio.clone())
        }
    }

    struct PanicAudioAdapter;

    impl AudioCaptureAdapter for PanicAudioAdapter {
        fn start(&self, _session_id: u64) -> Result<AudioStartSummary, String> {
            panic!("text-seeded probe should not start audio capture");
        }

        fn stop(&self, _session_id: u64) -> Result<CapturedAudio, String> {
            panic!("text-seeded probe should not stop audio capture");
        }
    }

    struct PreparingAudioAdapter {
        prepare_calls: Arc<AtomicUsize>,
        start_calls: Arc<AtomicUsize>,
        stop_calls: Arc<AtomicUsize>,
        prepare_succeeded: bool,
        captured_audio: CapturedAudio,
    }

    impl AudioCaptureAdapter for PreparingAudioAdapter {
        fn prepare(&self) -> AudioPreparationSummary {
            self.prepare_calls.fetch_add(1, Ordering::SeqCst);
            AudioPreparationSummary {
                prepare_ms: 4,
                prepare_succeeded: self.prepare_succeeded,
                backend_create_ms: 1,
                device_discovery_ms: 3,
            }
        }

        fn start(&self, _session_id: u64) -> Result<AudioStartSummary, String> {
            self.start_calls.fetch_add(1, Ordering::SeqCst);
            Ok(AudioStartSummary {
                total_ms: 6,
                backend_create_ms: 0,
                device_discovery_ms: 2,
                capture_start_ms: 4,
            })
        }

        fn stop(&self, _session_id: u64) -> Result<CapturedAudio, String> {
            self.stop_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.captured_audio.clone())
        }
    }

    struct RecordingInsertionAdapter {
        committed: Arc<Mutex<Vec<String>>>,
        selected_text: Option<String>,
        replaced: Mutex<Vec<String>>,
        commit_failure: Option<String>,
        commit_transport: CommitTransport,
        replace_transport: CommitTransport,
        strict_capture_calls: Arc<AtomicUsize>,
        fast_capture_calls: Arc<AtomicUsize>,
    }

    impl Default for RecordingInsertionAdapter {
        fn default() -> Self {
            Self {
                committed: Arc::new(Mutex::new(Vec::new())),
                selected_text: None,
                replaced: Mutex::new(Vec::new()),
                commit_failure: None,
                commit_transport: CommitTransport::Unknown,
                replace_transport: CommitTransport::Unknown,
                strict_capture_calls: Arc::new(AtomicUsize::new(0)),
                fast_capture_calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl TextInsertionAdapter for RecordingInsertionAdapter {
        fn commit_text(&self, _session_id: u64, text: &str) -> CommitResult {
            if let Some(error) = &self.commit_failure {
                return CommitResult {
                    status: CommitStatus::Failed(error.clone()),
                    transport: self.commit_transport.clone(),
                };
            }
            self.committed
                .lock()
                .expect("commit tracking mutex should not be poisoned")
                .push(text.to_string());
            CommitResult {
                status: CommitStatus::Success,
                transport: self.commit_transport.clone(),
            }
        }

        fn capture_selected_text_fast(&self, _session_id: u64) -> Result<Option<String>, String> {
            self.fast_capture_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.selected_text.clone())
        }

        fn capture_selected_text(&self, _session_id: u64) -> Result<Option<String>, String> {
            self.strict_capture_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.selected_text.clone())
        }

        fn replace_selection(&self, _session_id: u64, text: &str) -> CommitResult {
            self.replaced
                .lock()
                .expect("replace tracking mutex should not be poisoned")
                .push(text.to_string());
            CommitResult {
                status: CommitStatus::Success,
                transport: self.replace_transport.clone(),
            }
        }
    }

    #[derive(Default)]
    struct RecordingOverlayAdapter {
        published: Arc<Mutex<Vec<OverlayStatus>>>,
        summaries: Arc<Mutex<Vec<SessionSummary>>>,
    }

    impl OverlayAdapter for RecordingOverlayAdapter {
        fn publish(&self, status: OverlayStatus) -> OverlayPublishSummary {
            self.published
                .lock()
                .expect("overlay tracking mutex should not be poisoned")
                .push(status);
            OverlayPublishSummary::default()
        }

        fn publish_summary(&self, summary: &SessionSummary) {
            self.summaries
                .lock()
                .expect("overlay summary tracking mutex should not be poisoned")
                .push(summary.clone());
        }
    }

    struct FixedSelectedTextExecutor {
        output_text: String,
        action: SelectedTextExecutionAction,
        provider_diagnostics: Option<SelectedTextProviderDiagnostics>,
        calls: Arc<AtomicUsize>,
    }

    impl SelectedTextExecutor for FixedSelectedTextExecutor {
        fn execute(
            &self,
            request: &SelectedTextExecutionRequest,
        ) -> Result<SelectedTextExecutionResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(SelectedTextExecutionResponse {
                output_text: self.output_text.clone(),
                action: self.action.clone(),
                strategy: "mock provider selected-text edit".to_string(),
                normalized_instruction: request.instruction_text.trim().to_string(),
                provider_diagnostics: self.provider_diagnostics.clone(),
            })
        }
    }

    struct FixedInstructedDictationExecutor {
        output_text: String,
        calls: Arc<AtomicUsize>,
    }

    impl InstructedDictationExecutor for FixedInstructedDictationExecutor {
        fn transform(
            &self,
            request: &InstructedDictationTransformRequest,
        ) -> Result<InstructedDictationTransformResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(InstructedDictationTransformResponse {
                output_text: self.output_text.clone(),
                strategy: "mock provider instructed dictation transform".to_string(),
                provider_diagnostics: InstructedDictationProviderDiagnostics {
                    profile_label: "Best Quality".to_string(),
                    model_code: "deepseek-v4-flash".to_string(),
                    provider_preset: Some("bailian".to_string()),
                    provider_key_source: Some("credential_store".to_string()),
                    provider_attempted: true,
                    provider_succeeded: true,
                    deterministic_fallback_used: false,
                    fallback_reason: None,
                    instruction_char_count: request.instruction_text.chars().count(),
                    content_char_count: request.content_text.chars().count(),
                    output_char_count: self.output_text.chars().count(),
                },
            })
        }
    }

    struct RecordingFeedbackAdapter {
        failures: Arc<AtomicUsize>,
        stops: Arc<AtomicUsize>,
    }

    impl Default for RecordingFeedbackAdapter {
        fn default() -> Self {
            Self {
                failures: Arc::new(AtomicUsize::new(0)),
                stops: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl AudioFeedbackAdapter for RecordingFeedbackAdapter {
        fn recording_stopped(&self, _session_id: u64, _session_kind: &SessionKind) {
            self.stops.fetch_add(1, Ordering::SeqCst);
        }

        fn session_failed(&self, _session_id: u64, _session_kind: &SessionKind) {
            self.failures.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn build_test_engine(transcript: &str) -> SpeechEngine {
        build_test_engine_with_diagnostics(transcript, None)
    }

    fn build_test_engine_with_diagnostics(
        transcript: &str,
        diagnostics: Option<AsrDiagnostics>,
    ) -> SpeechEngine {
        SpeechEngine::new(
            RoutingPolicy::default(),
            FixedTranscriber {
                transcript: transcript.to_string(),
                diagnostics,
                calls: AtomicUsize::new(0),
            },
            DeterministicRefiner,
        )
    }

    fn build_test_engine_with_failing_refiner(transcript: &str) -> SpeechEngine {
        SpeechEngine::new(
            RoutingPolicy::default(),
            FixedTranscriber {
                transcript: transcript.to_string(),
                diagnostics: None,
                calls: AtomicUsize::new(0),
            },
            FailingRefiner,
        )
    }

    fn build_test_engine_with_provider_refiner(transcript: &str) -> SpeechEngine {
        SpeechEngine::new(
            RoutingPolicy::default(),
            FixedTranscriber {
                transcript: transcript.to_string(),
                diagnostics: None,
                calls: AtomicUsize::new(0),
            },
            ProviderBackedRefiner::with_request_provider_settings(SucceedingChatTransport),
        )
    }

    fn build_test_audio() -> CapturedAudio {
        CapturedAudio::from_samples(16_000, 1, vec![0.25, -0.25, 0.1, -0.1])
    }

    #[test]
    fn failed_model_warmup_is_non_fatal_and_has_no_session_side_effects() {
        let overlay = RecordingOverlayAdapter::default();
        let published = Arc::clone(&overlay.published);
        let summaries = Arc::clone(&overlay.summaries);
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            SpeechEngine::new(
                RoutingPolicy::default(),
                PreparingTranscriber {
                    warmup_succeeded: false,
                },
                DeterministicRefiner,
            ),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            overlay,
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        let preparation = runtime.prepare_speech_engine();

        assert!(preparation.prepare_succeeded);
        assert_eq!(preparation.model_warmup_succeeded, Some(false));
        assert!(runtime.session_history().is_empty());
        assert!(
            published
                .lock()
                .expect("overlay tracking mutex should not be poisoned")
                .is_empty()
        );
        assert!(
            summaries
                .lock()
                .expect("overlay summary tracking mutex should not be poisoned")
                .is_empty()
        );
    }

    #[test]
    fn audio_preparation_is_non_capturing_and_failure_is_non_fatal() {
        let prepare_calls = Arc::new(AtomicUsize::new(0));
        let start_calls = Arc::new(AtomicUsize::new(0));
        let stop_calls = Arc::new(AtomicUsize::new(0));
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("hello"),
            TestShortcutAdapter::default(),
            PreparingAudioAdapter {
                prepare_calls: Arc::clone(&prepare_calls),
                start_calls: Arc::clone(&start_calls),
                stop_calls: Arc::clone(&stop_calls),
                prepare_succeeded: false,
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        assert!(!runtime.prepare_audio_capture().prepare_succeeded);
        assert_eq!(prepare_calls.load(Ordering::SeqCst), 1);
        assert_eq!(start_calls.load(Ordering::SeqCst), 0);
        assert_eq!(stop_calls.load(Ordering::SeqCst), 0);
        assert!(runtime.session_history().is_empty());

        runtime.bootstrap().expect("bootstrap should succeed");
        runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("audio preparation failure must fall back to normal start");
        assert_eq!(start_calls.load(Ordering::SeqCst), 1);
        assert_eq!(stop_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn recording_wait_failure_stops_capture_and_allows_the_next_session() {
        struct FailingOnceShortcut(AtomicBool);
        impl ShortcutAdapter for FailingOnceShortcut {
            fn register(&self, _: &RuntimeSettings) -> Result<(), String> { Ok(()) }
            fn wait_for_trigger_release(&self, _: &TriggerMode, _: Option<Duration>) -> Result<bool, String> {
                if self.0.swap(false, Ordering::SeqCst) { Err("stop wait failed".into()) } else { Ok(true) }
            }
        }
        struct ActiveAudio(Arc<AtomicBool>);
        impl AudioCaptureAdapter for ActiveAudio {
            fn start(&self, _: u64) -> Result<AudioStartSummary, String> {
                assert!(!self.0.swap(true, Ordering::SeqCst), "previous capture was not stopped");
                Ok(AudioStartSummary::default())
            }
            fn stop(&self, _: u64) -> Result<CapturedAudio, String> {
                assert!(self.0.swap(false, Ordering::SeqCst));
                Ok(build_test_audio())
            }
        }
        let active = Arc::new(AtomicBool::new(false));
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings { shortcut_mode: ShortcutMode::PushToTalk, ..RuntimeSettings::default() }, build_test_engine("hello"),
            FailingOnceShortcut(AtomicBool::new(true)), ActiveAudio(Arc::clone(&active)),
            RecordingInsertionAdapter::default(), RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(), TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor, TemporarySelectedTextExecutor,
        );
        runtime.bootstrap().unwrap();
        let failure = runtime.run_live_session_for_trigger(TriggerMode::GlobalShortcut, None).unwrap_err();
        assert!(failure.message.contains("stop wait failed"));
        assert!(!active.load(Ordering::SeqCst));
        runtime.run_live_session_for_trigger(TriggerMode::GlobalShortcut, None).unwrap();
        assert!(!active.load(Ordering::SeqCst));
    }

    fn build_silent_test_audio(duration_ms: u64) -> CapturedAudio {
        let sample_count = (16 * duration_ms) as usize;
        CapturedAudio::from_samples(16_000, 1, vec![0.0; sample_count])
    }

    fn loaded_settings(settings: RuntimeSettings) -> crate::settings_store::LoadedRuntimeSettings {
        crate::settings_store::LoadedRuntimeSettings {
            settings,
            path: std::path::PathBuf::from("settings.json"),
            source: crate::settings_store::RuntimeSettingsSource::File,
            warnings: vec![],
        }
    }

    #[test]
    fn runtime_settings_reload_rebinds_shortcut_once_and_updates_snapshot() {
        let register_calls = Arc::new(AtomicUsize::new(0));
        let rebind_calls = Arc::new(AtomicUsize::new(0));
        let shortcut_adapter = ReloadTrackingShortcutAdapter {
            register_calls: Arc::clone(&register_calls),
            rebind_calls: Arc::clone(&rebind_calls),
            fail_rebind: Arc::new(AtomicBool::new(false)),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("settings reload"),
            shortcut_adapter,
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );
        runtime.bootstrap().expect("bootstrap should register once");

        let mut updated = RuntimeSettings::default();
        updated.dictation_shortcut.key = "R".to_string();
        updated.edit_shortcut.key = "R".to_string();
        updated.shortcut_mode = ShortcutMode::PushToTalk;
        updated.silence_gate_level = 4;
        updated.wake_phrase.enabled = false;
        let changed = runtime
            .apply_runtime_settings(updated.clone())
            .expect("valid reload should apply");

        assert_eq!(register_calls.load(Ordering::SeqCst), 1);
        assert_eq!(rebind_calls.load(Ordering::SeqCst), 1);
        assert!(changed.contains(&"shortcut"));
        assert!(changed.contains(&"shortcut_mode"));
        assert!(changed.contains(&"silence_gate"));
        assert!(changed.contains(&"instructed_dictation"));
        assert_eq!(runtime.settings(), &updated);

        assert!(
            runtime
                .apply_runtime_settings(updated)
                .expect("duplicate reload should be a no-op")
                .is_empty()
        );
        assert_eq!(rebind_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn shortcut_rebind_failure_keeps_previous_runtime_snapshot() {
        let fail_rebind = Arc::new(AtomicBool::new(true));
        let rebind_calls = Arc::new(AtomicUsize::new(0));
        let shortcut_adapter = ReloadTrackingShortcutAdapter {
            register_calls: Arc::new(AtomicUsize::new(0)),
            rebind_calls: Arc::clone(&rebind_calls),
            fail_rebind,
        };
        let original = RuntimeSettings::default();
        let mut runtime = HostRuntime::with_speech_engine(
            original.clone(),
            build_test_engine("settings rollback"),
            shortcut_adapter,
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );
        runtime.bootstrap().expect("bootstrap should succeed");

        let mut rejected = original.clone();
        rejected.dictation_shortcut.key = "R".to_string();
        rejected.edit_shortcut.key = "R".to_string();
        rejected.system_language = shared_protocol::SystemLanguage::Chinese;
        let error = runtime
            .apply_runtime_settings(rejected)
            .expect_err("failed shortcut registration should reject the full snapshot");

        assert!(error.contains("previous shortcut remains active"));
        assert_eq!(rebind_calls.load(Ordering::SeqCst), 1);
        assert_eq!(runtime.settings(), &original);
    }

    #[test]
    fn mode_reload_changes_the_next_session_stop_behavior_without_stale_release_state() {
        let release_calls = Arc::new(AtomicUsize::new(0));
        let mut initial = RuntimeSettings::default();
        initial.shortcut_mode = ShortcutMode::PushToTalk;
        let mut runtime = HostRuntime::with_speech_engine(
            initial,
            build_test_engine("mode reload test"),
            TestShortcutAdapter {
                release_calls: Arc::clone(&release_calls),
            },
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        let mut toggle = runtime.settings().clone();
        toggle.shortcut_mode = ShortcutMode::Toggle;
        runtime
            .apply_runtime_settings(toggle)
            .expect("toggle settings should apply");
        runtime
            .run_live_session_for_trigger(
                TriggerMode::GlobalShortcut,
                Some(Duration::from_millis(10)),
            )
            .expect("toggle session should complete");
        assert_eq!(release_calls.load(Ordering::SeqCst), 0);

        let mut push_to_talk = runtime.settings().clone();
        push_to_talk.shortcut_mode = ShortcutMode::PushToTalk;
        runtime
            .apply_runtime_settings(push_to_talk)
            .expect("push-to-talk settings should apply");
        runtime
            .run_live_session_for_trigger(
                TriggerMode::GlobalShortcut,
                Some(Duration::from_millis(10)),
            )
            .expect("push-to-talk session should complete");
        assert_eq!(release_calls.load(Ordering::SeqCst), 1);
    }

    fn assert_provider(
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
    }

    #[test]
    fn normal_dictation_provider_cache_keeps_effective_config_after_reload_failure() {
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
        let mut runtime = HostRuntime::with_speech_engine(
            startup,
            build_test_engine("provider cache test"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        set_env("VOICEFLOW_PROVIDER_API_KEY", "key-b");
        assert_provider(
            &runtime.current_provider_for_request(1, || Ok(loaded_settings(updated.clone()))),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );

        set_env("VOICEFLOW_PROVIDER_API_KEY", "key-c");
        set_env("VOICEFLOW_PROVIDER_TYPE", "custom_openai_compatible");
        set_env("VOICEFLOW_MODEL_BEST", "env-drift-model");
        set_env("VOICEFLOW_LLM_REFINE_TIMEOUT_MS", "4444");
        assert_provider(
            &runtime
                .current_provider_for_request(2, || Err("simulated reload failure".to_string())),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
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
        assert_provider(
            &runtime.current_provider_for_request(3, || Ok(loaded_settings(invalid_settings))),
            ProviderPreset::VolcengineArk,
            "https://updated.example/v1",
            "updated-model",
            3_000,
            "key-b",
            false,
        );
    }

    #[test]
    fn lighter_silence_gate_allows_quiet_short_audio_to_reach_asr() {
        let mut settings = RuntimeSettings::default();
        settings.silence_gate_level = 5;
        let quiet_short_audio = build_silent_test_audio(300);
        let mut runtime = HostRuntime::with_speech_engine(
            settings,
            build_test_engine("hello from the lighter gate"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: quiet_short_audio,
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let mut updated = runtime.settings().clone();
        updated.silence_gate_level = 1;
        runtime
            .apply_runtime_settings(updated)
            .expect("updated silence gate should apply before the next session");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("lighter silence gate should let the session reach ASR");

        assert_eq!(summary.final_state, SessionState::Committed);
        assert_eq!(summary.recognized_text, "hello from the lighter gate");
    }

    #[test]
    fn stricter_silence_gate_rejects_quiet_short_audio() {
        let mut settings = RuntimeSettings::default();
        settings.silence_gate_level = 5;
        let quiet_short_audio = build_silent_test_audio(180);
        let mut runtime = HostRuntime::with_speech_engine(
            settings,
            build_test_engine("hallucinated text from strict gate"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: quiet_short_audio,
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let error = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect_err("strict silence gate should reject the quiet short capture");

        assert!(error.message.contains("gate level 5"));
        assert_eq!(error.failure_phase, SessionFailurePhase::Recording);
    }

    #[test]
    fn fails_session_when_recognition_returns_no_speech() {
        let overlay = RecordingOverlayAdapter::default();
        let feedback = RecordingFeedbackAdapter::default();
        let failure_count = Arc::clone(&feedback.failures);
        let stop_count = Arc::clone(&feedback.stops);
        let insertion = RecordingInsertionAdapter::default();
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("   "),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            overlay,
            feedback,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let error = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect_err("empty transcript should fail the session");

        assert!(error.message.contains("no speech was recognized"));
        assert_eq!(error.failure_phase, SessionFailurePhase::Recognizing);
        assert_eq!(runtime.session_history().len(), 0);
        assert_eq!(failure_count.load(Ordering::SeqCst), 0);
        assert_eq!(stop_count.load(Ordering::SeqCst), 1);
        assert!(
            runtime
                .diagnostics_snapshot()
                .iter()
                .any(|event| event.message.contains("empty transcript")),
            "diagnostics should record the empty transcript failure"
        );
    }

    #[test]
    fn primary_shortcut_routes_into_selected_text_edit_when_selection_exists() {
        let insertion = RecordingInsertionAdapter {
            selected_text: Some("please rewrite me".to_string()),
            ..Default::default()
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("uppercase"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("selected-text edit session should succeed");

        assert_eq!(summary.session_kind, SessionKind::SelectedTextEdit);
        assert_eq!(summary.committed_text, "PLEASE REWRITE ME");
        assert!(!summary.route_decision.refine_fast_path_used);
        assert_eq!(
            summary.route_decision.refine_fast_path_reason,
            "selected_text_edit"
        );
        assert!(!summary.route_decision.cloud_refine_skipped);
        assert_eq!(
            summary.selected_text_execution,
            Some(SelectedTextExecutionSummary {
                action: shared_protocol::SelectedTextExecutionAction::Uppercase,
                strategy: "temporary executor applied uppercase formatting".to_string(),
                provider_diagnostics: None,
            })
        );
    }

    #[test]
    fn normal_dictation_does_not_call_selected_text_executor() {
        let selected_text_calls = Arc::new(AtomicUsize::new(0));
        let selected_text_executor = FixedSelectedTextExecutor {
            output_text: "provider output should not be used".to_string(),
            action: SelectedTextExecutionAction::ConciseRewrite,
            provider_diagnostics: Some(SelectedTextProviderDiagnostics {
                profile_label: "BestQuality".to_string(),
                model_code: "deepseek-v4-flash".to_string(),
                provider_preset: Some("bailian".to_string()),
                provider_key_source: Some("credential_store".to_string()),
                provider_attempted: true,
                provider_succeeded: true,
                deterministic_fallback_used: false,
                fallback_reason: None,
                selected_text_char_count: 10,
                output_char_count: 34,
            }),
            calls: Arc::clone(&selected_text_calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("hello world from dictation"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            selected_text_executor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("dictation session should succeed");

        assert_eq!(summary.session_kind, SessionKind::Dictation);
        assert_eq!(selected_text_calls.load(Ordering::SeqCst), 0);
        assert_eq!(summary.selected_text_execution, None);
    }

    #[test]
    fn normal_dictation_fast_path_is_reported_in_host_diagnostics() {
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("hello"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("short normal dictation should succeed");

        assert_eq!(summary.committed_text, "Hello.");
        assert!(summary.route_decision.refine_fast_path_used);
        assert_eq!(
            summary.route_decision.refine_fast_path_reason,
            "short_local"
        );
        assert!(summary.route_decision.cloud_refine_skipped);
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(diagnostics_text.contains("refine_fast_path_used=true"));
        assert!(diagnostics_text.contains("refine_fast_path_reason=short_local"));
        assert!(diagnostics_text.contains("dictation_text_count=1"));
        assert!(diagnostics_text.contains("dictation_refinement_mode=local_only"));
        assert!(diagnostics_text.contains("dictation_routing_reason=short_local"));
        assert!(diagnostics_text.contains("self_correction_detected=false"));
        assert!(diagnostics_text.contains("cloud_refine_skipped=true"));
        assert!(diagnostics_text.contains("provider_request_ms=0"));
        assert!(diagnostics_text.contains("refine_model=none"));
        assert!(!diagnostics_text.contains("refine_model=deepseek-v4-flash"));
        assert!(!diagnostics_text.contains("provider_key_source="));
    }

    #[test]
    fn host_runtime_construction_does_not_query_credential_store() {
        let store = InMemoryProviderCredentialStore::new();
        let _runtime = HostRuntime::with_speech_engine_and_credentials(
            RuntimeSettings::default(),
            build_test_engine("hello"),
            Arc::new(store.clone()),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        assert_eq!(store.lookup_count(), 0);
    }

    #[test]
    fn short_local_fast_path_dictation_does_not_query_credential_store() {
        let store = InMemoryProviderCredentialStore::new();
        let mut runtime = HostRuntime::with_speech_engine_and_credentials(
            RuntimeSettings::default(),
            build_test_engine("hello"),
            Arc::new(store.clone()),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("short normal dictation should succeed");

        assert!(summary.route_decision.refine_fast_path_used);
        assert_eq!(summary.route_decision.route_name, RouteName::LocalAsrOnly);
        assert_eq!(store.lookup_count(), 0);
    }

    #[test]
    fn provider_backed_normal_dictation_queries_credential_store_after_asr_routing() {
        let _env_lock = PROVIDER_ENV_LOCK
            .lock()
            .expect("provider env lock should not be poisoned");
        let _key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::Bailian, "provider-backed-key")
            .expect("test store should accept key");
        let mut runtime = HostRuntime::with_speech_engine_and_credentials(
            RuntimeSettings::default(),
            build_test_engine(CLOUD_DICTATION_TEXT),
            Arc::new(store.clone()),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );
        let recognition_request = EngineRequest {
            session_id: 42,
            requested_kind: SessionKind::Dictation,
            captured_audio: build_test_audio(),
            transcript_hint: None,
            refinement_profile: refinement_model_profile_for_quality(
                &RuntimeSettings::default().refinement_quality,
            ),
            provider_settings: ProviderSettings::default(),
            provider_runtime_config: None,
            dictation_routing: None,
        };
        let recognition = runtime
            .speech_engine
            .recognize(&recognition_request)
            .expect("test ASR should succeed");

        assert_eq!(store.lookup_count(), 0);
        let provider_request = runtime.engine_request_for_recognition_result(
            &recognition_request,
            &recognition,
            || Ok(loaded_settings(RuntimeSettings::default())),
        );

        assert!(provider_request.provider_runtime_config.is_some());
        assert_eq!(
            provider_request.dictation_routing,
            Some(shared_protocol::DictationRoutingDecision {
                text_count: 16,
                refinement_mode: DictationRefinementMode::LightCleanup,
                routing_reason: shared_protocol::DictationRoutingReason::LengthLight,
                self_correction_detected: false,
            })
        );
        assert_eq!(store.lookup_count(), 1);
    }

    #[test]
    fn live_normal_dictation_diagnostics_report_credential_store_key_source() {
        let _env_lock = PROVIDER_ENV_LOCK
            .lock()
            .expect("provider env lock should not be poisoned");
        let _provider_key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::Bailian, "stored-secret-sentinel")
            .expect("test store should accept key");
        let mut runtime = HostRuntime::with_speech_engine_and_credentials(
            RuntimeSettings::default(),
            build_test_engine_with_provider_refiner(CLOUD_DICTATION_TEXT),
            Arc::new(store),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("provider-backed dictation should succeed");
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(summary.committed_text, "provider refined text");
        let refine_diagnostics = summary
            .refine_diagnostics
            .as_ref()
            .expect("cloud dictation should retain refine diagnostics");
        assert_eq!(refine_diagnostics.model_code, "deepseek-v4-flash");
        assert_eq!(
            refine_diagnostics.prompt_profile,
            Some(shared_protocol::DictationPromptProfile::DictationLight)
        );
        assert_eq!(
            refine_diagnostics.prompt_source,
            Some(shared_protocol::PromptSource::Builtin)
        );
        assert!(diagnostics_text.contains("provider_preset=bailian"));
        assert!(diagnostics_text.contains("provider_key_source=credential_store"));
        assert!(diagnostics_text.contains("dictation_text_count=16"));
        assert!(diagnostics_text.contains("dictation_refinement_mode=light_cleanup"));
        assert!(diagnostics_text.contains("dictation_routing_reason=length_light"));
        assert!(diagnostics_text.contains("self_correction_detected=false"));
        assert!(diagnostics_text.contains("prompt_profile=dictation_light"));
        assert!(diagnostics_text.contains("prompt_source=builtin"));
        assert!(!diagnostics_text.contains("stored-secret-sentinel"));
    }

    #[test]
    fn short_self_correction_reports_light_exception_without_sensitive_text() {
        let _env_lock = PROVIDER_ENV_LOCK
            .lock()
            .expect("provider env lock should not be poisoned");
        let _provider_key_guard = EnvVarGuard::remove("VOICEFLOW_PROVIDER_API_KEY");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let store = InMemoryProviderCredentialStore::new();
        store
            .set(&ProviderPreset::Bailian, "stored-secret-sentinel")
            .expect("test store should accept key");
        let transcript = "明天，不对，后天要开会";
        let mut runtime = HostRuntime::with_speech_engine_and_credentials(
            RuntimeSettings::default(),
            build_test_engine_with_provider_refiner(transcript),
            Arc::new(store),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("short self-correction should use provider cleanup");
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(
            summary
                .route_decision
                .dictation_routing
                .as_ref()
                .map(|routing| routing.routing_reason),
            Some(shared_protocol::DictationRoutingReason::SelfCorrectionException)
        );
        assert!(diagnostics_text.contains("dictation_refinement_mode=light_cleanup"));
        assert!(diagnostics_text.contains("dictation_routing_reason=self_correction_exception"));
        assert!(diagnostics_text.contains("self_correction_detected=true"));
        assert!(diagnostics_text.contains("prompt_profile=dictation_light"));
        assert!(!diagnostics_text.contains(transcript));
        assert!(!diagnostics_text.contains("stored-secret-sentinel"));
    }

    #[test]
    fn live_normal_dictation_diagnostics_report_env_key_source() {
        let _env_lock = PROVIDER_ENV_LOCK
            .lock()
            .expect("provider env lock should not be poisoned");
        let _provider_key_guard =
            EnvVarGuard::set("VOICEFLOW_PROVIDER_API_KEY", "env-secret-sentinel");
        let _legacy_key_guard = EnvVarGuard::remove("DASHSCOPE_API_KEY");
        let store = InMemoryProviderCredentialStore::new();
        let mut runtime = HostRuntime::with_speech_engine_and_credentials(
            RuntimeSettings::default(),
            build_test_engine_with_provider_refiner(CLOUD_DICTATION_TEXT),
            Arc::new(store),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("provider-backed dictation should succeed");
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(diagnostics_text.contains("provider_preset=bailian"));
        assert!(diagnostics_text.contains("provider_key_source=provider_env"));
        assert!(!diagnostics_text.contains("env-secret-sentinel"));
    }

    #[test]
    fn instructed_dictation_parses_the_full_recognized_transcript() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "Tomorrow we need to confirm the invoice recognition timeline."
                .to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine(
                "Voice Flow, translate the following into English: 明天我们要确认发票识别项目的时间线。",
            ),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("instructed dictation session should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(summary.session_kind, SessionKind::InstructedDictation);
        assert_eq!(
            summary.committed_text,
            "Tomorrow we need to confirm the invoice recognition timeline."
        );
        assert_eq!(
            summary.recognized_text,
            REDACTED_INSTRUCTED_DICTATION_SOURCE
        );
        assert_eq!(
            summary.route_decision.route_name,
            RouteName::InstructedDictation
        );
        assert!(!summary.route_decision.refine_fast_path_used);
        assert_eq!(
            summary.route_decision.refine_fast_path_reason,
            "instructed_dictation"
        );
        assert!(!summary.route_decision.cloud_refine_skipped);
        assert!(summary.selected_text_execution.is_none());
        assert!(summary.wake_phrase_execution.is_none());
        let execution = summary
            .instructed_dictation_execution
            .expect("instructed execution summary should be present");
        assert!(execution.trigger_phrase_matched);
        assert!(execution.provider_diagnostics.provider_attempted);
        assert!(execution.provider_diagnostics.provider_succeeded);
        assert!(execution.provider_diagnostics.instruction_char_count > 0);
        assert_eq!(execution.provider_diagnostics.content_char_count, 0);
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(diagnostics_text.contains("Instructed dictation trigger matched"));
        assert!(diagnostics_text.contains("instruction_chars="));
        assert!(!diagnostics_text.contains("明天我们要确认发票识别项目的时间线"));
    }

    #[test]
    fn filler_prefixed_generic_instructed_task_executes_and_commits_once() {
        let raw_output = "Hello team,\r\n\r\n* **AP:** Send the update.\r\n\r\nThanks!";
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: raw_output.to_string(),
            calls: Arc::clone(&calls),
        };
        let task = format!(
            "帮我写一条通知告诉大家{}",
            std::iter::repeat_n("明天上午系统维护期间不要提交报销并确认 API rollout", 12)
                .collect::<Vec<_>>()
                .join("，")
        );
        let transcript = format!("嗯 voice flow {task}");
        let insertion = RecordingInsertionAdapter::default();
        let committed = Arc::clone(&insertion.committed);
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine(&transcript),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("filler-prefixed generic instructed input should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            committed
                .lock()
                .expect("commit tracking mutex should not be poisoned")
                .len(),
            1
        );
        assert_eq!(runtime.session_history().len(), 1);
        assert_eq!(summary.session_kind, SessionKind::InstructedDictation);
        assert_eq!(summary.committed_text, raw_output);
        assert_eq!(summary.final_state, SessionState::Committed);
        let ledger_root = env::temp_dir().join(format!(
            "voiceflow-multiline-integration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let settings_path = ledger_root.join("settings.json");
        // Replaying the same completed session must not count it a second time.
        for _ in 0..2 {
            let history = crate::history_ledger::append_live_host_history(
                &settings_path,
                "multiline-integration",
                runtime.session_history(),
                &shared_protocol::HistoryRetention::default(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(history.entries.len(), 1);
            assert_eq!(history.entries[0].text.as_deref(), Some(raw_output));
            let usage = append_live_host_usage(
                &settings_path,
                "multiline-integration",
                runtime.session_history(),
                &[],
            )
            .unwrap()
            .unwrap();
            assert_eq!(usage.total_sessions, 1);
            assert_eq!(usage.instructed_dictation_sessions, 1);
        }
        std::fs::remove_dir_all(&ledger_root).unwrap();
        assert!(summary.route_decision.dictation_routing.is_none());
        let execution = summary
            .instructed_dictation_execution
            .expect("instructed execution should be recorded");
        assert_eq!(execution.parser_result, "generic_task");
        assert!(execution.provider_diagnostics.instruction_char_count > 60);
        assert_eq!(execution.provider_diagnostics.content_char_count, 0);
        assert_eq!(
            runtime
                .insertion_adapter
                .committed
                .lock()
                .expect("commit tracking mutex should not be poisoned")
                .as_slice(),
            ["Hello team,\n\n• AP: Send the update.\n\nThanks!"]
        );
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(diagnostics_text.contains("instructed_trigger_matched=true"));
        assert!(diagnostics_text.contains("instructed_trigger_alias=spaced"));
        assert!(diagnostics_text.contains("instructed_leading_filler_removed=true"));
        assert!(diagnostics_text.contains("instructed_task_present=true"));
        assert!(!diagnostics_text.contains(&task));
        assert!(diagnostics_text.contains("insertion_text_multiline=true"));
        assert!(diagnostics_text.contains("insertion_format_normalized=true"));
        assert!(diagnostics_text.contains("insertion_line_count=5"));
    }

    #[test]
    #[cfg(windows)]
    fn instructed_transport_outcomes_commit_exactly_once() {
        use crate::windows_insertion::{
            combine_primary_and_restore_results, commit_text_with_transports,
        };
        use std::cell::RefCell;

        struct MockInsertion {
            prepare_fails: bool,
            restore_fails: bool,
            inserted: RefCell<Vec<String>>,
        }
        impl TextInsertionAdapter for MockInsertion {
            fn commit_text(&self, _: u64, text: &str) -> CommitResult {
                commit_text_with_transports(
                    text,
                    |text| {
                        self.inserted.borrow_mut().push(text.to_string());
                        Ok(())
                    },
                    |text| {
                        if self.prepare_fails {
                            return Err("clipboard unavailable".to_string());
                        }
                        self.inserted.borrow_mut().push(text.to_string());
                        combine_primary_and_restore_results(
                            Ok(()),
                            if self.restore_fails {
                                Err("restore unavailable".to_string())
                            } else {
                                Ok(())
                            },
                            |_| {},
                        )
                    },
                    |_| {},
                )
            }
        }
        struct CheckedExecutor {
            inner: FixedInstructedDictationExecutor,
        }
        impl InstructedDictationExecutor for CheckedExecutor {
            fn transform(
                &self,
                request: &InstructedDictationTransformRequest,
            ) -> Result<InstructedDictationTransformResponse, String> {
                assert_eq!(
                    request.instruction_text,
                    "write a short update for the team"
                );
                assert!(request.content_text.is_empty());
                self.inner.transform(request)
            }
        }

        for (raw, prepared, prepare_fails, restore_fails, transport) in [
            (
                "Done.",
                "Done.",
                false,
                false,
                CommitTransport::DirectUnicodeSendInput,
            ),
            (
                "Hi\n\n* **Team:** update",
                "Hi\n\n• Team: update",
                false,
                false,
                CommitTransport::ClipboardPasteFallback,
            ),
            (
                "Hi\n\n* **Team:** update",
                "Hi\n\n• Team: update",
                true,
                false,
                CommitTransport::ClipboardPasteFallback,
            ),
            (
                "Hi\n\n* **Team:** update",
                "Hi\n\n• Team: update",
                false,
                true,
                CommitTransport::ClipboardPasteFallback,
            ),
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let mut runtime = HostRuntime::with_speech_engine(
                RuntimeSettings::default(),
                build_test_engine("Voice Flow write a short update for the team"),
                TestShortcutAdapter::default(),
                FixedAudioAdapter {
                    captured_audio: build_test_audio(),
                },
                MockInsertion {
                    prepare_fails,
                    restore_fails,
                    inserted: RefCell::new(Vec::new()),
                },
                RecordingOverlayAdapter::default(),
                RecordingFeedbackAdapter::default(),
                TemporarySelectedTextExecutor,
                TemporarySelectedTextExecutor,
                CheckedExecutor {
                    inner: FixedInstructedDictationExecutor {
                        output_text: raw.to_string(),
                        calls: Arc::clone(&calls),
                    },
                },
            );
            runtime.bootstrap().unwrap();
            let summary = runtime
                .run_session_for_trigger(TriggerMode::GlobalShortcut)
                .unwrap();
            assert_eq!(summary.session_kind, SessionKind::InstructedDictation);
            assert_eq!(summary.final_state, if prepare_fails { SessionState::Failed } else { SessionState::Committed });
            assert_eq!(summary.commit_transport, transport);
            assert_eq!(summary.committed_text, raw);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            if prepare_fails {
                assert!(runtime.insertion_adapter.inserted.borrow().is_empty());
            } else {
                assert_eq!(runtime.insertion_adapter.inserted.borrow().as_slice(), [prepared]);
            }
        }
    }

    #[test]
    fn instructed_dictation_skips_normal_dictation_refiner() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "The timeline is confirmed.".to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine_with_failing_refiner(
                "Voice Flow, translate the following into English: 时间线已经确认。",
            ),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("instructed dictation should not require normal dictation refinement");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(summary.session_kind, SessionKind::InstructedDictation);
        assert_eq!(summary.committed_text, "The timeline is confirmed.");
        assert_eq!(summary.refine_diagnostics, None);
        assert!(!summary.degraded_to_asr);
    }

    #[test]
    fn voice_flow_chinese_translation_routes_to_instructed_dictation_not_general_draft() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "I don't think it's a wise choice to take on this task right now."
                .to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine_with_failing_refiner(
                "voice flow 把下面这段话翻译成英文我认为现在接下这个任务不是一个明智的选择",
            ),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("voice flow ASR variant should route to instructed dictation");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(summary.session_kind, SessionKind::InstructedDictation);
        assert_eq!(
            summary.route_decision.route_name,
            RouteName::InstructedDictation
        );
        assert_eq!(
            summary.committed_text,
            "I don't think it's a wise choice to take on this task right now."
        );
        assert_eq!(
            summary.recognized_text,
            REDACTED_INSTRUCTED_DICTATION_SOURCE
        );
        assert_eq!(summary.wake_phrase_execution, None);
        assert!(
            summary
                .instructed_dictation_execution
                .as_ref()
                .is_some_and(|execution| execution.parser_result == "generic_task")
        );
        assert_eq!(summary.refine_diagnostics, None);
        assert!(!summary.degraded_to_asr);
    }

    #[test]
    fn observed_help_me_chinese_translation_bypasses_general_draft() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "Ignore those two files for now; they are not official documents, so they can be skipped."
                .to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine_with_failing_refiner(
                "voice flow 帮我把下面这段话翻译成英文那两个文件先别管他们不是正式文档可以先不更新",
            ),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("observed command should route to instructed dictation");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(summary.session_kind, SessionKind::InstructedDictation);
        assert_eq!(
            summary.route_decision.route_name,
            RouteName::InstructedDictation
        );
        assert_eq!(summary.wake_phrase_execution, None);
        assert!(summary.instructed_dictation_execution.is_some());
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!diagnostics_text.contains("GeneralDraft"));
        assert!(!diagnostics_text.contains("general drafting scaffold"));
    }

    #[test]
    fn trigger_phrase_in_middle_stays_normal_dictation() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "should not be used".to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("我觉得 Voice Flow, translate this into English 这个功能挺有用。"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("normal dictation should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(summary.session_kind, SessionKind::Dictation);
        assert_eq!(summary.instructed_dictation_execution, None);
    }

    #[test]
    fn command_like_dictation_without_trigger_stays_normal_dictation() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "should not be used".to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("帮我把下面这句话翻译成英文我建议暂时停止新功能"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("normal dictation should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(summary.session_kind, SessionKind::Dictation);
        assert_eq!(summary.instructed_dictation_execution, None);
        assert_eq!(summary.wake_phrase_execution, None);
    }

    #[test]
    fn disabled_trigger_phrase_keeps_normal_dictation() {
        let mut settings = RuntimeSettings::default();
        settings.wake_phrase.enabled = false;
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "should not be used".to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            settings,
            build_test_engine(
                "Voice Flow, translate the following into English: 明天我们确认时间线。",
            ),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("normal dictation should succeed");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(summary.session_kind, SessionKind::Dictation);
    }

    #[test]
    fn hot_reloaded_custom_trigger_replaces_default_across_the_real_host_route() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "Confirmed timeline.".to_string(),
            calls: Arc::clone(&calls),
        };
        let insertion = RecordingInsertionAdapter::default();
        let committed = Arc::clone(&insertion.committed);
        let speech_engine = SpeechEngine::new(
            RoutingPolicy::default(),
            SequenceTranscriber {
                transcripts: Mutex::new(VecDeque::from([
                    "hi voice flow 帮我写一条通知".to_string(),
                    "嗯，小王啊，帮我写一条通知".to_string(),
                    "voice flow 帮我写一条通知".to_string(),
                ])),
            },
            DeterministicRefiner,
        );
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            speech_engine,
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let first = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("default English trigger should route to instructed input");
        assert_eq!(first.session_kind, SessionKind::InstructedDictation);
        assert!(first.route_decision.dictation_routing.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            committed
                .lock()
                .expect("commit tracking mutex should not be poisoned")
                .len(),
            1
        );
        assert_eq!(runtime.session_history().len(), 1);

        let usage_root = env::temp_dir().join(format!(
            "voiceflow-hot-reload-trigger-usage-{}",
            std::process::id()
        ));
        let settings_path = usage_root.join("settings.json");
        let first_usage = append_live_host_usage(
            &settings_path,
            "hot-reload-trigger",
            runtime.session_history(),
            &[],
        )
        .expect("first instructed session should persist usage")
        .expect("usage summary should be available");
        assert_eq!(first_usage.total_sessions, 1);
        assert_eq!(first_usage.instructed_dictation_sessions, 1);

        let mut updated = runtime.settings().clone();
        updated.wake_phrase.phrase = "小王啊".to_string();
        runtime
            .apply_runtime_settings(updated)
            .expect("updated trigger phrase should apply before the next session");
        let second = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("hot-reloaded Chinese trigger should route to instructed input");
        assert_eq!(second.session_kind, SessionKind::InstructedDictation);
        assert!(second.route_decision.dictation_routing.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            committed
                .lock()
                .expect("commit tracking mutex should not be poisoned")
                .len(),
            2
        );
        assert_eq!(runtime.session_history().len(), 2);

        let second_usage = append_live_host_usage(
            &settings_path,
            "hot-reload-trigger",
            runtime.session_history(),
            &[],
        )
        .expect("second instructed session should persist usage")
        .expect("usage summary should be available");
        assert_eq!(second_usage.total_sessions, 2);
        assert_eq!(second_usage.instructed_dictation_sessions, 2);

        let third = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("old default trigger should remain ordinary dictation");
        assert_eq!(third.session_kind, SessionKind::Dictation);
        assert!(third.route_decision.dictation_routing.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            committed
                .lock()
                .expect("commit tracking mutex should not be poisoned")
                .len(),
            3
        );
        assert_eq!(runtime.session_history().len(), 3);

        let third_usage = append_live_host_usage(
            &settings_path,
            "hot-reload-trigger",
            runtime.session_history(),
            &[],
        )
        .expect("ordinary dictation should persist usage once")
        .expect("usage summary should be available");
        assert_eq!(third_usage.total_sessions, 3);
        assert_eq!(third_usage.instructed_dictation_sessions, 2);
        assert_eq!(third_usage.dictation_sessions, 1);

        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(diagnostics_text.contains("wake_phrase_source=default wake_phrase_revision=0"));
        assert!(diagnostics_text.contains("wake_phrase_source=settings wake_phrase_revision=1"));
        assert!(diagnostics_text.contains("instructed_trigger_matched=true"));
        assert!(diagnostics_text.contains("instructed_trigger_matched=false"));
        assert!(diagnostics_text.contains("effective_session_kind=instructed_dictation"));
        assert!(diagnostics_text.contains("effective_session_kind=dictation"));
        assert!(!diagnostics_text.contains("帮我写一条通知"));

        let _ = std::fs::remove_dir_all(usage_root);
    }

    #[test]
    fn trigger_without_content_fails_softly_before_insertion() {
        let calls = Arc::new(AtomicUsize::new(0));
        let instructed_executor = FixedInstructedDictationExecutor {
            output_text: "should not be used".to_string(),
            calls: Arc::clone(&calls),
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("嗯 Voice Flow……"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            instructed_executor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let error = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect_err("missing instructed content should fail softly");

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(error.session_kind, SessionKind::InstructedDictation);
        assert_eq!(error.failure_phase, SessionFailurePhase::Executing);
        assert!(error.message.contains("no content was detected"));
        assert_eq!(runtime.session_history().len(), 0);
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(diagnostics_text.contains("instructed_trigger_matched=true"));
        assert!(diagnostics_text.contains("instructed_task_present=false"));
        assert!(!diagnostics_text.contains("嗯 Voice Flow"));
    }

    #[test]
    fn provider_selected_text_edit_redacts_summary_and_diagnostics_text() {
        let selected_text = "secret selected paragraph about staged files";
        let provider_output = "secret provider rewrite";
        let selected_text_calls = Arc::new(AtomicUsize::new(0));
        let selected_text_executor = FixedSelectedTextExecutor {
            output_text: provider_output.to_string(),
            action: SelectedTextExecutionAction::GeneralProviderEdit,
            provider_diagnostics: Some(SelectedTextProviderDiagnostics {
                profile_label: "BestQuality".to_string(),
                model_code: "deepseek-v4-flash".to_string(),
                provider_preset: Some("bailian".to_string()),
                provider_key_source: Some("credential_store".to_string()),
                provider_attempted: true,
                provider_succeeded: true,
                deterministic_fallback_used: false,
                fallback_reason: None,
                selected_text_char_count: selected_text.chars().count(),
                output_char_count: provider_output.chars().count(),
            }),
            calls: Arc::clone(&selected_text_calls),
        };
        let overlay = RecordingOverlayAdapter::default();
        let overlay_summaries = Arc::clone(&overlay.summaries);
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("translate it into English"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter {
                selected_text: Some(selected_text.to_string()),
                replace_transport: CommitTransport::ClipboardSelectionReplace,
                ..Default::default()
            },
            overlay,
            RecordingFeedbackAdapter::default(),
            selected_text_executor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("provider selected-text session should succeed");

        assert_eq!(selected_text_calls.load(Ordering::SeqCst), 1);
        assert_eq!(summary.session_kind, SessionKind::SelectedTextEdit);
        assert_eq!(summary.committed_text, REDACTED_SELECTED_TEXT_PROVIDER_EDIT);
        assert_eq!(
            summary.committed_text_count,
            Some(count_words_entered(provider_output))
        );
        assert_eq!(summary.audio_duration_ms, build_test_audio().duration_ms);
        let selected_text_execution = summary
            .selected_text_execution
            .as_ref()
            .expect("selected-text summary should be present");
        assert_eq!(
            selected_text_execution.action,
            SelectedTextExecutionAction::GeneralProviderEdit
        );
        let provider_diagnostics = selected_text_execution
            .provider_diagnostics
            .as_ref()
            .expect("provider diagnostics should be present");
        assert!(provider_diagnostics.provider_attempted);
        assert!(provider_diagnostics.provider_succeeded);
        assert_eq!(
            provider_diagnostics.selected_text_char_count,
            selected_text.chars().count()
        );
        assert_eq!(
            provider_diagnostics.output_char_count,
            provider_output.chars().count()
        );

        let mirrored_summaries = overlay_summaries
            .lock()
            .expect("overlay summaries mutex should not be poisoned");
        assert_eq!(mirrored_summaries.len(), 1);
        assert_eq!(
            mirrored_summaries[0].committed_text,
            REDACTED_SELECTED_TEXT_PROVIDER_EDIT
        );
        let diagnostics_text = runtime
            .diagnostics_snapshot()
            .iter()
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!diagnostics_text.contains(selected_text));
        assert!(!diagnostics_text.contains(provider_output));
        assert!(diagnostics_text.contains("selected-text provider action=GeneralProviderEdit"));
        assert!(diagnostics_text.contains("provider_preset=bailian"));
        assert!(diagnostics_text.contains("provider_key_source=credential_store"));
        assert!(diagnostics_text.contains("selected_text_chars="));
        assert!(diagnostics_text.contains("output_chars="));
    }

    #[test]
    fn text_seeded_selected_text_probe_bypasses_audio_capture() {
        let insertion = RecordingInsertionAdapter {
            selected_text: Some("please rewrite me".to_string()),
            replace_transport: CommitTransport::ClipboardSelectionReplace,
            ..Default::default()
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("this should not be used"),
            TestShortcutAdapter::default(),
            PanicAudioAdapter,
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_text_seeded_selected_text_edit_probe("uppercase")
            .expect("text-seeded selected-text probe should succeed");

        assert_eq!(summary.session_kind, SessionKind::SelectedTextEdit);
        assert_eq!(summary.recognized_text, "uppercase");
        assert_eq!(summary.audio_duration_ms, 0);
        assert_eq!(summary.recording_start_latency_ms, None);
        assert_eq!(summary.committed_text, "PLEASE REWRITE ME");
    }

    #[test]
    fn selection_replacement_preserves_newlines_and_normalizes_formatting() {
        let insertion = RecordingInsertionAdapter {
            selected_text: Some("replace this".to_string()),
            replace_transport: CommitTransport::ClipboardSelectionReplace,
            ..Default::default()
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("unused"),
            TestShortcutAdapter::default(),
            PanicAudioAdapter,
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        runtime
            .run_selection_replace_probe("First\r\n\r\n- **Second**")
            .expect("selection replacement should succeed");

        assert_eq!(
            runtime
                .insertion_adapter
                .replaced
                .lock()
                .expect("replace tracking mutex should not be poisoned")
                .as_slice(),
            ["First\n\n• Second"]
        );
    }

    #[test]
    fn selection_replace_probe_empty_capture_returns_parseable_compatibility_failure() {
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("unused"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let error = runtime
            .run_selection_replace_probe("VoiceFlow probe replacement")
            .expect_err("replace probe should fail when no selected text is captured");

        let issue = crate::selected_text_compatibility::parse_selected_text_failure(&error)
            .expect("replace probe failure should stay parseable for compatibility reporting");
        assert_eq!(issue.stage, "SelectionRead");
        assert_eq!(issue.reason, "Captured selection was empty");
    }

    #[test]
    fn session_summary_preserves_asr_diagnostics() {
        let diagnostics = AsrDiagnostics {
            backend: "test-backend".to_string(),
            worker_request_id: Some(42),
            worker_model_load_ms: Some(777),
            worker_restarted: false,
            metrics: vec![AsrLatencyMetric {
                name: "decode_ms".to_string(),
                value_ms: 19,
            }],
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine_with_diagnostics("hello world from test", Some(diagnostics.clone())),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("dictation session should succeed");

        assert_eq!(summary.asr_diagnostics, Some(diagnostics));
    }

    #[test]
    fn session_summary_preserves_refine_degradation_reason() {
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine_with_failing_refiner(CLOUD_DICTATION_TEXT),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("session should still succeed with recognized text");

        assert!(summary.degraded_to_asr);
        assert_eq!(summary.committed_text, CLOUD_DICTATION_TEXT);
        assert!(
            summary
                .fallback_reason
                .expect("fallback reason should be preserved")
                .contains("test refiner failed"),
            "refine fallback should remain visible in the session summary"
        );
        assert!(
            runtime
                .diagnostics_snapshot()
                .iter()
                .any(|event| event.message.contains("speech fallback:")),
            "diagnostics should record the degraded-success path"
        );
        let latency_diagnostic = runtime
            .diagnostics_snapshot()
            .iter()
            .find(|event| event.message.contains("post_recording_latency_ms="))
            .expect("session diagnostics should include phase latency fields");
        for field in [
            "audio_stop_finalize_ms=",
            "post_stop_prepare_ms=",
            "recording_feedback_ms=",
            "selected_text_probe_ms=",
            "speech_engine_recognize_ms=",
            "asr_total_ms=",
            "asr_host_total_ms=",
            "asr_host_audio_prepare_ms=",
            "asr_host_wav_write_ms=",
            "asr_host_worker_roundtrip_ms=",
            "asr_host_temp_cleanup_ms=",
            "post_asr_processing_ms=",
            "provider_config_prepare_ms=",
            "refine_total_ms=",
            "prompt_load_ms=",
            "payload_build_ms=",
            "provider_request_ms=",
            "provider_output_validation_ms=",
            "insertion_ms=",
            "post_recording_unattributed_ms=",
            "prompt_char_count=",
            "transcript_char_count=",
            "refine_model=",
            "guard_detected=",
            "provider_output_rejected=",
            "fallback_used=",
        ] {
            assert!(
                latency_diagnostic.message.contains(field),
                "latency diagnostic should include {field}"
            );
        }
    }

    #[test]
    fn fails_early_when_audio_looks_like_silence() {
        let feedback = RecordingFeedbackAdapter::default();
        let failure_count = Arc::clone(&feedback.failures);
        let stop_count = Arc::clone(&feedback.stops);
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("hallucinated text from silence"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_silent_test_audio(600),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            feedback,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let error = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect_err("silence gate should fail before ASR-driven commit");

        assert!(error.message.contains("minimum speech activity threshold"));
        assert_eq!(error.failure_phase, SessionFailurePhase::Recording);
        assert_eq!(failure_count.load(Ordering::SeqCst), 0);
        assert_eq!(stop_count.load(Ordering::SeqCst), 1);
        assert!(
            runtime
                .diagnostics_snapshot()
                .iter()
                .any(|event| event.message.contains("looked like silence before ASR")),
            "diagnostics should record the silence gate"
        );
    }

    #[test]
    fn session_summary_surfaces_commit_failure_reason() {
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("hello world from commit failure"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter {
                commit_failure: Some("target app rejected simulated input".to_string()),
                ..Default::default()
            },
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("session should still return a summary even when commit fails");

        assert_eq!(summary.final_state, SessionState::Failed);
        assert_eq!(
            summary.commit_failure_reason,
            Some("target app rejected simulated input".to_string())
        );
        assert_eq!(summary.commit_transport, CommitTransport::Unknown);
    }

    #[test]
    fn session_summary_preserves_dictation_commit_transport() {
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("hello world from fallback"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter {
                commit_transport: CommitTransport::ClipboardPasteFallback,
                ..Default::default()
            },
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("dictation session should succeed");

        assert_eq!(
            summary.commit_transport,
            CommitTransport::ClipboardPasteFallback
        );
    }

    #[test]
    fn session_summary_preserves_selected_text_commit_transport() {
        let insertion = RecordingInsertionAdapter {
            selected_text: Some("please rewrite me".to_string()),
            replace_transport: CommitTransport::ClipboardSelectionReplace,
            ..Default::default()
        };
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("uppercase"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("selected-text session should succeed");

        assert_eq!(
            summary.commit_transport,
            CommitTransport::ClipboardSelectionReplace
        );
    }

    #[test]
    fn standard_diagnostics_verbosity_suppresses_overlay_events() {
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("hello world from standard diagnostics"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("session should succeed");

        assert!(
            runtime
                .diagnostics_snapshot()
                .iter()
                .all(|event| !matches!(event.category, DiagnosticCategory::Overlay)),
            "standard diagnostics should suppress overlay echo events"
        );
    }

    #[test]
    fn verbose_diagnostics_verbosity_preserves_overlay_events() {
        let mut settings = RuntimeSettings::default();
        settings.diagnostics_verbosity = DiagnosticsVerbosity::Verbose;
        let mut runtime = HostRuntime::with_speech_engine(
            settings,
            build_test_engine("hello world from verbose diagnostics"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            RecordingInsertionAdapter::default(),
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect("session should succeed");

        assert!(
            runtime
                .diagnostics_snapshot()
                .iter()
                .any(|event| matches!(event.category, DiagnosticCategory::Overlay)),
            "verbose diagnostics should preserve overlay echo events"
        );
    }

    #[test]
    fn push_to_talk_dictation_waits_for_release_instead_of_second_trigger() {
        let release_calls = Arc::new(AtomicUsize::new(0));
        let insertion = RecordingInsertionAdapter::default();
        let strict_capture_calls = Arc::clone(&insertion.strict_capture_calls);
        let fast_capture_calls = Arc::clone(&insertion.fast_capture_calls);
        let mut settings = RuntimeSettings::default();
        settings.shortcut_mode = ShortcutMode::PushToTalk;
        let mut runtime = HostRuntime::with_speech_engine(
            settings,
            build_test_engine("hello from push to talk"),
            TestShortcutAdapter {
                release_calls: Arc::clone(&release_calls),
            },
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let session_started_at = Instant::now();
        let summary = runtime
            .run_live_session_for_trigger(TriggerMode::GlobalShortcut, None)
            .expect("push-to-talk dictation session should succeed");

        assert!(session_started_at.elapsed() < Duration::from_millis(500));
        assert_eq!(summary.session_kind, SessionKind::Dictation);
        assert_eq!(release_calls.load(Ordering::SeqCst), 1);
        assert_eq!(strict_capture_calls.load(Ordering::SeqCst), 0);
        assert_eq!(fast_capture_calls.load(Ordering::SeqCst), 1);
        assert!(
            runtime
                .diagnostics_snapshot()
                .iter()
                .any(|event| event.message.contains("waiting for push-to-talk release")),
            "diagnostics should reflect the release-based stop path"
        );
        assert!(
            runtime
                .diagnostics_snapshot()
                .iter()
                .any(|event| event.message.contains("selected_text_probe_mode=skipped")),
            "diagnostics should record the pre-recording skip"
        );
        assert!(
            runtime.diagnostics_snapshot().iter().any(|event| {
                event.message.contains("selected_text_probe_mode=fast")
                    && event
                        .message
                        .contains("selected_text_probe_result=no_selection")
            }),
            "diagnostics should record the fast no-selection result"
        );
    }

    #[test]
    fn push_to_talk_uses_fast_selected_text_probe_after_recording() {
        let mut settings = RuntimeSettings::default();
        settings.shortcut_mode = ShortcutMode::PushToTalk;
        let insertion = RecordingInsertionAdapter {
            selected_text: Some("please rewrite me".to_string()),
            replace_transport: CommitTransport::ClipboardSelectionReplace,
            ..Default::default()
        };
        let strict_capture_calls = Arc::clone(&insertion.strict_capture_calls);
        let fast_capture_calls = Arc::clone(&insertion.fast_capture_calls);
        let mut runtime = HostRuntime::with_speech_engine(
            settings,
            build_test_engine("uppercase"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::GlobalShortcut)
            .expect(
                "push-to-talk primary shortcut should route into selected-text edit after release",
            );

        assert_eq!(summary.session_kind, SessionKind::SelectedTextEdit);
        assert_eq!(summary.committed_text, "PLEASE REWRITE ME");
        assert_eq!(
            summary.commit_transport,
            CommitTransport::ClipboardSelectionReplace
        );
        assert_eq!(strict_capture_calls.load(Ordering::SeqCst), 0);
        assert_eq!(fast_capture_calls.load(Ordering::SeqCst), 1);
        assert!(
            runtime.diagnostics_snapshot().iter().any(|event| event
                .message
                .contains("captured selected-text context after push-to-talk release")),
            "diagnostics should record the deferred selected-text capture"
        );
        assert!(runtime.diagnostics_snapshot().iter().any(|event| {
            event.message.contains("selected_text_probe_mode=fast")
                && event
                    .message
                    .contains("selected_text_probe_result=selected")
        }));
    }

    #[test]
    fn explicit_selected_text_edit_keeps_strict_capture() {
        let insertion = RecordingInsertionAdapter {
            selected_text: Some("please rewrite me".to_string()),
            ..Default::default()
        };
        let strict_capture_calls = Arc::clone(&insertion.strict_capture_calls);
        let fast_capture_calls = Arc::clone(&insertion.fast_capture_calls);
        let mut runtime = HostRuntime::with_speech_engine(
            RuntimeSettings::default(),
            build_test_engine("uppercase"),
            TestShortcutAdapter::default(),
            FixedAudioAdapter {
                captured_audio: build_test_audio(),
            },
            insertion,
            RecordingOverlayAdapter::default(),
            RecordingFeedbackAdapter::default(),
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
            TemporarySelectedTextExecutor,
        );

        runtime.bootstrap().expect("bootstrap should succeed");
        let summary = runtime
            .run_session_for_trigger(TriggerMode::EditShortcut)
            .expect("explicit selected-text edit should succeed");

        assert_eq!(summary.session_kind, SessionKind::SelectedTextEdit);
        assert_eq!(strict_capture_calls.load(Ordering::SeqCst), 1);
        assert_eq!(fast_capture_calls.load(Ordering::SeqCst), 0);
        assert!(runtime.diagnostics_snapshot().iter().any(|event| {
            event.message.contains("selected_text_probe_mode=strict")
                && event
                    .message
                    .contains("selected_text_probe_result=selected")
        }));
    }
}
