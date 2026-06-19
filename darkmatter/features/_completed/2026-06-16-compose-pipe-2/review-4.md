---
ready: false
agent: codex
model: ""
---

# Review: Compose Pipeline v2 — Iteration 4

## Verdict

Not ready for production.

The iteration fixed the review-3 production-safety issue: root pre-approved validation now includes `ComposeOperation::ShellBlocks`, and there is focused L1 coverage for both local and transcluded shell-block-only failures. The new pre-flight approval lifecycle also gives the CLI a real collect → policy → prompt → membership-gate flow.

I found one new correctness issue in the preflight graph reuse implementation, plus one design-completion gap in the primary lifecycle.

## Findings

### High: preflight graph reuse can apply stale directive spans after normal inline-pre mutations

`collect_shell_commands_with_graph()` captures each `BlockDirective` from a preflight-prepared document and stores the directive span in `PreflightGraphEdge`:

- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:230`
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:232`
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:305`
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:346`

`prepare_block_transclusions_from_graph()` later trusts those cached spans directly when building `PreparedTransclusion::FixedReplace`, `PreparedTransclusion::Markdown`, and `PreparedTransclusion::RemoteFile`:

- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:666`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:681`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:704`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:714`

That is not safe for the normal compose pipeline. Preflight collection deliberately skips frontmatter shell expansion, then final composition runs frontmatter shell expansion, a second frontmatter interpolation pass, text replacement, page blocks, interpolation, shell expansion, and shell blocks before the Transclusion phase:

- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:226`
- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:249`
- `darkmatter/lib/src/markdown/compose/pipeline/operations.rs:162`
- `darkmatter/lib/src/markdown/compose/pipeline/operations.rs:170`
- `darkmatter/lib/src/markdown/compose/pipeline/operations.rs:178`
- `darkmatter/lib/src/markdown/compose/pipeline/operations.rs:186`
- `darkmatter/lib/src/markdown/compose/pipeline/operations.rs:194`

Any of those stages can change byte offsets before a later `::file` / `::url` directive. With `with_preflight_graph(...)`, the transclusion engine no longer reparses the actual transclusion-phase content; it applies replacement ranges from the preflight-prepared content. A document like "frontmatter shell value interpolated above a transclusion directive" is enough to make the cached span point at the wrong bytes, yielding corrupted output or replacing the wrong region.

This is a production blocker because `with_preflight_graph(...)` is a public option and its docs present it as the typical flow:

- `darkmatter/lib/src/markdown/compose/context/options.rs:958`
- `darkmatter/lib/src/markdown/compose/context/options.rs:966`
- `darkmatter/lib/src/markdown/compose/context/options.rs:970`

Verification level: Level 1 is appropriate. Add an in-process regression test that:

- Collects preflight for a document with a directive preceded by content whose length changes only after preflight, e.g. a `{{ doc.value }}` resolved by frontmatter shell expansion or a `::shell` result before `::file`.
- Composes with `with_preflight_graph(preflight.preflight_graph)`.
- Asserts byte-identical output to baseline compose without graph reuse.

Suggested fix: do not cache raw byte spans across stages that can mutate content. Reuse resolved target metadata, but re-anchor edges against the actual transclusion-phase document, for example by reparsing current directives and matching them to graph edges in directive order or by carrying a stable directive identity that can be validated against current content before replacement.

### Medium: the primary pre-flight approval lifecycle still discards the reusable graph

The design says the pre-flight graph should be reused so `compose_with()` can avoid re-collecting after the caller merges and returns the approved set. The new lifecycle computes a full `ComposePreflightReport`, but `ComposePreflightApprovals` returns only the approved command set and stats:

- `darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs:89`
- `darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs:102`
- `darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs:160`

The CLI then calls `compose_preflight_approvals(...)` and passes only `with_pre_approved_commands(...)` into `compose_with(...)`:

- `darkmatter/cli/src/commands/compose.rs:391`
- `darkmatter/cli/src/commands/compose.rs:401`

So the main CLI path still does a preflight collection, then the root pipeline re-collects for `validate_pre_approved(...)`, and final transclusion reparses/resolves its graph unless a library caller manually uses the lower-level `compose_preflight()` API. This leaves the "reuse the collection walk" design goal incomplete on the primary user-facing path.

Verification level: Level 1 is appropriate. Once the span-stability issue above is fixed, add a lifecycle/CLI-path test proving the graph returned by preflight approval is attached to the compose options and used by transclusion. If graph reuse is intentionally library-only for this slice, narrow the tech design and API docs so the production-ready contract no longer claims lifecycle reuse.

## Test Coverage Assessment

- Level 1 present: condition-blind collection for false page blocks and transclusions, shell-block collection, shell-block-only pre-approved validation, dynamic command-shape rejection, execution subset checks, pre-flight policy lifecycle, CLI `--shell` discovery, cache defaulting, no-cache syntaxes, and basic preflight-graph transclusion parity.
- Level 1 gap: graph reuse is not tested against content whose byte offsets change between preflight collection and the Transclusion phase.
- Level 1 gap: the primary approval lifecycle / CLI path is not tested for carrying the preflight graph into final compose.

No Level 2 or Level 3 tests are required for these findings. These requirements are compose orchestration and document-transformation semantics, not terminal rendering or OS keyboard behavior.

## Verification Run

I ran:

```text
cargo test --color=never -p darkmatter markdown::compose::preflight --lib
```

Result: passed, 62 tests.
