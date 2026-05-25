---
ready: true
agent: codex
model: ""
---

# Review: 2026-05-09 Slow Compose Prep

## Findings

### High: Ctrl+C during blocking prep is not prompt or bounded

`install_user_interrupt_guard()` only marks a process-scoped flag and writes the notice from the SIGINT handler:

- `claudine/cli/src/commands/compose.rs:995`
- `claudine/cli/src/commands/compose.rs:996`

The direct compose/inline-compose paths do not check that flag until after eager target resolution and shell preflight complete:

- `claudine/cli/src/commands/compose.rs:390`
- `claudine/cli/src/commands/compose.rs:395`

For selected dynamic providers with a frontmatter `model` and no cache, the refresh path still blocks inside `refresh_provider_async()` by falling back to `refresh_provider_blocking()`:

- `claudine/lib/src/model_catalog/service.rs:237`
- `claudine/lib/src/model_catalog/service.rs:241`
- `claudine/lib/src/model_catalog/service.rs:182`
- `claudine/lib/src/model_catalog/service.rs:192`

That means a Ctrl+C during a slow or hung `opencode models` subprocess renders the INFO notice, but the command cannot actually return 130 until the subprocess exits. The new acceptance test uses a 5 second sleep and asserts only eventual exit:

- `claudine/cli/tests/wrap_commands.rs:5562`
- `claudine/cli/tests/wrap_commands.rs:5601`
- `claudine/cli/tests/wrap_commands.rs:5604`

This leaves the original "prep appears hung" UX partly unfixed for the cold-cache dynamic-provider case. The test should assert a bounded interrupt latency, and the blocking refresh path should either be cancellable, poll the interrupt flag while waiting, or avoid a foreground `join()` around a child process that may still be running.

Verification level: Level 1 signal/process coverage is the right boundary for Claudine's SIGINT handling here, but the current Level 1 test is incomplete because it does not verify prompt exit from the blocked prep window.

### Medium: Inline Codex no-`opencode models` acceptance path is untested

The spec acceptance criteria cover explicit Claude/Codex compose and inline-compose runs:

- `claudine/features/2026-05-09-slow-prep/spec.md:238`

Current CLI coverage includes `compose --claude`, `inline-compose --claude`, and `compose --codex`, but I did not find the matching `inline-compose --codex` test with a failing `opencode models` double:

- `claudine/cli/tests/wrap_commands.rs:5354`
- `claudine/cli/tests/wrap_commands.rs:5382`
- `claudine/cli/tests/wrap_commands.rs:5414`

The code path is likely shared enough that this is low implementation risk, but the stated acceptance matrix is not fully verified. Add the missing Level 1 CLI test.

## Coverage Notes

- Provider-scoped refresh and OpenCode/Qwen dedup now have Level 1 unit coverage, including concurrent refresh with an injectable fetcher.
- The previous `--repo` launch-context error swallowing issue is addressed by preserving `prep_launch_detection_error` and testing `enforce_repo_launch_detection()`.
- Shell preflight remains Level 1 covered by existing CLI tests; I did not see evidence that the feature attempted unsafe preflight/final-compose reuse.
- No Level 2 terminal-rendering requirement is central to this performance feature unless exact styling of the interrupt notice is promoted to acceptance.
- No Level 3 keyboard injection is required for the catalog/perf behavior. For Ctrl+C, Level 1 signal delivery is appropriate if the contract is "SIGINT received during prep"; Level 3 would only be needed if the requirement becomes "pressing physical Ctrl+C in terminal X emits the expected signal."

## Validation

I attempted targeted validation:

```sh
cargo test -p claudine model_catalog::service::tests::concurrent_opencode_qwen_refresh_runs_fetcher_once --no-default-features --color=never
cargo test -p claudine-cli compose_sigint_during_prep_exits_130_with_notice --no-default-features --color=never
```

The workspace was still compiling dependencies after several minutes, so I stopped the cargo processes and did not get completed test results in this non-interactive review session.

## Readiness

Not ready for production. The main performance path is much improved, and the review-3 issues appear implemented, but Ctrl+C during a blocked prep subprocess is still not a bounded interrupt path, and the acceptance test currently cannot catch that.
