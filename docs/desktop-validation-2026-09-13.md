# Tauri Windows desktop validation — 2026-09-13

## Deliverable

`dist/VoiceFlow-desktop-0_1_2.zip` contains the Tauri 2.11.5 desktop application,
the Rust speech host, both UI asset directories, isolated CPython, sherpa_onnx,
SenseVoice INT8 model and token vocabulary. The user-supplied image was converted
with the official Tauri icon CLI and embedded into the EXE/window/tray.

Users launch `VoiceFlow.exe`. Closing the window hides it; relaunching restores
the same window. The tray has Settings and Quit actions. No external browser or
PowerShell launcher is required. The desktop internally retains the existing
loopback Settings bridge with no Tauri IPC capabilities granted to its page.

## Checks completed

- Release build succeeded; desktop metadata version is 0.1.2.
- `cargo test --workspace --release --locked --offline -- --test-threads=1`:
  491 passed, 5 ignored. Source runtime-state files were unchanged.
- Final desktop-only process cleanup test passed again after the final change.
- `scripts/verify-desktop.ps1` passed on the final bundle in normal Windows
  desktop permissions. The initial sandboxed WebView2 attempt timed out; it was
  not counted as a successful test.
- Startup handshake blocks an unmanaged host until the parent releases it;
  disconnecting beforehand aborts with exit code 1.
- Actual Tauri page finished loading with temporary settings/data, restricted
  PATH, invalid inherited Python/ASR paths, and a different working directory.
- Fresh desktop data did not load old diagnostics from the development TEMP
  directory. Backend TEMP/TMP are isolated below the desktop data directory.
- The Settings server stopped after desktop exit; bundle hashes stayed unchanged.
- Native UI inspection confirmed the icon, rendered Settings content, close-to-
  tray behavior, and reopening the same window/PID on a second launch.
- Terminating that test desktop process also removed its background Settings
  process. A Windows Job Object unit test separately verified process cleanup.
- Final bundled Python/worker/model transcribed the existing Chinese fixture:
  41 tokens and 41 timestamps. No microphone or cloud Provider was used.

Desktop startup pins Python, worker and model-directory overrides to its own
package. Backend processes wait for Job Object assignment before beginning work.
Normal exit closes their parent pipe, allows 3 seconds for orderly shutdown,
then terminates any remaining descendants. Finish recording before quitting.

## Remaining acceptance checks

No clean Windows VM or company-managed machine was available for this run.
Microphone capture, actual target-app insertion/selected-text editing, long
dictation, company network access and IT application-control approval still need
testing on the destination machine. Existing backend tests are not a substitute
for those checks.

This is an unsigned portable desktop test build, not an NSIS/MSI installer.
WebView2 Evergreen Runtime is a system prerequisite and is not bundled or
automatically installed. Startup failures surface a native error dialog with
the log location and WebView2 guidance. Portable packaging does not bypass
Windows application-control rules.

The accompanying `.zip.sha256` records the archive hash. The ZIP contains a
per-file SHA-256 manifest. Personal runtime mirrors are replaced by null
placeholders, and credentials/history/development environment files are excluded.

Final archive: 185,217,295 bytes. All 1,062 manifest file hashes matched the ZIP;
additional entries were directory records only. Runtime placeholders and absence
of private files/legacy launchers were checked.

SHA-256: `6D5C984A5C88223FB960BA4717E07F151D4C23D28DE1159A1C9D86728A5E5410`.
