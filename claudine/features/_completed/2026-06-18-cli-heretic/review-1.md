---
agent: codex
model: ""
ready: false
---

# Review 1

## Findings

### High: The implementation is not scoped to the CLI-only refactor promised by the spec

The spec defines this as a behavior-preserving `claudine-cli` restructure and explicitly says there should be "No reach into the `claudine` library or `claudine-contract`" (`spec.md`, lines 77-104). The current diff reaches well beyond that boundary: `claudine/lib/src/composition/{error.rs,loop_actions.rs,loop_config.rs,loop_engine.rs,mod.rs,prepare.rs,select.rs,types.rs}` are modified, and `darkmatter` has 26 changed files including new expression/catalog code.

That makes the branch hard to certify as a pure god-file dismantling pass. Even if those changes are needed by another feature, they need to be split out or explicitly documented as required compile fallout with their own behavioral review and tests. As-is, the review surface includes expression semantics and composition-library behavior changes that are outside this spec.

Verification level: L1 build/lint/tests pass, but this is a scope/behavior-preservation gap, not a terminal-behavior gap.

### Medium: Kimi wire-mode was not extracted into the designed wire protocol module

Phase 4 requires extracting Kimi wire-mode protocol code from `wrap/exec/wiring.rs` into `wrap/exec/wire/` modules for session lifecycle, request dispatch, `WireWriter`, and exit handling. The implementation still has the core wire session and dispatch path in `claudine/cli/src/commands/wrap/exec/wiring/mod.rs`: `run_kimi_wire_session` starts at line 615 and `handle_request_dispatch` starts at line 855. There is no `wrap/exec/wire/` module; the produced structure is still `wiring/{builders,dispatch,session,writer}.rs`.

The high-risk gate passes, but this misses a specified responsibility split and leaves `wiring/mod.rs` as a 765-SLOC moderate-risk file with the Kimi lifecycle still centralized. This is not a user-facing correctness failure, but it is an incomplete implementation of the planned ergonomic/maintainability work.

Verification level: L1 tests cover wrapper behavior; no additional L2/L3 requirement applies to this structural requirement.

### Medium: Repository whitespace hygiene check fails after the split

`git diff --check main...HEAD` fails with blank-line-at-EOF issues in multiple moved/split claudine files, including `claudine/cli/src/commands/wrap/sequence/mod.rs`, `claudine/cli/tests/sequence_cli.rs`, `claudine/cli/tests/wrap_basics.rs`, `claudine/cli/tests/wrap_inline_compose.rs`, `claudine/cli/tests/wrap_opencode.rs`, `claudine/cli/tests/wrap_structured_stream.rs`, and others. It also reports trailing whitespace in `prompts/review-suggestions.md`.

The spec is strict about no formatting churn and merge hygiene. This is easy to fix surgically without running `cargo fmt`: remove extra final blank lines and trailing spaces in the reported files.

Verification level: static repository hygiene check.

### High: Full Level 2 verification is not complete in this review pass

The spec requires relevant L2/PTY verification for moved PTY tests and terminal-rendered behavior. I ran `just test-l2`; it spawned the real terminal harnesses and passed 7/57 tests, then I interrupted it after about 130 seconds because the serial tmux capture cases were still running. The final result was nextest exit 100 with 50/57 tests not run.

This is not evidence that L2 is failing, but it means the production-readiness claim is not fully verified here. User-observable terminal layout/styling requirements touched by the moved L2 files currently have only partial Level 2 confirmation from this review.

Verification level: partial L2 only; full L2 required before marking ready.

## Coverage Classification

- God-file metric: verified. `hug god-files claudine/cli --high-risk --plain` reports 0 high-risk files.
- No new high-risk files: verified. `hug god-files claudine/cli --plain` reports 0 high-risk files and 72 moderate-risk files.
- Compile: Level 1, verified. `just build` passed.
- Lint: Level 1/static, verified. `just lint` passed.
- CLI test suite: Level 1, verified. `just test-cli` passed: 1606 run, 1606 passed, 67 skipped.
- Terminal-rendered behavior: Level 2, partial only. `just test-l2` passed 7/57 before interruption; 50 tests were not run.
- OS keyboard injection: Level 3 not applicable for this spec; claudine's `test-l3` recipe is a no-op.

## Production Readiness

Not ready. The core `hug` target is satisfied and the L1 gates are strong, but production readiness is blocked by the out-of-scope library/darkmatter changes, incomplete Kimi wire-mode extraction relative to the plan, whitespace hygiene failures, and incomplete full L2 verification.
