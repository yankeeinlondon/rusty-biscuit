---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-25-worktree-file/spec.md
plan: worktree/fixes/2026-09-25-worktree-file/plan.md
implemented_by: codex/default
started_phase: 1
source_files_during_phase_1:
    - worktree/fixes/2026-09-25-worktree-file/spike_git.py
    - worktree/fixes/2026-09-25-worktree-file/spike_windows.ps1
docs_updated_during_phase_1:
    - worktree/fixes/2026-09-25-worktree-file/plan.md
    - worktree/fixes/2026-09-25-worktree-file/spec.md
docs_created_during_phase_1:
    - worktree/fixes/2026-09-25-worktree-file/spikes.md
    - worktree/fixes/2026-09-25-worktree-file/implementation-log.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/windows.md
packages: []
---

# Implementation Log for 2026-09-25-worktree-file (5 phases)

## Phase 1

- Confirmed R1 and R2 from the author's spec decisions. Confirmed R3 and
  R5–R12; amended R4 to use a registration nonce and corrected R7 so
  non-UTF-8 rule bytes are allowed. No production code was changed.
- S1 target: the include candidate and standard-ignore intersection across
  Git rule forms, representations, and repository boundaries. `spike_git.py`
  asserts the selected set and the absolute fallback rules location on macOS
  and Linux. `spike_windows.ps1` asserts the NUL pipeline on Windows. The
  initially failing Windows PowerShell text-pipeline probe exposed a BOM;
  replacing that with byte streams made it pass. Directory-pattern results
  require a Phase 2 kind filter. A second Windows probe initially failed
  because Git traversed a junction and listed its child; Phase 2 must inspect
  every candidate ancestor for reparse points.
- S2 target: clone capability, mode, and independence. `spike_git.py` runs
  native clone commands: APFS succeeded and preserved `0600`; Linux's temp
  volume denied cloning. `spikes.md` records the selected crate, dependency
  cost, publication rule, and Windows caveat.
- S3 target: stale records after same-path re-registration. Both probes assert
  that Git reuses the admin directory name and removes a marker in that
  directory. A random marker is the chosen identity contract.
- S4 target: Windows link and junction handling. `spike_windows.ps1` asserts
  junction reparse attributes and reports symlink creation. The host permits
  links, so the error-1314 denial path is grounded in platform documentation
  and remains a Phase 2 injected failure case.
- No crate, CLI, parser, or persisted value was changed in Phase 1; the
  shipped-artifact corpus, end-to-end command, and read/write/read tests
  belong to the integration phases where those behaviors are implemented.
- Phase 1 targeted probes: `python3 spike_git.py` on macOS and build-linux;
  `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File
  spike_windows.ps1` on build-win-native. All passed after the probe fix.
- Broader gates: `just test` in `worktree/` passed (331/331 selected L1
  tests; 17 tier-filtered tests skipped). `just lint` passed for `worktree`
  and `worktree-cli`. No pre-existing failures were observed. `just test-l2`
  and cross-OS crate tests were not run because Phase 1 changes no package
  code or terminal behavior; the three probe hosts supplied the relevant
  platform evidence.
