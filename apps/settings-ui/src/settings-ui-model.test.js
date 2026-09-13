const assert = require("node:assert/strict");
const test = require("node:test");

const model = require("./settings-ui-model.js");

test("exposes the five localized Settings sections in product order", () => {
  assert.deepEqual(
    model.sections.map((section) => section.id),
    ["general", "voice-output", "history-privacy", "ai-provider", "advanced"],
  );
  assert.equal(model.sectionLabel("voice-output", "English"), "Voice & Output");
  assert.equal(model.sectionLabel("voice-output", "zh-CN"), "语音与输出");
  assert.equal(model.sectionLabel("history-privacy", "Chinese"), "历史与隐私");
});

test("falls back to General and English for unknown navigation values", () => {
  assert.equal(model.normalizeSectionId("missing"), "general");
  assert.equal(model.normalizeSectionId("prompts"), "advanced");
  assert.equal(model.sectionLabel("advanced", "fr-FR"), "Advanced");
});

test("keeps dirty values independent from Settings subsection navigation", () => {
  assert.equal(model.shouldConfirmSettingsExit(true, "settings", "settings"), false);
  assert.equal(model.shouldConfirmSettingsExit(true, "settings", "history"), true);
  assert.equal(model.shouldConfirmSettingsExit(false, "settings", "history"), false);
});

test("classifies live, next-dictation, and next-provider-request fields accurately", () => {
  assert.deepEqual(model.classifyChangedFields(["provider"]), {
    nextSession: [],
    nextProviderRequest: ["provider"],
    immediate: [],
  });
  assert.deepEqual(model.classifyChangedFields(["systemLanguage", "provider"]), {
    nextSession: [],
    nextProviderRequest: ["provider"],
    immediate: ["systemLanguage"],
  });
  assert.deepEqual(model.classifyChangedFields(["silenceGate", "wakePhrase"]), {
    nextSession: ["silenceGate", "wakePhrase"],
    nextProviderRequest: [],
    immediate: [],
  });
});

test("deduplicates fields before producing mixed save feedback", () => {
  assert.deepEqual(
    model.classifyChangedFields(["provider", "provider", "audioFeedback"]),
    {
      nextSession: [],
      nextProviderRequest: ["provider"],
      immediate: ["audioFeedback"],
    },
  );
});
