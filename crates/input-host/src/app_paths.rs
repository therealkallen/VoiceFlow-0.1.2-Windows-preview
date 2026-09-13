use std::path::{Path, PathBuf};

pub(crate) fn resolve_repo_root() -> Result<PathBuf, String> {
    // A portable bundle owns its assets regardless of the launcher's working directory.
    if let Ok(executable) = std::env::current_exe() {
        if let Some(root) = executable.parent().filter(|root| has_ui_assets(root)) {
            return Ok(root.to_path_buf());
        }
    }
    let current_dir = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory for app paths: {error}"))?;
    if has_ui_assets(&current_dir) {
        return Ok(current_dir);
    }

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|path| path.parent())
        .filter(|path| has_ui_assets(path))
        .map(PathBuf::from)
        .ok_or_else(|| "UI assets not found; keep the apps directory beside input-host.exe".to_string())
}

fn has_ui_assets(root: &Path) -> bool {
    root.join("apps/settings-ui/index.html").is_file()
        && root.join("apps/overlay-ui/index.html").is_file()
}

pub(crate) fn runtime_script_path_for_settings(settings: &Path, file_name: &str) -> PathBuf {
    settings.parent().unwrap_or_else(|| Path::new("."))
        .join("runtime").join(file_name)
}

pub(crate) fn overlay_runtime_script_path() -> Result<PathBuf, String> {
    Ok(runtime_script_path_for_settings(&crate::settings_store::resolve_settings_path()?, "overlay-state.js"))
}

#[cfg(test)]
pub(crate) fn settings_runtime_script_path() -> Result<PathBuf, String> {
    Ok(runtime_script_path_for_settings(&crate::settings_store::resolve_settings_path()?, "settings-state.js"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_repo_root_with_apps_directory() {
        let repo_root = resolve_repo_root().expect("repo root should resolve");

        assert!(repo_root.join("apps").exists());
        assert!(repo_root.join("README.md").exists());
    }

    #[test]
    fn runtime_scripts_live_beside_settings_not_in_assets() {
        let settings = Path::new("C:/Users/test/VoiceFlow/settings.json");
        assert_eq!(runtime_script_path_for_settings(settings, "settings-state.js"),
            PathBuf::from("C:/Users/test/VoiceFlow/runtime/settings-state.js"));
    }
}
