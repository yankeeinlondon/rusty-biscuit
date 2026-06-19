---
ready: false
agent: codex
model: ""
---

# Review: Always-Harness

## Verdict

Not ready for production.

The previous high-risk behavioral gaps are addressed: dry-run inline writability now fails before provider launch, parsed-harness loop rate-limit abort is covered at Level 1, and the focused tests pass. The remaining issue is cleanup/comment drift in live code that contradicts the new loop-signal contract and fails the spec's explicit acceptance scan.

## Findings

### Medium: Stale live-code comment still describes the removed non-harness signal path

Spec requirement: the cleanup acceptance scan should return no live-code or current-doc references to the removed split path:

```bash
rg -n "execute_without_harness|CompositionExecutionMode|run_structured_branch|inline_guards|non-harness path|without harness" claudine/cli/src claudine/docs .claude/skills/claudine
```

Implementation issue: the scan still reports a live source comment on [`SingleCompositionOutcome::iteration_signals`](../../cli/src/commands/wrap/composition/mod.rs:134):

```rust
/// Populated for the non-harness structured-stream path ...
/// `None` for the dry-run, harness, and legacy paths ...
```

That is now wrong. [`execute_composition_request_inner`](../../cli/src/commands/wrap/composition/mod.rs:1956) receives `harness_signals` from `run_harness_loop`, and [`run_harness_loop`](../../cli/src/commands/wrap/harness_orch.rs:794) returns the terminal attempt's `IterationSummarySignals`. This is the intended fix for `compose --loop` rate-limit and exit-reason propagation, so the code is correct and the comment is stale.

The same scan also reports two historical timeline entries in [timeline.md](../../../.claude/skills/claudine/timeline.md:7). Those entries are explicitly historical and do not describe current behavior, so I would not treat them as a production blocker. The live source comment should be fixed because it misleads future maintainers about exactly the contract this feature changed.

Verification level: documentation/comment drift, not user-observable runtime behavior. No L2/L3 verification applies.

Suggested fix: update the `iteration_signals` field docs to say dry-run returns `None`, while non-dry-run composition returns the harness loop's terminal-attempt structured summary signals when available. Then rerun the acceptance scan and decide whether to exempt historical timeline notes or reword them to avoid the legacy symbol names.

## Verification Notes

- `cargo test -p claudine-cli --test inline_compose_cli inline_compose_dry_run_fails_on_read_only_source --color=never -- --nocapture` passed.
- `cargo test -p claudine-cli --test loop_cli compose_loop_rate_limit_abort_exits_75_on_harness_doc --color=never -- --nocapture` passed.
- `cargo check -p claudine -p claudine-cli --color=never` passed.

## Coverage Matrix

| Requirement | Strongest verification found | Status |
|---|---:|---|
| Bare and parsed-harness direct compose converge | Level 1 integration (`compose_cli`) | OK |
| Bare and parsed-harness inline compose converge on final response only | Level 1 integration (`inline_compose_cli`, `wrap_commands`) | OK |
| Interactive composition routes through unified execution and interactive inline closure is preserved for supported providers | Level 1 integration plus existing Level 2 schema prompt coverage | OK |
| Provider non-zero exit code remains observable | Level 1 integration for compose/provider failure and loop failure classification | OK |
| Dry-run does not launch provider or mutate inline source files | Level 1 integration | OK |
| Dry-run validates inline writability | Level 1 regression test added and passing | OK |
| Loop exit-reason signals survive bare and parsed-harness direct composition | Level 1 integration | OK |
| Loop rate-limit signals survive bare and parsed-harness direct composition | Level 1 integration, including parsed-harness abort variant | OK |
| Inline writability pre-check is injected once per live attempt | Level 1/unit coverage for plan shape and live failure path | OK |
| Removed-path cleanup scan is satisfied for live source/current docs | Static scan | Gap |
