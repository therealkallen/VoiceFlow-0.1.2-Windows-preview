# Third-party components

The root MIT license covers VoiceFlow source. It does not relicense model
weights, dependencies, or their bundled components.

## SenseVoice model (verified 2026-09-13)

- Conversion repository: [csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17](https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17).
- Original model: [FunAudioLLM/SenseVoiceSmall](https://huggingface.co/FunAudioLLM/SenseVoiceSmall).
- File: `runtime/asr/model.int8.onnx` in the local desktop bundle.
- SHA256: `C71F0CE00BEC95B07744E116345E33D8CBBE08CEF896382CF907BF4B51A2CD51`.
- The local desktop 0.1.2 file was hashed and matches the upstream
  [file metadata](https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/blob/main/model.int8.onnx).
- The conversion repository's [LICENSE](https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/blob/main/LICENSE)
  refers to FunASR licensing. The original model card identifies a separate
  model license; see the [FunASR Model License](https://github.com/modelscope/FunASR/blob/main/MODEL_LICENSE).

The source distribution contains the worker but no weights or vocabulary.
Download model files from upstream and read the model terms before use.
A hash match establishes file identity, not blanket redistribution permission.
The Windows portable release includes the model, vocabulary and separate model
terms under `licenses/upstream/sensevoice/`. Model authors: Alibaba Group /
FunAudioLLM SenseVoice team; sherpa-onnx conversion by Fangjun Kuang
(csukuangfj). The model is not relicensed under VoiceFlow's MIT license.

## Runtime and build dependencies

| Component | Source / license information |
| --- | --- |
| CPython | [PSF license and bundled third-party licenses](https://docs.python.org/3/license.html); preserve the runtime's LICENSE.txt |
| sherpa-onnx / sherpa-onnx-core 1.12.28 | [Apache-2.0](https://github.com/k2-fsa/sherpa-onnx/blob/master/LICENSE); preserve both wheel distributions' license files |
| ONNX Runtime | [MIT and third-party notices](https://github.com/microsoft/onnxruntime/blob/main/ThirdPartyNotices.txt); included native components require their own notices |
| Tauri and Rust crates | Exact versions are in Cargo.lock; preserve each dependency's applicable license and notice files when distributing binaries |
| Tauri CLI / Node tooling | Exact versions are in apps/desktop/package-lock.json; development tooling is not needed to run the app |

## Windows portable distribution

The release includes CPython 3.11.9, ONNX Runtime 1.23.2 and a custom build of
sherpa-onnx 1.12.28 with TTS and speaker diarization disabled. No eSpeak/Piper
component is included. The only Python package change is an ASR-only entry
point; the upstream recognizer implementation is unchanged. Native hashes and
build flags are recorded in `runtime/asr-build.json`.

The package's `licenses/` directory contains full texts and copyright notices
for the resolved Rust dependency graph, native ASR build dependencies and
CPython libraries. `rust-dependencies.json` and `native-dependencies.json`
index the files. Download sources are recorded in `upstream/sources.json`.
Exact unmodified corresponding source for MPL-2.0 Rust crates and Eigen is
included in `licenses/sources/`. Those components remain under MPL-2.0; you may
obtain, modify and redistribute their source under that license. Build-only
dependency notices may also be included as a conservative superset.

CPython's Windows runtime includes Microsoft Distributable Code subject to the
additional Windows binary conditions in its included LICENSE.txt. Those
conditions apply to that code, not to VoiceFlow's MIT source. Preserve all
third-party notices when redistributing the portable package.
