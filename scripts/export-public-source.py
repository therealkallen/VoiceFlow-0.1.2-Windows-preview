"""Export reviewed source without local Git history, runtime data or models."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
ROOT_FILES = {".gitignore", "Cargo.toml", "Cargo.lock", "README.md", "LICENSE",
              "THIRD_PARTY_NOTICES.md", "architecture.md", "spec.md", "snapshot.md"}
PREFIXES = ("crates/", "apps/", "scripts/", "docs/", ".github/", "licenses/")
RUNTIME = {"runtime/asr/worker.py", "runtime/asr/requirements.txt"}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", default="VoiceFlow-source-preview")
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_-]+", args.name):
        parser.error("name must contain only letters, numbers, hyphens and underscores")
    # Audit before creating anything. Nonzero means fail closed.
    subprocess.run([__import__("sys").executable, str(ROOT / "scripts/audit-public-source.py")],
                   cwd=ROOT, check=True)
    out = ROOT / "dist" / args.name
    archive = out.with_suffix(".zip")
    if out.exists() or archive.exists():
        parser.error("output already exists; choose a new name")
    paths = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT).decode("utf-8").split("\0")
    selected = sorted(p for p in set(paths) if p and (
        p in ROOT_FILES or p in RUNTIME or p.startswith(PREFIXES)))
    for relative in selected:
        source = ROOT / relative
        if source.is_symlink():
            raise RuntimeError("Refusing symbolic link: " + relative)
        if not source.is_file():
            raise RuntimeError("Missing source: " + relative)
        if any(part in {".git", "node_modules", "target", "__pycache__"}
               for part in Path(relative).parts):
            raise RuntimeError("Unexpected generated path: " + relative)
        if source.suffix.lower() in {".onnx", ".wav", ".mp3", ".exe", ".dll", ".pyc"}:
            raise RuntimeError("Unexpected binary runtime: " + relative)
    out.mkdir(parents=True)
    for relative in selected:
        destination = out / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / relative, destination)
    manifest = {p: hashlib.sha256((out / p).read_bytes()).hexdigest() for p in selected}
    (out / "source-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED) as bundle:
        for path in sorted(out.rglob("*")):
            if path.is_file():
                bundle.write(path, str(Path(out.name) / path.relative_to(out)))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(".zip.sha256").write_text(digest + "  " + archive.name + "\n", encoding="ascii")
    print(json.dumps({"files": len(selected), "directory": str(out),
                      "archive": str(archive), "sha256": digest}, indent=2))

if __name__ == "__main__":
    main()
