const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

function loadOverlay() {
  const shell = {
    dataset: { state: "hidden" },
    setAttribute() {},
  };
  const widget = {
    attributes: {},
    setAttribute(name, value) {
      this.attributes[name] = value;
    },
  };
  const stateLabel = { textContent: "" };
  const statusText = { hidden: true, textContent: "" };
  const context = vm.createContext({
    document: {
      body: { appendChild() {} },
      createElement() {
        return { remove() {} };
      },
      documentElement: { lang: "en" },
      getElementById(id) {
        return id === "voice-state-label" ? stateLabel : statusText;
      },
      querySelector(selector) {
        return selector === ".overlay-shell" ? shell : widget;
      },
    },
    window: {
      clearTimeout() {},
      setInterval() {},
      setTimeout() {},
    },
  });
  const source = fs.readFileSync(path.join(__dirname, "main.js"), "utf8");
  vm.runInContext(source, context);
  return { context, shell, stateLabel, widget };
}

function recordingRuntime(locale, sessionId = 1) {
  return {
    generated_at_epoch_ms: Date.now(),
    locale,
    latest_status: {
      session_id: sessionId,
      session_kind: "Dictation",
      state: "Recording",
      detail: "",
    },
  };
}

test("normalizes supported locale variants and defaults to English", () => {
  const { context } = loadOverlay();

  for (const locale of ["zh", "zh-CN", "zh-TW", "Chinese", "zh_CN"]) {
    assert.equal(vm.runInContext(`normalizeLocale(${JSON.stringify(locale)})`, context), "zh");
  }
  for (const locale of ["en", "en-US", "en-GB", "English", "en_US"]) {
    assert.equal(vm.runInContext(`normalizeLocale(${JSON.stringify(locale)})`, context), "en");
  }
  assert.equal(vm.runInContext("normalizeLocale(undefined)", context), "en");
  assert.equal(vm.runInContext("normalizeLocale('unknown')", context), "en");
});

test("uses the locale from each new state payload", () => {
  const { context, stateLabel, widget } = loadOverlay();

  context.window.__VOICEFLOW_OVERLAY_SET_RUNTIME__(recordingRuntime("en-US", 1));
  assert.equal(stateLabel.textContent, "Listening");
  assert.equal(widget.attributes["aria-label"], "Voice input status");

  context.window.__VOICEFLOW_OVERLAY_SET_RUNTIME__(recordingRuntime("zh-TW", 2));
  assert.equal(stateLabel.textContent, "正在说话");
  assert.equal(widget.attributes["aria-label"], "语音输入状态");
});

test("localizes processing, success, silence, and generic errors", () => {
  const { context } = loadOverlay();
  const classify = (runtime) => {
    context.runtimeUnderTest = runtime;
    return vm.runInContext("classifyRuntime(runtimeUnderTest)", context);
  };

  assert.equal(
    classify({
      locale: "en",
      latest_status: { session_id: 1, state: "Recognizing" },
    }).label,
    "Thinking",
  );
  assert.equal(
    classify({
      locale: "zh",
      latest_status: { session_id: 2, state: "Committed" },
    }).label,
    "已完成",
  );
  assert.equal(
    classify({
      locale: "en",
      latest_status: { session_id: 3, state: "Failed" },
      last_failure_summary: {
        session_id: 3,
        failure_phase: "Recognizing",
        message: "no speech was recognized",
      },
    }).label,
    "No speech detected",
  );
  assert.equal(
    classify({
      locale: "zh",
      latest_status: { session_id: 4, state: "Failed" },
      last_failure_summary: {
        session_id: 4,
        failure_phase: "Executing",
        message: "private backend details",
      },
    }).label,
    "处理失败",
  );
});
