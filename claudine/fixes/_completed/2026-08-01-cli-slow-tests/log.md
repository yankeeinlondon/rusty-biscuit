---
fix: 2026-08-01-cli-slow-tests
implementation_3: "2026-09-07T04:52:47-07:00"
deferred_perf_measurement: true
---

# Implementation log — `2026-08-01-cli-slow-tests`

## Implementation of Review Findings #3

> **started at:** 2026-09-07T04:52:47-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-08-01-cli-slow-tests/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review contains **9 findings**, ordered by the review's own "Recommended order of work"
        - finding 1 (critical) — the environment-inheritance contract (RB 1's fourth bullet) is unimplemented; four families leak
        - finding 2 (high) — acceptance criterion 4 has no CI evidence
        - finding 3 (high) — the `#[cfg(windows)]` arms cannot be compiled on this host
        - finding 4 (medium) — a third live spawn form (`bin_exe!`) is invisible to the guard
        - finding 5 (medium) — isolation can be defeated after `command()`, and the guard is blind to it
        - finding 6 (medium-low) — the burn-down roll-up never reaches a CI log
        - finding 7 (low) — the `TMPDIR`-inside-the-checkout hazard needs an intentional guard
        - finding 8 (low) — `IN_SCOPE` is dead vocabulary kept alive by its own unit test
        - finding 9 (low) — the structural gates carry the fixture's whole dependency surface
- orchestration shape: one subagent per finding, dispatched serially, each running `just test-cli` and `just lint` from the `claudine` package area
- findings 8 and 9 are dispatched together: both are single-edit changes to `spawn_site_guard.rs`, and splitting them would have two agents editing the same file

### Finding 1 — environment-inheritance contract

- starting the work on 'finding 1 — environment-inheritance contract' at 04:55:19-07:00
        - **step 1 — renamed the probe control.** `CLAUDINE_PROBE_CONTROL` → `FIXTURE_PROBE_CONTROL` in `cli/tests/cli_process_fixture.rs` (the `sh` stub, the `.cmd` stub, both `set_var` sites, the assertion). Done *before* the scrub landed, so the "the parent's environment really does reach the child" control does not itself become a casualty of the `CLAUDINE_*` prefix sweep. `CLAUDINE_PROBE_CAPTURE` was left alone: `run_probe` sets it *after* `build()`, which is the per-key ordering the contract promises — that made every existing fixture test a live witness for the ordering rule, and the stub's doc comment now says so.
        - **step 2 — the scrub.** New `common::scrub_inherited_environment` in `cli/tests/common/mod.rs`, called from `ClaudineCommandBuilder::build` *before* the builder's own defaults so `CLAUDINE_RENDEZVOUS_REPORT=false` survives its own namespace sweep.
                - `CLAUDINE_*` by prefix over `std::env::vars_os()`, not by enumeration (review 3 counted 48 distinct names, and the spec's list names `CLAUDINE_OPTIONS`, which is a usage-string placeholder in `cli/src/argv/partition.rs` rather than an env var)
                - the five `GIT_*` plumbing keys by name, hoisted into a `GIT_PLUMBING_VARS` constant so the parent-side `git` hardening in step 5 shares one list and one WHY
                - `TERM_WIDTH`/`COLUMNS`/`FORCE_COLOR` **removed** rather than pinned, so claudine's documented 80-column fallback in `cli/src/log.rs` applies and `FORCE_COLOR` cannot out-vote the builder's `NO_COLOR=1`. Removal was chosen over pinning because the three surviving call sites pin three *different* widths — see the step-4 decisions below.
                - module docs gained an `### The inheritance contract` section naming the three families and the per-key ordering rule; `build()` gained a doc comment naming the two fixture keys that live inside a scrubbed namespace.
        - **step 3 — tests.** The recording stub (both the `sh` and `.cmd` forms) now reports `CLAUDINE_STEP_TIMEOUT`, `GIT_DIR`, `GIT_WORK_TREE`, `TERM_WIDTH`, `COLUMNS`, `FORCE_COLOR`. Four new tests in `cli_process_fixture.rs`, each pairing its assertion with the live `FIXTURE_PROBE_CONTROL` control through a new `assert_parent_environment_reached_the_child` helper, and each setting its sentinel in the *parent* under the existing `unsafe { set_var }` + process-per-test-under-nextest reasoning:
                - `default_command_drops_the_inherited_claudine_namespace`
                - `default_command_drops_the_inherited_git_plumbing_family`
                - `default_command_drops_the_inherited_render_inputs`
                - `a_scrubbed_key_set_after_build_still_reaches_the_child`
        - **non-vacuity transcripts** (each neuter applied to the production helper, run, then restored):
                - neuter A — deleted the `GIT_PLUMBING_VARS` loop from `scrub_inherited_environment`: `5 tests run: 4 passed, 1 failed`, `FAIL … default_command_drops_the_inherited_git_plumbing_family`, `assertion left == right failed: an inherited GIT_DIR reached the child and would relocate repository discovery`. Restored → 5/5 pass.
                - neuter B — deleted the `CLAUDINE_` prefix loop: `5 tests run: 4 passed, 1 failed`, `FAIL … default_command_drops_the_inherited_claudine_namespace`, `assertion left == right failed: an inherited CLAUDINE_* variable reached the child`. The after-build test stayed green under this neuter, which is correct — it asserts a different property. Restored → 5/5 pass.
                - neuter C — deleted the `["TERM_WIDTH", "COLUMNS", "FORCE_COLOR"]` loop: `5 tests run: 4 passed, 1 failed`, `FAIL … default_command_drops_the_inherited_render_inputs`, `assertion left == right failed: an inherited TERM_WIDTH reached the child and would reshape its rendering`. Restored → 5/5 pass.
                - neuter D — `a_scrubbed_key_set_after_build_still_reaches_the_child` has **no production-side neuter**: `std::process::Command` keys its env ops by name, so "a later `.env` wins" is structural and cannot be broken by editing the scrub. Neutered on the input side instead — deleted the call site's `.env("CLAUDINE_STEP_TIMEOUT", "0.5s").env("TERM_WIDTH", "200")` — and it went red: `left: "[]" right: "[0.5s]"`, `a CLAUDINE_* value chosen after build() must out-rank the scrub`. Restored → pass. Recorded plainly because it is a weaker demonstration than A–C.
        - **step 4 — the surviving manual render-input pins.** The review's premise was partly wrong, as suspected; decided site by site.
                - `argv_normalization.rs:217,268` (`TERM_WIDTH=120`) — **dropped**. Removed the pin and ran both tests: `headline_compose_with_setter_then_late_flags_preserves_flag_semantics` and `…_before_late_flags_…` both PASS at the builder's 80-column fallback. Their assertions are substring checks on provider name, the YOLO row, and the rendered body, none width-dependent.
                - `command_routing.rs:60` (`TERM_WIDTH=160`) — **dropped**. `hooks_support_command_routes_without_detected_agents` PASSes at 80. Its load-bearing assertion is `!stdout.contains("Table could not be rendered")`, i.e. the support view chunks rather than refusing — which 80 columns exercises *harder* than 160, so the pin was strictly a leak workaround.
                - `wrap_basics.rs:266` (`TERM_WIDTH=200`) — **kept**, exactly as the plan's AC 5 deviation note predicted. Removing it fails `wrapper_reports_removed_sensitive_env_names` with a host-specific snapshot: `-• PWD=<workspace>/cwd` becomes `+• PWD=/private/var/folders/…/wrap-basics-sensitive-env-…/cwd` wrapped across two lines, plus two more wrapped lines in the info banner and the system-prompt block. A wrapped path cannot be redacted to `<workspace>`, so the snapshot would then differ between macOS and Linux. No snapshot was regenerated. The call-site comment was rewritten to read as a deliberate render-width choice ("not an isolation workaround: the builder already removes an inherited `TERM_WIDTH`…") so a future reader does not delete it as redundant.
                - `command_routing.rs:289,307` (`.env_remove("FORCE_COLOR")`) — **both dropped** as redundant; the builder removes the key at build time. The sibling `.env_remove("NO_COLOR")` at 307 stays, with its comment: `NO_COLOR` is a builder *default*, not an inherited value, and `--plain` is that test's subject.
        - **step 5 — parent-side `git` hardening.** `common::init_git_repo` is the single funnel every fixture repository goes through (`common/wrap.rs:59`'s `create_claudine_monorepo` calls it; no other `Command::new("git")` exists in `common/`), so scrubbing `GIT_PLUMBING_VARS` there covers the whole parent-side surface. It carries a doc comment naming the 2026-08-31 incident by reference to the constant, which is where the incident is written out once.
        - **step 6 — leak probes, before → after** (all from the `claudine` package area, with `/tmp/gitprobe` created as a real repository first):
                - `CLAUDINE_TIMEOUT=0.3s just test-cli watchdog` — before: `64 passed, 3 failed` (`watchdog_subagent_hang_terminates_and_names_stuck_ids`, `watchdog_opencode_post_fanout_silence_does_not_kill_prematurely`, `watchdog_stream_idle_timeout_after_tool_call_hang`) → after: **67 passed**
                - `CLAUDINE_STEP_TIMEOUT=0.3s just test-cli watchdog` — before: **67 passed** already (those tests set `CLAUDINE_STEP_TIMEOUT` themselves at the call site, so the inherited value was being overridden anyway; review 3 listed this probe but its leak surfaced through `CLAUDINE_TIMEOUT`) → after: 67 passed
                - `GIT_DIR=/tmp/gitprobe/.git GIT_WORK_TREE=/tmp/gitprobe just test-cli ambient` — before: `3 passed, 1 failed` (`ambient_context_escape_pins_the_cwd_to_a_test_built_repository`, `left: …/repo/claudine/cli  right: …/repo`) → after: **4 passed**
                - `FORCE_COLOR=1 just test-cli non_tty_withholds_yaml` — before: `0 passed, 1 failed` → after: **1 passed**
                - `COLUMNS=44 just test-cli` — before: `19 passed, 5 failed`, all five in `characterization_error_routes`, the run cancelled at test 24 of 2397 by nextest's fail-fast → after: the five `characterization_error_routes` failures are **gone**; the run now reaches test 201 and stops on two *different* failures, `compose_schema_cli::inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub` and `composition_outputs::a_loop_accumulates_outputs_and_retains_mutations_across_iterations`. Both files are `SPAWN_ALLOWLIST` entries — explicitly outside this fix's scope — and neither mentions `CliProcessFixture`, so the builder change cannot have touched them; they were simply never reached in the "before" run. Recorded as a pre-existing leak in unmigrated files, not a regression, and not fixed here.
        - **verification** (all from the `claudine` package area, `just` recipes only, no bare `cargo test`, no `cargo fmt`):
                - `just test-cli` — **green**: `2401 tests run: 2401 passed, 10 skipped` in 15.1 s (2397 before; +4 new fixture self-tests)
                - `just lint` — **green** across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`, including the lifecycle-doc-facets guard
                - `just test-l2` — **green**: `claudine-cli` `230 tests run: 230 passed, 2415 skipped` (53.1 s) and `claudine-gen` `3 tests run: 3 passed`. The known host flake `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` did not fire on this run.
        - files touched: `claudine/cli/tests/common/mod.rs` (module docs, `GIT_PLUMBING_VARS`, `scrub_inherited_environment`, `build()` docs + call, `init_git_repo`), `claudine/cli/tests/cli_process_fixture.rs` (rename, stub keys, 4 new tests + control helper), `claudine/cli/tests/argv_normalization.rs` (2 pins dropped), `claudine/cli/tests/command_routing.rs` (1 pin + 2 `FORCE_COLOR` removals dropped), `claudine/cli/tests/wrap_basics.rs` (comment only; the pin stays)
        - not done here, and named rather than silently skipped: the two `COLUMNS=44` failures in `compose_schema_cli.rs` and `composition_outputs.rs`. Both are allow-listed, unmigrated files; closing them means migrating those files to the builder, which is Required behavior 2 scope, not finding 1's.
- work completed for 'finding 1 — environment-inheritance contract' at 05:06:59-07:00

### Finding 4 — the `bin_exe` spawn form

- starting the work on 'finding 4 — the bin_exe spawn form' at 05:10:50-07:00
        - **baseline roll-up, before any change** (`just test-cli l1_tests_spawn --success-output immediate`): `spawn-site burn-down: 169 raw sites in 35 allow-listed files (of 169 sites scanned)`, one roll-up line, `== outside this fix's scope: 35 file(s), 169 site(s)`. `wrap_ctrl_c_windows.rs` appears nowhere in it, exactly as review 3 reported.
        - **step 1 — the detector arm.** Added `FORM_BIN_EXE` (`"bin_exe"`) and a `b"bin_exe"` arm to `spawn_sites`. The review's premise that `names_claudine` "works unchanged" is right about the *helper* but not about the *call*: `bin_exe!("claudine")` puts a `!` between the identifier and the `(`, and `names_claudine` skips whitespace and then demands `(`, so calling it at `end` returns false. Added `macro_names_claudine`, which steps over the bang and then delegates — the arm is otherwise the `cargo_bin` arm verbatim, reading the *original* bytes because `sanitize` blanks the literal.
        - the guard's module docs gained the third form alongside the other two; the `FORM_BIN_EXE` const documents it as "fed to a raw `Command::new`", which is what makes it a spawn rather than a path lookup.
        - **step 2 — detector tests.** `detector_finds_both_spawn_forms_as_executable_code` was renamed `detector_finds_every_spawn_form_as_executable_code` (there are three forms now) and gained the macro form plus a `bin_exe ! ( "claudine" )` spacing case. `detector_ignores_prose_neighbors_and_other_binaries` gained five negatives: a `//!` doc-comment mention, a string-literal mention, `bin_exe!("md")` (darkmatter's shim is a legitimate spawn), the identifier-boundary pair `harness_bin_exe!` / `bin_exe_path!`, and `bin_exe("claudine")` without the `!` — a plain call of that name is something else.
        - **step 3 — the `wrap_ctrl_c_windows.rs` decision: allow-list, not migration.** Verified the review's `CREATE_NEW_PROCESS_GROUP` claim rather than taking it, and found a second, larger blocker it did not name.
                - read `assert_cmd` 2.2.1's own source (`~/.cargo/registry/.../assert_cmd-2.2.1/src/cmd.rs`). `Command` exposes `from_std`, `cargo_bin`, `write_stdin`, `timeout`, `pipe_stdin`, `ok`, `unwrap`, `unwrap_err`, `assert`, `new`, `arg`, `args`, `env`, `envs`, `env_remove`, `env_clear`, `current_dir`, `output`, and the four `get_*` accessors. There is **no `spawn`**, no `into_std`/`as_std_mut`, and the inner `process::Command` is private — `From<process::Command>` is one-way. So `CommandExt::creation_flags` is unreachable, confirming the review.
                - the larger blocker: the test must hold a **live `std::process::Child`**. It spawns, polls for a readiness marker, fires `GenerateConsoleCtrlEvent` at the running child's pid, then polls `try_wait` for 15 s and `kill`s on timeout. Every `assert_cmd` exit runs the child to completion first, so the process would already be dead before the console event could be sent. This is not a formatting mismatch; it is structural.
                - conclusion: no cheap safe migration exists, and a half-migration would be worse than none. Allow-listed with `NEEDS_LIVE_CHILD` = `"needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither"`, whose doc comment names both blockers and states the prerequisite for closing it (a fixture builder able to yield a `std::process::Command`, not a call-site migration).
                - `sequence_ctrl_c_windows.rs` was pointed at the same const. It has the identical blocker — `Command::new(common::claudine_bin())` + `.creation_flags(...)` + `.spawn()` + `try_wait` — and its previous `OUT_OF_SCOPE` reason told a future burn-down nothing about why. The two Windows console-control tests now roll up under one honest reason instead of being scattered through the generic bucket.
        - **the environment hazard in that file is real and I did NOT tighten it.** The child at `wrap_ctrl_c_windows.rs:135-148` gets a full host `PATH` (fixture `bin` first, then everything installed) and an unscrubbed `CLAUDINE_*` / `GIT_*` environment. Assessed for this specific test:
                - `CLAUDINE_*` is the dangerous arm, and it can produce a **silent false pass**: an inherited `CLAUDINE_TIMEOUT` short enough makes the wrapper terminate its own child inside the 15 s window, so `exited == true` is observed without `CTRL_BREAK_EVENT` having done anything. The assertion would be green while proving nothing.
                - `GIT_DIR`/`GIT_WORK_TREE` defeat the `current_dir(workspace)` anchor and walk claudine's repository discovery elsewhere — the same family as the 2026-08-31 incident. Failure mode here is loud, not silent: the 30 s readiness-marker poll panics with its own message.
                - host `PATH` is the mildest: `opencode` is shadowed by the fixture `bin` entry that precedes it, so the residual exposure is whatever *else* the wrapper shells out to.
                - live where? On the `windows-latest` CI leg the parent environment is clean, so no. On a developer's Windows host — or under a git hook, which is exactly how the `GIT_*` incident happened — yes.
                - why untouched: the file is `#[cfg(windows)]` and, per review 3's finding 3, cannot be compiled on this host at all (`aws-lc-sys` wants the Windows SDK headers, so `cargo check --target x86_64-pc-windows-msvc` never reaches the test crate). Editing it here would add unverifiable Windows-only code to a fix whose Windows arms have never been compiled anywhere, trading a latent hazard for a probable red leg. The right fix is the scrub the fixture already implements (`scrub_inherited_environment`), which becomes reachable at the same moment the allow-list reason's prerequisite is met. Recorded as a follow-up, not silently skipped.
        - **non-vacuity transcripts** (three neuters, each applied then restored):
                - neuter A — deleted the `wrap_ctrl_c_windows.rs` allow-list entry, leaving the new detector arm in place: `FAIL … l1_tests_spawn_claudine_through_the_fixture_builder`, "Raw `claudine` spawns outside the fixture builder …", `Unlisted sites:` / `wrap_ctrl_c_windows.rs:130 bin_exe` — the exact file, line, and form review 3 named. Restored → green.
                - neuter B — kept the entry, disabled the arm (`b"bin_exe" if false && …`): two failures, and they are the two the mechanism promises. `detector_finds_every_spawn_form_as_executable_code`: `left: []  right: [(1, "bin_exe")]`. `l1_tests_spawn_claudine_through_the_fixture_builder`: `Stale SPAWN_ALLOWLIST entries name files with no raw spawn site left.` / `Stale entries:` / `wrap_ctrl_c_windows.rs`. This is the stronger proof — it shows the *arm*, not the entry, is what surfaces the file, and that the stale-entry rule keeps the pair honest in both directions. Restored → green.
                - neuter C — loosened the argument check (`Some(&b'!') && (true || names_claudine(…))`): `FAIL … detector_ignores_prose_neighbors_and_other_binaries`, `assertion failed: spawn_sites("biscuit_test_harness::bin_exe!(\"md\");\n").is_empty()`. The `names_claudine` delegation is load-bearing, so darkmatter's `md` shim stays legitimate. Restored → green.
        - **roll-up, before → after**: `169 raw sites in 35 allow-listed files (of 169 sites scanned)` → `170 raw sites in 36 allow-listed files (of 170 sites scanned)`. The reason roll-up split from one line into two: `== needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither: 2 file(s), 2 site(s)` and `== outside this fix's scope: 34 file(s), 168 site(s)`. The census under-report review 3 measured is closed.
        - **verification** (from the `claudine` package area, `just` recipes only, no bare `cargo test`, no `cargo fmt`):
                - `just test-cli` — **green**: `2401 tests run: 2401 passed, 10 skipped` in 13.7 s. The count is unchanged because the work added assertions to two existing tests rather than new test functions.
                - `just lint` — **green** (exit 0) across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`, including the lifecycle-doc-facets guard.
                - formatting: `just lint` does not run rustfmt in this repo, and a standalone `rustfmt --edition 2024 --check` reports pre-existing diffs in `common/pty.rs`, `common/source_scan.rs`, and `spawn_site_guard.rs:370` (the untouched `scan` body), so plain rustfmt is not this repo's gate. Even so, both of my new lines that rustfmt wanted broken (`fn_call_width`) were written in its preferred form, leaving `spawn_site_guard.rs` with exactly the one pre-existing diff it started with.
        - files touched: `claudine/cli/tests/spawn_site_guard.rs` only (module docs, `FORM_BIN_EXE`, `macro_names_claudine`, the `b"bin_exe"` arm, `NEEDS_LIVE_CHILD`, two allow-list entries, two detector tests). `wrap_ctrl_c_windows.rs` deliberately unchanged.
- work completed for 'finding 4 — the bin_exe spawn form' at 05:15:22-07:00

### Finding 3 — the Windows arms

- starting the work on 'finding 3 — the Windows arms' at 05:17:32-07:00
        - **scope.** Finding 3 has two halves. The *compile* half — proving the `#[cfg(windows)]` arms type-check — is **deferred to the `windows-latest` CI leg**, not attempted here: `cargo check --target x86_64-pc-windows-msvc -p claudine-cli --tests` cannot run on this host because a transitive `aws-lc-sys` build script needs `windows.h` from the Windows SDK, so the check never reaches the test crate. The target is installed; the SDK is not, and installing it is not something a macOS host can do. Everything below is the half that a static read plus a macOS run *can* settle.
        - **step 1 — re-derived the Windows-compiled roster rather than trusting the review.** Scanned all 113 files in `claudine/cli/tests` for a `#![cfg(unix)]` file gate. Of the 29 migrated binaries, exactly 13 compile on Windows, and they are the 13 review 3 named — `inline_compose_hash`, `mcp_cli`, `shipped_prompt_contract`, `wrap_compose_validation`, `wrap_inline_compose`, `wrap_basics`, `command_routing`, `argv_normalization`, `hooks_cli`, `contextual_errors`, `inline_compose_sequence_mismatch`, `handle_repo_config`, `characterization_error_routes`. Then went one level finer than the file gate, because an ungated file can still have all its risky code gated per item: enumerated every `#[test]` in those 13 with its enclosing `#[cfg]` (including module-level gates, which a naive per-item scan misses — `wrap_inline_compose.rs`'s four `opencode_model_integration` tests are gated on the `mod`, not on the `fn`). That left 63 genuinely Windows-compiled tests to audit, not 13 files' worth.
        - **step 2 — the bare-name audit.** Result: **clean. Nothing needed fixing, and no `#[cfg]` gate was changed.** The three sanctioned remedies were already applied everywhere they were needed, by whoever migrated each file. Two facts about the production side anchor the table:
                - on Windows claudine spawns exactly one utility by bare name, `cmd` (`lifecycle/executor.rs:291`, `sequence/task/shell.rs:891`), which `%SystemRoot%\System32` resolves. Every `Command::new("git")` in `claudine/lib` and `claudine/cli` is inside a `#[cfg(test)]` unit-test module that runs in-process with the runner's full `PATH`, never in a path the child binary takes.
                - darkmatter's `::shell` executor resolves the program itself and spawns an absolute path (`shell_expansion/executor.rs:199,471`), so a `::shell rustc …` directive resolves `rustc` against the child's `PATH` — the fixture `bin` — rather than needing a shell to find it.

                | File | What it invokes on the Windows leg | Resolves? | Action |
                |---|---|---|---|
                | `argv_normalization.rs` | nothing; two tests take `fake_only_path()` | n/a | none |
                | `characterization_error_routes.rs` | `claude.cmd` stub via `shim_name()`/`provider_shim()`; the `::shell rm -rf /` document is `#[cfg(unix)]` | yes — fixture `bin` | none |
                | `cli_process_fixture.rs` | `claude.cmd` probe stub, cmd builtins only (`echo`, `>`, `%CD%`) | yes — needs `PATHEXT`, present by default; **restored under `inherit_no_env` by part B below** | part B |
                | `command_routing.rs` | nothing — help, completions, empty-state routes | n/a | none |
                | `contextual_errors.rs` | `claude.cmd` + `rustc.cmd` via `shim_name()`; `::shell rustc --edition=invalid` in its one ungated test | yes — `cmd.exe` from System32, stubs from fixture `bin` | none |
                | `handle_repo_config.rs` | `git init`, but through `common::init_git_repo` in the **parent** test process, which keeps the runner's full `PATH`; already skips when git is absent | yes — parent `PATH` | none |
                | `hooks_cli.rs` | nothing — hook views and legends | n/a | none |
                | `inline_compose_hash.rs` | `rustc` through `Command::new` in the **parent**, compiling `goose.exe` into the fixture `bin` | yes — parent `PATH`, then fixture `bin` | none |
                | `inline_compose_sequence_mismatch.rs` | `#!/bin/sh` stubs and the `::shell touch` document are all `#[cfg(unix)]` items | n/a on Windows | none |
                | `mcp_cli.rs` | 3 ungated tests; `init_git_repo` in the parent; every `#!/bin/sh` stub sits in a `#[cfg(unix)]` test | yes / n/a | none |
                | `shipped_prompt_contract.rs` | the `#!/bin/sh` + `/bin/cat` review stub is inside the `#[cfg(unix)]` test; the two ungated tests parse the shipped corpus and spawn nothing | n/a | none |
                | `wrap_basics.rs` | 5 ungated tests; two use `write_dry_run_provider_stub` (writes `.cmd` on Windows) and pin `PATHEXT`; every `#!/bin/sh` and `/bin/cat` stub is `#[cfg(unix)]` | yes — fixture `bin` | none |
                | `wrap_compose_validation.rs` | 10 ungated tests, all argv/validation failures before launch; every stub write is `#[cfg(unix)]` | n/a | none |
                | `wrap_inline_compose.rs` | 2 ungated tests (missing positional, missing `prompt`); the rest incl. `mod opencode_model_integration` are `#[cfg(unix)]` | n/a | none |
                | `ctx_launch_anchor.rs` (neighbour) | `claude` + `recordctx` through `write_command_stub`, `.cmd` on Windows using `echo`/`more`; lifecycle `{shell: "recordctx …"}` runs under `cmd` | yes — `more`/`cmd.exe` in System32, stubs in `bin`, `PATHEXT` pinned at the call site | none |
                | `propagated_context_fixtures.rs` (neighbour) | `claude.cmd` under `#[cfg(windows)]` with `PATHEXT` pinned; the `#!/bin/sh` codex capture stub is `#[cfg(unix)]` | yes | none |
                | `spawn_site_guard.rs`, `test_placement.rs`, `common/source_scan.rs` (neighbours) | text scans only, no spawn | n/a | none |

        - **step 3 — verified review 3's Unix-side claim instead of repeating it.** `sequence_schema.rs` (bare `sleep 30`), `wrap_structured_stream.rs` (`cat`/`printf`), and `wrap_opencode.rs` (the spec's other `sleep 30`) are all `#![cfg(unix)]` files, and `/usr/bin:/bin` resolves all three. The claim holds.
        - **step 4 — part B: `inherit_no_env()` on Windows.** `env_clear()` there also takes `PATHEXT`, `COMSPEC`, and `SystemRoot` — the first is what makes the fixture's `.cmd` stubs resolvable at all, the third is what `minimal_system_path()` reads — so the tightening knob was a trap on Windows rather than merely strict. `build()` now calls `restore_windows_console_variables` immediately after `env_clear()`, putting back those three and nothing else. Parent value first, with a fallback for each: `SystemRoot` → `C:\Windows`, `COMSPEC` → `<SystemRoot>\System32\cmd.exe`, `PATHEXT` → `.COM;.EXE;.BAT;.CMD`.
                - the `C:\Windows` fallback is no longer duplicated: it moved out of `minimal_system_path()`'s `#[cfg(windows)]` arm into a cfg-independent `windows_system_root()` plus a named constant, and both callers read it. Per the brief's point C, that helper and both constants compile on **every** platform, so a typo in them is a macOS compile error rather than a `windows-latest` surprise; only the three `.env()` calls are inside a `#[cfg(windows)]` block.
                - the `inherit_no_env` doc comment and the `common/mod.rs` module docs both told the call site to re-add what its run needs on Windows. That is now false, and both were rewritten in this change rather than left to drift: the docs now say the builder owns the console plumbing and the call site owns the rest (`TERM`, the temp-dir variables).
        - **step 5 — the self-test is cross-platform now, which is what AC 8 asked for.** `inherit_no_env_keeps_the_defaults_and_drops_everything_else` was `#[cfg(unix)]` with a comment naming exactly the `SystemRoot`/`COMSPEC` problem part B just fixed. The gate is gone and the comment with it. The probe stub (both the `.cmd` and the `#!/bin/sh` form) now records `PATHEXT`, `COMSPEC`, and `SystemRoot`, and the test asserts on them through `cfg!(windows)` rather than `#[cfg]`, so **both arms are compiled on every leg**: on Windows the three must be non-empty, on Unix they must be empty. The Unix arm is the half a macOS run can actually prove — that the restore stays Windows-only and a Unix run gains nothing.
        - **non-vacuity transcript** (one neuter, applied then restored): made `restore_windows_console_variables` set `PATHEXT` on the `#[cfg(not(windows))]` arm too. `FAIL … inherit_no_env_keeps_the_defaults_and_drops_everything_else`, `assertion left == right failed: PATHEXT is Windows console plumbing; a Unix run must not gain it`, `left: "[.COM;.EXE]"  right: "[]"`. Restored → green, and `diff` against the pre-neuter copy reports the file identical.
        - **what is verified on this host, and what is not.** Verified on macOS: the audit itself (a static read, and platform-independent), the whole cfg-independent half of part B (the constants, `windows_system_root`, the restore function's signature and call site, the probe stub's new keys, and the test's Unix arm including its non-vacuity). **Not verified anywhere**: the three `.env()` calls inside the `#[cfg(windows)]` block, and the test's Windows arm. Those have never been compiled — see the deferral at the top of this entry — and the honest risk is that a cleared Windows environment turns out to be missing a *fourth* thing claudine needs, in which case this one test is the first red on the `windows-latest` leg. Un-gating it is still the right call: it is the only way AC 8's cross-platform claim about this escape ever becomes evidence rather than assertion, and the failure it would produce is a named test with a recorded environment dump, not a mystery.
        - **verification** (from the `claudine` package area, `just` recipes only, no bare `cargo test`, no `cargo fmt`):
                - `just test-cli` — **green**: `2401 tests run: 2401 passed, 10 skipped` in 13.8 s. The count is unchanged: the Unix-only test became cross-platform rather than a new one being added.
                - `just lint` — **green** across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`.
                - `just test-l2` — run because `common/` is shared with the L2 binaries. **Green**: `claudine-cli` `230 tests run: 230 passed, 2415 skipped` (52.8 s) and `claudine-gen` `3 tests run: 3 passed`. The known host flake `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` did not fire on this run.
        - files touched: `claudine/cli/tests/common/mod.rs` (module docs, `inherit_no_env` docs, `build()` call, `WINDOWS_SYSTEM_ROOT_FALLBACK`, `WINDOWS_PATHEXT_FALLBACK`, `windows_system_root`, `restore_windows_console_variables`, `minimal_system_path` docs + body) and `claudine/cli/tests/cli_process_fixture.rs` (three new probe-stub keys in both stub forms, one gate removed, one test rewritten). **No test file was edited for the audit**, because the audit found nothing to fix.
- work completed for 'finding 3 — the Windows arms' at 05:28:38-07:00

### Finding 5 — post-`command()` isolation escapes

- starting the work on 'finding 5 — post-command() isolation escapes' at 05:29:57-07:00
        - **the trade-off, taken deliberately.** Review 3 offered two shapes for this finding: extend the source scan, or return a wrapper type from `build()` that exposes `arg`/`args`/`env`/`assert`/`output`/`timeout` and funnels CWD and `PATH` through builder methods. **The scan was chosen.** It buys the same enforcement guarantee — the build fails, named file and line — on allow-list mechanics that already exist, and it costs one file. The wrapper type costs a mechanical pass over 29 migrated test files during a phase whose only remaining blocker is CI evidence, and would put a fresh compile-error surface in front of that evidence. This is a trade, not an oversight: the wrapper stays available as a follow-up if the scan turns out noisy. The scan can express the guarantee — it flags the same five forms the finding names, at the same file/line granularity — with the one blind spot recorded below.
        - **where it lives.** A second gate inside `claudine/cli/tests/spawn_site_guard.rs`, not a new binary. Review 3's finding 9 already complains that the structural gates drag in the fixture's whole dependency surface; a second binary would double that. The gate reuses `AllowEntry`, `Reconciliation`, `reconcile`, `excluded`, `collect_rust_files`, and `relative` unchanged, so the stale-entry and blank-reason arms are literally the same code the spawn gate is checked by. The one rename: `struct SpawnSite` → `struct Site`, because it now carries `.current_dir(…)` sites as well as spawns.
        - **the five forms, and how each is told from its lookalike.** `.current_dir(…)`, `.env("PATH", …)`, `.env_remove("PATH")`, `.env_clear()` are recognized **in method position only** — a new `is_method_call` helper requires the nearest non-whitespace byte before the identifier to be a `.`. That is what separates `command.env("PATH", …)` from `std::env::var("PATH")` and `.current_dir(dir)` from `std::env::current_dir()`: the path-qualified forms are preceded by a `:`, never a dot. `cli_process_fixture.rs`'s `recorded["PATH"]` is an index with no `env` identifier and never matches. The literal argument is read from the **original** source via a generalized `names_literal` (the old `names_claudine` is now a one-line wrapper on it), because `sanitize` blanks string literals — the same trick the spawn gate already used to tell `cargo_bin("claudine")` from `cargo_bin("md")`. The fifth form, `augmented_path`, is flagged wherever it is *named* in executable code, `use` included: importing it is already the reach-around that `host_path()` exists to replace.
        - **scoping decision — the isolation gate governs 32 files, not the spawn gate's whole population.** Two narrowings on top of the shared tier exclusions (`level2_*`, `level3_*`, `real_*`, `common/`):
                - **`SPAWN_ALLOWLIST` files are out.** They still spawn raw and hand-assemble their own environment; `sequence_cli.rs` alone has ~20 `.env("PATH", augmented_path(…))` pairs. Flagging them would be ~500 findings of pure noise on code the burn-down has not reached, and would force an isolation allow-list that duplicates the spawn one entry for entry. Deleting a file's *spawn* entry is now the single edit that switches the isolation contract on for it — the two lists stay in lockstep by construction.
                - **A file that never names `CliProcessFixture` is out.** This is the principled half: you cannot undo isolation you were never given. It is also what handles the one real false positive on today's tree — `system_prompt_perf_bench.rs` pins `.current_dir(path)` on a `std::process::Command::new("git")`, has no fixture command anywhere, and is not on `SPAWN_ALLOWLIST` (it never spawns claudine at all), so the allow-list narrowing alone would not have saved it. The predicate runs on the **sanitized** source, so this guard's own module docs and assertion messages — which name `CliProcessFixture` repeatedly — do not enrol a file that holds none.
        - **`cli_process_fixture.rs`, checked honestly rather than exempted.** The fixture self-test is governed (it is migrated, and it is the heaviest `CliProcessFixture` user in the tree) and it **does not trip the scan**. Its four escape exercises go through the named builder methods — `inherit_no_env()`, `host_path()`, `fake_only_path()`, `ambient_context()` — never through raw command mutation, and its two textual hits (`/// … its own \`env_clear()\`` at line 358, `"…after env_clear, or the console host…"` inside an assertion message at 395) are a doc comment and a string literal, both blanked by `sanitize`. No allow-list entry, no scoping carve-out, no weakened detector: the file simply obeys the contract. `ISOLATION_ALLOWLIST` is therefore **empty**, and the guard's docs say an entry in it is a claim that the builder is missing an escape.
        - **residual blind spot, stated plainly.** The scan is textual and does not resolve receivers, so a `.current_dir(…)` on a *non-claudine* command inside a **governed** file — a fixture that shells out to `git`, say — reads identically to one on a fixture command and would be flagged. No governed file has one today (verified by grep across all 32 before writing the detector, and by the gate itself passing at 0 escapes), and the escape hatch is honest: an allow-list entry naming which command the site targets. The alternative rules all cost false negatives; this one costs a possible future false positive with a one-line resolution, which is the right side to err on for a gate.
        - **`augmented_path` visibility — cannot be narrowed, and the code now says so.** Verified the premise first: `augmented_path` is live in 20+ `level2_*`/`level3_*` binaries, so the "L2/L3 need it" justification still holds. But visibility cannot express the split at all. Every integration test binary compiles its **own** copy of `common/mod.rs` as a private top-level `mod common`, so `pub` there already means "this binary only" and `pub(crate)` reaches exactly as far — there is no marker that admits `level2_perf_capture` and refuses `wrap_basics`, because they are separate crates that each recompile the same file. Changing `pub` → `pub(crate)` would be a no-op that also risks `dead_code` noise in binaries that do not use it. Left as `pub`, with a new paragraph on the function's doc comment recording *why* the scan is the enforcement and not the type system.
        - **detector unit tests added** (5 new tests, all under `spawn_site_guard.rs`):
                - `isolation_detector_finds_every_escape_as_executable_code` — each of the five forms detected with its line; the chained `fixture.command().current_dir(root)` shape; whitespace around the dot and before the argument list (`command . env (\n "PATH",`).
                - `isolation_detector_ignores_prose_neighbors_and_path_qualified_lookalikes` — each form in a `//!` doc comment, a `///` doc comment, a `/* */` block, a plain string literal, and a raw string; `std::env::current_dir()`, `std::env::var("PATH")`, `env::var_os("PATH")`, `recorded["PATH"]`; the supported `.env("HOME", …)` / `.env_remove("HOMEDRIVE")` forms; identifier boundaries on both sides (`set_current_dir`, `current_dir_of`, `env_clear_all`, `host_augmented_path`, `augmented_path_of`); and a bare `command.current_dir;` with no call.
                - `a_file_that_never_builds_a_fixture_command_is_outside_the_isolation_gate` — the population predicate detects `CliProcessFixture::named` and a `use`, ignores it in a doc comment and in a `panic!` message, and holds identifier boundaries (`MyCliProcessFixture`, `CliProcessFixtureBuilder`).
                - `the_isolation_gate_governs_the_migrated_files_and_only_those` — the population non-vacuity guard. The gate reports **0 escapes**, which is indistinguishable from a gate that reads no files, so the population itself is asserted: >20 files, `wrap_basics.rs`/`cli_process_fixture.rs`/`ctx_launch_anchor.rs`/`propagated_context_fixtures.rs` in; `sequence_cli.rs` (spawn-allow-listed), `system_prompt_perf_bench.rs` (no fixture command), `level2_context_capture.rs` (tier), `common/mod.rs` (builder's home), and `spawn_site_guard.rs` (prose only) out.
                - the fifth is the gate proper, `migrated_l1_tests_keep_the_isolation_the_builder_gave_them`, which also prints a roll-up to stderr: `post-build isolation: 0 escape(s) across 32 governed file(s)`.
        - **non-vacuity transcript 1 — the unlisted arm, all five forms at once.** Inserted into `wrap_basics.rs` (a migrated, governed file) after `let mut command = fixture.command();`: `command.current_dir(env!("CARGO_MANIFEST_DIR"));`, `command.env("PATH", common::augmented_path(fixture.bin_dir()));`, `command.env_remove("PATH");`, `command.env_clear();`.

            ```
            FAIL [   0.042s] (1/1) claudine-cli::spawn_site_guard migrated_l1_tests_keep_the_isolation_the_builder_gave_them
            post-build isolation: 5 escape(s) across 32 governed file(s)
            thread '…' panicked at claudine/cli/tests/spawn_site_guard.rs:669:5:
            Post-`build()` escapes from the L1 isolation contract. `build()` returns a bare
            `assert_cmd::Command`, so these silently reinstate the launch context or the host PATH
            the fixture removed. Use the named builder escape instead — `ambient_context(dir)` for
            the launch CWD, `host_path()` or `fake_only_path()` for PATH, `inherit_no_env()` for a
            cleared environment — or, if the site targets a command the fixture did not build, add
            the file to ISOLATION_ALLOWLIST in cli/tests/spawn_site_guard.rs with a one-line reason.
            Escapes:
            wrap_basics.rs:19 .current_dir(…)
            wrap_basics.rs:20 .env("PATH", …)
            wrap_basics.rs:20 augmented_path
            wrap_basics.rs:21 .env_remove("PATH")
            wrap_basics.rs:22 .env_clear()
            ```

            Reverted → `PASS [   0.041s] (1/1) … migrated_l1_tests_keep_the_isolation_the_builder_gave_them`, and `diff` against the pre-neuter copy reports the file identical.
        - **non-vacuity transcript 2 — the stale arm.** With the file restored, added two `ISOLATION_ALLOWLIST` entries naming files that have no live escape (`wrap_basics.rs`, `wrap_perf.rs`):

            ```
            FAIL … migrated_l1_tests_keep_the_isolation_the_builder_gave_them
            post-build isolation: 0 escape(s) across 32 governed file(s)
            thread '…' panicked at claudine/cli/tests/spawn_site_guard.rs:689:5:
            Stale ISOLATION_ALLOWLIST entries name files with no live escape left. Delete them.
            Stale entries:
            wrap_basics.rs
            wrap_perf.rs
            ```

        - **non-vacuity transcript 3 — the blank-reason arm.** The stale assertion fires before the unexplained one, so this needed a *live* escape under an entry with an empty reason: `command.env_clear();` back in `wrap_basics.rs`, plus a single `AllowEntry { file: "wrap_basics.rs", reason: "   " }`.

            ```
            FAIL … migrated_l1_tests_keep_the_isolation_the_builder_gave_them
            post-build isolation: 1 escape(s) across 32 governed file(s)
              wrap_basics.rs: 1 escape(s) —
            thread '…' panicked at claudine/cli/tests/spawn_site_guard.rs:691:5:
            ISOLATION_ALLOWLIST entries without a reason:
            wrap_basics.rs
            ```

            Both files then restored from their pre-neuter copies; `diff` reports **both identical**, and `ISOLATION_ALLOWLIST` is back to `&[]`.
        - **docs updated alongside the behavior**, per the repo's authoring discipline:
                - `spawn_site_guard.rs` module docs rewritten from one gate to two, with named sections for the spawn gate, the isolation gate, what each gate governs (including *why* each of the two extra narrowings exists, naming `system_prompt_perf_bench.rs` as the live case), and the residual receiver blind spot.
                - `common/mod.rs` module docs gained a paragraph under "The L1 spawn contract" saying the contract does not end at the spawn — that `command()` returns a bare `assert_cmd::Command`, that the second gate flags the five forms in the migrated files, and that the builder escapes are how those needs are spelled.
                - `augmented_path`'s doc comment gained the visibility finding above.
        - **verification** (from the `claudine` package area, `just` recipes only; no bare `cargo test`, no `cargo fmt`):
                - `just test-cli` — **green**: `2406 tests run: 2406 passed, 10 skipped` in 13.5 s. Up 5 from the 2401 the finding-3 entry recorded, which is exactly the five tests added here.
                - `just lint` — **green** across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`.
        - files touched: `claudine/cli/tests/spawn_site_guard.rs` (module docs; five `FORM_*` constants; `SpawnSite` → `Site`; `ISOLATION_ALLOWLIST`; `names_literal`/`names_claudine`/`is_method_call`; `isolation_sites`; `obtains_a_fixture_command`; `governs_spawn`/`spawn_allowlisted`/`governs_isolation`/`governed_files`; `scan` re-signed to take a population predicate and a detector; one new gate test and four new unit tests) and `claudine/cli/tests/common/mod.rs` (module docs, `augmented_path` docs). **No migrated test file was edited** — the scan found nothing to fix, which is what review 3 predicted.
- work completed for 'finding 5 — post-command() isolation escapes' at 05:39:54-07:00

### Finding 6 — the burn-down roll-up artifact

- starting the work on 'finding 6 — the burn-down roll-up artifact' at 05:41:00-07:00
- **the problem, restated**: both gates in `spawn_site_guard.rs` print their census with `eprintln!` from a *passing* test. `.config/nextest.toml` sets no `success-output` in either profile and nextest's default is `never`, so the roll-up reached a log only on runs where the guard **failed** — the one time nobody needs a census. Required behavior 3's final bullet asks for the opposite.
- **the existing convention, found before writing anything** (grep over the workspace for `backend-executions` / `BISCUIT_JUNIT_STAGE_DIR`):
        - `tools/test-toolkit/src/evidence.rs` is the reference implementation. It defines `BISCUIT_JUNIT_STAGE_DIR`, resolves the staging root with `stage_dir()` (env var wins; a *relative* value resolves against the **workspace root**, not the CWD; otherwise `<workspace-root>/target/nextest/ci-reports`), derives that workspace root from its own compile-time `CARGO_MANIFEST_DIR` rather than the CWD (nextest gives each test its package directory, so a CWD-relative root would scatter the evidence), and writes **JSON Lines** — one flat `{"key":"value"}` object per line.
        - its I/O discipline is the one this finding needs verbatim: `create_dir_all` the parent, and on failure `eprintln!` a warning rather than propagate — "a test must not fail because the evidence directory is unwritable".
        - `just/devops.just:276` and `:980` stage into the same `${BISCUIT_JUNIT_STAGE_DIR:-target/nextest/ci-reports}`, alongside `manifest.jsonl` and `expected-<tier>.json`.
        - `docs/testing-strategy.md:186` and both testing skills document the variable with the same default.
        - `test-toolkit` is already a `[dev-dependencies]` entry of `claudine-cli`, so `test_toolkit::stage_dir()` was directly reachable from the guard. No new dependency was added; `serde`/`serde_json`/`tempfile` are already `claudine-cli` dependencies and are therefore already linked into every test target.
- **what was built** — all of it in `claudine/cli/tests/spawn_site_guard.rs`:
        - a `BurnDown` struct (gate, `governed_files`, `scanned_sites`, per-file `(file, sites, reason)`, per-reason `(files, sites)`) that both the stderr copy and the artifact are now rendered from. The existing `eprintln!` output is **byte-for-byte unchanged** — it is what makes a failing run legible — it is just fed from the struct instead of recomputing the reason roll-up inline.
        - `BurnDownRecord`, a `#[serde(tag = "kind")]` enum with three variants: `file` (one allow-listed file, its live-site count, its reason), `reason` (the roll-up: files and sites still exempt under that reason — the number that has to reach zero), and `total` (`files`/`sites` still allow-listed, `scanned_sites` the detector found, `governed_files` the population size). `sites` below `scanned_sites` is exactly the failure the gate asserts on, so the artifact is self-checking.
        - `to_jsonl()` — the pure serialization function, the unit tests' subject.
        - `write_report(path, contents)` — `create_dir_all` then a whole-file `fs::write`.
        - `emit_report(file_name, burn_down)` → `emit_report_to(path, burn_down)`, split so the failure path is testable. `emit_report` resolves `test_toolkit::stage_dir().join(file_name)`; `emit_report_to` warns on stderr and swallows the error.
        - `sites_in(governed, detect)` split out of `scan`, so a gate that needs the population size no longer reads all ~200 files twice (the isolation gate was doing exactly that: `governed_files` **and** `scan`).
        - both gate tests call `emit_report` **before** their assertions, so a failing run produces the census too.
- **location and format**: `$STAGE/spawn-site-burn-down.jsonl` and `$STAGE/isolation-burn-down.jsonl`, where `$STAGE` is `$BISCUIT_JUNIT_STAGE_DIR` else `target/nextest/ci-reports`. JSONL, because that is the house style (`backend-executions.jsonl`, `manifest.jsonl`) and it tails legibly in a CI log while staying `jq`-parseable.
        - **two files, not one, and why**: each file is rewritten *whole* by the single test that owns it. A truncating write is idempotent — it never accumulates stale records across runs, so it needs no `reset` bracket in `just/devops.just` the way `backend-executions.jsonl` does (`backend-proof reset`), and adding such a bracket would have been the CI restructuring this finding explicitly rules out. But nextest gives each test its own process, and two processes truncating one path would clobber each other. One owner per file is what makes truncation safe. Each record still carries a `gate` field, so the two files concatenate into one coherent stream if a consumer ever wants that.
- **CI collection — no workflow change was needed, and none was made**. `.github/workflows/_package-ci.yml:371-377` uploads `path: target/nextest/ci-reports` — the **whole staging directory**, not a single XML — as `junit-${{ inputs.package }}-L1-${{ matrix.environment }}`. That is the default `stage_dir()`, so both artifacts land inside the tree that is already collected. The WSL leg (`_wsl-ci.yml:269, 652, 659`) sets `BISCUIT_JUNIT_STAGE_DIR` to an absolute path and then `cp -R "$STAGE/."` into its upload directory — also the whole tree, also automatic. Verified by reading both workflows; `git diff main -- .github/` for this finding is empty.
- **`.config/nextest.toml` untouched**, per the instruction and AC7: `git diff main -- .config/nextest.toml` prints **nothing**. No `success-output` override was added — the reviewer's point is that a profile-wide output-mode change alters every test's output to buy one test's telemetry.
- **four new unit tests**, all over the pure functions rather than over run order:
        - `the_artifact_carries_per_file_counts_the_reason_rollup_and_the_totals` — exact JSONL for a synthetic three-site, two-reason census.
        - `the_artifact_is_written_even_when_the_staging_directory_does_not_exist` — writes into a two-level-deep path that was never created (the local `cargo nextest` case).
        - `a_second_run_replaces_the_previous_census_instead_of_appending` — two emissions, one census.
        - `an_unwritable_destination_warns_instead_of_failing_the_guard` — a regular file occupying the artifact's parent directory, so `create_dir_all` cannot succeed on any platform; asserts `write_report` errors and that `emit_report_to` nonetheless returns.
- **non-vacuity, two neuter rounds** (file copied to `/tmp` first, restored and `diff`-verified after):
        - round 1 — `to_jsonl` neutered to `return out;` immediately after `let mut out = String::new();`:

            ```
            PASS  the_artifact_is_written_even_when_the_staging_directory_does_not_exist
            PASS  a_second_run_replaces_the_previous_census_instead_of_appending
            FAIL  the_artifact_carries_per_file_counts_the_reason_rollup_and_the_totals
            assertion `left == right` failed
              left: ""
             right: "{\"kind\":\"file\",\"gate\":\"spawn\",\"file\":\"allowed.rs\",…}\n…"
            ```

            The two write tests survive this round because they compare the file against `to_jsonl()` — self-referentially — which is why the second round exists.
        - round 2 — `write_report` neutered to `return Ok(())` before doing any I/O:

            ```
            PASS  the_artifact_carries_per_file_counts_the_reason_rollup_and_the_totals
            FAIL  the_artifact_is_written_even_when_the_staging_directory_does_not_exist
                  artifact written: Os { code: 2, kind: NotFound, … }
            FAIL  a_second_run_replaces_the_previous_census_instead_of_appending
                  artifact written: Os { code: 2, kind: NotFound, … }
            FAIL  an_unwritable_destination_warns_instead_of_failing_the_guard
                  assertion failed: write_report(&path, "irrelevant").is_err()
            ```

            Every one of the four goes red under one of the two rounds. Restored from the pre-neuter copy; `diff` reports **identical** and `grep -c NEUTERED` is `0`.
- **emitted content after a local `just test-cli`** (`target/nextest/ci-reports/`), abridged in the middle:

    ```
    $ cat isolation-burn-down.jsonl
    {"kind":"total","gate":"isolation","files":0,"sites":0,"scanned_sites":0,"governed_files":32}

    $ cat spawn-site-burn-down.jsonl
    {"kind":"file","gate":"spawn","file":"completion_contract.rs","reason":"outside this fix's scope","sites":2}
    {"kind":"file","gate":"spawn","file":"compose_schema_cli.rs","reason":"outside this fix's scope","sites":18}
    {"kind":"file","gate":"spawn","file":"context_command.rs","reason":"outside this fix's scope","sites":24}
    …33 more `file` records…
    {"kind":"file","gate":"spawn","file":"sequence_ctrl_c_windows.rs","reason":"needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither","sites":1}
    {"kind":"file","gate":"spawn","file":"wrap_ctrl_c_windows.rs","reason":"needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither","sites":1}
    {"kind":"reason","gate":"spawn","reason":"needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither","files":2,"sites":2}
    {"kind":"reason","gate":"spawn","reason":"outside this fix's scope","files":34,"sites":168}
    {"kind":"total","gate":"spawn","files":36,"sites":170,"scanned_sites":170,"governed_files":83}
    ```

    39 lines for the spawn gate, 1 for the isolation gate. `files: 36` / `sites: 170` equalling `scanned_sites: 170` is the "every live site is accounted for" invariant stated as data. The isolation gate's single `total` line is the only honest census it has — its allow-list is empty and it found no escapes, so `governed_files: 32` is the number that proves the gate read anything at all.
- **module docs updated** with a `## Reading the burn-down` section: where the two files land, why the stderr copy is not enough, that CI collects them via the existing whole-staging-directory upload, what each of the three record kinds means, why the artifact is truncating and why there are two files, and a `cat` example with today's real numbers.
- **verification** (from the `claudine` package area, `just` recipes only; no bare `cargo test`, no `cargo fmt`):
        - `just test-cli` — **green**: `2410 tests run: 2410 passed, 10 skipped` in 13.7 s. Up 4 from the 2406 the finding-5 entry recorded, which is exactly the four tests added here.
        - `just lint` — **green** across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`.
        - `git diff main -- .config/nextest.toml` — **empty**.
- files touched: `claudine/cli/tests/spawn_site_guard.rs` only (module docs; `SPAWN_REPORT_FILE`/`ISOLATION_REPORT_FILE`/`GATE_SPAWN`/`GATE_ISOLATION`; `BurnDown`; `BurnDownRecord`; `write_report`/`emit_report`/`emit_report_to`; `sites_in` split out of `scan`; both gate tests re-wired; `sample_burn_down` plus four unit tests). No workflow, justfile, or nextest-config change.
- work completed for 'finding 6 — the burn-down roll-up artifact' at 05:47:12-07:00

### Finding 7 — the `TMPDIR` containment guard

- starting the work on 'finding 7 — the TMPDIR containment guard' at 05:52:01-07:00
- **the boundary is the checkout, found by walking up to `.git` — not `CARGO_MANIFEST_DIR`**. `env!("CARGO_MANIFEST_DIR")` is the *crate* directory, `<checkout>/claudine/cli`. The reviewer's reproduction is `TMPDIR=<checkout>/target/tmpdir-probe`, which is outside the crate and inside the checkout, so a `CARGO_MANIFEST_DIR` boundary would have missed the exact case the finding is about. `common::checkout_root()` therefore takes the nearest ancestor of `CARGO_MANIFEST_DIR` carrying a `.git` entry — a directory for a clone, a **file** for a worktree, which is what this branch is running in — and that is also the root claudine's own repository discovery walks out to, so the guard's boundary and the hazard's boundary are the same one.
        - `None` when no ancestor carries `.git`, which is the relocated `cargo nextest archive` case (the `wsl2-ubuntu` leg): the compile-time path no longer exists there, so there is no checkout to be captured by and nothing to assert.
- **the assertion**: `common::checkout_containment_error(workspace_root, checkout_root) -> Option<String>` in `cli/tests/common/mod.rs`, called from `CliProcessFixture::named` **before** `cwd`/`home`/`bin` are created, so it fires once, early, and on the construction line rather than on a downstream assertion.
        - canonicalization is reused rather than re-invented: `named()` canonicalizes both sides exactly as `ClaudineCommandBuilder::ambient_context` already canonicalizes its own containment check, then hands two canonical paths to the pure comparison. That keeps macOS's `/var` → `/private/var` symlink from reading as "not contained", and it keeps the two containment rules in the file spelled the same way.
        - the message names the offending workspace, the checkout it sits inside, and the variable to change. The platform split is one `const TEMP_DIR_VARIABLE: &str = if cfg!(windows) { "TMP (or TEMP)" } else { "TMPDIR" };` interpolated into a single message, rather than two `cfg`-gated copies of the whole paragraph — `std::env::temp_dir()` reads `TMPDIR` on Unix and `TMP` then `TEMP` on Windows.
- **the accidental guard is now labelled, not deleted**. `default_command_pins_cwd_home_and_the_minimal_system_path` keeps its CWD assertion — it is the direct proof that `build()` pins `current_dir` — and gains a comment recording that it used to be the suite's only defence against this hazard, that it reported the symptom rather than the cause, and that `CliProcessFixture::named` now rejects the workspace first. No live assertion was removed.
- **how the assertion is tested**: `a_workspace_inside_the_checkout_is_rejected_by_naming_the_temp_dir_variable` in `cli_process_fixture.rs` calls the pure function directly with stand-in paths. Inducing the real condition needs `set_var` on a variable whose *name* differs by platform (`TMPDIR` vs `TMP`/`TEMP`), so a `#[should_panic]` fixture construction would only ever run the Unix half; the rule is a comparison of two canonical paths, so calling it with stand-ins exercises exactly what `named()` calls, on all three OSes. Three cases: contained → the message names workspace, checkout, and the platform's variable; a path outside → `None`; and `rusty-biscuit-stand-in-2` against `rusty-biscuit-stand-in` → `None`, pinning that `starts_with` stays component-wise and a textual-prefix sibling is not containment.
- **reproduction of the reviewer's measurement** (idle 16-core Mac, `claudine` package area):
        ```
        # normal TMPDIR
        $ just test-cli agents_and_commands_route
        PASS [   0.093s] (1/1) claudine-cli::command_routing agents_and_commands_route_to_empty_state_messages

        # BEFORE — TMPDIR inside the checkout, guard temporarily disabled
        $ mkdir -p <checkout>/target/tmpdir-probe
        $ TMPDIR=<checkout>/target/tmpdir-probe just test-cli agents_and_commands_route
        PASS [   2.026s] (1/1) claudine-cli::command_routing agents_and_commands_route_to_empty_state_messages

        # AFTER — same TMPDIR, guard live
        $ TMPDIR=<checkout>/target/tmpdir-probe just test-cli agents_and_commands_route
        FAIL [   0.008s] (1/1) claudine-cli::command_routing agents_and_commands_route_to_empty_state_messages
        thread '…' panicked at claudine/cli/tests/common/mod.rs:253:17:
        fixture precondition: the fixture workspace <checkout>/target/tmpdir-probe/command-routing-empty-state-46646-… is inside the rusty-biscuit checkout <checkout>. Claudine's repository discovery walks out of the workspace and finds that checkout, so every L1 spawn would run against it rather than against the workspace the test built — slowly, and against topology no test wrote. Point TMPDIR at a directory outside the checkout: it is what `std::env::temp_dir()` reads, and the fixture workspace is built there.
        ```
        0.093 s → 2.026 s is the reviewer's 21× reproduced to within noise (they measured 0.096 s → 2.065 s), and the guard turns it into a 0.008 s failure naming `TMPDIR`. The BEFORE run is also the non-vacuity proof for the new check: with the `panic!` neutered the same tree passes slowly, with it live the same tree fails fast. `target/tmpdir-probe` was removed afterwards.
- **verification** (from the `claudine` package area, `just` recipes only; no bare `cargo test`, no `cargo fmt`):
        - `just test-cli` — **green**: `2411 tests run: 2411 passed, 10 skipped` in 13.8 s. Up 1 from the 2410 the finding-6 entry recorded, which is the one test added here.
        - `just lint` — **green** across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`.
        - `just test-l2` — **green**: `230 tests run: 230 passed, 2425 skipped` for `claudine-cli` in 53.0 s, then `3 tests run: 3 passed` for `claudine-gen`. `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` — the known host flake — passed this run.
- files touched: `claudine/cli/tests/common/mod.rs` (module docs gain a `### The temp-directory precondition` paragraph; `TEMP_DIR_VARIABLE`; `checkout_root`; `checkout_containment_error`; the guard call in `CliProcessFixture::named`) and `claudine/cli/tests/cli_process_fixture.rs` (the comment on the CWD assertion, plus the new test). No production code, no justfile, no nextest-config change. No broader "slow temp dir" detector, warning system, or runtime timing check was added — the finding is one assertion.
- work completed for 'finding 7 — the TMPDIR containment guard' at 05:56:07-07:00

### Finding 8 — deleting the `IN_SCOPE` dead vocabulary

- starting the work on 'finding 8 — deleting the IN_SCOPE dead vocabulary' at 05:58:37-07:00
- **the grep, before touching anything** (whole `claudine/` tree, not just the file):
        ```
        $ grep -rn "IN_SCOPE" claudine/ | grep -v OUT_OF_SCOPE
        claudine/fixes/2026-08-01-cli-slow-tests/review-2.md:268:### 8. (low) `IN_SCOPE` is dead vocabulary kept alive by its own unit test
        claudine/fixes/2026-08-01-cli-slow-tests/review-2.md:334:   assertion (9), and deleting `IN_SCOPE` (8).
        claudine/fixes/2026-08-01-cli-slow-tests/review-3.md:358:### 8. (low) `IN_SCOPE` is still dead vocabulary kept alive by its own unit test
        claudine/fixes/2026-08-01-cli-slow-tests/review-3.md:432:   deleting `IN_SCOPE` (8), and the `#[path]` include (9).
        claudine/fixes/2026-08-01-cli-slow-tests/log.md:23:        - finding 8 (low) — `IN_SCOPE` is dead vocabulary kept alive by its own unit test
        claudine/cli/tests/spawn_site_guard.rs:160:const IN_SCOPE: &str = "in-scope, migrates in this fix";
        claudine/cli/tests/spawn_site_guard.rs:1099:                reason: IN_SCOPE,
        ```
        Exactly two code hits: the definition and the one use the finding names. Everything else is this fix's own planning prose. The finding's premise held.
- **what was deleted**: the six-line doc comment and `const IN_SCOPE` (`spawn_site_guard.rs:153–160`). Its single consumer, `reconciliation_reports_unlisted_sites_stale_and_unexplained_entries`, now writes `reason: "a reason, any reason"` inline — the literal says what the field is for at the point it is read, which is what the six lines were spending their length explaining from 940 lines away.
- **`OUT_OF_SCOPE` and `NEEDS_LIVE_CHILD` were not touched.** Both back live `SPAWN_ALLOWLIST` entries (34 files and 2 files respectively, per the census below), so both are load-bearing vocabulary, not dead.
- **verification**: `just test-cli --test spawn_site_guard` — 15 tests run, 15 passed, and the census is unchanged at 170 sites in 36 allow-listed files (of 170 scanned, 83 governed), with the reason roll-up still reading `outside this fix's scope: 34 file(s), 168 site(s)` and `needs a live Child …: 2 file(s), 2 site(s)`.
- files touched: `claudine/cli/tests/spawn_site_guard.rs` only.
- work completed for 'finding 8 — deleting the IN_SCOPE dead vocabulary' at 06:00:12-07:00

### Finding 9 — the structural gates' dependency surface

- starting the work on 'finding 9 — the structural gates dependency surface' at 06:00:12-07:00
- **what each gate actually uses from `common`, checked before removing `mod common;`**:
        ```
        $ grep -n "common" claudine/cli/tests/spawn_site_guard.rs claudine/cli/tests/test_placement.rs
        ```
        `test_placement.rs` → `common::source_scan::{is_ident, sanitize}` and nothing else. `spawn_site_guard.rs` → `common::source_scan::{is_ident, line_at, sanitize}` and nothing else; its other `common::` mentions are all prose or string literals inside detector unit tests (`"use common::claudine_bin;"`, `"common/mod.rs"`), which are data the scanner reads, not imports. **Neither gate had to keep `mod common;`** — finding 9 applies fully to both.
        `spawn_site_guard.rs` also calls `test_toolkit::stage_dir()` (from finding 6's artifact work) and uses `serde_json` and `tempfile`. Those are crate dependencies declared in `cli/Cargo.toml`, not `common` helpers, so they are unaffected by the module-path change and stay.
- **the change**, applied to both gates:
        ```rust
        #[path = "common/source_scan.rs"]
        mod source_scan;
        ```
        `test_placement.rs` additionally carries `#[allow(dead_code)]` on the `mod` item: it uses `is_ident` and `sanitize` but not `line_at`, and a `pub fn` in a private module that nobody calls is a `dead_code` warning. This is the same allowance `common/mod.rs` grants with its file-level `#![allow(dead_code)]`, spelled as an outer attribute because the file is shared and must not gain an inner one. **Proven necessary rather than assumed**: with the attribute removed, `cargo build -p claudine-cli --test test_placement` emits `warning: function `line_at` is never used`; with it restored, clean.
- **two module-doc intra-doc links had to become plain code spans** — `[`common::claudine_bin`]` and `[`common::augmented_path`]` in `spawn_site_guard.rs`'s docs. `common` is no longer a module in that binary's scope, so as bracketed links they would be dangling. The prose is unchanged; only the brackets came off.
- **measurement** (idle 16-core Mac, warm shared `target/`, same toolchain, same `cargo build -p claudine-cli --test <gate>` invocation before and after; each rebuild forced with `touch` on the gate's own source):
        | gate | binary size before | after | Δ | incremental rebuild before | after |
        |---|---|---|---|---|---|
        | `test_placement` | 2,561,536 B | 1,614,592 B | **−37%** | 0.61 s | 0.52 s |
        | `spawn_site_guard` | 2,929,888 B | 1,913,600 B | **−35%** | 0.59 s | 0.53 s |
        The crisper evidence is the dep-info file cargo writes next to each binary, which lists exactly the sources compiled into it. Before, both gates listed five files from `cli/tests/`:
        ```
        common/completion.rs  common/mod.rs  common/pty.rs  common/source_scan.rs  common/wrap.rs  <gate>.rs
        ```
        After, both list two:
        ```
        common/source_scan.rs  <gate>.rs
        ```
        That is 1,418 lines of fixture helper (`mod.rs` 782 + `pty.rs` 288 + `wrap.rs` 201 + `completion.rs` 147) no longer compiled into either binary, against `source_scan.rs`'s 144 that are.
- **honest reading of the win**: ~0.07 s per gate per rebuild is noise-adjacent on a warm tree, and the workspace's dev-dependencies (`expectrl`, `chrono`, `serde_json`, the `claudine` lib) are still built for the other 81 test binaries, so nothing was removed from the *workspace* build. What did change is real but bounded: two binaries got a third smaller, and the fixture surface stopped being on their compile path. The durable argument is the fragility one the finding leads with — a text scanner should not fail to build because `common::wrap`'s `claudine::mcp::types` or `common::pty`'s expectrl broke.
- **non-vacuity — both gates still detect what they detected before.** A throwaway `cli/tests/scratch/probe.rs` was planted: a subdirectory file is scanned by both guards (they walk `tests/` recursively) but is not a cargo test target, so it induces the failures without a rebuild. With a raw spawn plus a post-`build()` escape in it, `just test-cli --test spawn_site_guard` went to `15 tests run: 13 passed, 2 failed`, both gates naming the file:
        ```
        FAIL … l1_tests_spawn_claudine_through_the_fixture_builder
          Unlisted sites:
          scratch/probe.rs:2 cargo_bin
        FAIL … migrated_l1_tests_keep_the_isolation_the_builder_gave_them
        ```
        Re-planted with `SpawnVisibility::Foreground`, `just test-cli --test test_placement` went to `9 tests run: 8 passed, 1 failed`:
        ```
        FAIL … focus_stealing_apis_stay_in_keyboard_tier_files
          cli/tests/scratch/probe.rs: SpawnVisibility::Foreground
        ```
        `cli/tests/scratch/` was deleted afterwards; `git status` confirms no stray file remains.
- **verification** (from the `claudine` package area, `just` recipes only; no bare `cargo test`, no `cargo fmt`):
        - `just test-cli --test spawn_site_guard --test test_placement --success-output=final` — **green**: `24 tests run: 24 passed`. Spawn census unchanged at 170 sites / 36 files / 83 governed; isolation census unchanged at `0 escape(s) across 32 governed file(s)`.
        - `just test-cli` — **green**: `2411 tests run: 2411 passed, 10 skipped` in 13.9 s. Same 2411 as the finding-7 entry recorded, as expected: these two findings add and remove no tests.
        - `just lint` — **green** across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`.
- files touched: `claudine/cli/tests/spawn_site_guard.rs` and `claudine/cli/tests/test_placement.rs` (module declaration and the two doc-link spans only). `common/source_scan.rs` was **not** modified — it is still `pub(crate) mod source_scan;` under `common/mod.rs` for every other consumer, and the two gates now reach the same file by path. No production code, no justfile, no nextest-config change.
- work completed for 'finding 9 — the structural gates dependency surface' at 06:03:21-07:00

### Finding 2 — acceptance criterion 4's CI evidence (deferred)

- starting the work on 'finding 2 — acceptance criterion 4' at 06:04:10-07:00
        - **deferred, not attempted.** Closing this finding means committing the branch, pushing it, and reading the JUnit artifacts of three consecutive green runs on four CI environments. Committing and pushing are separate human-driven steps this session is not authorized to take, so the twelve required data points cannot exist yet.
        - a local run cannot stand in for them, and the spec says so in terms: *"Local runs are for attributing cost, never for proving a target."* The reference environments are 3–4 vCPU hosted runners running this test group at `max-threads = 1`, and the WSL2 leg reads its workspace through the Windows disk at 20–50× local cost for exactly these test shapes.
        - the work that *can* be done ahead of the push already is: [junit-metrics.ts](junit-metrics.ts) reproduces the baseline run `33440897014`'s table to the decimal across all four environments, and [inventory.md](inventory.md)'s "2026-09 follow-up" section holds the recipe, the baseline table, and an explicitly empty post-change table naming what blocks it. This closes by pushing, not by more engineering.
        - full detail, including the closing procedure and the riskiest row, is recorded in [deferred-performance.md](deferred-performance.md); the log frontmatter carries `deferred_perf_measurement: true`.
- work completed for 'finding 2 — acceptance criterion 4' at 06:05:40-07:00 (recorded as deferred)

### Successful Completion

The implementation of review cycle 3 has completed successfully in 1 hour 12 minutes. During this implementation all 9 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 8 were fixed, 1 was deferred (see reasons below):

- **finding 2 — acceptance criterion 4's CI evidence — deferred in full.** It requires committing and pushing the branch and then reading three consecutive green CI runs across `ubuntu-latest`, `macos-latest`, `windows-latest`, and `wsl2-ubuntu`. Nothing on this branch is committed, and committing and pushing are human-driven steps outside this session's authorization. No local measurement can substitute — the spec forbids local runs as proof of a target, and the reference environments differ from this 16-core Mac by 20–50× for the test shapes in question. The measurement script is written and baseline-validated, so this closes by pushing rather than by further engineering. Detail: [deferred-performance.md](deferred-performance.md).
- **finding 3 — partially deferred.** Both locally-actionable halves were implemented: the bare-name utility audit across the 13 Windows-compiled migrated binaries (clean — no test shells out by a name `%SystemRoot%\System32` cannot resolve, and no `#[cfg]` gate was changed), and the `inherit_no_env` console-variable restoration that turns a Windows trap back into a usable tightening knob. The **compile** half stays deferred: `cargo check --target x86_64-pc-windows-msvc -p claudine-cli --tests` fails on this host because a transitive `aws-lc-sys` build script needs `windows.h` from the Windows SDK. That is an environment constraint, not a load or timing one. The `windows-latest` CI leg remains the first compiler these arms will see, and the same push that closes finding 2 closes this.

The files changed during this implementation cycle:

- `claudine/cli/tests/common/mod.rs` — the environment-inheritance scrub (`CLAUDINE_*` by prefix, the five `GIT_*` plumbing keys, the three render inputs), parent-side `git` hardening in `init_git_repo`, the Windows console-variable restoration and the cfg-independent `windows_system_root()`, the `TMPDIR` containment precondition, and the module docs for all of it
- `claudine/cli/tests/cli_process_fixture.rs` — `CLAUDINE_PROBE_CONTROL` → `FIXTURE_PROBE_CONTROL`, six new keys on both recording stubs, five new contract tests, and `inherit_no_env_keeps_the_defaults_and_drops_everything_else` un-gated to run on every platform
- `claudine/cli/tests/spawn_site_guard.rs` — the `bin_exe!` spawn form and its detector tests, the second post-`build()` isolation gate with its own allow-list, the JSONL burn-down artifact emitter, the `IN_SCOPE` deletion, and the `#[path]` include
- `claudine/cli/tests/test_placement.rs` — the `#[path]` include
- `claudine/cli/tests/argv_normalization.rs`, `claudine/cli/tests/command_routing.rs` — manual `TERM_WIDTH` pins and redundant `FORCE_COLOR` removals dropped now that the builder owns the render inputs
- `claudine/cli/tests/wrap_basics.rs` — its `TERM_WIDTH=200` pin kept, its comment rewritten to read as a deliberate render-width choice rather than an isolation workaround
- `claudine/fixes/2026-08-01-cli-slow-tests/log.md` — this log
- `claudine/fixes/2026-08-01-cli-slow-tests/deferred-performance.md` — new; the deferred CI evidence and Windows compilation, with their closing procedures

Final verification from the `claudine` package area, through the canonical recipes only:

- `just test-cli` — green, 2411 passed / 10 skipped, 13.5 s (2397 before this cycle; +14 new tests)
- `just lint` — green across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`
- `just test-l2` — green, 230 passed for `claudine-cli` and 3 for `claudine-gen`, run after each change that touched `common/`
- `git diff main -- .config/nextest.toml` — empty; no `slow-timeout` and no `success-output` override was added (acceptance criterion 7)

One residual worth naming, found while re-running the review's leak probes and **not** fixed here: `COLUMNS=44 just test-cli` no longer reddens the five `characterization_error_routes` tests, but the run now reaches two *unmigrated* files — `compose_schema_cli` and `composition_outputs` — that fail for the same inherited-width reason. Both are `SPAWN_ALLOWLIST` entries that never take the builder, so closing them means migrating them, which is Required behavior 2 scope and belongs to the follow-up burn-down fix the spec already names.
