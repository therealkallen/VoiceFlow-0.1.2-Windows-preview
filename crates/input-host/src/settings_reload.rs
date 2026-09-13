use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::settings_store::{LoadedRuntimeSettings, load_runtime_settings_from_path};

#[derive(Clone, Debug, PartialEq, Eq)]
struct SettingsFileRevision {
    created: Option<SystemTime>,
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug)]
pub(crate) enum SettingsReloadPoll {
    Unchanged,
    Loaded(LoadedRuntimeSettings),
    Failed(String),
}

#[derive(Debug)]
pub(crate) struct RuntimeSettingsReloader {
    path: PathBuf,
    observed_revision: Option<SettingsFileRevision>,
    pending_loaded: Option<LoadedRuntimeSettings>,
}

impl RuntimeSettingsReloader {
    pub(crate) fn new(initial: &LoadedRuntimeSettings) -> Result<Self, String> {
        let path = initial.path.clone();
        let current = load_runtime_settings_from_path(&path)?;
        let pending_loaded = (current.settings != initial.settings).then_some(current);
        Ok(Self {
            observed_revision: settings_file_revision(&path)?,
            path,
            pending_loaded,
        })
    }

    pub(crate) fn poll(&mut self) -> SettingsReloadPoll {
        if let Some(loaded) = self.pending_loaded.take() {
            return SettingsReloadPoll::Loaded(loaded);
        }
        let revision = match settings_file_revision(&self.path) {
            Ok(revision) => revision,
            Err(error) => return SettingsReloadPoll::Failed(error),
        };

        if revision == self.observed_revision {
            return SettingsReloadPoll::Unchanged;
        }
        self.observed_revision = revision;
        match load_runtime_settings_from_path(&self.path) {
            Ok(loaded) => SettingsReloadPoll::Loaded(loaded),
            Err(error) => SettingsReloadPoll::Failed(error),
        }
    }
}

fn settings_file_revision(path: &Path) -> Result<Option<SettingsFileRevision>, String> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(SettingsFileRevision {
            created: metadata.created().ok(),
            modified: metadata.modified().ok(),
            len: metadata.len(),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "failed to inspect runtime settings revision at {}: {}",
            path.display(),
            error
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_protocol::{RuntimeSettings, ShortcutMode, SystemLanguage};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    fn temp_settings_path(label: &str) -> PathBuf {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!(
                "voiceflow-settings-reload-{label}-{}-{sequence}",
                std::process::id()
            ))
            .join("settings.json")
    }

    fn write_settings(path: &Path, settings: &RuntimeSettings) {
        fs::create_dir_all(path.parent().unwrap()).expect("test directory should be created");
        fs::write(
            path,
            serde_json::to_string_pretty(settings).expect("settings should encode"),
        )
        .expect("settings should be written");
    }

    fn load_settings(path: &Path) -> LoadedRuntimeSettings {
        load_runtime_settings_from_path(path).expect("test settings should load")
    }

    #[test]
    fn valid_revision_loads_once_and_duplicate_polls_are_ignored() {
        let path = temp_settings_path("valid");
        write_settings(&path, &RuntimeSettings::default());
        let initial = load_settings(&path);
        let mut reloader = RuntimeSettingsReloader::new(&initial).expect("reloader");
        let mut updated = RuntimeSettings::default();
        updated.system_language = SystemLanguage::Chinese;
        updated.shortcut_mode = ShortcutMode::PushToTalk;
        write_settings(&path, &updated);

        assert!(matches!(
            reloader.poll(),
            SettingsReloadPoll::Loaded(LoadedRuntimeSettings { settings, .. })
                if settings == updated
        ));
        assert!(matches!(reloader.poll(), SettingsReloadPoll::Unchanged));

        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn invalid_revision_is_reported_once_and_later_valid_revision_recovers() {
        let path = temp_settings_path("invalid");
        write_settings(&path, &RuntimeSettings::default());
        let initial = load_settings(&path);
        let mut reloader = RuntimeSettingsReloader::new(&initial).expect("reloader");
        fs::write(&path, "{ invalid json").expect("invalid settings should be written");
        assert!(matches!(reloader.poll(), SettingsReloadPoll::Failed(_)));
        assert!(matches!(reloader.poll(), SettingsReloadPoll::Unchanged));

        let mut recovered = RuntimeSettings::default();
        recovered.system_language = SystemLanguage::Chinese;
        write_settings(&path, &recovered);
        assert!(matches!(
            reloader.poll(),
            SettingsReloadPoll::Loaded(LoadedRuntimeSettings { settings, .. })
                if settings == recovered
        ));

        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
