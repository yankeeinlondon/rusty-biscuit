---
ready: false
agent: codex
model: ""
---

# Review: 2026-05-09 Slow Compose Prep

## Findings


### Medium: OpenCode/Qwen dedup is not concurrency-safe

`fetch_opencode_with_dedup()` checks the dedup slot under a mutex, releases the lock, awaits `fetch_opencode_models()`, then stores the result:

- `claudine/lib/src/model_catalog/service.rs:197`
- `claudine/lib/src/model_catalog/service.rs:201`
- `claudine/lib/src/model_catalog/service.rs:203`

That dedupes sequential refreshes, including current `refresh_all()`, but two concurrent `refresh_provider(OpenCode)` / `refresh_provider(QwenCode)` calls can both observe `None` and both spawn `opencode models`. The spec requires the underlying OpenCode source to be shared when both providers need it in one process. The current implementation only satisfies that for sequential callers.

Verification level: Level 1 unit coverage exists for primed/sequential dedup, but not for concurrent refreshes. Use an in-flight shared future/OnceCell-style state or hold a single async-aware initialization path, and add a test that drives OpenCode and Qwen refresh concurrently with an injectable fake source.

### Medium: `--repo` launch-context errors can now be swallowed

`CompositionPrepContext::new()` builds the shared sniff result with `sniff::detect_with_plan(plan).unwrap_or_default()`:

- `claudine/cli/src/commands/wrap/composition/prep_context.rs:115`

The executor then trusts `request.prep_launch_context` and skips the old `LaunchContext::from_cwd()` error path:

- `claudine/cli/src/commands/wrap/composition/mod.rs:952`
- `claudine/cli/src/commands/wrap/composition/mod.rs:961`

Previously, `--repo` converted launch-context detection failure into a hard error. With a prep context present, a sniff failure becomes an empty launch context and the `--repo` guard is bypassed. Detection failures are uncommon, but this is a correctness regression in the new invocation-context plumbing.

Verification level: missing Level 1 unit/integration coverage. Preserve the sniff error in `CompositionPrepContext` or keep a fallible launch-context path for `--repo`, and test that a launch-context detection failure still fails when `--repo` is set.

## Coverage Notes

- Provider/model refresh gating now has Level 1 unit and CLI coverage for direct compose and sequence env-var override cases.
- The explicit Claude/Codex no-`opencode models` acceptance path has Level 1 CLI coverage via a failing `opencode` test double.
- Ctrl+C during prep has Level 1 signal/process coverage. Level 3 keyboard injection is not required here because Claudine observes the delivered SIGINT, not terminal key encoding.
- No Level 2 terminal rendering requirement is central to this performance feature unless exact styling of the interrupt notice is made part of acceptance.

## Validation

Ran targeted validation:

```sh
cargo test -p claudine-cli compose_sigint_during_prep_exits_130_with_notice --no-default-features
cargo test -p claudine-cli sequence_opencode_dry_run_with_env_model_skips_opencode_models_call --no-default-features
cargo test -p claudine model_catalog::service::tests::refresh_all_dedupes_opencode_for_opencode_and_qwen --no-default-features
```

All three passed.

## Readiness

Not ready for production. The slow-prep acceptance path is much better covered now, but the branch removes an existing OpenCode hang-recovery behavior outside the scope of this feature, and the new shared-context path can bypass the existing `--repo` detection-failure contract.
