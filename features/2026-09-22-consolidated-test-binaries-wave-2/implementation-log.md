---
spec: /Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
plan: features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.sh
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-probe.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-input-listings.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/consumer-sweep.py
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan.py
docs_updated_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/plan.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spec.md
docs_created_during_phase_1:
    - features/2026-09-22-consolidated-test-binaries-wave-2/rulings.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s1-feature-sets.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-hazards.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan-output.txt
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s3-cross-check.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s4-deps.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/pre-existing.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/skip-baseline.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-inputs.md
    - features/2026-09-22-consolidated-test-binaries-wave-2/baseline/test-selector-consumers.md
    - darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/build-hosts.md
packages: []
---

# Implementation Log for 2026-09-22-consolidated-test-binaries-wave-2 (6 phases)

## Phase 1

Phase 1 (rulings, spikes, baselines) ran at `208051f75`. No production source
changed. The spec `status` moved `draft-spec → planned`. Nothing was
committed, per the phase instructions.

### What was produced

| Plan task | Output | Result |
|---|---|---|
| Record rulings | `rulings.md` | R1–R13 decided, plus new R14–R19 from Phase 1 evidence |
| S1 | `spikes/s1-feature-sets.md` | Census matches the plan: 236 workspace targets, 136 in scope. `dmls` needs a 4th feature set, `(terminal-tests)`. `claudine` is `()`. |
| S2 | `spikes/s2-hazards.md`, `s2-scan.py`, `s2-scan-output.txt` | 144 files scanned with `consolidation.py`'s detectors. Zero crate-global constructs or crate-only inner attributes. 95 path repairs, 2 self-exec identities, 3 `crate::` false positives, a proptest relocation hazard, and a cross-package guard. Done by a subagent; I verified the proptest and guard claims independently. |
| S3 | `spikes/s3-cross-check.md` | `cross-check` ships the working tree (not `HEAD`) and builds the CI feature union, so `windows_captured_stdout` is compiled. Per-test PASS lines appear live only. |
| S4 | `spikes/s4-deps.md` | No cycle. Five packages need an unconditional `test-toolkit` dev-dependency, not two. The added crates are all already in `Cargo.lock`. |
| Pre-existing state | `baseline/pre-existing.md`, `pre-existing.sh`, `pre-existing-logs/` | 8 areas × `test`, `test-l2`, `lint`, `check-tier-coverage`, plus `check-canonical`: all green except 2 environment-induced `claudine-cli` failures. 0 stranded tests. |
| Skip baseline | `baseline/skip-baseline.md` | `ci-baseline.toml` has no entries at all |
| Test-input probes | `baseline/test-inputs.md`, `test-input-probe.py`, `test-input-listings.py`, `*-before.json` | 9 probes, 1 per package (`biscuit-tui-cli` has 0 references). Identities are confirmed derived from the `mod` walk. |
| Consumer sweep | `baseline/test-selector-consumers.md`, `consumer-sweep.py` | 27 active selector hits and 41 file-path references. One is load-bearing and crosses packages (R17). Done by a subagent. |

### Findings that changed the plan (written into the plan's table and F9–F14)

- **F9 / R14:** all four `sniff-cli` `level2_*` files are
  `#![cfg(feature = "test-fixtures")]`, including
  `level2_recent_commits_rendering`. So spec hazard 3's "no features"
  contract is empty without the feature. `sniff-cli` becomes 2 targets, and
  the in-scope total is 18. The author may restore the split before Phase 5.
- **F10 / R7:** `schematic-gen`, `biscuit-terminal-cli`, and `claudine-gen`
  have `test-toolkit` only as an optional dependency, so their feature-less
  `l1` layout gate needs an extra unconditional dev-dependency.
- **F11 / R18:** proptest 1.11 `SourceParallel` resolves to
  `tests/proptest-regressions/<stem>.txt` once `tests/<target>/main.rs`
  exists. I verified this from `proptest-1.11.0/src/test_runner/failure_persistence/file.rs:77-81`
  and `:336-367`. **This is a wave-1 defect:** darkmatter's 5 seeds under
  `darkmatter/lib/tests/l1/*.proptest-regressions` are no longer replayed,
  and `darkmatter/lib/tests/proptest-regressions/` does not exist. Filed as
  `darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md`.
- **F12 / R17:** `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053`
  reads `windows_captured_stdout.rs` by path, and `:7067` asserts its inner
  `#![cfg(windows)]`. R4's disposition is therefore an outer cfg on the `mod`
  line **and** the inner attribute kept in the file.
- **F13 / R15:** `test_inputs.py`'s `NON_L1_BINARIES` never narrows into a
  `level2*` binary. That already applies to F3's old targets, and
  `biscuit-tui-cli` has no references, so nothing new is hidden. No tool
  change.
- **F14 / R13, R16:** `cross-check` ships the working tree. Remote legs must
  finish before the next package's structural edit starts. Windows evidence
  is `tee`d PASS lines, and `--features` must never be passed.

### Pre-existing failures and gaps (not attributable to the migration)

- `claudine-cli::l1 shipped_prompt_route_drift::{fixture_body_matches_the_shipped_body, shipped_implement_prompts_have_not_drifted_from_their_fixture}`
  fail because this worktree carries the author's uncommitted
  `prompts/_implement/*.md` edits. `claudine-cli` is outside the ten. With
  `--no-fail-fast`, 7316 passed and 2 failed.
- F3's 49 L1-tier tests in `biscuit-terminal-cli` run in no local recipe;
  they run only in CI's L1 cell. F5's `windows_captured_stdout` runs only on
  Windows with `terminal-tests`. Neither counts as "stranded" under
  `check-tier-coverage`. Both must be preserved exactly.

### Other things noticed (out of scope, not acted on)

- `claudine/cli/claudine/cli/tests/snapshots/` is a tracked, doubly nested
  directory holding two `wrap_commands__*.snap` files. It looks like a stray
  insta write. It is outside the ten packages.
- The `os` skill's `build-hosts.md` said `cross-check` routes feature flags
  into the archive build. The code does the opposite: any build flag switches
  every host to native mode. **Drift was detected, the code was taken as
  correct, and the skill was corrected.**

### Gates run in this phase

- `just test`, `just test-l2`, `just lint`, and `just check-tier-coverage <area>`
  for all eight areas, plus `just check-canonical` over the eight. Results are
  in `baseline/pre-existing.md`. This phase changed no Rust, justfile, or
  manifest, so these runs are both the baseline and this phase's own gate.
- `python3 -m py_compile` over the five new evidence scripts.
- The three frontmatters (spec, plan, log) parse as YAML.
- There is no requirement-to-test mapping, because Phase 1 changes no
  behavior. Its outputs are evidence documents, and each claim cites file
  and line or a committed log.
