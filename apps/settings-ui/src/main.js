const summary = document.getElementById("hero-summary");
const wakePhrase = document.getElementById("wake-phrase");
const primaryShortcutKey = document.getElementById("primary-shortcut-key");
const audioToggle = document.getElementById("audio-toggle");
const silenceGate = document.getElementById("silence-gate");
const silenceLabel = document.getElementById("silence-label");
const reportPath = document.getElementById("report-path");
const reportWarning = document.getElementById("report-warning");
const reportFreshness = document.getElementById("report-freshness");
const reportRecencyChip = document.getElementById("report-recency-chip");
const runtimeHealth = document.getElementById("runtime-health");
const healthNote = document.getElementById("health-note");
const recentProblemSummary = document.getElementById("recent-problem-summary");
const recommendedActionCard = document.getElementById("recommended-action-card");
const recommendedAction = document.getElementById("recommended-action");
const settingsPath = document.getElementById("settings-path");
const settingsWarnings = document.getElementById("settings-warnings");
const saveCommand = document.getElementById("save-command");
const applyButton = document.getElementById("apply-settings");
const revertButton = document.getElementById("revert-settings");
const clearHistoryButton = document.getElementById("clear-history");
const clearHistoryStatus = document.getElementById("clear-history-status");
const promptOverridesList = document.getElementById("prompt-overrides-list");
const promptOverridesStatus = document.getElementById("prompt-overrides-status");
const reloadPromptsButton = document.getElementById("reload-prompts");
const saveStatus = document.getElementById("save-status");
const saveDock = document.querySelector(".save-dock");
const copySaveCommandButton = document.getElementById("copy-save-command");
const commandCopyStatus = document.getElementById("command-copy-status");
const lastApplyTime = document.getElementById("last-apply-time");
const lastApplySource = document.getElementById("last-apply-source");
const lastApplyFields = document.getElementById("last-apply-fields");
const bridgeStatus = document.getElementById("bridge-status");
const dirtyStatus = document.getElementById("dirty-status");
const draftStageChip = document.getElementById("draft-stage-chip");
const draftStageCopy = document.getElementById("draft-stage-copy");
const savedStageChip = document.getElementById("saved-stage-chip");
const savedStageCopy = document.getElementById("saved-stage-copy");
const runtimeStageChip = document.getElementById("runtime-stage-chip");
const runtimeStageCopy = document.getElementById("runtime-stage-copy");
const runtimeDriftBanner = document.getElementById("runtime-drift-banner");
const runtimeDriftBannerChip = document.getElementById("runtime-drift-banner-chip");
const runtimeDriftBannerCopy = document.getElementById("runtime-drift-banner-copy");
const runtimeDriftBannerFields = document.getElementById("runtime-drift-banner-fields");
const shortcutPanelStatus = document.getElementById("shortcut-panel-status");
const speechPanelStatus = document.getElementById("speech-panel-status");
const feedbackPanelStatus = document.getElementById("feedback-panel-status");
const diagnosticsPanelStatus = document.getElementById("diagnostics-panel-status");
const attemptedSessions = document.getElementById("attempted-sessions");
const successfulSessions = document.getElementById("successful-sessions");
const failedSessions = document.getElementById("failed-sessions");
const lateFailures = document.getElementById("late-failures");
const earlyFailures = document.getElementById("early-failures");
const directCommits = document.getElementById("direct-commits");
const clipboardCommits = document.getElementById("clipboard-commits");
const selectionCommits = document.getElementById("selection-commits");
const avgTotalLatency = document.getElementById("avg-total-latency");
const avgStartLatency = document.getElementById("avg-start-latency");
const avgAudioDuration = document.getElementById("avg-audio-duration");
const avgAudioRms = document.getElementById("avg-audio-rms");
const dictationMix = document.getElementById("dictation-mix");
const editMix = document.getElementById("edit-mix");
const intentMix = document.getElementById("intent-mix");
const selectedTextActionMix = document.getElementById("selected-text-action-mix");
const wakePhraseActionMix = document.getElementById("wake-phrase-action-mix");
const localOnlyMix = document.getElementById("local-only-mix");
const localRefineMix = document.getElementById("local-refine-mix");
const cloudRefineMix = document.getElementById("cloud-refine-mix");
const silenceFailures = document.getElementById("silence-failures");
const noSpeechFailures = document.getElementById("no-speech-failures");
const commitFailures = document.getElementById("commit-failures");
const recordingFailures = document.getElementById("recording-failures");
const recognizingFailures = document.getElementById("recognizing-failures");
const dominantFailureMode = document.getElementById("dominant-failure-mode");
const dominantFailureGuidance = document.getElementById("dominant-failure-guidance");
const failureProfileSection = document.getElementById("failure-profile-section");
const commitPathOutlookChip = document.getElementById("commit-path-outlook-chip");
const commitPathSignal = document.getElementById("commit-path-signal");
const commitPathGuidance = document.getElementById("commit-path-guidance");
const verificationFocusChip = document.getElementById("verification-focus-chip");
const verificationFocus = document.getElementById("verification-focus");
const verificationFocusGuidance = document.getElementById("verification-focus-guidance");
const verificationFocusSection = document.getElementById("verification-focus-section");
const workloadFocusChip = document.getElementById("workload-focus-chip");
const workloadFocus = document.getElementById("workload-focus");
const workloadFocusGuidance = document.getElementById("workload-focus-guidance");
const workloadFocusSection = document.getElementById("workload-focus-section");
const tuningRecommendationsChip = document.getElementById("tuning-recommendations-chip");
const tuningRecommendations = document.getElementById("tuning-recommendations");
const tuningSection = document.getElementById("tuning-section");
const stageAllRecommendationsButton = document.getElementById("stage-all-recommendations");
const tuningNextActionChip = document.getElementById("tuning-next-action-chip");
const tuningNextActionHeadline = document.getElementById("tuning-next-action-headline");
const tuningNextActionCopy = document.getElementById("tuning-next-action-copy");
const tuningStepStageChip = document.getElementById("tuning-step-stage-chip");
const tuningStepStageCopy = document.getElementById("tuning-step-stage-copy");
const tuningStepApplyChip = document.getElementById("tuning-step-apply-chip");
const tuningStepApplyCopy = document.getElementById("tuning-step-apply-copy");
const tuningStepRestartChip = document.getElementById("tuning-step-restart-chip");
const tuningStepRestartCopy = document.getElementById("tuning-step-restart-copy");
const tuningStepVerifyChip = document.getElementById("tuning-step-verify-chip");
const tuningStepVerifyCopy = document.getElementById("tuning-step-verify-copy");
const tuningRunbookChip = document.getElementById("tuning-runbook-chip");
const tuningRunbookList = document.getElementById("tuning-runbook-list");
const lastSuccess = document.getElementById("last-success");
const lastSuccessMeta = document.getElementById("last-success-meta");
const lastFailure = document.getElementById("last-failure");
const lastFailureMeta = document.getElementById("last-failure-meta");
const failureHistory = document.getElementById("failure-history");
const diagnosticPatternsSection = document.getElementById("diagnostic-patterns-section");
const recentFailurePatternGroup = document.getElementById("recent-failure-pattern-group");
const shortcutWarningGroup = document.getElementById("shortcut-warning-group");
const wakePhraseToggle = document.getElementById("wake-phrase-toggle");
const shortcutModifierButtons = Array.from(document.querySelectorAll(".modifier-pill"));
const shortcutWarning = document.getElementById("shortcut-warning");
const shortcutWarningNote = document.getElementById("shortcut-warning-note");
const shortcutRuntimeWarnings = document.getElementById("shortcut-runtime-warnings");
const runtimeAlignmentChip = document.getElementById("runtime-alignment-chip");
const runtimeDriftFields = document.getElementById("runtime-drift-fields");
const runtimeDriftList = document.getElementById("runtime-drift-list");
const runtimeShortcutHeadline = document.getElementById("runtime-shortcut-headline");
const runtimeShortcutNote = document.getElementById("runtime-shortcut-note");
const runtimeProfileList = document.getElementById("runtime-profile-list");
const overviewStatus = document.getElementById("overview-status");
const overviewReportHealth = document.getElementById("overview-report-health");
const overviewReportAge = document.getElementById("overview-report-age");
const overviewHealthNote = document.getElementById("overview-health-note");
const overviewAttention = document.getElementById("overview-attention");
const overviewRecentProblem = document.getElementById("overview-recent-problem");
const overviewRecommendedAction = document.getElementById("overview-recommended-action");
const diagnosticsConnectionStatus = document.getElementById("diagnostics-connection-status");
const usageTrendChart = document.getElementById("usage-trend-chart");
const usageTrendTotal = document.getElementById("usage-trend-total");
const usageEmptyState = document.getElementById("usage-empty-state");
const usageRecordingDuration = document.getElementById("usage-recording-duration");
const usageInsertedText = document.getElementById("usage-inserted-text");
const usageTimeSaved = document.getElementById("usage-time-saved");
const usageAverageSpeed = document.getElementById("usage-average-speed");
const usageAverageSpeedSuffix = document.getElementById("usage-average-speed-suffix");
const usageLatestApp = document.getElementById("usage-latest-app");
const usageAvgLatency = document.getElementById("usage-avg-latency");
const usageRouteLocalOnly = document.getElementById("usage-route-local-only");
const usageRouteLocalRefine = document.getElementById("usage-route-local-refine");
const usageRouteCloudRefine = document.getElementById("usage-route-cloud-refine");
const overviewRecentList = document.getElementById("overview-recent-list");
const overviewRecentEmpty = document.getElementById("overview-recent-empty");
const overviewPrivacyToggle = document.getElementById("overview-privacy-toggle");
const overviewPrivacyVisibleIcon = document.querySelector(".privacy-icon-visible");
const overviewPrivacyHiddenIcon = document.querySelector(".privacy-icon-hidden");
const overviewLocalAsr = document.getElementById("overview-local-asr");
const overviewRefinement = document.getElementById("overview-refinement");
const overviewSetupShortcut = document.getElementById("overview-setup-shortcut");
const overviewRecordingMode = document.getElementById("overview-recording-mode");
const historyEmptyState = document.getElementById("history-empty-state");
const historyList = document.getElementById("history-list");
const historyRetentionOptions = document.getElementById("history-retention-options");
const historyRetentionWarning = document.getElementById("history-retention-warning");
const historyRetentionButtons = Array.from(
  document.querySelectorAll("[data-retention-value]"),
);
const providerPreset = document.getElementById("provider-preset");
const providerBaseUrl = document.getElementById("provider-base-url");
const providerCustomBaseUrlField = document.getElementById(
  "provider-custom-base-url-field",
);
const providerPresetEndpointSettings = document.getElementById(
  "provider-preset-endpoint-settings",
);
const providerPresetBaseUrl = document.getElementById("provider-preset-base-url");
const providerEndpointOverride = document.getElementById("provider-endpoint-override");
const providerOverrideBaseUrl = document.getElementById("provider-override-base-url");
const providerOverrideBaseUrlField = document.getElementById(
  "provider-override-base-url-field",
);
const providerEndpointReset = document.getElementById("provider-endpoint-reset");
const providerActiveModel = document.getElementById("provider-active-model");
const providerActiveModelHelp = document.getElementById("provider-active-model-help");
const providerTimeoutSeconds = document.getElementById("provider-timeout-seconds");
const providerConfiguredChip = document.getElementById("provider-configured-chip");
const providerKeyStatus = document.getElementById("provider-key-status");
const providerStoredCredentialStatus = document.getElementById(
  "provider-stored-credential-status",
);
const providerKeySourceStatus = document.getElementById("provider-key-source-status");
const providerApiKeyInput = document.getElementById("provider-api-key");
const providerApiKeyHelp = document.getElementById("provider-api-key-help");
const providerApiKeyStatus = document.getElementById("provider-api-key-status");
const saveProviderApiKeyButton = document.getElementById("save-provider-api-key");
const clearProviderApiKeyButton = document.getElementById("clear-provider-api-key");
const testProviderConnectionButton = document.getElementById("test-provider-connection");
const providerTestStatus = document.getElementById("provider-test-status");
const providerTestDetails = document.getElementById("provider-test-details");
const providerEffectiveProvider = document.getElementById("provider-effective-provider");
const providerEffectiveBaseUrl = document.getElementById("provider-effective-base-url");
const providerEffectiveModel = document.getElementById("provider-effective-model");
const providerEffectiveTimeout = document.getElementById("provider-effective-timeout");
const providerOverrideList = document.getElementById("provider-override-list");
const providerSpecificNote = document.getElementById("provider-specific-note");
const providerBaseUrlMessage = document.getElementById("provider-base-url-message");
const providerOverrideBaseUrlMessage = document.getElementById(
  "provider-override-base-url-message",
);
const providerTimeoutMessage = document.getElementById("provider-timeout-message");
const settingsUiModel = window.VoiceFlowSettingsModel;
const providerSettingsModel = window.VoiceFlowProviderSettingsModel;
const settingsSectionButtons = Array.from(
  document.querySelectorAll("[data-settings-section]"),
);
const settingsPanes = Array.from(document.querySelectorAll("[data-settings-pane]"));

const segmentGroups = Array.from(document.querySelectorAll(".segmented"));
const supportedShortcutKeys = ["Space"];
for (let code = 65; code <= 90; code += 1) {
  supportedShortcutKeys.push(String.fromCharCode(code));
}
for (let digit = 0; digit <= 9; digit += 1) {
  supportedShortcutKeys.push(String(digit));
}

const modifierOrder = {
  Control: 0,
  Alt: 1,
  Shift: 2,
  Meta: 3,
};

function canonicalizeModifiers(modifiers) {
  return [...modifiers].sort(
    (left, right) => (modifierOrder[left] ?? 99) - (modifierOrder[right] ?? 99),
  );
}

function canonicalizeShortcutKey(key) {
  if (String(key).toLowerCase() === "space") {
    return "Space";
  }

  if (String(key).length === 1) {
    return String(key).toUpperCase();
  }

  return String(key);
}

const state = {
  systemLanguage: "English",
  primaryShortcutModifiers: canonicalizeModifiers(["Control"]),
  primaryShortcutKey: canonicalizeShortcutKey("Space"),
  mode: "PushToTalk",
  quality: "Best Quality",
  verbosity: "Standard",
  audioFeedback: true,
  wakePhraseEnabled: true,
  wakePhrase: wakePhrase.value,
  silenceGate: silenceGate.value,
  historyRetention: "latest_100",
  providerPreset: "bailian",
  providerBaseUrl: "",
  providerActiveModel: "",
  providerTimeoutSeconds: "",
};

const persistedState = {
  systemLanguage: state.systemLanguage,
  primaryShortcutModifiers: [...state.primaryShortcutModifiers],
  primaryShortcutKey: state.primaryShortcutKey,
  mode: state.mode,
  quality: state.quality,
  silenceGate: state.silenceGate,
  verbosity: state.verbosity,
  audioFeedback: state.audioFeedback,
  wakePhraseEnabled: state.wakePhraseEnabled,
  wakePhrase: state.wakePhrase,
  historyRetention: state.historyRetention,
  providerPreset: state.providerPreset,
  providerBaseUrl: state.providerBaseUrl,
  providerActiveModel: state.providerActiveModel,
  providerTimeoutSeconds: state.providerTimeoutSeconds,
};

let runtimeTimestamp = 0;
let saveInFlight = false;
let providerEndpointOverrideEnabled = false;
let providerValidationSubmitted = false;
const providerValidationTouched = {
  baseUrl: false,
  timeout: false,
};
const dirtyFields = new Set();
let bridgeConnected = false;
let runtimeDataSource = "static";
let latestTuningRecommendations = [];
let latestLiveSummary = null;
let historyClearInFlight = false;
let historyRenderSignature = "";
let usageChartSignature = "";
let overviewRecentEntries = [];
let overviewPreviewsHidden = readOverviewPrivacyPreference();
let promptOverrideReloadInFlight = false;
let providerCredentialInFlight = false;
let providerTestInFlight = false;
let providerTestResult = null;
let providerTestStale = false;
let activeSettingsSection = "general";
let lastSaveFeedback = null;
let saveFeedbackTimer = null;
const settingsControlTokenHeader = "X-VoiceFlow-Settings-Token";

const translations = {
  English: {
    "document.title": "VoiceFlow Settings",
    "brand.subtitle": "Speech Input Method",
    "brand.version": "v0.1.2-beta",
    "nav.overview": "Overview",
    "nav.history": "History",
    "nav.settings": "Settings",
    "nav.diagnostics": "Diagnostics",
    "common.localOnly": "Local only",
    "common.total": "Total",
    "common.selectedText": "Selected text",
    "common.unavailable": "Unavailable",
    "common.idle": "Idle",
    "overview.ready": "Your local voice activity at a glance.",
    "overview.readyLive": "Local activity is up to date.",
    "overview.readySaved": "Showing your latest saved activity.",
    "overview.connectedNow": "Connected to VoiceFlow now",
    "overview.offlineNow": "VoiceFlow is offline",
    "overview.currentState": "Current setup",
    "overview.latestReport": "Latest diagnostic report",
    "overview.usageSummary": "Recent activity and all-time totals",
    "overview.lastSevenDays": "Last 7 days",
    "overview.allTimeTotals": "All-time totals",
    "overview.shortcut": "Shortcut",
    "overview.usage": "Usage",
    "overview.sevenDayActivity": "Words entered and estimated time saved over seven days",
    "overview.wordsInWeek": "{count} words",
    "overview.insertedText": "Words entered",
    "overview.insertedTextNote": "Across successful completed sessions",
    "overview.totalDictationDuration": "Total dictation duration",
    "overview.totalDictationDurationNote": "Actual captured audio",
    "overview.timeSaved": "Time saved",
    "overview.timeSavedNote": "Compared with manual typing",
    "overview.averageDictationSpeed": "Average dictation speed",
    "overview.averageDictationSpeedNote": "Words entered per minute",
    "overview.speedSuffix": "words/min",
    "overview.chartTooltip": "{words} words · {time} saved",
    "overview.chartAria": "Seven-day words entered and estimated time saved",
    "overview.hidePreviews": "Hide recent dictation previews",
    "overview.showPreviews": "Show recent dictation previews",
    "overview.success": "Success",
    "overview.recentDictations": "Recent dictations",
    "overview.recentDictationsNote": "Your latest locally stored dictation outputs",
    "overview.viewHistory": "View history",
    "overview.noRecentDictations": "Your recent dictations will appear here.",
    "overview.currentSetup": "Current setup",
    "overview.currentSetupNote": "The essentials for your next dictation",
    "overview.speechRecognition": "Speech recognition",
    "overview.local": "Local",
    "overview.textRefinement": "Text refinement",
    "overview.localCleanup": "Local cleanup only",
    "overview.noUsage": "Complete a dictation to start your local activity chart.",
    "history.eyebrow": "History",
    "history.title": "History",
    "history.subtitle": "Stored locally on this device. Selected-text edits are redacted by default.",
    "history.localHistory": "Local history",
    "history.latestEntries": "Latest entries",
    "history.empty": "Your recent dictation will appear here.",
    "history.emptyWithPolicy": "Stored locally on this device · {policy}",
    "history.localPrivate": "Recent dictation is stored locally. Selected-text edits are redacted by default.",
    "history.staticSnapshot": "Stored locally on this device · Showing recent available history",
    "history.liveEntries": "Stored locally on this device · {count} retained · {policy}",
    "history.copy": "Copy",
    "history.copied": "Copied",
    "history.copyFailed": "Copy failed",
    "history.private": "Private",
    "history.redacted": "Replacement text is hidden for selected-text privacy.",
    "history.noText": "No text stored for this entry.",
    "history.selectedEdited": "Text edit completed",
    "history.dictationInserted": "Voice input inserted",
    "history.instructedCompleted": "Instructed input completed",
    "history.inputCompleted": "Input completed",
    "history.aiSucceeded": "AI succeeded",
    "history.aiFallback": "AI fallback",
    "history.aiProcessingSucceeded": "AI processing succeeded",
    "history.aiProcessingFailed": "AI processing failed; original ASR used",
    "history.localProcessing": "Local processing",
    "history.localEntry": "Local entry",
    "history.recent": "Recent",
    "history.developerDetails": "Developer details",
    "diagnostics.eyebrow": "Help & Diagnostics",
    "diagnostics.title": "VoiceFlow health",
    "diagnostics.subtitle": "See whether VoiceFlow is working and what to try when it needs attention.",
    "diagnostics.reliability": "Reliability",
    "diagnostics.runtimeSummary": "Runtime summary",
    "diagnostics.attempted": "Attempted",
    "diagnostics.successful": "Successful",
    "diagnostics.failed": "Failed",
    "diagnostics.lateFailures": "Late failures",
    "diagnostics.earlyFailures": "Early failures",
    "diagnostics.directCommits": "Direct commits",
    "diagnostics.clipboardFallback": "Clipboard fallback",
    "diagnostics.selectionReplace": "Selection replace",
    "diagnostics.validation": "Validation",
    "diagnostics.latest": "Latest",
    "diagnostics.sessionResult": "Session result",
    "diagnostics.failure": "Failure",
    "diagnostics.recent": "Recent",
    "diagnostics.recentProblems": "Recent problems",
    "diagnostics.help": "Help",
    "diagnostics.recommendedAction": "Recommended action",
    "diagnostics.noRecentProblems": "No recent problems have been recorded.",
    "diagnostics.hearingProblem": "VoiceFlow had trouble hearing speech in recent attempts.",
    "diagnostics.insertionProblem": "VoiceFlow recognized speech but could not insert text in some recent attempts.",
    "diagnostics.recognitionProblem": "Some recent speech could not be recognized.",
    "diagnostics.generalProblem": "{count} recent attempt{suffix} did not complete.",
    "diagnostics.hearingAction": "Check your microphone and try a short sentence. If quiet speech is missed, make Pause sensitivity more sensitive.",
    "diagnostics.insertionAction": "Try a different text field. If insertion still fails, restart VoiceFlow and try again.",
    "diagnostics.recognitionAction": "Try a shorter sentence and speak clearly, then run one more test.",
    "diagnostics.generalAction": "Try again. Open Advanced diagnostics if the problem continues.",
    "diagnostics.advanced": "Advanced diagnostics",
    "diagnostics.advancedTroubleshooting": "Advanced troubleshooting",
    "diagnostics.rawEvidence": "Raw technical evidence",
    "diagnostics.recentOutcomes": "Recent outcomes",
    "diagnostics.dictation": "Dictation",
    "diagnostics.edit": "Edit",
    "diagnostics.intent": "Instructed",
    "diagnostics.localRefine": "Local + refine",
    "diagnostics.cloudRefine": "Cloud + refine",
    "diagnostics.noSuccessful": "No successful session has been mirrored yet.",
    "diagnostics.nextCompleted": "The next completed session will appear here.",
    "diagnostics.noFailure": "No failure recorded in the current mirrored report.",
    "diagnostics.failureShort": "If something needs attention, the short version appears here.",
    "settings.eyebrow": "Settings",
    "settings.title": "Settings",
    "settings.subtitle": "Control how VoiceFlow listens, writes, and connects.",
    "settings.sectionGeneral": "General",
    "settings.sectionGeneralHelp": "Language and application feedback preferences.",
    "settings.sectionVoiceOutput": "Voice & Output",
    "settings.sectionVoiceOutputHelp": "Recording controls, pause detection, and instructed input.",
    "settings.sectionProvider": "AI Provider",
    "settings.sectionProviderHelp": "Configure optional cloud refinement without exposing credentials.",
    "settings.sectionPrompts": "Prompts",
    "settings.sectionPromptsHelp": "Customize prompts with an explicit save and a built-in reset option.",
    "settings.sectionHistoryPrivacy": "History & Privacy",
    "settings.sectionHistoryPrivacyHelp": "Control local retention and remove stored history.",
    "settings.sectionAdvanced": "Advanced",
    "settings.sectionAdvancedHelp": "Optional diagnostics and technical settings for troubleshooting.",
    "settings.effectImmediate": "Applies after saving",
    "settings.effectNextDictation": "Next dictation",
    "settings.effectNextRequest": "Changes apply to the next AI request.",
    "settings.languageHelp": "Changes the main UI and live voice widget language.",
    "settings.audioCuesHelp": "Play short sounds when recording starts and stops.",
    "settings.shortcutGroupHelp": "Use one shortcut for dictation and selected-text editing.",
    "settings.recordingBehavior": "Recording behavior",
    "settings.recordingBehaviorHelp": "Choose how recording ends and how pauses are detected.",
    "settings.providerConfiguration": "Provider configuration",
    "settings.advancedDetails": "Advanced details",
    "settings.promptCustomizationHelp": "Advanced customization. Changes may affect accuracy and formatting.",
    "settings.retentionPeriod": "Retention period",
    "settings.clearHistoryHelp": "Permanently removes retained local history from this device.",
    "settings.diagnosticsVerbosity": "Diagnostics detail",
    "settings.diagnosticsVerbosityHelp": "Verbose mode records more non-sensitive runtime events for troubleshooting.",
    "settings.diagnosticsStandard": "Standard",
    "settings.diagnosticsVerbose": "Verbose",
    "settings.runtimeInformation": "Runtime information",
    "settings.loadedSettingsFile": "Loaded settings file",
    "settings.settingsWarnings": "Settings warnings",
    "settings.runtimeReportStale": "Last report differs",
    "settings.runtimeReportStaleHelp": "Saved settings apply automatically; this diagnostic report may predate the latest save.",
    "settings.shortcut": "Shortcut",
    "settings.primaryTrigger": "Recording shortcut",
    "settings.modifiers": "Modifiers",
    "settings.key": "Key",
    "settings.mode": "Mode",
    "settings.pushToTalk": "Push-to-talk",
    "settings.toggle": "Toggle",
    "settings.pushToTalkHelp": "Push-to-talk: hold the shortcut to record, release to stop.",
    "settings.toggleHelp": "Toggle: press once to start, press again to stop.",
    "settings.speech": "Speech",
    "settings.aiQuality": "Refine quality",
    "settings.textRefinementBody": "VoiceFlow cleans up dictated text after local speech recognition.",
    "settings.defaultRefinementModel": "Active model is controlled by Provider Settings.",
    "settings.advancedModelSoon": "Edit non-secret provider settings in Advanced AI & prompts.",
    "settings.fast": "Fast",
    "settings.balanced": "Balanced",
    "settings.bestQuality": "Best Quality",
    "settings.qualityHelp": "Provider model selection is managed by the editable AI provider settings.",
    "settings.qualityProfileHelp": "Choose Fast for speed, Balanced for daily use, or Best Quality for more careful cleanup.",
    "settings.qualityMapping": "Quality profile mapping",
    "settings.voiceCommands": "Instructed dictation",
    "settings.experimental": "Experimental",
    "settings.intentNotValidated": "Instructed dictation is available, but still experimental.",
    "settings.intentHelper": "While using the shortcut, say the trigger phrase to translate, rewrite, or structure the following content directly.",
    "settings.wakePhrase": "Trigger phrase",
    "settings.wakePhraseEnabled": "Enable instructed dictation",
    "settings.feedback": "Feedback",
    "settings.listeningCues": "Listening cues",
    "settings.audioCues": "Audio cues",
    "settings.pauseSensitivity": "Pause sensitivity",
    "settings.pauseSensitivityHelp": "Controls how VoiceFlow detects pauses. Conservative reduces false triggers; sensitive captures quieter or shorter speech.",
    "settings.systemLanguage": "System language",
    "settings.shortcutSafe": "{shortcut} stays within the current Windows prototype support set and is less likely to collide with common app menus.",
    "settings.shortcutUnsafe": "{shortcut} may collide with common Windows or app shortcuts. Choose a less common combination if it interferes.",
    "settings.shortcutModifierless": "{key} without a modifier can interrupt normal typing. VoiceFlow will restore a supported shortcut.",
    "settings.shortcutApply": "The saved shortcut is active without restarting VoiceFlow.",
    "settings.shortcutApplyDirty": "Save this shortcut to activate it in the live host.",
    "settings.saveState": "Save state",
    "settings.revert": "Revert",
    "settings.apply": "Apply settings",
    "settings.saveChanges": "Save changes",
    "settings.discardChanges": "Discard changes",
    "settings.saveDetails": "Developer details",
    "settings.advancedConfiguration": "Advanced AI & prompts",
    "settings.advanced": "Advanced",
    "settings.comingSoon": "Coming soon",
    "settings.advancedAi": "Advanced AI",
    "settings.aiProvider": "AI provider",
    "settings.providerIntro": "Configure non-secret provider settings here. API keys stay outside settings.json.",
    "settings.textRefinement": "Text refinement",
    "settings.onOff": "On / Off",
    "settings.provider": "Provider",
    "settings.providerBailian": "Alibaba Cloud Bailian",
    "settings.providerVolcengineArk": "Volcengine Ark",
    "settings.providerTencentHunyuan": "Tencent Hunyuan",
    "settings.providerCustomOpenAi": "Custom OpenAI-compatible",
    "settings.providerPresets": "Provider presets",
    "settings.aliyunPreset": "Aliyun Bailian / DashScope-compatible",
    "settings.customOpenAiPreset": "Custom OpenAI-compatible endpoint",
    "settings.apiKey": "API key",
    "settings.storedCredential": "Stored credential",
    "settings.effectiveApiKey": "Effective API key",
    "settings.effectiveKeySource": "Effective key source",
    "settings.apiKeyStatus": "Configured / Not configured",
    "settings.apiKeyConfigured": "Configured",
    "settings.apiKeyMissing": "Missing key",
    "settings.apiKeySourceProviderEnv": "Environment variable",
    "settings.apiKeySourceLegacyDashscopeEnv": "Legacy DashScope environment variable",
    "settings.apiKeySourceCredentialStore": "Windows Credential Manager",
    "settings.apiKeySourceStoreErrorLastKnownGood": "Last-known-good after credential-store error",
    "settings.apiKeySourceMissing": "Missing",
    "settings.apiKeyHelp": "Stored securely in Windows Credential Manager for the selected provider.",
    "settings.saveApiKey": "Save / Replace API key",
    "settings.clearApiKey": "Clear stored API key",
    "settings.apiKeyBlank": "Paste an API key before saving.",
    "settings.apiKeySaving": "Saving API key...",
    "settings.apiKeySaved": "API key saved for {provider}.",
    "settings.apiKeyClearing": "Clearing stored API key...",
    "settings.apiKeyCleared": "Stored API key cleared for {provider}.",
    "settings.apiKeyClearedEnvActive": "Stored API key cleared for {provider}. Environment variables were not removed and remain active.",
    "settings.apiKeyBridgeRequired": "Open the live Settings URL to manage API keys.",
    "settings.apiKeySaveFailed": "Could not save API key.",
    "settings.apiKeyClearFailed": "Could not clear stored API key.",
    "settings.credentialStored": "Present",
    "settings.credentialMissing": "Missing",
    "settings.credentialError": "Credential store unavailable",
    "settings.overrideApiKey": "An environment API key is active and overrides the stored credential for this provider.",
    "settings.baseUrl": "Base URL",
    "settings.baseUrlHelp": "Leave blank to use the preset default where available.",
    "settings.baseUrlCustomRequired": "Custom providers require an explicit Base URL.",
    "settings.useCustomEndpoint": "Use custom endpoint",
    "settings.providerPresetEndpoint": "Default endpoint: {url}",
    "settings.customEndpointHelp": "Overrides the built-in endpoint for this provider.",
    "settings.resetProviderEndpoint": "Reset to provider default",
    "settings.activeModel": "Active model",
    "settings.activeModelHelp": "Enter the provider or gateway model ID manually.",
    "settings.activeModelHelpBailian": "Model name, for example qwen3.7-plus.",
    "settings.activeModelHelpVolcengine": "Enter a model or endpoint ID.",
    "settings.activeModelHelpTencent": "Model ID, for example hy3-preview.",
    "settings.activeModelHelpCustom": "Enter the provider or gateway model ID manually.",
    "settings.requestTimeout": "Request timeout",
    "settings.timeoutHelp": "Seconds. Leave blank for the 12 second default.",
    "settings.providerEndpoint": "Provider endpoint",
    "settings.modelProfiles": "Model profiles",
    "settings.defaultModelExamples": "Current defaults are shown as examples.",
    "settings.modelNameGuidance": "Model names depend on your configured provider or company gateway.",
    "settings.aiPrivacyNote": "Speech recognition runs locally. When text refinement is enabled, recognized text is sent to the configured AI provider.",
    "settings.testConnection": "Test connection",
    "settings.testConnectionBridgeRequired": "Open the live Settings URL to test the provider.",
    "settings.testConnectionApplyFirst": "Apply provider settings before testing.",
    "settings.testConnectionTesting": "Testing effective provider configuration...",
    "settings.testConnectionSuccess": "Connection test succeeded in {latency} ms.",
    "settings.testConnectionFailed": "Connection test failed: {status}.",
    "settings.testConnectionStale": "Provider settings changed. Apply before testing again.",
    "settings.testConnectionDetails": "Testing {provider} · {model} · {keySource}.",
    "settings.testStatusSuccess": "success",
    "settings.testStatusMissingKey": "missing key",
    "settings.testStatusInvalidConfiguration": "invalid configuration",
    "settings.testStatusAuthenticationFailed": "authentication failed",
    "settings.testStatusEndpointOrModelNotFound": "endpoint or model not found",
    "settings.testStatusRateLimited": "rate limited",
    "settings.testStatusTimeout": "timeout",
    "settings.testStatusNetworkError": "network error",
    "settings.testStatusInvalidResponse": "invalid response",
    "settings.testStatusProviderError": "provider error",
    "settings.effectiveProvider": "Effective provider",
    "settings.effectiveBaseUrl": "Effective base URL",
    "settings.effectiveModel": "Effective model",
    "settings.effectiveTimeout": "Effective timeout",
    "settings.sourceEnv": "environment",
    "settings.sourceSettings": "saved settings",
    "settings.sourceBuiltIn": "built-in default",
    "settings.sourceMissing": "missing",
    "settings.overrideProvider": "Provider is currently overridden by VOICEFLOW_PROVIDER_TYPE.",
    "settings.overrideBaseUrl": "Base URL is currently overridden by an environment variable.",
    "settings.overrideModel": "Model is currently overridden by an environment variable.",
    "settings.overrideTimeout": "Timeout is currently overridden by VOICEFLOW_LLM_REFINE_TIMEOUT_MS.",
    "settings.providerNoteBailian": "Bailian supports preset defaults when Base URL is blank. Existing DASHSCOPE_* environment variables remain supported.",
    "settings.providerNoteVolcengine": "Volcengine Ark uses the preset endpoint when Base URL is blank. Model IDs remain manually editable.",
    "settings.providerNoteTencent": "Tencent Hunyuan uses its OpenAI-compatible preset endpoint. Model IDs remain manually editable.",
    "settings.providerNoteCustom": "Custom providers require an explicit Base URL and send only conservative OpenAI-compatible chat-completions fields.",
    "settings.providerValidationUrl": "Base URL must be http(s), must not contain credentials, query, or fragment, and plain http is limited to localhost.",
    "settings.providerValidationCustomUrl": "Custom OpenAI-compatible provider requires a Base URL.",
    "settings.providerValidationTimeout": "Timeout must be between 1 and 120 seconds.",
    "settings.promptManagement": "Prompt management",
    "settings.promptOverrides": "Prompt overrides",
    "settings.promptStatusPreview": "Review the warning before editing. Each prompt is saved separately.",
    "settings.promptReloadNote": "Saved prompts apply to subsequent requests. UI settings take priority over file overrides; resetting uses built-in defaults without deleting files.",
    "settings.dictationLightCleanupPrompt": "Dictation light cleanup prompt",
    "settings.dictationStructuredCleanupPrompt":
      "Dictation structured cleanup prompt",
    "settings.selectedTextPrompt": "Selected-text edit prompt",
    "settings.instructedDictationPrompt": "Instructed dictation prompt",
    "settings.promptDefault": "Default",
    "settings.promptCustomActive": "Custom active",
    "settings.promptMissingFile": "Missing file",
    "settings.promptEmptyFile": "Empty file",
    "settings.promptUnreadable": "Unreadable",
    "settings.promptEnvVar": "Env var",
    "settings.promptPath": "Path",
    "settings.promptNoPath": "No override path configured.",
    "settings.promptStatusReady": "Prompt override status is loaded from the runtime bridge.",
    "settings.promptReloaded": "Prompt override status refreshed.",
    "settings.promptReloading": "Refreshing prompt override status...",
    "settings.promptReloadFailed": "Could not refresh prompt override status. Start VoiceFlow and try again.",
    "settings.promptBridgeRequired": "Start VoiceFlow to refresh prompt override status.",
    "settings.choosePromptFile": "Choose prompt file",
    "settings.openPromptFile": "Open prompt file",
    "settings.reloadPrompts": "Reload prompts",
    "settings.resetPrompts": "Reset to default",
    "settings.historyStorage": "Local history",
    "settings.historyPrivacy": "History & Privacy",
    "settings.historyPrivacyNote": "History is stored locally on this device.",
    "settings.latest100": "Latest 100 entries",
    "settings.latest500": "Latest 500 entries",
    "settings.latest1000": "Latest 1000 entries",
    "settings.last7Days": "Last 7 days",
    "settings.last30Days": "Last 30 days",
    "settings.unlimited": "Unlimited",
    "settings.unlimitedWarning": "Unlimited history may increase local storage usage.",
    "settings.clearHistory": "Clear History",
    "settings.clearHistoryBridgeRequired": "Start VoiceFlow to clear history.",
    "settings.clearHistoryConfirmTitle": "Clear local history on this device?",
    "settings.clearHistoryConfirmMessage": "This removes retained dictation history. It does not change settings.",
    "settings.clearHistoryClearing": "Clearing history...",
    "settings.clearHistorySuccess": "History cleared.",
    "settings.clearHistoryFailed": "Could not clear history. Start VoiceFlow and try again.",
    "settings.startBridge": "Start VoiceFlow to save settings.",
    "settings.manualCommand": "Manual save command for the same settings.",
    "settings.localDraft": "Local draft",
    "settings.savedSettings": "Saved settings",
    "settings.lastSave": "Last save",
    "settings.terminalFallback": "Terminal fallback",
    "settings.copyHostCommand": "Copy host command",
    "settings.bridgeConnected": "Connected. You can save settings.",
    "settings.bridgeOffline": "Start VoiceFlow to save settings.",
    "settings.noChanges": "All changes saved",
    "settings.unsavedChanges": "There are unsaved changes.",
    "settings.savedApplied": "Saved and applied",
    "settings.savedNextDictation": "Saved. Changes will apply to the next dictation.",
    "settings.savedNextAiRequest": "Saved. Changes will apply to the next AI request.",
    "settings.discardConfirm": "Discard unsaved Settings changes?",
    "settings.unsaved": "Unsaved",
    "settings.inSync": "In sync",
    "settings.draftPending": "Local edits are staged in the page but have not been saved yet.",
    "settings.noDraft": "No unsaved local changes are pending.",
    "settings.noSuggestions": "No apply-ready suggestions",
    "settings.stageApplyReady": "Stage apply-ready",
    "settings.noSettingsChanges": "No settings changes to apply.",
    "settings.applying": "Saving changes...",
    "settings.saved": "Saved",
    "settings.hostUnavailable": "Start VoiceFlow to save settings.",
    "settings.bridgeRequired": "Start VoiceFlow to save settings.",
    "settings.reverted": "Local edits were reverted to the persisted settings.",
    "settings.noFallback": "No fallback host command is available until local changes exist.",
    "settings.fallbackCopied": "Fallback host command copied. You can paste it directly into PowerShell.",
    "settings.fallbackCopyFailed": "Copy failed in this browser context. Select the preview text manually and copy it from there.",
    "summary.fast": "Fast",
    "summary.balanced": "Balanced",
    "summary.bestQuality": "Best Quality",
    "summary.refine": "AI cleanup",
    "summary.audioOn": "Audio cues on",
    "summary.audioOff": "Audio cues muted",
    "summary.standardDiagnostics": "Standard diagnostics",
    "summary.verboseDiagnostics": "Verbose diagnostics",
    "summary.gate": "pause",
    "silence.1": "Very light",
    "silence.2": "Conservative",
    "silence.3": "Balanced",
    "silence.4": "Firm",
    "silence.5": "Strict",
  },
  Chinese: {
    "document.title": "VoiceFlow 设置",
    "brand.subtitle": "语音输入法",
    "brand.version": "v0.1.2-beta",
    "nav.overview": "概览",
    "nav.history": "历史",
    "nav.settings": "设置",
    "nav.diagnostics": "诊断",
    "common.localOnly": "仅本地",
    "common.total": "总计",
    "common.selectedText": "选中文本",
    "common.unavailable": "不可用",
    "common.idle": "空闲",
    "overview.ready": "快速查看你的本地语音活动。",
    "overview.readyLive": "本地活动已是最新。",
    "overview.readySaved": "正在显示最近保存的活动。",
    "overview.connectedNow": "当前已连接到 VoiceFlow",
    "overview.offlineNow": "VoiceFlow 当前离线",
    "overview.currentState": "当前设置",
    "overview.latestReport": "最近诊断报告",
    "overview.usageSummary": "近期活动与累计数据",
    "overview.lastSevenDays": "最近 7 天",
    "overview.allTimeTotals": "累计数据",
    "overview.shortcut": "快捷键",
    "overview.usage": "使用情况",
    "overview.sevenDayActivity": "最近 7 天的输入字数与预计节省时间",
    "overview.wordsInWeek": "{count} 字",
    "overview.insertedText": "输入字数",
    "overview.insertedTextNote": "所有成功完成的会话",
    "overview.totalDictationDuration": "总听写时长",
    "overview.totalDictationDurationNote": "实际录音时长",
    "overview.timeSaved": "节省时间",
    "overview.timeSavedNote": "与手动输入相比",
    "overview.averageDictationSpeed": "平均听写速度",
    "overview.averageDictationSpeedNote": "每分钟输入字数",
    "overview.speedSuffix": "字/分钟",
    "overview.chartTooltip": "{words} 字 · 节省 {time}",
    "overview.chartAria": "最近 7 天的输入字数与预计节省时间",
    "overview.hidePreviews": "隐藏最近听写预览",
    "overview.showPreviews": "显示最近听写预览",
    "overview.success": "成功",
    "overview.recentDictations": "最近听写",
    "overview.recentDictationsNote": "最近保存在本地的听写内容",
    "overview.viewHistory": "查看历史",
    "overview.noRecentDictations": "最近听写会显示在这里。",
    "overview.currentSetup": "当前配置",
    "overview.currentSetupNote": "下一次听写所需的核心配置",
    "overview.speechRecognition": "语音识别",
    "overview.local": "本地",
    "overview.textRefinement": "文本润色",
    "overview.localCleanup": "仅本地整理",
    "overview.noUsage": "完成一次听写后，这里会显示本地活动趋势。",
    "history.eyebrow": "历史",
    "history.title": "历史",
    "history.subtitle": "记录保存在本机；选中文本编辑默认隐藏内容。",
    "history.localHistory": "本地历史",
    "history.latestEntries": "最新记录",
    "history.empty": "最近的语音输入会显示在这里。",
    "history.emptyWithPolicy": "记录保存在本机 · {policy}",
    "history.localPrivate": "最近输入记录保存在本机。选中文本编辑默认会隐藏内容。",
    "history.staticSnapshot": "记录保存在本机 · 显示最近可用的历史记录",
    "history.liveEntries": "记录保存在本机 · 已保留 {count} 条 · {policy}",
    "history.copy": "复制",
    "history.copied": "已复制",
    "history.copyFailed": "复制失败",
    "history.private": "私密",
    "history.redacted": "为保护选中文本隐私，替换内容已隐藏。",
    "history.noText": "此记录未保存文本。",
    "history.selectedEdited": "已完成文本编辑",
    "history.dictationInserted": "已插入语音输入",
    "history.instructedCompleted": "已完成指令输入",
    "history.inputCompleted": "输入已完成",
    "history.aiSucceeded": "AI 处理成功",
    "history.aiFallback": "AI 回退",
    "history.aiProcessingSucceeded": "AI 处理成功",
    "history.aiProcessingFailed": "AI 处理失败；已使用原始识别结果",
    "history.localProcessing": "本地处理",
    "history.localEntry": "本地记录",
    "history.recent": "最近",
    "history.developerDetails": "开发者详情",
    "diagnostics.eyebrow": "帮助与诊断",
    "diagnostics.title": "VoiceFlow 健康状态",
    "diagnostics.subtitle": "查看 VoiceFlow 是否正常工作，以及需要关注时可以尝试什么。",
    "diagnostics.reliability": "可靠性",
    "diagnostics.runtimeSummary": "运行摘要",
    "diagnostics.attempted": "尝试",
    "diagnostics.successful": "成功",
    "diagnostics.failed": "失败",
    "diagnostics.lateFailures": "后期失败",
    "diagnostics.earlyFailures": "早期失败",
    "diagnostics.directCommits": "直接输入",
    "diagnostics.clipboardFallback": "剪贴板回退",
    "diagnostics.selectionReplace": "选区替换",
    "diagnostics.validation": "验证",
    "diagnostics.latest": "最新",
    "diagnostics.sessionResult": "会话结果",
    "diagnostics.failure": "失败",
    "diagnostics.recent": "最近",
    "diagnostics.recentProblems": "最近问题",
    "diagnostics.help": "帮助",
    "diagnostics.recommendedAction": "建议操作",
    "diagnostics.noRecentProblems": "没有记录到最近问题。",
    "diagnostics.hearingProblem": "VoiceFlow 最近几次尝试没有清楚听到语音。",
    "diagnostics.insertionProblem": "VoiceFlow 已识别语音，但最近有些尝试未能插入文本。",
    "diagnostics.recognitionProblem": "最近有些语音未能识别。",
    "diagnostics.generalProblem": "最近有 {count} 次尝试未完成。",
    "diagnostics.hearingAction": "检查麦克风并尝试说一个短句。如果较轻的语音经常漏掉，请提高停顿灵敏度。",
    "diagnostics.insertionAction": "尝试另一个文本框。如果仍无法插入，请重启 VoiceFlow 后再试。",
    "diagnostics.recognitionAction": "尝试更短的句子并清晰说话，然后再测试一次。",
    "diagnostics.generalAction": "请再试一次。如果问题持续，请打开高级诊断。",
    "diagnostics.advanced": "高级诊断",
    "diagnostics.advancedTroubleshooting": "高级故障排查",
    "diagnostics.rawEvidence": "原始技术证据",
    "diagnostics.recentOutcomes": "最近结果",
    "diagnostics.dictation": "听写",
    "diagnostics.edit": "编辑",
    "diagnostics.intent": "指令式",
    "diagnostics.localRefine": "本地 + 优化",
    "diagnostics.cloudRefine": "云端 + 优化",
    "diagnostics.noSuccessful": "尚未同步成功会话。",
    "diagnostics.nextCompleted": "下一次完成的会话会显示在这里。",
    "diagnostics.noFailure": "当前同步报告中没有失败记录。",
    "diagnostics.failureShort": "如果有需要关注的问题，简短说明会显示在这里。",
    "settings.eyebrow": "设置",
    "settings.title": "设置",
    "settings.subtitle": "控制 VoiceFlow 的聆听、输出与连接方式。",
    "settings.sectionGeneral": "通用",
    "settings.sectionGeneralHelp": "设置界面语言和应用反馈偏好。",
    "settings.sectionVoiceOutput": "语音与输出",
    "settings.sectionVoiceOutputHelp": "设置录音控制、停顿判断和指令式输入。",
    "settings.sectionProvider": "AI 服务",
    "settings.sectionProviderHelp": "配置可选的云端润色，不会显示任何密钥内容。",
    "settings.sectionPrompts": "提示词",
    "settings.sectionPromptsHelp": "自定义提示词，明确保存，并可恢复内置默认值。",
    "settings.sectionHistoryPrivacy": "历史与隐私",
    "settings.sectionHistoryPrivacyHelp": "管理本地保留策略并清除历史记录。",
    "settings.sectionAdvanced": "高级设置",
    "settings.sectionAdvancedHelp": "用于故障排查的可选诊断与技术设置。",
    "settings.effectImmediate": "保存后生效",
    "settings.effectNextDictation": "下次听写生效",
    "settings.effectNextRequest": "更改将在下一次 AI 请求时生效。",
    "settings.languageHelp": "更改主界面和实时语音浮窗的语言。",
    "settings.audioCuesHelp": "在录音开始和结束时播放简短提示音。",
    "settings.shortcutGroupHelp": "听写和选中文本编辑共用一个快捷键。",
    "settings.recordingBehavior": "录音行为",
    "settings.recordingBehaviorHelp": "选择录音结束方式和停顿判断方式。",
    "settings.providerConfiguration": "服务配置",
    "settings.advancedDetails": "高级详情",
    "settings.promptCustomizationHelp": "高级自定义。修改可能影响准确性与输出格式。",
    "settings.retentionPeriod": "保留期限",
    "settings.clearHistoryHelp": "永久删除此设备上保留的本地历史记录。",
    "settings.diagnosticsVerbosity": "诊断详细程度",
    "settings.diagnosticsVerbosityHelp": "详细模式会记录更多不含敏感内容的运行事件，用于故障排查。",
    "settings.diagnosticsStandard": "标准",
    "settings.diagnosticsVerbose": "详细",
    "settings.runtimeInformation": "运行时信息",
    "settings.loadedSettingsFile": "已加载的设置文件",
    "settings.settingsWarnings": "设置警告",
    "settings.runtimeReportStale": "上次报告存在差异",
    "settings.runtimeReportStaleHelp": "已保存设置会自动生效；此诊断报告可能早于最近一次保存。",
    "settings.shortcut": "快捷键",
    "settings.primaryTrigger": "录音快捷键",
    "settings.modifiers": "修饰键",
    "settings.key": "按键",
    "settings.mode": "模式",
    "settings.pushToTalk": "按住说话",
    "settings.toggle": "开关录音",
    "settings.pushToTalkHelp": "按住说话：按住快捷键录音，松开结束。",
    "settings.toggleHelp": "开关录音：按一次开始，再按一次结束。",
    "settings.speech": "语音",
    "settings.aiQuality": "润色质量",
    "settings.textRefinementBody": "VoiceFlow 会在本地语音识别后整理文本，让输出更自然、清晰。",
    "settings.defaultRefinementModel": "当前模型由服务商设置控制。",
    "settings.advancedModelSoon": "高级模型配置暂未开放。",
    "settings.fast": "快速",
    "settings.balanced": "均衡",
    "settings.bestQuality": "最佳质量",
    "settings.qualityHelp": "服务商模型选择由可编辑的 AI 服务设置管理。",
    "settings.qualityProfileHelp": "快速更省时，均衡适合日常使用，最佳质量会更细致地整理文本。",
    "settings.qualityMapping": "质量档位对应关系",
    "settings.voiceCommands": "指令式听写",
    "settings.experimental": "实验性",
    "settings.intentNotValidated": "指令式听写已可用，仍为实验性功能。",
    "settings.intentHelper": "按住快捷键录音时，说出触发词后，可直接翻译、改写或整理后续内容。",
    "settings.wakePhrase": "触发词",
    "settings.wakePhraseEnabled": "启用指令式听写",
    "settings.feedback": "反馈",
    "settings.listeningCues": "聆听提示",
    "settings.audioCues": "声音提示",
    "settings.pauseSensitivity": "停顿判断",
    "settings.pauseSensitivityHelp": "用于判断停顿。更保守可减少误触发，更灵敏可捕捉更轻或更短的语音。",
    "settings.systemLanguage": "系统语言",
    "settings.shortcutSafe": "{shortcut} 属于当前 Windows 原型支持范围，较少与常见应用菜单冲突。",
    "settings.shortcutUnsafe": "{shortcut} 可能与常见 Windows 或应用快捷键冲突。如果影响使用，请选择更少见的组合。",
    "settings.shortcutModifierless": "{key} 没有修饰键，可能会干扰正常输入。VoiceFlow 会恢复为支持的快捷键。",
    "settings.shortcutApply": "已保存的快捷键无需重启 VoiceFlow 即可生效。",
    "settings.shortcutApplyDirty": "保存此快捷键后，live host 会自动启用。",
    "settings.saveState": "保存状态",
    "settings.revert": "还原",
    "settings.apply": "应用设置",
    "settings.saveChanges": "保存更改",
    "settings.discardChanges": "放弃更改",
    "settings.saveDetails": "开发者详情",
    "settings.advancedConfiguration": "高级 AI 与提示词",
    "settings.advanced": "高级",
    "settings.comingSoon": "暂未开放",
    "settings.advancedAi": "高级 AI",
    "settings.aiProvider": "AI 服务",
    "settings.providerIntro": "在这里配置非密钥的服务商设置。API 密钥不会写入 settings.json。",
    "settings.textRefinement": "文本润色",
    "settings.onOff": "开启 / 关闭",
    "settings.provider": "服务提供方",
    "settings.providerBailian": "阿里云百炼",
    "settings.providerVolcengineArk": "火山方舟",
    "settings.providerTencentHunyuan": "腾讯混元",
    "settings.providerCustomOpenAi": "自定义 OpenAI 兼容服务",
    "settings.providerPresets": "服务商预设",
    "settings.aliyunPreset": "阿里云百炼 / DashScope 兼容",
    "settings.customOpenAiPreset": "自定义 OpenAI 兼容端点",
    "settings.apiKey": "API 密钥",
    "settings.apiKeyStatus": "已配置 / 未配置",
    "settings.apiKeyConfigured": "已配置",
    "settings.apiKeyMissing": "缺少密钥",
    "settings.apiKeySourceProviderEnv": "通用环境变量",
    "settings.apiKeySourceLegacyDashscopeEnv": "旧 DashScope 环境变量",
    "settings.apiKeySourceCredentialStore": "已存储凭据",
    "settings.apiKeySourceCredentialStoreError": "凭据存储错误",
    "settings.apiKeySourceMissing": "缺失",
    "settings.apiKeyHelp": "密钥会按当前服务商安全存入 Windows Credential Manager。",
    "settings.saveApiKey": "保存 / 替换 API 密钥",
    "settings.clearApiKey": "清除已存储 API 密钥",
    "settings.apiKeyBlank": "请先粘贴 API 密钥。",
    "settings.apiKeySaving": "正在保存 API 密钥...",
    "settings.apiKeySaved": "API 密钥已安全保存。环境变量仍会覆盖已存储凭据。",
    "settings.apiKeyClearing": "正在清除已存储 API 密钥...",
    "settings.apiKeyCleared": "已清除存储的 API 密钥。如果环境变量仍存在，它会继续生效。",
    "settings.apiKeyBridgeRequired": "请打开实时 Settings URL 来管理 API 密钥。",
    "settings.apiKeySaveFailed": "无法保存 API 密钥。",
    "settings.apiKeyClearFailed": "无法清除已存储 API 密钥。",
    "settings.credentialStored": "已存储凭据",
    "settings.credentialMissing": "没有已存储凭据",
    "settings.credentialError": "凭据存储不可用",
    "settings.overrideApiKey": "环境变量 API 密钥正在生效，并覆盖此服务商的已存储凭据。",
    "settings.baseUrl": "基础 URL",
    "settings.baseUrlHelp": "留空时会尽量使用该预设的默认端点。",
    "settings.baseUrlCustomRequired": "自定义服务商需要填写明确的基础 URL。",
    "settings.useCustomEndpoint": "使用自定义服务地址",
    "settings.providerPresetEndpoint": "默认地址：{url}",
    "settings.customEndpointHelp": "覆盖当前服务商的内置地址。",
    "settings.resetProviderEndpoint": "恢复默认地址",
    "settings.activeModel": "当前模型",
    "settings.activeModelHelp": "手动输入服务商或公司网关的模型 ID。",
    "settings.activeModelHelpBailian": "模型名称，例如 qwen3.7-plus。",
    "settings.activeModelHelpVolcengine": "请输入模型或接入点 ID。",
    "settings.activeModelHelpTencent": "模型 ID，例如 hy3-preview。",
    "settings.activeModelHelpCustom": "手动输入服务商或公司网关的模型 ID。",
    "settings.requestTimeout": "请求超时",
    "settings.timeoutHelp": "以秒计。留空使用 12 秒默认值。",
    "settings.providerEndpoint": "服务端点",
    "settings.modelProfiles": "模型档位",
    "settings.defaultModelExamples": "以下为当前默认示例。",
    "settings.modelNameGuidance": "模型名称取决于你配置的服务商或公司网关。",
    "settings.aiPrivacyNote": "语音识别在本机完成。开启文本润色时，识别后的文本会发送到你配置的 AI 服务。",
    "settings.testConnection": "测试连接",
    "settings.testConnectionBridgeRequired": "请打开实时 Settings URL 后再测试服务商。",
    "settings.testConnectionApplyFirst": "请先应用服务商设置再测试。",
    "settings.testConnectionTesting": "正在测试实际生效的服务商配置...",
    "settings.testConnectionSuccess": "连接测试成功，用时 {latency} ms。",
    "settings.testConnectionFailed": "连接测试失败：{status}。",
    "settings.testConnectionStale": "服务商设置已更改，请应用后再测试。",
    "settings.testConnectionDetails": "正在测试 {provider} · {model} · {keySource}。",
    "settings.testStatusSuccess": "成功",
    "settings.testStatusMissingKey": "缺少 API 密钥",
    "settings.testStatusInvalidConfiguration": "配置无效",
    "settings.testStatusAuthenticationFailed": "认证失败",
    "settings.testStatusEndpointOrModelNotFound": "端点或模型不存在",
    "settings.testStatusRateLimited": "达到限流",
    "settings.testStatusTimeout": "超时",
    "settings.testStatusNetworkError": "网络错误",
    "settings.testStatusInvalidResponse": "响应格式无效",
    "settings.testStatusProviderError": "服务商错误",
    "settings.effectiveProvider": "实际服务商",
    "settings.effectiveBaseUrl": "实际基础 URL",
    "settings.effectiveModel": "实际模型",
    "settings.effectiveTimeout": "实际超时",
    "settings.sourceEnv": "环境变量",
    "settings.sourceSettings": "已保存设置",
    "settings.sourceBuiltIn": "内置默认值",
    "settings.sourceMissing": "缺失",
    "settings.overrideProvider": "服务商当前被 VOICEFLOW_PROVIDER_TYPE 环境变量覆盖。",
    "settings.overrideBaseUrl": "基础 URL 当前被环境变量覆盖。",
    "settings.overrideModel": "模型当前被环境变量覆盖。",
    "settings.overrideTimeout": "超时当前被 VOICEFLOW_LLM_REFINE_TIMEOUT_MS 覆盖。",
    "settings.providerNoteBailian": "百炼在基础 URL 留空时可使用预设默认值。现有 DASHSCOPE_* 环境变量仍然支持。",
    "settings.providerNoteVolcengine": "火山方舟在基础 URL 留空时使用预设端点。模型 ID 仍需手动填写。",
    "settings.providerNoteTencent": "腾讯混元使用预设的 OpenAI 兼容地址。模型 ID 仍可手动填写。",
    "settings.providerNoteCustom": "自定义服务商需要明确的基础 URL，并且只发送保守的 OpenAI 兼容 chat-completions 字段。",
    "settings.providerValidationUrl": "基础 URL 必须是 http(s)，不能包含凭据、查询或 fragment；普通 http 仅限 localhost。",
    "settings.providerValidationCustomUrl": "自定义 OpenAI 兼容服务商需要填写基础 URL。",
    "settings.providerValidationTimeout": "超时必须在 1 到 120 秒之间。",
    "settings.promptManagement": "提示词管理",
    "settings.promptOverrides": "提示词覆盖",
    "settings.promptStatusPreview": "编辑前请阅读风险提醒。每套提示词单独保存。",
    "settings.promptReloadNote": "保存后用于后续请求。UI 设置优先于文件覆盖；恢复默认使用内置版本，不删除原文件。",
    "settings.dictationLightCleanupPrompt": "听写轻度清理提示词",
    "settings.dictationStructuredCleanupPrompt": "听写结构化清理提示词",
    "settings.selectedTextPrompt": "选中文本编辑提示词",
    "settings.instructedDictationPrompt": "指令式听写提示词",
    "settings.promptDefault": "默认",
    "settings.promptCustomActive": "自定义已启用",
    "settings.promptMissingFile": "文件缺失",
    "settings.promptEmptyFile": "空文件",
    "settings.promptUnreadable": "无法读取",
    "settings.promptEnvVar": "环境变量",
    "settings.promptPath": "路径",
    "settings.promptNoPath": "未配置覆盖路径。",
    "settings.promptStatusReady": "提示词覆盖状态已从运行时桥接加载。",
    "settings.promptReloaded": "提示词覆盖状态已刷新。",
    "settings.promptReloading": "正在刷新提示词覆盖状态...",
    "settings.promptReloadFailed": "无法刷新提示词覆盖状态。请启动 VoiceFlow 后重试。",
    "settings.promptBridgeRequired": "需要启动 VoiceFlow 才能刷新提示词覆盖状态。",
    "settings.choosePromptFile": "选择提示词文件",
    "settings.openPromptFile": "打开提示词文件",
    "settings.reloadPrompts": "重新加载提示词",
    "settings.resetPrompts": "恢复默认",
    "settings.historyStorage": "本地历史",
    "settings.historyPrivacy": "历史记录与隐私",
    "settings.historyPrivacyNote": "历史记录仅保存在本机。",
    "settings.latest100": "最近 100 条",
    "settings.latest500": "最近 500 条",
    "settings.latest1000": "最近 1000 条",
    "settings.last7Days": "最近 7 天",
    "settings.last30Days": "最近 30 天",
    "settings.unlimited": "不设上限",
    "settings.unlimitedWarning": "无限保留可能增加本机存储占用。",
    "settings.clearHistory": "清除历史记录",
    "settings.clearHistoryBridgeRequired": "需要启动 VoiceFlow 才能清除历史记录。",
    "settings.clearHistoryConfirmTitle": "清除此设备上的本地历史记录？",
    "settings.clearHistoryConfirmMessage": "这会删除已保留的听写历史记录，不会更改设置。",
    "settings.clearHistoryClearing": "正在清除历史记录...",
    "settings.clearHistorySuccess": "历史记录已清除。",
    "settings.clearHistoryFailed": "无法清除历史记录。请启动 VoiceFlow 后重试。",
    "settings.startBridge": "需要启动 VoiceFlow 才能保存设置。",
    "settings.manualCommand": "用于保存相同设置的手动命令。",
    "settings.localDraft": "本地草稿",
    "settings.savedSettings": "已保存设置",
    "settings.lastSave": "上次保存",
    "settings.terminalFallback": "终端备用命令",
    "settings.copyHostCommand": "复制主机命令",
    "settings.bridgeConnected": "已连接，可以保存设置。",
    "settings.bridgeOffline": "需要启动 VoiceFlow 才能保存设置。",
    "settings.noChanges": "所有更改均已保存",
    "settings.unsavedChanges": "有未保存更改。",
    "settings.savedApplied": "已保存并生效",
    "settings.savedNextDictation": "已保存，将在下次听写生效",
    "settings.savedNextAiRequest": "已保存，将在下次 AI 请求中生效",
    "settings.discardConfirm": "放弃尚未保存的设置更改？",
    "settings.unsaved": "未保存",
    "settings.inSync": "已同步",
    "settings.draftPending": "页面中的本地编辑尚未保存。",
    "settings.noDraft": "没有待保存的本地更改。",
    "settings.noSuggestions": "没有可应用建议",
    "settings.stageApplyReady": "暂存可应用项",
    "settings.noSettingsChanges": "没有需要应用的设置更改。",
    "settings.applying": "正在保存更改...",
    "settings.saved": "已保存",
    "settings.hostUnavailable": "需要启动 VoiceFlow 才能保存设置。",
    "settings.bridgeRequired": "需要启动 VoiceFlow 才能保存设置。",
    "settings.reverted": "本地编辑已还原为已保存设置。",
    "settings.noFallback": "本地没有更改时不会生成备用主机命令。",
    "settings.fallbackCopied": "备用主机命令已复制，可直接粘贴到 PowerShell。",
    "settings.fallbackCopyFailed": "当前浏览器环境无法复制。请手动选中预览文本并复制。",
    "summary.fast": "快速",
    "summary.balanced": "均衡",
    "summary.bestQuality": "最佳质量",
    "summary.refine": "AI 清理",
    "summary.audioOn": "声音提示已开",
    "summary.audioOff": "声音提示静音",
    "summary.standardDiagnostics": "标准诊断",
    "summary.verboseDiagnostics": "详细诊断",
    "summary.gate": "停顿",
    "silence.1": "很灵敏",
    "silence.2": "保守",
    "silence.3": "均衡",
    "silence.4": "偏严格",
    "silence.5": "严格",
  },
};

const statusMessageKeys = {
  "No settings changes to apply.": "settings.noSettingsChanges",
  "Saving changes...": "settings.applying",
  "All changes saved.": "settings.saved",
  "Start VoiceFlow to save settings.": "settings.bridgeRequired",
  "Local edits were reverted to the persisted settings.": "settings.reverted",
  "No fallback host command is available until local changes exist.": "settings.noFallback",
  "Fallback host command copied. You can paste it directly into PowerShell.":
    "settings.fallbackCopied",
  "Copy failed in this browser context. Select the preview text manually and copy it from there.":
    "settings.fallbackCopyFailed",
};

function currentLanguage() {
  return normalizeSystemLanguageValue(state.systemLanguage);
}

function t(key, fallback = key) {
  const language = currentLanguage();
  return translations[language]?.[key] ?? translations.English[key] ?? fallback;
}

function tf(key, replacements, fallback = key) {
  return Object.entries(replacements).reduce(
    (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
    t(key, fallback),
  );
}

function silenceGateLabel(level) {
  return t(`silence.${level}`, translations.English[`silence.${level}`] ?? String(level));
}

function qualityDisplayLabel(quality) {
  switch (quality) {
    case "Fast":
      return t("summary.fast");
    case "Best Quality":
    case "BestQuality":
      return t("summary.bestQuality");
    case "Balanced":
    default:
      return t("summary.balanced");
  }
}

function renderSettingsSubnavLabels() {
  if (!settingsUiModel) {
    return;
  }
  settingsSectionButtons.forEach((button) => {
    button.textContent = settingsUiModel.sectionLabel(
      button.dataset.settingsSection,
      currentLanguage(),
    );
  });
}

function applyLocalization() {
  document.documentElement.lang = currentLanguage() === "Chinese" ? "zh-CN" : "en";
  document.title = t("document.title");
  document.querySelectorAll("[data-i18n]").forEach((element) => {
    const key = element.dataset.i18n;
    element.textContent = t(key, element.textContent);
  });
  renderSettingsSubnavLabels();
  renderSettingsHelperText();
  renderDiagnosticsStaticCopy();
}

function ensureSettingsHelperText() {
  const modeSegment = document.querySelector('.segmented[data-segment="mode"]');
  if (modeSegment && !document.getElementById("mode-helper")) {
    const helper = document.createElement("small");
    helper.id = "mode-helper";
    helper.className = "field-helper mode-helper";
    modeSegment.insertAdjacentElement("afterend", helper);
  }

  const experimentalNote = document.querySelector(".experimental-note");
  if (experimentalNote && !document.getElementById("intent-helper")) {
    const helper = document.createElement("p");
    helper.id = "intent-helper";
    helper.className = "report-note experimental-helper";
    experimentalNote.insertAdjacentElement("afterend", helper);
  }

  if (silenceLabel && !document.getElementById("silence-helper")) {
    const helper = document.createElement("small");
    helper.id = "silence-helper";
    helper.className = "field-helper silence-helper";
    silenceLabel.insertAdjacentElement("afterend", helper);
  }
}

function renderSettingsHelperText() {
  ensureSettingsHelperText();

  const modeHelper = document.getElementById("mode-helper");
  if (modeHelper) {
    modeHelper.textContent = `${t("settings.pushToTalkHelp")} ${t("settings.toggleHelp")}`;
  }

  const intentHelper = document.getElementById("intent-helper");
  if (intentHelper) {
    intentHelper.textContent = t("settings.intentHelper");
  }

  const silenceHelper = document.getElementById("silence-helper");
  if (silenceHelper) {
    silenceHelper.textContent = t("settings.pauseSensitivityHelp");
  }
}

function renderDiagnosticsStaticCopy() {
  const avgTotalLabel = document
    .getElementById("avg-total-latency")
    ?.closest(".timing-cell")
    ?.querySelector("span");
  if (avgTotalLabel) {
    avgTotalLabel.textContent = "Avg E2E";
  }

  const wakePhraseActionLabel = document
    .getElementById("wake-phrase-action-mix")
    ?.closest("div")
    ?.querySelector(".report-note");
  if (wakePhraseActionLabel) {
    wakePhraseActionLabel.textContent =
      currentLanguage() === "Chinese"
        ? "指令式听写动作"
        : "Instructed dictation actions";
  }
}

function localizeStatusMessage(message) {
  const key = statusMessageKeys[message];
  return key ? t(key, message) : message;
}

const segmentValueMap = {
  systemLanguage: {
    English: "English",
    Chinese: "Chinese",
  },
  mode: {
    PushToTalk: "PushToTalk",
    Toggle: "Toggle",
  },
  quality: {
    Fast: "Fast",
    Balanced: "Balanced",
    "Best Quality": "BestQuality",
  },
  verbosity: {
    Standard: "Standard",
    Verbose: "Verbose",
  },
};

const silenceLabels = {
  1: "Very light",
  2: "Conservative",
  3: "Balanced",
  4: "Firm",
  5: "Strict",
};

const panelDriftFields = {
  shortcut: ["Primary shortcut", "Shortcut mode"],
  speech: ["Refine quality", "Instructed dictation", "Trigger phrase"],
  feedback: ["Audio cues", "Silence gate"],
  diagnostics: ["Diagnostics verbosity"],
};

function toSelectedTextActionLabel(action) {
  switch (action) {
    case "Uppercase":
      return "Uppercase";
    case "Lowercase":
      return "Lowercase";
    case "TitleCase":
      return "Title case";
    case "SentenceCase":
      return "Sentence case";
    case "SnakeCase":
      return "Snake case";
    case "KebabCase":
      return "Kebab case";
    case "CamelCase":
      return "Camel case";
    case "PascalCase":
      return "Pascal case";
    case "ConstantCase":
      return "Constant case";
    case "InlineCode":
      return "Inline code";
    case "CodeBlock":
      return "Code block";
    case "StripCodeFence":
      return "Strip code fence";
    case "MarkdownBold":
      return "Markdown bold";
    case "MarkdownItalic":
      return "Markdown italic";
    case "StripMarkdownEmphasis":
      return "Strip markdown emphasis";
    case "WrapInQuotes":
      return "Wrap in quotes";
    case "BulletList":
      return "Bullet list";
    case "Checklist":
      return "Checklist";
    case "QuoteBlock":
      return "Quote block";
    case "NumberedList":
      return "Numbered list";
    case "SortLines":
      return "Sort lines";
    case "DeduplicateLines":
      return "Deduplicate lines";
    case "RemoveEmptyLines":
      return "Remove empty lines";
    case "CommaSeparated":
      return "Comma-separated";
    case "PipeSeparated":
      return "Pipe-separated";
    case "TabSeparated":
      return "Tab-separated";
    case "SemicolonSeparated":
      return "Semicolon-separated";
    case "JsonArray":
      return "JSON array";
    case "QuotedCsv":
      return "Quoted CSV";
    case "SqlInList":
      return "SQL IN list";
    case "YamlList":
      return "YAML list";
    case "YamlMapping":
      return "YAML mapping";
    case "MarkdownTable":
      return "Markdown table";
    case "HeaderBlock":
      return "Header block";
    case "JsonObject":
      return "JSON object";
    case "EnvBlock":
      return "Env block";
    case "QueryString":
      return "Query string";
    case "TomlTable":
      return "TOML table";
    case "ShellExports":
      return "Shell exports";
    case "PowershellEnv":
      return "PowerShell env";
    case "CurlHeaders":
      return "curl headers";
    case "PythonDict":
      return "Python dict";
    case "JavascriptObject":
      return "JavaScript object";
    case "RubyHash":
      return "Ruby hash";
    case "SqlValuesRows":
      return "SQL VALUES rows";
    case "StripListMarkers":
      return "Strip list markers";
    case "SentencePerLine":
      return "Sentence per line";
    case "MarkdownHeading":
      return "Markdown heading";
    case "SingleParagraph":
      return "Single paragraph";
    case "CleanupSpacing":
      return "Spacing cleanup";
    case "PolishWriting":
      return "Polish writing";
    case "ConciseRewrite":
      return "Concise rewrite";
    case "FormalRewrite":
      return "Formal rewrite";
    case "BulletSummary":
      return "Bullet summary";
    case "PromptScaffold":
      return "Prompt scaffold";
    case "GeneralProviderEdit":
      return "General edit";
    default:
      return null;
  }
}

function toWakePhraseActionLabel(action) {
  switch (action) {
    case "DraftEmail":
      return "Draft email";
    case "Summarize":
      return "Summarize";
    case "BulletPlan":
      return "Bullet plan";
    case "Rewrite":
      return "Rewrite";
    case "Checklist":
      return "Checklist";
    case "ReplyMessage":
      return "Reply draft";
    case "GeneralDraft":
      return "General draft";
    default:
      return null;
  }
}

function renderSummary() {
  const shortcutLabel = buildShortcutLabel(
    state.primaryShortcutModifiers,
    state.primaryShortcutKey,
  );
  const shortcutRisk = describeShortcutRisk(
    state.primaryShortcutModifiers,
    state.primaryShortcutKey,
  );
  const summaryLabel = summary?.querySelector("dt, span");
  const summaryValue = summary?.querySelector("dd, strong");
  if (summaryLabel) {
    summaryLabel.textContent = t("overview.shortcut");
  }
  if (summaryValue) {
    summaryValue.textContent = shortcutLabel;
  }
  if (overviewSetupShortcut) {
    overviewSetupShortcut.textContent = shortcutLabel;
  }

  silenceLabel.textContent = silenceGateLabel(state.silenceGate);
  shortcutWarning.textContent = shortcutRisk.message;
  shortcutWarning.classList.remove("is-risky", "is-stable");
  shortcutWarning.classList.add(shortcutRisk.tone);
  shortcutWarningNote.textContent = dirtyFields.has("primaryShortcut")
    ? t("settings.shortcutApplyDirty")
    : t("settings.shortcutApply");
  renderSaveCommand();
  renderActionState();
}

function settingsFieldHasChanges(fieldName) {
  switch (fieldName) {
    case "systemLanguage":
      return (
        normalizeSystemLanguageValue(state.systemLanguage) !==
        normalizeSystemLanguageValue(persistedState.systemLanguage)
      );
    case "primaryShortcut":
      return (
        canonicalizeShortcutKey(state.primaryShortcutKey) !==
          canonicalizeShortcutKey(persistedState.primaryShortcutKey) ||
        canonicalizeModifiers(state.primaryShortcutModifiers).join("|") !==
          canonicalizeModifiers(persistedState.primaryShortcutModifiers).join("|")
      );
    case "mode":
      return state.mode !== persistedState.mode;
    case "quality":
      return normalizeQualityValue(state.quality) !== normalizeQualityValue(persistedState.quality);
    case "silenceGate":
      return state.silenceGate !== persistedState.silenceGate;
    case "verbosity":
      return state.verbosity !== persistedState.verbosity;
    case "audioFeedback":
      return state.audioFeedback !== persistedState.audioFeedback;
    case "wakePhraseEnabled":
      return state.wakePhraseEnabled !== persistedState.wakePhraseEnabled;
    case "wakePhrase":
      return state.wakePhrase !== persistedState.wakePhrase;
    case "historyRetention":
      return state.historyRetention !== persistedState.historyRetention;
    case "provider":
      return buildProviderUpdatePayload() !== null;
    default:
      return false;
  }
}

function clearSaveFeedback() {
  lastSaveFeedback = null;
  if (saveFeedbackTimer !== null) {
    window.clearTimeout(saveFeedbackTimer);
    saveFeedbackTimer = null;
  }
}

function markDirty(fieldName) {
  clearSaveFeedback();
  if (settingsFieldHasChanges(fieldName)) {
    dirtyFields.add(fieldName);
  } else {
    dirtyFields.delete(fieldName);
  }
  if (fieldName === "provider") {
    markProviderTestStale();
  }
  renderSaveCommand();
  if (!bridgeConnected) {
    setSaveStatus("Start VoiceFlow to save settings.", true);
  }
}

function clearDirtyFields() {
  dirtyFields.clear();
}

function restoreStateFromPersisted() {
  state.systemLanguage = persistedState.systemLanguage;
  state.primaryShortcutModifiers = canonicalizeModifiers(
    persistedState.primaryShortcutModifiers,
  );
  state.primaryShortcutKey = canonicalizeShortcutKey(persistedState.primaryShortcutKey);
  state.mode = persistedState.mode;
  state.quality = persistedState.quality;
  state.silenceGate = persistedState.silenceGate;
  state.verbosity = persistedState.verbosity;
  state.audioFeedback = persistedState.audioFeedback;
  state.wakePhraseEnabled = persistedState.wakePhraseEnabled;
  state.wakePhrase = persistedState.wakePhrase;
  state.historyRetention = persistedState.historyRetention;
  state.providerPreset = persistedState.providerPreset;
  state.providerBaseUrl = persistedState.providerBaseUrl;
  providerEndpointOverrideEnabled =
    providerSettingsModel.shouldEnableEndpointOverride(
      state.providerPreset,
      state.providerBaseUrl,
    );
  state.providerActiveModel = persistedState.providerActiveModel;
  state.providerTimeoutSeconds = persistedState.providerTimeoutSeconds;
  resetProviderValidationState();
  wakePhrase.value = state.wakePhrase;
  silenceGate.value = state.silenceGate;
  renderHistoryRetentionControls();
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
}

function buildShortcutLabel(modifiers, key) {
  const labels = modifiers.map((modifier) => {
    switch (modifier) {
      case "Control":
        return "Ctrl";
      case "Alt":
        return "Alt";
      case "Shift":
        return "Shift";
      case "Meta":
        return "Meta";
      default:
        return modifier;
    }
  });
  labels.push(key);
  return labels.join("+");
}

function describeShortcutRisk(modifiers, key) {
  const shortcutLabel = buildShortcutLabel(modifiers, key);
  if (!modifiers || modifiers.length === 0) {
    return {
      tone: "is-risky",
      message: tf("settings.shortcutModifierless", { key }),
    };
  }

  const riskMessage = () => tf("settings.shortcutUnsafe", { shortcut: shortcutLabel });

  if (modifiers.length === 1 && modifiers[0] === "Alt") {
    return {
      tone: "is-risky",
      message:
        riskMessage(),
    };
  }

  if (modifiers.length === 1 && modifiers[0] === "Shift") {
    return {
      tone: "is-risky",
      message:
        riskMessage(),
    };
  }

  if (modifiers.length === 1 && modifiers[0] === "Meta") {
    return {
      tone: "is-risky",
      message:
        riskMessage(),
    };
  }

  if (modifiers.length === 1 && modifiers[0] === "Control") {
    const normalizedKey = String(key).toUpperCase();
    switch (normalizedKey) {
      case "C":
        return {
          tone: "is-risky",
          message:
            riskMessage(),
        };
      case "V":
        return {
          tone: "is-risky",
          message:
            riskMessage(),
        };
      case "X":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+X Cut shortcut in many apps.`,
        };
      case "A":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+A Select All shortcut in many apps.`,
        };
      case "F":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+F Find shortcut in many apps and browsers.`,
        };
      case "L":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+L location or focus shortcut in browsers and shells.`,
        };
      case "N":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+N New shortcut in many apps.`,
        };
      case "O":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+O Open shortcut in many apps.`,
        };
      case "P":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+P Print shortcut in many apps and browsers.`,
        };
      case "R":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+R Refresh or reload shortcut in many apps and browsers.`,
        };
      case "S":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+S Save shortcut in many apps.`,
        };
      case "T":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+T New Tab shortcut in many browsers and terminals.`,
        };
      case "W":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+W Close Tab or Close Window shortcut in many apps and browsers.`,
        };
      case "Y":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+Y Redo shortcut in many apps.`,
        };
      case "Z":
        return {
          tone: "is-risky",
          message:
            riskMessage() ??
            `${shortcutLabel} is risky because it matches the standard Ctrl+Z Undo shortcut in many apps.`,
        };
    }
  }

  return {
    tone: "is-stable",
    message: tf("settings.shortcutSafe", { shortcut: shortcutLabel }),
  };
}

function normalizeQualityValue(value) {
  return value === "Fast+" || value === "FastPlus"
    ? "Balanced"
    : value.replaceAll(" ", "");
}

function normalizeSystemLanguageValue(value) {
  return value === "Chinese" || String(value).toLowerCase() === "chinese"
    ? "Chinese"
    : "English";
}

function normalizeProviderPreset(value) {
  return providerSettingsModel.normalizePreset(value);
}

function normalizeOptionalProviderText(value) {
  const trimmed = String(value ?? "").trim();
  return trimmed.length === 0 ? "" : trimmed;
}

function providerLabel(preset) {
  switch (normalizeProviderPreset(preset)) {
    case "volcengine_ark":
      return t("settings.providerVolcengineArk");
    case "tencent_hunyuan":
      return t("settings.providerTencentHunyuan");
    case "custom_openai_compatible":
      return t("settings.providerCustomOpenAi");
    case "bailian":
    default:
      return t("settings.providerBailian");
  }
}

function sourceLabel(source) {
  switch (source) {
    case "env":
      return t("settings.sourceEnv");
    case "settings":
      return t("settings.sourceSettings");
    case "built_in":
      return t("settings.sourceBuiltIn");
    default:
      return t("settings.sourceMissing");
  }
}

function providerPresetSource(providerState) {
  if (providerState?.preset_source) {
    return providerState.preset_source;
  }
  return providerSettingsModel.isPresetProvider(providerState?.preset)
    ? "built_in"
    : "missing";
}

function keySourceLabel(source) {
  switch (source) {
    case "provider_env":
      return t("settings.apiKeySourceProviderEnv");
    case "legacy_dashscope_env":
      return t("settings.apiKeySourceLegacyDashscopeEnv");
    case "credential_store":
      return t("settings.apiKeySourceCredentialStore");
    case "store_error_last_known_good":
      return t("settings.apiKeySourceStoreErrorLastKnownGood");
    default:
      return t("settings.apiKeySourceMissing");
  }
}

function providerTestStatusLabel(status) {
  switch (status) {
    case "success":
      return t("settings.testStatusSuccess");
    case "missing_key":
      return t("settings.testStatusMissingKey");
    case "invalid_configuration":
      return t("settings.testStatusInvalidConfiguration");
    case "authentication_failed":
      return t("settings.testStatusAuthenticationFailed");
    case "endpoint_or_model_not_found":
      return t("settings.testStatusEndpointOrModelNotFound");
    case "rate_limited":
      return t("settings.testStatusRateLimited");
    case "timeout":
      return t("settings.testStatusTimeout");
    case "network_error":
      return t("settings.testStatusNetworkError");
    case "invalid_response":
      return t("settings.testStatusInvalidResponse");
    case "provider_error":
      return t("settings.testStatusProviderError");
    default:
      return t("settings.testStatusProviderError");
  }
}

function providerTestDetailText(providerState, result = null) {
  const providerName = result?.provider_label || providerLabel(providerState?.preset);
  const model = result?.model_code || providerState?.model_code || "--";
  const keySource = keySourceLabel(
    result?.effective_key_source ??
      providerState?.effective_key_source ??
      providerState?.key_source,
  );
  return tf("settings.testConnectionDetails", {
    provider: providerName,
    model,
    keySource,
  });
}

function markProviderTestStale() {
  providerTestResult = null;
  providerTestStale = true;
}

function credentialStoreStatusLabel(status) {
  switch (status) {
    case "available":
    case "present":
      return t("settings.credentialStored");
    case "error":
      return t("settings.credentialError");
    default:
      return t("settings.credentialMissing");
  }
}

function credentialStatusForSelectedProvider(providerState) {
  const preset = normalizeProviderPreset(state.providerPreset);
  const mapped = providerState?.credential_status_by_preset?.[preset];
  if (mapped) {
    return mapped;
  }
  return {
    stored_credential_present:
      providerState?.stored_credential_present === true ||
      providerState?.credential_store_present === true,
    credential_store_status: providerState?.credential_store_status ?? "missing",
  };
}

function providerTimeoutSecondsFromMs(timeoutMs) {
  return timeoutMs === null || timeoutMs === undefined ? "" : String(timeoutMs / 1000);
}

function providerTimeoutMsFromSeconds(value) {
  const trimmed = String(value ?? "").trim();
  if (trimmed.length === 0) {
    return null;
  }
  return Math.round(Number(trimmed) * 1000);
}

function isValidProviderBaseUrl(value) {
  const trimmed = String(value ?? "").trim();
  if (!trimmed) {
    return true;
  }
  let parsed;
  try {
    parsed = new URL(trimmed);
  } catch (_error) {
    return false;
  }
  if (parsed.username || parsed.password || parsed.search || parsed.hash) {
    return false;
  }
  if (parsed.protocol === "https:") {
    return true;
  }
  if (parsed.protocol !== "http:") {
    return false;
  }
  const host = parsed.hostname.toLowerCase();
  return host === "localhost" || host === "127.0.0.1" || host === "::1" || host === "[::1]";
}

function providerBaseUrlValidationMessage() {
  const baseUrl = normalizeOptionalProviderText(state.providerBaseUrl);
  if (
    state.providerPreset === "custom_openai_compatible" &&
    baseUrl.length === 0
  ) {
    return t("settings.providerValidationCustomUrl");
  }
  if (!isValidProviderBaseUrl(baseUrl)) {
    return t("settings.providerValidationUrl");
  }
  return null;
}

function providerTimeoutValidationMessage() {
  const timeout = String(state.providerTimeoutSeconds ?? "").trim();
  if (timeout.length > 0) {
    const timeoutNumber = Number(timeout);
    if (
      !Number.isFinite(timeoutNumber) ||
      timeoutNumber < 1 ||
      timeoutNumber > 120
    ) {
      return t("settings.providerValidationTimeout");
    }
  }
  return null;
}

function providerValidationMessage() {
  return providerBaseUrlValidationMessage() ?? providerTimeoutValidationMessage();
}

function resetProviderValidationState() {
  providerValidationSubmitted = false;
  providerValidationTouched.baseUrl = false;
  providerValidationTouched.timeout = false;
}

function renderProviderFieldMessage(element, input, helper, error, showError) {
  const isError = Boolean(error && showError);
  element.textContent = isError ? error : helper;
  element.classList.toggle("is-error", isError);
  input.setAttribute("aria-invalid", String(isError));
}

function renderProviderValidationMessages(presentation) {
  const baseUrlError = providerBaseUrlValidationMessage();
  const timeoutError = providerTimeoutValidationMessage();
  const showBaseUrlError =
    providerValidationSubmitted || providerValidationTouched.baseUrl;
  const showTimeoutError =
    providerValidationSubmitted || providerValidationTouched.timeout;

  renderProviderFieldMessage(
    providerBaseUrlMessage,
    providerBaseUrl,
    t("settings.baseUrlCustomRequired"),
    presentation.showCustomBaseUrl ? baseUrlError : null,
    showBaseUrlError,
  );
  renderProviderFieldMessage(
    providerOverrideBaseUrlMessage,
    providerOverrideBaseUrl,
    t("settings.customEndpointHelp"),
    presentation.showOverrideBaseUrl ? baseUrlError : null,
    showBaseUrlError,
  );
  renderProviderFieldMessage(
    providerTimeoutMessage,
    providerTimeoutSeconds,
    t("settings.timeoutHelp"),
    timeoutError,
    showTimeoutError,
  );
}

function quoteForPowerShell(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function formatPowerShellSettingAssignment(key, value) {
  const assignment = `${key}=${value}`;
  return /^[A-Za-z0-9_.=+-]+$/.test(assignment)
    ? assignment
    : quoteForPowerShell(assignment);
}

function buildHostApplyCommand(payload) {
  const commandParts = ["cargo run -p input-host -- --update-settings"];
  if (payload.system_language !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "system_language",
        String(payload.system_language).toLowerCase(),
      ),
    );
  }
  if (payload.primary_shortcut_modifiers !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "primary_shortcut_modifiers",
        payload.primary_shortcut_modifiers.join("+"),
      ),
    );
  }
  if (payload.primary_shortcut_key !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "primary_shortcut_key",
        payload.primary_shortcut_key,
      ),
    );
  }
  if (payload.shortcut_mode !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment("shortcut_mode", payload.shortcut_mode),
    );
  }
  if (payload.refinement_quality !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "refinement_quality",
        payload.refinement_quality,
      ),
    );
  }
  if (payload.silence_gate_level !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "silence_gate_level",
        payload.silence_gate_level,
      ),
    );
  }
  if (payload.diagnostics_verbosity !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "diagnostics_verbosity",
        payload.diagnostics_verbosity,
      ),
    );
  }
  if (payload.audio_feedback_enabled !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "audio_feedback_enabled",
        payload.audio_feedback_enabled,
      ),
    );
  }
  if (payload.wake_phrase_enabled !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "wake_phrase_enabled",
        payload.wake_phrase_enabled,
      ),
    );
  }
  if (payload.wake_phrase_phrase !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "wake_phrase_phrase",
        payload.wake_phrase_phrase,
      ),
    );
  }
  if (payload.history_retention !== undefined) {
    commandParts.push(
      formatPowerShellSettingAssignment(
        "history_retention",
        payload.history_retention,
      ),
    );
  }
  if (payload.provider !== undefined) {
    if (payload.provider.preset !== undefined) {
      commandParts.push(
        formatPowerShellSettingAssignment("provider.preset", payload.provider.preset),
      );
    }
    if (payload.provider.base_url !== undefined) {
      commandParts.push(
        formatPowerShellSettingAssignment(
          "provider.base_url",
          payload.provider.base_url ?? "",
        ),
      );
    }
    if (payload.provider.active_model !== undefined) {
      commandParts.push(
        formatPowerShellSettingAssignment(
          "provider.active_model",
          payload.provider.active_model ?? "",
        ),
      );
    }
    if (payload.provider.request_timeout_ms !== undefined) {
      commandParts.push(
        formatPowerShellSettingAssignment(
          "provider.request_timeout_ms",
          payload.provider.request_timeout_ms ?? "",
        ),
      );
    }
  }

  return commandParts.length === 1 ? null : commandParts.join(" ");
}

function renderSaveCommand() {
  if (!saveCommand) {
    return;
  }

  const payload = buildSettingsUpdatePayload();
  const command = buildHostApplyCommand(payload);

  saveCommand.textContent = command ?? "No settings changes pending.";

  if (commandCopyStatus) {
    commandCopyStatus.textContent =
      command === null
        ? "No fallback host command is needed until local changes exist."
        : "Copy the fallback command if you want to apply the same changes from the terminal.";
  }
}

function saveFeedbackMessage(feedback) {
  if ((feedback?.nextSession?.length ?? 0) > 0) {
    return t("settings.savedNextDictation");
  }
  if ((feedback?.nextProviderRequest?.length ?? 0) > 0) {
    return t("settings.savedNextAiRequest");
  }
  return t("settings.savedApplied");
}

function scheduleSaveFeedbackDismissal() {
  if (saveFeedbackTimer !== null) {
    window.clearTimeout(saveFeedbackTimer);
  }
  saveFeedbackTimer = window.setTimeout(() => {
    saveFeedbackTimer = null;
    if (dirtyFields.size === 0) {
      lastSaveFeedback = null;
      renderActionState();
    }
  }, 2400);
}

function renderActionState() {
  const hasDirtyChanges = dirtyFields.size > 0;
  const hasSaveFeedback = lastSaveFeedback !== null;
  const stageableRecommendations = latestTuningRecommendations.filter(
    (recommendation) =>
      !isRecommendationAlreadyStaged(recommendation) &&
      !recommendation.saved_already_matches_target,
  );
  if (applyButton) {
    applyButton.disabled = saveInFlight || !hasDirtyChanges || !bridgeConnected;
    applyButton.setAttribute("aria-busy", String(saveInFlight));
    applyButton.title =
      hasDirtyChanges && !bridgeConnected ? t("settings.bridgeRequired") : "";
  }
  if (revertButton) {
    revertButton.disabled = saveInFlight || !hasDirtyChanges;
  }
  if (copySaveCommandButton) {
    copySaveCommandButton.disabled = currentHostApplyCommand() === null;
  }
  if (clearHistoryButton) {
    clearHistoryButton.disabled = historyClearInFlight || !bridgeConnected;
    clearHistoryButton.setAttribute("aria-busy", String(historyClearInFlight));
    clearHistoryButton.title = !bridgeConnected ? t("settings.clearHistoryBridgeRequired") : "";
  }
  if (clearHistoryStatus && !historyClearInFlight && !bridgeConnected) {
    clearHistoryStatus.textContent = t("settings.clearHistoryBridgeRequired");
    clearHistoryStatus.classList.toggle("is-error", false);
  }
  if (reloadPromptsButton) {
    reloadPromptsButton.disabled = promptOverrideReloadInFlight || !bridgeConnected;
    reloadPromptsButton.setAttribute("aria-busy", String(promptOverrideReloadInFlight));
    reloadPromptsButton.title = !bridgeConnected ? t("settings.promptBridgeRequired") : "";
  }
  if (saveProviderApiKeyButton) {
    saveProviderApiKeyButton.disabled = providerCredentialInFlight || !bridgeConnected;
    saveProviderApiKeyButton.setAttribute("aria-busy", String(providerCredentialInFlight));
    saveProviderApiKeyButton.title = !bridgeConnected
      ? t("settings.apiKeyBridgeRequired")
      : "";
  }
  if (clearProviderApiKeyButton) {
    clearProviderApiKeyButton.disabled = providerCredentialInFlight || !bridgeConnected;
    clearProviderApiKeyButton.setAttribute("aria-busy", String(providerCredentialInFlight));
    clearProviderApiKeyButton.title = !bridgeConnected
      ? t("settings.apiKeyBridgeRequired")
      : "";
  }
  if (providerApiKeyStatus && !providerCredentialInFlight && !bridgeConnected) {
    providerApiKeyStatus.textContent = t("settings.apiKeyBridgeRequired");
    providerApiKeyStatus.classList.toggle("is-error", false);
  }
  if (testProviderConnectionButton) {
    const providerDirty = dirtyFields.has("provider");
    testProviderConnectionButton.disabled =
      providerTestInFlight || !bridgeConnected;
    testProviderConnectionButton.setAttribute("aria-busy", String(providerTestInFlight));
    testProviderConnectionButton.title = !bridgeConnected
      ? t("settings.testConnectionBridgeRequired")
      : providerDirty
        ? t("settings.testConnectionApplyFirst")
        : "";
  }
  renderProviderTestStatus(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  if (promptOverridesStatus && !promptOverrideReloadInFlight && !bridgeConnected) {
    promptOverridesStatus.textContent = t("settings.promptBridgeRequired");
    promptOverridesStatus.classList.toggle("is-error", false);
  }
  if (bridgeStatus) {
    bridgeStatus.textContent = bridgeConnected
      ? t("settings.bridgeConnected")
      : t("settings.bridgeOffline");
    bridgeStatus.classList.toggle("is-live", bridgeConnected);
  }
  if (overviewStatus) {
    overviewStatus.textContent = bridgeConnected
      ? t("overview.connectedNow")
      : t("overview.offlineNow");
    overviewStatus.classList.toggle("is-offline", !bridgeConnected);
  }
  if (diagnosticsConnectionStatus) {
    diagnosticsConnectionStatus.textContent = bridgeConnected
      ? t("overview.connectedNow")
      : t("overview.offlineNow");
    diagnosticsConnectionStatus.classList.toggle("is-offline", !bridgeConnected);
  }
  if (dirtyStatus) {
    dirtyStatus.textContent = hasDirtyChanges
      ? t("settings.unsavedChanges")
      : t("settings.noChanges");
    dirtyStatus.classList.toggle("is-dirty", hasDirtyChanges);
    dirtyStatus.hidden = !hasDirtyChanges;
  }
  if (saveStatus && !saveInFlight) {
    if (hasDirtyChanges && !bridgeConnected) {
      setSaveStatus(t("settings.bridgeOffline"));
    } else if (hasDirtyChanges) {
      setSaveStatus(t("settings.unsavedChanges"));
    } else if (hasSaveFeedback) {
      setSaveStatus(saveFeedbackMessage(lastSaveFeedback));
    } else {
      setSaveStatus("");
    }
  }
  if (saveDock) {
    const saveDockState = hasDirtyChanges && !bridgeConnected
      ? "offline"
      : hasDirtyChanges
        ? "pending"
        : "ready";
    ["offline", "pending", "restart", "ready"].forEach((stateName) => {
      saveDock.classList.toggle(`is-${stateName}`, stateName === saveDockState);
    });
    saveDock.classList.toggle(
      "is-hidden",
      !saveInFlight && !hasDirtyChanges && !hasSaveFeedback,
    );
  }
  if (stageAllRecommendationsButton) {
    stageAllRecommendationsButton.disabled =
      saveInFlight || stageableRecommendations.length === 0;
    stageAllRecommendationsButton.textContent =
      stageableRecommendations.length === 0
        ? t("settings.noSuggestions")
        : t("settings.stageApplyReady");
  }

  if (draftStageChip && draftStageCopy) {
    draftStageChip.classList.remove("is-stable", "is-watch", "is-alert");
    if (hasDirtyChanges) {
      draftStageChip.textContent = t("settings.unsaved");
      draftStageChip.classList.add("is-watch");
      draftStageCopy.textContent = t("settings.draftPending");
    } else {
      draftStageChip.textContent = t("settings.inSync");
      draftStageChip.classList.add("is-stable");
      draftStageCopy.textContent = t("settings.noDraft");
    }
  }
}

function getSettingsControlUrl() {
  return (
    window.__VOICEFLOW_SETTINGS_RUNTIME__?.settings_control_url ??
    "http://127.0.0.1:47831"
  );
}

function getSettingsControlToken() {
  return window.__VOICEFLOW_SETTINGS_BRIDGE__?.controlToken ?? null;
}

function buildMutationHeaders() {
  const headers = {
    "Content-Type": "application/json",
  };
  const token = getSettingsControlToken();
  if (token) {
    headers[settingsControlTokenHeader] = token;
  }
  return headers;
}

function buildProviderUpdatePayload() {
  const provider = {};
  const preset = normalizeProviderPreset(state.providerPreset);
  if (preset !== normalizeProviderPreset(persistedState.providerPreset)) {
    provider.preset = preset;
  }

  const baseUrl = normalizeOptionalProviderText(state.providerBaseUrl);
  const persistedBaseUrl = normalizeOptionalProviderText(persistedState.providerBaseUrl);
  const persistedEndpointOverrideEnabled =
    providerSettingsModel.shouldEnableEndpointOverride(
      persistedState.providerPreset,
      persistedState.providerBaseUrl,
    );
  if (
    baseUrl !== persistedBaseUrl ||
    providerEndpointOverrideEnabled !== persistedEndpointOverrideEnabled
  ) {
    provider.base_url = baseUrl.length === 0 ? null : baseUrl;
  }

  const activeModel = normalizeOptionalProviderText(state.providerActiveModel);
  if (
    activeModel !== normalizeOptionalProviderText(persistedState.providerActiveModel)
  ) {
    provider.active_model = activeModel.length === 0 ? null : activeModel;
  }

  const timeoutMs = providerTimeoutMsFromSeconds(state.providerTimeoutSeconds);
  const persistedTimeoutMs = providerTimeoutMsFromSeconds(
    persistedState.providerTimeoutSeconds,
  );
  if (timeoutMs !== persistedTimeoutMs) {
    provider.request_timeout_ms = timeoutMs;
  }

  return Object.keys(provider).length === 0 ? null : provider;
}

function buildSettingsUpdatePayload() {
  const payload = {};
  if (
    normalizeSystemLanguageValue(state.systemLanguage) !==
    normalizeSystemLanguageValue(persistedState.systemLanguage)
  ) {
    payload.system_language = normalizeSystemLanguageValue(state.systemLanguage);
  }
  if (
    canonicalizeShortcutKey(state.primaryShortcutKey) !==
      canonicalizeShortcutKey(persistedState.primaryShortcutKey) ||
    canonicalizeModifiers(state.primaryShortcutModifiers).join("|") !==
      canonicalizeModifiers(persistedState.primaryShortcutModifiers).join("|")
  ) {
    payload.primary_shortcut_modifiers = canonicalizeModifiers(
      state.primaryShortcutModifiers,
    );
    payload.primary_shortcut_key = canonicalizeShortcutKey(state.primaryShortcutKey);
  }
  if (state.mode !== persistedState.mode) {
    payload.shortcut_mode = state.mode;
  }
  if (normalizeQualityValue(state.quality) !== normalizeQualityValue(persistedState.quality)) {
    payload.refinement_quality = normalizeQualityValue(state.quality);
  }
  if (state.silenceGate !== persistedState.silenceGate) {
    payload.silence_gate_level = Number(state.silenceGate);
  }
  if (state.verbosity !== persistedState.verbosity) {
    payload.diagnostics_verbosity = state.verbosity;
  }
  if (state.audioFeedback !== persistedState.audioFeedback) {
    payload.audio_feedback_enabled = state.audioFeedback;
  }
  if (state.wakePhraseEnabled !== persistedState.wakePhraseEnabled) {
    payload.wake_phrase_enabled = state.wakePhraseEnabled;
  }
  if (state.wakePhrase !== persistedState.wakePhrase) {
    payload.wake_phrase_phrase = state.wakePhrase;
  }
  if (state.historyRetention !== persistedState.historyRetention) {
    payload.history_retention = state.historyRetention;
  }
  const providerPayload = buildProviderUpdatePayload();
  if (providerPayload !== null) {
    payload.provider = providerPayload;
  }

  return payload;
}

function renderShortcutControls() {
  shortcutModifierButtons.forEach((button) => {
    button.classList.toggle(
      "is-active",
      state.primaryShortcutModifiers.includes(button.dataset.modifier),
    );
  });
  primaryShortcutKey.value = state.primaryShortcutKey;
}

function renderProviderControls(providerState) {
  if (!providerPreset) {
    return;
  }

  const presentation = providerSettingsModel.presentation(
    state.providerPreset,
    state.providerBaseUrl,
    providerEndpointOverrideEnabled,
  );
  providerPreset.value = presentation.preset;
  providerBaseUrl.value = state.providerBaseUrl;
  providerOverrideBaseUrl.value = state.providerBaseUrl;
  providerActiveModel.value = state.providerActiveModel;
  providerActiveModel.placeholder = presentation.modelPlaceholder;
  providerTimeoutSeconds.value = state.providerTimeoutSeconds;

  providerCustomBaseUrlField.hidden = !presentation.showCustomBaseUrl;
  providerPresetEndpointSettings.hidden = !presentation.showPresetEndpoint;
  providerEndpointOverride.checked = presentation.showOverrideBaseUrl;
  providerOverrideBaseUrlField.hidden = !presentation.showOverrideBaseUrl;
  providerEndpointReset.disabled =
    !presentation.showOverrideBaseUrl &&
    normalizeOptionalProviderText(state.providerBaseUrl).length === 0;
  providerPresetBaseUrl.textContent = presentation.presetBaseUrl
    ? tf("settings.providerPresetEndpoint", { url: presentation.presetBaseUrl })
    : "";
  providerActiveModelHelp.textContent = t(presentation.modelHelpKey);
  renderProviderValidationMessages(presentation);

  const keyConfigured =
    providerState?.effective_key_present === true || providerState?.key_present === true;
  const effectiveKeySource =
    providerState?.effective_key_source ?? providerState?.key_source ?? "missing";
  const selectedCredential = credentialStatusForSelectedProvider(providerState);
  const storedCredentialPresent = selectedCredential.stored_credential_present === true;
  providerKeyStatus.textContent = keyConfigured
    ? t("settings.apiKeyConfigured")
    : t("settings.apiKeyMissing");
  if (providerStoredCredentialStatus) {
    providerStoredCredentialStatus.textContent =
      selectedCredential.credential_store_status === "error"
        ? credentialStoreStatusLabel("error")
        : storedCredentialPresent
          ? t("settings.credentialStored")
          : t("settings.credentialMissing");
    providerStoredCredentialStatus.classList.toggle(
      "is-error",
      selectedCredential.credential_store_status === "error",
    );
  }
  if (providerKeySourceStatus) {
    providerKeySourceStatus.textContent = keySourceLabel(effectiveKeySource);
  }
  if (providerApiKeyHelp) {
    providerApiKeyHelp.textContent = providerState?.env_key_override
      ? t("settings.overrideApiKey")
      : t("settings.apiKeyHelp");
  }
  if (providerApiKeyStatus && !providerCredentialInFlight && bridgeConnected) {
    if (selectedCredential.credential_store_status === "error") {
      providerApiKeyStatus.textContent = credentialStoreStatusLabel("error");
      providerApiKeyStatus.classList.toggle("is-error", true);
    } else if (providerApiKeyStatus.textContent === t("settings.credentialError")) {
      providerApiKeyStatus.textContent = "";
      providerApiKeyStatus.classList.toggle("is-error", false);
    }
  }
  providerConfiguredChip.textContent = keyConfigured
    ? t("settings.apiKeyConfigured")
    : t("settings.apiKeyMissing");
  providerConfiguredChip.classList.toggle("is-stable", keyConfigured);
  providerConfiguredChip.classList.toggle("is-alert", !keyConfigured);

  const effectivePreset = normalizeProviderPreset(providerState?.preset);
  providerEffectiveProvider.textContent = `${providerLabel(effectivePreset)} · ${sourceLabel(providerPresetSource(providerState))}`;
  providerEffectiveBaseUrl.textContent = `${providerState?.base_url || "--"} · ${sourceLabel(providerState?.base_url_source)}`;
  providerEffectiveModel.textContent = `${providerState?.model_code || "--"} · ${sourceLabel(providerState?.model_source)}`;
  providerEffectiveTimeout.textContent = `${Math.round((providerState?.request_timeout_ms ?? 12000) / 1000)}s · ${sourceLabel(providerState?.request_timeout_source)}`;

  providerSpecificNote.textContent = t(presentation.noteKey);

  renderProviderOverrideNotices(providerState);
  renderProviderTestStatus(providerState);
}

function renderProviderTestStatus(providerState) {
  if (!providerTestStatus && !providerTestDetails) {
    return;
  }
  const providerDirty = dirtyFields.has("provider");
  if (providerTestDetails) {
    providerTestDetails.textContent = providerTestDetailText(providerState, providerTestResult);
  }
  if (!providerTestStatus) {
    return;
  }
  providerTestStatus.classList.toggle("is-error", false);
  if (!bridgeConnected) {
    providerTestStatus.textContent = t("settings.testConnectionBridgeRequired");
    return;
  }
  if (providerDirty) {
    providerTestStatus.textContent = providerTestStale
      ? t("settings.testConnectionStale")
      : t("settings.testConnectionApplyFirst");
    return;
  }
  if (providerTestInFlight) {
    providerTestStatus.textContent = t("settings.testConnectionTesting");
    return;
  }
  if (!providerTestResult) {
    providerTestStatus.textContent = "";
    return;
  }
  if (providerTestResult.success === true) {
    providerTestStatus.textContent = tf("settings.testConnectionSuccess", {
      latency: providerTestResult.latency_ms ?? "--",
    });
    return;
  }
  providerTestStatus.textContent = tf("settings.testConnectionFailed", {
    status: providerTestStatusLabel(providerTestResult.status),
  });
  providerTestStatus.classList.toggle("is-error", true);
}

function renderProviderOverrideNotices(providerState) {
  if (!providerOverrideList) {
    return;
  }
  providerOverrideList.replaceChildren();
  const notices = [];
  if (providerPresetSource(providerState) === "env") {
    notices.push(t("settings.overrideProvider"));
  }
  if (providerState?.base_url_source === "env") {
    notices.push(t("settings.overrideBaseUrl"));
  }
  if (providerState?.model_source === "env") {
    notices.push(t("settings.overrideModel"));
  }
  if (providerState?.request_timeout_source === "env") {
    notices.push(t("settings.overrideTimeout"));
  }
  if (providerState?.env_key_override) {
    notices.push(t("settings.overrideApiKey"));
  }
  notices.forEach((notice) => {
    const row = document.createElement("div");
    row.className = "provider-override-row";
    const label = document.createElement("span");
    label.textContent = sourceLabel("env");
    const copy = document.createElement("strong");
    copy.textContent = notice;
    row.append(label, copy);
    providerOverrideList.appendChild(row);
  });
}

function setSaveStatus(message, isError = false) {
  if (!saveStatus) {
    return;
  }

  saveStatus.textContent = localizeStatusMessage(message);
  saveStatus.classList.toggle("is-error", isError);
}

function setActiveSegment(groupName, runtimeValue) {
  const group = document.querySelector(`.segmented[data-segment="${groupName}"]`);
  if (!group) {
    return;
  }

  const mapping = segmentValueMap[groupName];
  group.querySelectorAll(".segment").forEach((button) => {
    const isActive = mapping?.[button.dataset.value] === runtimeValue;
    button.classList.toggle("is-active", isActive);
    if (button.getAttribute("role") === "radio") {
      button.setAttribute("aria-checked", String(isActive));
      button.tabIndex = isActive ? 0 : -1;
    }
  });
}

function renderWarnings(warnings) {
  settingsWarnings.replaceChildren();
  if (!warnings || warnings.length === 0) {
    const item = document.createElement("li");
    item.textContent = "No runtime warnings.";
    settingsWarnings.appendChild(item);
    return;
  }

  warnings.forEach((warning) => {
    const item = document.createElement("li");
    item.textContent = warning;
    settingsWarnings.appendChild(item);
  });
}

function renderShortcutWarnings(warnings) {
  shortcutRuntimeWarnings.replaceChildren();
  const shortcutWarnings = (warnings ?? []).filter((warning) =>
    warning.toLowerCase().includes("shortcut"),
  );
  if (shortcutWarningGroup) {
    shortcutWarningGroup.hidden = shortcutWarnings.length === 0;
  }
  updateDiagnosticPatternsVisibility();

  if (shortcutWarnings.length === 0) {
    const item = document.createElement("li");
    item.textContent = "No persisted shortcut warnings mirrored from the Rust host.";
    shortcutRuntimeWarnings.appendChild(item);
    return;
  }

  shortcutWarnings.forEach((warning) => {
    const item = document.createElement("li");
    item.textContent = warning;
    shortcutRuntimeWarnings.appendChild(item);
  });
}

function renderRuntimeShortcutAlignment(runtime) {
  const liveSummary = runtime?.latest_live_host_summary;
  if (!liveSummary) {
    runtimeAlignmentChip.textContent = "Unavailable";
    runtimeAlignmentChip.classList.remove("is-stable", "is-watch", "is-alert");
    runtimeShortcutHeadline.textContent = "No live host runtime has been mirrored yet.";
    runtimeShortcutNote.textContent =
      "Launch the live host to compare the running shortcut and runtime profile against the saved settings.";
    runtimeDriftFields.replaceChildren();
    runtimeDriftList.replaceChildren();
    renderRuntimeProfileRows([
      ["Shortcut", "Unavailable"],
      ["Mode", "Unavailable"],
      ["Refine", "Unavailable"],
      ["Audio cues", "Unavailable"],
      ["Diagnostics", "Unavailable"],
      ["Trigger phrase", "Unavailable"],
    ]);
    return;
  }

  const liveShortcut = liveSummary.effective_primary_shortcut;
  const liveMode =
    liveSummary.effective_shortcut_mode === "PushToTalk" ? "Push-to-talk" : "Toggle";
  const liveRefine = normalizeQualityLabel(liveSummary.effective_refinement_quality);
  const liveAudio = liveSummary.effective_audio_feedback_enabled ? "Audio cues on" : "Audio cues muted";
  const liveDiagnostics =
    liveSummary.effective_diagnostics_verbosity === "Verbose"
      ? "Verbose diagnostics"
      : "Standard diagnostics";
  const liveWakePhrase = liveSummary.effective_wake_phrase_enabled
    ? `On | ${liveSummary.effective_wake_phrase_text}`
    : "Off";

  runtimeShortcutHeadline.textContent = `Live host is using ${liveShortcut} | ${liveMode} | ${liveRefine} refine | ${liveAudio} | ${liveDiagnostics}.`;

  runtimeAlignmentChip.classList.remove("is-stable", "is-watch", "is-alert");
  runtimeDriftFields.replaceChildren();
  runtimeDriftList.replaceChildren();
  renderRuntimeProfileRows([
    ["Shortcut", liveShortcut],
    ["Mode", liveMode],
    ["Refine", liveRefine],
    ["Audio cues", liveAudio],
    ["Diagnostics", liveDiagnostics],
    ["Trigger phrase", liveWakePhrase],
  ]);

  if (liveSummary.matches_saved_settings) {
    runtimeAlignmentChip.textContent = "Aligned";
    runtimeAlignmentChip.classList.add("is-stable");
    runtimeShortcutNote.textContent =
      "The last mirrored live host matches the currently saved settings.";
    return;
  }

  runtimeAlignmentChip.textContent = "Last report differs";
  runtimeAlignmentChip.classList.add("is-watch");
  runtimeShortcutNote.textContent =
    "Saved settings apply automatically. This completed host report may predate the latest save.";

  (liveSummary.drift_fields ?? []).forEach((field) => {
    const chip = document.createElement("span");
    chip.className = "mix-pill";
    chip.textContent = field;
    runtimeDriftFields.appendChild(chip);
  });

  renderRuntimeDriftRows(runtime, liveSummary);
}

function renderRuntimeProfileRows(entries) {
  runtimeProfileList.replaceChildren();
  entries.forEach(([label, value]) => {
    const wrapper = document.createElement("div");
    wrapper.className = "runtime-profile-item";

    const term = document.createElement("dt");
    term.textContent = label;
    const description = document.createElement("dd");
    description.textContent = value;

    wrapper.appendChild(term);
    wrapper.appendChild(description);
    runtimeProfileList.appendChild(wrapper);
  });
}

function renderRuntimeDriftRows(runtime, liveSummary) {
  const savedEntries = buildSavedProfileEntries(runtime);
  const liveEntries = buildLiveProfileEntries(liveSummary);
  const driftFields = liveSummary.drift_fields ?? [];

  driftFields.forEach((field) => {
    const row = document.createElement("div");
    row.className = "runtime-drift-item";

    const label = document.createElement("p");
    label.className = "runtime-drift-label";
    label.textContent = field;

    const liveValue = document.createElement("p");
    liveValue.className = "runtime-drift-live";
    liveValue.textContent = `Live: ${liveEntries[field] ?? "Unavailable"}`;

    const savedValue = document.createElement("p");
    savedValue.className = "runtime-drift-saved";
    savedValue.textContent = `Saved: ${savedEntries[field] ?? "Unavailable"}`;

    row.appendChild(label);
    row.appendChild(liveValue);
    row.appendChild(savedValue);
    runtimeDriftList.appendChild(row);
  });
}

function buildSavedProfileEntries(runtime) {
  return {
    "Primary shortcut": buildShortcutLabel(
      runtime.primary_shortcut_modifiers ?? persistedState.primaryShortcutModifiers,
      runtime.primary_shortcut_key ?? persistedState.primaryShortcutKey,
    ),
    "Shortcut mode":
      (runtime.shortcut_mode ?? persistedState.mode) === "PushToTalk"
        ? "Push-to-talk"
        : "Toggle",
    "Refine quality": normalizeQualityLabel(
      runtime.refinement_quality ?? persistedState.quality,
    ),
    "Silence gate":
      silenceLabels[runtime.silence_gate_level ?? Number(persistedState.silenceGate)],
    "Diagnostics verbosity":
      (runtime.diagnostics_verbosity ?? persistedState.verbosity) === "Verbose"
        ? "Verbose diagnostics"
        : "Standard diagnostics",
    "Audio cues":
      runtime.audio_feedback_enabled ?? persistedState.audioFeedback
        ? "Audio cues on"
        : "Audio cues muted",
    "Instructed dictation":
      runtime.wake_phrase_enabled ?? persistedState.wakePhraseEnabled ? "On" : "Off",
    "Trigger phrase": runtime.wake_phrase_text ?? persistedState.wakePhrase,
  };
}

function buildLiveProfileEntries(liveSummary) {
  return {
    "Primary shortcut": liveSummary.effective_primary_shortcut,
    "Shortcut mode":
      liveSummary.effective_shortcut_mode === "PushToTalk"
        ? "Push-to-talk"
        : "Toggle",
    "Refine quality": normalizeQualityLabel(liveSummary.effective_refinement_quality),
    "Silence gate":
      silenceLabels[liveSummary.effective_silence_gate_level] ??
      `Level ${liveSummary.effective_silence_gate_level}`,
    "Diagnostics verbosity":
      liveSummary.effective_diagnostics_verbosity === "Verbose"
        ? "Verbose diagnostics"
        : "Standard diagnostics",
    "Audio cues": liveSummary.effective_audio_feedback_enabled
      ? "Audio cues on"
      : "Audio cues muted",
    "Instructed dictation": liveSummary.effective_wake_phrase_enabled ? "On" : "Off",
    "Trigger phrase": liveSummary.effective_wake_phrase_text,
  };
}

function normalizeQualityLabel(value) {
  switch (value) {
    case "FastPlus":
      return "Balanced";
    case "BestQuality":
      return "Best Quality";
    default:
      return value;
  }
}

function getFreshnessState(epochMs) {
  if (!epochMs) {
    return {
      label: t("common.unavailable"),
      tone: "is-stale",
      note: "No mirrored report timestamp is available yet.",
    };
  }

  const deltaMs = Date.now() - epochMs;
  if (deltaMs < 90_000) {
    return {
      label: currentLanguage() === "Chinese" ? "刚刚记录" : "Recorded just now",
      tone: "is-fresh",
      note: "Report updated just now.",
    };
  }

  const deltaMinutes = Math.round(deltaMs / 60_000);
  if (deltaMinutes < 15) {
    return {
      label: currentLanguage() === "Chinese"
        ? `${deltaMinutes} 分钟前记录`
        : `Recorded ${deltaMinutes} minute${deltaMinutes === 1 ? "" : "s"} ago`,
      tone: "is-recent",
      note: `Report updated ${deltaMinutes} minute${deltaMinutes === 1 ? "" : "s"} ago.`,
    };
  }

  if (deltaMinutes < 60) {
    return {
      label: currentLanguage() === "Chinese"
        ? `${deltaMinutes} 分钟前记录`
        : `Recorded ${deltaMinutes} minutes ago`,
      tone: "is-aging",
      note: `Report updated ${deltaMinutes} minute${deltaMinutes === 1 ? "" : "s"} ago.`,
    };
  }

  const deltaHours = Math.round(deltaMinutes / 60);
  return {
    label: currentLanguage() === "Chinese"
      ? `${deltaHours} 小时前记录`
      : `Recorded ${deltaHours} hour${deltaHours === 1 ? "" : "s"} ago`,
    tone: "is-stale",
    note: `Report updated ${deltaHours} hour${deltaHours === 1 ? "" : "s"} ago.`,
  };
}

function getCommitPathOutlookTone(outlook) {
  switch (outlook) {
    case "Healthy":
      return "is-stable";
    case "Fallback-heavy":
      return "is-watch";
    case "Edit-heavy":
      return "is-watch";
    case "At risk":
      return "is-alert";
    default:
      return "";
  }
}

function getVerificationFocusTone(focus) {
  switch (focus) {
    case "Insertion reliability":
    case "Caret coverage":
      return "is-watch";
    case "Speech capture":
    case "Edit execution":
      return "is-watch";
    case "Real-app confidence":
      return "is-stable";
    default:
      return "";
  }
}

function getWorkloadFocusTone(focus) {
  switch (focus) {
    case "Dictation-heavy workload":
      return "is-stable";
    case "Formatting edits":
    case "Structural edits":
    case "Drafting workload":
    case "Synthesis workload":
      return "is-watch";
    case "Edit-heavy workload":
    case "Intent-heavy workload":
      return "is-watch";
    default:
      return "";
  }
}

function renderSimpleDiagnosticGuidance(summaryState) {
  if (!recentProblemSummary || !recommendedActionCard || !recommendedAction) {
    return;
  }

  const failedSessionsCount = summaryState?.failed_sessions ?? 0;
  if (!summaryState || failedSessionsCount === 0) {
    recentProblemSummary.textContent = t("diagnostics.noRecentProblems");
    recommendedAction.textContent = "";
    recommendedActionCard.classList.add("is-hidden");
    if (overviewAttention) {
      overviewAttention.hidden = true;
    }
    return;
  }

  let problemKey = "diagnostics.generalProblem";
  let actionKey = "diagnostics.generalAction";
  if (
    (summaryState.recording_failures ?? 0) > 0 ||
    (summaryState.silence_gate_failures ?? 0) > 0 ||
    (summaryState.no_speech_failures ?? 0) > 0
  ) {
    problemKey = "diagnostics.hearingProblem";
    actionKey = "diagnostics.hearingAction";
  } else if ((summaryState.committing_failures ?? 0) > 0) {
    problemKey = "diagnostics.insertionProblem";
    actionKey = "diagnostics.insertionAction";
  } else if ((summaryState.recognizing_failures ?? 0) > 0) {
    problemKey = "diagnostics.recognitionProblem";
    actionKey = "diagnostics.recognitionAction";
  }

  recentProblemSummary.textContent =
    problemKey === "diagnostics.generalProblem"
      ? tf(problemKey, {
          count: failedSessionsCount,
          suffix: failedSessionsCount === 1 ? "" : "s",
        })
      : t(problemKey);
  recommendedAction.textContent = t(actionKey);
  recommendedActionCard.classList.remove("is-hidden");
  if (overviewRecentProblem && overviewRecommendedAction && overviewAttention) {
    overviewRecentProblem.textContent = recentProblemSummary.textContent;
    overviewRecommendedAction.textContent = recommendedAction.textContent;
    overviewAttention.hidden = false;
  }
}

function updateDiagnosticPatternsVisibility() {
  if (diagnosticPatternsSection) {
    diagnosticPatternsSection.hidden =
      recentFailurePatternGroup?.hidden === true && shortcutWarningGroup?.hidden === true;
  }
}

function renderOverviewDiagnosticEvidence(summaryState, freshness) {
  if (overviewReportAge) {
    overviewReportAge.textContent = freshness.label;
  }
  if (!overviewReportHealth || !overviewHealthNote) {
    return;
  }

  if (!summaryState || summaryState.attempted_sessions === 0) {
    overviewReportHealth.textContent = currentLanguage() === "Chinese" ? "暂无结论" : "No conclusion";
    overviewHealthNote.textContent = currentLanguage() === "Chinese"
      ? "使用一次 VoiceFlow 后，这里会显示诊断结论。"
      : "Use VoiceFlow once to record a diagnostic conclusion.";
    return;
  }

  if (summaryState.failed_sessions === 0) {
    overviewReportHealth.textContent = currentLanguage() === "Chinese" ? "正常" : "Stable";
    overviewHealthNote.textContent = currentLanguage() === "Chinese"
      ? `最近镜像的 ${summaryState.successful_sessions} 个会话均已完成。`
      : `All ${summaryState.successful_sessions} mirrored session${summaryState.successful_sessions === 1 ? "" : "s"} completed.`;
    return;
  }

  const failureRate = summaryState.failed_sessions / Math.max(summaryState.attempted_sessions, 1);
  overviewReportHealth.textContent = failureRate >= 0.4
    ? (currentLanguage() === "Chinese" ? "需要注意" : "Needs attention")
    : (currentLanguage() === "Chinese" ? "结果不一致" : "Mixed");
  overviewHealthNote.textContent = currentLanguage() === "Chinese"
    ? `最近 ${summaryState.attempted_sessions} 个会话中有 ${summaryState.failed_sessions} 个未完成。`
    : `${summaryState.failed_sessions} of ${summaryState.attempted_sessions} recent session${summaryState.attempted_sessions === 1 ? "" : "s"} did not complete.`;
}

function renderActionMix(container, items, emptyLabel) {
  if (!container) {
    return;
  }

  container.replaceChildren();
  if (!items || items.length === 0) {
    const empty = document.createElement("span");
    empty.className = "mix-pill";
    empty.textContent = emptyLabel;
    container.appendChild(empty);
    return;
  }

  items.forEach((item) => {
    const pill = document.createElement("span");
    pill.className = "mix-pill";
    pill.textContent = `${item.label} ${item.count}`;
    container.appendChild(pill);
  });
}

function renderLiveHostSummary(summaryState) {
  latestLiveSummary = summaryState ?? null;
  renderSimpleDiagnosticGuidance(summaryState);
  attemptedSessions.textContent = `${summaryState?.attempted_sessions ?? 0}`;
  successfulSessions.textContent = `${summaryState?.successful_sessions ?? 0}`;
  failedSessions.textContent = `${summaryState?.failed_sessions ?? 0}`;
  lateFailures.textContent = `${summaryState?.failed_with_summary_sessions ?? 0}`;
  earlyFailures.textContent = `${summaryState?.failed_before_summary_sessions ?? 0}`;
  directCommits.textContent = `${summaryState?.direct_unicode_commits ?? 0}`;
  clipboardCommits.textContent = `${summaryState?.clipboard_fallback_commits ?? 0}`;
  selectionCommits.textContent = `${summaryState?.selection_replace_commits ?? 0}`;
  avgTotalLatency.textContent =
    typeof summaryState?.avg_total_session_latency_ms === "number"
      ? `${summaryState.avg_total_session_latency_ms} ms`
      : "--";
  avgStartLatency.textContent =
    typeof summaryState?.avg_recording_start_latency_ms === "number"
      ? `${summaryState.avg_recording_start_latency_ms} ms`
      : "--";
  avgAudioDuration.textContent =
    typeof summaryState?.avg_audio_duration_ms === "number"
      ? `${summaryState.avg_audio_duration_ms} ms`
      : "--";
  avgAudioRms.textContent =
    typeof summaryState?.avg_audio_rms_level === "number"
      ? summaryState.avg_audio_rms_level.toFixed(3)
      : "--";
  dictationMix.textContent = `${t("diagnostics.dictation")} ${summaryState?.dictation_sessions ?? 0}`;
  editMix.textContent = `${t("diagnostics.edit")} ${summaryState?.selected_text_sessions ?? 0}`;
  intentMix.textContent = `${t("diagnostics.intent")} ${summaryState?.wake_phrase_sessions ?? 0}`;
  renderActionMix(
    selectedTextActionMix,
    summaryState?.selected_text_action_mix ?? [],
    currentLanguage() === "Chinese" ? "暂无编辑动作" : "No edit actions yet",
  );
  renderActionMix(
    wakePhraseActionMix,
    summaryState?.wake_phrase_action_mix ?? [],
    currentLanguage() === "Chinese"
      ? "暂无指令式听写动作"
      : "No instructed dictation actions yet",
  );
  localOnlyMix.textContent = `${t("common.localOnly")} ${summaryState?.local_asr_only_sessions ?? 0}`;
  localRefineMix.textContent = `${t("diagnostics.localRefine")} ${summaryState?.local_asr_with_refine_sessions ?? 0}`;
  cloudRefineMix.textContent = `${t("diagnostics.cloudRefine")} ${summaryState?.cloud_asr_with_refine_sessions ?? 0}`;
  silenceFailures.textContent = `Silence gate ${summaryState?.silence_gate_failures ?? 0}`;
  noSpeechFailures.textContent = `No speech ${summaryState?.no_speech_failures ?? 0}`;
  commitFailures.textContent = `Commit ${summaryState?.committing_failures ?? 0}`;
  recordingFailures.textContent = `Recording ${summaryState?.recording_failures ?? 0}`;
  recognizingFailures.textContent = `Recognizing ${summaryState?.recognizing_failures ?? 0}`;
  dominantFailureMode.textContent =
    summaryState?.dominant_failure_mode ?? "No dominant failure mode yet.";
  dominantFailureGuidance.textContent =
    summaryState?.dominant_failure_guidance ??
    "Failure guidance will appear here once the live host has enough failed sessions to classify.";
  if (failureProfileSection) {
    failureProfileSection.hidden = (summaryState?.failed_sessions ?? 0) === 0;
  }
  const commitPathOutlook = summaryState?.commit_path_outlook ?? "Unavailable";
  commitPathOutlookChip.textContent = commitPathOutlook;
  commitPathOutlookChip.classList.remove("is-stable", "is-watch", "is-alert");
  const commitPathTone = getCommitPathOutlookTone(commitPathOutlook);
  if (commitPathTone) {
    commitPathOutlookChip.classList.add(commitPathTone);
  }
  commitPathSignal.textContent =
    summaryState?.commit_path_signal ?? "No commit-path signal yet.";
  commitPathGuidance.textContent =
    summaryState?.commit_path_guidance ??
    "Commit transport guidance will appear here once the mirrored live host has enough successful sessions to classify.";
  const verificationFocusLabel = summaryState?.verification_focus ?? "Unavailable";
  verificationFocusChip.textContent = verificationFocusLabel;
  verificationFocusChip.classList.remove("is-stable", "is-watch", "is-alert");
  const verificationFocusTone = getVerificationFocusTone(verificationFocusLabel);
  if (verificationFocusTone) {
    verificationFocusChip.classList.add(verificationFocusTone);
  }
  verificationFocus.textContent =
    summaryState?.verification_focus ?? "No verification focus yet.";
  verificationFocusGuidance.textContent =
    summaryState?.verification_focus_guidance ??
    "The diagnostics runbook will become more specific once the mirrored live host has enough signal to suggest what kind of test should happen next.";
  if (verificationFocusSection) {
    verificationFocusSection.hidden = !summaryState?.verification_focus;
  }
  const workloadFocusLabel = summaryState?.workload_focus ?? "Unavailable";
  workloadFocusChip.textContent = workloadFocusLabel;
  workloadFocusChip.classList.remove("is-stable", "is-watch", "is-alert");
  const workloadFocusTone = getWorkloadFocusTone(workloadFocusLabel);
  if (workloadFocusTone) {
    workloadFocusChip.classList.add(workloadFocusTone);
  }
  workloadFocus.textContent =
    summaryState?.workload_focus ?? "No workload focus yet.";
  workloadFocusGuidance.textContent =
    summaryState?.workload_focus_guidance ??
    "This card will summarize what the runtime is mostly being used for once the mirrored live host has enough successful sessions to classify recent workload shape.";
  if (workloadFocusSection) {
    workloadFocusSection.hidden = !summaryState?.workload_focus;
  }
  renderTuningRecommendations(summaryState?.tuning_recommendations ?? []);
  const freshness = getFreshnessState(summaryState?.report_generated_at_epoch_ms);
  renderOverviewDiagnosticEvidence(summaryState, freshness);
  reportFreshness.textContent = freshness.note;
  reportRecencyChip.textContent = freshness.label;
  reportRecencyChip.classList.remove("is-fresh", "is-recent", "is-aging", "is-stale");
  reportRecencyChip.classList.add(freshness.tone);
  if (summaryState?.latest_successful_session) {
    const success = summaryState.latest_successful_session;
    const visibleText = success.committed_text || success.recognized_text || "Latest committed session mirrored.";
    const routeLabel = success.route_decision?.route_name ?? "Unknown route";
    const selectedTextAction = toSelectedTextActionLabel(
      success.selected_text_execution?.action,
    );
    const wakePhraseAction = toWakePhraseActionLabel(
      success.wake_phrase_execution?.action,
    );
    const fallbackLabel = success.degraded_to_asr
      ? "ASR fallback"
      : success.fallback_reason
        ? "Fallback used"
        : null;
    lastSuccess.textContent = visibleText;
    lastSuccessMeta.textContent = `Session ${success.session_id} | ${success.session_kind} | ${routeLabel} | ${success.commit_transport}${selectedTextAction || wakePhraseAction ? ` | ${selectedTextAction ?? wakePhraseAction}` : ""}${fallbackLabel ? ` | ${fallbackLabel}` : ""}${success.fallback_reason ? ` | ${success.fallback_reason}` : ""}`;
  } else {
    lastSuccess.textContent = t("diagnostics.noSuccessful");
    lastSuccessMeta.textContent = t("diagnostics.nextCompleted");
  }
  lastFailure.textContent =
    summaryState?.last_failure ??
    t("diagnostics.noFailure");
  if (summaryState?.last_failure_summary) {
    const failure = summaryState.last_failure_summary;
    const metrics = [];
    if (typeof failure.audio_duration_ms === "number") {
      metrics.push(`${failure.audio_duration_ms} ms`);
    }
    if (typeof failure.audio_peak_level === "number") {
      metrics.push(`peak ${failure.audio_peak_level.toFixed(3)}`);
    }
    if (typeof failure.audio_rms_level === "number") {
      metrics.push(`rms ${failure.audio_rms_level.toFixed(3)}`);
    }
    const compatibility = failure.selected_text_compatibility;
    lastFailureMeta.textContent = `Session ${failure.session_id} | ${failure.session_kind} | ${failure.failure_phase}${metrics.length > 0 ? ` | ${metrics.join(" / ")}` : ""}${compatibility ? ` | ${compatibility.reason}` : ""}`;
    if (compatibility) {
      lastFailure.textContent = `${failure.error} ${compatibility.guidance}`;
    }
  } else {
    lastFailureMeta.textContent = t("diagnostics.failureShort");
  }
  failureHistory.replaceChildren();
  const recentFailures = summaryState?.recent_failures ?? [];
  if (recentFailurePatternGroup) {
    recentFailurePatternGroup.hidden = recentFailures.length === 0;
  }
  updateDiagnosticPatternsVisibility();
  if (recentFailures.length === 0) {
    const item = document.createElement("li");
    item.textContent = "No recent failure pattern mirrored yet.";
    failureHistory.appendChild(item);
  } else {
    recentFailures.forEach((failure) => {
      const item = document.createElement("li");
      const compatibility = failure.selected_text_compatibility;
      item.textContent = `#${failure.attempted_session_index} | Session ${failure.session_id} | ${failure.session_kind} | ${failure.failure_phase}${compatibility ? ` | ${compatibility.reason}` : ""} | ${failure.error}`;
      failureHistory.appendChild(item);
    });
  }

  runtimeHealth.classList.remove("is-stable", "is-watch", "is-alert");
  if (!summaryState || summaryState.attempted_sessions === 0) {
    runtimeHealth.textContent = t("common.idle");
    healthNote.textContent = "Use VoiceFlow once to see a health summary.";
    return;
  }

  const failureRate = summaryState.failed_sessions / Math.max(summaryState.attempted_sessions, 1);
  if (summaryState.failed_sessions === 0) {
    runtimeHealth.textContent = "Stable";
    runtimeHealth.classList.add("is-stable");
    healthNote.textContent = `All ${summaryState.successful_sessions} mirrored session${summaryState.successful_sessions === 1 ? "" : "s"} completed cleanly.`;
    return;
  }

  if (failureRate >= 0.4) {
    runtimeHealth.textContent = "Needs attention";
    runtimeHealth.classList.add("is-alert");
  } else {
    runtimeHealth.textContent = "Mixed";
    runtimeHealth.classList.add("is-watch");
  }

  healthNote.textContent = `${summaryState.failed_sessions} of ${summaryState.attempted_sessions} recent session${summaryState.attempted_sessions === 1 ? "" : "s"} did not complete.`;
}

function renderUsageChart(analytics) {
  if (!usageTrendChart) {
    return;
  }

  const locale = currentLanguage() === "Chinese" ? "zh-CN" : "en-US";
  const signature = JSON.stringify({
    locale,
    daily: analytics.daily.map((day) => [
      day.dateKey,
      day.insertedTextUnits,
      Math.round(day.estimatedTimeSavedMs),
    ]),
  });
  if (signature === usageChartSignature) {
    return;
  }
  usageChartSignature = signature;

  const maximumUnits = Math.max(
    1,
    ...analytics.daily.map((day) => day.insertedTextUnits),
  );
  const maximumSavedMs = Math.max(
    1,
    ...analytics.daily.map((day) => day.estimatedTimeSavedMs),
  );
  const weekdayFormatter = new Intl.DateTimeFormat(locale, {
    weekday: "short",
    timeZone: "UTC",
  });
  const weekUnits = analytics.daily.reduce(
    (total, day) => total + day.insertedTextUnits,
    0,
  );

  usageTrendChart.replaceChildren();
  const leftScale = document.createElement("div");
  leftScale.className = "usage-chart-scale is-left";
  leftScale.innerHTML = `<span>${formatCompactNumber(maximumUnits)}</span><span>0</span>`;

  const plot = document.createElement("div");
  plot.className = "usage-chart-plot";
  const bars = document.createElement("div");
  bars.className = "usage-chart-bars";
  analytics.daily.forEach((day) => {
    const dayElement = document.createElement("div");
    dayElement.className = `usage-chart-day ${day.insertedTextUnits === 0 ? "is-empty" : ""}`;
    dayElement.title = t("overview.chartTooltip")
      .replace("{words}", formatInteger(day.insertedTextUnits))
      .replace("{time}", formatDuration(day.estimatedTimeSavedMs / 1000));
    const bar = document.createElement("span");
    bar.className = "usage-chart-bar";
    bar.style.height = `${day.insertedTextUnits === 0 ? 2 : Math.max(6, (day.insertedTextUnits / maximumUnits) * 100)}%`;
    dayElement.appendChild(bar);
    bars.appendChild(dayElement);
  });
  plot.appendChild(bars);

  const svgNamespace = "http://www.w3.org/2000/svg";
  const line = document.createElementNS(svgNamespace, "svg");
  line.classList.add("usage-chart-line");
  line.setAttribute("viewBox", "0 0 700 100");
  line.setAttribute("preserveAspectRatio", "none");
  line.setAttribute("aria-hidden", "true");
  const points = analytics.daily.map((day, index) => {
    const x = index * 100 + 50;
    const y = 96 - (day.estimatedTimeSavedMs / maximumSavedMs) * 88;
    return { x, y };
  });
  const polyline = document.createElementNS(svgNamespace, "polyline");
  polyline.setAttribute(
    "points",
    points.map((point) => `${point.x},${point.y}`).join(" "),
  );
  line.appendChild(polyline);
  points.forEach((point) => {
    const marker = document.createElementNS(svgNamespace, "circle");
    marker.setAttribute("cx", point.x);
    marker.setAttribute("cy", point.y);
    marker.setAttribute("r", "3");
    line.appendChild(marker);
  });
  plot.appendChild(line);

  const rightScale = document.createElement("div");
  rightScale.className = "usage-chart-scale is-right";
  rightScale.innerHTML = `<span>${formatDuration(maximumSavedMs / 1000)}</span><span>0</span>`;

  const labels = document.createElement("div");
  labels.className = "usage-chart-labels";
  analytics.daily.forEach((day) => {
    const label = document.createElement("span");
    label.textContent = weekdayFormatter.format(new Date(day.weekdayEpochMs));
    labels.appendChild(label);
  });

  usageTrendChart.appendChild(leftScale);
  usageTrendChart.appendChild(plot);
  usageTrendChart.appendChild(rightScale);
  usageTrendChart.appendChild(labels);

  const weekLabel = t("overview.wordsInWeek").replace(
    "{count}",
    formatCompactNumber(weekUnits),
  );
  usageTrendChart.setAttribute("aria-label", t("overview.chartAria"));
  if (usageTrendTotal) {
    usageTrendTotal.textContent = weekLabel;
  }
}

function renderUsageSummary(usage) {
  const analytics = window.VoiceFlowOverviewAnalytics.buildOverviewAnalytics(usage);
  const hasUsage = analytics.completedSessions > 0;
  if (usageEmptyState) {
    usageEmptyState.textContent = t("overview.noUsage");
    usageEmptyState.hidden = hasUsage;
  }
  renderUsageChart(analytics);
  if (usageRecordingDuration) {
    usageRecordingDuration.textContent = formatDuration(
      analytics.totals.audioDurationMs / 1000,
    );
  }
  if (usageInsertedText) {
    usageInsertedText.textContent = formatCompactNumber(
      analytics.totals.insertedTextUnits,
    );
  }
  if (usageTimeSaved) {
    usageTimeSaved.textContent = formatDuration(
      analytics.totals.estimatedTimeSavedMs / 1000,
    );
  }
  if (usageAverageSpeed) {
    const hasAverageSpeed = analytics.totals.averageUnitsPerMinute !== null;
    usageAverageSpeed.textContent = hasAverageSpeed
      ? formatInteger(Math.round(analytics.totals.averageUnitsPerMinute))
      : "--";
    if (usageAverageSpeedSuffix) {
      usageAverageSpeedSuffix.hidden = !hasAverageSpeed;
    }
  }
  if (usageLatestApp) {
    usageLatestApp.textContent =
      usage?.latest_app?.process_name ??
      usage?.latest_app?.window_class ??
      "Not available";
  }
  if (usageAvgLatency) {
    usageAvgLatency.textContent =
      typeof usage?.average_end_to_end_latency_ms === "number"
        ? `${usage.average_end_to_end_latency_ms} ms`
        : "--";
  }
  if (usageRouteLocalOnly) {
    usageRouteLocalOnly.textContent = formatInteger(
      usage?.route_counts?.local_asr_only_sessions ?? 0,
    );
  }
  if (usageRouteLocalRefine) {
    usageRouteLocalRefine.textContent = formatInteger(
      usage?.route_counts?.local_asr_with_refine_sessions ?? 0,
    );
  }
  if (usageRouteCloudRefine) {
    usageRouteCloudRefine.textContent = formatInteger(
      usage?.route_counts?.cloud_asr_with_refine_sessions ?? 0,
    );
  }
}

function renderPromptOverrides(promptOverrides) {
  if (!promptOverridesList) {
    return;
  }

  if (promptOverridesStatus && !promptOverrideReloadInFlight && bridgeConnected) {
    promptOverridesStatus.textContent = t("settings.promptStatusReady");
    promptOverridesStatus.classList.toggle("is-error", false);
  }

  const slots = Array.isArray(promptOverrides) ? promptOverrides : [];
  promptOverridesList.replaceChildren();

  if (slots.length === 0) {
    const empty = document.createElement("p");
    empty.className = "report-note";
    empty.textContent = t("common.unavailable");
    promptOverridesList.appendChild(empty);
    return;
  }

  slots.forEach((slot) => {
    const row = document.createElement("div");
    row.className = "prompt-override-row";

    const body = document.createElement("div");
    body.className = "prompt-override-body";

    const title = document.createElement("div");
    title.className = "prompt-override-title";
    title.textContent = promptOverrideSlotLabel(slot);

    const env = document.createElement("p");
    env.className = "report-note";
    env.textContent = `${t("settings.promptEnvVar")}: ${slot.env_var ?? "--"}`;

    const path = document.createElement("p");
    path.className = "report-note prompt-override-path";
    path.textContent = slot.configured_path
      ? `${t("settings.promptPath")}: ${slot.configured_path}`
      : t("settings.promptNoPath");
    if (slot.configured_path) {
      path.title = slot.configured_path;
    }

    body.appendChild(title);
    if (slot.configured_path) { body.appendChild(env); body.appendChild(path); }

    if (slot.warning) {
      const warning = document.createElement("p");
      warning.className = "report-note prompt-override-warning";
      warning.textContent = slot.warning;
      body.appendChild(warning);
    }

    const chip = document.createElement("span");
    chip.className = `report-chip prompt-override-chip is-${slot.status ?? "default"}`;
    chip.textContent = promptOverrideStatusLabel(slot.status);

    row.appendChild(body);
    row.appendChild(chip);
    const edit = document.createElement("button");
    edit.type = "button";
    edit.className = "secondary-button prompt-edit-button";
    edit.textContent = currentLanguage() === "Chinese" ? "编辑提示词" : "Edit prompt";
    edit.disabled = !bridgeConnected || !getSettingsControlToken();
    edit.title = edit.disabled ? t("settings.promptBridgeRequired") : "";
    edit.addEventListener("click", () => openPromptEditor(slot));
    row.appendChild(edit);
    promptOverridesList.appendChild(row);
  });
}

function openPromptEditor(slot) {
  const label = (zh, en) => currentLanguage() === "Chinese" ? zh : en;
  const element = (tag, text, className) => {
    const node = document.createElement(tag);
    if (text) node.textContent = text;
    if (className) node.className = className;
    return node;
  };
  const dialog = element("dialog", "", "prompt-editor-dialog");
  dialog.setAttribute("aria-labelledby", "prompt-editor-title");
  const title = element("h2", promptOverrideSlotLabel(slot)); title.id = "prompt-editor-title";
  const warning = element("p", label(
    "提示词会影响文字准确性、润色程度和输出格式。通常无需修改；不合适的修改可能造成内容遗漏或语意变化。请勿填入密码或 API Key。",
    "Prompts affect accuracy, rewriting, and formatting. Changes are usually unnecessary and may omit or alter meaning. Do not enter passwords or API keys."), "prompt-editor-warning");
  const status = element("p", "", "prompt-editor-status"); status.setAttribute("role", "status");
  const source = element("p", "", "report-note");
  const input = element("textarea", "", "prompt-editor-input");
  input.setAttribute("aria-label", promptOverrideSlotLabel(slot)); input.spellcheck = false;
  input.hidden = true;
  const actions = element("div", "", "prompt-editor-actions");
  const button = (zh, en, fn) => { const b = element("button", label(zh, en), "secondary-button"); b.type = "button"; b.addEventListener("click", fn); return b; };
  const confirmArea = element("div", "", "prompt-editor-confirm"); confirmArea.hidden = true;
  let loaded = null, busy = false, resetPending = false;
  const dirty = () => loaded && (resetPending || input.value !== loaded.text);
  const beforeUnload = (event) => { if (dirty() || busy) { event.preventDefault(); event.returnValue = ""; } };
  window.addEventListener("beforeunload", beforeUnload);
  const confirm = (message, accept) => {
    confirmArea.replaceChildren(element("p", message)); confirmArea.hidden = false;
    const yes = button("确认", "Confirm", () => { confirmArea.hidden = true; accept(); });
    confirmArea.append(yes, button("取消", "Cancel", () => { confirmArea.hidden = true; })); yes.focus();
  };
  const dispose = () => { window.removeEventListener("beforeunload", beforeUnload); dialog.close(); dialog.remove(); };
  const close = button("关闭", "Close", () => {
    if (busy) return;
    if (dirty()) confirm(label("放弃未保存的提示词修改？", "Discard unsaved prompt changes?"), dispose);
    else dispose();
  });
  const refresh = () => {
    unlock.disabled = busy; close.disabled = busy; input.disabled = busy;
    save.disabled = busy || !dirty(); reset.disabled = busy || !loaded;
  };
  const request = async (action) => {
    busy = true; refresh(); status.textContent = label("处理中…", "Working…"); status.classList.remove("is-error");
    const controller = new AbortController(); const timeout = setTimeout(() => controller.abort(), 15000);
    try {
      const response = await fetch(`${getSettingsControlUrl()}/prompt-settings`, {
        method: "POST", headers: buildMutationHeaders(), signal: controller.signal,
        body: JSON.stringify({ slot_id: slot.slot_id, action, ...(action === "save" ? { text: input.value } : {}) }),
      });
      if (!response.ok) throw new Error("prompt request failed");
      loaded = await response.json(); input.value = loaded.text; resetPending = false;
      input.hidden = false; save.hidden = false; reset.hidden = false; unlock.hidden = true;
      source.textContent = loaded.source === "custom" ? label("已自定义", "Customized") : loaded.source === "file" ? label("当前来自文件覆盖", "Using a file override") : label("内置默认", "Built-in default");
      status.textContent = action === "read" ? label("修改后请单独保存。", "Save this prompt when ready.") : label("已保存，后续请求生效。", "Saved for subsequent requests.");
      if (action !== "read") {
        slot.status = loaded.source === "builtin" ? "default" : "custom_active";
        slot.configured_path = null; slot.env_var = "UI"; slot.warning = null;
        renderPromptOverrides(window.__VOICEFLOW_SETTINGS_RUNTIME__?.prompt_overrides);
      }
      input.focus();
    } catch {
      status.textContent = label("操作未确认成功。请确认 VoiceFlow 正在运行，并从实时设置页重试；当前输入已保留。", "Could not confirm success. Open live Settings with VoiceFlow running and retry; your input is preserved.");
      status.classList.add("is-error");
    } finally { clearTimeout(timeout); busy = false; refresh(); }
  };
  const unlock = button("我了解，开始编辑", "I understand — edit", () => request("read"));
  const save = button("保存提示词", "Save prompt", () => {
    if (!resetPending && (!input.value.trim() || new TextEncoder().encode(input.value).length > loaded.max_bytes)) {
      status.textContent = label("提示词不能为空，且不能超过 32,000 字节。", "Enter a nonempty prompt of at most 32,000 UTF-8 bytes."); return;
    }
    request(resetPending ? "reset" : "save");
  });
  save.classList.add("apply-button"); save.hidden = true;
  const reset = button("恢复默认", "Reset to default", () => confirm(
    label("用内置提示词替换当前内容？点击保存后生效。", "Replace this draft with the built-in prompt? Save to apply."),
    () => { input.value = loaded.builtin_text; resetPending = true; status.textContent = label("已载入默认值，尚未保存。", "Default loaded; not saved yet."); refresh(); input.focus(); }
  )); reset.hidden = true;
  input.addEventListener("input", () => { resetPending = false; refresh(); });
  dialog.addEventListener("cancel", (event) => { event.preventDefault(); close.click(); });
  actions.append(reset, unlock, save, close);
  dialog.append(title, warning, source, input, status, confirmArea, actions);
  document.body.append(dialog); refresh(); dialog.showModal(); unlock.focus();
}

function promptOverrideSlotLabel(slot) {
  const translationKey = {
    dictation_light_cleanup: "settings.dictationLightCleanupPrompt",
    dictation_structured_cleanup: "settings.dictationStructuredCleanupPrompt",
    selected_text_edit: "settings.selectedTextPrompt",
    instructed_dictation: "settings.instructedDictationPrompt",
  }[slot?.slot_id];

  return translationKey ? t(translationKey) : slot?.display_label || t("common.unavailable");
}

function promptOverrideStatusLabel(status) {
  const translationKey = {
    default: "settings.promptDefault",
    custom_active: "settings.promptCustomActive",
    missing_file: "settings.promptMissingFile",
    empty_file: "settings.promptEmptyFile",
    unreadable: "settings.promptUnreadable",
  }[status];

  return translationKey ? t(translationKey) : t("common.unavailable");
}

function readOverviewPrivacyPreference() {
  try {
    return window.localStorage.getItem("voiceflow.overview.hideRecentPreviews") === "true";
  } catch (_error) {
    return false;
  }
}

function persistOverviewPrivacyPreference() {
  try {
    window.localStorage.setItem(
      "voiceflow.overview.hideRecentPreviews",
      String(overviewPreviewsHidden),
    );
  } catch (_error) {
    // The display preference remains active for this page when storage is unavailable.
  }
}

function maskRecentPreview(text) {
  return text.replace(/\S/g, "*");
}

function renderOverviewPrivacyControl() {
  if (!overviewPrivacyToggle) {
    return;
  }
  const label = overviewPreviewsHidden
    ? t("overview.showPreviews")
    : t("overview.hidePreviews");
  overviewPrivacyToggle.setAttribute("aria-pressed", String(overviewPreviewsHidden));
  overviewPrivacyToggle.setAttribute("aria-label", label);
  overviewPrivacyToggle.title = label;
  if (overviewPrivacyVisibleIcon) {
    overviewPrivacyVisibleIcon.hidden = overviewPreviewsHidden;
  }
  if (overviewPrivacyHiddenIcon) {
    overviewPrivacyHiddenIcon.hidden = !overviewPreviewsHidden;
  }
}

function renderOverviewRecentEntries() {
  if (!overviewRecentList || !overviewRecentEmpty) {
    return;
  }

  overviewRecentList.replaceChildren();
  overviewRecentEmpty.hidden = overviewRecentEntries.length > 0;
  overviewRecentEntries.forEach((entry) => {
    const item = document.createElement("article");
    item.className = "overview-recent-entry";

    const originalPreview = entry.text || t("history.noText");
    const preview = document.createElement("p");
    preview.textContent = overviewPreviewsHidden
      ? maskRecentPreview(originalPreview)
      : originalPreview;
    const timestamp = document.createElement("time");
    timestamp.dateTime = new Date(entry.timestamp_epoch_ms).toISOString();
    timestamp.textContent = formatHistoryTimestamp(entry.timestamp_epoch_ms);

    item.appendChild(preview);
    item.appendChild(timestamp);
    overviewRecentList.appendChild(item);
  });
  renderOverviewPrivacyControl();
}

function renderOverviewHistory(entries) {
  overviewRecentEntries = entries
    .filter(
      (entry) =>
        entry.mode === "Dictation" ||
        entry.mode === "InstructedInput" ||
        entry.mode === "InstructedDictation" ||
        entry.mode === "WakePhraseIntent",
    )
    .slice(0, 5);
  renderOverviewRecentEntries();
}

function renderHistorySummary(history) {
  const entries = Array.isArray(history?.entries) ? history.entries : [];
  const hasEntries = entries.length > 0;
  const nextHistoryRenderSignature = buildHistoryRenderSignature(entries);

  renderOverviewHistory(entries);

  if (historyEmptyState) {
    if (runtimeDataSource === "live" && hasEntries) {
      historyEmptyState.textContent = t("history.liveEntries")
        .replace("{count}", formatInteger(entries.length))
        .replace("{policy}", historyRetentionLabel(state.historyRetention));
    } else if (runtimeDataSource === "live") {
      historyEmptyState.textContent = t("history.emptyWithPolicy").replace(
        "{policy}",
        historyRetentionLabel(state.historyRetention),
      );
    } else {
      historyEmptyState.textContent = t("history.staticSnapshot");
    }
  }

  if (!historyList) {
    return;
  }

  if (nextHistoryRenderSignature === historyRenderSignature) {
    return;
  }
  historyRenderSignature = nextHistoryRenderSignature;

  historyList.replaceChildren();
  if (!hasEntries) {
    return;
  }

  entries.forEach((entry) => {
    const card = document.createElement("article");
    card.className = `history-entry ${entry.redacted ? "is-redacted" : ""}`;

    const header = document.createElement("div");
    header.className = "history-entry-head";

    const headingWrap = document.createElement("div");
    const label = document.createElement("p");
    label.className = "report-label";
    label.textContent = formatHistoryTimestamp(entry.timestamp_epoch_ms);
    const title = document.createElement("h3");
    title.textContent = buildHistoryTitle(entry);
    headingWrap.appendChild(label);
    headingWrap.appendChild(title);
    header.appendChild(headingWrap);

    if (!entry.redacted && entry.text) {
      const copyButton = document.createElement("button");
      copyButton.type = "button";
      copyButton.className = "history-copy-button";
      copyButton.setAttribute("aria-live", "polite");
      copyButton.textContent = t("history.copy");
      copyButton.addEventListener("click", () => copyHistoryText(entry.text, copyButton));
      header.appendChild(copyButton);
    }

    const body = document.createElement("p");
    body.className = "history-entry-text";
    body.textContent = entry.redacted
      ? t("history.redacted")
      : entry.text || t("history.noText");

    card.appendChild(header);
    card.appendChild(body);

    const developerMeta = buildHistoryDeveloperMeta(entry);
    if (developerMeta) {
      const details = document.createElement("details");
      details.className = "history-developer-details";
      const summary = document.createElement("summary");
      summary.textContent = t("history.developerDetails");
      const developerCopy = document.createElement("p");
      developerCopy.className = "report-note";
      developerCopy.textContent = developerMeta;
      details.appendChild(summary);
      details.appendChild(developerCopy);
      card.appendChild(details);
    }

    historyList.appendChild(card);
  });
}

function buildHistoryRenderSignature(entries) {
  return JSON.stringify({
    language: currentLanguage(),
    entries: entries.map((entry) => ({
      entryId: entry.entry_id ?? `${entry.source_run_id ?? ""}:${entry.session_id ?? ""}`,
      timestamp: entry.timestamp_epoch_ms ?? null,
      mode: entry.mode ?? "",
      text: entry.text ?? null,
      redacted: entry.redacted === true,
      modelName: entry.model_name ?? null,
      modelCode: entry.model_code ?? null,
      providerPreset: entry.provider_preset ?? null,
      providerAttempted: entry.provider_attempted ?? null,
      providerSucceeded: entry.provider_succeeded ?? null,
      deterministicFallbackUsed: entry.deterministic_fallback_used ?? null,
      selectedTextActionLabel: entry.selected_text_action_label ?? null,
    })),
  });
}

function buildHistoryTitle(entry) {
  if (entry.mode === "Dictation") {
    return t("history.dictationInserted");
  }
  if (entry.mode === "SelectedTextEdit") {
    return t("history.selectedEdited");
  }
  if (
    entry.mode === "InstructedInput" ||
    entry.mode === "InstructedDictation" ||
    entry.mode === "WakePhraseIntent"
  ) {
    return t("history.instructedCompleted");
  }
  return t("history.inputCompleted");
}

function historyRetentionLabel(value) {
  const translationKey = {
    latest_100: "settings.latest100",
    latest_500: "settings.latest500",
    latest_1000: "settings.latest1000",
    last_7_days: "settings.last7Days",
    last_30_days: "settings.last30Days",
    unlimited: "settings.unlimited",
  }[value];
  return translationKey ? t(translationKey) : t("settings.latest100");
}

function renderHistoryRetentionControls() {
  historyRetentionButtons.forEach((button) => {
    const isActive = button.dataset.retentionValue === state.historyRetention;
    button.classList.toggle("is-active", isActive);
    button.setAttribute("aria-pressed", String(isActive));
  });
  if (historyRetentionWarning) {
    historyRetentionWarning.hidden = state.historyRetention !== "unlimited";
  }
}

function buildHistoryDeveloperMeta(entry) {
  if (entry.provider_attempted === false) {
    return "";
  }

  const executionLabel = entry.model_code || entry.model_name;
  if (
    entry.provider_attempted === true &&
    executionLabel &&
    typeof entry.provider_succeeded === "boolean"
  ) {
    return [
      executionLabel,
      entry.provider_succeeded
        ? t("history.aiProcessingSucceeded")
        : t("history.aiProcessingFailed"),
    ]
      .filter(Boolean)
      .join(" · ");
  }

  return "";
}

function formatHistoryTimestamp(epochMs) {
  if (typeof epochMs !== "number") {
    return t("history.recent");
  }

  return new Intl.DateTimeFormat(currentLanguage() === "Chinese" ? "zh-CN" : undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(epochMs));
}

async function copyHistoryText(text, button) {
  const previousLabel = button.textContent;
  try {
    await navigator.clipboard.writeText(text);
    button.textContent = t("history.copied");
  } catch (_error) {
    button.textContent = t("history.copyFailed");
  }

  window.setTimeout(() => {
    button.textContent = previousLabel;
  }, 1400);
}

function formatInteger(value) {
  return Number(value ?? 0).toLocaleString();
}

function formatCompactNumber(value) {
  const number = Math.max(0, Number(value) || 0);
  if (number < 1_000) {
    return formatInteger(Math.round(number));
  }
  return new Intl.NumberFormat(currentLanguage() === "Chinese" ? "zh-CN" : "en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(number);
}

function formatDuration(totalSeconds) {
  const seconds = Math.max(0, Number(totalSeconds) || 0);
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  }
  const minutes = seconds / 60;
  if (minutes < 60) {
    return `${Math.round(minutes)}m`;
  }
  const hours = minutes / 60;
  return `${hours.toFixed(hours < 10 ? 1 : 0)}h`;
}

function renderReportWarning(warning) {
  if (!warning) {
    reportWarning.textContent = "";
    reportWarning.classList.add("is-hidden");
    return;
  }

  reportWarning.textContent = warning;
  reportWarning.classList.remove("is-hidden");
}

function renderTuningRecommendations(recommendations) {
  if (tuningSection) {
    tuningSection.hidden = recommendations.length === 0;
  }
  if (!tuningRecommendations || !tuningRecommendationsChip) {
    return;
  }

  latestTuningRecommendations = recommendations;
  tuningRecommendations.replaceChildren();
  tuningRecommendationsChip.classList.remove("is-stable", "is-watch", "is-alert");
  const applyReadyRecommendations = recommendations.filter(
    (recommendation) => !recommendation.saved_already_matches_target,
  );
  const restartOnlyRecommendations = recommendations.filter(
    (recommendation) => recommendation.restart_required,
  );
  const stagedApplyRecommendations = applyReadyRecommendations.filter((recommendation) =>
    isRecommendationAlreadyStaged(recommendation),
  );
  const unstagedApplyRecommendations = applyReadyRecommendations.filter(
    (recommendation) => !isRecommendationAlreadyStaged(recommendation),
  );
  renderTuningNextAction(
    unstagedApplyRecommendations,
    stagedApplyRecommendations,
    restartOnlyRecommendations,
  );
  renderTuningWorkflow(
    recommendations,
    unstagedApplyRecommendations,
    stagedApplyRecommendations,
    restartOnlyRecommendations,
  );
  renderTuningRunbook(
    recommendations,
    unstagedApplyRecommendations,
    stagedApplyRecommendations,
    restartOnlyRecommendations,
  );

  if (!recommendations.length) {
    tuningRecommendationsChip.textContent = "Idle";
    tuningRecommendationsChip.classList.add("is-stable");
    const empty = document.createElement("p");
    empty.className = "report-note";
    empty.textContent =
      "Host-derived tuning suggestions will appear here once the mirrored live host has a clearer failure pattern.";
    tuningRecommendations.appendChild(empty);
    return;
  }

  if (applyReadyRecommendations.length > 0 && restartOnlyRecommendations.length > 0) {
    tuningRecommendationsChip.textContent = `${applyReadyRecommendations.length} apply | ${restartOnlyRecommendations.length} restart`;
    tuningRecommendationsChip.classList.add("is-watch");
  } else if (applyReadyRecommendations.length > 0) {
    tuningRecommendationsChip.textContent = `${applyReadyRecommendations.length} apply-ready`;
    tuningRecommendationsChip.classList.add("is-watch");
  } else {
    tuningRecommendationsChip.textContent = `${restartOnlyRecommendations.length} restart-only`;
    tuningRecommendationsChip.classList.add("is-stable");
  }

  recommendations.forEach((recommendation) => {
    const card = document.createElement("article");
    card.className = "recommendation-card";

    const metaRow = document.createElement("div");
    metaRow.className = "recommendation-meta";

    const scope = document.createElement("span");
    scope.className = "mix-pill";
    scope.textContent = recommendation.panel ?? "Runtime";

    const strength = document.createElement("span");
    strength.className = "mix-pill recommendation-strength";
    strength.textContent = recommendation.strength ?? "Signal";

    const stateChip = document.createElement("span");
    stateChip.className = "mix-pill recommendation-state";
    if (recommendation.restart_required) {
      stateChip.textContent = "Saved already | restart needed";
    } else if (recommendation.saved_already_matches_target) {
      stateChip.textContent = "Saved already";
    } else {
      stateChip.textContent = "Apply through host";
    }

    const heading = document.createElement("p");
    heading.className = "recommendation-title";
    heading.textContent = recommendation.label;

    const evidence = document.createElement("p");
    evidence.className = "recommendation-evidence";
    evidence.textContent =
      recommendation.evidence_summary ??
      `${recommendation.evidence_count ?? 0} recent runtime signal${recommendation.evidence_count === 1 ? "" : "s"}.`;

    const rationale = document.createElement("p");
    rationale.className = "recommendation-copy";
    rationale.textContent = recommendation.rationale;

    const values = document.createElement("p");
    values.className = "recommendation-values";
    values.textContent = `Live: ${formatRecommendationValue(
      recommendation.setting_key,
      recommendation.live_value,
    )} | Saved: ${formatRecommendationValue(
      recommendation.setting_key,
      recommendation.saved_value,
    )} | Target: ${formatRecommendationValue(
      recommendation.setting_key,
      recommendation.target_value,
    )}`;

    const button = document.createElement("button");
    button.type = "button";
    button.className = "secondary-button recommendation-action";
    button.dataset.settingKey = recommendation.setting_key;
    button.dataset.targetValue = recommendation.target_value;
    if (recommendation.restart_required) {
      button.textContent = "Saved already | restart live host";
      button.disabled = true;
    } else if (recommendation.saved_already_matches_target) {
      button.textContent = "Saved already";
      button.disabled = true;
    } else if (isRecommendationAlreadyStaged(recommendation)) {
      button.textContent = "Already staged";
      button.disabled = true;
    } else {
      button.textContent = "Stage in local draft";
      button.disabled = false;
    }

    metaRow.appendChild(scope);
    metaRow.appendChild(strength);
    metaRow.appendChild(stateChip);
    card.appendChild(metaRow);
    card.appendChild(heading);
    card.appendChild(evidence);
    card.appendChild(values);
    card.appendChild(rationale);
    card.appendChild(button);
    tuningRecommendations.appendChild(card);
  });
}

function renderTuningNextAction(
  unstagedApplyRecommendations,
  stagedApplyRecommendations,
  restartOnlyRecommendations,
) {
  if (
    !tuningNextActionChip ||
    !tuningNextActionHeadline ||
    !tuningNextActionCopy
  ) {
    return;
  }

  tuningNextActionChip.classList.remove("is-stable", "is-watch", "is-alert");

  if (
    !unstagedApplyRecommendations.length &&
    !stagedApplyRecommendations.length &&
    !restartOnlyRecommendations.length
  ) {
    tuningNextActionChip.textContent = "Observe";
    tuningNextActionChip.classList.add("is-stable");
    tuningNextActionHeadline.textContent = "No host-derived action is pending yet.";
    tuningNextActionCopy.textContent =
      "Once the mirrored live host accumulates enough failure evidence, this panel will suggest whether you should apply settings changes, restart the runtime, or keep observing.";
    return;
  }

  if (stagedApplyRecommendations.length > 0) {
    tuningNextActionChip.textContent = "Apply staged";
    tuningNextActionChip.classList.add("is-watch");
    tuningNextActionHeadline.textContent =
      stagedApplyRecommendations.length === 1
        ? "One suggested fix is already staged locally and should be applied next."
        : `${stagedApplyRecommendations.length} suggested fixes are already staged locally and should be applied next.`;
    tuningNextActionCopy.textContent =
      unstagedApplyRecommendations.length > 0
        ? `${unstagedApplyRecommendations.length} additional recommendation${unstagedApplyRecommendations.length === 1 ? "" : "s"} can still be staged before you apply.`
        : restartOnlyRecommendations.length > 0
          ? `${restartOnlyRecommendations.length} additional recommendation${restartOnlyRecommendations.length === 1 ? "" : "s"} are already saved and will only need a live-host restart afterward.`
          : "The local draft already contains the recommended host changes. Apply through the Rust host when you are ready.";
    return;
  }

  if (unstagedApplyRecommendations.length > 0) {
    tuningNextActionChip.textContent = "Apply next";
    tuningNextActionChip.classList.add("is-watch");
    tuningNextActionHeadline.textContent =
      unstagedApplyRecommendations.length === 1
        ? "One host-ready fix should be staged and applied next."
        : `${unstagedApplyRecommendations.length} host-ready fixes should be staged and applied next.`;
    tuningNextActionCopy.textContent =
      restartOnlyRecommendations.length > 0
        ? `${restartOnlyRecommendations.length} additional recommendation${restartOnlyRecommendations.length === 1 ? "" : "s"} are already saved and will only need a live-host restart afterward.`
        : "These recommendations still need a Rust-host apply before the runtime can pick them up.";
    return;
  }

  tuningNextActionChip.textContent = "Restart next";
  tuningNextActionChip.classList.add("is-stable");
  tuningNextActionHeadline.textContent =
    restartOnlyRecommendations.length === 1
      ? "The remaining recommendation is already saved and only needs a live-host restart."
      : `The remaining ${restartOnlyRecommendations.length} recommendations are already saved and only need a live-host restart.`;
  tuningNextActionCopy.textContent =
    "The saved settings already match the proposed target values, so the live runtime is simply behind the current configuration.";
}

function renderTuningWorkflow(
  recommendations,
  unstagedApplyRecommendations,
  stagedApplyRecommendations,
  restartOnlyRecommendations,
) {
  const workflow = tuningStepStageChip?.closest(".tuning-workflow");
  if (workflow) {
    let idleMessage = workflow.querySelector(".tuning-idle-message");
    if (!idleMessage) {
      idleMessage = document.createElement("p");
      idleMessage.className = "report-note tuning-idle-message";
      workflow.prepend(idleMessage);
    }
    idleMessage.textContent =
      "No actionable tuning item is pending. Keep observing the next mirrored report.";
    workflow.classList.toggle("is-idle", recommendations.length === 0);
  }

  renderTuningStep(
    tuningStepStageChip,
    tuningStepStageCopy,
    unstagedApplyRecommendations.length > 0
      ? "Pending"
      : recommendations.length > 0
        ? "Done"
        : "Idle",
    unstagedApplyRecommendations.length > 0
      ? unstagedApplyRecommendations.length === 1
        ? "One recommended fix still needs to be staged locally."
        : `${unstagedApplyRecommendations.length} recommended fixes still need to be staged locally.`
      : recommendations.length > 0
        ? "No additional staging is required right now."
        : "No fixes need staging yet.",
  );

  renderTuningStep(
    tuningStepApplyChip,
    tuningStepApplyCopy,
    stagedApplyRecommendations.length > 0
      ? "Pending"
      : unstagedApplyRecommendations.length > 0
        ? "Waiting"
        : recommendations.length > 0
          ? "Done"
          : "Idle",
    stagedApplyRecommendations.length > 0
      ? stagedApplyRecommendations.length === 1
        ? "One staged fix is ready to apply through the Rust host."
        : `${stagedApplyRecommendations.length} staged fixes are ready to apply through the Rust host.`
      : unstagedApplyRecommendations.length > 0
        ? "Stage the recommended fixes first, then apply them through the Rust host."
        : recommendations.length > 0
          ? "No host apply is pending right now."
          : "No host apply is pending.",
  );

  renderTuningStep(
    tuningStepRestartChip,
    tuningStepRestartCopy,
    restartOnlyRecommendations.length > 0
      ? stagedApplyRecommendations.length > 0 || unstagedApplyRecommendations.length > 0
        ? "Waiting"
        : "Pending"
      : recommendations.length > 0
        ? "Done"
        : "Idle",
    restartOnlyRecommendations.length > 0
      ? stagedApplyRecommendations.length > 0 || unstagedApplyRecommendations.length > 0
        ? "A live-host restart is also needed, but only after the recommended apply path is complete."
        : restartOnlyRecommendations.length === 1
          ? "One saved fix is waiting on a live-host restart."
          : `${restartOnlyRecommendations.length} saved fixes are waiting on a live-host restart.`
      : recommendations.length > 0
        ? "No runtime restart is pending right now."
        : "No runtime restart is pending.",
  );

  renderTuningStep(
    tuningStepVerifyChip,
    tuningStepVerifyCopy,
    recommendations.length === 0
      ? "Pending"
      : "Waiting",
    recommendations.length === 0
      ? "Run the live host again and verify whether the next mirrored report stays healthier."
      : "Verification comes after staging, applying, and any required runtime restart.",
  );
}

function renderTuningStep(chip, copy, state, note) {
  if (!chip || !copy) {
    return;
  }

  chip.textContent = state;
  chip.classList.remove("is-stable", "is-watch", "is-alert");
  if (state === "Done" || state === "Idle") {
    chip.classList.add("is-stable");
  } else {
    chip.classList.add("is-watch");
  }
  copy.textContent = note;
}

function renderTuningRunbook(
  recommendations,
  unstagedApplyRecommendations,
  stagedApplyRecommendations,
  restartOnlyRecommendations,
) {
  if (!tuningRunbookChip || !tuningRunbookList) {
    return;
  }

  tuningRunbookList.replaceChildren();
  tuningRunbookChip.classList.remove("is-stable", "is-watch", "is-alert");

  const items = [];
  if (stagedApplyRecommendations.length > 0) {
    items.push({
      state: "Now",
      title:
        stagedApplyRecommendations.length === 1
          ? "Apply the staged tuning change through the Rust host."
          : `Apply the ${stagedApplyRecommendations.length} staged tuning changes through the Rust host.`,
      note: "Use the existing Apply button so the host stays authoritative over persisted settings.",
      command: currentHostApplyCommand(),
    });
  } else if (unstagedApplyRecommendations.length > 0) {
    items.push({
      state: "Now",
      title:
        unstagedApplyRecommendations.length === 1
          ? "Stage the recommended fix into the local draft."
          : `Stage the ${unstagedApplyRecommendations.length} recommended fixes into the local draft.`,
      note: "Use the per-card action or the bulk stage action before you apply through the host, or run the direct CLI shortcut below if you want to skip the page staging step.",
      command: buildRecommendationApplyCommand(unstagedApplyRecommendations),
    });
  }

  if (
    unstagedApplyRecommendations.length > 0 ||
    stagedApplyRecommendations.length > 0
  ) {
    items.push({
      state: stagedApplyRecommendations.length > 0 ? "Next" : "Then",
      title: "Persist the updated settings through the Rust host.",
      note: "After the draft looks right, use Apply through host to update the real settings file.",
      command: stagedApplyRecommendations.length > 0 ? currentHostApplyCommand() : null,
    });
  }

  if (
    restartOnlyRecommendations.length > 0 ||
    stagedApplyRecommendations.length > 0 ||
    unstagedApplyRecommendations.length > 0
  ) {
    items.push({
      state:
        restartOnlyRecommendations.length > 0 &&
        stagedApplyRecommendations.length === 0 &&
        unstagedApplyRecommendations.length === 0
          ? "Now"
          : "After apply",
      title: "Restart the live host before judging the new behavior.",
      note: `The running runtime can lag behind the saved settings, so the next live session should start from a fresh launch using ${buildShortcutLabel(
        state.primaryShortcutModifiers,
        state.primaryShortcutKey,
      )}.`,
      command: buildVerificationCommand(),
    });
  }

  items.push({
    state:
      recommendations.length === 0
        ? "Now"
        : restartOnlyRecommendations.length > 0 ||
            stagedApplyRecommendations.length > 0 ||
            unstagedApplyRecommendations.length > 0
          ? "After restart"
          : "Next",
      title: buildVerificationTitle(),
      note:
        recommendations.length === 0
          ? buildVerificationNote(
              "Run the live host again and watch whether the next report stays healthy without new tuning prompts.",
            )
        : buildVerificationNote(
            "Use the next mirrored report to confirm whether the failure profile improves and the tuning recommendations shrink.",
          ),
    command: buildVerificationCommand(),
  });

  if (!recommendations.length) {
    tuningRunbookChip.textContent = "Observe";
    tuningRunbookChip.classList.add("is-stable");
  } else if (stagedApplyRecommendations.length > 0) {
    tuningRunbookChip.textContent = "Apply staged";
    tuningRunbookChip.classList.add("is-watch");
  } else if (unstagedApplyRecommendations.length > 0) {
    tuningRunbookChip.textContent = "Stage next";
    tuningRunbookChip.classList.add("is-watch");
  } else if (restartOnlyRecommendations.length > 0) {
    tuningRunbookChip.textContent = "Restart next";
    tuningRunbookChip.classList.add("is-watch");
  } else {
    tuningRunbookChip.textContent = "Verify";
    tuningRunbookChip.classList.add("is-stable");
  }

  items.forEach((item) => {
    const card = document.createElement("article");
    card.className = "runbook-item";

    const meta = document.createElement("div");
    meta.className = "recommendation-meta";

    const state = document.createElement("span");
    state.className = "mix-pill recommendation-state";
    state.textContent = item.state;

    const title = document.createElement("p");
    title.className = "recommendation-title";
    title.textContent = item.title;

    const note = document.createElement("p");
    note.className = "recommendation-copy";
    note.textContent = item.note;

    card.appendChild(meta);
    card.appendChild(title);
    card.appendChild(note);

    if (item.command) {
      const command = document.createElement("pre");
      command.className = "runbook-command";
      command.textContent = item.command;
      card.appendChild(command);
      card.appendChild(buildRunbookCommandActions(item.command));
    }
    meta.appendChild(state);
    tuningRunbookList.appendChild(card);
  });
}

function isRecommendationAlreadyStaged(recommendation) {
  switch (recommendation.setting_key) {
    case "silence_gate_level":
      return String(state.silenceGate) === String(recommendation.target_value);
    case "refinement_quality":
      return normalizeQualityValue(state.quality) === recommendation.target_value;
    case "audio_feedback_enabled":
      return String(state.audioFeedback) === String(recommendation.target_value);
    case "diagnostics_verbosity":
      return state.verbosity === recommendation.target_value;
    default:
      return false;
  }
}

function currentHostApplyCommand() {
  return buildHostApplyCommand(buildSettingsUpdatePayload());
}

async function copyTextToClipboard(text) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  textarea.setSelectionRange(0, text.length);
  const copied = document.execCommand("copy");
  document.body.removeChild(textarea);
  if (!copied) {
    throw new Error("copy command was not supported");
  }
}

function buildRunbookCommandActions(commandText) {
  const actions = document.createElement("div");
  actions.className = "runbook-command-actions";

  const button = document.createElement("button");
  button.type = "button";
  button.className = "secondary-button";
  button.textContent = "Copy step command";

  const status = document.createElement("p");
  status.className = "runbook-command-status";
  status.textContent = "Copy this command if you want to run the step from the terminal.";

  button.addEventListener("click", async () => {
    try {
      await copyTextToClipboard(commandText);
      status.textContent = "Step command copied. You can paste it directly into PowerShell.";
    } catch (_error) {
      status.textContent =
        "Copy failed in this browser context. Select the command text manually and copy it from there.";
    }
  });

  actions.appendChild(button);
  actions.appendChild(status);
  return actions;
}

function buildRecommendationApplyCommand(recommendations) {
  if (!recommendations?.length) {
    return null;
  }

  const payload = {};
  recommendations.forEach((recommendation) => {
    switch (recommendation.setting_key) {
      case "silence_gate_level":
        payload.silence_gate_level = recommendation.target_value;
        break;
      case "refinement_quality":
        payload.refinement_quality = recommendation.target_value;
        break;
      case "audio_feedback_enabled":
        payload.audio_feedback_enabled = recommendation.target_value;
        break;
      case "diagnostics_verbosity":
        payload.diagnostics_verbosity = recommendation.target_value;
        break;
      default:
        break;
    }
  });

  return buildHostApplyCommand(payload);
}

function buildVerificationNote(prefix) {
  if (latestLiveSummary?.verification_note) {
    return `${prefix} ${latestLiveSummary.verification_note}`;
  }

  const gesture =
    latestLiveSummary?.verification_gesture ??
    (state.mode === "PushToTalk"
      ? `Hold ${buildShortcutLabel(state.primaryShortcutModifiers, state.primaryShortcutKey)} while speaking, then release to stop.`
      : `Press ${buildShortcutLabel(state.primaryShortcutModifiers, state.primaryShortcutKey)} once to start, then press it again to stop.`);
  const focusGuidance =
    latestLiveSummary?.verification_focus_guidance ??
    "Use the next check to confirm that the runtime behavior actually improved in your real target app.";
  const verificationScenario = buildVerificationScenario();
  const verificationExample = buildVerificationExample();

  return `${prefix} ${focusGuidance} ${verificationScenario} ${verificationExample} For the next check, ${gesture} If text is selected, the runtime should route into edit mode instead of plain dictation.`;
}

function buildVerificationTitle() {
  if (latestLiveSummary?.verification_title) {
    return latestLiveSummary.verification_title;
  }

  switch (latestLiveSummary?.verification_focus) {
    case "Speech capture":
      return "Verify the next speech-capture check.";
    case "Edit execution":
      return "Verify the next selected-text edit check.";
    case "Caret coverage":
      return "Verify the next caret-insertion check.";
    case "Insertion reliability":
      return "Verify the next real-app insertion check.";
    case "Real-app confidence":
      return "Verify the next real-app confidence check.";
    default:
      return "Verify the next mirrored live-host report.";
  }
}

function buildVerificationScenario() {
  if (latestLiveSummary?.verification_scenario) {
    return latestLiveSummary.verification_scenario;
  }

  switch (latestLiveSummary?.verification_focus) {
    case "Speech capture":
      return "Use a plain dictation take with no text selected, and focus on whether the runtime cleanly captures and recognizes a short sentence before you judge insertion.";
    case "Edit execution":
      return "Highlight a short phrase first, use an explicit edit instruction like uppercase, and confirm that the runtime routes into selected-text edit before it commits.";
    case "Caret coverage":
      return "Use a plain caret dictation case in the app you care about, and check whether the text lands through the cleaner caret path instead of relying on clipboard-style fallback behavior.";
    case "Insertion reliability":
      return "Use the real target app and confirm that the resulting text lands once, in the correct place, without a late commit failure or duplicated insertion.";
    case "Real-app confidence":
      return "Repeat a normal real-app dictation or edit case in the app you actually care about, so the next mirrored report confirms the runtime is still healthy outside the prototype path.";
    default:
      return "Use the next run to confirm that both the speech path and the final insertion behavior improved together.";
  }
}

function buildVerificationExample() {
  if (latestLiveSummary?.verification_example) {
    return latestLiveSummary.verification_example;
  }

  switch (latestLiveSummary?.verification_focus) {
    case "Speech capture":
      return 'Example: say "all right, testing the speech capture path" with no text selected and ignore insertion quality until recognition looks stable.';
    case "Edit execution":
      return 'Example: highlight a short phrase and say "uppercase" before checking whether the selection is replaced correctly.';
    case "Caret coverage":
      return 'Example: place the caret in the target app, say "this should land through the cleaner caret path," and confirm the text lands once in place.';
    case "Insertion reliability":
      return 'Example: say a short sentence like "all right, testing the insertion path again" and confirm it lands once in the target app.';
    case "Real-app confidence":
      switch (latestLiveSummary?.workload_focus) {
        case "Formatting edits":
          return 'Example: highlight a short phrase and say "title case" to confirm the formatting edit still lands cleanly in the real app.';
        case "Structural edits":
          return 'Example: highlight a short multi-line block and say "numbered list" to confirm the structural edit still lands cleanly in the real app.';
        case "Drafting workload":
          return 'Example: say "Hey VoiceFlow, draft an email to finance about the revised budget" and confirm the draft lands cleanly in the real app.';
        case "Synthesis workload":
          return 'Example: say "Hey VoiceFlow, summarize the launch update" and confirm the structured output lands cleanly in the real app.';
        case "Edit-heavy workload":
          return 'Example: highlight a short phrase and say "uppercase" to confirm the edit path still lands cleanly in the real app.';
        case "Intent-heavy workload":
          return 'Example: say "Hey VoiceFlow, draft a short reply to the client" and confirm the generated output lands cleanly in the real app.';
        default:
          return "Example: run one normal dictation or edit case in the real target app and confirm the next mirrored report stays healthy.";
      }
    default:
      return "Example: run one normal live case and confirm the next mirrored report stays healthy.";
  }
}

function buildVerificationCommand() {
  return (
    latestLiveSummary?.verification_command ??
    "cargo run -p input-host -- --serve-live"
  );
}

function stageRecommendation(settingKey, targetValue) {
  switch (settingKey) {
    case "silence_gate_level":
      state.silenceGate = String(targetValue);
      silenceGate.value = state.silenceGate;
      markDirty("silenceGate");
      break;
    case "refinement_quality":
      state.quality = normalizeQualityLabel(targetValue);
      markDirty("quality");
      break;
    case "audio_feedback_enabled":
      state.audioFeedback = String(targetValue) === "true";
      audioToggle.classList.toggle("is-on", state.audioFeedback);
      audioToggle.setAttribute("aria-pressed", String(state.audioFeedback));
      markDirty("audioFeedback");
      break;
    case "diagnostics_verbosity":
      state.verbosity = targetValue;
      markDirty("verbosity");
      break;
    default:
      return;
  }

  renderRuntimeState();
  renderActionState();
}

function formatRecommendationValue(settingKey, value) {
  switch (settingKey) {
    case "silence_gate_level":
      return silenceLabels[value] ?? `Level ${value}`;
    case "refinement_quality":
      return normalizeQualityLabel(value);
    case "audio_feedback_enabled":
      return String(value) === "true" ? "On" : "Off";
    case "diagnostics_verbosity":
      return value === "Verbose" ? "Verbose" : "Standard";
    default:
      return value;
  }
}

function stageAllRecommendations() {
  const stageableRecommendations = latestTuningRecommendations.filter(
    (recommendation) =>
      !isRecommendationAlreadyStaged(recommendation) &&
      !recommendation.saved_already_matches_target,
  );

  if (stageableRecommendations.length === 0) {
    renderActionState();
    return;
  }

  stageableRecommendations.forEach((recommendation) => {
    stageRecommendationFieldOnly(
      recommendation.setting_key,
      recommendation.target_value,
    );
  });

  renderRuntimeState();
  renderActionState();
}

function stageRecommendationFieldOnly(settingKey, targetValue) {
  switch (settingKey) {
    case "silence_gate_level":
      state.silenceGate = String(targetValue);
      silenceGate.value = state.silenceGate;
      markDirty("silenceGate");
      break;
    case "refinement_quality":
      state.quality = normalizeQualityLabel(targetValue);
      markDirty("quality");
      break;
    case "audio_feedback_enabled":
      state.audioFeedback = String(targetValue) === "true";
      audioToggle.classList.toggle("is-on", state.audioFeedback);
      audioToggle.setAttribute("aria-pressed", String(state.audioFeedback));
      markDirty("audioFeedback");
      break;
    case "diagnostics_verbosity":
      state.verbosity = targetValue;
      markDirty("verbosity");
      break;
    default:
      break;
  }
}

function formatApplyTimestamp(epochMs) {
  if (!epochMs) {
    return "No host-side apply mirrored yet.";
  }

  return new Date(epochMs).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function renderLastApply(lastApply) {
  if (!lastApply) {
    lastApplyTime.textContent = "No host-side apply mirrored yet.";
    lastApplySource.textContent =
      "The next successful save will note whether it came from the page or the CLI.";
    lastApplyFields.textContent =
      "The next successful save will list the fields the Rust host persisted.";
    if (savedStageChip && savedStageCopy) {
      savedStageChip.classList.remove("is-stable", "is-watch", "is-alert");
      savedStageChip.textContent = "Waiting";
      savedStageCopy.textContent =
        "The Rust host has not mirrored a save in this page session yet.";
    }
    return;
  }

  lastApplyTime.textContent = `Saved ${formatApplyTimestamp(lastApply.applied_at_epoch_ms)}`;
  lastApplySource.textContent = `Source: ${lastApply.source ?? "Rust host"}`;
  lastApplyFields.textContent =
    lastApply.applied_fields?.length > 0
      ? lastApply.applied_fields.join(" | ")
      : "The host accepted the save, but no changed fields were reported.";
  if (savedStageChip && savedStageCopy) {
    savedStageChip.classList.remove("is-stable", "is-watch", "is-alert");
    savedStageChip.textContent = "Persisted";
    savedStageChip.classList.add("is-stable");
    savedStageCopy.textContent =
      lastApply.applied_fields?.length > 0
        ? `Latest host save touched ${lastApply.applied_fields.join(", ")}.`
        : "The Rust host mirrored a successful save.";
  }
}

function renderLifecycleRuntimeState(runtime) {
  if (!runtimeStageChip || !runtimeStageCopy) {
    return;
  }

  const liveSummary = runtime?.latest_live_host_summary;
  runtimeStageChip.classList.remove("is-stable", "is-watch", "is-alert");

  if (!liveSummary) {
    runtimeStageChip.textContent = "Unavailable";
    runtimeStageChip.classList.add("is-alert");
    runtimeStageCopy.textContent =
      "Launch the live host to compare the running runtime against the saved settings.";
    return;
  }

  if (liveSummary.matches_saved_settings) {
    runtimeStageChip.textContent = "Aligned";
    runtimeStageChip.classList.add("is-stable");
    runtimeStageCopy.textContent =
      "The last mirrored live host already matches the saved shortcut and runtime settings.";
    return;
  }

  runtimeStageChip.textContent = "Last report differs";
  runtimeStageChip.classList.add("is-watch");
  runtimeStageCopy.textContent =
    liveSummary.drift_fields?.length > 0
      ? `The completed report differs on ${liveSummary.drift_fields.join(", ")}; the live host applies saved changes automatically.`
      : "The completed report predates the saved settings; the live host applies saves automatically.";
}

function setPanelStatus(chip, label, tone) {
  if (!chip) {
    return;
  }

  chip.textContent = label;
  chip.classList.remove("is-stable", "is-watch", "is-alert");
  if (tone) {
    chip.classList.add(tone);
  }
}

function renderRuntimeDriftBanner(runtime) {
  if (!runtimeDriftBanner || !runtimeDriftBannerChip || !runtimeDriftBannerCopy || !runtimeDriftBannerFields) {
    return;
  }

  runtimeDriftBannerFields.replaceChildren();
  const liveSummary = runtime?.latest_live_host_summary;
  if (!liveSummary || liveSummary.matches_saved_settings) {
    runtimeDriftBanner.classList.add("is-hidden");
    return;
  }

  runtimeDriftBanner.classList.remove("is-hidden");
  runtimeDriftBannerChip.textContent = t("settings.runtimeReportStale");
  runtimeDriftBannerCopy.textContent = t("settings.runtimeReportStaleHelp");
}

function renderPanelStatuses(runtime) {
  const liveSummary = runtime?.latest_live_host_summary;
  const driftFields = new Set(liveSummary?.drift_fields ?? []);
  if (!liveSummary) {
    setPanelStatus(shortcutPanelStatus, "Not mirrored", "is-alert");
    setPanelStatus(speechPanelStatus, "Not mirrored", "is-alert");
    setPanelStatus(feedbackPanelStatus, "Not mirrored", "is-alert");
    setPanelStatus(diagnosticsPanelStatus, "Not mirrored", "is-alert");
    return;
  }

  const setStatusForPanel = (chip, fields) => {
    const drifted = fields.filter((field) => driftFields.has(field));
    if (drifted.length > 0) {
      setPanelStatus(chip, "Last report differs", "is-watch");
      return;
    }
    setPanelStatus(chip, "Aligned", "is-stable");
  };

  setStatusForPanel(shortcutPanelStatus, panelDriftFields.shortcut);
  setStatusForPanel(speechPanelStatus, panelDriftFields.speech);
  setStatusForPanel(feedbackPanelStatus, panelDriftFields.feedback);
  setStatusForPanel(diagnosticsPanelStatus, panelDriftFields.diagnostics);
}

function renderOverviewSetup(runtime) {
  if (overviewLocalAsr) {
    overviewLocalAsr.textContent = t("overview.local");
  }
  if (overviewSetupShortcut) {
    overviewSetupShortcut.textContent = buildShortcutLabel(
      runtime?.primary_shortcut_modifiers ?? state.primaryShortcutModifiers,
      runtime?.primary_shortcut_key ?? state.primaryShortcutKey,
    );
  }
  if (overviewRecordingMode) {
    overviewRecordingMode.textContent = state.mode === "Toggle"
      ? t("settings.toggle")
      : t("settings.pushToTalk");
  }
  if (overviewRefinement) {
    const keyPresent =
      runtime?.provider?.effective_key_present === true ||
      runtime?.provider?.key_present === true;
    overviewRefinement.textContent = keyPresent
      ? runtime?.provider?.model_code ||
        runtime?.refinement_profile?.model_code ||
        t("common.unavailable")
      : t("overview.localCleanup");
  }
}

function renderRuntimeState() {
  const runtime = window.__VOICEFLOW_SETTINGS_RUNTIME__;
  if (!runtime) {
    setActiveSegment("systemLanguage", state.systemLanguage);
    applyLocalization();
    renderHistoryRetentionControls();
    renderSummary();
    renderLifecycleRuntimeState(null);
    renderRuntimeDriftBanner(null);
    renderPanelStatuses(null);
    renderLiveHostSummary(null);
    renderUsageSummary(null);
    renderOverviewSetup(null);
    renderPromptOverrides(null);
    renderHistorySummary(null);
    renderLastApply(null);
    renderProviderControls(null);
    return;
  }

  const runtimePrimaryShortcutModifiers =
    runtime.primary_shortcut_modifiers ?? persistedState.primaryShortcutModifiers;
  const runtimePrimaryShortcutKey = canonicalizeShortcutKey(
    runtime.primary_shortcut_key ?? persistedState.primaryShortcutKey,
  );
  const runtimeMode = runtime.shortcut_mode ?? persistedState.mode;
  const persistedRuntimeQuality = normalizeQualityLabel(
    runtime.refinement_quality ?? persistedState.quality,
  );
  const runtimeSilenceGate = String(
    runtime.silence_gate_level ?? Number(persistedState.silenceGate),
  );
  const runtimeVerbosity = runtime.diagnostics_verbosity ?? persistedState.verbosity;
  const runtimeAudioFeedback =
    runtime.audio_feedback_enabled ?? persistedState.audioFeedback;
  const runtimeWakePhraseEnabled =
    runtime.wake_phrase_enabled ?? persistedState.wakePhraseEnabled;
  const runtimeWakePhrase = runtime.wake_phrase_text ?? persistedState.wakePhrase;
  const runtimeSystemLanguage = normalizeSystemLanguageValue(
    runtime.system_language ?? persistedState.systemLanguage,
  );
  const runtimeHistoryRetention =
    runtime.history_retention ?? persistedState.historyRetention;
  const runtimeProvider = runtime.provider ?? {};
  const runtimeProviderPreset = normalizeProviderPreset(
    runtimeProvider.saved_preset ?? persistedState.providerPreset,
  );
  const runtimeProviderBaseUrl = normalizeOptionalProviderText(
    runtimeProvider.saved_base_url ?? persistedState.providerBaseUrl,
  );
  const runtimeProviderEndpointOverrideEnabled =
    providerSettingsModel.shouldEnableEndpointOverride(
      runtimeProviderPreset,
      runtimeProviderBaseUrl,
    );
  const runtimeProviderActiveModel = normalizeOptionalProviderText(
    runtimeProvider.saved_active_model ?? persistedState.providerActiveModel,
  );
  const runtimeProviderTimeoutSeconds = providerTimeoutSecondsFromMs(
    runtimeProvider.saved_request_timeout_ms,
  );

  persistedState.systemLanguage = runtimeSystemLanguage;
  persistedState.primaryShortcutModifiers = canonicalizeModifiers(
    runtimePrimaryShortcutModifiers,
  );
  persistedState.primaryShortcutKey = canonicalizeShortcutKey(runtimePrimaryShortcutKey);
  persistedState.mode = runtimeMode;
  persistedState.quality = persistedRuntimeQuality;
  persistedState.silenceGate = runtimeSilenceGate;
  persistedState.verbosity = runtimeVerbosity;
  persistedState.audioFeedback = runtimeAudioFeedback;
  persistedState.wakePhraseEnabled = runtimeWakePhraseEnabled;
  persistedState.wakePhrase = runtimeWakePhrase;
  persistedState.historyRetention = runtimeHistoryRetention;
  persistedState.providerPreset = runtimeProviderPreset;
  persistedState.providerBaseUrl = runtimeProviderBaseUrl;
  persistedState.providerActiveModel = runtimeProviderActiveModel;
  persistedState.providerTimeoutSeconds = runtimeProviderTimeoutSeconds;

  if (!dirtyFields.has("primaryShortcut")) {
    state.primaryShortcutModifiers = canonicalizeModifiers(
      runtimePrimaryShortcutModifiers,
    );
    state.primaryShortcutKey = canonicalizeShortcutKey(runtimePrimaryShortcutKey);
  }
  if (!dirtyFields.has("mode")) {
    state.mode = runtimeMode;
  }
  if (!dirtyFields.has("quality")) {
    state.quality = "Best Quality";
  }
  if (!dirtyFields.has("silenceGate")) {
    state.silenceGate = runtimeSilenceGate;
    silenceGate.value = state.silenceGate;
  }
  if (!dirtyFields.has("verbosity")) {
    state.verbosity = runtimeVerbosity;
  }
  if (!dirtyFields.has("audioFeedback")) {
    state.audioFeedback = runtimeAudioFeedback;
  }
  if (!dirtyFields.has("wakePhraseEnabled")) {
    state.wakePhraseEnabled = runtimeWakePhraseEnabled;
  }
  if (!dirtyFields.has("wakePhrase")) {
    state.wakePhrase = runtimeWakePhrase;
    wakePhrase.value = state.wakePhrase;
  }
  if (!dirtyFields.has("systemLanguage")) {
    state.systemLanguage = runtimeSystemLanguage;
  }
  if (!dirtyFields.has("historyRetention")) {
    state.historyRetention = runtimeHistoryRetention;
  }
  if (!dirtyFields.has("provider")) {
    state.providerPreset = runtimeProviderPreset;
    state.providerBaseUrl = runtimeProviderBaseUrl;
    providerEndpointOverrideEnabled = runtimeProviderEndpointOverrideEnabled;
    state.providerActiveModel = runtimeProviderActiveModel;
    state.providerTimeoutSeconds = runtimeProviderTimeoutSeconds;
  }

  applyLocalization();
  renderHistoryRetentionControls();
  renderProviderControls(runtimeProvider);

  audioToggle.classList.toggle("is-on", state.audioFeedback);
  audioToggle.setAttribute("aria-pressed", String(state.audioFeedback));
  wakePhraseToggle.classList.toggle("is-on", state.wakePhraseEnabled);
  wakePhraseToggle.setAttribute("aria-pressed", String(state.wakePhraseEnabled));
  reportPath.textContent =
    runtime.latest_live_host_report_path ??
    "No live-host report has been written in this session yet.";
  renderReportWarning(runtime.latest_live_host_report_warning);
  settingsPath.textContent = `${runtime.settings_path} (${runtime.settings_source})`;
  renderWarnings(runtime.settings_warnings);
  renderShortcutWarnings(runtime.settings_warnings);
  renderRuntimeShortcutAlignment(runtime);
  renderLifecycleRuntimeState(runtime);
  renderRuntimeDriftBanner(runtime);
  renderPanelStatuses(runtime);
  renderLiveHostSummary(runtime.latest_live_host_summary);
  renderUsageSummary(runtime.usage_summary);
  renderOverviewSetup(runtime);
  renderPromptOverrides(runtime.prompt_overrides);
  renderHistorySummary(runtime.history_summary);
  renderLastApply(runtime.last_settings_apply);

  renderShortcutControls();
  setActiveSegment("systemLanguage", state.systemLanguage);
  setActiveSegment("mode", state.mode);
  setActiveSegment("verbosity", state.verbosity);
  renderSummary();
}

async function fetchRuntimeStateFromHost() {
  const response = await fetch(`${getSettingsControlUrl()}/runtime-state`, {
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error(`host returned ${response.status}`);
  }

  return response.json();
}

async function applySettingsThroughHost() {
  if (saveInFlight) {
    return;
  }

  const payload = buildSettingsUpdatePayload();
  const savedFields = [...dirtyFields];
  if (Object.keys(payload).length === 0) {
    renderSaveCommand();
    renderActionState();
    return;
  }
  if (payload.provider) {
    providerValidationSubmitted = true;
    renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  }
  const providerError = payload.provider ? providerValidationMessage() : null;
  if (providerError) {
    setSaveStatus(providerError, true);
    renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
    renderActionState();
    return;
  }
  if (!bridgeConnected) {
    setSaveStatus("Start VoiceFlow to save settings.", true);
    renderActionState();
    return;
  }
  if (!getSettingsControlToken()) {
    setSaveStatus("Open the live Settings URL to save settings.", true);
    renderActionState();
    return;
  }

  saveInFlight = true;
  setSaveStatus("Saving changes...");
  renderActionState();

  try {
    const response = await fetch(`${getSettingsControlUrl()}/update-settings`, {
      method: "POST",
      headers: buildMutationHeaders(),
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      throw new Error(`host returned ${response.status}`);
    }

    const runtime = await response.json();
    window.__VOICEFLOW_SETTINGS_RUNTIME__ = runtime;
    runtimeTimestamp = runtime.generated_at_epoch_ms ?? runtimeTimestamp;
    if (dirtyFields.has("provider")) {
      providerTestResult = null;
      providerTestStale = false;
    }
    clearDirtyFields();
    lastSaveFeedback = settingsUiModel?.classifyChangedFields(savedFields) ?? {
      nextSession: [],
      nextProviderRequest: [],
      immediate: savedFields,
    };
    bridgeConnected = true;
    runtimeDataSource = "live";
    renderRuntimeState();
    scheduleSaveFeedbackDismissal();
  } catch (error) {
    bridgeConnected = false;
    runtimeDataSource = "static";
    setSaveStatus("Start VoiceFlow to save settings.", true);
  } finally {
    saveInFlight = false;
    renderActionState();
  }
}

async function clearHistoryThroughHost() {
  if (historyClearInFlight) {
    return;
  }
  if (!bridgeConnected) {
    if (clearHistoryStatus) {
      clearHistoryStatus.textContent = t("settings.clearHistoryBridgeRequired");
      clearHistoryStatus.classList.toggle("is-error", true);
    }
    renderActionState();
    return;
  }
  if (!getSettingsControlToken()) {
    if (clearHistoryStatus) {
      clearHistoryStatus.textContent = "Open the live Settings URL to clear history.";
      clearHistoryStatus.classList.toggle("is-error", true);
    }
    renderActionState();
    return;
  }

  const confirmed = window.confirm(
    `${t("settings.clearHistoryConfirmTitle")}\n\n${t(
      "settings.clearHistoryConfirmMessage",
    )}`,
  );
  if (!confirmed) {
    return;
  }

  historyClearInFlight = true;
  if (clearHistoryStatus) {
    clearHistoryStatus.textContent = t("settings.clearHistoryClearing");
    clearHistoryStatus.classList.toggle("is-error", false);
  }
  renderActionState();

  try {
    const response = await fetch(`${getSettingsControlUrl()}/clear-history`, {
      method: "POST",
      headers: buildMutationHeaders(),
    });
    if (!response.ok) {
      throw new Error(`host returned ${response.status}`);
    }

    const runtime = await response.json();
    window.__VOICEFLOW_SETTINGS_RUNTIME__ = runtime;
    runtimeTimestamp = runtime.generated_at_epoch_ms ?? runtimeTimestamp;
    bridgeConnected = true;
    runtimeDataSource = "live";
    renderRuntimeState();
    if (clearHistoryStatus) {
      clearHistoryStatus.textContent = t("settings.clearHistorySuccess");
      clearHistoryStatus.classList.toggle("is-error", false);
    }
  } catch (_error) {
    bridgeConnected = false;
    runtimeDataSource = "static";
    if (clearHistoryStatus) {
      clearHistoryStatus.textContent = t("settings.clearHistoryFailed");
      clearHistoryStatus.classList.toggle("is-error", true);
    }
  } finally {
    historyClearInFlight = false;
    renderActionState();
  }
}

async function saveProviderCredentialThroughHost() {
  if (providerCredentialInFlight) {
    return;
  }
  if (!bridgeConnected || !getSettingsControlToken()) {
    if (providerApiKeyStatus) {
      providerApiKeyStatus.textContent = t("settings.apiKeyBridgeRequired");
      providerApiKeyStatus.classList.toggle("is-error", true);
    }
    renderActionState();
    return;
  }
  const apiKey = String(providerApiKeyInput?.value ?? "").trim();
  if (!apiKey) {
    if (providerApiKeyStatus) {
      providerApiKeyStatus.textContent = t("settings.apiKeyBlank");
      providerApiKeyStatus.classList.toggle("is-error", true);
    }
    return;
  }

  providerCredentialInFlight = true;
  if (providerApiKeyStatus) {
    providerApiKeyStatus.textContent = t("settings.apiKeySaving");
    providerApiKeyStatus.classList.toggle("is-error", false);
  }
  renderActionState();

  try {
    const response = await fetch(`${getSettingsControlUrl()}/provider-credential`, {
      method: "PUT",
      headers: buildMutationHeaders(),
      body: JSON.stringify({
        provider: normalizeProviderPreset(state.providerPreset),
        api_key: apiKey,
      }),
    });
    if (!response.ok) {
      throw new Error(`host returned ${response.status}`);
    }
    const runtime = await response.json();
    window.__VOICEFLOW_SETTINGS_RUNTIME__ = runtime;
    runtimeTimestamp = runtime.generated_at_epoch_ms ?? runtimeTimestamp;
    bridgeConnected = true;
    runtimeDataSource = "live";
    if (providerApiKeyInput) {
      providerApiKeyInput.value = "";
    }
    renderRuntimeState();
    if (providerApiKeyStatus) {
      providerApiKeyStatus.textContent = tf("settings.apiKeySaved", {
        provider: providerLabel(normalizeProviderPreset(state.providerPreset)),
      });
      providerApiKeyStatus.classList.toggle("is-error", false);
    }
  } catch (_error) {
    if (providerApiKeyStatus) {
      providerApiKeyStatus.textContent = t("settings.apiKeySaveFailed");
      providerApiKeyStatus.classList.toggle("is-error", true);
    }
  } finally {
    providerCredentialInFlight = false;
    renderActionState();
  }
}

async function clearProviderCredentialThroughHost() {
  if (providerCredentialInFlight) {
    return;
  }
  if (!bridgeConnected || !getSettingsControlToken()) {
    if (providerApiKeyStatus) {
      providerApiKeyStatus.textContent = t("settings.apiKeyBridgeRequired");
      providerApiKeyStatus.classList.toggle("is-error", true);
    }
    renderActionState();
    return;
  }

  providerCredentialInFlight = true;
  if (providerApiKeyStatus) {
    providerApiKeyStatus.textContent = t("settings.apiKeyClearing");
    providerApiKeyStatus.classList.toggle("is-error", false);
  }
  renderActionState();

  try {
    const response = await fetch(`${getSettingsControlUrl()}/provider-credential`, {
      method: "DELETE",
      headers: buildMutationHeaders(),
      body: JSON.stringify({
        provider: normalizeProviderPreset(state.providerPreset),
      }),
    });
    if (!response.ok) {
      throw new Error(`host returned ${response.status}`);
    }
    const runtime = await response.json();
    window.__VOICEFLOW_SETTINGS_RUNTIME__ = runtime;
    runtimeTimestamp = runtime.generated_at_epoch_ms ?? runtimeTimestamp;
    bridgeConnected = true;
    runtimeDataSource = "live";
    if (providerApiKeyInput) {
      providerApiKeyInput.value = "";
    }
    renderRuntimeState();
    if (providerApiKeyStatus) {
      const providerName = providerLabel(normalizeProviderPreset(state.providerPreset));
      const selectedCredential = credentialStatusForSelectedProvider(runtime?.provider);
      providerApiKeyStatus.textContent =
        runtime?.provider?.effective_key_present === true &&
        selectedCredential.stored_credential_present === false
          ? tf("settings.apiKeyClearedEnvActive", { provider: providerName })
          : tf("settings.apiKeyCleared", { provider: providerName });
      providerApiKeyStatus.classList.toggle("is-error", false);
    }
  } catch (_error) {
    if (providerApiKeyStatus) {
      providerApiKeyStatus.textContent = t("settings.apiKeyClearFailed");
      providerApiKeyStatus.classList.toggle("is-error", true);
    }
  } finally {
    providerCredentialInFlight = false;
    renderActionState();
  }
}

async function testProviderConnectionThroughHost() {
  if (providerTestInFlight) {
    return;
  }
  providerValidationSubmitted = true;
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  if (providerValidationMessage()) {
    renderActionState();
    return;
  }
  if (dirtyFields.has("provider")) {
    providerTestStale = true;
    renderProviderTestStatus(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
    renderActionState();
    return;
  }
  if (!bridgeConnected || !getSettingsControlToken()) {
    providerTestResult = null;
    providerTestStale = false;
    if (providerTestStatus) {
      providerTestStatus.textContent = t("settings.testConnectionBridgeRequired");
      providerTestStatus.classList.toggle("is-error", true);
    }
    renderActionState();
    return;
  }

  providerTestInFlight = true;
  providerTestResult = null;
  providerTestStale = false;
  renderProviderTestStatus(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  renderActionState();

  try {
    const response = await fetch(`${getSettingsControlUrl()}/test-provider`, {
      method: "POST",
      headers: buildMutationHeaders(),
    });
    if (!response.ok) {
      throw new Error(`host returned ${response.status}`);
    }
    providerTestResult = await response.json();
    bridgeConnected = true;
    runtimeDataSource = "live";
  } catch (_error) {
    providerTestResult = {
      success: false,
      status: "network_error",
      latency_ms: null,
      message: "Connection test failed.",
    };
  } finally {
    providerTestInFlight = false;
    renderProviderTestStatus(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
    renderActionState();
  }
}

async function reloadPromptOverrideStatus() {
  if (promptOverrideReloadInFlight) {
    return;
  }
  if (!bridgeConnected) {
    if (promptOverridesStatus) {
      promptOverridesStatus.textContent = t("settings.promptBridgeRequired");
      promptOverridesStatus.classList.toggle("is-error", true);
    }
    renderActionState();
    return;
  }

  promptOverrideReloadInFlight = true;
  if (promptOverridesStatus) {
    promptOverridesStatus.textContent = t("settings.promptReloading");
    promptOverridesStatus.classList.toggle("is-error", false);
  }
  renderActionState();

  try {
    const runtime = await fetchRuntimeStateFromHost();
    window.__VOICEFLOW_SETTINGS_RUNTIME__ = runtime;
    runtimeTimestamp = runtime.generated_at_epoch_ms ?? runtimeTimestamp;
    bridgeConnected = true;
    runtimeDataSource = "live";
    renderRuntimeState();
    if (promptOverridesStatus) {
      promptOverridesStatus.textContent = t("settings.promptReloaded");
      promptOverridesStatus.classList.toggle("is-error", false);
    }
  } catch (_error) {
    bridgeConnected = false;
    runtimeDataSource = "static";
    if (promptOverridesStatus) {
      promptOverridesStatus.textContent = t("settings.promptReloadFailed");
      promptOverridesStatus.classList.toggle("is-error", true);
    }
  } finally {
    promptOverrideReloadInFlight = false;
    renderActionState();
  }
}

function reloadRuntimeScript() {
  const nextScript = document.createElement("script");
  nextScript.src = `./src/runtime-state.js?ts=${Date.now()}`;
  nextScript.async = true;
  nextScript.onload = () => {
    const runtime = window.__VOICEFLOW_SETTINGS_RUNTIME__;
    if (!runtime) {
      return;
    }

    if (runtime.generated_at_epoch_ms !== runtimeTimestamp) {
      if (!bridgeConnected) {
        runtimeDataSource = "static";
      }
      runtimeTimestamp = runtime.generated_at_epoch_ms;
      renderRuntimeState();
    }
  };
  nextScript.onerror = () => {
    nextScript.remove();
  };
  document.body.appendChild(nextScript);
  if (document.body.children.length > 18) {
    document.body.removeChild(document.body.children[document.body.children.length - 2]);
  }
}

segmentGroups.forEach((group) => {
  const segmentButtons = Array.from(group.querySelectorAll(".segment"));
  const selectButton = (button) => {
    segmentButtons.forEach((item) => {
      const isActive = item === button;
      item.classList.toggle("is-active", isActive);
      if (item.getAttribute("role") === "radio") {
        item.setAttribute("aria-checked", String(isActive));
        item.tabIndex = isActive ? 0 : -1;
      }
    });

    const groupName = group.dataset.segment;
    state[groupName] =
      groupName === "systemLanguage"
        ? normalizeSystemLanguageValue(button.dataset.value)
        : button.dataset.value;
    markDirty(groupName);
    if (groupName === "systemLanguage") {
      applyLocalization();
      renderUsageSummary(window.__VOICEFLOW_SETTINGS_RUNTIME__?.usage_summary);
      renderOverviewSetup(window.__VOICEFLOW_SETTINGS_RUNTIME__);
      renderPromptOverrides(window.__VOICEFLOW_SETTINGS_RUNTIME__?.prompt_overrides);
      renderHistorySummary(window.__VOICEFLOW_SETTINGS_RUNTIME__?.history_summary);
      renderLiveHostSummary(window.__VOICEFLOW_SETTINGS_RUNTIME__?.latest_live_host_summary);
      renderLastApply(window.__VOICEFLOW_SETTINGS_RUNTIME__?.last_settings_apply);
      renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
    }
    renderSummary();
  };

  group.addEventListener("click", (event) => {
    const button = event.target.closest(".segment");
    if (!button || !group.contains(button)) {
      return;
    }
    selectButton(button);
  });

  group.addEventListener("keydown", (event) => {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) {
      return;
    }
    const current = event.target.closest(".segment");
    if (!current || !group.contains(current)) {
      return;
    }
    event.preventDefault();
    const currentIndex = Math.max(0, segmentButtons.indexOf(current));
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? segmentButtons.length - 1
          : event.key === "ArrowLeft" || event.key === "ArrowUp"
            ? (currentIndex - 1 + segmentButtons.length) % segmentButtons.length
            : (currentIndex + 1) % segmentButtons.length;
    const nextButton = segmentButtons[nextIndex];
    nextButton.focus();
    selectButton(nextButton);
  });
});

shortcutModifierButtons.forEach((button) => {
  button.addEventListener("click", () => {
    const modifier = button.dataset.modifier;
    if (!modifier) {
      return;
    }

    if (state.primaryShortcutModifiers.includes(modifier)) {
      if (state.primaryShortcutModifiers.length === 1) {
        return;
      }
      state.primaryShortcutModifiers = state.primaryShortcutModifiers.filter(
        (value) => value !== modifier,
      );
    } else {
      state.primaryShortcutModifiers = canonicalizeModifiers([
        ...state.primaryShortcutModifiers,
        modifier,
      ]);
    }

    markDirty("primaryShortcut");
    renderShortcutControls();
    renderSummary();
  });
});

primaryShortcutKey.addEventListener("change", () => {
  state.primaryShortcutKey = canonicalizeShortcutKey(primaryShortcutKey.value);
  markDirty("primaryShortcut");
  renderSummary();
});

audioToggle.addEventListener("click", () => {
  state.audioFeedback = !state.audioFeedback;
  audioToggle.classList.toggle("is-on", state.audioFeedback);
  audioToggle.setAttribute("aria-pressed", String(state.audioFeedback));
  markDirty("audioFeedback");
  renderSummary();
});

wakePhraseToggle.addEventListener("click", () => {
  state.wakePhraseEnabled = !state.wakePhraseEnabled;
  wakePhraseToggle.classList.toggle("is-on", state.wakePhraseEnabled);
  wakePhraseToggle.setAttribute("aria-pressed", String(state.wakePhraseEnabled));
  markDirty("wakePhraseEnabled");
  renderSummary();
});

applyButton.addEventListener("click", () => {
  void applySettingsThroughHost();
});

revertButton.addEventListener("click", () => {
  clearSaveFeedback();
  clearDirtyFields();
  restoreStateFromPersisted();
  renderRuntimeState();
  renderActionState();
});

copySaveCommandButton?.addEventListener("click", async () => {
  const command = currentHostApplyCommand();
  if (!command) {
    if (commandCopyStatus) {
      commandCopyStatus.textContent = t("settings.noFallback");
    }
    return;
  }

  try {
    await copyTextToClipboard(command);
    if (commandCopyStatus) {
      commandCopyStatus.textContent = t("settings.fallbackCopied");
    }
  } catch (_error) {
    if (commandCopyStatus) {
      commandCopyStatus.textContent = t("settings.fallbackCopyFailed");
    }
  }
});

silenceGate.addEventListener("input", () => {
  state.silenceGate = silenceGate.value;
  markDirty("silenceGate");
  renderSummary();
});

clearHistoryButton?.addEventListener("click", () => {
  void clearHistoryThroughHost();
});

reloadPromptsButton?.addEventListener("click", () => {
  void reloadPromptOverrideStatus();
});

saveProviderApiKeyButton?.addEventListener("click", () => {
  void saveProviderCredentialThroughHost();
});

clearProviderApiKeyButton?.addEventListener("click", () => {
  void clearProviderCredentialThroughHost();
});

testProviderConnectionButton?.addEventListener("click", () => {
  void testProviderConnectionThroughHost();
});

wakePhrase.addEventListener("input", () => {
  state.wakePhrase = wakePhrase.value;
  markDirty("wakePhrase");
});

historyRetentionOptions?.addEventListener("click", (event) => {
  const button = event.target.closest("[data-retention-value]");
  if (!button) {
    return;
  }
  if (button.dataset.retentionValue === state.historyRetention) {
    return;
  }
  state.historyRetention = button.dataset.retentionValue;
  markDirty("historyRetention");
  renderHistoryRetentionControls();
  renderHistorySummary(window.__VOICEFLOW_SETTINGS_RUNTIME__?.history_summary);
  renderSummary();
});

providerPreset?.addEventListener("change", () => {
  const nextPreset = normalizeProviderPreset(providerPreset.value);
  if (nextPreset !== state.providerPreset) {
    state.providerBaseUrl = "";
    providerEndpointOverrideEnabled = false;
    resetProviderValidationState();
  }
  state.providerPreset = nextPreset;
  markDirty("provider");
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  renderSummary();
});

providerBaseUrl?.addEventListener("input", () => {
  state.providerBaseUrl = providerBaseUrl.value;
  markDirty("provider");
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  renderSummary();
});

providerBaseUrl?.addEventListener("blur", () => {
  providerValidationTouched.baseUrl = true;
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
});

providerActiveModel?.addEventListener("input", () => {
  state.providerActiveModel = providerActiveModel.value;
  markDirty("provider");
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  renderSummary();
});

providerTimeoutSeconds?.addEventListener("input", () => {
  state.providerTimeoutSeconds = providerTimeoutSeconds.value;
  markDirty("provider");
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  renderSummary();
});

providerTimeoutSeconds?.addEventListener("blur", () => {
  providerValidationTouched.timeout = true;
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
});

supportedShortcutKeys.forEach((key) => {
  const option = document.createElement("option");
  option.value = key;
  option.textContent = key;
  primaryShortcutKey.appendChild(option);
});

renderRuntimeState();
tuningRecommendations?.addEventListener("click", (event) => {
  const button = event.target.closest(".recommendation-action");
  if (!button || button.disabled) {
    return;
  }

  stageRecommendation(button.dataset.settingKey, button.dataset.targetValue);
});

stageAllRecommendationsButton?.addEventListener("click", () => {
  stageAllRecommendations();
});

providerEndpointOverride?.addEventListener("change", () => {
  const next = providerSettingsModel.setEndpointOverride(
    state.providerPreset,
    state.providerBaseUrl,
    providerEndpointOverride.checked,
  );
  providerEndpointOverrideEnabled = next.enabled;
  state.providerBaseUrl = next.baseUrl;
  providerValidationSubmitted = false;
  providerValidationTouched.baseUrl = false;
  markDirty("provider");
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  if (providerEndpointOverrideEnabled) {
    providerOverrideBaseUrl.focus();
  }
  renderSummary();
});

providerOverrideBaseUrl?.addEventListener("input", () => {
  state.providerBaseUrl = providerOverrideBaseUrl.value;
  markDirty("provider");
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  renderSummary();
});

providerOverrideBaseUrl?.addEventListener("blur", () => {
  providerValidationTouched.baseUrl = true;
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
});

providerEndpointReset?.addEventListener("click", () => {
  providerEndpointOverrideEnabled = false;
  state.providerBaseUrl = "";
  providerValidationSubmitted = false;
  providerValidationTouched.baseUrl = false;
  markDirty("provider");
  renderProviderControls(window.__VOICEFLOW_SETTINGS_RUNTIME__?.provider);
  renderSummary();
});

overviewPrivacyToggle?.addEventListener("click", () => {
  overviewPreviewsHidden = !overviewPreviewsHidden;
  persistOverviewPrivacyPreference();
  renderOverviewRecentEntries();
});

const companionSections = Array.from(document.querySelectorAll(".companion-section"));
const sectionLinks = Array.from(document.querySelectorAll("[data-section-link]"));
const sectionJumps = Array.from(document.querySelectorAll("[data-section-jump]"));

function showSettingsSection(sectionId) {
  const normalizedSection = settingsUiModel?.normalizeSectionId(sectionId) ?? "general";
  activeSettingsSection = normalizedSection === "prompts" ? "advanced" : normalizedSection;
  settingsSectionButtons.forEach((button) => {
    const isActive = button.dataset.settingsSection === activeSettingsSection;
    button.classList.toggle("is-active", isActive);
    if (isActive) {
      button.setAttribute("aria-current", "page");
    } else {
      button.removeAttribute("aria-current");
    }
  });
  settingsPanes.forEach((pane) => {
    const isActive = pane.dataset.settingsPane === activeSettingsSection;
    pane.classList.toggle("is-active", isActive);
    pane.hidden = !isActive;
  });
}

settingsSectionButtons.forEach((button) => {
  button.addEventListener("click", () => {
    showSettingsSection(button.dataset.settingsSection);
  });
});

function currentCompanionSectionId() {
  return companionSections.find((section) => section.classList.contains("is-active-section"))?.id;
}

function discardSettingsDraft() {
  clearSaveFeedback();
  clearDirtyFields();
  restoreStateFromPersisted();
  renderRuntimeState();
  renderActionState();
}

function confirmSettingsExit(nextSectionId) {
  const shouldConfirm = settingsUiModel?.shouldConfirmSettingsExit(
    dirtyFields.size > 0,
    currentCompanionSectionId(),
    nextSectionId,
  );
  if (!shouldConfirm) {
    return true;
  }
  if (!window.confirm(t("settings.discardConfirm"))) {
    return false;
  }
  discardSettingsDraft();
  return true;
}

function showCompanionSection(sectionId, focusTargetId) {
  const normalizedSectionId = sectionId === "diagnostics" ? "sessions" : sectionId;
  const nextSection = companionSections.find((section) => section.id === normalizedSectionId);

  if (!nextSection) {
    return;
  }

  companionSections.forEach((section) => {
    section.classList.toggle("is-active-section", section === nextSection);
  });

  sectionLinks.forEach((link) => {
    const isMatchingSection = link.dataset.sectionLink === normalizedSectionId;
    const isMatchingFocus = link.dataset.focusTarget === focusTargetId;
    link.classList.toggle(
      "is-active",
      link.dataset.focusTarget ? isMatchingFocus : isMatchingSection,
    );
  });

  if (focusTargetId) {
    document.getElementById(focusTargetId)?.scrollIntoView({ block: "start" });
  }
}

sectionLinks.forEach((link) => {
  link.addEventListener("click", (event) => {
    event.preventDefault();
    const sectionId = link.dataset.sectionLink;
    const focusTargetId = link.dataset.focusTarget;
    if (!confirmSettingsExit(sectionId)) {
      return;
    }
    showCompanionSection(sectionId, focusTargetId);
    window.history.replaceState(null, "", link.getAttribute("href"));
  });
});

sectionJumps.forEach((link) => {
  link.addEventListener("click", (event) => {
    event.preventDefault();
    const sectionId = link.dataset.sectionJump;
    if (!confirmSettingsExit(sectionId)) {
      return;
    }
    showCompanionSection(sectionId);
    window.history.replaceState(null, "", link.getAttribute("href"));
  });
});

showSettingsSection(activeSettingsSection);

window.addEventListener("beforeunload", (event) => {
  if (dirtyFields.size === 0) {
    return;
  }
  event.preventDefault();
  event.returnValue = "";
});

const initialHash = window.location.hash.replace("#", "");
if (initialHash) {
  const matchingLink = sectionLinks.find(
    (link) =>
      link.getAttribute("href") === `#${initialHash}` ||
      link.dataset.focusTarget === initialHash,
  );
  showCompanionSection(
    matchingLink?.dataset.sectionLink ?? initialHash,
    matchingLink?.dataset.focusTarget,
  );
}

window.setInterval(async () => {
  try {
    const runtime = await fetchRuntimeStateFromHost();
    const shouldRenderLiveState =
      runtime.generated_at_epoch_ms !== runtimeTimestamp || runtimeDataSource !== "live";
    bridgeConnected = true;
    runtimeDataSource = "live";
    if (shouldRenderLiveState) {
      window.__VOICEFLOW_SETTINGS_RUNTIME__ = runtime;
      runtimeTimestamp = runtime.generated_at_epoch_ms;
      renderRuntimeState();
    } else {
      renderActionState();
    }
  } catch (_error) {
    bridgeConnected = false;
    runtimeDataSource = "static";
    renderActionState();
    // Fall back to the generated runtime-state.js mirror when the host bridge is not running.
    reloadRuntimeScript();
  }
}, 1100);
