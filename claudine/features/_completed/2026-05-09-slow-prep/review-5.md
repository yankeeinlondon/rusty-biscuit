---
ready: false
agent: codex
model: ""
---

# Review: 2026-05-09 Slow Compose Prep

## Findings

### High: selected dynamic-provider frontmatter models can be rejected against stale cache

The spec allows dynamic refresh to avoid blocking only when no selected model needs catalog validation:

- `claudine/features/2026-05-09-slow-prep/spec.md:89`
- `claudine/features/2026-05-09-slow-prep/spec.md:120`
- `claudine/features/2026-05-09-slow-prep/spec.md:239`

The current refresh gate correctly runs only when a frontmatter `model` hint will be validated, but it now calls `refresh_provider_async()`:

- `claudine/cli/src/commands/wrap/composition/mod.rs:484`
- `claudine/cli/src/commands/wrap/composition/mod.rs:508`

For OpenCode/Qwen with an existing cache entry, `refresh_provider_async()` intentionally spawns a detached refresh and returns immediately, leaving the current invocation on the old cache:

- `claudine/lib/src/model_catalog/service.rs:247`
- `claudine/lib/src/model_catalog/service.rs:255`
- `claudine/lib/src/model_catalog/service.rs:260`

Immediately after that, model resolution validates frontmatter `model` against the catalog it has right now. If the hinted model is absent from the stale cache, it is silently ignored and the provider default is used:

- `claudine/lib/src/composition/select.rs:395`
- `claudine/lib/src/composition/select.rs:399`

This regresses the selected dynamic-provider case the spec explicitly preserved. A user with a warm but stale OpenCode/Qwen cache who writes `model: newly-available-model` will not get that model on the current run, even though `opencode models` would validate it. Stale cache is acceptable as a fallback after refresh failure or unavailability; it should not replace validation when the selected provider and selected frontmatter model require correctness.

Verification level: Level 1 is appropriate for this requirement. Current Level 1 tests verify that the async path returns promptly when cache exists and that a frontmatter model attempts refresh on cold cache, but I did not find a test proving a selected OpenCode/Qwen frontmatter model missing from stale cache is validated against the fresh dynamic result before launch.

Suggested fix: for selected dynamic providers with frontmatter `model` and no CLI/env override, either block on a cancellable provider-scoped refresh before validation, or change resolution to treat stale-cache misses as "catalog unavailable" rather than "model invalid" while the refresh is in flight. Add a Level 1 test with a seeded stale cache and an injectable/fake dynamic catalog containing the hinted model.

## Coverage Notes

- Explicit Claude/Codex compose and inline-compose no longer invoke `opencode models`; Level 1 CLI tests now cover all four direct acceptance paths.
- Ctrl+C during a slow dynamic refresh now has Level 1 process/signal coverage with a bounded latency assertion. That is the right level for Claudine's SIGINT contract; Level 3 would only be needed for a terminal-specific "physical Ctrl+C keypress emits SIGINT" requirement.
- Shell preflight behavior remains Level 1 covered. I did not see evidence that this feature skipped shell discovery for speed.
- The feature has no Level 2 terminal-rendering requirement unless exact styling of the interrupt notice becomes part of acceptance.

## Validation

I attempted targeted cargo validation:

```sh
cargo test -p claudine model_catalog::service::tests::concurrent_opencode_qwen_refresh_runs_fetcher_once --no-default-features --color=never
cargo test -p claudine-cli compose_sigint_during_prep_exits_130_with_notice --no-default-features --color=never
cargo test -p claudine-cli inline_compose_codex_dry_run_does_not_call_opencode_models --no-default-features --color=never
```

The commands contended on Cargo package/artifact locks and were still compiling dependencies after about a minute, so I stopped them in this non-interactive review session. I did not get completed test results.

## Readiness

Not ready for production. The prior review's concrete test gaps were addressed, and the main `--claude`/`--codex` performance path looks substantially improved, but selected OpenCode/Qwen frontmatter model validation is still incorrect when an existing cache is stale.
