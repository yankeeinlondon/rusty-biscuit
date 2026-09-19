# DMLS

DMLS is Darkmatter's language server. Use this reference for protocol,
workspace, completion, hover, and diagnostic work.

## Contents

- [Architecture](#architecture)
- [Passive-analysis contract](#passive-analysis-contract)
- [Features](#features)
- [Unknown identifiers](#unknown-identifiers)
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

## Unknown identifiers

`dm.expression.unknown_identifier` is a **Warning** on body `{{ … }}` and on
Expression-typed frontmatter values. That is the same severity `md compose`
uses for the same condition.

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
- **Known roots.** `overlay::expressions::is_unknown_root` classifies the first
  dotted segment. Reserved roots, `null`, and bare runtime-context names come
  from the library's `is_statically_known_root`, so the editor never flags a
  root the runtime knows. `providers::dsl::KnownRoots` adds frontmatter keys and
  schema properties. It computes `known_shape` once per diagnostics pass, and
  it is `None` for a frontmatter-less document, which is never diagnosed.
- **Dash-separated keys.** A subtraction of only variables (`foo--bar`, `a- b`)
  that contains an unknown operand, and whose whitespace-free source is a
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
- `just check-zed` compiles `zed-dmls` for Zed's `wasm32-wasip2` target and
  requires that target to have been provisioned explicitly.
- `just zed-verify` adds Zed's pinned official packager and artifact contract;
  CI runs this as an Ubuntu companion gate, not as L2.
- `just install-dmls` installs the binary and, when Zed's data directory
  exists, stages the extension (with the bundled `extension.wasm`) to a stable
  per-user directory and creates or repairs Zed's `installed/dmls` link.
- `just zed-doctor` diagnoses the host's native binary, stable dev-extension
  registration, manifest, and recent Zed log evidence without launching Zed.
