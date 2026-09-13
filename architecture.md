# VoiceFlow Architecture

This document describes the current implementation shape of the Windows-first
VoiceFlow speech input prototype.

The Tauri 2 desktop shell lives in `apps/desktop`. Its native Settings window
loads the existing loopback Settings bridge; the page receives no Tauri IPC
capabilities and top-level navigation is restricted to that bridge origin.
The shell starts the packaged Settings and live-host executables, pins ASR
paths to the package, and supervises their process tree with a Windows Job
Object. Closing Settings hides it; the tray provides reopen and exit actions.
The existing Wry overlay remains owned by the speech host. This preserves the
tested audio and insertion pipeline while adding the requested desktop entry.

## Runtime Shape

VoiceFlow is centered on the Rust `input-host`. UI surfaces are companions, not
the runtime center.

```text
Global shortcut
  -> input-host session lifecycle
  -> native audio capture
  -> speech-engine local ASR / short fast path / provider refinement
  -> host routing
      -> Dictation
      -> SelectedTextEdit
      -> InstructedDictation
  -> Windows insertion / replacement layer
  -> active target app

input-host
  -> Wry/WebView2 overlay window -> apps/overlay-ui renderer
  -> overlay runtime-state mirror (development only)
  -> settings runtime-state mirror
  -> local settings, usage, history, and diagnostics files
```

## Main Crates And Responsibilities

### `crates/input-host`

Owns:

- Windows shortcut prototype and session lifecycle.
- Native audio capture and live-host commands.
- Settings loading, sanitizing, updating, localhost settings bridge, and
  per-provider credential storage integration.
- Runtime-state mirrors for overlay/settings static pages.
- Production Wry/WebView2 overlay hosting, state injection, window lifecycle,
  focus safety, and DPI-aware active-monitor placement.
- Routing between dictation, selected-text edit, and instructed dictation.
- Windows insertion, selected-text capture/replacement, and insertion probes.
- Local usage and history ledger updates.
- Live-host reports and selected-text validation reports.

### `crates/speech-engine`

Owns:

- Local ASR worker interface.
- Conservative outer-silence trimming for short whole-clip ASR.
- Routing policy for local-only, local-refine, and cloud-refine decisions.
- Short simple normal-dictation eligibility and deterministic punctuation
  cleanup.
- Provider-compatible request construction for Alibaba Cloud Bailian,
  Volcengine Ark, Tencent Hunyuan, and Custom OpenAI-compatible endpoints.
- Shared HTTP-agent reuse for compatible provider timeouts.
- Provider Test Connection request construction and sanitized result mapping.
- Dictation refinement prompt.
- Selected-text edit prompt.
- Instructed Dictation prompt.
- Prompt-file overrides through environment variables.
- Per-request prompt-file loading with built-in fallback and safe override
  status diagnostics.
- Degrade-to-ASR behavior for normal dictation refinement failures.

### `crates/shared-protocol`

Owns:

- Session kinds, session states, route names, and settings types.
- Runtime settings schema, including `system_language`, `ui_style`,
  `history_retention`, and non-secret provider settings.
- Refinement model profile mapping and model override handling.
- Instructed Dictation parser and parser diagnostics.
- Shared summary structs for selected-text edit, wake-phrase intent, and
  instructed dictation.

### `crates/intent-edit-executor`

Owns:

- Deterministic selected-text edit transforms.
- Provider-backed selected-text edit executor trait boundary.
- Legacy deterministic wake-phrase intent executor.
- Instructed Dictation executor trait boundary.

## Session Kinds

Current session kinds:

- `Dictation`
- `SelectedTextEdit`
- `InstructedDictation`
- `WakePhraseIntent`

`WakePhraseIntent` remains in the protocol for legacy deterministic probes and
historical diagnostics. The user-facing command-like path added in the latest
checkpoint is `InstructedDictation`, not always-on wake word detection and not
broad app/system command execution.

## Dictation Pipeline

Normal dictation flow:

1. User triggers VoiceFlow with the configured shortcut.
2. Host captures audio.
3. Speech engine returns raw recognition.
4. Host first checks whether the raw transcript starts with the configured
   Instructed Dictation trigger phrase.
5. If no trigger matches, the normal dictation route continues.
6. Up to 15 text units uses local cleanup; 16–60 uses light provider cleanup;
   over 60 uses structured provider cleanup. Text units count Chinese characters,
   English words and number groups. Detected self-corrections use light cleanup.
7. Commands mentioned in ordinary dictation do not independently trigger execution.
8. If refinement fails after usable ASR text exists, the engine degrades to ASR
   text instead of failing the session.
9. Host inserts the final text into the active target app.

## Live-Host Preparation

Before `ready_for_first_dictation`, the live host:

- loads and validates settings
- registers the configured shortcut
- constructs and caches the CPAL host, then probes the current default input
  device/configuration without building or playing an input stream
- starts the persistent SenseVoice worker and, on a cold worker, runs one
  inference over 250 ms of synthetic silence

Audio preparation never starts microphone capture or retains samples. Recording
still resolves the current default device/configuration again so device changes
and temporary device loss can recover through the normal start path. Speech
warm-up creates no session, overlay state, History/usage entry, insertion, or
cloud request; failure is nonfatal and leaves normal lazy worker startup
available.

The production overlay window/WebView is still created lazily on the first
overlay state. Eager WebView2 initialization was tested on the dedicated overlay
thread, including overlapped preparation and an isolated WebView profile, but
blocked inside WebView2 environment/controller creation before host readiness.
The proven single-window lazy lifecycle remains the production path.

## Instructed Dictation Pipeline

Instructed Dictation flow:

1. The user starts a normal VoiceFlow recording.
2. The raw ASR transcript must start with the configured trigger phrase.
3. Trigger matching tolerates the default `Voice Flow` and common ASR variants
   such as `hey voiceflow`, `hey voice flow`, and `hey, voice flow`.
4. Host removes the trigger phrase and immediate punctuation/spacing.
5. Parser splits the remaining text into instruction and spoken content.
6. Host routes to `InstructedDictation` before legacy wake-phrase intent or
   normal dictation refinement.
7. Provider receives separate instruction/content fields.
8. Provider prompt asks for only the final transformed output.
9. Host inserts only that transformed output.

If a trigger phrase matches but no content can be parsed, the session fails
softly with sanitized diagnostics and does not insert the raw command.

No-punctuation Chinese parsing supports forms such as:

- `把下面这段话翻译成英文<content>`
- `把下面这段翻译成英文<content>`
- `把下面内容翻译成英文<content>`
- `将下面这段话翻译成英文<content>`
- `请把下面这段话翻译成英文<content>`
- `帮我把下面这段话翻译成英文<content>`

Supported target-language words include `英文`, `中文`, `日文`, `韩文`,
`德文`, `法文`, and `西班牙文`.

## Selected-Text Edit Pipeline

Selected-text edit is separate from Instructed Dictation:

1. User selects text in the target app.
2. Explicit selected-text edit can use strict clipboard-backed capture.
   Ordinary push-to-talk does not unconditionally pay the strict no-selection
   timeout; its probe is skipped or fast unless the route needs selection.
3. The spoken instruction applies to the already selected text.
4. Deterministic transforms handle known local operations.
5. Freeform provider-backed edit handles general rewrite/translation requests.
6. Host replaces the selected text through the temporary replacement path.

History entries for selected-text edits are redacted by default.

## Settings And Runtime State

Default settings path:

```text
%LOCALAPPDATA%\VoiceFlow Speech Input\settings.json
```

Development override:

```text
VOICEFLOW_SETTINGS_PATH
```

Persisted settings include:

- shortcut key/modifiers
- shortcut mode
- refinement quality
- silence gate level
- diagnostics verbosity
- audio feedback
- wake phrase enabled/phrase
- `system_language`: `English` or `Chinese`
- `ui_style`: `Dark` or `Light`
- `history_retention`: `latest_100`, `latest_500`, `latest_1000`,
  `last_7_days`, `last_30_days`, or `unlimited`
- provider preset, base URL, active model, and request timeout

The default history retention policy is `latest_100`, preserving the original
latest-100 behavior.

The host can update settings through:

```text
cargo run -p input-host -- --update-settings key=value [...]
cargo run -p input-host -- --serve-settings [port]
```

The live Settings UI should be opened from the URL printed by
`--serve-settings`. The loopback Settings server serves the UI and API from the
same origin. State-changing routes require a per-process control token, and
wildcard CORS is not used. The HTML/bootstrap response that carries the token
is sent with cache disabled. Direct `file://` loading remains an offline/static
view and cannot obtain the live mutation token.

The Settings UI also reads safe runtime-state metadata. Runtime-state may show
provider preset, effective base URL/model/timeout, source labels, credential
presence, and key source, but never raw API keys.

Settings are written through atomic file replacement. The live host polls the
authoritative settings path and keeps the last valid runtime settings when a
reload is invalid. Shortcut rebinding is transactional: if the new binding
fails, the previous binding and runtime snapshot remain active.

Application boundaries are intentional:

- Immediate: UI language, shortcut, Push to Talk / Toggle, audio cues,
  diagnostics, and History retention.
- Next session: pause sensitivity, trigger phrase / Instructed Input, and
  refinement quality.
- Next AI request: provider, model, Base URL, timeout, and API key.

Normal Settings controls therefore do not require a host restart. Developer
environment variables are inherited process state and may require one.

## Provider Configuration

The provider path supports four presets:

- Alibaba Cloud Bailian:
  `https://dashscope.aliyuncs.com/compatible-mode/v1`
- Volcengine Ark: `https://ark.cn-beijing.volces.com/api/v3`
- Tencent Hunyuan / TokenHub: `https://tokenhub.tencentmaas.com/v1`
- Custom OpenAI-compatible, with an explicit Base URL

Non-secret provider configuration is layered:

1. Environment variables.
2. Saved settings in `settings.json`.
3. Built-in defaults.

Saved provider settings include provider preset, base URL, active model, and
request timeout. API keys are not part of the settings schema.

Built-in presets provide their default Base URL. Users normally supply a model
or model ID and API key, while Advanced details permits a saved custom endpoint
override. Reset clears that saved override and returns to the current built-in
default. Environment URL overrides remain highest priority. Tencent uses its
own Credential Manager slot; request authentication and OpenAI-compatible
response parsing remain shared with other compatible providers.

HTTP agents are cached by timeout and reused across compatible requests so
connections can stay warm without retaining provider API keys or base URLs in
the cache.

Relevant names:

- `VOICEFLOW_PROVIDER_TYPE`
- `VOICEFLOW_PROVIDER_API_KEY`
- `VOICEFLOW_PROVIDER_BASE_URL`
- `DASHSCOPE_API_KEY`
- `DASHSCOPE_BASE_URL`
- `VOICEFLOW_LLM_REFINE_TIMEOUT_MS`
- `VOICEFLOW_MODEL_FAST`
- `VOICEFLOW_MODEL_FAST_THINKING`
- `VOICEFLOW_MODEL_BALANCED`
- `VOICEFLOW_MODEL_BALANCED_THINKING`
- `VOICEFLOW_MODEL_BEST`
- `VOICEFLOW_MODEL_BEST_THINKING`
- `VOICEFLOW_DICTATION_PROMPT_FILE`
- `VOICEFLOW_SELECTED_TEXT_PROMPT_FILE`
- `VOICEFLOW_INSTRUCTED_DICTATION_PROMPT_FILE`

`DASHSCOPE_API_KEY` and `DASHSCOPE_BASE_URL` are legacy Bailian-only overrides.
`VOICEFLOW_PROVIDER_API_KEY` and `VOICEFLOW_PROVIDER_BASE_URL` are generic
developer overrides. Environment values take precedence over saved settings,
so developers should remove old API-key values from `.env.local` when testing
Windows Credential Manager behavior.

API keys saved from Settings are stored per provider in Windows Credential
Manager. They are never written to `settings.json`, runtime-state mirrors,
History, reports, logs, diagnostics, or tracked files. Normal provider-backed
requests reload settings and credentials at request/session time so saved
provider settings and stored credentials can take effect without restarting the
live host. If a provider-backed reload fails, the host may use the latest
successfully resolved effective provider configuration; deleted credentials are
authoritative and must not be resurrected through this cache. Local short
fast-path dictation avoids Credential Manager access.

Missing provider keys fall back safely for normal dictation after ASR succeeds.
Selected-text edit and Instructed Dictation still require provider-backed
execution for freeform transforms.

Test Connection is exposed through the Settings bridge as a token-protected
mutation route. It accepts no arbitrary URL/model/key/body from the UI, tests
the current effective saved configuration, sends a minimal synthetic request,
returns sanitized categories only, and writes no History, reports, settings,
or runtime-state mirrors.

The Provider UI keeps one stable full-width form across presets. Background
effective-configuration polling updates status without replacing dirty provider
drafts, including an unsaved custom-endpoint toggle or URL. Effective values
are rendered in Advanced details as one full-width value per row.

## Prompt Resolution

VoiceFlow has built-in prompts for Dictation, Text Edit, and Instructed Input.
Each relevant provider request resolves its prompt-file environment variable
and reads a non-empty file at use time; missing, unreadable, or empty files fall
back to the built-in prompt. Settings exposes safe override status diagnostics
and a status refresh without serializing prompt contents.

There is no in-UI Prompt Editor yet. The planned editor will write per-category
user overrides under the application data directory, restore built-in defaults,
apply without restart, and preserve environment/file overrides for advanced
users.

## Local Data And Reports

Local user data:

- settings JSON under `%LOCALAPPDATA%`
- `usage-ledger.json` next to the settings file
- `history-ledger.json` next to the settings file

Usage-ledger schema v2 stores one deduplicated analytics record for each
successfully committed session and is written immediately after that session.
It does not depend on live-host shutdown or History retention. Settings reads
the same ledger next to the same authoritative settings path.

Analytics count each Han character, each Latin word, and each contiguous number
run as one inserted-text unit; punctuation and whitespace are excluded. Manual
typing time is `inserted units / 40 * 60 seconds`. Estimated time saved is that
manual estimate minus captured audio duration, clamped at zero. Average
dictation speed is inserted units divided by captured audio duration. Legacy
schema-v1 data cannot be completely backfilled because it lacks reliable
per-session counts and recording durations.

History is stored locally on this device. Retention is enforced after appending
history, after loading history, and after saving a stricter retention setting.
Count-based policies keep the newest retained entries. Time-based policies use
per-session completion timestamps. The history ledger also prunes dedup
metadata for entries no longer retained, so applied session keys stay bounded by
visible history.

Temporary diagnostics:

- live-host reports under `%TEMP%\voiceflow-speech-input\reports`
- selected-text validation report and archives under the same report directory

Generated runtime-state mirrors:

- `runtime/overlay-state.js` beside the settings file
- `runtime/settings-state.js` beside the settings file

The runtime-state mirrors are generated local files, not product source of
truth.

## Insertion Layer

The current insertion layer is intentionally temporary:

- Direct Unicode `SendInput` commit.
- Safe clipboard-first Ctrl+V for multiline and Markdown-style bullet/list
  output.
- Clipboard backup/restore and CRLF canonicalization for `CF_UNICODETEXT`.
  Multiline paste failure returns a failed commit without direct Enter-key input.
  Selection capture attempts restore the clipboard on success and error exits.
- Clipboard-backed selected-text capture and replacement.
- Foreground process/class/title diagnostics.
- Win32 last-error clearing and immediate `GetLastError()` capture around
  `SendInput`.

Diagnostic probes:

- `--insert-probe [text]`
- `--replace-selection-probe [text]`
- `--sendinput-key-probe [--delay-ms ms]`
- `--clipboard-only-probe "text"`
- `--ctrl-v-probe [--delay-ms ms]`

The insertion path is not TSF/TIP-class integration yet.

## Recognition During Recording

The CPAL adapter exposes an owned snapshot callback over its capture buffer.
For recordings longer than 25 seconds, a single background task polls snapshots
and speculatively recognizes completed chunks using the existing local worker.
The microphone callback only captures samples; it does not perform inference.
The task is stopped and joined before final recognition. An in-flight worker
request remains subject to the existing timeout, and join time is measured.

The final whole-recording segmentation is authoritative. Reuse requires the
same session, exact sample range, and identical normalized samples. Mismatches
or speculative failures use the normal final-recognition path. The growing
tail is excluded unless it has at least 900 ms of quiet after existing padding,
with every frame below the minimum VAD threshold. Hard-cut tails are excluded
from that additional eligibility rule. Existing segmentation limits are not
shortened and no new forced cuts are introduced.

Normalization reuses an exact raw-audio prefix with matching rate/channels;
partial frames and the interpolation boundary are recomputed. Source changes
invalidate reuse. WAV output uses a 64 KiB buffer and explicit flush, preserving
the PCM16 format and surfacing write failures. Recognition cache retention is
capped at 128 chunks; normalization retains the latest source/normalized prefix.

Routing and LLM refinement still receive the full transcript after recording
ends. There is no per-chunk LLM refinement or incremental text insertion.

## Latency Diagnostics

Startup diagnostics separate settings/reloader/runtime-state initialization,
overlay/insertion adapter construction, shortcut registration, safe audio
preparation, speech model load, speech warm-up, host-ready time, and total
process-to-ready time. They explicitly report that microphone capture has not
started and that overlay WebView initialization is deferred.

Recording-start diagnostics separate:

- overlay state publication
- first-use overlay window and WebView construction
- start feedback and recording-context preparation
- audio backend creation, device discovery, and capture start
- remaining unattributed recording-start time

Post-recording diagnostics separate:

- audio stop/finalization and speech-engine recognition
- ASR host audio preparation, WAV write, worker roundtrip, and temp cleanup
- prompt loading, payload building, provider request, and output validation
- insertion time
- prompt/transcript character counts and refinement model
- semantic guard/rejection/fallback state
- selected-text probe mode, result, reason, and duration
- short-dictation fast-path usage, reason, and cloud-skip state

Additional ASR logs report `prefetch_join_ms` (foreground wait for speculation),
`prefetched_chunk_count` and `final_chunk_count` (reuse counts), and
`normalization_reused_ms` (duration of reused audio, not elapsed time saved).
`prefetched_worker_ms` records historical worker time for reused chunks;
foreground worker metrics exclude that work and must not have it added back.

Local-only fast-path sessions use the explicit safe value
`refine_model=none` and omit provider preset/key-source fields. Cloud-refined
and cloud-fallback sessions report the provider/model actually attempted.
These timing records do not contain prompt contents, transcripts by default,
credentials, authorization headers, or provider responses.

## UI Surfaces

Overlay UI:

- `apps/overlay-ui` HTML/CSS/JS is the single production renderer.
- A single reusable Wry/WebView2 window receives state through native script
  injection; generated runtime-state JS is only a development mirror.
- The compact bottom-center widget presents listening, thinking, done, and
  error states in English or Chinese.
- The host creates it hidden, always on top, no-activate, click-through, and
  skip-taskbar. It repositions against the foreground monitor work area using
  effective DPI and supports negative monitor coordinates.
- Language changes are carried by subsequent state updates without restarting
  the live host.

Settings UI:

- Static assets in `apps/settings-ui`, served by the localhost Settings server
  for live mutation support.
- Includes settings, diagnostics, usage, history, provider configuration,
  credential management, and Test Connection surfaces.
- English/Chinese localization exists for normal user-facing UI.
- Dark/Light style persistence exists; Dark is the default.
- Overview presents seven-day inserted-text/time-saved activity, four reliable
  usage metrics, recent privacy-masked content, and current setup.
- Settings uses five sections: General, Voice & Output, History &
  Privacy, AI Provider, and Advanced.
- History & Privacy exposes local history retention policies and keeps the
  local-only privacy copy visible.

## Packaging Readiness

Packaging is not complete.

Packaging still needs to decide and include:

- `input-host` executable.
- SenseVoice/local ASR worker runtime.
- Model files.
- Static overlay/settings assets.
- Safe config templates.
- Packaged Settings asset resolution.

Packaging must exclude:

- `.env.local`
- `.codex/`
- debug `target/` artifacts
- reports/cache/temp files
- runtime-state mirrors
- local usage/history ledgers
- secrets and local config

## Current Architectural Risks

1. `SendInput` and clipboard insertion are compatibility fallbacks, not final
   Windows input-method architecture.
2. Selected-text capture depends on target-app clipboard behavior.
3. Provider-backed features require clean missing-config behavior.
4. Model discovery, custom headers, and enterprise authentication adapters are
   not implemented.
5. The in-UI Prompt Editor is not implemented.
6. Packaging work must avoid accidentally shipping local secrets, reports, or
   generated state.
7. Existing parallel environment-variable tests have a known race that should
   be retired with better env isolation.
