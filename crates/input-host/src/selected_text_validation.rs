use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::live_host_reports::{
    discover_selected_text_validation_archive_paths, ensure_live_host_report_dir,
    ensure_selected_text_validation_archive_dir,
    selected_text_validation_archive_path_for_timestamp, selected_text_validation_report_path,
};
use crate::selected_text_compatibility::{
    SelectedTextCompatibilityIssue, selected_text_compatibility_signal,
};
use crate::windows_foreground::ForegroundAppMetadata;

const VALIDATION_HISTORY_LIMIT: usize = 120;
const PROTOTYPE_BOUNDARY_NOTE: &str = "Selected-text validation results describe the current temporary clipboard-backed prototype only. Treat them as real-app compatibility hints, not a production-stable guarantee.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SelectedTextValidationHistoryScope {
    #[serde(rename = "cumulative-history")]
    CumulativeHistory,
    #[serde(rename = "current-baseline")]
    CurrentBaseline,
}

impl SelectedTextValidationHistoryScope {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::CumulativeHistory => "cumulative-history",
            Self::CurrentBaseline => "current-baseline",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub(crate) enum SelectedTextProbeKind {
    EditProbe,
    ReplaceSelectionProbe,
}

impl SelectedTextProbeKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::EditProbe => "Edit probe",
            Self::ReplaceSelectionProbe => "Replace-selection probe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SelectedTextProbeOutcome {
    Passed,
    Failed,
}

impl SelectedTextProbeOutcome {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Passed => "Passed",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SelectedTextValidationVerdict {
    Clean,
    Mixed,
    Blocked,
}

impl SelectedTextValidationVerdict {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Mixed => "mixed",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectedTextValidationRecordInput {
    pub app_label: Option<String>,
    pub foreground_app: Option<ForegroundAppMetadata>,
    pub probe_kind: SelectedTextProbeKind,
    pub probe_input: String,
    pub probe_note: Option<String>,
    pub outcome: SelectedTextProbeOutcome,
    pub commit_transport: Option<String>,
    pub selected_text_action: Option<String>,
    pub failure_reason: Option<String>,
    pub selected_text_compatibility: Option<SelectedTextCompatibilityIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectedTextValidationRecord {
    pub recorded_at_epoch_ms: u128,
    pub app_label: Option<String>,
    #[serde(default = "default_unknown_fingerprint")]
    pub app_fingerprint: String,
    #[serde(default)]
    pub foreground_app: ForegroundAppMetadata,
    pub probe_kind: SelectedTextProbeKind,
    pub probe_input: String,
    pub probe_note: Option<String>,
    pub outcome: SelectedTextProbeOutcome,
    pub commit_transport: Option<String>,
    pub selected_text_action: Option<String>,
    pub failure_reason: Option<String>,
    pub selected_text_compatibility: Option<SelectedTextCompatibilityIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectedTextValidationMatrixEntry {
    pub app_fingerprint: String,
    pub process_name: Option<String>,
    pub latest_app_label: Option<String>,
    pub probe_kind: SelectedTextProbeKind,
    pub verdict: SelectedTextValidationVerdict,
    pub attempted_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
    pub last_outcome: SelectedTextProbeOutcome,
    pub last_recorded_at_epoch_ms: u128,
    pub last_probe_note: Option<String>,
    pub last_commit_transport: Option<String>,
    pub dominant_commit_transport: Option<String>,
    pub dominant_selected_text_compatibility: Option<String>,
    pub last_failure_reason: Option<String>,
    pub last_selected_text_action: Option<String>,
    pub last_window_class: Option<String>,
    pub last_window_title: Option<String>,
    pub last_capture_state: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectedTextValidationReport {
    pub generated_at_epoch_ms: u128,
    pub prototype_boundary_note: String,
    #[serde(default = "default_history_scope")]
    pub history_scope: SelectedTextValidationHistoryScope,
    #[serde(default)]
    pub baseline_started_at_epoch_ms: Option<u128>,
    #[serde(default)]
    pub baseline_label: Option<String>,
    pub total_runs: u64,
    pub total_apps: u64,
    pub total_fingerprints: u64,
    pub matrix: Vec<SelectedTextValidationMatrixEntry>,
    pub records: Vec<SelectedTextValidationRecord>,
}

pub(crate) struct SelectedTextValidationPersistence {
    pub path: PathBuf,
    pub report: SelectedTextValidationReport,
}

pub(crate) struct SelectedTextValidationBaselineReset {
    pub path: PathBuf,
    pub archived_path: Option<PathBuf>,
    pub report: SelectedTextValidationReport,
}

#[derive(Debug, Deserialize)]
struct LegacySelectedTextValidationFile {
    records: Vec<SelectedTextValidationRecord>,
}

pub(crate) fn append_selected_text_validation_record(
    input: SelectedTextValidationRecordInput,
) -> Result<SelectedTextValidationPersistence, String> {
    let _report_dir = ensure_live_host_report_dir()?;
    let path = selected_text_validation_report_path();
    let mut report = read_existing_report_or_default(&path)?;
    report.records.push(build_record(input));
    if report.records.len() > VALIDATION_HISTORY_LIMIT {
        let excess = report.records.len() - VALIDATION_HISTORY_LIMIT;
        report.records.drain(0..excess);
    }

    let report = refresh_report(report);
    write_report(&path, &report)?;

    Ok(SelectedTextValidationPersistence { path, report })
}

pub(crate) fn load_selected_text_validation_report()
-> Result<Option<SelectedTextValidationPersistence>, String> {
    let path = selected_text_validation_report_path();
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).map_err(|error| {
        format!(
            "failed to read selected-text validation report {}: {}",
            path.display(),
            error
        )
    })?;
    let report = parse_report(&raw, &path)?;

    Ok(Some(SelectedTextValidationPersistence { path, report }))
}

pub(crate) fn reset_selected_text_validation_baseline(
    baseline_label: Option<String>,
) -> Result<SelectedTextValidationBaselineReset, String> {
    let _report_dir = ensure_live_host_report_dir()?;
    let path = selected_text_validation_report_path();
    let archived_path = archive_existing_report_if_present(&path)?;
    let now = current_epoch_ms();
    let report = refresh_report(SelectedTextValidationReport {
        generated_at_epoch_ms: now,
        prototype_boundary_note: PROTOTYPE_BOUNDARY_NOTE.to_string(),
        history_scope: SelectedTextValidationHistoryScope::CurrentBaseline,
        baseline_started_at_epoch_ms: Some(now),
        baseline_label: normalize_optional_text(baseline_label),
        total_runs: 0,
        total_apps: 0,
        total_fingerprints: 0,
        matrix: Vec::new(),
        records: Vec::new(),
    });
    write_report(&path, &report)?;

    Ok(SelectedTextValidationBaselineReset {
        path,
        archived_path,
        report,
    })
}

pub(crate) fn selected_text_validation_archive_count() -> Result<u64, String> {
    Ok(discover_selected_text_validation_archive_paths()?.len() as u64)
}

fn build_record(input: SelectedTextValidationRecordInput) -> SelectedTextValidationRecord {
    let foreground_app = input
        .foreground_app
        .unwrap_or_else(|| ForegroundAppMetadata::unknown("not-captured"));

    SelectedTextValidationRecord {
        recorded_at_epoch_ms: current_epoch_ms(),
        app_label: normalize_optional_text(input.app_label),
        app_fingerprint: foreground_app.app_fingerprint.clone(),
        foreground_app,
        probe_kind: input.probe_kind,
        probe_input: input.probe_input,
        probe_note: normalize_optional_text(input.probe_note),
        outcome: input.outcome,
        commit_transport: normalize_optional_text(input.commit_transport),
        selected_text_action: normalize_optional_text(input.selected_text_action),
        failure_reason: normalize_optional_text(input.failure_reason),
        selected_text_compatibility: input.selected_text_compatibility,
    }
}

fn refresh_report(mut report: SelectedTextValidationReport) -> SelectedTextValidationReport {
    let matrix = derive_matrix(&report.records);
    let total_fingerprints = report
        .records
        .iter()
        .map(|record| record.app_fingerprint.clone())
        .collect::<BTreeSet<_>>()
        .len() as u64;

    report.generated_at_epoch_ms = current_epoch_ms();
    report.prototype_boundary_note = PROTOTYPE_BOUNDARY_NOTE.to_string();
    report.total_runs = report.records.len() as u64;
    report.total_apps = total_fingerprints;
    report.total_fingerprints = total_fingerprints;
    report.matrix = matrix;
    report
}

fn derive_report(records: Vec<SelectedTextValidationRecord>) -> SelectedTextValidationReport {
    refresh_report(SelectedTextValidationReport {
        generated_at_epoch_ms: current_epoch_ms(),
        prototype_boundary_note: PROTOTYPE_BOUNDARY_NOTE.to_string(),
        history_scope: SelectedTextValidationHistoryScope::CumulativeHistory,
        baseline_started_at_epoch_ms: None,
        baseline_label: None,
        total_runs: 0,
        total_apps: 0,
        total_fingerprints: 0,
        matrix: Vec::new(),
        records,
    })
}

fn derive_matrix(
    records: &[SelectedTextValidationRecord],
) -> Vec<SelectedTextValidationMatrixEntry> {
    let mut groups =
        BTreeMap::<(String, SelectedTextProbeKind), Vec<&SelectedTextValidationRecord>>::new();
    for record in records {
        groups
            .entry((record.app_fingerprint.clone(), record.probe_kind))
            .or_default()
            .push(record);
    }

    let mut entries = groups
        .into_iter()
        .filter_map(|((app_fingerprint, probe_kind), records)| {
            let latest = records
                .iter()
                .max_by_key(|record| record.recorded_at_epoch_ms)
                .copied()?;
            let attempted_runs = records.len() as u64;
            let successful_runs = records
                .iter()
                .filter(|record| matches!(record.outcome, SelectedTextProbeOutcome::Passed))
                .count() as u64;
            let failed_runs = attempted_runs - successful_runs;

            Some(SelectedTextValidationMatrixEntry {
                app_fingerprint,
                process_name: latest.foreground_app.process_name.clone(),
                latest_app_label: latest.app_label.clone(),
                probe_kind,
                verdict: derive_verdict(successful_runs, failed_runs),
                attempted_runs,
                successful_runs,
                failed_runs,
                last_outcome: latest.outcome,
                last_recorded_at_epoch_ms: latest.recorded_at_epoch_ms,
                last_probe_note: latest.probe_note.clone(),
                last_commit_transport: latest.commit_transport.clone(),
                dominant_commit_transport: dominant_label(
                    records
                        .iter()
                        .filter_map(|record| {
                            if matches!(record.outcome, SelectedTextProbeOutcome::Passed) {
                                record.commit_transport.clone()
                            } else {
                                None
                            }
                        })
                        .collect(),
                ),
                dominant_selected_text_compatibility: dominant_label(
                    records
                        .iter()
                        .filter_map(|record| {
                            record
                                .selected_text_compatibility
                                .as_ref()
                                .map(selected_text_compatibility_signal)
                        })
                        .collect(),
                ),
                last_failure_reason: latest.failure_reason.clone(),
                last_selected_text_action: latest.selected_text_action.clone(),
                last_window_class: latest.foreground_app.window_class.clone(),
                last_window_title: latest.foreground_app.window_title.clone(),
                last_capture_state: latest.foreground_app.capture_state.clone(),
            })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        left.process_name
            .cmp(&right.process_name)
            .then_with(|| left.app_fingerprint.cmp(&right.app_fingerprint))
            .then_with(|| left.probe_kind.cmp(&right.probe_kind))
    });
    entries
}

fn derive_verdict(successful_runs: u64, failed_runs: u64) -> SelectedTextValidationVerdict {
    if successful_runs > 0 && failed_runs == 0 {
        SelectedTextValidationVerdict::Clean
    } else if successful_runs == 0 && failed_runs > 0 {
        SelectedTextValidationVerdict::Blocked
    } else {
        SelectedTextValidationVerdict::Mixed
    }
}

fn dominant_label(labels: Vec<String>) -> Option<String> {
    let mut counts = BTreeMap::<String, u64>::new();
    for label in labels {
        *counts.entry(label).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)))
        .map(|(label, count)| format!("{label} ({count})"))
}

#[cfg(test)]
fn read_existing_records(path: &Path) -> Result<Vec<SelectedTextValidationRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    Ok(read_existing_report_or_default(path)?.records)
}

fn read_existing_report_or_default(path: &Path) -> Result<SelectedTextValidationReport, String> {
    if !path.exists() {
        return Ok(derive_report(Vec::new()));
    }

    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read selected-text validation report {}: {}",
            path.display(),
            error
        )
    })?;
    parse_report(&raw, path)
}

fn parse_report(raw: &str, path: &Path) -> Result<SelectedTextValidationReport, String> {
    match serde_json::from_str::<SelectedTextValidationReport>(raw) {
        Ok(report) => Ok(report),
        Err(report_error) => {
            let legacy =
                serde_json::from_str::<LegacySelectedTextValidationFile>(raw).map_err(|error| {
                    format!(
                        "failed to parse selected-text validation report {}: {}; legacy parse also failed: {}",
                        path.display(),
                        report_error,
                        error
                    )
                })?;
            Ok(derive_report(legacy.records))
        }
    }
}

fn write_report(path: &Path, report: &SelectedTextValidationReport) -> Result<(), String> {
    let encoded = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to encode selected-text validation report: {error}"))?;
    fs::write(path, encoded).map_err(|error| {
        format!(
            "failed to write selected-text validation report {}: {}",
            path.display(),
            error
        )
    })
}

fn archive_existing_report_if_present(path: &Path) -> Result<Option<PathBuf>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let existing_report = read_existing_report_or_default(path)?;
    if existing_report.records.is_empty() && existing_report.baseline_started_at_epoch_ms.is_none()
    {
        return Ok(None);
    }

    let _archive_dir = ensure_selected_text_validation_archive_dir()?;
    let archive_path = selected_text_validation_archive_path_for_timestamp(current_epoch_ms());
    write_report(&archive_path, &existing_report)?;
    Ok(Some(archive_path))
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn default_unknown_fingerprint() -> String {
    "process:unknown".to_string()
}

fn default_history_scope() -> SelectedTextValidationHistoryScope {
    SelectedTextValidationHistoryScope::CumulativeHistory
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::{
        PROTOTYPE_BOUNDARY_NOTE, SelectedTextProbeKind, SelectedTextProbeOutcome,
        SelectedTextValidationHistoryScope, SelectedTextValidationRecord,
        SelectedTextValidationRecordInput, SelectedTextValidationVerdict, derive_report,
        refresh_report,
    };
    use crate::selected_text_compatibility::SelectedTextCompatibilityIssue;
    use crate::windows_foreground::ForegroundAppMetadata;
    use std::{
        env, fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn foreground(
        process_name: &str,
        process_id: u32,
        window_class: &str,
        window_title: &str,
    ) -> ForegroundAppMetadata {
        ForegroundAppMetadata {
            app_fingerprint: format!("process:{}", process_name.to_ascii_lowercase()),
            process_name: Some(process_name.to_ascii_lowercase()),
            process_id: Some(process_id),
            window_class: Some(window_class.to_string()),
            window_title: Some(window_title.to_string()),
            capture_state: "captured".to_string(),
        }
    }

    fn build_record(
        app_label: Option<&str>,
        foreground_app: ForegroundAppMetadata,
        probe_kind: SelectedTextProbeKind,
        outcome: SelectedTextProbeOutcome,
        recorded_at_epoch_ms: u128,
    ) -> SelectedTextValidationRecord {
        SelectedTextValidationRecord {
            recorded_at_epoch_ms,
            app_label: app_label.map(|value| value.to_string()),
            app_fingerprint: foreground_app.app_fingerprint.clone(),
            foreground_app,
            probe_kind,
            probe_input: "uppercase".to_string(),
            probe_note: None,
            outcome,
            commit_transport: Some("ClipboardSelectionReplace".to_string()),
            selected_text_action: Some("Uppercase".to_string()),
            failure_reason: if matches!(outcome, SelectedTextProbeOutcome::Failed) {
                Some("selected-text compatibility issue".to_string())
            } else {
                None
            },
            selected_text_compatibility: if matches!(outcome, SelectedTextProbeOutcome::Failed) {
                Some(SelectedTextCompatibilityIssue {
                    stage: "SelectionCopy".to_string(),
                    reason: "Ctrl+C capture timed out".to_string(),
                    guidance: "The focused app did not expose selected text through synthetic Ctrl+C within the prototype timeout window.".to_string(),
                    detail: "attempt 2 of 2 timed out".to_string(),
                })
            } else {
                None
            },
        }
    }

    fn append_record_in(
        path: &Path,
        input: SelectedTextValidationRecordInput,
        history_limit: usize,
    ) -> Result<super::SelectedTextValidationReport, String> {
        let mut records = super::read_existing_records(path)?;
        records.push(super::build_record(input));
        if records.len() > history_limit {
            let excess = records.len() - history_limit;
            records.drain(0..excess);
        }
        let report = super::derive_report(records);
        let encoded = serde_json::to_string_pretty(&report).map_err(|error| {
            format!("failed to encode selected-text validation report: {error}")
        })?;
        fs::write(path, encoded).map_err(|error| {
            format!(
                "failed to write selected-text validation report {}: {}",
                path.display(),
                error
            )
        })?;
        Ok(report)
    }

    #[test]
    fn derives_matrix_entries_by_fingerprint_and_probe_kind() {
        let report = derive_report(vec![
            build_record(
                Some("Notepad"),
                foreground("notepad.exe", 100, "Notepad", "alpha.txt - Notepad"),
                SelectedTextProbeKind::EditProbe,
                SelectedTextProbeOutcome::Passed,
                100,
            ),
            build_record(
                Some("Notepad"),
                foreground("notepad.exe", 101, "Notepad", "beta.txt - Notepad"),
                SelectedTextProbeKind::EditProbe,
                SelectedTextProbeOutcome::Failed,
                200,
            ),
            build_record(
                Some("Word"),
                foreground("winword.exe", 200, "OpusApp", "Report.docx - Word"),
                SelectedTextProbeKind::ReplaceSelectionProbe,
                SelectedTextProbeOutcome::Passed,
                300,
            ),
        ]);

        assert_eq!(report.prototype_boundary_note, PROTOTYPE_BOUNDARY_NOTE);
        assert_eq!(report.total_runs, 3);
        assert_eq!(report.total_apps, 2);
        assert_eq!(report.total_fingerprints, 2);
        assert_eq!(
            report.history_scope,
            SelectedTextValidationHistoryScope::CumulativeHistory
        );
        assert_eq!(report.matrix.len(), 2);
        assert_eq!(report.matrix[0].app_fingerprint, "process:notepad.exe");
        assert_eq!(report.matrix[0].attempted_runs, 2);
        assert_eq!(report.matrix[0].failed_runs, 1);
        assert_eq!(
            report.matrix[0].verdict,
            SelectedTextValidationVerdict::Mixed
        );
        assert_eq!(
            report.matrix[0]
                .dominant_selected_text_compatibility
                .as_deref(),
            Some("SelectionCopy: Ctrl+C capture timed out (1)")
        );
        assert_eq!(
            report.matrix[0].last_window_title.as_deref(),
            Some("beta.txt - Notepad")
        );
    }

    #[test]
    fn persisted_report_truncates_old_history() {
        let temp_path = env::temp_dir().join(format!(
            "voiceflow-selected-text-validation-test-{}.json",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let first = append_record_in(
            &temp_path,
            SelectedTextValidationRecordInput {
                app_label: Some("Notepad".to_string()),
                foreground_app: Some(foreground(
                    "notepad.exe",
                    100,
                    "Notepad",
                    "alpha.txt - Notepad",
                )),
                probe_kind: SelectedTextProbeKind::EditProbe,
                probe_input: "uppercase".to_string(),
                probe_note: None,
                outcome: SelectedTextProbeOutcome::Passed,
                commit_transport: Some("ClipboardSelectionReplace".to_string()),
                selected_text_action: Some("Uppercase".to_string()),
                failure_reason: None,
                selected_text_compatibility: None,
            },
            2,
        )
        .expect("first report should persist");
        assert_eq!(first.records.len(), 1);

        let second = append_record_in(
            &temp_path,
            SelectedTextValidationRecordInput {
                app_label: Some("Word".to_string()),
                foreground_app: Some(foreground(
                    "winword.exe",
                    200,
                    "OpusApp",
                    "Report.docx - Word",
                )),
                probe_kind: SelectedTextProbeKind::ReplaceSelectionProbe,
                probe_input: "VoiceFlow probe".to_string(),
                probe_note: Some("rich text".to_string()),
                outcome: SelectedTextProbeOutcome::Failed,
                commit_transport: None,
                selected_text_action: None,
                failure_reason: Some("selection replace failed".to_string()),
                selected_text_compatibility: None,
            },
            2,
        )
        .expect("second report should persist");
        assert_eq!(second.records.len(), 2);

        let third = append_record_in(
            &temp_path,
            SelectedTextValidationRecordInput {
                app_label: Some("OneNote".to_string()),
                foreground_app: Some(foreground(
                    "onenote.exe",
                    300,
                    "Framework::CFrame",
                    "Notes - OneNote",
                )),
                probe_kind: SelectedTextProbeKind::EditProbe,
                probe_input: "summarize this".to_string(),
                probe_note: None,
                outcome: SelectedTextProbeOutcome::Passed,
                commit_transport: Some("ClipboardSelectionReplace".to_string()),
                selected_text_action: Some("Bullet summary".to_string()),
                failure_reason: None,
                selected_text_compatibility: None,
            },
            2,
        )
        .expect("third report should persist");

        assert_eq!(third.records.len(), 2);
        assert_eq!(third.records[0].app_label.as_deref(), Some("Word"));
        assert_eq!(third.records[1].app_label.as_deref(), Some("OneNote"));

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn refresh_report_preserves_current_baseline_metadata() {
        let report = refresh_report(super::SelectedTextValidationReport {
            generated_at_epoch_ms: 10,
            prototype_boundary_note: PROTOTYPE_BOUNDARY_NOTE.to_string(),
            history_scope: SelectedTextValidationHistoryScope::CurrentBaseline,
            baseline_started_at_epoch_ms: Some(777),
            baseline_label: Some("post-wordpad-fix".to_string()),
            total_runs: 0,
            total_apps: 0,
            total_fingerprints: 0,
            matrix: Vec::new(),
            records: vec![build_record(
                Some("WordPad"),
                foreground("wordpad.exe", 400, "WordPadClass", "Doc - WordPad"),
                SelectedTextProbeKind::EditProbe,
                SelectedTextProbeOutcome::Passed,
                123,
            )],
        });

        assert_eq!(
            report.history_scope,
            SelectedTextValidationHistoryScope::CurrentBaseline
        );
        assert_eq!(report.baseline_started_at_epoch_ms, Some(777));
        assert_eq!(report.baseline_label.as_deref(), Some("post-wordpad-fix"));
        assert_eq!(report.total_runs, 1);
    }

    #[test]
    fn legacy_parse_derives_cumulative_history_scope() {
        let temp_path = env::temp_dir().join(format!(
            "voiceflow-selected-text-validation-legacy-test-{}.json",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let legacy_json = serde_json::json!({
            "records": [
                {
                    "recorded_at_epoch_ms": 123,
                    "app_label": "WordPad",
                    "app_fingerprint": "process:wordpad.exe",
                    "foreground_app": {
                        "app_fingerprint": "process:wordpad.exe",
                        "process_name": "wordpad.exe",
                        "process_id": 9,
                        "window_class": "WordPadClass",
                        "window_title": "Doc - WordPad",
                        "capture_state": "captured"
                    },
                    "probe_kind": "EditProbe",
                    "probe_input": "make this more concise",
                    "probe_note": "plain text paragraph",
                    "outcome": "Passed",
                    "commit_transport": "ClipboardSelectionReplace",
                    "selected_text_action": "Concise rewrite",
                    "failure_reason": null,
                    "selected_text_compatibility": null
                }
            ]
        });
        fs::write(
            &temp_path,
            serde_json::to_string_pretty(&legacy_json).unwrap(),
        )
        .expect("legacy report should write");

        let raw = fs::read_to_string(&temp_path).expect("legacy report should read");
        let report = super::parse_report(&raw, &temp_path).expect("legacy report should parse");

        assert_eq!(
            report.history_scope,
            SelectedTextValidationHistoryScope::CumulativeHistory
        );
        assert_eq!(report.baseline_started_at_epoch_ms, None);

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn baseline_reset_archives_existing_report_and_starts_fresh_scope() {
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-selected-text-validation-reset-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let report_dir = temp_root.join("reports");
        let archive_dir = report_dir.join("selected-text-validation-archive");
        fs::create_dir_all(&archive_dir).expect("archive dir should be creatable");
        let report_path = report_dir.join("selected-text-validation-report.json");

        let existing = derive_report(vec![build_record(
            Some("WordPad"),
            foreground("wordpad.exe", 100, "WordPadClass", "Doc - WordPad"),
            SelectedTextProbeKind::EditProbe,
            SelectedTextProbeOutcome::Failed,
            100,
        )]);
        super::write_report(&report_path, &existing).expect("existing report should write");

        let archived_path = archive_dir.join("selected-text-validation-report-555.json");
        let archived = super::read_existing_report_or_default(&report_path)
            .expect("existing report should load");
        super::write_report(&archived_path, &archived).expect("archive report should write");

        let fresh = refresh_report(super::SelectedTextValidationReport {
            generated_at_epoch_ms: 777,
            prototype_boundary_note: PROTOTYPE_BOUNDARY_NOTE.to_string(),
            history_scope: SelectedTextValidationHistoryScope::CurrentBaseline,
            baseline_started_at_epoch_ms: Some(777),
            baseline_label: Some("post-fix".to_string()),
            total_runs: 0,
            total_apps: 0,
            total_fingerprints: 0,
            matrix: Vec::new(),
            records: Vec::new(),
        });
        super::write_report(&report_path, &fresh).expect("fresh baseline should write");
        let loaded = super::read_existing_report_or_default(&report_path)
            .expect("fresh baseline should load");

        assert_eq!(
            loaded.history_scope,
            SelectedTextValidationHistoryScope::CurrentBaseline
        );
        assert_eq!(loaded.baseline_label.as_deref(), Some("post-fix"));
        assert_eq!(loaded.total_runs, 0);
        assert!(archived_path.exists());

        let _ = fs::remove_dir_all(temp_root);
    }
}
