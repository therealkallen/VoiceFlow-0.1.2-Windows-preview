# VoiceFlow Product Spec

VoiceFlow is a Windows-first speech input method prototype. Voice should feel
like another way of typing: the user stays in the app they are already using,
starts VoiceFlow with a global shortcut, speaks, and receives clean text in the
active text field.

## Product Principles

1. Input-first, not window-first.
2. App-in-place, not app-switching.
3. Plain dictation is literal unless the user explicitly invokes another mode.
4. Selected-text edit and Instructed Dictation are separate flows.
5. Provider use and diagnostics should be visible without exposing secrets.
6. If refinement fails after ASR succeeds, preserve usable ASR text.

## Core User Flows

### Dictation

1. User focuses a text field in any target app.
2. User starts VoiceFlow with the configured shortcut.
3. VoiceFlow records audio and runs local ASR.
4. Very short, simple text can receive deterministic local punctuation cleanup
   and skip provider refinement.
5. Structurally complex or longer text can be refined through the configured
   provider.
6. VoiceFlow inserts the final text into the active app.

Without an explicit trigger phrase at the beginning of the raw transcript,
VoiceFlow must not run broad intent detection on normal dictation.

Normal dictation uses text-unit thresholds: up to 15 is local cleanup, 16–60
is light provider cleanup, and over 60 is structured provider cleanup. Units
count Chinese characters, English words and number groups. Self-corrections
use light cleanup even for short input. Instructed dictation and selected-text
edit remain separate routes; merely mentioning a command does not execute it.

Multiline insertion must not fall back to synthetic Enter keys when clipboard
paste is unavailable, because Enter can send a message or execute a command.

### Selected-Text Edit

1. User selects existing text in the target app.
2. User triggers VoiceFlow and speaks an edit instruction.
3. VoiceFlow applies the instruction to the selected text.
4. VoiceFlow replaces the selected text with the result.

Selected-text edit applies to already selected text. It is not the same as
Instructed Dictation.

Current MVP behavior:

- deterministic transforms are still available
- freeform provider-backed selected-text edit exists
- selected-text history entries are redacted by default
- selected-text capture/replacement uses a temporary clipboard-backed path

### Instructed Dictation

Instructed Dictation means trigger phrase + spoken instruction + spoken content
in one recording.

Example:

```text
Hey VoiceFlow，把下面内容翻译成英文：明天我们确认时间线。
```

Expected behavior:

1. Detect that the raw ASR transcript starts with the configured trigger phrase.
2. Remove the trigger phrase and immediate punctuation/spacing.
3. Split the remaining text into instruction and content.
4. Send instruction and content to the provider.
5. Insert only the transformed result.

It is not always-on wake word detection. It is not broad app/system command
execution.

Trigger matching must be prefix-only and tolerate common ASR variants:

- `Hey VoiceFlow`
- `Voice Flow`
- `hey voiceflow`
- `hey voice flow`
- `hey, voice flow`
- `Hey Voice Flow`

Trigger phrases in the middle of an utterance must not activate Instructed
Dictation.

No-punctuation Chinese translation commands should parse when ASR removes the
colon, for example:

```text
hey voice flow 把下面这段话翻译成英文我认为现在接下这个任务不是一个明智的选择
```

If the trigger matches but no instruction/content split is found, VoiceFlow
must fail softly and must not insert the raw command or run normal dictation
refinement on the command.

## Settings Requirements

VoiceFlow settings should include:

- shortcut
- input mode: push-to-talk or toggle
- AI quality
- audio cues
- pause/silence sensitivity
- system language: English or Chinese
- UI style: Dark or Light
- trigger phrase enabled/phrase for Instructed Dictation
- diagnostics verbosity
- history retention: Latest 100 entries, Latest 500 entries, Latest 1000
  entries, Last 7 days, Last 30 days, or Unlimited
- provider preset: Alibaba Cloud Bailian, Volcengine Ark, Tencent Hunyuan, or
  Custom OpenAI-compatible
- provider Base URL, active model, and request timeout

Defaults:

- system language: English
- UI style: Dark
- default trigger phrase: `Voice Flow`
- history retention: Latest 100 entries

Wake phrase or trigger phrase controls must not imply always-on wake word
detection. They are for explicit shortcut-held Instructed Dictation.

Settings navigation is ordered as General, Voice & Output, History &
Privacy, AI Provider, and Advanced. Dirty state and saved state must reflect
the actual persisted draft. Background effective-status refreshes must not
overwrite unsaved provider drafts.

Normal Settings changes must not require restarting VoiceFlow. UI language,
shortcut, Push to Talk / Toggle, audio cues, diagnostics, and History retention
apply immediately. Pause sensitivity, trigger phrase / Instructed Input, and
refinement quality apply on the next session. Provider, model, Base URL,
timeout, and API key apply on the next AI request. Developer environment
variables may still require a process restart.

## Overlay Requirements

The production overlay is the HTML/CSS/JS renderer in `apps/overlay-ui`, hosted
by one reusable Wry/WebView2 window. It appears as a compact bottom-center
widget only during active voice input and presents listening, thinking, done,
and compact error states.

It must follow the effective English/Chinese UI language without a host restart,
remain click-through and non-activating, stay above normal windows without a
taskbar entry, and use the foreground monitor work area with DPI-aware
placement. There must not be a second native-drawn presentation path.

## Overview Requirements

Overview uses usage-ledger schema v2 and shows:

- A seven-day inserted-text and estimated-time-saved chart.
- Total dictation duration.
- Inserted text.
- Estimated time saved.
- Average dictation speed.
- Recent locally stored content with a privacy-mask control.
- The three user-facing categories Dictation, Text Edit, and Instructed Input.

One Han character, one Latin word, or one contiguous number run counts as one
inserted-text unit. Punctuation and whitespace do not count. Manual typing time
uses a baseline of 40 units per minute. Estimated time saved is manual typing
time minus captured audio duration, clamped at zero. Analytics persist after
each successful session and remain independent of History retention. Legacy
schema-v1 analytics cannot be fully backfilled.

## Provider And Model Requirements

The provider path supports these built-in or explicit presets:

- Alibaba Cloud Bailian:
  `https://dashscope.aliyuncs.com/compatible-mode/v1`
- Volcengine Ark: `https://ark.cn-beijing.volces.com/api/v3`
- Tencent Hunyuan / TokenHub: `https://tokenhub.tencentmaas.com/v1`
- Custom OpenAI-compatible, which requires an explicit Base URL

Supported behavior:

- provider-backed dictation refinement
- provider-backed selected-text edit
- provider-backed Instructed Dictation transform
- saved non-secret provider preset, Base URL, active model, and request timeout
- environment overrides taking precedence over saved settings
- saved settings taking precedence over built-in defaults
- per-provider API-key storage in Windows Credential Manager
- legacy `DASHSCOPE_API_KEY` support for Bailian
- generic `VOICEFLOW_PROVIDER_API_KEY` developer override
- provider Test Connection using the current effective saved configuration
- prompt-file overrides

Built-in providers normally require a model/model ID and API key. Their Base
URL comes from the preset, with an Advanced custom endpoint override and Reset
to built-in default. Environment overrides remain highest priority. API keys
use isolated Credential Manager slots per provider. The UI may display only
sanitized connection-error categories and must not surface raw provider error
payloads.

The Provider form remains stable and full width across presets. The model field
must not change layout between providers. Effective configuration belongs in
Advanced details, one full-width value per row.

Provider prompts for Instructed Dictation must instruct the model to return only
the transformed output and not include labels such as `Draft:` or `Next step:`.

If provider configuration is missing for Instructed Dictation, VoiceFlow should
fail clearly rather than insert a poor deterministic fallback.

Built-in prompts and file-based overrides currently exist for Dictation, Text
Edit, and Instructed Input. Prompt files are loaded for each relevant request,
and Settings exposes safe override status diagnostics. The planned in-UI Prompt
Editor will edit these three categories, store user overrides under application
data, restore built-in defaults, apply without restart, and retain the advanced
environment/file override path. The editor is not implemented yet.

## Privacy Requirements

- Do not store provider API keys in project docs.
- Do not copy `.env.local` values into docs.
- Do not store provider API keys in `settings.json`, runtime-state mirrors,
  History, reports, logs, diagnostics, or tracked files.
- History is stored locally on this device.
- Usage ledger stores aggregate local usage, not transcripts.
- Secrets must not be stored in usage ledgers, History, settings, runtime-state
  mirrors, logs, reports, or provider Test Connection output.
- Dictation history may store final committed output.
- Selected-text edit history is redacted by default.
- Instructed Dictation history stores final transformed output, not raw trigger,
  instruction, or spoken content.
- Time-based history retention uses per-session completion timestamps.
- History dedup metadata should be bounded and pruned with retention.
- Reports and runtime summaries should avoid raw selected text and raw
  instructed-dictation command payloads by default.

## Diagnostics Requirements

Diagnostics should expose:

- session kind
- route decision
- ASR/refine provider metadata
- provider attempted/succeeded/fallback metadata
- commit transport
- insertion failures and foreground app metadata
- sanitized Instructed Dictation parser outcome and character counts
- selected-text compatibility status
- post-recording, ASR, refinement, provider-request, and insertion timing
- process-to-ready settings/audio/speech preparation timing
- recording-start overlay publication, first-use renderer creation, audio
  device discovery, and capture-start timing
- host-side ASR audio preparation, WAV write, worker roundtrip, and cleanup
- prompt loading, request payload construction, and provider-output validation
- selected-text probe mode/result/reason/timing
- short-dictation fast-path usage/reason and cloud-skip state

Local-only fast-path diagnostics must not attribute a configured cloud provider
or model to the session. Cloud success and fallback diagnostics should identify
the provider/model actually attempted. New timing diagnostics must not include
prompt text, transcript content by default, credentials, authorization headers,
or provider responses.

Technical language is acceptable in Diagnostics. Overview and Settings should
use product-facing copy.

## Out Of Scope

- Packaging completion in the current checkpoint.
- Always-on wake word detection.
- Broad app/system voice commands.
- Destructive automation.
- TSF/TIP-class Windows input-method integration.
- macOS/Linux support.

## Current Known Limitations

- Windows insertion uses temporary `SendInput` and clipboard fallback paths.
- Packaged Settings asset resolution remains packaging backlog.
- Model discovery is not implemented.
- Custom provider headers and enterprise authentication adapters are not
  implemented.
- An in-UI Prompt Editor is not implemented.
- Selected-text behavior varies by target app.
- The ASR worker and model files still need packaging treatment.
- Packaging is not ready.
- An existing parallel environment-variable test race remains known technical
  debt.
