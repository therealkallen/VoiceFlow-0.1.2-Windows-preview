"""Collect local Cargo and Python notices for a Windows portable release.

Run cargo metadata --locked --offline --format-version 1
--filter-platform x86_64-pc-windows-msvc into the metadata input first.
Upstream native/model notices are downloaded separately and kept in licenses/.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

def notice(path):
    name = path.name.lower()
    return any(word in name for word in ('license', 'licence', 'copying', 'copyright', 'notice'))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--metadata', required=True)
    parser.add_argument('--runtime', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--native-build', help='Configured sherpa-onnx build directory')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    out = Path(args.output).resolve()
    if out.exists():
        raise SystemExit('Refusing to overwrite an existing license bundle')
    out.mkdir(parents=True)
    metadata = json.loads(Path(args.metadata).read_text(encoding='utf-8-sig'))
    inventory, missing = [], []
    for package in metadata['packages']:
        if not package.get('source'):
            continue
        source = Path(package['manifest_path']).parent
        files = {p for p in source.rglob('*') if p.is_file() and notice(p)}
        if package.get('license_file'):
            files.add(source / package['license_file'])
        label = package['name'] + '-' + package['version']
        entry = {'name': package['name'], 'version': package['version'],
                 'license': package.get('license'), 'repository': package.get('repository'),
                 'source': package['source'], 'files': []}
        for file in sorted(files):
            if not file.is_file():
                continue
            relative = file.relative_to(source)
            target = out / 'rust' / label / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(file, target)
            entry['files'].append(str(target.relative_to(out)).replace('\\', '/'))
        supplement = root / 'licenses' / 'rust' / label
        if supplement.is_dir():
            for file in supplement.rglob('*'):
                if file.is_file():
                    target = out / 'rust' / label / file.relative_to(supplement)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(file, target)
                    entry['files'].append(target.relative_to(out).as_posix())
        if 'MPL' in (package.get('license') or ''):
            archive = source.parent.parent.parent / 'cache' / source.parent.name / (label + '.crate')
            if not archive.is_file():
                raise RuntimeError('Missing corresponding source archive: ' + label)
            target = out / 'sources' / archive.name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(archive, target)
            entry['corresponding_source'] = target.relative_to(out).as_posix()
        if not entry['files']:
            missing.append({'name': label, 'license': package.get('license')})
        inventory.append(entry)
    runtime = Path(args.runtime)
    for file in runtime.rglob('*'):
        if file.is_file() and notice(file):
            target = out / 'runtime' / file.relative_to(runtime)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(file, target)
    if (root / 'licenses').is_dir():
        shutil.copytree(root / 'licenses', out / 'upstream')
    if args.native_build:
        build = Path(args.native_build)
        cache = (build / 'CMakeCache.txt').read_text(encoding='utf-8')
        if 'SHERPA_ONNX_ENABLE_TTS:BOOL=OFF' not in cache:
            raise RuntimeError('Release runtime must be built with TTS disabled')
        native = []
        for source in sorted((build / '_deps').glob('*-src')):
            copied = []
            for file in source.rglob('*'):
                if file.is_file() and notice(file):
                    target = out / 'native' / source.name / file.relative_to(source)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(file, target)
                    copied.append(target.relative_to(out).as_posix())
            if not copied:
                raise RuntimeError('Missing native notices: ' + source.name)
            native.append({'component': source.name, 'files': copied})
        # Eigen is MPL-2.0: provide the exact, unmodified corresponding source.
        eigen = build / 'eigen-3.4.1.tar.gz'
        if not eigen.is_file():
            raise RuntimeError('Missing corresponding Eigen source archive')
        (out / 'sources').mkdir(exist_ok=True)
        shutil.copyfile(eigen, out / 'sources' / eigen.name)
        (out / 'native-dependencies.json').write_text(json.dumps(native, indent=2) + '\n', encoding='utf-8')
    (out / 'rust-dependencies.json').write_text(json.dumps(inventory, indent=2) + '\n', encoding='utf-8')
    (out / 'missing-license-files.json').write_text(json.dumps(missing, indent=2) + '\n', encoding='utf-8')
    hashes = {str(p.relative_to(out)).replace('\\', '/'): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in out.rglob('*') if p.is_file()}
    (out / 'license-hashes.json').write_text(json.dumps(hashes, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'packages': len(inventory), 'notice_files': len(hashes), 'missing': missing}, indent=2))
    if missing:
        raise SystemExit(1)

if __name__ == '__main__':
    main()
