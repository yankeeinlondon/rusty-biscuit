---
prompt: |-
    We intend to use the [IWES](https://lib.rs/crates/iwes#:~:text=LSP%20capabilities) LSP as a foundation of Darkmatter's LSP.

    Your task is to do deep research into the IWES LSP:

    - use the 'lsp' skill while doing this research
    - provide a full description of the functional reach of IWES
    - describe it's interaction with Crossbeam and Rayon
    - describe it's relationship with tower-lsp
    - describe all of the "out of the box" features that IWES provides
        - How much of the CommonMark standard is covered?
        - Is GFM covered?
    - describe what mechanism a developer should take to _extend_ the functionality that IWES provides
    - describe what mechanism a developer should take to _disable_ functionality that IWES provides out of the box
last_updated: 2026-05-02
---
# IWES LSP Research

## Summary

IWES is the Language Server Protocol server in the IWE project. It is not a general Markdown compiler and it is not a `tower-lsp`-based framework. It is a concrete Markdown knowledge-graph language server built around the `liwe` core crate, `lsp-server`, `lsp-types`, `crossbeam-channel`, and Rayon.

As of `iwes` `0.1.0`, published May 2, 2026, IWES provides a substantial out-of-the-box Markdown authoring environment for linked notes: navigation, backlinks, hover previews, link completion, document/workspace symbols, rename refactoring, code actions, formatting, inlay hints, folding ranges, and configurable text transformations. Its strongest architectural value for Darkmatter is not generic Markdown syntax coverage; it is the graph model: documents are parsed into nodes, linked documents become graph edges, inclusion links create parent-child structure, and operations like extract, inline, rename, and search are expressed as graph transformations.

For Darkmatter, IWES is best treated as a reference implementation and possible foundation for graph-aware Markdown LSP behavior, but not as a drop-in implementation of the Darkmatter language. Its CommonMark support is mediated through `pulldown-cmark`, but IWES only enables a small set of Markdown extensions and only maps a subset of parsed Markdown events into its internal model. It does not provide full GitHub Flavored Markdown support, diagnostics, semantic tokens, frontmatter schema validation, or Darkmatter-specific syntax awareness out of the box.

Primary sources checked:

- `iwes` `0.1.0` docs.rs crate page: <https://docs.rs/crate/iwes/0.1.0>
- IWE repository: <https://github.com/iwe-org/iwe>
- IWE README: <https://github.com/iwe-org/iwe#editor-integration-lsp>
- IWE configuration docs: <https://github.com/iwe-org/iwe/blob/master/docs/configuration.md>
- IWE data model docs: <https://github.com/iwe-org/iwe/blob/master/docs/data-model.md>
- IWE inclusion links docs: <https://github.com/iwe-org/iwe/blob/master/docs/inclusion-links.md>
- IWES source, especially `crates/iwes/src/main.rs`, `crates/iwes/src/router.rs`, `crates/iwes/src/router/server.rs`, and `crates/liwe/src/markdown/reader.rs`.

## Functional Reach

IWES is the editor-facing LSP server for IWE's Markdown memory system. The larger IWE project has three main surfaces:

- `iwe`: CLI for graph operations on Markdown files.
- `iwes`: LSP server for editors.
- `iwec`: MCP server for AI-agent access to the same graph operations.
- `liwe`: shared core library containing the graph model, parser integration, configuration, query machinery, and operations.

IWES uses `liwe` to build an in-memory graph of Markdown documents. IWE documentation describes the model as a directed graph where every header, paragraph, list, list item, code block, table, and reference becomes a node. Nodes are stored in an arena: `Vec<GraphNode>` plus `Vec<Line>`, with `NodeId` as an array index. This gives O(1) node lookup and cache-friendly traversal.

The graph distinguishes two important link classes:

- Inclusion links: Markdown or wiki links placed alone in a paragraph. These create structural parent-child relationships between documents.
- Inline references: links inside prose. These create reference/backlink edges but do not define hierarchy.

This distinction is central to IWES. It allows the server to support knowledge-base operations that ordinary Markdown LSPs usually do not handle: extract a section into a new note, inline a referenced note back into its parent, maintain backlinks, show parent context, rename a document key across the graph, and produce hierarchical symbol/search results.

## LSP Capabilities

IWES advertises and implements these LSP capabilities in `crates/iwes/src/main.rs` and routes them in `crates/iwes/src/router.rs`:

| LSP Capability                | IWES Behavior                                                                                                      |
|-------------------------------|--------------------------------------------------------------------------------------------------------------------|
| `textDocument/definition`     | Go to the target of a Markdown or wiki link. External URLs are opened through `window/showDocument`.               |
| `textDocument/references`     | Find backlinks and inclusion references to the document/link under the cursor.                                     |
| `textDocument/hover`          | Preview a linked note as Markdown, with frontmatter stripped.                                                      |
| `textDocument/completion`     | Complete document links. Trigger character is `+`; completions can insert Markdown links or wiki links.            |
| `completionItem/resolve`      | Present but currently returns the completion item unchanged.                                                       |
| `textDocument/rename`         | Rename a document key and update graph references. Uses `prepareRename`.                                           |
| `textDocument/prepareRename`  | Allows rename only when the cursor is on a link key.                                                               |
| `textDocument/documentSymbol` | Expose document outline/navigation symbols from graph paths.                                                       |
| `workspace/symbol`            | Fuzzy search over graph-derived search paths.                                                                      |
| `textDocument/codeAction`     | Offer graph/text actions such as extract, inline, attach, transform, sort, link, delete, and structure conversion. |
| `codeAction/resolve`          | Materialize a selected action into workspace edits.                                                                |
| `textDocument/formatting`     | Re-render the document through the graph, normalizing Markdown structure and links.                                |
| `textDocument/inlayHint`      | Show parent references, reference counts, and inclusion metadata.                                                  |
| `workspace/inlayHint/refresh` | Requested after resolved code actions when inlay hints have been used.                                             |
| `textDocument/foldingRange`   | Fold sections, code blocks, block quotes, lists, and tables.                                                       |
| `textDocument/inlineValues`   | Advertised, but implemented as an empty response.                                                                  |
| `textDocumentSync`            | Full-document sync, not incremental sync.                                                                          |

IWES does not appear to implement diagnostics, semantic tokens, document links, selection ranges, call hierarchy, type hierarchy, or custom Darkmatter-specific requests.

## Out-of-the-Box Features

IWES provides these out of the box:

- Link navigation: go to definition for Markdown links, wiki links, and external URLs.
- Backlinks: find references to the current document or linked target.
- Hover previews: render linked note content in a Markdown hover.
- Link completion: suggest existing documents as links.
- Workspace search: fuzzy search across graph paths and document titles.
- Document symbols: expose outline-like navigation from the graph.
- Rename refactoring: rename document keys and update references.
- Formatting: normalize Markdown through the graph writer.
- Inlay hints: show parent references and reference counts.
- Folding ranges: fold sections, code blocks, quotes, lists, and tables.
- Extract section: move a section into a new document and replace it with a link.
- Extract all subsections: split subsections into separate documents.
- Inline section: replace an inclusion link with the target content.
- Inline quote: inline linked content as a block quote.
- Attach: move/link selected content into a configured target document, such as a daily note.
- Link action: turn selected text or a cursor word into a new linked note.
- Delete action: remove a note and clean references.
- Sort action: sort list items.
- Structure conversion: convert lists to sections, sections to lists, and switch list types.
- Transform actions: pipe selected/contextual text through configured shell commands using stdin/stdout.
- Templates: create notes from configured templates.
- Frontmatter title selection: use a configured YAML frontmatter field as the document title for completion/search/link text.
- Configurable Markdown output style: emphasis token, strong token, list token, ordered-list token, code fence token/count, horizontal-rule token/count, ordered-list numbering.

## CommonMark Coverage

IWES uses `pulldown-cmark` `0.13.3` in `liwe::markdown::reader::MarkdownEventsReader`. The parser is invoked with:

```rust
Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
    | Options::ENABLE_WIKILINKS
    | Options::ENABLE_TABLES
```

Because `pulldown-cmark` is a CommonMark parser, IWES can parse normal CommonMark input at the parsing layer. However, IWES does not preserve or model every parsed event equally. The internal reader maps the constructs it needs into `DocumentBlock` and `DocumentInline` variants and ignores several event classes.

IWES meaningfully models:

- Paragraphs
- ATX headings
- Block quotes
- Fenced and indented code blocks
- Ordered lists
- Bullet lists
- List items
- Thematic breaks
- Tables
- Markdown links
- Wiki links
- Images
- Inline code
- Emphasis
- Strong emphasis
- Inline HTML as text
- YAML metadata blocks/frontmatter
- Soft breaks as spaces
- Inline math events if emitted by parser options, though math options are not enabled in the parser call

IWES ignores or only partially handles:

- HTML blocks
- Raw HTML structure
- Hard breaks
- Footnote references
- Footnote definitions
- Definition lists
- Task-list markers
- Display math
- Superscript and subscript tags
- Strikethrough unless enabled by parser options, which IWES does not do in the observed parser configuration

So the practical answer is: IWES has broad ordinary Markdown support because `pulldown-cmark` handles CommonMark, but IWES's graph model covers only the Markdown structures relevant to its knowledge-graph operations. It should not be described as a complete CommonMark semantic model.

## GFM Coverage

IWES does not provide full GFM support.

It enables `ENABLE_TABLES`, so GFM-style pipe tables are supported and represented in the graph. It does not enable the main other GFM-style extensions exposed by `pulldown-cmark`, such as task lists or strikethrough. It also does not implement GFM-specific semantics such as GitHub issue references, user/team mentions, autolinked bare URLs as Markdown nodes, or GitHub-compatible sanitization/rendering behavior.

Bare external URLs are handled separately in `liwe::parser::Parser::bare_url_at` for cursor navigation over `https://`, `http://`, and `mailto:` text. That is an LSP navigation convenience, not full GFM autolink parsing.

The accurate statement is: IWES supports tables and wiki links, but it is not a GFM-complete language server.

## Crossbeam Interaction

IWES uses `lsp-server`, which itself is the rust-analyzer-style synchronous LSP transport crate. The IWES public `main_loop` accepts an `lsp_server::Connection`. A `Connection` exposes sender and receiver channels for LSP messages. IWES then builds a `Router` around those channels.

The direct Crossbeam interaction is in `crates/iwes/src/router.rs`:

- The router imports `crossbeam_channel::{select, Receiver, Sender}`.
- The router stores a `Sender<Message>` for outbound messages.
- The event loop receives inbound messages through a `Receiver<Message>`.
- `next_event` uses `select!` over the inbound receiver.
- Responses and server-initiated requests are sent through the outbound sender.
- LSP requests are handled on spawned OS threads.
- Notifications are handled synchronously against the mutable server state.

This means Crossbeam is part of the server's transport and dispatch concurrency model. IWES is not built around async/await or Tokio for the LSP request path. It uses synchronous channels plus `std::thread::spawn`.

One implementation detail matters for Darkmatter: request handlers receive an `Arc<Server>` clone and run concurrently, while notifications such as `didChange`, `didSave`, and watched-file changes attempt to mutate the `Server` through `Arc::get_mut`. That shape works only when no other `Arc` clones are alive. It is a pragmatic but delicate design. A Darkmatter LSP should review this carefully if it needs deterministic ordering between edit notifications, graph invalidation, diagnostics, and request responses.

## Rayon Interaction

Rayon is used for CPU-bound graph and search operations, not for LSP transport.

Observed Rayon use includes:

- Building the initial graph from workspace state in parallel.
- Exporting graph documents in parallel.
- Generating graph paths in parallel.
- Updating and searching the workspace symbol/search index in parallel.
- Query evaluation and query execution over sets of keys in parallel.
- Stats aggregation in parallel.

In IWES specifically, the search index uses `par_iter()` both when building search paths from graph nodes and when scoring fuzzy matches. In `liwe`, graph construction parses documents in parallel, graph export uses `par_iter()`, path generation uses `into_par_iter()`, and the query subsystem uses parallel filtering/evaluation.

For Darkmatter, this is the useful split to preserve:

- Crossbeam/lsp-server: message transport and request dispatch.
- Rayon: parallel graph/index/query work over already-owned data.
- Core graph operations: mostly synchronous and deterministic from the caller's perspective.

## Relationship With `tower-lsp`

IWES does not use `tower-lsp`.

The current `iwes` crate depends on:

- `lsp-server`
- `lsp-types`
- `crossbeam-channel`
- `rayon`

It does not depend on `tower-lsp` or `tower-lsp-server`. Its server entrypoint calls `Connection::stdio()`, builds a `ServerCapabilities` value manually, calls `connection.initialize(...)`, then enters a custom router loop.

The relationship is therefore architectural contrast, not dependency:

- `tower-lsp` gives an async trait-based server framework where handlers are methods on a `LanguageServer` trait.
- IWES uses `lsp-server`, manually matches method strings, deserializes params, calls handler methods, and serializes responses.
- `tower-lsp` would hide much of the JSON-RPC dispatch and concurrency plumbing.
- IWES keeps direct control over routing, channels, request threading, and custom response behavior.

If Darkmatter wants to use IWES as a code foundation, it inherits an `lsp-server` architecture, not a `tower-lsp` architecture. If Darkmatter wants to use `tower-lsp`, IWES is still useful as a source of graph, Markdown, and operation patterns, but the LSP shell would need to be ported.

## Extension Mechanisms

IWES has two extension layers: configuration-level extension and code-level extension.

The configuration-level extension mechanism is `.iwe/config.toml`. This is the intended developer/user extension surface. The most important sections are:

- `[commands]`: define external shell or direct commands that read stdin and write stdout.
- `[actions.*]`: define editor code actions backed by those commands or graph operations.
- `[templates]`: define note templates.
- `[completion]`: configure completion link style and minimum prefix length.
- `[library]`: configure library path, title source, date/time formats, locale.
- `[markdown.formatting]`: configure output tokens and formatting style.

Configurable action types include:

- `transform`: pipe selected/contextual text through a named command.
- `attach`: attach selected content to a generated/configured target document.
- `sort`: sort list content.
- `inline`: inline a referenced document as a section or quote.
- `extract`: extract one section into a new document.
- `extract_all`: extract all direct subsections.
- `link`: create a new note from selected text and replace text with a link.

For many IWE use cases, adding a new transform or attach action requires no Rust code. A developer adds a command and an action to `.iwe/config.toml`.

For deeper extension, the Rust mechanism is to add a new `ActionProvider` implementation under `crates/iwes/src/router/server/actions/`, add a new `ActionDefinition` variant in `liwe::model::config`, and include it in `all_action_types`. The existing action system is the main internal extension seam: actions can inspect graph context, compute `liwe::operations::Changes`, and resolve into LSP `WorkspaceEdit` document operations.

For language-level extension, such as Darkmatter directives, the relevant code-level seams are lower:

- `liwe::markdown::reader::MarkdownEventsReader` for parsing Markdown events into `DocumentBlock` and `DocumentInline`.
- `liwe::model::document` and `liwe::model::graph` for adding new block/inline/node representations.
- `liwe::graph::sections_builder` for inserting parsed document structures into the graph.
- `iwes::router::server` handlers for LSP behavior on those new nodes.
- `iwes::router::server::actions` for custom refactorings and code actions.

Darkmatter-specific extension would likely require code-level changes, not only configuration, because Darkmatter has custom syntax, frontmatter semantics, composition/transclusion rules, shell expansion, interpolation, and validation needs.

## Disabling Mechanisms

IWES has limited built-in support for disabling features.

There is no general `.iwe/config.toml` switch that disables advertised LSP capabilities such as hover, definition, references, formatting, rename, workspace symbols, folding ranges, or inlay hints. The server always advertises those capabilities in `crates/iwes/src/main.rs`.

There are still several practical ways to reduce behavior:

- Disable features in the editor client. For example, do not bind code actions, disable format-on-save, disable inlay hints, or avoid registering IWES for files where it should not operate.
- Control configured actions by editing `.iwe/config.toml`. Configured actions are loaded from `configuration.actions`.
- Remove or avoid defining transform/attach/custom actions if those should not appear.
- Set `[completion].min_prefix_length` high to make completions effectively quiet, or set it to the desired threshold.
- Change `[completion].link_format` to control Markdown vs wiki-link insertion.
- Configure formatting tokens, though not whether formatting is advertised.
- Fork/patch IWES to remove capabilities from `ServerCapabilities` or remove built-in action providers from `all_action_types`.

A key caveat: some code actions are built in independently of config. `all_action_types` always includes list type changes, list-to-sections, sections-to-list, and delete, then extends the list with configured actions. Default configuration also supplies actions such as rewrite, expand, keywords, emojify, sort, inline, extract, extract_all, link, and attach. Config migrations may add default extract/inline/link actions to older configs.

So the accurate rule is:

- Config can extend and shape many actions.
- Config can reduce configured actions.
- Config cannot reliably disable every out-of-the-box feature.
- Full disabling requires editor-side suppression or code changes.

## Fit for Darkmatter

IWES is most useful to Darkmatter as a graph-aware Markdown LSP foundation. Its strongest transferable ideas are:

- Arena-backed document graph.
- Distinction between structural inclusion links and inline references.
- Workspace-wide reference index.
- Graph-derived LSP navigation.
- Graph-derived code actions.
- Shared core between CLI/MCP/LSP surfaces.
- Parallel graph/search/query work with Rayon.
- Simple local-first configuration model.

However, Darkmatter should not assume IWES already solves these Darkmatter-specific requirements:

- Darkmatter DSL parsing.
- Darkmatter composition pipeline awareness.
- Frontmatter schema validation.
- Diagnostics for malformed directives/interpolation/transclusion.
- GFM-complete parsing.
- Incremental text synchronization.
- Semantic tokens.
- Darkmatter render preview semantics.
- Disabling feature subsets by configuration.
- `tower-lsp` integration.

The likely path is to reuse or adapt the `liwe` graph concepts and selected IWES handlers, while replacing or extending the parser/model layer with Darkmatter's own Markdown and composition semantics. If the Darkmatter LSP remains committed to `tower-lsp`, IWES should be treated as a design/reference source rather than a direct server shell.
