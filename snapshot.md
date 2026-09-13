# VoiceFlow Project Snapshot

## Tauri desktop integration — 2026-09-12

- User confirmed the intended deliverable is a Tauri Windows desktop app for
  company-machine testing and an AI competition, with their supplied icon.
- `apps/desktop` adds a native Settings window, tray, single-instance activation,
  visible startup errors and supervised sidecar cleanup. The speech-host overlay
  and existing localhost Settings API remain in use.
- `scripts/build-desktop-portable.ps1` stages `VoiceFlow.exe`, the host, UI and
  independent Python/model copies into a new portable folder and ZIP.
- Desktop startup pins all three ASR path overrides to the bundle. Company
  application-control approval and clean-machine testing remain external checks;
  a portable ZIP still contains executables and does not bypass IT policy.
- Release 0.1.2 builds successfully. All 491 workspace tests pass; native page
  load, startup gating, close/reopen, single instance, orderly shutdown and forced
  termination cleanup were checked. Final-bundle ASR returned 41 timed tokens on
  the existing Chinese fixture. Source runtime mirrors stayed unchanged.
- Desktop child TEMP/TMP now use the application's data directory, preventing
  development diagnostic reports from appearing in a fresh desktop session.
- Delivery: `dist/VoiceFlow-desktop-0_1_2.zip`. See `docs/desktop-usage.md` and
  `docs/desktop-validation-2026-09-13.md` for evidence and remaining checks.

## Release review fixes — 2026-09-12

- Multiline clipboard failures no longer fall back to Enter-key input.
- Truncated HTTP bodies return an error; recording-wait failures stop capture;
  selection capture restores the clipboard on error; failed worker startup reaps its process.
- Runtime mirrors and WebView2 cache use the user settings directory. Settings
  serves its runtime script dynamically; tests no longer write source mirrors.
- ASR uses exe-relative portable paths or explicit development overrides. The
  repository owns `runtime/asr/worker.py`. Overlapping chunk text is assigned by
  token timestamps, preserving real repetitions rather than deleting matching words.
- Five-section Settings navigation includes prompt editing under Advanced.
  Browser tests now use isolated provider fixtures and fresh hover coordinates.
- Portable bundle layout and scripts are in `docs/portable-usage.md` and `scripts/`.
  User chose a portable validation build first; clean-machine and real-app QA remain required.
- Historical findings and validation details: `docs/release-review-2026-09-12.md`.
- Validation: 490 workspace tests, 2 explicitly enabled worker tests and 24 frontend
  tests pass. Release builds without warnings. The portable ZIP in `dist/` passed
  Settings smoke checks, real ASR sample transcription and archive hash validation.

Last refreshed after:

- `32b1cba Add web-rendered live overlay widget`
- `cb0bb2e Redesign overview with reliable usage analytics`
- `697a993 Redesign settings with live runtime updates`
- `3cd835b Add provider presets and Tencent Hunyuan support`
- `188e381 Update Tencent Hunyuan preset to TokenHub`
- `5b4ed1b Reduce first-session latency and improve diagnostics`

Use [spec.md](spec.md) for product behavior, [architecture.md](architecture.md)
for implementation boundaries, [docs/milestones.md](docs/milestones.md) for
tracking history, and [README.md](README.md) for operator-facing commands.

## Current State

VoiceFlow is a Windows-first speech input prototype centered on the Rust
`input-host`. The host owns shortcut handling, audio capture, local ASR,
routing, provider refinement, Windows insertion, settings, and local ledgers.

Current user-facing input categories are:

- Dictation
- Text Edit
- Instructed Input

Selected-text edit and Instructed Input remain explicit flows. Instructed Input
requires the configured trigger phrase at the beginning of a shortcut-initiated
recording; it is not always-on wake-word detection or broad system automation.

## Live Overlay

Settings now uses a macOS-inspired light neutral palette, inset sidebar navigation,
system typography, subtle grouped surfaces, and restrained blue controls. The voice
widget shares the typography and uses a graphite translucent capsule. No simulated
macOS window buttons are added. `apps/overlay-ui/preview.html` previews all four
visible states using the production stylesheet without microphone access.
Overview cards have consistent inner spacing and a single setup separator; the
Settings save bar shares content-column dimensions and the 980px layout breakpoint.

`apps/overlay-ui` is the single production renderer. A reusable Wry/WebView2
window hosts its HTML/CSS/JS and receives state through native script injection.
The compact bottom-center widget shows listening, thinking, done, and compact
error states in English or Chinese. Language updates are observed without a
host restart.

The host owns visibility and placement. The overlay is click-through,
non-activating, always on top, excluded from the taskbar, and positioned against
the foreground monitor work area with effective-DPI scaling. The generated
overlay runtime-state script remains development diagnostics, not a second
production renderer.

Overlay window/WebView construction remains first-state lazy. Attempts to move
WebView2 construction into startup blocked inside WebView2 initialization, so
the verified single-window production lifecycle was retained. First-use
window/WebView costs are now measured explicitly.

## Startup And Latency

Built-in normal-dictation prompts now share compact fidelity/safety rules with
separate light and structured editing instructions. Structured cleanup explicitly
uses paragraphs for narrative/explanation, bullets for parallel items, and numbers
for ordered steps, based on meaning rather than length alone. The 15/60 routing
thresholds remain fixed. Light cleanup explicitly preserves spoken enumeration
such as 第一、第二、第三 as a numbered list, uses bullets for clearly listed
unordered items, and distinguishes procedural steps from narrative transitions.
These are formatting instructions, not permission to summarize or reorder.
The user's light-mode test still returned explicit enumeration inline with commas
using the built-in prompt. Rules now require one item per line, explicitly
exempt list formatting from the no-reorganization restriction, and allow clear
parallel items without enumeration markers. A short numbered example and the
reported regression case are included. Live model compliance remains to be tested.
The
short self-correction exception, full-utterance refinement, command
guard, and file override precedence are unchanged. Prompt assembly/routing tests
verify these contracts, not model output quality; live same-input comparisons
are still needed before claiming an improvement in wording or latency.

Long recordings now speculatively recognize completed local ASR chunks during
capture (after 25 seconds), leaving the growing final chunk for release. A tail
with at least 900 ms of quiet after its existing 500 ms post-padding may also be
prefetched; any frame at the minimum VAD threshold blocks this optimization.
No new cuts or shorter chunk-length limits are introduced. Existing hard-cut
tails are excluded from this additional optimization. The
final segmentation remains authoritative: cached results are reused only for
the same session, exact sample range, and identical normalized samples. Changed
VAD boundaries or failed speculation fall back to normal final recognition.
The background task is stopped/joined before final recognition; an in-flight
worker request still obeys the existing request timeout. Full-text routing,
refinement, and single final insertion are unchanged. Cache retention is capped
at 128 chunks. `prefetched_chunk_count` is logged separately, and
`prefetched_worker_ms` reports work moved out of foreground ASR timing.
PCM16 WAV output uses a 64 KiB buffer with explicit flush/error propagation,
replacing per-sample filesystem writes. Normalization now reuses an exact,
session-scoped source prefix, recomputing interpolation/partial-frame boundaries;
changed samples or formats fall back to full normalization. The log field
`normalization_reused_ms` is the duration of reused audio, not time saved.
Verification: 481 workspace tests pass when run serially; the fake-worker
prefetch/final-tail integration and WAV benchmark also pass. An 85-second
synthetic WAV took ~62 ms buffered versus ~3.84 s with per-sample file writes,
with byte-identical output (local debug measurement, not end-to-end latency).
The user's first-round real microphone log showed 6.179 s post-recording latency
for 81.777 s audio versus 13.880 s for an earlier 85.221 s sample (different
content, not a controlled benchmark); five of six chunks were reused. The second
round log showed 3.577 s post-recording latency for 76.454 s audio, with four of
five chunks reused. Foreground audio preparation took 40 ms, tail recognition
317 ms, and full-text refinement 2.470 s. Its tail was about four seconds long,
so the improvement cannot be attributed solely to code changes. The two logged
hard cuts were from the existing long-region protection, not new tail splitting.
The build and automated tests passed, and the user accepted the latency work
for commit. Full-utterance LLM refinement remains a user constraint; output
quality/prompt tuning is a separate follow-up, not part of this change.

The persistent SenseVoice worker performs one 250 ms synthetic-silence
inference after a cold model load. It has no user-visible or cloud side effects,
and failure is nonfatal. Before reporting ready, the host also caches the CPAL
host and probes the default input device/configuration without opening an input
stream or starting microphone capture. The actual device is resolved again
when recording starts.

Diagnostics now break down process-to-ready, recording start, host/worker ASR,
prompt/payload/provider validation, and insertion timing. Local-only dictation
reports no cloud model/provider metadata; refined and fallback sessions report
the provider/model actually attempted.

Final insertion now has a presentation-only plain-text boundary. It canonicalizes
newlines, preserves blank lines, converts leading Markdown unordered-list markers
to Unicode bullets, and removes clear paired bold markers. Session summaries and
History retain the raw final model output. Multiline commits prefer the
clipboard-backed path with clipboard restoration; its direct-input fallback emits
real Enter key events between lines. Ordinary single-line text retains the fast
direct Unicode path.

The `feature/routing-insertion-integration` branch combines main
`d7394b8` with routing checkpoint `09ff4cc`: the 15/60 thresholds, short
self-correction exception, Unicode/custom triggers, and generic instructed task
forwarding are retained alongside main's newer Settings UI. A successful
clipboard paste is now committed even if clipboard restoration fails; restoration
still uses the existing retry window and emits a nonfatal, content-free warning.
This prevents a second insertion through the direct fallback after a successful
paste. Automated checks use an isolated `VOICEFLOW_SETTINGS_PATH` to avoid reading
developer model settings. The user accepted the integration for commit on 2026-09-08;
automated Outlook/Gmail smoke was not performed.

## Overview And Local Analytics

Overview uses usage-ledger schema v2 to show a seven-day inserted-text and
estimated-time-saved chart, plus:

- total dictation duration
- inserted text
- estimated time saved
- average dictation speed

Successful sessions are persisted immediately and deduplicated by stable
session key. Analytics are independent of History retention. Legacy schema-v1
records cannot be fully backfilled because they lack reliable per-session text
counts and audio durations.

Counting is mixed-language aware: each Han character, Latin word, and
contiguous number run is one unit; punctuation and whitespace are excluded.
Manual typing time uses 40 units per minute. Estimated time saved is manual
typing time minus captured audio duration, clamped at zero. Recent local content
can be privacy-masked.

## Settings Runtime

Settings is organized into General, Voice & Output, History & Privacy,
AI Provider, and Advanced. The frontend tracks dirty drafts and saved state,
including provider endpoint drafts during background status refresh.

Settings saves replace `settings.json` atomically. The live host watches the
same path, preserves the last valid configuration after an invalid reload, and
rebinds shortcut changes transactionally.

Application timing:

- Immediate: UI language, shortcut, Push to Talk / Toggle, audio cues,
  diagnostics, and History retention.
- Next session: pause sensitivity, trigger phrase / Instructed Input, and
  refinement quality.
- Next AI request: provider, model, Base URL, timeout, and API key.

Normal Settings controls do not require restarting VoiceFlow. Developer
environment-variable changes may still require a process restart.

## Providers And BYOK

Built-in providers and endpoints are:

- Alibaba Cloud Bailian:
  `https://dashscope.aliyuncs.com/compatible-mode/v1`
- Volcengine Ark: `https://ark.cn-beijing.volces.com/api/v3`
- Tencent Hunyuan / TokenHub: `https://tokenhub.tencentmaas.com/v1`
- Custom OpenAI-compatible: explicit Base URL required

Built-in presets supply the Base URL; users normally enter a model/model ID and
API key. Advanced details permits a custom endpoint override, and Reset returns
to the built-in default. Environment overrides take precedence over saved
settings, which take precedence over built-ins.

Each provider has an isolated Windows Credential Manager slot. Test Connection
uses the current effective configuration and reports sanitized categories only.
Secrets are not persisted in settings, History, usage ledgers, runtime mirrors,
logs, reports, or tracked files.

The Provider form is full width and stable across presets. Background polling
can refresh effective status without overwriting an unsaved provider draft.
Effective configuration is shown in Advanced details, one value per row.

## History Layout

History layout now gives all entries consistent horizontal insets (24px desktop,
16px narrow). Redacted selected-text edits use the same transparent list surface
and neutral divider as other entries, with secondary privacy copy instead of a
white panel and blue edge. Developer details keep their disclosure and privacy
behavior; excess spacing above the disclosure has been removed.
History Copy uses a quiet text action rather than the shared bordered settings
button, with hover/pressed surfaces, visible keyboard focus, and a larger coarse-
pointer target. Existing localized success/failure labels remain and are announced
through a polite live region.
Source diff checks pass; live visual verification is pending because the browser
connection is unavailable. The Impeccable layout guide was applied manually;
its detector could not run because the local engine binary is missing.

## Prompts

Built-in prompts and environment-selected prompt files exist for Dictation,
Text Edit, and Instructed Input. Relevant requests load the prompt file at use
time and fall back to the built-in prompt when the file is missing, unreadable,
or empty. Settings exposes safe override status diagnostics and refresh without
including prompt contents.

The connected Settings UI now edits all four prompt slots behind an explicit
risk acknowledgement. Saving is separate from general settings; restoring the
built-in prompt requires confirmation and an explicit save. Dirty drafts survive
polling and failed requests and require confirmation before closing the editor.
The token-protected, no-store endpoint reads/writes full prompts, while runtime
mirrors still expose only status. Overrides live in application-data prompts.json
(VOICEFLOW_SETTINGS_PATH uses a .prompts.json sidecar). UI overrides take priority
over environment files; reset pins the slot to the current built-in without
deleting environment files. Subsequent requests reload the override. Existing
selected-text-edit built-in wording is unchanged.

Windows audio cues now use quiet PCM sine tones with smooth attack/release
envelopes instead of Beep. Start, selected-edit start, stop, and failure remain
distinct; playback runs off the input thread and has no system-sound fallback.
Preview WAVs can be exported with the ignored export_cue_previews test.
Workspace Rust tests and build pass; browser visual verification remains pending
because the browser connection is unavailable. Settings editor interaction tests
use DOM doubles, not a live browser. The Settings Node suite has 19 passing
tests; two existing browser test files cannot load because Playwright is absent.

## Privacy And Local Data

- Settings: `%LOCALAPPDATA%\VoiceFlow Speech Input\settings.json`
- Usage ledger: `usage-ledger.json` next to the settings file
- History ledger: `history-ledger.json` next to the settings file
- Temporary reports: `%TEMP%\voiceflow-speech-input\reports`

History is local and retention is configurable. Dictation and Instructed Input
may retain final committed output. Text Edit history is redacted by default.
Instructed Input does not retain the raw trigger, instruction, or spoken source.

## Known Limitations

- Packaging and packaged Settings asset resolution are not complete.
- SenseVoice/local ASR worker and model packaging decisions remain open.
- Windows insertion still uses temporary `SendInput` and clipboard paths.
- Selected-text compatibility varies by target app.
- The in-UI Prompt Editor and model discovery are not implemented.
- Custom headers and enterprise authentication adapters are not implemented.
- There is no always-on wake-word detection or broad app/system voice command
  execution.
- A known parallel environment-variable test race still requires stronger test
  isolation.
- `input-host` provider tests can read real developer settings unless
  `VOICEFLOW_SETTINGS_PATH` is isolated for the test process.

## Next Actions

1. Design and implement the in-UI Prompt Editor without removing advanced
   file/environment override support.
2. Resolve packaged asset lookup and define packaging for the host, local ASR
   worker, models, and static UI assets.
3. Continue selected-text and insertion compatibility testing in real apps.
4. Add model discovery and evaluate enterprise/custom authentication needs.
5. Retire the parallel environment-variable test race with scoped isolation.
# Open-source preparation — 2026-09-13

- Source runtime mirrors reset to null placeholders; production writes already
  use the settings data directory instead of these source assets.
- Added MIT LICENSE, third-party inventory, pinned Python ASR requirements,
  Windows build guide, redacted heuristic audit and history-free source exporter.
- Bundled SenseVoice INT8 SHA256 matches the named upstream model exactly;
  model weights have separate FunASR model terms, not the source MIT license.
- Public binary redistribution remains pending complete dependency notices.
- Preserve local Git history; publish a reviewed source export as a fresh repo.
