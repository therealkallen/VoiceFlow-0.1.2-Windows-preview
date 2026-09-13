use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use shared_protocol::{HistoryRetention, SessionKind, SessionState, SessionSummary};

use crate::diagnostics_profile::selected_text_action_label;
use crate::text_count::count_words_entered;

const HISTORY_LEDGER_FILE_NAME: &str = "history-ledger.json";
const HISTORY_SCHEMA_VERSION: u32 = 2;
const DAY_MS: u128 = 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HistoryLedger {
    pub schema_version: u32,
    pub updated_at_epoch_ms: u128,
    #[serde(default)]
    pub cleared_at_epoch_ms: Option<u128>,
    #[serde(default)]
    pub applied_session_keys: Vec<String>,
    #[serde(default)]
    pub entries: Vec<HistoryEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HistoryEntry {
    pub entry_id: String,
    pub source_run_id: String,
    pub session_id: u64,
    pub timestamp_epoch_ms: u128,
    pub mode: String,
    pub summary: String,
    pub text: Option<String>,
    pub redacted: bool,
    pub word_count: u64,
    #[serde(default)]
    pub text_count: Option<u64>,
    pub char_count: usize,
    pub refinement_profile_label: Option<String>,
    pub model_name: Option<String>,
    pub model_code: Option<String>,
    #[serde(default)]
    pub provider_preset: Option<String>,
    pub provider_attempted: Option<bool>,
    pub provider_succeeded: Option<bool>,
    pub deterministic_fallback_used: Option<bool>,
    pub fallback_reason: Option<String>,
    pub selected_text_action: Option<String>,
    pub selected_text_action_label: Option<String>,
    pub selected_text_char_count: Option<usize>,
    pub output_char_count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct HistoryDashboardSummary {
    pub updated_at_epoch_ms: u128,
    pub entries: Vec<HistoryEntry>,
}

pub(crate) fn history_ledger_path_for_settings_path(settings_path: &Path) -> PathBuf {
    settings_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(HISTORY_LEDGER_FILE_NAME)
}

pub(crate) fn append_live_host_history(
    settings_path: &Path,
    source_run_id: &str,
    sessions: &[SessionSummary],
    retention: &HistoryRetention,
) -> Result<Option<HistoryDashboardSummary>, String> {
    if sessions.is_empty() {
        return load_history_dashboard_summary_for_settings_path(settings_path, retention);
    }

    let path = history_ledger_path_for_settings_path(settings_path);
    let mut ledger = read_history_ledger_or_default(&path)?;
    ledger.schema_version = HISTORY_SCHEMA_VERSION;
    let mut applied_session_keys = ledger
        .applied_session_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let now = current_epoch_ms();
    let source_run_fallback_timestamp = ledger
        .entries
        .iter()
        .filter(|entry| entry.source_run_id == source_run_id)
        .map(|entry| entry.timestamp_epoch_ms)
        .max()
        .unwrap_or(now);
    let mut new_entries = Vec::new();

    for session in sessions.iter().rev() {
        let session_key = format!("{source_run_id}:session:{}", session.session_id);
        if applied_session_keys.contains(&session_key) {
            continue;
        }

        let timestamp_epoch_ms =
            match history_entry_timestamp(&ledger, session, source_run_fallback_timestamp) {
                Some(timestamp_epoch_ms) => timestamp_epoch_ms,
                None => continue,
            };

        if let Some(entry) = history_entry_from_session(source_run_id, session, timestamp_epoch_ms)
        {
            applied_session_keys.insert(session_key.clone());
            ledger.applied_session_keys.push(session_key);
            new_entries.push(entry);
        }
    }

    if new_entries.is_empty() {
        if enforce_history_retention_at(&mut ledger, retention, now) {
            ledger.updated_at_epoch_ms = now;
            write_history_ledger(&path, &ledger)?;
        }
        return Ok(Some(summarize_history_ledger(&ledger)));
    }

    new_entries.append(&mut ledger.entries);
    ledger.entries = new_entries;
    ledger.entries.sort_by(|left, right| {
        right
            .timestamp_epoch_ms
            .cmp(&left.timestamp_epoch_ms)
            .then_with(|| right.source_run_id.cmp(&left.source_run_id))
            .then_with(|| right.session_id.cmp(&left.session_id))
    });
    enforce_history_retention_at(&mut ledger, retention, now);
    ledger.updated_at_epoch_ms = now;
    write_history_ledger(&path, &ledger)?;

    Ok(Some(summarize_history_ledger(&ledger)))
}

pub(crate) fn load_history_dashboard_summary_for_settings_path(
    settings_path: &Path,
    retention: &HistoryRetention,
) -> Result<Option<HistoryDashboardSummary>, String> {
    let path = history_ledger_path_for_settings_path(settings_path);
    if !path.exists() {
        return Ok(None);
    }
    let mut ledger = read_history_ledger_or_default(&path)?;
    let now = current_epoch_ms();
    if enforce_history_retention_at(&mut ledger, retention, now) {
        ledger.updated_at_epoch_ms = now;
        write_history_ledger(&path, &ledger)?;
    }
    Ok(Some(summarize_history_ledger(&ledger)))
}

pub(crate) fn enforce_history_retention_for_settings_path(
    settings_path: &Path,
    retention: &HistoryRetention,
) -> Result<Option<HistoryDashboardSummary>, String> {
    load_history_dashboard_summary_for_settings_path(settings_path, retention)
}

pub(crate) fn clear_history_for_settings_path(
    settings_path: &Path,
) -> Result<HistoryDashboardSummary, String> {
    let path = history_ledger_path_for_settings_path(settings_path);
    let mut ledger = read_history_ledger_or_default(&path)?;
    ledger.schema_version = HISTORY_SCHEMA_VERSION;
    let now = current_epoch_ms();
    ledger.entries.clear();
    ledger.applied_session_keys.clear();
    ledger.cleared_at_epoch_ms = Some(now);
    ledger.updated_at_epoch_ms = now;
    write_history_ledger(&path, &ledger)?;
    Ok(summarize_history_ledger(&ledger))
}

fn history_entry_timestamp(
    ledger: &HistoryLedger,
    session: &SessionSummary,
    source_run_fallback_timestamp: u128,
) -> Option<u128> {
    let timestamp_epoch_ms = match (ledger.cleared_at_epoch_ms, session.completed_at_epoch_ms) {
        (Some(_), None) => return None,
        (_, Some(completed_at_epoch_ms)) => completed_at_epoch_ms,
        (None, None) => source_run_fallback_timestamp,
    };

    if ledger
        .cleared_at_epoch_ms
        .is_some_and(|cleared_at_epoch_ms| timestamp_epoch_ms <= cleared_at_epoch_ms)
    {
        return None;
    }

    Some(timestamp_epoch_ms)
}

fn history_entry_from_session(
    source_run_id: &str,
    session: &SessionSummary,
    timestamp_epoch_ms: u128,
) -> Option<HistoryEntry> {
    if !matches!(session.final_state, SessionState::Committed) {
        return None;
    }

    match session.session_kind {
        SessionKind::Dictation => Some(dictation_history_entry(
            source_run_id,
            session,
            timestamp_epoch_ms,
        )),
        SessionKind::SelectedTextEdit => {
            selected_text_history_entry(source_run_id, session, timestamp_epoch_ms)
        }
        SessionKind::WakePhraseIntent | SessionKind::InstructedDictation => Some(
            instructed_input_history_entry(source_run_id, session, timestamp_epoch_ms),
        ),
    }
}

fn dictation_history_entry(
    source_run_id: &str,
    session: &SessionSummary,
    timestamp_epoch_ms: u128,
) -> HistoryEntry {
    let text = session.committed_text.trim().to_string();
    let refine = session.refine_diagnostics.as_ref();
    HistoryEntry {
        entry_id: format!("{source_run_id}:session:{}", session.session_id),
        source_run_id: source_run_id.to_string(),
        session_id: session.session_id,
        timestamp_epoch_ms,
        mode: "Dictation".to_string(),
        summary: "Dictation inserted".to_string(),
        word_count: session
            .committed_text_count
            .unwrap_or_else(|| count_words_entered(&text)),
        text_count: Some(
            session
                .committed_text_count
                .unwrap_or_else(|| count_words_entered(&text)),
        ),
        char_count: text.chars().count(),
        text: Some(text),
        redacted: false,
        refinement_profile_label: refine.map(|diagnostics| diagnostics.profile_label.clone()),
        model_name: refine.map(|diagnostics| diagnostics.model_name.clone()),
        model_code: refine.map(|diagnostics| diagnostics.model_code.clone()),
        provider_preset: refine.and_then(|diagnostics| diagnostics.provider_preset.clone()),
        provider_attempted: Some(
            refine
                .map(|diagnostics| diagnostics.provider_attempted)
                .unwrap_or(false),
        ),
        provider_succeeded: refine.map(|diagnostics| diagnostics.provider_succeeded),
        deterministic_fallback_used: refine
            .map(|diagnostics| diagnostics.deterministic_fallback_used),
        fallback_reason: refine.and_then(|diagnostics| diagnostics.fallback_reason.clone()),
        selected_text_action: None,
        selected_text_action_label: None,
        selected_text_char_count: None,
        output_char_count: None,
    }
}

fn selected_text_history_entry(
    source_run_id: &str,
    session: &SessionSummary,
    timestamp_epoch_ms: u128,
) -> Option<HistoryEntry> {
    let execution = session.selected_text_execution.as_ref()?;
    let provider = execution.provider_diagnostics.as_ref();
    Some(HistoryEntry {
        entry_id: format!("{source_run_id}:session:{}", session.session_id),
        source_run_id: source_run_id.to_string(),
        session_id: session.session_id,
        timestamp_epoch_ms,
        mode: "SelectedTextEdit".to_string(),
        summary: "Selected text edited".to_string(),
        text: None,
        redacted: true,
        word_count: session.committed_text_count.unwrap_or(0),
        text_count: session.committed_text_count,
        char_count: 0,
        refinement_profile_label: provider.map(|diagnostics| diagnostics.profile_label.clone()),
        model_name: None,
        model_code: provider.map(|diagnostics| diagnostics.model_code.clone()),
        provider_preset: provider.and_then(|diagnostics| diagnostics.provider_preset.clone()),
        provider_attempted: Some(
            provider
                .map(|diagnostics| diagnostics.provider_attempted)
                .unwrap_or(false),
        ),
        provider_succeeded: provider.map(|diagnostics| diagnostics.provider_succeeded),
        deterministic_fallback_used: provider
            .map(|diagnostics| diagnostics.deterministic_fallback_used),
        fallback_reason: provider.and_then(|diagnostics| diagnostics.fallback_reason.clone()),
        selected_text_action: Some(format!("{:?}", execution.action)),
        selected_text_action_label: Some(selected_text_action_label(&execution.action).to_string()),
        selected_text_char_count: provider.map(|diagnostics| diagnostics.selected_text_char_count),
        output_char_count: provider.map(|diagnostics| diagnostics.output_char_count),
    })
}

fn instructed_input_history_entry(
    source_run_id: &str,
    session: &SessionSummary,
    timestamp_epoch_ms: u128,
) -> HistoryEntry {
    let text = session.committed_text.trim().to_string();
    let provider = session
        .instructed_dictation_execution
        .as_ref()
        .map(|execution| &execution.provider_diagnostics);
    HistoryEntry {
        entry_id: format!("{source_run_id}:session:{}", session.session_id),
        source_run_id: source_run_id.to_string(),
        session_id: session.session_id,
        timestamp_epoch_ms,
        mode: "InstructedInput".to_string(),
        summary: "Instructed input completed".to_string(),
        word_count: session
            .committed_text_count
            .unwrap_or_else(|| count_words_entered(&text)),
        text_count: Some(
            session
                .committed_text_count
                .unwrap_or_else(|| count_words_entered(&text)),
        ),
        char_count: text.chars().count(),
        text: Some(text),
        redacted: false,
        refinement_profile_label: provider.map(|diagnostics| diagnostics.profile_label.clone()),
        model_name: None,
        model_code: provider.map(|diagnostics| diagnostics.model_code.clone()),
        provider_preset: provider.and_then(|diagnostics| diagnostics.provider_preset.clone()),
        provider_attempted: provider
            .map(|diagnostics| diagnostics.provider_attempted)
            .or_else(|| session.wake_phrase_execution.as_ref().map(|_| false)),
        provider_succeeded: provider.map(|diagnostics| diagnostics.provider_succeeded),
        deterministic_fallback_used: provider
            .map(|diagnostics| diagnostics.deterministic_fallback_used),
        fallback_reason: provider.and_then(|diagnostics| diagnostics.fallback_reason.clone()),
        selected_text_action: None,
        selected_text_action_label: None,
        selected_text_char_count: None,
        output_char_count: provider.map(|diagnostics| diagnostics.output_char_count),
    }
}

fn summarize_history_ledger(ledger: &HistoryLedger) -> HistoryDashboardSummary {
    HistoryDashboardSummary {
        updated_at_epoch_ms: ledger.updated_at_epoch_ms,
        entries: ledger.entries.clone(),
    }
}

fn read_history_ledger_or_default(path: &Path) -> Result<HistoryLedger, String> {
    if !path.exists() {
        return Ok(default_history_ledger());
    }

    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read history ledger {}: {}",
            path.display(),
            error
        )
    })?;
    serde_json::from_str::<HistoryLedger>(&raw).map_err(|error| {
        format!(
            "failed to parse history ledger {}: {}",
            path.display(),
            error
        )
    })
}

fn write_history_ledger(path: &Path, ledger: &HistoryLedger) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create history ledger directory {}: {}",
                parent.display(),
                error
            )
        })?;
    }
    let encoded = serde_json::to_string_pretty(ledger)
        .map_err(|error| format!("failed to encode history ledger: {error}"))?;
    fs::write(path, encoded).map_err(|error| {
        format!(
            "failed to write history ledger {}: {}",
            path.display(),
            error
        )
    })
}

fn default_history_ledger() -> HistoryLedger {
    HistoryLedger {
        schema_version: HISTORY_SCHEMA_VERSION,
        updated_at_epoch_ms: 0,
        cleared_at_epoch_ms: None,
        applied_session_keys: Vec::new(),
        entries: Vec::new(),
    }
}

fn enforce_history_retention_at(
    ledger: &mut HistoryLedger,
    retention: &HistoryRetention,
    now_epoch_ms: u128,
) -> bool {
    let original_entries_len = ledger.entries.len();
    let original_keys_len = ledger.applied_session_keys.len();

    match retention {
        HistoryRetention::Latest100 => ledger.entries.truncate(100),
        HistoryRetention::Latest500 => ledger.entries.truncate(500),
        HistoryRetention::Latest1000 => ledger.entries.truncate(1_000),
        HistoryRetention::Last7Days => {
            let cutoff = now_epoch_ms.saturating_sub(7 * DAY_MS);
            ledger
                .entries
                .retain(|entry| entry.timestamp_epoch_ms >= cutoff);
        }
        HistoryRetention::Last30Days => {
            let cutoff = now_epoch_ms.saturating_sub(30 * DAY_MS);
            ledger
                .entries
                .retain(|entry| entry.timestamp_epoch_ms >= cutoff);
        }
        HistoryRetention::Unlimited => {}
    }

    let retained_entry_ids = ledger
        .entries
        .iter()
        .map(|entry| entry.entry_id.as_str())
        .collect::<BTreeSet<_>>();
    ledger
        .applied_session_keys
        .retain(|key| retained_entry_ids.contains(key.as_str()));

    ledger.entries.len() != original_entries_len
        || ledger.applied_session_keys.len() != original_keys_len
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_protocol::{
        AsrProvider, CommitStatus, CommitTransport, InstructedDictationExecutionSummary,
        InstructedDictationProviderDiagnostics, RefineDiagnostics, RefineProvider, RouteDecision,
        RouteName, SelectedTextExecutionAction, SelectedTextExecutionSummary,
        SelectedTextProviderDiagnostics, WakePhraseIntentAction, WakePhraseIntentExecutionSummary,
    };

    fn temp_settings_path(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "voiceflow-history-ledger-{label}-{}",
            current_epoch_ms()
        ));
        fs::create_dir_all(&dir).expect("temp history dir should be created");
        dir.join("settings.json")
    }

    fn dictation_summary(session_id: u64, text: &str) -> SessionSummary {
        SessionSummary {
            session_id,
            session_kind: SessionKind::Dictation,
            final_state: SessionState::Committed,
            completed_at_epoch_ms: None,
            start_feedback_latency_ms: None,
            recording_start_latency_ms: None,
            total_session_latency_ms: 1000,
            audio_duration_ms: 500,
            audio_peak_level: 0.0,
            audio_rms_level: 0.0,
            asr_diagnostics: None,
            refine_diagnostics: Some(RefineDiagnostics {
                profile_label: "Best Quality".to_string(),
                model_name: "Test Model".to_string(),
                model_code: "test-model".to_string(),
                provider_preset: None,
                provider_key_source: None,
                provider_configured: true,
                provider_attempted: true,
                provider_succeeded: true,
                deterministic_fallback_used: false,
                fallback_reason: None,
                refine_total_ms: None,
                provider_request_ms: None,
                prompt_load_ms: None,
                payload_build_ms: None,
                provider_output_validation_ms: None,
                prompt_char_count: None,
                transcript_char_count: None,
                guard_detected: false,
                provider_output_rejected: false,
                prompt_profile: None,
                prompt_source: None,
            }),
            recognized_text: text.to_string(),
            committed_text: text.to_string(),
            committed_text_count: Some(count_words_entered(text)),
            route_decision: RouteDecision {
                route_name: RouteName::LocalAsrWithRefine,
                asr_provider: AsrProvider::Local,
                refine_provider: Some(RefineProvider::Llm),
                refinement_applied: true,
                reason: "test".to_string(),
                refine_fast_path_used: false,
                refine_fast_path_reason: "not_evaluated".to_string(),
                cloud_refine_skipped: false,
                dictation_routing: None,
            },
            degraded_to_asr: false,
            fallback_reason: None,
            commit_status: CommitStatus::Success,
            commit_transport: CommitTransport::DirectUnicodeSendInput,
            commit_failure_reason: None,
            mode_reason: "test".to_string(),
            selected_text_execution: None,
            wake_phrase_execution: None,
            instructed_dictation_execution: None,
        }
    }

    fn selected_text_summary(session_id: u64) -> SessionSummary {
        let mut summary = dictation_summary(session_id, "[redacted selected-text provider edit]");
        summary.session_kind = SessionKind::SelectedTextEdit;
        summary.committed_text_count = Some(12);
        summary.commit_transport = CommitTransport::ClipboardSelectionReplace;
        summary.selected_text_execution = Some(SelectedTextExecutionSummary {
            action: SelectedTextExecutionAction::GeneralProviderEdit,
            strategy: "provider edited the selected text".to_string(),
            provider_diagnostics: Some(SelectedTextProviderDiagnostics {
                profile_label: "Best Quality".to_string(),
                model_code: "test-model".to_string(),
                provider_preset: None,
                provider_key_source: None,
                provider_attempted: true,
                provider_succeeded: true,
                deterministic_fallback_used: false,
                fallback_reason: None,
                selected_text_char_count: 126,
                output_char_count: 104,
            }),
        });
        summary
    }

    fn instructed_dictation_summary(session_id: u64) -> SessionSummary {
        let mut summary = dictation_summary(
            session_id,
            "Tomorrow we need to confirm the invoice recognition timeline.",
        );
        summary.session_kind = SessionKind::InstructedDictation;
        summary.recognized_text = "[redacted instructed dictation source]".to_string();
        summary.route_decision.route_name = RouteName::InstructedDictation;
        summary.instructed_dictation_execution = Some(InstructedDictationExecutionSummary {
            strategy: "provider transformed instructed dictation".to_string(),
            trigger_phrase_matched: true,
            parser_result: "chinese_colon".to_string(),
            provider_diagnostics: InstructedDictationProviderDiagnostics {
                profile_label: "Best Quality".to_string(),
                model_code: "test-model".to_string(),
                provider_preset: None,
                provider_key_source: None,
                provider_attempted: true,
                provider_succeeded: true,
                deterministic_fallback_used: false,
                fallback_reason: None,
                instruction_char_count: 12,
                content_char_count: 18,
                output_char_count: summary.committed_text.chars().count(),
            },
        });
        summary
    }

    fn policy_entry(id: u64, timestamp_epoch_ms: u128) -> HistoryEntry {
        HistoryEntry {
            entry_id: format!("report:session:{id}"),
            source_run_id: "report".to_string(),
            session_id: id,
            timestamp_epoch_ms,
            mode: "Dictation".to_string(),
            summary: "Dictation inserted".to_string(),
            text: Some(format!("entry {id}")),
            redacted: false,
            word_count: 2,
            text_count: Some(2),
            char_count: 7,
            refinement_profile_label: None,
            model_name: None,
            model_code: None,
            provider_preset: None,
            provider_attempted: None,
            provider_succeeded: None,
            deterministic_fallback_used: None,
            fallback_reason: None,
            selected_text_action: None,
            selected_text_action_label: None,
            selected_text_char_count: None,
            output_char_count: None,
        }
    }

    fn policy_ledger(count: u64, timestamp_epoch_ms: u128) -> HistoryLedger {
        let entries = (0..count)
            .map(|id| policy_entry(id, timestamp_epoch_ms))
            .collect::<Vec<_>>();
        HistoryLedger {
            schema_version: HISTORY_SCHEMA_VERSION,
            updated_at_epoch_ms: timestamp_epoch_ms,
            cleared_at_epoch_ms: None,
            applied_session_keys: entries.iter().map(|entry| entry.entry_id.clone()).collect(),
            entries,
        }
    }

    #[test]
    fn stores_dictation_text_in_separate_history_ledger() {
        let settings_path = temp_settings_path("dictation");
        let sessions = vec![dictation_summary(1, "Hello from history.")];

        let summary = append_live_host_history(
            &settings_path,
            "report-1",
            &sessions,
            &HistoryRetention::Latest100,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].mode, "Dictation");
        assert_eq!(
            summary.entries[0].text.as_deref(),
            Some("Hello from history.")
        );
        assert_eq!(
            summary.entries[0].char_count,
            "Hello from history.".chars().count()
        );
        assert_eq!(summary.entries[0].model_code.as_deref(), Some("test-model"));

        let raw = fs::read_to_string(history_ledger_path_for_settings_path(&settings_path))
            .expect("history ledger should read");
        assert!(raw.contains("Hello from history."));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn redacts_selected_text_history_entries() {
        let settings_path = temp_settings_path("selected-text");
        let sensitive_output = "This provider output should not be stored";
        let mut session = selected_text_summary(1);
        session.committed_text = sensitive_output.to_string();

        let summary = append_live_host_history(
            &settings_path,
            "report-2",
            &[session],
            &HistoryRetention::Latest100,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        let entry = &summary.entries[0];
        assert_eq!(entry.mode, "SelectedTextEdit");
        assert!(entry.redacted);
        assert_eq!(entry.text, None);
        assert_eq!(
            entry.selected_text_action.as_deref(),
            Some("GeneralProviderEdit")
        );
        assert_eq!(
            entry.selected_text_action_label.as_deref(),
            Some("General edit")
        );
        assert_eq!(entry.selected_text_char_count, Some(126));
        assert_eq!(entry.output_char_count, Some(104));
        assert_eq!(entry.text_count, Some(12));

        let raw = fs::read_to_string(history_ledger_path_for_settings_path(&settings_path))
            .expect("history ledger should read");
        assert!(!raw.contains(sensitive_output));
        assert!(!raw.contains("Selected text:"));
        assert!(!raw.contains("Instruction:"));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn instructed_dictation_history_stores_final_output_without_raw_command() {
        let settings_path = temp_settings_path("instructed-dictation");
        let session = instructed_dictation_summary(1);

        let summary = append_live_host_history(
            &settings_path,
            "report-4",
            &[session],
            &HistoryRetention::Latest100,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        let entry = &summary.entries[0];
        assert_eq!(entry.mode, "InstructedInput");
        assert!(!entry.redacted);
        assert_eq!(
            entry.text.as_deref(),
            Some("Tomorrow we need to confirm the invoice recognition timeline.")
        );
        assert_eq!(entry.model_code.as_deref(), Some("test-model"));
        assert_eq!(entry.provider_attempted, Some(true));
        assert_eq!(entry.provider_succeeded, Some(true));
        assert_eq!(
            entry.text_count,
            Some(count_words_entered(
                "Tomorrow we need to confirm the invoice recognition timeline."
            ))
        );

        let raw = fs::read_to_string(history_ledger_path_for_settings_path(&settings_path))
            .expect("history ledger should read");
        assert!(raw.contains("Tomorrow we need to confirm"));
        assert!(!raw.contains("Hey VoiceFlow"));
        assert!(!raw.contains("把下面内容翻译成英文"));
        assert!(!raw.contains("明天我们要确认发票识别项目的时间线"));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn intention_routes_share_one_product_history_category_and_session_key() {
        let settings_path = temp_settings_path("intention-category");
        let mut wake_phrase = dictation_summary(1, "Draft ready");
        wake_phrase.session_kind = SessionKind::WakePhraseIntent;
        wake_phrase.wake_phrase_execution = Some(WakePhraseIntentExecutionSummary {
            action: WakePhraseIntentAction::GeneralDraft,
            strategy: "local intent executor".to_string(),
        });
        let instructed = instructed_dictation_summary(1);

        let summary = append_live_host_history(
            &settings_path,
            "one-intention-session",
            &[wake_phrase, instructed],
            &HistoryRetention::Latest100,
        )
        .expect("intention history append should work")
        .expect("intention history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].mode, "InstructedInput");
        assert_eq!(
            summary.entries[0].entry_id,
            "one-intention-session:session:1"
        );

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn history_preserves_actual_provider_and_model_metadata() {
        let settings_path = temp_settings_path("provider-models");
        let mut bailian = dictation_summary(1, "Bailian output");
        let bailian_diagnostics = bailian
            .refine_diagnostics
            .as_mut()
            .expect("dictation diagnostics should exist");
        bailian_diagnostics.model_code = "qwen3.7-plus".to_string();
        bailian_diagnostics.provider_preset = Some("bailian".to_string());

        let mut volcengine = selected_text_summary(2);
        let volcengine_diagnostics = volcengine
            .selected_text_execution
            .as_mut()
            .and_then(|execution| execution.provider_diagnostics.as_mut())
            .expect("selected text diagnostics should exist");
        volcengine_diagnostics.model_code = "doubao-seed".to_string();
        volcengine_diagnostics.provider_preset = Some("volcengine_ark".to_string());

        let mut custom = instructed_dictation_summary(3);
        let custom_diagnostics = &mut custom
            .instructed_dictation_execution
            .as_mut()
            .expect("instructed diagnostics should exist")
            .provider_diagnostics;
        custom_diagnostics.model_code = "custom-model".to_string();
        custom_diagnostics.provider_preset = Some("custom_openai_compatible".to_string());

        let summary = append_live_host_history(
            &settings_path,
            "provider-run",
            &[bailian, volcengine, custom],
            &HistoryRetention::Unlimited,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        let by_session = summary
            .entries
            .iter()
            .map(|entry| (entry.session_id, entry))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(by_session[&1].model_code.as_deref(), Some("qwen3.7-plus"));
        assert_eq!(by_session[&1].provider_preset.as_deref(), Some("bailian"));
        assert_eq!(by_session[&2].model_code.as_deref(), Some("doubao-seed"));
        assert_eq!(
            by_session[&2].provider_preset.as_deref(),
            Some("volcengine_ark")
        );
        assert_eq!(by_session[&3].model_code.as_deref(), Some("custom-model"));
        assert_eq!(
            by_session[&3].provider_preset.as_deref(),
            Some("custom_openai_compatible")
        );

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn history_distinguishes_skipped_and_failed_refinement() {
        let settings_path = temp_settings_path("refinement-outcomes");
        let mut skipped = dictation_summary(1, "Local output");
        skipped.refine_diagnostics = None;

        let mut failed = dictation_summary(2, "Original ASR output");
        let failed_diagnostics = failed
            .refine_diagnostics
            .as_mut()
            .expect("dictation diagnostics should exist");
        failed_diagnostics.model_code = "qwen3.7-plus".to_string();
        failed_diagnostics.provider_preset = Some("bailian".to_string());
        failed_diagnostics.provider_attempted = true;
        failed_diagnostics.provider_succeeded = false;
        failed_diagnostics.deterministic_fallback_used = true;

        let summary = append_live_host_history(
            &settings_path,
            "outcome-run",
            &[skipped, failed],
            &HistoryRetention::Unlimited,
        )
        .expect("history append should work")
        .expect("history summary should exist");
        let by_session = summary
            .entries
            .iter()
            .map(|entry| (entry.session_id, entry))
            .collect::<std::collections::BTreeMap<_, _>>();

        assert_eq!(by_session[&1].provider_attempted, Some(false));
        assert_eq!(by_session[&1].provider_succeeded, None);
        assert_eq!(by_session[&1].model_code, None);
        assert_eq!(by_session[&2].provider_attempted, Some(true));
        assert_eq!(by_session[&2].provider_succeeded, Some(false));
        assert_eq!(by_session[&2].model_code.as_deref(), Some("qwen3.7-plus"));
        assert_eq!(by_session[&2].provider_preset.as_deref(), Some("bailian"));

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn legacy_history_entry_without_provider_metadata_still_deserializes() {
        let mut value =
            serde_json::to_value(policy_entry(1, 100)).expect("history entry should serialize");
        value
            .as_object_mut()
            .expect("history entry should be an object")
            .remove("provider_preset");

        let decoded = serde_json::from_value::<HistoryEntry>(value)
            .expect("legacy history entry should deserialize");
        assert_eq!(decoded.provider_preset, None);
        assert_eq!(decoded.provider_attempted, None);
    }

    #[test]
    fn caps_history_entries_at_latest_100() {
        let settings_path = temp_settings_path("cap");
        let sessions = (1..=105)
            .map(|id| dictation_summary(id, &format!("entry {id}")))
            .collect::<Vec<_>>();

        let summary = append_live_host_history(
            &settings_path,
            "report-3",
            &sessions,
            &HistoryRetention::Latest100,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 100);
        assert_eq!(summary.entries[0].session_id, 105);
        assert_eq!(summary.entries[99].session_id, 6);
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn replaying_a_source_run_does_not_displace_newer_count_retained_entries() {
        let settings_path = temp_settings_path("replay");
        let sessions = (1..=105)
            .map(|id| dictation_summary(id, &format!("entry {id}")))
            .collect::<Vec<_>>();

        append_live_host_history(
            &settings_path,
            "report-replay",
            &sessions,
            &HistoryRetention::Latest100,
        )
        .expect("first history append should work");
        let summary = append_live_host_history(
            &settings_path,
            "report-replay",
            &sessions,
            &HistoryRetention::Latest100,
        )
        .expect("replayed history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 100);
        assert_eq!(summary.entries[0].session_id, 105);
        assert_eq!(summary.entries[99].session_id, 6);
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn repeated_live_history_appends_with_same_run_id_do_not_duplicate_entries() {
        let settings_path = temp_settings_path("live-dedup");
        let mut first = dictation_summary(1, "first live entry");
        first.completed_at_epoch_ms = Some(10);
        let mut second = dictation_summary(2, "second live entry");
        second.completed_at_epoch_ms = Some(20);

        append_live_host_history(
            &settings_path,
            "live-host-run",
            &[first.clone()],
            &HistoryRetention::Unlimited,
        )
        .expect("immediate history append should work");
        let summary = append_live_host_history(
            &settings_path,
            "live-host-run",
            &[first.clone(), second.clone()],
            &HistoryRetention::Unlimited,
        )
        .expect("second immediate history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 2);
        assert_eq!(summary.entries[0].session_id, 2);
        assert_eq!(summary.entries[1].session_id, 1);

        let summary = append_live_host_history(
            &settings_path,
            "live-host-run",
            &[first, second],
            &HistoryRetention::Unlimited,
        )
        .expect("final shutdown history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 2);
        let persisted =
            read_history_ledger_or_default(&history_ledger_path_for_settings_path(&settings_path))
                .expect("history should reload");
        assert_eq!(persisted.applied_session_keys.len(), 2);
        assert_eq!(
            persisted.applied_session_keys,
            vec![
                "live-host-run:session:1".to_string(),
                "live-host-run:session:2".to_string()
            ]
        );
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn repeated_live_history_appends_respect_clear_watermark() {
        let settings_path = temp_settings_path("live-clear");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let mut ledger = policy_ledger(1, 10);
        ledger.entries.clear();
        ledger.applied_session_keys.clear();
        ledger.cleared_at_epoch_ms = Some(100);
        ledger.updated_at_epoch_ms = 100;
        write_history_ledger(&path, &ledger).expect("history ledger should write");

        let mut before = dictation_summary(1, "before clear");
        before.completed_at_epoch_ms = Some(99);
        let mut missing = dictation_summary(2, "missing completion");
        missing.completed_at_epoch_ms = None;
        let mut after = dictation_summary(3, "after clear");
        after.completed_at_epoch_ms = Some(101);

        append_live_host_history(
            &settings_path,
            "live-host-run",
            &[before.clone()],
            &HistoryRetention::Unlimited,
        )
        .expect("pre-clear immediate append should be skipped safely");
        append_live_host_history(
            &settings_path,
            "live-host-run",
            &[before.clone(), missing.clone()],
            &HistoryRetention::Unlimited,
        )
        .expect("missing timestamp immediate append should be skipped safely");
        let summary = append_live_host_history(
            &settings_path,
            "live-host-run",
            &[before, missing, after],
            &HistoryRetention::Unlimited,
        )
        .expect("post-clear immediate append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].session_id, 3);
        assert_eq!(summary.entries[0].text.as_deref(), Some("after clear"));
        let persisted = read_history_ledger_or_default(&path).expect("history should reload");
        assert_eq!(
            persisted.applied_session_keys,
            vec!["live-host-run:session:3".to_string()]
        );
        assert_eq!(persisted.cleared_at_epoch_ms, Some(100));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn retention_applies_after_repeated_live_history_appends() {
        let settings_path = temp_settings_path("live-retention");
        let mut sessions = Vec::new();

        for id in 1..=105 {
            let mut summary = dictation_summary(id, &format!("entry {id}"));
            summary.completed_at_epoch_ms = Some(u128::from(id));
            sessions.push(summary);
            append_live_host_history(
                &settings_path,
                "live-host-run",
                &sessions,
                &HistoryRetention::Latest100,
            )
            .expect("repeated immediate history append should work");
        }

        let summary = append_live_host_history(
            &settings_path,
            "live-host-run",
            &sessions,
            &HistoryRetention::Latest100,
        )
        .expect("final shutdown history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 100);
        assert_eq!(summary.entries[0].session_id, 105);
        assert_eq!(summary.entries[99].session_id, 6);
        let persisted =
            read_history_ledger_or_default(&history_ledger_path_for_settings_path(&settings_path))
                .expect("history should reload");
        assert_eq!(persisted.applied_session_keys.len(), 100);
        assert!(
            !persisted
                .applied_session_keys
                .contains(&"live-host-run:session:1".to_string())
        );
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn count_policies_keep_the_expected_newest_entries() {
        for (retention, expected) in [
            (HistoryRetention::Latest100, 100),
            (HistoryRetention::Latest500, 500),
            (HistoryRetention::Latest1000, 1_000),
        ] {
            let mut ledger = policy_ledger(1_050, 1_000);

            assert!(enforce_history_retention_at(&mut ledger, &retention, 1_000));
            assert_eq!(ledger.entries.len(), expected);
            assert_eq!(ledger.entries[0].session_id, 0);
            assert_eq!(ledger.entries[expected - 1].session_id, expected as u64 - 1);
            assert_eq!(ledger.applied_session_keys.len(), expected);
        }
    }

    #[test]
    fn time_policies_remove_expired_entries() {
        let now = 40 * DAY_MS;
        for (retention, recent_age_days) in [
            (HistoryRetention::Last7Days, 6),
            (HistoryRetention::Last30Days, 29),
        ] {
            let mut ledger = HistoryLedger {
                schema_version: HISTORY_SCHEMA_VERSION,
                updated_at_epoch_ms: now,
                cleared_at_epoch_ms: None,
                applied_session_keys: vec![
                    "report:session:1".to_string(),
                    "report:session:2".to_string(),
                ],
                entries: vec![
                    policy_entry(1, now - recent_age_days * DAY_MS),
                    policy_entry(2, 0),
                ],
            };

            assert!(enforce_history_retention_at(&mut ledger, &retention, now));
            assert_eq!(ledger.entries.len(), 1);
            assert_eq!(ledger.entries[0].session_id, 1);
            assert_eq!(ledger.applied_session_keys, vec!["report:session:1"]);
        }
    }

    #[test]
    fn unlimited_keeps_visible_entries_but_drops_irrelevant_dedup_keys() {
        let mut ledger = policy_ledger(1_200, 1_000);
        ledger
            .applied_session_keys
            .push("report:session:no-visible-entry".to_string());

        assert!(enforce_history_retention_at(
            &mut ledger,
            &HistoryRetention::Unlimited,
            1_000,
        ));
        assert_eq!(ledger.entries.len(), 1_200);
        assert_eq!(ledger.applied_session_keys.len(), 1_200);
    }

    #[test]
    fn append_uses_each_sessions_completion_timestamp() {
        let settings_path = temp_settings_path("completion-time");
        let mut older = dictation_summary(1, "older");
        older.completed_at_epoch_ms = Some(10);
        let mut newer = dictation_summary(2, "newer");
        newer.completed_at_epoch_ms = Some(20);

        let summary = append_live_host_history(
            &settings_path,
            "report-time",
            &[older, newer],
            &HistoryRetention::Unlimited,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries[0].timestamp_epoch_ms, 20);
        assert_eq!(summary.entries[1].timestamp_epoch_ms, 10);
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn loading_history_applies_time_retention_and_persists_pruning() {
        let settings_path = temp_settings_path("load-retention");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let now = current_epoch_ms();
        let ledger = HistoryLedger {
            schema_version: HISTORY_SCHEMA_VERSION,
            updated_at_epoch_ms: now,
            cleared_at_epoch_ms: None,
            applied_session_keys: vec![
                "report:session:1".to_string(),
                "report:session:2".to_string(),
            ],
            entries: vec![
                policy_entry(1, now),
                policy_entry(2, now.saturating_sub(31 * DAY_MS)),
            ],
        };
        write_history_ledger(&path, &ledger).expect("history ledger should write");

        let summary = load_history_dashboard_summary_for_settings_path(
            &settings_path,
            &HistoryRetention::Last30Days,
        )
        .expect("history should load")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        let persisted = read_history_ledger_or_default(&path).expect("history should reload");
        assert_eq!(persisted.entries.len(), 1);
        assert_eq!(persisted.applied_session_keys.len(), 1);
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn clearing_history_empties_entries_keys_and_records_watermark() {
        let settings_path = temp_settings_path("clear");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let ledger = policy_ledger(3, 10);
        write_history_ledger(&path, &ledger).expect("history ledger should write");

        let summary =
            clear_history_for_settings_path(&settings_path).expect("history clear should work");

        assert!(summary.entries.is_empty());
        let persisted = read_history_ledger_or_default(&path).expect("history should reload");
        assert!(persisted.entries.is_empty());
        assert!(persisted.applied_session_keys.is_empty());
        assert!(persisted.cleared_at_epoch_ms.is_some());
        assert_eq!(
            persisted.updated_at_epoch_ms,
            persisted.cleared_at_epoch_ms.unwrap()
        );
        assert!(persisted.updated_at_epoch_ms >= 10);
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn append_after_clear_skips_sessions_completed_before_or_at_watermark() {
        let settings_path = temp_settings_path("clear-skip-old");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let mut ledger = policy_ledger(1, 10);
        ledger.entries.clear();
        ledger.applied_session_keys.clear();
        ledger.cleared_at_epoch_ms = Some(100);
        ledger.updated_at_epoch_ms = 100;
        write_history_ledger(&path, &ledger).expect("history ledger should write");

        let mut before = dictation_summary(1, "before clear");
        before.completed_at_epoch_ms = Some(99);
        let mut at_watermark = dictation_summary(2, "at clear");
        at_watermark.completed_at_epoch_ms = Some(100);

        let summary = append_live_host_history(
            &settings_path,
            "report-clear",
            &[before, at_watermark],
            &HistoryRetention::Unlimited,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert!(summary.entries.is_empty());
        let persisted = read_history_ledger_or_default(&path).expect("history should reload");
        assert!(persisted.entries.is_empty());
        assert!(persisted.applied_session_keys.is_empty());
        assert_eq!(persisted.cleared_at_epoch_ms, Some(100));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn append_after_clear_accepts_sessions_completed_after_watermark() {
        let settings_path = temp_settings_path("clear-accept-new");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let mut ledger = policy_ledger(1, 10);
        ledger.entries.clear();
        ledger.applied_session_keys.clear();
        ledger.cleared_at_epoch_ms = Some(100);
        ledger.updated_at_epoch_ms = 100;
        write_history_ledger(&path, &ledger).expect("history ledger should write");

        let mut after = dictation_summary(1, "after clear");
        after.completed_at_epoch_ms = Some(101);

        let summary = append_live_host_history(
            &settings_path,
            "report-clear",
            &[after],
            &HistoryRetention::Unlimited,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].text.as_deref(), Some("after clear"));
        assert_eq!(summary.entries[0].timestamp_epoch_ms, 101);
        let persisted = read_history_ledger_or_default(&path).expect("history should reload");
        assert_eq!(
            persisted.applied_session_keys,
            vec!["report-clear:session:1"]
        );
        assert_eq!(persisted.cleared_at_epoch_ms, Some(100));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn append_after_clear_skips_sessions_with_missing_completion_time() {
        let settings_path = temp_settings_path("clear-skip-missing");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let mut ledger = policy_ledger(1, 10);
        ledger.entries.clear();
        ledger.applied_session_keys.clear();
        ledger.cleared_at_epoch_ms = Some(100);
        ledger.updated_at_epoch_ms = 100;
        write_history_ledger(&path, &ledger).expect("history ledger should write");

        let missing_time = dictation_summary(1, "unknown completion");

        let summary = append_live_host_history(
            &settings_path,
            "report-clear",
            &[missing_time],
            &HistoryRetention::Unlimited,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert!(summary.entries.is_empty());
        let persisted = read_history_ledger_or_default(&path).expect("history should reload");
        assert!(persisted.entries.is_empty());
        assert!(persisted.applied_session_keys.is_empty());
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn retention_still_applies_after_post_clear_append() {
        let settings_path = temp_settings_path("clear-retention");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let mut ledger = policy_ledger(1, 10);
        ledger.entries.clear();
        ledger.applied_session_keys.clear();
        ledger.cleared_at_epoch_ms = Some(100);
        ledger.updated_at_epoch_ms = 100;
        write_history_ledger(&path, &ledger).expect("history ledger should write");
        let sessions = (1..=105)
            .map(|id| {
                let mut summary = dictation_summary(id, &format!("entry {id}"));
                summary.completed_at_epoch_ms = Some(100 + u128::from(id));
                summary
            })
            .collect::<Vec<_>>();

        let summary = append_live_host_history(
            &settings_path,
            "report-clear",
            &sessions,
            &HistoryRetention::Latest100,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 100);
        assert_eq!(summary.entries[0].session_id, 105);
        assert_eq!(summary.entries[99].session_id, 6);
        let persisted = read_history_ledger_or_default(&path).expect("history should reload");
        assert_eq!(persisted.applied_session_keys.len(), 100);
        assert_eq!(persisted.cleared_at_epoch_ms, Some(100));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn selected_text_entries_remain_redacted_after_clear_and_later_append() {
        let settings_path = temp_settings_path("clear-selected-redacted");
        let path = history_ledger_path_for_settings_path(&settings_path);
        let mut ledger = policy_ledger(1, 10);
        ledger.entries.clear();
        ledger.applied_session_keys.clear();
        ledger.cleared_at_epoch_ms = Some(100);
        ledger.updated_at_epoch_ms = 100;
        write_history_ledger(&path, &ledger).expect("history ledger should write");
        let sensitive_output = "Provider output after clear should not be stored";
        let mut session = selected_text_summary(1);
        session.completed_at_epoch_ms = Some(101);
        session.committed_text = sensitive_output.to_string();

        let summary = append_live_host_history(
            &settings_path,
            "report-clear",
            &[session],
            &HistoryRetention::Unlimited,
        )
        .expect("history append should work")
        .expect("history summary should exist");

        assert_eq!(summary.entries.len(), 1);
        assert!(summary.entries[0].redacted);
        assert_eq!(summary.entries[0].text, None);
        let raw = fs::read_to_string(path).expect("history ledger should read");
        assert!(!raw.contains(sensitive_output));
        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }
}
