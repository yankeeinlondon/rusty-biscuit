---
kind: fix
name: archive-key-helper
date: 2026-10-06
status: draft-spec
related:
  - 2026-10-06-archive-guard-full-tree
  - 2026-09-12-single-os-compile
reviewed: false
review_iterations: 0
clarified: false
implemented: false
human_review: true
$schema:
  status: |-
    enum(
        draft-spec,
        finalized-spec,
        planned,
        implemented,
        review-findings,
        human-in-the-loop,
        completed,
        on-hold,
        abandoned
    ) -> an indicator of progress for this specification
  reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
  reviewed_by: string -> the agent and model used in the spec review
  reviewed_on: date -> the date the spec was reviewed
  review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
  clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
---

# Archive consumers hand the planner its prebuilt key helper

## Summary

Every run of the planner (`scripts/ci/affected_scope.py`) computes at least one
build key. Build keys come from the `ci-build` helper, which
`scripts/ci/build_key.py` looks for at `<root>/target/{release,debug}/ci-build`.
When the helper is not there and `BISCUIT_CI_BUILD_BIN` is unset, it falls back
to `cargo run … --bin ci-build`. An archive consumer's checkout has no
`target/` and restores no Cargo cache, so the first planner call in every
archive-mode test process compiles `ci-build` from scratch.

That one compile accounts for most of the time the two planner contract tests in
`tools/test-toolkit/tests/ci_workflow_contracts.rs` take in archive mode. It is
why they run 64–82 s on `ubuntu-latest` and hit the 90 s kill on
`windows-latest`.

The consumer already holds a verified, checksummed `ci-build` binary: the
verifier that travels in every build artifact as `<build>/tools/ci-build[.exe]`.
This fix exports that binary as `BISCUIT_CI_BUILD_BIN` in the archive-mode test
step, in CI and in `just cross-check`, so that no consumer compiles the key
helper.

This is the same class of defect, with the same remedy, as the existing
"an archive consumer asks Cargo for nothing" bindings
(`BISCUIT_NEXTEST_BIN`, `BISCUIT_JUNIT_*`, `BISCUIT_BACKEND_PROOF_BIN`).

## Evidence (measured 2026-10-06)

The tests were instrumented temporarily: an `Instant` around each planner
call, plus stderr timings inside the planner for `cargo metadata`, the key
helper call, and `test_inputs.scan`. They were run through
`./scripts/cross-check.sh test-toolkit --os {wsl,windows} <filter>`, which is
archive mode with the producer's `target/` hidden.

- Test 1 is `the_shipped_planner_emits_plans_the_guards_reader_accepts`.
- Test 2 is `the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`.

| Host, mode | Run | Test 1 | Test 2 | First key call (`cargo run`, cold) |
|---|---|---:|---:|---|
| `BUILD_WSL`, archive | test 2 alone | — | 25.0 s | 21.2 s |
| `BUILD_WSL`, archive | test 1 alone | 29.3 s | — | 22.0 s |
| `BUILD_WSL`, archive | both | 29.7 s | 28.8 s | 20.2 s / 23.5 s, concurrent |
| `BUILD_WIN`, archive | test 2 alone | — | 34.0 s | 28.8 s |
| `BUILD_WIN`, archive | both | 58.6 s | 57.6 s | 24.7 s / 30.1 s, plus a one-off 22 s package-cache lock wait on the first `cargo metadata` |
| `BUILD_WIN`, archive | both, test 1 fed `scripts/ci/schema.py` instead of `README.md` | 40.8 s | 37.0 s | 29.9 s / 28.8 s |

What the measurements rule out:

- **The test-input scan is not the cause.** It costs 2.2–2.7 s on WSL and
  4.3 s on Windows. Avoiding it (the last row) leaves both tests well above
  the 30 s slow mark on Windows.
- **Contention is not the main cost.** Test 2 is as slow alone as alongside
  test 1. The two concurrent compiles share one build behind Cargo's lock, so
  running the tests one after the other would not save the compile. The 22 s
  `cargo metadata` lock wait appeared once in three Windows runs.
- **The cold compile is the main cost, and it happens once per test process.** After it, each
  key call takes about 0.01 s (the helper is found in `target/debug/`), and a
  whole planner call takes 1.1–2.2 s.

Even a documentation-only plan needs a key: `skip_policy.content_hash`
(`affected_scope.py`, the baseline policy snapshot) is computed on every
plan. Changing a test's input therefore cannot avoid the helper.

The CI consumer is in the same position. `_package-ci.yml`'s archive tier
"installs no toolchain … restores no Cargo cache" (`.github/ci/README.md`,
"Consumers verify, then run"), and its verifier lives under
`$RUNNER_TEMP/build/tools/`, not `target/`. A `requires-toolchain` suite gets
a toolchain but no cache, so the CI compile is colder than the build hosts'
and also downloads crates. This is read from the workflow, not measured in CI.

## Scope

### 1. `_ci_build_verify` reports the helper it verified

`just/devops.just::_ci_build_verify` already resolves
`tool="${dir}/tools/ci-build${suffix}"`, refuses to continue without it, and
has `ci-build verify` check its checksum. Add one line to its `$GITHUB_OUTPUT`
block:

```bash
echo "key_helper=${tool}"
```

The path keeps the native spelling the recipe already produces for
`archive_file` and `workspace`.

### 2. The CI archive gate exports it

In `.github/workflows/_package-ci.yml`, the `test` job's `gate` step:

- Add the step env `ARCHIVE_KEY_HELPER: ${{ steps.verified.outputs.key_helper }}`
  next to the existing `ARCHIVE_WORKSPACE`.
- Inside the archive-mode branch that already exports `BISCUIT_NEXTEST_BIN`
  and `BISCUIT_JUNIT_WORKSPACE_ROOT`, add:

  ```bash
  export BISCUIT_CI_BUILD_BIN="$ARCHIVE_KEY_HELPER"
  ```

The `expected` listing step runs no tests and is left unchanged. Only the archive branch
exports the variable. A native-mode cell keeps today's behavior, and
`build_key.py` already ignores an empty value.

### 3. `just cross-check` matches CI

`scripts/cross-check.sh` reproduces the consumer by hand, so it must bind the
same variable or it stops reproducing CI timing:

- `unix_run_archive`, step 7:
  `export BISCUIT_CI_BUILD_BIN="\$consume/build/tools/ci-build"`.
- `windows_run_archive`: `\$env:BISCUIT_CI_BUILD_BIN =
  "\$consume\\build\\tools\\ci-build.exe"`, next to `BISCUIT_NEXTEST_BIN`.

### 4. The contract pins it

Extend `an_archive_consumer_asks_cargo_for_nothing_including_metadata` in
`tools/test-toolkit/tests/ci_workflow_contracts.rs`:

- The `gate` step's knob list gains
  `export BISCUIT_CI_BUILD_BIN="$ARCHIVE_KEY_HELPER"`.
- The `gate` step binds `ARCHIVE_KEY_HELPER` from
  `steps.verified.outputs.key_helper`.
- `_ci_build_verify` writes `key_helper=${tool}` to `$GITHUB_OUTPUT`, where
  `tool` is the path it has already normalized. Assert this in
  `an_archive_consumer_hands_native_programs_native_paths`, which already
  reads that recipe's body.
- Update the test's doc comment to name the key helper beside `_stage_junit` and
  `_backend_proof` as a path that would otherwise reach `cargo run`.

### 5. `build_key.py` treats an empty value as unset

This is already the behavior (`if override:`). Pin it with one case in
`scripts/ci/test_build_key.py`: an empty `BISCUIT_CI_BUILD_BIN` falls back to
candidate discovery instead of raising "names '', which is not a file". The
workflow step relies on this for non-archive cells.

### 6. Documentation

- `.github/ci/README.md`, "Consumers verify, then run": one sentence saying
  that the verified `ci-build` also serves as the planner's key helper
  (`BISCUIT_CI_BUILD_BIN`), so a `requires-toolchain` suite that runs the
  planner compiles nothing.
- `.claude/skills/os/windows.md`, "Environment and processes" (the CLAUDE.md
  rule to record an OS fact in the same change): on `BUILD_WIN`, SSH sessions
  resolve `python3` and `python` to the WindowsApps Store aliases ahead of
  `C:\Users\ken\AppData\Local\Programs\Python\Python313`. Both fail
  `--version`, so every test gated on `python_interpreter()` skips there
  and nextest reports PASS in about 0.3 s. Verifying a planner test on that
  host needs the interpreter on `PATH` first. The skill's symptom table gets a
  matching row.

## Out of scope

- **Local fresh worktrees.** A native run in a new worktree still compiles
  `ci-build` once, into the shared `target/`, and later runs find it.
  That is the existing, intended fallback.
- **`wsl2-ubuntu`.** `test-toolkit` and `repo-deps` declare
  `requires-toolchain = true`, so their WSL2 cells are governed gaps. The
  planner also needs `cargo metadata`, which the guest cannot run.
  `_wsl-ci.yml` is not changed.
- **`repo-deps`'s own planner tests**
  (`scripts/ci-build-archive-tests.rs`, `scripts/ci-rollup-tests.rs`). They
  run in archive cells and benefit from §2 without any edit. Having them pass
  `bin_exe!("ci-build")` explicitly is possible, but nothing here requires it.
- **Whether the archive guard should always scan the full tree.**
  `2026-10-06-archive-guard-full-tree` is decided on its own merits. This fix
  removes the timeout that spec cites as motivation, but it neither requires
  nor rules out that change.
- **Test-input scan performance.** It costs 2–4 s and is not on the critical
  path.

## Acceptance criteria

1. In an archive-mode `just cross-check test-toolkit --os windows
   the_shipped_planner` run with Python visible, neither planner contract test
   is marked SLOW (30 s). The expected figure, from the warm calls measured
   above, is about 10 s or less for test 1 and about 5 s or less for test 2.
   The run's planner stderr must show no `cargo run` (a temporary probe or
   `PYTHONVERBOSE` is enough to confirm this; it is not shipped).
2. The same holds for `--os wsl` (expected about 6 s and 2 s).
3. `an_archive_consumer_asks_cargo_for_nothing_including_metadata` fails when
   the new export, the step env binding, or the `key_helper` output is removed.
   Prove this once by deleting each line in turn.
4. `test_build_key.py`'s new empty-value case passes.
5. The first `ubuntu-latest` CI run that schedules `test-toolkit` L1 shows both
   tests well under 30 s. `windows-latest` evidence arrives with the next push
   to `main`, or from a pull request labeled `ci:all-os` if the author wants it
   before merge.

## Verification plan

- Local: `just test` in `tools/test-toolkit`, and `python3 -m unittest
  scripts/ci/test_build_key.py`.
- `./scripts/cross-check.sh test-toolkit --os wsl the_shipped_planner` and the
  same with `--os windows`. On `BUILD_WIN`, check that the tests did not skip:
  a PASS in under 1 s means `python_interpreter()` found nothing (see §6).
- No full-scope CI run. The `_package-ci.yml` and `devops.just` edits schedule
  their own contract tests through the normal planner.

## Risks

- **Spelling of the verifier path on Windows.** `_ci_build_verify` already
  exports its other paths through `_native_path`, and `build_key.py` passes
  the value to `subprocess.run` as `argv[0]`, so a native `D:\…\ci-build.exe`
  is what is needed. Acceptance criterion 1 exercises this path through
  `cross-check`. CI exercises it on the first `windows-latest` run.
- **The helper is a release build from the producer, not a local debug build.**
  The key algorithm does not depend on the build profile, and `ci-build
  verify` has already bound this binary to the plan's head revision, so the
  keys are the ones the producer used.
