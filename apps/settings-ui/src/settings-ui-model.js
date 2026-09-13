(function exposeSettingsUiModel(root, factory) {
  const model = factory();
  if (typeof module !== "undefined" && module.exports) {
    module.exports = model;
  }
  if (root) {
    root.VoiceFlowSettingsModel = model;
  }
})(typeof globalThis !== "undefined" ? globalThis : this, function createSettingsUiModel() {
  const sections = Object.freeze([
    Object.freeze({ id: "general", English: "General", Chinese: "通用" }),
    Object.freeze({ id: "voice-output", English: "Voice & Output", Chinese: "语音与输出" }),
    Object.freeze({ id: "history-privacy", English: "History & Privacy", Chinese: "历史与隐私" }),
    Object.freeze({ id: "ai-provider", English: "AI Provider", Chinese: "AI 服务" }),
    Object.freeze({ id: "advanced", English: "Advanced", Chinese: "高级设置" }),
  ]);

  const immediateFields = new Set([
    "systemLanguage",
    "primaryShortcut",
    "mode",
    "verbosity",
    "audioFeedback",
    "historyRetention",
  ]);

  const nextSessionFields = new Set([
    "silenceGate",
    "wakePhraseEnabled",
    "wakePhrase",
    "quality",
  ]);
  const nextProviderRequestFields = new Set(["provider"]);

  function normalizeLanguage(language) {
    return language === "Chinese" || String(language).toLowerCase().startsWith("zh")
      ? "Chinese"
      : "English";
  }

  function sectionLabel(sectionId, language) {
    const section = sections.find((candidate) => candidate.id === sectionId);
    return section?.[normalizeLanguage(language)] ?? sectionId;
  }

  function normalizeSectionId(sectionId) {
    if (sectionId === "prompts") return "advanced";
    return sections.some((section) => section.id === sectionId) ? sectionId : "general";
  }

  function classifyChangedFields(fields) {
    const uniqueFields = [...new Set(fields)];
    return {
      nextSession: uniqueFields.filter((field) => nextSessionFields.has(field)),
      nextProviderRequest: uniqueFields.filter((field) =>
        nextProviderRequestFields.has(field),
      ),
      immediate: uniqueFields.filter(
        (field) =>
          immediateFields.has(field) ||
          (!nextSessionFields.has(field) && !nextProviderRequestFields.has(field)),
      ),
    };
  }

  function shouldConfirmSettingsExit(hasDirtyChanges, currentSection, nextSection) {
    return Boolean(hasDirtyChanges && currentSection === "settings" && nextSection !== "settings");
  }

  return Object.freeze({
    sections,
    normalizeLanguage,
    sectionLabel,
    normalizeSectionId,
    classifyChangedFields,
    shouldConfirmSettingsExit,
  });
});
