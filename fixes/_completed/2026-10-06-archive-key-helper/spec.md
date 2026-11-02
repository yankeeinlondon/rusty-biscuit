---
kind: fix
name: archive-key-helper
date: 2026-10-06
status: draft-spec
related:
  - 2026-10-06-archive-guard-full-tree
  - 2026-09-12-single-os-compile
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: "2026-10-06"
review_iterations: 1
completed: true
clarified: false
implemented: true
human_review: false
message_to_agent: |-
  Phase 4 done: docs (CI README, docs/topics/ci-cd.md, rust-devops ci-cd skill) updated; plan complete and ready for author review.
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

# Archive consumers reuse the shipped build-key helper

## Summary

The [CI planner](../../scripts/ci/affected_scope.py), owned by the `repo-deps`
package, computes build keys through that package's
[build-key module](../../scripts/ci/build_key.py). The module first checks
`BISCUIT_CI_BUILD_BIN`, then looks for `ci-build[.exe]` under this checkout's
`target/release` and `target/debug`. Without a helper there, it invokes
`cargo run … --bin ci-build`. An archive consumer runs prebuilt tests in a
checkout without these build outputs or a restored Cargo cache, so its first
planner invocation starts a fresh helper build. Concurrent invocations in the
same checkout can wait on that build; they do not necessarily compile separate
copies.

On the build hosts, that compile accounts for most of the archive-mode
duration of the two planner contract tests in
`tools/test-toolkit/tests/ci_workflow_contracts.rs`. The hosted CI consumer
takes the same code path (see Evidence). Build-host durations say nothing about
hosted-runner durations, so this spec makes no claim about how much CI time
the fix saves. That is judged only against earlier runs of the same CI
environment.

The consumer already holds `ci-build`: the producer-built verifier that
travels in every build artifact as `<build>/tools/ci-build[.exe]`. This fix
exports that same binary as `BISCUIT_CI_BUILD_BIN` after successful archive
verification, in CI and in `just cross-check`, so the planner does not compile
its key helper.

This is the same class of defect, with the same remedy, as the existing
"an archive consumer asks Cargo for nothing" bindings
(`BISCUIT_NEXTEST_BIN`, `BISCUIT_JUNIT_*`, `BISCUIT_BACKEND_PROOF_BIN`).

**Review note:** the shipped verifier checks the manifest, archive, and
sidecars; the [manifest](../../scripts/ci-build-archive.rs) in `repo-deps`
does not record a checksum for the verifier itself. Reusing the verifier
preserves the existing artifact trust boundary. This spec does not add
self-verification or claim that archive verification independently authenticates
the helper's bytes.

The archive runner avoids Cargo for its own setup and execution. Tests in a
`requires-toolchain` suite may still intentionally invoke Cargo, including
the planner's `cargo metadata` call. Removing the helper build does not remove
that declared toolchain requirement.

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
- **Contention can amplify the cost, but cannot explain it alone.** Test 2
  still takes 34 s on Windows when run alone. Concurrent helper requests
  share one build behind Cargo's lock, so serial execution would retain the
  cold compile. The 22 s `cargo metadata` lock wait appeared once in three
  Windows runs.
- **The cold compile is the main cost in a fresh consumer checkout.** After it, each
  key call takes about 0.01 s (the helper is found in `target/debug/`), and a
  whole planner call takes 1.1–2.2 s.

Even a documentation-only plan needs a key: `skip_policy.content_hash`
(`affected_scope.py`, the baseline policy snapshot) is computed on every
plan. Changing a test's input therefore cannot avoid the helper.

The CI consumer is in the same position. `_package-ci.yml`'s archive tier
"installs no toolchain … restores no Cargo cache" (`.github/ci/README.md`,
"Consumers verify, then run"), and its verifier lives under
`$RUNNER_TEMP/build/tools/`, not `target/`. A `requires-toolchain` suite gets
a toolchain but no Cargo cache, so its first planner call reaches the same
`cargo run` fallback. This is read from the workflow; it establishes the
code path, not the cost.

## Design decisions and scope

Reuse the binary already shipped with the artifact; do not add a sidecar,
another helper build, a new hashing implementation, or a new CI job. The
`ci-build key` protocol, its schema version, and its use of `biscuit-hash`
remain unchanged. `key_helper` is an internal step output, so it does not
change a versioned plan, manifest, receipt, or completion-record format.

### 1. Verification reports the verifier path after success

The repository's shared [_ci_build_verify recipe](../../just/devops.just)
already resolves `tool="${dir}/tools/ci-build${suffix}"`, refuses to continue
without it, and invokes it to verify the artifact. Add one line to its
existing `$GITHUB_OUTPUT` block, which runs only after verification succeeds:

```bash
echo "key_helper=${tool}"
```

The output is the exact binary used for verification, rather than a later
search through `PATH` or the checkout's build outputs. Keep the recipe's
existing executable-permission handling and `.exe` selection. On Windows,
`_native_path` returns a drive-qualified path with forward slashes, such as
`D:/a/_temp/build/tools/ci-build.exe`; this works for both Git Bash and native
programs. Update the recipe's output documentation to include `key_helper`.

### 2. The CI archive gate exports it

In the [package workflow](../../.github/workflows/_package-ci.yml), the
`test` job's `gate` step:

- Add the step env `ARCHIVE_KEY_HELPER: ${{ steps.verified.outputs.key_helper }}`
  next to the existing `ARCHIVE_WORKSPACE`.
- Inside the archive-mode branch that already exports `BISCUIT_NEXTEST_BIN`
  and `BISCUIT_JUNIT_WORKSPACE_ROOT`, add:

  ```bash
  : "${ARCHIVE_KEY_HELPER:?verification did not report the ci-build key helper}"
  export BISCUIT_CI_BUILD_BIN="$ARCHIVE_KEY_HELPER"
  ```

The nonempty check prevents a broken output binding from silently returning
to Cargo. A nonempty path that is missing or cannot execute already fails in
the build-key module; retain that behavior and do not retry by building a
replacement. In archive mode this binding replaces any inherited helper
override, so every planner subprocess uses the artifact's binary.

The `expected` step starts archived test binaries to list test identities,
but does not execute their test bodies or the planner; it needs no helper
binding. Only the archive branch exports the variable. A native-mode cell
retains its existing helper resolution and any caller-provided override.

### 3. `just cross-check` matches CI

[The cross-check script](../../scripts/cross-check.sh) reproduces the consumer
by hand, so it must bind the
same variable or it stops reproducing CI timing:

- `unix_run_archive`, step 7:
  `export BISCUIT_CI_BUILD_BIN="\$consume/build/tools/ci-build"`.
- `windows_run_archive`: `\$env:BISCUIT_CI_BUILD_BIN =
  "\$consume\\build\\tools\\ci-build.exe"`, next to `BISCUIT_NEXTEST_BIN`.

These assignments are in the generated remote scripts, after successful
verification and before `just _test`. The backslash before `$` above prevents
the local Bash heredoc from expanding a remote variable; it must disappear
in the generated script. Quote paths as shown so spaces remain part of a
single filename. The PowerShell environment assignment goes directly to the
native Python process, so its backslashes do not pass through Just's argument
expansion. Native cross-check mode receives no new assignment.

### 4. The contract pins it

Extend the archive runner contracts in the `test-toolkit` package's
[workflow contract suite](../../tools/test-toolkit/tests/ci_workflow_contracts.rs):

- The `gate` step's knob list gains
  `export BISCUIT_CI_BUILD_BIN="$ARCHIVE_KEY_HELPER"`.
- The `gate` step binds `ARCHIVE_KEY_HELPER` from
  `steps.verified.outputs.key_helper`.
- The required-value check and export occur inside the archive branch,
  before the canonical test recipe. Check the relevant branch, rather than
  accepting the strings anywhere in the job.
- `_ci_build_verify` writes `key_helper=${tool}` to `$GITHUB_OUTPUT`, where
  `tool` is the path it has already normalized. Assert this in
  `an_archive_consumer_hands_native_programs_native_paths`, which already
  reads that recipe's body. Limit the inspected text to that recipe and
  establish that the output follows successful verification.
- Update the no-Cargo contract's doc comment to distinguish the runner's
  setup from deliberate Cargo calls inside `requires-toolchain` tests. Name
  the planner's key helper as another unintended build this binding prevents.

Also extend the existing generated-script tests in `repo-deps`'
[test_cross_check.py](../../scripts/ci/test_cross_check.py). The Unix archive
sequence test must find the quoted helper binding for Linux, macOS, and WSL;
the Windows sequence test must find the `.exe` environment assignment. Each
must establish `verification → binding → test recipe` ordering in the
generated script, with no leftover local heredoc escapes. Keep native mode's
existing behavior covered. These tests use the existing simulated hosts;
they require no extra remote runs or CI cells.

### 5. Preserve helper resolution and isolate compiler-wrapper mode

Empty-as-unset is already the behavior (`if override:`); no resolver change
in the `repo-deps` [build-key module](../../scripts/ci/build_key.py) is required.
Extend its [resolution tests](../../scripts/ci/test_build_key.py) to pin two
boundaries:

- An empty `BISCUIT_CI_BUILD_BIN` falls back to candidate discovery. Supply
  a temporary candidate file and substitute `_candidates` so the test does
  not depend on this host's build outputs or compile a helper.
- A nonempty override takes precedence even when a candidate exists.
  Assert the returned command is the override path as one argument,
  including a path containing spaces. Resolution alone requires no POSIX
  shim and can be tested on Windows as well.

Use the suite's existing environment restoration and cache-reset pattern.
Retain its invalid-override and helper-error tests. Empty-as-unset is a
general resolver contract; the archive branch now rejects an empty verified
binding before reaching that fallback.

The module's `planned_keys` function, which invokes the helper to hash a batch
of canonical inputs, does need one environment boundary. Pass a copy of the
current environment to its helper subprocess with `BISCUIT_CI_BUILD_WRAP`
removed and `RUSTC_WRAPPER` explicitly empty. The `repo-deps`
[ci-build executable](../../scripts/ci-build.rs)
uses that switch to enter rustc-wrapper mode before parsing subcommands;
a measured CI gate sets it even during archive execution. Without isolation,
the newly bound helper would try to execute `key` as a compiler command.

**Review decision:** clear the two coupled wrapper bindings only in the
key-helper child, rather than in the whole gate. If the resolver chooses its
Cargo fallback, retaining `RUSTC_WRAPPER` while removing the mode switch would
break compiler-wrapper invocations during that helper build. Disabling both
keeps helper resolution valid in either mode; an explicitly empty
`RUSTC_WRAPPER` also prevents Cargo configuration from restoring a wrapper.
The helper's bootstrap build
then remains outside compiler-work measurement, as the counter's own bootstrap
already is. Archive consumers use the explicit binary and perform no such
build. Preserve the counter directory and the rest of the inherited
environment, and leave the parent's environment unchanged, so deliberate
Cargo calls made elsewhere in the suite retain their measurement bindings.

Add one portable subprocess-boundary test using the existing Python unittest
tools: set wrapper mode in the parent, supply a temporary helper path, and
replace `subprocess.run` with a captured successful key response. Assert
the helper's environment omits the mode switch, sets `RUSTC_WRAPPER` to an
empty string, preserves the counter directory and ordinary variables, and
leaves the parent's variables intact.
Reset the helper cache and restore every environment variable changed by
the test. No real compiler or new test executable is needed.

### 6. Documentation

- [.github/ci/README.md](../../.github/ci/README.md), "Consumers verify, then
  run", and the matching archive-consumer explanation in
  [docs/topics/ci-cd.md](../../docs/topics/ci-cd.md): explain that the shipped
  verifier also serves as the planner's key helper (`BISCUIT_CI_BUILD_BIN`)
  after verification succeeds. A planner-running `requires-toolchain` suite
  avoids this helper build while retaining its intentional Cargo calls.
  State behavior directly without linking current documentation to this fix.
- [.claude/skills/rust-devops/ci-cd.md](../../.claude/skills/rust-devops/ci-cd.md):
  update the archive-consumer workflow guidance with the same binding and
  toolchain distinction.
- `.claude/skills/os/windows.md`, "Environment and processes" (the CLAUDE.md
  rule to record an OS fact in the same change): on `BUILD_WIN`, SSH sessions
  resolve `python3` and `python` to the WindowsApps Store aliases ahead of
  `C:\Users\ken\AppData\Local\Programs\Python\Python313`. Both fail
  `--version`, so every test gated on `python_interpreter()` skips there
  and nextest reports PASS in about 0.3 s. Verifying a planner test on that
  host needs the interpreter on the consuming process's `PATH` first, using
  a process-scoped change rather than changing the host's persistent setup.
  Do not assume Python313 is always installed: confirm the real interpreter
  with a successful `--version`. The [OS skill](../../.claude/skills/os/SKILL.md)'s
  symptom table gets a matching row. A short duration is only a warning sign;
  captured output confirming execution, rather than the Python-unavailable
  early return, is the evidence that the test ran.

## Out of scope

- **Local fresh worktrees.** A native run in a new worktree still compiles
  `ci-build` when no override or discoverable helper exists. Candidate
  discovery currently checks this checkout's `target/`; do not assume a
  configured external target directory will be discovered. Changing that
  fallback or worktree build-storage policy is separate work.
- **`wsl2-ubuntu`.** `test-toolkit` and `repo-deps` declare
  `requires-toolchain = true`, so their WSL2 cells are governed gaps. The
  planner also needs `cargo metadata`, which the guest cannot run.
  `_wsl-ci.yml` is not changed.
- **`repo-deps`'s own planner tests**
  (`scripts/ci-build-archive-tests.rs`, `scripts/ci-rollup-tests.rs`). They
  run in archive cells and benefit from the CI archive gate's binding without
  any edit. Having them pass
  `bin_exe!("ci-build")` explicitly is possible, but nothing here requires it.
- **Whether the archive guard should always scan the full tree.**
  `2026-10-06-archive-guard-full-tree` is decided on its own merits. This fix
  addresses the helper build that contributes to the timeout cited there;
  it neither requires nor rules out that change. That draft also removes
  the two planner-to-guard tests. If it lands first, do not restore those
  retired tests to measure this fix: keep the runner and resolver contracts,
  and verify the binding through a surviving `repo-deps` planner test such as
  `the_real_planners_plan_rolls_up` in
  [ci-rollup-tests.rs](../../scripts/ci-rollup-tests.rs). The helper binding
  remains useful to planner callers independently of the guard API.
- **Test-input scan performance.** It costs 2–4 s and is not the dominant
  measured cost.
- **Artifact authentication, WSL toolchain provisioning, and CI scheduling.**
  Keep the current trust model, governed capability gaps, event policy,
  package identities, and evidence-reuse rules. This wiring change introduces
  no new jobs, environments, schema versions, or timeout changes.

## Acceptance criteria

1. After successful verification, the CI archive gate exports the exact
   shipped verifier path as `BISCUIT_CI_BUILD_BIN`. An absent or empty
   `key_helper` output fails before the tier runs. Native mode retains its
   existing resolver behavior.
2. Generated Unix and Windows archive scripts bind the transferred helper
   after verification and before testing. Paths with spaces remain one
   filename; Windows uses the `.exe` spelling. The workflow, native-path,
   and generated-script contracts all pass.
3. The workflow contract fails if its gate binding or export is removed;
   the native-path contract fails if the recipe's output is removed. Exercise
   these three removals once, with the owning contract, and restore each
   before continuing. No remote builds are needed for these negative checks.
4. The resolver's empty-value and override-precedence cases pass, alongside
   its existing error and pinned-digest tests. The helper selection changes
   neither canonical input bytes nor resulting keys. The subprocess-boundary
   test proves key requests ignore compiler-wrapper mode while preserving
   the parent's measurement bindings and the child's other environment values.
5. One focused native-Windows archive run executes the planner tests with a
   working Python interpreter. A temporary diagnostic at helper resolution
   records the actual command and nonempty override; it must be the consumer's
   `<build>/tools/ci-build.exe`, not a Cargo command or a producer path.
   Remove the diagnostic after collecting this evidence. Use the surviving
   `repo-deps` planner test if the guard tests have already been retired.
6. Record test durations from that same run. The two guard tests, if present,
   should stay below nextest's existing 30 s slow mark; the draft's estimates
   of roughly 10 s and 5 s are context, not new per-test timing assertions.
   A remaining slow result requires checking its cause before claiming the
   timeout is resolved, even if helper selection is correct.

**Review note:** the existing cold-versus-warm measurements already identify
the avoidable compile. No separate performance spike is needed. The single
Windows archive run checks real path handling and that the tests execute;
its durations also show whether removing the build resolved the observed
slowdown. Absence of `cargo run` in planner stderr is not proof: the build-key
module captures the helper subprocess's output internally, and
`PYTHONVERBOSE` does not report its command arguments.

## Verification plan

These are checks for implementation; this inline document review does not
build packages, contact build hosts, or trigger CI.

- From the repository root, run `just test test-toolkit` (the toolkit's
  own justfile does not define `test`), and `python3 -m unittest
  scripts/ci/test_build_key.py scripts/ci/test_cross_check.py`.
- Run `just cross-check test-toolkit --os windows the_shipped_planner`
  once, with the temporary helper-command diagnostic described above. If
  those tests were removed, use `just cross-check repo-deps --os windows
  the_real_planners_plan_rolls_up`. Confirm Python succeeds in the remote
  consumer environment and inspect captured output for the unavailable-Python
  early return; a nextest PASS alone does not establish execution. Reuse
  qualifying existing evidence where it answers the same requirement.
- Other operating systems keep their normal required coverage. Hosted
  `wsl2-ubuntu` retains the toolchain capability gap for these packages;
  a developer WSL host with Cargo is a different capability setup. Another
  timed WSL sample is not required to establish this fix's cause or remedy.
- Review `just ci-local --plan` before an implementation push. Let the
  canonical planner schedule affected packages and file-reader contracts;
  do not add scheduling rules to guarantee these checks. Observe the next
  normally scheduled `ubuntu-latest` and `windows-latest` results where the
  planner tests still exist. Compare each environment's durations only with
  that environment's own earlier runs, never with the build-host figures in
  Evidence. These observations are follow-up evidence, not a reason to
  trigger a full-workspace run or require `ci:all-os`.

## Risks

- **Spelling of the verifier path on Windows.** `_ci_build_verify` already
  exports its other paths through `_native_path`, and `build_key.py` passes
  the value to `subprocess.run` as one `argv[0]`. CI uses a drive-qualified
  forward-slash spelling; PowerShell's direct environment assignment may
  use native backslashes. The focused Windows archive run checks native
  execution, while the workflow and recipe contracts check CI's normalized
  output. The next normally scheduled Windows cell exercises the hosted path.
- **The helper is a release build from the producer, not a local debug build.**
  The `repo-deps` [ci-build key command](../../scripts/ci-build.rs) uses
  `biscuit-hash` independently of build profile. Archive verification binds
  the archive's source identity to the plan, not the verifier's bytes. Reuse
  the verifier shipped by the same producer and retain the existing
  key-schema checks and pinned digest tests; do not claim a stronger
  guarantee from verification than the implementation supplies.
- **Compiler-counter wrapper mode.** A measured archive gate can inherit
  `BISCUIT_CI_BUILD_WRAP`. Isolate key-helper command mode in the build-key
  module as specified above. Do not unset measurement bindings for the whole
  suite or change the counter's workflow; intentional Cargo calls still
  require the existing instrumentation.

## Open Questions

None blocking this design. Reusing the shipped helper is compatible with the
existing resolver and artifact contracts. The independent guard simplification
may remove the original timing workload; the alternate verification above
keeps this fix reviewable without retaining an obsolete guard reader.
