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
reviewed: false
clarified: false
implemented: false
human_review: false
message_to_agent: ''
parent: ../../reviews/2026-07-31-macro-review/spec.md
depends_on:
    - 2026-10-11-schematic-definition-api
refresh_required: true
related: []
---

# Schematic definition vocabulary

## Problem

`schematic-define`'s vocabulary is smaller than the APIs it describes (parent section 4, D18). After spec 4a every definition is written with builders and a typed definition context, so adding vocabulary no longer means rewriting about 519 endpoint literals: each item below is a **new builder method or new field** on 4a's builders, and shipped definitions keep compiling unchanged unless one deliberately adopts an item. Verified in the parent review (July evidence, re-checked in part on 2026-10-11 by this draft):

1. `EndpointParams` has `query`, `header`, `cookie` but no `path`: path placeholders are untyped strings.
2. There is no per-endpoint auth override (a public endpoint on an authenticated API cannot be stated), no `deprecated` (the OpenAPI exporter hard-codes `deprecated: None`), and no rate-limit or retry vocabulary.
3. Basic auth keeps its password positionally in `env_auth[0]` (`schematic/define/src/types.rs`, `legacy_env_mapping_for`).
4. A hand-written definition cannot attach structural `oneOf` models.
5. `RestApi`, `Endpoint` and `EndpointParams` are not `Serialize`.
6. `OAuth2Config` is a flat struct on which illegal grant and URL combinations can be written.
7. The Rust generator discards query-parameter `required`, `style`, `explode` and `Enum`.

D52 (2026-10-11) split the original definition-API spec: 4a is the rewrite and builders; this spec (4b) is the vocabulary (D18's list). Part of this vocabulary changes shipped generated output (item 7) or breaking types (item 6); that is why it is separate from a pass whose proof is "output is unchanged".

## Goal and non-goals

**Goal.** Every item above exists as new builder methods or fields on 4a's builders, is consumed by the outputs that can use it (or yields the D10 backstop diagnostic), is tested with fixture definitions, and leaves the committed definitions compiling with no source change except the deliberate adoptions listed under R12.

**Non-goals (out of scope, with owner).**

- The builders, context, descriptors, typed registration, the rewrite of the endpoint literals, the pagination removal, the importer, `auth_policy`/`oauth_scopes`, per-endpoint `cross_origin`, `request_id_header` and `.secret()` sugar: spec 4a. This spec relies on its outcomes.
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
- **D1 (scope).** Nothing is deferred; every D18 item is built.
- **D2/D44 (typed registration).** Descriptors come only from the definition context. 4a leaves an optional structural component and serde derives on the descriptor (author-approved constraint, unused in 4a); R6 uses it.
- **D10 (backstop rule).** A declared feature must either reach the selected output or produce a diagnostic identifying API, endpoint, feature and unsupported target; one output's gap never blocks another output.
- **D8 (shared building blocks).** WebSocket reuses `ParamDef`, auth types and `EnvMapping`; do not bake REST-only assumptions into them.
- **D49 (dead-surface criterion).** A field is either read by generator or exporter code and covered by a test, or it does not exist. R4 must meet this despite being metadata (see Risks).
- **D50 (plan structure).** Each spec leaves the repo green on macOS, Linux, native Windows and WSL2; every dependent builds and passes; terminal state is "implementation complete, ready for review". Pre-authorizations are items the author must confirm before the unattended run (D50 item 8).

The runtime trait is `EndpointSpec`; `Endpoint` here is always the definition struct.

## Requirements

"Package" names the crate: `schematic-define`, `schematic-definitions`, `schematic-gen`, `schematic-schema` (generated), `schematic-oauth`. Counts are re-verified at the start of the run (R11).

**R1. Typed path parameters.** `EndpointParams` gains `path` (PD2 for the element type) and the endpoint builder gains `.path_param(..)`. When an endpoint declares path parameters, the declared set must equal the set of `{name}` and `{+name}` placeholders in the path template (build error otherwise, naming API, endpoint and parameter); an endpoint that declares none keeps today's behavior (every placeholder is an untyped string field). Declared type and description reach the generated Rust request struct (field type and doc) and the exported OpenAPI path parameters (type, description, `required: true`), still emitted once per path item, not once per method (a previously fixed defect).
Validated by: L1 unit tests in `schematic-define` (template/declaration match and mismatch); fixture-definition L1 tests in `schematic-gen` asserting generated Rust and exported OpenAPI for a fixture endpoint with an integer path parameter; the isolated compile check (spec 1) compiles the fixture output.

**R2. Per-endpoint auth override.** An endpoint may declare that it is public or that it uses a different `AuthPolicy` than the API (PD4), through builder methods `.public()` and `.auth(..)`. Generated clients send no credential for a public endpoint; OpenAPI export emits `security: []` (or the override's scheme); Postman export uses `noauth` (or the override). If an output cannot express the override, the D10 backstop diagnostic names API, endpoint, feature and output, and the other outputs still generate.
Validated by: mock-server L1 test in `schematic-gen` (a generated client against a local mock records no `Authorization` header on a public endpoint of an authenticated fixture API and the credential on a sibling endpoint); generated-output assertions for OpenAPI and Postman.

**R3. `deprecated`.** `Endpoint` and `ParamDef` gain `deprecated` (PD3), through `.deprecated()` builder methods. It reaches OpenAPI (`deprecated: true` on the operation or parameter; the exporter stops hard-coding `deprecated: None`) and the generated Rust docs. It adds no Rust `#[deprecated]` attribute so `just lint-schema` and dependents stay warning-free.
Validated by: L1 generated-output assertions in `schematic-gen` for Rust docs and OpenAPI; round trip in `schematic-define` (serialize/deserialize keeps the flag, after R7).

**R4. Rate-limit and retry vocabulary (metadata only).** The definition can state, at API level with an optional endpoint override, which response headers carry rate-limit information and the provider's retry guidance (shape in PD1; builder methods `.rate_limit(..)` and `.retry_guidance(..)`). The vocabulary is validated, serializable, and exported (OpenAPI `x-schematic` operation/document extension and generated documentation). **This spec adds no automatic retry, sleeping or enforcement to the runtime.** To avoid becoming another unused field like `oauth_scopes`, the D49 criterion applies: every field has a reader (the exporter and the docs generator) and a test, and at least one shipped definition adopts it (R12).
Validated by: L1 unit tests in `schematic-define` (validation of header names and numeric bounds); generated-output assertions in `schematic-gen`; a source-scan test that every field of the new types has a non-test reader; the adopting definition's output assertion; a test (grep-level) that the runtime crate does not read these types.

**R5. `Basic` carries its environment shape; `env_auth[0]`-as-password is retired.** `AuthStrategy::Basic` names its username and password environment lists itself (PD6). `env_username` is removed or unused for Basic, and no code reads `env_auth.first()` as a password. The 4a builder's Basic authoring call keeps its signature, so Bitbucket and EMQX (the only shipped Basic APIs, verified by grep of `AuthStrategy::Basic`) are migrated with no change to generated environment mapping and docs. If 4a's call did not accept both lists, Bitbucket and EMQX are the deliberate adoption in R12.
Validated by: L1 unit tests in `schematic-define` (the `legacy_env_mapping_for` replacement yields `basic_user`/`basic_pass`); the safety net (R11) byte-compares Bitbucket and EMQX output; source scan asserting no `env_auth.first()` password read remains.

**R6. Polymorphic `oneOf` bodies from hand-written definitions.** `DefinitionContext` can register a polymorphic type described structurally (`TypeRef::OneOf`, `AnyOf`, `AllOf` over registered variants) and bind it to the Rust type that deserializes it (PD5). The structural model is stored through the optional structural component 4a left in the descriptor and in a `ModelCatalog` on the context, is available to OpenAPI export (`oneOf` with `$ref`s) and Postman, and the Rust generator references the bound Rust type. A variant that is not registered is a validation error. If 4a's descriptor component cannot carry what is needed, the descriptor type may be extended, never replaced, and the change is logged for review (no definition names the descriptor's fields, so none is rewritten).
Validated by: fixture-definition L1 tests in `schematic-gen` (OpenAPI contains `oneOf` with the variants' refs; Rust output references the bound type; isolated compile check passes); negative test for an unregistered variant in `schematic-define`; round trip of a descriptor with a `Some` structure.

**R7. Serializable definitions.** `RestApi`, `Endpoint`, `EndpointParams`, `ParamDef` and every type they contain derive `Serialize` and `Deserialize` (`ParamDef`, `QueryParamType`, `ParamStyle` and the descriptor already do, from 4a). A definition together with the schemas its context registered serializes as one self-contained bundle (PD7), because by-name `Schema` references alone are not self-contained. Serialization of every shipped definition round-trips to an equal value and is deterministic. No secret values exist in definitions (only environment variable names), so serialization needs no redaction.
Validated by: L1 round-trip and byte-equal determinism test over all 18 REST APIs in `schematic-definitions`; the 4a `Debug` baseline fixtures are re-baselined into JSON fixtures and the comparison test (with its mutation check) is moved onto them (PD10).

**R8. OAuth2 per-grant payloads (breaking).** `OAuth2Config` becomes one variant per grant, each carrying only the fields legal for it (authorization-code with PKCE, client credentials, device code), so an illegal combination (a device-code grant with no device authorization URL) cannot be written (PD9). Consumers in `schematic-gen` (`codegen/api_struct/auth.rs`, `request_method.rs`, `client/helpers.rs`, `export/auth.rs`, `output/assemble/shared.rs`, `codegen/module_docs.rs`) and `schematic-oauth` (`types.rs`, `manager/*`, tests) migrate. No shipped definition uses OAuth2 (verified by grep at drafting), so shipped generated output is expected not to change; fixture definitions prove each grant.
Validated by: L1 unit tests in `schematic-define` (each grant constructs; illegal combinations are unrepresentable, compile-fail where the point is the type); fixture-definition generation tests in `schematic-gen`; the existing `schematic-oauth` tests (`just test` runs them, including `oauth/tests/wiremock_flows.rs`) pass against the new type.

**R9. Query-parameter fidelity in the Rust generator.** For query parameters (`ParamDef`): `required: true` makes the generated request constructor take the parameter and the field non-optional; `explode`/`style` control array serialization for the supported styles (PD8; unsupported styles produce the D10 backstop diagnostic, not silent loss); `QueryParamType::Enum` generates a Rust enum (PD8 for naming) instead of a string. Header and cookie parameters keep their existing behavior. This changes generated `new()` signatures: required parameters become constructor arguments. Dependents are inventoried first (R11) and migrated in this spec.
Validated by: fixture-definition L1 tests in `schematic-gen` asserting generated source and, with a local mock server, the exact query string on the wire (array explode true versus false; enum value spelling; required parameter present); isolated compile check; dependents' builds (R11).

**R10. Everything is additive on 4a's builders; no definition is rewritten again.** Each item is a new method or field with a default that reproduces today's meaning; none changes a 4a signature. A test proves the committed definitions need no source change except the deliberate adoptions of R12.
Validated by: a diff check recorded in the log showing zero changed lines in `schematic/definitions/src` other than the files and lines the adoption list in R12 names; a compile test that 4a's builder call forms (a recorded sample per API) still compile.

**R11. Migration safety net and dependents.** *Generated output:* before changing anything, regenerate Rust, OpenAPI and Postman into a temporary directory and prove no diff against the committed trees (the baseline is the tree as left by 4a). After each requirement's batch and at the end, regeneration into a temporary directory is byte-identical to the baseline **or** every difference is explained in `implementation-log.md` per API and file with its cause (only R3 adoption, R4 adoption and R9 are expected to change shipped output); the 4a comparison test runs on `just test` with an allow-list that is empty at the end unless one of those legitimately changes output. *Definitions:* the R7 JSON comparison. *Dependents:* **inventory first**: before writing R9, grep every dependent for use of generated request `new()` of any endpoint with a required query parameter (the parent cites 288 OpenAI query parameters), list the call sites, then migrate them in this spec. Dependents: `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, `homelab/lib`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`, plus `schematic-oauth` (R8). If the number of broken call sites is large, R9 lands behind a clearly named generator option defaulting to on, and the choice is logged.
Validated by: L1 tests in `schematic-gen` (regeneration versus committed trees; JSON definition comparison with a deliberately mutated definition failing); `just test` and `just lint` in `schematic/`; `just test {pkg}` for each dependent; `just test-schema` and `just lint-schema`.

**R12. Deliberate adoptions in shipped definitions (the only definition source changes).** Default list, to be confirmed or reduced at planning:

| Vocabulary item | Adopting definition | Reason |
|---|---|---|
| rate-limit/retry (R4) | GitHub (`github/mod.rs`) | well-known headers (unverified at drafting); gives the vocabulary a real consumer and test |
| `Basic` shape (R5) | Bitbucket, EMQX | only if 4a's Basic authoring call needs a changed form; otherwise no source change |
| path parameters, auth override, `deprecated`, `oneOf` | none | exercised by fixture definitions; shipped definitions adopt later by authors, not by this spec |
| query fidelity (R9) | none (generator-only) | acts on data already in the definitions (the OpenAI-imported `required`, style, `explode`, enum flags) |

Validated by: the R10 diff check lists exactly these files; output assertions for the adopting definitions.

**R13. Documentation and skills reflect the new vocabulary.** The `schematic-define` README and doc examples, `schematic/docs/` topic pages, `.claude/skills/schematic-define/SKILL.md` and `.claude/commands/create-schematic-api.md` describe each new method (with a compact example per rule); the OAuth2 and `Basic` pages describe the new shapes. Rustdoc and comments of every symbol whose behavior changed get a drift pass (the code is right when they disagree). Docs never name a feature or fix directory.
Validated by: doctests (`just test` runs them); grep over docs for the removed `env_username` and flat-OAuth2 field names returning only deliberate historical mentions; reviewer check at review 1.

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
| `Serialize`/`Deserialize` on definition types; `DefinitionBundle` | `schematic-define` | self-contained serialized definition | Decided that definitions are serializable (D18); bundle Proposed (PD7) |
| `OAuth2Config` as per-grant variants | `schematic-define` (consumers in `schematic-oauth`, `schematic-gen`) | illegal grant combinations unrepresentable | Decided (D18, "breaking"); shape Proposed (PD9) |
| generator consumers for path params, auth override, deprecated, rate limits, query fidelity, `oneOf` | `schematic-gen` (`codegen/request_structs`, `codegen/client`, `export/openapi`, `export/postman`) | make the vocabulary reach outputs | Proposed (reach-or-diagnose is Decided, D10) |
| fixture definitions and JSON baselines | `schematic-gen` | tests for every item; R7 baseline | Proposed |

## Suggested phases

Each phase leaves the tree green (`just test`, `just lint` in `schematic/`, R11 comparison clean or explained). The baseline is the tree as left by 4a.

1. **Baseline and inventory.** Re-verify counts (Basic and OAuth2 users, dependents, required query parameters, `x-schematic` consumers). Prove regeneration reproduces the committed trees. Inventory dependent call sites that R9 will break. No code change except the log.
2. **Serialization (R7).** Derives, bundle, round trip, re-baseline the 4a fixtures to JSON. Additive.
3. **Define-side additions, additive ones.** Path parameters, `deprecated`, auth override, rate-limit/retry (R1-R4, define side), each with validation and builder methods.
4. **`Basic` shape (R5).** Migrate `legacy_env_mapping_for`, Bitbucket and EMQX; byte compare.
5. **`oneOf` (R6).** Context registration over the descriptor's structural component.
6. **OAuth2 per-grant (R8).** Breaking; `schematic-oauth` and generator files.
7. **Generator consumers.** R1-R4 and R6 outputs with fixture definitions and D10 diagnostics; R12 adoption.
8. **Query-parameter fidelity (R9).** Last, because it changes shipped output and dependents; migrate dependents from the phase 1 inventory.
9. **Regenerate, dependents, docs.** Regenerate `schematic-schema`, OpenAPI and Postman with the generator (never by hand); review every explained diff; dependents; R13; final gates.

## Acceptance criteria and definition of done

- Every requirement R1-R13 has a passing test or a recorded, justified exception in `implementation-log.md`.
- The workspace builds and tests pass on macOS, Linux, native Windows and WSL2 per repo rules. The implementer proves the host it runs on and records which hosts could not be exercised; load the `os` skill before claiming any OS cannot be tested; no CI scope change.
- `just test` and `just lint` pass in `schematic/`; `just test-schema` and `just lint-schema` pass for the generated crate.
- Generated trees are regenerated by `just generate`, never hand-edited; every difference from the baseline is explained in the log (R11).
- The committed definitions changed only as R12/R10 list.
- Every dependent builds and passes its tests; each edited call site is logged.
- Docs, README, skill and command updated; behavior decided but not yet built is marked **planned**; a drift pass covers every symbol whose behavior changed.
- No commits unless the plan says so (then identity and signing rules: `git verify-commit HEAD`, no agent trailers, never `--no-gpg-sign`).
- The agent never moves this spec to `_completed` and never runs `just complete`. Terminal state: "implementation complete, ready for review".

## Pre-authorizations

The author must confirm these before the unattended run (D50 item 8). Anything not listed pauses the run and is logged as an open question.

- Regenerate `schematic-schema`, `schematic/openapi` and `schematic/postman` with the generator.
- Change `OAuth2Config` and edit `schematic-oauth` and the generator files that consume it (R8).
- Edit dependent crates (`sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, the five homelab crates) only as far as the generated required-parameter signatures force (R9, R11).
- Edit the shipped definitions only at the R12 adoption sites.
- Add test fixtures and JSON baselines under `schematic/gen/tests/fixtures/`.
- Add a dependency only if unavoidable (record in `docs/dependencies.md`). Expected: none.
- Not pre-authorized, so log and continue with the default: touching CI configuration; committing; deleting outside listed files; contacting a real provider.

## Dependencies and ordering

- **Needs landed first:** `2026-10-11-schematic-definition-api` (4a): the builders, the context, the descriptor's optional structural component and serde derives, the `Debug` baseline, and the serde derives on `ParamDef`, `QueryParamType`, `ParamStyle`. Earlier specs (1 diagnostics and isolated compile check; 2 runtime types; 3 pagination) are transitive.
- **Blocks:** spec 5 (reads the context and descriptors, may use `oneOf` registration for typed errors); spec 6 (shares `ParamDef`, auth and env types as changed here); spec 7 (reads definitions through the serializable form and takes the accepted-gap candidates).
- **Internal order:** serialization before the JSON re-baseline; query fidelity last.

## Planning-time details assigned to this spec

Each default is adopted by the implementer if the author has not ruled before planning, and is logged.

**PD1: rate-limit and retry vocabulary shape.** The parent gives no shape. Default if not ruled before planning: API-level `RateLimit { limit_header, remaining_header, reset_header, retry_after_header: Option<String>, reset_unit: Seconds|EpochSeconds|HttpDate }` and `RetryGuidance { retry_on: Vec<u16 status or range>, honor_retry_after: bool, max_attempts: u8, backoff: None|Fixed(ms)|Exponential{base_ms,cap_ms} }`, endpoint-level override of either; descriptive only, no runtime behavior. Flag at review 1: consumers are documentation and the OpenAPI extension only, the same shape of problem D49 calls out for `oauth_scopes`.

**PD2: typed path parameter element.** Default if not ruled before planning: reuse `ParamDef` (name, description, `QueryParamType`, `required` forced to `true`, style `Simple`) so WebSocket can share it (D8); validation rejects a path parameter whose type is `Array`, `Json` or an object style.

**PD3: `deprecated` shape.** Default if not ruled before planning: a plain `bool` on `Endpoint` and `ParamDef`; docs get a "Deprecated." line; OpenAPI `deprecated: true`; no `#[deprecated]` attribute in generated Rust (it would fail `-D warnings` linting of the generated crate and warn in dependents); no note or since-version field.

**PD4: per-endpoint auth override shape.** Default if not ruled before planning: `Endpoint::auth: Option<EndpointAuth>` with `EndpointAuth::Public` and `EndpointAuth::Policy(AuthPolicy)`; `None` inherits. No optional-auth variant (recorded under Risks).

**PD5: polymorphic body shape.** Default if not ruled before planning: `ctx.one_of::<RustType>([variants...])` (and `any_of`, `all_of`) where `RustType` is a hand-written `serde` enum or untagged type that already exists and `variants` are descriptors from the same context; the structural `TypeRef` is stored in a `ModelCatalog` attached to the context and exported to OpenAPI and Postman; the Rust generator only references `RustType`.

**PD6: `Basic` shape (and the dual-source remainder).** Default if not ruled before planning: `AuthStrategy::Basic { username_env: EnvList, password_env: EnvList }`; `env_username` is removed; `env_auth` stays for Bearer and ApiKey; `env_mapping` and `auth_policy` remain explicit overrides. Do only what R5 requires; the unification of `auth`/`env_auth` with `auth_policy`/`env_mapping` is an open assignment.

**PD7: serialized bundle.** Default if not ruled before planning: `DefinitionBundle { api: RestApi, schemas: BTreeMap<String, serde_json::Value>, models: Option<ModelCatalog> }` with schemas as the OpenAPI-3.0-settings JSON the registry already produces; `BTreeMap` for deterministic output.

**PD8: query-parameter fidelity rules.** Default if not ruled before planning: supported styles are `Form` (explode true and false), `SpaceDelimited` and `PipeDelimited`; `DeepObject` and `Simple` on a query parameter yield the D10 backstop diagnostic for the Rust output; `Enum(values)` generates `pub enum {EndpointId}{ParamPascalCase}` with a variant per value (sanitized identifiers, collision-checked, wire spelling preserved with `#[serde(rename)]`), `#[non_exhaustive]`; `required: true` puts the parameter in `new()`.

**PD9: OAuth2 per-grant shape.** Default if not ruled before planning: `OAuth2Config` becomes a `#[non_exhaustive]` enum with variants `AuthorizationCodePkce(..)`, `ClientCredentials(..)`, `DeviceCode(..)`, each a struct with only its legal URLs plus shared `default_scopes`, `pkce` where it applies, and `client_auth`; the serde representation is internally tagged by `grant_type` so existing serialized field names are preserved; `OAuth2GrantType` stays as an accessor result.

**PD10: baseline format.** Default if not ruled before planning: re-baseline the 4a `Debug` fixtures to pretty-printed JSON at `schematic/gen/tests/fixtures/definition-baseline/<api>.json` once R7 lands; if the total exceeds 5 MB keep a hash per API and a per-endpoint hash table (log the choice).

**PD11: R9 landing option.** Default if not ruled before planning: land R9 unconditionally; use the option-behind-a-flag fallback of R11 only if the phase 1 inventory shows a large number of broken call sites (more than about 25, a threshold proposed here, not measured).

## Risks and failure modes

- **Query fidelity breaks dependents.** Required parameters change generated `new()` signatures; the OpenAI-imported definitions carry many required flags. Mitigation: inventory first, migrate in the same spec, flag fallback (R11, PD11).
- **Rate-limit vocabulary has no mandated shape or runtime consumer.** Risk of becoming another unused field like `oauth_scopes`. Mitigation: D49 criterion (reader plus test per field), one adopting shipped definition, review 1 item. If review 1 finds the vocabulary unwanted, removing it is cheap because nothing else reads it.
- **Descriptor constraint from 4a may not fit `oneOf`.** Unverified: 4a's structural carrier did not exist at drafting. R6 permits extending, not replacing, the descriptor.
- **OAuth2 break touches `schematic-oauth`.** Its wiremock tests may be sensitive to the serde form; PD9 keeps the tag.
- **Serialization of by-name `Schema` references is not self-contained.** The bundle (PD7) carries the schemas; round-trip equality must be tested on bundles, not bare `RestApi`.
- **Byte-identical claim vs R9.** Query fidelity legitimately changes shipped output; the allow-list must be per-file and explained, not blanket.
- **Basic migration regressions.** Bitbucket and EMQX are the only users; any change in environment mapping would break real credential lookup. Mitigation: byte comparison and a unit test of `basic_user`/`basic_pass`.
- **Spec 4a's outcomes are unverified here.** Builder names, the typestate shape, reserved names and the descriptor component did not exist when this was drafted; the refresh must replace them with 4a's actual surface.
- **Unassigned adjacent gaps (not done here; accepted-gap candidates for spec 7's ledger table, D20).** The optional-auth variant (Ollama ignores API keys but accepts them); the broader legacy/modern auth dual-source cleanup beyond `env_auth[0]`; invented HuggingFace stub shapes; orphaned types.
- **Long unattended run.** Each phase ends only when tests pass; the log records the last completed phase; commit-free.

## Process

Per D50 and D51 (parent), these specs are built for the following process:

1. This spec has a 1:1 plan, implemented end to end, phase by phase, **without a human in the loop**.
2. Then **review 1**: time is set aside for the author to review decisions made during implementation and open design questions.
3. Then the **next** spec and plan are refreshed from the outcomes of review 1 (this spec's review 1 refreshes spec 5 and spec 6 where they touch the changed types).
4. Then fix and review cycles repeat until the reviewer deems the spec production ready.

An agent's terminal state is "implementation complete, ready for review". Only the author moves a spec to `_completed`; an agent never does and never runs `just complete`.

Because no human is available while the plan runs, every planning-time detail has a default (PD1-PD11), and the implementer logs each item **as it occurs** in `implementation-log.md` (format of `worktree/features/2026-10-09-more-pr-support/implementation-log.md`): adopted defaults, design choices this spec does not dictate, deviations, open questions. If a requirement cannot be met as written, the implementer chooses the smallest change that keeps the tree green, logs it, and continues.

## Human review agenda

Record each in `implementation-log.md`, as it occurs, for review 1:

1. **Every default adopted:** PD1-PD11, with the value used and any adjustment.
2. **Rate-limit/retry vocabulary:** final shape, readers, the adopting definition, and a keep/drop recommendation (the unused-field risk).
3. **The adoption list (R12):** which definitions changed and why; confirmation that no other definition source line changed (R10 diff check).
4. **Generated-output differences:** the per-API, per-file explained diffs from R11 with cause.
5. **Dependents:** the phase 1 inventory, each call site edited in sniff, biscuit-speaks, model-citizen, the homelab crates or `schematic-oauth`, and whether the PD11 fallback was used.
6. **The descriptor and `oneOf` bridge:** whether 4a's structural carrier sufficed (R6) and what was extended.
7. **Basic and OAuth2 changes:** final shapes, and the unassigned auth remainder.
8. **Every design choice not dictated by this spec, every deviation, every open question.**
9. **Tests deleted or replaced** and doc, skill and command updates.
10. **Hosts and operating systems actually exercised**, and which of macOS, Linux, native Windows, WSL2 were not.

## Refresh notes

`refresh_required: true`. Before this spec's plan is made, refresh it from the review 1 outcomes of specs 1-3 and, most of all, 4a. Things those outcomes may change:

- **From 4a:** the final builder method names and reserved names (`.path_param`, `.public`, `.auth`, `.deprecated`, `.rate_limit`, `.retry_guidance`); the typestate shape (where new required-or-optional facts attach); the descriptor's structural component and serde derives (R6); the `Debug` baseline fixtures (R7, PD10); the Basic authoring call's signature (R5, R12); the `auth_policy`/`oauth_scopes` outcome; whether `#[non_exhaustive]` stayed (new fields rely on it); the serde derives that now exist (R7); the `EndpointParams` `Default` allow-list (R1 adds a field to it); the true endpoint count and any definition changes 4a made.
- **From spec 1:** diagnostics format and validator API the new builders call; the isolated compile check's form; the list of unsupported-feature diagnostics (R2, R9).
- **From spec 2:** whether the runtime consumes anything resembling rate-limit or retry guidance (R4, PD1); the mock-server harness for R2 and R9 tests.
- **From spec 3:** query-parameter adoption rules (R9 interacts with the pager's adopted parameters; a required pagination parameter must not become a constructor argument the pager sets).
- **Counts:** re-verify Basic users (Bitbucket, EMQX), OAuth2 users (none), dependents and required query parameters.
- **From the author's rulings at review 1:** any ruling on the PD items replaces the defaults; rulings on the unassigned items are added.
