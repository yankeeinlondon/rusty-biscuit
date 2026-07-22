# Gate run — 2026-07-19 — Linux behavioral + Windows compile (review-7 finding 2)

Runtime evidence for review-7 finding 2 ("the process-tree rewrite is
unverified on current Linux and native Windows"). This file records measurement
only. No code was changed to produce it, and no red result was repaired in the
course of running it. Format follows `gate-run-2026-07-18.md`: nothing is
reported green unless the command ran to completion and its summary line is
quoted verbatim.

## Revision under test

```
git rev-parse HEAD
237b86a41e54b937c1fb521f90281d4e59b34b83
```

The working tree is **not** clean, and that is deliberate: the review-7
finding-3 fix — the wait-error cleanup epilogue in
`claudine/lib/src/composition/sequence/task/shell.rs` (including the new
`TaskShellError::Wait` variant at `shell.rs:200`) and its regressions in
`claudine/lib/src/composition/sequence/task/tests.rs` — exists only as
uncommitted changes on top of `237b86a41`. The evidence below covers the tree
*including* those changes.

`git status --short` at run time:

```
 M .claudine/memory/commits.md
 M CLAUDE.md
 M biscuit-test-harness/src/lib.rs
 M claudine/features/2026-07-11-sequence-plus/review-6.md
 M claudine/features/2026-07-11-sequence-plus/spec.md
 M claudine/lib/src/composition/error/mod.rs
 M claudine/lib/src/composition/sequence/task/mod.rs
 M claudine/lib/src/composition/sequence/task/shell.rs
 M claudine/lib/src/composition/sequence/task/tests.rs
?? biscuit-test-harness/src/win_input.rs
?? biscuit-test-harness/src/xdotool.rs
?? claudine/features/2026-07-11-sequence-plus/review-7.md
?? prompts/_implement/implement-review-findings-plan.md
?? prompts/_implement/review-findings-plan.md
```

The four `claudine/lib/src/composition/**` modifications are the finding-3
fix. The Linux source was copied with
`rsync -a --exclude=target/ --exclude=.git` to `/tmp/rb-linux-src`, so the
container tested exactly this tree (the worktree `.git` file points outside the
mount and cannot be carried into a container).

## Summary of verdicts

| Gate | Verdict | Exit code | Duration |
|---|---|---|---|
| Linux: `composition::sequence::task` test set (96 tests, incl. all 34 shell-runner/process-tree tests) | **Green** — 96/96 | `0` | 2.53 s test time |
| Linux: full `claudine` lib suite (3710 tests) | **Green modulo 3 classified environment failures**, none process-tree | `101` | 4.02 s test time |
| Windows: `cargo check --target x86_64-pc-windows-gnu -p claudine --tests` | **Green — compile-only, NOT runtime verification** | `0` | 35.01 s |
| Windows: native behavioral run | **Not run — impossible on this host.** The gap stands. | — | — |

Host load context (macOS side, at recording time): `load averages: 9.52 12.72
40.09` on 16 cores. Both Linux test phases completed in single-digit seconds
with zero timing-sensitive failures, so no drift bracket was needed.

## Environment

**Host:** macOS 26.5.2 (build 25F84), Apple Silicon, 16 logical cores, 128 GiB
RAM. Docker Desktop VM: 7.75 GiB memory, aarch64.

**Container** (`uname -a`, `rustc --version`, `cargo --version` verbatim from
the container transcript):

```
Linux 718f57104eb6 6.12.76-linuxkit #1 SMP Thu Jun 25 13:45:40 UTC 2026 aarch64 GNU/Linux
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
```

**Image:** `rust:latest`, digest
`rust@sha256:9a2cd304a852f05d3352f75bc2775242371c0169a72dbb40d5d881379d571989`
(Debian). The repo's `rust-toolchain.toml` pins `channel = "stable"`, which the
image's 1.97.1 satisfies; the macOS host currently resolves stable to 1.96.0,
so the Linux run floats one stable release newer than the host. This is a real
Linux kernel (Docker Desktop's linuxkit VM), aarch64 — not x86_64.

**Procedure** (per the verified 2026-07-16 procedure): host-mounted cargo
target dir (`-v /tmp/claudine-linux-target:/t -e CARGO_TARGET_DIR=/t`) to avoid
container-overlay exhaustion; `--memory=7g` and `CARGO_BUILD_JOBS=2` to keep
the linker out of OOM range; `cargo test --lib` rather than nextest (single
test binary to link). Container deps installed non-interactively: `cmake`,
`pkg-config`, `libasound2-dev`, `libssl-dev`. Tests inside the container run as
**root**, which matters for one classified failure below.

## Gate 1 — Linux process-tree / shell-runner tests

The hard requirement of the finding: the shell/process-tree test modules in
`claudine/lib/src/composition/sequence/task/tests.rs` — `shell_tasks` (26
tests on Linux) and `shell_streaming` (8 tests) — executed on a real Linux
kernel, plus every other test the `composition::sequence::task` filter reaches
(no cherry-picking).

```
docker run --rm --memory=7g \
  -v /tmp/rb-linux-src:/src -v /tmp/claudine-linux-target:/t \
  -v /tmp/rb-cargo-registry:/usr/local/cargo/registry \
  -e CARGO_TARGET_DIR=/t -e CARGO_BUILD_JOBS=2 -w /src rust:latest \
  bash -c '… cargo test -p claudine --lib composition::sequence::task::tests …'
```

Verbatim summary:

```
running 96 tests
test result: ok. 96 passed; 0 failed; 0 ignored; 0 measured; 3614 filtered out; finished in 2.53s
FILTERED_EXIT=0
```

Composition of the 96 (the substring filter also matches the sibling
`tasks::tests` group modules, which is a superset, not a substitution):

| Module | Tests |
|---|---|
| `shell_tasks` | 26 |
| `shell_streaming` | 8 |
| `stages` | 8 |
| `side_effect_tasks` | 5 |
| `prompt_tasks` | 9 |
| `outcome_contract` | 5 |
| `group_framing` | 12 |
| `serial_groups` | 9 |
| `parallel_groups` | 14 |

### Finding-2 behavior → executed Linux test, all `ok`

Every behavior the finding names, mapped to the test that exercised it against
real processes on the Linux kernel above (verbatim result lines in the
transcript; all 34 `shell_tasks` + `shell_streaming` results were `ok`):

| Required behavior | Test (all `… ok`) |
|---|---|
| Success-path descendant cleanup | `shell_tasks::a_successful_command_reaps_its_backgrounded_descendant` |
| Timeout (direct child) | `shell_tasks::the_system_shell_kills_a_command_that_overruns_its_budget` |
| Timeout (nested tree) | `shell_tasks::the_system_shell_times_out_a_nested_tree` |
| Interrupt of a running tree | `shell_tasks::the_system_shell_interrupts_a_running_tree` |
| Runaway output (cap trip) | `shell_tasks::the_system_shell_aborts_a_command_that_floods_stdout` |
| Runaway byte counters | `shell_tasks::a_byte_limit_trip_carries_the_byte_counters` |
| Runaway line counters | `shell_tasks::a_line_limit_trip_carries_the_line_counters` |
| Ownership-establishment failure (typed) | `shell_tasks::a_failed_ownership_setup_is_a_typed_isolation_error` |
| Ownership failure kills the spawned command | `shell_tasks::a_failed_ownership_setup_kills_the_spawned_command` |
| Inherited-pipe closure (descendant holding stdout) | `shell_tasks::the_system_shell_kills_a_backgrounded_descendant_holding_stdout` |
| Wait-error settlement — tree reaped | `shell_tasks::an_early_wait_error_still_reaps_the_whole_tree` |
| Wait-error settlement — readers settle, nothing after footer | `shell_streaming::a_wait_error_settles_readers_before_returning_and_nothing_follows_the_footer` |
| Wait-error stage preserved in diagnostics | `shell_tasks::a_command_whose_wait_fails_is_reported_as_having_run` |
| Pipeline capture | `shell_tasks::the_system_shell_captures_a_pipeline` |
| Live streaming order/tearing/flush | `shell_streaming::*` (all 8) |

The last three wait-error rows exercise the uncommitted finding-3 fix
(`TaskShellError::Wait` + the cleanup epilogue): **the new code ran on Linux
and passed.**

**Green: 96/96, exit `0`.** No skips, no timeouts, no leaked-descendant
assertion fired.

## Gate 2 — Linux full `claudine` lib suite

Run in the same container immediately after Gate 1 (same binary):

```
cargo test -p claudine --lib
test result: FAILED. 3707 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
FULL_EXIT=101
```

3710 tests total — matching the macOS host's lib-test population. The 3
failures, verbatim:

```
failures:
    composition::resolve::tests::validate_permissions_readonly_file
    harness::audit::tests::audit_approved_command
    harness::audit::tests::audit_mixed_commands
```

**None of the three touches `SystemTaskShell` or the process tree.** Each was
discriminated rather than hand-waved:

### `validate_permissions_readonly_file` — root artifact, environment skip

The test chmods a file read-only and expects
`CompositionError::InsufficientFilePermissions`. The container runs tests as
**root**, and root bypasses mode bits. Discriminating re-run of the same
prebuilt test binary as a fresh non-root user (`useradd -m tester; su tester`):

```
test composition::resolve::tests::validate_permissions_readonly_file ... ok
```

Passes as non-root on the same Linux kernel. Classification: **environment
skip (container-as-root), not a behavioral failure.**

### `audit_approved_command`, `audit_mixed_commands` — host-config-sensitive tests, environment skip

Both assert that `echo hello` is approved under
`ShellApprovalOptions::default()`. With `policy_root: None`,
`resolve_policy_paths` resolves the **user-level darkmatter shell-policy store
under `$HOME`** — on Ken's host that store whitelists `echo`; a pristine
container has no store, so the command is denied
(`no approval handler and not whitelisted, denying command`). Discrimination:

- Non-root container user with a fresh `$HOME`: **still fails** → not a root
  artifact.
- Same two tests on the macOS host against the same tree:
  `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3708 filtered out; finished in 0.02s`
  → not a code regression on this tree.

Classification: **environment skip (test depends on the developer's persisted
whitelist in `$HOME`), not a behavioral failure — and not process-tree
related.** Worth recording as a latent test-portability defect for whoever owns
`harness/audit.rs` tests: the `permissive_options()` helper's comment ("no
policy root … → commands on PATH are approved") does not match behavior in a
home directory with no whitelist store. Per this file's scope, nothing was
fixed.

## Gate 3 — Windows compile evidence (compile-only, NOT a behavioral run)

Run on the macOS host from the worktree root, with the load-bearing env
workarounds from `claudine/justfile::check-windows` (a first attempt without
them failed inside `aws-lc-sys`: the host's `kache` rustc-wrapper leaks into
cc-rs as a compiler launcher — the recipe's `RUSTC_WRAPPER=""` is required, as
its comment says):

```
RUSTC_WRAPPER="" \
CARGO_TARGET_DIR=target/windows-check \
CFLAGS_x86_64_pc_windows_gnu="-Wa,-mbig-obj" \
CXXFLAGS_x86_64_pc_windows_gnu="-Wa,-mbig-obj" \
cargo check --target x86_64-pc-windows-gnu -p claudine --tests
```

Verbatim result:

```
warning: `claudine` (lib test) generated 16 warnings (2 duplicates) (run `cargo fix --lib -p claudine --tests` to apply 10 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 35.01s
WIN_CHECK_EXIT=0
```

The 16 warnings are dead-code/unused warnings (helpers used only by
`cfg(unix)` tests appear unused under the Windows cfg) — not errors.

**This is compile-only evidence.** It proves the Windows Job Object arms
(`termination/windows.rs`), the suspended-assign/resume path, the
`TaskShellError::Wait` plumbing, and the `#[cfg(windows)]` twins of the
process-tree tests still type-check on the current tree. It proves **nothing**
about `GenerateConsoleCtrlEvent`, `TerminateJobObject`, kill-on-close, or any
runtime behavior. It must not be cited as runtime verification.

## What this run does and does not certify

### Does certify

- **The process-tree contract holds on a current real Linux kernel
  (6.12.76, aarch64) against the current tree including the uncommitted
  finding-3 fix**: success-path descendant reap, timeout (direct and nested),
  interrupt, runaway-output caps with typed counters, fail-closed ownership
  failure, inherited-pipe closure, and wait-error settlement (tree reap +
  reader settlement + footer ordering) — 34/34 shell tests green, 96/96 for
  the whole task filter, exit `0`.
- **The broader `claudine` lib suite is green on Linux modulo 3 classified
  environment failures** (1 root artifact, 2 `$HOME`-whitelist artifacts),
  each discriminated by re-run rather than asserted.
- **The Windows arms of the current tree compile** under
  `x86_64-pc-windows-gnu`, exit `0`.

### Does not certify

- **Native Windows runtime behavior. This gap is still open.** No Windows
  behavioral run exists for the rewritten ownership contract; this host cannot
  produce one (macOS + Docker Linux only; the `duckdb`/mingw cross-compile
  wall also blocks a Wine-shaped shortcut, and faking it would be worse than
  the gap). Job creation/configuration, suspended assignment, thread
  discovery/resume, termination, and the `#[cfg(windows)]` twins have **never
  executed**. Closing review-7 finding 2 for Windows requires a native Windows
  host.
- **x86_64 Linux specifically.** The kernel is real but the architecture is
  aarch64. Process groups, signals, and pipe semantics are
  architecture-independent kernel interfaces, so this is a footnote rather
  than a hole — but it is recorded.
- **L2/L3 tiers on Linux.** This run is the L1 lib suite only; no terminal
  harness exists in the container.
- **The 2 audit tests and 1 permissions test in a pristine-\$HOME / root
  environment.** They are classified environment failures, not green.
