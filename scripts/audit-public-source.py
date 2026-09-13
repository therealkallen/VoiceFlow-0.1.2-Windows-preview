"""Read-only heuristic audit. Reports locations/rules, never matched secrets.

Use --history to inspect unique blobs reachable from all local Git refs.
This is a release check, not a guarantee that arbitrary secrets are absent.
"""
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
RULES = {
    "private-key": rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
    "provider-key": rb"\bsk-[A-Za-z0-9_-]{24,}",
    "github-token": rb"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})",
    "aws-key-id": rb"\bAKIA[A-Z0-9]{16}\b",
}
# Compare known local credentials without logging their names or values.
# This file is read locally only and is never included in exports.
env_file = ROOT / ".env.local"
if env_file.is_file():
    for line in env_file.read_text(encoding="utf-8-sig").splitlines():
        key, separator, value = line.partition("=")
        if separator and re.search(r"KEY|TOKEN|SECRET|PASSWORD", key.upper()):
            value = value.strip().strip("\"'")
            if len(value) >= 16:
                RULES["known-local-credential-" + str(len(RULES))] = re.escape(value.encode("utf-8"))

def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT)

def inspect(label, data):
    findings = []
    basename = label.replace("\\", "/").rsplit("/", 1)[-1]
    if basename == ".env" or (basename.startswith(".env.") and basename != ".env.example"):
        findings.append({"location": label, "rule": "environment-file"})
    for rule, pattern in RULES.items():
        if re.search(pattern, data):
            findings.append({"location": label, "rule": rule})
    if label.endswith("runtime-state.js") and not re.fullmatch(
        rb"\s*window\.__VOICEFLOW_(?:SETTINGS|OVERLAY)_RUNTIME__ = null;\s*",
        data.removeprefix(b"\xef\xbb\xbf"),
    ):
        findings.append({"location": label, "rule": "nonempty-runtime-mirror"})
    return findings

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--history", action="store_true")
    args = parser.parse_args()
    findings = []
    count = 0
    if args.history:
        objects = git("rev-list", "--objects", "--all").decode("utf-8").splitlines()
        for entry in objects:
            oid, _, path = entry.partition(" ")
            if not path or git("cat-file", "-t", oid).strip() != b"blob":
                continue
            count += 1
            findings.extend(inspect(oid[:12] + ":" + path, git("cat-file", "blob", oid)))
    else:
        paths = git("ls-files", "--cached", "--others", "--exclude-standard", "-z")
        for path in sorted(set(paths.decode("utf-8").split("\0")) - {""}):
            file = ROOT / path
            if file.is_file():
                count += 1
                findings.extend(inspect(path, file.read_bytes()))
    print(json.dumps({"mode": "history" if args.history else "worktree",
                      "files_or_blobs": count, "findings": findings}, indent=2))
    return bool(findings)

if __name__ == "__main__":
    raise SystemExit(main())
