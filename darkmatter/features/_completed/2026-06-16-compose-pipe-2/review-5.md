---
ready: false
agent: codex
model: ""
---

# Review: Compose Pipeline v2 — Iteration 5

## Verdict

Not ready for production.

Iteration 5 fixes the review-4 correctness blocker. Preflight graph reuse no longer trusts byte spans captured before offset-shifting inline-pre stages; the transclusion phase reparses current content and uses the graph only as a target-resolution cache. The approval lifecycle also returns the graph and the CLI-shaped flow attaches it to `compose_with(...)`.

I found one remaining design-completion gap in graph reuse. It is not a shell-safety issue, but it leaves an explicit v2 performance/architecture requirement only partially implemented and documented more strongly than the code supports.

## Findings

### Medium: preflight graph reuse stops at the root document

The design says the pre-flight walk should cache the discovered child set and resolved graph metadata so graph composition can reuse it and the graph is traversed for structure once:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:121`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:123`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:258`

The data model also advertises recursive reuse: `PreflightGraphEdge::child` says reuse recurses through `child.edges`, and `PreflightGraphNode` says the final transclusion stage should not have to re-parse directives and re-resolve targets for the full tree:

- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:57`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:76`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:78`

The implementation only builds a resolution cache from the currently attached node's outgoing edges:

- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:34`
- `darkmatter/lib/src/markdown/compose/pipeline/phases.rs:211`

When a local Markdown child is recursively composed, the child pipeline explicitly drops the graph instead of passing the matching child node:

- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:1340`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:1345`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:1346`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:1351`

That means a root document gets root-level `::file` / `::url` target-resolution reuse, but any grandchild and deeper transclusion still reparses and re-resolves from scratch. Frontmatter prologue/epilogue children are also collected for approval but are not represented as reusable edges for final transclusion reuse.

This is not a correctness regression; the baseline and graph-seeded outputs match. It does leave the "graph traversed for structure once" requirement incomplete, and the public docs/comments overstate the current behavior. Either thread the appropriate `edge.child` graph node into each child compose invocation, with focused L1 tests for a root → child → grandchild document, or narrow the design/API docs to state that v2 reuses only root-level body-transclusion target resolution.

Verification level: Level 1 is appropriate. This is compose orchestration and target-resolution reuse, not terminal rendering, terminal input encoding, or keyboard UX.

## Test Coverage Assessment

- Level 1 present: condition-blind approval collection, false page blocks and transclusions, remote transclusion discovery, shell-block discovery, `DynamicCommandShape`, execution subset checks, loop stability, cache default and all opt-out syntaxes, volatile warnings, pre-approved validation before shell execution, approval lifecycle, CLI-shaped graph handoff, and stale-span regression coverage.
- Level 1 gap: no test proves recursive preflight graph reuse for root → child → grandchild transclusions, because recursive child composition currently clears the graph.
- Level 2/Level 3: not required for this feature slice. The reviewed behavior is document transformation and shell-approval orchestration, not real terminal rendering or OS keyboard input.

## Verification Run

I ran:

```text
cargo test --color=never -p darkmatter markdown::compose::preflight --lib
```

Result: passed, 64 tests.
