---
prompt: |-
  We are building DMLS, the Darkmatter Language Server, as an IWES-derived Rust
  LSP. Architectural decision AD-1 in @darkmatter/features/2026-07-04-dmls/design.md
  was accepted as: vendor/adapt a minimal IWES-derived subset (arena graph, key
  index, router shape) into `dmls`, validated by a short spike that first
  attempts a `liwe` crates.io dependency to confirm its API does not fit.

  Your task is to produce the API inventory that spike needs, by studying the
  upstream IWE repository (https://github.com/iwe-org/iwe — clone it locally
  for inspection):

  1. Inventory the public API of the `liwe` crate: graph/arena types, key
     index, parser entry points, config model. For each module state whether
     DMLS could consume it as-is, would need to wrap it, or cannot use it.
  2. Specifically evaluate: can `liwe`'s graph represent typed edge kinds
     beyond its own (we need `references`, `includes`, `transcludes`,
     `uses_schema`, `uses_file`, `uses_variable`, `defines_anchor`,
     `defines_symbol`)? Can nodes carry foreign payloads (frontmatter key
     paths, directive arguments)?
  3. Inventory the `iwes` server: router structure, request dispatch,
     document sync handling, the multibyte/position-encoding fixes and their
     tests, and the Helix selection-range quirk handling. Identify the exact
     files/functions worth porting into DMLS and the ones to leave behind.
  4. Check licensing: what license does IWE carry, and what attribution
     obligations apply if we vendor/adapt code versus only porting concepts?
  5. Estimate the minimal-subset size: roughly how much code (modules, LOC)
     would the vendored graph + router shape amount to?

  Deliverables: a module-by-module inventory table, a ported-vs-left-behind
  list for the router, a licensing note, and a final recommendation with
  confidence level on whether the spike should even attempt the `liwe`
  dependency route or go straight to adaptation.
last_updated: 2026-07-06
hash: bad5bc730ab21e1c-bffade3b5fd33369
---
## Upstream Snapshot

Inspected local clone of `https://github.com/iwe-org/iwe` at `d1bed53` (`2026-07-03`, version `0.7.0`).

## `liwe` API Inventory

| Area                                                                     | Public surface                                                                                                                                                                                       | DMLS fit                       | Notes                                                                                                                                                                                               |
|--------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------:|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `liwe::graph::Graph`                                                     | `new`, `new_with_options`, `from_state`, `from_path`, `update_document`, `remove_document`, `keys`, `key_index`, `frontmatter`, `get_*_edges_*`, `node_line_range`, `paths`, Markdown export helpers | Wrap at best                   | Useful lifecycle shape, but owns parsing, document content, frontmatter as `serde_yaml::Mapping`, and a closed IWE graph model. Not a good DMLS substrate as-is.                                    |
| `liwe::graph::arena`                                                     | `Arena`, `BuildArena`, `BuildIds`, `NodeStore`, `finalize_build`                                                                                                                                     | Adapt concept                  | Good minimal arena/build-arena pattern. Not directly importable because it stores `GraphNode`, `Line`, and IWE inline nodes.                                                                        |
| `liwe::graph::graph_node`                                                | Closed `GraphNode` enum: `Document`, `Section`, `Quote`, `BulletList`, `OrderedList`, `Leaf`, `Raw`, `HorizontalRule`, `Reference`, `Table`                                                          | Cannot use as-is               | No `Custom`, no generic payload, no typed DMLS overlay variants. DMLS would need its own `NodeKind` and payload model.                                                                              |
| `liwe::graph::index::RefIndex`                                           | Reverse index with `inclusion_edges` and `reference_edges` only                                                                                                                                      | Cannot use as-is               | This is the decisive mismatch. DMLS needs arbitrary typed edge kinds and one reverse index over all of them.                                                                                        |
| `liwe::graph::builder::GraphBuilder`                                     | Markdown-oriented builder for sections, lists, raw blocks, references, tables                                                                                                                        | Adapt selected ideas           | Builder is ergonomic but tied to IWE nodes/inlines and only emits IWE reference/inclusion semantics.                                                                                                |
| `liwe::graph::sections_builder`                                          | Converts parsed `DocumentBlock`s to graph nodes and line maps                                                                                                                                        | Leave behind                   | Useful as a reference for source line mapping, but DMLS should index Darkmatter Markdown/overlay nodes directly.                                                                                    |
| `liwe::graph::path`, `basic_iter`, `squash_iter`, `walk`                 | Tree/path traversal and inclusion/reference walks                                                                                                                                                    | Adapt concept                  | Traversal shape is useful, but the implementation depends on closed node variants and two edge classes.                                                                                             |
| `liwe::model::key_index::KeyIndex`                                       | Wiki-link basename index: `build`, `insert`, `remove`, `resolve_wiki`, `shorten_wiki`, `resolve_link_key`                                                                                            | Wrap or port                   | This is the most directly reusable `liwe` API. DMLS can port the algorithm, but should replace `Key` with DMLS document IDs/file refs and add anchor/symbol awareness.                              |
| `liwe::model::Key`                                                       | Relative path key, strips `.md`/`.dj`, relative-link resolution                                                                                                                                      | Wrap only                      | Useful idea, but DMLS should use `Url`/workspace path IDs plus `biscuit-file::FileReference` where applicable.                                                                                      |
| `liwe::parser::Parser`                                                   | `new(content, format)`, `link_at`, `url_at`                                                                                                                                                          | Cannot use as DMLS parser      | It only exposes link/URL hit-testing over IWE’s parsed document. DMLS needs source maps, frontmatter key paths, directives, schemas, and Darkmatter semantics. Some UTF-16 fixes are worth porting. |
| `liwe::markdown::reader::MarkdownEventsReader`                           | Pulldown parser to IWE `Document`; YAML metadata block into `serde_yaml::Mapping`; line ranges                                                                                                       | Leave behind                   | Darkmatter is DMLS’s Markdown authority. This parser is useful only as prior art for position tests.                                                                                                |
| `liwe::djot::reader`                                                     | Djot parser to IWE `Document`                                                                                                                                                                        | Leave behind                   | DMLS v1 is Darkmatter Markdown.                                                                                                                                                                     |
| `liwe::model::document`                                                  | Large Pandoc-like AST, inline ranges, link key ranges, UTF-16 offsets in link text                                                                                                                   | Leave behind, port tests/ideas | Too broad and IWE-specific. The `DocumentInline::key_range` UTF-16 logic is worth reviewing when building DMLS source maps.                                                                         |
| `liwe::model::config`                                                    | `Configuration`, `MarkdownOptions`, `DjotOptions`, `LibraryOptions`, `CompletionOptions`, action/template command model, `load_config`                                                               | Cannot use as-is               | IWE product config, `.iwe/config.toml` discovery, command/action model. DMLS needs `DmlsConfig`, schema baselines, policy settings, and client profiles.                                            |
| `liwe::schema`                                                           | Frontmatter schema inference from graph/frontmatter fields                                                                                                                                           | Leave behind                   | Not a validation model and not Darkmatter SimplifiedSchema.                                                                                                                                         |
| `liwe::query`, `operations`, `retrieve`, `find`, `format`, `fs`, `state` | IWE app/query/editing features                                                                                                                                                                       | Leave behind                   | Not part of the DMLS minimal substrate.                                                                                                                                                             |

## Graph Extensibility Findings

`liwe` cannot represent DMLS’s required edge model as-is.

`RefIndex` has exactly two maps: `inclusion_edges` and `reference_edges`. Edge classification is a boolean emitted by `walk_edges`: `GraphNode::Reference` becomes inclusion, and inline `ref_keys()` in section/leaf/table lines become reference edges. There is no public or private extension point for additional edge kinds such as `transcludes`, `uses_schema`, `uses_file`, `uses_variable`, `defines_anchor`, or `defines_symbol`.

`GraphNode` is also a closed enum. Nodes cannot carry foreign payloads except through existing IWE fields: `Document` has a `Key`, `Raw` has `lang/content`, `Reference` has `key/text/reference_type/url`, and text-bearing nodes point at IWE `Line`/`Inlines`. There is no generic metadata map or payload slot for frontmatter key paths, directive arguments, source-map spans, schema references, or Darkmatter overlay nodes.

Conclusion: using `liwe::Graph` would force DMLS to maintain a parallel overlay index anyway. That violates AD-1’s accepted “single workspace graph with edge-kind filters” direction.

## `iwes` Router Inventory

### Worth Porting

| File/function                                                                  | Port value    | Notes                                                                                                                                                                                                               |
|--------------------------------------------------------------------------------|---------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `crates/iwes/src/main.rs` capability construction                              | Medium        | Keep the direct `lsp-server` stdio initialization and explicit `ServerCapabilities` shape, especially full sync. Rewrite around DMLS capabilities.                                                                  |
| `crates/iwes/src/router.rs::Router`                                            | High as shape | The simple `Connection` receiver loop, `Request`/`Notification` dispatch, `shutdown`/`exit` handling, and panic boundary are worth adapting. Replace the giant match with DMLS provider/router modules as it grows. |
| `router.rs::on_notification`                                                   | Medium        | Full-sync `didChange`, `didSave`, and watched-file delete handling are useful patterns. DMLS needs `didOpen`/`didClose`, version tracking, diagnostics scheduling, and stale generation checks.                     |
| `router.rs::on_request`                                                        | Medium        | Good baseline dispatch style and error responses. Keep external `window/showDocument` idea only if DMLS exposes external links. Drop inlay/code-action-specific refresh behavior initially.                         |
| `router/server.rs::handle_did_change_text_document`                            | Medium        | Confirms full-text replacement model. DMLS should preserve client version and open-document authority, not just update graph content.                                                                               |
| `router/server.rs::handle_did_save_text_document`                              | Low/medium    | Useful only if DMLS chooses save-triggered workspace validation.                                                                                                                                                    |
| `router/server/extensions.rs::utf16_to_byte_offset` and `byte_to_utf16_offset` | High          | Port into DMLS `source_map` tests or use as a minimal helper. DMLS should probably generalize this into a per-document line index.                                                                                  |
| `liwe/src/parser.rs` tests for multibyte/astral URLs and wiki links            | High          | Port the test cases, not the parser. They cover UTF-16 LSP positions after Greek and astral characters.                                                                                                             |
| `iwes/tests/go_to_definition_test.rs` multibyte/astral cases                   | High          | Port as DMLS L2 fixture cases for definition/reference hit-testing.                                                                                                                                                 |
| `iwes/tests/hover_test.rs` multibyte cases                                     | High          | Port as hover hit-testing cases.                                                                                                                                                                                    |
| `iwes/tests/completion_test.rs::completion_does_not_panic_on_multibyte_prefix` | High          | Port for completion replacement range correctness.                                                                                                                                                                  |
| `iwes/tests/link_test.rs` multibyte selection cases                            | Medium        | Useful if DMLS code actions manipulate selected text.                                                                                                                                                               |
| `iwes/tests/delete_test.rs::delete_inline_link_after_astral_character`         | Medium        | Useful for action range conversion if DMLS implements link-edit actions.                                                                                                                                            |
| `router/server/base_path.rs` URL/path tests                                    | Medium        | Port the Windows drive-letter, percent-encoding, spaces, and anchor-fragment cases. Implementation should be DMLS-owned and use the repo’s file-reference conventions.                                              |
| `iwes/tests/fixture.rs` in-memory LSP fixture                                  | Medium        | Good proven shape for L2 request/response tests. Rewrite smaller for DMLS.                                                                                                                                          |

### Leave Behind

| Area                                                                              | Reason                                                                                                               |
|-----------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| `router/server/actions/**`                                                        | IWE editing commands: extract, inline, attach, sort, transform, delete, link. Not DMLS v1 language-server substrate. |
| `router/server/query.rs`                                                          | IWE graph query helpers over IWE node model.                                                                         |
| `router/server/search.rs`                                                         | IWE workspace symbol search over IWE title paths. DMLS providers should query DMLS graph/providers directly.         |
| `Server` methods for IWE formatting, inlay hints, action resolution, rename edits | Product-specific and coupled to `liwe::Graph` mutation/export.                                                       |
| `Configuration`-driven action kinds and command execution                         | Outside DMLS passive-analysis model.                                                                                 |
| Inlay hint refresh after code-action resolve                                      | Specific IWES UX behavior; not part of DMLS baseline.                                                                |

## Position-Encoding and Helix Notes

IWES advertises full document sync through `TextDocumentSyncKind::FULL`. It does not explicitly negotiate `positionEncoding`; its fixes assume LSP UTF-16 positions.

The important fixes are local:

- `utf16_to_byte_offset(line, utf16_offset)` converts an LSP character offset into a Rust byte index and rejects offsets inside surrogate pairs.
- `byte_to_utf16_offset(line, byte_offset)` converts valid UTF-8 byte offsets back to LSP character offsets.
- `Parser::find_url_at_position` uses `encode_utf16().count()` for bare URL hit-testing.
- `DocumentInline::key_range` uses UTF-16 length for Markdown/wiki link key ranges after multibyte link text.
- Completion computes the cursor byte from UTF-16 before slicing the Rust string, then converts the replacement start byte back to UTF-16.

The Helix quirk is in `Server::handle_code_action`: when the client is `helix` and the selection range width is exactly one character, IWES treats it as an empty selection at the start character. Port this as a named client-profile normalization function only if DMLS exposes selection-sensitive code actions. Do not bury it inside provider logic.

## Licensing

IWE is Apache-2.0. The workspace manifest declares `license = "Apache-2.0"` and all crates inherit it. The repository includes `LICENSE-APACHE` at the root and in the published crates.

If DMLS depends on `liwe` only as a crate, normal Cargo dependency license disclosure is sufficient.

If DMLS vendors or adapts code, preserve Apache-2.0 notices, include attribution to IWE/Dmytro Halichenko where copied/adapted code lives, retain license text in the distribution, and mark files or modules with material modifications. Apache-2.0 is permissive and compatible with adaptation, but copied source should not be made attribution-anonymous.

If DMLS ports only concepts and writes fresh code, keep a design note citing IWE/IWES as architectural inspiration; the Apache notice obligations are much lighter because no copyrightable code is copied.

## Minimal-Subset Size Estimate

Raw upstream sizes:

| Upstream slice                         | LOC   |
|----------------------------------------|------:|
| `liwe/src/graph.rs`                    | 1,002 |
| `liwe/src/graph/*.rs`                  | 3,088 |
| `liwe/src/model/key_index.rs`          | 291   |
| `iwes/src/router.rs`                   | 307   |
| `iwes/src/router/server.rs`            | 894   |
| `iwes/src/router/server/base_path.rs`  | 264   |
| `iwes/src/router/server/extensions.rs` | 438   |
| `iwes/src/router/server/search.rs`     | 165   |

A literal vendored subset would be too large because `graph` pulls in IWE model, parser, writer, operations, and config types. The practical DMLS adaptation should be much smaller:

| DMLS-owned subset                                                       | Estimated LOC |
|-------------------------------------------------------------------------|--------------:|
| Arena IDs/storage and build arena                                       | 180-260       |
| Node/edge model with typed `NodeKind`, `EdgeKind`, payload enum/structs | 220-350       |
| Reverse index by `EdgeKind` + target/source                             | 120-180       |
| Key/wiki basename index adapted from `KeyIndex`                         | 120-220       |
| Graph update/removal/invalidation skeleton                              | 200-350       |
| Router loop and request/notification dispatch                           | 250-450       |
| Source-map UTF-16 helpers and tests                                     | 180-320       |
| URI/base-path helpers and cross-platform tests                          | 150-260       |

Expected minimal spike size: about 1,400-2,400 LOC of DMLS-owned code plus tests. A direct vendor of upstream graph/router would start around 4,000-6,000 LOC before pruning and still require invasive changes.

## Recommendation

Do the short `liwe` dependency attempt only as a documented negative check, not as a serious integration route. Confidence: high.

The dependency route fails the core AD-1 requirement: `liwe` has a closed graph node model, a two-kind reverse edge index, no foreign node payloads, and parser/config APIs tied to IWE product behavior. Wrapping it would produce two graphs or a sidecar overlay, which is exactly what DMLS should avoid.

Proceed straight to adaptation after the dependency spike records these mismatches. Port the IWES router shape, UTF-16 conversion tests, base-path test cases, and the `KeyIndex` algorithm. Rebuild the graph substrate in DMLS with first-class `EdgeKind`, DMLS node payloads, Darkmatter parser/semantic inputs, and a source-map API designed for frontmatter and directive ranges.
