use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize};
use shared_protocol::{
    DiagnosticsVerbosity, HistoryRetention, KeyModifier, ProviderPreset, ProviderSettings,
    RefinementQuality, RuntimeSettings, Shortcut, ShortcutMode, SystemLanguage, UiStyle,
    WakePhraseConfig,
};

use crate::history_ledger::enforce_history_retention_for_settings_path;

const SETTINGS_OVERRIDE_ENV: &str = "VOICEFLOW_SETTINGS_PATH";
const MIN_PROVIDER_TIMEOUT_MS: u64 = 1_000;
const MAX_PROVIDER_TIMEOUT_MS: u64 = 120_000;
static SETTINGS_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum RuntimeSettingsSource {
    Defaults,
    File,
}

#[derive(Clone, Debug, Serialize)]
pub struct LoadedRuntimeSettings {
    pub settings: RuntimeSettings,
    pub path: PathBuf,
    pub source: RuntimeSettingsSource,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSettingsUpdate {
    pub primary_shortcut_modifiers: Option<Vec<KeyModifier>>,
    pub primary_shortcut_key: Option<String>,
    pub shortcut_mode: Option<ShortcutMode>,
    pub refinement_quality: Option<RefinementQuality>,
    pub silence_gate_level: Option<u8>,
    pub diagnostics_verbosity: Option<DiagnosticsVerbosity>,
    pub system_language: Option<SystemLanguage>,
    pub ui_style: Option<UiStyle>,
    pub audio_feedback_enabled: Option<bool>,
    pub wake_phrase_enabled: Option<bool>,
    pub wake_phrase_phrase: Option<String>,
    pub history_retention: Option<HistoryRetention>,
    pub provider: Option<ProviderSettingsUpdate>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSettingsUpdate {
    pub preset: Option<ProviderPreset>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub base_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub active_model: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub request_timeout_ms: Option<Option<u64>>,
}

impl ProviderSettingsUpdate {
    pub fn is_empty(&self) -> bool {
        self.preset.is_none()
            && self.base_url.is_none()
            && self.active_model.is_none()
            && self.request_timeout_ms.is_none()
    }
}

fn deserialize_nullable_update<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

impl RuntimeSettingsUpdate {
    pub fn is_empty(&self) -> bool {
        self.primary_shortcut_modifiers.is_none()
            && self.primary_shortcut_key.is_none()
            && self.shortcut_mode.is_none()
            && self.refinement_quality.is_none()
            && self.silence_gate_level.is_none()
            && self.diagnostics_verbosity.is_none()
            && self.system_language.is_none()
            && self.ui_style.is_none()
            && self.audio_feedback_enabled.is_none()
            && self.wake_phrase_enabled.is_none()
            && self.wake_phrase_phrase.is_none()
            && self.history_retention.is_none()
            && self
                .provider
                .as_ref()
                .map_or(true, ProviderSettingsUpdate::is_empty)
    }
}

pub fn load_runtime_settings() -> Result<LoadedRuntimeSettings, String> {
    let path = resolve_settings_path()?;
    load_runtime_settings_from_path(&path)
}

pub fn write_default_settings_template() -> Result<PathBuf, String> {
    let path = resolve_settings_path()?;
    if path.exists() {
        return Err(format!(
            "settings file already exists at {}; edit it directly or remove it before writing a new template",
            path.display()
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create settings directory {}: {}",
                parent.display(),
                error
            )
        })?;
    }

    let encoded = serde_json::to_string_pretty(&RuntimeSettings::default())
        .map_err(|error| format!("failed to encode default runtime settings: {error}"))?;
    fs::write(&path, encoded).map_err(|error| {
        format!(
            "failed to write default runtime settings template {}: {}",
            path.display(),
            error
        )
    })?;

    Ok(path)
}

pub fn update_runtime_settings(
    update: RuntimeSettingsUpdate,
) -> Result<LoadedRuntimeSettings, String> {
    let path = resolve_settings_path()?;
    update_runtime_settings_at_path(&path, update)
}

pub(crate) fn load_runtime_settings_from_path(
    path: &Path,
) -> Result<LoadedRuntimeSettings, String> {
    if !path.exists() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings::default());
        return Ok(LoadedRuntimeSettings {
            settings,
            path: path.to_path_buf(),
            source: RuntimeSettingsSource::Defaults,
            warnings,
        });
    }

    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read runtime settings {}: {}",
            path.display(),
            error
        )
    })?;
    let normalized_raw = strip_utf8_bom(&raw);
    let settings = serde_json::from_str::<RuntimeSettings>(normalized_raw).map_err(|error| {
        format!(
            "failed to parse runtime settings {} as JSON: {}",
            path.display(),
            error
        )
    })?;
    let (settings, warnings) = sanitize_runtime_settings(settings);

    Ok(LoadedRuntimeSettings {
        settings,
        path: path.to_path_buf(),
        source: RuntimeSettingsSource::File,
        warnings,
    })
}

fn update_runtime_settings_at_path(
    path: &Path,
    update: RuntimeSettingsUpdate,
) -> Result<LoadedRuntimeSettings, String> {
    if update.is_empty() {
        return Err("at least one runtime setting update must be provided".to_string());
    }
    validate_runtime_settings_update(&update)?;

    let base_settings = if path.exists() {
        load_runtime_settings_from_path(path)?.settings
    } else {
        RuntimeSettings::default()
    };
    let should_enforce_history_retention = update.history_retention.is_some();
    let (settings, warnings) =
        sanitize_runtime_settings(apply_runtime_settings_update(base_settings, update));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create settings directory {}: {}",
                parent.display(),
                error
            )
        })?;
    }

    let encoded = serde_json::to_string_pretty(&settings)
        .map_err(|error| format!("failed to encode updated runtime settings: {error}"))?;
    write_settings_atomically(path, encoded.as_bytes())?;
    if should_enforce_history_retention {
        enforce_history_retention_for_settings_path(path, &settings.history_retention)?;
    }

    Ok(LoadedRuntimeSettings {
        settings,
        path: path.to_path_buf(),
        source: RuntimeSettingsSource::File,
        warnings,
    })
}

fn write_settings_atomically(path: &Path, encoded: &[u8]) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings.json");
    let sequence = SETTINGS_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));

    let write_result = (|| {
        let mut temp_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "failed to create temporary runtime settings file {}: {}",
                    temp_path.display(),
                    error
                )
            })?;
        temp_file.write_all(encoded).map_err(|error| {
            format!(
                "failed to write temporary runtime settings file {}: {}",
                temp_path.display(),
                error
            )
        })?;
        temp_file.sync_all().map_err(|error| {
            format!(
                "failed to flush temporary runtime settings file {}: {}",
                temp_path.display(),
                error
            )
        })?;
        drop(temp_file);

        replace_settings_file(&temp_path, path)
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

#[cfg(windows)]
fn replace_settings_file(temp_path: &Path, path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_WRITE_THROUGH, MoveFileExW, REPLACEFILE_WRITE_THROUGH, ReplaceFileW,
    };

    let wide = |value: &Path| {
        value
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let destination = wide(path);
    let replacement = wide(temp_path);
    let result = if path.exists() {
        unsafe {
            ReplaceFileW(
                destination.as_ptr(),
                replacement.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null(),
                std::ptr::null(),
            )
        }
    } else {
        unsafe {
            MoveFileExW(
                replacement.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        }
    };

    if result == 0 {
        Err(format!(
            "failed to atomically replace runtime settings {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_settings_file(temp_path: &Path, path: &Path) -> Result<(), String> {
    fs::rename(temp_path, path).map_err(|error| {
        format!(
            "failed to atomically replace runtime settings {}: {}",
            path.display(),
            error
        )
    })
}

pub(crate) fn resolve_settings_path() -> Result<PathBuf, String> {
    let current_dir = env::current_dir()
        .map_err(|error| format!("failed to determine the current working directory: {error}"))?;
    let override_value = env::var(SETTINGS_OVERRIDE_ENV).ok();
    let local_app_data = env::var("LOCALAPPDATA").ok();
    Ok(resolve_settings_path_from_inputs(
        override_value.as_deref(),
        local_app_data.as_deref(),
        &current_dir,
    ))
}

fn resolve_settings_path_from_inputs(
    override_value: Option<&str>,
    local_app_data: Option<&str>,
    current_dir: &Path,
) -> PathBuf {
    if let Some(path) = override_value
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(path);
    }

    if let Some(local_app_data) = local_app_data
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(local_app_data)
            .join("VoiceFlow Speech Input")
            .join("settings.json");
    }

    current_dir.join("voiceflow-settings.json")
}

fn apply_runtime_settings_update(
    mut settings: RuntimeSettings,
    update: RuntimeSettingsUpdate,
) -> RuntimeSettings {
    if let Some(shortcut_mode) = update.shortcut_mode {
        settings.shortcut_mode = shortcut_mode;
    }
    if let Some(primary_shortcut_modifiers) = update.primary_shortcut_modifiers {
        settings.dictation_shortcut.modifiers = primary_shortcut_modifiers.clone();
        settings.edit_shortcut.modifiers = primary_shortcut_modifiers;
    }
    if let Some(primary_shortcut_key) = update.primary_shortcut_key {
        settings.dictation_shortcut.key = primary_shortcut_key.clone();
        settings.edit_shortcut.key = primary_shortcut_key;
    }
    if let Some(refinement_quality) = update.refinement_quality {
        settings.refinement_quality = refinement_quality;
    }
    if let Some(silence_gate_level) = update.silence_gate_level {
        settings.silence_gate_level = silence_gate_level;
    }
    if let Some(diagnostics_verbosity) = update.diagnostics_verbosity {
        settings.diagnostics_verbosity = diagnostics_verbosity;
    }
    if let Some(system_language) = update.system_language {
        settings.system_language = system_language;
    }
    if let Some(ui_style) = update.ui_style {
        settings.ui_style = ui_style;
    }
    if let Some(audio_feedback_enabled) = update.audio_feedback_enabled {
        settings.audio_feedback_enabled = audio_feedback_enabled;
    }
    if let Some(wake_phrase_enabled) = update.wake_phrase_enabled {
        settings.wake_phrase.enabled = wake_phrase_enabled;
    }
    if let Some(wake_phrase_phrase) = update.wake_phrase_phrase {
        settings.wake_phrase.phrase = wake_phrase_phrase;
    }
    if let Some(history_retention) = update.history_retention {
        settings.history_retention = history_retention;
    }
    if let Some(provider_update) = update.provider {
        apply_provider_settings_update(&mut settings.provider, provider_update);
    }

    settings
}

fn apply_provider_settings_update(provider: &mut ProviderSettings, update: ProviderSettingsUpdate) {
    if let Some(preset) = update.preset {
        provider.preset = preset;
    }
    if let Some(base_url) = update.base_url {
        provider.base_url = normalize_optional_provider_update_string(base_url);
    }
    if let Some(active_model) = update.active_model {
        provider.active_model = normalize_optional_provider_update_string(active_model);
    }
    if let Some(request_timeout_ms) = update.request_timeout_ms {
        provider.request_timeout_ms = request_timeout_ms;
    }
}

fn normalize_optional_provider_update_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_runtime_settings_update(update: &RuntimeSettingsUpdate) -> Result<(), String> {
    let Some(provider) = update.provider.as_ref() else {
        return Ok(());
    };
    if let Some(Some(base_url)) = provider.base_url.as_ref() {
        let trimmed = base_url.trim();
        if !trimmed.is_empty() && !is_valid_provider_base_url(trimmed) {
            return Err(
                "provider.base_url must be an http(s) URL without credentials, query, or fragment"
                    .to_string(),
            );
        }
    }
    if let Some(Some(timeout_ms)) = provider.request_timeout_ms {
        if !(MIN_PROVIDER_TIMEOUT_MS..=MAX_PROVIDER_TIMEOUT_MS).contains(&timeout_ms) {
            return Err(format!(
                "provider.request_timeout_ms must be between {MIN_PROVIDER_TIMEOUT_MS} and {MAX_PROVIDER_TIMEOUT_MS}"
            ));
        }
    }
    Ok(())
}

fn sanitize_runtime_settings(mut settings: RuntimeSettings) -> (RuntimeSettings, Vec<String>) {
    let mut warnings = Vec::new();

    settings.dictation_shortcut = sanitize_shortcut(
        settings.dictation_shortcut,
        &RuntimeSettings::default().dictation_shortcut,
        "dictation_shortcut",
        &mut warnings,
    );
    settings.edit_shortcut = sanitize_shortcut(
        settings.edit_shortcut,
        &RuntimeSettings::default().edit_shortcut,
        "edit_shortcut",
        &mut warnings,
    );
    settings.wake_phrase = sanitize_wake_phrase(settings.wake_phrase, &mut warnings);
    settings.refinement_quality =
        sanitize_refinement_quality(settings.refinement_quality, &mut warnings);
    settings.silence_gate_level =
        sanitize_silence_gate_level(settings.silence_gate_level, &mut warnings);
    settings.provider = sanitize_provider_settings(settings.provider, &mut warnings);
    append_shortcut_advisories(&settings, &mut warnings);

    (settings, warnings)
}

fn sanitize_provider_settings(
    mut provider: ProviderSettings,
    warnings: &mut Vec<String>,
) -> ProviderSettings {
    provider.base_url = sanitize_optional_provider_string(
        provider.base_url,
        "provider.base_url",
        warnings,
        |value| is_valid_provider_base_url(value),
    );
    provider.active_model = sanitize_optional_provider_string(
        provider.active_model,
        "provider.active_model",
        warnings,
        |_| true,
    );
    provider.request_timeout_ms = sanitize_provider_timeout(provider.request_timeout_ms, warnings);
    provider
}

fn sanitize_optional_provider_string<F>(
    value: Option<String>,
    field_name: &str,
    warnings: &mut Vec<String>,
    is_valid: F,
) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let value = value?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        warnings.push(format!("{field_name} was empty and was ignored"));
        return None;
    }
    if !is_valid(trimmed) {
        warnings.push(format!(
            "{field_name} was rejected because it may contain credentials or unsupported URL parts"
        ));
        return None;
    }
    if trimmed != value {
        warnings.push(format!(
            "{field_name} had surrounding whitespace and was normalized"
        ));
    }
    Some(trimmed.to_string())
}

fn sanitize_provider_timeout(timeout_ms: Option<u64>, warnings: &mut Vec<String>) -> Option<u64> {
    let timeout_ms = timeout_ms?;
    if (MIN_PROVIDER_TIMEOUT_MS..=MAX_PROVIDER_TIMEOUT_MS).contains(&timeout_ms) {
        return Some(timeout_ms);
    }
    warnings.push(format!(
        "provider.request_timeout_ms `{timeout_ms}` is outside the supported {MIN_PROVIDER_TIMEOUT_MS}-{MAX_PROVIDER_TIMEOUT_MS} range and was ignored"
    ));
    None
}

fn is_valid_provider_base_url(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.contains('?') || value.contains('#') {
        return false;
    }
    let Some((scheme, after_scheme)) = value.split_once("://") else {
        return false;
    };
    if scheme != "https" && scheme != "http" {
        return false;
    }
    let authority = after_scheme.split('/').next().unwrap_or(after_scheme);
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    if scheme == "https" {
        return true;
    }
    is_loopback_authority(authority)
}

fn is_loopback_authority(authority: &str) -> bool {
    let host = authority
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or(authority)
        .to_ascii_lowercase();
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}

fn sanitize_refinement_quality(
    quality: RefinementQuality,
    warnings: &mut Vec<String>,
) -> RefinementQuality {
    if quality == RefinementQuality::BestQuality {
        return quality;
    }

    warnings.push(format!(
        "refinement_quality `{:?}` is a legacy profile and was upgraded to `BestQuality`",
        quality
    ));
    RefinementQuality::BestQuality
}

fn sanitize_silence_gate_level(level: u8, warnings: &mut Vec<String>) -> u8 {
    if (1..=5).contains(&level) {
        return level;
    }

    warnings.push(format!(
        "silence_gate_level `{level}` is outside the supported 1-5 range and was reset to `{}`",
        RuntimeSettings::default().silence_gate_level
    ));
    RuntimeSettings::default().silence_gate_level
}

fn sanitize_shortcut(
    mut shortcut: Shortcut,
    default_shortcut: &Shortcut,
    field_name: &str,
    warnings: &mut Vec<String>,
) -> Shortcut {
    let trimmed_key = shortcut.key.trim();
    if trimmed_key.is_empty() {
        warnings.push(format!(
            "{field_name}.key was empty and was reset to `{}`",
            default_shortcut.key
        ));
        shortcut.key = default_shortcut.key.clone();
    } else if !is_supported_shortcut_key(trimmed_key) {
        warnings.push(format!(
            "{field_name}.key `{}` is not supported by the current Windows prototype and was reset to `{}`",
            shortcut.key, default_shortcut.key
        ));
        shortcut.key = default_shortcut.key.clone();
    } else {
        let canonical_key = canonicalize_shortcut_key(trimmed_key);
        if canonical_key != shortcut.key {
            warnings.push(format!(
                "{field_name}.key was normalized to `{}`",
                canonical_key
            ));
            shortcut.key = canonical_key;
        }
    }

    let original_len = shortcut.modifiers.len();
    let mut deduped = Vec::new();
    for modifier in shortcut.modifiers {
        if !deduped.contains(&modifier) {
            deduped.push(modifier);
        }
    }
    if deduped.len() != original_len {
        warnings.push(format!(
            "{field_name}.modifiers contained duplicates and were deduplicated"
        ));
    }
    let original_order = deduped.clone();
    deduped.sort_by_key(shortcut_modifier_sort_key);
    if deduped != original_order {
        warnings.push(format!(
            "{field_name}.modifiers were normalized to the canonical order `{}`",
            format_shortcut(&Shortcut {
                modifiers: deduped.clone(),
                key: shortcut.key.clone(),
            })
        ));
    }
    if deduped.is_empty() {
        warnings.push(format!(
            "{field_name}.modifiers was empty and was reset to `{}` because the current Windows prototype requires at least one modifier",
            format_shortcut(default_shortcut)
        ));
        shortcut.modifiers = default_shortcut.modifiers.clone();
    } else {
        shortcut.modifiers = deduped;
    }

    shortcut
}

fn sanitize_wake_phrase(
    mut wake_phrase: WakePhraseConfig,
    warnings: &mut Vec<String>,
) -> WakePhraseConfig {
    let normalized_phrase = wake_phrase.phrase.trim();
    if wake_phrase.enabled && normalized_phrase.is_empty() {
        let default_phrase = RuntimeSettings::default().wake_phrase.phrase;
        warnings.push(format!(
            "wake_phrase.phrase was empty while wake phrase mode was enabled, so it was reset to `{default_phrase}`"
        ));
        wake_phrase.phrase = default_phrase;
    } else if normalized_phrase != wake_phrase.phrase {
        warnings.push(format!(
            "wake_phrase.phrase had surrounding whitespace and was normalized to `{}`",
            normalized_phrase
        ));
        wake_phrase.phrase = normalized_phrase.to_string();
    }

    wake_phrase
}

fn append_shortcut_advisories(settings: &RuntimeSettings, warnings: &mut Vec<String>) {
    if settings.dictation_shortcut == settings.edit_shortcut {
        if let Some(warning) =
            shortcut_compatibility_warning(&settings.dictation_shortcut, "shared primary shortcut")
        {
            warnings.push(warning);
        }
        return;
    }

    if let Some(warning) =
        shortcut_compatibility_warning(&settings.dictation_shortcut, "dictation_shortcut")
    {
        warnings.push(warning);
    }
    if let Some(warning) = shortcut_compatibility_warning(&settings.edit_shortcut, "edit_shortcut")
    {
        warnings.push(warning);
    }
}

fn shortcut_compatibility_warning(shortcut: &Shortcut, label: &str) -> Option<String> {
    if shortcut.modifiers.len() == 1 {
        let modifier = shortcut.modifiers[0].clone();
        let warning = match modifier {
            KeyModifier::Alt => Some(
                "may collide with Windows or app menu accelerators because it uses Alt as the only modifier"
                    .to_string(),
            ),
            KeyModifier::Shift => Some(
                "may trigger during normal typing because it uses Shift as the only modifier"
                    .to_string(),
            ),
            KeyModifier::Meta => Some(
                "may collide with Windows shell shortcuts because it uses Meta as the only modifier"
                    .to_string(),
            ),
            KeyModifier::Control => match shortcut.key.as_str() {
                "C" => Some(
                    "may hijack Copy in many apps, and selected-text capture also relies on synthetic Ctrl+C"
                        .to_string(),
                ),
                "F" => Some(
                    "may hijack Find in many apps and browsers because it matches the standard Ctrl+F command"
                        .to_string(),
                ),
                "L" => Some(
                    "may hijack location or focus shortcuts in browsers and shells because it matches the standard Ctrl+L command"
                        .to_string(),
                ),
                "N" => Some(
                    "may hijack New in many apps because it matches the standard Ctrl+N command"
                        .to_string(),
                ),
                "O" => Some(
                    "may hijack Open in many apps because it matches the standard Ctrl+O command"
                        .to_string(),
                ),
                "P" => Some(
                    "may hijack Print in many apps and browsers because it matches the standard Ctrl+P command"
                        .to_string(),
                ),
                "R" => Some(
                    "may hijack Refresh or reload in many apps and browsers because it matches the standard Ctrl+R command"
                        .to_string(),
                ),
                "S" => Some(
                    "may hijack Save in many apps because it matches the standard Ctrl+S command"
                        .to_string(),
                ),
                "T" => Some(
                    "may hijack New Tab in many browsers and terminals because it matches the standard Ctrl+T command"
                        .to_string(),
                ),
                "V" => Some(
                    "may hijack Paste in many apps, and clipboard fallback commit also relies on synthetic Ctrl+V"
                        .to_string(),
                ),
                "W" => Some(
                    "may hijack Close Tab or Close Window in many apps and browsers because it matches the standard Ctrl+W command"
                        .to_string(),
                ),
                "X" => Some(
                    "may hijack Cut in many apps because it matches the standard Ctrl+X edit chord"
                        .to_string(),
                ),
                "Y" => Some(
                    "may hijack Redo in many apps because it matches the standard Ctrl+Y command"
                        .to_string(),
                ),
                "Z" => Some(
                    "may hijack Undo in many apps because it matches the standard Ctrl+Z command"
                        .to_string(),
                ),
                "A" => Some(
                    "may hijack Select All in many apps because it matches the standard Ctrl+A edit chord"
                        .to_string(),
                ),
                _ => None,
            },
        };

        if let Some(reason) = warning {
            return Some(format!("{label} `{}` {reason}", format_shortcut(shortcut)));
        }
    }

    None
}

fn shortcut_modifier_sort_key(modifier: &KeyModifier) -> u8 {
    match modifier {
        KeyModifier::Control => 0,
        KeyModifier::Alt => 1,
        KeyModifier::Shift => 2,
        KeyModifier::Meta => 3,
    }
}

fn format_shortcut(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();
    for modifier in &shortcut.modifiers {
        let label = match modifier {
            KeyModifier::Control => "Ctrl",
            KeyModifier::Alt => "Alt",
            KeyModifier::Shift => "Shift",
            KeyModifier::Meta => "Meta",
        };
        parts.push(label.to_string());
    }
    parts.push(shortcut.key.clone());
    parts.join("+")
}

fn is_supported_shortcut_key(key: &str) -> bool {
    if key.eq_ignore_ascii_case("Space") {
        return true;
    }

    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => ch.is_ascii_alphanumeric(),
        _ => false,
    }
}

fn canonicalize_shortcut_key(key: &str) -> String {
    if key.eq_ignore_ascii_case("Space") {
        return "Space".to_string();
    }

    if key.len() == 1 {
        return key.to_ascii_uppercase();
    }

    key.to_string()
}

fn strip_utf8_bom(raw: &str) -> &str {
    raw.strip_prefix('\u{feff}').unwrap_or(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_protocol::KeyModifier;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn uses_override_path_when_present() {
        let resolved = resolve_settings_path_from_inputs(
            Some(r"D:\custom\voiceflow.json"),
            Some(r"C:\Users\Alice\AppData\Local"),
            Path::new(r"D:\workspace"),
        );

        assert_eq!(resolved, PathBuf::from(r"D:\custom\voiceflow.json"));
    }

    #[test]
    fn uses_local_app_data_when_override_is_missing() {
        let resolved = resolve_settings_path_from_inputs(
            None,
            Some(r"C:\Users\Alice\AppData\Local"),
            Path::new(r"D:\workspace"),
        );

        assert_eq!(
            resolved,
            PathBuf::from(r"C:\Users\Alice\AppData\Local")
                .join("VoiceFlow Speech Input")
                .join("settings.json")
        );
    }

    #[test]
    fn falls_back_to_repo_local_file_without_local_app_data() {
        let resolved = resolve_settings_path_from_inputs(None, None, Path::new(r"D:\workspace"));

        assert_eq!(
            resolved,
            PathBuf::from(r"D:\workspace").join("voiceflow-settings.json")
        );
    }

    #[test]
    fn loads_defaults_when_settings_file_is_missing() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir()
            .join("voiceflow-settings-tests")
            .join(format!("missing-{unique_id}.json"));

        let loaded = load_runtime_settings_from_path(&path)
            .expect("missing settings should fall back to defaults");

        assert_eq!(loaded.settings, RuntimeSettings::default());
        assert_eq!(loaded.path, path);
        assert_eq!(loaded.source, RuntimeSettingsSource::Defaults);
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn sanitizes_invalid_shortcut_keys_and_duplicate_modifiers() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control, KeyModifier::Control, KeyModifier::Alt],
                key: "  Escape  ".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(settings.dictation_shortcut.key, "Space");
        assert_eq!(
            settings.dictation_shortcut.modifiers,
            vec![KeyModifier::Control, KeyModifier::Alt]
        );
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("dictation_shortcut.key"));
        assert!(warnings[1].contains("dictation_shortcut.modifiers"));
    }

    #[test]
    fn normalizes_shortcut_key_casing_and_whitespace() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "  r  ".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(settings.dictation_shortcut.key, "R");
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("dictation_shortcut.key"));
        assert!(warnings[0].contains("`R`"));
        assert!(warnings[1].contains("Ctrl+R"));
    }

    #[test]
    fn resets_modifierless_shortcut_to_default_modifier() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![],
                key: "R".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(
            settings.dictation_shortcut.modifiers,
            RuntimeSettings::default().dictation_shortcut.modifiers
        );
        assert_eq!(warnings.len(), 2);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("dictation_shortcut.modifiers"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("requires at least one modifier"))
        );
        assert!(warnings.iter().any(|warning| warning.contains("Ctrl+R")));
    }

    #[test]
    fn normalizes_modifier_order_for_shortcuts() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Shift, KeyModifier::Control, KeyModifier::Alt],
                key: "R".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(
            settings.dictation_shortcut.modifiers,
            vec![KeyModifier::Control, KeyModifier::Alt, KeyModifier::Shift]
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("dictation_shortcut.modifiers"));
        assert!(warnings[0].contains("Ctrl+Alt+Shift+R"));
    }

    #[test]
    fn normalizes_modifier_order_when_persisting_runtime_updates() {
        let temp_dir = env::temp_dir().join(format!(
            "voiceflow-settings-update-order-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let path = temp_dir.join("settings.json");

        let updated = update_runtime_settings_at_path(
            &path,
            RuntimeSettingsUpdate {
                primary_shortcut_modifiers: Some(vec![
                    KeyModifier::Shift,
                    KeyModifier::Control,
                    KeyModifier::Alt,
                ]),
                primary_shortcut_key: Some("R".to_string()),
                ..RuntimeSettingsUpdate::default()
            },
        )
        .expect("settings update should succeed");

        let loaded = load_runtime_settings_from_path(&path).expect("settings should load");
        assert!(
            updated
                .warnings
                .iter()
                .any(|warning| warning.contains("Ctrl+Alt+Shift+R"))
        );
        assert_eq!(
            loaded.settings.dictation_shortcut.modifiers,
            vec![KeyModifier::Control, KeyModifier::Alt, KeyModifier::Shift]
        );
        assert_eq!(
            loaded.settings.edit_shortcut.modifiers,
            vec![KeyModifier::Control, KeyModifier::Alt, KeyModifier::Shift]
        );
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn sanitizes_out_of_range_silence_gate_level() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            silence_gate_level: 9,
            ..RuntimeSettings::default()
        });

        assert_eq!(settings.silence_gate_level, 2);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("silence_gate_level"));
    }

    #[test]
    fn sanitizes_empty_enabled_wake_phrase() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            wake_phrase: WakePhraseConfig {
                enabled: true,
                phrase: "   ".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(settings.wake_phrase.phrase, "Voice Flow");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("wake_phrase.phrase"));
    }

    #[test]
    fn warns_when_shared_primary_shortcut_uses_alt_as_only_modifier() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Alt],
                key: "S".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Alt],
                key: "S".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(settings.dictation_shortcut.key, "S");
        assert_eq!(settings.edit_shortcut.key, "S");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("shared primary shortcut"));
        assert!(warnings[0].contains("Alt+S"));
    }

    #[test]
    fn warns_when_shared_primary_shortcut_uses_shift_as_only_modifier() {
        let (_, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Shift],
                key: "R".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Shift],
                key: "R".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("shared primary shortcut"));
        assert!(warnings[0].contains("Shift+R"));
        assert!(warnings[0].contains("normal typing"));
    }

    #[test]
    fn warns_when_shared_primary_shortcut_uses_meta_as_only_modifier() {
        let (_, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Meta],
                key: "R".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Meta],
                key: "R".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("shared primary shortcut"));
        assert!(warnings[0].contains("Meta+R"));
        assert!(warnings[0].contains("Windows shell shortcuts"));
    }

    #[test]
    fn warns_when_shared_primary_shortcut_uses_ctrl_c() {
        let (_, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "C".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "C".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("shared primary shortcut"));
        assert!(warnings[0].contains("Ctrl+C"));
        assert!(warnings[0].contains("selected-text capture"));
    }

    #[test]
    fn warns_when_shared_primary_shortcut_uses_ctrl_v() {
        let (_, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "V".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "V".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("shared primary shortcut"));
        assert!(warnings[0].contains("Ctrl+V"));
        assert!(warnings[0].contains("clipboard fallback commit"));
    }

    #[test]
    fn warns_when_shared_primary_shortcut_uses_ctrl_z() {
        let (_, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "Z".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "Z".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("shared primary shortcut"));
        assert!(warnings[0].contains("Ctrl+Z"));
        assert!(warnings[0].contains("Undo"));
    }

    #[test]
    fn warns_when_shared_primary_shortcut_uses_ctrl_s() {
        let (_, warnings) = sanitize_runtime_settings(RuntimeSettings {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "S".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "S".to_string(),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("shared primary shortcut"));
        assert!(warnings[0].contains("Ctrl+S"));
        assert!(warnings[0].contains("Save"));
    }

    #[test]
    fn strips_utf8_bom_before_parsing_json() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir()
            .join("voiceflow-settings-tests")
            .join(format!("bom-{unique_id}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temp settings dir should be creatable");
        }

        let encoded = format!(
            "\u{feff}{}",
            serde_json::to_string(&RuntimeSettings::default())
                .expect("default runtime settings should encode")
        );
        fs::write(&path, encoded).expect("bom-prefixed settings should write");

        let loaded = load_runtime_settings_from_path(&path)
            .expect("bom-prefixed settings should still parse");

        assert_eq!(loaded.settings, RuntimeSettings::default());
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn sanitizes_non_secret_provider_settings() {
        let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
            provider: ProviderSettings {
                preset: ProviderPreset::VolcengineArk,
                base_url: Some("  https://dashscope.example/compatible-mode/v1  ".to_string()),
                active_model: Some("  custom-best  ".to_string()),
                request_timeout_ms: Some(250),
            },
            ..RuntimeSettings::default()
        });

        assert_eq!(settings.provider.preset, ProviderPreset::VolcengineArk);
        assert_eq!(
            settings.provider.base_url.as_deref(),
            Some("https://dashscope.example/compatible-mode/v1")
        );
        assert_eq!(settings.provider.request_timeout_ms, None);
        assert_eq!(
            settings.provider.active_model.as_deref(),
            Some("custom-best")
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("provider.base_url"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("provider.request_timeout_ms"))
        );
    }

    #[test]
    fn unrelated_runtime_settings_update_preserves_provider_settings() {
        let temp_dir = env::temp_dir().join(format!(
            "voiceflow-settings-provider-preserve-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let path = temp_dir.join("settings.json");
        let initial_settings = RuntimeSettings {
            provider: ProviderSettings {
                preset: ProviderPreset::CustomOpenAiCompatible,
                base_url: Some("https://provider.example/v1".to_string()),
                active_model: Some("company-model".to_string()),
                request_timeout_ms: Some(4_500),
            },
            ..RuntimeSettings::default()
        };
        fs::write(
            &path,
            serde_json::to_string_pretty(&initial_settings).expect("settings should encode"),
        )
        .expect("settings should be written");

        let loaded = update_runtime_settings_at_path(
            &path,
            RuntimeSettingsUpdate {
                system_language: Some(SystemLanguage::Chinese),
                history_retention: Some(HistoryRetention::Latest500),
                audio_feedback_enabled: Some(false),
                ..RuntimeSettingsUpdate::default()
            },
        )
        .expect("settings update should succeed");

        assert_eq!(loaded.settings.provider, initial_settings.provider);
    }

    #[test]
    fn provider_update_persists_non_secret_fields() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir()
            .join("voiceflow-settings-tests")
            .join(format!("provider-update-{unique_id}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temp settings dir should be creatable");
        }

        let loaded = update_runtime_settings_at_path(
            &path,
            RuntimeSettingsUpdate {
                provider: Some(ProviderSettingsUpdate {
                    preset: Some(ProviderPreset::VolcengineArk),
                    base_url: Some(Some(" https://ark.example/api/v3 ".to_string())),
                    active_model: Some(Some(" company-gateway-model ".to_string())),
                    request_timeout_ms: Some(Some(15_000)),
                }),
                ..RuntimeSettingsUpdate::default()
            },
        )
        .expect("provider settings update should persist");

        assert_eq!(
            loaded.settings.provider.preset,
            ProviderPreset::VolcengineArk
        );
        assert_eq!(
            loaded.settings.provider.base_url.as_deref(),
            Some("https://ark.example/api/v3")
        );
        assert_eq!(
            loaded.settings.provider.active_model.as_deref(),
            Some("company-gateway-model")
        );
        assert_eq!(loaded.settings.provider.request_timeout_ms, Some(15_000));

        let round_tripped = load_runtime_settings_from_path(&path).expect("settings should load");
        assert_eq!(round_tripped.settings.provider, loaded.settings.provider);
    }

    #[test]
    fn provider_update_normalizes_blank_optional_fields_to_none() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir()
            .join("voiceflow-settings-tests")
            .join(format!("provider-blank-{unique_id}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temp settings dir should be creatable");
        }

        let loaded = update_runtime_settings_at_path(
            &path,
            RuntimeSettingsUpdate {
                provider: Some(ProviderSettingsUpdate {
                    preset: Some(ProviderPreset::CustomOpenAiCompatible),
                    base_url: Some(Some("   ".to_string())),
                    active_model: Some(None),
                    request_timeout_ms: Some(None),
                }),
                ..RuntimeSettingsUpdate::default()
            },
        )
        .expect("blank provider optionals should clear saved values");

        assert_eq!(
            loaded.settings.provider.preset,
            ProviderPreset::CustomOpenAiCompatible
        );
        assert_eq!(loaded.settings.provider.base_url, None);
        assert_eq!(loaded.settings.provider.active_model, None);
        assert_eq!(loaded.settings.provider.request_timeout_ms, None);
    }

    #[test]
    fn provider_update_deserializes_null_fields_as_clear_operations() {
        let update: RuntimeSettingsUpdate = serde_json::from_str(
            r#"{
                "provider": {
                    "base_url": null,
                    "active_model": null,
                    "request_timeout_ms": null
                }
            }"#,
        )
        .expect("provider update payload should decode");

        let provider = update.provider.expect("provider update should be present");
        assert_eq!(provider.base_url, Some(None));
        assert_eq!(provider.active_model, Some(None));
        assert_eq!(provider.request_timeout_ms, Some(None));
    }

    #[test]
    fn provider_update_rejects_invalid_url_and_timeout() {
        let path = env::temp_dir().join("voiceflow-settings-provider-invalid.json");
        let url_error = update_runtime_settings_at_path(
            &path,
            RuntimeSettingsUpdate {
                provider: Some(ProviderSettingsUpdate {
                    base_url: Some(Some("https://user:pass@example.test/v1".to_string())),
                    ..ProviderSettingsUpdate::default()
                }),
                ..RuntimeSettingsUpdate::default()
            },
        )
        .expect_err("credential-bearing URL should be rejected");
        assert!(url_error.contains("provider.base_url"));

        let timeout_error = update_runtime_settings_at_path(
            &path,
            RuntimeSettingsUpdate {
                provider: Some(ProviderSettingsUpdate {
                    request_timeout_ms: Some(Some(999)),
                    ..ProviderSettingsUpdate::default()
                }),
                ..RuntimeSettingsUpdate::default()
            },
        )
        .expect_err("too-small timeout should be rejected");
        assert!(timeout_error.contains("provider.request_timeout_ms"));
    }

    #[test]
    fn updates_and_persists_selected_runtime_settings() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = env::temp_dir()
            .join("voiceflow-settings-tests")
            .join(format!("update-{unique_id}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temp settings dir should be creatable");
        }

        let loaded = update_runtime_settings_at_path(
            &path,
            RuntimeSettingsUpdate {
                primary_shortcut_modifiers: Some(vec![KeyModifier::Control, KeyModifier::Shift]),
                primary_shortcut_key: Some("R".to_string()),
                shortcut_mode: Some(ShortcutMode::PushToTalk),
                refinement_quality: Some(RefinementQuality::BestQuality),
                silence_gate_level: Some(4),
                diagnostics_verbosity: Some(DiagnosticsVerbosity::Verbose),
                system_language: Some(SystemLanguage::Chinese),
                ui_style: Some(UiStyle::Light),
                audio_feedback_enabled: Some(false),
                wake_phrase_enabled: Some(false),
                wake_phrase_phrase: Some("  Hey Team Flow  ".to_string()),
                history_retention: Some(HistoryRetention::Last30Days),
                provider: None,
            },
        )
        .expect("settings update should persist");

        assert_eq!(loaded.source, RuntimeSettingsSource::File);
        assert_eq!(
            loaded.settings.dictation_shortcut,
            Shortcut {
                modifiers: vec![KeyModifier::Control, KeyModifier::Shift],
                key: "R".to_string()
            }
        );
        assert_eq!(
            loaded.settings.edit_shortcut,
            loaded.settings.dictation_shortcut
        );
        assert_eq!(loaded.settings.shortcut_mode, ShortcutMode::PushToTalk);
        assert_eq!(
            loaded.settings.refinement_quality,
            RefinementQuality::BestQuality
        );
        assert_eq!(loaded.settings.silence_gate_level, 4);
        assert_eq!(
            loaded.settings.diagnostics_verbosity,
            DiagnosticsVerbosity::Verbose
        );
        assert_eq!(loaded.settings.system_language, SystemLanguage::Chinese);
        assert_eq!(loaded.settings.ui_style, UiStyle::Light);
        assert!(!loaded.settings.audio_feedback_enabled);
        assert!(!loaded.settings.wake_phrase.enabled);
        assert_eq!(loaded.settings.wake_phrase.phrase, "Hey Team Flow");
        assert_eq!(
            loaded.settings.history_retention,
            HistoryRetention::Last30Days
        );
        assert_eq!(loaded.warnings.len(), 1);
        assert!(loaded.warnings[0].contains("wake_phrase.phrase"));

        let reloaded =
            load_runtime_settings_from_path(&path).expect("persisted settings should reload");
        assert_eq!(reloaded.settings, loaded.settings);
        assert!(reloaded.warnings.is_empty());
        let temp_prefix = format!(
            ".{}.",
            path.file_name()
                .and_then(|name| name.to_str())
                .expect("test path should have a UTF-8 file name")
        );
        assert!(
            fs::read_dir(path.parent().unwrap())
                .expect("settings directory should be readable")
                .filter_map(Result::ok)
                .all(|entry| !entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&temp_prefix)),
            "atomic settings save should not leave a temporary file"
        );
    }

    #[test]
    fn upgrades_legacy_refinement_profiles_to_best_quality() {
        for legacy_quality in [
            RefinementQuality::Fast,
            RefinementQuality::FastPlus,
            RefinementQuality::Balanced,
        ] {
            let (settings, warnings) = sanitize_runtime_settings(RuntimeSettings {
                refinement_quality: legacy_quality,
                ..RuntimeSettings::default()
            });

            assert_eq!(settings.refinement_quality, RefinementQuality::BestQuality);
            assert_eq!(warnings.len(), 1);
            assert!(warnings[0].contains("legacy profile"));
            assert!(warnings[0].contains("BestQuality"));
        }
    }

    #[test]
    fn rejects_empty_runtime_settings_update() {
        let path = env::temp_dir()
            .join("voiceflow-settings-tests")
            .join("empty-update.json");

        let error = update_runtime_settings_at_path(&path, RuntimeSettingsUpdate::default())
            .expect_err("empty updates should be rejected");

        assert!(error.contains("at least one runtime setting update"));
    }

    #[test]
    fn saving_stricter_history_retention_prunes_immediately() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let temp_dir =
            env::temp_dir().join(format!("voiceflow-settings-history-prune-{unique_id}"));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let settings_path = temp_dir.join("settings.json");
        let mut settings = RuntimeSettings::default();
        settings.history_retention = HistoryRetention::Unlimited;
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&settings).expect("settings should encode"),
        )
        .expect("settings should write");

        let entries = (0..105)
            .map(|id| {
                serde_json::json!({
                    "entry_id": format!("report:session:{id}"),
                    "source_run_id": "report",
                    "session_id": id,
                    "timestamp_epoch_ms": 1,
                    "mode": "Dictation",
                    "summary": "Dictation inserted",
                    "text": format!("entry {id}"),
                    "redacted": false,
                    "word_count": 2,
                    "char_count": 7,
                    "refinement_profile_label": null,
                    "model_name": null,
                    "model_code": null,
                    "provider_attempted": null,
                    "provider_succeeded": null,
                    "deterministic_fallback_used": null,
                    "fallback_reason": null,
                    "selected_text_action": null,
                    "selected_text_action_label": null,
                    "selected_text_char_count": null,
                    "output_char_count": null
                })
            })
            .collect::<Vec<_>>();
        let keys = (0..105)
            .map(|id| format!("report:session:{id}"))
            .collect::<Vec<_>>();
        let ledger = serde_json::json!({
            "schema_version": 1,
            "updated_at_epoch_ms": 1,
            "applied_session_keys": keys,
            "entries": entries
        });
        let ledger_path = temp_dir.join("history-ledger.json");
        fs::write(
            &ledger_path,
            serde_json::to_string_pretty(&ledger).expect("ledger should encode"),
        )
        .expect("ledger should write");

        update_runtime_settings_at_path(
            &settings_path,
            RuntimeSettingsUpdate {
                history_retention: Some(HistoryRetention::Latest100),
                ..RuntimeSettingsUpdate::default()
            },
        )
        .expect("settings update should prune history");

        let persisted: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&ledger_path).expect("ledger should read"))
                .expect("ledger should decode");
        assert_eq!(
            persisted["entries"]
                .as_array()
                .expect("entries should be an array")
                .len(),
            100
        );
        assert_eq!(
            persisted["applied_session_keys"]
                .as_array()
                .expect("keys should be an array")
                .len(),
            100
        );
        let _ = fs::remove_dir_all(temp_dir);
    }
}
