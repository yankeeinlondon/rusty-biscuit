---
kind: plan
name: archive-key-helper
spec: 2026-10-06-archive-key-helper
total_phases: 4
created: 2026-10-06
phase: 3
agent: claude/sonnet
yolo: true
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
packages:
  - repo-deps
  - test-toolkit
---

# Plan: archive consumers reuse the shipped build-key helper

## Summary and definition of done

**The work.** An archive consumer (CI `gate` step, and `just cross-check`)
runs planner tests in a checkout with no `ci-build` outside the artifact, so
`scripts/ci/build_key.py` falls back to `cargo run --bin ci-build` and pays a
cold compile. The consumer already holds the verifier at
`<build>/tools/ci-build[.exe]`. The fix:

1. `_ci_build_verify` (`just/devops.just`) emits `key_helper=${tool}` after
   successful verification.
2. The `gate` step of `.github/workflows/_package-ci.yml` (the second
   archive branch, ~line 760/813, **not** the `expected` step at ~line 697)
   binds `ARCHIVE_KEY_HELPER`, requires it nonempty, and exports
   `BISCUIT_CI_BUILD_BIN`.
3. `scripts/cross-check.sh` binds the same variable in `unix_run_archive`
   and `windows_run_archive`.
4. Contracts pin all of it (`ci_workflow_contracts.rs`,
   `test_cross_check.py`, `test_build_key.py`).
5. `planned_keys` in `build_key.py` isolates the helper child from
   compiler-wrapper mode (`BISCUIT_CI_BUILD_WRAP` removed, `RUSTC_WRAPPER=""`)
   — required, otherwise the newly bound helper would run `key` as a rustc
   command in a measured gate.
6. Docs plus an OS-skill fact (BUILD_WIN WindowsApps Python aliases).

**Done when** (maps to spec acceptance criteria):

- AC1/AC2: gate exports the verifier path, fails on empty `key_helper`;
  generated Unix and Windows archive scripts bind after verification and
  before `just _test`; native mode unchanged.
- AC3: each of the three contract removals (gate binding, gate export,
  recipe output) is shown to fail its owning contract once, then restored.
- AC4: resolver empty/override tests and the subprocess-boundary test pass;
  keys and canonical bytes unchanged (existing pinned-digest tests green).
- AC5/AC6: one native-Windows archive run proves the planner executes with a
  real Python and the consumer's `ci-build.exe`; durations recorded;
  temporary diagnostic removed.
- Docs updated in the same change; `just lint` clean for touched packages.

**Agent terminal state:** implementation complete, ready for review. Do not
move the fix to `_completed`, run `just complete`, or commit unless told to.

**Input Robustness Matrix:** not applicable. No parser, manifest, or config
reader is added or changed; `key_helper` is an internal step output and the
env override is an existing string contract.

## Phase 1: Producer output, resolver boundary, and rulings

### Necessary Rules

Rulings needed from the author (each has a recommended default so work is
not blocked; implementers proceed with the default unless told otherwise):

1. **Which workflow step.** `_package-ci.yml` has two archive branches that
   export `BISCUIT_NEXTEST_BIN` (the `expected` step and the `gate` step).
   Spec §2 says only `gate`. *Default: gate only; `expected` gets no
   binding and a contract asserts that.*
2. **Windows path spelling in cross-check.** Spec §3 uses backslashes in
   PowerShell (`\$consume\build\tools\ci-build.exe`); CI uses forward slashes
   via `_native_path`. *Default: follow the spec per surface; both are valid
   single `argv[0]` for `subprocess.run`, verified in Phase 3.*
3. **`RUSTC_WRAPPER` emptiness vs. unset.** Spec requires explicit empty
   string (prevents Cargo config restoring a wrapper). *Default: set to `""`
   in the child env only.*
4. **Contract-test depth for "after successful verification".** Spec asks the
   recipe contract to establish the output follows verification.
   *Default: assert byte-order within the extracted recipe body: the
   verifier invocation precedes the `GITHUB_OUTPUT` block containing
   `key_helper=`; no cross-recipe scanning.*
5. **Surviving-test fallback for AC5.** If `2026-10-06-archive-guard-full-tree`
   lands first and retires the two guard tests, the focused Windows run uses
   `the_real_planners_plan_rolls_up`. *Default: decide at Phase 3 start by
   checking whether the `the_shipped_planner_*` tests still exist.*
6. **Wider measurement (not scheduled).** The spec forbids extra perf spikes;
   hosted-runner timing is observed only from later normally scheduled CI
   runs. *Default: none added; flagged for the author only if they want a
   hosted baseline.*

### Spikes

None. The spec's cold-vs-warm measurements already answer the performance
question (recorded as ruling, no spike). The single Windows run in Phase 3 is
acceptance evidence (AC5/AC6), not a spike.

### Wave 1 (parallel — disjoint files)

- [x] **Recipe output** — `just/devops.just`, `_ci_build_verify`:
    - add `echo "key_helper=${tool}"` inside the existing `$GITHUB_OUTPUT`
      block (~line 284–296), which runs only after verification succeeds
    - keep the existing exec-permission handling and `.exe` suffix selection
    - update the recipe's output documentation comment to list `key_helper`
- [x] **Wrapper isolation** — `scripts/ci/build_key.py`, `planned_keys`
  (~line 137–156):
    - pass `env=` a copy of `os.environ` with `BISCUIT_CI_BUILD_WRAP`
      removed and `RUSTC_WRAPPER` set to `""`
    - leave the counter directory variable and everything else inherited;
      never mutate the parent's `os.environ`
    - add a short WHY comment at the line (coupled bindings; fallback Cargo
      build must not run under wrapper mode); update `planned_keys` docstring
- [x] **Resolver + boundary tests** — `scripts/ci/test_build_key.py`
  (depends on nothing; can be written against current code, boundary test
  fails until the wrapper task lands):
    - empty `BISCUIT_CI_BUILD_BIN` falls back to candidate discovery
      (temp candidate file, substitute `_candidates`, no compile)
    - nonempty override wins over an existing candidate; returned command is
      the override as a single argument, including a path with spaces
    - subprocess-boundary test: set wrapper mode in parent, temp helper
      path, patch `subprocess.run` to return a captured successful key
      response; assert child env lacks `BISCUIT_CI_BUILD_WRAP`, has
      `RUSTC_WRAPPER == ""`, preserves counter dir and ordinary vars; parent
      env unchanged
    - use the suite's existing env-restoration and `reset_helper_cache()`
      pattern; keep invalid-override, helper-error, pinned-digest tests

### Checkpoint 1

- [x] `python3 -m unittest scripts/ci/test_build_key.py` passes
- [x] `git diff` of `build_key.py` shows only the env change plus comment
      (no resolver change)

## Phase 2: Consumers bind the helper

Depends on Phase 1 (the `key_helper` output must exist and the wrapper
isolation must be in place before the binding is safe in measured gates).

### Wave 2 (parallel — disjoint files)

- [x] **Gate binding** — `.github/workflows/_package-ci.yml`, `test` job
  `gate` step only:
    - add step env `ARCHIVE_KEY_HELPER: ${{ steps.verified.outputs.key_helper }}`
      beside `ARCHIVE_WORKSPACE` (~line 760)
    - in the archive branch (~line 813–822), add
      `: "${ARCHIVE_KEY_HELPER:?verification did not report the ci-build key helper}"`
      and `export BISCUIT_CI_BUILD_BIN="$ARCHIVE_KEY_HELPER"`
    - do not touch the `expected` step (~line 697–727) or native branches
    - extend the archive-branch comment to name the planner key helper as
      another unintended build avoided
- [x] **Cross-check Unix** — `scripts/cross-check.sh` `unix_run_archive`
  step 7 (~line 662): add `export BISCUIT_CI_BUILD_BIN="\$consume/build/tools/ci-build"`
  after verification, before `just _test`; verify the `\$` escape disappears
  in the generated script and spaces stay quoted
- [x] **Cross-check Windows** — `windows_run_archive` (~line 862): add
  `\$env:BISCUIT_CI_BUILD_BIN = "\$consume\\build\\tools\\ci-build.exe"`
  next to `BISCUIT_NEXTEST_BIN`; native mode unchanged
- [x] **OS skill fact** — `.claude/skills/os/windows.md` ("Environment and
  processes") and a matching row in `.claude/skills/os/SKILL.md` symptom
  table: BUILD_WIN SSH sessions resolve `python3`/`python` to WindowsApps
  Store aliases ahead of the real interpreter; `--version` fails; tests
  gated on `python_interpreter()` skip and nextest PASSes in ~0.3 s;
  prepend the real interpreter (confirm via successful `--version`; do not
  assume Python313) on a process-scoped `PATH`; PASS alone is not evidence,
  captured output is

### Checkpoint 2

- [x] `bash -n scripts/cross-check.sh` clean; workflow YAML parses
      (`python3 -c 'import yaml…'` or the repo's workflow lint)

## Phase 3: Contracts, then end-to-end evidence

### Wave 3 (parallel — disjoint files)

- [x] **Workflow contracts** —
  `tools/test-toolkit/tests/ci_workflow_contracts.rs` (load `rust-testing`
  skill first; follow the suite's existing helpers):
    - `gate` knob list gains `export BISCUIT_CI_BUILD_BIN="$ARCHIVE_KEY_HELPER"`
    - `gate` binds `ARCHIVE_KEY_HELPER` from `steps.verified.outputs.key_helper`
    - required-value check and export asserted **inside the archive branch,
      before the canonical test recipe**, scoped to the `gate` step text
      (not anywhere in the job); `expected` step has no binding
    - in `an_archive_consumer_hands_native_programs_native_paths`: limit the
      inspected text to `_ci_build_verify`, assert `key_helper=${tool}` is in
      the `GITHUB_OUTPUT` block and follows the verifier invocation
    - update the no-Cargo contract doc comment: runner setup vs. deliberate
      Cargo calls in `requires-toolchain` tests; name the planner key helper
      as another unintended build this binding prevents
- [x] **Generated-script tests** — `scripts/ci/test_cross_check.py`:
    - Unix archive sequence test (Linux, macOS, WSL simulated hosts): quoted
      `BISCUIT_CI_BUILD_BIN` binding present; ordering
      verification → binding → test recipe; no leftover `\$` escapes
    - Windows sequence test: `.exe` env assignment present with the same
      ordering
    - native mode generated scripts contain no new assignment

### Checkpoint 3 (negative checks, AC3 — local, no remote builds)

- [x] remove gate binding → workflow contract fails → restore
- [x] remove gate export → workflow contract fails → restore
- [x] remove `key_helper=` output from the recipe → native-path contract
      fails → restore
- [x] `just test test-toolkit` and
      `python3 -m unittest scripts/ci/test_build_key.py scripts/ci/test_cross_check.py`
      green (record counts)

### Wave 4 (sequential — needs a build host)

- [x] **Focused Windows archive run** (AC5/AC6):
    - decide test target per ruling 5:
      `just cross-check test-toolkit --os windows the_shipped_planner`, else
      `just cross-check repo-deps --os windows the_real_planners_plan_rolls_up`
    - temporarily add a diagnostic in `build_key.py` helper resolution
      recording the actual command and override; make the real Python
      reachable first on the remote `PATH` (process-scoped; confirm
      `--version` succeeds)
    - confirm: command is `<build>/tools/ci-build.exe`, not Cargo and not a
      producer path; captured output shows the planner ran (not the
      Python-unavailable early return)
    - record per-test durations; guard tests (if present) below the 30 s
      slow mark, else investigate cause before claiming resolution
    - **remove the diagnostic**; `git diff` confirms `build_key.py` is back
      to only the Phase 1 change
    - reuse qualifying existing evidence if any answers the same question
      (check `just ci-local --plan` receipts first)

## Phase 4: Documentation and closeout

Can start alongside Wave 3 (docs do not depend on test results) but closes
after Phase 3 so wording matches measured behavior.

### Wave 5 (parallel — disjoint files)

- [ ] **CI README** — `.github/ci/README.md`, "Consumers verify, then run":
  the shipped verifier doubles as the planner's key helper
  (`BISCUIT_CI_BUILD_BIN`) after verification succeeds; a planner-running
  `requires-toolchain` suite avoids the helper build but keeps intentional
  Cargo calls. State behavior directly; no link to this fix
- [ ] **Topic doc** — `docs/topics/ci-cd.md`: matching archive-consumer
  explanation, same rules; a compact example of the binding and, if it
  clarifies the flow, a Mermaid diagram (verify → export → planner)
- [ ] **rust-devops skill** — `.claude/skills/rust-devops/ci-cd.md`:
  archive-consumer guidance with the binding and the toolchain distinction
- [ ] **Drift pass** — re-read comments/docs on every changed symbol
  (`planned_keys`, `_ci_build_verify`, gate step, cross-check generators);
  fix drift in the same change; check `docs/` pages never name this fix

### Final checkpoint

- [ ] `just lint` for touched areas (`test-toolkit`; repo-level lint for
      scripts) clean; `just ci-local --plan` reviewed (expect `test-toolkit`
      and `repo-deps` cells only; no new scheduling rules)
- [ ] spec `status`/`implemented` frontmatter left for the author's review
      cycle; no commit, no move to `_completed`
- [ ] follow-up (post-merge, not scheduled by this plan): compare the next
      normal `ubuntu-latest` and `windows-latest` planner-test durations
      only against each environment's own earlier runs
