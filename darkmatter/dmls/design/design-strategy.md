# DMLS Design Strategy 2: IWES-Based Architecture

## Status

Draft specification.

This document assumes the Darkmatter Language Server, `dmls`, will be based on
IWES/IWE rather than implemented as a greenfield Markdown LSP. The goal is not
to clone IWES unchanged. The goal is to adopt IWES's graph-centered Markdown
language-server architecture, preserve the parts that are already strong, and
add a Darkmatter semantic overlay for frontmatter schemas, composition
directives, interpolation, transclusion, rendering metadata, and Claudine
extensions.

## Research Inputs

Local notes used as input:

- `darkmatter/dmls/design/markdown-lsp.md`
- `darkmatter/dmls/design/wiki-style-links.md`
- `darkmatter/dmls/design/extending-iwes-lsp.md`
- `darkmatter/dmls/design/design-strategy-1.md`
- `darkmatter/docs/lsp/architecture.md`
- `darkmatter/docs/lsp/features.md`
- `darkmatter/docs/lsp/frontmatter.md`

Primary sources checked:

- IWE repository: <https://github.com/iwe-org/iwe>
- IWE documentation: <https://iwe.md/>
- IWE inclusion-link docs: <https://iwe.md/docs/concepts/inclusion-links/>
- IWE data-model docs: <https://iwe.md/docs/architecture/data-model/>
- Upstream source clone inspected at `/tmp/iwe-research`, especially
  `crates/iwes/src/router.rs`, `crates/iwes/src/router/server.rs`,
  `crates/liwe/src/model/key_index.rs`, `crates/liwe/src/parser.rs`, and
  `crates/liwe/src/model/config.rs`.
- VS Code Markdown Language Server announcement:
  <https://code.visualstudio.com/blogs/2022/08/16/markdown-language-server>
- LSP 3.17 specification:
  <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/>

## Key Decisions

1. DMLS should be an IWES-derived server, not a `tower-lsp` rewrite.
   Upstream IWES uses `lsp-server` plus `lsp-types`, not `tower-lsp`. Keeping
   this protocol stack lowers divergence from IWES and avoids inventing an async
   server model before Darkmatter has a feature-complete semantic layer.

2. DMLS should embed the IWES graph model in-process.
   Running `iwes` as a sidecar process would make Darkmatter features difficult
   to compose with core navigation, references, rename, code actions, and
   workspace symbols. DMLS needs one request router with one shared view of the
   workspace.

3. The first implementation should fork or vendor the IWES server/router shape
   into `dmls`, while depending on or porting the graph concepts from `liwe`.
   The current IWES server has useful public modules, but its `Server` fields and
   handlers are concrete rather than extension-oriented. A pure downstream
   wrapper would only reach the existing Markdown/graph features and would not
   cleanly add Darkmatter diagnostics, completions, frontmatter range mapping, or
   compose previews.

4. Long term, prefer upstreamable extension seams over a permanent hard fork.
   The target end-state is a small set of generic IWES hooks: parser providers,
   semantic analyzers, completion providers, code-action providers, and graph
   edge providers. DMLS can start with local adaptation, then push seams upstream
   when the shape is proven.

5. Darkmatter's existing library remains the semantic authority for Darkmatter
   syntax.
   DMLS should not duplicate Darkmatter's compose expression parser, schema
   resolver, file-reference resolution, `LanguageGrammar`, style frontmatter
   schema, or Markdown-aware hashing rules. Language-server features should call
   library APIs or add library APIs where source positions are missing.

6. The LSP should target LSP 3.17 first.
   That matches the stable surface used by most editors and covers the needed
   features: completion, hover, definition, references, document symbols,
   workspace symbols, document links, folding ranges, inlay hints, code actions,
   rename, formatting, execute command, diagnostics, file operations, and work
   done progress.

7. DMLS should initially use full document sync, matching IWES.
   Full sync keeps the first implementation simple and compatible with the IWES
   graph rebuild model. Incremental text sync can be added after DMLS has stable
   source maps and performance measurements.

## Why IWES Is the Right Base

IWES already solves the hard Markdown-as-knowledge-graph problems that DMLS
needs:

- It treats Markdown files as a graph, not independent text buffers.
- It distinguishes structural inclusion links from inline cross-references.
- It supports wiki-style links and Markdown links.
- It exposes LSP navigation, hover preview, completion, references, workspace
  symbols, document symbols, inlay hints, rename, formatting, folding ranges,
  and code actions.
- It has a CLI and MCP sibling architecture, which aligns with Darkmatter's
  agent-oriented use cases.

This is a better starting point than a generic Markdown language service because
Darkmatter's composition model is also graph-shaped. Transclusions, prologue and
epilogue references, `::toc-linking`, `::file-links`, schema `file(...)`
properties, and wiki-style links all create edges that should be visible to
navigation, diagnostics, backlinks, and refactoring.

## What Must Change From IWES

IWES is optimized for Markdown knowledge bases. DMLS is a Markdown knowledge
server plus a Darkmatter authoring server. These differences drive the
architecture:

- IWES parses Markdown/Djot into a `liwe` graph. DMLS needs a graph plus a
  Darkmatter source map that knows frontmatter, directives, expressions,
  compose operations, style frontmatter, and render hints.
- IWES has no first-class diagnostics path in the inspected router. DMLS needs
  diagnostics as a core pipeline output.
- IWES frontmatter handling is semantic enough for titles and query operations,
  but not position-aware enough for schema diagnostics, completion, rename, and
  document links inside nested YAML values.
- IWES link resolution is key-based and wiki-aware. DMLS must add Darkmatter
  file-reference resolution using `biscuit-file::FileReference` conventions and
  Darkmatter schema `file`, `file(eager)`, remote URL, and path-projection
  behavior.
- IWES formatting normalizes documents through its graph writer. DMLS formatting
  must use Darkmatter cleanup/normalization paths and must not execute unsafe
  compose operations.
- IWES code actions are extension-like for actions, but not enough for all
  Darkmatter semantic providers. DMLS needs provider registries across all major
  LSP capabilities.

## Architecture Overview

```text
+-----------------------------------------------------------------+
| dmls binary                                                     |
| lsp-server stdio loop + lsp-types capabilities                  |
+----------------+------------------------------------------------+
                 |
+----------------v------------------------------------------------+
| Router                                                          |
| request dispatch, notifications, diagnostics publishing,        |
| cancellation/progress, client quirks                            |
+----------------+------------------------------------------------+
                 |
+----------------v------------------------------------------------+
| Workspace State                                                 |
| open documents, filesystem snapshot, watch events, config,      |
| Darkmatter/IWES graph indexes, diagnostic cache                 |
+--------+-------------------------------+------------------------+
         |                               |
+--------v-------------------+   +-------v------------------------+
| IWES Graph Substrate       |   | Darkmatter Semantic Overlay     |
| documents, headings,       |   | frontmatter, schemas, DSL       |
| Markdown links, wiki       |   | directives, expressions, style, |
| links, inclusion edges,    |   | compose graph edges, render     |
| backlinks, symbols         |   | metadata, Claudine schemas      |
+--------+-------------------+   +-------+------------------------+
         |                               |
+--------v-------------------------------v------------------------+
| LSP Provider Registry                                           |
| completion, hover, definition, references, rename, document     |
| links, folding, inlay hints, code actions, formatting, commands |
+-----------------------------------------------------------------+
```

## Crate Shape

Recommended target shape:

```text
darkmatter/dmls/
  Cargo.toml
  src/
    main.rs
    lib.rs
    router.rs
    capabilities.rs
    workspace/
    graph/
    semantic/
    providers/
    diagnostics/
    source_map/
    config/
```

The `dmls` package can remain a single crate initially. Split into `dmls` and
`dmls-cli` only if the package area grows a reusable library surface that other
crates consume.

## Dependency Strategy

Use IWES-compatible protocol dependencies:

- `lsp-server`
- `lsp-types`
- `crossbeam-channel`
- `serde`
- `serde_json`
- `toml`
- `tracing`
- `anyhow` or repo-preferred error types

Use Darkmatter authorities:

- `darkmatter` for Markdown, frontmatter, schema, compose, style, cleanup,
  render metadata, `LanguageGrammar`, and Markdown-aware hashing.
- `biscuit-file` for file reference parsing/resolution.
- `biscuit-hash` only for non-Markdown hashing needs.

Use IWES graph concepts:

- Either depend on published `liwe` where possible, or locally adapt the minimal
  `liwe` graph/index model if the public API is not sufficient.
- Avoid depending on `iwes` as an opaque server for the first DMLS
  implementation; its concrete router prevents deep Darkmatter integration.

Potential new dependencies requiring prototype research:

- A position-aware YAML parser for frontmatter ranges. Existing Darkmatter
  frontmatter parsing is semantic, but DMLS needs exact ranges for nested keys
  and values.
- `ropey` or similar text storage if DMLS moves from full-sync rebuilds to
  incremental document updates.

## Workspace State Model

`WorkspaceState` should own:

- `base_path`
- active `DmlsConfig`
- open document text by URI
- file snapshot for workspace Markdown and Darkmatter files
- graph index
- Darkmatter semantic index
- diagnostics by URI
- client capability profile
- source maps by URI

The source of truth for workspace membership should follow repository
configuration and editor workspace folders, not directory-name assumptions.

Initial sync policy:

- On startup, load Markdown/Darkmatter files from the configured library root.
- On `textDocument/didOpen`, add or override the in-memory text.
- On `textDocument/didChange`, replace the document text and re-index that
  document.
- On `textDocument/didSave`, reconcile with disk.
- On file delete/rename/create notifications, update the graph and affected
  diagnostics.

The first version can rebuild affected indexes conservatively. Once behavior is
correct, optimize invalidation by dependency edge:

- Markdown link edge
- wiki-link edge
- inclusion edge
- transclusion edge
- frontmatter file-reference edge
- schema edge
- style asset edge
- remote URL edge, cache-backed and opt-in for network validation

## Graph Model

DMLS should maintain one merged graph view with edge kinds.

Core IWES-compatible node kinds:

- document
- heading/section
- paragraph
- list/list item
- table
- code block
- quote
- reference/link

Darkmatter overlay node kinds:

- frontmatter document
- frontmatter key path
- schema declaration
- style declaration
- interpolation expression
- condition expression
- directive
- directive argument
- transclusion target
- shell directive
- page block
- disclosure block
- horizontal-rule style attributes
- code fence info string

Core edge kinds:

- `includes`: structural inclusion link
- `references`: inline link or wiki-link
- `transcludes`: `::file`, `::code`, prologue, epilogue, and related directives
- `uses_schema`: `$schema` or configured baseline schema
- `uses_file`: frontmatter `file(...)`, image, stylesheet, or local asset
- `uses_variable`: interpolation to frontmatter/context/environment
- `defines_anchor`: heading or custom heading ID
- `defines_symbol`: heading, frontmatter key, directive, schema-defined key

This graph should power definition, references, backlinks, rename, diagnostics,
document links, workspace symbols, inlay hints, and code actions.

## Source Maps and Position Rules

DMLS must be strict about LSP positions:

- LSP positions are UTF-16 code units.
- Internal parser offsets may be byte offsets, line/column pairs, or Rust char
  indices depending on the subsystem.
- Every provider must convert through one source-map API rather than slicing
  strings ad hoc.

The IWES source already includes fixes and tests for multibyte and astral-plane
characters. DMLS should carry that discipline forward. The source-map module
should provide:

- URI plus document version identity
- byte offset to LSP position
- LSP position to byte offset
- byte range to LSP range
- frontmatter-relative range to document range
- virtual-document range to host-document range for future embedded language
  support

## Frontmatter Architecture

Frontmatter is a first-class language surface in DMLS.

Semantic authority:

- Darkmatter's `Markdown` and `markdown::schemas` modules determine the meaning
  of frontmatter, `$schema`, simplified schemas, coercion, `file(eager)`, and
  validation behavior.

Position authority:

- DMLS needs a position-aware frontmatter tree. This is not fully solved by the
  current semantic parser. The implementation should start with a narrow
  `FrontmatterAst` abstraction and hide the chosen parser behind it.

Required `FrontmatterAst` features:

- delimiter range
- key path ranges
- scalar value ranges
- sequence item ranges
- mapping/object ranges
- comments preserved if the parser supports them
- JSON Pointer or dotted-path lookup to source range
- original text slice for hover and code actions

Initial support should focus on YAML frontmatter because Darkmatter's active
frontmatter model is YAML-based. TOML/JSON frontmatter can be evaluated later if
needed.

Diagnostics:

- YAML parse failures
- invalid `$schema` shape
- schema preparation errors
- schema validation errors
- missing required keys
- unknown keys when strict schema mode is active
- deprecated style keys
- invalid style values
- invalid `file(eager)` references
- pending shell/interpolation values that cannot be validated in the current
  compose mode

Completion:

- schema keys
- enum values
- booleans and typed scalar scaffolds
- file paths for schema `file` fields
- style frontmatter keys and values
- Claudine lifecycle-event schemas

Navigation:

- `$schema` to schema file or schema definition
- `file(...)` values to resolved file
- frontmatter variable definitions from interpolation references
- style asset paths to file

Open research:

- Choose the YAML parser. The earlier notes mention `serde-saphyr` and
  `rlsp-yaml-parser`; both need local prototype validation for maintenance
  status, span fidelity, YAML feature coverage, and cross-platform behavior.
- Decide whether Darkmatter should expose position-aware frontmatter parsing in
  the library or whether it stays private to DMLS.

## Darkmatter DSL Architecture

DMLS should add a line-oriented and span-oriented Darkmatter syntax layer over
the Markdown graph.

Directive families:

- `::file`
- `::code`
- `::toc-linking`
- `::file-links`
- `::shell`
- `::shell-block`
- `::block` / `::end-block`
- `::disclosure`, `::details`, `::end-disclosure`
- prologue and epilogue from frontmatter
- deprecated or migration-only constructs where Darkmatter still accepts them

Expression families:

- frontmatter interpolation
- body interpolation
- condition expressions
- shell expansion tokens
- replacement maps
- path projection functions

The parser layer should not execute compose operations. It should parse and
resolve enough static structure to support editor feedback. Expensive or unsafe
work, especially shell execution and network fetches, must be opt-in through
explicit commands or preview flows.

## Provider Strategy

DMLS should implement each LSP capability through a provider registry. The
IWES-derived provider runs first for generic Markdown graph behavior, and the
Darkmatter provider augments or overrides where it has a more precise answer.

### Completion

Advertise trigger characters for:

- `[`
- `#`
- `/`
- `{`
- `.`
- `:`
- `=`
- `@`
- `$`
- space after directive introducers

Completion categories:

- Markdown link and wiki-link targets from the graph
- heading anchors
- file paths
- directive names
- directive option keys
- directive enum values
- interpolation identifiers
- `ctx.*` variables
- frontmatter key paths
- schema keys and values
- style keys and values
- code fence language tokens from `LanguageGrammar`
- tags, if Darkmatter adopts body/frontmatter tag indexing

### Hover

Hover should show concise Markdown content:

- linked document preview, inherited from IWES
- resolved file path and existence
- heading slug and duplicate status
- frontmatter schema type and description
- interpolation parsed form and current static resolution when safe
- directive semantics and resolved target
- shell policy verdict without executing the command
- code fence grammar and renderer language

### Definition and Document Links

Definition should route:

- Markdown links to file or heading
- wiki-links to resolved key
- frontmatter file paths to file
- `$schema` to schema source
- interpolation variables to frontmatter keys when statically resolvable
- directive paths to files
- `exclude="Heading"` to the referenced heading in the target file

`textDocument/documentLink` should be implemented early. It gives clients a
standard way to make paths clickable without overloading go-to-definition.

### References and Backlinks

References should include:

- inclusion links
- inline Markdown links
- wiki-links
- transclusion directives
- frontmatter file references
- interpolation uses of a frontmatter key
- schema uses

Backlinks and reference counts should be derived from the same reverse index.
Inlay hints can expose counts, but the index must be debounced to avoid churn.

### Rename

Initial rename targets:

- file/document key, including Markdown links, wiki-links, and transclusions
- heading anchor, including `#anchor`, wiki heading links, and
  directive-heading references
- frontmatter key path, including interpolation and `set.NAME` references
- link reference labels, if the graph preserves them with enough fidelity

Rename must refuse or require confirmation for:

- ambiguous wiki-link targets
- generated anchors with duplicates
- reserved roots such as `ctx` and `env`
- file-system conflicts
- edits requiring unsupported client resource operations

Use `WorkspaceEdit.documentChanges` with file resource operations when the
client supports them.

### Diagnostics

DMLS should publish diagnostics after indexing each affected document.

Diagnostic sources:

- `darkmatter.markdown`
- `darkmatter.links`
- `darkmatter.frontmatter`
- `darkmatter.schema`
- `darkmatter.compose`
- `darkmatter.style`
- `darkmatter.security`
- `darkmatter.embedded`

Initial high-value diagnostics:

- broken Markdown links and anchors
- unresolved wiki-links
- duplicate heading slugs
- frontmatter parse/schema errors
- invalid Darkmatter directive name
- malformed directive options
- broken transclusion path
- transclusion cycle
- unknown interpolation variable
- malformed interpolation expression
- shell directive not allowed by policy
- unknown code fence language

Pull diagnostics from LSP 3.17 are worth researching later, but push
diagnostics are simpler for the first implementation and match broad editor
support.

### Code Actions

Preserve IWES-style action providers and extend them with Darkmatter actions:

- create missing linked file
- create missing transclusion target
- convert Markdown link to wiki-link or back
- extract section to `::file`
- inline `::file` content
- add missing schema-required key
- remove deprecated style key
- migrate top-level `hr` to `style.hr`
- close an unclosed directive block
- add shell approval entry
- format table
- normalize headings
- run Darkmatter cleanup
- preview composed document

Code actions that execute commands or write new files should carry clear
`WorkspaceEdit` operations and avoid side effects during `codeAction` discovery.
Use `codeAction/resolve` for expensive edit construction, following the IWES
pattern.

### Formatting

Formatting should use Darkmatter cleanup and normalization, not IWES graph
writing. It must not execute shell commands, fetch remote content, or perform
unsafe compose expansion.

Initial formatting commands:

- whole-document cleanup
- range cleanup where source maps can preserve context
- list/table cleanup
- heading normalization if explicitly requested

Do not run `cargo fmt` or repo formatting tooling as part of LSP behavior; this
is document formatting only.

### Folding and Symbols

Combine IWES folding with Darkmatter constructs:

- headings
- lists
- tables
- quotes
- code fences
- frontmatter block
- directive blocks
- disclosure blocks
- shell blocks

Document symbols should include:

- heading hierarchy
- frontmatter keys, configurable
- directive blocks
- transclusion sites
- schema declarations

Workspace symbols should index:

- document titles
- headings
- frontmatter `title` and aliases
- tags, if enabled
- named directives where meaningful

### Inlay Hints

Inlay hints should be opt-in by category:

- backlink/reference counts
- parent/inclusion contexts, inherited from IWES
- resolved transclusion target
- interpolation static value, truncated
- condition result when statically evaluable
- schema type beside frontmatter keys
- composed heading level after transclusion, where relevant

### Execute Commands

Commands should cover actions that are not clean LSP edits:

- `darkmatter.composePreview`
- `darkmatter.renderPreview`
- `darkmatter.openGraph`
- `darkmatter.validateWorkspace`
- `darkmatter.refreshIndex`
- `darkmatter.approveShellCommand`
- `darkmatter.previewMermaid`

These commands must be designed so VS Code, Neovim, Zed, Helix, and agents can
either use them directly or degrade gracefully.

## Client Targets

Primary:

- VS Code
- Neovim
- Zed

Secondary:

- Helix
- agents and terminal editors that speak LSP

Client differences to account for:

- IWES already has a Helix-specific selection-range quirk; DMLS should keep
  client quirk handling isolated.
- Not every client supports resource operations in `WorkspaceEdit`.
- Zed extension packaging may impose binary or WASM constraints; this needs
  dedicated research before promising a WASM server.
- Hover, inlay hint, code lens, and command support differ materially by editor.

## Configuration

DMLS should read a Darkmatter config file if one exists, while also supporting
editor-provided configuration.

Proposed top-level areas:

```toml
[dmls]
diagnostics = "push"

[dmls.markdown]
wiki_links = true
wiki_link_path = "preserve" # preserve | full | short
anchor_style = "github"

[dmls.darkmatter]
enable_compose_semantics = true
enable_render_semantics = true

[dmls.frontmatter]
strict_schema = false
baseline_schema = ""

[dmls.shell]
diagnose_policy = true
execute_in_preview = false

[dmls.inlay_hints]
references = true
parents = true
interpolation_values = false
schema_types = false
```

The exact config home should be finalized after checking existing Darkmatter
configuration conventions. If DMLS also supports IWES `.iwe/config.toml`, the
interaction must be explicit and deterministic.

## Safety Model

DMLS must never execute unsafe document behavior during passive LSP requests.

Passive requests:

- completion
- hover
- definition
- references
- document symbols
- folding
- diagnostics
- formatting
- inlay hints

These may parse, validate, and resolve local files. They must not run shell
commands or perform network fetches.

Explicit commands may execute or fetch only when configured and approved:

- compose preview
- render preview
- shell approval flow
- remote URL validation
- Mermaid rendering if it invokes external tools

Diagnostics should expose policy state so the author understands what will
happen at compose time.

## Performance Targets

Initial budgets:

- single-document parse and semantic index: under 50 ms for typical documents
- completion/hover/definition: under 20 ms from warm indexes
- workspace initial index: progress-reported above 500 ms
- large workspace graph rebuild: incremental or background after the first
  correctness milestone

Design constraints:

- no network checks on keystroke
- no shell execution on keystroke
- debounce diagnostics
- cache schema resolution
- cache file indexes
- use dependency edges for invalidation once the basic model is stable

## Implementation Phases

### Phase 0: Upstream IWES Spike

Success criteria:

- Build a local DMLS branch that starts from the IWES `lsp-server` router shape.
- Load a workspace and answer Markdown link completion, hover, definition,
  references, workspace symbols, document symbols, folding, and rename.
- Keep tests small and source-level, using IWES fixtures as patterns.

Key output:

- Decision on dependency versus vendored adaptation of `liwe`.

### Phase 1: Darkmatter Frontmatter and Diagnostics

Success criteria:

- Parse YAML frontmatter with source ranges.
- Validate `$schema` using Darkmatter schema semantics.
- Publish diagnostics for frontmatter parse/schema failures.
- Provide schema-aware hover and completion.
- Navigate from file-valued frontmatter fields to files.

Key output:

- Decision on position-aware YAML parser.
- New Darkmatter library APIs if needed.

### Phase 2: Darkmatter Directive Index

Success criteria:

- Parse directive lines and block pairs.
- Index transclusion edges.
- Diagnose broken transclusion paths and cycles.
- Provide directive completion, hover, definition, document links, and folding.

### Phase 3: Interpolation and Expression Intelligence

Success criteria:

- Parse interpolation expressions through Darkmatter expression parser APIs.
- Complete frontmatter, `ctx`, and `env` identifiers.
- Navigate from interpolation variable to frontmatter key.
- Diagnose unknown identifiers and malformed expressions.
- Rename frontmatter keys across interpolation sites.

### Phase 4: Refactors and Preview Commands

Success criteria:

- Implement create-file, extract, inline, cleanup, migration, and preview code
  actions.
- Add `darkmatter.composePreview` and render-preview commands with explicit
  safety controls.
- Add richer inlay hints and code lenses where client support is good.

### Phase 5: Embedded Languages and Advanced Markdown

Success criteria:

- Parse JSON/YAML/TOML fences for syntax diagnostics.
- Complete code fence language tokens from `LanguageGrammar`.
- Prototype virtual-document delegation for one embedded language before
  generalizing.

## Open Research

1. YAML range parser.
   Need a maintained parser with precise ranges for nested YAML keys/values.
   The earlier notes name `serde-saphyr` and `rlsp-yaml-parser`, but DMLS should
   prototype both against real Darkmatter frontmatter before choosing.

2. IWES upstream extension seams.
   Determine whether to contribute provider traits and parser hooks upstream or
   keep a local fork. This depends on how much Darkmatter-specific behavior can
   be expressed generically.

3. `liwe` dependency surface.
   Decide whether the published `liwe` API can support DMLS graph extensions or
   whether a local graph module is cleaner.

4. Zed packaging.
   Confirm whether the DMLS server must compile to WASM for the intended Zed
   integration, or whether an extension-managed native binary is acceptable.

5. Anchor algorithm.
   Decide the default anchor slug style and whether it is configurable by
   project. GitHub-style anchors are the likely default, but Darkmatter render
   behavior should be the authority.

6. Remote URL validation.
   Decide if this belongs in DMLS at all. If it does, it must be background,
   cache-backed, rate-limited, and disabled by default.

7. Persistent index.
   Large workspaces may need a persisted graph and schema cache. Do not add this
   until in-memory correctness and invalidation semantics are proven.

8. Diagnostics model.
   Push diagnostics are the initial choice. Pull diagnostics should be revisited
   after editor compatibility testing.

9. Claudine schema extension model.
   The mechanism should be generic enough for Claudine lifecycle-event schemas
   without hard-coding Claudine into core Darkmatter semantics.

## Non-Goals for the First Milestone

- Running shell commands during diagnostics or hover.
- Full embedded language server delegation.
- Remote URL validation by default.
- A custom graph visualization UI.
- A new Markdown parser that replaces Darkmatter or IWES parsing wholesale.
- A speculative plugin system before the provider seams are known.

## Final Recommendation

Build DMLS as an IWES-derived, graph-first language server using
`lsp-server`/`lsp-types`, with Darkmatter providing the semantic authority for
frontmatter, schemas, composition, style, rendering metadata, and hashing. Start
with a local adaptation of the IWES router and graph integration because the
current upstream server is not extension-oriented enough for Darkmatter's needs.
Keep the adaptation disciplined: preserve IWES behavior where it already works,
add provider seams locally, and upstream general seams only after DMLS proves
them against real Darkmatter features.

The central architecture bet is that Markdown, wiki links, Darkmatter
transclusions, schema file references, and frontmatter interpolation are all
edges in one workspace graph. Once DMLS has that graph with reliable source
ranges, the LSP features become straightforward projections of the same model
rather than independent string-processing features.
