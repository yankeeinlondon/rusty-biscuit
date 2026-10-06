# DMLS

DMLS is Darkmatter's language server. Use this reference for protocol,
workspace, completion, hover, and diagnostic work.

## Contents

- [Architecture](#architecture)
- [Passive-analysis contract](#passive-analysis-contract)
- [File-resolution contexts](#file-resolution-contexts)
- [Features](#features)
- [Undeclared properties and unknown functions](#undeclared-properties-and-unknown-functions)
- [Rollout chronology](#rollout-chronology)
- [Verification](#verification)

## Architecture

The package area has two editor-facing crates:

- `dmls`: the Language Server Protocol server and workspace/document state.
- `zed-dmls`: the Zed extension that launches and configures DMLS.

DMLS owns protocol transport, document snapshots, workspace indexing,
capability negotiation, and publication. Darkmatter owns Markdown parsing,
frontmatter spans, SimplifiedSchema, expression parsing, schema selection, and
validation. Do not fork those authorities in the server.

## Passive-analysis contract

Editor analysis must be safe for every keystroke:

- No shell execution or interpolation effects.
- No remote fetch, credential lookup, or implicit network access.
- No file mutation.
- No ambient CWD/repository recapture for an already-open document.
- No target-specific rendering required to classify a problem.

Use source text plus captured document/workspace context. Keep incomplete syntax
recoverable where possible and publish typed diagnostics with stable ranges.

## File-resolution contexts

User-facing contract: `dmls/docs/file-references.md`. Code: `dmls/src/context.rs`.

- `RunOptions::new(snapshot)`: `main.rs` calls `RequestSnapshot::from_process()`
  once and hands it to the server (and to `--bench-index`). Nothing else in
  DMLS reads process `HOME`, environment, or CWD for resolution.
- `RepositoryContexts` (shared `Arc`, on `ServerState.contexts`) caches one
  `build_resolution_context` result per repository root (key from
  `biscuit_file::find_git_root`; a document in no repository keys its folder),
  failures included. `for_document(path)` derives with `for_source`;
  `for_untitled(roots)` needs exactly one repository across the folders.
- Every provider resolves through `DocumentContext.resolution`
  (`ctx.file_context()` is `Result<&FileResolutionContext, &ContextFailure>`,
  `ctx.resolve_reference(raw)`; no context is ever an `Option`, which the
  context guard enforces). A failed resolution
  means **no** resolution: never reintroduce a lexical join (the old
  `normalize_join` is deleted). `providers/diagnostics.rs` publishes the one
  `dm.context.build_failure` diagnostic. With a failed context the overlay
  assembles no schema at all (`SchemaOutcome::Ready(None)`): `$schema` and
  trigger payloads cannot resolve without one, and `DarkmatterSchemas::new`
  requires a context.
- The graph takes `&dyn DocumentContexts` (`WorkspaceIndex::new(contexts)`):
  `arena::locate` takes the first planned candidate when it is indexed (no
  probe), else asks `context::resolve_reference`, else, only on a clean
  `NoMatch`, a later indexed candidate (an unsaved buffer); an `Io` probe
  failure stays a broken path. An existing unindexed file is
  `EdgeTarget::File`, never a broken link; the index decides headings, never
  existence, and anchors on unindexed files are not checked. Tests use `context::test_support`
  (`abs`, `workspace_contexts`, `resolution_for`) because a context rejects a
  rootless `/w` path on Windows; integration tests use `FixedContext` or a
  real `RepositoryContexts`; empty graphs use `NoContexts`
  (`ContextFailure::NotProvided`).
- Invalidation: every watched path calls `invalidate_contexts`, which drops
  ancestor-keyed entries (plus folder keys inside the repository for a
  `.git/` path), then `relink`s the graph. Package manifests and `.git/config`
  are watched (`watch::context_input_globs`, from sniff's
  `PACKAGE_MANIFEST_FILE_NAMES`) but never indexed (`is_context_input`).
  Rescan mode diffs `scan_manifests` fingerprints; config reload clears all.
- The overlay schema cache key includes
  `darkmatter::markdown::compose::file_resolution_context_identity(context)`
  (the same exhaustive encoding the compose graph identity uses: source,
  `cwd`, repository/package/area roots, home, launch `@` scope, sorted
  environment, magic and vault roots, tree root and origin) plus the
  resolution's `generation()`, so the same text under another context (a
  different `^` root or `SCHEMAS_DIR`) re-assembles and a rebuilt context
  re-validates file values. Never hand-roll a context hash in DMLS. A test
  that needs a cache miss must vary the context with no trigger registry in
  play (the registry's `Debug` also enters the key and masks the context
  term), as `overlay::tests::schema_cache_keys_on_the_package_root` does.
- `dmls/tests/l1/schema_roots_parity.rs` is the `md` vs DMLS parity gate for
  schema roots, applied triggers, bare-name `$schema`, and `$path`
  definition errors; DMLS shows missing required properties only with
  `[schema] strict = true`.
- `context_build_count()` is a process-wide work counter; L1 tests assert
  deltas (nextest runs one test per process).
- Fixture traps: sniff recognizes a package only under a workspace manifest,
  and counts a declared member whose directory exists even without its own
  manifest. Inline `$schema` `file` is lazy (syntax only); use `file(eager)`
  to exercise resolution in validation.

## Features

DMLS projects Darkmatter's typed surfaces into LSP:

- Frontmatter and schema diagnostics with source spans.
- Schema-aware completion for keys, values, suggestions, literal-discriminated
  union arms, and imported types.
- Expression completion, hover, and parse diagnostics inside
  expression-typed frontmatter.
- Hover/documentation from typed descriptor catalogs.
- Workspace dependency tracking for referenced schemas and documents.

Completion and validation must call the same schema-arm selection and parser
authorities. A completion-only reconstruction is drift.

Frontmatter `$( … )` suffixes follow the same rule: completion, hover, and the
`dm.shell.invalid_suffix` diagnostic (`providers/dsl.rs`) read the library's
`FRONTMATTER_SHELL_SUFFIXES`, `describe_suffix`, and
`parse_frontmatter_shell_suffixes`, the grammar `md compose` parses with.
`frontmatter_shell_values` strips a quoted scalar's quotes before parsing,
because the YAML span includes them.

## Undeclared properties and unknown functions

`dm.expression.undeclared_property` is an advisory **Warning** on body
`{{ … }}` and on Expression-typed frontmatter values, with the same code,
severity, and wording `md compose` uses: the property is valid, of unknown
type, and `null` unless supplied at runtime. `dm.expression.unknown_function`
is an **Error** at both sites, ranged on the name and reported in every
branch, even without frontmatter. It comes from the library's passive
`expression::validate_expression` against `BindingView::baseline()`
(`overlay::expressions::unknown_function_calls`), the check
`validate_prepared` runs for preparation; never add a DMLS-side function list.

- **Expression-typed values.** `providers::frontmatter::expression_values`
  decodes each value with the library's `decode_scalar_node`, the decoder
  composition uses to anchor frontmatter failures, so block (`|`, `>`) and
  multi-line values are parsed as composition evaluates them and warnings land
  on exact identifier ranges. A value is kept only when the decoded text
  equals the parser's `FmEntry::scalar`. Never add a DMLS-side reconstruction.
  `rlsp-yaml-parser` starts a scalar's span after its tag and anchor, so
  tagged and anchored values need nothing extra. An alias value is checked
  against `FmEntry::alias_target` (the last scalar anchor before it) and
  diagnosed inside the anchor's scalar, which DMLS decodes where its own YAML
  tree found it: `FmEntry::alias_target_start` into the library's
  `decode_alias_definition`. Every alias of one definition shares its
  `alias_target` (`Arc<str>`), and `expression_values` decodes and checks each
  distinct start once per call, sharing the `Arc<DecodedScalar>`; a copy or
  decode per alias is O(aliases × length). Never resolve an alias by calling
  `decode_scalar_node` on the `*name` token: that searches the text before
  the alias, once per alias, and made a publication quadratic. The tree also
  sees through an anchor name repeated in a comment or string. The start is
  withheld for an anchor redefined before the alias, and the decoded text may
  disagree; then the `alias_target` text is still analyzed and every
  diagnostic lands on the alias token (`AuthoredExpression::AliasToken`):
  same codes and severities, identical diagnostics collapsed, and no
  quick-fix, since no replacement range exists there. Never skip an
  Expression-typed alias.
  `diagnostics()` computes the expression-value set once per publication and
  hands it to both `schema_problem_diagnostics` and `expression_diagnostics`.
  `distinct_expression_aliases_publish_without_searching_the_document` pins
  both through work counters (the library's `alias_search_work`, behind its
  `work-counters` feature, which dmls enables as a dev-dependency only).
  `many_aliases_of_one_long_expression_share_its_text_and_one_decode` pins the
  shared target (`Arc::ptr_eq`) and one decode (`ALIAS_TARGET_DECODES`).
  `expression_diagnostics` skips a value that is a whole literal token
  (`{{!data:v1:…}}`): it is data, and its encoded bytes cannot anchor a parse
  error (`a_literal_token_is_not_parsed_as_an_expression`).
  `expression_diagnostics` parses and reports each authored expression once,
  so an anchored value and its aliases never double-report; a malformed one
  whose property's union accepts it keeps its parse error for a later alias
  whose property rejects it (`EXPRESSION_DIAGNOSTIC_PARSES`,
  `a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias`).
  Hover/completion
  ignore the alias token.
- **Walk.** Diagnostics use the library's
  `expression::static_variable_reads`. It yields every `Variable` with its own
  span, classified by the same `AbsenceScope` rules the runtime uses (fallback
  primary, ternary condition plus its guarded root, direct `is_null`/`is_empty`
  argument). Never re-derive those rules in DMLS. The only deliberate
  difference is that the static walk visits both ternary branches.
- **`root_identifier` keeps its one-root contract.** Hover, definition, and
  graph indexing depend on it. Only the two diagnostic providers use the walk.
- **One classification.** `overlay::expressions::is_undeclared_property`
  classifies the first dotted segment through Darkmatter's binding model:
  `BindingView::baseline().names_document_property` (reserved namespaces from
  `reserved_root_descriptors()`, plus the `null` literal). DMLS supplies no
  host descriptors, so Claudine's `err`/`timing`/`group` are undeclared
  properties here, beneath lifecycle keys too, until R7c distributes host
  descriptors. Root names DMLS needs (`ctx` hover/completion, the `doc[...]`
  quick-fix) come from `context_root()`/`document_root()`, which read the same
  catalog. `undeclared_property.rs` fails on any root-name string literal, a
  host-global list, or an evaluation-session type in production `src/`.
  `providers::dsl::KnownRoots` adds frontmatter keys and schema properties. It
  computes `known_shape` once per diagnostics pass, and it is `None` for a
  frontmatter-less document, whose names are never reported undeclared.
- **Dash-separated keys.** A subtraction of only variables (`foo--bar`, `a- b`)
  that contains an undeclared operand, and whose whitespace-free source is a
  top-level key, gets one key-level finding. A literal operand
  (`iteration - 1`) stays arithmetic. A fix is attached only when the edited
  expression reparses with the replacement as one reference to the key: the
  bare key first, then `doc['key']` quoted to avoid the value's YAML quote.
  The fix rides in `Diagnostic.data` as `KeyReferenceFix`. The code action
  reads `data`, or recomputes the producing provider's diagnostics when a
  client drops `data`, and never parses the message.

## Rollout chronology

The DMLS stream was delivered in dependency order:

1. Span-aware validation and style diagnostics in the library.
2. Passive standalone schema classification.
3. Server transport and document/workspace state.
4. Schema selection, imports, completion, hover, and diagnostics.
5. Suggestion, literal-discriminant, expression, and meta-schema support.
6. Zed packaging and end-to-end editor integration.

This chronology is historical routing, not a second architecture. Current code
and public types are authoritative if an older phase note disagrees.

## Verification

Use package-scoped DMLS and Zed gates. Protocol behavior needs integration tests
that open real documents through the server's normal request path. Schema or
expression changes also require passive shipped-artifact corpus coverage in the
Darkmatter library, so editor tests are not the only proof of grammar behavior.

- `just test` runs the cross-platform L1 extension manifest and crate-shape
  contract alongside the DMLS protocol suite.
- `tests/l1/repository_contexts.rs` holds the per-repository context
  acceptance tests (one build, every feature, invalidation, failure
  diagnostic, untitled buffers). `LspFixture::start_with_snapshot` supplies
  a test's own `HOME`/environment/`@` roots; the default fixture snapshot uses
  a `HOME` inside the workspace.
- `just check-zed` compiles `zed-dmls` for Zed's `wasm32-wasip2` target and
  requires that target to have been provisioned explicitly.
- `just zed-verify` adds Zed's pinned official packager and artifact contract;
  CI runs this as an Ubuntu companion gate, not as L2.
- `just install-dmls` installs the binary and, when Zed's data directory
  exists, stages the extension (with the bundled `extension.wasm`) to a stable
  per-user directory and creates or repairs Zed's `installed/dmls` link.
- `just zed-doctor` diagnoses the host's native binary, stable dev-extension
  registration, manifest, and recent Zed log evidence without launching Zed.
