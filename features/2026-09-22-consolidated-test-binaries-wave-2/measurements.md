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

## Phase 4

Same protocol and host. The before checkout was a detached worktree of
`5a396aeb5` (`/tmp/w2-base`, removed afterward). Each package was built with its
CI feature union.

| Package | Features | Test executables | Executable bytes | Clean test build | Warm edit (test) | R8 trigger |
|---|---|---:|---:|---:|---:|---|
| `biscuit-file` before | `fetch` | 15 | 107.3 MB | 17 s | 1.20 s (`toml_json_round_trip_basic`) | |
| `biscuit-file` after | `fetch` | **2** | **39.9 MB** | 17 s | 2.09 s | no (+0.89 s) |
| `schematic-gen` before | `terminal-tests` | 14 | 178.2 MB | 39 s | 1.46 s (`single_path_param_struct_has_field`) | |
| `schematic-gen` after | `terminal-tests` | **2** | **60.5 MB** | 41 s | 1.38 s | no (−0.08 s) |
| `biscuit-terminal-cli` before | `terminal-tests` | 14 | 47.3 MB | 57 s | 1.50 s (`test_about_kitty_plain_renders_report`) | |
| `biscuit-terminal-cli` after | `terminal-tests` | **2** | **12.8 MB** | 53 s | 1.55 s | no (+0.05 s) |
| `claudine-gen` before | `terminal-tests` | 11 | 322.1 MB | 99 s | 1.55 s (`every_research_vocabulary_projects_to_runtime_strings`) | |
| `claudine-gen` after | `terminal-tests` | **2** | **198.3 MB** | 72 s | 1.77 s | no (+0.22 s) |
| `dmls` before | `terminal-tests,effects-instrumentation` | 11 | 286.9 MB | 91 s | 1.19 s (`dist_recipe_and_zed_extension_agree_on_archive_names`) | |
| `dmls` after | `terminal-tests,effects-instrumentation` | **2** | **59.5 MB** | 75 s | 1.71 s | no (+0.52 s) |

Across the five packages, the test executables go from 65 to 10, and from
941.8 MB to 371.0 MB (−60.6%). No package reached the R8 trigger. The same
caveat applies as in Phase 3: each edit figure is one warm observation on a
loaded, shared host, and the after runs overlapped other local builds.

`measure.sh` counted 0 executables for `dmls` on its first before run. Cargo
drops the name from a package ID whose directory has the same name
(`…/darkmatter/dmls#0.1.0`), and the script matched only `#dmls@`. The script
now matches both spellings, and the `dmls` before run was repeated. The
figures from Phase 3 are unaffected, because none of those packages has the
short spelling.

## Phase 5

Same protocol and host. The before checkout was a detached worktree of
`8255ee228` (`/tmp/w2p5-base`, removed afterward). Each package was built with its
CI feature union.

| Package | Features | Test executables | Executable bytes | Clean test build | Warm edit (test) | R8 trigger |
|---|---|---:|---:|---:|---:|---|
| `sniff-cli` before | `test-fixtures` | 11 | 75.3 MB | 84 s | 3.47 s (`install_plan_vim_renders_text_output`) | |
| `sniff-cli` after | `test-fixtures` | **2** | **50.7 MB** | 84 s | 3.33 s | no (−0.14 s) |
| `biscuit-tui-cli` before | `terminal-tests` | 15 | 32.8 MB | 9 s | 1.10 s (`top_level_help_uses_canonical_public_names`) | |
| `biscuit-tui-cli` after | `terminal-tests` | **3** | **10.3 MB** | 7 s | 1.18 s | no (+0.08 s) |

Across the two packages, the test executables go from 26 to 5, and from 108.1 MB to
61.0 MB (−43.6%). Neither package reached the R8 trigger. The same caveat applies:
each edit figure is one warm observation on a shared host.

Across all ten packages of this wave (Phases 3–5), the test executables go from 136 to
18.
