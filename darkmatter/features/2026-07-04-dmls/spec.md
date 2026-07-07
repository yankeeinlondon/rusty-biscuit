---
status: draft
inputs:
  - ../../dmls/design/design-strategy.md
  - ../../dmls/design/markdown-lsp.md
  - ../../dmls/design/wiki-style-links.md
  - ../../dmls/design/zed-lsp.md
  - ../../dmls/design/extending-iwes-lsp.md
related:
  - ./design.md
  - ./research-areas.md
---

# DMLS — Darkmatter Language Server Specification

**Status:** Draft for review. This document defines *what* DMLS is and what v1
must deliver. The *how* (architecture, component design, caching, indexing)
lives in [design.md](./design.md).

## Purpose

DMLS (`dmls`) is the editor-facing language server for Markdown documents,
with first-class intelligence for the Darkmatter DSL, SimplifiedSchema-driven
frontmatter, and wiki-style links. It serves two audiences at once:

1. **Ordinary Markdown authors.** DMLS must be a credible, standalone Markdown
   language server — good enough that choosing it costs nothing even for
   documents that use no Darkmatter features ("lose nothing" principle).
2. **Darkmatter and Claudine authors.** Humans (and agents) writing composed
   documents and agent prompts get guard rails no generic Markdown server can
   provide: schema-validated frontmatter, directive and interpolation
   intelligence, transclusion navigation, and safety-aware handling of shell
   and remote content.

The architecture is IWES-derived rather than greenfield: DMLS adopts IWES's
graph-centered Markdown language-server model (documents, headings, links, and
inclusions as one workspace graph) and layers a Darkmatter semantic overlay on
top. Darkmatter's existing library remains the semantic authority for parsing,
frontmatter, schemas, compose semantics, style, `LanguageGrammar`, cleanup,
and Markdown-aware hashing — DMLS never re-implements those rules.

## Naming and Placement

- Package: `darkmatter/dmls`, binary name `dmls`.
- Single crate initially; split into `dmls` (lib) + `dmls-cli` only if another
  crate needs a reusable library surface.
- A separate, thin Zed extension (`zed-dmls`, compiled to WASM) launches the
  native binary; it contains no language logic (see Editor Targets).
- Must build and run on macOS, Windows, and Linux.

## Editor Targets

Primary targets, in support order:

| Editor | Integration path | Notes |
|--------|-----------------|-------|
| VS Code | Standard LSP client (extension or generic LSP config) | Richest client capability surface; reference client for feature testing. |
| Zed | Thin `zed_extension_api` WASM extension launching native `dmls` | The server itself stays native. Extension resolves the binary via PATH, settings override, or GitHub release download — the proven IWE pattern. |
| Neovim | `nvim-lspconfig`-style registration | Broad LSP support; images/inlay rendering differ from GUI editors. |
| Helix | `languages.toml` registration | Conservative capability set; IWES already carries a Helix selection-range quirk — keep client-quirk handling isolated. |

DMLS speaks standard LSP 3.17 over stdio, so any conforming client (including
agents) is a secondary target for free. Capability-gated behavior handles
client differences; no feature may hard-require a bespoke client extension.

## Feature Model

DMLS features are organized in four layers. Lower layers must not depend on
higher ones; a plain-Markdown workspace exercises only Layers 0–1.

### Layer 0 — Markdown Baseline (CommonMark + GFM)

Everything a modern Markdown LSP is expected to do, powered by the IWES-style
workspace graph:

- **Navigation:** go-to-definition for links and anchors, find references,
  document symbols (heading outline), workspace symbols, backlinks (via
  references), document highlights.
- **Diagnostics:** broken local links and anchors, duplicate heading slugs,
  malformed structures the renderer would silently reinterpret (unclosed
  fences, table column mismatches).
- **Completion:** link path completion, heading/anchor completion, code fence
  language tokens (authority: `LanguageGrammar`).
- **Hover:** linked-document preview (title/first paragraph), resolved path
  and existence, heading slug.
- **Structure:** folding ranges (sections, fences, lists, tables, quotes,
  frontmatter block), document links (clickable paths/URLs), selection ranges.
- **Editing:** rename for files and heading anchors with workspace-wide
  reference updates; create-missing-file code action; document formatting via
  Darkmatter cleanup (never IWES graph rewriting).

GFM extensions in scope: tables, task lists, strikethrough, autolinks,
footnotes. Anchor slug algorithm defaults to GitHub-style; Darkmatter render
behavior is the authority where they differ.

### Layer 1 — Wiki-Style Links

First-class `[[target]]` support, aligned with the MediaWiki/Obsidian
convention family:

- **Forms:** `[[target]]`, `[[target|alias]]` (target-first, MediaWiki order),
  `[[target#heading]]`, `[[target#heading|alias]]`.
- **Resolution:** targets may be workspace-relative paths or bare basenames.
  The resolution universe is the LSP workspace folder(s); an optional
  `wiki_root` config value narrows or redirects it (a future Darkmatter
  vault concept can plug in as another override source). Basename resolution
  ranks: same directory → unique basename match across the workspace →
  ambiguity. Ambiguous targets produce multiple definition locations plus a
  diagnostic; rename refuses ambiguous targets.
- **Features:** completion (filenames, then headings after `#`), definition,
  references/backlinks, broken/unresolved-link diagnostics, hover preview,
  rename participation, "create missing note" code action.
- **Interop stance:** wiki-links are an authoring convenience and a knowledge-
  graph feature; Darkmatter compose/render behavior for wiki-links is defined
  separately and DMLS follows the library once that lands. Obsidian vault
  compatibility (aliases from frontmatter, `![[embed]]`, block refs `^id`) is
  explicitly post-v1.

### Layer 2 — Frontmatter Intelligence (SimplifiedSchema)

Frontmatter is a first-class language surface, not an opaque header. All
schema semantics come from `darkmatter::markdown::schemas`; DMLS adds source
positions and editor projections.

- **Schema sources**, in effective order (mirroring `md compose`):
  1. The Darkmatter base schema (`darkmatter/docs/schemas/darkmatter.yaml`,
     exposed as `darkmatter_base_schema()`) — injected as the default baseline.
  2. Configured baseline extensions (e.g. the Claudine schema) activated per
     workspace/glob via DMLS config.
  3. The document's own `$schema` (inline, file reference, or root union),
     which overrides baseline properties on conflict.
- **Diagnostics:** YAML parse failures; invalid `$schema` shape; schema
  preparation and validation errors mapped to precise key/value ranges;
  missing required keys (ranged on the frontmatter block or nearest parent);
  unknown keys under strict mode; deprecated style keys; invalid
  `file(...)` / `file(eager)` references; values that cannot be statically
  validated (pending `$(...)`) reported as informational, never executed.
- **Completion:** schema property keys (with required-marking), enum values,
  boolish scaffolds, file paths for `file(...)`-typed properties, `style.*`
  keys/values, nested object keys.
- **Hover:** the SimplifiedSchema type, constraints, default, and the `->`
  description text for the key under the cursor; generated-key annotation for
  `ctx.*` (Darkmatter-owned, read-only).
- **Navigation:** `$schema` file references → schema source; `file(...)`
  values → resolved file; document links on all file-valued entries.
- **Structure:** frontmatter block folding; frontmatter keys in document
  symbols (configurable).

YAML frontmatter only in v1. TOML/JSON frontmatter is out of scope until the
Darkmatter library itself supports them.

### Layer 3 — Darkmatter DSL

Static intelligence for the compose-pipeline surface, **without ever executing
it**. Sub-areas, roughly in low-hanging-fruit order:

1. **Directive syntax** — recognize the directive families (`::file`,
   `::code`, `::toc-linking`, `::file-links`, `::shell`, `::shell-block`,
   `::block`/`::end-block`, `::disclosure`/`::details`/`::end-disclosure`):
   directive-name completion, unknown-directive and unclosed-block
   diagnostics, folding for block directives, hover describing directive
   semantics.
2. **Transclusion navigation** — `::file`/`::code` targets plus frontmatter
   `prologue`/`epilogue`: document links, go-to-definition, broken-path
   diagnostics, transclusion-cycle diagnostics, references (which documents
   transclude this file).
3. **Interpolation and expressions** — `{{ ... }}` in body and frontmatter:
   parse via Darkmatter expression APIs; completion for frontmatter keys,
   `ctx.*` (catalog enumerated by the base schema), and `env.*`; hover showing
   the parsed form and the statically-resolved value when safe;
   unknown-variable and malformed-expression diagnostics; definition from an
   interpolation site to the frontmatter key.
4. **Shell awareness** — parse `::shell` / `::shell-block` / frontmatter
   `$(...)` syntax; hover shows the command and its policy verdict
   (approved/denied/unknown) without executing; diagnostics for
   policy-disallowed commands.
5. **Compose refactors and preview** — extract-section-to-`::file`, inline a
   transclusion, compose/render preview commands with explicit safety gates.

Items 1–3 (and the read-only part of 4) are v1 candidates; item 5 is post-v1.

## v1 Scope

v1 delivers: **all of Layer 0, Layer 1 basics, all of Layer 2, and the
low-hanging subset of Layer 3.**

| Capability | v1 | Notes |
|---|---|---|
| Markdown navigation, symbols, folding, document links | ✅ | Layer 0 |
| Broken link / anchor / heading diagnostics | ✅ | Layer 0 |
| Path, anchor, fence-language completion | ✅ | Layer 0 |
| File + heading rename | ✅ | refusal rules for unsafe cases |
| Formatting via Darkmatter cleanup | ✅ | whole-document; range formatting later |
| Wiki-link completion/definition/diagnostics/references | ✅ | Layer 1 basics |
| Obsidian aliases, embeds, block refs | ❌ | post-v1 |
| Frontmatter schema diagnostics/completion/hover/links | ✅ | Layer 2, YAML only |
| Claudine baseline schema activation | ✅ | via generic extension config |
| Directive syntax intelligence | ✅ | Layer 3.1 |
| Transclusion links/diagnostics/cycles | ✅ | Layer 3.2 |
| Interpolation completion/hover/diagnostics | ✅ | Layer 3.3, static only |
| Shell policy hover/diagnostics (read-only) | ✅ | no execution ever |
| Compose/render preview commands | ❌ | post-v1 |
| Extract/inline transclusion refactors | ❌ | post-v1 |
| Embedded-language delegation (fences → other LSPs) | ❌ | research frontier; post-v1 |
| Remote URL validation | ❌ | post-v1, opt-in, cache-backed |
| Semantic tokens | ❌ | evaluate after v1 (TextMate/Tree-sitter grammars may suffice) |
| Incremental text sync | ❌ | full sync first; revisit with measurements |

## Architecture Summary

Detail lives in [design.md](./design.md). The contract-level commitments:

1. **Protocol stack:** `lsp-server` + `lsp-types`, LSP 3.17, stdio, full
   document sync, push diagnostics initially. UTF-16 position encoding unless
   the client negotiates otherwise.
2. **One workspace graph:** documents, headings, Markdown links, wiki-links,
   transclusions, frontmatter file references, schema uses, and interpolation
   references are typed edges in a single graph. Every navigation, diagnostic,
   and refactor feature is a projection of that graph.
3. **Source-map discipline:** all providers convert positions through one
   source-map API (byte offsets ↔ UTF-16 LSP positions, frontmatter-relative →
   document ranges). No ad-hoc string slicing.
4. **Semantic authority:** Darkmatter library for Markdown/compose/schema/
   style/grammar semantics; `biscuit-file` for file-reference resolution.
   DMLS-local parsing exists only where the library lacks source geometry, and
   is hidden behind DMLS abstractions until it can move into the library.
5. **Provider registry:** each LSP capability is served by an ordered provider
   chain (generic Markdown first, Darkmatter overlay augmenting or overriding
   with more precise answers) with deterministic merge rules.

## Safety and Side-Effect Policy

DMLS runs inside editors and agents; accidental execution is a design failure.

Passive requests (diagnostics, completion, hover, definition, references,
symbols, folding, document links, formatting) MAY parse anything and resolve
local files. They MUST NOT:

- execute shell commands (including `$(...)` and `::shell` evaluation),
- fetch remote URLs,
- mutate files,
- run compose phases with side effects.

Shell and remote surfaces are *explained* statically: hover and diagnostics
report what compose *would* do and whether policy allows it. Side-effecting
behavior is reserved for explicit, post-v1 `workspace/executeCommand` commands
gated by Darkmatter policy.

## Extension Model (Claudine)

Claudine is the first consumer of a **generic** baseline-extension mechanism —
no Claudine special cases in DMLS core:

- An extension contributes: a SimplifiedSchema baseline
  (`darkmatter/docs/schemas/claudine.yaml` for Claudine), activation rules,
  and optionally extra hover documentation. Activation in v1 is **config +
  globs only** (e.g. `.claude/**`, `prompts/**` → Claudine schema): explicit
  and deterministic. Frontmatter-based auto-detection may be layered on
  post-v1.
- DMLS merges extension baselines with the Darkmatter base schema using the
  same precedence semantics as compose (document `$schema` wins on conflict).
- Claudine's lifecycle-event keys (`initialize`, `start`, `success`,
  `failure`, …), `prompt`, `sequence`, `loop`, timeouts, and guard settings
  then get diagnostics, completion, and hover with zero Claudine-specific
  server code.
- Behavior that cannot be expressed as a schema (e.g. deep lifecycle `stack`
  action validation) stays owned by Claudine and is out of DMLS v1 scope.

## Configuration

`DmlsConfig`, sourced from LSP `workspace/configuration` plus an optional repo
config file (exact file name/location decided in design.md):

- workspace/library root overrides; include/exclude globs
- wiki-link behavior (enable, path style, ambiguity policy)
- baseline schema extensions (name → schema path, activation globs)
- strict schema mode, strict style mode
- shell policy discovery path
- inlay hint and code-action category toggles
- formatting behavior (cleanup variant, fixed width)
- diagnostics debounce tuning

Config changes must be reloadable without server restart, invalidating only
affected indexes where possible.

## Packaging and Distribution

- Cross-platform release artifacts for macOS (universal), Linux (x86_64 +
  aarch64), and Windows (x86_64) so the Zed extension (and any installer) can
  download by platform.
- Logging goes to stderr / log file, never stdout (stdio is reserved for LSP
  framing). `RUST_LOG`-style filtering plus a `--log-file` flag.
- CLI flags: `--version`, `--log-level`, `--config <path>`, and a `--stdio`
  no-op flag for clients that pass it by convention.
- Editor setup documentation for all four target editors ships with v1.

## Acceptance Criteria (v1)

1. `dmls` completes the LSP initialize/shutdown lifecycle over stdio on macOS,
   Windows, and Linux.
2. Open documents are indexed from client buffer text, not stale disk state;
   watched-file changes update the graph for unopened files.
3. Source-map tests cover ASCII, multibyte, astral-plane characters, CRLF,
   frontmatter-relative ranges, and round trips.
4. Layer 0: navigation, symbols, document links, folding, and broken-link
   diagnostics work on a plain CommonMark+GFM workspace with zero Darkmatter
   config.
5. Layer 1: `[[target]]` completion, definition, references, and unresolved
   diagnostics work, including basename resolution and ambiguity handling.
6. Layer 2: frontmatter schema diagnostics point at precise key/value ranges;
   completion and hover reflect the effective schema (base + extensions +
   document `$schema`); the Claudine schema activates via config with no
   Claudine-specific code paths.
7. Layer 3: directive, transclusion, and interpolation features produce
   correct static results; a test proves passive requests spawn no processes
   and open no sockets.
8. Formatting is byte-equivalent to `Markdown::cleanup` output for the same
   options.
9. Rename refuses ambiguous/unsafe edits rather than applying partial changes.
10. `just test` and `just lint` pass in the package area; L2 integration tests
    cover an in-memory LSP session end to end.

## Out of Scope for v1

- Compose/render preview and any side-effecting commands
- Embedded-language server delegation and fence virtual documents
- Remote URL validation
- Obsidian vault parity (aliases, embeds, block references, daily notes)
- TOML/JSON frontmatter
- Semantic tokens, incremental sync, persistent on-disk index
- Extract/inline transclusion refactors, link-definition organizing
- Prose/style linting (markdownlint-class rules)

Each of these has a natural home in the layered model and none is precluded by
v1 architecture decisions; see design.md for the seams that keep them open.

## Resolved Decisions

Decided with Ken on 2026-07-06:

1. **Layer 3 v1 cut:** all four low-hanging sub-areas ship in v1 — directive
   syntax, transclusion navigation, interpolation, and read-only shell policy
   awareness. Compose refactors/preview remain post-v1.
2. **Wiki-link root:** LSP workspace folder(s), with an optional `wiki_root`
   config override. No new Darkmatter vault concept in v1.
3. **Claudine activation:** workspace config + glob patterns only in v1.
4. **Formatting:** ships in v1 as whole-document formatting backed by
   `Markdown::cleanup`; range formatting is post-v1.

## Open Questions

1. Binary/config naming details (`dmls` config file name and location) —
   settled in design.md alongside the configuration model.
