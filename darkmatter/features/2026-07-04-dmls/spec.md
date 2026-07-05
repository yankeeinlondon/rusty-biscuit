---
status: draft
source: ./design-strategy.md
---

# Darkmatter Language Server Specification

**Status:** Draft. This specification turns the IWES-based direction in `./design-strategy.md` into an implementation-oriented DMLS contract.

## Context

Darkmatter Language Server, `dmls`, is the editor-facing language server for
Markdown documents that use Darkmatter composition, schema, style, rendering,
and Claudine extensions. It must be useful as a strong Markdown language server
for ordinary Markdown, while adding Darkmatter-specific intelligence where
generic Markdown servers cannot help.

The selected architecture is IWES-derived rather than a greenfield LSP rewrite.
IWES already treats Markdown workspaces as graphs, which matches Darkmatter's
transclusion, schema, file-reference, interpolation, and render metadata model.
DMLS should preserve that graph-centered base and add a Darkmatter semantic
overlay driven by the existing Darkmatter library.

## Goals

1. Provide editor diagnostics, navigation, completion, hover, symbols, document
   links, references, rename, code actions, folding, inlay hints, and formatting
   for Markdown plus Darkmatter syntax.
2. Reuse IWES graph concepts and the `lsp-server` plus `lsp-types` protocol
   stack to reduce language-server risk.
3. Use Darkmatter library APIs as the semantic authority for Markdown parsing,
   frontmatter, schemas, composition, style frontmatter, language grammar,
   cleanup, and Markdown-aware hashing.
4. Maintain one shared workspace view for generic Markdown links, wiki-links,
   inclusion links, Darkmatter transclusions, frontmatter file references,
   schemas, interpolation references, and style assets.
5. Keep unsafe or expensive compose behavior out of automatic analysis. Shell
   execution, remote fetches, and full compose previews must be explicit,
   policy-controlled commands.
6. Support LSP 3.17 first, with capability-gated behavior for client-specific
   support in VS Code, Neovim, Helix, Zed, and other standard LSP clients.

## Non-Goals

1. DMLS is not a full runtime replacement for `md compose`.
2. DMLS must not execute shell directives or fetch remote content during ordinary
   diagnostics, completion, hover, definition, references, symbols, or document
   link requests.
3. DMLS should not duplicate Darkmatter schema, compose, style, cleanup, or
   `LanguageGrammar` rules in server-local code.
4. DMLS should not start with incremental text synchronization unless source-map
   and invalidation performance measurements prove full sync is insufficient.
5. DMLS should not depend on `iwes` as an opaque sidecar server. Darkmatter
   semantics need direct access to the shared graph, source maps, diagnostics,
   and provider registry.

## Architecture

The initial server should be a single `dmls` crate under `darkmatter/dmls`.
It can split into library and CLI crates later if another package needs to
consume a reusable DMLS library surface.

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

The server is organized around five layers:

1. **Protocol router**: owns the `lsp-server` stdio loop, request dispatch,
   notification handling, cancellation, progress reporting, and capability
   gates.
2. **Workspace state**: owns open documents, file snapshots, configuration,
   source maps, client profile, graph index, semantic index, and diagnostics.
3. **IWES graph substrate**: provides the base Markdown graph for documents,
   headings, links, wiki-links, inclusion links, backlinks, symbols, folding,
   and generic Markdown refactors.
4. **Darkmatter semantic overlay**: indexes frontmatter, schema declarations,
   schema validation data, directives, interpolation expressions, transclusion
   targets, style declarations, render metadata, shell policy verdicts, and
   Claudine extensions.
5. **Provider registry**: merges IWES-derived providers and Darkmatter providers
   for each LSP capability.

Provider ordering should be deterministic: the IWES-derived provider supplies
generic Markdown answers first, then Darkmatter providers augment or override
when they can provide a more precise semantic result.

## Protocol Contract

DMLS targets LSP 3.17 and should use `lsp-server` and `lsp-types` initially.
The server should advertise only capabilities that are implemented and should
gate optional features on client capabilities.

Initial synchronization policy:

- Use full document sync.
- Treat the client buffer as authoritative for open files.
- Re-index a changed document after `textDocument/didChange`.
- Reconcile with disk on `textDocument/didSave`.
- Update graph and diagnostics on create, delete, and rename file-operation
  notifications when the client supports them.

The source-map module is mandatory. All providers must convert through it rather
than slicing strings directly. It must support byte offsets, LSP positions,
LSP ranges, frontmatter-relative ranges, document versions, URI identity, and
future virtual-document range projection.

## Workspace State

`WorkspaceState` must own:

- workspace folders and resolved base path
- active `DmlsConfig`
- normalized URI-to-path mappings
- open document text and versions
- disk snapshot for indexed Markdown and Darkmatter files
- IWES-compatible graph index
- Darkmatter semantic index
- source maps by URI and document version
- diagnostics by URI
- client capability profile

Workspace membership must come from LSP workspace folders and DMLS
configuration. It must not infer package identity from directory names.

## Graph Model

DMLS maintains one merged graph with typed nodes and typed edges.

Core Markdown/IWES-compatible nodes:

- document
- heading or section
- paragraph
- list and list item
- table
- code block
- quote
- link or reference

Darkmatter overlay nodes:

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
- horizontal-rule attribute block
- code fence info string

Required edge kinds:

- `includes`: IWES structural inclusion edge
- `references`: Markdown link, wiki-link, reference-style link, or inline anchor
- `transcludes`: `::file`, `::code`, `::toc-linking`, `::file-links`,
  prologue, epilogue, and related Darkmatter content edges
- `uses_schema`: `$schema` or configured baseline schema
- `uses_file`: frontmatter `file(...)`, image, stylesheet, local asset, or
  directive path
- `uses_variable`: interpolation or condition reference to frontmatter, `ctx`,
  `env`, or directive-set state
- `defines_anchor`: heading or explicit custom anchor
- `defines_symbol`: heading, frontmatter key, directive, schema-defined key, or
  named block where supported

The same graph should power definition, references, backlinks, rename,
diagnostics, document links, workspace symbols, inlay hints, and code actions.

## Darkmatter Semantic Overlay

The overlay parser must understand the author-written Darkmatter surface without
executing the compose pipeline.

Required frontmatter surfaces:

- YAML delimiter range
- `$schema`
- `replace`
- `prologue`
- `epilogue`
- `interpolate_code_blocks`
- `style.*`
- schema-defined arbitrary author keys
- Claudine lifecycle-event schemas
- `file`, `file(eager)`, remote URL, and path-projection values where exposed by
  schema semantics

Required body surfaces:

- `::file`
- `::code`
- `::toc-linking`
- `::file-links`
- `::shell`
- `::shell-block`
- `::block` / `::end-block`
- `::disclosure`, `::details`, `::end-disclosure`
- interpolation expressions
- condition expressions
- directive option keys and values
- horizontal-rule attribute blocks
- code fence info strings

Darkmatter semantics must call existing library APIs or add new library APIs
when needed. Server-local parsing is acceptable only for source geometry that
the library does not yet expose, and it should be hidden behind stable DMLS
abstractions.

## Frontmatter Requirements

Frontmatter is a first-class language surface. DMLS needs a position-aware
`FrontmatterAst` abstraction independent of the chosen YAML parser.

The abstraction must provide:

- delimiter range
- key path ranges
- scalar value ranges
- sequence item ranges
- mapping/object ranges
- comments when the selected parser preserves them
- JSON Pointer or dotted-path lookup to source range
- original text slices for hover and code actions
- conversion from frontmatter-relative ranges to host-document LSP ranges

Darkmatter library behavior remains the semantic authority for schema detection,
schema validation, coercion, `file(eager)`, and strict-style behavior.

Initial frontmatter support should be YAML only. TOML and JSON frontmatter can
be considered after YAML frontmatter delivers diagnostics, completion, hover,
definition, document links, and code actions with precise source ranges.

## Diagnostics

DMLS should publish push diagnostics after indexing each affected document.
Pull diagnostics can be researched later.

Diagnostic sources:

- `darkmatter.markdown`
- `darkmatter.links`
- `darkmatter.frontmatter`
- `darkmatter.schema`
- `darkmatter.compose`
- `darkmatter.style`
- `darkmatter.security`
- `darkmatter.embedded`

Initial diagnostics:

- broken Markdown links and anchors
- unresolved wiki-links
- duplicate heading slugs
- YAML frontmatter parse failures
- invalid `$schema` value
- schema preparation and validation errors
- missing required schema keys
- unknown keys when strict schema mode is active
- deprecated style keys
- invalid style values
- invalid `file(eager)` and file-reference values
- unknown or malformed Darkmatter directives
- malformed directive options
- broken transclusion paths
- transclusion cycles
- unknown interpolation variables
- malformed interpolation or condition expressions
- shell directives disallowed by policy
- unknown code fence language tokens according to `LanguageGrammar`

Diagnostics must include stable diagnostic codes and related information where a
second location explains the issue, such as a duplicate heading, cycle ancestry,
or schema source.

## Completion

Completion should be context-sensitive and should advertise trigger characters
for `[`, `#`, `/`, `{`, `.`, `:`, `=`, `@`, `$`, and space after directive
introducers.

Completion categories:

- Markdown link targets
- wiki-link targets
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
- Claudine lifecycle-event keys and values

Completion items should use snippets only when the client supports snippets.
Expensive documentation should be deferred through `completionItem/resolve` when
client support is available.

## Hover

Hover content should be concise Markdown. It should cover:

- linked document preview inherited from the IWES graph provider
- resolved path, existence, and link kind
- heading slug and duplicate status
- frontmatter schema type, description, default, and enum values
- interpolation parsed form and static resolution when safe
- directive semantics and resolved target
- shell policy verdict without command execution
- code fence grammar and renderer language
- style value interpretation for supported style keys

## Definition, Document Links, And References

Definition must route:

- Markdown links to file or heading
- wiki-links to resolved graph key
- frontmatter file paths to files
- `$schema` to schema file or schema definition
- interpolation variables to frontmatter keys when statically resolvable
- directive paths to files
- directive heading references, including `exclude="Heading"` where supported

`textDocument/documentLink` should be implemented early for Markdown links,
frontmatter file references, schema references, style assets, and Darkmatter
directive paths.

References must include:

- inclusion links
- inline Markdown links
- reference-style links
- wiki-links
- transclusion directives
- frontmatter file references
- interpolation uses of a frontmatter key
- schema uses

Backlinks and reference counts should be derived from the same reverse index.

## Rename

Initial rename targets:

- file or document key, including Markdown links, wiki-links, and transclusions
- heading anchor, including `#anchor`, wiki heading links, and directive-heading
  references
- frontmatter key path, including interpolation and `set.NAME` references
- link reference labels when the graph preserves enough source fidelity

Rename must refuse or require confirmation for:

- ambiguous wiki-link targets
- generated anchors with duplicates
- reserved roots such as `ctx` and `env`
- filesystem conflicts
- edits requiring file resource operations when the client does not support them

When supported, file renames should use `WorkspaceEdit.documentChanges` with
resource operations.

## Code Actions And Commands

Code actions should avoid side effects during discovery. Expensive edits should
be built through `codeAction/resolve` when the client supports it.

Initial quick fixes and refactors:

- create missing linked file
- create missing transclusion target
- convert Markdown link to wiki-link or back
- add missing schema-required key
- remove deprecated style key
- migrate top-level `hr` to `style.hr`
- close an unclosed directive block
- add or open a shell approval entry
- format table
- normalize headings
- run Darkmatter cleanup
- preview composed document through an explicit command
- extract section to `::file`
- inline `::file` content

Commands that can execute shell, fetch network content, or write files must be
explicit and must follow Darkmatter approval and policy semantics.

## Formatting

Formatting should use Darkmatter cleanup and normalization paths, not IWES graph
writing. It must not execute shell commands, fetch remote content, or perform
unsafe compose expansion.

Initial formatting surface:

- whole-document cleanup
- range cleanup where source maps can preserve context
- list and table cleanup
- heading normalization as an explicit source action or command

## Folding, Symbols, And Inlay Hints

Folding should combine IWES folding with Darkmatter constructs:

- headings
- lists
- tables
- quotes
- code fences
- frontmatter block
- directive blocks
- disclosure blocks
- shell blocks

Document symbols should expose headings as the main spine and may include
frontmatter, directives, code fences, and schema-defined symbols where useful.

Workspace symbols should include document titles, headings, schema-visible
frontmatter symbols, and important Darkmatter directives.

Inlay hints should be opt-in by category and may show:

- backlink/reference counts
- resolved transclusion target paths
- interpolation static value previews
- condition evaluation previews when safe
- shell policy verdicts
- code fence grammar names

## Security And Side-Effect Policy

DMLS must distinguish static analysis from execution.

Automatic analysis may:

- parse shell directive syntax
- resolve command tokens enough to explain policy
- report whether a command is approved, denied, or unknown
- show cached metadata if already present and valid

Automatic analysis must not:

- execute shell commands
- fetch remote URLs
- mutate files
- compose a document with unsafe operations

Explicit commands may perform side-effecting work only after the user invokes
the command and the relevant Darkmatter policy allows it.

## Configuration

`DmlsConfig` should support:

- workspace root and library root overrides
- file include and exclude globs
- Markdown dialect and wiki-link behavior
- schema defaults by glob
- strict schema and strict style modes
- shell policy path
- remote URL validation policy
- inlay hint categories
- code action categories
- formatting behavior
- embedded language support tiers
- Claudine extension enablement and schema locations

Configuration must be reloadable and should invalidate only affected indexes
where possible.

## Dependency Strategy

Preferred LSP stack:

- `lsp-server`
- `lsp-types`
- `crossbeam-channel`
- `serde`
- `serde_json`
- `toml`
- `tracing`
- repo-preferred error handling

Preferred semantic authorities:

- `darkmatter`
- `biscuit-file`
- `biscuit-hash` only for non-Markdown hashing needs

IWES integration:

- Depend on published `liwe` where its public API is sufficient.
- Locally adapt minimal graph/index concepts when published APIs are too
  concrete.
- Avoid relying on the upstream `iwes` server as an opaque process.

Potential dependencies that require prototype research:

- a position-aware YAML parser
- `ropey` or equivalent only if incremental sync becomes necessary

Any new dependency must update `darkmatter/docs/dependencies.md`.

## Implementation Phases

### Phase 1: Protocol And Workspace Skeleton

- Start `dmls` over stdio.
- Implement initialize, shutdown, full sync, workspace folder handling, logging,
  and baseline configuration.
- Add source-map module with UTF-16 conversion tests.
- Publish no-op or minimal diagnostics for open Markdown documents.

### Phase 2: IWES Graph Substrate

- Port or adapt the IWES-compatible graph and router shape.
- Index documents, headings, Markdown links, wiki-links, inclusion links, and
  symbols.
- Implement generic Markdown document symbols, workspace symbols, definition,
  references, document links, folding, and broken-link diagnostics.

### Phase 3: Frontmatter And Schema Intelligence

- Add `FrontmatterAst`.
- Connect Darkmatter schema detection and validation.
- Map schema diagnostics to precise frontmatter ranges.
- Add schema-aware completion, hover, document links, and code actions.

### Phase 4: Darkmatter DSL Overlay

- Parse directives, interpolation, conditions, transclusion targets, shell
  directives, disclosure blocks, horizontal-rule attributes, and code fence info
  strings.
- Add diagnostics, completion, hover, definition, references, document links,
  symbols, folding, and inlay hints for static Darkmatter semantics.

### Phase 5: Refactors, Formatting, And Commands

- Implement rename for files, headings, frontmatter key paths, and link labels.
- Implement code actions and command resolution.
- Wire Darkmatter cleanup into formatting.
- Add explicit compose preview and shell-approval workflows.

### Phase 6: Performance And Editor Hardening

- Measure full-sync indexing on realistic workspaces.
- Add dependency-edge invalidation where it materially improves latency.
- Add client-specific capability profiles and integration tests for VS Code,
  Neovim, Helix, and Zed where feasible.
- Revisit incremental text sync only with source-map tests and measured need.

## Acceptance Criteria

1. DMLS starts over stdio, completes the LSP initialize lifecycle, and shuts down
   cleanly on macOS, Windows, and Linux.
2. Open Markdown documents are indexed from client text, not stale disk state.
3. Source-map tests cover ASCII, multibyte Unicode, astral-plane characters, CRLF
   line endings, frontmatter-relative ranges, and range round trips.
4. Generic Markdown navigation, document links, symbols, folding, and broken-link
   diagnostics work for local Markdown links and wiki-links.
5. YAML frontmatter parse and schema diagnostics point to precise key or value
   ranges.
6. Darkmatter directives and interpolation expressions produce static
   diagnostics, completion, hover, and document links without executing compose
   operations.
7. Shell and remote URL features never run during automatic LSP requests.
8. Formatting uses Darkmatter cleanup behavior and does not run unsafe compose
   phases.
9. Rename refuses ambiguous or unsupported edits rather than applying partial or
   unsafe changes.
10. The implementation has focused unit tests for source maps, frontmatter range
    mapping, graph edge extraction, diagnostic mapping, and provider merging.
11. `just test` in the `darkmatter` package area passes.
