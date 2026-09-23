---
kind: measurements
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
harness: measure/measure.sh
---

# Measurements — target count, executable bytes, and warm edit latency

Spec §1 replaces the first feature's criterion 9 with a lightweight
observation per package. Rulings R8 set the trigger for the full protocol.

## Protocol (R8)

`measure/measure.sh <package> <tests-dir> <series> <out-dir> <edit-test> [features]`
runs the same commands for both series. Only the checkout differs.

| Condition | Value |
|---|---|
| Host | macOS `aarch64-apple-darwin`, Apple M4 Max, 16 cores (shared dev host, never idle) |
| Toolchain / nextest | rustc 1.98.1 / cargo-nextest 0.9.136 |
| Compiler wrapper | kache off: `RUSTC_WRAPPER=""` (an unset variable does not override `~/.cargo/config.toml`), and kache's `cc` shims removed from `PATH` |
| Build | `cargo test --no-run --locked -p <pkg> [--features <CI set>]` into a fresh `mktemp` target dir per series |
| Count / bytes | `compiler-artifact` messages of kind `test` for the package, and the sum of their executables' sizes |
| Edit loop | `touch` the file defining `<edit-test>` (found by content, so it follows the move), then `cargo nextest run --locked -p <pkg> [--features …] -E "$(just _tier_filter L1 <pkg>)" --no-tests=fail <edit-test>`. One untimed warm-up, then one timed run |
| Before checkout | a detached worktree of `c02e691d9` (`/tmp/w2-base`, removed afterward) |
| After checkout | this worktree with the package's uncommitted move |
| Trigger | the full five-trial protocol runs only if the after edit time exceeds the before by more than 50% **and** more than 5 s |

Raw output: `measure/<package>-{before,after}.txt` and `measure/<package>-{before,after}-edit.log`.

## Phase 3

| Package | Features | Test executables | Executable bytes | Clean test build | Warm edit (test) | R8 trigger |
|---|---|---:|---:|---:|---:|---|
| `tree-hugger` before | none | 10 | 264.0 MB | 11 s | 1.14 s (`test_rust_lint_query_compiles`) | |
| `tree-hugger` after | none | **1** | **52.5 MB** | 11 s | 1.35 s | no (+0.21 s) |
| `claudine` before | none | 15 | 846.9 MB | 97 s | 1.68 s (`wire_greet_fixture_replays_end_to_end`) | |
| `claudine` after | none | **1** | **220.8 MB** | 95 s | 2.06 s | no (+0.38 s) |
| `sniff` before | `remote` | 20 | 377.9 MB | 63 s | 1.34 s (`programs_info_serialization_roundtrip`) | |
| `sniff` after | `remote` | **1** | **89.4 MB** | 62 s | 1.90 s | no (+0.56 s) |

`sniff`'s 20 before-executables include the zero-test `fixtures` target that
R19 drops.

Across the three packages, the test executables go from 45 to 3, and from
1,488.8 MB to 362.6 MB (−75.6%). No package reached the R8 trigger, so none
needed the full protocol. Each edit figure is a single warm observation on a
loaded, shared host (1-minute load between 3.8 and 8.7), so a difference under
a second is within noise.
