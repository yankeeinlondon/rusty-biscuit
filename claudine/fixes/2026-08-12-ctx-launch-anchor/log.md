---
implementation_1: "2026-08-26T15:46:25+01:00"
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
