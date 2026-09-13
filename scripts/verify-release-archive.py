"""Verify the final portable ZIP, including licenses, model and private-file exclusions."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
import zipfile

archive = Path(sys.argv[1])
with zipfile.ZipFile(archive) as bundle:
    names = [n for n in bundle.namelist() if not n.endswith('/')]
    roots = {n.split('/')[0] for n in names}
    assert len(roots) == 1, 'Expected one top-level directory'
    prefix = next(iter(roots)) + '/'
    paths = {n.removeprefix(prefix) for n in names}
    assert all('..' not in PurePosixPath(p).parts for p in paths), 'Unsafe archive path'
    def read(path): return bundle.read(prefix + path)
    manifest = json.loads(read('manifest.json').decode('utf-8-sig'))
    expected = {e['path'].replace('\\', '/'): e['sha256'].lower() for e in manifest}
    assert paths == set(expected) | {'manifest.json'}, 'Manifest coverage mismatch'
    for path, digest in expected.items():
        assert hashlib.sha256(read(path)).hexdigest() == digest, 'Hash mismatch: ' + path
        assert not re.search(r'(^|/)(\.git|\.env(?:\.[^/]*)?|credentials[^/]*|history-ledger\.json|usage-ledger\.json)(/|$)|\.(log|wav|pdb)$', path, re.I), 'Private/debug file: ' + path
    for app, name in [('settings-ui', 'SETTINGS'), ('overlay-ui', 'OVERLAY')]:
        assert read('apps/' + app + '/src/runtime-state.js').decode('utf-8-sig').strip() == 'window.__VOICEFLOW_' + name + '_RUNTIME__ = null;'
    assert hashlib.sha256(read('runtime/asr/model.int8.onnx')).hexdigest() == 'c71f0ce00bec95b07744e116345e33d8cbbe08cef896382cf907bf4b51a2cd51'
    assert len(read('runtime/asr/tokens.txt')) > 0
    assert read('licenses/upstream/sensevoice/FunASR-MODEL-LICENSE.txt')
    assert not json.loads(read('licenses/missing-license-files.json'))
    provenance = json.loads(read('runtime/asr-build.json'))
    assert provenance['tts_enabled'] is False
    for name, digest in provenance['native_sha256'].items():
        data = read('runtime/python/Lib/site-packages/sherpa_onnx/lib/' + name)
        assert hashlib.sha256(data).hexdigest() == digest
        assert b'espeak_Initialize' not in data and b'espeak-ng' not in data
    print(json.dumps({'files_verified': len(expected), 'model_verified': True, 'licenses_present': True, 'runtime_placeholders': True, 'asr_only_runtime': True}))
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
assert archive.with_suffix('.zip.sha256').read_text().split()[0].lower() == digest
print('Archive SHA256:', digest)
