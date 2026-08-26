---
implementation_1: "2026-08-26T15:46:25+01:00"
implementation_2: "2026-08-26T20:15:11+01:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-08-26T15:46:25+01:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-08-12-ctx-launch-anchor/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'bracket-indexed target identity graph-preflight rejection' at 15:48:16
        - confirmed that `Expr::Index` was walked as independent base/index expressions, so equivalent bracket paths were never reconstructed before target-identity policy enforcement
        - implemented canonical dotted identities for statically keyed variable/member/index chains
        - defined computed indexes rooted at `ctx` or `env` to fail closed at graph preflight because their dynamic key can select a target-dependent identity leaf; computed indexes under other roots retain their existing behavior
        - added Level 1 graph-preflight coverage for all four bracket-indexed identity paths across primary, setup, teardown, and nested expression positions, plus computed-index policy boundaries
        - `just test` passed in the `claudine` package area for all five package suites
- work completed for 'bracket-indexed target identity graph-preflight rejection' at 16:13:03
        - `just lint` passed in the `claudine` package area with no lint findings
        - `git diff --check` passed and the updated Claudine composition skill reference has a verified Darkmatter hash
- starting the work on 'complete route and interpolation-surface matrix' at 16:16:45
        - added discriminating canonical-owner coverage for all five document-entry reasons, real loop and opposing-area routes, primary and appendix relocation, overlay and harness re-entry, and distributed task and JIT preparation
        - every matrix fixture separates launch-owned context, environment, and effective values from source-owned body, schema, eager file, and file-resolution provenance
        - focused regressions exposed missing prepared context and environment lookup in loop gates and missing file provenance on system-prompt, overlay, and harness Markdown; fixed those ownership gaps
        - focused matrix verification passed for three library owner tests and six CLI tests
        - `just test` passed for all five package suites in the Claudine package area
        - `just lint` passed for the Claudine package area with no lint findings
        - `git diff --check` passed; no finding was blocked or deferred
- work completed for 'complete route and interpolation-surface matrix' at 16:59:45
- starting the work on 'snapshot consumer accounting and observable fallback' at 17:03:10
        - added invocation-owned, concurrency-safe observations for the named `preflight`, `body`, `effective-frontmatter`, `loop-condition`, and `lifecycle` consumers; performance reports expose the deterministic sorted set
        - instrumented direct, loop, lifecycle, sequence graph/JIT/task, system-prompt, overlay, and harness preparation owners without adding snapshot identity to Darkmatter
        - made `derive_compose_context` increment ambient-fallback accounting whenever an invocation owner exists but the prepared context is absent
        - added Level 1 coverage for populated and missing prepared-context paths, concurrent accounting, exact direct-route consumer sets, and a stabilized reread that proves one construction plus one retained-evidence extension, unchanged launch/target values, zero fallbacks, and the complete harness consumer set
        - focused Level 1 verification passed for all six accounting, direct-route, lifecycle, and stabilized-reread tests
        - the first full gate exposed only generated dispatch-inventory line drift from the inserted instrumentation; the inventory was regenerated and its 12-test guard passed
        - the enhanced stabilized-reread fixture was tightened after the repository test-placement guard identified that its inline test module exceeded the 300-line policy by two lines; the focused placement guard then passed
        - `just test` passed all five directly impacted Claudine package groups: `claudine-catalog-types` (21 tests), `claudine` (4,046 tests), `claudine-contract` (48 tests), `claudine-cli` (2,384 tests), and `claudine-gen` (154 tests)
        - `just lint` passed its 18 error-guard tests and Clippy checks for all five Claudine package groups
        - `git diff --check` passed; Darkmatter was not changed, and no work for this finding was blocked or deferred
- work completed for 'snapshot consumer accounting and observable fallback' at 17:43:33
- starting the work on 'native portability and AC14 validation matrix' at 17:48:51
        - removed the module-wide Unix gate from the launch-anchor CLI regressions so all seven semantic tests compile and execute on native Windows
        - added platform-native fake-command fixtures: executable POSIX shell scripts on macOS/Linux/WSL and `.cmd` scripts on native Windows, including provider stdin capture, lifecycle failure, and shell-command argument recording
        - audited the fixtures to retain `Path`/`PathBuf`, `join_paths`, and canonicalized path comparisons without hard-coded separators, case assumptions, or ambient working-directory dependencies
        - focused macOS Level 1 verification passed all seven `ctx_launch_anchor` CLI regressions
        - macOS `just test` and `just lint` passed in the `darkmatter` package area: `darkmatter` ran 6,247 tests, `darkmatter-cli` ran 653 tests, and `dmls` ran 640 tests
        - macOS `just test` and `just lint` passed in the `claudine` package area for all five package groups, including all seven portable launch-anchor CLI regressions
        - macOS `just test-l2` passed through Claudine's canonical background self-spawn recipe: the CLI suite ran 230 tests with eight isolated workers and no shared pane, and the generator suite ran three tests
        - the Level 2 run used only its canonical background resources, its focus-policy guard passed, and this implementation introduced no focus-stealing API or foreground terminal/browser action
        - the post-portability focused command `cargo nextest run --color=never -p claudine-cli --test ctx_launch_anchor` passed all seven tests after the explicit Windows `PATHEXT` fixture was added
        - Linux execution was deferred because `ssh -o BatchMode=yes build-linux` resolved `/home/build/coding/rusty-biscuit` at stale commit `7685d1ac29920f874a2a83a7cecec0775c08aaaa`; the clone did not contain `claudine/cli/tests/ctx_launch_anchor.rs`, while the current local tree was based on `d4be99a9a13e995bb8e0aaa6b036ae8185bb6844`, so no remote result could exercise or validate the uncommitted implementation
        - WSL execution was deferred because `ssh -o BatchMode=yes build-win pwd` initially reported `/home/ken`, then every bounded read-only path probe, including a final 15-second `pwd`, remained open without output and was terminated; current-worktree visibility could not be established and no test result was claimed
        - native Windows execution was deferred because `ssh -o BatchMode=yes build-win-native` resolved `C:\Users\ken\rusty-biscuit` at the same stale commit `7685d1ac29920f874a2a83a7cecec0775c08aaaa` and that clone also lacked `claudine\cli\tests\ctx_launch_anchor.rs`
        - the supplementary local cross-target command `cargo check --color=never --target x86_64-pc-windows-gnu -p claudine-cli --test ctx_launch_anchor` reached the Claudine dependency graph but could not complete because MinGW rejected oversized `libduckdb-sys` objects with `too many sections` and `file too big`; this dependency-toolchain failure is not recorded as native Windows validation
        - hosted-CI validation was deferred because it requires publishing the current changes through a commit and push or equivalent external authority, while this implementation was explicitly prohibited from committing or pushing
- work completed for 'native portability and AC14 validation matrix' at 18:25:34
        - the final Claudine `just lint` rerun passed all 18 error guards, the lifecycle-document guard, and Clippy for all five package groups after the Windows fixture adjustment
        - `git diff --check` passed; all runnable macOS gates succeeded, and every unavailable AC14 environment was recorded without claiming validation

### Successful Completion

The implementation of review cycle 1 has completed successfully in 2 hours, 41 minutes, and 1 second. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 1 was deferred (see reasons below):

- finding 4, 'native portability and AC14 validation matrix', was fixed for portable fixtures and every runnable local gate; its external validation evidence was deferred because the available environments could not exercise the current uncommitted implementation
        - Linux and native Windows build clones were at stale commit `7685d1ac29920f874a2a83a7cecec0775c08aaaa` and lacked the changed launch-anchor test module
        - the WSL alias stopped returning output during bounded read-only visibility probes, so current-worktree visibility could not be established
        - hosted CI requires a commit and push or equivalent external authority, which this task explicitly prohibited
        - a supplementary Windows GNU cross-target check stopped in `libduckdb-sys` because MinGW rejected oversized assembler objects before reaching the Claudine test target; it was not treated as native Windows evidence

The files changed by this implementation are:

- `.claude/skills/claudine/composition.md`
- `claudine/docs/topics/composition.md`
- `claudine/docs/providers/dispatch-inventory.json`
- `claudine/lib/src/invocation_context.rs` and `claudine/lib/src/invocation_context/tests.rs`
- `claudine/lib/src/composition/error/mod.rs`
- `claudine/lib/src/composition/prepare.rs`, `claudine/lib/src/composition/prepare/tests.rs`, and `claudine/lib/src/composition/prepare/service/tests.rs`
- `claudine/lib/src/composition/resolve.rs`
- `claudine/lib/src/composition/looping/engine.rs` and `claudine/lib/src/composition/looping/expression.rs`
- `claudine/lib/src/composition/sequence/preflight/shape.rs` and `claudine/lib/src/composition/sequence/preflight/tests.rs`
- `claudine/lib/src/system_prompt/prepare.rs` and `claudine/lib/src/system_prompt/prepare/tests.rs`
- `claudine/cli/src/commands/compose/prep.rs`
- `claudine/cli/src/commands/wrap/composition/pipeline.rs`
- `claudine/cli/src/commands/wrap/harness_orch/prompt.rs`
- `claudine/cli/src/commands/wrap/overlay.rs`
- `claudine/cli/src/commands/wrap/sequence/jit.rs`, `claudine/cli/src/commands/wrap/sequence/jit/tests.rs`, `claudine/cli/src/commands/wrap/sequence/mod.rs`, and `claudine/cli/src/commands/wrap/sequence/task_run.rs`
- `claudine/cli/src/perf/report.rs`
- `claudine/cli/tests/ctx_launch_anchor.rs`
- `claudine/fixes/2026-08-12-ctx-launch-anchor/plan.md`
- `claudine/fixes/2026-08-12-ctx-launch-anchor/log.md`
- `claudine/fixes/2026-08-12-ctx-launch-anchor/review-1.md`

## Implementation of Review Findings #2

> **started at:** 2026-08-26T20:15:11+01:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-08-12-ctx-launch-anchor/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'per-document-epoch consumer accounting and canonical re-entry proof' at 20:16:28
        - confirmed that prepared-context consumers were retained only as invocation-wide totals and that performance reporting discarded their counts, allowing cross-epoch subsets to appear complete in aggregate
        - added exact document-epoch deltas for launch constructions, stabilized-read extensions, ambient fallbacks, and counted prepared-context consumers without adding identity plumbing to Darkmatter
        - retained consumer counts in performance notes while preserving the existing concise spelling for consumers observed once
        - closed a retry/resume instrumentation blind spot by recording lifecycle consumption when a fresh re-entry snapshot reaches the harness lifecycle seam
        - added Level 1 exact-delta coverage for direct, proxy-target, stabilized-reread, retry, resume, loop, and sequence epochs
                - retry and resume now traverse `dispatch_terminal_control`, including provider-attempt replacement and entry selection, rather than mutating `state.entry` to select the preparation branch
                - every epoch assertion proves exactly one construction, no unpermitted extension, zero ambient fallback, and the exact counted consumer map; the stabilized reread proves zero constructions and exactly one retained-evidence extension
        - focused Level 1 verification passed for the direct, proxy/stabilized-read, canonical retry/resume, loop, sequence, epoch-delta, and performance-report regressions
        - regenerated the checked dispatch inventory after adding the retry/resume re-entry observer; the inventory guard passed with the updated production surface
        - package-area verification completed successfully
                - `just test` passed for `claudine-catalog-types` (21 tests), `claudine` (4,048 tests), `claudine-contract`, and `claudine-gen` (154 tests, 4 skipped)
                - `just test-cli` passed all 2,386 `claudine-cli` tests (245 skipped)
                - `just lint` passed for every Claudine package and its lifecycle/error guards
                - Darkmatter required no verification because this finding made no Darkmatter code or test changes
        - `git diff --check` passed with no whitespace errors
- work completed for 'per-document-epoch consumer accounting and canonical re-entry proof' at 20:43:59
- starting the work on 'current-worktree cross-platform and hosted-CI validation matrix' at 20:44:52
        - confirmed the Review 2 gap is validation-only: the portable source fixtures exist, but the current uncommitted implementation still needs execution evidence on Linux, WSL, and native Windows plus the complete affected macOS and Darkmatter gates
        - selected an isolated transfer strategy that leaves every stale remote clone untouched: a self-contained bundle for local `HEAD` plus a binary tracked-worktree overlay, cloned and applied only in validated temporary directories
        - the snapshot source is macOS host `Mac.home.ken.net`, branch `fix/ctx-launch-anchor`, `HEAD` `a0aa98cff6aefc10053d047f09a12abe84873bfe`; all current worktree changes are tracked and there are no relevant untracked files
        - the current macOS Claudine matrix is green
                - the review-cycle Finding 1 gate passed the current implementation's full Level 1 suite and lint checks before this validation finding began; only this Markdown log changed before the Level 2 run
                - canonical `just test-l2` passed 230 of 230 `claudine-cli` tests and three of three `claudine-gen` tests
                - the CLI leg explicitly reported parallel self-spawn mode with eight workers and no shared pane; the generator leg used managed background resources, only tmux-backed tests executed, and neither a browser tier nor OS-input tier was invoked
        - the fresh affected macOS Darkmatter matrix is green: `just test` passed 6,247 `darkmatter`, 653 `darkmatter-cli`, and 640 `dmls` tests; `just lint` passed all three packages
        - created and locally reconstructed the exact tracked worktree as a 208 KiB incremental Git bundle plus a 44 KiB binary overlay
                - the bundle SHA-256 is `7553a22b52d7bd9f9aa7f17489ceb42a8126ecec560973e97c61d061eaee69f4`
                - the overlay SHA-256 is `f08eb4065f4c927d2126da842f7384991cf3a00abe69fffe2c5dc3966cfcf27a`
                - local clone, prerequisite verification, checkout, `git apply --check`, application, and reconstructed-diff comparison all passed
        - staged the exact snapshot without touching either stale clone on the Windows build host
                - WSL path `/tmp/rb-ctx-anchor-Eo6mKS/repo` is at `a0aa98cff6aefc10053d047f09a12abe84873bfe`; both transferred hashes and the reconstructed binary-diff hash match the local values
                - native Windows path `C:\Users\ken\AppData\Local\Temp\rb-ctx-anchor-d840e08095364fbbb4c8553bdec4ac22\repo-long` is at the same commit with matching transfer and reconstructed-diff hashes
                - the first native checkout exposed the repository's long tracked paths; a fresh temp subdirectory with repository-local `core.longpaths=true` completed successfully without altering the stale clone
        - WSL `just test` began a cold Claudine build but the remote SSH server closed the connection before any test summary; the immediate bounded reconnect then timed out, so no WSL test evidence was claimed
        - native Windows `just test` reached the repository storage preflight and ran no tests because C: had 1.9 GiB free against the required 50 GiB
                - `sniff storage --json` found only C: with 1.9 GiB free and W: with 8.2 GiB free, so no available volume satisfies the safety preflight
                - the storage guard was not bypassed and unrelated build artifacts were not removed
        - `build-linux` initially responded and reported Debian 13, Rust/Cargo 1.97.1, Just 1.57.0, tmux 3.5a, and both incremental-bundle prerequisites; three subsequent bounded SSH attempts timed out before creating a temp directory, so no Linux snapshot or test evidence was claimed
        - repository hosted-CI audit found no workflow input capable of receiving the uncommitted bundle or patch; the relevant workflows always use `actions/checkout` at a published GitHub ref
        - recovered WSL through the native Windows SSH endpoint and bounded noninteractive `wsl.exe --exec` commands after the WSL SSH daemon became unavailable
                - the original `/tmp` snapshot disappeared when the WSL distribution stopped because `/tmp` is a tmpfs mount; the exact source snapshot was restaged on persistent storage at `/home/ken/rb-ctx-anchor-qlt02v/repo`
                - `HEAD` is `a0aa98cff6aefc10053d047f09a12abe84873bfe`, the transferred bundle and overlay hashes match the local values, the reconstructed binary diff matches `f08eb4065f4c927d2126da842f7384991cf3a00abe69fffe2c5dc3966cfcf27a`, and all 15 expected tracked modifications are present
        - the complete WSL Claudine matrix is green on Rust/Cargo 1.97.1 and Just 1.57.0
                - `just test` passed all five package groups: 21 of 21 `claudine-catalog-types`, 4,048 of 4,048 `claudine`, 48 of 48 `claudine-contract`, 2,386 of 2,386 `claudine-cli`, and 154 of 154 `claudine-gen` tests
                - canonical `just test-l2` passed 230 of 230 `claudine-cli` tests and three of three `claudine-gen` tests; no direct Level 2 invocation was used
                - the Level 2 recipe reported parallel self-spawn mode with eight workers and no shared pane; WSL ran the supported tmux-backed fixtures, unsupported WezTerm probes returned without opening a window, and the generator used a managed tmux session, so no terminal or browser window gained focus
                - no Level 2 fix was needed, so AC14 did not require a Level 1 rerun
                - `just lint` passed its 18 lifecycle/error guard tests and all catalog, library, contract, CLI, and generator lint phases with no findings
        - the complete affected WSL Darkmatter matrix is green
                - `just test` passed 6,247 of 6,247 `darkmatter`, 653 of 653 `darkmatter-cli`, and 640 of 640 `dmls` tests
                - the first `just lint` attempt reached no code diagnostic because the cumulative cold matrix filled the WSL filesystem while Rust wrote metadata; the isolated snapshot's reproducible `target` directory measured 48 GiB and accounted for the exhaustion
                - after exact-path validation, only `/home/ken/rb-ctx-anchor-qlt02v/repo/target` was removed, restoring 48 GiB; the clean canonical `just lint` rerun passed `darkmatter`, `darkmatter-cli`, and `dmls` with no findings
        - `build-linux` briefly recovered and reported kernel `7.0.14-6-pve`, 160 GiB free in `/tmp`, and the unchanged stale base clone at `7685d1ac29920f874a2a83a7cecec0775c08aaaa`
                - created isolated path `/tmp/rb-ctx-anchor-a3Nzbs` and transferred the incremental bundle plus a refreshed binary overlay whose `56d50e691a274958fdd2c5fc485b6b4a1d61411763830cec473cd1263e0e6438` hash includes the validation-log progress written after the first snapshot
                - the SSH service returned to banner-exchange and connection timeouts before clone, patch verification, or tests could run; three bounded staging attempts and a final cleanup attempt failed, and the stale clone was not modified
                - Linux validation is deferred because the build host did not remain reachable long enough to reconstruct or execute the current worktree; `/tmp/rb-ctx-anchor-a3Nzbs` may still contain only the transferred 208 KiB bundle and 44 KiB overlays and must be removed on the next successful connection
        - native Windows validation is deferred because the canonical repository storage preflight requires 50 GiB free and no available native volume met it; the guard was not bypassed, so no native Level 1, Level 2, or lint verdict is claimed
        - hosted-CI validation is deferred because every applicable workflow checks out a published GitHub ref and none accepts an uncommitted bundle or overlay; the task did not authorize a commit or push, so hosted CI cannot exercise this exact implementation safely
        - cleaned the isolated validation artifacts after the completed WSL matrix
                - exact-path-validated removal of `/home/ken/rb-ctx-anchor-qlt02v` restored WSL to 49 GiB free
                - exact-path-validated removal of `C:\Users\ken\AppData\Local\Temp\rb-ctx-anchor-d840e08095364fbbb4c8553bdec4ac22` restored native C: to 5,428,461,568 free bytes
                - the local `/tmp` audit found no remaining `rb-ctx-*` staging directory; the removed source copies and build artifacts are reproducible from the local worktree
- work completed for 'current-worktree cross-platform and hosted-CI validation matrix' at 21:35:08

### Successful Completion

The implementation of review cycle 2 has completed successfully in 1 hour and 22 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 was fixed, 1 was deferred (see reasons below):

- finding 2, 'current-worktree cross-platform and hosted-CI validation matrix', was completed for macOS and WSL but deferred for Linux, native Windows, and hosted CI because those environments could not safely execute the exact current worktree
        - Linux accepted the isolated bundle and overlay, but repeated SSH banner-exchange and connection timeouts prevented reconstruction, verification, testing, and final cleanup; `/tmp/rb-ctx-anchor-a3Nzbs` must be removed on the next successful connection
        - native Windows had no volume satisfying the repository's canonical 50 GiB free-space preflight, so the guard was not bypassed and no test or lint verdict was claimed
        - hosted workflows only test published GitHub refs and accept no uncommitted bundle or overlay; this task did not authorize the commit and push required to make the exact implementation available to hosted CI

The files changed by this implementation are:

- `claudine/lib/src/invocation_context.rs` and `claudine/lib/src/invocation_context/tests.rs`
- `claudine/lib/src/composition/prepare/tests.rs`
- `claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs`
- `claudine/cli/src/commands/wrap/harness_orch/prompt.rs`
- `claudine/cli/src/commands/wrap/sequence/jit/tests.rs`
- `claudine/cli/src/perf/report.rs` and `claudine/cli/src/perf/tests/report.rs`
- `claudine/cli/tests/ctx_launch_anchor.rs`
- `claudine/docs/providers/dispatch-inventory.json`
- `claudine/fixes/2026-08-12-ctx-launch-anchor/review-2.md`
- `claudine/fixes/2026-08-12-ctx-launch-anchor/log.md`
