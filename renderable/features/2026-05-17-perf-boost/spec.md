# Spec: Render Tree Performance Boost

**Date:** 2024-04-17
**Status:** Draft
**Crates:** `renderable`, `biscuit-terminal`

## Problem

The new render-tree path is currently slower than the bespoke terminal
renderers for components that have both implementations. A quick Criterion run
against `biscuit-terminal`'s `render_tree` bench showed the tree path slower
across representative components:

| Component                     | Bespoke  | Tree     | Notes                                                              |
|-------------------------------|----------|----------|--------------------------------------------------------------------|
| `Progress`                    | ~0.56 us | ~3.33 us | Small component; fixed overhead dominates.                         |
| `UnorderedList` with 80 items | ~24.7 us | ~131 us  | Inline rendering and list joins dominate.                          |
| `OrderedList` with 80 items   | ~27.1 us | ~131 us  | Same shape as unordered list.                                      |
| `Section` with 60 items       | ~7.34 us | ~88.9 us | Plain text still goes through inline markup and `Prose`.           |
| `TwoColumn`                   | ~1.60 us | ~5.21 us | Column rendering does extra splitting and joining.                 |
| `Table` 80x3                  | ~198 us  | ~387 us  | Closest case because bespoke table rendering is already expensive. |

These numbers are directional only. They used a short local Criterion run:

```sh
cargo bench -p biscuit-terminal --bench render_tree -- component_render_path_comparison --sample-size 10 --warm-up-time 0.1 --measurement-time 0.3
```

The goal is not to beat bespoke rendering immediately. The goal is to remove
obvious overhead so the tree path is cheap enough to keep using while the
architecture settles.

## Goal

Identify quick, low-risk performance wins in the render-tree implementation
that preserve output semantics and reduce overhead for common terminal
components.

Success criteria:

- Preserve existing parity assertions and snapshots.
- Improve the benchmark group `component_render_path_comparison`.
- Avoid broad rewrites of the render tree model.
- Avoid changing public component APIs unless a performance win clearly needs
    an opt-in option.

## Non-Goals

- Comprehensive profiling across every render target.
- Replacing `Prose`, `Table`, or existing terminal components.
- Removing validation entirely from public render entry points.
- Changing browser or Markdown tree renderer semantics.
- Solving allocation behavior in component projection broadly.

## Baseline Cost Centers

### Validation on every render

`render_terminal_node` calls `validate(node, ValidationMode::Full)` before every
render. This is correct for public safety, but it means benchmarked component
tree rendering includes:

1. component projection into `RenderNode`,
2. full validation walk,
3. terminal folding.

For small components like `Progress`, the validation and setup cost dominates
the actual render.

Opportunity:

- Add an internal or explicitly named fast path for already-trusted trees, for
    example `render_terminal_node_unchecked` or an option on
    `TerminalRenderOptions`.

- Keep the checked path as the default public API.
- Use the unchecked path only in callers that just produced the tree from a
    known-good component projection and are already covered by parity tests.

Risk:

- Skipping validation can hide malformed trees. The API name must make the
    trade-off explicit, and tests should keep exercising the checked path.

Expected impact:

- High for small components.
- Moderate for large components.

### Plain text goes through `Prose`

The terminal tree renderer renders paragraph and inline content by producing
`Prose` block-tag markup, then parsing/rendering that markup through
`Prose::new(...).render(...)`.

That is useful for styled spans, links, and rich inline content, but it is
overhead for the common case:

```text
Paragraph([Text("plain text only")])
```

Current hot paths affected:

- `Writer::render_inline`
- `Writer::render_inline_node`
- `Writer::render_prose`
- `Writer::render_list_text`
- `Writer::render_heading_line`
- `render_progress_bar`

Opportunity:

- Detect plain inline runs before building `Prose` markup.
- For `Paragraph` containing only text and no classes, return the escaped or
    raw text directly.

- For list items with plain paragraph text, wrap with the existing terminal
    word-wrap helper directly instead of constructing `Prose`.

- For `Progress`, avoid rendering the generated progress bar through `Prose`
    when the generated bar has no styling tags.

Risk:

- Plain text must still be escaped correctly when it can contain block-tag-like
    sequences.

- Links and styled spans must continue through `Prose`.

Expected impact:

- High for `Section`.
- High for ordered and unordered list cases with string items.
- High for `Progress`.

### String joins allocate intermediate vectors

Several renderer helpers collect strings into vectors only to join them:

- `Writer::render_blocks` builds `Vec<String>` then `join("\n\n")`.
- `Writer::render_list` builds `Vec<String>` then `join("\n")`.
- `indent_block` maps every line into a `Vec<String>` then `join("\n")`.
- `Writer::render_code` maps lines into a `Vec<String>` then `join("\n")`.
- `Writer::render_columns` builds `Vec<String>` rows then `join("\n")`.

Opportunity:

- Stream into a single `String` with `push_str`.
- Use first-item checks to insert separators without building a `Vec`.
- Pre-size conservatively when input lengths are available.

Risk:

- Separator behavior is easy to change accidentally, especially trailing
    newlines and blank lines. Existing parity tests should catch this.

Expected impact:

- Moderate for lists, sections, code blocks, and column layouts.
- Low risk and easy to verify.

### Inline class wrapping repeatedly reallocates

`apply_classes` starts with `inner.to_string()` and repeatedly wraps the whole
string with `format!` for each class.

Opportunity:

- Fast-return borrowed/plain output when `classes.is_empty()`.
- For known class sets, build prefixes and suffixes once into a single string.
- Consider returning `Cow<'_, str>` from helpers that often do not modify text.

Risk:

- Low. Unknown classes are ignored today and should remain ignored.

Expected impact:

- Low for plain text once plain-text fast paths exist.
- Moderate for heavily styled inline content.

### `render_progress_bar` reconstructs label text

`Progress::render_tree_node` stores visible text such as `"Indexing 72%"`, and
`render_progress_bar` then infers the label by stripping a percentage suffix
from that text.

Opportunity:

- Extend `ProgressHints` with an optional `label`.
- Render the progress bar directly from hints instead of parsing rendered
    paragraph text.

- If changing hints is too broad, add a terminal-only helper that avoids
    calling `render_inline` first for progress paragraphs.

Risk:

- Hint shape changes affect serialized render trees and other render targets.
    This is not as small as renderer-only changes.

Expected impact:

- Moderate for `Progress`.
- Also simplifies code.

### Table rendering reconstructs component state

`Writer::render_table` reconstructs `TableColumn` and `TableCellContent` from
tree hints, clones columns and data into a temporary `Table`, runs width
planning, then emits the table itself.

Current overhead includes:

- collecting data rows,
- reconstructing typed cells,
- cloning `columns` and `data` into `Table::with_columns` and
    `Table::with_data`,

- converting cells to strings again during emit.

Opportunity:

- Add a width-planning helper that accepts borrowed columns and data, avoiding
    the temporary `Table` clone.

- Cache or carry display strings during typed-cell reconstruction so emit does
    not call `ToString` again for every visible cell.

- Avoid collecting `data_rows: Vec<&RenderNode>` if the rows can be traversed
    once for first-row kinds and data reconstruction.

Risk:

- Medium. Table behavior has the most layout nuance, including conditional
    columns, drop notes, vertical alignment, and striping.

- The safest step is to remove clones around width planning without changing
    layout behavior.

Expected impact:

- Moderate for table-heavy output.
- Table is already closest to bespoke, so quick wins here help but are less
    urgent than plain-inline overhead.

### Repeated context cloning for narrowed renders

`render_with_layout` and `render_blocks_in_width` clone
`TerminalRenderOptions`, mutate the context, and instantiate a nested `Writer`.

Opportunity:

- Add a small helper on `TerminalRenderOptions` or `TerminalRenderContext` to
    create a narrowed context with less incidental work.

- Alternatively make `Writer` hold context state separately from immutable
    options, so narrowing can push/pop width state without cloning all options.

Risk:

- Medium. Width and terminal state are subtle. A push/pop context stack must be
    exception-safe across early returns.

Expected impact:

- Low to moderate, mostly in columns and layout-heavy trees.

### Validation report allocates even when there are no findings

`validate` returns a `ValidationReport` with a `Vec<ValidationFinding>`. The
common case is no findings, but the renderer still builds a full report and
then scans it for errors and warnings.

Opportunity:

- Use `ValidationMode::FailFast` for strict error rejection when diagnostics
    are not needed.

- Add a lightweight `is_valid(node) -> bool` or `validate_errors_only` path.
- In `RenderStrictness::Lossy`, avoid collecting warning findings that will be
    discarded.

Risk:

- Warning diagnostics under `RenderStrictness::Warn` are part of observable
    behavior. The warn path still needs warning collection.

Expected impact:

- High for small valid trees.
- Moderate for large valid trees.

## Prioritized Quick Wins

1. **Stream joins into `String`.**
   Replace short-lived `Vec<String>` plus `join` patterns in terminal tree
   rendering. This is the safest first change and should not affect APIs.

2. **Add plain-inline fast paths.**
   Detect all-text inline children and bypass `Prose` where no styling, links,
   classes, or hard breaks require it.

3. **Add a trusted render fast path.**
   Keep `render_terminal_node` checked, but add an explicitly named unchecked
   path or option for component-produced trees after tests establish projection
   validity.

4. **Optimize list item rendering.**
   Lists are far slower than bespoke in the quick bench. After plain-inline
   fast paths, render string-only list items with direct wrapping and streaming
   output.

5. **Remove table clones around width planning.**
   Refactor table width planning to accept borrowed inputs or expose a
   planner helper, then keep emission unchanged.

6. **Reduce validation allocation.**
   Add an errors-only validation mode for render paths that do not need
   warnings, preserving the current `Warn` behavior.

## Benchmarking

Use the component comparison benchmark as the guardrail:

```sh
cargo bench -p biscuit-terminal --bench render_tree -- component_render_path_comparison
```

For quick iteration:

```sh
cargo bench -p biscuit-terminal --bench render_tree -- component_render_path_comparison --sample-size 10 --warm-up-time 0.1 --measurement-time 0.3
```

Suggested success checks:

- `Progress` tree path moves closer to bespoke after validation/plain-output
    work.

- `Section` and list cases improve after plain-inline and streaming changes.
- `Table` improves after clone removal, without changing table parity tests.

## Verification

Before accepting any implementation from this spec:

```sh
cargo test -p biscuit-terminal --test progress_parity
cargo test -p biscuit-terminal --test list_parity
cargo test -p biscuit-terminal --test section_parity
cargo test -p biscuit-terminal --test two_column_parity
cargo test -p biscuit-terminal --test table_parity
cargo bench -p biscuit-terminal --bench render_tree -- component_render_path_comparison
```

If validation behavior changes:

```sh
cargo test -p renderable tree::validate
cargo test -p biscuit-terminal render_tree
```

## Open Questions

- Should component-produced tree nodes be considered trusted enough to render
    through an unchecked terminal path?

- Should `ProgressHints` grow target-neutral fields such as `label`, or should
    the terminal renderer keep inferring labels from paragraph text?

- Is `Prose` required for all text escaping semantics, or can the tree renderer
    safely render literal terminal text directly for plain `Text` nodes?

- Should validation strictness and render strictness be separated so
    `RenderStrictness::Warn` can still render through an errors-only validation
    path when diagnostics are not requested?
