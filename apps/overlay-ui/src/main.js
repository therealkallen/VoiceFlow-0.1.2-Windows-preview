let overlayRuntimeTimestamp = 0;
let activeSessionId = null;
let hideTimer = null;

const RUNTIME_STALE_AFTER_MS = 15 * 60 * 1000;
const SUCCESS_VISIBLE_MS = 460;
const ERROR_VISIBLE_MS = 1200;

const shell = document.querySelector(".overlay-shell");
const widget = document.querySelector(".voice-widget");
const stateLabel = document.getElementById("voice-state-label");
const statusText = document.getElementById("voice-status-text");

const PROCESSING_STATES = new Set([
  "Recognizing",
  "ModeRouting",
  "Executing",
  "ReadyToCommit",
  "Committing",
]);

const DEFAULT_LOCALE = "en";

const COPY = {
  zh: {
    ariaLabel: "语音输入状态",
    listening: "正在说话",
    processing: "思考中",
    success: "已完成",
    silence: "未识别到语音",
    error: "处理失败",
    apiKey: "未配置 API key",
  },
  en: {
    ariaLabel: "Voice input status",
    listening: "Listening",
    processing: "Thinking",
    success: "Done",
    silence: "No speech detected",
    error: "Something went wrong",
    apiKey: "API key not configured",
  },
};

function normalizeLocale(value) {
  const locale = String(value ?? "")
    .trim()
    .toLowerCase()
    .replaceAll("_", "-");

  if (locale === "chinese" || locale === "zh" || locale.startsWith("zh-")) {
    return "zh";
  }
  if (locale === "english" || locale === "en" || locale.startsWith("en-")) {
    return "en";
  }

  return DEFAULT_LOCALE;
}

function copyForRuntime(runtime) {
  const locale = normalizeLocale(
    runtime?.locale ??
      runtime?.system_language ??
      runtime?.effective_system_language ??
      runtime?.language,
  );
  document.documentElement.lang = locale === "zh" ? "zh-CN" : "en";
  widget.setAttribute("aria-label", COPY[locale].ariaLabel);
  return COPY[locale];
}

function currentRuntimeState() {
  if (!window.__VOICEFLOW_OVERLAY_RUNTIME__) {
    return null;
  }

  const runtime = window.__VOICEFLOW_OVERLAY_RUNTIME__;
  if (
    typeof runtime.generated_at_epoch_ms === "number" &&
    Date.now() - runtime.generated_at_epoch_ms > RUNTIME_STALE_AFTER_MS
  ) {
    return null;
  }

  if (!runtime.latest_status && !runtime.last_session_summary && !runtime.last_failure_summary) {
    return null;
  }

  return runtime;
}

function currentOutcome(runtime) {
  const summary = runtime?.last_session_summary;
  const failure = runtime?.last_failure_summary;

  if (summary && failure) {
    return summary.session_id >= failure.session_id
      ? { kind: "summary", value: summary }
      : { kind: "failure", value: failure };
  }

  if (summary) {
    return { kind: "summary", value: summary };
  }

  if (failure) {
    return { kind: "failure", value: failure };
  }

  return null;
}

function isLatestOutcomeForStatus(status, outcome) {
  return Boolean(
    status &&
      outcome?.value &&
      typeof status.session_id === "number" &&
      status.session_id === outcome.value.session_id,
  );
}

function toErrorMessage(failure, copy) {
  if (!failure) {
    return copy.error;
  }

  const message = String(failure.message ?? "").toLowerCase();
  if (message.includes("api key") || message.includes("apikey")) {
    return copy.apiKey;
  }
  if (
    failure.failure_phase === "Recognizing" ||
    message.includes("no speech") ||
    message.includes("not recognized") ||
    message.includes("empty transcript") ||
    message.includes("speech activity threshold") ||
    message.includes("silence gate")
  ) {
    return copy.silence;
  }

  return copy.error;
}

function classifyRuntime(runtime) {
  const status = runtime?.latest_status;
  const outcome = currentOutcome(runtime);
  const sessionId = status?.session_id ?? outcome?.value?.session_id ?? null;
  const copy = copyForRuntime(runtime);

  if (status?.state === "Arming" || status?.state === "Recording") {
    return {
      state: "listening",
      sessionId,
      label: copy.listening,
      text: "",
      duration: null,
    };
  }

  if (status && PROCESSING_STATES.has(status.state)) {
    return {
      state: "processing",
      sessionId,
      label: copy.processing,
      text: "",
      duration: null,
    };
  }

  if (
    status?.state === "Failed" ||
    (outcome?.kind === "failure" && (!status || isLatestOutcomeForStatus(status, outcome))) ||
    (outcome?.kind === "summary" &&
      outcome.value.final_state !== "Committed" &&
      (!status || isLatestOutcomeForStatus(status, outcome)))
  ) {
    return {
      state: "error",
      sessionId,
      label: toErrorMessage(outcome?.value, copy),
      text: "",
      duration: ERROR_VISIBLE_MS,
    };
  }

  if (
    status?.state === "Committed" ||
    (outcome?.kind === "summary" && (!status || isLatestOutcomeForStatus(status, outcome)))
  ) {
    return {
      state: "success",
      sessionId,
      label: copy.success,
      text: "",
      duration: SUCCESS_VISIBLE_MS,
    };
  }

  return {
    state: "hidden",
    sessionId,
    label: "",
    text: "",
    duration: null,
  };
}

function clearHideTimer() {
  if (hideTimer) {
    window.clearTimeout(hideTimer);
    hideTimer = null;
  }
}

function notifyNativeHidden() {
  if (window.__VOICEFLOW_NATIVE_OVERLAY__ && window.ipc?.postMessage) {
    window.ipc.postMessage(JSON.stringify({ type: "overlay-hidden" }));
  }
}

function setHidden() {
  shell.dataset.state = "hidden";
  shell.setAttribute("aria-hidden", "true");
  stateLabel.textContent = "";
  statusText.textContent = "";
  statusText.hidden = true;
  notifyNativeHidden();
}

function renderWidget(view) {
  clearHideTimer();

  if (view.state === "listening" || view.state === "processing") {
    activeSessionId = view.sessionId;
  } else if (view.sessionId !== null) {
    activeSessionId = view.sessionId;
  }

  shell.dataset.state = view.state;
  shell.setAttribute("aria-hidden", view.state === "hidden" ? "true" : "false");
  stateLabel.textContent = view.label;
  statusText.textContent = view.text;
  statusText.hidden = !view.text;

  if (view.state === "hidden") {
    notifyNativeHidden();
    return;
  }

  if (view.duration !== null) {
    const timerSessionId = view.sessionId;
    const timerState = view.state;
    hideTimer = window.setTimeout(() => {
      if (timerSessionId !== activeSessionId || shell.dataset.state !== timerState) {
        return;
      }
      setHidden();
    }, view.duration);
  }
}

function renderCurrentView() {
  renderWidget(classifyRuntime(currentRuntimeState()));
}

function setRuntime(runtime) {
  window.__VOICEFLOW_OVERLAY_RUNTIME__ = runtime;
  overlayRuntimeTimestamp = runtime?.generated_at_epoch_ms ?? 0;
  renderCurrentView();
}

function reloadRuntimeScript() {
  const nextScript = document.createElement("script");
  nextScript.src = `./src/runtime-state.js?ts=${Date.now()}`;
  nextScript.async = true;
  nextScript.onload = () => {
    const runtime = currentRuntimeState();
    const nextTimestamp = runtime?.generated_at_epoch_ms ?? 0;
    if (nextTimestamp !== overlayRuntimeTimestamp) {
      overlayRuntimeTimestamp = nextTimestamp;
      renderCurrentView();
    }
    nextScript.remove();
  };
  nextScript.onerror = () => {
    nextScript.remove();
  };
  document.body.appendChild(nextScript);
}

window.__VOICEFLOW_OVERLAY_SET_RUNTIME__ = setRuntime;

if (window.__VOICEFLOW_OVERLAY_PENDING_RUNTIME__) {
  setRuntime(window.__VOICEFLOW_OVERLAY_PENDING_RUNTIME__);
  window.__VOICEFLOW_OVERLAY_PENDING_RUNTIME__ = null;
} else {
  renderCurrentView();
}

window.setInterval(reloadRuntimeScript, 250);
