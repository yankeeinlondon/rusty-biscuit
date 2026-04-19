# Policy Engine Implementation Review

Reviewed against:

- `claudine/features/2026-03-30-permissions-engine/spec.md`
- `claudine/features/2026-03-30-permissions-engine/policy-engine-design.md`

## Findings

### P1: Trust-gated repo config is still applied when trust is unknown

Files:

- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/providers/codex.rs`
- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/providers/gemini.rs`
- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/query.rs`

The design explicitly says unknown trust must not silently activate project-local policy. In both Codex and Gemini, `discover_sources` includes repo-scoped config whenever `ctx.trust.is_trusted != Some(false)`, which means repo config is loaded even when trust is `None`.

That is a spec/design mismatch. The query layer then returns ordinary allow/ask/deny answers from those layers, while `QueryResult` drops the trust warning entirely. The result is false confidence: callers can get a definitive answer from policy that may not actually be active.

Recommended fix:

- Do not load trust-gated repo sources when trust is unknown.
- If you intentionally keep them in the snapshot for explanation purposes, query results must surface that ambiguity through `effect: None` or at least `certainty/stability/warnings`.
- Add explicit tests for `is_trusted: None` on Codex and Gemini.

### P1: `PolicyChangeTarget::LocalOverride` for Claude writes to the wrong file

Files:

- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/providers/claude.rs`

Source discovery models `.claude/settings.local.json` as a distinct writable `LocalOverride` layer. Mutation targeting does not preserve that distinction: `PolicyChangeTarget::LocalOverride` is routed to `.claude/settings.json` instead of `.claude/settings.local.json`.

That means local-only permission edits will leak into shared repo settings.

Recommended fix:

- Map `PolicyChangeTarget::LocalOverride` to `.claude/settings.local.json`.
- Add a mutation-planning test that asserts the target path for `LocalOverride`.
- Add a round-trip test proving the edit lands in local settings and changes the reloaded query result.

### P2: Relative path queries are not canonicalized against `cwd`

Files:

- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/query.rs`
- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/matchers.rs`

The design says path queries should canonicalize relative paths against `cwd` where possible and classify them by scope. That does not happen today. Snapshot helpers forward the caller path directly into raw string matching, so provider rules written as absolute paths can miss relative queries like `src/main.rs` or equivalent normalized paths.

This makes the query API less ergonomic than designed and can produce incorrect answers depending on how the caller formats the same path.

Recommended fix:

- Normalize query paths relative to `PolicyContext.cwd` before matching.
- Reuse the existing path classification helpers so explanations can mention workspace/home/system/provider-config scope.
- Add tests covering relative paths, `.`-relative paths, and normalized equivalents like `src/../src/main.rs`.

### P2: Query stability and warnings are effectively unimplemented

Files:

- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/query.rs`

The design makes `QueryStability`, warnings, and explainability first-class. In practice:

- matched-rule results are always marked `Stable`
- `no_match()` always returns `Unknown`
- `warnings` are always empty
- snapshot-level policy warnings are not propagated into query results

That means callers never learn that a result is CLI-sensitive, trust-sensitive, or runtime-sensitive, even though the engine tracks some of that ambiguity during canonicalization.

Recommended fix:

- Thread policy warnings into `QueryResult`.
- Mark configured snapshots as `MayChangeWithCli` where provider semantics depend on runtime flags.
- Mark trust-dependent answers as `Unknown` or at least `BestEffort` with warnings.
- Add explanation snapshot tests, as called for in the design.

### P2: Codex MCP support is still missing

Files:

- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/providers/codex.rs`

The design explicitly calls out Codex permission truth as spanning sandbox mode, approval policy, execution rules, named permission profiles, trust state, and MCP controls. The current backend still reports `mcp_queries: false` and does not canonicalize any MCP axis.

This is a real functionality gap relative to the design targets.

Recommended fix:

- Add native parsing and canonicalization for Codex MCP controls.
- Surface any ambiguity with fidelity/warnings if full fidelity is not possible yet.
- Add query and mutation-planning tests for the MCP axis once implemented.

### P2: Test coverage is materially below the design target

Files:

- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/permissions/`

There are useful unit tests and the module is not untested, but coverage is still thin compared to the design’s expected layers. Missing or notably weak areas include:

- trust-unknown behavior for trust-gated providers
- relative-path query behavior
- round-trip mutation tests
- explanation snapshot tests
- broader precedence/interaction tests for configured vs effective policy
- more negative-path tests for unsupported mutations and degraded fidelity

Recommended additions:

1. Backend fixture tests for representative real config layouts.
2. Canonicalization tests that assert fidelity and warnings, not just effect.
3. Query contract tests for trust ambiguity and relative paths.
4. Mutation round-trip tests: load, plan, apply, reload, verify changed query outcome.
5. Explanation snapshot tests so trust/explanation regressions are visible.

## Ergonomics and Performance Opportunities

### Normalize paths once per query surface

The snapshot API would be more ergonomic if callers could pass relative paths safely and get the same answer as absolute normalized paths. Carrying a normalized resolver or context into the snapshot would also make explanations better.

### Precompile matcher state during canonicalization

The current query layer reparses plain string patterns on every lookup. If this subsystem becomes a hot path, precompiling path/command/domain matcher state when building the canonical policy will reduce per-query work and make precedence handling easier to evolve.

### Add lightweight snapshot caching

The design does not require caching in v1, but the current engine re-discovers sources and reparses files on every call. A small in-memory cache keyed by provider plus source mtimes or content hashes would improve repeated queries without complicating the public API.

## Verification

I ran:

```bash
cargo test -p claudine
```

Result:

- passed: 861 unit tests
- passed: 2 doctests

Note:

- `claudine/just test` did not succeed in this environment because it hit a workspace manifest/toolchain resolution problem before the package tests ran. Direct `cargo test -p claudine` completed successfully and was used for verification instead.
