---
ready: false
agent: codex
model: ""
---

# Review 1

## Findings

### High: No-color terminals still receive emphasis and reset SGR

The specification requires `ColorDepth::None` to emit no color or style SGR.
`text_appearance_sgr` suppresses color through `color_sgr`, but always calls
`emphasis_sgr` first
(`biscuit-terminal/lib/src/render_tree/style.rs:255`). Strong, emphasis,
delete, styled spans, and inline code then always append a reset sequence
(`biscuit-terminal/lib/src/render_tree/render.rs:971`).

Consequently, the feature's own `<dim>` and `<b>` fixture still emits style
escapes under `ColorDepth::None`. The test passes because it rejects only
`38;` and `48;` color sequences, not `ESC` or emphasis/reset SGR
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:283`).

Make the no-color policy consistent in `text_appearance_sgr` and its closing
paths, then assert that the complete output contains no escape character for
both standard tree rendering and cursor-aligned bespoke rendering.

### High: Terminal-visible styling and geometry have no Level 2 verification

The feature promises capability-resolved dim, bold, color, links, visible-width
wrapping, border reset behavior, and cursor-aligned output. All new terminal
tests are Level 1 and inspect manufactured strings. The multiline test strips
ANSI before checking borders, so it cannot detect style bleed or misplaced
resets (`biscuit-terminal/lib/tests/prose_cells_parity.rs:309`). The two-path
test also strips ANSI and checks only token presence
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:382`).

Despite the specification's statement that no Level 2 test is required, the
review rubric requires real-terminal capture for glyph widths, SGR styling,
wrapping, and cursor-positioned geometry. Add Level 2 coverage through the
canonical terminal harness for styled multiline/wrapped cells and the
cursor-alignment path. Capture pane text for geometry and, where supported by
the harness, verify the styled runs or terminal cell attributes. This is a
production-readiness blocker.

### High: Browser style preservation is not verified in a real browser

The Browser requirement includes supported style attributes inside `<td>`, but
the tests only search generated HTML for `<strong>`, `<em>`, and a link
attribute (`biscuit-terminal/lib/tests/prose_cells_parity.rs:426`). There is no
colored or otherwise styled Prose-cell assertion, and no `browser_*` test
checking computed style.

Add a browser-tier test that renders a colored/background/underline Prose cell
and asserts computed styles on the descendant inside the `<td>`. Keep the L1
HTML structure checks, but they do not establish rendered browser behavior.

### Medium: The “resolve once” test does not measure resolution count

`bespoke_resolves_prose_once` counts the visible word `hello` in final output
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:401`). Rendering or parsing
the same cell repeatedly during measurement and emission would still produce
one final occurrence, so this test cannot catch the regression identified by
the specification.

Use focused instrumentation around Prose resolution, or expose a test-only
resolver hook/counter, and assert one call per `StyledProse` cell before either
bespoke planner runs.

### Medium: Markdown and MarkdownPlus tests do not establish parity

Portable Markdown is required to match standalone Prose for color degradation
and significant-character escaping, while MarkdownPlus must match richer style
lowering. The new portable tests cover semantic wrappers, links, one pipe, and
one newline. The MarkdownPlus test checks only bold and italic independently in
both outputs rather than comparing results, and never exercises its richer
color/background/underline behavior
(`biscuit-terminal/lib/tests/prose_cells_parity.rs:541`).

Use a shared corpus and compare the rendered cell body with standalone Prose
for portable Markdown and MarkdownPlus. Include color, background, underline,
link destinations containing significant characters, literal Markdown
characters, pipes, and hard/soft line breaks.

### Medium: The public enum shape differs from the specified API

The specification and plan define `StyledProse(Prose)`, but the implementation
publishes `StyledProse(Box<Prose>)`
(`biscuit-terminal/lib/src/components/table/cell.rs:58`). `From<Prose>` remains
ergonomic, but callers cannot construct or pattern-match the documented
specified variant without knowing about the allocation.

Either implement the specified direct payload or document and approve the
boxing as an intentional API/performance decision. The current plan still
claims the direct variant was implemented.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Conversion, structured projection, hints, code degradation, layout isolation | L1 | Appropriate |
| Terminal capability resolution and no-color behavior | L1 | Functional defect; no-color assertion is incomplete |
| Terminal wrapping, visible width, reset isolation, borders | L1 | Gap; Level 2 required |
| Cursor-alignment terminal path | L1 | Gap; Level 2 required |
| Browser semantic markup and links | L1 HTML string checks | Useful structural coverage |
| Browser supported visual styles | None | Gap; Browser computed-style test required |
| Portable Markdown semantics and GFM escaping | L1 | Appropriate level, incomplete corpus |
| MarkdownPlus richer styles | L1 | Appropriate level, requirement not exercised |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Verification Run

- `cargo test -p biscuit-terminal --test prose_cells_parity`: 53 passed.
- `cargo test -p renderable --lib`: 475 passed.
- `cargo check -p biscuit-terminal -p renderable`: passed.
- `just test` for `biscuit-terminal`: 2,516 library tests and 357 CLI tests
  passed; resource-gated tests were skipped by the configured test profile.

The green Level 1 results confirm that the current assertions pass, but they do
not resolve the functional defect or the required higher-level verification
gaps.

## Additional Recommendations

`render_bespoke` currently builds resolved data and then clones the entire
table, including the original data, before replacing that clone
(`biscuit-terminal/lib/src/components/table/table.rs:1611`). Resolve Prose
cells in-place on one cloned table, or clone only table metadata, to avoid two
full data clones on the cursor-alignment path.

The filesystem source/test edits in this feature worktree are unrelated to
Prose table cells. Keep them out of the feature change so review and rollback
remain scoped.
