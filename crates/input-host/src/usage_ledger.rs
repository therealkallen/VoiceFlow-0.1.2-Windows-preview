use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use shared_protocol::{CommitTransport, RouteName, SessionKind, SessionState, SessionSummary};

use crate::text_count::count_words_entered;
use crate::windows_foreground::ForegroundAppMetadata;

const USAGE_LEDGER_FILE_NAME: &str = "usage-ledger.json";
const USAGE_SCHEMA_VERSION: u32 = 2;
const TYPING_BASELINE_WPM: u64 = 40;
const DAILY_BUCKET_LIMIT: usize = 120;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UsageFailureInput {
    pub attempted_session_index: u64,
    pub session_id: u64,
    pub session_kind: SessionKind,
    pub foreground_app: Option<ForegroundAppMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageAppSummary {
    pub app_fingerprint: String,
    pub process_name: Option<String>,
    pub window_class: Option<String>,
    pub capture_state: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageRouteCounts {
    pub local_asr_only_sessions: u64,
    pub local_asr_with_refine_sessions: u64,
    pub cloud_asr_with_refine_sessions: u64,
    #[serde(default)]
    pub instructed_dictation_sessions: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageCounters {
    pub total_sessions: u64,
    pub successful_sessions: u64,
    pub failed_sessions: u64,
    pub dictation_sessions: u64,
    pub selected_text_edit_sessions: u64,
    pub wake_phrase_intent_sessions: u64,
    #[serde(default)]
    pub instructed_dictation_sessions: u64,
    pub words_committed: u64,
    pub selected_text_replacements: u64,
    pub total_end_to_end_latency_ms: u128,
    pub latency_sample_count: u64,
    pub estimated_typing_time_ms: u128,
    pub estimated_time_saved_ms: u128,
    pub route_counts: UsageRouteCounts,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageDailyBucket {
    pub date: String,
    pub counters: UsageCounters,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageSessionRollup {
    pub source_run_id: String,
    pub session_id: u64,
    pub session_kind: SessionKind,
    pub final_state: SessionState,
    pub committed_word_count: u64,
    pub route_name: Option<RouteName>,
    pub commit_transport: Option<CommitTransport>,
    pub total_session_latency_ms: Option<u64>,
    pub app: Option<UsageAppSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageAnalyticsSession {
    pub session_key: String,
    pub completed_at_epoch_ms: u128,
    pub session_kind: SessionKind,
    pub inserted_text_units: u64,
    pub audio_duration_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageLedger {
    pub schema_version: u32,
    pub updated_at_epoch_ms: u128,
    pub typing_baseline_wpm: u64,
    #[serde(default)]
    pub applied_source_run_ids: Vec<String>,
    #[serde(default)]
    pub applied_session_keys: Vec<String>,
    pub totals: UsageCounters,
    pub daily: Vec<UsageDailyBucket>,
    #[serde(default)]
    pub analytics_sessions: Vec<UsageAnalyticsSession>,
    pub latest_app: Option<UsageAppSummary>,
    pub latest_session: Option<UsageSessionRollup>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct UsageDashboardSummary {
    pub updated_at_epoch_ms: u128,
    pub typing_baseline_wpm: u64,
    pub total_sessions: u64,
    pub successful_sessions: u64,
    pub failed_sessions: u64,
    pub success_rate_percent: Option<u64>,
    pub dictation_sessions: u64,
    pub selected_text_edit_sessions: u64,
    pub wake_phrase_intent_sessions: u64,
    pub instructed_dictation_sessions: u64,
    pub words_committed: u64,
    pub selected_text_replacements: u64,
    pub estimated_typing_time_seconds: u64,
    pub estimated_time_saved_seconds: u64,
    pub analytics_started_at_epoch_ms: Option<u128>,
    pub analytics_completed_sessions: u64,
    pub inserted_text_units: u64,
    pub recorded_audio_duration_ms: u64,
    pub average_dictation_speed_units_per_minute: Option<u64>,
    pub analytics_sessions: Vec<UsageAnalyticsSession>,
    pub productivity_estimate_label: String,
    pub productivity_estimate_note: String,
    pub average_end_to_end_latency_ms: Option<u64>,
    pub route_counts: UsageRouteCounts,
    pub latest_app: Option<UsageAppSummary>,
    pub latest_session: Option<UsageSessionRollup>,
    pub daily: Vec<UsageDailyBucket>,
}

pub(crate) fn usage_ledger_path_for_settings_path(settings_path: &Path) -> PathBuf {
    settings_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(USAGE_LEDGER_FILE_NAME)
}

pub(crate) fn append_live_host_usage(
    settings_path: &Path,
    source_run_id: &str,
    sessions: &[SessionSummary],
    failures: &[UsageFailureInput],
) -> Result<Option<UsageDashboardSummary>, String> {
    if sessions.is_empty() && failures.is_empty() {
        return load_usage_dashboard_summary_for_settings_path(settings_path);
    }

    let path = usage_ledger_path_for_settings_path(settings_path);
    let mut ledger = read_usage_ledger_or_default(&path)?;
    let now = current_epoch_ms();
    let date = utc_date_for_epoch_ms(now);
    let session_ids = sessions
        .iter()
        .map(|session| session.session_id)
        .collect::<BTreeSet<_>>();
    let mut applied_session_keys = ledger
        .applied_session_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    for session in sessions {
        let session_key = format!("{source_run_id}:session:{}", session.session_id);
        if applied_session_keys.insert(session_key.clone()) {
            apply_session_to_ledger(&mut ledger, &date, source_run_id, session);
            ledger.applied_session_keys.push(session_key.clone());
        }
        apply_session_to_analytics(&mut ledger, session_key, session);
    }

    for failure in failures {
        if session_ids.contains(&failure.session_id) {
            if let Some(app) = failure
                .foreground_app
                .as_ref()
                .and_then(UsageAppSummary::from_foreground_app)
            {
                ledger.latest_app = Some(app);
            }
            continue;
        }
        let session_key = format!(
            "{source_run_id}:failure:{}:{}",
            failure.attempted_session_index, failure.session_id
        );
        if !applied_session_keys.insert(session_key.clone()) {
            continue;
        }
        apply_failure_to_ledger(&mut ledger, &date, source_run_id, failure);
        ledger.applied_session_keys.push(session_key);
    }

    if !ledger
        .applied_source_run_ids
        .iter()
        .any(|existing| existing == source_run_id)
    {
        ledger
            .applied_source_run_ids
            .push(source_run_id.to_string());
    }
    ledger.schema_version = USAGE_SCHEMA_VERSION;
    ledger.updated_at_epoch_ms = now;
    truncate_daily_buckets(&mut ledger.daily);
    write_usage_ledger(&path, &ledger)?;

    Ok(Some(summarize_usage_ledger(&ledger)))
}

fn apply_session_to_analytics(
    ledger: &mut UsageLedger,
    session_key: String,
    session: &SessionSummary,
) {
    if !matches!(session.final_state, SessionState::Committed)
        || ledger
            .analytics_sessions
            .iter()
            .any(|existing| existing.session_key == session_key)
    {
        return;
    }

    let Some(completed_at_epoch_ms) = session.completed_at_epoch_ms else {
        return;
    };
    let committed_text_count = match session.committed_text_count {
        Some(count) => count,
        None if matches!(session.session_kind, SessionKind::SelectedTextEdit) => return,
        None => count_words_entered(&session.committed_text),
    };
    ledger.analytics_sessions.push(UsageAnalyticsSession {
        session_key,
        completed_at_epoch_ms,
        session_kind: session.session_kind.clone(),
        inserted_text_units: committed_text_count,
        audio_duration_ms: session.audio_duration_ms,
    });
}

pub(crate) fn load_usage_dashboard_summary_for_settings_path(
    settings_path: &Path,
) -> Result<Option<UsageDashboardSummary>, String> {
    let path = usage_ledger_path_for_settings_path(settings_path);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(summarize_usage_ledger(&read_usage_ledger_or_default(
        &path,
    )?)))
}

fn apply_session_to_ledger(
    ledger: &mut UsageLedger,
    date: &str,
    source_run_id: &str,
    session: &SessionSummary,
) {
    let mut counters = UsageCounters::default();
    counters.total_sessions = 1;
    match session.final_state {
        SessionState::Committed => counters.successful_sessions = 1,
        SessionState::Failed => counters.failed_sessions = 1,
        _ => {}
    }
    apply_kind_count(&mut counters, &session.session_kind);

    if matches!(session.final_state, SessionState::Committed) {
        counters.words_committed = session
            .committed_text_count
            .unwrap_or_else(|| count_words_entered(&session.committed_text));
        if matches!(
            session.commit_transport,
            CommitTransport::ClipboardSelectionReplace
        ) {
            counters.selected_text_replacements = 1;
        }
        let estimated_typing_ms =
            estimated_typing_time_ms(counters.words_committed, TYPING_BASELINE_WPM);
        counters.estimated_typing_time_ms = estimated_typing_ms;
        counters.estimated_time_saved_ms =
            estimated_typing_ms.saturating_sub(session.total_session_latency_ms as u128);
    }

    counters.total_end_to_end_latency_ms = session.total_session_latency_ms as u128;
    counters.latency_sample_count = 1;
    apply_route_count(&mut counters, &session.route_decision.route_name);

    apply_counters(&mut ledger.totals, &counters);
    apply_counters(
        &mut daily_bucket_mut(&mut ledger.daily, date).counters,
        &counters,
    );
    ledger.latest_session = Some(UsageSessionRollup {
        source_run_id: source_run_id.to_string(),
        session_id: session.session_id,
        session_kind: session.session_kind.clone(),
        final_state: session.final_state.clone(),
        committed_word_count: counters.words_committed,
        route_name: Some(session.route_decision.route_name.clone()),
        commit_transport: Some(session.commit_transport.clone()),
        total_session_latency_ms: Some(session.total_session_latency_ms),
        app: None,
    });
}

fn apply_failure_to_ledger(
    ledger: &mut UsageLedger,
    date: &str,
    source_run_id: &str,
    failure: &UsageFailureInput,
) {
    let mut counters = UsageCounters {
        total_sessions: 1,
        failed_sessions: 1,
        ..UsageCounters::default()
    };
    apply_kind_count(&mut counters, &failure.session_kind);

    let app = failure
        .foreground_app
        .as_ref()
        .and_then(UsageAppSummary::from_foreground_app);
    if app.is_some() {
        ledger.latest_app = app.clone();
    }

    apply_counters(&mut ledger.totals, &counters);
    apply_counters(
        &mut daily_bucket_mut(&mut ledger.daily, date).counters,
        &counters,
    );
    ledger.latest_session = Some(UsageSessionRollup {
        source_run_id: source_run_id.to_string(),
        session_id: failure.session_id,
        session_kind: failure.session_kind.clone(),
        final_state: SessionState::Failed,
        committed_word_count: 0,
        route_name: None,
        commit_transport: None,
        total_session_latency_ms: None,
        app,
    });
}

fn apply_kind_count(counters: &mut UsageCounters, session_kind: &SessionKind) {
    match session_kind {
        SessionKind::Dictation => counters.dictation_sessions += 1,
        SessionKind::SelectedTextEdit => counters.selected_text_edit_sessions += 1,
        SessionKind::WakePhraseIntent => counters.wake_phrase_intent_sessions += 1,
        SessionKind::InstructedDictation => counters.instructed_dictation_sessions += 1,
    }
}

fn apply_route_count(counters: &mut UsageCounters, route_name: &RouteName) {
    match route_name {
        RouteName::LocalAsrOnly => counters.route_counts.local_asr_only_sessions += 1,
        RouteName::LocalAsrWithRefine => counters.route_counts.local_asr_with_refine_sessions += 1,
        RouteName::CloudAsrWithRefine => counters.route_counts.cloud_asr_with_refine_sessions += 1,
        RouteName::InstructedDictation => counters.route_counts.instructed_dictation_sessions += 1,
    }
}

fn apply_counters(target: &mut UsageCounters, increment: &UsageCounters) {
    target.total_sessions += increment.total_sessions;
    target.successful_sessions += increment.successful_sessions;
    target.failed_sessions += increment.failed_sessions;
    target.dictation_sessions += increment.dictation_sessions;
    target.selected_text_edit_sessions += increment.selected_text_edit_sessions;
    target.wake_phrase_intent_sessions += increment.wake_phrase_intent_sessions;
    target.instructed_dictation_sessions += increment.instructed_dictation_sessions;
    target.words_committed += increment.words_committed;
    target.selected_text_replacements += increment.selected_text_replacements;
    target.total_end_to_end_latency_ms += increment.total_end_to_end_latency_ms;
    target.latency_sample_count += increment.latency_sample_count;
    target.estimated_typing_time_ms += increment.estimated_typing_time_ms;
    target.estimated_time_saved_ms += increment.estimated_time_saved_ms;
    target.route_counts.local_asr_only_sessions += increment.route_counts.local_asr_only_sessions;
    target.route_counts.local_asr_with_refine_sessions +=
        increment.route_counts.local_asr_with_refine_sessions;
    target.route_counts.cloud_asr_with_refine_sessions +=
        increment.route_counts.cloud_asr_with_refine_sessions;
    target.route_counts.instructed_dictation_sessions +=
        increment.route_counts.instructed_dictation_sessions;
}

fn daily_bucket_mut<'a>(
    daily: &'a mut Vec<UsageDailyBucket>,
    date: &str,
) -> &'a mut UsageDailyBucket {
    if let Some(index) = daily.iter().position(|bucket| bucket.date == date) {
        return &mut daily[index];
    }

    daily.push(UsageDailyBucket {
        date: date.to_string(),
        counters: UsageCounters::default(),
    });
    daily
        .last_mut()
        .expect("daily bucket should exist after push")
}

fn summarize_usage_ledger(ledger: &UsageLedger) -> UsageDashboardSummary {
    let analytics_completed_sessions = ledger.analytics_sessions.len() as u64;
    let inserted_text_units = ledger
        .analytics_sessions
        .iter()
        .map(|session| session.inserted_text_units)
        .sum::<u64>();
    let recorded_audio_duration_ms = ledger
        .analytics_sessions
        .iter()
        .map(|session| session.audio_duration_ms)
        .sum::<u64>();
    let estimated_typing_time_ms =
        estimated_typing_time_ms(inserted_text_units, ledger.typing_baseline_wpm);
    let estimated_time_saved_ms =
        estimated_typing_time_ms.saturating_sub(recorded_audio_duration_ms as u128);

    UsageDashboardSummary {
        updated_at_epoch_ms: ledger.updated_at_epoch_ms,
        typing_baseline_wpm: ledger.typing_baseline_wpm,
        total_sessions: ledger.totals.total_sessions,
        successful_sessions: ledger.totals.successful_sessions,
        failed_sessions: ledger.totals.failed_sessions,
        success_rate_percent: if ledger.totals.total_sessions == 0 {
            None
        } else {
            Some((ledger.totals.successful_sessions * 100) / ledger.totals.total_sessions)
        },
        dictation_sessions: ledger.totals.dictation_sessions,
        selected_text_edit_sessions: ledger.totals.selected_text_edit_sessions,
        wake_phrase_intent_sessions: ledger.totals.wake_phrase_intent_sessions,
        instructed_dictation_sessions: ledger.totals.instructed_dictation_sessions,
        words_committed: ledger.totals.words_committed,
        selected_text_replacements: ledger.totals.selected_text_replacements,
        estimated_typing_time_seconds: ms_to_seconds(estimated_typing_time_ms),
        estimated_time_saved_seconds: ms_to_seconds(estimated_time_saved_ms),
        analytics_started_at_epoch_ms: ledger
            .analytics_sessions
            .iter()
            .map(|session| session.completed_at_epoch_ms)
            .min(),
        analytics_completed_sessions,
        inserted_text_units,
        recorded_audio_duration_ms,
        average_dictation_speed_units_per_minute: units_per_minute(
            inserted_text_units,
            recorded_audio_duration_ms,
        ),
        analytics_sessions: ledger.analytics_sessions.clone(),
        productivity_estimate_label: "Estimated typing time saved".to_string(),
        productivity_estimate_note: format!(
            "{} words-per-minute typing baseline minus measured recording time; clamped at zero.",
            ledger.typing_baseline_wpm
        ),
        average_end_to_end_latency_ms: if ledger.totals.latency_sample_count == 0 {
            None
        } else {
            Some(
                (ledger.totals.total_end_to_end_latency_ms
                    / ledger.totals.latency_sample_count as u128) as u64,
            )
        },
        route_counts: ledger.totals.route_counts.clone(),
        latest_app: ledger.latest_app.clone(),
        latest_session: ledger.latest_session.clone(),
        daily: ledger.daily.clone(),
    }
}

impl UsageAppSummary {
    fn from_foreground_app(value: &ForegroundAppMetadata) -> Option<Self> {
        if value.process_name.is_none() && value.window_class.is_none() {
            return None;
        }

        Some(Self {
            app_fingerprint: value.app_fingerprint.clone(),
            process_name: value.process_name.clone(),
            window_class: value.window_class.clone(),
            capture_state: value.capture_state.clone(),
        })
    }
}

fn read_usage_ledger_or_default(path: &Path) -> Result<UsageLedger, String> {
    if !path.exists() {
        return Ok(default_usage_ledger());
    }

    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read usage ledger {}: {}", path.display(), error))?;
    serde_json::from_str::<UsageLedger>(&raw)
        .map_err(|error| format!("failed to parse usage ledger {}: {}", path.display(), error))
}

fn write_usage_ledger(path: &Path, ledger: &UsageLedger) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create usage ledger directory {}: {}",
                parent.display(),
                error
            )
        })?;
    }
    let encoded = serde_json::to_string_pretty(ledger)
        .map_err(|error| format!("failed to encode usage ledger: {error}"))?;
    fs::write(path, encoded)
        .map_err(|error| format!("failed to write usage ledger {}: {}", path.display(), error))
}

fn default_usage_ledger() -> UsageLedger {
    UsageLedger {
        schema_version: USAGE_SCHEMA_VERSION,
        updated_at_epoch_ms: 0,
        typing_baseline_wpm: TYPING_BASELINE_WPM,
        applied_source_run_ids: Vec::new(),
        applied_session_keys: Vec::new(),
        totals: UsageCounters::default(),
        daily: Vec::new(),
        analytics_sessions: Vec::new(),
        latest_app: None,
        latest_session: None,
    }
}

fn estimated_typing_time_ms(words: u64, baseline_wpm: u64) -> u128 {
    if baseline_wpm == 0 {
        return 0;
    }
    (words as u128 * 60_000) / baseline_wpm as u128
}

fn units_per_minute(units: u64, audio_duration_ms: u64) -> Option<u64> {
    if audio_duration_ms == 0 {
        return None;
    }
    Some(((units as u128 * 60_000) / audio_duration_ms as u128) as u64)
}

fn ms_to_seconds(value_ms: u128) -> u64 {
    ((value_ms + 500) / 1000) as u64
}

fn truncate_daily_buckets(daily: &mut Vec<UsageDailyBucket>) {
    if daily.len() <= DAILY_BUCKET_LIMIT {
        return;
    }
    daily.sort_by(|left, right| left.date.cmp(&right.date));
    let excess = daily.len() - DAILY_BUCKET_LIMIT;
    daily.drain(0..excess);
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis()
}

fn utc_date_for_epoch_ms(epoch_ms: u128) -> String {
    let days = (epoch_ms / 86_400_000) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_protocol::{AsrProvider, CommitStatus, RefineProvider, RouteDecision};
    use std::{env, time::UNIX_EPOCH};

    fn session(
        session_id: u64,
        kind: SessionKind,
        state: SessionState,
        committed_text: &str,
        transport: CommitTransport,
        route_name: RouteName,
        latency_ms: u64,
    ) -> SessionSummary {
        SessionSummary {
            session_id,
            session_kind: kind,
            final_state: state.clone(),
            completed_at_epoch_ms: Some(1_700_000_000_000 + u128::from(session_id)),
            start_feedback_latency_ms: Some(0),
            recording_start_latency_ms: Some(100),
            total_session_latency_ms: latency_ms,
            audio_duration_ms: 800,
            audio_peak_level: 0.1,
            audio_rms_level: 0.01,
            asr_diagnostics: None,
            refine_diagnostics: None,
            recognized_text: String::new(),
            committed_text: committed_text.to_string(),
            committed_text_count: Some(count_words_entered(committed_text)),
            route_decision: RouteDecision {
                route_name,
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
            commit_status: if matches!(state, SessionState::Committed) {
                CommitStatus::Success
            } else {
                CommitStatus::Failed("test failure".to_string())
            },
            commit_transport: transport,
            commit_failure_reason: None,
            mode_reason: "test".to_string(),
            selected_text_execution: None,
            wake_phrase_execution: None,
            instructed_dictation_execution: None,
        }
    }

    fn temp_settings_path(label: &str) -> PathBuf {
        env::temp_dir()
            .join(format!(
                "voiceflow-usage-ledger-{label}-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time should be after unix epoch")
                    .as_nanos()
            ))
            .join("settings.json")
    }

    #[test]
    fn appends_usage_without_storing_text_and_dedupes_source_run() {
        let settings_path = temp_settings_path("dedupe");
        let sessions = vec![
            session(
                1,
                SessionKind::Dictation,
                SessionState::Committed,
                "hello real dashboard",
                CommitTransport::DirectUnicodeSendInput,
                RouteName::LocalAsrWithRefine,
                1_000,
            ),
            session(
                2,
                SessionKind::SelectedTextEdit,
                SessionState::Committed,
                "short replacement",
                CommitTransport::ClipboardSelectionReplace,
                RouteName::LocalAsrOnly,
                1_000,
            ),
        ];

        let first = append_live_host_usage(&settings_path, "live-host-report-100", &sessions, &[])
            .expect("usage append should succeed")
            .expect("summary should exist");
        let second = append_live_host_usage(&settings_path, "live-host-report-100", &sessions, &[])
            .expect("duplicate append should succeed")
            .expect("summary should exist");

        assert_eq!(first.total_sessions, 2);
        assert_eq!(second.total_sessions, 2);
        assert_eq!(second.words_committed, 5);
        assert_eq!(second.selected_text_replacements, 1);

        let raw = fs::read_to_string(usage_ledger_path_for_settings_path(&settings_path))
            .expect("ledger should read");
        assert!(!raw.contains("hello real dashboard"));
        assert!(!raw.contains("short replacement"));

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn productivity_estimate_uses_typing_baseline_minus_recording_time() {
        let settings_path = temp_settings_path("estimate");
        let sessions = vec![session(
            1,
            SessionKind::Dictation,
            SessionState::Committed,
            "one two three four",
            CommitTransport::DirectUnicodeSendInput,
            RouteName::LocalAsrOnly,
            1_000,
        )];

        let summary =
            append_live_host_usage(&settings_path, "live-host-report-200", &sessions, &[])
                .expect("usage append should succeed")
                .expect("summary should exist");

        assert_eq!(summary.estimated_typing_time_seconds, 6);
        assert_eq!(summary.estimated_time_saved_seconds, 5);
        assert!(
            summary
                .productivity_estimate_note
                .contains("40 words-per-minute")
        );

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn analytics_include_each_committed_session_kind_once() {
        let settings_path = temp_settings_path("analytics-scope");
        let mut sessions = vec![
            session(
                1,
                SessionKind::Dictation,
                SessionState::Committed,
                "hello",
                CommitTransport::DirectUnicodeSendInput,
                RouteName::LocalAsrOnly,
                1_000,
            ),
            session(
                2,
                SessionKind::SelectedTextEdit,
                SessionState::Committed,
                "你好",
                CommitTransport::ClipboardSelectionReplace,
                RouteName::LocalAsrOnly,
                1_000,
            ),
            session(
                3,
                SessionKind::WakePhraseIntent,
                SessionState::Committed,
                "draft ready",
                CommitTransport::DirectUnicodeSendInput,
                RouteName::LocalAsrWithRefine,
                1_000,
            ),
            session(
                4,
                SessionKind::InstructedDictation,
                SessionState::Committed,
                "计划 2026",
                CommitTransport::DirectUnicodeSendInput,
                RouteName::InstructedDictation,
                1_000,
            ),
            session(
                5,
                SessionKind::Dictation,
                SessionState::Failed,
                "excluded",
                CommitTransport::DirectUnicodeSendInput,
                RouteName::LocalAsrOnly,
                1_000,
            ),
            session(
                6,
                SessionKind::Dictation,
                SessionState::Cancelled,
                "excluded too",
                CommitTransport::DirectUnicodeSendInput,
                RouteName::LocalAsrOnly,
                1_000,
            ),
        ];
        sessions[0].completed_at_epoch_ms = Some(1_700_000_000_000);
        sessions[1].completed_at_epoch_ms = Some(1_700_000_001_000);
        sessions[2].completed_at_epoch_ms = Some(1_700_000_002_000);
        sessions[3].completed_at_epoch_ms = Some(1_700_000_003_000);

        let first = append_live_host_usage(&settings_path, "live-run", &sessions[..2], &[])
            .expect("first append should succeed")
            .expect("summary should exist");
        assert_eq!(first.analytics_completed_sessions, 2);

        let second = append_live_host_usage(&settings_path, "live-run", &sessions, &[])
            .expect("incremental append should succeed")
            .expect("summary should exist");
        assert_eq!(second.analytics_completed_sessions, 4);
        assert_eq!(second.inserted_text_units, 8);
        assert_eq!(second.recorded_audio_duration_ms, 3_200);

        let replay = append_live_host_usage(&settings_path, "live-run", &sessions, &[])
            .expect("replay should succeed")
            .expect("summary should exist");
        assert_eq!(replay.analytics_completed_sessions, 4);
        assert_eq!(replay.analytics_sessions.len(), 4);

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn selected_text_edit_analytics_use_safe_count_and_deduplicate() {
        let settings_path = temp_settings_path("selected-text-analytics");
        let mut committed = session(
            1,
            SessionKind::SelectedTextEdit,
            SessionState::Committed,
            "[redacted selected-text provider edit]",
            CommitTransport::ClipboardSelectionReplace,
            RouteName::LocalAsrOnly,
            4_000,
        );
        committed.completed_at_epoch_ms = Some(1_750_000_000_000);
        committed.committed_text_count = Some(4);
        committed.audio_duration_ms = 3_000;

        let mut failed = session(
            2,
            SessionKind::SelectedTextEdit,
            SessionState::Failed,
            "failed edit output",
            CommitTransport::ClipboardSelectionReplace,
            RouteName::LocalAsrOnly,
            4_000,
        );
        failed.committed_text_count = Some(99);
        failed.audio_duration_ms = 2_000;

        let sessions = [committed, failed];
        let first = append_live_host_usage(&settings_path, "live-edit", &sessions, &[])
            .expect("selected-text usage append should succeed")
            .expect("selected-text usage summary should exist");
        let replay = append_live_host_usage(&settings_path, "live-edit", &sessions, &[])
            .expect("selected-text usage replay should succeed")
            .expect("selected-text usage summary should exist");

        assert_eq!(first.analytics_completed_sessions, 1);
        assert_eq!(replay.analytics_completed_sessions, 1);
        assert_eq!(replay.inserted_text_units, 4);
        assert_eq!(replay.recorded_audio_duration_ms, 3_000);
        assert_eq!(replay.estimated_typing_time_seconds, 6);
        assert_eq!(replay.estimated_time_saved_seconds, 3);
        assert_eq!(replay.average_dictation_speed_units_per_minute, Some(80));
        assert_eq!(replay.analytics_sessions.len(), 1);
        assert_eq!(
            replay.analytics_sessions[0].session_kind,
            SessionKind::SelectedTextEdit
        );
        assert_eq!(
            replay.analytics_sessions[0].completed_at_epoch_ms,
            1_750_000_000_000
        );

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn intention_routing_states_share_one_analytics_session_key() {
        let settings_path = temp_settings_path("intention-dedupe");
        let wake_phrase = session(
            7,
            SessionKind::WakePhraseIntent,
            SessionState::Committed,
            "draft ready",
            CommitTransport::DirectUnicodeSendInput,
            RouteName::LocalAsrWithRefine,
            2_000,
        );
        let instructed = session(
            7,
            SessionKind::InstructedDictation,
            SessionState::Committed,
            "final instructed output",
            CommitTransport::DirectUnicodeSendInput,
            RouteName::InstructedDictation,
            2_000,
        );

        let summary = append_live_host_usage(
            &settings_path,
            "one-intention-session",
            &[wake_phrase, instructed],
            &[],
        )
        .expect("intention usage append should succeed")
        .expect("intention usage summary should exist");

        assert_eq!(summary.analytics_completed_sessions, 1);
        assert_eq!(summary.analytics_sessions.len(), 1);
        assert_eq!(
            summary.analytics_sessions[0].session_key,
            "one-intention-session:session:7"
        );

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn time_saved_and_speed_use_recording_duration() {
        let settings_path = temp_settings_path("recording-time");
        let mut summary = session(
            1,
            SessionKind::Dictation,
            SessionState::Committed,
            "one two three four",
            CommitTransport::DirectUnicodeSendInput,
            RouteName::LocalAsrWithRefine,
            30_000,
        );
        summary.audio_duration_ms = 3_000;

        let usage = append_live_host_usage(&settings_path, "live-run", &[summary], &[])
            .expect("append should succeed")
            .expect("summary should exist");

        assert_eq!(usage.estimated_typing_time_seconds, 6);
        assert_eq!(usage.estimated_time_saved_seconds, 3);
        assert_eq!(usage.average_dictation_speed_units_per_minute, Some(80));

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn latest_app_omits_window_title_and_requires_reliable_metadata() {
        let settings_path = temp_settings_path("app");
        let failure = UsageFailureInput {
            attempted_session_index: 1,
            session_id: 42,
            session_kind: SessionKind::SelectedTextEdit,
            foreground_app: Some(ForegroundAppMetadata {
                app_fingerprint: "process:winword.exe".to_string(),
                process_name: Some("winword.exe".to_string()),
                process_id: Some(123),
                window_class: Some("OpusApp".to_string()),
                window_title: Some("Sensitive Document Name.docx".to_string()),
                capture_state: "captured".to_string(),
            }),
        };

        let summary =
            append_live_host_usage(&settings_path, "live-host-report-300", &[], &[failure])
                .expect("usage append should succeed")
                .expect("summary should exist");

        let latest_app = summary.latest_app.expect("latest app should be captured");
        assert_eq!(latest_app.process_name.as_deref(), Some("winword.exe"));
        assert_eq!(latest_app.window_class.as_deref(), Some("OpusApp"));

        let raw = fs::read_to_string(usage_ledger_path_for_settings_path(&settings_path))
            .expect("ledger should read");
        assert!(!raw.contains("Sensitive Document Name"));

        let _ = fs::remove_dir_all(settings_path.parent().unwrap());
    }

    #[test]
    fn formats_utc_date_for_epoch() {
        assert_eq!(utc_date_for_epoch_ms(0), "1970-01-01");
        assert_eq!(utc_date_for_epoch_ms(86_400_000), "1970-01-02");
    }
}
