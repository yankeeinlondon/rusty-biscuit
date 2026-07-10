---
features:
  - 2026-07-04-dmls
  - 2026-07-08-modal-and-autocomplete
---
# DMLS Autocomplete

Autocomplete (the LSP `textDocument/completion` capability) offers context-aware
suggestions as you type: link paths, heading anchors, fenced-code languages,
wiki targets, frontmatter schema keys and values, directives, and `{{ }}`
interpolation variables and functions.

## Eager by design

DMLS completion is **eager and self-contained**:

- Every item carries an explicit **`textEdit`** that replaces the exact token
  under the cursor — nothing is left to the client to compute.
- The server advertises **`resolveProvider: false`**: it never defers `detail`,
  `documentation`, or `textEdit` to a `completionItem/resolve` round-trip. All
  fields are present on the first response.
- Insertion is **plain text, not snippets** — no tab stops, and functions are
  inserted as a bare name with no synthesized parentheses.

This is deliberately **Zed-safe**: Zed does not resolve `completionItem.textEdit`
for performance, so a server that relies on resolve would silently misbehave
there. Eager `textEdit` works identically across VS Code, Zed, Neovim, and Helix.

## Trigger characters

Completion is offered on manual invocation and automatically after these
characters:

| Character | Triggers |
|-----------|----------|
| `/` | link and wiki path segments |
| `#` | heading anchors (`[...](doc.md#…)`, `[[target#…]]`) |
| `(` | directive options / expression function arguments |
| `.` | `ctx.` members inside an open `{{ }}` interpolation |

Each provider re-checks context, so a trigger character in ordinary prose
produces nothing. In particular, a `.` outside an open interpolation yields no
DSL completion items.

## What answers a completion

Completion is a provider-chain capability. Each provider contributes items for
the contexts it owns, and the registry **merges them (union, de-duplicated)** —
unlike hover, which stops at the first non-empty result. Providers run in
registration order (substrate → wiki → frontmatter → DSL).

| Provider | Context | Offers |
|----------|---------|--------|
| substrate (Markdown) | inside a link path / after `#` | document paths; `#`-anchor names for the target; fenced-code language tokens |
| wiki | inside `[[ … ]]` | wiki targets (path style configurable) and `#heading` names; never inserts a `.md` extension |
| frontmatter | inside the frontmatter block | schema keys (required-marked), enum values, boolish scaffolds, `file(...)` paths, `style.*` keys, and `ctx.*` variables |
| DSL | on a `::` line or inside `{{ }}` | directive names, per-family option keys/enum values, and interpolation variables/functions |

## Interpolation completion (`{{ … }}`)

Inside an interpolation, the DSL provider offers three candidate families:

1. **Frontmatter keys** — the document's own top-level keys.
2. **`ctx.*` context variables** — offered fully qualified (`ctx.today`,
   `ctx.packages`, …).
3. **Expression functions** — every function in the catalog, including the six
   list-formatting functions (`as_csv`, `as_tsv`, `as_space_separated`,
   `as_line_separated`, `as_unordered_list`, `as_ordered_list`).

Matching is **prefix-based and case-sensitive**. `{{ ctx.pa }}` offers matching
`ctx.*` variables; it does not offer the removed `*_list` aliases.

### `ctx.` requires an explicit prefix

A `ctx.*` variable is only offered/annotated when the cursor is on an explicitly
`ctx.`-qualified expression. A bare `{{ today }}` is treated as a **frontmatter**
variable, even when `today` is also a known context-variable tail — the two
namespaces never bleed into each other.

## Completion item metadata

Items populate the LSP fields intended for type and documentation so editors can
render rich completion popups without a resolve round-trip.

**`ctx.<name>` variable:**

| Field | Value |
|-------|-------|
| `label` / inserted text | the qualified `ctx.<name>` |
| `kind` | `VARIABLE` |
| `detail` | the descriptor's rendered type (e.g. `string[]`) |
| `documentation` | eager Markdown containing the descriptor's description |
| `textEdit` | eagerly replaces the current interpolation token |

**Expression function:**

| Field | Value |
|-------|-------|
| `label` | the untyped signature (e.g. `as_csv(list)`) |
| inserted text | the bare function name (no parentheses) |
| `detail` | the typed signature (e.g. `as_csv(list: any[]) -> string \| error`) |
| `documentation` | the descriptor description as eager Markdown |

The type and documentation come from the single-sourced Darkmatter catalogs via
one passive adapter — DMLS does not maintain a parallel table of variable types,
function signatures, or descriptions, so completion never drifts from the
library's semantics.

## Configuration

Via `.dmls.toml` (layered under LSP `workspace/configuration`, reloadable without
restart):

- **`wiki.path_style`** — `shortest` / `relative` / `root-relative`: how an
  inserted wiki target is spelled.
- **`wiki.heading_completion_style`** — how `#heading` completions are rendered.

Wiki completion never inserts a `.md` extension regardless of style.

## Passivity

Answering a completion is read-only: it reads open buffers and the in-memory
workspace graph and resolves local paths, but never executes a shell command,
evaluates an expression, fetches a remote URL, or mutates a file. Offering a
`ctx.*` variable or a function does **not** compute its value.

## Not included in v1

- LSP `textDocument/signatureHelp`, active-parameter tracking, and automatic
  insertion of parentheses or arguments.
- `completionItem/resolve` (everything is eager).
- Snippet insert text / tab stops.
- Compatibility completions for the removed `ctx.*_list` variables.

## See also

- [Hover](./hover.md) — the sibling capability; how `ctx.*`, functions, and
  directives are *explained*, and the Markdown-formatting limits DMLS works
  within.
- [Diagnostics](./diagnostics.md) — problem reporting and the stable code taxonomy.
- [Features](./features.md) — the full capability overview and per-editor matrix.
