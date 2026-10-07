---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-skill/fixes/2026-10-06-archive-key-helper/spec.md"
plan: "fixes/2026-10-06-archive-key-helper/plan.md"
implemented_by: "claude/sonnet"
started_phase: 1
completed_phase: 4
implemented: true
source_files_during_phase_1:
  - just/devops.just
  - scripts/ci/build_key.py
  - scripts/ci/test_build_key.py
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - .github/workflows/_package-ci.yml
  - scripts/cross-check.sh
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/os/windows.md
  - .claude/skills/os/SKILL.md
source_files_during_phase_3:
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - scripts/ci/test_cross_check.py
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - .github/ci/README.md
  - docs/topics/ci-cd.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/rust-devops/ci-cd.md
source_code:
  - just/devops.just
  - scripts/ci/build_key.py
  - scripts/ci/test_build_key.py
  - .github/workflows/_package-ci.yml
  - scripts/cross-check.sh
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - scripts/ci/test_cross_check.py
documentation:
  - .github/ci/README.md
  - docs/topics/ci-cd.md
  - .claude/skills/os/windows.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/rust-devops/ci-cd.md
packages:
  - repo-deps
  - test-toolkit
---

# Implementation Log for 2026-10-06-archive-key-helper (4 phases)

## Phase 1

- `just/devops.just` `_ci_build_verify`: emits `key_helper=${tool}` inside the
  `$GITHUB_OUTPUT` block (after the verifier ran); output doc comment lists it.
- `scripts/ci/build_key.py` `planned_keys`: helper child gets a copy of the
  environment with `BISCUIT_CI_BUILD_WRAP` removed and `RUSTC_WRAPPER=""`;
  parent env untouched; resolver unchanged. Docstring and WHY comment added.
- `scripts/ci/test_build_key.py`: added empty-override fallback, override
  precedence with a spaced path (single argv element), and subprocess-boundary
  env tests. Requirement-to-test: AC4 -> those three tests plus the existing
  invalid-override, helper-error, and pinned-digest tests.
- Gate: `python3 -m unittest scripts/ci/test_build_key.py` -> 15 tests OK.
- Rulings: all defaults taken (no human decision needed).
- Not run: `just lint` (no Rust changed; Python-only plus a justfile line).

## Phase 2

- `.github/workflows/_package-ci.yml` `gate` step only: step env
  `ARCHIVE_KEY_HELPER` from `steps.verified.outputs.key_helper`; archive branch
  requires it nonempty and exports `BISCUIT_CI_BUILD_BIN`; comment extended.
  `expected` step and native branch untouched.
- `scripts/cross-check.sh`: Unix archive script exports
  `BISCUIT_CI_BUILD_BIN="$consume/build/tools/ci-build"` after verification and
  before `just _test`; Windows script sets `$env:BISCUIT_CI_BUILD_BIN` to
  `...\tools\ci-build.exe` beside `BISCUIT_NEXTEST_BIN` (spec's backslash
  spelling, ruling 2 default). Native mode unchanged.
- OS skill: `windows.md` bullet and `SKILL.md` symptom row for BUILD_WIN
  WindowsApps Python aliases.
- Checks: `bash -n scripts/cross-check.sh` clean; workflow YAML parses;
  `python3 -m unittest scripts/ci/test_cross_check.py` 26 tests OK.
- Tests for these bindings (workflow contracts, generated-script tests) are
  Phase 3 by plan; not added here. `ci_workflow_contracts` not yet run.

## Phase 3

- `ci_workflow_contracts.rs`: `an_archive_consumer_asks_cargo_for_nothing_including_metadata`
  now pins the gate's `ARCHIVE_KEY_HELPER` binding from
  `steps.verified.outputs.key_helper`, the nonempty requirement and export
  inside the archive branch (in order, before `just "$RECIPE"`), a single
  `BISCUIT_CI_BUILD_BIN` in the gate, and none in the `expected` step. Doc
  comment names the planner key helper as a third unintended build.
  `an_archive_consumer_hands_native_programs_native_paths` now limits its text
  to `_ci_build_verify` and asserts `key_helper=${tool}` follows the verifier
  call inside the `GITHUB_OUTPUT` block.
- `test_cross_check.py`: Unix archive test (Linux/macOS/WSL hosts) asserts one
  quoted binding, after verification, before the test recipe, no `\$` leftovers;
  Windows test asserts the `.exe` assignment with the same ordering; native
  mode (Unix and Windows) has no `BISCUIT_CI_BUILD_BIN`.
- Checkpoint 3 (AC3), each restored afterward: removing the gate binding, the
  gate export, and the recipe's `key_helper=` output each failed exactly its
  owning contract (asks_cargo_for_nothing x2, native_programs_native_paths).
  A first attempt at the binding removal used a regex grep that matched
  nothing and passed; redone with a fixed-string match and it failed.
- Gates: `just test test-toolkit` 461 passed (4 skipped); `cargo clippy -p
  test-toolkit --tests` clean; `test_build_key.py` + `test_cross_check.py`
  41 tests OK. `just lint` has no recipe in `tools/test-toolkit` or at the root
  by that name, so clippy was run directly.
- Wave 4 (AC5/AC6), ruling 5: the `the_shipped_planner_*` tests no longer exist,
  so the run used `just cross-check repo-deps --os windows
  the_real_planners_plan_rolls_up`. A plain run PASSed in 0.309 s with no
  evidence the planner ran (BUILD_WIN resolves `python3` to the Store alias).
  Rerun with temporary diagnostics (a stderr/file line in `planned_keys` and
  a process-scoped `PATH` prepend of `Python313` in the Windows script):
  the helper command was `[<consume>\build\tools\ci-build.exe]`, override
  the same string, interpreter `...\Python313\python3.exe`, three planner
  calls; not Cargo, not a producer path. Test duration 4.755 s with a real
  Python (vs. 0.309 s skipped). No guard tests exist, so no 30 s mark applies.
  Diagnostics removed: `build_key.py` has no diff against HEAD (Phase 1 only);
  `cross-check.sh` carries only the Phase 2 two-line change.
- Not run: Linux/WSL cross-check (Windows was the acceptance leg; contracts
  cover Unix generation).

## Phase 4

- Docs only; no source changed. `.github/ci/README.md` ("Consumers verify,
  then run") and `docs/topics/ci-cd.md` (archive-consumer section, with a
  Mermaid flow) state that the verified `ci-build` is the planner's key helper
  (`key_helper` output, `gate` export of `BISCUIT_CI_BUILD_BIN`, `expected`
  unbound, wrapper mode stripped, `requires-toolchain` keeps its Cargo calls,
  cross-check binds it too). `.claude/skills/rust-devops/ci-cd.md` carries the
  same guidance plus the BUILD_WIN Store-alias caution.
- Drift pass: `planned_keys` docstring, `_ci_build_verify` output comment,
  gate comment, and cross-check generators match behavior; no `docs/` page
  names this fix.
- `just ci-local --plan`: only `repo-deps` and `test-toolkit` cells (plus their
  lint/check); no new scheduling rules. `just lint` has no recipe in
  `tools/test-toolkit` or at the root; clippy ran clean in Phase 3 and no code
  changed since.
- Not run: nothing else; follow-up timing comparison is post-merge.
