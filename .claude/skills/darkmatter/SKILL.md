---
name: darkmatter
description: Expert knowledge for the darkmatter Rust library - Markdown parsing, composition, frontmatter, terminal/HTML/Markdown rendering, style frontmatter, syntax highlighting, document comparison, and disclosure blocks. Use when parsing or composing Markdown, rendering Markdown to terminal/HTML/Markdown, working with DarkmatterPage, `style:` frontmatter, frontmatter hashing, disclosure blocks (`::disclosure` / `::details` / `::end-disclosure`), or comparing documents.
hash: 87f17662fa397abe-c0eb7c8a0924fdd4
last_updated: 2026-07-21
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
  single newlines in top-level and list-item prose by default before the
  existing whitespace/list cleanup. List soft-break collapse removes
  source-only continuation indentation. Use
  `Markdown::strip_incidental_newlines` directly for only that pass, or
  `Markdown::cleanup_with_fixed_width(width)` to clean and then reflow complete
  logical prose blocks to a display-column width. Newly wrapped list
  continuations use a hanging prefix that retains every enclosing list and
  blockquote container. The `md clean` CLI exposes the same behavior with
  `--fixed-width <#>` and can preserve source single newlines with
  `--ignore-incidental-newlines`; those two flags conflict.
- Use `darkmatter::style` for document `style:` frontmatter.
- Use `biscuit-terminal` components for rich terminal UI outside ordinary
  parsed Markdown rendering.

## Browser Test Contract

Darkmatter browser-rendering behavior is verified with real **headless**
Chrome through `biscuit-browser-harness`. Run it with `just test-browser` from
the `darkmatter` package area. Browser tests must not open or activate a visible
browser window, steal foreground focus, move the host pointer, or inject host
OS input. Use CDP-dispatched keyboard/pointer events, media emulation, live DOM
state, computed styles, and used geometry for interaction and layout evidence.

Keep host-UI and Level-3 tests out of
`darkmatter/lib/tests/browser_render.rs`. The shared Nextest selector
`test(/browser_/)` can match the `browser_render` integration-binary name, so a
test in that file may be selected by the Browser tier regardless of its
function prefix. Never add headed-launch switches, application activation,
`osascript`, `cliclick`, `xdotool`, `SendInput`, or equivalent desktop
automation to this browser suite or its harness path.

Only introduce a Darkmatter Level-3 browser-adjacent test when the requirement
explicitly concerns OS-to-browser event delivery rather than HTML/CSS/DOM
behavior. Put it in a dedicated integration-test binary whose filename does not
contain `browser_`, gate it with `RUN_LEVEL3=1`, and do not duplicate coverage
already provided by the headless Browser tier.

## Cleanup Pipeline

`darkmatter::markdown::cleanup` is the source-compatible public facade. Its
`cleanup_content_internal` orchestrator explicitly preserves the two-stage pass
order: event-stream passes (list-marker capture, emphasis placeholders, empty
fence language, table alignment, cmark serialization), then string passes
(emphasis restoration/unescaping, list spacing, blockquotes, list markers and
indentation, brackets, trailing-newline normalization).

Domain implementation lives under `markdown/cleanup/` in `emphasis.rs`,
`tables.rs`, `lists.rs`, `blockquote.rs`, `brackets.rs`, and `reflow.rs`.
Keep the facade paths stable and the pass calls explicit; do not introduce a
pass trait or implicit chaining.

## Responsibility Split

| Need | Owner |
|------|-------|
| CommonMark/GFM parsing | `darkmatter` |
| Compose pipeline, interpolation, shell directives, transclusion | `darkmatter` |
| Frontmatter extraction and Markdown-aware hashing | `darkmatter` / `md hash` |
| `$schema` / SimplifiedSchema validation, detection, compose integration | `darkmatter::markdown::schemas` |
| Expression-function signatures, descriptions, ordering, and examples | `darkmatter/docs/schemas/expression-functions.yaml` |
| `style:` frontmatter parsing and application | `darkmatter::style` |
| HTML and terminal Markdown renderers | `darkmatter` |
| Terminal capability detection, images, Mermaid, graph adapters | `biscuit-terminal` |
| Shared render tree and target-agnostic layout/style types | `renderable` |

## Expression-Function Catalog

`darkmatter/docs/schemas/expression-functions.yaml` is the authored authority
for expression-function descriptors. The library embeds and validates it once,
then projects the public `expression_function_descriptors()` values with stable
static lifetimes. Runtime handlers and aliases remain Rust-owned under
`markdown/compose/expression/functions/`; do not duplicate handler-free
descriptor metadata there.

## Trigger-Schema CLI Integration

Repository-scoped trigger schemas are opt-in for library hosts through
`ComposeOptions::with_trigger_schemas(true)` or
`DarkmatterSchemas::with_trigger_discovery(document_path, boundary)`. The `md`
CLI opts in by default for file-backed `compose` and `schema validate` commands,
using the discovered Git root as the inclusive boundary. Stdin, URL, and
in-memory sources do not discover roots. `--no-trigger-schemas` disables both
discovery and schema-root bare-name lookup, while path-qualified schema
references continue to resolve normally.

Use `md schema triggers <file>` to inspect roots, shadowed envelopes, matching
triggers, and arm-level defeat explanations. Its presentation-neutral library
model is `schemas::TriggerTrace`, built with `trace_registry`.

Trigger payloads layer in deterministic baseline → matching triggers → document
order and must resolve to merge-compatible object schemas. Match expressions
reuse SimplifiedSchema types but permit only pure constraints; `all`, `any`,
`none`, `min-match`, outer-OR arms, and case-sensitive boundary-relative
`$path` globs are supported. Keep dialect families disjoint with explicit
`none:` carve-outs, and keep every envelope separate from its payload.

## Demand-Driven Context Capture

Runtime `ctx.*` capture is grouped under `markdown/compose/context/capture/`. Domain modules own
their recognized key lists, while the facade preserves capture sequencing and always-on local
datetime values. Repository, filesystem, OS, hardware, and GPU discovery must continue to use
`sniff`. GPU population is independent of CPU/memory population, so a `ctx.gpu`-only request does
not require the hardware-summary probe.

The `Git` capture group owns `ctx.branch`, `ctx.worktree`, and `ctx.merge_conflicts`. Capture all
requested Git values from one trusted `sniff::filesystem::git::GitRepo` discovery. A failure in one
field must not discard its siblings: use `null` for unavailable scalar identity and an empty array
for unavailable conflicts while recording the field-specific partial-capture diagnostic.

`predict_conflicts(branch)` is a separate read-side expression surface. It calls
`sniff::filesystem::git::merge_conflicts_with_branch_at` with
`ResolutionContext::caller_dir()` so the launch-area repository is authoritative,
then projects Sniff's already sorted portable paths without reordering. It uses
committed local branch tips only; live index/worktree state does not affect it.
DMLS completion and hover consume the authored function/context catalogs but
must never evaluate this function or capture Git state while serving requests.

Remote expression functions use Sniff's one configured-remote resolver and
focused provider client. `branch_exists_on_remote` and `remote_vendor` remain
read-only; `pr`/`pr_list` and `cicd`/`cicd_list` return deterministic compact
Markdown projected from structured Sniff records. Network-capable calls are
deny-by-default, exact-host allowlisted, redirect-disabled, concurrency-bounded,
and memoized through the compose run's shared remote runtime. Frontmatter
interpolation, body interpolation, and `$()` evaluation attach that same
runtime; DMLS only consumes authored descriptors and never executes a call.
A focused provider failure (not-found, denied host, authentication, rate
limit, unsupported capability, incomplete domain, transport failure) is the
typed `ExpressionError::Provider` carrying a `ProviderFailureKind`; the
classification survives run-local memoization (cache slots store the typed
error, not rendered text) and is authoring-fatal on every expression
surface — composition aborts with the actionable message rather than
substituting an empty value or leaving an unevaluated `{{ … }}` behind.
Generic `ExpressionError::Other` stays non-fatal in lenient body mode.

The expression grammar supports immutable JSON-like array and object literals.
The authored catalog may use `enum(...)` only for return values; variants are
preserved in public descriptors and typed signatures, including an explicitly
quoted empty-string member.

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
- sub-spec #6 HR migration: inline `{ style: ... }` is parsed as a deprecated
  alias for `{ kind: ... }`; `apply_hr_style` wires `style.hr.*` onto
  `DarkmatterPage`
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
- `ReferenceGraph` is an **opaque, immutable, builder-produced artifact**. It
  has no public constructor and no public/`pub(crate)` data fields — only the
  crate's builders (`Markdown::reference_graph` / `transclusion_graph`, routed
  through the private `from_build`) produce one, and it carries private build
  provenance (document/source/mode/options identities + a visited-descendant
  dependency manifest). Downstream callers **inspect** via the read-only
  accessors `root()`, `nodes()`, `iter()` (root once, then children),
  `node_by_id()`, `node_count()`, `to_mermaid()`, `to_dot()`, and serialize via
  `graph.view(follow)` — never field access. Provenance is JSON-invisible and
  absent from `Debug`. `Markdown::validate_references_with_graph` verifies the
  prebuilt graph's provenance (document → source → `Full`-mode → `options.graph`)
  **and** re-reads every visited local descendant from disk before flattening; a
  mismatch or changed/missing/unreadable child is a hard
  `ReferenceError::ReferenceGraphMismatch` — it never silently rebuilds, so
  plain build-and-validate callers should use `validate_references`. Identity is
  clone-stable (a graph built from `options.clone()` still validates against the
  original — the Finding 18 reuse guard). See
  `darkmatter/features/2026-07-15-reference-graph/`.
- `darkmatter::style::CliStyleClaims`, `apply_cli_claims`, and the
  `*_style_overrides_from_claims` helpers are the single authority for CLI
  style precedence.
- `darkmatter::markdown::span` is the shared span vocabulary for span-aware
  parse products: `SourceSpan` (byte-offset `Range<usize>`), `Spanned<T>`,
  and the `line_of_offset` / `line_col_of_offset` helpers (all re-exported
  from the crate root). Added for DMLS (the Darkmatter Language Server).
- `darkmatter::markdown::extract_frontmatter_block` locates a document's
  frontmatter block with byte-accurate spans (`yaml_span`, `block_span`,
  `body_span`, delimiter lines, `yaml_base_line`) and preserved line endings
  — the span-aware companion to the internal `parse_frontmatter` (which it
  never changes).
- `darkmatter::markdown::generate_heading_slug` is the public slug authority
  (wraps the private TOC slug generator), and
  `darkmatter::markdown::extract_headings` returns `HeadingRecord`s (level,
  title, slug, `title_span`, `heading_span`, line) with duplicate slugs
  disambiguated `-1`/`-2` in document order (GitHub anchor semantics; the
  TOC itself keeps duplicate slugs identical).
- `darkmatter::markdown::extract_document_references` extracts one document's
  links/images/HTML references with byte-span provenance (`ReferenceRecord`s
  carrying `origin.span`/`origin.line`) **without composing** — no
  transclusion following, no shell, no network — so it is safe on every
  keystroke. This is the span-carrying, side-effect-free entry point DMLS's
  substrate indexer builds `references` edges from; the composing
  `Markdown::composed_references` is the wrong shape for a passive analyzer.
- **Spanned DSL parse products (added for DMLS, R-4 items 3–5).** These are the
  span-carrying, side-effect-free companions to the compose parsers, so a
  language server sees the same structure `md compose` does, plus byte spans:
  - `compose::expression::{parse_spanned, parse_condition_spanned, lex_spanned}`
    produce the `SpannedExpr`/`SpannedExprKind` AST (and `Spanned<Token>`
    stream). This is the **primary** parse — `parse`/`parse_condition` are
    exactly `parse_spanned(_)?.erase()`, so there is one grammar and
    `ParseError.position` is now a byte offset (not a token index).
  - `compose::directives_api::{scan_darkmatter_directives, scan_darkmatter_blocks}`
    → `ParsedDirective` (keyword/target/per-option `key`/`value` spans across all
    twelve directive keywords) and `DarkmatterBlock`/`BlockScanError` (the
    read-only view of the crate-private block-pair scanner). Shares the `Cursor`
    and code-region helpers; lenient (never errors on a malformed line).
  - `render_tree::disclosure_scan::scan_disclosures` → `DisclosureParse`
    (opener/summary/details/body/closer spans + opener style-token spans);
    structural malformation still raises `MarkdownError::MalformedDisclosure`.
  - `compose::parse_frontmatter_shell_value_spanned` → `FrontmatterShellValue`
    (a read-only mirror of a `$(...)` value: `$(`/`)`/inner spans,
    `::timeout`/`::no-cache` suffix spans, and a pipeline-with-action/token-spans
    or ternary-branch-spans body). No execution surface is exposed.

## DMLS (Darkmatter Language Server)

`darkmatter/dmls` is a third crate in the package area (binary `dmls`): an
LSP 3.17 server over stdio built on `lsp-server` + `lsp-types`, with the
`darkmatter` library as its semantic authority. It is wired into the area
`justfile` (`test`, `test-l2`, `lint`, `sanity`, `build`, `check`).

Phase 2 (skeleton) provides the full LSP lifecycle: position-encoding
negotiation (UTF-8/UTF-16, UTF-16 default), per-client `ClientProfile`
gates, a `line-index`-backed source map (CRLF + lone-CR aware, frontmatter
region projection), a full-sync open-document store with stale-change
rejection, layered `.dmls.toml` / `workspace/configuration` config (a
`didChangeConfiguration` reload is live: it recomputes wiki roots and rebuilds
the graph, reconciles workspace discovery against the new
`workspace.include`/`exclude` globs, and re-publishes diagnostics for open
documents without a restart), and an in-memory L2 LSP-session test fixture.

Phase 3 (workspace graph substrate) adds the single in-memory graph
(`dmls::graph`): one arena carrying every node kind and all eight edge kinds
(`references`, `includes`, `transcludes`, `uses_schema`, `uses_file`,
`uses_variable`, `defines_anchor`, `defines_symbol`) with a single reverse
index, a wiki basename `KeyIndex`, and a Markdown substrate indexer that
parses through the side-effect-free `darkmatter` span APIs into an immutable
`WorkspaceGraph` snapshot. The indexer materializes every committed edge
source: headings/slugs (`defines_anchor`/`defines_symbol`), links and
`[[wiki]]` links (`references`), `::file`/`::code` transclusions
(`transcludes`), the `$schema` file reference (`uses_schema`), file uses
(`uses_file` — images, frontmatter `file(...)` values, `style.page.stylesheet`
assets, and `::file-links`/`::toc-linking` paths), and `{{ }}` body
interpolation variables (`uses_variable` → same-document frontmatter-key node).
`file(...)`-typed detection at index time is the pure document-local slice —
the document's own inline `$schema`; extension-baseline `file(...)` properties
stay request-time in the frontmatter provider. `WorkspaceIndex` owns
invalidation (xxHash content-hash compare, generation-stamped snapshot swap —
AD-3); its dependent fan-out reaches documents over `transcludes` **and**
`uses_file` edges.
Workspace support (`dmls::workspace`) adds `ignore`/`globset` discovery
(symlinks not followed), a crossbeam worker-pool startup indexer with
`window/workDoneProgress`, dynamic `didChangeWatchedFiles` registration (with
event coalescing) plus a save-driven server-side rescan fallback for
watcher-less clients (`WatchMode::ServerRescan` — e.g. Neovim on Linux): a
`didSave` re-runs discovery and `WorkspaceIndex::reconcile_disk`
(content-hash compare) so an unopened file's create/change/delete reaches the
graph. And a `SharedSnapshot` handoff. The R-6 bench harness ships with it: `dmls --bench-index <dir>
[--json]` (per-stage timings, graph counts, peak RSS) and a deterministic
`dmls --gen-corpus <tier> <dir>` corpus generator.

Phase 4 (Layer-0 Markdown providers) turns the graph into a credible
plain-Markdown LSP through the AD-5 provider registry (`dmls::providers`): a
single `Provider` trait with one defaulted method per capability, an ordered
`ProviderRegistry` (substrate first; overlay providers append in Phases 7/9),
and a per-provider `catch_unwind` boundary with the design's merge policies
(union/dedup, first-non-empty hover, capped workspace symbols). The substrate
provider answers document/workspace symbols (heading hierarchy, subsequence
match), definition + eagerly-resolved document links, references + same-document
highlights (from the single reverse index), folding (frontmatter, sections,
fences, quotes, lists, tables — line-only, Helix-gated), hover (graph-sourced
link preview, no disk read), and completion (link path, `#`-anchor, and
fenced-language tokens with eager `textEdit`, Zed-safe). The push-diagnostics
pipeline (`dmls::diagnostics`) owns the stable code taxonomy
(`dm.links.broken_path` / `missing_anchor` / `duplicate_heading` under source
`darkmatter.links`), a `DiagnosticsScheduler`/`DiagnosticsPublisher` publishing
version-stamped `publishDiagnostics`, and `relatedInformation` linking
duplicate-heading twins.

Phase 5 (Layer-1 wiki links) adds the `[[wiki]]` rule set (R-8). A pure
`dmls::wiki` module holds the lexical scanner (six v1 forms, `\|`/`\#`/`\]`/`\\`
escapes, code-span/fence skipping, embed/`#^block` → `wiki.unsupported-syntax`),
logical-path canonicalization (NFC, final-segment Markdown-extension elision,
percent-decode-once-before-NFC), and the matcher/ranker (case-sensitive
everywhere; leading `/` = root-relative, `/`-containing = path-suffix, bare =
basename; same-directory → unique → ambiguous). Wiki links become
`NodeKind::WikiLink` nodes resolved at snapshot assembly (against wiki roots
from `wiki.wiki_root` / the workspace folders, threaded through
`WorkspaceGraph::build_with_roots`), emitting `references` edges so backlinks
surface through the existing reverse index. The `dmls::providers::wiki` provider
(merged after the substrate) answers definition, references, hover, document
links, and completion (`wiki.path_style` shortest/relative/root-relative,
`wiki.heading_completion_style`, never inserts `.md`), plus the full `wiki.*`
diagnostic taxonomy under source `darkmatter.wiki` (unresolved/ambiguous/
heading-missing/empty/unsupported, workspace-scope `wiki.portability-collision`,
and `wiki.invalid-percent-escape`). Rename participation is P10.

Phase 7 (Layer-2 frontmatter intelligence) adds the `dmls::overlay` module: a
position-aware `FrontmatterAst` (built from the Phase-1 `extract_frontmatter_block`
plus `rlsp-yaml-parser` in lossless mode — dotted-path / JSON-Pointer / byte-span
lookups; the parser type never leaves the module) and the effective-schema
assembler (`overlay::schema`: Darkmatter base baseline + extension baselines whose
globs match + document `$schema`, compose precedence, cached per document by
content hash). `OverlayState` keeps a per-document last-good tree so completion/hover
survive a mid-keystroke YAML error. The `dmls::providers::frontmatter` provider
(registered after wiki) answers, inside the frontmatter block only: schema-key
completion (required-marked, enum values, boolish scaffolds, `file(...)` paths,
`style.*` keys), hover (SimplifiedSchema type/constraints/default/`->` description,
plus `ctx.*` generated-key annotations), `$schema`/`file(...)` definition +
document links, nested-mapping folds, and config-gated frontmatter document
symbols (`symbols.frontmatter`). `dmls::diagnostics::frontmatter` emits the R-5
`dm.*` taxonomy under source `darkmatter.frontmatter` / `darkmatter.schema`
(`yaml_parse`, `invalid_schema_shape`, `prepare`, `type_mismatch`, `constraint`,
`missing_required`, `unknown_key`, `invalid_file_reference`, `pending_shell_value`)
plus `dm.style.*` for `style:` keys, ranged against the `FrontmatterAst` (missing
keys → parent mapping; unknown keys → the offending key; values → the value node)
with `relatedInformation` to a referenced schema file. Claudine activation is pure
config (a `[schema.extensions.claudine]` entry in `.dmls.toml` + `.claude/**`
globs) — zero Claudine-specific code. The substrate materializes `uses_schema`
edges for the `$schema` file reference and `uses_file` edges for inline-schema
`file(...)`-typed values (and images / style assets / directive paths); the
provider's `$schema`/`file(...)` navigation keeps its own request-time path
resolution because it also honors extension-baseline `file(...)` properties the
pure index-time slice does not see.

Schema-trigger integration extends that overlay without adding a second
assembler. DMLS selects the nearest containing workspace folder as the
discovery boundary, narrowed to a Git root when that root remains inside the
workspace. `OverlayState` caches transactionally loaded trigger registries and
the last-good frontmatter source: malformed YAML retains the previous
activation set, and a malformed trigger envelope retains the previous registry
while publishing a schema diagnostic on the envelope. Dynamic watchers include
`**/schemas/*.yaml` and `**/schemas/*.yml`; watcher-less clients rescan trigger
state on save. Effective assembly still routes through
`DarkmatterSchemas::with_trigger_registry`, preserving baseline → triggers →
document precedence and the library's dependency/origin accounting.

Phase 9 (Layer-3 Darkmatter DSL overlay) adds `dmls::overlay::{directives,
expressions, shell}` (thin passive wrappers over the Phase-8 library scanners —
`scan_darkmatter_directives`/`_blocks`, `scan_disclosures`, `ExpressionFinder` +
`parse_spanned`, and the shell-policy lookups) and the `DslProvider` (registered
last). Directives get `::`-prefix name completion, per-family option-key/enum
completion, semantics + resolved-target hover, block/disclosure folds, and the
`dm.directive.*` diagnostics (unknown, malformed option, unclosed block/unmatched
end, malformed disclosure). Transclusion gets document links + definition on
`::file`/`::code`/`prologue`/`epilogue`, `dm.transclusion.broken_path` (lexical
join + `stat`), `dm.transclusion.cycle` (DFS over the new `transcludes` graph
edges, ancestry in `relatedInformation`), and references ("who transcludes this
file"). Interpolation gets `{{ }}` completion (frontmatter keys, `ctx.*`,
functions), erased-parsed-form + frontmatter-backed static-value hover, variable→
frontmatter-key definition, and `dm.expression.{malformed,unknown_identifier}`.
Interpolation hover is catalog-enriched (modal-and-autocomplete Phase 2): an
explicitly `ctx.`-qualified root renders the shared
`overlay::expressions::format_ctx_hover_block` block (the same bytes as the
frontmatter `ctx_hover`) plus an interpolation-only compose-time note; a bare
identifier is always a frontmatter variable even when it matches a `ctx.*`
tail; and the deepest known `FunctionCall` under the cursor
(`overlay::expressions::function_call_at`) renders its typed signature +
description via `format_function_block`.
Interpolation completion is catalog-backed (modal-and-autocomplete Phase 3):
`ctx.*` items carry the rendered `display_type` in `detail` and the
description as eager Markdown `documentation`; function items keep the untyped
`signature` label and bare-name insertion but carry `typed_signature()` in
`detail` (incl. the `| error` suffix for fallible functions) plus eager
Markdown `documentation`. `.` is an advertised completion trigger
(`capabilities.rs`, alongside `/`, `(`, `#`), gated by the open-interpolation
guard in `overlay::expressions::completion_partial` so a period in prose
offers nothing.
Shell awareness (read-only) hovers `::shell` / frontmatter `$()` with an
approved/denied/unknown policy verdict and emits `dm.security.disallowed_command`
(source `darkmatter.security`) for built-in-blacklist denials. Fenced-code info
strings get `dm.fence.unknown_language` (via `LanguageGrammar::from_token`, with a
nearest-match suggestion). The overlay is **passive** — spec acceptance criterion
7 (`tests/no_side_effects.rs`): no directive, expression, or command is ever
executed. The graph substrate carries `NodeKind::TransclusionTarget` nodes and
`transcludes` edges (`::file`/`::code` → resolved `.md` document root), plus
`NodeKind::Interpolation` nodes and `uses_variable` edges (`{{ ident }}` → the
same-document top-level frontmatter-key node, else an `Unresolved(ident)` for
`ctx.*`/`env.*`/functions). The DSL provider keeps its request-time
interpolation definition/hover path; `prologue`/`epilogue` transclusion
resolution also stays request-time (no persistent edge).

Phase 10 (rename, code actions, formatting) adds the v1 editing surface as
standalone editing providers (not registry-merged — each has a single correct
answer): `providers::{rename, code_actions, formatting}` plus the shared
`providers::edits::EditBuilder` (lowers text edits + create-file ops to a
`WorkspaceEdit` in the richest form the client profile allows — `documentChanges`
with resource ops, else plain `changes`). **Rename** (`textDocument/prepareRename`
+ `rename`): heading-anchor rename rewrites the heading and every Markdown `#slug`
and wiki `#heading` reference that uniquely resolves to it, preserving each link's
spelling class (text-form → new title, slug-form → new slug); duplicate-text
headings refuse (prepare `None`, rename LSP error). Cross-document edits load each
affected buffer/disk file and build its own `SourceMap`. **File rename**
(`workspace/willRenameFiles`, gated on `supports_file_operations` — not Neovim)
runs the R-8 simulate-post-rename algorithm (rewrite wiki links unique before and
after, escalate to shortest unique suffix, atomically abort — `None` — on a
filesystem conflict or any non-unique replacement, never a partial rename) and
re-paths Markdown links relative to their document. **Code actions**
(`providers::code_actions`, diagnostic-driven, all eager — `resolveProvider` off):
create-missing-file/wiki-note (`CreateFile` + `# H1` template, Windows-invalid
filename guard), add-missing-schema-required-key (insertion from `FrontmatterAst`),
migrate-deprecated-`style:`-key, close-unclosed-`::block` — each gated by
`code_actions.categories`. **Formatting** (`textDocument/formatting`) is
byte-equivalent to the `md clean` cleanup sequence (`FormattingConfig` cleanup
variant + optional `reflow_to_width`, reassembled via `Markdown::as_string`) — spec
criterion 8. `wiki.ambiguous-after-rename` joins the diagnostic-code taxonomy.
Deviation: the Neovim file-rename code-action/command path and `ChangeAnnotation`s
are deferred.

Phase 11 (editors, packaging, hardening, closure) is documentation + packaging
only — no library or `dmls` source surface changed. Per-editor setup guides
(VS Code, Zed, Neovim, Helix) and a manual smoke checklist ship under
`darkmatter/dmls/docs/editors/`; the Zed extension is a thin WASM shim launching
the native binary, scaffolded at `darkmatter/dmls/zed-dmls/` (a `cdylib` on
`zed_extension_api` with PATH → settings → GitHub-release binary resolution,
workspace-**excluded** since it targets wasm32). `just dist` in the area
`justfile` builds a per-platform release archive
(`dmls-<version>-{macos-universal,linux-x86_64,linux-aarch64,windows-x86_64}`)
the extension downloads by. The release-build performance sign-off
(`phase11-bench-results.md`) confirms all 11 spec acceptance criteria and the
AD-2 verdict — full repo (3,141 files) ~1.9 s cold, `vault-5k` ~0.5 s, both
inside the R-6 budget — so the v1 in-memory-only model stands and no warm-start
cache is built. See `darkmatter/features/2026-07-04-dmls/plan.md`.

Semantic tokens (`darkmatter/features/2026-07-11-semantic-tokens/`) add a
**standalone** `providers::semantic_tokens` module (not registry-merged — one
correct answer, like Phase 10's editing providers): the frozen V1 wire legend
(standard types `macro`/`function`/`variable`/`property`/`string`/`number`/
`operator`; custom modifiers `interpolation`/`inert`/`directive`/`closer`/`wiki`/
`injected` then standard `defaultLibrary`/`readonly` — the legend trio
`TokenType`/`modifier`/`legend()` lives in the crate-level `semantic_legend`
leaf module so `capabilities` can advertise it without importing a provider,
keeping the module graph acyclic), a private `RawToken`
model, three pure `(text, body_base) → Vec<RawToken>` family emitters over the
existing passive scanners (F1 `overlay::expressions::{interpolations,literals}`
→ whole-span `macro.interpolation` `+inert`; F2 `scan_darkmatter_directives` +
`scan_disclosures` → `macro.directive` `+closer`, `string`/`property.directive`
targets/options, disclosure style split at source `=`; F4 `scan_wiki_links` →
`macro.wiki` frame + `string.wiki` segments), and one deterministic pipeline
(family precedence F1 > F4 > F2 with clipping, half-open range intersection,
per-line splitting via `SourceMap::split_span_by_line`, byte→position, sort/dedup,
overlap assertion, LSP relative-delta encode). `ScannedWikiLink` gained
`hash_span`/`pipe_span`/`alias_span` (source-derived, single-byte separators) so
the provider never reconstructs spans from decoded strings. `full` + `range`
handlers route through the router (standalone, missing-doc → `null`), gated on
`ClientProfile.supports_semantic_tokens`; `SemanticTokensConfig { enable }`
(default true) is the master switch and `wiki.enable = false` suppresses only F4;
`didChangeConfiguration` sends `workspace/semanticTokens/refresh` when
`supports_semantic_tokens_refresh`. Delta, F3 fine-grained expressions, F5
frontmatter, and F6 shell payloads are deferred. Per-editor styling recipes
(VS Code `semanticTokenColorCustomizations`, Neovim `@lsp.*` highlight links,
Zed `"semantic_tokens": "combined"` opt-in + theme override — Helix is
capability-gated off) live in `darkmatter/dmls/docs/editors/`.

## Expression Function Registrations

Expression callables are bound once in the owning module under
`markdown/compose/expression/functions/`. A runtime-only `FunctionBinding`
contains the canonical name, aliases, explicit evaluation mode, and optional
handler pointer (`None` for lazy operators). The cached registry joins bindings
to the authored catalog by canonical name with bidirectional parity and derives
dispatch arity eligibility from catalog parameter shapes. Add descriptive
metadata and overloads to the authored catalog; add executable behavior and
aliases to the owning Rust domain slice.

Public consumers must read the projected, handler-free catalog through
`expression_function_descriptors()`. The accessor is backed by one `LazyLock`
and is the sole public expression-function catalog API.

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
2. **Schema Validation** (pre-operation stage) - Validate frontmatter against `$schema` or `ComposeOptions::baseline_schema`. Runs after `--set` / `--state` overrides and frontmatter interpolation, but before shell expansion. **Coerces** schema-recognized top-level scalars to their declared types (default-on, e.g. the string `"true"` → real boolean) and writes the coerced values back into frontmatter, skipping `$(...)`-pending values. On validation success also **rewrites eager `file(eager)` values** to their resolved repo-relative path (the same projection `relative(value)`/`dirname(value)` produce); bare/lazy `file`, `string`, remote URLs, and pending values are left verbatim. Problems on fields still holding `$(...)` are deferred to downstream re-validation only when frontmatter shell expansion is enabled; when it is disabled they fail fast.
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
  default from top-level and list-item prose, removes source-only list
  continuation indentation, applies list/indent normalization, and can reflow
  complete logical prose blocks with `ComposeOptions::with_fixed_width(...)`.
  Reflowed list continuations retain their full list/blockquote container
  prefix. Programmatic callers can preserve source single newlines with
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
- `parse_standalone_schema_document` is the content-classification authority for
  standalone SimplifiedSchema YAML. It recognizes a pure document only when
  `$schema` is the sole top-level key, and recognizes a tagged document when
  `kind: schema` claims it with a `types` mapping. Its
  `StandaloneSchemaDocument` product retains the authoring path, source-aware
  candidate spans, and suggestion lint problems without performing I/O or
  composition. Mapping payloads support whole-file resolution and
  `Name@fileref` imports; pure sequence payloads support whole-file root unions
  only. Raw JSON Schema remains distinct and never produces this product.
- `DarkmatterSchemas` library API with baseline merging and LRU validator cache.
- Base frontmatter schema authored in `darkmatter/docs/schemas/darkmatter.yaml` — the source of truth for Darkmatter-owned frontmatter properties such as `ctx`, `hash`, `style`, and `replace`. The schema is exposed as a first-class library surface via `darkmatter_base_schema()` (returns `SimplifiedSchema`), `darkmatter_base_json_schema()` (returns the compiled Draft 2020-12 JSON Schema), and `ComposeOptions::with_darkmatter_baseline_schema()` (injects it into compose). Both accessors are also re-exported from the crate root.
- `md compose` injects the Darkmatter base schema by default. Use `--no-baseline-schema` or `DARKMATTER_NO_BASELINE_SCHEMA=1` for raw compose behavior, or `--baseline-schema PATH` to replace the default with a custom SimplifiedSchema YAML baseline. `md schema validate` keeps its explicit `--schema` / `BASELINE_SCHEMA` baseline contract.
- Always-on compose pipeline stage (after `--set`/`--state` and interpolation, before shell expansion) that also **coerces** schema-recognized scalars to their declared types and writes them back (default-on; `$(...)`-pending values are skipped and coerced at post-shell re-validation). On validation success, the same stage **rewrites eager `file(eager)` values** to their resolved repo-relative path via `EffectiveSchema::normalize_frontmatter` (bare/lazy `file`, `string`, remote URLs, and pending values are left verbatim; validation-only APIs stay read-only).
- `ComposeOptions::with_baseline_schema(...)` for programmatic baseline injection.
- Typed schema-language descriptor catalog (`schema_type_descriptors()`, `schema_constraint_descriptors()`, `schema_shape_descriptors()`, `inline_object_rule_descriptors()`, `coercion_rule_descriptors()`, `validation_behavior_descriptors()`) — the authoritative source for `md schema about` and the same surface library callers render their own reports from.
- Span-aware diagnostic shapes (added for DMLS, R-5). `ValidationProblem` carries, alongside the legacy fields, a fine-grained `code: ValidationProblemCode` (missing-required / type-mismatch / constraint-violation / unknown-key / invalid-file-reference), a parsed `instance_path: JsonPointer`, optional `schema_path`, `offending_property` (the undeclared key for `additionalProperties` failures), and `file_reference: Option<FileReferenceDiagnostic>` (invalid-syntax / resolution-failed / no-match, resolved-from context). These are purely additive — `message` and `md schema validate` output are byte-identical. `EffectiveSchema::origins: SchemaOriginMap` records each top-level property's provenance (document vs baseline vs referenced-file path). `EffectiveSchema::validate_with_options(_, _, ValidationOptions { pending_policy, excluded_keys })` mirrors the compose deferral rules as data — populating `ValidationReport.pending: Vec<PendingValue>` (`$(...)` → shell-expression, `{{ }}` → unresolved-template) and dropping deferred/excluded problems — **without executing anything**. The plain `validate` / `validate_with_positions` entry points are unchanged (empty `pending`). On the style side, `darkmatter::style::build_yaml_position_map` maps a dotted YAML key path to a raw-YAML-relative `StyleSpan`, `StyleWarning::source_span` is populated by `from_frontmatter` when `Frontmatter::raw_source()` is available, and `StyleParseError::source_span()` surfaces the first `Strict`-warning span.

- Composition primitives (schema-plus). SimplifiedSchema composes named types and dictionaries on top of the base grammar: `example(...)` attaches documentation-only example artifacts (validated at schema-load time, emitted as the `x-darkmatter-example` extension); `Name@file` / `Name@this` inline a named type's definition from another schema file's top-level `$schema:` entries (eager, bounded, cycle-checked — `SchemaError::ImportCycle`, dependency edges on `ResolvedSchema.imports`); pattern keys (`<string>` → `additionalProperties`, `<starting::P>` / `<ending::S>` / `<pattern::RE>` → `patternProperties` with literal-key precedence via negative-lookahead wrapping) plus `min-keys` / `max-keys` (authored postfix or via the reserved `$constraints` block key) type dictionaries; `suggest(...)` provides advisory, non-validating completion candidates for `string` and `number` properties (at most one per complete property definition; emitted as `x-darkmatter-suggest`, never `examples`; library-owned structured linting via `lint_suggestions()` → `SuggestionLintProblem`; DMLS completion via `suggestions_for_path()` → `SuggestionQuery`/`SuggestionItem`); and `yaml` / `json` are content-format string types (`format: darkmatter-yaml` / `darkmatter-json`) that accept a string or a coerced native value. A malformed example is `SchemaError::InvalidExample`. Schemas emitting a lookaround-bearing pattern validate on `jsonschema`'s `fancy-regex` engine per-schema; all others keep the ReDoS-safe linear engine.
- `literal(value)` and `expression` types. `literal(value)` (`SimplifiedType::Literal` + `Constraint::LiteralValue(serde_json::Value)`) validates a property against exactly one scalar: it compiles to JSON Schema `const`, types its bare value like YAML (bare `true`/`2` typed, quoted → string, bare `null` rejected), permits only `required` and an equal `default`, coerces non-string document values to the literal's scalar type, and is trigger-match-safe. `literal` arms with a shared discriminant key drive a presentation-neutral union-arm selector (`schemas::select_literal_discriminant_arm`, in `schemas/discriminant.rs`) reused by both library validation narrowing and DMLS. `expression` is the third content-format string type alongside `yaml`/`json` (`format: darkmatter-expression`, registered in `format.rs`): parse-only (a pure `parse_condition(value).is_ok()` check accepting either dialect), never evaluated, native bool/number values coerce to their string form, and it flips on DMLS expression completion/hover/`dm.expression.*` diagnostics inside frontmatter values. Neither type is ever inferred by `md schema detect`; `expression(condition)` is reserved and rejected in v1. See `darkmatter/features/2026-07-12-literal-expression/`.
- SimplifiedSchema semantic meta-types are canonical primitive keywords:
  `type-definition` denotes one complete `PropertyDef`, while `schema` denotes
  one complete `$schema` declaration. Both permit only `required`,
  `default(...)`, and `generated` at the definition level; semantic defaults
  are checked through the passive `parse_property_definition` /
  `parse_schema_declaration` authorities without I/O. Terminal import syntax
  still wins (`schema@file` and `type-definition@file` remain named imports),
  and `md schema detect` never infers either nominal type. Each type lowers to
  the `string | object | array` carrier domain plus its registered
  `x-darkmatter-*` keyword, whose validator delegates to the same passive
  parser. Trigger matching is likewise parse-based; validation, matching, and
  coercion perform no I/O and never rewrite the carrier value. See
  `darkmatter/features/2026-07-13-meta-schema/`.

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
  inline `{ style: ... }` is parsed as a deprecated alias for `{ kind: ... }`.
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