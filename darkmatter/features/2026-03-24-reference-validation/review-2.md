# Reference Validation Review 2

The implementation is in much better shape than the initial review: the recursive graph builder now works for normal trees, the InlinePre gate was removed, the Phase 2 graph APIs were added, transclusion records are emitted, child-relative validation is fixed, and both library integration tests and CLI `validate refs` tests now exist and pass under `just test`.

The remaining recommendations are narrower, but they are still worth addressing before calling the feature fully complete.

## Findings

### 1. [High] Root-cycle handling still admits duplicate nodes for the same file

Relevant code:

- `darkmatter/lib/src/markdown/reference/graph.rs:42-48`
- `darkmatter/lib/src/markdown/reference/graph.rs:61-67`
- `darkmatter/lib/src/markdown/reference/graph.rs:171`
- `darkmatter/lib/src/markdown/reference/graph.rs:216-221`
- `darkmatter/lib/src/markdown/reference/graph.rs:437-456`
- `darkmatter/lib/src/markdown/reference/graph.rs:463-467`

`build_reference_graph()` starts traversal without first entering the root node into the `TransclusionRuntime` stack. That means a child document can legally transclude the root once before cycle detection kicks in. On top of that, node identity is derived from the raw `ComposeSource` string, while recursive child resolution may canonicalize paths.

In practice this means the same file can appear twice in the graph under different spellings. I reproduced this with a two-node cycle on macOS temp files:

```text
a.md -> ::file b.md
b.md -> ::file a.md
```

`md validate refs a.md --graph mermaid` emitted `a.md` twice, once via `/var/...` and once via `/private/var/...`, with a back-edge from `b.md` to the duplicate root node.

Impact:

- graph output is incorrect for cycles involving the root
- the graph can contain duplicate nodes for the same physical file whenever path spellings differ
- flattened/composed output can double-count references or become traversal-order-dependent

Recommendation:

- canonicalize or otherwise normalize the root source before assigning `node_id`
- seed the runtime stack with the root node before descending into children
- add integration tests that assert:
  - a two-node cycle produces exactly two unique nodes
  - symlinked or aliased paths do not duplicate graph nodes
  - `composed_references()` on a cycle stays finite and non-duplicative

### 2. [High] Fragment validation still checks raw target headings instead of prepared/composed headings

Relevant code:

- `darkmatter/lib/src/markdown/reference/validate.rs:152-158`
- `darkmatter/lib/src/markdown/reference/validate.rs:364-388`
- `darkmatter/lib/src/markdown/reference/validate.rs:432-444`

The validator now resolves relative paths from `record.origin.source`, which is good, but fragment validation still builds heading sets from raw `Markdown::toc()` output:

- `collect_composed_heading_slugs()` reloads child docs with `Markdown::try_from(...)` and extracts raw headings
- `validate_cross_doc_fragment()` loads the target document and calls `collect_heading_slugs(&target_md)` directly

That means fragment validation still disagrees with reference extraction whenever headings are introduced or changed by InlinePre preparation.

Concrete repro:

```md
<!-- root.md -->
[go](./target.md#visible)
```

```md
<!-- target.md -->
---
title: Visible
---

# {{ title }}
```

`md validate refs root.md --fragments` currently reports `Fragment '#visible' not found in ./target.md`, even though the prepared target heading is `# Visible`.

Impact:

- false negatives for interpolation-driven headings
- the same problem still exists for page blocks and shell expansion if they affect heading text/presence
- same-document fragment validation is also only “composed” at the raw-node level, not at the prepared-content level

Recommendation:

- validate fragments against the same prepared node content used by reference extraction
- ideally reuse the analysis graph or a shared cached heading loader rather than reparsing raw markdown
- add integration tests for:
  - cross-document fragments where the target heading comes from interpolation
  - same-document fragments where the heading is introduced/removed by page blocks
  - shell-expanded headings, if that remains supported for reference analysis

### 3. [Medium] The feature still does not fully leverage the recent caching work

Relevant code:

- `darkmatter/lib/src/markdown/reference/graph.rs:42-45`
- `darkmatter/lib/src/markdown/reference/graph.rs:61-64`
- `darkmatter/lib/src/markdown/reference/graph.rs:412-430`
- `darkmatter/lib/src/markdown/reference/validate.rs:380-385`
- `darkmatter/lib/src/markdown/reference/validate.rs:432-444`

This branch now shares a run-local `RunLocalCache` across graph traversal, which is a real improvement, but the cache integration is still incomplete relative to the compose cache architecture:

- the reference-analysis runtime is created with `RunLocalCache::new(options.compose.cache_access_mode)` and never attaches `cache_root` or `cache_namespace`
- `prepare_content()` preserves compose cache options for each per-node `compose_with(...)` call, but direct child loads in graph traversal are run-local only
- heading extraction in validation bypasses cache helpers entirely and reloads markdown directly

So the answer to “does reference validate adequately leverage the new caching work?” is still “not fully.”

Impact:

- no persistent cache reuse for child document loads inside the analysis graph
- no cached TOC/heading reuse during fragment validation
- caller-supplied cache root/namespace/freshness only help indirectly via `compose_with`, not through the rest of the analysis path

Recommendation:

- construct the reference-analysis runtime with the same persistent cache root resolution used by compose
- route heading extraction through cached helpers instead of `Markdown::try_from(...)`
- add tests that exercise `ComposeOptions::with_cache_root(...)` and confirm repeated analysis/fragment validation hits the shared cache path

### 4. [Medium] One public spec item is still missing: `set_meta_tag`

Relevant code:

- `darkmatter/lib/src/markdown/reference/mod.rs:380-401`

The graph-aware Phase 2 APIs that were missing in the first review are now present, but the meta-tag surface is still short one method from the spec/design:

- `set_meta_tag(key, value)` is still not implemented on `Markdown`

If this feature is meant to satisfy the documented contract, that method still needs to exist, or the spec/design/plan should be narrowed to match the actual delivered API.

Recommendation:

- implement `set_meta_tag(...)` plus unit tests for insert, update, duplicate-key handling, and charset/property/name cases
- or explicitly descoped it in the feature docs

### 5. [Low] Two public options are still advertised but inert

Relevant code:

- `darkmatter/lib/src/markdown/reference/types.rs:383-400`

`ReferenceGraphOptions` still exposes:

- `include_generated_toc_links`
- `follow_remote_transclusions`

Both are still documented as “not yet implemented”, and I did not find any wiring for either in the graph builder or validator.

That is acceptable if they are intentionally deferred, but it is still public API surface that callers can set without effect.

Recommendation:

- either implement them end-to-end now
- or remove/hide them until the behavior exists

## Remaining Test Gaps

The broad “needs more integration coverage” conclusion from the first review is no longer true. The remaining test gaps are narrower:

- cycle tests should assert unique-node identity and safe flattening, not just “does not hang”
- fragment-validation tests should cover prepared/composed headings, not just raw headings in child docs
- cache tests should prove persistent cache reuse for reference analysis and fragment heading lookup

## Overall

Most of the original review items are closed. The remaining work is concentrated in three areas:

1. cycle identity/canonicalization
2. prepared/composed heading validation
3. fuller cache integration

Those are the main blockers I still see before marking `reference validate` as fully finished against the current spec and tech design.
