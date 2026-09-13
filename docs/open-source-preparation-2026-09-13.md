# Source publication preparation — 2026-09-13

## Completed

- Replaced both source runtime-state mirrors with null placeholders. Current
  production code writes live mirrors under the settings data directory.
- Added root MIT license consistent with existing workspace metadata; desktop
  now inherits the same license field.
- Added upstream model identity and separate license references, Python ASR
  dependency pins, a build guide and a source-check GitHub Actions workflow.
- Added a heuristic audit that reports only locations and rule names; it also
  compares credential values from a local `.env.local` when available.
- Added a fail-closed source exporter with a file manifest and ZIP SHA256.
  It copies the current source without local `.git` history or runtime assets.
- Local release workspace tests passed; 20 Node model/localization tests passed.
  These do not include the browser UI tests or clean-machine microphone checks.
- Both packaging scripts now copy project LICENSE and THIRD_PARTY_NOTICES.md.
  This does not complete the binary dependency license bundle.

## History findings

The initial all-refs scan inspected 410 unique file blobs from 52 reachable
commits and found four nonempty runtime mirror blobs. No common provider key,
GitHub token, AWS key ID or private-key signature was detected by those rules.
Runtime snapshots can contain user data and must not be treated as safe simply
because they contain no recognized key format. The audit is not a comprehensive
secret detector and does not inspect unreachable objects or external accounts.

Local history was preserved. Publish the reviewed export as a new repository;
do not push local branches, tags or stashes from the development repository.

## Model evidence

The model in the desktop 0.1.2 runtime hashes to
`C71F0CE00BEC95B07744E116345E33D8CBBE08CEF896382CF907BF4B51A2CD51`.
This matches upstream file metadata. See THIRD_PARTY_NOTICES.md for source links.
The conversion's LICENSE points to FunASR, whose model terms are distinct from
its source code license. No weights or vocabulary are included in the source ZIP.

## Before public distribution

- Review the source ZIP and choose the target GitHub account/repository.
- For a binary Release, assemble the full model license and exact dependency
  notices (including native transitive dependencies) and validate the final ZIP.
- Verify microphone, text insertion, selected-text editing and long dictation
  on a clean Windows machine. Existing desktop test ZIPs remain local test builds.

No remote repository was created, no files uploaded, and no old Git history
rewritten during preparation.
