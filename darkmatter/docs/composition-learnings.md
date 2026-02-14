# Composition Learnings

Troubleshooting notes and design decisions discovered while working on the `md compose` subcommand.

## Definition List Interference with `::` Directives

**Discovered**: 2026-02-14

pulldown-cmark's `ENABLE_DEFINITION_LIST` option (included in `Options::all()`) parses `::` at the start of a line as a definition list marker (`: ` + content). This mangles transclusion directives:

```
::file ./one.md        <-- parsed as paragraph (term)
                       <-- blank line
::file ./two.md        <-- parsed as definition: ": :file ./two.md"
```

The second directive is silently converted from `::file` to `: :file`, so `parse_directives` never finds it. The literal mangled text appears in the output instead of the transcluded content.

**Fix**: `cleanup_parser_options()` now excludes `ENABLE_DEFINITION_LIST`. This is safe because darkmatter's `::` directive syntax conflicts with definition list syntax at a fundamental level — both use `:` at line start. If definition list support is needed in the future, the cleanup stage would need to protect `::` directives (via placeholders) before parsing.

## Loose List Marker Preservation

**Discovered**: 2026-02-14

A "loose list" has blank lines between items (each item becomes a paragraph). The original `extract_list_markers` captured one marker per `List(None)` event — one per **list**, not per **item**. For a 4-item loose list using `-`, only one `-` was captured.

During restoration, `restore_list_markers` used a stack-based heuristic that treated each item in a loose list as a "new list" (because `prev_was_list_item` was false after the blank line). After consuming the single captured marker for the first item, subsequent items fell back to the default `*`.

**Fix**: `extract_list_markers` now captures one marker per `Item` event within unordered lists (tracked via a `list_type_stack`). `restore_list_markers` was simplified to sequential 1:1 consumption — each `* ` line in the cmark output consumes the next marker from the extracted list.

**Key insight**: pulldown-cmark-to-cmark always normalizes unordered list markers to `*`. The extraction and restoration must have a 1:1 mapping between source items and output `* ` lines. Per-item extraction guarantees this regardless of tight vs loose list structure.

## Trailing Blank Line After Transcluded Content

**Discovered**: 2026-02-14

When `::file` directive content replaces the directive span in the parent document, the child content may not end with a blank line. If the next content in the parent starts on the very next line, markdown parsers can absorb it into the child's last block element (e.g., a list item continuing with an indented paragraph).

**Fix**: `render_markdown_transclusion` now ensures block-directive content ends with `\n\n`. This matches what `render_code_transclusion` already does via `ensure_vertical_spacing`. The fix is scoped to block directives (`insertion_context.is_some()`) — frontmatter prologue/epilogue transclusion doesn't need it because sections are joined with `"\n\n"`.

## Wrapper Functions Strip Trailing Blank Lines

**Discovered**: 2026-02-14

A variant of the trailing blank line issue above. When `::file` uses `quotation="..."`, the `wrap_quotation` function transforms content using `.lines()` + `.join("\n")`. The `.lines()` iterator strips trailing newlines, and `.join("\n")` produces output with no trailing `\n`. This means the `\n\n` guarantee added before wrapping is consumed by the wrapper, and subsequent parent content becomes a lazy continuation of the blockquote.

**Example**: `::file ./quote.md quotation="Wikipedia"` followed by a paragraph — the paragraph appears inside the blockquote instead of after it.

**Fix**: The trailing `\n\n` guarantee was moved from before `apply_wrappers` to after it. This ensures the final output (post-wrapping) always has a blank line separator, regardless of what wrappers do internally. The same fix covers any future wrappers that might similarly consume trailing whitespace.

## Pipeline Stage Ordering Matters

The transform pipeline runs stages in this order:

1. **Stage 1**: Text Replacement → Interpolation → Cleanup → Normalization
2. **Stage 2**: Block Transclusion → Frontmatter Transclusion

Cleanup runs **before** transclusion. This means:

- `::file` directives in the root document pass through cleanup (and must survive it)
- Child documents get their own full Stage 1 pipeline before their content is inserted
- Any cleanup option that modifies `::` line syntax will break transclusion (see definition list issue above)
