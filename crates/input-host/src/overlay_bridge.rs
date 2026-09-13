use crate::app_paths::overlay_runtime_script_path;
use crate::host::{OverlayAdapter, OverlayPublishSummary, StdoutOverlayAdapter};
#[cfg(windows)]
use crate::windows_overlay::WindowsOverlayAdapter;
use serde::Serialize;
use shared_protocol::{
    DiagnosticsVerbosity, FailedSessionSummary, OverlayStatus, RefinementQuality, RuntimeSettings,
    SessionSummary, ShortcutMode, shortcut_to_string,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_RECENT_OVERLAY_EVENTS: usize = 8;

#[derive(Debug, Serialize)]
struct OverlayRuntimeSnapshot {
    generated_at_epoch_ms: u128,
    locale: &'static str,
    primary_shortcut: String,
    shortcut_mode: ShortcutMode,
    refinement_quality: RefinementQuality,
    silence_gate_level: u8,
    diagnostics_verbosity: DiagnosticsVerbosity,
    audio_feedback_enabled: bool,
    latest_status: Option<OverlayStatus>,
    recent_statuses: Vec<OverlayStatus>,
    last_session_summary: Option<SessionSummary>,
    last_failure_summary: Option<FailedSessionSummary>,
}

#[derive(Debug)]
pub struct OverlayStateScriptAdapter {
    script_path: PathBuf,
    settings: Mutex<OverlaySnapshotSettings>,
    recent_statuses: Mutex<Vec<OverlayStatus>>,
    last_session_summary: Mutex<Option<SessionSummary>>,
    last_failure_summary: Mutex<Option<FailedSessionSummary>>,
}

#[derive(Clone, Debug)]
struct OverlaySnapshotSettings {
    locale: &'static str,
    primary_shortcut: String,
    shortcut_mode: ShortcutMode,
    refinement_quality: RefinementQuality,
    silence_gate_level: u8,
    diagnostics_verbosity: DiagnosticsVerbosity,
    audio_feedback_enabled: bool,
}

impl OverlaySnapshotSettings {
    fn from_runtime(settings: &RuntimeSettings) -> Self {
        Self {
            locale: overlay_locale(&settings.system_language),
            primary_shortcut: shortcut_to_string(&settings.dictation_shortcut),
            shortcut_mode: settings.shortcut_mode.clone(),
            refinement_quality: settings.refinement_quality.clone(),
            silence_gate_level: settings.silence_gate_level,
            diagnostics_verbosity: settings.diagnostics_verbosity.clone(),
            audio_feedback_enabled: settings.audio_feedback_enabled,
        }
    }
}

impl OverlayStateScriptAdapter {
    pub fn new(settings: &RuntimeSettings) -> Result<Self, String> {
        let script_path = overlay_runtime_script_path()?;
        let adapter = Self {
            script_path,
            settings: Mutex::new(OverlaySnapshotSettings::from_runtime(settings)),
            recent_statuses: Mutex::new(Vec::new()),
            last_session_summary: Mutex::new(None),
            last_failure_summary: Mutex::new(None),
        };
        adapter.reset()?;
        Ok(adapter)
    }

    pub fn script_path(&self) -> &Path {
        &self.script_path
    }

    pub fn reset(&self) -> Result<(), String> {
        self.write_snapshot(None, Vec::new(), None, None)
    }

    fn write_snapshot(
        &self,
        latest_status: Option<OverlayStatus>,
        recent_statuses: Vec<OverlayStatus>,
        last_session_summary: Option<SessionSummary>,
        last_failure_summary: Option<FailedSessionSummary>,
    ) -> Result<(), String> {
        let settings = self
            .settings
            .lock()
            .map_err(|_| "overlay settings mutex should not be poisoned".to_string())?
            .clone();
        let snapshot = OverlayRuntimeSnapshot {
            generated_at_epoch_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| format!("failed to compute overlay state timestamp: {error}"))?
                .as_millis(),
            locale: settings.locale,
            primary_shortcut: settings.primary_shortcut,
            shortcut_mode: settings.shortcut_mode,
            refinement_quality: settings.refinement_quality,
            silence_gate_level: settings.silence_gate_level,
            diagnostics_verbosity: settings.diagnostics_verbosity,
            audio_feedback_enabled: settings.audio_feedback_enabled,
            latest_status,
            recent_statuses,
            last_session_summary,
            last_failure_summary,
        };
        let snapshot_json = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| format!("failed to encode overlay runtime snapshot: {error}"))?;
        let script_body = format!(
            "window.__VOICEFLOW_OVERLAY_RUNTIME__ = {};\n",
            snapshot_json
        );

        if let Some(parent) = self.script_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create overlay runtime state directory {}: {}",
                    parent.display(),
                    error
                )
            })?;
        }

        fs::write(&self.script_path, script_body).map_err(|error| {
            format!(
                "failed to write overlay runtime state script {}: {}",
                self.script_path.display(),
                error
            )
        })
    }

    fn write_current_snapshot(&self) -> Result<(), String> {
        let recent_statuses = self
            .recent_statuses
            .lock()
            .expect("overlay runtime script mutex should not be poisoned")
            .clone();
        let latest_status = recent_statuses.last().cloned();
        let last_session_summary = self
            .last_session_summary
            .lock()
            .expect("overlay runtime script summary mutex should not be poisoned")
            .clone();
        let last_failure_summary = self
            .last_failure_summary
            .lock()
            .expect("overlay runtime script failure mutex should not be poisoned")
            .clone();
        self.write_snapshot(
            latest_status,
            recent_statuses,
            last_session_summary,
            last_failure_summary,
        )
    }
}

impl OverlayAdapter for OverlayStateScriptAdapter {
    fn publish(&self, status: OverlayStatus) -> OverlayPublishSummary {
        let mut recent = self
            .recent_statuses
            .lock()
            .expect("overlay runtime script mutex should not be poisoned");
        recent.push(status.clone());
        if recent.len() > MAX_RECENT_OVERLAY_EVENTS {
            let drain_count = recent.len() - MAX_RECENT_OVERLAY_EVENTS;
            recent.drain(0..drain_count);
        }
        drop(recent);

        if let Err(error) = self.write_current_snapshot() {
            eprintln!("{error}");
        }
        OverlayPublishSummary::default()
    }

    fn publish_summary(&self, summary: &SessionSummary) {
        let mut last_summary = self
            .last_session_summary
            .lock()
            .expect("overlay runtime script summary mutex should not be poisoned");
        *last_summary = Some(summary.clone());
        drop(last_summary);

        if let Err(error) = self.write_current_snapshot() {
            eprintln!("{error}");
        }
    }

    fn publish_failure(&self, failure: &FailedSessionSummary) {
        let mut last_failure = self
            .last_failure_summary
            .lock()
            .expect("overlay runtime script failure mutex should not be poisoned");
        *last_failure = Some(failure.clone());
        drop(last_failure);

        if let Err(error) = self.write_current_snapshot() {
            eprintln!("{error}");
        }
    }

    fn update_settings(&self, settings: &RuntimeSettings) {
        if let Ok(mut current) = self.settings.lock() {
            *current = OverlaySnapshotSettings::from_runtime(settings);
        }
    }
}

#[derive(Debug)]
pub struct MirroringOverlayAdapter {
    stdout: StdoutOverlayAdapter,
    script: OverlayStateScriptAdapter,
    #[cfg(windows)]
    native: WindowsOverlayAdapter,
}

impl MirroringOverlayAdapter {
    pub fn new(settings: &RuntimeSettings) -> Result<Self, String> {
        Ok(Self {
            stdout: StdoutOverlayAdapter,
            script: OverlayStateScriptAdapter::new(settings)?,
            #[cfg(windows)]
            native: WindowsOverlayAdapter::new(&settings.system_language),
        })
    }

    pub fn script_path(&self) -> &Path {
        self.script.script_path()
    }
}

impl OverlayAdapter for MirroringOverlayAdapter {
    fn publish(&self, status: OverlayStatus) -> OverlayPublishSummary {
        let _ = self.stdout.publish(status.clone());
        #[cfg(windows)]
        let native_summary = self.native.publish_status(&status);
        #[cfg(not(windows))]
        let native_summary = OverlayPublishSummary::default();
        let _ = self.script.publish(status);
        native_summary
    }

    fn publish_summary(&self, summary: &SessionSummary) {
        #[cfg(windows)]
        self.native.publish_summary(summary);
        self.script.publish_summary(summary);
    }

    fn publish_failure(&self, failure: &FailedSessionSummary) {
        #[cfg(windows)]
        self.native.publish_failure(failure);
        self.script.publish_failure(failure);
    }

    fn update_settings(&self, settings: &RuntimeSettings) {
        #[cfg(windows)]
        self.native.update_settings(settings);
        self.script.update_settings(settings);
    }
}

fn overlay_locale(system_language: &shared_protocol::SystemLanguage) -> &'static str {
    match system_language {
        shared_protocol::SystemLanguage::Chinese => "zh",
        shared_protocol::SystemLanguage::English => "en",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_protocol::{SessionFailurePhase, SessionKind, SessionState};

    #[test]
    fn writes_runtime_snapshot_script_with_latest_status() {
        let temp_dir = std::env::temp_dir().join("voiceflow-overlay-bridge-test");
        let script_path = temp_dir.join("runtime-state.js");
        let adapter = OverlayStateScriptAdapter {
            script_path: script_path.clone(),
            settings: Mutex::new(OverlaySnapshotSettings {
                locale: "en",
                primary_shortcut: "Ctrl+Space".to_string(),
                shortcut_mode: ShortcutMode::PushToTalk,
                refinement_quality: RefinementQuality::Balanced,
                silence_gate_level: 2,
                diagnostics_verbosity: DiagnosticsVerbosity::Standard,
                audio_feedback_enabled: true,
            }),
            recent_statuses: Mutex::new(Vec::new()),
            last_session_summary: Mutex::new(None),
            last_failure_summary: Mutex::new(None),
        };

        adapter.publish(OverlayStatus {
            session_id: 3,
            session_kind: SessionKind::Dictation,
            state: SessionState::Recording,
            detail: "Microphone capture started".to_string(),
        });

        let script_contents =
            fs::read_to_string(&script_path).expect("runtime-state.js should be written");
        assert!(script_contents.contains("__VOICEFLOW_OVERLAY_RUNTIME__"));
        assert!(script_contents.contains("Microphone capture started"));
        assert!(script_contents.contains("Ctrl+Space"));
        assert!(script_contents.contains(r#""locale": "en""#));

        let _ = fs::remove_file(&script_path);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn next_overlay_update_uses_reloaded_language() {
        let temp_dir = std::env::temp_dir().join("voiceflow-overlay-language-reload-test");
        let script_path = temp_dir.join("runtime-state.js");
        let adapter = OverlayStateScriptAdapter {
            script_path: script_path.clone(),
            settings: Mutex::new(OverlaySnapshotSettings::from_runtime(
                &RuntimeSettings::default(),
            )),
            recent_statuses: Mutex::new(Vec::new()),
            last_session_summary: Mutex::new(None),
            last_failure_summary: Mutex::new(None),
        };
        let mut updated = RuntimeSettings::default();
        updated.system_language = shared_protocol::SystemLanguage::Chinese;
        adapter.update_settings(&updated);
        adapter.publish(OverlayStatus {
            session_id: 4,
            session_kind: SessionKind::Dictation,
            state: SessionState::Recording,
            detail: "Microphone capture started".to_string(),
        });

        let script_contents =
            fs::read_to_string(&script_path).expect("runtime-state.js should be written");
        assert!(script_contents.contains(r#""locale": "zh""#));

        let _ = fs::remove_file(&script_path);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn writes_runtime_snapshot_script_with_latest_outcome() {
        let temp_dir = std::env::temp_dir().join("voiceflow-overlay-bridge-outcome-test");
        let script_path = temp_dir.join("runtime-state.js");
        let adapter = OverlayStateScriptAdapter {
            script_path: script_path.clone(),
            settings: Mutex::new(OverlaySnapshotSettings {
                locale: "zh",
                primary_shortcut: "Ctrl+Space".to_string(),
                shortcut_mode: ShortcutMode::PushToTalk,
                refinement_quality: RefinementQuality::Balanced,
                silence_gate_level: 2,
                diagnostics_verbosity: DiagnosticsVerbosity::Standard,
                audio_feedback_enabled: true,
            }),
            recent_statuses: Mutex::new(Vec::new()),
            last_session_summary: Mutex::new(None),
            last_failure_summary: Mutex::new(None),
        };

        adapter.publish_summary(&SessionSummary {
            session_id: 9,
            session_kind: SessionKind::Dictation,
            final_state: SessionState::Committed,
            completed_at_epoch_ms: None,
            start_feedback_latency_ms: Some(0),
            recording_start_latency_ms: Some(250),
            total_session_latency_ms: 3200,
            audio_duration_ms: 2200,
            audio_peak_level: 0.2,
            audio_rms_level: 0.03,
            asr_diagnostics: None,
            refine_diagnostics: None,
            recognized_text: "hello there".to_string(),
            committed_text: "Hello there.".to_string(),
            committed_text_count: None,
            route_decision: shared_protocol::RouteDecision {
                route_name: shared_protocol::RouteName::LocalAsrOnly,
                asr_provider: shared_protocol::AsrProvider::Local,
                refine_provider: None,
                refinement_applied: false,
                reason: "test".to_string(),
                refine_fast_path_used: false,
                refine_fast_path_reason: "not_evaluated".to_string(),
                cloud_refine_skipped: false,
                dictation_routing: None,
            },
            degraded_to_asr: false,
            fallback_reason: None,
            commit_status: shared_protocol::CommitStatus::Success,
            commit_transport: shared_protocol::CommitTransport::DirectUnicodeSendInput,
            commit_failure_reason: None,
            mode_reason: "dictation".to_string(),
            selected_text_execution: None,
            wake_phrase_execution: None,
            instructed_dictation_execution: None,
        });
        adapter.publish_failure(&FailedSessionSummary {
            session_id: 10,
            session_kind: SessionKind::Dictation,
            failure_phase: SessionFailurePhase::Recognizing,
            message: "no speech was recognized".to_string(),
            audio_duration_ms: Some(900),
            audio_peak_level: Some(0.001),
            audio_rms_level: Some(0.0),
        });

        let script_contents =
            fs::read_to_string(&script_path).expect("runtime-state.js should be written");
        assert!(script_contents.contains("\"last_session_summary\""));
        assert!(script_contents.contains("Hello there."));
        assert!(script_contents.contains("\"last_failure_summary\""));
        assert!(script_contents.contains("no speech was recognized"));

        let _ = fs::remove_file(&script_path);
        let _ = fs::remove_dir_all(temp_dir);
    }
}
