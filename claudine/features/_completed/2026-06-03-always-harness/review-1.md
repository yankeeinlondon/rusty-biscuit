---
ready: false
agent: codex
model: ""
---

# Review: Always-Harness

## Verdict

Not ready for production.

The main execution-path unification compiles and the new L1 convergence tests pass, but one dry-run requirement is broken and one loop requirement lacks the required coverage for parsed-harness documents.

## Findings

### High: `inline-compose --dry-run` does not validate source writability

Spec requirement: dry-run must still perform inline writability validation before returning, while not launching the provider or mutating the file.

Implementation issue: [`execute_composition_request_inner`](../../cli/src/commands/wrap/composition/mod.rs:1723) finalizes the plan and runs shell approval, then returns at the dry-run seam at [`composition/mod.rs:1766`](../../cli/src/commands/wrap/composition/mod.rs:1766). The actual pre-check evaluation, including the system-owned `HasWritePermission` rule, only happens later inside `run_harness_loop` at [`harness_orch.rs:995`](../../cli/src/commands/wrap/harness_orch.rs:995), which dry-run never calls.

I confirmed this with a manual L1 CLI probe: a `0444` inline source file under `inline-compose --goose --dry-run` exited `0` and rendered dry-run output instead of failing the writability pre-check.

Verification level: missing/broken Level 1. This is process behavior and filesystem validation, so L1 is the right level.

Suggested fix: before the dry-run return, evaluate the finalized effective plan's pre-checks for inline mode with the same `WrapperHarnessPermissionProbe` used by the harness loop, or factor a shared preflight helper so dry-run and live execution cannot drift.

### High: Parsed-harness `compose --loop` rate-limit behavior is not verified

Spec requirement: `compose --loop` must still receive rate-limit and exit-reason signals from both bare direct composition and parsed-harness direct composition.

Current tests cover exit-reason propagation for both bare and minimal harness documents in [`loop_cli.rs`](../../cli/tests/loop_cli.rs:987), and existing rate-limit tests cover the bare document path around [`loop_cli.rs:641`](../../cli/tests/loop_cli.rs:641). I did not find a parsed-harness sibling for rate-limit pause/abort behavior.

Verification level: missing Level 1 for parsed-harness rate-limit behavior. L1 is sufficient because the observable contract is CLI exit/status behavior from manufactured structured stream bytes, not terminal emulator rendering.

Suggested fix: add minimal-harness variants for at least `--on-rate-limit abort` and preferably the pause/no-reset cases. Use `post_checks: []` or another harmless harness key so the parsed-harness plan is exercised.

### Medium: The acceptance cleanup scan still reports live references

Spec acceptance says this command should return no live-code or current-doc references:

```bash
rg -n "execute_without_harness|CompositionExecutionMode|run_structured_branch|inline_guards|non-harness path|without harness" claudine/cli/src claudine/docs .claude/skills/claudine
```

It still reports live `inline_guards` references in [`composition/mod.rs`](../../cli/src/commands/wrap/composition/mod.rs:46) and [`inline.rs`](../../cli/src/commands/wrap/inline.rs:185), plus a `without harness frontmatter` comment in [`composition/mod.rs`](../../cli/src/commands/wrap/composition/mod.rs:1641). The old `apply_inline_closure` path is gone, so this is not the same severity as a runtime split, but the stated acceptance scan is not satisfied.

Suggested fix: move the remaining cleanup/frontmatter helpers out of `composition/inline_guards.rs` into a name that reflects their current role, such as `inline_cleanup.rs`, and update comments/docs so the acceptance scan is meaningful.

## Verification Notes

- `cargo check -p claudine -p claudine-cli --color=never` passed.
- `cargo test -p claudine-cli --test compose_cli --test inline_compose_cli -- --nocapture` passed.
- `cargo test -p claudine-cli --test loop_cli compose_loop_exit_reason_surfaces -- --nocapture` passed.
- `cargo test -p claudine-cli --test wrap_commands inline_compose -- --nocapture` passed.

## Coverage Matrix

| Requirement | Strongest verification found | Status |
|---|---:|---|
| Bare and parsed-harness direct compose converge | L1 integration (`compose_cli`) | OK |
| Bare and parsed-harness inline compose converge on final response only | L1 integration (`inline_compose_cli`, `wrap_commands`) | OK |
| Interactive inline closure with Codex still rewrites body | L1 integration; related L2 PTY schema collection exists | OK |
| Provider non-zero exit code remains observable | L1 integration for compose/provider failure | OK |
| Dry-run does not launch provider or mutate files | L1 integration | OK |
| Dry-run validates inline writability | Manual L1 probe fails; no dedicated regression test | Gap |
| Loop exit-reason signals survive bare and parsed-harness direct composition | L1 integration | OK |
| Loop rate-limit signals survive bare and parsed-harness direct composition | L1 only for bare; no parsed-harness variant found | Gap |
| Inline writability pre-check is injected once per live attempt | L1/unit coverage for plan shape and live failure path | OK |
