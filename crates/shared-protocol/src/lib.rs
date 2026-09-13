use std::{env, fmt};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    Dictation,
    SelectedTextEdit,
    WakePhraseIntent,
    InstructedDictation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Arming,
    Recording,
    Recognizing,
    ModeRouting,
    Executing,
    ReadyToCommit,
    Committing,
    Committed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionFailurePhase {
    Arming,
    Recording,
    Recognizing,
    Executing,
    Committing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerMode {
    GlobalShortcut,
    EditShortcut,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShortcutMode {
    Toggle,
    PushToTalk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyModifier {
    Control,
    Alt,
    Shift,
    Meta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub modifiers: Vec<KeyModifier>,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefinementQuality {
    Fast,
    FastPlus,
    Balanced,
    BestQuality,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefinementModelProfile {
    pub display_label: String,
    pub model_name: String,
    pub model_code: String,
    pub enable_thinking: Option<bool>,
}

pub fn refinement_model_profile_for_quality(quality: &RefinementQuality) -> RefinementModelProfile {
    refinement_model_profile_for_quality_with_lookup(quality, |key| env::var(key).ok())
}

pub fn builtin_refinement_model_profile_for_quality(
    quality: &RefinementQuality,
) -> RefinementModelProfile {
    refinement_model_profile_for_quality_with_lookup(quality, |_| None)
}

pub fn refinement_model_profile_for_quality_with_lookup<F>(
    quality: &RefinementQuality,
    lookup: F,
) -> RefinementModelProfile
where
    F: Fn(&str) -> Option<String>,
{
    let (mut profile, model_override_env, thinking_override_env) = match quality {
        RefinementQuality::Fast => RefinementModelProfile {
            display_label: "Fast / 极速".to_string(),
            model_name: "Qwen Turbo".to_string(),
            model_code: "qwen-turbo".to_string(),
            enable_thinking: None,
        }
        .with_override_keys("VOICEFLOW_MODEL_FAST", "VOICEFLOW_MODEL_FAST_THINKING"),
        RefinementQuality::FastPlus | RefinementQuality::Balanced => RefinementModelProfile {
            display_label: "Balanced / 均衡".to_string(),
            model_name: "Qwen 3.5 Flash".to_string(),
            model_code: "qwen3.5-flash".to_string(),
            enable_thinking: Some(false),
        }
        .with_override_keys(
            "VOICEFLOW_MODEL_BALANCED",
            "VOICEFLOW_MODEL_BALANCED_THINKING",
        ),
        RefinementQuality::BestQuality => RefinementModelProfile {
            display_label: "Best Quality / 高质量".to_string(),
            model_name: "DeepSeek V4 Flash".to_string(),
            model_code: "deepseek-v4-flash".to_string(),
            enable_thinking: Some(false),
        }
        .with_override_keys("VOICEFLOW_MODEL_BEST", "VOICEFLOW_MODEL_BEST_THINKING"),
    };

    if let Some(model_code) = lookup(model_override_env)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        profile.model_code = model_code;
    }

    if let Some(enable_thinking) =
        lookup(thinking_override_env).and_then(|value| parse_model_thinking_override(&value))
    {
        profile.enable_thinking = Some(enable_thinking);
    }

    profile
}

trait RefinementModelProfileOverrideKeys {
    fn with_override_keys(
        self,
        model_override_env: &'static str,
        thinking_override_env: &'static str,
    ) -> (Self, &'static str, &'static str)
    where
        Self: Sized;
}

impl RefinementModelProfileOverrideKeys for RefinementModelProfile {
    fn with_override_keys(
        self,
        model_override_env: &'static str,
        thinking_override_env: &'static str,
    ) -> (Self, &'static str, &'static str) {
        (self, model_override_env, thinking_override_env)
    }
}

pub fn parse_model_thinking_override(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub fn refinement_model_profile_override_keys_for_quality(
    quality: &RefinementQuality,
) -> (&'static str, &'static str) {
    match quality {
        RefinementQuality::Fast => ("VOICEFLOW_MODEL_FAST", "VOICEFLOW_MODEL_FAST_THINKING"),
        RefinementQuality::FastPlus | RefinementQuality::Balanced => (
            "VOICEFLOW_MODEL_BALANCED",
            "VOICEFLOW_MODEL_BALANCED_THINKING",
        ),
        RefinementQuality::BestQuality => ("VOICEFLOW_MODEL_BEST", "VOICEFLOW_MODEL_BEST_THINKING"),
    }
}

fn default_refinement_profile() -> RefinementModelProfile {
    refinement_model_profile_for_quality(&RefinementQuality::BestQuality)
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderPreset {
    #[default]
    Bailian,
    VolcengineArk,
    TencentHunyuan,
    CustomOpenAiCompatible,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSettings {
    #[serde(default)]
    pub preset: ProviderPreset,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub active_model: Option<String>,
    #[serde(default)]
    pub request_timeout_ms: Option<u64>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RuntimeProviderConfig {
    pub preset: ProviderPreset,
    pub api_key: String,
    pub base_url: String,
    pub request_timeout_ms: u64,
    pub supports_dashscope_enable_thinking: bool,
    pub provider_key_source: Option<String>,
}

impl std::fmt::Debug for RuntimeProviderConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeProviderConfig")
            .field("preset", &self.preset)
            .field("api_key", &"<redacted>")
            .field("base_url", &self.base_url)
            .field("request_timeout_ms", &self.request_timeout_ms)
            .field(
                "supports_dashscope_enable_thinking",
                &self.supports_dashscope_enable_thinking,
            )
            .field("provider_key_source", &self.provider_key_source)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakePhraseConfig {
    pub enabled: bool,
    pub phrase: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticsVerbosity {
    Standard,
    Verbose,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemLanguage {
    English,
    Chinese,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiStyle {
    Dark,
    Light,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryRetention {
    #[default]
    #[serde(rename = "latest_100")]
    Latest100,
    #[serde(rename = "latest_500")]
    Latest500,
    #[serde(rename = "latest_1000")]
    Latest1000,
    #[serde(rename = "last_7_days")]
    Last7Days,
    #[serde(rename = "last_30_days")]
    Last30Days,
    #[serde(rename = "unlimited")]
    Unlimited,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSettings {
    pub dictation_shortcut: Shortcut,
    pub edit_shortcut: Shortcut,
    pub shortcut_mode: ShortcutMode,
    pub refinement_quality: RefinementQuality,
    #[serde(default = "default_silence_gate_level")]
    pub silence_gate_level: u8,
    pub audio_feedback_enabled: bool,
    pub wake_phrase: WakePhraseConfig,
    pub diagnostics_verbosity: DiagnosticsVerbosity,
    #[serde(default = "default_system_language")]
    pub system_language: SystemLanguage,
    #[serde(default = "default_ui_style")]
    pub ui_style: UiStyle,
    #[serde(default)]
    pub history_retention: HistoryRetention,
    #[serde(default)]
    pub provider: ProviderSettings,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            dictation_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "Space".to_string(),
            },
            edit_shortcut: Shortcut {
                modifiers: vec![KeyModifier::Control],
                key: "Space".to_string(),
            },
            shortcut_mode: ShortcutMode::Toggle,
            refinement_quality: RefinementQuality::BestQuality,
            silence_gate_level: 2,
            audio_feedback_enabled: true,
            wake_phrase: WakePhraseConfig {
                enabled: true,
                phrase: "Voice Flow".to_string(),
            },
            diagnostics_verbosity: DiagnosticsVerbosity::Standard,
            system_language: SystemLanguage::English,
            ui_style: UiStyle::Dark,
            history_retention: HistoryRetention::Latest100,
            provider: ProviderSettings::default(),
        }
    }
}

fn default_silence_gate_level() -> u8 {
    2
}

fn default_system_language() -> SystemLanguage {
    SystemLanguage::English
}

fn default_ui_style() -> UiStyle {
    UiStyle::Dark
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteName {
    LocalAsrOnly,
    LocalAsrWithRefine,
    CloudAsrWithRefine,
    InstructedDictation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsrProvider {
    Local,
    Cloud,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefineProvider {
    Llm,
}

pub const DICTATION_LOCAL_THRESHOLD: u64 = 15;
pub const DICTATION_STRUCTURED_THRESHOLD: u64 = 60;

pub fn count_text_units(text: &str) -> u64 {
    let mut count = 0_u64;
    let mut in_latin_word = false;
    let mut in_number = false;

    for character in text.chars() {
        if is_han_character(character) {
            count += 1;
            in_latin_word = false;
            in_number = false;
        } else if is_latin_character(character) {
            if !in_latin_word {
                count += 1;
            }
            in_latin_word = true;
            in_number = false;
        } else if character.is_numeric() {
            if !in_number {
                count += 1;
            }
            in_number = true;
            in_latin_word = false;
        } else {
            in_latin_word = false;
            in_number = false;
        }
    }

    count
}

fn is_latin_character(character: char) -> bool {
    matches!(
        character as u32,
        0x0041..=0x005A
            | 0x0061..=0x007A
            | 0x00C0..=0x02AF
            | 0x1D00..=0x1D7F
            | 0x1D80..=0x1DBF
            | 0x1E00..=0x1EFF
            | 0xAB30..=0xAB6F
    )
}

fn is_han_character(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationRefinementMode {
    LocalOnly,
    LightCleanup,
    StructuredCleanup,
}

impl DictationRefinementMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalOnly => "local_only",
            Self::LightCleanup => "light_cleanup",
            Self::StructuredCleanup => "structured_cleanup",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationRoutingReason {
    ShortLocal,
    SelfCorrectionException,
    LengthLight,
    LengthStructured,
}

impl DictationRoutingReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShortLocal => "short_local",
            Self::SelfCorrectionException => "self_correction_exception",
            Self::LengthLight => "length_light",
            Self::LengthStructured => "length_structured",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictationRoutingDecision {
    pub text_count: u64,
    pub refinement_mode: DictationRefinementMode,
    pub routing_reason: DictationRoutingReason,
    pub self_correction_detected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationPromptProfile {
    DictationLight,
    DictationStructured,
}

impl DictationPromptProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DictationLight => "dictation_light",
            Self::DictationStructured => "dictation_structured",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptSource {
    Builtin,
    Override,
}

impl PromptSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Override => "override",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub route_name: RouteName,
    pub asr_provider: AsrProvider,
    pub refine_provider: Option<RefineProvider>,
    pub refinement_applied: bool,
    pub reason: String,
    #[serde(default)]
    pub refine_fast_path_used: bool,
    #[serde(default)]
    pub refine_fast_path_reason: String,
    #[serde(default)]
    pub cloud_refine_skipped: bool,
    #[serde(default)]
    pub dictation_routing: Option<DictationRoutingDecision>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsrLatencyMetric {
    pub name: String,
    pub value_ms: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsrDiagnostics {
    pub backend: String,
    pub worker_request_id: Option<u64>,
    pub worker_model_load_ms: Option<u32>,
    pub worker_restarted: bool,
    pub metrics: Vec<AsrLatencyMetric>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefineDiagnostics {
    pub profile_label: String,
    pub model_name: String,
    pub model_code: String,
    #[serde(default)]
    pub provider_preset: Option<String>,
    #[serde(default)]
    pub provider_key_source: Option<String>,
    pub provider_configured: bool,
    pub provider_attempted: bool,
    pub provider_succeeded: bool,
    pub deterministic_fallback_used: bool,
    pub fallback_reason: Option<String>,
    #[serde(default)]
    pub refine_total_ms: Option<u64>,
    #[serde(default)]
    pub provider_request_ms: Option<u64>,
    #[serde(default)]
    pub prompt_load_ms: Option<u64>,
    #[serde(default)]
    pub payload_build_ms: Option<u64>,
    #[serde(default)]
    pub provider_output_validation_ms: Option<u64>,
    #[serde(default)]
    pub prompt_char_count: Option<u64>,
    #[serde(default)]
    pub transcript_char_count: Option<u64>,
    #[serde(default)]
    pub guard_detected: bool,
    #[serde(default)]
    pub provider_output_rejected: bool,
    #[serde(default)]
    pub prompt_profile: Option<DictationPromptProfile>,
    #[serde(default)]
    pub prompt_source: Option<PromptSource>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineRequest {
    pub session_id: u64,
    pub requested_kind: SessionKind,
    pub captured_audio: CapturedAudio,
    pub transcript_hint: Option<String>,
    #[serde(default = "default_refinement_profile")]
    pub refinement_profile: RefinementModelProfile,
    #[serde(default)]
    pub provider_settings: ProviderSettings,
    #[serde(skip)]
    pub provider_runtime_config: Option<RuntimeProviderConfig>,
    #[serde(skip)]
    pub dictation_routing: Option<DictationRoutingDecision>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineResponse {
    pub route_decision: RouteDecision,
    pub transcript: String,
    pub final_text: String,
    pub degraded_to_asr: bool,
    pub fallback_reason: Option<String>,
    pub asr_diagnostics: Option<AsrDiagnostics>,
    pub refine_diagnostics: Option<RefineDiagnostics>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitStatus {
    NotAttempted,
    Success,
    TemporaryStub,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitTransport {
    DirectUnicodeSendInput,
    ClipboardPasteFallback,
    ClipboardSelectionReplace,
    TemporaryStub,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitResult {
    pub status: CommitStatus,
    pub transport: CommitTransport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayStatus {
    pub session_id: u64,
    pub session_kind: SessionKind,
    pub state: SessionState,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    Bootstrap,
    SessionLifecycle,
    Audio,
    Routing,
    ModeDecision,
    Commit,
    Overlay,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub category: DiagnosticCategory,
    pub session_id: Option<u64>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeOutcome {
    pub session_kind: SessionKind,
    pub text_for_commit: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstructedDictationParseOutcome {
    NoTrigger,
    Parsed(InstructedDictationParts),
    MissingContent(InstructedDictationParseFailure),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructedTriggerAlias {
    Canonical,
    Spaced,
}

impl InstructedTriggerAlias {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Spaced => "spaced",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructedDictationParts {
    pub task_text: String,
    pub parser_result: String,
    pub trigger_alias: InstructedTriggerAlias,
    pub leading_filler_removed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructedDictationParseFailure {
    pub parser_result: String,
    pub reason: String,
    pub task_char_count: usize,
    pub trigger_alias: InstructedTriggerAlias,
    pub leading_filler_removed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedTextExecutionRequest {
    pub session_id: u64,
    pub selected_text: String,
    pub instruction_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectedTextExecutionAction {
    Uppercase,
    Lowercase,
    TitleCase,
    SentenceCase,
    SnakeCase,
    KebabCase,
    CamelCase,
    PascalCase,
    ConstantCase,
    InlineCode,
    CodeBlock,
    StripCodeFence,
    MarkdownBold,
    MarkdownItalic,
    StripMarkdownEmphasis,
    WrapInQuotes,
    BulletList,
    Checklist,
    QuoteBlock,
    NumberedList,
    SortLines,
    DeduplicateLines,
    RemoveEmptyLines,
    CommaSeparated,
    PipeSeparated,
    TabSeparated,
    SemicolonSeparated,
    JsonArray,
    QuotedCsv,
    SqlInList,
    YamlList,
    YamlMapping,
    MarkdownTable,
    HeaderBlock,
    JsonObject,
    EnvBlock,
    QueryString,
    TomlTable,
    ShellExports,
    PowershellEnv,
    CurlHeaders,
    PythonDict,
    JavascriptObject,
    RubyHash,
    SqlValuesRows,
    StripListMarkers,
    SentencePerLine,
    MarkdownHeading,
    SingleParagraph,
    CleanupSpacing,
    PolishWriting,
    ConciseRewrite,
    FormalRewrite,
    BulletSummary,
    GeneralProviderEdit,
    PromptScaffold,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedTextExecutionResponse {
    pub output_text: String,
    pub action: SelectedTextExecutionAction,
    pub strategy: String,
    pub normalized_instruction: String,
    pub provider_diagnostics: Option<SelectedTextProviderDiagnostics>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedTextExecutionSummary {
    pub action: SelectedTextExecutionAction,
    pub strategy: String,
    #[serde(default)]
    pub provider_diagnostics: Option<SelectedTextProviderDiagnostics>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedTextProviderDiagnostics {
    pub profile_label: String,
    pub model_code: String,
    #[serde(default)]
    pub provider_preset: Option<String>,
    #[serde(default)]
    pub provider_key_source: Option<String>,
    pub provider_attempted: bool,
    pub provider_succeeded: bool,
    pub deterministic_fallback_used: bool,
    pub fallback_reason: Option<String>,
    pub selected_text_char_count: usize,
    pub output_char_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakePhraseIntentExecutionRequest {
    pub session_id: u64,
    pub command_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakePhraseIntentAction {
    DraftEmail,
    Summarize,
    BulletPlan,
    Rewrite,
    Checklist,
    ReplyMessage,
    GeneralDraft,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakePhraseIntentExecutionResponse {
    pub output_text: String,
    pub action: WakePhraseIntentAction,
    pub strategy: String,
    pub normalized_command: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakePhraseIntentExecutionSummary {
    pub action: WakePhraseIntentAction,
    pub strategy: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructedDictationTransformRequest {
    pub session_id: u64,
    pub instruction_text: String,
    pub content_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructedDictationTransformResponse {
    pub output_text: String,
    pub strategy: String,
    pub provider_diagnostics: InstructedDictationProviderDiagnostics,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructedDictationExecutionSummary {
    pub strategy: String,
    pub trigger_phrase_matched: bool,
    pub parser_result: String,
    pub provider_diagnostics: InstructedDictationProviderDiagnostics,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructedDictationProviderDiagnostics {
    pub profile_label: String,
    pub model_code: String,
    #[serde(default)]
    pub provider_preset: Option<String>,
    #[serde(default)]
    pub provider_key_source: Option<String>,
    pub provider_attempted: bool,
    pub provider_succeeded: bool,
    pub deterministic_fallback_used: bool,
    pub fallback_reason: Option<String>,
    pub instruction_char_count: usize,
    pub content_char_count: usize,
    pub output_char_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CapturedAudio {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub sample_count: usize,
    pub duration_ms: u64,
    pub peak_level: f32,
    pub rms_level: f32,
    pub samples: Vec<f32>,
}

impl CapturedAudio {
    pub fn from_samples(sample_rate_hz: u32, channels: u16, samples: Vec<f32>) -> Self {
        let safe_channels = channels.max(1) as usize;
        let frame_count = samples.len() / safe_channels;
        let duration_ms = if sample_rate_hz == 0 {
            0
        } else {
            ((frame_count as u128) * 1000 / (sample_rate_hz as u128)) as u64
        };
        let peak_level = samples
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        let rms_level = if samples.is_empty() {
            0.0
        } else {
            let mean_square =
                samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32;
            mean_square.sqrt()
        };

        Self {
            sample_rate_hz,
            channels,
            sample_count: samples.len(),
            duration_ms,
            peak_level,
            rms_level,
            samples,
        }
    }

    pub fn stub(duration_ms: u64) -> Self {
        Self {
            sample_rate_hz: 16_000,
            channels: 1,
            sample_count: 0,
            duration_ms,
            peak_level: 0.0,
            rms_level: 0.0,
            samples: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: u64,
    pub session_kind: SessionKind,
    pub final_state: SessionState,
    #[serde(default)]
    pub completed_at_epoch_ms: Option<u128>,
    pub start_feedback_latency_ms: Option<u64>,
    pub recording_start_latency_ms: Option<u64>,
    pub total_session_latency_ms: u64,
    pub audio_duration_ms: u64,
    pub audio_peak_level: f32,
    pub audio_rms_level: f32,
    pub asr_diagnostics: Option<AsrDiagnostics>,
    #[serde(default)]
    pub refine_diagnostics: Option<RefineDiagnostics>,
    pub recognized_text: String,
    pub committed_text: String,
    #[serde(default)]
    pub committed_text_count: Option<u64>,
    pub route_decision: RouteDecision,
    pub degraded_to_asr: bool,
    pub fallback_reason: Option<String>,
    pub commit_status: CommitStatus,
    pub commit_transport: CommitTransport,
    pub commit_failure_reason: Option<String>,
    pub mode_reason: String,
    pub selected_text_execution: Option<SelectedTextExecutionSummary>,
    pub wake_phrase_execution: Option<WakePhraseIntentExecutionSummary>,
    #[serde(default)]
    pub instructed_dictation_execution: Option<InstructedDictationExecutionSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FailedSessionSummary {
    pub session_id: u64,
    pub session_kind: SessionKind,
    pub failure_phase: SessionFailurePhase,
    pub message: String,
    pub audio_duration_ms: Option<u64>,
    pub audio_peak_level: Option<f32>,
    pub audio_rms_level: Option<f32>,
}

impl fmt::Display for FailedSessionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (session_id={}, kind={:?}, phase={:?})",
            self.message, self.session_id, self.session_kind, self.failure_phase
        )
    }
}

pub fn shortcut_to_string(shortcut: &Shortcut) -> String {
    let mut parts: Vec<String> = shortcut
        .modifiers
        .iter()
        .map(|modifier| match modifier {
            KeyModifier::Control => "Ctrl".to_string(),
            KeyModifier::Alt => "Alt".to_string(),
            KeyModifier::Shift => "Shift".to_string(),
            KeyModifier::Meta => "Meta".to_string(),
        })
        .collect();
    parts.push(shortcut.key.clone());
    parts.join("+")
}

pub fn starts_with_wake_phrase(transcript: &str, wake_phrase: &WakePhraseConfig) -> bool {
    wake_phrase_prefix_match(transcript, wake_phrase).is_some()
}

pub fn strip_wake_phrase(transcript: &str, wake_phrase: &WakePhraseConfig) -> String {
    if let Some(trigger_match) = wake_phrase_prefix_match(transcript, wake_phrase) {
        return trim_trigger_separator(trigger_match.rest).to_string();
    }

    transcript.trim_start().to_string()
}

pub fn parse_instructed_dictation(
    transcript: &str,
    wake_phrase: &WakePhraseConfig,
) -> InstructedDictationParseOutcome {
    let Some(trigger_match) = wake_phrase_prefix_match(transcript, wake_phrase) else {
        return InstructedDictationParseOutcome::NoTrigger;
    };
    let task_text = trim_trigger_separator(trigger_match.rest).trim();
    if !task_text.chars().any(char::is_alphanumeric) {
        return InstructedDictationParseOutcome::MissingContent(InstructedDictationParseFailure {
            parser_result: "missing_content".to_string(),
            reason: "Instructed dictation trigger matched, but no content was detected."
                .to_string(),
            task_char_count: task_text.chars().count(),
            trigger_alias: trigger_match.alias,
            leading_filler_removed: trigger_match.leading_filler_removed,
        });
    }

    InstructedDictationParseOutcome::Parsed(InstructedDictationParts {
        task_text: task_text.to_string(),
        parser_result: "generic_task".to_string(),
        trigger_alias: trigger_match.alias,
        leading_filler_removed: trigger_match.leading_filler_removed,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WakePhraseMatch<'a> {
    rest: &'a str,
    alias: InstructedTriggerAlias,
    leading_filler_removed: bool,
}

fn wake_phrase_prefix_match<'a>(
    transcript: &'a str,
    wake_phrase: &WakePhraseConfig,
) -> Option<WakePhraseMatch<'a>> {
    if !wake_phrase.enabled {
        return None;
    }
    let phrase = wake_phrase.phrase.trim();
    if phrase.is_empty() {
        return None;
    }

    let phrase_key = compact_trigger_key(phrase);
    if phrase_key.is_empty() {
        return None;
    }

    let trimmed = trim_trigger_separator(transcript);
    if let Some((rest, alias)) = wake_phrase_variant_prefix_rest(trimmed, &phrase_key) {
        return trigger_match_with_boundary(rest, alias, false);
    }

    let (after_fillers, leading_filler_removed) = trim_leading_trigger_fillers(trimmed);
    if !leading_filler_removed {
        return None;
    }
    let (rest, alias) = wake_phrase_variant_prefix_rest(after_fillers, &phrase_key)?;
    trigger_match_with_boundary(rest, alias, true)
}

fn trigger_match_with_boundary(
    rest: &str,
    alias: InstructedTriggerAlias,
    leading_filler_removed: bool,
) -> Option<WakePhraseMatch<'_>> {
    match rest.chars().next() {
        None => Some(WakePhraseMatch {
            rest,
            alias,
            leading_filler_removed,
        }),
        Some(ch) if is_trigger_separator(ch) || is_cjk_character(ch) => Some(WakePhraseMatch {
            rest,
            alias,
            leading_filler_removed,
        }),
        _ => None,
    }
}

fn wake_phrase_variant_prefix_rest<'a>(
    trimmed: &'a str,
    phrase_key: &str,
) -> Option<(&'a str, InstructedTriggerAlias)> {
    let mut matched = String::new();
    let mut saw_trigger_char = false;
    let mut saw_internal_separator = false;

    for (index, ch) in trimmed.char_indices() {
        if is_trigger_key_character(ch) {
            saw_trigger_char = true;
            matched.extend(ch.to_lowercase());
            if !phrase_key.starts_with(&matched) {
                return None;
            }
            let next_index = index + ch.len_utf8();
            if matched == phrase_key {
                let rest = &trimmed[next_index..];
                if rest
                    .chars()
                    .next()
                    .is_some_and(|next| next.is_ascii_alphanumeric())
                {
                    return None;
                }
                return Some((
                    rest,
                    if saw_internal_separator {
                        InstructedTriggerAlias::Spaced
                    } else {
                        InstructedTriggerAlias::Canonical
                    },
                ));
            }
        } else if !saw_trigger_char || !is_trigger_internal_separator(ch) {
            return None;
        } else {
            saw_internal_separator = true;
        }
    }

    if matched == phrase_key {
        Some((
            "",
            if saw_internal_separator {
                InstructedTriggerAlias::Spaced
            } else {
                InstructedTriggerAlias::Canonical
            },
        ))
    } else {
        None
    }
}

fn trim_leading_trigger_fillers(mut input: &str) -> (&str, bool) {
    const FILLERS: &[&str] = &["\u{55ef}", "\u{5443}", "\u{55e8}", "\u{5582}", "hey", "hi"];
    let mut removed = false;

    for _ in 0..2 {
        let Some(filler) = FILLERS.iter().find(|filler| {
            input.len() > filler.len()
                && input
                    .get(..filler.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(filler))
                && input[filler.len()..]
                    .chars()
                    .next()
                    .is_some_and(is_trigger_separator)
        }) else {
            break;
        };
        input = trim_trigger_separator(&input[filler.len()..]);
        removed = true;
    }

    (input, removed)
}

fn compact_trigger_key(input: &str) -> String {
    input
        .chars()
        .filter(|ch| is_trigger_key_character(*ch))
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_trigger_key_character(ch: char) -> bool {
    ch.is_alphanumeric()
}

fn trim_trigger_separator(input: &str) -> &str {
    input.trim_start_matches(|ch: char| is_trigger_separator(ch))
}

fn is_trigger_separator(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            ',' | '\u{ff0c}'
                | ':'
                | '\u{ff1a}'
                | ';'
                | '\u{ff1b}'
                | '.'
                | '\u{3002}'
                | '!'
                | '\u{ff01}'
                | '?'
                | '\u{ff1f}'
                | '\u{2026}'
        )
}

fn is_trigger_internal_separator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ',' | '\u{ff0c}')
}

fn is_cjk_character(ch: char) -> bool {
    matches!(
        ch,
        '\u{1100}'..='\u{11ff}'
            | '\u{3040}'..='\u{30ff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{4e00}'..='\u{9fff}'
            | '\u{ac00}'..='\u{d7af}'
            | '\u{f900}'..='\u{faff}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn profile_with_overrides(
        quality: &RefinementQuality,
        overrides: &[(&str, &str)],
    ) -> RefinementModelProfile {
        let overrides = overrides
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<HashMap<_, _>>();
        refinement_model_profile_for_quality_with_lookup(quality, |key| overrides.get(key).cloned())
    }

    #[test]
    fn strips_wake_phrase_when_present() {
        let wake_phrase = WakePhraseConfig {
            enabled: true,
            phrase: "Hey VoiceFlow".to_string(),
        };

        let stripped = strip_wake_phrase("Hey VoiceFlow, draft the email", &wake_phrase);
        assert_eq!(stripped, "draft the email");
    }

    #[test]
    fn default_wake_phrase_is_voice_flow() {
        assert_eq!(RuntimeSettings::default().wake_phrase.phrase, "Voice Flow");
    }

    #[test]
    fn configured_voice_flow_accepts_spaced_and_compact_cjk_prefixes() {
        let wake_phrase = WakePhraseConfig {
            enabled: true,
            phrase: "Voice Flow".to_string(),
        };

        for (transcript, expected_rest) in [
            (
                "voice flow把下面这句话翻译成英文我建议暂时停止新功能",
                "把下面这句话翻译成英文我建议暂时停止新功能",
            ),
            (
                "voiceflow把下面这句话翻译成英文我建议暂时停止新功能",
                "把下面这句话翻译成英文我建议暂时停止新功能",
            ),
            ("VOICE FLOW请把下面内容翻译成英文", "请把下面内容翻译成英文"),
        ] {
            assert!(starts_with_wake_phrase(transcript, &wake_phrase));
            assert_eq!(strip_wake_phrase(transcript, &wake_phrase), expected_rest);
        }
    }

    #[test]
    fn configured_voice_flow_in_the_middle_does_not_activate() {
        let wake_phrase = WakePhraseConfig {
            enabled: true,
            phrase: "Voice Flow".to_string(),
        };

        assert!(!starts_with_wake_phrase(
            "我觉得 voice flow把下面这句话翻译成英文",
            &wake_phrase
        ));
    }

    fn enabled_wake_phrase() -> WakePhraseConfig {
        WakePhraseConfig {
            enabled: true,
            phrase: "Voice Flow".to_string(),
        }
    }

    fn parsed_instructed_task(transcript: &str) -> InstructedDictationParts {
        match parse_instructed_dictation(transcript, &enabled_wake_phrase()) {
            InstructedDictationParseOutcome::Parsed(parts) => parts,
            other => panic!("expected parsed instructed dictation, got {other:?}"),
        }
    }

    #[test]
    fn instructed_trigger_normalizes_aliases_punctuation_and_leading_fillers() {
        for (transcript, expected_alias, filler_removed, expected_task) in [
            (
                "VoiceFlow 帮我写一条通知告诉大家明天上午系统维护",
                InstructedTriggerAlias::Canonical,
                false,
                "帮我写一条通知告诉大家明天上午系统维护",
            ),
            (
                "voice flow 帮我写一条通知",
                InstructedTriggerAlias::Spaced,
                false,
                "帮我写一条通知",
            ),
            (
                "嗯 voice flow 帮我写一条通知",
                InstructedTriggerAlias::Spaced,
                true,
                "帮我写一条通知",
            ),
            (
                "呃，VoiceFlow，请帮我整理会议记录",
                InstructedTriggerAlias::Canonical,
                true,
                "请帮我整理会议记录",
            ),
            (
                "Hey Voice Flow, please write a short message",
                InstructedTriggerAlias::Spaced,
                true,
                "please write a short message",
            ),
            (
                "！HI，VOICEFLOW：总结 API rollout 风险",
                InstructedTriggerAlias::Canonical,
                true,
                "总结 API rollout 风险",
            ),
        ] {
            let parts = parsed_instructed_task(transcript);

            assert_eq!(parts.task_text, expected_task);
            assert_eq!(parts.trigger_alias, expected_alias);
            assert_eq!(parts.leading_filler_removed, filler_removed);
            assert_eq!(parts.parser_result, "generic_task");
        }
    }

    #[test]
    fn instructed_trigger_allows_at_most_two_leading_fillers() {
        let parsed = parsed_instructed_task("嗯，呃，VoiceFlow 写一条通知");
        assert_eq!(parsed.task_text, "写一条通知");
        assert!(parsed.leading_filler_removed);

        let outcome =
            parse_instructed_dictation("嗯，呃，嗨，VoiceFlow 写一条通知", &enabled_wake_phrase());
        assert!(matches!(
            outcome,
            InstructedDictationParseOutcome::NoTrigger
        ));
    }

    #[test]
    fn instructed_trigger_in_middle_remains_normal_dictation() {
        for transcript in [
            "我觉得 VoiceFlow 这个名字还不错",
            "我刚才打开了 Voice Flow 的设置",
            "他说嗯，然后开始讲话",
            "这个通知需要用 VoiceFlow 来输入",
        ] {
            assert!(matches!(
                parse_instructed_dictation(transcript, &enabled_wake_phrase()),
                InstructedDictationParseOutcome::NoTrigger
            ));
        }
    }

    #[test]
    fn configured_trigger_phrase_still_matches_as_one_prefix() {
        let wake_phrase = WakePhraseConfig {
            enabled: true,
            phrase: "Hey VoiceFlow".to_string(),
        };
        let parts = match parse_instructed_dictation(
            "Hey Voice Flow, translate the following: 我建议暂时停止新功能",
            &wake_phrase,
        ) {
            InstructedDictationParseOutcome::Parsed(parts) => parts,
            other => panic!("expected configured trigger to parse, got {other:?}"),
        };

        assert_eq!(
            parts.task_text,
            "translate the following: 我建议暂时停止新功能"
        );
        assert!(!parts.leading_filler_removed);
    }

    #[test]
    fn unicode_custom_trigger_supports_chinese_and_mixed_language_forms() {
        let chinese_trigger = WakePhraseConfig {
            enabled: true,
            phrase: "小王啊".to_string(),
        };
        for (transcript, expected_alias, filler_removed, expected_task) in [
            (
                "小王啊，帮我写一条通知",
                InstructedTriggerAlias::Canonical,
                false,
                "帮我写一条通知",
            ),
            (
                "小 王 啊 帮我整理会议记录",
                InstructedTriggerAlias::Spaced,
                false,
                "帮我整理会议记录",
            ),
            (
                "嗯，小王啊：总结这次发布风险",
                InstructedTriggerAlias::Canonical,
                true,
                "总结这次发布风险",
            ),
        ] {
            let parts = match parse_instructed_dictation(transcript, &chinese_trigger) {
                InstructedDictationParseOutcome::Parsed(parts) => parts,
                other => panic!("expected Unicode trigger to parse, got {other:?}"),
            };

            assert_eq!(parts.task_text, expected_task);
            assert_eq!(parts.trigger_alias, expected_alias);
            assert_eq!(parts.leading_filler_removed, filler_removed);
        }

        let mixed_trigger = WakePhraseConfig {
            enabled: true,
            phrase: "小王 AI".to_string(),
        };
        let parts = match parse_instructed_dictation("小王ai，draft a release note", &mixed_trigger)
        {
            InstructedDictationParseOutcome::Parsed(parts) => parts,
            other => panic!("expected mixed-language trigger to parse, got {other:?}"),
        };
        assert_eq!(parts.task_text, "draft a release note");
        assert_eq!(parts.trigger_alias, InstructedTriggerAlias::Canonical);
    }

    #[test]
    fn unicode_custom_trigger_keeps_prefix_and_task_guards() {
        let wake_phrase = WakePhraseConfig {
            enabled: true,
            phrase: "小王啊".to_string(),
        };

        assert!(matches!(
            parse_instructed_dictation("我刚才叫小王啊帮忙整理记录", &wake_phrase),
            InstructedDictationParseOutcome::NoTrigger
        ));
        assert!(matches!(
            parse_instructed_dictation("小王啊……", &wake_phrase),
            InstructedDictationParseOutcome::MissingContent(_)
        ));
    }

    #[test]
    fn instructed_trigger_forwards_the_complete_generic_task() {
        for task in [
            "帮我写一条通知告诉大家明天系统维护",
            "写一封邮件提醒团队提交周报",
            "translate the following: 我建议暂时停止新功能",
            "把这段内容整理成 bullet points，保留 timeline 和负责人",
        ] {
            let transcript = format!("VoiceFlow，{task}");
            let parts = parsed_instructed_task(&transcript);

            assert_eq!(parts.task_text, task);
            assert_eq!(parts.parser_result, "generic_task");
        }
    }

    #[test]
    fn instructed_trigger_preserves_long_unicode_task_text() {
        let task = format!(
            "请整理{}",
            std::iter::repeat_n("这次 API rollout 的风险和负责人", 16)
                .collect::<Vec<_>>()
                .join("，")
        );
        let parts = parsed_instructed_task(&format!("嗯 voice flow {task}"));

        assert_eq!(parts.task_text, task);
        assert!(count_text_units(&parts.task_text) > DICTATION_STRUCTURED_THRESHOLD);
        assert!(parts.leading_filler_removed);
    }

    #[test]
    fn instructed_trigger_without_meaningful_task_fails_safely() {
        for transcript in [
            "VoiceFlow",
            "嗯 Voice Flow",
            "VoiceFlow……",
            "VoiceFlow？！  ",
        ] {
            let failure = match parse_instructed_dictation(transcript, &enabled_wake_phrase()) {
                InstructedDictationParseOutcome::MissingContent(failure) => failure,
                other => panic!("expected missing content, got {other:?}"),
            };

            assert_eq!(failure.task_char_count, 0);
            assert_eq!(failure.parser_result, "missing_content");
        }
    }

    #[test]
    fn disabled_instructed_trigger_does_not_activate() {
        let wake_phrase = WakePhraseConfig {
            enabled: false,
            phrase: "Voice Flow".to_string(),
        };

        assert!(matches!(
            parse_instructed_dictation("VoiceFlow 写一条通知", &wake_phrase),
            InstructedDictationParseOutcome::NoTrigger
        ));
    }

    #[test]
    fn computes_audio_summary_from_samples() {
        let audio = CapturedAudio::from_samples(16_000, 1, vec![0.25, -0.75, 0.5]);

        assert_eq!(audio.sample_count, 3);
        assert_eq!(audio.duration_ms, 0);
        assert_eq!(audio.peak_level, 0.75);
        assert!(audio.rms_level > 0.0);
    }

    #[test]
    fn runtime_settings_defaults_missing_silence_gate_level_when_deserializing() {
        let decoded: RuntimeSettings = serde_json::from_str(
            r#"{
                "dictation_shortcut":{"modifiers":["Control"],"key":"Space"},
                "edit_shortcut":{"modifiers":["Control"],"key":"Space"},
                "shortcut_mode":"Toggle",
                "refinement_quality":"Balanced",
                "audio_feedback_enabled":true,
                "wake_phrase":{"enabled":true,"phrase":"Hey VoiceFlow"},
                "diagnostics_verbosity":"Standard"
            }"#,
        )
        .expect("legacy runtime settings should still deserialize");

        assert_eq!(decoded.silence_gate_level, 2);
        assert_eq!(decoded.provider, ProviderSettings::default());
    }

    #[test]
    fn runtime_settings_defaults_language_and_ui_style() {
        let settings = RuntimeSettings::default();

        assert_eq!(settings.system_language, SystemLanguage::English);
        assert_eq!(settings.ui_style, UiStyle::Dark);
    }

    #[test]
    fn runtime_settings_serializes_language_and_ui_style() {
        let settings = RuntimeSettings {
            system_language: SystemLanguage::Chinese,
            ui_style: UiStyle::Light,
            ..RuntimeSettings::default()
        };

        let encoded = serde_json::to_string(&settings).expect("runtime settings should serialize");

        assert!(encoded.contains(r#""system_language":"Chinese""#));
        assert!(encoded.contains(r#""ui_style":"Light""#));

        let decoded: RuntimeSettings =
            serde_json::from_str(&encoded).expect("runtime settings should deserialize");

        assert_eq!(decoded.system_language, SystemLanguage::Chinese);
        assert_eq!(decoded.ui_style, UiStyle::Light);
    }

    #[test]
    fn runtime_settings_defaults_missing_language_and_ui_style_when_deserializing() {
        let decoded: RuntimeSettings = serde_json::from_str(
            r#"{
                "dictation_shortcut":{"modifiers":["Control"],"key":"Space"},
                "edit_shortcut":{"modifiers":["Control"],"key":"Space"},
                "shortcut_mode":"Toggle",
                "refinement_quality":"Balanced",
                "audio_feedback_enabled":true,
                "wake_phrase":{"enabled":true,"phrase":"Hey VoiceFlow"},
                "diagnostics_verbosity":"Standard"
            }"#,
        )
        .expect("legacy runtime settings should still deserialize");

        assert_eq!(decoded.system_language, SystemLanguage::English);
        assert_eq!(decoded.ui_style, UiStyle::Dark);
    }

    #[test]
    fn runtime_settings_rejects_invalid_language_or_ui_style() {
        let language_error = serde_json::from_str::<RuntimeSettings>(
            r#"{
                "dictation_shortcut":{"modifiers":["Control"],"key":"Space"},
                "edit_shortcut":{"modifiers":["Control"],"key":"Space"},
                "shortcut_mode":"Toggle",
                "refinement_quality":"Balanced",
                "audio_feedback_enabled":true,
                "wake_phrase":{"enabled":true,"phrase":"Hey VoiceFlow"},
                "diagnostics_verbosity":"Standard",
                "system_language":"Klingon",
                "ui_style":"Dark"
            }"#,
        )
        .expect_err("invalid system language should be rejected");

        assert!(language_error.to_string().contains("Klingon"));

        let ui_error = serde_json::from_str::<RuntimeSettings>(
            r#"{
                "dictation_shortcut":{"modifiers":["Control"],"key":"Space"},
                "edit_shortcut":{"modifiers":["Control"],"key":"Space"},
                "shortcut_mode":"Toggle",
                "refinement_quality":"Balanced",
                "audio_feedback_enabled":true,
                "wake_phrase":{"enabled":true,"phrase":"Hey VoiceFlow"},
                "diagnostics_verbosity":"Standard",
                "system_language":"English",
                "ui_style":"Solarized"
            }"#,
        )
        .expect_err("invalid UI style should be rejected");

        assert!(ui_error.to_string().contains("Solarized"));
    }

    #[test]
    fn maps_refinement_quality_to_llm_profile() {
        let fast = profile_with_overrides(&RefinementQuality::Fast, &[]);
        assert_eq!(fast.display_label, "Fast / 极速");
        assert_eq!(fast.model_name, "Qwen Turbo");
        assert_eq!(fast.model_code, "qwen-turbo");
        assert_eq!(fast.enable_thinking, None);

        let balanced = profile_with_overrides(&RefinementQuality::Balanced, &[]);
        assert_eq!(balanced.display_label, "Balanced / 均衡");
        assert_eq!(balanced.model_name, "Qwen 3.5 Flash");
        assert_eq!(balanced.model_code, "qwen3.5-flash");
        assert_eq!(balanced.enable_thinking, Some(false));

        let best = profile_with_overrides(&RefinementQuality::BestQuality, &[]);
        assert_eq!(best.display_label, "Best Quality / 高质量");
        assert_eq!(best.model_name, "DeepSeek V4 Flash");
        assert_eq!(best.model_code, "deepseek-v4-flash");
        assert_eq!(best.enable_thinking, Some(false));
    }

    #[test]
    fn runtime_settings_default_to_best_quality() {
        let settings = RuntimeSettings::default();
        let profile = refinement_model_profile_for_quality(&settings.refinement_quality);

        assert_eq!(settings.refinement_quality, RefinementQuality::BestQuality);
        assert_eq!(settings.history_retention, HistoryRetention::Latest100);
        assert_eq!(profile.model_name, "DeepSeek V4 Flash");
        assert_eq!(profile.model_code, "deepseek-v4-flash");
    }

    #[test]
    fn runtime_settings_without_history_retention_use_latest_100() {
        let mut value =
            serde_json::to_value(RuntimeSettings::default()).expect("settings should serialize");
        value
            .as_object_mut()
            .expect("settings should be an object")
            .remove("history_retention");

        let settings: RuntimeSettings =
            serde_json::from_value(value).expect("legacy settings should deserialize");

        assert_eq!(settings.history_retention, HistoryRetention::Latest100);
    }

    #[test]
    fn history_retention_uses_supported_snake_case_values() {
        for (encoded, expected) in [
            ("latest_100", HistoryRetention::Latest100),
            ("latest_500", HistoryRetention::Latest500),
            ("latest_1000", HistoryRetention::Latest1000),
            ("last_7_days", HistoryRetention::Last7Days),
            ("last_30_days", HistoryRetention::Last30Days),
            ("unlimited", HistoryRetention::Unlimited),
        ] {
            let decoded: HistoryRetention = serde_json::from_str(&format!("\"{encoded}\""))
                .expect("supported retention should deserialize");
            assert_eq!(decoded, expected);
        }

        assert!(serde_json::from_str::<HistoryRetention>("\"forever\"").is_err());
    }

    #[test]
    fn model_env_overrides_apply_to_fast_balanced_and_best_profiles() {
        let fast = profile_with_overrides(
            &RefinementQuality::Fast,
            &[("VOICEFLOW_MODEL_FAST", "qwen-fast-packaged")],
        );
        let balanced = profile_with_overrides(
            &RefinementQuality::Balanced,
            &[("VOICEFLOW_MODEL_BALANCED", "qwen-balanced-packaged")],
        );
        let best = profile_with_overrides(
            &RefinementQuality::BestQuality,
            &[("VOICEFLOW_MODEL_BEST", "deepseek-best-packaged")],
        );

        assert_eq!(fast.model_code, "qwen-fast-packaged");
        assert_eq!(balanced.model_code, "qwen-balanced-packaged");
        assert_eq!(best.model_code, "deepseek-best-packaged");
    }

    #[test]
    fn fast_plus_uses_balanced_model_env_override() {
        let fast_plus = profile_with_overrides(
            &RefinementQuality::FastPlus,
            &[("VOICEFLOW_MODEL_BALANCED", "qwen-balanced-packaged")],
        );

        assert_eq!(fast_plus.model_code, "qwen-balanced-packaged");
    }

    #[test]
    fn empty_model_env_override_is_ignored() {
        let fast =
            profile_with_overrides(&RefinementQuality::Fast, &[("VOICEFLOW_MODEL_FAST", "   ")]);

        assert_eq!(fast.model_code, "qwen-turbo");
    }

    #[test]
    fn thinking_env_override_parses_valid_booleans() {
        for (value, expected) in [
            ("true", true),
            ("1", true),
            ("yes", true),
            ("on", true),
            ("false", false),
            ("0", false),
            ("no", false),
            ("off", false),
        ] {
            let profile = profile_with_overrides(
                &RefinementQuality::Fast,
                &[("VOICEFLOW_MODEL_FAST_THINKING", value)],
            );

            assert_eq!(
                profile.enable_thinking,
                Some(expected),
                "thinking override {value} should parse"
            );
        }
    }

    #[test]
    fn invalid_thinking_env_override_is_ignored() {
        let fast = profile_with_overrides(
            &RefinementQuality::Fast,
            &[("VOICEFLOW_MODEL_FAST_THINKING", "maybe")],
        );
        let balanced = profile_with_overrides(
            &RefinementQuality::Balanced,
            &[("VOICEFLOW_MODEL_BALANCED_THINKING", "")],
        );

        assert_eq!(fast.enable_thinking, None);
        assert_eq!(balanced.enable_thinking, Some(false));
    }
}
