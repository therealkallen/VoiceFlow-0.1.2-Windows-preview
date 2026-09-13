# Build from source (Windows x64 preview)

Install Rust stable with the MSVC target, Visual Studio Build Tools with Desktop
development with C++, Windows SDK, CPython 3.11 x64 (3.11.9 was tested), and
Microsoft Edge WebView2 Evergreen Runtime. Node is only needed for frontend
tests or Tauri CLI tooling, not the application at runtime.

Run commands from the repository root in PowerShell. Use an isolated environment:

```powershell
py -3.11 -m venv .venv
& ./.venv/Scripts/python.exe -m pip install -r runtime/asr/requirements.txt
```

Obtain `model.int8.onnx` and `tokens.txt` from the
[SenseVoice conversion repository](https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/tree/main)
and place both in `runtime/asr/`. Read [model licensing](../THIRD_PARTY_NOTICES.md)
first. These files are intentionally excluded from source control.

```powershell
$expectedModelHash = 'C71F0CE00BEC95B07744E116345E33D8CBBE08CEF896382CF907BF4B51A2CD51'
if ((Get-FileHash runtime/asr/model.int8.onnx -Algorithm SHA256).Hash -ne $expectedModelHash) {
    throw 'Unexpected SenseVoice model; verify the upstream source.'
}
$env:VOICEFLOW_ASR_PYTHON = (Resolve-Path .venv/Scripts/python.exe).Path
$env:VOICEFLOW_ASR_WORKER = (Resolve-Path runtime/asr/worker.py).Path
$env:VOICEFLOW_ASR_MODEL_DIR = (Resolve-Path runtime/asr).Path
& ./.venv/Scripts/python.exe scripts/verify-asr-worker.py $env:VOICEFLOW_ASR_PYTHON $env:VOICEFLOW_ASR_WORKER
cargo build --release --locked -p input-host -p voiceflow-desktop
```

The worker smoke test uses synthetic silence, no microphone or cloud request.
For development, launch `cargo run -p input-host -- --serve-live 8000 3` with
the environment above. Configure your own Provider and API key through Settings.
ASR runs locally; provider refinement sends text to the configured endpoint.
Selected-text editing may include the selection. Never commit your API keys,
history, recordings or diagnostics.

## Local desktop package

The packaging script needs the base CPython installation (not the venv root)
plus the isolated environment's site-packages directory:

```powershell
$pythonBase = & ./.venv/Scripts/python.exe -c 'import sys; print(sys.base_prefix)'
./scripts/build-portable.ps1 -PythonHome $pythonBase -SitePackages .venv/Lib/site-packages -ModelDirectory runtime/asr -Name VoiceFlow-runtime-local
./scripts/build-desktop-portable.ps1 -RuntimeBundle dist/VoiceFlow-runtime-local -Name VoiceFlow-desktop-local
```

Double-click `dist/VoiceFlow-desktop-local/VoiceFlow.exe`.
See [desktop usage](desktop-usage.md) for lifecycle and destination-machine checks.
These commands make a local test package. A public binary requires the complete
third-party license bundle described in THIRD_PARTY_NOTICES.md.

## Tests and source release

```powershell
$env:VOICEFLOW_SETTINGS_PATH = Join-Path $env:TEMP ('voiceflow-tests-' + [guid]::NewGuid() + '.json')
cargo test --workspace --release --locked -- --test-threads=1
& ./.venv/Scripts/python.exe scripts/audit-public-source.py
& ./.venv/Scripts/python.exe scripts/audit-public-source.py --history
& ./.venv/Scripts/python.exe scripts/export-public-source.py
```

The audit is heuristic and prints locations/rules, never secret values. Review
findings before publishing. The export copies the current reviewed source,
including new source files, into `dist/VoiceFlow-source-preview` plus a ZIP and
SHA256 manifest. It deliberately excludes `.git`, environment files, local
models, Python runtime and test recordings. Never upload the existing local
development Git history without separately reviewing and cleaning it.

After reviewing the export, initialize a new Git repository in a copy of that
directory outside the development repository and publish that new repository.
This preserves local development history without exposing it publicly.
