---
features:
  - 2026-07-04-dmls
  - 2026-07-09-suggest-constraint
  - 2026-07-10-interpolation-literal
  - 2026-07-10-schema-triggers
  - 2026-07-11-semantic-tokens
---
# DMLS Features

The Darkmatter Language Server (`dmls`) brings IDE-grade intelligence to
Markdown. It speaks standard **LSP 3.17 over stdio**, so it works in any
conforming editor, and it uses the `darkmatter` library as its single source of
truth for parsing, compose, schema, style, language grammar, cleanup, and
Markdown-aware hashing — it never re-implements those rules.

It serves two audiences from the same server:

- **Ordinary Markdown authors** get a credible, standalone Markdown language
  server, even for documents that use no Darkmatter features.
- **Darkmatter and Claudine authors** additionally get schema-validated
  frontmatter, directive and interpolation intelligence, transclusion
  navigation, and safety-aware handling of shell and remote content.

## Passive by construction

`dmls` runs inside editors and AI agents, so it is **read-only by design**.
Requests parse your documents and resolve local file paths, but they **never**:

- execute shell commands (`$(...)`, `::shell`),
- fetch remote URLs,
- mutate files outside an edit you explicitly accept, or
- run any side-effecting compose phase.

Shell and remote surfaces are *explained* statically — hover and diagnostics
tell you what compose *would* do and whether policy allows it, without ever
doing it. This guarantee is enforced by a dedicated test
([`tests/no_side_effects.rs`](../tests/no_side_effects.rs)).

## Feature reference

Features are organized in layers. Every editor gets Layer 0; the higher layers
add Darkmatter-specific intelligence on top of the same graph.

### Layer 0 — Markdown baseline

A full-featured Markdown LSP that stands on its own:

- **Go-to-definition** — jump from a link or `#anchor` to the heading or file
  it targets.
- **References / backlinks** — find every place that links to the heading or
  document under the cursor, sourced from a workspace-wide reverse index.
- **Document symbols** — the heading hierarchy of the current file for the
  outline / breadcrumb view.
- **Workspace symbols** — fuzzy (subsequence) search for any heading across the
  whole workspace.
- **Document links** — headings, links, and paths are clickable and
  eagerly resolved.
- **Folding** — collapse frontmatter, sections, fenced code, block quotes,
  lists, and tables.
- **Hover** — preview the target of a link without opening it (graph-sourced,
  no disk read).
- **Completion** — link paths, `#`-anchor names within a document, and
  fenced-code language tokens.
- **Diagnostics** — broken link paths, missing anchors, and duplicate headings
  (with related-information links between duplicate twins).

### Layer 1 — Wiki links

First-class support for `[[wiki]]`-style links:

- **Forms** — `[[target]]`, `[[target#heading]]`, and `[[target|alias]]`.
- **Completion, definition, references, and hover** for wiki targets and their
  headings.
- **Resolution** is case-sensitive and understands basename, path-suffix, and
  root-relative targets, with same-directory preference and ambiguity handling.
- **Diagnostics** — the full `wiki.*` taxonomy: unresolved, ambiguous,
  missing-heading, empty, unsupported-syntax, workspace-scope portability
  collisions, and invalid percent-escapes.
- **Configurable** insertion style (`shortest` / `relative` / `root-relative`)
  and heading-completion style; wiki links never insert a `.md` extension.

### Layer 2 — Frontmatter intelligence

Schema-aware editing inside a document's YAML frontmatter block, driven by
Darkmatter's **SimplifiedSchema**:

- **Effective schema** — the Darkmatter base schema, plus any configured
  extension schemas whose globs match the file, plus repository-scoped
  **trigger schemas** (content-triggered envelopes discovered under `schemas/`
  roots within the workspace boundary — watched for changes, with last-good
  retention and a file-level diagnostic on a malformed envelope), plus the
  document's own `$schema`, merged with compose precedence.
- **Completion** — schema keys (required keys marked), enum values, boolish
  scaffolds, `file(...)` paths, `style.*` keys, and advisory `suggest(...)`
  value candidates (including block-sequence items).
- **Hover** — a key's type, constraints, default, and `->` description, plus
  annotations for generated `ctx.*` keys.
- **Navigation** — go-to-definition and document links for `$schema` and
  `file(...)` references.
- **Structure** — nested-mapping folding and (config-gated) frontmatter
  document symbols.
- **Diagnostics** — precise key/value ranges for YAML parse errors, type
  mismatches, constraint violations, missing required keys, unknown keys,
  invalid file references, invalid `suggest(...)` candidates, malformed
  standalone schema envelopes, and `style.*` problems.

Claudine (and any other tool) activates as **pure configuration**: point a
`[schema.extensions.*]` entry at a schema file and a set of globs. There is no
Claudine-specific code in the server.

### Layer 3 — Darkmatter DSL

Intelligence for Darkmatter's composition DSL — all **read-only**:

- **Directives** (`::file`, `::code`, `::shell`, `::block`, disclosures, …) —
  `::`-prefix name completion, per-family option-key/enum completion, semantics
  and resolved-target hover, block/disclosure folding, and `dm.directive.*`
  diagnostics (unknown directive, malformed option, unclosed block, malformed
  disclosure).
- **Transclusion** (`::file` / `::code` / `prologue` / `epilogue`) — document
  links, go-to-definition, "who transcludes this file" references,
  broken-path diagnostics, and **cycle detection** (with the cycle ancestry
  reported).
- **Interpolation** (`{{ }}`) — completion (frontmatter keys, `ctx.*`,
  functions), hover showing the resolved static value (falling back to the
  effective-schema property description for a declared-but-unset key),
  variable → frontmatter-key definition, and malformed / unknown-identifier
  diagnostics (a schema-declared property counts as known).
- **Interpolation literals** (`{{{ … }}}`) — recognized as inert: hover shows
  the composed `{{ … }}` output with an inertness note, the content produces
  no interpolation diagnostics, and a quick-fix can wrap a spurious malformed
  interpolation into a literal.
- **Shell awareness** — `::shell` and frontmatter `$()` values hover with an
  approved / denied / unknown policy verdict, and denied built-ins raise a
  `darkmatter.security.*` diagnostic. Nothing is ever executed.
- **Fenced-code languages** — unknown fence languages are flagged with a
  nearest-match suggestion.

### Semantic tokens

Server-emitted LSP semantic tokens let a theme *de-emphasize Darkmatter
machinery* so prose stands out — the one LSP mechanism that lets the server
influence styling without per-editor grammar forks. Tokens **classify**, they
never style; the theme decides the colors, and each editor guide ships a copyable
styling recipe.

- **Interpolations** — the whole `{{ … }}` span is `macro.interpolation`;
  `{{{ literal }}}` adds `inert`.
- **Directives** — the twelve `::` keywords are `macro.directive`; the three
  structural closers (`::end-block`, `::details`, `::end-disclosure`) add
  `closer`. Structured targets are `string.directive`, option keys/values
  `property.directive` / `string.directive`. Unknown directives, `::shell`
  payloads, and disclosure summary prose get no token.
- **Wiki links** — brackets and `#`/`|` separators are `macro.wiki`; the
  path/heading/alias segments are `string.wiki`, rendered link-like rather than
  muted (identical for resolved and unresolved targets).
- **Correctness** — tokens never overlap and are strictly ordered in both UTF-8
  and UTF-16; multi-line spans split per line; fenced-code content yields no
  tokens; overlapping constructs follow a fixed precedence (interpolation > wiki
  > directive), so `when="{{ x }}"` is an interpolation inside a directive.
- **Endpoints** — `textDocument/semanticTokens/full` and `…/range` (a range
  response is exactly the full response intersected and clipped). Delta is
  deferred.
- **Gating & config** — advertised only to clients that support semantic tokens
  (Helix is naturally excluded and unaffected). The `[semantic_tokens] enable`
  master switch (default on) toggles emission live; `wiki.enable = false`
  suppresses only the wiki family. See each editor guide for the styling recipe.

### Editing

Refactoring and formatting that respect workspace-wide references:

- **Heading rename** rewrites the heading and every Markdown `#slug` and wiki
  `#heading` reference that uniquely resolves to it, preserving each link's
  spelling class. Ambiguous (duplicate-text) headings refuse to rename rather
  than corrupt links.
- **File rename** rewrites the wiki links and Markdown links that point at a
  moved file, escalating to the shortest unique path and **aborting atomically**
  on any conflict — never a partial rename. (Requires client file-operation
  support; see the matrix below.)
- **Code actions** (diagnostic-driven): create the missing file / wiki note,
  add a missing required schema key, migrate a deprecated `style:` key,
  close an unclosed `::block`, and wrap a malformed `{{ … }}` in an
  interpolation literal (`{{{ … }}}`).
- **Formatting** — whole-document formatting that is byte-equivalent to the
  `md clean` cleanup sequence, with optional reflow to a fixed width.

## Editor support matrix

`dmls` gates optional behavior on a per-client capability profile built at
`initialize`, so each editor gets the richest surface it actually supports and
degrades safely otherwise. All four primary editors get the **complete feature
set** described above; the differences below are in *how* a few capabilities are
delivered, driven by what each client advertises.

Legend: ✅ full · ⚠️ supported with a caveat (see notes) · ❌ not available in v1.

| Capability | VS Code | Zed | Neovim | Helix |
|------------|:-------:|:---:|:------:|:-----:|
| Navigation (definition, references, symbols) | ✅ | ✅ | ✅ | ✅ |
| Document links | ✅ | ✅ | ✅ | ✅ |
| Diagnostics (push) | ✅ | ✅ | ✅ | ✅ |
| Completion (link, anchor, fence, wiki, schema, DSL) | ✅ | ✅ | ✅ | ✅ |
| Hover | ✅ | ✅ | ✅ | ✅ |
| Wiki links (Layer 1) | ✅ | ✅ | ✅ | ✅ |
| Frontmatter schema intelligence (Layer 2) | ✅ | ✅ | ✅ | ✅ |
| Darkmatter DSL intelligence (Layer 3) | ✅ | ✅ | ✅ | ✅ |
| Read-only shell-policy hover + security diagnostics | ✅ | ✅ | ✅ | ✅ |
| Heading rename | ✅ | ✅ | ✅ | ✅ |
| File rename (link rewriting) | ✅ | ✅ | ❌ | ✅ |
| Code actions (v1 set) | ✅ | ✅ | ✅ | ✅ |
| Whole-document formatting | ✅ | ✅ | ✅ | ✅ |
| Semantic tokens (de-emphasis) | ✅ | ⚠️ | ✅ | ❌ |
| Folding | ✅ | ✅ | ⚠️ | ⚠️ |
| Rename-preview change annotations | ✅ | ⚠️ | ✅ | ⚠️ |
| Client-side file watching | ✅ | ✅ | ⚠️ | ✅ |
| Position encoding | UTF-16 | UTF-16 | UTF-8/UTF-16 | UTF-8 |
| Hover Markdown fidelity | Richest | Text-first | Text-first | Text-first |

### Notes on the caveats

- **File rename (Neovim).** Neovim does not report file-operation notifications
  (`workspace/willRenameFiles`), so renaming a file does not rewrite the links
  pointing at it. Heading rename works normally. All other editors rewrite links
  on file rename.
- **Folding (Neovim).** Neovim requests line-only folding ranges; `dmls` emits
  line-safe ranges everywhere, so this needs no configuration. On 0.11+ you can
  wire folds through `vim.lsp.foldexpr()`.
- **Folding (Helix).** Helix does not advertise LSP folding ranges and uses its
  own tree-sitter folding instead; `dmls` gates LSP folding off for Helix.
- **Semantic tokens (Zed).** Zed ships semantic tokens **disabled by default**;
  enable them per language with `"semantic_tokens": "combined"` (or `"full"`).
  Colors come from the active theme via the recipe in the Zed guide.
- **Semantic tokens (Helix).** Helix's LSP client does not advertise
  semantic-token support, so `dmls` does not offer the provider to Helix
  (capability-gated off); Helix keeps its tree-sitter Markdown highlighting
  unchanged and loses nothing. VS Code and Neovim classify tokens with no opt-in.
- **Change annotations.** Rename-preview grouping via `ChangeAnnotation`s is
  applied only where advertised (VS Code, Neovim). For Zed and Helix, `dmls`
  puts the explanation in explicit code-action titles instead, so nothing is
  lost.
- **File watching (Neovim on Linux).** Client-side watching is limited on Linux;
  `dmls` keeps a server-side rescan fallback (a save re-runs discovery), so
  changes to unopened files still reach the workspace graph — no configuration
  needed.
- **Hover fidelity.** Hover is text-first Markdown everywhere and never requires
  images. VS Code renders the richest Markdown; the others render conservatively
  in a floating window or popover.

For the full, source-cited capability matrix behind these gates, see
[`design/research/r7-editor-capability-matrix.md`](../design/research/r7-editor-capability-matrix.md).

## Setup and configuration

- **Per-editor setup guides** (VS Code, Zed, Neovim, Helix) and a manual smoke
  checklist live in [`docs/editors/`](./editors/).
- **Configuration** is a `.dmls.toml` file at the workspace root (which also
  serves as the editor root marker), layered under LSP
  `workspace/configuration` and reloadable without a restart. Keys cover wiki
  behavior, baseline schema extensions, strict schema/style modes, shell-policy
  discovery, code-action categories, formatting, semantic tokens
  (`[semantic_tokens] enable`), and diagnostics debounce.
