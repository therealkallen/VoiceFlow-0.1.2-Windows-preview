# VoiceFlow portable test build

Windows x64; Microsoft Edge WebView2 Runtime is required for the overlay.

1. Extract the whole folder. Keep `apps` and `runtime` beside `input-host.exe`.
2. Double-click `Start-VoiceFlow.cmd`. It starts the local Settings server and speech host and opens Settings in your browser.
3. Configure your shortcut and provider in Settings. Python, the local ASR worker and model are bundled; no separate Python install is needed.
4. Double-click `Stop-VoiceFlow.cmd` before moving or replacing this folder. Finish recording first.

Settings, history, prompt overrides, generated runtime state and launcher logs are in `%LOCALAPPDATA%\VoiceFlow Speech Input`. API keys use Windows Credential Manager. A portable application folder does not imply that private user data travels inside the folder.

Do not run the development host and this bundle at the same time: they share settings and may compete for the shortcut. The stop script only stops this bundle's processes.

If the host fails to start, inspect `live-host-errors.log` and `settings-server-errors.log` in the user data directory. The launcher does not install WebView2 or register startup entries.

This is a local validation build, not a signed public installer. Validate microphone capture, long dictation, multiline paste, and selected-text edit in your actual applications before distribution. The ZIP must never include `.env.local`, saved credentials, history, generated runtime snapshots or diagnostic reports from the development machine.

## Developer build

Run `cargo build -p input-host --release --offline`, then `scripts/build-portable.ps1` with `-PythonHome` (matching CPython installation), `-SitePackages` (containing sherpa_onnx), and `-ModelDirectory` (containing model.int8.onnx and tokens.txt). The builder creates a fresh folder in `dist`, copies an explicit runtime/asset allowlist, and writes a SHA-256 manifest. It never copies a virtualenv wholesale or uses its absolute `pyvenv.cfg` paths.

For development outside the bundle, set `VOICEFLOW_ASR_PYTHON`, `VOICEFLOW_ASR_WORKER` and optionally `VOICEFLOW_ASR_MODEL_DIR`. Use the repository's `runtime/asr/worker.py`; the updated host requires token timestamps for overlapping long-audio chunks.
