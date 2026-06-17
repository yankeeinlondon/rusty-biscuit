---
ready: false
agent: codex
model: ""
---

# Review: Compose Pipeline v2

## Verdict

Not ready for production.

The branch has a good start on the condition-blind collector and the focused L1 unit tests for that collector pass, but there are production-blocking gaps in execution semantics and in the pipeline integration promised by the design.

## Findings

### High: shell pipeline cache key collapses different chain operators

The per-compose shell cache keys a directive with:

- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:397`
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:447`

`cache_key()` joins the normalized actions with `" && "` regardless of the original chain operators. That makes these two directives share one cache entry:

```md
::shell false || echo fallback
::shell false && echo fallback
```

I reproduced this through the CLI with `false` and `echo fallback` whitelisted. The cached run printed:

```text
fallback
fallback
```

and exited `0`. With `--no-cache`, the second directive correctly failed:

```text
Error: Shell command failed (exit 1) at line 2: 'false && echo fallback'
```

This violates the cache requirement in the tech design: only identical commands should execute once per compose. These are not identical commands because `&&` and `||` have different execution semantics.

Verification level: needs L1 coverage. Add a unit/integration test that composes the two directives above and asserts the second one is not served from the first directive's cache entry. No L2/L3 coverage is needed because this is pure compose semantics, not terminal rendering or real keyboard behavior.

Suggested fix: key pipelines by the full normalized pipeline shape, including chain operators and redirection configuration. `ShellPipeline::display_string()` may be closer to the correct shape than joining per-action normalized commands, but the key should use a canonical representation rather than display text if quoting can differ.

### High: `compose_with()` does not run the designed up-front pre-flight stage

The design requires pre-flight to run after schema validation and before any shell execution:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:107`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:117`

The pipeline currently goes straight from schema validation into frontmatter shell expansion:

- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:199`
- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:210`

There is a `Markdown::compose_preflight()` API, and Claudine uses it before execution, but ordinary `Markdown::compose_with()` and `md compose` still rely on `prepare_directive()` approvals during execution unless the caller manually supplies `with_pre_approved_commands(...)`. That means the library path can still execute an earlier frontmatter shell command before discovering that a later body command needs approval, which is exactly the failure mode the v2 design is meant to remove.

Verification level: needs L1 coverage. Add a test with a frontmatter shell command plus a later unapproved body command and assert the compose fails before the frontmatter command executes. Also cover that the execution stage uses membership checks against the preflight approval set when `pre_approved_commands` is supplied.

Suggested fix: integrate pre-flight into `run_compose_pipeline_internal()` or add the internal execution mode described by the design so `compose_with()` can use a previously collected report after the caller merges approvals. If the intended contract is "Darkmatter only collects; callers must approve," the design and public docs need to be narrowed, because they currently describe a pipeline stage.

### High: pre-flight collection ignores `::url` remote transclusion children

The design says collection walks the transclusion graph and reuses metadata for resolved child paths/URLs:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:121`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:123`

The collector only recurses body directives whose kind is `DirectiveKind::File`:

- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:253`
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:254`

For frontmatter prologue/epilogue refs it also resolves URLs but then only accepts `ResolvedTarget::File`, silently skipping URL targets:

- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:302`
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:308`

Remote `::url` transclusion is a supported compose surface elsewhere in the transclusion engine. A remote Markdown child containing `::shell` can therefore be missed by pre-flight and then either fail late as `NotPreApproved` or prompt/execute outside the single up-front approval contract.

Verification level: needs L1 coverage with an in-process HTTP fixture, not L2/L3. Add a remote Markdown child containing a shell directive, enable remote transclusion for the host, and assert `compose_preflight()` includes that child command even when the transclusion has a false `when=`.

Suggested fix: make the collector share the same resolved-target handling as `TransclusionEngine`, including `ResolvedTarget::Url` when remote transclusion is enabled and fetch policy permits it. If remote child command discovery is intentionally unsupported, it must be rejected explicitly during pre-flight rather than skipped.

### Medium: graph metadata reuse from pre-flight is not implemented

The design explicitly calls for collection to cache reusable graph metadata so the final transclusion stage does not rediscover the graph:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:121`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:123`

`ComposePreflightReport` currently carries only entries and warnings:

- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:84`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:89`

The collector recursively performs its own inline compose and path resolution, and the later transclusion phase resolves again. This preserves behavior for many local cases, but it does not meet the "remove duplicated discovery compose" goal and leaves room for collector/executor graph drift.

Verification level: L1 is sufficient. Add an assertion around collector/transclusion reuse once metadata exists, or narrow the design if graph reuse is deferred.

## Test Coverage Assessment

The current L1 coverage is strong for local condition-blind collection, dynamic-shape detection, dead page-block execution, cache default, opt-out syntax, and volatile warnings.

Gaps that must be covered before production:

- L1: pipeline cache keys preserve chain operators and redirections.
- L1: `compose_with()` or the documented public execution path performs pre-flight before any shell command executes.
- L1: remote `::url` transclusion children participate in condition-blind pre-flight collection, or fail explicitly.
- L1/property: the stated randomized `execution_set ⊆ approval_set` invariant is only represented by fixed examples, not a property test.

No Level 2 or Level 3 testing is required for these specific requirements because they are not terminal-rendering or OS-keyboard behaviors.

## Verification Run

I ran:

```text
cargo test --color=never -p darkmatter markdown::compose::preflight::collect --lib
cargo test --color=never -p darkmatter markdown::compose::shell_expansion::integration_tests --lib
```

Both passed. I also reproduced the pipeline cache collision with the CLI using a temporary document and whitelist.
