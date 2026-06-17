---
ready: true
agent: codex
model: ""
---

# Review: Compose Pipeline v2 — Iteration 7

## Verdict

Ready for production.

Iteration 7 closes the remaining iteration-6 gap for recursive remote
preflight-graph reuse. The implementation now has a URL lookup on
`PreflightGraphNode` and threads the matching remote child subgraph into the
recursive remote compose path, mirroring the local-file path:

- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:144`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:153`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:971`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:977`

This satisfies the design requirement that the pre-flight graph carry resolved
child paths/URLs and nested graph metadata so final transclusion can reuse the
walked structure instead of rediscovering it:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:121`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:123`

## Findings

No blocking findings.

## Non-Blocking Notes

The new remote recursion regression test proves the recursive URL graph shape
and verifies graph-seeded output matches the no-graph baseline:

- `darkmatter/lib/src/markdown/compose/tests.rs:3434`
- `darkmatter/lib/src/markdown/compose/tests.rs:3483`
- `darkmatter/lib/src/markdown/compose/tests.rs:3515`

That is enough for production readiness because the implementation path itself
is straightforward and covered by the targeted code inspection above. A future
test hardening pass could make the regression sharper by adding a direct unit
test for `child_for_url(...)` or by using a stale/unresolvable grandchild target
where success depends on the child subgraph being attached. That would make the
test fail specifically if `engine.rs:977-980` regresses, instead of relying on
output parity.

## Test Coverage Assessment

- Level 1 present: condition-blind approval collection across frontmatter, page
  blocks, shell blocks, local and remote transclusions; condition-aware
  execution; `execution_set ⊆ approval_set`; loop stability; pre-execution
  membership validation; cache default; body/frontmatter/shell-block no-cache
  parsing and execution; volatile warnings; dynamic command-shape rejection;
  approval lifecycle; CLI `--shell` listing; stale-span graph reuse; local
  recursive graph reuse; remote recursive graph shape and graph-seeded compose
  parity.
- Level 2/Level 3: not required for this feature. The reviewed behavior is
  document transformation, shell-approval orchestration, command execution
  caching, and transclusion graph reuse. It does not assert real-terminal
  rendering, terminal input encoding, modifier-key visibility, or OS keyboard
  behavior.

## Verification Run

I ran:

```text
cargo test --color=never -p darkmatter preflight_graph_reuse_recurses_to_remote_grandchild --lib
cargo test --color=never -p darkmatter markdown::compose::preflight --lib
cargo test --color=never -p darkmatter markdown::compose::shell_expansion --lib
cargo test --color=never -p darkmatter-cli test_compose_shell_reports_discovered_commands_without_executing --test cli
```

Results:

- `darkmatter` remote recursive graph test: passed, 1 test.
- `darkmatter` preflight unit slice: passed, 66 tests.
- `darkmatter` shell-expansion unit slice: passed, 257 tests; 1 ignored alias
  test.
- `darkmatter-cli` targeted CLI integration test: passed, 1 test.
