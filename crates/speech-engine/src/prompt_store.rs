//! User-managed full system prompts. Explicit null pins a slot to built-in defaults.
use super::*;
use std::{collections::BTreeMap, path::PathBuf};

type Store = BTreeMap<String, Option<String>>;
static WRITE_LOCK: Mutex<()> = Mutex::new(());
pub const MAX_PROMPT_BYTES: usize = 32_000;

fn path() -> PathBuf {
    if let Some(settings) = env::var_os("VOICEFLOW_SETTINGS_PATH").filter(|v| !v.is_empty()) {
        return PathBuf::from(settings).with_extension("prompts.json");
    }
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VoiceFlow Speech Input")
        .join("prompts.json")
}

fn load(path: &std::path::Path) -> Result<Store, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
            "Prompt settings are invalid; restore the configuration file before saving.".into()
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Store::new()),
        Err(_) => Err("Could not read prompt settings.".into()),
    }
}

pub(super) fn saved(slot: &str) -> Option<Option<String>> {
    load(&path()).ok()?.remove(slot)
}

fn builtin(slot: &str) -> Result<String, String> {
    match slot {
        "dictation_light_cleanup" => Ok(compose_dictation_prompt(light_cleanup_rules())),
        "dictation_structured_cleanup" => Ok(compose_dictation_prompt(structured_cleanup_rules())),
        "selected_text_edit" => Ok(default_selected_text_system_prompt().into()),
        "instructed_dictation" => Ok(default_instructed_dictation_system_prompt().into()),
        _ => Err("Unknown prompt slot.".into()),
    }
}

#[derive(Serialize)]
pub struct PromptEditorState {
    pub slot_id: String,
    pub text: String,
    pub builtin_text: String,
    pub source: String,
    pub max_bytes: usize,
}

pub fn read(slot: &str) -> Result<PromptEditorState, String> {
    let builtin_text = builtin(slot)?;
    let store = load(&path())?;
    let (text, source) = match store.get(slot) {
        Some(Some(text)) => (text.clone(), "custom"),
        Some(None) => (builtin_text.clone(), "builtin"),
        None => {
            let status = prompt_override_statuses()
                .into_iter()
                .find(|s| s.slot_id == slot);
            let source =
                if status.is_some_and(|s| s.status == PromptOverrideStatusKind::CustomActive) {
                    "file"
                } else {
                    "builtin"
                };
            let text = match slot {
                "dictation_light_cleanup" => {
                    resolve_dictation_prompt(DictationRefinementMode::LightCleanup).text
                }
                "dictation_structured_cleanup" => {
                    resolve_dictation_prompt(DictationRefinementMode::StructuredCleanup).text
                }
                "selected_text_edit" => {
                    resolve_prompt_from_env(SELECTED_TEXT_PROMPT_FILE_ENV, &builtin_text)
                }
                _ => resolve_prompt_from_env(INSTRUCTED_DICTATION_PROMPT_FILE_ENV, &builtin_text),
            };
            (text, source)
        }
    };
    Ok(PromptEditorState {
        slot_id: slot.into(),
        text,
        builtin_text,
        source: source.into(),
        max_bytes: MAX_PROMPT_BYTES,
    })
}

fn save_at(path: &std::path::Path, slot: &str, text: Option<&str>) -> Result<(), String> {
    builtin(slot)?;
    if text.is_some_and(|t| t.trim().is_empty() || t.len() > MAX_PROMPT_BYTES || t.contains('\0')) {
        return Err(
            "Prompt must be nonempty, without NUL characters, and at most 32,000 UTF-8 bytes."
                .into(),
        );
    }
    let _lock = WRITE_LOCK.lock().map_err(|_| "Prompt settings are busy.")?;
    let mut store = load(path)?;
    store.insert(slot.into(), text.map(|t| t.trim().to_string()));
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    fs::create_dir_all(parent).map_err(|_| "Could not create prompt settings directory.")?;
    let tmp = path.with_extension(format!("prompts-{}.tmp", std::process::id()));
    let result = (|| {
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(&store).map_err(|_| "Could not encode prompts.")?,
        )
        .map_err(|_| "Could not write prompts.")?;
        fs::rename(&tmp, path).map_err(|_| "Could not replace prompt settings.")
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(String::from)
}

pub fn save(slot: &str, text: Option<&str>) -> Result<PromptEditorState, String> {
    save_at(&path(), slot, text)?;
    read(slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_reset_preserves_other_slots_and_rejects_bad_input() {
        let dir = env::temp_dir().join(format!(
            "voiceflow-prompts-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("prompts.json");
        save_at(&path, "selected_text_edit", Some("original")).unwrap();
        save_at(&path, "dictation_light_cleanup", Some("custom")).unwrap();
        save_at(&path, "dictation_light_cleanup", None).unwrap();
        let state = load(&path).unwrap();
        assert_eq!(state["selected_text_edit"].as_deref(), Some("original"));
        assert_eq!(state["dictation_light_cleanup"], None);
        for (slot, value) in [
            ("../escape", "x"),
            ("selected_text_edit", "  "),
            ("selected_text_edit", "a\0b"),
        ] {
            assert!(save_at(&path, slot, Some(value)).is_err());
        }
        assert!(
            save_at(
                &path,
                "selected_text_edit",
                Some(&"x".repeat(MAX_PROMPT_BYTES + 1))
            )
            .is_err()
        );
        assert_eq!(load(&path).unwrap(), state);
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
