---
name: darkmatter
description: Expert knowledge for the darkmatter Rust library - Markdown parsing, composition, frontmatter, terminal/HTML/Markdown rendering, style frontmatter, syntax highlighting, document comparison, and disclosure blocks. Use when parsing or composing Markdown, rendering Markdown to terminal/HTML/Markdown, working with DarkmatterPage, `style:` frontmatter, frontmatter hashing, disclosure blocks (`::disclosure` / `::details` / `::end-disclosure`), or comparing documents.
hash: 87f17662fa397abe-13cb5bd473770411
last_updated: 2026-06-30
---

# darkmatter

Darkmatter owns Markdown parsing, composition, frontmatter, document comparison,
and the public Markdown rendering paths. Terminal capability detection,
terminal components, images, Mermaid, and graph rendering are delegated to
`biscuit-terminal`.

## Start Here

The simplified-rendering model centers on **two public components**:

- `darkmatter::markdown::code_block::CodeBlock` — the atomic renderer for
  one syntax-highlighted code block. Use it for a single snippet, a fenced
  block in Markdown (which routes through the same helper), or a direct
  `CodeBlock::yaml/rust/json/toml/from_source_file` call. It implements
  `TerminalRenderable` and `BrowserRenderable` directly.
- `darkmatter::layout::DarkmatterPage` — the page assembler that renders
  a full Markdown document. It owns page-frame layout (margins, padding,
  max-width, page background) and delegates nested code blocks to
  `CodeBlock` so a fence in a `DarkmatterPage` renders byte-for-byte
  equal to a direct `CodeBlock` call.

Other entry points:

- Use `darkmatter::markdown::Markdown` for document content.
- Use the compose pipeline for source transformations before rendering.
- Use `Markdown::cleanup`, `cleanup_compact`, `cleanup_loose`, and
  `cleanup_with_indent*` for canonical cleanup. These strip incidental
  single newlines in prose by default before the existing whitespace/list
  cleanup. Use `Markdown::strip_incidental_newlines` directly for only that
  pass, or `Markdown::cleanup_with_fixed_width(width)` to clean and then
  reflow prose to a display-column width. The `md clean` CLI exposes the same
  behavior with `--fixed-width <#>` and can preserve source single newlines
  with `--ignore-incidental-newlines`; those two flags conflict.
- Use `darkmatter::style` for document `style:` frontmatter.
- Use `biscuit-terminal` components for rich terminal UI outside ordinary
  parsed Markdown rendering.

## Responsibility Split

| Need | Owner |
|------|-------|
| CommonMark/GFM parsing | `darkmatter` |
| Compose pipeline, interpolation, shell directives, transclusion | `darkmatter` |
| Frontmatter extraction and Markdown-aware hashing | `darkmatter` / `md hash` |
| `$schema` / SimplifiedSchema validation, detection, compose integration | `darkmatter::markdown::schemas` |
| `style:` frontmatter parsing and application | `darkmatter::style` |
| HTML and terminal Markdown renderers | `darkmatter` |
| Terminal capability detection, images, Mermaid, graph adapters | `biscuit-terminal` |
| Shared render tree and target-agnostic layout/style types | `renderable` |

## Grammar Authority

`darkmatter::markdown::language_grammar::LanguageGrammar` is the single
production grammar authority in Darkmatter. All code that resolves a fence
token, extension, filename, or syntect name to a syntax grammar must route
through `LanguageGrammar` — do not call
`syntect::parsing::SyntaxSet::find_syntax_by_extension`,
`find_syntax_by_name`, or equivalent syntect lookup APIs directly in
production code outside the `LanguageGrammar` implementation.

Preferred entry points:

- `from_token` — Markdown fence info strings (ignores metadata after the first
  token).
- `from_token_or_plain_text` / `from_lossy` — infallible; fall back to
  `LanguageGrammar::PlainText` for unknown input.
- `from_extension` / `from_name` / `from_filename` — explicit caller intent.
- `yaml()`, `rust()`, `json()`, `toml()`, `markdown()` — guaranteed named
  variants.

Because `LanguageGrammar` resolves against the two-face extended grammar set,
code transclusion recognizes extensions that syntect's bare defaults lack
(e.g. `.ts`, `.toml`). A two-face-only extension emits its real token instead
of the fallback token in composed Markdown output; this is intended widening,
not a regression.

## `style:` Frontmatter Status

The active style-frontmatter wiring phase is
`darkmatter::style::parse::ACTIVE_STYLE_WIRING_SUB_SPEC = 8`.

Implemented:

- schema/parser with kebab-case canonical keys and snake-case deprecation
  aliases
- `style.page.*` layout/background wiring
- `style.table.*`, `style.images.*`, `style.block-quote.*` layout/fill wiring
- `style.ul.*`, `style.ol.*`, `style.li.*` list wiring
- page and component `color` / `bg-color` wiring
- `md --strict-style`, which fails on unknown/deprecated schema keys but not
  on valid future-phase keys
- sub-spec #6 HR migration: top-level `hr:` merges into `style.hr.*` with
  `Deprecated` warnings; inline `{ style: ... }` is parsed as a deprecated alias
  for `{ kind: ... }`; `apply_hr_style` wires `style.hr.*` onto `DarkmatterPage`
- sub-spec #7 bespoke knobs: `page.stylesheet`, `page.meta`, `page.code.theme`,
  hyperlink/image local-style behavior; `apply_bespoke_style` wired into the
  CLI render pipeline
- sub-spec #8 `style.disclosure.*` disclosure blocks
- sub-spec #9 (Style Everywhere): the schema recognizes expanded per-component
  layout/appearance keys (`margin`, `padding`, `border`, `emphasis`, `word-wrap`)
  and explicit `width` mode keywords (`auto`, `fit-content`, lengths). The
  applicator lowers these values to `ComponentPolicy`. See the
  [Style Everywhere matrix](../../../renderable/features/2026-06-30-style-everywhere/matrix.md)
  for the per-component support contract.

No valid v1 schema keys are silently ignored: each is either honored,
rejected, or reported via `KnownButInactive`.

CLI flags win over frontmatter field-by-field. For implementation details, read
`darkmatter/lib/src/style/{parse.rs,apply.rs,cli_claims.rs}` and
`renderable/features/2026-05-23-style-property/`.

## CLI-Neutral Style Claims (`CliStyleClaims`)

`darkmatter::style::CliStyleClaims` is a neutral data model that captures every
layout/style flag a CLI (or any other caller) may claim. It uses only
library/layout types — no clap or CLI-only wrappers:

- `apply_cli_claims(page, claims)` applies the **value side** of CLI layout
  precedence (`margin > mx > mt`, `alignment > align-lists > align-ul`, etc.).
- `page_style_overrides_from_claims`, `component_style_overrides_from_claims`,
  `list_style_overrides_from_claims`, `hr_style_overrides_from_claims`,
  `disclosure_style_overrides_from_claims`, and
  `bespoke_style_overrides_from_claims` return the **claim-side** override bits
  consumed by `apply_page_style`, `apply_component_style`, `apply_list_style`,
  `apply_hr_style`, `apply_disclosure_style`, and `apply_bespoke_style` so
  frontmatter does not overwrite CLI-claimed fields.

`darkmatter-cli` lowers its parsed `Cli` into `CliStyleClaims` in exactly one
place: `darkmatter/cli/src/style_claims.rs`. This collapses the previous
duplication where value application and override-bit construction were both
implemented near terminal rendering. `darkmatter/cli/src/render.rs` consumes
the claims for render-time page setup and style-frontmatter override bits.

## CLI Module Layout

The CLI source is split by responsibility:

- `darkmatter/cli/src/args/` — `Cli`, `Command`, targets, output enums,
  value parsers, wrapper conversions, and shell completion helpers.
- `darkmatter/cli/src/commands/mod.rs` — subcommand dispatch and top-level
  subcommand usage validation.
- `darkmatter/cli/src/commands/{render,clean,validate,graph}.rs` — focused
  command implementations for render, clean, validate, and graph.
- `darkmatter/cli/src/commands/{compose,frontmatter,hash,code_block}.rs` —
  larger or specialized command implementations that already had focused homes.
- `darkmatter/cli/src/io/` — shared Markdown input loading, stdin reads, and
  path resolution.
- `darkmatter/cli/src/render.rs` — terminal render setup, theme resolution,
  CLI claim application, and style-frontmatter application.
- `darkmatter/cli/src/artifact.rs` — output artifact creation, writing,
  opening, and terminal-image environment handling.
- `darkmatter/cli/src/style_claims.rs` — the only CLI-specific lowering from
  parsed flags into `darkmatter::style::CliStyleClaims`.

The removed god-files (`args.rs`, `commands.rs`, `output.rs`,
`tests/cli.rs`, and `tests/level2_layout.rs`) should not be recreated.

## Extracted Library Surfaces

- `renderable::color::Tailwind::from_kebab_name` resolves canonical Tailwind
  color names such as `red-500`.
- `renderable::style::PaintColor::from_css_str` parses CLI-compatible paint
  values (`#RGB`, `#RRGGBB`, `R,G,B`, Tailwind names, and CSS paint keywords)
  and reports failures with `renderable::style::ParseColorError`.
- `darkmatter::markdown::toc::TocTree` renders `MarkdownToc` as a terminal
  component.
- `darkmatter::markdown::delta::DeltaReport` renders `MarkdownDelta` as a
  terminal component; use `with_documents(...).verbose()` when visual diffs are
  needed.
- `darkmatter::markdown::reference` record/report types own their serde JSON
  shapes, so CLI JSON paths should call `serde_json` on the library values.
- `darkmatter::markdown::reference::validate::ValidationReportView` is the
  terminal view for reference validation reports.
- `darkmatter::style::CliStyleClaims`, `apply_cli_claims`, and the
  `*_style_overrides_from_claims` helpers are the single authority for CLI
  style precedence.

## Common Entry Points

```rust
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::TerminalOptions;

let md: Markdown = "# Hello\n\nWorld".into();
print!("{}", md.as_terminal(TerminalOptions::default())?);
```

```rust
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageBackground};
use darkmatter::markdown::Markdown;

let term = Terminal::new_optimistic(120);
let md: Markdown = "# Hello\n\nWorld".into();
let output = DarkmatterPage::new(&term)
    .with_margin(2)
    .with_padding(1)
    .with_page_background(PageBackground::Subtle)
    .with_max_width(100)
    .render(&md)?;
```

## Compose Pipeline

The compose pipeline runs in four phases:

**Inline Pre** (serial):

1. **Frontmatter Interpolation (pass 1)** - Resolve `{{ variable }}` in frontmatter values against seed values, `doc.*`, `ctx.*`, `env.*`. Defers keys that reference a whole-value `$(...)`.
2. **Schema Validation** (pre-operation stage) - Validate frontmatter against `$schema` or `ComposeOptions::baseline_schema`. Runs after `--set` / `--state` overrides and frontmatter interpolation, but before shell expansion. **Coerces** schema-recognized top-level scalars to their declared types (default-on, e.g. the string `"true"` → real boolean) and writes the coerced values back into frontmatter, skipping `$(...)`-pending values. Problems on fields still holding `$(...)` are deferred to downstream re-validation only when frontmatter shell expansion is enabled; when it is disabled they fail fast.
3. **Frontmatter Shell Expansion** - Execute top-level `$(cmd)` frontmatter values. Tokens in executed position follow the [`$()` token-resolution ladder](compose.md): literal → `name(...)` safe function → executable → frontmatter property → null.
4. **Frontmatter Interpolation (pass 2)** - Resolve the keys deferred in pass 1 against the now-concrete shell-expanded values.
5. **Text Replacement** - Replace literal strings from `replace:` map.
6. **Page Blocks** - Evaluate `::block`/`::end-block` conditional regions.
7. **Interpolation** - Expand `{{ variable }}` in body content.
8. **Shell Expansion** - Execute `::shell` directives.
9. **Shell Blocks** - Execute `::shell-block` directives.
10. **Link Resolve** - Resolve local links to absolute paths.

**Transclusion** (concurrent):

- `::file`, `::code`, `::toc-linking`, `::file-links`, `prologue`/`epilogue`

**Inline Post** (serial):

- Cleanup and normalization. Cleanup strips incidental single newlines by
  default, applies list/indent normalization, and can reflow prose with
  `ComposeOptions::with_fixed_width(...)`. Programmatic callers can preserve
  source single newlines with
  `ComposeOptions::with_incidental_newline_mode(IncidentalNewlineMode::Preserve)`.

**Finalization** (root-only):

- Link normalization (absolute → portable paths).

Context capture happens once at compose start, driven by `sniff` (OS, hardware,
git repo structure, monorepo discovery, file changes, document inventory) and
simple environment reads (`AGENT`, `MODEL`).
Only the context groups actually referenced by the document are captured
(demand-driven). The `--allow-ctx-override` CLI flag downgrades non-object
`ctx` frontmatter errors to warnings.

See `compose.md` for the full API, interpolation syntax, and transclusion details.

## Schema Validation

Darkmatter defines, detects, and evaluates schemas for Markdown frontmatter via **SimplifiedSchema** — a single-line YAML grammar that compiles to Draft 2020-12 JSON Schema. Optional properties are nullable: a frontmatter value that resolves to `null` validates the same way as a missing key. Key surfaces:

- `$schema` frontmatter property (inline, file reference, or root-level union).
- `md schema validate`, `md schema detect`, and `md schema about` CLI subcommands.
- `DarkmatterSchemas` library API with baseline merging and LRU validator cache.
- Always-on compose pipeline stage (after `--set`/`--state` and interpolation, before shell expansion) that also **coerces** schema-recognized scalars to their declared types and writes them back (default-on; `$(...)`-pending values are skipped and coerced at post-shell re-validation).
- `ComposeOptions::with_baseline_schema(...)` for programmatic baseline injection.
- Typed schema-language descriptor catalog (`schema_type_descriptors()`, `schema_constraint_descriptors()`, `schema_shape_descriptors()`, `inline_object_rule_descriptors()`, `coercion_rule_descriptors()`, `validation_behavior_descriptors()`) — the authoritative source for `md schema about` and the same surface library callers render their own reports from.

See `darkmatter/docs/topics/schema-definition.md` for the full topic documentation.

## Remote URL Referencing

Remote URL composition is supported for HTTP(S) `::file` / `::code` targets
and read-side expression function file arguments. Ordinary rendered HTTP(S)
links are preserved and not fetched.

- All network egress goes through `biscuit-file`'s `FetchPolicy`, which is
  deny-all by default.
- CLI callers allow exact hosts with `md compose --allow-host <host>`.
- Persistent remote artifacts require `--cache-root` or
  `ComposeOptions::with_cache_root(...)`.
- Remote freshness is controlled by `--remote-freshness`,
  `--remote-refresh`, `--remote-ttl`, and `RemoteReadConfig`.
- The side-effect `EffectEngine::http_post` uses the same shared fetch policy
  for host allowlist enforcement.
- Remote URL arguments to read-side functions work only on surfaces with a
  remote runtime — body interpolation. The **frontmatter** context (both the
  pre-shell and post-shell interpolation passes, and the `$()` shell ternary
  condition/branch) is local-only, so a remote URL there **fails loudly**.
  `absolute` and `relative` are local-only path transforms — never remote,
  never part of remote-fetch discovery.

See `darkmatter/docs/topics/remote-url-references.md` for public guidance.

## Disclosure Blocks

Darkmatter supports render-time disclosure blocks using the `::disclosure` / `::details` / `::end-disclosure` directive triple. The syntax is a block-level extension; directives must appear at line boundaries and be followed by ASCII whitespace or end-of-line.

```md
::disclosure License *Agreement*
::details
Keep your **hands** off.
::end-disclosure
```

- The summary region (between `::disclosure` and `::details`) is phrasing-only: no paragraph breaks, hard line breaks, or block-level elements.
- The body region (between `::details` and `::end-disclosure`) is full block-level Markdown and may contain nested disclosures.
- `::file` and `::code` transclusion can wrap content in a disclosure block with `disclosure="Summary text"` or `disclosure=true` (default summary is `"Details"`).

Render-tree representation: `renderable::tree::NodeKind::Disclosure { summary, children, layout, style }`, where `summary` and `children` are slices of `RenderNode` and `style` carries inline opener hints. Target behavior:

| Target | Behavior |
|--------|----------|
| Terminal | Summary rendered normally; body rendered as a block quote with dim and italic text. |
| Markdown (dialect) | DSL emitted verbatim: `::disclosure`, `::details`, `::end-disclosure`. |
| MarkdownPlus | Summary and body rendered to Markdown, then wrapped in `<details><summary>…</summary>…</details>`. |
| Browser | Summary and body rendered to HTML, then wrapped in native `<details>`/`<summary>` elements; no JavaScript. |
| JSON | Serialized natively through `Markdown::as_document()` as `NodeKind::Disclosure`. |

### Styling

Disclosures honor `style.disclosure.*` frontmatter using the same `CommonStyle` shape as tables and block-quotes (`width`, `max-width`, `alignment`, `color`, `bg-color`). Kebab-case keys are canonical; snake-case aliases (`max_width`, `bg_color`) emit a `Deprecated` warning and `--strict-style` rejects them. `width` and `max-width` are mutually exclusive.

Inline opener style tokens take precedence over frontmatter:

```md
::disclosure max-width=60ch color=red-500 License Agreement
::details
Keep your **hands** off.
::end-disclosure
```

Recognized inline keys are `width`, `max-width`/`max_width`, `alignment`, `color`, `bg-color`/`bg_color`. Unrecognized or invalid tokens become part of the summary text.

Malformed disclosures raise `MarkdownError::MalformedDisclosure { reason, range }`.

## Progressive Disclosure

Open only the topic file needed for the task:

| Topic | File |
|-------|------|
| Compose pipeline | `compose.md` |
| Expressions (read-side functions, `doc.*`/`ctx.*`/`env.*` namespaces, `$()` token resolution, authoring guide) | `darkmatter/docs/topics/darkmatter-expressions.md` |
| Context variables (`ctx.*`: date/time, repo, file changes, OS, hardware, docs, agent/model) | `darkmatter/docs/topics/context-variables.md` |
| Schema validation | `darkmatter/docs/topics/schema-definition.md` |
| Terminal rendering options | `terminal.md` |
| Frontmatter model | `frontmatter.md` |
| Error/status block conventions | `errors.md` |
| Document comparison | `comparison.md` |
| Markdown hashing (`md hash`: kinds, `--save`, `--diff`, env vars) | `darkmatter/docs/cli/hash.md` |
| Module layout | `structure.md` |
| Parser details | `pulldown-cmark.md` |

For render-tree work, switch to the `renderable` skill for the IR model and
the `biscuit-terminal` skill for terminal tree rendering.

## Current Rendering Notes

- Public `Markdown::as_html` and `Markdown::as_terminal` (and
  `DarkmatterPage::render`) route through the render-tree document renderers;
  the legacy `pulldown-cmark` event-stream serializers and `RuleProcessor` have
  been deleted (tree-cutover Phase 5).
- `darkmatter::markdown::render_tree::fold_markdown_to_document` is the
  Markdown-to-`Document` bridge every public render path folds through.
- `YamlBlock` is a deprecated thin compatibility wrapper around
  [`CodeBlock::yaml`](darkmatter/lib/src/markdown/code_block.rs); both
  render through the same shared code-block helpers, so terminal and
  browser output is byte-for-byte equal for the same payload. New code
  should use [`CodeBlock`](darkmatter/lib/src/markdown/code_block.rs)
  directly with `CodeBlock::yaml`, `CodeBlock::rust`, `CodeBlock::json`,
  `CodeBlock::toml`, `CodeBlock::from_source_file`, or
  `CodeBlock::new(code, Some("lang"))`; `YamlBlock`'s validation
  constructors remain (with a deprecation warning) for callers that need
  upfront YAML validation. The CLI exposes the same surface as
  `md code-block <file-or-content> --language LANG [--theme THEME] [--title TITLE] [--line-numbering] [--highlight RANGE] --output terminal|html|markdown`,
  which constructs a `CodeBlock` directly without routing through the
  Markdown fold.
- Code-block themes resolve against the inverted page color mode for contrast
  **by default**; ordinary prose follows the real mode. The code panel's mode is
  derived from the terminal (the same source as the page), not a separate
  env-only detector. The inversion is configurable via the `CodeBlockMode` enum
  (`inverse` (default) / `dark` / `light` / `same`) — exposed as the global
  `md --code-block <...>` flag and `DarkmatterPage::with_code_block_mode(...)`.
  `CodeBlockMode` is honored on **both** terminal and browser: the browser code
  path resolves through `HtmlOptions::code_block_mode`, and the injected
  `.code-block` stylesheet background is computed against the same mode so
  markup and stylesheet agree. A direct `CodeBlock::with_theme(theme)` /
  `md code-block --theme` override wins over the page/context theme on both
  surfaces. See `darkmatter/docs/rendering/code-highlighting.md`.
- Horizontal rules: canonical styling is `style.hr.*` with `apply_hr_style`;
  top-level `hr:` and inline `{ style: ... }` remain deprecated aliases.
- The darkmatter cutover is complete: deprecated `PageMargin`, `PagePadding`,
  `PageAlignment`, `PageFill`, `WidthUnit`, and `PageComponent::Lists` have
  been deleted. `style:` frontmatter lowers **directly** into a per-component
  `ComponentPolicy` — a `renderable::layout::Layout` plus `color` / `bg_color`
  carried as alpha-bearing `renderable::style::PaintColor`. The parsed
  `StyleColor` is lowered to `PaintColor` at the parser/apply boundary
  (`style/apply.rs`), so opacity rides in the paint's alpha channel; no
  `StyleColor` survives on post-construction component types.
- Production rendering is **one context-aware fold followed by one target fold**.
  Darkmatter's `render_tree::build_context` (`TreeBuildContext`) bakes component
  policy, page-inheriting color, alpha paint, hyperlink/image text layout,
  structured link/image browser attrs, and HR defaults onto the nodes during
  construction; the target fold then resolves all width, padding, alignment, and
  CSS. The old post-fold `decorate` pass (`decorate_document` / `component_for`)
  and the `darkmatter.style` / `darkmatter.li` render hints are **deleted** — the
  browser fold lowers alpha straight to `rgba(...)` with no HTML rewrite, and a
  malformed fenced code-block directive remains a fatal
  `MarkdownError::InvalidLineRange` via the `validate_code_directives` preflight
  the HTML entry points run over the folded tree. `DarkmatterPage` survives as a
  slim, renderable-typed page frame — the constrained **Option A** boundary
  signed off by the CSS Box Architecture closeout
  (`renderable/features/_completed/2026-06-06-tree-closeout`): a viewport-level assembler (page
  width/margin/padding, full-page background, max-width centering,
  `PageBackground::Pronounced` code-theme contrast, browser page-wrapper metadata
  + stylesheet assembly) that wraps the *folded output*, carrying no component
  policy, inspecting no component node kinds, and mutating no component content.