---
created: 2026-06-12
reviewed: true
status: published
audience: darkmatter library callers migrating from YamlBlock
---

# Migrating from `YamlBlock` to `CodeBlock`

`YamlBlock` is a thin compatibility wrapper around
[`CodeBlock::yaml`](../../lib/src/markdown/code_block.rs). Its render methods
delegate to `CodeBlock` for both terminal and browser targets, so output is
byte-for-byte equal to a Markdown ` ```yaml ` fence for the same payload. The
two surfaces stay in lockstep because the shared helper is the only
implementation.

The type is now `#[deprecated]`. New code should construct a `CodeBlock`
directly. The constructors that validate YAML up front remain on `YamlBlock`
for callers that need upfront validation; suppress the deprecation warning
locally if you need them.

## Quick migration

Before:

```rust
use darkmatter::markdown::YamlBlock;

let block = YamlBlock::new("foo: 1\nbar: 2")?;
let out = block.render(&term);
```

After:

```rust
use darkmatter::markdown::code_block::CodeBlock;

let block = CodeBlock::yaml("foo: 1\nbar: 2");
let out = block.render(&term);
```

A few other quick conversions:

| Old (YamlBlock)                                  | New (CodeBlock)                                 |
|--------------------------------------------------|-------------------------------------------------|
| `YamlBlock::new(yaml)?`                          | `CodeBlock::yaml(yaml)`                         |
| `YamlBlock::from_yaml_file(path)?`               | `CodeBlock::from_source_file(path)?`            |
| `YamlBlock::from_markdown_content(md)?`          | `Markdown::try_from_content(md)?.frontmatter()` |
| `YamlBlock::from_markdown_file(path)?`           | `Markdown::try_from_file(path)?.frontmatter()`  |
| `block.yaml()`                                   | `block.code()`                                  |
| `block.into_yaml()`                              | `block.into_code()`                             |
| `block.render(term)`                             | `block.render(term)` (same call)                |
| `block.render_html_fragment()`                   | `block.render_html_fragment()` (same call)      |

`CodeBlock` also exposes typed convenience constructors for other languages:

- `CodeBlock::rust(code)`
- `CodeBlock::json(code)`
- `CodeBlock::toml(code)`
- `CodeBlock::new(code, Some("language"))` — for arbitrary languages
- `CodeBlock::with_fence_language(token)` — resolves via
  [`LanguageGrammar::from_fence_token`](../../lib/src/markdown/language_grammar.rs)
  (preserves the `sh` / `shell` / `python` / `py` / `yml` / `yaml` aliases)
- `CodeBlock::with_meta(meta)` — sets `CodeBlockMeta` (title, line
  numbering, highlight ranges) used by the renderer

## Why the change

`YamlBlock` accumulated overlapping rendering paths with `CodeBlock` and
Markdown fence rendering. The simplified-rendering model centers on two
public components:

- `CodeBlock` — the atomic renderer for one syntax-highlighted code block.
- `DarkmatterPage` — the page assembler for full Markdown documents.

A fenced ` ```yaml ` block in a `DarkmatterPage` and a direct
`CodeBlock::yaml(payload).render(...)` produce byte-for-byte equal output for
the same code, language, and metadata — there is no separate fence path. A
`YamlBlock` rendering the same payload produces the same bytes too, because
it delegates to `CodeBlock::yaml`.

The motivational defect the consolidation fixes (a real dark terminal whose
code panel failed to separate from the page) was a dual-color-mode-source
problem, not a `YamlBlock`-specific defect. The fix lives at the
`ThemePair -> Theme` boundary resolver
([`themes.rs::resolve_for_surface`](../../lib/src/markdown/highlighting/themes.rs))
and applies to all code-block surfaces. The cross-surface contrast
guardrail in
[`page.rs::tests::cross_surface_contrast_guardrail_terminal_and_browser`](../../lib/src/layout/page.rs)
locks the contract.

## Behavior differences

A few small differences apply to migrated code:

1. **No upfront YAML validation.** `CodeBlock::yaml` accepts any string. If
   you relied on `YamlBlock::new` returning an error for malformed YAML,
   call `serde_yaml_ng::from_str::<serde_yaml_ng::Value>(payload)` yourself
   before constructing the `CodeBlock`.

2. **No `from_markdown_content` / `from_markdown_file`.** Use
   `Markdown::try_from_content(md)?.frontmatter()` (or the file variant)
   to extract the frontmatter, then pass the serialized YAML payload to
   `CodeBlock::yaml`. Note that `FrontmatterMap` round-trips through
   `serde_json::Value`, so comments, anchors, and custom tags are dropped —
   the same lossy behavior `YamlBlock` already documented.

3. **No `YamlBlock` layout state.** The `Layout` (margin, padding, max-width)
   moves onto the `CodeBlock` itself via `layout_mut()`. The byte-for-byte
   parity test `test_terminal_render_with_layout_equals_code_block` proves
   the same layout applied to a `YamlBlock` reaches the same output when
   applied to a `CodeBlock::yaml`.

## Suppressing the deprecation

If you still need `YamlBlock::new` for validation up front, suppress the
warning at the call site (do not add `#[allow(deprecated)]` to a module —
that hides the migration signal from other readers):

```rust
#[allow(deprecated)]
fn parse_yaml_with_validation(payload: &str) -> Result<(), YamlBlockError> {
    YamlBlock::new(payload).map(|_| ())
}
```

The deprecation does not delete `YamlBlock`; it documents the migration
path. Removal is tracked separately.

## Where this lives

- `darkmatter/lib/src/markdown/code_block.rs` — the canonical
  `CodeBlock` type and its constructors.
- `darkmatter/lib/src/markdown/yaml_block.rs` — the deprecated
  `YamlBlock` wrapper (its render methods delegate to `CodeBlock`).
- `darkmatter/lib/src/markdown/language_grammar.rs` —
  `LanguageGrammar::from_fence_token` and the guaranteed aliases.
- `darkmatter/lib/src/markdown/highlighting/themes.rs` —
  `ThemePair::resolve_for_surface` is the single boundary resolver for
  terminal and browser theme/mode selection.
- `darkmatter/lib/src/layout/page.rs` — `DarkmatterPage`, the page
  assembler, with the cross-surface contrast guardrail that catches the
  dual-color-mode defect.
- `darkmatter/cli/src/commands.rs` — `md code-block` CLI command, the
  direct `CodeBlock` rendering entry point.
