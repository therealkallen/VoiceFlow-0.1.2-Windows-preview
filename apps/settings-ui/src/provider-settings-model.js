(function exposeProviderSettingsModel(root, factory) {
  const model = factory();
  if (typeof module !== "undefined" && module.exports) {
    module.exports = model;
  }
  if (root) {
    root.VoiceFlowProviderSettingsModel = model;
  }
})(typeof globalThis !== "undefined" ? globalThis : this, function createProviderSettingsModel() {
  const presets = Object.freeze({
    bailian: Object.freeze({
      baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      modelPlaceholder: "qwen3.7-plus",
      modelHelpKey: "settings.activeModelHelpBailian",
      noteKey: "settings.providerNoteBailian",
    }),
    volcengine_ark: Object.freeze({
      baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
      modelPlaceholder: "Model or endpoint ID",
      modelHelpKey: "settings.activeModelHelpVolcengine",
      noteKey: "settings.providerNoteVolcengine",
    }),
    tencent_hunyuan: Object.freeze({
      baseUrl: "https://tokenhub.tencentmaas.com/v1",
      modelPlaceholder: "hy3-preview",
      modelHelpKey: "settings.activeModelHelpTencent",
      noteKey: "settings.providerNoteTencent",
    }),
    custom_openai_compatible: Object.freeze({
      baseUrl: null,
      modelPlaceholder: "provider-model-id",
      modelHelpKey: "settings.activeModelHelpCustom",
      noteKey: "settings.providerNoteCustom",
    }),
  });

  function normalizePreset(value) {
    const normalized = String(value ?? "").trim().toLowerCase().replaceAll("-", "_");
    return Object.hasOwn(presets, normalized) ? normalized : "bailian";
  }

  function presetDefinition(value) {
    return presets[normalizePreset(value)];
  }

  function normalizeEndpoint(value) {
    return String(value ?? "").trim().replace(/\/+$/, "");
  }

  function isPresetProvider(value) {
    return presetDefinition(value).baseUrl !== null;
  }

  function isPresetDefaultUrl(preset, baseUrl) {
    const defaultUrl = presetDefinition(preset).baseUrl;
    return (
      defaultUrl !== null &&
      normalizeEndpoint(baseUrl).toLowerCase() === normalizeEndpoint(defaultUrl).toLowerCase()
    );
  }

  function shouldEnableEndpointOverride(preset, savedBaseUrl) {
    return (
      isPresetProvider(preset) &&
      normalizeEndpoint(savedBaseUrl).length > 0 &&
      !isPresetDefaultUrl(preset, savedBaseUrl)
    );
  }

  function setEndpointOverride(preset, baseUrl, enabled) {
    const nextEnabled = isPresetProvider(preset) && Boolean(enabled);
    const nextBaseUrl =
      !nextEnabled || isPresetDefaultUrl(preset, baseUrl) ? "" : String(baseUrl ?? "");
    return Object.freeze({ enabled: nextEnabled, baseUrl: nextBaseUrl });
  }

  function presentation(preset, savedBaseUrl, overrideEnabled) {
    const normalizedPreset = normalizePreset(preset);
    const definition = presetDefinition(normalizedPreset);
    const usesPreset = definition.baseUrl !== null;
    return Object.freeze({
      preset: normalizedPreset,
      presetBaseUrl: definition.baseUrl,
      showCustomBaseUrl: !usesPreset,
      showPresetEndpoint: usesPreset,
      showOverrideBaseUrl: usesPreset && Boolean(overrideEnabled),
      modelPlaceholder: definition.modelPlaceholder,
      modelHelpKey: definition.modelHelpKey,
      noteKey: definition.noteKey,
      savedUrlMatchesPreset: isPresetDefaultUrl(normalizedPreset, savedBaseUrl),
    });
  }

  return Object.freeze({
    presets,
    normalizePreset,
    normalizeEndpoint,
    isPresetProvider,
    isPresetDefaultUrl,
    shouldEnableEndpointOverride,
    setEndpointOverride,
    presentation,
  });
});
