# `::shell` Output Ignores Surrounding Indentation Context

## Symptom

When a `::shell` directive appears inside an indented Markdown context (e.g., nested under a list item), the captured `stdout` is spliced back into the document at column 1 — even though the directive line itself was indented. This breaks the structural nesting of the surrounding container (lists, block quotes, definition continuations) and produces malformed CommonMark whenever the shell command's output is more than a single inline word.

The bug is symmetric with the previously-fixed `::toc-linking` indentation bug ([`2026-05-07-indent-toc-linking`](../_completed/2026-05-07-indent-toc-linking/spec.md)) — same family of problem, different transclusion-adjacent directive.

### Input

```md
- This review is focused on the '{{area}}' package area which has the following packages:

    ::shell sniff repo packages --package-area "{{ctx.current_package_area}}" --md

- Next list item
```

### Actual Output

```md
- This review is focused on the 'darkmatter' package area which has the following packages:

- darkmatter
- darkmatter-cli
- dmls

- Next list item
```

The shell command's multi-line `stdout` is inserted flush-left. The lines that happen to start with `-` get absorbed as siblings of the outer list, collapsing the intended structure. Plain-text output is worse: it breaks the parent list item entirely because a blank line followed by non-indented text terminates the list-item continuation.

### Expected Output

```md
- This review is focused on the 'darkmatter' package area which has the following packages:

    - darkmatter
    - darkmatter-cli
    - dmls

- Next list item
```

Every emitted line of the shell output — including blank lines between paragraphs — must be prefixed with the indentation of the container in which the `::shell` directive appeared, so the resulting Markdown remains structurally valid.

## Requirements

1. **Indent to container level.** When `::shell` (and `::shell-block`) output is spliced into the document, every line of the captured `stdout` (plus `stderr` if the directive merges them) must be prefixed with the indentation of the directive's line in the source document.
2. **Preserve directive's own indentation as the floor.** Use the leading whitespace of the line containing the `::shell` directive as the indent prefix. If that line is at column 1, no prefix is added (current behavior).
3. **Apply to every interior line, including blanks.** Blank separator lines *inside* the captured output must also receive the indent prefix, so the indented block reads as a single continuation under CommonMark rules. (A bare blank line — zero characters — between two output lines would terminate the parent block.) The trailing newline that terminates the captured output is *not* turned into an indentation-only line — see the trailing-newline note below.
4. **Tabs vs spaces.** Mirror the directive line's leading whitespace byte-for-byte. Do not normalize tabs to spaces or vice versa; the surrounding document chose its convention and the spliced output must match.
5. **Do not modify command output otherwise.** Only leading whitespace is added. Trailing whitespace on each line, line endings, embedded code fences, and Unicode content are preserved unchanged.
6. **Frontmatter shell expansion is out of scope.** Top-level frontmatter `$(...)` values are scalars; indentation does not apply. This spec covers only body `::shell` and `::shell-block` directives.
7. **Verify `::shell-block` shares the fix.** Both directive forms run through the same splice mechanism; the fix must cover both. Add a fixture for each.

## Acceptance Criteria

- A test fixture with `::shell` at 4-space indent inside a list item, whose command emits multiple lines, produces output where every emitted line (including blanks) starts with the same 4-space prefix.
- A test fixture with `::shell` at 2-space indent inside a block quote (`> > ::shell ...`) produces output where every emitted line is prefixed with the block-quote markers and the directive's indent.
- A test fixture with `::shell` at column 1 (document root) produces output unchanged from current behavior — no leading whitespace is added.
- A test fixture with `::shell-block` mirroring each of the above produces the same indented results.
- Round-tripping any of the above through a CommonMark parser produces the structurally-correct nested document — the shell-output lines are children of the parent container, not siblings.
- Existing tests that assert column-1 splice behavior either still pass (column-1 case is preserved) or are updated to reflect the new behavior, with the diff justified in the implementation plan.

## Likely Code Locations

- Body shell-expansion splice site: `darkmatter/lib/src/markdown/compose/shell_expansion/` — specifically `executor.rs` and the call site in `darkmatter/lib/src/markdown/compose/mod.rs` (`run_shell_expansion_stage`, `apply_replacements_in_reverse`).
- Shell-block splice site: `darkmatter/lib/src/markdown/compose/shell_blocks.rs` (or equivalent under `shell_blocks/`) — `run_shell_blocks_stage_for_markdown`.
- Directive parser that records the directive's `span` and source line: `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs`. The parser must also capture the column / leading-whitespace prefix of the directive line, since the splice is currently span-based and discards that information.
- Compare with the TOC-linking fix: `darkmatter/lib/src/markdown/compose/toc_linking/` — that fix already plumbs a container-indent prefix through the renderer; reuse the same helper or pattern.

## Notes

- This is a correctness bug, not cosmetic — the malformed output silently corrupts list and block-quote structure, and any downstream HTML/terminal renderer faithfully reflects that corruption.
- The fix is conceptually identical to the `::toc-linking` indent fix: capture the leading whitespace of the directive line and prepend it to every output line before splicing. If a shared helper does not already exist, extract one.
- Be careful around `apply_replacements_in_reverse`: indentation must be applied to the replacement string before it is handed to the splicer, since the splicer operates on raw byte ranges.
- Shell output that already contains a trailing newline is left with that newline intact; the indent is applied only to lines that have content (or that sit *between* content lines). The final newline is **not** followed by an indentation-only `"    "` line. This is intentional: there is no further output line for such a prefix to keep nested, and CommonMark treats a trailing `""` and a trailing `"    "` identically at the end of a container (trailing whitespace on an otherwise-blank line is stripped). Materializing `"    "` would only add stray trailing whitespace to the document, so the shared `indent_text` helper deliberately preserves the bare trailing newline.
