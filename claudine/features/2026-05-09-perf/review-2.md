---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: OpenCode/Qwen cold-cache model validation is still synchronous

The spec's second goal requires a `>= 10x` prep reduction for cold-cache OpenCode or Qwen invocations with frontmatter `model:` validation (`spec.md:25-27`), and W3 explicitly designs background model-catalog refresh for that path (`spec.md:166-183`). The implementation still calls `catalog.refresh_provider_blocking(provider)` from `refresh_for_model_validation` (`claudine/cli/src/commands/wrap/composition/mod.rs:456`).

That means the exact slow path W3 was designed for still shells out synchronously before execution can continue. The post-W0 trace only exercises `--claude`, and the trace itself says W3 was not required for that tested scenario. It does not verify the OpenCode/Qwen cold-cache requirement. This is a designed requirement that remains unimplemented, not just an optimization left on the table.

Verification level present: no OpenCode/Qwen cold-cache automated test or trace exercising frontmatter `model:` validation. Required: Level 1 is sufficient for this non-terminal subprocess/caching behavior, with a fake provider `models` command or injectable catalog refresh proving the compose path does not block when a stale cache exists, plus a no-cache cold-start fallback case.

### High: Time-to-first-feedback target is not automatically verified

W1 now has useful Level 1 integration tests for ordering and suppression in `claudine/cli/tests/compose_receipt_banner.rs`, and the code emits the receipt banner before source/prep work (`claudine/cli/src/commands/compose.rs:283-294`, `:561-572`). Those tests do not verify the actual requirement from the spec: `<= 50 ms` from `main()` entry to first stderr line (`spec.md:25`).

The only timing evidence I found is the manual `trace-after-w0.md` claim that the banner appears within ~50 ms. That is useful for investigation, but it is not a regression test. A future change could move config loading, terminal detection, or another expensive operation before the banner and the current tests would still pass as long as the banner remains before the execution header.

Verification level present: Level 1 subprocess tests for ordering/suppression; manual timing note for latency. Required: Level 1 timing harness is appropriate here. It does not need Level 2/3 because no terminal encoder behavior is involved, but it should measure first stderr byte from process start under a representative compose invocation and fail or at least be an explicitly ignored perf gate with a documented threshold.

### Medium: The checked-in post-W0 trace is stale and contradicts the fixed renderer

`trace-after-w0.md` still records the pre-review broken totals: `CLI Overhead TOTAL: 1.38s` and `Composition Report TOTAL: 0µs` (`trace-after-w0.md:26-34`). The code now fixes that renderer, and `review-1.md` says the finding was addressed, but the feature's supporting trace still documents the old output and then marks report formatting accepted (`trace-after-w0.md:107-112`).

This is not a runtime bug, but it weakens the diagnosability goal. Contributors using the feature folder as the evidence trail will see a trace that demonstrates the exact bug that was supposedly fixed.

Recommendation: regenerate `trace-after-w0.md` after the renderer fix, or add a note that the captured output predates review-1 fixes and should not be used to validate section totals.

### Medium: W0's automated regression test does not prove the "at most one scan" contract

The W0 code path now threads `prep_launch_workspace` into execution and falls back to `resolve_launch_workspace_context` only when absent (`claudine/cli/src/commands/wrap/composition/mod.rs:556-560`), and the implementation direction is sound. The new prep-context tests check shape and split-contract behavior, but `prep_context_launch_workspace_avoids_redundant_walks` does not instrument `detect_git`, `detect_repo`, or `resolve_launch_workspace_context`; it infers no-walk behavior from resulting fields (`prep_context.rs:278-316`).

That leaves the exact performance regression from the spec under-tested. A future refactor could accidentally add another scan while preserving the same returned values. The spec requested a synthetic counter or tracing-driven equivalent for this reason.

Verification level present: Level 1 unit tests for data shape and a manual trace. Required: Level 1 instrumentation/counter test proving the compose hot path performs no fallback workspace scan when prep supplies the launch workspace.

## Test Rigor Matrix

| Requirement | Strongest verification observed | Required | Status |
|---|---:|---:|---|
| W0 reuses precomputed launch workspace for header/env build | Level 1 shape tests + manual trace | Level 1 counter/instrumented regression | Partial |
| W0 preserves source-repo metadata while child CWD follows launch repo | Level 1 unit tests | Level 1 | OK |
| W1 receipt banner appears before execution header | Level 1 subprocess integration | Level 1 | OK |
| W1 `--silent` / `--quiet` behavior | Level 1 subprocess integration | Level 1 | OK |
| TTFF `<= 50 ms` from process start to first stderr byte | Manual trace/note only | Level 1 timing harness | Gap |
| W2 binary path snapshot avoids `which` | Level 1 unit test with synthetic snapshot path | Level 1 | OK |
| W8/W9 perf report totals and alignment | Level 1 snapshot-style unit test | Level 1 | OK |
| W3 background refresh for stale OpenCode/Qwen model catalogs | None; implementation remains blocking | Level 1 | Gap |

## Verification Run

- `cargo check -p claudine --no-default-features` passed.
- `cargo test -p claudine-cli --test compose_receipt_banner --no-default-features` passed.
- `cargo test -p claudine-cli perf --no-default-features` passed.

## Production Readiness

Not ready. The W0/W1/W2 implementation is much stronger after the prior review fixes, but the feature still misses the spec's OpenCode/Qwen cold-cache model-validation path and lacks automated verification for the headline `<= 50 ms` first-feedback requirement.
