---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 2
completed: true
clarified: false
implemented: true
human_review: false
message_to_agent: ''
parent: ../../reviews/2026-07-31-macro-review/spec.md
depends_on:
    - 2026-10-17-schematic-websocket
refresh_required: true
related: []
implemented_by: "claude/opus"
---

# Schematic fidelity and polish

## Problem

Schematic turns one API definition into three artifacts: a Rust client crate
(`schematic-schema`), an OpenAPI document, and a Postman collection.
WebSocket APIs currently have no exported API document. The Rust client is the strong output. The external artifacts and generated documentation
are less faithful than the Rust client. Facts below come from the parent review
(`2026-07-31-macro-review`, July evidence unless
marked) and, where marked **checked**, from the tree on 2026-10-11. Anything
marked **unverified** was not re-checked and must be re-measured by the
implementer before it is relied on.

1. **OpenAPI schemas lose types.** The schemars-to-OpenAPI bridge
   (`schematic/definitions/src/registry/conversion.rs`) handles no type arrays,
   `enum`, `oneOf`/`anyOf`/`allOf`, or `additionalProperties`. Every `Option<T>`
   and every Rust enum falls to `SchemaKind::Any`. July measurement:
   bitbucket 109 of 216 properties untyped, huggingface 99 of 227, elevenlabs
   73 of 233 (about 30 percent overall). **Unverified today.**
2. **Nested `$ref`s can dangle.** `validate_completeness`
   (`schematic/definitions/src/registry/mod.rs`) checks endpoint names against
   registered names, so it cannot see a nested type that schemars references
   but nobody registered (parent examples: `anthropic::Usage`,
   `elevenlabs::SampleModel`). Export-time closure validation already rejects
   dangling refs, so this is a gap in the early check, not evidence of a
   published dangling ref (ledger row 7).
3. **Required constant headers are not OpenAPI parameters.** Anthropic's
   mandatory `anthropic-version` lives only in `x-schematic`, so a client
   generated from the OpenAPI file gets HTTP 400.
4. **The `x-schematic` round trip is documented but not implemented.** The
   exporter writes the extension; the importer never reads it (zero
   references under `schematic/define/src/openapi/import/`, July). **Checked:**
   `schematic/define/src/openapi/extensions.rs` defines four extension structs
   and has no version field in any of them. `SchematicSchemaExtension` and
   `AuthExtension` are never constructed in production; `AuthExtension`'s
   rustdoc claims an embedding that does not exist. Three files claim
   round-trip fidelity. Ledger row 14 says `skip_serializing_if` without
   `serde(default)` breaks re-import of the tool's own export. **Checked
   caveat:** serde treats a missing `Option` field as `None` without
   `default`, and the `Vec` fields in the file already carry
   `#[serde(default)]`, so the row may not reproduce as written (see R14).
5. **Import gaps.** Path-item-level parameters are dropped, multipart bodies
   import as `request: None`, and `strict` mode fails on INFO diagnostics
   (**checked:** `ImportOptions::strict` documents "any diagnostic becomes an
   error", and `builder.rs` fails when the diagnostics list is non-empty), so
   it is unusable on realistic input. The test for it is tautological (ledger
   row 17).
6. **Postman collections are browsable, not runnable.** Every JSON body is the
   literal string `"{}"` (the schema registry is never passed in;
   `schematic/gen/src/export/postman/examples.rs` is a placeholder). There are
   no environment files, no saved example responses, OAuth2 collapses to
   `bearer`, `Text` and `Binary` bodies lose content types, and header and
   cookie parameters are ignored.
7. **WebSocket APIs have no external artifact.** The AsyncAPI importer
   (`schematic/gen/src/asyncapi_import.rs`) returns a `WebSocketApi` and a
   model catalog in memory, but writes only a count summary. **Checked:** the
   missing behavior is reusable output, not retention in the return value. No AsyncAPI exporter
   exists. Five WebSocket APIs, zero artifacts.
8. **The generated crate has no feature flags.** Every consumer compiles all 18
   REST clients, 5 WebSocket modules, reqwest, tokio, and tungstenite
   (`schematic/gen/src/cargo_gen.rs`). Eight crates depend on it (D20).
9. **Parallel API lists drift.** The parent says seven hand-maintained
   lists exist but does not enumerate them. Read-only review found six runtime
   lists plus test tables (R33 gives the starting inventory). One visible symptom: the strict-completeness gate skips 2 of
   18 APIs (ledger row 15). **Checked:** the test lives at
   `schematic/gen/tests/l1/openapi_strict_completeness.rs` (the parent cites
   an older path).
10. **Two module-path resolvers disagree.** `gen/src/output/assemble/helpers.rs`
    applies a suffix heuristic; `definitions/src/lib.rs` only lowercases. An API
    named `PagerDutyApi` with no explicit `module_path` lands in `pagerduty.rs`
    but `pagerdutyapi.json` (ledger row 22).
11. **Generated docs lie.** 478 doc fences are all `text` (none compiled); the
    quick-start calls `client.list_models()`, which does not exist for JSON
    endpoints; the `AuthStrategy::ApiKey` literal omits `value_prefix`; 283
    stray `///;` lines (ledger row 8). Typed per-endpoint convenience methods
    are an accepted gap, not a defect to fix (D53, R39). A module shared by
    several APIs (emqx) also loses its combined-module docs (R51).
12. **`schematic-define` hygiene.** The README has about 7 drifted,
    non-compiling examples and is not doctested (ledger row 19). The vendor
    pagination constructors (`github()`, `gitlab()`, `bitbucket()`, `gitea()`,
    **checked:** present in `schematic/define/src/pagination.rs`) sat in the
    vendor-neutral crate. The author ruled (D55-4) that each API owns its own
    small strategy function in `schematic-definitions`, and spec 4a does that
    work. This spec moves no presets; it verifies none remains in
    `schematic-define` and that no shared preset layer was introduced (R43).
13. **No closing audit.** The whole body of work (parent D20) requires every
    defect-ledger row and every recommendation 1-14 closed or recorded as an
    accepted gap, and requires sniff to retire its hand-written provider
    client. Nothing yet collects that evidence in one place.

## Goal and non-goals

**Goal.** Make the OpenAPI, Postman, and AsyncAPI outputs, the generated crate's
packaging, and the generated documentation faithful to the definitions, finish
the hygiene items the earlier specs left, and close the work by publishing a
ledger disposition table and the whole-work done checklist (D18, D46, D47,
D49, D20).

**Non-goals.**

- Changing the definition vocabulary. That was specs 4a, 4b and 5 (builders and
  typed registration; typed path parameters, auth override, `deprecated`, rate
  limit vocabulary, OAuth2 grants, serializable definitions; response model v2,
  GraphQL). This spec consumes their output.
- Changing the runtime, envelope, credentials, or pagination model (specs 2, 3
  and 4a). Spec 4a removes the old pagination types and gives each API its own
  strategy function (D55-4); this spec only verifies the result (R43).
- Per-endpoint typed wrapper methods (D53, accepted gap against D49 item 3 and
  recommendation 12; R39).
- WebSocket fixes and the typed message enums (spec 6). This spec only exports
  what spec 6 produced.
- A blanket ban on `SchemaKind::Any`. Free-form JSON is valid (D18 detail;
  parent recommendation 8).
- Editing sniff. sniff retires its hand-written client under its own spec; this
  spec only checks and records the state (R47).
- Moving this spec, or the parent, to `_completed`. Only the author does.
- Any CI scope or workflow change.

## Reading this specification

The `parent` property identifies the shared review that produced these sibling
specifications; it does not make this document an implementation of that file.
The existing property is retained. References such as `R19` identify a named
requirement under Requirements; `P10-c` identifies a named default under
Planning-time details; `D18` identifies a restated author decision in Background.
Use the descriptive heading when discussing any of them. Historical counts are
baselines to refresh, not hard-coded limits on the supported API inventory.

## Background: decisions this spec implements

Restated in full so the implementer need not open the parent. Provenance:
`2026-07-31-macro-review`, section 15 (under the Schematic reviews directory).

- **D18 (fourth landing, fidelity and hygiene).** A fourth landing carries
  fidelity and hygiene work that does not change the definition vocabulary:
  OpenAPI schemars-bridge fixes; the `x-schematic` round trip; Postman example
  bodies; README doctests; vendor pagination presets out of the neutral crate
  (superseded by D55-4: each API owns its own strategy function, delivered by
  spec 4a, so this spec only verifies it); per-API feature flags in the
  generated crate.
- **D46 (stage order).** WebSocket is landing 3 and fidelity and polish is
  landing 4. The single generated API list (replacing seven parallel lists) and
  required constant headers as OpenAPI parameters go to landing 4.
- **D47 (AsyncAPI exporter).** In scope, in landing 4, after WebSocket reuses
  the REST types (D8). The existing AsyncAPI importer should read the result
  back into an equivalent WebSocket definition (suggested check). AsyncAPI
  version and supported subset are planning details (P10).
- **D49 (loose items).** Landing 4 also owns: unifying the two module-path
  resolvers (ledger row 22, recommendation 10), and the generated-docs honesty
  pass (ledger row 8, recommendation 12: `no_run` fences, a correct
  quick-start, no `///;` lines, typed convenience methods where applicable).
  The author ruled (D53) against per-endpoint wrapper methods, so the last
  item is an ACCEPTED GAP.
  D49 also corrects two stale July figures: 35 endpoints declare pagination
  (all page-number: gitea 8, github 9, gitlab 9, bitbucket 9), and the
  "six endpoints POST empty bodies" figure is partly fixed (verified
  2026-10-11: text 0, binary 0, `ApiKeyParam` 0, header or cookie parameters
  0, multipart about 29).
- **D20 (whole-work done).** Done means: (1) all four landings pass their
  parent section 14 criteria; (2) every dependent crate (sniff,
  biscuit-speaks, model-citizen, `homelab/lib`, `eversolo-integration`,
  `arcam-amp-integration`, `sony-receiver-integration`,
  `unfolded-integration-helper`) builds and passes tests after each landing;
  (3) every defect-ledger row (parent section 10) and every recommendation
  (parent section 9) is closed or recorded as an accepted gap in the
  implementation log; (4) sniff retires `FocusedProviderClient`, uses the
  generated clients, and passes its own tests. The author closes the work.
  Whether an accepted gap is acceptable is the author's judgment.
- **D8 (WebSocket extent, consumed here).** WebSocket reuses the REST
  parameter, auth-policy, env-mapping, and header types; `WebSocketApi` stays
  its own type with its own generator. The AsyncAPI exporter exports what that
  model holds.
- **D14/D19/D42 (pagination, consumed here).** `PaginationStyle` was replaced
  by the model of spec 3.
- **D55-4 (vendor presets).** Each API owns its own small pagination strategy
  function in `schematic-definitions`. No preset moves and no shared preset
  layer exists. Spec 4a delivers this; R43 verifies it.
- **D55 (`?` in paths).** Spec 1 only warns about `?` in a path template. Spec
  4a owns the hard error and the migration of the nine endpoints (Gitea 1,
  GitLab 1, OpenAI 7). This spec verifies the error is in force and no `?`
  path remains.
- **D56 (consumed here).** The runtime trait is `EndpointSpec` (Decided).
  `auth_policy` is kept with a builder form and `oauth_scopes` is removed
  (D56-6). The response vocabulary of spec 5 includes the end-marker `keep`
  modifier and the `Bytes` framing. Definitions serialize through plain serde
  derives on `RestApi`/`Endpoint`/`EndpointParams`/`ParamDef`, with JSON
  definition snapshots from spec 4a. No credential resolves means anonymous
  (D56-1).
- **D57 (rulings on the second child-spec review; items 12, 19, 21 to 24
  apply here).** Item 12: container registrations such as `Vec<Commit>` are
  dropped (spec 4a); containers reuse the element's registration and export
  inline. Item 19: inbound WebSocket messages declare selectors on existing
  wire fields (`.matches(field, value)`), with at most one `.fallback()`
  message per direction (spec 6); the AsyncAPI exporter serializes the
  selectors. Item 21: WebSocket heartbeats are caller-managed (spec 6); this
  spec keeps the `HeartbeatHints` metadata on export. Item 22: only the auth
  details standard OpenAPI cannot express (a bearer header override, an
  API-key value prefix) are carried in the versioned `x-schematic` extension;
  `securitySchemes` stays authoritative for everything else. Item 23: Postman
  exports each GraphQL operation as a runnable `POST` with
  `{query, variables, operationName}`; OpenAPI skips it with a diagnostic;
  this spec owns the Postman form. Item 24: the WebSocket endpoint role
  (`Client`, `Host`, `Both`) from spec 6 is exported as `x-schematic-role` on
  the AsyncAPI channel and is part of R27's equality.
- **REST endpoint identity (D36 withdrawn).** Per the author ruling 2026-10-12 (`2026-10-11-schematic-generation-integrity` review 1, item 1),
  a REST endpoint is `(method, path)` and never a selector; D36's same-address
  selector pairs no longer exist. An endpoint with several response shapes
  exports as one OpenAPI operation listing each shape's content type (`oneOf`
  where shapes share one), with no selector extension. This spec's exporter
  changes must not regress that.
- **D9 (consumed here).** Partial generation regenerates a whole module group
  and keeps siblings. Feature flags and the single API list must work with it.
- **D53 (no wrapper methods).** The author ruled that no per-endpoint typed
  wrapper methods are generated. This is an accepted gap against D49 item 3 and
  recommendation 12. Reason: about 7 s of extra compile time per 550 wrapper
  instantiations, and typed requests already carry the response type.
- **D50 (plan structure).** This is child spec 7 of 8 (order: 1
  schematic-generation-integrity, 2 schematic-runtime-envelope, 3
  schematic-pagination, 4a schematic-definition-api, 4b
  schematic-definition-vocabulary, 5 schematic-response-model-graphql, 6
  schematic-websocket, 7 this spec). Each child leaves the
  repo green and ends at "implementation complete, ready for review".
- **Parent recommendations (D1: everything in scope).** Recommendations 7
  (constant headers), 8 (bridge), 9 (`x-schematic`), 10 (module-path
  resolvers, feature flags), 11 (single list), 12 (docs honesty; typed
  convenience methods are an accepted gap, D53), and 14 (README doctests; the
  vendor-preset item is verification only, R43) belong here.

## Requirements

Each requirement names how it is validated. Tests run only through the repo
recipes (`just test`, `just test-l2`, `just lint`, `just lint-schema`, and
`just test {pkg}` from the repo root), plus `just doctest` for Rust examples,
never raw `cargo test` or `cargo nextest`. Existing nextest recipes do not run
doctests; generated-crate docs compile through an extended `doctest` recipe or
the isolated-crate mechanism (R38).
"L1" means unit and isolated subprocess tests run by `just test` (run from
`schematic/`, covers `schematic-define --all-features`, `schematic-gen`,
`schematic-definitions`, `schematic-oauth`). "Isolated crate" means generating
into a temporary crate and compiling it without network access to providers,
the mechanism spec 1 built (D49). Nothing here is an L2 test; if an L2 test is
added it uses `just test-l2` and must never take window focus.

### OpenAPI schema bridge (D18, recommendation 8)

- **R1. Type arrays become typed, nullable schemas.** A schemars schema whose
  `type` is an array such as `["string","null"]` exports as a typed schema with
  `nullable: true` (OpenAPI 3.0) and not as `SchemaKind::Any`. Arrays with
  more than one non-null type export as `anyOf` of the typed members:
  `integer` overlaps `number`, so `oneOf` would reject valid integers.
  Nullability must preserve null and non-null values, including references
  and enums; `nullable` alone on an untyped composition wrapper is insufficient.
  Follow the [OpenAPI schema rules](https://spec.openapis.org/oas/v3.0.3#schema-object).
  Validated by: L1 unit tests in `schematic-definitions` (bridge) over
  hand-built schemars schemas, plus an export test over `Option<String>`,
  `Option<Vec<T>>`, `Option<Struct>`, a nullable enum, and overlapping integer/number unions.
  Check representative accepted and rejected values, not only schema shape.
- **R2. Enums keep their value domain.** String enums, integer enums, and
  single-value constants export as `enum` with the values. A schemars `const`
  exports as a one-value `enum`. Validated by: L1 unit tests (bridge) and one
  export test from a Rust enum with `#[serde(rename_all)]`.
- **R3. Composition survives.** `oneOf`, `anyOf`, and `allOf` export with their
  members converted recursively, with `$ref` members kept as refs.
  Validated by: L1 unit tests (bridge), including an internally tagged serde
  enum and a flattened struct. Combine constraints alongside composition or
  `$ref` rather than dropping them by early return. Preserve expressible
  bounds, lengths, patterns, formats, required fields, and defaults while
  recovering types. Unsupported validation keywords produce a diagnostic
  naming the schema path instead of silently becoming unconstrained.
- **R4. `additionalProperties` survives.** A map type exports as an object with
  `additionalProperties` set to the value schema; `true` exports as free-form;
  `false` is kept. Validated by: L1 unit tests (bridge), including
  `HashMap<String, T>` and `HashMap<String, serde_json::Value>`.
- **R5. Deliberate free-form is distinguishable from accidental loss.** The
  bridge must classify every property that exports as `SchemaKind::Any` as
  either deliberate (the schemars schema is `true`, an empty schema, or an
  object with no constraints, as `serde_json::Value` and foreign opaque types
  produce) or lost (the schemars schema carried `type`, `enum`, `const`,
  composition, or `additionalProperties` that the output dropped). A test
  fails on any lost case. There is no blanket ban on `SchemaKind::Any`.
  Validated by: L1 tests in `schematic-definitions` with paired fixtures (a
  `serde_json::Value` field passes; a field with a constraint exported as `Any`
  fails), and a scan over all 18 registries that fails if any property is
  classified lost. Walk array items, composition members, map values, and
  referenced components too, with a visited set. A typed output that dropped
  an input constraint is still loss; counting only `Any` nodes is insufficient.
- **R6. The untyped-property count is re-measured and recorded.** Before and
  after the bridge change, the implementer records the per-API count of
  untyped properties (the July figures are the baseline to compare, not to
  trust). After the change the count of *lost* properties across the 18
  registries is zero; deliberate free-form properties are listed in the
  implementation log. Validated by: the scan in R5, plus the log entry.
- **R7. Nested `$ref` completeness.** `validate_completeness` (or a new check
  it calls) walks the transitive `$ref` closure of every registered schema and
  of every schema an endpoint names, and reports each reference that no
  registered schema satisfies, with the parent schema, the property path, and
  the missing name. Resolve against emitted components, including nested
  definitions extracted from registered roots; these need no separate manual
  registration. Track visited references so recursive models terminate, decode
  JSON-pointer escapes, and diagnose unsupported external references without
  fetching them. A failure blocks generation and export. Validated by: L1
  unit tests with a genuinely unresolved nested reference (fails with all
  facts named), an extracted nested definition, and a complete recursive model (pass), plus a pass over the 18
  shipped registries. If spec 4a's typed registration already registers nested
  types automatically, the test asserts the closure holds and the log records
  that the check is now a backstop.
- **R53. Containers export inline, never as named components (D57 item 12).**
  Spec 4a drops container registrations such as `Vec<Commit>` and lists the
  removal in its R24 JSON comparison. The OpenAPI export reuses the element's
  registration and writes the container inline (for example
  `{"type":"array","items":{"$ref":"#/components/schemas/Commit"}}`); no
  component key names a container or contains `<`. The R5 scan and the R7
  closure walk treat such inline arrays like any other schema. Validated by:
  an L1 export test over an endpoint returning `Vec<T>` (inline array with an
  element `$ref`, no container component), plus an assertion over the 18
  registries' exported `components.schemas` keys.

### Required constant headers (D46, recommendation 7)

- **R8. A required constant header exports as an OpenAPI parameter.** A header
  declared on an API or endpoint with a fixed value exports as a parameter with
  `in: header`, `required: true`, and a schema that is a string `enum` with the
  single fixed value (so `anthropic-version` appears with its value).
  Validated by: L1 export test in `schematic-gen` over a fixture API, and an
  assertion over the committed Anthropic artifact.
- **R9. Precedence is endpoint over API, with no duplicates.** When an API-level
  and an endpoint-level header have the same name (case-insensitive), the
  endpoint value wins and exactly one parameter is emitted. A header that is
  also a credential header (covered by a `securitySchemes` entry) is not
  emitted as a parameter. Use effective operation auth to identify credential
  headers. `Accept` and `Content-Type` use response/request content, and
  `Authorization` uses security: OpenAPI ignores parameters with these names.
  Preserve extra fixed-header detail in the extension and diagnose any gap.
  Fixed ordinary headers also supply a schema default. Merge a matching
  ordinary header parameter or diagnose incompatible declarations. Validated
  by: L1 fixtures for precedence, effective auth override, reserved headers,
  and collisions. See the [OpenAPI parameter contract](https://spec.openapis.org/oas/v3.0.3#parameter-object).

### `x-schematic` round trip (D18, recommendation 9)

- **R10. The importer reads `x-schematic` for the supported subset.** The
  supported subset is the set in P10-b below. For each field in the subset,
  export, then import, then export yields a semantically equal definition and
  document: ordering and normalization may differ; meaning may not. The subset
  includes the D56 end-marker `keep` modifier and the `Bytes` framing from
  spec 5's response vocabulary. Definition serialization tests, and the
  round trip where it compares definitions, build on the JSON definition
  snapshots and plain serde derives (`RestApi`, `Endpoint`, `EndpointParams`,
  `ParamDef`) from spec 4a rather than a new serialization form.
  Validated by: L1 round-trip tests in `schematic-define` / `schematic-gen`
  over the committed OpenAPI artifacts and over purpose-built fixtures; the
  assertion compares parsed values, not bytes.
- **R11. Extension version handling.** Exported extensions carry a version
  (P10-b default: `x-schematic.version`, integer major, starting at 1). The
  importer accepts the current major, ignores unknown fields within it,
  and on any unsupported major ignores all Schematic extensions in the
  document and emits a WARN diagnostic. A malformed version behaves likewise.
  An absent version means major 1. The document-level version governs operation
  metadata too; operation structs add no version.
  Decode known fields independently so one malformed field does not discard
  unrelated valid metadata.
  Validated by: L1 import tests for each of the four cases.
- **R12. Standard fields win over the extension.** When an extension value
  conflicts with the equivalent standard OpenAPI field (for example an
  extension `response` against the operation's real responses), the standard
  field is used and a WARN diagnostic names the location. The extension fills
  only what OpenAPI cannot express. Validated by: L1 import tests with a
  deliberately conflicting fixture.
- **R13. Unsupported extension data produces a diagnostic.** Extension keys
  outside the subset, or values that fail to deserialize, produce an INFO
  (unknown key) or WARN (malformed value) diagnostic carrying the JSON-pointer
  location, and never panic or abort decoding unrelated fields. Strict mode may reject
  the resulting WARN as required by the strict-mode rule. Validated by: L1
  import tests in both permissive and strict modes.
- **R14. The re-import defect is reproduced, then fixed or retired.** The
  implementer first writes a reproduction test that exports a definition with empty
  or absent optional collections and imports the result (ledger row 14). If it
  fails, fix with `#[serde(default)]` where required and keep the test. If it
  does not reproduce (the `Option` fields in `extensions.rs` already
  deserialize when missing), keep the test as a regression guard and record row
  14 as "not reproducible, guarded" in the ledger table. Validated by: the L1
  test itself.
- **R15. The two dead extension structs are deleted or wired.**
  `SchematicSchemaExtension` and `AuthExtension` are either wired into the
  export and import (and covered by R10) or deleted with their rustdoc and
  public re-exports. `AuthExtension` is not wired as a whole (D57 item 22);
  the auth details it would have carried that OpenAPI cannot express live in
  the versioned extension instead (R54). Default: delete both unused structs
  once that mapping is in place. Every doc, comment, and file that
  claims a round trip is made accurate for the supported subset (the three
  files named in the parent: `extensions.rs`, `export/mod.rs`, and
  `schematic/docs/io/openapi-extensions.md`). Validated by: L1 compile check
  (the symbols are absent or used), and a grep recorded in the log.
- **R54. Auth details OpenAPI cannot express round-trip in `x-schematic`
  (D57 item 22).** `securitySchemes` and operation `security` stay
  authoritative for the auth method, credential location, name, and scopes.
  Only two details are carried in the versioned `x-schematic` extension: a
  bearer header override (a bearer token sent in a header other than
  `Authorization`) and an API-key value prefix. No other auth field, and no
  credential value, is written to the extension. These details are part of
  R10's definition equality, not excluded from it. A conflict between the
  extension and the standard scheme follows R12 (standard wins, WARN); a
  malformed value follows R13. Proposed: the details sit at the level where
  the auth is declared (document level for the API's auth, operation level
  for an endpoint override). Validated by: L1 round-trip tests over fixtures
  with an API-key value prefix (the prefix survives export and import) and a
  bearer header override; a conflicting fixture (standard wins, WARN); and an
  assertion that the extension holds no auth field beyond these two.

### Import fidelity (recommendation 9 context; ledger row 17)

- **R16. Strict import mode fails on warnings and errors, not INFO.**
  `ImportOptions::strict` fails the import when any diagnostic of severity WARN
  or ERROR exists and passes when only INFO exists. Its doc is corrected. The
  tautological test (ledger row 17) is replaced by tests with INFO-only,
  WARN-only, and ERROR cases. Validated by: L1 tests in `schematic-define`.
- **R17. Path-item-level parameters are imported.** Parameters declared on a
  path item are merged into every operation under it; an operation-level
  parameter with the same name and location overrides the path-item one.
  Resolve local parameter references before merging. Identity is name plus
  location; header names compare case-insensitively. Unresolvable references
  produce diagnostics. Validated by: L1 fixtures with inline and referenced
  parameters, operation overrides, and unresolved references.
- **R18. Multipart request bodies are imported.** `multipart/form-data` bodies
  import as the multipart `ApiRequest` with `FormField`s (names, required,
  file versus text, `accept` where declared) and no longer as `request: None`.
  Export of the result reproduces an equivalent multipart body. Validated by:
  L1 import and round-trip tests, using the ElevenLabs `AddVoiceSample`
  endpoint shape as a fixture. Read part media types from
  `encoding.<field>.contentType`; retain arrays of files where supported.
  Object-valued parts and encoding details the model cannot represent produce
  diagnostics rather than silently becoming text.

### Postman (recommendation 7.3 items; D18)

- **R19. Example bodies come from the schema registry.** For each JSON-bodied
  endpoint with a registered request schema, the collection body is a
  generated JSON document that validates against that schema. Output is
  deterministic (same input, same bytes). Value rules are P10-c. Validated by:
  L1 test in `schematic-gen` that generates bodies for every JSON-bodied
  endpoint of all 18 APIs and validates each against its schema with a JSON
  Schema validator (a dev-dependency; record the choice), against the original
  registered JSON Schema with local definitions, not the OpenAPI conversion.
  Unsupported or unsatisfiable schemas return a diagnostic naming the endpoint
  and schema path, never an invalid example. Every shipped JSON request needs
  a valid generated or authored example, or a specifically recorded accepted gap.
- **R20. Environment files.** Each exported collection can be accompanied by a
  Postman environment file declaring the variables the collection uses (base
  URL, auth variables already implied by the auth strategy). Validated by: L1
  test that every `{{variable}}` in a collection is declared in the collection
  or its environment file (extends the existing variable-consistency test).
  Include path/query/header/cookie placeholders and distinct server/auth
  variables for members of shared-module collections. Credentials have empty
  values; never read secrets from the host environment. Environment output
  uses an explicit optional export switch; collection variable declarations
  are always complete. Generated environments participate in drift checks.
- **R21. Saved example responses.** Each endpoint carries a saved example
  response generated from its declared success schema, with status and
  content type, and (if spec 5's response model v2 declares error shapes) one
  per declared error shape. Validated by: L1 test: each saved response body
  validates against its schema. Emit success media types separately and omit
  bodies for empty responses. Binary examples are documented placeholders.
  Stream examples are finite and respect framing/end markers; do not validate
  a complete stream as one JSON object. Error ranges use a deterministic
  concrete status whose resolved error shape matches, honoring narrower overrides.
- **R22. OAuth2 keeps its type.** An OAuth2 auth strategy exports as Postman
  `oauth2` auth with the grant's URLs and scopes where the definition has them,
  and no longer collapses to `bearer`. Validated by: L1 golden test.
- **R23. Text and Binary bodies keep content types.** Text bodies export with a
  raw body and the declared `Content-Type`; binary bodies export with the file
  body mode and the declared `Content-Type` header. Validated by: L1 test with a
  fixture API.
- **R24. Header and cookie parameters export.** Endpoint header parameters
  become request headers and cookie parameters become `Cookie` header entries,
  with variables for values. Merge cookies into one `Cookie` header using
  runtime encoding rules; apply constant-header precedence and credential
  exclusions. Disable optional parameters unless an explicit example/default
  supplies a value. Validate combined cookies, optional values, and collisions
  using L1 fixtures.
- **R55. GraphQL operations export to Postman and are skipped by OpenAPI (D57
  item 23).** Spec 5 places typed GraphQL operations inside `RestApi`; this
  spec owns their Postman form. Postman exports each operation as a runnable
  `POST` to the resolved GraphQL URL whose JSON body is
  `{query, variables, operationName}`, with the authored document as
  `query`. The OpenAPI exporter skips each GraphQL operation and emits a
  diagnostic naming it; it does not present operations as separate HTTP
  paths and adds no GraphQL extension. GraphQL operations are therefore
  outside the OpenAPI round-trip subset (R10) and listed as such. Proposed:
  `variables` is an example built per P10-c from the operation's variables
  schema where one is registered, otherwise `{}`. Validated by: L1 tests in
  `schematic-gen` over a fixture with GraphQL operations: one Postman request
  per operation with the expected method, URL, and body keys; the OpenAPI
  document contains no path for them and the export reports one diagnostic
  per skipped operation.

### AsyncAPI (D47)

- **R25. An AsyncAPI exporter exists for WebSocket APIs.** Given a
  `WebSocketApi`, it emits an AsyncAPI document (version per P10-a) with
  servers, channels, and the typed messages and their schemas, for all five
  WebSocket APIs, deterministically and with no dangling `$ref`. Validated by:
  L1 tests in `schematic-gen`: structure and `$ref`-closure assertions for the
  five APIs, plus a schema validation of the output against the AsyncAPI JSON
  Schema if a small dev-dependency or vendored schema is available
  (otherwise recorded as unverified).
- **R26. The importer persists its returned definition and catalog.**
  The `schematic-gen` [AsyncAPI import pipeline](../../gen/src/asyncapi_import.rs)
  already returns both. Write reusable definition/model output through the
  staged import-output contract used by OpenAPI, retaining the in-memory
  result and diagnostics. Dry run writes nothing. Validated by: an L1
  temporary-directory test that reads saved output back and checks the
  definition/catalog, diagnostics, and absence of dry-run writes.
- **R27. Export then import is semantically equal.** For each of the five
  WebSocket APIs, exporting then importing yields a `WebSocketApi` equal in
  meaning (channels, messages, directions, message schemas, inbound message
  selectors and the fallback message (R56), endpoint role (R57), auth,
  parameters) to the original, within the supported subset (P10-a).
  Validated by: L1 round-trip test per API, including a fixture whose
  messages have overlapping payloads distinguished only by selectors and
  fixtures for each role (`Client`, `Host`, `Both`); differences outside the
  subset are listed in the log.
- **R28. A generate recipe and committed artifacts.** A `just generate-asyncapi`
  recipe in `schematic/justfile` (modeled on `generate-openapi` and
  `generate-postman`) writes the documents to `schematic/asyncapi/` (P10-a
  default) and an artifact-drift test (modeled on `artifact_drift.rs`) fails
  when committed files differ from fresh output. Validated by: L1 drift test.
- **R56. Inbound message selectors are exported and re-imported (D57 item
  19).** Spec 6 has each message declare selectors on existing wire fields
  (`.matches("kind", "resp").matches("msg", "pong")`), with at most one
  `.fallback()` message per direction; untagged declaration-order decoding is
  not used. The AsyncAPI exporter serializes every message's selectors and
  its fallback marker, and the importer reads them back so the re-imported
  definition selects the same message for the same frame. Proposed: a
  Schematic extension on the message (for example `x-schematic-selectors`, a
  list of field/value pairs, and `x-schematic-fallback: true`); the name and
  shape are logged. An imported document with more than one fallback message
  in a direction is a diagnostic, never a silent pick. Validated by: L1
  round-trip tests (part of R27) over a fixture with overlapping payloads,
  and an importer test for the duplicate-fallback diagnostic.
- **R57. The endpoint role is exported and re-imported (D57 item 24).** The
  role spec 6 introduces (`Client`, `Host`, `Both`) is exported as
  `x-schematic-role` on the AsyncAPI channel and read back by the importer;
  it is part of R27's equality. The document still describes the client
  application for every role (P10-a); a host role is not exported from the
  server's view. Validated by: L1 round-trip tests (part of R27) with one
  fixture per role.
- **R58. Heartbeat hints are kept on export (D57 item 21).** Heartbeats are
  caller-managed (spec 6), but a definition's `HeartbeatHints` metadata is
  written to the AsyncAPI document rather than dropped. Proposed: a Schematic
  extension on the channel or message where the hint is declared, read back
  by the importer and compared by R27; if read-back is not built, the loss is
  listed in the log as an R27 difference. Validated by: an L1 export test
  over a fixture declaring `HeartbeatHints`.

### Generated crate packaging (D18, D46, recommendation 10, 11)

- **R29. One Cargo feature per API module, default features include all.**
  The generated `schematic-schema` manifest declares a feature per API module
  (P10-d); the default feature set enables all of them; the transport
  dependencies a module needs (for example `tokio-tungstenite` for WebSocket
  modules) are optional and enabled by that module's feature. Validated by:
  L1 test on the generated `Cargo.toml` text, and the isolated-crate check
  below.
- **R30. A single-feature build exposes only that module group.** Building the
  generated crate with `--no-default-features --features <one module>` compiles,
  and APIs outside that module group are absent. Validated by: isolated-crate compile
  check for at least one REST module, one WebSocket module, and one
  shared-module group (two APIs in one module, per D9), plus an item-absence
  assertion. Also compile with no features and a REST/WebSocket feature pair:
  features are additive. Gate modules, prelude re-exports, WebSocket shared
  helpers, and module-dependent docs consistently. Inspect the dependency
  graph to verify a REST-only build does not enable tungstenite. Dependencies
  still needed by the shared runtime may remain enabled.
- **R31. Dependents keep working unchanged.** sniff, biscuit-speaks,
  model-citizen, `homelab/lib`, `eversolo-integration`, `arcam-amp-integration`,
  `sony-receiver-integration`, and `unfolded-integration-helper` (eight
  dependents in all) build and pass their tests with
  no `Cargo.toml` change, because default features include all APIs.
  Validated by: building and testing each dependent through repository recipes
  (record commands and results). A dependent that
  needs a manifest change is a failure of this requirement, not a reason to
  edit it.
- **R32. Features and partial generation agree.** Partial generation (D9)
  regenerates the manifest from the full inventory, so the feature set is the
  same whether the run generated all APIs or one group. Validated by: L1 test
  comparing manifests from a full and a partial run.

### Single API list and completeness gate (D46, recommendation 11; ledger row 15)

- **R33. The API lists are enumerated first.** Before changing anything the
  implementer lists every place that holds a hand-maintained list of APIs (the
  parent says seven but does not name them) and records them in the log. The
  starting inventory is **six runtime lists plus test tables**: the runtime
  lists are `definitions/src/lib.rs:224` (`apis_by_module`),
  `registry/lookup.rs:41-56` and `:87-103`, and `gen/src/pipeline.rs:21`,
  `:49-65`, `:94-114`; the test table is the strict-completeness table at
  `gen/tests/l1/openapi_strict_completeness.rs:176-193` (16 names; the real
  path is under `tests/l1`), and a candidate test table at
  `registry/mod.rs:744`. Line numbers are July/October evidence and will have
  moved; the implementer re-derives them. Endpoint literals number about 519
  (re-measure; any older figure such as 253 is superseded). Validated by: the
  log entry (not tool-testable).
- **R34. One generated list replaces them.** One authoritative registration inventory, from which
  `apis_by_module()` is derived (P10-e), feeds every consumer in the
  enumeration found in R33: registry construction, generator input, OpenAPI and
  Postman export, the strict-completeness gate, and test lists. Adding an API
  means one registration step. Entries bind selection names/aliases, API
  identity, module group, protocol, definition construction, and registry
  construction. `RestApi` values alone cannot derive registry constructor
  functions or discover WebSocket APIs. Include WebSocket entries for
  packaging and AsyncAPI; REST checks filter explicitly by protocol. Preserve
  existing selection aliases and ordering. Validated by: L1 test that adds a fixture API in
  one place and observes it in every consumer; compile-level check that no
  second hand-written list remains (grep recorded in the log).
- **R35. The strict-completeness gate cannot skip an API silently.** The gate
  covers all 18 APIs (the current table names 16), and a test fails if the
  gate's covered set differs from `apis_by_module()`. Any deliberate skip is an
  explicit, named, reasoned entry in one place. Validated by: L1 test in
  `schematic/gen/tests/l1/openapi_strict_completeness.rs`.

### One module-path resolver (D49, ledger row 22)

- **R36. A single resolver serves Rust, OpenAPI, and Postman.** One function
  maps an API (with or without an explicit `module_path`) to its module name
  and file stem, used by the Rust assembler, the OpenAPI writer, and the
  Postman writer. The other implementation is deleted. Validated by: L1 test
  feeding `PagerDutyApi`-style names (no explicit `module_path`) and asserting
  the Rust, OpenAPI, and Postman outputs all use the same stem; plus a test
  over the 18 real APIs asserting the committed file names are unchanged or the
  change is listed (P10-f).
- **R37. Renamed artifacts are handled once.** If unifying changes any
  committed artifact file name, the regeneration renames them in the same
  change, the old files are removed, and the drift tests pass. Validated by:
  L1 drift tests.

### Generated documentation honesty (D49, ledger row 8, recommendation 12)

- **R38. Doc fences compile.** Generated executable Rust code fences are
  `no_run` and compile as doctests. Validated by: a doc-compile
  test over the generated crate, run through the extended
  `just doctest` recipe or the isolated-crate mechanism, not raw
  `cargo test`. `just test-schema` currently uses nextest and does not compile
  doctests. Only executable Rust examples become `no_run`; prose, shell, and
  JSON retain suitable languages. Check default features and a selected module
  so docs do not import disabled modules.
- **R39. No doc mentions a method that does not exist; no wrapper methods are
  generated (D53, ACCEPTED GAP).** The generated docs and the quick-start use
  `client.send(Request::new(..))` and contain no method that does not exist
  (for example no `client.list_models()`). Per-endpoint typed wrapper methods
  are **not** generated: the author ruled (D53) this an accepted gap against
  D49 item 3 and recommendation 12. Reason: about 7 s of extra compile time per
  550 wrapper instantiations, and typed requests already carry the response
  type. Validated by: R38's doc-compile test, plus a grep assertion that the
  generated docs name no method absent from the generated client.
- **R40. The `ApiKey` doc literal is complete.** The emitted
  `AuthStrategy::ApiKey` literal includes every required field (`value_prefix`
  and any added by earlier specs). Validated by: R38's doc-compile test.
- **R41. No `///;` lines.** None remain in the generated output, and the
  generator cannot emit one. Validated by: a grep assertion in an L1 test over
  freshly generated output.
- **R51. Combined-module docs are not lost (emqx).** When several APIs share one
  generated module (D9), the module-level docs of the generated module carry
  the docs of every API in it, not only one. The implementer first measures the
  current loss on emqx (the exact symptom is **unverified** here), then fixes
  the generator. Validated by: an L1 test over freshly generated output
  asserting the combined module's docs name each member API.

### `schematic-define` hygiene (D18, recommendation 14, ledger row 19)

- **R42. The README compiles as doctests.** `schematic-define` includes its
  README with `#![doc = include_str!("../README.md")]` (or an equivalent that
  makes the examples run), every example compiles or is explicitly `ignore`d
  with a stated reason, and a drifted example fails `just doctest`. Examples that
  describe vocabulary changed by specs 3 to 5 (including 4a and 4b) are rewritten to the current API.
  Validated by: doctests in `schematic-define` via `just doctest`.
  Use `no_run` for network/credential examples so they still compile; reserve
  `ignore` for a stated reason that prevents compilation itself.
- **R43. Verify vendor pagination constructors are gone (D55-4).** Each API owns
  its own small strategy function in `schematic-definitions`, delivered by spec
  4a. This spec moves no presets. It verifies that the public API of
  `schematic-define` has no vendor-named pagination constructor (`github()`,
  `gitlab()`, `bitbucket()`, `gitea()`) after spec 4a, and that no shared
  preset layer (a common module or crate of vendor presets) was introduced in
  `schematic-definitions` or elsewhere. Generated output and exported OpenAPI
  for the four APIs are unchanged by this spec. Validated by: a compile-level
  check or grep (absence from the define API, recorded in the log), a grep for
  a shared preset layer, and the artifact-drift tests.
- **R52. Verify `?` in path templates is a hard error and none remains
  (D55).** Spec 1 only warns; spec 4a owns the hard error and the migration of
  the nine endpoints (Gitea 1, GitLab 1, OpenAI 7). This spec verifies the
  error is in force and that no registered endpoint path contains `?`. Validated
  by: an L1 scan over all registered endpoints (or a recorded grep plus the
  existing error test from spec 4a), and the log entry.

### Ledger disposition and whole-work done (D20)

- **R44. Remaining ledger rows are closed or recorded.** Rows 14, 15, 17, 19
  are closed by R14, R35, R16, R42. Row 21 (clippy warnings in the generated
  crate, marked FIXED in the parent) is verified by `just lint-schema` passing
  with `-D warnings` after regeneration. Row 22 is closed by R36. Rows 7, 8, 13
  are closed by R7, R38 to R41, R51, R10 to R15. Row 18 is verified by R50. Any
  row that cannot be closed is recorded as an accepted gap with a reason.
  Validated by: the disposition table (R45).
- **R45. A final ledger disposition table is written.** The implementation log
  contains one table with a row for **every** one of the 22 defect-ledger rows
  (parent section 10) and **every** one of recommendations 1 to 14 (parent
  section 9), each mapped to the child spec that closed it (by number and
  name, with the evidence: test name, command, or log entry) or to an accepted
  gap with its reason. The table also lists the stale July figures D49
  corrected. The table must record these as **ACCEPTED GAPS**, each with its
  reason: the optional-auth variant (Ollama); the legacy-versus-modern auth
  dual-source cleanup; invented HuggingFace stub shapes; orphaned types; typed
  convenience methods (D53, R39). It must map: `Schema::module_path` to spec
  4a; `?` in path templates to specs 1 (warning) and 4a (hard error and
  migration of the nine endpoints), verified here (R52); removal of the old
  pagination types and conversion to spec 4a; the emqx combined-module docs
  loss to this spec (R51); vendor pagination constructors to spec 4a (each API
  owns its own strategy function, D55-4), verified here with no move (R43);
  `oauth_scopes` as removed in spec 4a (D56-6), with the `x-schematic`
  `oauth_scopes` copy deleted accordingly. Validated by: self-check that 22 + 14
  rows are present and none is blank (a script or checklist; not committed as
  a test, because the log moves when the spec closes). The starting assignment
  is in "Ledger and recommendation starting map" below.
- **R46. The whole-work done checklist is evaluated and recorded.** For each
  of D20's four parts, the log records the current state with evidence:
  (1) the parent section 14 criteria per landing, pointing to each child spec's
  log; (2) every dependent crate builds and passes its tests now (commands and
  results); (3) the disposition table; (4) the sniff check in R47, which includes sniff retiring
  `FocusedProviderClient` under sniff's own spec. A part that
  does not hold is reported as open, not closed.
- **R47. The sniff check is run and reported, never faked.** The implementer
  searches the sniff tree for `FocusedProviderClient` and reads the status of
  sniff's own spec (`2026-10-09-consistent-use-schematic`, resolved under sniff's features/fixes
  directories, including lifecycle subdirectories). If any
  reference remains, or that spec is not at "implementation complete", the log
  records D20 part 4 as **open** with the grep output; this spec still reaches
  "ready for review". The implementer does not edit sniff. Validated by: the
  grep and the log entry.
- **R50. The stale `gen/schematic` tree is verified gone.** Deleting the stale
  tracked tree `schematic/gen/schematic/` is spec 1's only. This spec verifies
  it is absent and the check **fails** if it exists; it does not delete it.
  Validated by: an L1 test (or recipe check) asserting the path does not exist,
  plus the log entry (ledger row 18).

### Documentation and drift

- **R48. Docs, README, and skills match the new behavior.** Updated in the same
  change: `schematic/README.md`; the `schematic/docs/` pages for the OpenAPI
  and Postman I/O (`docs/io/openapi-extensions.md`, `export-openapi.md`,
  `import-openapi.md`, `export-postman.md`, `crates-openapi.md`), a new topic
  page for AsyncAPI export, and the feature-flag and single-list behavior; and
  the affected skills (`.claude/skills/schematic-define/` and any
  `create-schematic-api` guidance that lists the per-API registration steps).
  `docs/` pages describe behavior, never link to or name a feature or fix, use
  a Mermaid diagram where a flow is easier to see (the round trip and the
  generation pipeline), and are written for a developer new to the repo.
  Validated by: review of the diff; the comments/docs drift pass of R49.
- **R49. Comment and doc drift pass.** Every symbol whose behavior changed has
  its `///` and `//!` docs and inline comments checked in the same change; where
  code and comment disagree, the code is assumed right and the comment fixed or
  deleted, and the drift is reported in the log. Validated by: diff review.

## Design

"Decided" means the parent decided it (cited). "Proposed" is this spec's
proposal; the implementer may adapt it and logs the change. Paths are
repo-relative. Symbol names marked Proposed are illustrative.

| Symbol or file | Package | What it does | Status |
|---|---|---|---|
| Schemars-to-OpenAPI bridge, `registry/conversion.rs` | `schematic-definitions` | Converts type arrays, `enum`/`const`, `oneOf`/`anyOf`/`allOf`, `additionalProperties`; keeps genuine free-form as `Any` (R1 to R4); exports containers inline with an element `$ref` (R53) | Decided (D18, rec. 8): the fixes; containers inline (D57 item 12). Proposed: structure |
| `classify_any(...)` and a lost-versus-deliberate scan test | `schematic-definitions` | Tells deliberate free-form from lost constraint (R5, R6) | Proposed |
| `validate_completeness`, `registry/mod.rs` | `schematic-definitions` | Walks the transitive `$ref` closure (R7) | Decided (ledger row 7): the need. Proposed: the walk |
| Constant-header parameter mapping, `define/src/openapi/export/` | `schematic-define` | Emits required constant headers as `in: header` single-value-enum parameters, endpoint over API (R8, R9) | Decided (D46, rec. 7) |
| `SchematicDocExtension`, `SchematicOpExtension`, `extensions.rs` | `schematic-define` | Gain a document-level version and field defaults; read by the importer (R10 to R14) | Decided (D18, rec. 9): read or strike. Proposed: the version field |
| `SchematicSchemaExtension`, `AuthExtension` | `schematic-define` | Deleted, or wired (R15) | Decided: "delete or wire"; `AuthExtension` not wired as a whole (D57 item 22). Proposed: delete |
| Auth details in the versioned extension (bearer header override, API-key value prefix) | `schematic-define` | Carries only what `securitySchemes` cannot express (R54) | Decided (D57 item 22). Proposed: placement level |
| Import pipeline: extension reader, path-item parameter merge, multipart mapping, strict-mode severity rule, `define/src/openapi/import/` | `schematic-define` | R10 to R18 | Decided (rec. 9; ledger rows 13, 14, 17): the defects. Proposed: structure |
| Postman exporter, `gen/src/export/postman/` (examples, environment, saved responses, OAuth2, bodies, parameters) | `schematic-gen` | R19 to R24; receives the schema registry | Decided (D18): the items. Proposed: structure |
| Postman GraphQL operation export; OpenAPI skip diagnostic | `schematic-gen` (Postman), `schematic-define` (OpenAPI exporter) | Runnable `POST` with `{query, variables, operationName}`; OpenAPI skips with a diagnostic (R55) | Decided (D57 item 23). Proposed: `variables` example source |
| `ExampleValueSource`-style generator | `schematic-gen` | Builds a JSON value from a schema, deterministic (R19, P10-c) | Proposed |
| AsyncAPI exporter, new `gen/src/export/asyncapi/` | `schematic-gen` | Exports a `WebSocketApi` to AsyncAPI (R25), with message selectors and fallback (R56), `x-schematic-role` on each channel (R57), and heartbeat hints (R58) | Decided (D47): existence; selectors serialized (D57 item 19); `x-schematic-role` (D57 item 24); hints kept (D57 item 21). Proposed: module layout, selector and hint extension names |
| `asyncapi_import.rs` | `schematic-gen` | Persists the already-returned definition and catalog (R26); reads back selectors, fallback, and role (R56, R57) | Decided (parent section 8); read-back of selectors and role (D57 items 19, 24) |
| `just generate-asyncapi`, `schematic/asyncapi/`, drift test | `schematic` (justfile), `schematic-gen` | Writes and guards the documents (R28) | Proposed |
| Manifest generation, `gen/src/cargo_gen.rs` | `schematic-gen` | Emits per-API features, optional deps, default = all (R29, R32) | Decided (D18): per-API flags. Proposed: granularity (P10-d) |
| Single registration inventory, with `apis_by_module()` derived from it | `schematic-definitions` (source), consumers in `schematic-gen` | Replaces the six runtime lists and the test tables (R33 to R35) | Decided (D46): one generated list. Proposed: mechanism (P10-e) |
| Module-path resolver | `schematic-definitions` (owner), used by `schematic-gen` | One function for Rust, OpenAPI, Postman names (R36, R37) | Decided (D49): unify. Proposed: location |
| Doc emission, `gen/src/output/assemble/lib_rs.rs` and `codegen/client/` doc generation | `schematic-gen` | `no_run` fences, correct quick-start, complete `ApiKey` literal, no `///;` (R38 to R41) | Decided (D49) |
| `#![doc = include_str!("../README.md")]` and README rewrites | `schematic-define` | R42 | Decided (D18) |
| Vendor pagination constructor and `?`-path verification | `schematic-define`, `schematic-definitions` | R43, R52; verification only, nothing moves (D55) | Decided (D55-4) |
| Ledger disposition table | the implementation log of this feature | R45 to R47 | Decided (D20) |

## Suggested phases

The plan may refine these. Each phase ends with `just test` (schematic) and
`just lint` green, with the generated crate regenerated and `just lint-schema`
green whenever generation changed.

1. **Measure and enumerate.** Re-measure untyped properties per API (R6);
   enumerate the API lists (R33); verify the stale tree is gone (R50); measure
   the emqx docs loss (R51); write the reproduction test for row
   14 (R14); record baselines in the log. No behavior change.
2. **Schemars bridge and nested `$ref` check.** R1 to R7, R53. Regenerate OpenAPI
   artifacts; review the diff is only intended type recovery.
3. **Constant headers.** R8, R9. Regenerate OpenAPI artifacts.
4. **Importer and `x-schematic`.** R10 to R18, R54. Delete or wire the dead
   structs; fix docs that claim a round trip.
5. **Single API list and the completeness gate.** R33 to R35. Mechanical, but
   done before the module-path and feature-flag work, which both enumerate APIs.
6. **Module-path resolver unification.** R36, R37.
7. **Per-API feature flags.** R29 to R32. Regenerate `schematic-schema`;
   build every dependent crate.
8. **Generated-docs honesty pass.** R38 to R41, R51. Regenerate.
9. **Postman.** R19 to R24, R55. Regenerate collections.
10. **AsyncAPI.** R25 to R28 and R56 to R58 after spec 6's WebSocket types are confirmed.
11. **README doctests and verifications.** R42; verify R43 (no vendor-named
    pagination constructor, no shared preset layer) and R52 (`?` error in
    force, no `?` path remains).
12. **Docs, skills, drift pass.** R48, R49.
13. **Ledger disposition and whole-work done.** R44 to R47; final full run of
    every acceptance command.

## Acceptance criteria and definition of done

Done means all of the following, with evidence in `implementation-log.md`:

- Every requirement R1 to R58 is met, or listed in the log as a deviation or
  accepted gap with a reason.
- The build and tests pass on macOS, Linux, native Windows, and WSL2 per the
  repository rules. The implementer runs what the current host can run and uses
  the `os` skill (`.claude/skills/os/`) to reach other hosts or records, with
  evidence, which environments could not be exercised and why. File-name and
  path handling in the resolver (R36), the manifest (R29), and the new output
  directories must not assume `/` or a case-sensitive file system.
- `just test`, `just doctest`, and `just lint` pass for the schematic area; `just lint-schema`
  passes (clippy with `-D warnings` on the generated crate); `just test-schema`
  passes (dependents are tested with `just test {pkg}` from the repo root);
  and `just test-l2` is run only if an L2 test was touched (it must
  never take window focus).
- `schematic-schema` is regenerated by the generator, never hand-edited, and a
  second regeneration produces no diff (determinism). OpenAPI, Postman, and
  AsyncAPI artifacts are regenerated and the drift tests pass.
- Every dependent crate builds and passes its tests: sniff, biscuit-speaks,
  model-citizen, `homelab/lib`, `eversolo-integration`, `arcam-amp-integration`,
  `sony-receiver-integration`, `unfolded-integration-helper`.
- AsyncAPI export and re-import keep each endpoint's role and each inbound
  message's selectors and fallback marker (R56, R57, D57 items 19 and 24);
  heartbeat hints appear in the exported document (R58, D57 item 21).
- An OpenAPI round trip keeps an API-key value prefix and a bearer header
  override, and the `x-schematic` extension carries no other auth field
  (R54, D57 item 22).
- Each GraphQL operation exports to Postman as a runnable `POST` with
  `{query, variables, operationName}`, and the OpenAPI export skips it with a
  diagnostic (R55, D57 item 23).
- No exported OpenAPI component names a container; containers export inline
  with an element `$ref` (R53, D57 item 12).
- The README, `schematic/docs/` topic pages, and affected skills in
  `.claude/skills/` are updated for changed behavior (R48), and the
  comment/docs drift pass is done (R49).
- The ledger disposition table (R45) and the whole-work done checklist (R46)
  are written, and the sniff check (R47, including sniff retiring
  `FocusedProviderClient` under sniff's own spec) is reported honestly.
- No commits are made unless the user explicitly requests them. A plan alone
  does not grant authorization. When explicitly requested: author name and email, signed, no agent attribution in the
  message, and `git verify-commit HEAD` after each commit.
- The agent's terminal state is "implementation complete, ready for review".
  It does not run `just complete` and does not move any spec to `_completed`.

## Ledger and recommendation starting map

A starting point for R45, from the parent's own assignments. The implementer
verifies each against the earlier specs' implementation logs and corrects it.
Spec numbers are the parent's D50 numbering: 1 generation integrity, 2 runtime
and envelope, 3 pagination, 4a definition API (builders and typed
registration; owns removal of the old pagination types and the
`Schema::module_path` disposition), 4b definition vocabulary (typed path
params, auth override, `deprecated`, rate-limit vocabulary, `env_auth[0]`,
`oneOf`, serializable definitions, OAuth2 grants, query-param fidelity), 5
response model v2 and GraphQL, 6 WebSocket, 7 this spec. Child spec directory
names are not given by the parent; use the real directory names when writing
the table.

| Ledger row | Defect | Starting assignment |
|---|---|---|
| 1 | Non-JSON request bodies dropped | Spec 1 (D10, D33); text and binary now implemented |
| 2 | `generate --api <one>` deletes others | Spec 1 (D9) |
| 3 | Wrong `ws://remote.local/ws/ws` URL | Spec 6 |
| 4 | Generator panic on non-identifier type name | Already FIXED (parent); regression only |
| 5 | `ApiKeyParam` never sends key | Spec 1 (D10, D33) |
| 6 | Duplicate `operationId` in `openapi/ollama.json` | Spec 1 |
| 7 | Nested reference completeness | This spec, R7 |
| 8 | Generated docs lie | This spec, R38 to R41, R51 (typed convenience methods: ACCEPTED GAP, D53) |
| 9 | WS reconnect counter and lifecycle open | Spec 6 |
| 10 | Dormant `E0308` in Bearer host template | Spec 6 |
| 11 | WS env auth never applied | Spec 6 |
| 12 | WS plan diagnostics discarded | Spec 6 |
| 13 | `x-schematic` round trip unimplemented | This spec, R10 to R15 |
| 14 | `skip_serializing_if` without `serde(default)` | This spec, R14; stays reproduce-first (may be "not reproducible, guarded") |
| 15 | Test API list drift (2 of 18 skipped) | This spec, R34, R35 |
| 16 | `generated_code_compiles` is `#[ignore]`d | Spec 1 (D49 isolated compile check); partly fixed by `just lint-schema` |
| 17 | Strict import fails on INFO; tautological test | This spec, R16 |
| 18 | Stale tracked duplicate tree `gen/schematic/schema/` | Spec 1 only (its deletion); this spec verifies it is gone and fails the check otherwise (R50); **checked** 2026-10-11: `schematic/gen/schematic` still existed |
| 19 | README drifted, not doctested | This spec, R42 |
| 20 | Inline schemas import as `serde_json::Value` | Already FIXED (parent); regression only |
| 21 | 21 clippy warnings in generated crate | Already FIXED (parent); verified by `just lint-schema` (R44) |
| 22 | Two module-path resolvers | This spec, R36 |

| Recommendation | Subject | Starting assignment |
|---|---|---|
| 1 | Builders for `Endpoint` and `RestApi` | Spec 4a (D15) |
| 2 | Bind type references to types | Spec 4a (D2, D44) |
| 3 | Silent drops become errors or features | Spec 1 (D10, D33; per-output validation) |
| 4 | Verify fresh code compiles | Spec 1 (D49) |
| 5 | Streaming and response content types | Spec 5 (D6, D37; multi-shape responses replace D36 per the author ruling 2026-10-12) |
| 6 | `ApiResponse` as status-to-schema map | Spec 5 (D6, D38) |
| 7 | Params and required constant headers as OpenAPI parameters | Params done earlier (parent); typed path params and query-param fidelity: spec 4b; constant headers: this spec, R8, R9 |
| 8 | Schemars bridge | This spec, R1 to R7 |
| 9 | `x-schematic` read or strike | This spec, R10 to R15 |
| 10 | `--api <one>` cleanup, stale tree, resolver, feature flags | Spec 1 (cleanup, stale tree); this spec (resolver R36, flags R29 to R32, stale-tree check R50) |
| 11 | One generated list | This spec, R34 |
| 12 | Docs honesty pass | This spec, R38 to R41 (typed send and `send_raw` from spec 2); typed convenience methods: ACCEPTED GAP (D53) |
| 13 | WebSocket follow-through | Spec 6 |
| 14 | `schematic-define` hygiene | README doctests: this spec (R42); vendor pagination constructors: spec 4a (per-API strategy functions, D55-4), verified here (R43); `ConnectionParam`/`ParamType` collapse: spec 6 (D8); `env_auth[0]` retirement and `deprecated`: spec 4b; old pagination types removal: spec 4a; pagination payload: spec 3 |

Additional dispositions the final table must carry (not ledger rows or
numbered recommendations):

| Item | Disposition |
|---|---|
| `Schema::module_path` dead on the REST path | Spec 4a |
| `?` in path templates | Spec 1 (warning) and spec 4a (hard error, migration of the nine endpoints: Gitea 1, GitLab 1, OpenAI 7); this spec verifies the error is in force and no `?` path remains (R52) |
| Removal of the old pagination types, and conversion to the spec 3 model | Spec 4a |
| Vendor pagination constructors | Spec 4a (each API owns its own small strategy function in `schematic-definitions`, D55-4); this spec moves nothing and verifies none remains in `schematic-define` and no shared preset layer exists (R43) |
| `oauth_scopes` | Removed in spec 4a (D56-6); `auth_policy` kept with a builder form; the `x-schematic` `oauth_scopes` copy is deleted accordingly |
| emqx combined-module docs loss | This spec, R51 |
| Optional-auth variant (Ollama) | ACCEPTED GAP; reason recorded by the implementer from the earlier specs' logs |
| Legacy-versus-modern auth dual-source cleanup | ACCEPTED GAP; reason recorded likewise |
| Invented HuggingFace stub shapes | ACCEPTED GAP; reason recorded likewise |
| Orphaned types | ACCEPTED GAP; reason recorded likewise |
| Typed convenience methods (D49 item 3, recommendation 12) | ACCEPTED GAP (D53): about 7 s extra compile time per 550 wrapper instantiations; typed requests already carry the response type |

Gaps to resolve while writing the table: the parent's section 6 items 5 to 8
(declared response type discarded at the call site; `Schema::module_path` dead
on the REST path; type-name validation) are not ledger rows or numbered
recommendations; confirm they were closed by specs 2, 4a and 4b and list them
as extra rows if so. Whether an accepted gap is acceptable is the author's
judgment at review 1.

## Pre-authorizations

Taken from the parent's D50 list and specific to this spec. The author must
confirm these before the unattended run (D50 item 8); the author confirmed them
on 2026-10-21. The implementer does not pause for them and logs each use.

- Regenerate `schematic-schema` (large tree; generated output is never
  hand-edited) and regenerate the OpenAPI, Postman, and (new) AsyncAPI
  artifacts under `schematic/openapi/`, `schematic/postman/`, and
  `schematic/asyncapi/`.
- Rename or remove committed artifact files when the resolver unification
  changes a stem (R37), and update the drift tests accordingly.
- Change the generated crate's `Cargo.toml` (features, optional dependencies).
  Dependents are not edited (R31).
- Add dev-dependencies to `schematic-gen` or `schematic-definitions` for JSON
  Schema validation (R19, R25), and a build-script helper if P10-e needs one.
  Runtime (non-dev) dependencies in shipped crates need a recorded reason and
  an update to `schematic/docs/dependencies.md`.
- Create `schematic/asyncapi/`, `schematic/docs/` pages, and the new justfile
  recipe.
- Delete `SchematicSchemaExtension` and `AuthExtension` if P10-b's default is
  taken (`AuthExtension` is not wired as a whole, D57 item 22; R54 carries the
  two auth details it cannot lose).
- Edit `schematic-definitions` source files to adopt the single API list
  (R34). No vendor pagination preset moves (R43 is verification). This is *not* a
  rewrite of the roughly 519 endpoint literals; if a change would touch more than
  the API-level constructor calls, leave that change unmade, log it as an open
  question, and continue (ruled 2026-10-21).
- Build and test every dependent crate listed above (read-only use of their
  trees).
- The stale tracked `schematic/gen/schematic/` tree is spec 1's only to
  delete; this spec only verifies absence (R50) and does not delete it.
- Not pre-authorized: editing sniff or any other dependent crate's source;
  moving specs between lifecycle directories; CI workflow changes; commits
  (unless the user explicitly requests them); contacting real providers.

## Dependencies and ordering

- `depends_on` lists spec 6 (`2026-10-17-schematic-websocket`) because the
  AsyncAPI exporter (D47) must export the WebSocket types after they reuse the
  REST types (D8). The parent runs the eight specs strictly in order, so the
  full chain 1 to 6 (1 schematic-generation-integrity, 2
  schematic-runtime-envelope, 3 schematic-pagination, 4a
  schematic-definition-api, 4b schematic-definition-vocabulary, 5
  schematic-response-model-graphql, 6 schematic-websocket) is landed first.
  Their outputs this spec consumes:
  - Spec 1: the isolated compile check (R30, R38 use it), partial generation
    (R32), per-output validation, the stale-tree deletion (verified by R50),
    and the `?`-in-path-template warning (the hard error is spec 4a's).
  - Spec 2: typed send and `send_raw` (what the quick-start and docs call, R39);
    rate-limit, request-id, and error-table extension data.
  - Spec 3: the pagination model (what R43 verifies against).
  - Spec 4a: builders and typed registration (R7, R19 rely on an authoritative
    registry; the API-level constructors R34 touches); removal of the
    old pagination types and the vendor-named constructors, with per-API
    strategy functions (R43); the hard error for `?` in paths and the
    migration of the nine endpoints (R52); `oauth_scopes` removal; JSON
    definition snapshots and plain serde derives on `RestApi`/`Endpoint`/
    `EndpointParams`/`ParamDef` (R10); the `Schema::module_path` disposition.
  - Spec 4b: the definition vocabulary the exporters and importer must read and
    write (typed path params, auth override, `deprecated`, rate-limit
    vocabulary, `env_auth[0]` retirement, `oneOf`, serializable definitions,
    OAuth2 grants, query-param fidelity).
  - Spec 5: the response vocabulary (R21 saved responses, R2 to R3 error and
    stream shapes, multi-shape operations).
  - Spec 6: WebSocket typed messages and shared REST types (R25 to R28).
- It must not start before every dependency is at "implementation complete,
  ready for review" and the refresh in "Refresh notes" is done.
- Within the spec: phase 5 (single list) before phases 6 and 7; phase 1
  before everything.
- Outside the eight: sniff retiring `FocusedProviderClient` is tracked by
  sniff's own spec and starts after spec 3. It may or may not be finished when
  this spec runs; R47 reports the state and this spec does not wait for it, but
  the author cannot close the whole work until it is done (D20 part 4).

## Planning-time details assigned to this spec

These are the parent's P10 items (and sub-details P10 leaves open) assigned
here by D50. Each default is adopted by the implementer and logged for review 1
unless the author rules otherwise before planning. The items in Open Questions
are put to the author in the pre-plan pass (D50 item 8); an unruled item takes
its recommended option as the default (D51 item 5).

- **P10-a. AsyncAPI version and supported subset (D47).**
  Default if not ruled before planning: AsyncAPI **3.0** (the document is not
  AsyncAPI 2.x). One document per WebSocket API, with collision-safe API-specific stems
  within shared module groups; naming follows the Open Questions item below. Supported subset: `info`,
  `servers` (from the base URL and protocol, with the security scheme),
  `channels` (one per endpoint path, with address and parameters), `operations`
  (send and receive, from message direction), `components.messages`,
  `components.schemas` (from the model catalog), `components.securitySchemes`
  for the auth policy, inbound message selectors and the per-direction
  fallback marker (R56, D57 item 19), the endpoint role as `x-schematic-role`
  on each channel (R57, D57 item 24), and `HeartbeatHints` metadata (R58, D57
  item 21; the hints are kept on export even though the client does not
  schedule heartbeats). Not exported: bindings beyond WebSocket query and
  headers, traits, and correlation hints (the parent notes the hints have
  zero users). Output directory `schematic/asyncapi/`, one file per API
  derived from the unified resolver (R36). The importer must add a genuine
  3.0 reader: the current `schematic-gen`
  [AsyncAPI importer](../../gen/src/asyncapi_import.rs) reads 2.x `url` and
  `publish`/`subscribe`, not 3.0 `host`/`pathname` and operations. Preserve
  existing 2.x imports and reject unsupported versions explicitly. Export
  describes the client application: client messages use `send`, server
  messages use `receive`, bidirectional messages appear in both; this holds
  for every endpoint role, which travels in `x-schematic-role` (D57 item 24).
  Preserve
  handshake URL assembly without duplicating base-path segments. Security is
  derived from applied server/operation declarations, not merely the first
  component scheme. See the [AsyncAPI 3.0 contract](https://www.asyncapi.com/docs/reference/specification/v3.0.0).
  Local references remain self-contained; imports never fetch remote schemas.
  The importer is only required to read back this subset in 3.0; selectors,
  the fallback marker, and the role are part of it (R27).
- **P10-b. `x-schematic` supported subset and version handling (D18).**
  Default if not ruled before planning: the subset is what the exporter writes
  and OpenAPI cannot express, **refreshed to include everything written by
  specs 2, 4a, 4b and 5**: rate limit, `deprecated`, auth override, endpoint
  `cross_origin`, request-id, the error table, response
  shapes (including the D56 end-marker `keep` modifier and the `Bytes`
  framing), secret markers, and the two auth details OpenAPI cannot express
  (a bearer header override and an API-key value prefix; R54, D57 item 22),
  in addition to the earlier document level
  (`module_path`, `request_suffix`, `env_mapping`, `headers`) and operation
  level (`request` where it carries more than OpenAPI shows, such as
  `FormField` detail; `response` kind and content type where standard fields
  are not enough; `headers`). `oauth_scopes` was removed in spec 4a (D56-6),
  so its `x-schematic` copy is deleted, not carried. `auth_policy` is kept
  with a builder form. Where the extension carries definition data, it builds
  on spec 4a's plain serde derives and JSON definition snapshots. The implementer
  **re-derives the subset from the actual extension structs at plan time**; the
  list above is a starting point, not authority. Add `version: 1` (integer
  major) to the document-level extension and read an absent version as 1.
  Standard OpenAPI fields win on conflict (R12). `deprecated`, scopes, and
  expressible auth alternatives use standard fields; extensions retain only
  additional semantics. Authentication preservation is ruled (D57 item 22):
  `securitySchemes` is authoritative, and the extension carries only the
  bearer header override and the API-key value prefix (R54); deleting
  `AuthExtension` must not lose either. GraphQL operations are outside the
  subset; OpenAPI skips them with a diagnostic (R55, D57 item 23). Delete
  unused extension structs only after the supported field mapping is settled. If an earlier spec removed or renamed
  an extension field, the subset follows the current struct. The claim "full
  round trip" is replaced with "semantic round trip for the supported subset"
  everywhere.
- **P10-c. Postman example value source (D18).**
  Default if not ruled before planning: values derive from the registered JSON
  Schema in this priority order: schema `examples` (first), schema `default`,
  `const`/single-value `enum`, first `enum` value, then a typed placeholder
  (`"string"`, `0`, `false`, `"2000-01-01T00:00:00Z"` for `date-time`, a
  minimal valid value respecting `minimum`/`minLength`/`format`). Only required
  properties plus properties with an explicit example or default are emitted;
  recursion has a depth-3 limit with the failure rules below; `oneOf`/`anyOf`
  try branches in declaration order, accepting only candidates that validate
  against the complete schema (`oneOf` must match exactly one branch). Combine
  `allOf` constraints. Validate authored examples/defaults before choosing them.
  At depth 3 omit optional recursive properties or use null only when allowed;
  required unbounded recursion yields a diagnostic. General regex solving and
  arbitrary schema satisfiability are not required. Output is deterministic
  (no randomness or clock). Every emitted example must validate; generation
  is not guaranteed for every possible schema.
- **P10-d. Feature-flag granularity and default set (D18; interplay with D9).**
  Default if not ruled before planning: **one Cargo feature per API module**
  (the generated Rust module that D9 treats as a group: when two APIs share a
  module, they share one feature, named for the module). Feature names are the
  module names. Default features: all of them. Transport dependencies
  (`tokio-tungstenite`, and any dependency used only by WebSocket modules) are
  optional and enabled by the WebSocket modules' features. The manifest is
  generated from the full inventory on every run, including partial runs. An
  `all-apis` umbrella feature is not added unless the default set cannot
  express "all" for dependents.
- **P10-e. Mechanism for the single API list (D46).**
  Default if not ruled before planning: a function in `schematic-definitions`
  returning registration entries (definition and registry constructors,
  identity/aliases, module, protocol), with `apis_by_module()` and other
  consumers derived from it. A plain function is the least machinery. The
  generator writes compile-time module declarations from this inventory; do
  not add a generated-client build dependency on `schematic-definitions`
  merely to enumerate modules. Tests supply a local fixture inventory to
  consumers without editing global shipped registrations.
- **P10-f. File-name changes from resolver unification (D49).**
  Default if not ruled before planning: the unified resolver adopts the Rust
  assembler's behavior (the suffix heuristic), since the generated crate's
  module names are public API that dependents import; OpenAPI and Postman file
  stems follow it, renaming any committed artifact that differs
  (for example `pagerdutyapi.json`, if such a file exists; unverified).
- **P10-g. "Typed convenience methods where applicable" (D49; D53).**
  Ruled by the author (D53): **no** per-endpoint wrapper methods are generated.
  This is an ACCEPTED GAP against D49 item 3 and recommendation 12 (reason:
  about 7 s of extra compile time per 550 wrapper instantiations; typed
  requests already carry the response type). The generated docs and quick-start
  use `client.send(Request::new(..))` and name no nonexistent method (R39). The
  implementer does not revisit this and records the outcome in the log.
- **P10-h. Diagnostics format for import extension findings (parent question
  14, shared with spec 1).**
  Default if not ruled before planning: reuse the existing import
  `OpenApiDiagnostic` (severity, location, message) with a JSON-pointer
  location and a message that names the extension key; no new machine-readable
  format. Spec 1 owns the format of generator validation errors; do not
  duplicate it here.
- **P10-i. Strict mode rule.** The parent says only that strict "fails on INFO"
  and is unusable. Default if not ruled before planning: strict fails on WARN
  and ERROR and never on INFO; no new option is added.
- **P10-j. Extra Postman artifacts' location and naming.**
  Default if not ruled before planning: environment files are written next to
  the collection as `{stem}.postman_environment.json`; saved example responses
  live inside the collection, not in separate files.
- **P10-k. Validation library for generated examples and AsyncAPI.**
  Default if not ruled before planning: use one maintained JSON Schema
  validator crate already in the lockfile if there is one (check first);
  otherwise add a single dev-dependency. For the AsyncAPI document schema, if
  no small validator is available, check structure and `$ref` closure only and
  record schema validation as unverified.
- **P10-l. Round-trip corpus.** Default if not ruled before planning: the 15
  committed OpenAPI artifacts plus small purpose-built fixtures; where an
  artifact cannot round-trip in the supported subset, the difference is listed
  in the log rather than hidden by loosening the comparison.

## Open Questions

These choices affect public artifacts or fidelity guarantees, so they are put to
the author in the pre-plan pass (D50 item 8) rather than settled silently. If the
author has not ruled before planning, the recommended option is the default:
the implementer adopts it and logs it for review 1 (D51 item 5), and no run
pauses for it. Record the author's choice, or the adopted default, in the plan.

### AsyncAPI document boundaries for shared modules

The default asks for one document per WebSocket API but also a module-based
filename. Two APIs sharing a module would overwrite each other. AsyncAPI 3.0
also requires importer work beyond writing its current in-memory result.

1. **API-specific documents with a 3.0 reader — recommended.** Use the module
   stem for a single API and append a stable selection name when several
   WebSocket APIs share it. Pros: each document has one application identity,
   server/auth policy, and reversible definition; retains the planned 3.0 output.
   Cons: adds a documented filename rule and a version-specific importer path.
2. **One grouped 3.0 document per module.** Pros: follows current OpenAPI and
   Postman grouping. Cons: needs extra metadata to separate API identities,
   servers, and security on import; shared paths can collide.
3. **API-specific 2.x documents initially.** Pros: reduces importer changes.
   Cons: changes the proposed output version and defers the 3.0 work.

Recommend API-specific 3.0 documents because round-trip identity matters more
than matching REST grouping. Validate shared-module filenames and preserve 2.x
import fixtures. A schema validation gap may be recorded, but writing 2.x fields
under a 3.0 version is a defect, not an accepted validation shortcut.

**Ruled by the author on 2026-10-21:** option 1, API-specific 3.0 documents.

### Ruled since this spec was written

- Authentication details beyond OpenAPI security schemes: ruled in D57 item
  22; see R54 and P10-b.
- GraphQL operations added by the response-model work: ruled in D57 item 23;
  see R55.
- WebSocket endpoint role on round trip: ruled in D57 item 24; see R57 and
  R27.

## Risks and failure modes

- **Regenerating many artifacts hides unintended changes.** Mitigation:
  regenerate after each phase and review that each diff contains only the
  intended change; the bridge change should only add types, enums, and
  nullability, never remove properties.
- **Bridge change alters generated Rust.** The Rust client is generated from
  Rust types, not the OpenAPI schema, so it should not move; assert that
  `schematic-schema` is byte-identical after the bridge phase (it should only
  change at phases 7 and 8).
- **Over-eager `Any` classification.** A heuristic too strict fails valid
  foreign schemas; too loose hides loss. The paired fixtures (R5) pin both
  sides. Foreign or unconstrained schemas are listed in the log.
- **Postman examples that validate but mislead.** Placeholders are not real
  data. The guarantee is validity only.
- **Feature flags break dependents.** Default = all is the guard; R31 builds
  every dependent. A dependent that enables `default-features = false` is not
  known to exist (unverified; the implementer greps the dependents' manifests).
- **Feature flags and shared modules.** Two APIs in one module (D9) cannot be
  split by a per-API feature; P10-d chooses module granularity for that reason.
- **Resolver unification renames committed artifacts**, which could break a
  consumer that reads an artifact by name (unverified whether any does).
  Mitigation: grep the repo for the old names before renaming (R37).
- **AsyncAPI exporter depends on spec 6's model.** If spec 6 changed the
  WebSocket types more than expected, the exporter shape and P10-a change;
  see Refresh notes. The AsyncAPI JSON Schema validation may be unavailable
  offline (unverified).
- **Schematic-only AsyncAPI metadata.** Selectors, the fallback marker, the
  role, and heartbeat hints (R56 to R58, D57 items 19, 21, 24) live in
  Schematic extensions that third-party AsyncAPI tools ignore, so those tools
  see overlapping messages with no way to tell them apart. Mitigation: the
  AsyncAPI topic page states this; the round trip through Schematic's own
  importer is the guarantee.
- **Round-trip claims overreach.** Counter: the supported subset is explicit
  and anything outside it is listed, not silently accepted.
- **README doctests take long or need network.** Use `no_run` or `ignore` with
  a stated reason for examples that cannot run offline; never leave a broken
  example.
- **Ledger table inconsistent with earlier specs' logs.** Earlier specs may
  have closed rows differently than the starting map; the table follows the
  evidence, and a row with no evidence is an accepted gap, not a silent close.
- **D20 part 4 may be open when this spec finishes.** That is expected and is
  reported, not hidden (R47).
- **A cross-platform trap.** Case-insensitive file systems (macOS default,
  Windows) can collide stems that differ only by case after the resolver
  change; assert stem uniqueness case-insensitively.
- **Time and size.** The scope is wide (58 requirements). If a phase
  cannot be completed without a decision, the implementer takes the default,
  logs it, and continues; it does not stop the run for a human.

## Process

Each spec has a 1:1 plan. The plan is implemented end to end, phase by phase,
**without a human in the loop**. Then **review 1** runs, with time set aside
for the author to review decisions made during implementation and any open
design questions. Then the **next** spec and plan are refreshed from the
outcomes. Then fix/review cycles repeat until the reviewer deems the spec
production ready. An agent's terminal state is "implementation complete, ready
for review"; only the author moves a spec to `_completed`, and an agent never
runs `just complete`.

Because no human is available while the plan runs:

- This spec restates every decision and requirement the implementer needs;
  D-numbers and the parent path are provenance only, and the implementer need
  not open the parent (they may, for the evidence).
- Every remaining planning-time detail has a default, marked "Default if not
  ruled before planning". The implementer adopts it and logs it.
- The implementer records, **as each occurs** and not at the end, every item in
  the Human review agenda below in `implementation-log.md` in this directory
  (frontmatter in the style of the `worktree` feature logs: `spec`, `plan`,
  `implemented_by`, per-phase source files, docs updated or created, skill
  files updated, packages).
- Where the spec is silent, the implementer picks the simplest option that
  satisfies the requirements and logs it. An assumption is labeled as one.

This is the last spec of the eight. Its review 1 is also the occasion for the
author to review the ledger disposition table and the whole-work done
checklist, and to decide whether any accepted gap is acceptable.

## Human review agenda

The implementer logs each of the following in `implementation-log.md`, as it
occurs, for the author's review 1.

1. Every default adopted from "Planning-time details" (P10-a to P10-l) and
   "Open Questions", with
   the alternative that was not taken and why it did not matter or did.
2. Every design choice the spec does not dictate (names, module layout, which
   crate hosts the resolver and the single list, validator choice, example-value
   rules beyond the defaults).
3. Every deviation from a requirement or from the Design table, with the reason.
4. Every open design question found while implementing, with the options seen.
5. The baseline and final untyped-property counts per API, and the full list of
   deliberate free-form properties (R5, R6).
6. The enumerated API lists (R33: six runtime lists plus test tables), and
   any list that could not be replaced.
7. The outcome of the row 14 reproduction: fixed, or not reproducible (R14).
8. The supported `x-schematic` subset as built, re-derived from the actual
   extension structs and refreshed for specs 2, 4a, 4b and 5, the version
   rule, the conflict rule, and anything dropped from the round trip (R10 to
   R15), including where the bearer header override and API-key value prefix
   are placed (R54, D57 item 22).
9. Which extension structs were deleted or wired.
10. The strict-mode default and behavior change (R16, P10-i): strict fails on
    WARN and ERROR, never INFO, as it changes what users of `strict` observe.
11. Any committed artifact renamed or removed and why (R37).
12. The feature-flag set as built, and the evidence that each dependent builds
    unchanged (R31).
13. The P10-g outcome: no per-endpoint convenience methods (D53, accepted
    gap), and the docs and quick-start verified to use `client.send(..)`.
14. The Postman example values for three representative endpoints, so the
    author can judge them; any endpoint whose body could not be generated.
15. The AsyncAPI round-trip results per API, with differences (R27), and
    whether schema validation of the output was possible; the extension names
    and shapes chosen for selectors, the fallback marker, and heartbeat hints,
    and whether hints are read back (R56 to R58, D57 items 19, 21, 24).
16. The ledger disposition table (R45), the full accepted-gap list (optional
    auth (Ollama), auth dual-source cleanup, invented HuggingFace stub shapes,
    orphaned types, typed convenience methods) with reasons, and every row
    whose starting assignment turned out wrong.
17. The whole-work done checklist state (R46) and the sniff check result
    (R47), including the grep output.
18. Any environment among macOS, Linux, native Windows, WSL2 that could not be
    exercised, and the evidence used instead.
19. Any unverified claim in this spec that the implementer verified, with the
    result (for example the fate of the stale `gen/schematic` tree, the
    existence of the old `pagerdutyapi`-style file names).
20. Any comment/doc drift found and how it was resolved (R49).
21. Pre-authorizations used, with what was done under each.
22. The stale-tree check (R50) and the emqx combined-module docs fix (R51).
23. The vendor-constructor and shared-preset-layer verification (R43) and the
    `?`-path verification (R52), with the grep or scan output.
24. The `x-schematic` subset's coverage of the `keep` modifier and `Bytes`
    framing, the deletion of the `oauth_scopes` copy, and which spec 4a JSON
    snapshots the round trip and definition-serialization tests build on.
25. The GraphQL operations exported to Postman, the `variables` example
    source, and the OpenAPI skip diagnostics (R55, D57 item 23).
26. The container-component check over the 18 registries (R53, D57 item 12),
    with any container component still found.

## Refresh notes

This spec is built before specs 1 to 6 (including 4a and 4b) have been reviewed (`refresh_required:
true`). Before its plan is written, refresh it from the outcomes of their
review 1 and their implementation logs:

- **Spec 1 (generation integrity).** Whether the isolated compile check exists
  and what it is called (R30, R38); whether `gen/schematic/` was deleted; the
  format of per-output validation diagnostics (P10-h defers to it); whether
  duplicate `operationId` was fixed and how (this spec's artifacts inherit it);
  whether the destination of text/binary/header/cookie features changed what
  Postman (R23, R24) should export.
- **Spec 2 (runtime and envelope).** Whether typed send or `send_raw` or a
  convenience method is the documented call path (R39, P10-g); the final
  `AuthMethod`/auth strategy field set (R40's `ApiKey` literal).
- **Spec 3 (pagination).** What of `PaginationStyle` remains (R43 verifies no
  vendor-named constructor remains in `schematic-define`); whether the
  35-endpoint inventory still holds.
- **x-schematic subset (P10-b).** Re-derive the subset from the actual
  extension structs after specs 2, 4a, 4b and 5 land: rate-limit, deprecated,
  auth override, endpoint `cross_origin`, request-id, error table,
  response shapes, and secret markers. Record the final
  subset in the plan.
- **Spec 4a (definition API) and 4b (definition vocabulary).** Typed registration and whether nested types are
  auto-registered (R7); the shape of API-level constructors that the single
  list touches (R34); the removal of container registrations and its R24
  listing (R53, D57 item 12); the per-API strategy functions and the `?` hard error
  and migration (R43, R52); the `oauth_scopes` removal and `auth_policy`
  builder form (decided, D56-6), reflected in the `x-schematic` subset
  (P10-b) and the OpenAPI `security` mapping; serializable definitions and
  JSON snapshots (which may let `x-schematic` carry more); the typed path parameters, `deprecated`, and rate limit
  vocabulary that the exporters and importer must now read or write.
- **Spec 5 (response model v2 and GraphQL).** Success/error/stream shapes and
  multi-shape operations in the exporters (R2 to R4, R21); the GraphQL operations added to `RestApi`, whose export treatment
  is ruled (R55, D57 item 23), and whether spec 5 R42's skip wording names
  this spec as the Postman owner; the extension's `response`
  field type (P10-b).
- **Spec 6 (WebSocket).** The final `WebSocketApi` model, the typed message
  enums, the explicit response link, and the reused REST types (R25 to R27, and
  the P10-a subset); the final selector, fallback, role, and `HeartbeatHints`
  shapes (R56 to R58, D57 items 19, 21, 24).
- **sniff's own spec.** Whether `FocusedProviderClient` is already retired (R47).
- **Counts.** Refresh the API count (18 REST, 5 WebSocket), the endpoint
  literal count (about 519), the API-list inventory (six runtime lists plus
  test tables, R33), and the dependents list (eight crates) if any changed.
