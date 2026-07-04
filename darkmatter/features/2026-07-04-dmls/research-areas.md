# DMLS Research Areas

## Purpose

This document lists areas that need more detailed attention before implementing the Darkmatter Language Server. Each area increases granularity in the design and lowers delivery risk by turning broad requirements into testable decisions.

## 1. Source Mapping and Position Semantics

Why it needs attention:

DMLS has to reconcile `pulldown-cmark` byte offsets, YAML parser ranges, Darkmatter directive spans, and LSP UTF-16 positions. Incorrect range conversion will undermine diagnostics, hover, completion, and edits.

Questions to answer:

- What text buffer type should DMLS use for incremental edits?
- What is the canonical internal position type?
- How are byte offsets, scalar values, Unicode graphemes, and LSP UTF-16 characters translated?
- How are ranges preserved through placeholder preprocessing, if preprocessing is still used?
- How are ranges represented for virtual composed output?

Expected output:

- A range mapping design note.
- Fixture tests covering ASCII, emoji, combining characters, CRLF, and Windows paths.
- A policy for when diagnostics may fall back to enclosing-line ranges.

## 2. Tolerant Parser Boundary

Why it needs attention:

The strategy assumes a preprocessor plus `pulldown-cmark`, but language servers must analyze invalid, incomplete text continuously. A batch compose parser may fail too early.

Questions to answer:

- Which authored constructs need a tolerant parser distinct from compose execution?
- Can existing Darkmatter parsers expose recoverable partial ASTs?
- Which directive parsers currently discard enough source detail to block LSP features?
- How should invalid nodes be represented?
- Should the parser catalog be generated from Darkmatter descriptor catalogs?

Expected output:

- A tolerant syntax model for directives, interpolation spans, and frontmatter keys.
- A list of library APIs that need range-preserving variants.
- Parser fixture corpus for incomplete syntax.

## 3. Directive Catalog and Grammar Authority

Why it needs attention:

The design documents contain some stale directive examples. The LSP must complete and validate the current Darkmatter syntax, not historical or speculative directives.

Questions to answer:

- What is the authoritative directive list for v1 DMLS?
- Which options apply to each directive?
- Which options are shared versus directive-specific?
- How are deprecated aliases represented?
- Can the directive catalog be exposed from the library rather than duplicated in DMLS?

Expected output:

- A versioned directive catalog.
- Completion and diagnostic rules generated from that catalog.
- Tests that fail when library directive support and DMLS completions drift.

## 4. Frontmatter Range Mapping

Why it needs attention:

Many high-value features depend on jumping to, renaming, validating, and explaining YAML frontmatter keys. Typical serde deserialization loses source positions.

Questions to answer:

- Which YAML parser should provide position-aware key and value ranges?
- How are duplicate keys handled?
- How are anchors, aliases, block scalars, quoted scalars, and comments handled?
- How are nested key paths represented for rename and definition?
- How does `$schema` validation map errors back to YAML ranges?

Expected output:

- A frontmatter source-map design.
- A compatibility matrix for YAML constructs DMLS supports in v1.
- Fixtures for nested objects, arrays, duplicate keys, block scalars, and invalid YAML.

## 5. Expression Intelligence

Why it needs attention:

Interpolation completion, hover, diagnostics, rename, and semantic tokens all depend on accurately parsing and resolving Darkmatter expressions without executing side effects.

Questions to answer:

- Which expression parser API should DMLS call?
- Can expression parsing return token-level ranges?
- What can be safely evaluated during hover?
- How are unknown roots distinguished from known-but-null values?
- How are `ctx.*`, `env.*`, `doc.*`, and function descriptors surfaced?
- How does rename handle nested key paths and dynamic indexing?

Expected output:

- Expression AST range contract.
- Safe evaluation policy for editor features.
- Completion strategy for roots, functions, operators, and nested properties.

## 6. Dependency Graph and Invalidation

Why it needs attention:

The graph is central to diagnostics, references, rename, preview, and file-operation handling. A naive full workspace recompute will not scale.

Questions to answer:

- What are the graph node and edge types?
- Are edges keyed by URI, canonical path, or `FileReference`?
- How are unsaved buffers represented when a transcluded file is open?
- What is the invalidation strategy for reverse dependencies?
- How are cycles detected incrementally?
- How are remote URLs represented?

Expected output:

- Dependency graph data model.
- Incremental invalidation algorithm.
- Cycle diagnostic format with related information.
- Multi-root workspace rules.

## 7. File Reference Resolution

Why it needs attention:

Darkmatter uses document-relative paths, launch-area fallback, magic `@` references, eager schema file references, standard Markdown links, and remote URLs. DMLS needs a consistent resolution model.

Questions to answer:

- Which surfaces use `biscuit-file::FileReference`?
- How are `@` references completed and resolved?
- What is the current document directory for unsaved buffers?
- Which references may cross workspace roots?
- How are symlinks, case-insensitive filesystems, and Windows drive letters handled?
- How are globs for `::file-links` indexed?

Expected output:

- File resolution matrix by syntax surface.
- Cross-platform path fixture suite.
- Completion ranking policy for path suggestions.

## 8. Shell and Remote Safety

Why it needs attention:

The LSP runs in an always-on editor context. Shell commands and remote fetches are valuable but high risk if triggered implicitly.

Questions to answer:

- Which LSP requests are guaranteed side-effect free?
- What approval UX is editor-neutral?
- Where are shell approvals persisted?
- How are command fingerprints computed?
- How are remote host allowlists configured?
- What metadata can hover/inlay show without executing?
- How are process cancellation and cleanup handled on Windows?

Expected output:

- Safety policy for every LSP feature.
- Approval persistence design.
- Redaction rules for shell, env, URL, diagnostics, hover, and logs.

## 9. Schema Integration

Why it needs attention:

Schema support is high value but should not block the first server milestone. The integration seam must be specific enough that schema features can land later without refactoring the server.

Questions to answer:

- Does schema diagnostics use `darkmatter.schema` or the same source as frontmatter diagnostics?
- How are schema completions merged with frontmatter and descriptor completions?
- Does DMLS invoke the existing SimplifiedSchema cache?
- How are schema errors mapped to frontmatter key ranges?
- How are eager `file` normalization behaviors represented without mutating editor buffers?

Expected output:

- Schema integration seam specification.
- Diagnostic and completion merge policy.
- Tests for `$schema`, missing required properties, enum completions, and file constraints.

## 10. Completion Ranking and Context Detection

Why it needs attention:

Completion quality depends on knowing the current syntactic context. Poor context detection produces noisy suggestions and makes the server feel unreliable.

Questions to answer:

- What context detector runs before completion?
- How are completions ranked across files, frontmatter, descriptors, and schema keys?
- What trigger characters should be advertised statically?
- Which completions require resolve-provider hydration?
- How are snippets handled across clients that support different snippet capabilities?

Expected output:

- Completion context grammar.
- Ranking rules.
- Client capability fallback behavior.

## 11. Hover and Inlay Value Policy

Why it needs attention:

Hover and inlay hints can expose secrets or imply stale values are current. They also risk becoming expensive if they compose documents too often.

Questions to answer:

- Which values are computed live versus read from cache?
- How are stale values labeled?
- How are secret patterns applied?
- What value size limits apply?
- Which hover fields are stable across editor adapters?

Expected output:

- Hover payload schema.
- Inlay category model.
- Redaction and truncation rules.

## 12. Formatting and Code Actions

Why it needs attention:

Formatting and code actions write back to user documents. They need predictable edit ranges and must avoid hidden compose side effects.

Questions to answer:

- Which cleanup operations are safe for format-on-save?
- Can range formatting be implemented without corrupting Markdown structure?
- Which source actions should be opt-in only?
- How should DMLS preview edits before applying them?
- Which migrations are current enough to include in v1?

Expected output:

- Formatting safety matrix.
- Code action catalog with edit examples.
- Fixture tests for idempotence.

## 13. Preview and Virtual Documents

Why it needs attention:

Compose preview is a major DMLS value proposition, but editor APIs differ. The core server should return editor-neutral results.

Questions to answer:

- Should preview be a virtual document URI, a custom command result, or both?
- How is `ComposeReport` represented?
- How are warnings displayed: diagnostics, virtual report, or status item?
- How are long previews canceled?
- How are shell and remote-disabled preview gaps represented?

Expected output:

- Preview command protocol.
- Virtual URI scheme design.
- Compose report projection design.

## 14. Capability Negotiation and Editor Adapters

Why it needs attention:

VS Code/Cursor, Neovim, and Zed support different LSP and extension capabilities. Static assumptions will cause uneven behavior.

Questions to answer:

- Which capabilities are advertised statically?
- Which capabilities are dynamically registered?
- What is the minimum Neovim configuration?
- How does the VS Code extension locate the binary?
- How does the Zed extension handle native binary discovery?
- What client capability fallbacks are required?

Expected output:

- Editor adapter requirements.
- Initialize capability matrix.
- Manual smoke-test instructions per editor.

## 15. Performance Budgets

Why it needs attention:

Language servers must feel responsive during typing. Full compose, graph rebuilds, and workspace scans can exceed acceptable latency.

Questions to answer:

- What latency budget applies to diagnostics after `didChange`?
- What latency budget applies to completion?
- Which work runs synchronously in request handlers?
- Which work is debounced or backgrounded?
- What cache keys are used for semantic indexes, graph edges, and compose preview?
- How are large workspaces indexed without blocking?

Expected output:

- Performance budget table.
- Benchmark corpus.
- Debounce and scheduling policy.

## 16. Cancellation and Progress

Why it needs attention:

LSP clients can cancel requests. DMLS must not keep stale shell, remote, graph, or compose work alive after cancellation.

Questions to answer:

- What cancellation primitive is used internally?
- Where are cancellation checks required?
- How are child processes killed cross-platform?
- How are partial results discarded or cached?
- Which long tasks report `window/workDoneProgress`?

Expected output:

- Cancellation design.
- Process cleanup design for macOS, Linux, and Windows.
- Tests for canceled preview and canceled workspace requests.

## 17. Observability and Debugging

Why it needs attention:

Editor-hosted servers are hard to debug. DMLS needs useful logs without leaking document secrets.

Questions to answer:

- What tracing targets are used?
- Where are logs written per platform?
- How does `darkmatter.trace.server` affect verbosity?
- Which request timings are recorded?
- How are paths, env values, shell commands, and URLs redacted?

Expected output:

- Logging and tracing policy.
- Diagnostic bundle command design.
- Redaction tests.

## 18. Test Harness

Why it needs attention:

The core behaviors are protocol-level and cross-file. Unit tests alone will not catch request sequencing, cancellation, or editor-client mismatches.

Questions to answer:

- What JSON-RPC fixture harness should DMLS use?
- How are workspace fixtures laid out?
- Which tests are unit, integration, and L2?
- How are Windows path cases tested from macOS CI or local development?
- Should we add a reusable LSP client harness crate?

Expected output:

- Test plan by capability.
- Fixture workspace structure.
- Initial smoke tests for initialize, didOpen, didChange, diagnostics, completion, hover, and definition.

## 19. Dependency Choices

Why it needs attention:

The strategy mentions `tower-lsp`, `ropey`, `dashmap`, and arena graph crates. Each should be validated against current maintenance status and monorepo fit before hardening the design.

Questions to answer:

- Is `tower-lsp` still the best LSP framework choice?
- Should DMLS use `ropey`, `crop`, or a simpler line-indexed string model?
- Does `dashmap` help, or does it complicate cancellation and consistency?
- Which graph crate, if any, should represent dependencies?
- Are new dependencies acceptable under `darkmatter/docs/dependencies.md`?

Expected output:

- Dependency decision record.
- Alternatives considered.
- Update plan for dependency documentation.

## 20. Drift Management

Why it needs attention:

DMLS will mirror Darkmatter syntax. If syntax catalogs are duplicated, the language server will drift from the library.

Questions to answer:

- Which catalogs should be exported by `darkmatter`?
- How are descriptor docs reused in completion and hover?
- What compile-time or test-time checks detect drift?
- Should DMLS have golden tests generated from library descriptor catalogs?

Expected output:

- Drift prevention strategy.
- Library API requests.
- Golden fixture generation plan.
