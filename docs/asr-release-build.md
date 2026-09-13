# ASR-only Windows release runtime

Public portable packages use sherpa-onnx 1.12.28 built without TTS. The general
PyPI wheel includes unused eSpeak TTS code, so it is not copied into this release.
The SenseVoice model and recognizer implementation are unchanged.

Download the [upstream source archive](https://github.com/k2-fsa/sherpa-onnx/archive/refs/tags/v1.12.28.zip).
Its SHA256 is `09205DA70D684117D1EB7174934F4F8A3D3841CA019B51305091F80BBE2454A1`.
Extract under `dist/asr-build-source/`. Use CMake, Visual Studio C++ build tools,
and a full CPython 3.11 x64 installation with headers and libraries.

```powershell
$pythonExe = 'C:/path/to/Python311/python.exe'
$env:GIT_CEILING_DIRECTORIES = (Resolve-Path dist/asr-build-source).Path
cmake -S dist/asr-build-source/sherpa-onnx-1.12.28 -B dist/asr-build -A x64 `
  -DSHERPA_ONNX_ENABLE_PYTHON=ON -DPYBIND11_FINDPYTHON=ON `
  "-DPython_EXECUTABLE=$pythonExe" "-DPYTHON_EXECUTABLE=$pythonExe" `
  -DSHERPA_ONNX_ENABLE_TTS=OFF -DSHERPA_ONNX_ENABLE_SPEAKER_DIARIZATION=OFF `
  -DSHERPA_ONNX_ENABLE_PORTAUDIO=OFF -DSHERPA_ONNX_ENABLE_WEBSOCKET=OFF `
  -DSHERPA_ONNX_ENABLE_BINARY=OFF -DSHERPA_ONNX_ENABLE_C_API=OFF `
  -DSHERPA_ONNX_BUILD_C_API_EXAMPLES=OFF
cmake --build dist/asr-build --config Release --target _sherpa_onnx --parallel 4
```

Upstream CMake downloads dependencies and checks their pinned hashes. If a proxy
is required, configure it for CMake or predownload the archives into the exact
local filenames listed in upstream `cmake/*.cmake`. Do not disable TLS checks.
The verified build used Visual Studio 18, MSVC 19.51, CMake 4.3.1 and CPython
3.11.9. Default static CRT was retained. ONNX Runtime is 1.23.2.

Prepare a local runtime input using [the source build guide](build-from-source.md),
then create a fresh runtime and collect notices (replace Python command as needed):

```powershell
python scripts/prepare-asr-runtime.py --source-runtime dist/VoiceFlow-runtime-local/runtime --native-build dist/asr-build --output dist/VoiceFlow-asr-runtime
cargo metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc > .tmp-release-metadata.json
python scripts/collect-release-licenses.py --metadata .tmp-release-metadata.json --runtime dist/VoiceFlow-asr-runtime/runtime --native-build dist/asr-build --output dist/VoiceFlow-release-licenses
```

The collector also uses the reviewed upstream supplements in `licenses/` and
the local Cargo source cache. For Eigen's corresponding source, retain the
verified `eigen-3.4.1.tar.gz` in `dist/asr-build/`. The native download copies can
otherwise be found below the build's `_deps/` directories.

Build and verify the desktop package:

```powershell
./scripts/build-desktop-portable.ps1 -RuntimeBundle dist/VoiceFlow-asr-runtime -LicenseBundle dist/VoiceFlow-release-licenses -Name VoiceFlow-0.1.2-windows-x64-portable
./scripts/verify-desktop.ps1 -Bundle dist/VoiceFlow-0.1.2-windows-x64-portable
python scripts/verify-asr-worker.py dist/VoiceFlow-0.1.2-windows-x64-portable/runtime/python/python.exe dist/VoiceFlow-0.1.2-windows-x64-portable/runtime/asr/worker.py
python scripts/verify-release-archive.py dist/VoiceFlow-0.1.2-windows-x64-portable.zip
```

The Python package exposes only OfflineRecognizer. This entry-point change is
recorded in `runtime/asr-build.json`, alongside native binary hashes and flags.
No upstream native source is modified. Preserve all bundled notices.
