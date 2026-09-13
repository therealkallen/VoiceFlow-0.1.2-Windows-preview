"""Make a fresh portable runtime from CPython/model inputs and an ASR-only build."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source-runtime', required=True)
parser.add_argument('--native-build', required=True)
parser.add_argument('--output', required=True)
args = parser.parse_args()
source, build, output = map(Path, (args.source_runtime, args.native_build, args.output))
if output.exists():
    raise SystemExit('Refusing to overwrite an existing runtime')
cache = (build / 'CMakeCache.txt').read_text(encoding='utf-8')
if 'SHERPA_ONNX_ENABLE_TTS:BOOL=OFF' not in cache:
    raise SystemExit('TTS must be disabled in the native build')
extensions = list(build.rglob('_sherpa_onnx*.pyd'))
if len(extensions) != 1:
    raise SystemExit('Expected one built Python extension')
libraries = list((build / 'bin' / 'Release').glob('*.dll'))
if not libraries:
    raise SystemExit('Missing native DLLs')
for file in extensions + libraries:
    data = file.read_bytes()
    if b'espeak_Initialize' in data or b'espeak-ng' in data:
        raise SystemExit('Unexpected eSpeak code in ' + file.name)
for file in source.rglob('*'):
    if not file.is_file():
        continue
    relative = file.relative_to(source)
    if relative.as_posix().startswith('python/Lib/site-packages/') or '__pycache__' in relative.parts:
        continue
    target = output / 'runtime' / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(file, target)
package = output / 'runtime/python/Lib/site-packages/sherpa_onnx'
(package / 'lib').mkdir(parents=True)
shutil.copyfile(source / 'python/Lib/site-packages/sherpa_onnx/offline_recognizer.py', package / 'offline_recognizer.py')
(package / '__init__.py').write_text('# VoiceFlow ASR-only package entry point.\nfrom .offline_recognizer import OfflineRecognizer\n__version__ = "1.12.28"\n', encoding='utf-8')
(package / 'lib/__init__.py').write_text('', encoding='utf-8')
for file in extensions + libraries:
    shutil.copyfile(file, package / 'lib' / file.name)
provenance = {'sherpa_onnx_version': '1.12.28', 'onnxruntime_version': '1.23.2',
              'source_url': 'https://github.com/k2-fsa/sherpa-onnx/tree/v1.12.28',
              'source_archive_sha256': '09205da70d684117d1eb7174934f4f8a3d3841ca019b51305091f80bbe2454a1',
              'tts_enabled': False, 'speaker_diarization_enabled': False,
              'package_change': 'ASR-only Python entry point; upstream recognizer and native source unchanged.',
              'native_sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in extensions + libraries}}
(output / 'runtime/asr-build.json').write_text(json.dumps(provenance, indent=2) + '\n', encoding='utf-8')
print('Prepared ASR-only runtime:', output)
