# Phase 4 Process and Executable Classification

## Scope

This pass covered the native-Windows failures in the process-spawn,
termination, system-shell, model-catalog subprocess, inline-compose hash, and
wrap Ctrl+C tests assigned to the process cluster. Unix behavior was preserved,
and no test was silently disabled.

GitNexus could not resolve the private functions or tests in this cluster and
reported `UNKNOWN` risk rather than `HIGH` or `CRITICAL`. A direct caller
inventory found one production caller of `system_shell_command`, three spawn
paths calling `debug_assert_child_env`, and otherwise test-local helpers and
fixtures.

## Classification

| Failure | Classification | Resolution |
| --- | --- | --- |
| Nested quotes changed before `cmd.exe` received a system-shell command | Product defect | Pass the command tail with Windows `raw_arg` after `cmd /D /C`; added a focused nested-quote regression. |
| Spawn debug assertion rejected `Path` and `USERPROFILE` | Product debug-invariant defect | Match Windows environment keys case-insensitively and accept `USERPROFILE` as the Windows home variable; added a focused regression. |
| Model-catalog smoke test invoked `echo` as an executable | Fixture defect | Use `cmd /D /C` on Windows, where `echo` is shell syntax. |
| System-shell timeout tests resolved an incompatible `timeout.exe` from `PATH` | Fixture defect, plus the nested-quote product defect above | Resolve `%SYSTEMROOT%\System32\PING.EXE` directly for deterministic long-running Windows process trees. |
| Spawn tests assumed `/tmp`, `/bin/echo`, `/usr/bin/env`, and Unix `true` paths | Fixture defect | Use the host temporary directory and platform shell commands with a viable child environment. |
| Wait tests resolved an incompatible `timeout.exe` from `PATH` | Fixture defect | Invoke `%SYSTEMROOT%\System32\PING.EXE` directly, with stdio disconnected. |
| Job-close ownership test sometimes observed an empty descendant marker | Windows process-fixture race | Replace the `cmd.exe`/`start`/`ping` fixture with a native Rust parent/descendant handshake. A PID-scoped test hook signals the parent immediately after Claudine's Job assignment succeeds, and a separate descendant-ready marker proves the descendant ran before the parent exited. The inherited marker handle denies delete sharing, so deletion proves the descendant released it rather than relying on Rust's permissive default Windows share mode. |
| Windows integration providers used `.cmd` files for multiline arguments and process-tree semantics | Fixture defect | Compile tiny native provider executables with the test host's `rustc`; this keeps the tests on the intended provider and termination seams. |
| Inline hash test hard-coded `target/debug/md` and omitted `.exe` | Fixture defect | Resolve `CARGO_BIN_EXE_md` when present, otherwise derive the active profile directory from the integration-test executable and append `EXE_SUFFIX`. |

## Verification

- Library process cluster: 4 passed (`system_shell` nested quotes, descendant
  timeout, nested-tree timeout, and model-catalog subprocess parsing).
- CLI spawn and wait cluster: 8 passed (environment invariant, captured and
  inherited spawn paths, and both Windows wait/termination paths).
- The job-close test reproduced its fixture race after 10 consecutive passes
  in a 20-iteration stress run, failing on iteration 11 because the descendant
  had not written before correct kill-on-close teardown. After the native
  handshake replacement and exclusive marker-handle proof, the same CI-profile,
  no-retry stress run passed 20/20.
  Production Job Object creation, assignment, termination, and handle ownership
  logic was untouched.
- Integration cluster: 2 passed (`inline_compose_writes_hash_that_passes_md_diff`
  and `ctrl_c_terminates_wrapped_child_on_windows`).

The only platform gates retained are explicit tests of named platform
facilities: Unix shell/process behavior remains `cfg(unix)`, while the native
Windows counterparts remain `cfg(windows)`.
