# Developer reference

Configuration, diagnostic commands and implementation notes for contributors.
For installation, start with the [build guide](build-from-source.md).

## Latency Expectations

Current development-machine expectations after recording stops are:

- Short, simple normal dictation fast path: roughly `1–2 s`.
- Normal dictation requiring provider refinement: roughly `2–4 s`.
- Short-clip ASR: commonly `0.8–1.5 s`, depending on audio length and hardware.
- Warm provider request: commonly `1.0–1.8 s`, depending on network/provider
  load.
- Text insertion: normally tens of milliseconds, with target-app variance.

These are working ranges, not service-level guarantees. Long dictation depends
on the remaining tail length, natural pauses, hardware, and provider latency.
Development-machine long-recording samples showed post-stop waits of `13.880 s`
before optimization, then `6.179 s` and `3.577 s` after successive changes, for
85.221, 81.777, and 76.454 seconds of audio respectively. These used different
spoken content and are observations, not a controlled benchmark or guarantee.

After 25 seconds, completed local ASR chunks can be recognized during recording.
An existing tail can also be prefetched after a sufficiently quiet natural pause;
this optimization adds no forced cuts. Buffered WAV writes and exact-prefix
audio normalization reuse reduce preparation work. The complete transcript is
still refined in one request after recording stops, then inserted once.
Use the live latency diagnostics to identify slow stages rather than assuming
all waiting is caused by the refinement model.

## Terminology

The live host reports ready only after settings, shortcut registration, safe
audio preparation, and local speech-worker preparation complete. Overlay
window/WebView construction remains deferred to the first overlay state because
eager WebView2 construction is not reliable in the current host lifecycle. The
first recording-start summary reports overlay window/WebView creation, audio
backend/device discovery, and capture start as separate phases.

Instructed Dictation means trigger phrase + spoken instruction + spoken content
in one recording, while the user is already using the normal VoiceFlow
shortcut. VoiceFlow inserts only the transformed result.

It is not always-on wake word detection. It is not broad app/system command
execution. Selected-text edit remains separate and applies to already selected
text.

## Main Commands

Build and test:

```powershell
cargo build -p input-host
cargo test -p input-host
cargo test -p speech-engine
cargo test
node --check apps/settings-ui/src/main.js
node --check apps/overlay-ui/src/main.js
```

`input-host` provider tests should not read a developer's real saved provider
configuration. On a configured development machine, run them with an isolated
settings path:

```powershell
$env:VOICEFLOW_SETTINGS_PATH = Join-Path $env:TEMP ("voiceflow-tests-{0}.json" -f [guid]::NewGuid())
cargo test -p input-host
```

Run the live host:

```powershell
cargo run -p input-host -- --serve-live 8000 3
```

Settings:

```powershell
cargo run -p input-host -- --print-settings
cargo run -p input-host -- --write-default-settings
cargo run -p input-host -- --update-settings system_language=chinese ui_style=dark history_retention=last_30_days
cargo run -p input-host -- --serve-settings 8765
```

Open the Settings UI from the URL printed by `--serve-settings`. Direct
`file://` loading is useful only as an offline/read-only static view; live
saving, credential management, Clear History, and Test Connection use the
localhost Settings server.

Insertion and selected-text probes:

```powershell
cargo run -p input-host -- --insert-probe "hello"
cargo run -p input-host -- --replace-selection-probe "replacement text"
cargo run -p input-host -- --sendinput-key-probe --delay-ms 3000
cargo run -p input-host -- --clipboard-only-probe "clipboard text"
cargo run -p input-host -- --ctrl-v-probe --delay-ms 3000
```

Speech probes:

```powershell
cargo run -p input-host -- --capture-probe 3000
cargo run -p input-host -- --transcribe-probe 3000
cargo run -p input-host -- --edit-probe "translate this into English"
cargo run -p input-host -- --edit-transcribe-probe 5000
```

## Configuration

Default settings path:

```text
%LOCALAPPDATA%\VoiceFlow Speech Input\settings.json
```

Development override:

```text
VOICEFLOW_SETTINGS_PATH
```

Provider presets:

- Alibaba Cloud Bailian:
  `https://dashscope.aliyuncs.com/compatible-mode/v1`
- Volcengine Ark: `https://ark.cn-beijing.volces.com/api/v3`
- Tencent Hunyuan / TokenHub: `https://tokenhub.tencentmaas.com/v1`
- Custom OpenAI-compatible, which requires an explicit Base URL

Editable non-secret fields are Base URL, active model, and request timeout.

Built-in presets supply their Base URL, so normal setup needs only a model or
model ID and API key. An Advanced endpoint override remains available, and
Reset restores the built-in default. The Provider form keeps its full-width
layout stable across presets; unsaved endpoint drafts are not overwritten by
background effective-status refreshes. Effective configuration is shown in
Advanced details, one value per row.

Configuration precedence is:

1. Environment variables.
2. Saved non-secret settings in `settings.json`.
3. Built-in defaults.

Provider-related environment names:

- `VOICEFLOW_PROVIDER_TYPE`
- `VOICEFLOW_PROVIDER_API_KEY`
- `VOICEFLOW_PROVIDER_BASE_URL`
- `DASHSCOPE_API_KEY`
- `DASHSCOPE_BASE_URL`
- `VOICEFLOW_LLM_REFINE_TIMEOUT_MS`

Model override names:

- `VOICEFLOW_MODEL_FAST`
- `VOICEFLOW_MODEL_FAST_THINKING`
- `VOICEFLOW_MODEL_BALANCED`
- `VOICEFLOW_MODEL_BALANCED_THINKING`
- `VOICEFLOW_MODEL_BEST`
- `VOICEFLOW_MODEL_BEST_THINKING`

Prompt override names:

- `VOICEFLOW_DICTATION_PROMPT_FILE`
- `VOICEFLOW_SELECTED_TEXT_PROMPT_FILE`
- `VOICEFLOW_INSTRUCTED_DICTATION_PROMPT_FILE`

API keys are stored in isolated per-provider Windows Credential Manager slots
when saved from the Settings UI. They are never stored in `settings.json`,
runtime-state mirrors, History, reports, logs, diagnostics, or tracked files. Legacy
`DASHSCOPE_API_KEY` and `DASHSCOPE_BASE_URL` remain supported for Bailian.
`VOICEFLOW_PROVIDER_API_KEY` and `VOICEFLOW_PROVIDER_BASE_URL` are generic
developer overrides and take precedence over saved settings. Remove old API-key
values from `.env.local` when testing Windows Credential Manager behavior.

Provider, model, Base URL, timeout, and API-key changes are resolved on the
next AI request. Developer environment variables are process environment and
may still require restarting the host.

`.env.local` is for development only. Do not commit, package, or copy local
secret values into documentation.

## Local Data

VoiceFlow stores local runtime data near the settings file:

- `settings.json`
- `usage-ledger.json`
- `history-ledger.json`

Overview analytics use usage-ledger schema v2 and are independent of History
retention. Each successful session is persisted immediately, rather than only
when the host exits. The four summary metrics are total dictation duration,
inserted text, estimated time saved, and average dictation speed. The seven-day
chart shows inserted-text units and estimated time saved.

An inserted-text unit is one Han character, one Latin word, or one contiguous
number run; punctuation and whitespace do not count. Estimated manual typing
time uses a 40-unit-per-minute baseline. Estimated time saved is manual typing
time minus captured audio duration, clamped at zero. Schema-v1 records do not
contain enough data for a reliable analytics backfill.

History is stored locally on this device. History retention is configurable in
the Settings UI under History & Privacy and through `history_retention` updates.
Supported policies are:

- Latest 100 entries
- Latest 500 entries
- Latest 1000 entries
- Last 7 days
- Last 30 days
- Unlimited

The default remains Latest 100 entries, preserving the original behavior.
Time-based retention uses per-session completion timestamps. History dedup
metadata is pruned with the retained entries so it does not grow without bound.
Clear History is available through the localhost Settings UI.

Recent Overview content can be privacy-masked. User-facing activity categories
are Dictation, Text Edit, and Instructed Input.

Temporary diagnostics live under:

```text
%TEMP%\voiceflow-speech-input\reports
```

Latency diagnostics include process-to-ready startup phases, first recording
startup, host/worker ASR phases, prompt/payload/provider/validation phases, and
insertion. A local-only fast-path session reports `refine_model=none` and omits
provider preset/key-source metadata; cloud success or fallback reports the
provider and model actually attempted. Diagnostics do not include prompt text,
transcripts by default, credentials, or provider responses.

Generated runtime mirrors:

- `runtime/overlay-state.js` beside the settings file
- `runtime/settings-state.js` beside the settings file

Those mirrors are generated local state, not source-of-truth docs or packaged
configuration. Live Settings serves `/src/runtime-state.js` dynamically.
Native overlay state and WebView2 cache also use the user data directory.

## Settings Bridge Security

The live Settings UI is served from the loopback Settings server started by:

```powershell
cargo run -p input-host -- --serve-settings
```

The server binds to localhost, serves the UI and API from the same origin,
requires a per-process control token for state-changing routes, and no longer
uses wildcard CORS. The HTML/bootstrap response that carries the control token
is sent with cache disabled. Do not paste the token into logs or docs.

## Provider Test Connection

The Settings UI Test Connection action tests the current effective saved
configuration after provider settings have been applied. It uses a minimal
synthetic provider request, caps the test timeout, reports sanitized result
categories only, and discards provider output. It does not write History,
reports, settings fields, or generated runtime-state mirrors.

## Live Settings Application

The live host watches the same settings file written by the Settings server.
Settings saves use atomic file replacement, and an invalid reload leaves the
last valid runtime configuration active. Shortcut changes are rebound
transactionally; a failed rebind keeps the previous shortcut.

Changes apply at these boundaries:

- Immediately: UI language, shortcut, Push to Talk / Toggle, audio cues,
  diagnostics, and History retention.
- Next session: pause sensitivity, trigger phrase / Instructed Input, and
  refinement quality.
- Next AI request: provider, model, Base URL, timeout, and API key.

Normal Settings controls do not require restarting VoiceFlow. Environment
variable changes may still require a process restart.

## Prompt Overrides

VoiceFlow includes built-in prompts for Dictation, Text Edit, and Instructed
Input. In the connected Settings page, Advanced prompt settings provides an
editor for light cleanup, structured cleanup, selected-text editing, and
instructed dictation. A warning precedes editing; each prompt has an explicit
Save action and a confirmed Restore default action (also saved explicitly).
Unsaved edits survive background refreshes and failed saves.

Overrides are stored locally in `%LOCALAPPDATA%\VoiceFlow Speech Input\prompts.json`
and apply to subsequent requests without a restart. They contain full system
prompts, including shared rules, so edit carefully and do not enter secrets.
Advanced users can still use environment-selected prompt files: UI overrides
take precedence, and restoring defaults pins that slot to the built-in prompt
without changing those files. Runtime mirrors and diagnostics contain status
only; reading or writing prompt contents requires the live Settings control token.

## Instructed Dictation

The configured trigger phrase defaults to `Voice Flow`.

Accepted beginning-of-transcript variants include:

- `Hey VoiceFlow`
- `Voice Flow`
- `hey voiceflow`
- `hey voice flow`
- `hey, voice flow`
- `Hey Voice Flow`

Example:

```text
Hey VoiceFlow，把下面这段话翻译成英文：我认为现在接下这个任务不是一个明智的选择。
```

If ASR removes punctuation, this should still parse:

```text
hey voice flow 把下面这段话翻译成英文我认为现在接下这个任务不是一个明智的选择
```

The trigger must be at the beginning. Without the trigger, the utterance stays
normal dictation.

## Current Limitations

- The portable build still needs clean-machine and target-app acceptance testing.
- No model discovery.
- No custom headers or enterprise authentication adapters.
- Windows insertion is still a temporary `SendInput`/clipboard path.
- Selected-text support depends on target-app clipboard behavior.
- The portable build requires a matching CPython runtime, sherpa_onnx package,
  SenseVoice model and token file as build inputs.
- There is no always-on wake word detection.
- There are no broad app/system voice commands.
- An existing parallel environment-variable test race remains known technical
  debt.

## Packaging Notes

See [portable build and usage](portable-usage.md). `scripts/build-portable.ps1`
creates an allowlisted bundle; assets and ASR paths resolve beside the executable,
while user state remains in Local AppData. Development overrides are
`VOICEFLOW_ASR_PYTHON`, `VOICEFLOW_ASR_WORKER`, and `VOICEFLOW_ASR_MODEL_DIR`.
Use `runtime/asr/worker.py`, which returns token timestamps for overlap handling.

Packaging must include the runtime executable, local ASR worker, model files,
and static overlay/settings assets. It must not include `.env.local`, `.codex/`,
debug `target/` artifacts, reports/cache/temp files, generated runtime-state
mirrors, local usage/history ledgers, or secrets.
