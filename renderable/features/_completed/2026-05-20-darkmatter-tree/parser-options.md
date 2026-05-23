---
feature: "@renderable/features/2026-05-20-darkmatter-tree"
prompt: |-
    Before we kickoff the Darkmatter implementation of the new [tree-rendering](@renderable/docs/tree-rendering.md)
    architecture we need to decide what `pulldown-cmark` options the tree path is allowed to use relative to the legacy renderers.
    
    This should happen first because it affects parity expectations. If the fold enables task lists, footnotes, superscript, or subscript while
    legacy renderers do not, then “parity failure” may actually be an intentional behavior expansion.
    
    ## Task
    
    Write mini-design spec to the body of this Markdown document that classifies each option as public now, tree experimental only, or deferred. 

    > **Note:** this mini-design complements the existing {{ feature }}/spec.md
last_updated: 2026-05-20
---
# Darkmatter Parser Option Classification

## Context

Darkmatter's legacy renderers and the new render-tree fold do not use the same `pulldown-cmark` option set.  
This document makes the split **deliberate** so that parity tests know whether a divergence is an intentional behavior expansion or a bug.

### Pre-existing legacy inconsistency

| Pipeline                              | Current options                               |
|---------------------------------------|-----------------------------------------------|
| `Markdown::as_html`                   | `ENABLE_STRIKETHROUGH`                        |
| `Markdown::for_terminal`              | `ENABLE_TABLES \| ENABLE_STRIKETHROUGH`        |
| `Markdown::links`, `image_references` | `ENABLE_TABLES \| ENABLE_STRIKETHROUGH`        |
| `cleanup::cleanup_content`            | `all() - SMART_PUNCTUATION - DEFINITION_LIST` |

The tree fold currently uses:

```text
ENABLE_TABLES | ENABLE_STRIKETHROUGH
  | ENABLE_TASKLISTS | ENABLE_FOOTNOTES
  | ENABLE_SUPERSCRIPT | ENABLE_SUBSCRIPT
```

## Classification

| Option                                    | Classification             | Rationale                                                                                                                                                                                                         |
|-------------------------------------------|----------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `ENABLE_TABLES`                           | **public now**             | Required for GFM pipe tables. The terminal renderer already depends on it. The legacy HTML renderer (`as_html`) is the **only** production path that omits it; this is a pre-existing gap the tree path corrects. |
| `ENABLE_STRIKETHROUGH`                    | **public now**             | Used by every legacy pipeline. Required so `~~` pairs are consumed before `InlineStyleProcessor` runs.                                                                                                            |
| `ENABLE_TASKLISTS`                        | **tree experimental only** | The tree model supports `ListItem.checked`, but legacy terminal and HTML renderers do not emit checkboxes. Enabling it publicly would change list rendering.                                                      |
| `ENABLE_FOOTNOTES`                        | **tree experimental only** | The tree model has `FootnoteDefinition` / `FootnoteReference` nodes. Legacy renderers ignore footnote events entirely.                                                                                            |
| `ENABLE_SUPERSCRIPT`                      | **tree experimental only** | Folds to `Span { classes: ["sup"] }`. Legacy parses `^text^` as plain text.                                                                                                                                       |
| `ENABLE_SUBSCRIPT`                        | **tree experimental only** | Folds to `Span { classes: ["sub"] }`. When disabled, pulldown-cmark may interpret `~text~` through strikethrough rules depending on delimiter shape; fixtures must pin exact behavior.                            |
| `ENABLE_GFM`                              | **deferred**               | Currently adds alert-style blockquotes (`> [!NOTE]`). The tree drops the alert kind as `Lossy` today. Enable only when alert rendering is designed.                                                               |
| `ENABLE_MATH`                             | **deferred**               | No `NodeKind` for math. `InlineMath` / `DisplayMath` are mapped to `Unsupported`. Needs dedicated math-node design.                                                                                               |
| `ENABLE_DEFINITION_LIST`                  | **deferred**               | Needs new `NodeKind` variants. Also conflicts with `::file` / `::code` transclusion directives.                                                                                                                   |
| `ENABLE_HEADING_ATTRIBUTES`               | **deferred**               | Would populate `Tag::Heading { id, classes, attrs }` from source. Darkmatter currently auto-generates slugs. Enabling it requires a source-vs-generated slug policy.                                              |
| `ENABLE_SMART_PUNCTUATION`                | **deferred**               | Mutates ASCII quotes/dashes into typographic characters globally. Breaks round-trip expectations.                                                                                                                 |
| `ENABLE_YAML_STYLE_METADATA_BLOCKS`       | **deferred**               | Darkmatter extracts frontmatter before parsing. Enabling this would route `---` through `MetadataBlock` events, conflicting with existing frontmatter handling.                                                   |
| `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS` | **deferred**               | Same conflict as YAML-style metadata blocks.                                                                                                                                                                      |
| `ENABLE_WIKILINKS`                        | **deferred**               | `[[...]]` syntax conflicts with Darkmatter's `{{ }}` interpolation and `::file` transclusion conventions.                                                                                                         |
| `ENABLE_OLD_FOOTNOTES`                    | **deferred**               | Modern `ENABLE_FOOTNOTES` is preferred. Never enable the legacy footnote format.                                                                                                                                  |

## Parity implications

- **Public-now options** (`TABLES`, `STRIKETHROUGH`) are the parity baseline. A tree render that diverges from legacy on these constructs is a **regression**, not an expansion.
- **HTML table behavior is a known pre-existing gap.** Terminal already parses
  tables publicly; HTML currently does not. A tree-backed Browser/HTML path
  that renders tables structurally is the desired public contract, but the
  legacy HTML table gap must be fixed or explicitly recorded before using
  "legacy vs tree" byte output as the only oracle.
- **Tree-experimental options** (`TASKLISTS`, `FOOTNOTES`, `SUPERSCRIPT`, `SUBSCRIPT`) are **intentionally widened** in the fold. Parity tests must:

    1. Expect legacy to ignore or mangle the syntax.
    2. Expect the tree path to produce structural nodes (`ListItem.checked`, `FootnoteDefinition`, `Span { classes: ["sup"/"sub"] }`).
    3. Document the divergence in the parity ledger rather than treating it as a failure.

- **Deferred options** must remain off in both legacy and tree paths until they are re-classified. If a future `pulldown-cmark` release adds a new `Options` flag, it defaults to **deferred**.

## Decision record

1. **Fix the legacy HTML table gap separately.** `as_html` should add `ENABLE_TABLES` so that legacy and tree agree on the public contract. This is a bug-fix, not a tree-path change.
2. **Do not back-port experimental options to legacy.** The cost of adding task-list, footnote, super/subscript rendering to the legacy event-driven serializers exceeds the value; those features will become public automatically when the tree path flips.
3. **Keep the fold's option set explicit.** Do not use `Options::all()` or `ENABLE_GFM` as shorthand. Every enabled flag must be listed individually so that additions are code-reviewed.
4. **Do not enable raw metadata options.** Frontmatter continues to be extracted
   by Darkmatter before parsing and attached above the fold. `MetadataBlock`
   events are not part of the render parser contract.
5. **Revisit `ENABLE_GFM` independently.** It is tempting because "GFM" sounds
   like tables/task lists, but pulldown-cmark exposes tables and task lists as
   separate flags. `ENABLE_GFM` specifically changes blockquote alert
   semantics and needs its own alert design.

## Required fixtures

Add parser-option fixtures before any public tree cutover:

| Fixture | Expected legacy behavior | Expected tree behavior |
|---------|--------------------------|------------------------|
| GFM table | Terminal table; HTML gap until fixed | Structural `Table` |
| `- [x] done` | Ordinary list text or legacy-specific output | `ListItem.checked = Some(true)` |
| Footnote reference and definition | Plain/ignored legacy behavior | `FootnoteReference` / `FootnoteDefinition` |
| `x^2^` | Plain text | `Span` class `sup` |
| `H~2~O` | Legacy delimiter behavior pinned by fixture | `Span` class `sub` |
| `> [!NOTE]` | Plain block quote text unless GFM enabled elsewhere | Deferred; no tree public behavior yet |

## Acceptance criteria

- [x] The fold's parser construction uses only flags listed as **public now** or **tree experimental** above. *(See `render_tree_parser_options()` in `darkmatter/lib/src/markdown/render_tree/fold.rs`.)*
- [x] Parity fixtures for task lists, footnotes, superscript, and subscript exist and explicitly document the legacy-vs-tree divergence. *(See `render_tree_parity_task_list`, `render_tree_parity_footnote_divergence`, `render_tree_parity_superscript_divergence`, and `render_tree_parity_subscript_divergence` in `darkmatter/lib/tests/render_tree_parity.rs`; each pins legacy ignoring/mangling the syntax against the tree path producing structural nodes.)*
- [x] The legacy `as_html` parser options are updated to include `ENABLE_TABLES` (or the pre-existing gap is documented as accepted). *(`as_html` enables `ENABLE_TABLES`; pinned by `render_tree_parity_table`'s structural `<table>`/`<tr>`/`<td>` assertions.)*
- [x] No deferred option is enabled in either legacy or tree code paths. *(The fold lists each enabled flag individually; no `Options::all()` / `ENABLE_GFM` shorthand.)*
