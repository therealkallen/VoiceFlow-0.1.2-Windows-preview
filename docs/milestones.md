# Milestones

This file tracks the current VoiceFlow rebuild status. The latest checkpoints
are:

- `32b1cba Add web-rendered live overlay widget`
- `cb0bb2e Redesign overview with reliable usage analytics`
- `697a993 Redesign settings with live runtime updates`
- `3cd835b Add provider presets and Tencent Hunyuan support`
- `188e381 Update Tencent Hunyuan preset to TokenHub`
- `5b4ed1b Reduce first-session latency and improve diagnostics`

## Completed

### Milestone 1: Background Runtime

Status: completed for prototype use.

- Rust workspace established around `input-host`, `speech-engine`,
  `shared-protocol`, `intent-edit-executor`, and diagnostics-oriented host
  modules.
- Host runs independently of the companion settings page.
- Windows global shortcut prototype exists.
- Native CPAL capture path exists.
- Persistent local ASR worker integration exists.
- Live host commands support one-shot and persistent sessions.
- One reusable Wry/WebView2 overlay window hosts `apps/overlay-ui` as the single
  production renderer.

### Milestone 2: Speech Pipeline And Provider Refinement

Status: completed for current prototype.

- Local ASR is the primary runtime path.
- Routing policy supports local-only short text, local ASR plus refinement, and
  cloud-ASR route metadata for longer audio.
- Provider refinement supports the configured provider preset.
- Refinement failures degrade to usable ASR text for normal dictation.
- Model profile overrides are available through environment variables.
- Prompt-file overrides exist for dictation, selected-text edit, and instructed
  dictation.
- Compatible provider requests reuse HTTP agents.
- Short whole-clip ASR conservatively trims leading/trailing silence.
- Eligible short simple normal dictation uses deterministic local punctuation
  cleanup without a provider request.

### Milestone 3: Settings, Diagnostics, And Local Ledgers

Status: completed for current prototype.

- JSON-backed settings store exists.
- Default settings path is user-local under `%LOCALAPPDATA%`.
- `--update-settings` and `--serve-settings` persist settings through the host.
- Settings bridge mirrors effective settings, warnings, report summaries,
  usage summaries, and history summaries to the static Settings UI.
- System language persists as English or Chinese.
- UI style persists as Dark or Light, with Dark as the default.
- Usage ledger records local aggregate usage values.
- History ledger records recent local outputs with privacy redaction rules.
- History retention is configurable with Latest 100, Latest 500, Latest 1000,
  Last 7 days, Last 30 days, and Unlimited policies. Latest 100 remains the
  default.
- Time-based retention uses per-session completion timestamps, and dedup
  metadata is pruned with retained history.
- Clear History is available through the localhost Settings UI.
- Usage-ledger schema v2 persists successful-session analytics immediately and
  independently of History retention.

### Milestone 4: Selected-Text Edit

Status: completed for MVP.

- Selected-text capture/replacement works through a temporary clipboard-backed
  Windows path.
- Local deterministic selected-text transforms still exist.
- Provider-backed freeform selected-text edit is available.
- Selected-text history entries are redacted by default.
- Validation probes and compatibility ledgers help track app-specific behavior.

### Milestone 5: Instructed Dictation

Status: completed for MVP.

- Instructed Dictation is a distinct session/route.
- It activates only when the raw ASR transcript starts with the configured
  trigger phrase.
- Trigger matching tolerates the default `Voice Flow`, plus `Hey VoiceFlow`,
  `hey voiceflow`, `hey voice flow`, `hey, voice flow`, and `Hey Voice Flow`.
- Trigger matches route to Instructed Dictation before legacy wake-phrase
  intent or normal dictation refinement.
- No-punctuation Chinese command parsing supports common translation forms,
  including `hey voice flow 把下面这段话翻译成英文...`.
- Provider prompt construction separates instruction and spoken content and
  asks for only the final transformed output.
- Missing content after a trigger match fails softly and does not insert the
  raw command.
- History and diagnostics avoid storing raw trigger/instruction/content.

### Milestone 6: Insertion Diagnostics

Status: completed for diagnostic MVP.

- Production insertion path logs foreground diagnostics and `SendInput`
  acceptance/error details.
- Minimal probes exist for isolating environment failures:
  `--sendinput-key-probe`, `--clipboard-only-probe`, and `--ctrl-v-probe`.
- Earlier insertion failures were likely caused by 360 security software
  blocking or quarantining `target/debug/input-host.exe`; probes and live smoke
  tests worked again after restoring the executable and disabling 360.

### Milestone 7: Latency And Formatted-Text Reliability

Status: completed for current prototype.

- Ordinary push-to-talk no longer waits for the strict two-attempt
  selected-text no-selection timeout.
- Selected-text probe diagnostics report skipped, fast, or strict mode plus
  result, reason, and duration.
- Live latency diagnostics separate ASR, refinement, provider request,
  insertion, and post-recording time.
- A cold persistent SenseVoice worker runs one nonfatal inference over 250 ms
  of synthetic silence before live-host readiness, with no cloud or
  user-visible side effects.
- The host safely caches the CPAL host and probes the default input
  device/configuration before readiness without starting microphone capture.
- Recording-start diagnostics identify first-use overlay window/WebView
  construction, audio backend/device work, capture start, and unattributed
  time.
- Post-recording diagnostics identify host-side WAV/worker/cleanup work and
  prompt loading, payload building, provider request, and output validation.
- Local-only fast-path diagnostics no longer attribute the configured cloud
  model to a request that was never attempted.
- Eager WebView2 construction remains disabled after startup experiments
  blocked inside WebView2 initialization; the proven single lazy renderer is
  retained and measured.
- Multiline and Markdown-style list text prefers safe clipboard paste, restores
  the previous clipboard, and falls back to direct Unicode insertion.
- Clipboard Unicode newlines are canonicalized to CRLF.
- Short whole-clip ASR trims only conservative outer silence and preserves
  internal pauses and padding.
- Short simple normal dictation up to 30 characters can skip cloud refinement.
- Current development-machine expectation is roughly `1–2 s` after recording
  for the short fast path and `2–4 s` for provider-refined normal dictation.

### Milestone 8: Multi-Provider Configuration And Secure BYOK

Status: completed for current prototype.

- Supported provider presets are Alibaba Cloud Bailian, Volcengine Ark,
  Tencent Hunyuan, and Custom OpenAI-compatible.
- Built-in endpoints are Bailian
  `https://dashscope.aliyuncs.com/compatible-mode/v1`, Volcengine Ark
  `https://ark.cn-beijing.volces.com/api/v3`, and Tencent Hunyuan / TokenHub
  `https://tokenhub.tencentmaas.com/v1`.
- Saved non-secret provider settings include provider preset, Base URL, active
  model, and request timeout.
- Provider configuration resolves environment variables first, saved settings
  second, and built-in defaults last.
- API keys are stored per provider in Windows Credential Manager and are never
  written to settings JSON, runtime-state mirrors, History, reports, logs, or
  tracked files.
- Legacy `DASHSCOPE_API_KEY` remains supported for Bailian, while
  `VOICEFLOW_PROVIDER_API_KEY` is the generic developer override.
- Provider-backed requests can observe saved settings and stored credential
  changes without restarting the live host.
- Deleted credentials are authoritative and are not resurrected through
  last-known-good provider caching.
- Local short fast-path dictation does not access Windows Credential Manager.
- Missing provider keys fall back safely for normal dictation after local ASR
  succeeds.
- Provider Test Connection uses the current effective saved configuration,
  sends a minimal synthetic request, returns sanitized categories only, and
  writes no History, reports, settings, or runtime-state mirrors.
- The localhost Settings bridge serves the UI same-origin, requires a
  per-process control token for state-changing routes, removes wildcard CORS,
  and disables caching on token-bearing HTML/bootstrap responses.

### Milestone 9: Production Overlay, Overview, And Live Settings

Status: completed for current prototype.

- The compact bottom-center overlay supports localized listening, thinking,
  done, silence, and error presentation.
- The overlay is click-through, no-focus, always on top, skip-taskbar, and
  placed on the foreground monitor with DPI-aware coordinates.
- UI language changes reach the overlay without restarting the live host.
- Overview shows seven-day inserted text and estimated time saved, plus total
  dictation duration, inserted text, estimated time saved, and average speed.
- Mixed-language counting treats each Han character, Latin word, and contiguous
  number run as one unit while excluding punctuation and whitespace.
- Estimated time saved uses a 40-unit-per-minute typing baseline minus captured
  audio duration, clamped at zero.
- Recent content supports privacy masking, and user-facing activity is grouped
  as Dictation, Text Edit, and Instructed Input.
- Settings is organized as General, Voice & Output, Prompts, History & Privacy,
  AI Provider, and Advanced.
- Settings writes are atomic, invalid reloads preserve the last valid runtime
  configuration, and shortcut changes rebind transactionally.
- Normal user settings apply immediately, on the next session, or on the next
  AI request according to their runtime boundary; host restart is not required.
- The Provider form remains stable across presets and preserves dirty endpoint
  drafts during background effective-status polling.

## In Progress

### Packaging Readiness

Status: not ready.

Known work:

- Define the packaged executable set.
- Package the SenseVoice worker and model files.
- Include static overlay/settings assets.
- Provide safe config templates without secrets.
- Exclude `.env.local`, `.codex/`, debug `target/` artifacts, reports/cache,
  runtime-state mirrors, local usage/history ledgers, and local secrets.
- Resolve packaged Settings asset lookup for the localhost Settings server.

### Prompt Editor

Status: not implemented.

- Current built-in prompts and file/environment overrides remain active.
- Settings can inspect and refresh safe override status but cannot edit prompt
  contents.
- The planned editor should cover Dictation, Text Edit, and Instructed Input,
  save user overrides under application data, restore built-in defaults, apply
  without restart, and preserve advanced file/environment overrides.

## Next Milestones

1. Packaging plan and first package script.
2. Manual smoke pass for Instructed Dictation:
   - normal dictation without trigger
   - `Hey VoiceFlow` trigger
   - `hey voice flow` ASR variant
   - no-punctuation Chinese translation command
3. Broader selected-text compatibility testing across target apps.
4. Long-term Windows insertion strategy beyond temporary `SendInput`.
5. In-UI Prompt Editor for the three product categories.
6. Model discovery and custom headers or enterprise authentication adapters.
7. Decide whether legacy deterministic wake-phrase intent remains as a
   developer probe or is retired from user-facing behavior.
8. Retire the known parallel environment-variable test race with stronger env
   isolation.
