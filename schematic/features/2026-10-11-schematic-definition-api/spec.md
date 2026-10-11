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
    - 2026-10-11-schematic-pagination
refresh_required: true
related: []
---

# Schematic definition API: builders and typed registration

## Problem

An API is described today by a hand-written `fn define_x_api() -> RestApi` that returns one struct literal, and each endpoint is a fixed nine-field `Endpoint { .. }` literal (parent section 2). Three things are wrong, all verified in the parent review (July evidence, re-checked in part on 2026-10-11 by this draft):

1. **Ceremony without safety.** There are about 519 endpoint literals across the 18 REST APIs (OpenAI alone has 265; the parent's July figure of 253 is stale and is not used anywhere in this spec). The parent's July counts of defaulted fields (`oauth_scopes: None`, `headers: vec![]`, `params: None`, `request: None`, about 1,387 `.to_string()` calls) were taken on the smaller set and are re-measured at the start of the run (R18). The links between a definition and the Rust types it names are hand-typed strings: `ApiResponse::json_type("RepositoryInfo")` has no compile-time relationship to `pub struct RepositoryInfo`, and a parallel `SchemaRegistry` (`schematic/definitions/src/registry/mod.rs`) maps other hand-typed name strings to `schemars` schemas. `validate_completeness` compares one set of hand-written strings with another. The crate bends to fit strings (`pub type Commit = CommitInfo;` in `schematic/definitions/src/huggingface/types/stubs.rs` exists so a string resolves).
2. **Dead or doubtful surface.** `Endpoint::oauth_scopes` has no `Some` in any shipped definition; the only non-test reader copies it into the `x-schematic` operation extension (`schematic/define/src/openapi/export/paths.rs:88`). `RestApi::auth_policy` is `None` in every shipped definition, but the generator does read it through `effective_auth_policy()` (`schematic/gen/src/codegen/api_struct/mod.rs:77`, `helpers.rs:7`) and an API-variant path (`api_struct/variant.rs`), so the parent's phrase "zero codegen consumers" is not accurate for `auth_policy` (see Risks). Several definitions also write the same environment variable twice (`openai/mod.rs`: `env_auth` and `env_mapping`).
3. **A rewrite that must happen once.** The parent decided (D5, D42, D50) that the endpoint literals are rewritten in exactly one pass, and that the builder surface must already fit the response vocabulary of a later spec so no definition is rewritten twice. D52 (2026-10-11) split the original definition-API spec in two so that this pass, which is mechanical and large, is not entangled with the vocabulary work: this spec (4a) is the pass and the surface it needs; the vocabulary additions (typed path parameters, auth override, `deprecated`, rate limits, Basic shape, `oneOf` bodies, serialization, OAuth2 grants, query-parameter fidelity) are the next spec, `2026-10-11-schematic-definition-vocabulary` (4b), and are added to the builders this spec creates as new methods and fields, so no definition is rewritten again.

**Not stated in the parent, found while drafting.** The `just import-openai` recipe regenerates `schematic/definitions/src/openai/{mod,types}.rs` through `schematic/gen/src/definition_gen.rs`, which emits `Endpoint { .. }` struct literals as source text (it also emits `oauth_scopes`). If only the hand-written definitions are rewritten, the next OpenAI re-import silently reintroduces literals. The emitter is therefore in scope here (R22).

## Goal and non-goals

**Goal.** After this spec, an API author (human or agent) writes a definition only with builders and a typed definition context. Every shipped definition is rewritten to that form in one pass, the old pagination types are gone, and generated output is unchanged except for differences explained in the implementation log.

**Non-goals (out of scope, with owner).**

- The vocabulary additions: typed path parameters, per-endpoint auth override, `deprecated`, rate-limit and retry vocabulary, retiring `env_auth[0]`, polymorphic `oneOf` bodies, serializable definitions (the `Serialize`/`Deserialize` derives on `RestApi`, `Endpoint`, `EndpointParams`; the self-contained bundle), OAuth2 per-grant payloads, and query-parameter `required`/`style`/`explode`/enum fidelity: spec 4b. This spec leaves a seam for them (R2, the builders' additive shape) and builds none of them.
- Response vocabulary v2 (success sets, error sets and status mapping, streaming, content types, selectors) and GraphQL: spec 5. This spec only shapes the builder surface for it (R13).
- The pagination model, pagers, `collect_all`, `with_session`: spec 3 builds them. This spec moves each exception onto its endpoint and removes the old pagination types (R19, R20).
- The runtime package, envelope, credentials, redaction, redirect-following, cross-origin runtime: spec 2. This spec adds only the definition-side forms that name them (R14, R15).
- OpenAPI bridge fixes, `x-schematic` round trip, Postman example bodies, AsyncAPI, feature flags, the single generated API list, required constant headers as OpenAPI parameters, module-path resolver unification, generated-docs honesty pass, README doctests, moving vendor pagination presets, and the accepted-gap ledger (D20): spec 7.
- WebSocket definitions: spec 6. WebSocket reuses the shared types this spec touches (`ParamDef`, auth types, `EnvMapping`); this spec must not break the five WebSocket definitions' build, and does not rewrite them.
- The wider legacy-versus-modern auth dual-source cleanup, the optional-auth variant (the "Ollama ignores API keys but accepts them" case), invented HuggingFace stub shapes, and orphaned types: not done here beyond the minimum below. They are **accepted-gap candidates** for spec 7's ledger table (D20) and are listed in Risks and the Human review agenda so each is recorded rather than assumed fixed.
- Diagnostics for duplicate endpoint ids, duplicate `(method, path)`, and per-output validation: spec 1. The builders call the validation spec 1 delivers; this spec adds no second validator.

## Background: decisions this spec implements

Provenance: parent spec `schematic/reviews/2026-07-31-macro-review/spec.md` (section 15 holds the decisions, section 13a "Plan structure (D50)" the mapping, section 14 the criteria). Everything the implementer needs is restated here; the parent is evidence, not a prerequisite read.

- **D1 (scope).** Nothing is deferred. Every item the parent assigned to the definition-API work is built, in this spec or in 4b.
- **D2 (typed registration, explicit context).** `schematic-define` provides a definition context passed through each definition function in `schematic-definitions`. One call registers a type's schema and returns a descriptor carrying a validated generated Rust path. Containers such as `Vec<T>` reuse their element registrations. Rejected: descriptors carrying schema-producing callbacks, a deriving proc-macro, deferral.
- **D44 (naming rules, closes D2's details).** Defaults come from the type itself: the schema name comes from the type, the Rust path from a module the API states once (for example `types_in(...)`). Containers (`Vec`, `Option`, `HashMap`) reuse their element's registration. Free-form `serde_json::Value` registers no component. Aliases are stated explicitly (for example `ctx.alias::<Commit, CommitInfo>()`). Name collisions are reported when the definition is built. Left to planning: nested container and `HashMap` key registration, the format of collision reports (PD1, PD2).
- **D15 (no public `Default` on `Endpoint` and `RestApi`).** `Endpoint`, `RestApi` and their builders get constructors and builders and no public `Default`. Required facts go in constructors so an unusable definition cannot be written. Generated request types in `schematic-schema` keep their `Default`. D15 covers `Endpoint` and `RestApi`; it does not by itself cover related types such as `EndpointParams`, so `EndpointParams`'s `Default` is a separate, explicit allow-list decision (R8, PD6). Rejected: a public `Default`; `Default` behind a test feature.
- **D5, D42 (one pass).** The landing that introduces builders is the only pass over the endpoint definitions. Spec 3 puts the pagination exceptions in a table on each API definition; the builders move each exception onto its endpoint and the table is removed.
- **D43 (list recognition).** A GET endpoint returning `Vec<T>` is a list with no table entry; a POST returning an array is a list only when authored as one.
- **D30 (per-endpoint override).** The API-level `cross_origin` field and the redirect runtime land in spec 2. The per-endpoint `cross_origin` override lands here: an endpoint-level policy replaces the API-level one for that endpoint only. The policy has five values (D28): `same_origin` (default), `allow_origins([...])` exact scheme+host+port, `allow_without_credentials`, `allow_any` (explicit, clearly named opt-in; credentials sent), `deny`. Callers may tighten the policy at runtime, never loosen it.
- **D40 (secret marking).** Secrets are marked automatically from auth strategies, by an explicit `.secret()` marker on any header or parameter in the definition, and by a built-in list of well-known names. The redaction set is the union. Spec 2 owns the semantics, the API-level fields and the redaction tests; this spec supplies the builder sugar only.
- **D48 (envelope details, definition-side part).** The definition names the provider's request-id header (for example `request_id_header("x-github-request-id")`). The built-in list and the redaction are runtime work (spec 2).
- **D49 item 4 (`oauth_scopes` and `auth_policy`).** Whether `Endpoint::oauth_scopes` and `RestApi::auth_policy` are wired in or removed is decided while the definitions are rewritten. Criterion: whichever is chosen, the field is either read by generator or exporter code and covered by a test, or it no longer exists; a field that is neither fails review.
- **D50 (plan structure).** The builders' surface (for example `.returns(...)` and `.errors(...)`) is shaped here for spec 5's response vocabulary, so the definitions are not rewritten twice. Each spec leaves the whole repo green on macOS, Linux, native Windows and WSL2; every dependent crate builds and passes its tests; an agent's terminal state is "implementation complete, ready for review". Pre-authorizations are items the author must confirm before the unattended run (D50 item 8).
- **D52 (split of the definition-API spec, approved 2026-10-11).** Eight specs: 1 generation-integrity, 2 runtime-envelope, 3 pagination, 4a definition-api (this), 4b definition-vocabulary, 5 response-model-graphql, 6 websocket, 7 fidelity-polish. The parent text for D52 may still be arriving; this spec restates what it needs.
- **D10 (backstop rule).** A declared feature must either reach the selected output or produce a diagnostic identifying the API, endpoint, feature and unsupported target; one output's gap never blocks another output.
- **D8 (shared building blocks).** WebSocket reuses the REST parameter, auth-policy, env-mapping and header types. Changes made here to `ParamDef`, `AuthStrategy`, `EnvMapping` must not close that door (do not bake REST-only assumptions into them).
- **Parent recommendation 14 (pagination shrink).** Resolved here as **remove**: spec 3 keeps `PaginationStyle`, `PaginationResponse`, `EndpointParams::with_pagination` and the `pagination`/`response_pagination` fields inert (its R8, and states that removal belongs here); this spec removes them and converts the 35 declared endpoints (R20).
- **D33/D36/D38 (context only).** Selectors and error mapping are spec 5's; the placeholders in R13 must not make them harder.

The runtime trait that generated clients implement is named `EndpointSpec` (spec 2's default), so this spec's struct `Endpoint` is unambiguous: `Endpoint` is the definition, `EndpointSpec` is the runtime trait.

## Requirements

Counts below were verified in the parent review (some in July) or at drafting and must be re-verified at the start of the run (R18 states the check). "Package" names the crate: `schematic-define` (`schematic/define`), `schematic-definitions` (`schematic/definitions`), `schematic-gen` (`schematic/gen`), `schematic-schema` (`schematic/schema`, generated).

### Definition context and typed registration (D2, D44)

**R1. A definition context exists and returns validated descriptors.** `schematic-define` exposes a context type (working name `DefinitionContext`) with a registration call that, for a type `T: JsonSchema` (plus whatever bound the Rust-path derivation needs), records `T`'s schema under a schema name derived from `T` and returns a descriptor holding the schema name and the generated Rust path (module stated once by `types_in(...)` plus the type identifier). A descriptor can only be obtained from a context: no public constructor takes a bare string.
Validated by: L1 unit tests in `schematic-define` (registration returns the expected name and path); compile-fail test (trybuild or `compile_fail` doctest) showing a descriptor cannot be built from a string; the Rust path of every registered type compiles in the isolated generated crate (the spec 1 compile check, run from `schematic-gen` L1 against the rewritten definitions).

**R2. Descriptors can carry optional structural type information and derive serde from the start (author-approved constraint).** The descriptor type has an optional structural component (working name `structure: Option<TypeRef>`, where `TypeRef` is the existing structural type in `schematic-define`'s model; the exact carrier is Proposed) that is `None` for every registration this spec performs, and the descriptor derives `Serialize` and `Deserialize`. Nothing in this spec reads the structural component. It exists so that 4b can implement `oneOf`/`anyOf`/`allOf` bodies and serializable definitions without changing the descriptor type or any rewritten definition. The structural component can be set only by the context (same rule as R1).
Validated by: L1 test in `schematic-define` (a descriptor round-trips through `serde_json` with `structure: None`, and with a `Some` value built through a crate-internal test helper); the compile-fail test of R1 also covers the component; an L1 source scan asserting no code outside the context sets it.

**R3. Containers reuse the element's registration.** `Vec<T>`, `Option<T>`, `HashMap<K, V>` (and nesting such as `Option<Vec<T>>`) register the element type once; the descriptor's path renders the container around the element path (`Vec<RepositoryInfo>`). `HashMap` key types follow PD1.
Validated by: L1 unit tests in `schematic-define` (registry holds one entry for the element; descriptor path text; nested containers; a non-supported key type is a build error).

**R4. Free-form JSON registers no component.** `serde_json::Value` (and type aliases that resolve to it) yields a descriptor with path `serde_json::Value` and adds nothing to the registry.
Validated by: L1 unit test in `schematic-define`; the exported OpenAPI for an endpoint returning it carries an inline free-form schema, as today (L1 in `schematic-gen`).

**R5. Aliases are explicit and collisions are reported when the definition is built.** `ctx.alias::<A, B>()` (name illustrative) declares that the Rust type `A` is an alias of the registered type `B`; a descriptor for `A` then carries `A`'s Rust identifier as its path and `B`'s schema name. Without the alias call, registering both is a collision, never a silent merge. Two distinct Rust types that map to the same schema name are reported by the build step (not at first use, not by rustc), with all collisions listed in one error, in the format of PD2.
Validated by: L1 unit tests in `schematic-define` (alias resolves; missing alias reports a collision; two same-named types in different modules); the HuggingFace `Commit`/`CommitInfo` and Ollama `PushModelProgress`/`PullModelProgress` aliases in `schematic-definitions` are migrated using it (L1 in `schematic-definitions`).

**R6. The context replaces the hand-typed registry strings and the string-vs-string check.** After the rewrite, no definition in `schematic-definitions` passes a type name as a string literal to register or reference a schema, `SchemaRegistry::register::<T>("Name")` is no longer how registries are filled, and `validate_completeness` no longer compares two hand-written string sets. The registries returned by `get_registry`, `get_registries_for_module` and each module's `openapi_registry()` are produced from the context, with the same entries in the same insertion order as before.
Validated by: source-scan L1 test in `schematic-definitions` (no `register::<..>("` call and no `json_type("` string literal remain); the equivalence tests of R10 and R24 prove registry content and order are unchanged; `openapi_strict_completeness` in `schematic-gen` still passes.

**R7. An endpoint cannot reference an unregistered type.** Because descriptors come only from the context (R1), an endpoint whose request or response type was never registered cannot be constructed; for the paths that bypass registration (imported definitions, R22) the build step reports an unregistered reference as a validation error naming API, endpoint and type.
Validated by: compile-fail test (descriptor from another context or from a string) and an L1 negative test for the validation error, in `schematic-define`.

### Builders (D15)

**R8. Endpoint and API builders exist; `Endpoint` and `RestApi` have no public `Default`; `EndpointParams`'s `Default` is explicitly allow-listed.** `Endpoint` gets method constructors (`Endpoint::get(id, path)` and the other verbs, names per PD8); `RestApi` gets a builder whose constructor takes the required facts (PD7). Neither `Endpoint`, `RestApi` nor their builders implement `Default` publicly (this is D15, which covers exactly those). `EndpointParams` derives `Default` today (`schematic/define/src/params/mod.rs`) and D15 does not cover it: it **keeps** its `Default` as a recorded allow-list entry because an endpoint with no parameters is a normal, valid endpoint (PD6). The audit also lists every other public definition type that derives `Default` today; each is either given a constructor and loses its public `Default`, or is added to the allow-list with the reason an empty value is valid. Generated request types in `schematic-schema` keep their `Default` (unchanged).
Validated by: compile-fail test in `schematic-define` (`Endpoint::default()` and `RestApi::default()` do not compile); an L1 audit test listing the public definition types and asserting the `Default` set equals the recorded allow-list (which names `EndpointParams`).

**R9. A builder cannot produce an endpoint missing a required fact.** Required facts (PD7) are enforced by the type system (typestate), not by a runtime check: `.build()` (or the conversion into `Endpoint`) is unavailable until the description and the success shape are set.
Validated by: compile-fail tests in `schematic-define` (missing description; missing success shape; missing API description or auth choice).

**R10. A built value equals the literal it replaces.** `Endpoint` and `RestApi` already derive `PartialEq`/`Eq`. For each of the 18 APIs, the builder-built `RestApi` equals the literal-built `RestApi` captured before the rewrite, modulo the explicit additions listed in the implementation log (new fields at their defaults, for example the pagination fields spec 3 added). This is the primary migration check; R24 backs it up.
Validated by: L1 equivalence test in `schematic-definitions` or `schematic-gen` (R24 states how the baseline is captured).

**R11. The `RestApi` builder takes each fact once.** One authoring call declares authentication and sets the fields that must agree (`auth`, and the environment-variable names for it as the current model stores them), so a definition cannot contradict itself (the `openai/mod.rs` double write of `OPENAI_API_KEY` becomes impossible to write). The builder leaves `env_mapping` and `auth_policy` for the explicit forms that need them. The shape of the stored auth types is unchanged (4b changes `Basic`). Output must be unchanged.
Validated by: L1 test in `schematic-define` (one auth call yields the same `RestApi` fields the literal wrote); R10 per API.

**R12. `Endpoint` and `RestApi` are `#[non_exhaustive]` (PROPOSED, beyond D15).** D15 requires no public `Default` and required facts in constructors; it does not require `#[non_exhaustive]`. This spec proposes it so that later field additions (4b's vocabulary, spec 5's stored response type) break no definition. Because it is beyond the decision, it is flagged for review 1 (agenda item). After this spec, code outside `schematic-define` cannot build either with a struct literal; test and fixture code in `schematic-gen` and elsewhere is migrated to builders or a test helper (existing test literal sites live in `gen/src/output/mod.rs`, `gen/src/codegen/api_struct/tests.rs`, `gen/src/codegen/module_docs.rs`, `gen/src/validation.rs`, `gen/src/definition_gen.rs` tests, `gen/tests/l1/http_client.rs` and `define/src/openapi/export/mod.rs`). Fallback if the churn is disproportionate: enforce "no literals in `schematic-definitions`" by source scan only, and log the deviation.
Validated by: compile-fail test in `schematic-definitions` (a struct literal of `Endpoint` does not compile there); the whole workspace builds.

**R13. The builder surface is shaped for response vocabulary v2, and the definitions never name the stored response type.** Exactly these builder methods are placeholders for spec 5, with the stated acceptance for now:

| Method | Package | Accepts now | Spec 5 will |
|---|---|---|---|
| `.returns(Success)` | `schematic-define` | one `Success` built by `Success::json(desc)`, `Success::text()`, `Success::binary()`, `Success::empty()`; a second `.returns(..)` call is a build error naming "several success shapes arrive with response model v2" | add `Success::sse(..)`, `Success::ndjson(..)`, a content-type setter, and allow several `.returns(..)` calls as a success set |
| `.errors(ErrorSet)` | `schematic-define` | only `ErrorSet::none()` (the D38 "no declared errors" form); any other constructor does not exist yet | add status and range entries, per-endpoint add/override |
| `RestApi` builder `.default_errors(ErrorSet)` | `schematic-define` | only `ErrorSet::none()` | add the API default list (D38) |
| `.returns_json::<T>()`, `.returns_text()`, `.returns_binary()`, `.returns_empty()` | `schematic-define` | shorthands for the four `Success` constructors | unchanged (kept as sugar) |

`Success` and `ErrorSet` are `#[non_exhaustive]`. The stored form (`Endpoint::response: ApiResponse`) is unchanged by this spec; the builder lowers `Success` into it. No method for selectors, stream end conditions, content types or GraphQL is added here (their names are reserved, not implemented). After the rewrite, no file under `schematic/definitions/src` names `ApiResponse`, `ApiRequest` constructors, `Schema` or an `Endpoint { .. }` literal, so spec 5 can change the stored type without touching a definition.
Validated by: unit tests in `schematic-define` for each accepted form and each rejected form (second `.returns`, non-`none` error set cannot be constructed); source-scan L1 test in `schematic-definitions` (no `ApiResponse`, `ApiRequest::`, `Schema::`, `Endpoint {` outside `types` modules and tests that deliberately exercise the stored form).

### Definition-side forms for runtime behavior

**R14. Per-endpoint `cross_origin` override (D30).** The endpoint builder accepts a cross-origin policy (the type spec 2 created for the API level) that replaces the API-level policy for that endpoint only; `allow_origins` entries are validated as exact origins by the validator spec 2 uses (PD5). The generated client applies the endpoint policy to that endpoint's redirects and followed links. Callers can still only tighten at runtime.
Validated by: mock-server L1 test in `schematic-gen`/`schematic-runtime` using two local origins (the D28 matrix, spec 2's harness): an endpoint overriding `same_origin` with `allow_origins([B])` reaches B while a sibling endpoint refuses B and B records zero requests for it; validation tests for malformed origins in `schematic-define`. The compile-fail test spec 2 wrote ("no endpoint-level override") is retired by this requirement (R27).

**R15. Builder sugar for `request_id_header` and the per-parameter `.secret()` marker only.** The `RestApi` builder gets `.request_id_header(name)` over the definition-side field spec 2 added (PD3), and the parameter builder gets `.secret()` setting `ParamDef::secret` (PD4). Header name lists (`.secret_header`) are included only if spec 2 defined the corresponding field; otherwise they are not added (spec 2 owns semantics, API-level fields and redaction tests). This spec adds no redaction logic. Its test checks **emission only**: the builder call produces the field value the literal form would have, and the generated client source carries the marker in the form spec 2's runtime consumes.
Validated by: L1 test in `schematic-define` (builder call sets the expected field; header name validated as a legal HTTP header name); generated-source assertion in `schematic-gen` (marker present in emitted client for a fixture definition). The redaction behavior test remains spec 2's.

**R16. The `auth_policy` and `oauth_scopes` decision is made and recorded (D49, review 1).** After the rewrite, apply PD17: `Endpoint::oauth_scopes` is removed (no shipped definition sets it; verified count at drafting: zero `Some`), together with its copy in the `x-schematic` operation extension and in `definition_gen.rs`; `RestApi::auth_policy` is **kept** and given a builder form because the generator reads it through `effective_auth_policy()`. This deviates from reading "zero users" as "remove both" and is flagged for review 1. The decision, its evidence (grep output) and the dependents it touched go in the implementation log.
Validated by: the D49 criterion expressed as a test: a source scan asserts that every remaining field of `Endpoint` and `RestApi` has at least one non-test reader, or the field is absent; L1 test covering the `Some(AuthPolicy)` path through the builder in `schematic-gen`.

**R17. `ParamDef`, `QueryParamType` and `ParamStyle` derive `Serialize` and `Deserialize`.** They derive no serde traits today (verified), and spec 6 asserts they exist. The derives are additive and keep field names stable. This is the only serde work on definition types in this spec; serializable `RestApi`/`Endpoint`/`EndpointParams` and the bundle are 4b.
Validated by: L1 round-trip test in `schematic-define` for each of the three types.

### Migration of the shipped definitions

**R18. Every endpoint definition of the 18 REST APIs is rewritten to builders in one pass.** Count the endpoint literals at the start of the run (about 519, OpenAI 265; spec 3 or earlier work may have changed it), record the number per API, and rewrite all of them. Also re-measure the defaulted-field counts quoted in the Problem, the Basic and OAuth2 users, and the dependents. Definition functions take the context (R1). The WebSocket definitions are not rewritten. Hand-maintained endpoint tables in module docs and per-endpoint unit tests that assert "the literal says what it says" are kept only where they still test behavior; a test that only restates a builder call is deleted or replaced by the equivalence tests (log each).
Validated by: R10/R24 per API; source-scan test (R13); count test asserting each rewritten API exposes the recorded endpoint count and id list.

**R19. Pagination exceptions move onto endpoints and the exceptions table is removed.** For each of the 18 APIs, every entry spec 3 placed in the API-level exceptions table (D42) becomes an endpoint-level builder call; the API default strategy stays on the API; the table type and its validation ("an exception id that matches no endpoint") are deleted. Generated Rust and OpenAPI are unchanged except for intended differences recorded in the implementation log (D42's criterion).
Validated by: L1 tests in `schematic-definitions`/`schematic-gen`: the per-endpoint pagination resolved for every list endpoint (spec 3's resolution function) equals the pre-rewrite resolution captured as a fixture; byte comparison of generated output (R24); a source scan asserts the table type no longer exists.

**R20. The old pagination types are removed and the 35 declared endpoints are converted (resolves parent recommendation 14).** `PaginationStyle`, `PaginationResponse`, `EndpointParams::with_pagination`, and the `EndpointParams.pagination` and `response_pagination` fields are removed from `schematic-define`. The 35 endpoints that declare page-number pagination today (Gitea 8, GitHub 9, GitLab 9, Bitbucket 9; re-count at the start) are converted to the pagination model spec 3 built, expressed through the endpoint builder; the vendor constructors (`PaginationStyle::github()` and kin) are not kept (their move to presets is spec 7's concern and spec 3's model already carries the strategies). Every dependent that referenced the removed names is migrated and logged. Generated output for those endpoints is unchanged except as spec 3's pager emission already changed it.
Validated by: compile check that the removed names no longer exist (source scan for the identifiers across the workspace and dependents); the R19 resolution fixture covers the 35; dependents build (R25); the generated request structs for the 35 endpoints keep their `page`/`per_page` setters or the adoption rule's equivalent (spec 3's R7), byte-compared (R24).

**R21. `Vec<T>` automatic list recognition survives the rewrite.** GET endpoints returning a `Vec<T>` descriptor are still recognized as lists (D43); the descriptor's rendered type text must be what spec 3's recognition reads.
Validated by: part of R19's resolution fixture (covers GET `Vec<T>`, a POST returning an array that is not a list, and the converted declared endpoints).

**R22. The OpenAPI import emitter writes builder code.** `schematic/gen/src/definition_gen.rs` (used by `just import-openai` and the `import` subcommand) emits builder calls and context registration instead of `Endpoint { .. }` literals and string types, and the OpenAI definitions are converted. **Reproduce first:** run the unchanged emitter and require that it reproduces the committed `schematic/definitions/src/openai/{mod,types}.rs` byte for byte (proof there are no hand edits). If it does, regenerate with the new emitter; if it does not, convert by a mechanical transform and log the hand edits found. The importer's in-memory `RestApi` construction (`define/src/openapi/import/builder.rs`) may keep using crate-internal construction but must not need a public `Default`.
Validated by: L1 golden test in `schematic-gen` (emitter output for a small fixture OpenAPI document compiles and, when evaluated, equals the importer's `RestApi`); generated-code compile test of the emitted source; the reproduce-first result and a `just import-openai` dry comparison recorded in the log.

**R23. `Schema::module_path` disposition on the REST path.** `Schema::module_path` is wired or removed, and the choice is recorded. Default (PD21): remove it from the REST definition path if the post-rewrite scan finds no reader (the context's `types_in(...)` now states the module once); wire it only if a reader is found. Either way, the D49-style criterion holds: the field is read and tested, or it no longer exists. WebSocket uses of the shared type are not broken.
Validated by: the source-scan test of R16 extended to `Schema`; L1 test for the chosen outcome.

**R24. Migration safety net: generated output is byte-identical or explained, and definitions are snapshot-compared.** *Level 1.* Before any definition changes (after spec 3's last commit state), the committed `schematic/schema`, `schematic/openapi` and `schematic/postman` trees are proven reproducible (regenerate into a temporary directory; no diff) and are the baseline. After each API's rewrite batch and again at the end, regeneration into a temporary directory is compared with the baseline. Any difference is either an intended change (R20) listed per API and per file in `implementation-log.md` with the cause, or a defect to fix. The comparison is also an L1 test that runs on a normal `just test` (the existing `gen/tests/l1/artifact_drift.rs` covers OpenAPI and Postman against committed artifacts; extend or add a sibling for the Rust client tree), with an allow-list file of explained differences that is empty at the end unless the removal of the old pagination types legitimately changes shipped output. *Level 2.* Because level 1 sees only what the generators consume, a second net compares the definitions themselves. Since serializable definitions are 4b's, the snapshot here is the stable `Debug` rendering (with map-like fields sorted) of each `RestApi` and its registry entry list, captured to fixtures (`schematic/gen/tests/fixtures/definition-baseline/<api>.txt`, a name proposal) from the unchanged literal-based definitions before any definition changes. After the rewrite, an L1 test renders the builder-built definitions and compares, ignoring only keys listed as additions and the explicitly explained changes. 4b may re-baseline these fixtures into JSON once serialization exists. If the fixtures total more than 5 MB, keep a per-endpoint hash table instead (PD20). The fixtures are deleted by the author after review 1 if they prove too heavy (log the size).
Validated by: L1 tests in `schematic-gen` (Rust, OpenAPI and Postman regeneration versus committed trees; definition snapshot); a deliberately mutated definition (a test-only copy with one description changed) must fail the snapshot test; a recipe or script the plan names (default a `just generate`-style dry run into a temp directory) for per-batch checks.

**R25. Every dependent builds and passes.** Within the area: `schematic-define`, `schematic-definitions`, `schematic-gen`, `schematic-oauth`, and the generated `schematic-schema`. Outside the area, the dependents of the generated crate recorded in the parent (path dependencies of `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, and five homelab crates: `homelab/lib`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`) build and pass their tests. The removal in R20 can break call sites that used the removed names; those are fixed in this spec (pre-authorized below) and logged.
Validated by: `just test` and `just lint` in `schematic/`, then `just test {pkg}` for each dependent package (plan phase gate); `just test-schema` and `just lint-schema`.

**R26. Documentation and skills reflect the new authoring path.** The `schematic-define` README and doc examples (compiling doctests where they exist), `schematic/docs/` topic pages describing definitions, `.claude/skills/schematic-define/SKILL.md`, and the `/create-schematic-api` command (`.claude/commands/create-schematic-api.md`, which teaches authors how to write a definition) describe builders and the context, not literals and strings. Comments and rustdoc of every symbol whose behavior changed get a drift pass (repo rule: when comments and code disagree, the code is right). Docs never name a feature or fix directory.
Validated by: doctests (`just test` runs them); a grep over docs, README, the skill and the command for `Endpoint {`, `json_type("` and `oauth_scopes` returning only deliberate historical mentions; reviewer check at review 1.

**R27. Earlier specs' positive tests that pin the pre-builder surface are retired or updated (frozen-test list).** Specs 2 and 3 wrote tests that assert the pre-builder surface; they are expected to be green when those specs land and would fail here by design. This spec retires or updates exactly these, and logs each (the refresh confirms the final names):
1. Spec 2's compile-fail test in `schematic-define` asserting "no endpoint-level override" for `cross_origin` (its R20): retired and replaced by R14's positive test. The "no GraphQL primitive" half of the same test stays; spec 5 retires it.
2. Spec 3's compile test that builds an `Endpoint` literal with exactly the pre-change nine fields (its R3): retired (literals no longer compile under R12); replaced by R9/R10.
3. Spec 3's compile test that builds an `EndpointParams` literal with exactly the pre-change fields (its R3): updated to the post-removal field set (R20) and to the `EndpointParams::new()`/`Default` allow-list entry (R8).
4. Spec 3's diff check that no endpoint-literal lines changed (its R3 log check): retired; superseded by R24.
5. Spec 3's tests asserting the old pagination types exist and are inert (its R8): retired (R20).
6. Spec 3's exceptions-table construction and validation tests (its R5, `RestApi` table type): retired with the table (R19); the per-endpoint resolution coverage test of its R27 is updated to read endpoint-level pagination and must still pass with the same expected resolutions.
Validated by: the list above appears in the implementation log with the replacement test for each; no retired test is deleted without a named replacement (or a stated reason none is needed).

## Design

"Decided" means the parent decided it (cite). Everything else is **Proposed**: the implementer adopts it unless a Planning-time default (below) or a better fit found in the code says otherwise, and logs the choice.

| Symbol | Package | What it does | Status |
|---|---|---|---|
| definition context (`DefinitionContext`) | `schematic-define` | owns the registry for one module group; registers types, resolves aliases, reports collisions at build; replaces the registry-name strings | Decided that a context exists and is passed through definition functions (D2); name and exact signature Proposed |
| `ctx.types_in("path")` | `schematic-define` | states the module that holds the API's types once | Decided as the mechanism (D44, "for example `types_in(...)`"); exact name Proposed |
| registration call (working name `ctx.register::<T>()`) -> descriptor | `schematic-define` | records schema and returns a validated name+path descriptor; containers reuse element registrations | Decided behavior (D2, D44); names Proposed |
| `ctx.alias::<A, B>()` | `schematic-define` | explicit alias declaration | Decided (D44, "for example") |
| descriptor type (`TypeDesc`) with optional structure, serde derives | `schematic-define` | schema name + Rust path + optional structural type; constructible only by the context | Proposed (D2 says "a descriptor"); the optional structure and serde derives are an author-approved constraint (2026-10-11), unused in this spec |
| definition-time error (`DefinitionError`) | `schematic-define` | collisions, unregistered refs, placeholder rejections | Proposed |
| `Endpoint::get/post/put/patch/delete(id, path)` -> `EndpointBuilder` | `schematic-define` | typestate builder; `.describes(..)`, `.returns(..)`, `.returns_json::<T>()`, `.request_json::<T>()`, `.request_form(..)`, `.header(..)`, `.query(..)`, `.cross_origin(..)`, `.pagination(..)`, `.errors(..)`, parameter `.secret()` | Decided that builders exist without public `Default` on `Endpoint`/`RestApi` (D15); names and typestate Proposed |
| `RestApi` builder (`RestApi::builder(ctx, name, base_url)`) | `schematic-define` | required facts in the constructor, `.describes(..)`, one auth call, `.auth_policy(..)`, `.header(..)`, `.endpoints(..)`, `.module_path(..)`, `.cross_origin(..)`, `.request_id_header(..)`, `.default_errors(..)` | Decided (D15, D48's `request_id_header` example, D30); names Proposed |
| `Success`, `ErrorSet` (`#[non_exhaustive]`) | `schematic-define` | placeholders of R13 | Proposed; the table in R13 is the contract spec 5 inherits |
| `#[non_exhaustive]` on `Endpoint` and `RestApi` | `schematic-define` | forbids literals outside the crate | **PROPOSED, beyond D15**; flagged for review 1 |
| `EndpointParams` `Default` | `schematic-define` | empty parameter set is valid | Allow-listed exception, not covered by D15 (PD6) |
| `RestApi::auth_policy` with builder form; `Endpoint::oauth_scopes` removed | `schematic-define` | PD17 | Proposed; flagged for review 1 |
| serde derives on `ParamDef`, `QueryParamType`, `ParamStyle` | `schematic-define` | additive derives spec 6 relies on | Proposed (R17) |
| endpoint-level pagination builder methods | `schematic-define` | take over the table entries and the 35 declared endpoints | Proposed; names follow spec 3's model |
| removal of `PaginationStyle`, `PaginationResponse`, `with_pagination`, `pagination`/`response_pagination` | `schematic-define` | resolves recommendation 14 as remove | Proposed (R20) |
| `definition_gen.rs` emitter | `schematic-gen` | emits builder and context code for imports | Proposed (R22) |
| registry assembly (`get_registry`, `get_registries_for_module`, `openapi_registry()`) | `schematic-definitions` | built from the context instead of hand-typed names | Proposed |
| per-module `define_x_api(ctx)` functions | `schematic-definitions` | all 18 rewritten | Decided that definitions are rewritten once through builders and typed registration (D5, D42) |
| equivalence tests and baseline fixtures | `schematic-gen` / `schematic-definitions` | R24 | Proposed |

Sketch of the intended authoring shape (names illustrative, Proposed):

```rust
pub fn define_github_api(ctx: &mut DefinitionContext) -> RestApi {
    ctx.types_in("schematic_definitions::github");   // module stated once
    ctx.alias::<Commit, CommitInfo>();                // only where an alias exists

    RestApi::builder(ctx, "GitHub", "https://api.github.com")
        .describes("GitHub REST API v2022-11-28 ...")
        .bearer_env(["GITHUB_TOKEN", "GH_TOKEN"])     // one call sets auth + env
        .request_id_header("x-github-request-id")
        .endpoints([
            Endpoint::get("GetRepository", "/repos/{owner}/{repo}")
                .describes("Get repository metadata including default branch")
                .returns_json::<RepositoryInfo>(),
            Endpoint::get("ListTags", "/repos/{owner}/{repo}/tags")
                .describes("List repository tags")
                .returns_json::<Vec<TagInfo>>(),
        ])
        .build()                                       // collisions reported here
}
```

Everything 4b adds (`.path_param(..)`, `.public()`, `.deprecated()`, `.rate_limit(..)`, `oneOf` registration) is a new method on these builders; the sketch above does not change when it lands.

## Suggested phases

Each phase leaves the tree green (`just test`, `just lint` in `schematic/`, R24 comparison clean or explained). The plan may refine. The baseline is the tree as left by spec 3, not the tree this draft read.

1. **Baseline and safety net (no behavior change).** Re-verify counts (endpoint literals per API, `Some` uses of `oauth_scopes`/`auth_policy`, the 35 declared endpoints, dependents). Regenerate Rust, OpenAPI and Postman into a temporary directory and prove no diff against the committed trees. Capture the `Debug` definition baseline (R24). Add the Rust-tree comparison test. Add serde derives on `ParamDef`, `QueryParamType`, `ParamStyle` (R17, additive). Run the reproduce-first check of R22 now (read-only) and log the result.
2. **Definition context.** Context, descriptors with the optional structural component and serde (R2), containers, `serde_json::Value`, aliases, collisions, `DefinitionError` (R1-R5, R7). Registries can be produced from it alongside the old `SchemaRegistry` (additive).
3. **Builders and placeholders.** `Endpoint` and `RestApi` builders with typestate, `Success`/`ErrorSet`, the `Default` audit and allow-list, `#[non_exhaustive]`, migrate in-repo test literals, retire/update the frozen tests of R27 (R8-R13, R27). Builders initially produce today's fields only.
4. **Definition-side forms.** Per-endpoint `cross_origin`, `request_id_header` and `.secret()` sugar, `auth_policy` builder form (R14-R16).
5. **Emitter.** `definition_gen.rs` emits builders; OpenAI conversion (R22).
6. **The rewrite, API by API.** Smallest API first; for each API: rewrite to builders and context, move its pagination exceptions onto endpoints (R19), convert declared pagination (R20, for Gitea, GitHub, GitLab, Bitbucket), run R10/R24. Then remove the exceptions table, the old pagination types, the hand-typed registry strings, and the string `validate_completeness` (R6, R19, R20). Apply PD17 and PD21 (R16, R23) at the end of this phase.
7. **Regenerate and dependents.** Regenerate `schematic-schema`, OpenAPI and Postman with the generator (never by hand); review every explained diff; build and test dependents (R25); `just lint-schema`.
8. **Docs, skill, drift.** R26; delete stale tests and tables; final gates.

## Acceptance criteria and definition of done

- Every requirement R1-R27 has a passing test or a recorded, justified exception in `implementation-log.md`.
- The workspace builds and tests pass on macOS, Linux, native Windows and WSL2 per repo rules. The implementer proves the host it runs on and records which hosts could not be exercised; load the `os` skill before claiming any OS cannot be tested, and do not alter CI scope (no CI cells added here).
- `just test` and `just lint` pass in `schematic/`; `just lint-schema` and `just test-schema` pass for the generated crate. If the plan adds or touches L2 tests, `just test-l2` passes and takes no window focus.
- `schematic-schema`, `schematic/openapi/` and `schematic/postman/` are regenerated by the generator (`just generate`), never hand-edited. Every difference from the baseline is explained in the log.
- Every dependent crate listed in R25 builds and passes its tests.
- The names `PaginationStyle`, `PaginationResponse`, `with_pagination`, the exceptions-table type and `oauth_scopes` no longer exist in the workspace.
- README, `schematic/docs/` topic pages, `.claude/skills/schematic-define/SKILL.md`, `.claude/commands/create-schematic-api.md` and the `docs/dependencies.md` pages (if a crate dependency changed) are updated for changed behavior; docs describe current behavior, never name a feature or fix directory (repo rule); behavior decided but not yet built, if any, is marked **planned**.
- A comments and docs drift pass covers every symbol whose behavior changed.
- The agent makes no commits unless the plan says otherwise; if the plan says to commit, commits follow the repo's identity and signing rules (verify `git verify-commit HEAD`; no agent attribution trailers; never `--no-gpg-sign`).
- The agent never moves this spec to `_completed` and never runs `just complete`. Terminal state: "implementation complete, ready for review".

## Pre-authorizations

The author must confirm these before the unattended run (D50 item 8); they would otherwise pause it. Anything not listed pauses the run and is logged as an open question instead.

- Regenerate `schematic-schema`, `schematic/openapi` and `schematic/postman` with the generator (large tree change; expected).
- Rewrite all endpoint definitions across the 18 REST APIs and the API-level definitions (D50's pre-authorization for the builder spec).
- Remove `PaginationStyle`, `PaginationResponse`, `with_pagination` and the `pagination`/`response_pagination` fields, and convert the 35 declared endpoints (R20).
- Edit dependent crates only as far as a changed generated or define API forces: `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib` and the five homelab crates (R25).
- Regenerate `schematic/definitions/src/openai/{mod,types}.rs` through the import tooling, under the proof condition of R22 (reproduce first); this reads the vendored OpenAPI file referenced in the `import-openai` recipe and makes no network request.
- Delete obsolete tests (the R27 list and tests that only restate literals), the exceptions-table type, the hand-typed registry code, and `oauth_scopes` per PD17 (log each).
- Add test fixtures and baseline snapshot files under `schematic/gen/tests/fixtures/`.
- Add a dependency only if a requirement cannot be met without it (record in `docs/dependencies.md` per repo rule). Expected: none.
- Not pre-authorized, so the run must log and continue with the default rather than act: touching CI configuration; committing; deleting anything outside the files listed above; contacting a real provider.

## Dependencies and ordering

- **Needs landed first:** `2026-10-11-schematic-pagination` (spec 3) because this spec moves its exceptions table (R19), removes the types it leaves inert (R20) and keeps its list recognition working (R21). Spec 3 depends on `2026-10-11-schematic-runtime-envelope` (spec 2), which supplies the `cross_origin` type (R14), the definition-side request-id field and redaction interface (R15) and the managed-client mock harness; spec 2 depends on spec 1 (generation integrity), whose per-output validation, diagnostics format and isolated compile check this spec relies on. `depends_on` lists only spec 3 because the chain is transitive.
- **Blocks:** spec 4b (`2026-10-11-schematic-definition-vocabulary`), which adds vocabulary on these builders and uses the descriptor's structural component (R2); spec 5 builds on the placeholders of R13 and on the context for GraphQL typed variables and data (D34); spec 6 reuses `ParamDef`, auth and env types and the serde derives of R17 (D8); spec 7 reads definitions through the shared lists and takes the accepted-gap candidates.
- **Internal order:** the safety net (phase 1) precedes every definition change; the builders and removals precede the rewrite; PD17 and PD21 follow the rewrite.

## Planning-time details assigned to this spec

Each default is adopted by the implementer if the author has not ruled before planning, and is logged.

**PD1 (D44): nested container and `HashMap` key registration.** Default if not ruled before planning: containers recurse to the element or value type; `HashMap<K, V>` registers `V` only; `K` must be `String`, a string-like newtype that serializes as a string, or an integer type, and any other key type is a build error naming API, endpoint and type; `Option` is transparent.

**PD2 (D44): format of collision reports.** Default if not ruled before planning: a structured `DefinitionError::Collision { schema_name, first, second }` (both fully qualified Rust type names from `std::any::type_name`, used only as diagnostic text) collected for the whole API, rendered as one line per collision: `API <name>: schema name "<n>" is claimed by <type A> and <type B>; register an alias or rename one`. Human-readable only; adopt spec 1's diagnostics format if it defined one.

**PD3 (D48): request-id marker form.** Default if not ruled before planning: `RestApi` builder `.request_id_header("x-github-request-id")`, one header per API, stored lowercase, validated as a legal HTTP header name; no per-endpoint override. Adopt the definition-side field spec 2 added and put the builder method over it.

**PD4 (D40): `.secret()` forms.** Default if not ruled before planning: for parameters, `ParamDef::secret: bool` set by a `.secret()` call on the parameter builder; header name lists only if spec 2 defined them (see R15). Adopt spec 2's interface; if it has none for a form, do not add that form and note the gap in the log.

**PD5: per-endpoint `cross_origin` override scope.** Default if not ruled before planning: the endpoint value replaces the API value wholesale for that endpoint, any of the five values may be used (including looser than the API, which is the point of D30's example of a GitHub upload host), `allow_any` requires the same explicit named constructor spec 2 defined, `allow_origins` entries are validated as exact origins by spec 2's validator, and the builder offers no way to inherit "partially". The tighten-only comparison for callers is spec 2's.

**PD6: related types that derive `Default` (R8).** Default if not ruled before planning: `EndpointParams` **keeps** its `Default` as the one explicitly allow-listed exception (an endpoint with no parameters is valid; D15 covers `Endpoint` and `RestApi` only), with `EndpointParams::new()` as the preferred spelling in docs. Every other definition type that derives `Default` today is audited in phase 3 and loses it unless an empty value is meaningful; log the list and the allow-list.

**PD7: the required facts.** Default if not ruled before planning: `Endpoint` requires id, method, path (constructor), description and one success shape (typestate-enforced). `RestApi` requires name and base URL (constructor), description, and an explicit authentication choice (`no_auth()` is a choice) before `.build()`; headers, module path, request suffix, version, env mapping, endpoints may be empty or absent.

**PD8: builder method names.** Default if not ruled before planning: the names in the Design table. D15's `Endpoint::get("GetRepository", "/repos/{owner}/{repo}").describes(..).returns_json::<RepositoryInfo>()` is the anchor. Names that conflict with spec 3's pagination method names yield to spec 3, and 4b's names (`.path_param`, `.public`, `.auth`, `.deprecated`, `.rate_limit`) are reserved so this spec does not use them for anything else.

**PD17 (D49): `oauth_scopes` and `auth_policy`.** Default if not ruled before planning: remove `Endpoint::oauth_scopes` (and its copies in the `x-schematic` operation extension and `definition_gen.rs`) when the post-rewrite scan finds no shipped definition setting it; keep `RestApi::auth_policy` and expose it through the builder because `effective_auth_policy()` is read by the generator and an `UpdateStrategy` variant path, with `gen/tests/l1/http_client.rs` covering `Some`. Re-run the scan at the end of the rewrite and log the evidence. Flagged for review 1.

**PD19: how the rewrite is batched.** Default if not ruled before planning: one batch per API, smallest first, with OpenAI last (the largest and the generated one); R24 runs after every batch; a batch that fails is fixed before the next starts.

**PD20: where the baseline lives and how large it may be.** Default if not ruled before planning: `schematic/gen/tests/fixtures/definition-baseline/<api>.txt`; if the total exceeds 5 MB, keep only the hash of each API's rendering and a per-endpoint hash table instead (log the choice).

**PD21: `Schema::module_path` on the REST path (R23).** Default if not ruled before planning: remove it from the REST path when the post-rewrite scan finds no reader, since `types_in(...)` states the module once; if a reader is found, wire it from the context and cover it with a test. Record the outcome.

(PD9 to PD16 and PD18 of the earlier numbering moved to spec 4b with their vocabulary; PD number gaps here are intentional so cross-references in logs stay stable.)

## Risks and failure modes

- **Hidden behavior change in the rewrite.** A mistyped path or description in about 519 conversions is silent. Mitigation: R10, R24 and per-API batching (PD19). A conversion that cannot be verified by equality must fail the batch, not be waved through.
- **Typed registration changes names.** The Rust identifier derived from `T` may differ from the string the old definition used (serde `rename` on a container changes the `schemars` name; an alias; a re-exported type). Unverified at drafting for all registered types. Mitigation: the equivalence tests compare descriptor-derived names with the old strings; mismatches become explicit aliases or PD2 collisions, each logged.
- **Registered set must not grow.** Registering nested types automatically would add components to OpenAPI output. Default: register exactly what the old registries registered (R6); changing that is an explained diff and a review item.
- **`#[non_exhaustive]` is beyond D15 and causes churn.** Many in-repo test literals (R12) must move to builders or a helper. Fallback stated in R12.
- **Removing the old pagination types can break dependents and generated setters.** The 35 declared endpoints feed `page`/`per_page` setters that dependents may call. Mitigation: R20's byte comparison and dependent builds; spec 3's adoption rule (R7) keeps the setters.
- **Descriptor constraint may be wrong.** The optional structural component is designed before 4b exists. If 4b needs a different carrier, the descriptor type changes but no definition does (definitions never name the descriptor's fields). 4b's refresh checks this.
- **Parent inaccuracy about `auth_policy`.** The parent says `auth_policy` has zero codegen consumers; the generator reads `effective_auth_policy()`. Removing the field would remove the way a definition states "several explicit auth methods". Surfaced in PD17 and the agenda.
- **Debug-rendering snapshot is brittle.** Level-2 snapshot uses `Debug` because serialization is 4b's; ordering of map-like fields must be normalized or false diffs appear. Mitigation: sort in the renderer; 4b re-baselines to JSON.
- **Spec 3's output is unverified here.** The exceptions table's form, the resolution function, and how the 35 page-number declarations are expressed did not exist when this was drafted. R19-R21 and R27 are written against D42/D43 and spec 3's draft; the refresh must replace them with spec 3's actual names.
- **Spec 2's definition-side interface is unverified.** `CrossOrigin`, redaction sets, the request-id field and the harness may exist in some other form (R14, R15, PD3, PD4).
- **Placeholders may not fit spec 5.** `Success`/`ErrorSet` are guesses at spec 5's needs (R13). Mitigation: `#[non_exhaustive]`, a single stored lowering point, and the refresh after review 1 of this spec before spec 5 is planned.
- **Import regeneration may lose hand edits.** R22's reproduce-first proof guards against it.
- **Unassigned adjacent gaps (not done here; accepted-gap candidates for spec 7's ledger table, D20).** (a) The optional-auth variant (the Ollama "ignores API keys but accepts them" case, parent section 4.5); (b) the legacy-versus-modern auth dual-source cleanup (`auth`/`env_auth`/`env_username` versus `auth_policy`/`env_mapping`), of which only the `env_auth[0]` retirement is done, in 4b; (c) invented HuggingFace stub shapes (parent section 4.5); (d) orphaned types (parent section 4.6). Listed so nobody assumes they were fixed; each needs a close-or-accept decision by spec 7.
- **Long unattended run.** The rewrite is the largest mechanical change in the whole plan; a context-window or time failure mid-batch must leave the tree green: commit-free, so each batch ends only when tests pass, and the log records the last completed API.

## Process

Per D50 and D51 (parent), these specs are built for the following process, which this spec follows:

1. This spec has a 1:1 plan. The plan is implemented end to end, phase by phase, **without a human in the loop**.
2. Then **review 1**: time is deliberately set aside for the author to review the decisions made during implementation and any open design questions.
3. Then the **next** spec and plan are refreshed from the outcomes of review 1. This spec's review 1 refreshes spec 4b first (descriptor constraint, builder surface), then spec 5 (R13).
4. Then fix and review cycles repeat until the reviewer deems the spec production ready.

An agent's terminal state is "implementation complete, ready for review". Only the author moves a spec to `_completed`; an agent never does and never runs `just complete`.

Because no human is available while the plan runs: this spec is self-contained; every planning-time detail assigned here has a default, flagged "Default if not ruled before planning", which the implementer adopts and logs; and the "Human review agenda" below lists what the implementer must record in `implementation-log.md` for review 1. **The implementer logs each item as it occurs**, not in a summary at the end: when a default is adopted, when a design choice is made that this spec does not dictate, when the implementation deviates from this spec, and when an open question is found. If a requirement cannot be met as written, the implementer chooses the smallest change that keeps the tree green, logs it, and continues; it does not stop for a question.

The implementation log follows the format used by `worktree/features/2026-10-09-more-pr-support/implementation-log.md` (frontmatter listing `spec`, `plan`, `implemented_by`, source files, docs and skills files touched per phase, packages touched).

## Human review agenda

Record each of these in `implementation-log.md`, as it occurs, for review 1:

1. **Every default adopted:** PD1-PD8, PD17, PD19-PD21, with the value actually used and any adjustment made.
2. **The `auth_policy` decision:** PD17 outcome with grep evidence, the parent inaccuracy, and whether `Endpoint::oauth_scopes` removal touched any dependent.
3. **`#[non_exhaustive]` on `Endpoint` and `RestApi`:** marked PROPOSED beyond D15; the churn it caused, whether the fallback was used, and a keep/drop recommendation.
4. **`Schema::module_path`:** the PD21 outcome (removed or wired) and evidence.
5. **The `EndpointParams` `Default` allow-list:** the final allow-list, and the other types that lost or kept `Default`.
6. **The migration evidence:** the endpoint count found (expected about 519, OpenAI 265) per API; the per-API list of explained diffs from R24 with cause; the size of the baseline fixtures (PD20); the OpenAI conversion route and the reproduce-first result (R22).
7. **The pagination removal:** the 35 declared endpoints as found, how each was converted, every dependent call site changed, and any generated-output change (R20).
8. **The descriptor constraint:** the final shape of the optional structural component and serde derives (R2), for 4b's refresh.
9. **The builder surface contract with spec 5 and spec 4b:** the final text of the R13 table as implemented, the reserved names, and anything spec 5 or 4b will need that the placeholders do not give.
10. **Every design choice not dictated by this spec:** final names of the context, descriptor, builders, `Success`/`ErrorSet`, error types; typestate structure; how `Vec<T>` descriptors render for spec 3's recognition; whether registries kept insertion order by construction or by sorting; how test literals were migrated.
11. **Every deviation from this spec** and every open question found, including anything in spec 3's or spec 2's actual interfaces that differs from what R14, R15, R19-R21 assumed.
12. **Frozen tests:** the R27 list as actually retired or updated, with the replacement for each.
13. **Accepted-gap candidates for spec 7's ledger table (D20):** the four unassigned items of Risks, with any new evidence.
14. **Dependents touched:** each call site edited in sniff, biscuit-speaks, model-citizen or the homelab crates, with the reason (R20, R25).
15. **Tests deleted or replaced**, and doc, skill and command updates made.
16. **Hosts and operating systems actually exercised**, and which of macOS, Linux, native Windows, WSL2 were not.

## Refresh notes

`refresh_required: true`. Before this spec's plan is made, refresh it from the review 1 outcomes of specs 1, 2 and 3 (spec 3 most of all). The 2026-10-11 split (D52) moved the vocabulary work to spec 4b; this spec was rewritten accordingly, and the PD numbers of the moved items were left to 4b. Things the earlier specs' outcomes may change:

- **From spec 1:** the diagnostics format and validator API the builders call (PD2, R8-R9); whether the isolated compile check is a cargo-level or generator-level test (R1); whether duplicate id and `(method, path)` validation exists yet; the state of `gen/schematic/`.
- **From spec 2:** the `CrossOrigin` type and validator (R14, PD5); whether a definition-side request-id field and redaction interface exist and what they are called (R15, PD3, PD4); the managed-client mock harness available for R14; the final name of the runtime trait (assumed `EndpointSpec`) and package.
- **From spec 3:** the pagination model's names, the exceptions-table form, the resolution function usable by R19's fixture, the list-recognition rule's exact input (R21), how the 35 page-number declarations and the OpenAI/EMQX adjustments were expressed (R20), the exact text and names of spec 3's tests listed in R27, and the true endpoint count (R18).
- **Counts:** re-verify the ~519 endpoints (OpenAI 265), 18 REST APIs, the 35 declared endpoints, the `oauth_scopes`/`auth_policy` users, and the dependent list.
- **Outcomes of this spec that change spec 4b and spec 5 (recorded for their refresh):** the descriptor's final structural carrier (R2), the builder surface and reserved names (R13, PD8), the `Debug` baseline fixtures (R24, which 4b may re-baseline), the `auth_policy`/`oauth_scopes` outcome (PD17), whether `#[non_exhaustive]` stayed, and the serde derives that now exist on `ParamDef` and its style types (R17).
- **From the author's rulings at review 1:** any ruling on the planning details assigned here replaces the defaults; rulings on the accepted-gap candidates are added.
