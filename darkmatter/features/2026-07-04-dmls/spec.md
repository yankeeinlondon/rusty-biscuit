# Darkmatter Language Server Draft Specification

## Status

Draft, based on the current Darkmatter LSP strategy documents and the existing `darkmatter/dmls` crate.

The requested source document, `darkmatter/dmls/design/design-strategy-2.md`, was not present in this worktree. This draft uses the closest available strategy material:

- `darkmatter/design/lsp-technical-strategy.md`
- `darkmatter/design/lsp-features.md`
- `darkmatter/docs/lsp/features.md`
- `darkmatter/features/_unscheduled/dmlsp/spec.md`

## Goal

DMLS is a Language Server Protocol server for Darkmatter documents: Markdown plus Darkmatter composition syntax. Its purpose is to make authoring safer and faster by providing diagnostics, completion, hover, navigation, document structure, semantic highlighting, formatting, and preview commands directly in editors.

The server should be useful before it is complete. The first production milestone should make common authoring mistakes visible without requiring a full compose on every keystroke.

## Non-Goals

- Replacing the `darkmatter` library compose pipeline.
- Executing shell commands or remote fetches silently from an editor.
- Building a general-purpose Markdown knowledge-management LSP.
- Implementing a polyglot embedded-language server.
- Making schema validation a hard dependency for the first DMLS milestone.

Schema-aware diagnostics and completion should plug into the architecture through explicit seams, but the first DMLS milestone should not block on completing every schema feature.

## Architecture

DMLS should be a Rust binary using `tower-lsp` as the protocol boundary. The implementation should reuse Darkmatter library code wherever the library exposes parse, compose, descriptor, validation, cleanup, and reference-resolution surfaces.

```text
LSP client
  |
  v
tower-lsp backend
  |
  +-- Document store / VFS
  |     - open unsaved buffers
  |     - incremental text changes
  |     - byte, line, and UTF-16 position translation
  |
  +-- Semantic index
  |     - frontmatter ranges
  |     - Markdown headings
  |     - Darkmatter directives
  |     - interpolations and expressions
  |     - file and URL references
  |
  +-- Dependency graph
  |     - transclusion edges
  |     - reverse references
  |     - invalidation propagation
  |     - cycle reporting
  |
  +-- Feature providers
        - diagnostics
        - completion
        - hover
        - definition/references
        - document symbols/folding
        - semantic tokens
        - code actions
        - formatting
        - execute commands
```

## Core Components

### Protocol Backend

The backend handles LSP lifecycle and request routing:

- `initialize`
- `initialized`
- `shutdown`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didSave`
- `textDocument/didClose`
- `workspace/didChangeConfiguration`
- `workspace/didChangeWatchedFiles`

The server should support workspace folders and multi-root workspaces, but cross-root references should be a configuration decision rather than an implicit behavior.

### Document Store

The document store owns open document text and versioned analysis state. It must support unsaved editor buffers; disk state is not authoritative for open files.

Required data:

- URI
- version
- current text
- line index
- byte offset to LSP range conversion
- LSP range to byte offset conversion
- last successful semantic index
- last diagnostics by source

The store may use `ropey` or another incremental text structure, but the conversion contract must be explicit because `pulldown-cmark` reports byte ranges while LSP clients use UTF-16 positions.

### Semantic Index

The semantic index is a tolerant, position-aware representation of the authored source. It should preserve invalid and incomplete syntax so editor features can still respond while a user is typing.

The index should capture:

- frontmatter block range and parsed YAML key ranges
- frontmatter scalar interpolation spans
- frontmatter `$(...)` shell expansion candidates
- Markdown headings and slugs
- standard Markdown links and images
- `{{ ... }}` expression spans in body text
- `::block` / `::end-block`
- `::shell`
- `::shell-block` / `::end-block`
- `::file`
- `::code`
- `::file-links`
- `::toc-linking`
- disclosure blocks
- horizontal-rule attribute blocks

This index should not require all operations to be semantically valid. Invalid nodes should keep ranges and parse errors.

### Dependency Graph

The dependency graph tracks document edges created by transclusion and frontmatter path fields:

- `::file`
- `::code`
- `::toc-linking`
- `::file-links`
- `prologue`
- `epilogue`
- standard Markdown links and images when useful for navigation

Graph updates should be incremental. When a document changes, dependent documents are invalidated without recomposing the entire workspace. Cycles must produce precise diagnostics with related information that shows the cycle chain.

### Compose Integration

DMLS should not duplicate the compose pipeline. It should call Darkmatter library surfaces for full compose preview, cleanup, schema validation, expression parsing, context descriptors, style descriptors, and path resolution where those APIs are available.

For keystroke-time diagnostics, DMLS should use lightweight analysis:

- parse frontmatter and record key ranges
- parse expressions without evaluating dangerous surfaces
- validate directive shape and option keys
- resolve local paths without network or shell execution
- maintain the dependency graph

Full compose should run only for explicit commands, code lenses, saves when configured, or low-frequency background refreshes.

## Security Model

DMLS runs inside an editor session, so it must default to preview-only behavior for dangerous operations.

Required defaults:

- no shell execution on open, change, save, hover, completion, diagnostics, semantic tokens, or inlay hints
- no remote URL fetch unless allowed by configuration and initiated by a safe command path
- no raw `env.*` value display unless allowed by configuration
- redact likely secrets in hover, diagnostics, logs, and virtual documents
- honor client cancellation for every long-running operation

Shell-related features should show command shape, policy verdict, approval state, timeout, and cached output when available. Running a command requires an explicit code action or command.

Remote URL features should show URL shape, host allowlist status, cache status, and last known fetch status. Fetching requires allowlist configuration.

## LSP Capability Surface

### P0: Useful Authoring Core

P0 should ship first.

Required capabilities:

- incremental document sync
- diagnostics
- completion for directive names, path references, and interpolation roots
- hover for directives, paths, expressions, and frontmatter keys
- definition for file references and frontmatter references
- document links for local file references and standard Markdown links
- document symbols for headings and directives
- folding ranges for headings, fenced code, frontmatter, and block directives
- semantic tokens for Darkmatter-specific syntax

P0 diagnostics:

- malformed frontmatter fence
- YAML frontmatter parse error
- malformed interpolation expression
- unknown expression root
- malformed directive line
- unknown directive
- unknown directive option
- unmatched `::block` / `::end-block`
- unmatched `::shell-block` / `::end-block`
- missing local transclusion path
- transclusion cycle
- transclusion max-depth violation
- remote host not allowed
- shell command requires approval

### P1: Correctness and Navigation

P1 should focus on reducing cross-file authoring mistakes.

Required capabilities:

- references for frontmatter keys and transclusion targets
- rename for frontmatter keys
- file rename handling for transclusion targets
- completion for nested frontmatter paths
- completion for `ctx.*`, `env.*`, and expression functions from descriptor catalogs
- completion for directive options and enum-like values
- code actions for broken paths, unknown options, missing block closers, and shell approval
- workspace symbols for headings and directives

### P2: Rich Feedback

P2 should improve the authoring loop without increasing default risk.

Required capabilities:

- inlay hints for resolved non-secret values
- code lenses for compose preview, cache state, reference count, and shell approval
- execute command for compose preview
- execute command for graph preview
- execute command for Mermaid preview
- formatting and range formatting through Darkmatter cleanup
- source actions for cleanup and syntax migration

### P3: Production Hardening

P3 should make the server robust across large workspaces and different editors.

Required capabilities:

- dynamic capability registration
- pull diagnostics where supported
- partial results for workspace-wide searches
- cancellation tokens through parse, graph, compose, remote, and shell paths
- progress reporting for long operations
- editor-specific wrappers for VS Code/Cursor, Neovim, and Zed
- sidecar embedded-language delegation only after the base server is stable

## Completion Requirements

Completion should be context-sensitive and cheap.

Trigger contexts:

- `::` at line start or after indentation: directive names
- whitespace after a directive name: directive path or option keys
- `=` after known options: option values
- `{{`: interpolation roots
- `.` inside an interpolation: nested properties and descriptor members
- path-like positions in frontmatter: local file suggestions
- `$schema`: schema file suggestions once schema integration is active

Completion sources:

- current document frontmatter
- inherited/effective frontmatter where cheaply available
- Darkmatter context descriptor catalog
- expression function descriptor catalog
- style descriptor catalog
- known directive catalog
- workspace file index
- schema descriptor catalog when enabled

## Hover Requirements

Hover should explain authored code without causing side effects.

Required hover outputs:

- expression: parsed expression, inferred value when safely known, lookup source, and errors
- frontmatter key: parsed YAML value, inferred type, schema hint when enabled
- directive: directive purpose, parsed options, and diagnostics
- local file path: resolved path, existence, size, and target kind
- transclusion: direct target, graph depth, and cycle risk
- shell: command preview, executable, policy verdict, timeout, and approval state
- remote URL: host policy, cache status, and fetch disabled state
- heading: computed slug and references from links, `exclude`, and `toc-linking`

## Diagnostics Requirements

All diagnostics must have stable, precise ranges. When the exact range is unknown, prefer the smallest enclosing construct over whole-document diagnostics.

Diagnostic sources should be separated:

- `darkmatter.syntax`
- `darkmatter.frontmatter`
- `darkmatter.expression`
- `darkmatter.directive`
- `darkmatter.graph`
- `darkmatter.security`
- `darkmatter.schema` when enabled

Diagnostics should use related information for:

- transclusion cycles
- duplicate headings or labels
- unmatched block pairs
- references to missing frontmatter keys when a likely key exists
- file path diagnostics with a suggested existing path

## Formatting Requirements

Formatting should delegate to Darkmatter cleanup APIs. It must not run shell commands, fetch remote URLs, or modify transcluded files.

Supported operations:

- whole-document cleanup
- range cleanup where the range can be cleaned safely
- organize frontmatter keys only if explicitly configured
- migrate deprecated Darkmatter syntax through source actions, not automatic formatting

## Execute Commands

Initial command set:

- `darkmatter.composePreview`
- `darkmatter.graph`
- `darkmatter.cleanup`
- `darkmatter.approveShellCommand`
- `darkmatter.invalidateCache`

Later command set:

- `darkmatter.renderTransclusion`
- `darkmatter.previewMermaid`
- `darkmatter.migrateSyntax`
- `darkmatter.toggleInlayCategory`

Command results should be editor-neutral. Prefer virtual documents and structured command payloads over editor-specific UI assumptions.

## Configuration

Minimum configuration:

- `darkmatter.trace.server`
- `darkmatter.workspace.allowCrossRootReferences`
- `darkmatter.diagnostics.enable`
- `darkmatter.inlay.enable`
- `darkmatter.inlay.categories`
- `darkmatter.completion.maxItems`
- `darkmatter.transclusion.maxDepth`
- `darkmatter.shell.policyPath`
- `darkmatter.shell.enableExecution`
- `darkmatter.shell.approvalStore`
- `darkmatter.remote.allowHosts`
- `darkmatter.remote.enableFetch`
- `darkmatter.env.allowList`
- `darkmatter.env.secretPatterns`
- `darkmatter.schema.enable`
- `darkmatter.format.enable`

## Packaging

The existing crate split should be preserved:

- `darkmatter` remains the library crate.
- `darkmatter-cli` remains the CLI crate.
- `dmls` is the language server binary crate.

Editor adapters should contain process-launch logic only. Semantic behavior belongs in the Rust language server.

Initial editor targets:

- VS Code and Cursor through a minimal TypeScript extension.
- Neovim through native LSP configuration.
- Zed through a small extension that locates or downloads the native binary.

## Testing

Testing should be layered.

Unit tests:

- range conversion
- tolerant directive parsing
- interpolation span parsing
- frontmatter range mapping
- dependency graph updates
- completion context detection

Integration tests:

- initialize and capability negotiation
- didOpen/didChange diagnostics
- completion request/response fixtures
- hover request/response fixtures
- definition and references across files
- cancellation of long-running commands

L2/editor-shape tests:

- stdio JSON-RPC smoke tests
- unsaved buffer path resolution
- file rename notifications
- multi-root workspace behavior
- Windows path handling

Cross-platform requirements:

- no Unix-only path assumptions
- no shell-specific execution assumptions
- use process-group cancellation strategies appropriate to macOS, Linux, and Windows
- normalize URI/path conversion through shared file-reference utilities where possible

## Implementation Sequence

1. Stand up `tower-lsp` server with initialize, shutdown, logging, and text sync.
2. Implement document store and byte/UTF-16 range conversion.
3. Build tolerant semantic indexing for frontmatter, headings, directives, paths, and interpolation spans.
4. Publish P0 diagnostics.
5. Add P0 completion, hover, document symbols, folding, document links, semantic tokens, and definition.
6. Add dependency graph invalidation and cycle diagnostics.
7. Add P1 references, rename, and file-operation handling.
8. Add P2 preview, formatting, inlay hints, and code lenses.
9. Harden cancellation, progress, dynamic registration, and editor adapters.

## Acceptance Criteria for P0

- DMLS starts as a normal stdio LSP server.
- A Markdown file with Darkmatter syntax can be opened and changed incrementally.
- Diagnostics update after edits without requiring file save.
- Directive, interpolation, and path completion work in unsaved buffers.
- Hover explains expressions, file targets, and shell safety without side effects.
- Missing transclusion targets and cycles are reported with precise ranges.
- Semantic tokens distinguish Darkmatter directives and interpolation spans.
- Document symbols include headings and Darkmatter directive regions.
- The implementation works on macOS, Windows, and Linux by design.
