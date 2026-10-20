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
review_iterations: 0
clarified: false
implemented: false
human_review: false
message_to_agent: ''
parent: ../../reviews/2026-07-31-macro-review/spec.md
depends_on:
    - 2026-10-14-schematic-definition-api
refresh_required: true
related: []
next_spec: 2026-10-16-schematic-response-model-graphql
---

# Schematic definition vocabulary

## How to read this draft

This is a sibling specification in the series coordinated by `2026-07-31-macro-review`; the existing `parent` property identifies that coordinating document, not an implementation dependency. Resolve specification identifiers under Schematic's `features/` or `reviews/` directories, including moved documents. The implementation dependency is `2026-10-14-schematic-definition-api`, which supplies the builders and typed registration context. This draft does not claim that 4a's proposed types already exist.

> **Reader's note — inline review, 2026-10-11.** This review preserves D18 and the D55 exemption for pager-managed constructor arguments. It clarifies credential isolation, path export, structural schemas, query encoding, and migration checks. One choice remains under **Open Questions**: credential sources for an endpoint using different authentication. Whether request structs keep `Default` once they carry required fields was ruled in D57 item 13 (R9). How a direct send supplies a required pager-managed parameter is spec 3's question; this spec follows its resolution. Recommendations there are proposals for the author, not new author rulings. No performance spike is added: this work needs correctness checks, and this draft identifies no measured performance uncertainty requiring a separate experiment.

## Existing contracts and source map

These are the current sources to check again after 4a lands:

| Symbol or component | Package | Purpose and relevance |
|---|---|---|
| [Endpoint and RestApi](../../define/src/types.rs) | `schematic-define` | Store endpoint and API configuration; the new vocabulary extends these through 4a's builders. |
| [ParamDef, QueryParamType, ParamStyle](../../define/src/params/param_def.rs) | `schematic-define` | Describe parameter types and encoding; path declarations reuse this vocabulary without changing header or cookie behavior. |
| [AuthPolicy, AuthMethod, EnvAuthStrategy, AuthStrategy](../../define/src/auth.rs) | `schematic-define` | Separate explicit credentials from environment fallback; an endpoint override must preserve that distinction. Only the legacy `AuthStrategy::Basic` acquires environment lists. |
| [OAuth2Config](../../define/src/oauth.rs) | `schematic-define` | Describes provider grants; this spec intentionally replaces its flat shape. |
| [TypeRef](../../define/src/models/type_ref.rs) and [ModelCatalog](../../define/src/models/mod.rs) | `schematic-define` | Describe structural models; composition must agree with the registered Rust type's JSON representation. |
| [OpenAPI path exporter](../../define/src/openapi/export/paths.rs) | `schematic-define` | Shares path declarations across operations today; differing typed declarations need operation overrides. |
| [QueryParamInfo and query formatting](../../gen/src/codegen/request_structs/shared.rs) | `schematic-gen` | Currently lose required/style/explode information; retaining it is necessary for correct request fields and wire encoding. |
| [Path constructor generation](../../gen/src/codegen/request_structs/path_params.rs) | `schematic-gen` | Currently accepts strings for every path parameter; typed constructors must still preserve reserved-expansion encoding. |
| [Schematic just recipes](../../justfile) | Schematic package area | `just test` runs nextest; `just doctest` separately checks documentation examples. |

## Problem

`schematic-define`'s vocabulary is smaller than the APIs it describes (parent section 4, D18). After 4a every definition is written with builders and a typed definition context, so adding vocabulary no longer means rewriting about 519 endpoint literals: each item below is a **new builder method or new field** on 4a's builders, and shipped definitions keep compiling unchanged unless one deliberately adopts an item. Verified in the parent review (July evidence, re-checked in part on 2026-10-11 by this draft):

1. `EndpointParams` has `query`, `header`, `cookie` but no `path`: path placeholders are untyped strings.
2. There is no per-endpoint auth override (a public endpoint on an authenticated API cannot be stated), no `deprecated` (the OpenAPI exporter hard-codes `deprecated: None`), and no rate-limit or retry vocabulary.
3. Basic auth keeps its password positionally in `env_auth[0]` (`schematic/define/src/types.rs`, `legacy_env_mapping_for`).
4. A hand-written definition cannot attach structural `oneOf` models.
5. A serialized definition is not self-contained: spec 4a (D55) adds plain serde derives on `RestApi`, `Endpoint`, `EndpointParams` and `ParamDef` and snapshots definitions to JSON, but those types name schemas and models by string (the by-name `Schema` contract), so the JSON cannot be loaded or compared in isolation.
6. `OAuth2Config` is a flat struct on which illegal grant and URL combinations can be written.
7. The Rust generator discards query-parameter `required`, `style`, `explode` and `Enum`.

D52 (2026-10-11) split the original definition-API spec: 4a is the rewrite and builders; this spec (4b) is the vocabulary (D18's list). Part of this vocabulary changes shipped generated output (item 7) or breaking types (item 6); that is why it is separate from a pass whose proof is "output is unchanged".

## Goal and non-goals

**Goal.** Every item above exists as new builder methods or fields on 4a's builders, is consumed by the outputs that can use it (or yields the D10 backstop diagnostic), is tested with fixture definitions, and leaves the committed definitions compiling with no source change except the deliberate adoptions listed under R12.

**Non-goals (out of scope, with owner).**

- The builders, context, descriptors, typed registration, the rewrite of the endpoint literals, the pagination removal, the importer, the `oauth_scopes` removal and the `auth_policy` retention (D56), per-endpoint `cross_origin`, `request_id_header`, `.secret()` sugar, the plain serde derives on `RestApi`, `Endpoint`, `EndpointParams` and `ParamDef` with JSON snapshots (D55), and the hard error for `?` in path templates with the migration of the nine endpoints that carry one: 4a. This spec relies on its outcomes and does not re-implement or re-validate any of them.
- Convenience methods on generated clients: not built (D53, accepted gap).
- Runtime retry, sleeping, rate-limit enforcement or any runtime consumption of the rate-limit vocabulary: not built anywhere in this plan (R4 is metadata only).
- Response vocabulary v2 and GraphQL: spec 5. Streaming and error mapping are not touched.
- WebSocket definitions: spec 6. They reuse `ParamDef` and the auth types changed here (D8); this spec must not break their build and does not rewrite them.
- OpenAPI bridge fixes, `x-schematic` round trip beyond the new fields' emission, Postman example bodies, the accepted-gap ledger: spec 7.
- The optional-auth variant (Ollama), the wider legacy-versus-modern auth dual-source cleanup beyond `env_auth[0]`, invented HuggingFace stub shapes, orphaned types: not done here; accepted-gap candidates for spec 7's ledger table (D20), listed in Risks.
- Generating a Rust enum from structural `oneOf` alone (R6 binds to an existing Rust type).


## Background: decisions this spec implements

Provenance: parent spec `schematic/reviews/2026-07-31-macro-review/spec.md` (section 15 decisions; section 13a "Plan structure (D50)"). Everything needed is restated here.

- **D18 (the vocabulary list).** Typed path parameters; per-endpoint auth override, `deprecated`, rate limits and retry; retiring `env_auth[0]` as the Basic-auth password (make `Basic` carry its env shape); polymorphic `oneOf` bodies (bridge `Schema` names to the structural `TypeRef`/`ModelCatalog`); serializable definitions; OAuth2 per-grant payloads (breaking); query-parameter `required`, style, `explode` and enum fidelity.
- **D52 (2026-10-11, approved).** The definition-API spec is split: 4a builders and typed registration (and the single rewrite), 4b this vocabulary. Order: 1 generation-integrity, 2 runtime-envelope, 3 pagination, 4a, 4b, 5 response-model-graphql, 6 websocket, 7 fidelity-polish.
- **D53 (2026-10-11).** No convenience methods on generated clients; recorded as an accepted gap. Nothing in this spec adds one (the new vocabulary reaches the generated request structs and docs only).
- **D54 (2026-10-11).** Ownership of definition-side artifacts is settled in the parent; this spec owns only the vocabulary items listed here and the generator consumers for them, and does not take over 4a's builders, context or importer.
- **D55 (2026-10-11, author ruling).** (1) Spec 4a adds the plain serde derives on `RestApi`, `Endpoint`, `EndpointParams` and `ParamDef` and snapshots to JSON from the start; 4b keeps only self-contained serialization (R7). (5) Parameters managed by a pagination strategy are exempt from query-parameter `required` fidelity (R14).
- **D56 (2026-10-11, author ruling).** The runtime trait is `EndpointSpec`; `auth_policy` is kept; `oauth_scopes` is removed (all in 4a). R2 and R8 build on a retained `auth_policy` and do not reintroduce `oauth_scopes`.
- **D57 (2026-10-11, author ruling).** (13) A generated request struct that has a required non-pager query field or a typed path field does not derive `Default`; a struct whose fields are all optional keeps it (R9; refines D55 item 5).
- **D1 (scope).** Nothing is deferred; every D18 item is built.
- **D2/D44 (typed registration).** Descriptors come only from the definition context. Spec 4a leaves an optional structural component and serde derives on the descriptor (author-approved constraint, unused in 4a); R6 uses it.
- **D10 (backstop rule).** A declared feature must either reach the selected output or produce a diagnostic identifying API, endpoint, feature and unsupported target; one output's gap never blocks another output.
- **D8 (shared building blocks).** WebSocket reuses `ParamDef`, auth types and `EnvMapping`; do not bake REST-only assumptions into them.
- **D49 (dead-surface criterion).** A field is either read by generator or exporter code and covered by a test, or it does not exist. R4 must meet this despite being metadata (see Risks).
- **D50 (plan structure).** Each spec leaves the repo green on macOS, Linux, native Windows and WSL2; every dependent builds and passes; terminal state is "implementation complete, ready for review". Pre-authorizations are items the author must confirm before the unattended run (D50 item 8).

The runtime trait is `EndpointSpec`; `Endpoint` here is always the definition struct.

## Requirements

"Package" names the crate: `schematic-define`, `schematic-definitions`, `schematic-gen`, `schematic-schema` (generated), `schematic-oauth`. Counts are re-verified at the start of the run (R11).

**R1. Typed path parameters.** `EndpointParams` gains `path` (PD2 for the element type) and the endpoint builder gains `.path_param(..)`. When an endpoint declares path parameters, the declared set must equal the set of `{name}` and `{+name}` placeholders in the path template (build error otherwise, naming API, endpoint and parameter); an endpoint that declares none keeps today's behavior (every placeholder is an untyped string field). Rejecting `?` in a path template is 4a's hard error and is not re-checked here; the placeholder scan assumes a template 4a already accepted. Declared type and description reach the generated Rust request struct (field type and doc) and the exported OpenAPI path parameters (type, description, `required: true`), shared at path-item level when all operations agree. When methods on the same path declare different types or metadata, emit one common declaration plus an operation-level override for each differing `(name, in)` pair; never silently use whichever method was visited first. This follows the [OpenAPI path-item and operation parameter rules](https://spec.openapis.org/oas/v3.0.3#path-item-object).
Duplicate declarations are rejected; repeated occurrences of the same placeholder use one request field. Normalize `{+name}` to `name` for matching and OpenAPI export, but keep the existing reserved-expansion encoding in generated requests. Typed values are converted to their wire spelling before that encoding is applied. Integer, number, and boolean use the same Rust scalar types as query parameters; enum path values use the enum naming and wire-spelling rules under PD8. Constructor arguments follow first occurrence in the template, rather than builder registration order. Reject collisions between sanitized field names and generated body/query fields through 4a's diagnostics.
Validated by: L1 unit tests in `schematic-define` (template/declaration match and mismatch); fixture-definition L1 tests in `schematic-gen` asserting generated Rust and exported OpenAPI for a fixture endpoint with an integer path parameter; the isolated compile check (spec 1) compiles the fixture output. Include a shared GET/DELETE path with differing parameter metadata, a repeated placeholder, a reserved-expansion string, an enum, and duplicate/sanitized-name failures.

**R2. Per-endpoint auth override.** An endpoint may declare that it is public or that it uses a different `AuthPolicy` than the API (PD4), through builder methods `.public()` and `.auth(..)`. Resolve the endpoint policy before credential lookup and request assembly: absent override inherits the API policy; public disables authentication; a policy override replaces rather than merges the API policy. Public requests skip environment lookup and token acquisition and omit Schematic-managed credentials from Authorization, custom auth headers, query parameters, and auth cookies, including credentials injected through the supported client builders. Ordinary non-auth headers and cookies remain. Authentication must be applied per request, so a public call cannot mutate authentication for the next call. An explicit raw header that conflicts with a known auth location is removed for a public call; document this behavior. Credential locations known to the API or selected endpoint policy remain subject to spec 2's secret handling and redirect rules.
Generated clients send no credential for a public endpoint; OpenAPI export emits `security: []` (or the override's scheme); Postman export uses `noauth` (or the override). If an output cannot express the override, the D10 backstop diagnostic names API, endpoint, feature and output, and the other outputs still generate. Register override-only OpenAPI security schemes in components with deterministic, collision-checked names. Multiple accepted explicit methods are alternatives, not a requirement to supply them all; never export an environment variable name as a credential. Credential sourcing for a replacement policy is an author choice under Open Questions.
Validated by: mock-server L1 test in `schematic-gen` (a generated client against a local mock records no `Authorization` header on a public endpoint of an authenticated fixture API and the credential on a sibling endpoint); generated-output assertions for OpenAPI and Postman. Cover header, query and cookie credentials, programmatic injection, missing environment credentials on the public call, and an authenticated call after the public call; include a replacement policy whose scheme appears only on the endpoint.

**R3. `deprecated`.** `Endpoint` and `ParamDef` gain `deprecated` (PD3), through `.deprecated()` builder methods. It reaches OpenAPI (`deprecated: true` on the operation or parameter; the exporter stops hard-coding `deprecated: None`) and the generated Rust docs. It adds no Rust `#[deprecated]` attribute so `just lint-schema` and dependents stay warning-free.
Validated by: L1 generated-output assertions in `schematic-gen` for Rust docs and OpenAPI; round trip in `schematic-define` (serialize/deserialize keeps the flag, after R7).

**R4. Rate-limit and retry vocabulary (metadata only).** The definition can state, at API level with an optional endpoint override, which response headers carry rate-limit information and the provider's retry guidance (shape in PD1; builder methods `.rate_limit(..)` and `.retry_guidance(..)`). The vocabulary is validated, serializable, and exported (OpenAPI `x-schematic` operation/document extension and generated documentation). **This spec adds no automatic retry, sleeping or enforcement to the runtime.** To avoid becoming another unused field like `oauth_scopes`, the D49 criterion applies: every field has a reader (the exporter and the docs generator) and a test, and at least one shipped definition adopts it (R12).
Validated by: L1 unit tests in `schematic-define` (validation of header names and numeric bounds); generated-output assertions in `schematic-gen`; focused assertions that both extension output and generated docs preserve every declared field; the adopting definition's output assertion; a test (grep-level) that the runtime crate does not read these types, alongside a review of the runtime diff confirming no retry behavior was added.

**R5. `Basic` carries its environment shape; `env_auth[0]`-as-password is retired.** `AuthStrategy::Basic` names its username and password environment lists itself (PD6). `env_username` is removed from the stored API definition; 4a's builder still accepts the username list and stores it inside Basic instead; no code reads `env_auth.first()` as a password. The 4a builder's Basic authoring call keeps its signature, so Bitbucket and EMQX (the only shipped Basic APIs, verified by grep of `AuthStrategy::Basic`) are migrated with no change to effective generated environment mapping or docs. Preserve explicit `env_mapping` precedence and `auth_policy` precedence as separate existing contracts. Preserve fallback order across every username/password list, not just its first entry. `AuthMethod::Basic` and `EnvAuthStrategy::Basic` stay source-neutral; they do not acquire environment fields. Update shared WebSocket matches and fixtures that use `AuthStrategy` without rewriting shipped WebSocket definitions. If 4a's call did not accept both lists, Bitbucket and EMQX are the deliberate adoption in R12.
Validated by: L1 unit tests in `schematic-define` (the `legacy_env_mapping_for` replacement yields `basic_user`/`basic_pass`); the safety net (R11) byte-compares Bitbucket and EMQX output; a negative check that no positional password read remains, plus a fixture with multiple fallback names and an explicit mapping overriding the embedded lists.

**R6. Polymorphic `oneOf` bodies from hand-written definitions.** `DefinitionContext` can register a polymorphic type described structurally (`TypeRef::OneOf`, `AnyOf`, `AllOf` over registered variants) and bind it to the Rust type that deserializes it (PD5). The structural model is stored through the optional structural component 4a left in the descriptor and in a `ModelCatalog` on the context, is available to OpenAPI export (`oneOf` with `$ref`s), and the Rust generator references the bound Rust type. Reject variants not registered in the same context, empty composition lists, and conflicting schema names. `oneOf` means exactly one matching schema, `anyOf` at least one, and `allOf` every member; a Rust untagged enum alone does not prove `oneOf` exclusivity. Authors are responsible for matching the Rust serde representation to the declared structure; fixtures use distinguishable variant shapes and demonstrate both serialization and deserialization. `allOf` uses a Rust type that satisfies all members, usually a struct rather than an enum. The explicitly supplied structure is authoritative for export; compare against the registry representation and diagnose conflicting definitions instead of keeping two schemas with the same name. Postman has no equivalent schema-composition declaration: retain an existing concrete example if available and issue the D10 backstop diagnostic for composition itself rather than inventing a payload. This does not take over the later Postman-example work. If 4a's descriptor component cannot carry what is needed, the descriptor type may be extended, never replaced, and the change is logged for review (no definition names the descriptor's fields, so none is rewritten).
Validated by: fixture-definition L1 tests in `schematic-gen` (OpenAPI contains `oneOf` with the variants' refs; Rust output references the bound type; isolated compile check passes); negative test for an unregistered variant in `schematic-define`; round trip of a descriptor with a `Some` structure; semantic fixtures for `anyOf` and `allOf` as well as `oneOf`, and a Postman diagnostic assertion.

**R7. Self-contained serialization.** The plain serde derives on `RestApi`, `Endpoint`, `EndpointParams` and `ParamDef`, and the JSON snapshots with their comparison test, already exist from 4a (D55); this spec does not introduce them. It adds derives only on the types its own vocabulary introduces (R1-R6, R8) and on any contained type 4a left without them. The remaining work is the by-name `Schema` contract: a serialized definition names request and response types by string, so alone it cannot be loaded or compared without the Rust registry. A definition together with the schemas (and, for R6, the `ModelCatalog`) its context registered serializes as one self-describing bundle (PD7) that loads in isolation, with no Rust type registry. Serialization of every shipped definition bundle round-trips to an equal value and is deterministic. A dangling by-name reference (a name with no entry in the bundle) is a load error naming API, endpoint and name. The bundle includes the full closure of nested schema references, including recursive models, container registrations, aliases and arbitrary JSON. Resolve names with 4a's canonical schema identity; a Rust path or `Vec<Model>` spelling is not itself a component key. Bundle construction normalizes local schema references to its component namespace, handles cycles with a visited set, and refuses unresolved external references without fetching files or URLs. Include Rust type bindings from the context so the bundle does not lose the distinction between a schema name and its Rust path. Loading supports comparison and schema-based export without a Rust registry; it does not provide executable Rust types or compile a client in isolation.
Definitions may contain literal headers, examples or default values as well as environment variable names, so a bundle is not automatically safe to publish. Preserve these values for equality; do not silently redact or serialize resolved environment values. Review the literals in fixtures and shipped definitions before committing bundles.
Validated by: L1 round-trip and byte-equal determinism test over bundles of all 18 REST APIs in `schematic-definitions`; a load-in-isolation test (deserialize a bundle in a test that does not touch the registry, then re-serialize equal); a negative test with a removed schema entry; nested and recursive schemas load without the registry; an isolated export uses the loaded components. 4a's JSON snapshots remain a regression check through the narrowly defined comparison in R11; structural changes cannot be byte-identical to the old snapshots.

**R8. OAuth2 per-grant payloads (breaking).** `OAuth2Config` becomes one variant per grant, each carrying only the fields legal for it (authorization-code with PKCE, client credentials, device code), so a missing grant-required field (such as the device authorization URL) cannot be written. Nonempty, well-formed URL strings and supported PKCE combinations still require validation; a type change does not make every string valid. Keep optional token revocation metadata on each grant and preserve current default scopes and client-authentication behavior. Use an authorization-code-specific PKCE type that cannot express `NotUsed`; preserve both `Required` and `Supported` semantics rather than strengthening them silently. Consumers in `schematic-gen` (`codegen/api_struct/auth.rs`, `request_method.rs`, `client/helpers.rs`, `export/auth.rs`, `output/assemble/shared.rs`, `codegen/module_docs.rs`) and `schematic-oauth` (`types.rs`, `manager/*`, tests) migrate. No shipped definition uses OAuth2 (verified by grep at drafting), so shipped generated output is expected not to change; fixture definitions prove each grant.
Validated by: L1 unit tests in `schematic-define` (each grant constructs; illegal combinations are unrepresentable, compile-fail where the point is the type); fixture-definition generation tests in `schematic-gen`; the existing `schematic-oauth` tests (`just test` runs them, including `oauth/tests/wiremock_flows.rs`) pass against the new type.

**R9. Query-parameter fidelity in the Rust generator.** For query parameters (`ParamDef`): `required: true` makes the generated request constructor take the parameter and the field non-optional; `explode`/`style` control array serialization for the supported styles (PD8; unsupported styles produce the D10 backstop diagnostic, not silent loss); `QueryParamType::Enum` generates a Rust enum (PD8 for naming) instead of a string. Preserve non-required fields as `Option<T>` and make required fields `T`; setters assign the corresponding shape. Generated request structs derive `Default` today, which can fill a required field with an empty value. A request struct that has a required non-pager query field (R14) or a typed path field (R1) does not derive `Default`; a struct whose fields are all optional keeps it (D57 item 13), so a required input can only be supplied through `new()`. A parameter with an explicitly authored default still remains constructor-required unless the pager exemption applies. Constructor order is path parameters in template order, then required query parameters in declaration order, then body. For an endpoint with required query parameters but no path/body, emit a real constructor. Update all field, setter, enum-dispatch, serialization and documentation paths together; retaining metadata only in a definition is insufficient.
Header and cookie parameters keep their existing behavior. Parameters managed by a pagination strategy are exempt from `required` fidelity (R14). This changes generated `new()` signatures: required parameters become constructor arguments. Dependents are inventoried first (R11) and migrated in this spec.
Validated by: fixture-definition L1 tests in `schematic-gen` asserting generated source and, with a local mock server, the exact query string on the wire (array explode true versus false, space and pipe delimiters, reserved characters and Unicode encoded once, omitted versus empty arrays; enum value spelling including array elements; required parameter present); isolated compile check; a trybuild compile-fail test that `Default::default()` does not compile for a fixture request with a required query field, and a compile test that an all-optional fixture request still implements `Default` (D57 item 13); dependents' builds (R11).

**R10. Preserve builder calls; do not rewrite definitions again.** Optional vocabulary is additive on 4a's builders, with serde defaults for absent new fields and builder defaults that reproduce existing behavior. The approved Basic and OAuth2 type changes are intentional exceptions for direct users of those stored types; query fidelity intentionally changes generated request constructors. Preserve 4a's builder call signatures. Basic values may change representation without requiring different authoring calls. A test proves the committed definitions need no source change except the deliberate adoptions of R12.
Validated by: a diff check recorded in the log showing zero changed definition-authoring lines beyond the deliberate adoptions; separately log mechanical Basic assertions and legacy test-fixture fields updated for the removed stored field; a compile test that 4a's builder call forms (a recorded sample per API) still compile.

**R11. Migration safety net and dependents.** *Generated output:* before changing anything, regenerate Rust, OpenAPI and Postman into a temporary directory and prove no diff against the committed trees (the baseline is the tree as left by 4a). After each requirement's batch and at the end, regeneration into a temporary directory is byte-identical to the baseline **or** every difference is explained in `implementation-log.md` per API and file with its cause (R4 adoption and R9 are expected to change shipped output; other differences require a specific explanation); the 4a comparison test runs on `just test` with an allow-list that is empty at the end unless one of those legitimately changes output. *Definitions:* keep 4a's original snapshots. Compare old and new definitions through an explicit normalization of newly defaulted fields, removed unused `env_username` keys, and equivalent old/new Basic environment representations. Allow deliberate adoption changes only for named APIs and fields. A test-only description mutation must still fail. Do not broadly ignore `auth`, `params` or a whole endpoint, and do not replace old snapshots merely to make the comparison pass. Add new bundle fixtures separately. *Dependents:* **inventory first**: before writing R9, search every dependent for constructors, struct literals, `Default` use (including `..Default::default()` on structs that lose the derive, D57 item 13), setters and enum-valued assignments of any endpoint affected by required or enum query fidelity (the parent cites 288 OpenAI query parameters), list the call sites, then migrate them in this spec. Dependents: `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, `homelab/lib`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`, plus `schematic-oauth` (R8). Required-parameter fidelity lands unconditionally and affected callers migrate in the same change. A compatibility option would create two generated APIs and is not authorized automatically by a call-site count; see the landing default below.
Validated by: L1 tests in `schematic-gen` (regeneration versus committed trees; 4a's JSON definition comparison with a deliberately mutated definition failing); `just test` and `just lint` in `schematic/`; `just test {pkg}` for each dependent; `just test-schema` and `just lint-schema`.

**R12. Deliberate adoptions in shipped definitions (the only definition source changes).** Default list, to be confirmed or reduced at planning:

| Vocabulary item | Adopting definition | Reason |
|---|---|---|
| rate-limit/retry (R4) | GitHub (`github/mod.rs`) | verified response-header metadata: `x-ratelimit-limit`, `x-ratelimit-remaining`, `x-ratelimit-reset` in epoch seconds, and optional `retry-after`; gives the vocabulary a real consumer and test |
| `Basic` shape (R5) | Bitbucket, EMQX | only if 4a's Basic authoring call needs a changed form; otherwise no source change |
| path parameters, auth override, `deprecated`, `oneOf` | none | exercised by fixture definitions; shipped definitions adopt later by authors, not by this spec |
| query fidelity (R9) | none (generator-only) | acts on data already in the definitions (the OpenAI-imported `required`, style, `explode`, enum flags) |

GitHub header meanings are documented in [GitHub's REST rate-limit guidance](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api). Adopt only these header facts initially. Do not label every HTTP 403 retryable or invent provider-wide attempt/backoff values; retry fixtures exercise guidance independently of the shipped GitHub definition.
Validated by: the R10 diff check lists exactly these files; output assertions for the adopting definitions. Assertions that still match the old Basic enum shape may be mechanically updated and logged; this is not another endpoint rewrite.

**R13. Documentation and skills reflect the new vocabulary.** The `schematic-define` README and doc examples, `schematic/docs/` topic pages, `.claude/skills/schematic-define/SKILL.md` and `.claude/commands/create-schematic-api.md` describe each new method (with a compact example per rule); the OAuth2 and `Basic` pages describe the new shapes. Rustdoc and comments of every symbol whose behavior changed get a drift pass (the code is right when they disagree). Docs never name a feature or fix directory.
Validated by: `just doctest` separately from nextest-based `just test`; search docs for positional Basic password lookup and the removed `env_username` field and flat-OAuth2 field names returning only deliberate historical mentions; reviewer check at review 1.

**R14. Pager-managed parameters are exempt from required fidelity (D55).** A query parameter that a pagination strategy adopts (spec 3's adopted-parameter rule) never appears as an argument of a request's `new()`, even when the provider marks it required: the pager always sends it (D55 item 5). How a direct send without a pager supplies such a parameter is settled by D57 item 3: adopted fields stay removed, and one page is fetched through the pager with `fetch_page()` (spec 3 R11a). If the provider requires such a parameter, the strategy must carry a type-valid, nonempty default where applicable for the first request, and definition validation rejects a definition whose strategy adopts a required parameter without one (error names API, endpoint, strategy and parameter). A required query parameter that no strategy adopts becomes a `new()` argument as R9 says.
Validated by: fixture-definition L1 tests in `schematic-gen` with two required query parameters on one paginated endpoint, one adopted by the strategy and one not: generated `new()` takes only the non-adopted one, and a mock-server test sees the adopted parameter on the first and subsequent pages with the expected changing continuation values; a negative test in `schematic-define` for an adopted required parameter with no strategy default; a generated-output assertion that an OpenAI-style paginated endpoint's `new()` omits the adopted `limit`/`after`-style parameters.

## Design

"Decided" means the parent decided it (cite). Everything else is **Proposed**.

| Symbol | Package | What it does | Status |
|---|---|---|---|
| `EndpointParams::path`, `.path_param(..)` | `schematic-define` | typed path parameters, validated against the template | Decided that `EndpointParams` gets `path` (D18); element type Proposed (PD2) |
| `Endpoint::auth: Option<EndpointAuth>`, `.public()`, `.auth(..)` | `schematic-define` | public or alternative `AuthPolicy` | Decided that an override exists (D18); shape Proposed (PD4) |
| `deprecated` on `Endpoint` and `ParamDef`, `.deprecated()` | `schematic-define` | flag | Decided (D18); shape Proposed (PD3) |
| `RateLimit`, `RetryGuidance`, `.rate_limit(..)`, `.retry_guidance(..)` | `schematic-define` | declare rate-limit headers and retry guidance; metadata only | Decided that the vocabulary exists (D18); entire shape Proposed (PD1) |
| `AuthStrategy::Basic { .. }` carrying env lists | `schematic-define` | retires `env_auth[0]` | Decided (D18); shape Proposed (PD6) |
| `ctx.one_of::<RustType>(..)` and kin; `ModelCatalog` on the context | `schematic-define` | structural variants bound to a Rust type, via the descriptor's structural component | Decided that the bridge exists (D18); shape Proposed (PD5) |
| `DefinitionBundle` (derives on the core types come from 4a, D55) | `schematic-define` | self-describing serialized definition that loads in isolation | Decided that definitions are serializable (D18, D55); bundle Proposed (PD7) |
| `Default` derive only on request structs with no required non-pager query field and no typed path field | `schematic-gen` (`codegen/request_structs`) | a required input cannot bypass `new()` | Decided (D57 item 13) |
| pager-adopted parameter exemption and strategy-default validation | `schematic-define`, `schematic-gen` (`codegen/request_structs`) | adopted required query parameters stay out of `new()` | Decided (D55); validation shape Proposed |
| `OAuth2Config` as per-grant variants | `schematic-define` (consumers in `schematic-oauth`, `schematic-gen`) | illegal grant combinations unrepresentable | Decided (D18, "breaking"); shape Proposed (PD9) |
| generator/exporter consumers for path params, auth override, deprecated, rate limits, query fidelity, `oneOf` | `schematic-define` (`openapi/export`), `schematic-gen` (`codegen/request_structs`, `codegen/client`, `export/openapi`, `export/postman`) | make the vocabulary reach outputs | Proposed (reach-or-diagnose is Decided, D10) |
| fixture definitions and JSON baselines | `schematic-gen` | tests for every item; R7 baseline | Proposed |

## Suggested phases

Resolve the remaining Open Question before planning dependent phases. Each phase leaves the tree green (`just test`, `just lint` in `schematic/`, R11 comparison clean or explained). The baseline is the tree as left by 4a.

1. **Baseline and inventory.** Re-verify counts (Basic and OAuth2 users, dependents, required query parameters, `x-schematic` consumers). Prove regeneration reproduces the committed trees. Inventory dependent call sites that R9 will break. No code change except the log.
2. **Self-contained serialization (R7).** Bundle, load in isolation, round trip over bundles. Additive; 4a's derives and JSON snapshots are the starting point.
3. **Define-side additions, additive ones.** Path parameters, `deprecated`, auth override, rate-limit/retry (R1-R4, define side), each with validation and builder methods.
4. **`Basic` shape (R5).** Migrate `legacy_env_mapping_for`, Bitbucket and EMQX; byte compare.
5. **`oneOf` (R6).** Context registration over the descriptor's structural component.
6. **OAuth2 per-grant (R8).** Breaking; `schematic-oauth` and generator files.
7. **Generator consumers.** R1-R4 and R6 outputs with fixture definitions and D10 diagnostics; R12 adoption.
8. **Query-parameter fidelity (R9) and the pager exemption (R14).** Last, because it changes shipped output and dependents; migrate dependents from the phase 1 inventory.
9. **Regenerate, dependents, docs.** Regenerate `schematic-schema`, OpenAPI and Postman with the generator (never by hand); review every explained diff; dependents; R13; final gates.

## Acceptance criteria and definition of done

- Every requirement in Requirements has a passing check. An unsupported output may be covered by the specified diagnostic test; an implementation log does not waive missing behavior. Unresolved requirements are reported as incomplete.
- The workspace builds and tests pass on macOS, Linux, native Windows and WSL2 per repo rules. The implementer proves the host it runs on and records which hosts could not be exercised; load the `os` skill before claiming any OS cannot be tested; no CI scope change.
- `just test`, `just lint`, and `just doctest` pass in `schematic/`; `just test-schema` and `just lint-schema` pass for the generated crate.
- Generated trees are regenerated by `just generate`, never hand-edited; every difference from the baseline is explained in the log (R11).
- A generated request with a required query field has no `Default` impl (compile-fail test); an all-optional request keeps it (R9, D57 item 13).
- The committed definitions changed only as R12/R10 list.
- Every dependent builds and passes its tests; each edited call site is logged.
- Docs, README, skill and command updated; behavior decided but not yet built is marked **planned**; a drift pass covers every symbol whose behavior changed.
- No commits unless the plan says so (then identity and signing rules: `git verify-commit HEAD`, no agent trailers, never `--no-gpg-sign`).
- The agent never moves this spec to `_completed` and never runs `just complete`. Terminal state: "implementation complete, ready for review".

## Pre-authorizations

The author must confirm these before the unattended run (D50 item 8). In an unattended run, log newly discovered scope needs and continue independent authorized work; report required blocked work as incomplete rather than silently dropping it.

- Regenerate `schematic-schema`, `schematic/openapi` and `schematic/postman` with the generator.
- Change `OAuth2Config` and edit `schematic-oauth` and the generator files that consume it (R8).
- Edit dependent crates (`sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, the five homelab crates) only as far as generated required-parameter signatures, typed-enum fields and the removed `Default` derive (D57 item 13) force (R9, R11).
- Edit the shipped definitions only at the R12 adoption sites.
- Add test fixtures and JSON baselines under `schematic/gen/tests/fixtures/`.
- Add a dependency only if unavoidable (record in `docs/dependencies.md`). Expected: none.
- Outside the proposed authorization: CI configuration changes, commits, deletion outside the listed scope, and live provider calls. These are not default actions in an unattended run.

## Dependencies and ordering

- **Needs landed first:** `2026-10-14-schematic-definition-api` (4a): the builders, the context, the descriptor's optional structural component and serde derives, the plain serde derives on `RestApi`, `Endpoint`, `EndpointParams`, `ParamDef` (and `QueryParamType`, `ParamStyle`) with JSON snapshots and their comparison test (D55), the hard error for `?` in path templates, and the `auth_policy` retention and `oauth_scopes` removal (D56). Earlier specs (1 diagnostics and isolated compile check; 2 runtime types; 3 pagination) are transitive.
- **Blocks:** spec 5 (reads the context and descriptors, may use `oneOf` registration for typed errors); spec 6 (shares `ParamDef`, auth and env types as changed here); spec 7 (reads definitions through the serializable form and takes the accepted-gap candidates).
- **Internal order:** query fidelity (R9, R14) last. Also needs spec 3's adopted-parameter rule before R14.

## Planning-time details assigned to this spec

Defaults below may be adopted and logged unless the author rules otherwise. The endpoint-authentication shape remains provisional until the credential-source Open Question is resolved.

**PD1: rate-limit and retry vocabulary shape.** The parent gives no shape. Default if not ruled before planning: API-level `RateLimit { limit_header, remaining_header, reset_header, retry_after_header: Option<String>, reset_unit: Seconds|EpochSeconds|HttpDate }` and `RetryGuidance { retry_on: Vec<StatusSelector>, honor_retry_after: bool, max_attempts: u8, backoff: None|Fixed(ms)|Exponential{base_ms,cap_ms} }`, endpoint-level override of either; descriptive only, no runtime behavior. `StatusSelector` is a tagged `Code(u16)` or inclusive `Range { start: u16, end: u16 }`; validate HTTP status bounds 100–599 and ordered range endpoints. `max_attempts` includes the initial attempt and must be at least one; fixed delays and exponential base are positive, and the cap is at least the base. Header names must be legal HTTP names. `reset_unit` applies only to the reset header; `Retry-After` keeps its own HTTP semantics. An absent endpoint value inherits; a present value replaces the entire API-level declaration rather than partially merging fields. No suppression builder is added without an actual adopting endpoint needing it. Flag at review 1: consumers are documentation and the OpenAPI extension only, the same shape of problem D49 calls out for `oauth_scopes`.

**PD2: typed path parameter element.** Default if not ruled before planning: reuse `ParamDef` (name, description, `QueryParamType`, `required` forced to `true`, style `Simple`) so WebSocket can share it (D8); validation rejects a path parameter whose type is `Array` or `Json`, or style is not `Simple`. The path builder sets `required: true`; a malformed deserialized path declaration with `required: false` is rejected rather than silently normalized. Duplicate names and enum wire values are errors. Use `i64`, `f64`, `bool`, `String`, or the generated enum as appropriate; reject non-finite numeric values before sending.

**PD3: `deprecated` shape.** Default if not ruled before planning: a plain `bool` on `Endpoint` and `ParamDef`, with `#[serde(default)]` so older additive definitions load as not deprecated; docs get a "Deprecated." line; OpenAPI `deprecated: true`; no `#[deprecated]` attribute in generated Rust (it would fail `-D warnings` linting of the generated crate and warn in dependents); no note or since-version field.

**PD4: per-endpoint auth override shape.** Provisional shape, subject to the credential-source Open Question: `Endpoint::auth: Option<EndpointAuth>` with `EndpointAuth::Public` and `EndpointAuth::Policy(AuthPolicy)`; `None` inherits. No optional-auth variant (recorded under Risks).

**PD5: polymorphic body shape.** Default if not ruled before planning: `ctx.one_of::<RustType>([variants...])` (and `any_of`, `all_of`) where `RustType` is an existing hand-written serde type whose JSON representation matches the composition and `variants` are descriptors from the same context; the structural `TypeRef` is stored in a `ModelCatalog` attached to the context and exported to OpenAPI; Postman gets the explicit unsupported-composition diagnostic described in R6. The Rust generator only references `RustType`, and does not also generate its catalog type.

**PD6: `Basic` shape (and the dual-source remainder).** Default if not ruled before planning: `AuthStrategy::Basic { username_env: EnvList, password_env: EnvList }`; `env_username` is removed from the stored definition without changing existing builder calls; `env_auth` stays for Bearer and ApiKey; `env_mapping` and `auth_policy` remain explicit overrides. Do only what R5 requires; the unification of `auth`/`env_auth` with `auth_policy`/`env_mapping` is an open assignment.

**PD7: serialized bundle.** Default if not ruled before planning: `DefinitionBundle { format_version: u32, api: RestApi, bindings: BTreeMap<String, RustTypeBinding>, schemas: BTreeMap<String, serde_json::Value>, models: Option<ModelCatalog> }` with schemas as the OpenAPI-3.0-settings JSON the registry already produces; `format_version` starts at 1 and unsupported versions fail explicitly. `RustTypeBinding` records the schema identity and Rust type path supplied by the context, not executable code. Sort map keys recursively when writing canonical JSON; a top-level `BTreeMap` alone does not guarantee stable nested maps. Preserve array order where meaningful, including variant order and environment fallback order. A bundle loads in isolation: loading validates that every by-name reference resolves inside the bundle.

**PD8: query-parameter fidelity rules.** Default if not ruled before planning: supported styles are `Form` (scalar or one-dimensional scalar/enum array, explode true and false), `SpaceDelimited` and `PipeDelimited` (one-dimensional arrays, explode false). Unsupported type/style/explode combinations, including nested arrays or object-valued JSON, yield an output-specific diagnostic rather than reverting to strings. Optional arrays use `None` for omission; an explicit empty array emits `name=`. Encode each element once before joining non-exploded values with the style delimiter, so a literal delimiter inside an element remains escaped; do not URL-encode the joined string again. The space delimiter is `%20` and the pipe delimiter is `%7C`; commas remain literal separators. For example, Form non-exploded `tags = ["a,b", "c"]` becomes `tags=a%2Cb,c`, while exploded becomes `tags=a%2Cb&tags=c`. These rules follow the [OpenAPI parameter serialization table](https://spec.openapis.org/oas/v3.0.3#style-values). `DeepObject` and `Simple` on a query parameter yield the D10 backstop diagnostic for the Rust output; `Enum(values)` generates `pub enum {ResolvedRequestType}{ParamPascalCase}` with a variant per value (sanitized identifiers, collision-checked, wire spelling preserved with `#[serde(rename)]`), `#[non_exhaustive]`, with serde derives and a shared wire-value accessor used by path/query serializers. Include 4a's resolved request suffix in the name, and check collisions across the whole generated module, including grouped APIs and request/body names. Empty value sets, duplicate wire values and sanitized variant collisions are validation errors, with no panic or silent renaming. The same enum machinery applies to enums inside arrays and typed path parameters. `required: true` puts the parameter in `new()`, except for a parameter a pagination strategy adopts (R14, D55).

**PD9: OAuth2 per-grant shape.** Default if not ruled before planning: `OAuth2Config` becomes a `#[non_exhaustive]` enum with variants `AuthorizationCodePkce(..)`, `ClientCredentials(..)`, `DeviceCode(..)`, each a struct with only its legal URLs plus shared `default_scopes`, `pkce` where it applies (using the authorization-code-specific type described in R8), and `client_auth`; the serde representation is internally tagged by `grant_type`, with explicit variant renames preserving current grant spellings (`AuthorizationCodePkce`, `ClientCredentials`, `DeviceCode`). Preserve shared field names and omit grant-inapplicable fields. This is an intentional serialized-shape change, not byte-compatible merely because the tag name is unchanged. Serialize only fields applicable to the selected grant. Deserialization rejects non-null contradictory fields and validates required URLs and PKCE settings. No legacy compatibility adapter is required; existing serialized test fixtures migrate explicitly and tag-spelling tests guard against accidental changes. Preserve optional `revocation_url` on each variant; `OAuth2GrantType` stays as an accessor result.

**PD10: query-fidelity landing.** Default: land required fields and typed enums unconditionally and migrate affected callers. **Reader's note:** the former draft offered a generator flag after an arbitrary number of broken call sites. That would maintain two generated APIs without solving incorrect requests. In this early-lifecycle repository, a single migration is simpler and consistent with the strategic preference for low technical debt. If inventory reveals an actual blocking integration, record the concrete problem for the author; a large count alone does not authorize a compatibility mode.

## Open Questions

These choices affect the public API or runtime behavior and should be resolved by the author before a dependent implementation plan is finalized. The recommendations below do not override the coordinating review's approved rules.

### Where do credentials for a replacement endpoint policy come from?

The existing [AuthPolicy](../../define/src/auth.rs) in `schematic-define` identifies accepted authentication methods and an environment fallback, while [RestApi's environment mapping](../../define/src/types.rs) supplies variable names separately. Replacing bearer authentication with Basic or a query key therefore needs more than an endpoint policy alone. Reusing the API's credential bytes without checking the selected method could send the wrong secret to a new location.

| Solution | Pros | Cons |
|---|---|---|
| **Recommended: endpoint policy with an optional endpoint environment mapping** | Can describe a genuinely different credential source; reuses existing definition types; keeps explicit runtime credentials separate from environment names. | Adds a mapping field and builder overload; requires explicit precedence and validation. |
| Reuse only the API environment mapping | Keeps the proposed endpoint shape small. | Cannot name independent credentials for two endpoints using the same method; rejects some otherwise valid overrides. |
| Require explicit runtime credentials for every replacement policy | Avoids endpoint environment metadata. | Makes environment-only clients unable to call replacement endpoints; needs a new credential injection interface and is less convenient. |

**Recommendation and proposed contract.** Add a builder form accepting a policy and optional mapping while preserving `.auth(policy)` for reuse of compatible API mappings. Resolution is endpoint mapping if present, otherwise the API's effective mapping; an explicit endpoint mapping replaces the whole mapping, matching existing API-level precedence. Resolve only the credential method selected by the endpoint, and reject missing or incompatible sources before sending. Credentials injected for one method cannot automatically become credentials for a different method. If the runtime-envelope implementation stores only one prebuilt auth header and cannot represent the replacement method, report that output as unsupported rather than reinterpret the header. Include mock tests for distinct bearer and Basic sources, query/cookie replacement, and missing sources. This enables the approved override goal without expanding into a general authentication redesign.

Whether request structs keep `Default` once they carry required fields was ruled in D57 item 13; the rule is stated in R9.

How a direct send without a pager supplies a required pager-managed parameter is not an Open Question here: D57 item 3 settled it (`fetch_page()` on the pager, spec 3 R11a), and R14 follows it.

## Risks and failure modes

- **Query fidelity breaks dependents.** Required parameters change generated `new()` signatures; the OpenAI-imported definitions carry many required flags. Mitigation: inventory all affected constructor/field/enum uses first and migrate in the same spec; no automatic compatibility flag. Pager-adopted parameters are excluded from the count (R14), which should reduce the broken call sites. Request structs that gain a required query field or a typed path field also lose `Default` (D57 item 13); dependents that build them with `Default` migrate to `new()` in the same change.
- **Pager exemption depends on spec 3's adoption data.** R14 needs a queryable list of the parameters each strategy adopts and a place for the strategy default; neither shape existed at drafting. If spec 3 adopts a parameter by a mechanism R14 cannot see, a required paginated parameter would wrongly become a `new()` argument.
- **Rate-limit vocabulary has no mandated shape or runtime consumer.** Risk of becoming another unused field like `oauth_scopes`. Mitigation: the D49 criterion (reader plus test per field), one adopting shipped definition, review 1 item. If review 1 finds the vocabulary unwanted, removing it is cheap because nothing else reads it.
- **Descriptor constraint from 4a may not fit `oneOf`.** Unverified: 4a's structural carrier did not exist at drafting. R6 permits extending, not replacing, the descriptor.
- **OAuth2 break touches `schematic-oauth`.** Its mock-server and serde tests may be sensitive to the serialized form; the per-grant default preserves tag spellings and explicitly migrates affected fixtures.
- **Serialization of by-name `Schema` references is not self-contained.** The bundle (PD7) carries the schemas; round-trip equality must be tested on bundles, not bare `RestApi`. 4a's JSON snapshots stay bare-definition snapshots; if they drift from the bundle form, the snapshot comparison remains authoritative for the definition itself.
- **Byte-identical claim vs R9.** Query fidelity legitimately changes shipped output; the allow-list must be per-file and explained, not blanket.
- **Basic migration regressions.** Bitbucket and EMQX are the only users; any change in environment mapping would break real credential lookup. Mitigation: byte comparison and a unit test of `basic_user`/`basic_pass`.
- **4a's outcomes are unverified here.** Builder names, the typestate shape, reserved names and the descriptor component did not exist when this was drafted; the refresh must replace them with 4a's actual surface.
- **Unassigned adjacent gaps (not done here; accepted-gap candidates for spec 7's ledger table, D20).** The optional-auth variant (Ollama ignores API keys but accepts them); the broader legacy/modern auth dual-source cleanup beyond `env_auth[0]`; invented HuggingFace stub shapes; orphaned types.
- **Long unattended run.** Each phase ends only when tests pass; the log records the last completed phase; commit-free.

## Process

Per D50 and D51 (parent), these specs are built for the following process:

1. This spec has a 1:1 plan, implemented end to end, phase by phase, **without a human in the loop**.
2. Then **review 1**: time is set aside for the author to review decisions made during implementation and open design questions.
3. Then the **next** spec and plan are refreshed from the outcomes of review 1 (this spec's review 1 refreshes spec 5 and spec 6 where they touch the changed types).
4. Then fix and review cycles repeat until the reviewer deems the spec production ready.

An agent's terminal state is "implementation complete, ready for review". Only the author moves a spec to `_completed`; an agent never does and never runs `just complete`.

Because no human is available while the plan runs, routine planning-time details have defaults in Planning-time details; the remaining major Open Question must be resolved before the dependent work starts, and the implementer logs each item **as it occurs** in `implementation-log.md` (format of the implementation log for `2026-10-09-more-pr-support` in the worktree package area): adopted defaults, design choices this spec does not dictate, deviations, open questions. If a requirement cannot be met as written, log the blocker and continue independent work. Any mitigation must preserve the approved behavior; report the implementation incomplete if required behavior remains missing. Open Questions recommendations require author resolution before dependent implementation, and cannot be silently promoted to approved decisions.

## Human review agenda

Record each in `implementation-log.md`, as it occurs, for review 1:

1. **Every default adopted:** every default in Planning-time details, with the value used and any adjustment.
2. **Rate-limit/retry vocabulary:** final shape, readers, the adopting definition, and a keep/drop recommendation (the unused-field risk).
3. **The adoption list (R12):** which definitions changed and why; confirmation that no other definition authoring line changed except logged mechanical Basic assertions (R10 diff check).
4. **Generated-output differences:** the per-API, per-file explained diffs from R11 with cause.
5. **Dependents:** the phase 1 inventory, each call site edited in sniff, biscuit-speaks, model-citizen, the homelab crates or `schematic-oauth` (including `Default` uses migrated under D57 item 13), and any integration that prevented unconditional query fidelity.
6. **The descriptor and `oneOf` bridge:** whether 4a's structural carrier sufficed (R6) and what was extended.
7. **Basic and OAuth2 changes:** final shapes, and the unassigned auth remainder.
8. **Every design choice not dictated by this spec, every deviation, every open question.**
9. **Tests deleted or replaced** and doc, skill and command updates.
10. **Self-contained serialization (R7):** the bundle shape used, how isolation loading was proven, and any by-name reference that could not be bundled.
11. **Pager exemption (R14):** how adopted parameters were detected from spec 3's data, where the strategy default lives, every shipped endpoint whose `new()` omits a required parameter because of it, and any definition validation rejected.
12. **Hosts and operating systems actually exercised**, and which of macOS, Linux, native Windows, WSL2 were not.

## Refresh notes

`refresh_required: true`. Before this spec's plan is made, refresh it from the review 1 outcomes of specs 1-3 and, most of all, 4a. Things those outcomes may change:

- **From 4a:** the final builder method names and reserved names (`.path_param`, `.public`, `.auth`, `.deprecated`, `.rate_limit`, `.retry_guidance`); the typestate shape (where new required-or-optional facts attach); the descriptor's structural component and serde derives (R6); the JSON snapshot fixtures and comparison test (R7, R11); the Basic authoring call's signature (R5, R12); confirmation of the D56 outcome (`auth_policy` kept, `oauth_scopes` removed, runtime trait `EndpointSpec`); whether `#[non_exhaustive]` stayed (new fields rely on it); the serde derives that exist (R7, D55); the `?`-in-path hard error and the nine migrated endpoints (D54, R1 relies on it); the `EndpointParams` `Default` allow-list (R1 adds a field to it); the true endpoint count and any definition changes 4a made.
- **From spec 1:** diagnostics format and validator API the new builders call; the isolated compile check's form; the list of unsupported-feature diagnostics (R2, R9).
- **From spec 2:** whether the runtime consumes anything resembling rate-limit or retry guidance (R4, PD1); the mock-server harness for R2 and R9 tests.
- **From spec 3:** the shape of the adopted-parameter data and of the strategy default (R14). Resolved by D55: a parameter managed by a pagination strategy never becomes a `new()` argument; if the provider requires it, the strategy must carry a default and validation rejects a definition that does not; the pager always sends it. The refresh only has to bind R14 to spec 3's actual types and to spec 3's `fetch_page()` (D57 item 3). D53 (no convenience methods) is unchanged by anything here.
- **Counts:** re-verify Basic users (Bitbucket, EMQX), OAuth2 users (none), dependents and required query parameters.
- **From the author's rulings at review 1:** any ruling on the PD items replaces the defaults; rulings on the unassigned items are added.
