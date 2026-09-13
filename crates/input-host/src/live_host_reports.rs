use std::{
    env, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

const LIVE_HOST_REPORT_RETENTION_LIMIT: usize = 20;

pub(crate) fn default_live_host_report_dir() -> PathBuf {
    env::temp_dir()
        .join("voiceflow-speech-input")
        .join("reports")
}

pub(crate) fn selected_text_validation_report_path() -> PathBuf {
    default_live_host_report_dir().join("selected-text-validation-report.json")
}

pub(crate) fn selected_text_validation_archive_dir() -> PathBuf {
    default_live_host_report_dir().join("selected-text-validation-archive")
}

pub(crate) fn selected_text_validation_archive_path_for_timestamp(
    archived_at_epoch_ms: u128,
) -> PathBuf {
    selected_text_validation_archive_dir().join(format!(
        "selected-text-validation-report-{archived_at_epoch_ms}.json"
    ))
}

pub(crate) fn live_host_report_path_for_timestamp(generated_at_epoch_ms: u128) -> PathBuf {
    default_live_host_report_dir().join(format!("live-host-report-{generated_at_epoch_ms}.json"))
}

pub(crate) fn ensure_live_host_report_dir() -> Result<PathBuf, String> {
    let report_dir = default_live_host_report_dir();
    fs::create_dir_all(&report_dir).map_err(|error| {
        format!(
            "failed to create live-host report directory {}: {}",
            report_dir.display(),
            error
        )
    })?;
    Ok(report_dir)
}

pub(crate) fn ensure_selected_text_validation_archive_dir() -> Result<PathBuf, String> {
    let archive_dir = selected_text_validation_archive_dir();
    fs::create_dir_all(&archive_dir).map_err(|error| {
        format!(
            "failed to create selected-text validation archive directory {}: {}",
            archive_dir.display(),
            error
        )
    })?;
    Ok(archive_dir)
}

pub(crate) fn discover_selected_text_validation_archive_paths() -> Result<Vec<PathBuf>, String> {
    let archive_dir = selected_text_validation_archive_dir();
    if !archive_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&archive_dir).map_err(|error| {
        format!(
            "failed to read selected-text validation archive directory {}: {}",
            archive_dir.display(),
            error
        )
    })?;
    let mut archive_entries = Vec::new();

    for entry_result in entries {
        let entry = entry_result.map_err(|error| {
            format!(
                "failed to inspect selected-text validation archive directory {}: {}",
                archive_dir.display(),
                error
            )
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if !(file_name.starts_with("selected-text-validation-report-")
            && file_name.ends_with(".json"))
        {
            continue;
        }

        let modified = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .unwrap_or(std::time::UNIX_EPOCH);
        archive_entries.push((path, modified, file_name));
    }

    archive_entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.2.cmp(&left.2)));
    Ok(archive_entries
        .into_iter()
        .map(|(path, _, _)| path)
        .collect())
}

pub(crate) fn discover_latest_live_host_report_path() -> Result<Option<PathBuf>, String> {
    discover_latest_live_host_report_path_in(&default_live_host_report_dir())
}

pub(crate) fn prune_old_live_host_reports() -> Result<Vec<PathBuf>, String> {
    prune_old_live_host_reports_in(
        &default_live_host_report_dir(),
        LIVE_HOST_REPORT_RETENTION_LIMIT,
    )
}

pub(crate) fn discover_latest_live_host_report_path_in(
    report_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    Ok(discover_live_host_report_paths_in(report_dir)?
        .into_iter()
        .next())
}

pub(crate) fn discover_live_host_report_paths_in(
    report_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    if !report_dir.exists() {
        return Ok(Vec::new());
    }

    let mut report_entries = collect_live_host_report_entries(report_dir)?;
    report_entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.2.cmp(&left.2)));
    Ok(report_entries
        .into_iter()
        .map(|(path, _, _)| path)
        .collect())
}

pub(crate) fn prune_old_live_host_reports_in(
    report_dir: &Path,
    retention_limit: usize,
) -> Result<Vec<PathBuf>, String> {
    if retention_limit == 0 {
        return Err("live-host report retention limit must be greater than 0".to_string());
    }
    if !report_dir.exists() {
        return Ok(Vec::new());
    }

    let mut report_entries = collect_live_host_report_entries(report_dir)?;
    if report_entries.len() <= retention_limit {
        return Ok(Vec::new());
    }

    report_entries.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.2.cmp(&right.2)));

    let prune_count = report_entries.len() - retention_limit;
    let mut pruned_paths = Vec::with_capacity(prune_count);
    for (path, _, _) in report_entries.into_iter().take(prune_count) {
        fs::remove_file(&path).map_err(|error| {
            format!(
                "failed to remove stale live-host report {}: {}",
                path.display(),
                error
            )
        })?;
        pruned_paths.push(path);
    }

    Ok(pruned_paths)
}

fn collect_live_host_report_entries(
    report_dir: &Path,
) -> Result<Vec<(PathBuf, SystemTime, String)>, String> {
    let entries = fs::read_dir(report_dir).map_err(|error| {
        format!(
            "failed to read live-host report directory {}: {}",
            report_dir.display(),
            error
        )
    })?;
    let mut report_entries = Vec::new();

    for entry_result in entries {
        let entry = entry_result.map_err(|error| {
            format!(
                "failed to inspect live-host report directory {}: {}",
                report_dir.display(),
                error
            )
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if !(file_name.starts_with("live-host-report-") && file_name.ends_with(".json")) {
            continue;
        }

        let modified = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .unwrap_or(std::time::UNIX_EPOCH);
        report_entries.push((path, modified, file_name));
    }

    Ok(report_entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn discovers_latest_live_host_report_from_directory() {
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-live-report-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let older = temp_root.join("live-host-report-100.json");
        let newer = temp_root.join("live-host-report-200.json");
        fs::write(&older, "{}").expect("older report should write");
        std::thread::sleep(Duration::from_millis(5));
        fs::write(&newer, "{}").expect("newer report should write");

        let discovered = discover_latest_live_host_report_path_in(&temp_root)
            .expect("report discovery should succeed")
            .expect("latest report should be discovered");

        assert_eq!(discovered, newer);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn prunes_old_live_host_reports_while_preserving_newest_entries() {
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-live-report-prune-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");

        let oldest = temp_root.join("live-host-report-100.json");
        let middle = temp_root.join("live-host-report-200.json");
        let newest = temp_root.join("live-host-report-300.json");
        let unrelated = temp_root.join("notes.txt");
        fs::write(&oldest, "{}").expect("oldest report should write");
        std::thread::sleep(Duration::from_millis(5));
        fs::write(&middle, "{}").expect("middle report should write");
        std::thread::sleep(Duration::from_millis(5));
        fs::write(&newest, "{}").expect("newest report should write");
        fs::write(&unrelated, "keep").expect("unrelated file should write");

        let pruned =
            prune_old_live_host_reports_in(&temp_root, 2).expect("report pruning should succeed");

        assert_eq!(pruned, vec![oldest.clone()]);
        assert!(!oldest.exists());
        assert!(middle.exists());
        assert!(newest.exists());
        assert!(unrelated.exists());

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn discovers_live_host_report_paths_in_newest_first_order() {
        let temp_root = env::temp_dir().join(format!(
            "voiceflow-live-report-order-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp report dir should be creatable");
        let oldest = temp_root.join("live-host-report-100.json");
        let newest = temp_root.join("live-host-report-300.json");
        let middle = temp_root.join("live-host-report-200.json");
        fs::write(&oldest, "{}").expect("oldest report should write");
        std::thread::sleep(Duration::from_millis(5));
        fs::write(&middle, "{}").expect("middle report should write");
        std::thread::sleep(Duration::from_millis(5));
        fs::write(&newest, "{}").expect("newest report should write");

        let discovered = discover_live_host_report_paths_in(&temp_root)
            .expect("report discovery should succeed");

        assert_eq!(discovered, vec![newest, middle, oldest]);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn builds_selected_text_validation_report_path_in_report_directory() {
        let path = selected_text_validation_report_path();

        assert!(path.ends_with(PathBuf::from(
            "voiceflow-speech-input\\reports\\selected-text-validation-report.json"
        )));
    }

    #[test]
    fn builds_selected_text_validation_archive_path_in_archive_directory() {
        let path = selected_text_validation_archive_path_for_timestamp(123);

        assert!(path.ends_with(PathBuf::from(
            "voiceflow-speech-input\\reports\\selected-text-validation-archive\\selected-text-validation-report-123.json"
        )));
    }
}
