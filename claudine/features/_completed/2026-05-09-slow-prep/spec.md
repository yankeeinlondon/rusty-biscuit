# Slow Compose Prep

## Problem

`claudine compose`, `claudine inline-compose`, and `claudine sequence` can spend multiple seconds in the pre-launch phase before the provider process is spawned. The user sees no provider output during this interval and, before the companion Ctrl+C UX fix, a SIGINT during this window did not render the same clean user-interrupt notice used by the loop executor.

The most important distinction for this feature is that the delay is not in Claude, Codex, OpenCode, MCP setup, or provider streaming. The captured trace shows the slow work happening before the agentic session exists.

## Observed Trace

The motivating trace is `trace.md`, captured on **2026-05-09** from:

```sh
RUST_LOG=trace c compose prompts/implement-phase.md \
  plan="features/2026-05-08-expression-syntax/plan.md" \
  -y --claude total_phases=6
```

The trace was taken from `darkmatter/darkmatter`. Wall-clock from first log line to the final compose-prep `discover` close was about **2.64 seconds** (`11:17:21.604` to `11:17:24.241`). The user pressed Ctrl+C at the boundary between prep and provider launch, making the prep delay appear as a hang.

Relevant timing landmarks:

| Time | Event | Notes |
| --- | --- | --- |
| `11:17:21.610` → `11:17:21.656` | `biscuit-file` file reference resolution | Builds ambient resolution context, discovers git root, runs package-area lookup. |
| `11:17:21.659` | `sniff::filesystem::git::discover` close | First eager source-root discovery. |
| `11:17:22.225` | `sniff::filesystem::git::discover` close | Second eager source-root discovery. |
| `11:17:22.922` → `11:17:23.507` | Tokio poller registers two child pipes, then deregisters them | First dynamic model-catalog subprocess, consistent with `opencode models`. |
| `11:17:23.508` → `11:17:24.076` | Tokio poller registers two child pipes, then deregisters them | Second dynamic model-catalog subprocess, consistent with `opencode models` being run again for Qwen filtering. |
| `11:17:24.079` | `sniff::filesystem::git::discover` close | Third eager source-root discovery. |
| `11:17:24.079` → `11:17:24.087` | OS, hardware, repo detection | Fast individually, but still eager work on the hot path. |
| `11:17:24.089` → `11:17:24.121` | ignore-aware repo walk logs | Walks across sibling package areas and ignored build output. |
| `11:17:24.239` → `11:17:24.241` | two `compose_with` runs | First from shell-command discovery, second from final preparation. |

The long wall-clock gaps in this trace are not reflected in `busy` time because they are mostly waiting on subprocesses or tracing spans that stay open while surrounding work proceeds.

## Confirmed Code Path

The current direct compose path in `claudine/cli/src/commands/compose.rs` does this before launching the provider:

1. Parse positionals and resolve the prompt file via `composition::resolve_composition_source`.
2. Parse raw frontmatter selection hints.
3. Discover the source repo root with `sniff::filesystem::git::detect_git`.
4. Call `wrap::composition::eagerly_resolve_target`.
5. Install `AGENT` into the composition environment.
6. Run shell preflight through `composition::resolve_shell_approvals`.
7. Run `composition::prepare_direct` or `composition::prepare_inline`.
8. Enter `execute_composition_request`.

The main confirmed hot spot is inside `eagerly_resolve_target`:

```rust
let catalog = match &selection_config {
    Some(cfg) => ModelCatalogService::with_overrides(cfg.model_overrides.clone()),
    None => ModelCatalogService::new(),
};
catalog.refresh_blocking();
```

`refresh_blocking()` refreshes all supported provider catalogs. Today that includes `Provider::OpenCode` and `Provider::QwenCode`. Both dynamic sources call `opencode models`; Qwen fetches the same OpenCode list and filters it. That means a command explicitly targeting `--claude` still pays for two `opencode models` subprocesses before it can launch Claude.

Secondary confirmed contributors:

- `resolve_composition_source()` uses `biscuit-file::FileReference::resolve()`, which builds an ambient resolution context and may run git and Cargo workspace discovery to support package-area magic paths.
- Compose then independently calls `sniff::filesystem::git::detect_git(parent, false, 1)` for eager source-root detection.
- `prepare_direct()` and `prepare_inline()` run their own simple `find_git_root_from_path()` scan after composition to set `PreparedComposition.source_repo_root`.
- `resolve_shell_approvals()` calls Darkmatter's `collect_shell_commands()`. That collector runs `compose_with()` once for discovery, and final preparation runs `compose_with()` again. The trace confirms two full composition passes. In the captured run they were sub-millisecond, so this is not the dominant cause here, but it is still duplicate work on every command.
- `execute_composition_request_inner()` later calls `events::detect_environment_fast(effective_repo_root)` before provider launch. In this trace OS, hardware, and repo detection were only milliseconds, but they are still part of the unstreamed prep window.

## Root Cause

The immediate root cause is over-eager setup. Compose prep resolves global context for capabilities that may not be needed by the selected provider or by the current prompt.

The worst instance is model catalog refresh:

- It is unconditional.
- It refreshes every supported provider.
- It shells out for dynamic providers.
- It runs before the provider is launched.
- It runs even when an explicit provider flag already determines the provider.
- It duplicates `opencode models` because OpenCode and Qwen share the same underlying source.

The broader design issue is that prep currently has no single invocation-local context object. File-reference resolution, eager selection, final preparation, lifecycle setup, and environment detection each rediscover overlapping facts about the same source path and repo.

## Goals

- Make ordinary `compose` and `inline-compose` prep feel immediate. On the motivating `darkmatter` prompt, the time from command invocation to provider launch should be **under 500 ms** on a warm machine and **under 1 second** on a cold machine.
- Explicit provider runs must not refresh unrelated dynamic model catalogs.
- Dynamic catalog refresh must never block provider launch unless the selected provider and selected model require it for correctness.
- Reuse invocation-local repo/source context instead of rediscovering git roots and source roots across phases.
- Keep shell preflight behavior correct. Security-sensitive shell discovery must not be skipped merely for speed.
- Keep the Ctrl+C behavior from `2026-05-09-loop-ctrl-c-ux`: a user interrupt during prep exits 130 with the clean INFO notice.

## Non-Goals

- Do not rewrite Darkmatter's composition engine.
- Do not remove shell-command preflight.
- Do not change provider selection semantics from the 2026-04-25 agent-selection work.
- Do not optimize provider startup, MCP injection, system prompt delivery, or stream parsing in this feature.
- Do not add long-lived background daemons or external cache services.

## Required Behavior

### Provider and Model Resolution

Provider resolution remains unchanged:

1. Explicit provider flag wins.
2. Raw frontmatter `agent` hint wins when usable.
3. Configured favorite wins in non-TTY mode.
4. TTY mode may show the picker when needed.
5. Non-TTY mode errors when no resolution signal exists.

Model resolution changes only in when catalog refresh happens:

- CLI model values and provider-specific env vars should resolve without refreshing unrelated providers.
- Static providers, including Claude and Codex, should use static catalogs plus existing cache/overrides without dynamic refresh.
- Dynamic refresh should be provider-scoped. If the selected provider is OpenCode, refresh OpenCode only. If the selected provider is Qwen, refresh Qwen only.
- If OpenCode and Qwen both need the same dynamic source in one process, the underlying `opencode models` result should be shared.
- For explicit provider runs with no model hint that requires catalog validation, provider launch must not wait for dynamic catalog refresh.
- Stale cache remains acceptable fallback. A refresh failure must not block selection when current behavior already treats catalog unavailability as skippable.

### Invocation Context

Create or formalize a `CompositionPrepContext` owned by the CLI layer for a single compose invocation. It should hold at least:

- Original file reference.
- Resolved source path.
- Source parent directory.
- Source repo root, if any.
- Ambient CWD.
- Loaded selection config for the effective source repo root or CWD.
- Optional installed-provider snapshot.

This context should be passed through eager target resolution, shell preflight setup, and preparation instead of each phase rediscovering the same source root.

This does not need to be a public library API in the first implementation. It can start as a CLI-private struct if that keeps the change small.

### Shell Preflight

Shell preflight must continue to scan all shell command surfaces:

- Frontmatter shell commands in the source document and transcluded documents.
- Body `::shell` directives and shell block pairs after interpolation/transclusion discovery.
- Harness commands where applicable.

However, this feature should avoid extra composition passes where safe:

- If the preflight discovery pass already produced a composed document/report compatible with final preparation, final preparation may reuse it.
- If reuse is not straightforward, leave the two-pass behavior in place and document why. The trace shows this is not currently the dominant cost.

### Environment Detection

Environment detection should be kept out of the critical pre-launch path where possible:

- The live stream needs an `EnvironmentContext`, but it does not need OS/hardware/repo details before spawning the child unless lifecycle dispatch or rendered output will immediately use them.
- Prefer lazy fields, cached detection, or a minimal context created before spawn and enriched after spawn.
- If this is too invasive for this feature, add perf spans and defer deeper environment work to a follow-up.

## Implementation Plan

### Phase 1: Stop Global Catalog Refresh

- Replace `ModelCatalogService::refresh_blocking()` in eager composition resolution with provider-scoped refresh.
- Do not refresh any dynamic catalog until a provider has been selected.
- For explicit provider runs, select the provider first, then resolve the model against that provider's static/catalog data.
- Add `refresh_provider_blocking(provider)` or an equivalent API.
- Deduplicate OpenCode/Qwen dynamic sourcing inside one refresh operation so `opencode models` runs at most once per process-level refresh.

Expected result: the two `opencode models` subprocess windows disappear from `--claude` runs.

### Phase 2: Share Source and Repo Context

- Build source-root information once after resolving the prompt file.
- Thread that source repo root into eager selection, final preparation, lifecycle setup, and prompt display.
- Replace `prepare_direct()` / `prepare_inline()`'s `find_git_root_from_path()` call with a value provided in `PrepareOptions` or a new prep context.
- Avoid calling both `biscuit-file` workspace discovery and `sniff::filesystem::git::detect_git` where one resolved source path plus one repo-root lookup is enough.

Expected result: repeated `discover` spans and `searching for git root` logs are reduced to the minimum needed by file-reference semantics.

### Phase 3: Measure and Decide on Compose Pass Reuse

- Add or use existing `--perf` spans for:
  - file reference resolution
  - source repo/root discovery
  - selection config load
  - installed client detection
  - model catalog refresh
  - shell preflight discovery
  - final composition preparation
  - environment detection
- Run the motivating prompt with and without shell directives.
- If the two Darkmatter composition passes become material after Phase 1, add a reuse path between `collect_shell_commands()` and final `prepare_*()`.
- If they remain sub-millisecond, leave them alone and capture the reason in the feature notes.

### Phase 4: Environment Detection Follow-Up

- Make environment detection lazy or minimally scoped only if Phase 3 shows it matters after catalog and repo-context fixes.
- Preserve dispatch metadata correctness for lifecycle events and stream events.

## Testing

Unit tests:

- Explicit `--claude` eager resolution does not call dynamic catalog refresh.
- Explicit `--codex` eager resolution does not call dynamic catalog refresh.
- Explicit `--opencode` refreshes only OpenCode.
- Explicit `--qwen` refreshes only Qwen and does not run a second identical OpenCode fetch when OpenCode was already fetched in-process.
- Non-TTY frontmatter agent selection still follows existing resolution precedence.
- Unknown model behavior remains unchanged for providers with available catalogs.
- Catalog refresh failure still falls back to stale/static catalog where current behavior allows fallback.

Integration or CLI-level tests:

- `claudine compose fast.md --claude --dry-run` completes prep without invoking `opencode models`. A test double on `PATH` should fail the test if executed.
- `claudine inline-compose fast.md --claude --dry-run` has the same guarantee.
- A prompt containing `::shell` still triggers preflight approval/discovery.
- Ctrl+C during prep exits 130 and emits the clean user-interrupt notice.

Manual verification:

```sh
RUST_LOG=trace claudine compose prompts/implement-phase.md \
  plan="features/2026-05-08-expression-syntax/plan.md" \
  -y --claude total_phases=6
```

The trace must show:

- No Tokio child-pipe poller windows attributable to `opencode models`.
- No dynamic model catalog refresh for unselected providers.
- At most one source repo-root discovery outside `biscuit-file`'s required file-reference resolution.
- Provider launch or dry-run output reached in under 1 second on the same repo.

## Acceptance Criteria

- The motivating `--claude` run reaches provider launch in **under 1 second** on the same repo, with a target of **under 500 ms**.
- Explicit Claude/Codex compose and inline-compose runs do not execute `opencode models`.
- OpenCode/Qwen model validation still works when those providers are selected.
- Existing provider-selection tests continue to pass.
- Existing shell-preflight tests continue to pass.
- Ctrl+C during prep produces exit code 130 and the same INFO notice as loop interruption.

## Risks

- Skipping eager catalog refresh could hide invalid frontmatter model hints for dynamic providers until later. This is acceptable only if selected dynamic providers still refresh or validate when needed.
- Provider selection UI may rely on model catalogs for display. TTY picker behavior should keep a provider-scoped refresh after selection, or display stale/static data until a provider is chosen.
- Sharing source context across phases risks changing path-resolution semantics. Keep `biscuit-file` as the authority for resolving the original prompt reference.
- Reusing a preflight-composed document may accidentally reuse a version composed without shell expansion or with different approved-command state. Do not implement reuse unless the operation set and context are proven equivalent.

## Open Questions

- Should dynamic catalog refresh be entirely cache-first with an opt-in refresh command, rather than blocking any compose invocation?
- Should `ModelCatalogService` own an in-process shared dynamic-source cache so OpenCode and Qwen dedupe naturally across all callers?
- Should `--perf` become the canonical acceptance artifact for this feature, or is a trace-only comparison sufficient?
- Can Darkmatter expose a preflight result that includes a reusable composed document/report, or should Claudine treat shell discovery and final preparation as separate by design?

## References

- `claudine/features/2026-05-09-slow-prep/trace.md`
- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/wrap/composition/mod.rs`
- `claudine/lib/src/model_catalog/service.rs`
- `claudine/lib/src/model_catalog/provider_sources.rs`
- `claudine/lib/src/composition/preflight.rs`
- `claudine/lib/src/composition/prepare.rs`
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`
- Companion UX fix: `claudine/features/2026-05-09-loop-ctrl-c-ux`
