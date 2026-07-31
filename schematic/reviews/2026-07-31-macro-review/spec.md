---
created: "2026-07-31T12:45:56-07:00"
agent: claude
---

# Schematic Definition-and-Generation Review

**Date:** 2026-07-31
**Scope:** How APIs are defined with `schematic-define` primitives, and how `schematic-gen` converts those definitions into Rust client code (`schematic-schema`), OpenAPI documents (`schematic/openapi/`), and Postman collections (`schematic/postman/`).
**Method:** Four parallel full-source reviews (definition primitives; the 18 REST + 5 WebSocket definitions in `schematic-definitions`; Rust codegen and generated output; OpenAPI/Postman export-import), each with independent file:line verification, cross-checked against each other and spot-verified in the main review. Regeneration was verified byte-identical against the committed output; the generated crate `cargo check`s clean. All paths below are relative to `schematic/`.

---

## 1. Executive Summary

The pipeline's core competencies are real: deterministic byte-identical regeneration, `quote!`/`syn`/`prettyplease` codegen with atomic ownership-gated writes, a layered auth model (`AuthMethod` / `EnvAuthStrategy` / `AuthPolicy`) that is genuinely better thought out than most, correct `#[non_exhaustive]` discipline, env fallback chains as data, and export-side `$ref`-closure validation that provably holds across all 15 artifacts. For a JSON-in/JSON-out REST API with bearer auth, the system works end to end and the generated client is pleasant to consume.

The honest overall verdict, though, is that **the definition vocabulary is significantly richer than what the generators consume, and the generators are more trustworthy than the definitions deserve.** Three structural problems dominate:

1. **Ceremony without safety.** Defining an endpoint costs a fixed ~11-line struct literal (253 of them, no builder, no `Default`, no macro), yet the thing that ceremony should buy — a verified link between the definition and the Rust types it names — does not exist. Type wiring is by hand-typed string, validated string-against-string. The crate currently survives on discipline: a systematic sweep found zero dangling references, but also found the type system being bent to fit strings (`pub type Commit = CommitInfo;` exists solely so a string literal resolves) and the only test that would catch a bad name (`generated_code_compiles`) is `#[ignore]`d.

2. **Silent fidelity loss, concentrated in exactly the features the vocabulary advertises.** Form/multipart request bodies generate clients that POST **no body at all** (6 endpoints today). `Endpoint.params` never reaches OpenAPI output (0 query parameters across all 15 exported specs, while the same data *does* reach Postman). `ApiKeyParam` auth generates clients that never send the key. Pagination's 12 fields of structure are read as one boolean. Every one of these fails silently — compiles, exports, looks done.

3. **Two of the four output targets are materially weaker than they appear.** The OpenAPI documents are browsable but not generatable (~30% of schema properties export as untyped `{}`, no error responses, required headers only in `x-schematic`), and the documented `x-schematic` round-trip is fiction — the importer never reads the extension. WebSocket definitions produce no external artifact at all, and the generated WS clients are transport-rich but type-empty and never authenticate from env.

None of this requires a ground-up redesign. The single highest-leverage change is ergonomic (builders/macro + typed schema references, §8.1–8.2); the single most urgent changes are converting silent drops into hard validation errors (§8.3) and fixing the destructive single-API regeneration (§7.4).

---

## 2. What Defining an API Looks Like Today

An API is a hand-written `fn define_x_api() -> RestApi` returning one struct literal: 14 top-level fields, then a `Vec<Endpoint>` of 9-field literals. Neither type has a constructor, builder, or `Default` (`define/src/types.rs:120-217`, `types.rs:417-461`). The best case is 11 lines carrying 5 facts (`definitions/src/github/mod.rs:160-170`):

```rust
Endpoint {
    id: "GetRepository".to_string(),
    method: RestMethod::Get,
    path: "/repos/{owner}/{repo}".to_string(),
    description: "Get repository metadata including default branch".to_string(),
    request: None,
    response: ApiResponse::json_type("RepositoryInfo"),
    headers: vec![],
    params: None,
    oauth_scopes: None,
},
```

The worst case is 55 lines for one endpoint, most of it hand-typed enum member strings (`definitions/src/gitlab/endpoints/projects.rs:64-118`). Measured across `definitions/src`: 253 endpoint literals, median 11 lines, and the lines-per-endpoint ratio is a flat 11.0–13.8 across every module — the signature of fixed struct-literal cost, not of varying API complexity. 27,638 lines of definitions yield 43,325 lines of generated client — a thin ~1.6× for a code generator.

Response and request types are wired **by name**: `json_type("RepositoryInfo")` has no compile-time relationship to `pub struct RepositoryInfo`, and `Vec<T>` responses are the literal string `"Vec<RepositoryInfo>"` (`define/src/response.rs:125`). A parallel `SchemaRegistry` maps hand-typed name strings to `schemars` schemas (`.register::<RepositoryInfo>("RepositoryInfo")`), and `validate_completeness` checks endpoint strings against registry strings — string vs. string, both written by the same hand (`definitions/src/registry/mod.rs:238-267`). The upstream test says it plainly: *"content does not matter, only the name does"* (`gen/tests/openapi_strict_completeness.rs:122-124`).

The intended authoring path is agent-driven (`/create-schematic-api`), which raises rather than lowers the bar for ergonomics: an LLM author is exactly the writer most likely to produce a typo-consistent pair of wrong strings that passes every check.

---

## 3. What Works Well

- **Determinism and pipeline hygiene.** Full regeneration is byte-identical to the committed tree. `BTreeMap` ordering, `prettyplease` formatting, atomic temp+rename writes, and ownership-gated cleanup that will never delete a file it can't prove it wrote (`gen/src/output/mod.rs:304-339`, `output/write.rs:42-68`).
- **The auth model's layering.** The `AuthMethod` (caller-injectable) / `EnvAuthStrategy` (env-sourced) / `AuthPolicy` split (`define/src/auth.rs:191-309`) is a real modeling insight; OAuth2 is correctly explicit-only with no env fallback. `EnvList` models "first set env var wins" as data, and empty values correctly count as unset (`define/src/headers/env.rs:162-171`).
- **Request-body vocabulary.** `ApiRequest` covers JSON, multipart, urlencoded, text, binary with correct content-type semantics, and `FormField` is the one place with a real builder — including multi-file `min`/`max` and per-field MIME `accept` (`define/src/request.rs:66-295`). (The tragedy is what the Rust generator does with it — §6.1.)
- **Generated REST client shape.** Request structs with `new()` for required params, `with_*` chaining, `From` conversions, a `ResponseKind` runtime guard whose error names the correct method to call instead (`gen/src/codegen/client/methods.rs:148-171`), `thiserror` error types, env fallback consulted at request time. Comfortable to consume for JSON APIs.
- **`{+param}` reserved expansion** handled correctly on all three export paths and in codegen, with tests asserting the marker never leaks into serialized output.
- **Export-side `$ref` closure validation** (`define/src/openapi/export/validate.rs`) — export fails on dangling refs, and all 15 committed artifacts verify at 0 dangling.
- **Import-side name sanitization** is the best-tested code in the area: keyword/digit/empty handling plus genuine deconfliction when two wire names collapse to one Rust name (`define/src/openapi/import/naming.rs:95-170`).
- **Postman collection craftsmanship** in the parts that exist: auth-implied variables are declared so Postman prompts, mixed-auth grouped collections disambiguate only genuinely colliding ids, endpoint-wins header merge is correct (`gen/src/export/postman/`).
- **WS transport runtime.** Real `tokio-tungstenite` supervisor loop, watch-channel state, capped jittered backoff, oneshot correlation map, and even a server side for `Both`-role endpoints — more ambitious than the REST path at the transport layer.
- **`#[non_exhaustive]` + doctest discipline** in `schematic-define`: nearly every public type carries a compiled example, which is exactly why the module docs stayed accurate while the README rotted (§5.6).

---

## 4. What Is Awkward to Define

### 4.1 Fixed ceremony, absorbed rather than abstracted

Per-line counts across `definitions/src`: `oauth_scopes: None` ×253 (with **zero** `Some` anywhere — a feature no definition uses taxes every endpoint), `headers: vec![]` ×268 (~4 non-empty), `params: None` ×201, `request: None` ×193, `.to_string()` ×1,387. Authors absorbed the repetition instead of abstracting it: the only real de-duplication helper in the crate is `bitbucket_pagination()` (`definitions/src/bitbucket/mod.rs:102-104`), whose exact wrapped expression is inlined verbatim 15 more times in gitea/gitlab/github. There is no `macro_rules!` anywhere in `definitions/src`. Four incompatible file-organization conventions coexist for "chunk the endpoint list."

### 4.2 Dual sources of truth in the core types

`RestApi` carries both the legacy fields (`auth`, `env_auth`, `env_username`) and their successors (`auth_policy`, `env_mapping`), reconciled at runtime (`define/src/types.rs:289-307`). Real definitions set **both** — `definitions/src/openai/mod.rs:72,113-116` writes `OPENAI_API_KEY` twice, and the `env_auth` copy is silently ignored. Basic auth's password lives positionally in `env_auth[0]`, a contract that exists only in prose (`define/src/auth.rs:116-117`).

### 4.3 Everything is written three times

The struct literal, a hand-maintained Markdown endpoint table in the module doc, and a unit test asserting the literal says what it says (e.g. `definitions/src/emqx/mod.rs:37-50` + `:387-677`). The registry adds four more hand-maintained parallel lists of the same 18 API names across `definitions/src/lib.rs`, `registry/lookup.rs`, and `gen/src/pipeline.rs` — **seven total, already drifted**: `gen/tests/openapi_strict_completeness.rs:176-192` lists 16 of 18, so both ArtificialAnalysis APIs are silently skipped by that gate today. Adding one API requires ~13 mandatory edits across two crates.

### 4.4 Vendor knowledge leaked into the neutral layer

`PaginationStyle::github()/gitlab()/bitbucket()/gitea()` live in `schematic-define` (`define/src/pagination.rs:72-128`) — and `gitlab()` is literally `Self::github()`. A new page-number API must either call a competitor's constructor or edit the primitive crate. The WS generator hard-codes `UnfoldedCircleEventHub` as the host struct name for every API and an Unfolded-Circle auth envelope as the generic success response (`gen/src/ws_codegen/host.rs:66`, `shared.rs:436-444`).

### 4.5 The model's gaps get patched in the wrong place

- Query params smuggled into path strings — `"…/trees/{sha}?recursive=true"` (`definitions/src/gitea/mod.rs:202`) — works only by accident of a `path.contains('?')` check in the generator, while GitHub solved the identical problem properly with `with_query_param`. Both APIs carry tests policing the two incompatible encodings.
- Path templates deliberately made wrong relative to the real API (`{git_ref}` for `{ref}`) to dodge Rust keyword collisions (`definitions/src/gitea/mod.rs:386-387`).
- Missing "optional auth" variant expressed as a comment: `auth: AuthStrategy::None, // Ollama ignores API keys but accepts them` (`definitions/src/ollama/mod.rs:304`).
- Enum-valued params degraded to prose: allowed values listed in the description string (`definitions/src/bitbucket/mod.rs:204-209`).
- HuggingFace ships invented placeholder shapes as the published contract for 8 endpoints — two of which shadow correct, fully-written types sitting unreferenced in the same module (`RepoUrl` vs `CreateRepoResponse`, `UserInfo` vs `WhoAmIResponse`) — and the refactor commit `b668b3564` that moved them into `types/stubs.rs` deleted every `TODO` marker that used to flag them. A clean `grep TODO` over this crate is not evidence of a clean crate.

### 4.6 Orphaned effort

74 unreferenced types in huggingface (entire Inference-API and webhooks surfaces with no endpoints), 15 in ollama (all ten streaming response types — written, then stranded because the model has no slot for them), 14 in anthropic. Nothing detects an orphan.

---

## 5. API Features Not Supported (additive fixes possible)

1. **Streaming / SSE / NDJSON.** No `ApiResponse::Stream`. Four APIs invented four different wrong answers: Ollama and LM Studio annotate `ApiResponse::Binary` with `// SSE streaming` comments; Anthropic's `CreateMessage` declares a plain JSON response that would fail to deserialize the moment `stream: true` is set; ElevenLabs' `StreamSpeech` is byte-identical to its non-streaming sibling and the generated method fully buffers (`schema/src/elevenlabs/client.rs:382-387`). `Binary` is also type-erasing in the *non*-streaming direction: Ollama's `stream:false` JSON responses can't be typed either.
2. **Error response bodies.** `ApiResponse` is a single value, not a status→schema map; the exporter structurally cannot emit any status but 200/204 (`define/src/openapi/export/responses.rs:11-42`). The definitions know: `artificial_analysis/types.rs:210-213` documents `RateLimitError` as *"intentionally not registered … It exists to document the wire shape callers should expect when a 429 is returned."* A correct, tested type with no slot to live in.
3. **Per-status success shapes and content negotiation.** One shape per endpoint, period. `ApiResponse::Text`/`Binary` are unit variants while their `ApiRequest` twins carry `content_type` in the same crate (`define/src/request.rs:353-365` vs `response.rs:53-60`) — a demonstrable oversight, not a choice. Export therefore misreports every ElevenLabs audio endpoint as `application/octet-stream`, and import collapses `audio/mpeg` into the same variant, so round-trips rewrite content types.
4. **Typed path parameters.** `EndpointParams` has `query`/`header`/`cookie` but no `path`; `{owner}` exists only as a substring recovered by string-scraping (`gen/src/parser.rs:29`). Untyped, undescribed, unvalidated — nothing checks declared params against the template, or that two endpoints don't share `(method, path)`.
5. **Per-endpoint auth override, deprecation, rate limits, retry policy.** No way to mark one endpoint public on an authenticated API; no `deprecated` field anywhere (export hardcodes `deprecated: None`); zero rate-limit/retry vocabulary despite `PaginationResponse::LinkHeader` proving response-header semantics are modelable.
6. **Feature-usable but dead surface.** `AuthStrategy::ApiKeyParam` (query/cookie keys) exists but: legacy env mapping silently drops it (`define/src/types.rs:334`), `EnvMapping` has no field that can hold it, and no codegen ever injects the key (§6.1). `Endpoint::oauth_scopes` and `RestApi::auth_policy` have zero users and zero codegen consumers. 253 lines of `oauth_scopes: None` buy nothing.

## 5b. Not Supportable Without Redesign

1. **Polymorphic (`oneOf`) bodies from hand-written definitions.** `TypeRef` models `OneOf`/`AnyOf`/`AllOf`, but `ModelCatalog` is unreachable from `RestApi` — it exists only as a sibling output of the importers. A hand-authored definition cannot attach structural models; it can only name Rust types. Bridging `Schema` (a name) to `TypeRef` (a structure) is the redesign.
2. **Status-discriminated responses + typed errors together** mean changing the type of `Endpoint::response`, which breaks the vendor-extension format, both exporters, the importer, and all 253 endpoint literals. Doable, but it is v2 of the vocabulary.
3. **Serializing a hand-authored `RestApi`.** `RestApi`/`Endpoint`/`EndpointParams`/`ParamDef` are not `Serialize` (while `WebSocketApi` is — with no stated principle for the split), and the by-name `Schema` contract means a serialized definition still isn't self-contained.
4. **WebSocket as a peer of REST.** The WS model forked the vocabulary (`ConnectionParam`/`ParamType` vs `ParamDef`/`QueryParamType` — split over a name clash, acknowledged in a doc-comment apology at `define/src/params/param_def.rs:47-48`), lacks `AuthPolicy`/`EnvMapping`/headers entirely, and the generator collapses auth to three WS-specific cases. Unifying means one `ApiDefinition` with a transport discriminant — a rewrite of both trees.
5. **OAuth2 grants beyond the three enumerated.** `OAuth2Config` is a flat struct where every illegal grant/URL combination is representable; per-grant payloads are the fix and a breaking change.

---

## 6. Fidelity: Definition → Generated Rust

**Preserved faithfully:** auth strategy/policy as reified constructor literals (including full OAuth2 config), env-mapping fallback chains actually consulted at request time, API- and endpoint-level headers with correct endpoint-wins merge, percent-encoded path params with `{+param}` opt-out, query params as `Option<T>` + `with_*` builders, response *kind* discriminants with a runtime guard, response *types* parsed via `syn` (so `Vec<T>` works), descriptions into docs.

**Lost or degraded, ranked by damage:**

1. **Form/multipart/urlencoded/text/binary request bodies vanish silently.** `generate_body_field` emits nothing for non-JSON variants — the comment says "The generated code will handle these differently" (`gen/src/codegen/request_structs/body.rs:24-27`) but `into_parts` emits `body: None` for everything non-JSON (`into_parts.rs:20-28`). ElevenLabs' `AddVoiceSample` — a file-upload endpoint with a `FormField` definition — generates a request struct with no audio field that POSTs an empty body (`schema/src/elevenlabs/requests.rs:676-705`). Six endpoints affected today. No error, no diagnostic. **This is the single worst fidelity failure in the pipeline.**
2. **`ApiKeyParam` emits clients that never authenticate.** Header resolution is header-only; `headers_satisfy_fallback` returns `false` for it, so such an API would be permanently un-authable with a misleading "Authentication required" error (`gen/src/codegen/client/helpers.rs:128-217`).
3. **Query-param `required`, `ParamStyle`, `explode`, and `Enum` are all discarded.** Everything becomes `Option<String>`-ish: required params silently optional (runtime 400 instead of compile error), arrays always `form/explode=true`, GitLab's 11-value status enum collapses to `String` (`gen/src/codegen/request_structs/shared.rs:50-176`). `EndpointParams.header`/`.cookie` have zero readers.
4. **Pagination becomes an empty marker trait.** All 12 fields of `PaginationStyle` reduce to `impl Paginated for X {}` where `Paginated` has no methods — and its doc describes a `fetch_all_pages` helper that does not exist (`gen/src/codegen/traits.rs:86-102`, `schema/src/shared.rs:376`). `response_pagination` has zero readers anywhere.
5. **The declared response type is discarded at the call site.** `request<T>` is fully generic; `client.request::<WrongType>(ListModelsRequest::default())` compiles. `EndpointSpec::Response` types only the mutator hooks.
6. **Generated documentation lies.** 478 doc code fences, all ` ```text `, zero compiled. The flagship quick-start in three places calls `client.list_models()` — a method that does not exist (convenience methods are only generated for *non-JSON* endpoints; `gen/src/output/assemble/lib_rs.rs:246-254` vs `codegen/client/helpers.rs:239`). The same block emits an `AuthStrategy::ApiKey` literal missing its required `value_prefix` field. 283 stray `///;` lines. Combined modules (emqx) lose nearly all docs.
7. **No feature flags.** Every consumer compiles all 18 REST clients + 5 WS modules + reqwest + tokio + tungstenite (`gen/src/cargo_gen.rs:14-41`). Four crates depend on this.
8. **`Schema::module_path` is dead on the REST path** — documented as used (`define/src/schema.rs:98-112`), never read; and a non-identifier `type_name` on a *request* (`ApiRequest::json_type("Vec<Foo>")`) panics the generator via `format_ident!` while the response path handles it via `syn`. `validation.rs` never checks `schema.type_name`.

**Failure-mode summary:** identifier validation for API/endpoint/param names is loud and good; type-name errors are loud-but-late (rustc errors in a workspace-excluded crate whose compile test is `#[ignore]`d); body drops, param drops, and auth drops are fully silent.

---

## 7. Fidelity: Definition → OpenAPI and Postman

### 7.1 OpenAPI export

Component schemas are **real JSON Schema**, not stubs — the strongest part of the export. But:

- **`Endpoint.params` is dropped entirely** — `map_operation` hardcodes `parameters: vec![]` (`define/src/openapi/export/paths.rs:83`). Verified across all 15 artifacts: **0 query parameters anywhere**, while the Postman exporter reads the same data and emits them. 52 endpoints with params, 27 with pagination, invisible to every OpenAPI consumer. A GitHub client generated from `openapi/github.json` cannot paginate.
- **~30% of schema properties export as untyped `{}`.** The schemars→openapiv3 bridge (`definitions/src/registry/conversion.rs:86-204`) handles no `enum`, no `oneOf/anyOf/allOf`, no `additionalProperties`, and — critically — no type arrays, so every `Option<T>` (`{"type": ["string","null"]}`) and every Rust enum falls to `SchemaKind::Any`. Measured: bitbucket 109/216 properties untyped, huggingface 99/227, elevenlabs 73/233. Every string-enum value domain in the catalog is absent from the published contracts.
- **Required headers never become header parameters.** Anthropic's mandatory `anthropic-version` lives only in `x-schematic`; a generically-generated client gets HTTP 400.
- **One status code per operation, no error models, one security scheme max**, `value_prefix` discarded, unsupported OAuth grants silently emit *no* scheme (the document then claims the API is unauthenticated), per-endpoint `oauth_scopes` never become operation `security`.
- **Spec violation shipped:** `openapi/ollama.json` has duplicate `operationId`s (`Embeddings`, `ListModels` each on two paths) — the grouped merge dedupes on `(path, method)` but never checks ids (`gen/src/export/openapi.rs:215-237`).
- **Nested-type `$ref`s can dangle**: registry emission is register-list-driven, but schemars parents `$ref` nested types that were never registered (`anthropic::Usage`, `elevenlabs::SampleModel`…). `validate_completeness` structurally cannot see this — it checks top-level names only.

### 7.2 The x-schematic round-trip is fiction

The extension structs are emitted at document and operation level and **read by nothing** — there are zero `x-schematic` references anywhere under `define/src/openapi/import/`. The importer hardcodes `env_mapping: None, headers: vec![], version: None, …` (`import/builder.rs:339-354`). Three files claim round-trip fidelity (`extensions.rs:4-5`, `export/mod.rs:48-49`, `docs/io/openapi-extensions.md:3`); no round-trip test exists. Two of the four extension structs (`SchematicSchemaExtension`, `AuthExtension`) are never constructed in production at all, and `AuthExtension`'s rustdoc claims an embedding that doesn't exist. Additional import losses: path-item-level parameters silently dropped (a very common OpenAPI idiom in foreign specs), multipart bodies dropped to `request: None`, inline schemas become the literal type name `"serde_json::Value"` — which then makes `import → export` **error out** on the ref-closure check for most hand-written specs. `strict` mode fails on INFO diagnostics and is unusable on realistic input.

### 7.3 Postman

More faithful than OpenAPI on params and headers, with careful auth-variable and mixed-auth-grouping work. But: **every JSON body is the literal string `"{}"`** (the schema registry is never passed in; `postman/examples.rs` is a 5-line placeholder), no test scripts, no saved example responses, no environment files, OAuth2 collapses to `bearer`, `Text`/`Binary` bodies lose content types, and header/cookie params are ignored. Collections are browsable, not runnable.

### 7.4 Pipeline hazards

- **`generate --api <one>` against the real output directory deletes the other seventeen clients.** Cleanup builds its expected-file set from only the requested API, then removes every generator-owned file not in it — empirically verified (24 files/dirs → 10). `lib.rs` is rewritten consistently, so it even compiles afterward and nothing screams (`gen/src/output/mod.rs:257-297,482-506`; `gen/README.md:38` documents this invocation as normal usage).
- A stale, git-tracked duplicate output tree exists at `gen/schematic/schema/` — residue of a wrong-cwd run, easy to mistake for real output.
- Two divergent module-path resolvers (`gen/src/output/assemble/helpers.rs:33-37` applies a suffix heuristic; `definitions/src/lib.rs:248-252` only lowercases) — an API named `PagerDutyApi` without explicit `module_path` would land in `pagerduty.rs` but `pagerdutyapi.json`.

---

## 8. WebSocket Assessment

The five WS definitions are **five dialects**: identical auth handshakes expressed as prose in one module and as `AuthFlowHints` in its sibling; `integration_ws` marks all six messages `Bidirectional` not because they are but to trip a generator heuristic (with a test asserting the contortion); ElevenLabs' WS types don't use `schematic_define` at all. `CorrelationHints` and `HeartbeatHints` have zero users; the generator instead string-matches messages literally named `"RequestEnvelope"` (`gen/src/ws_codegen/plan.rs:188-206`), so Samsung's declared `RequestIdType::String` is dead.

Generated WS clients have a genuinely good transport core (supervisor loop, correlation, backoff, even a host side) but: the typed message lists are computed and **no emitter reads them** — the surface is `send(serde_json::Value)` / `next_event() -> Option<Result<Value, _>>`; **WS auth is a no-op** — `Headers::build()` never consults the emitted `EnvMapping`, so no generated WS client ever sends an env-derived credential, while the Dock module's docs advertise `UCR_DOCK_TOKEN`; the reconnect loop's attempt counter never resets after success (supervisor dies after N *lifetime* reconnects) and lifecycle `BOS` is never re-sent, so a "reconnected" stream is protocol-dead while reporting `Ready`; `receive_timeout` is accepted and ignored; WS plan diagnostics — including errors — are discarded wholesale, so a failing definition produces no file and no message.

Externally, **WebSocket definitions are invisible**: no AsyncAPI export exists, and the AsyncAPI *importer* builds a full `WebSocketApi` + catalog and then discards both, writing only a count summary. Five WS APIs, zero artifacts.

Two concrete defects worth fixing regardless of any redesign: the shipped `ws://remote.local/ws/ws` URL (base_url ends in the endpoint path; `definitions/src/unfolded_circle/core_ws/mod.rs:182,144` → `schema/src/unfolded_circle_core_ws.rs:106` — found independently by two reviewers), and a dormant guaranteed `E0308` in the Bearer host template (trailing `;` making an `else` arm yield `()` against a `Result` tail, `gen/src/ws_codegen/shared.rs:526-534`) that survives because `validate_code` is grammar-only and its unit test asserts substrings.

---

## 9. How the Ergonomics Should Improve (ranked)

The directory name says "macro review," so the headline question first: **should definition authoring get a macro?** Recommendation: **builders first, macro optional on top.** A `define_api!` proc-macro could collapse the ceremony, but it would trade struct-literal debuggability and IDE support for a bespoke syntax, and the deeper problems (string wiring, silent drops) are not macro-shaped. Builders + typed references fix the same ceremony additively, keep rustc as the error surface, and leave room for a thin declarative macro later if wanted. A `macro_rules!` sugar layer over builders is cheap once builders exist.

1. **Builders/constructors for `Endpoint` and `RestApi`** (`Endpoint::get("GetRepository", "/repos/{owner}/{repo}").returns_json::<RepositoryInfo>()`…), `impl Into<String>` everywhere, `Default` on both. Collapses the median literal from 11 lines to 2–4, deletes ~1,000 `.to_string()` calls and all 253/268/201 `None`/`vec![]` lines, and — because callers stop writing literals — makes future field additions non-breaking. Purely additive; existing literals keep compiling.
2. **Bind type references to types.** Invert the registry: `ApiResponse::json::<RepositoryInfo>()` deriving the name from the type (via a `SchemaName` trait or `schemars::JsonSchema::schema_name`), auto-registering into the registry. Kills the typo-consistent failure class, the 36 hand-registered `Vec<T>` strings, and alias hacks like `pub type Commit = CommitInfo`. Interim cheap version: `register::<T>(name)` asserts `name` against `std::any::type_name::<T>()`.
3. **Turn silent drops into hard `validate_api` errors** until implemented: non-JSON request bodies (§6.1), `ApiKeyParam` (§6.2), non-identifier `schema.type_name`/`module_path` (panic → diagnostic), header/cookie param identifiers, duplicate endpoint ids, duplicate `(method, path)`, `?` in path templates, base_url/path join producing doubled segments (would have caught `/ws/ws`).
4. **Un-`#[ignore]` `generated_code_compiles`** and extend it to all APIs (slow tier). It is the only check that catches a dangling type name; today nothing on a normal run verifies the generated crate compiles.
5. **Model streaming** — `ApiResponse::Stream { framing: Sse | Ndjson, item: Schema }` — and give `Binary`/`Text` responses the `content_type` their request twins already have. Un-strands ten Ollama types, fixes the buffering `stream_speech`, stops the exporter misreporting audio endpoints.
6. **Make `ApiResponse` a status→schema map** (or add `errors: Vec<(u16, Schema)>` as the v1.5 compromise). Unlocks typed errors everywhere: generated clients, OpenAPI error responses, Postman examples.
7. **Emit `Endpoint.params` and required constant headers as real OpenAPI parameters** (`export/paths.rs:83`) — `ParamDef` maps 1:1 onto `openapiv3::Parameter`. This single fix makes exported specs generatable by third parties and closes the embarrassing gap where Postman is more faithful than OpenAPI.
8. **Fix the schemars→openapiv3 bridge** (type arrays → `nullable`, `enum`, `oneOf/anyOf/allOf`, `additionalProperties`), plus a regression test asserting zero `SchemaKind::Any` across every registry. Recovers the ~30% untyped-property hole.
9. **Read `x-schematic` on import or strike the round-trip claim**; either way add one `export → import → export` equality test. Delete or wire the two dead extension structs.
10. **Fix `--api <one>` destructive cleanup** (scope cleanup to regenerated modules, or require `--prune`), delete the stale `gen/schematic/` tree, unify the two module-path resolvers, feature-gate the generated crate per API.
11. **Generate the registry and API lists** from one source (build script or macro over `apis_by_module()`), collapsing the seven drifted parallel name lists to one.
12. **Generated-docs honesty pass:** emit ` ```no_run ` instead of ` ```text `, generate typed convenience methods for JSON endpoints (fixes the nonexistent-`list_models()` example *and* the unchecked `request<T>` hole in one move), fix the `ApiKey` doc literal, drop `///;`.
13. **WS follow-through:** emit the already-computed typed message enums, resolve env auth at connect time (or fail loudly when `env_auth` is declared), replace the `"RequestEnvelope"` string-match with an explicit `responds_with` link on `MessageSchema`, fix the reconnect counter + lifecycle replay, surface WS diagnostics, fix the dormant Bearer-template `E0308`.
14. **`schematic-define` hygiene:** `#![doc = include_str!("../README.md")]` so the ~7 currently-broken README examples become compiling doctests; move vendor pagination constructors to `schematic-definitions`; collapse `ConnectionParam`/`ParamType` into `ParamDef`/`QueryParamType`; either consume `PaginationStyle`'s payload (a real `fetch_all_pages` needs exactly those fields) or shrink it; retire `env_auth[0]`-as-password by making `Basic` carry its env shape; add `deprecated` to `Endpoint`/`ParamDef`.

---

## 10. Defect Ledger

Concrete bugs found during this review, independent of any design opinion:

> **Status update (2026-07-31, OpenAI import work).** Items 1, 4, 20, and 21 are
> fixed, and #16's concern is partly addressed: the generated crate is now
> compiled and clippy-clean as part of `just lint-schema`. Everything else stands.
>
> Separately, §7.1's "0 query parameters across all 15 artifacts" and §9's
> recommendation 7 are now done: operation
> parameters now export (OpenAI 288, GitHub 31, GitLab 32, Gitea 22, Bitbucket 17).
> HuggingFace and ElevenLabs still show 0 because their *definitions* declare none.
> A second bug surfaced while fixing it — path parameters were emitted once per
> method sharing a path, so `/models/{model}` declared `model` twice, which OpenAPI
> forbids. Both are now covered by wiring-level tests in `export/mod.rs`;
> `map_parameters` had full unit coverage while the call site still said
> `parameters: vec![]`, which is how the defect survived.

| # | Defect | Location | Status |
|---|---|---|---|
| 1 | Non-JSON request bodies silently dropped; 6 endpoints POST empty bodies | `gen/src/codegen/request_structs/body.rs:24-27`, `into_parts.rs:20-28` | **FIXED** — `RequestParts` now carries `shared::RequestBody`; form endpoints get a generated `{EndpointId}Form` |
| 2 | `generate --api <one>` deletes all other generated clients | `gen/src/output/mod.rs:257-297,482-506` | |
| 3 | Shipped wrong URL `ws://remote.local/ws/ws` | `definitions/src/unfolded_circle/core_ws/mod.rs:182` → `schema/src/unfolded_circle_core_ws.rs:106` | |
| 4 | Generator panic on non-identifier request `type_name` / `module_path` | `gen/src/codegen/request_structs/body.rs:16`, `output/assemble/lib_rs.rs:29` | **FIXED** — all type-name sites route through `type_name_to_tokens` (syn-based) |
| 5 | `ApiKeyParam` clients never send the key; env fallback silently dropped | `gen/src/codegen/client/helpers.rs:128-217`, `define/src/types.rs:334` | |
| 6 | Duplicate `operationId`s in `openapi/ollama.json` (OpenAPI 3.0 violation) | `gen/src/export/openapi.rs:215-237` | |
| 7 | Nested unregistered types → dangling `$ref`s in published specs | `definitions/src/registry/mod.rs:244-257` (can't see them) | |
| 8 | Generated docs call nonexistent `list_models()` in 3+ places; `ApiKey` doc literal missing required field | `gen/src/output/assemble/lib_rs.rs:246-254,317-319` | |
| 9 | WS reconnect counter never resets; lifecycle open never re-sent | `gen/src/ws_codegen/client.rs:641-682` | |
| 10 | Dormant `E0308` in Bearer host template (trailing `;`) | `gen/src/ws_codegen/shared.rs:526-534` | |
| 11 | WS env auth never applied despite emitted `EnvMapping` + docs advertising the env var | `gen/src/ws_codegen/client.rs:376-377` | |
| 12 | WS plan diagnostics (incl. errors) discarded; failing definition → no file, no message | `gen/src/output/ws_modules.rs:123-126` | |
| 13 | x-schematic round-trip documented, unimplemented, untested | `define/src/openapi/import/builder.rs:339-354` | |
| 14 | `x-schematic` `skip_serializing_if` without `#[serde(default)]` breaks re-import of own exports (empty-collection case) | `define/src/openapi/extensions.rs:55-57,96-97,150-151` | |
| 15 | Test API list drift: 2 of 18 APIs skipped by strict-completeness gate | `gen/tests/openapi_strict_completeness.rs:176-192` | |
| 16 | `generated_code_compiles` is `#[ignore]`d; nothing on a normal run compiles generated code | `gen/tests/e2e_generation.rs:85-87` | **PARTLY** — `just lint-schema` compiles + clippies the generated crate; the `#[ignore]` test itself remains |
| 17 | `strict` import mode fails on INFO diagnostics; tautological test | `define/src/openapi/import/builder.rs:191-196,765` | |
| 18 | Stale git-tracked duplicate output tree | `gen/schematic/schema/` | |
| 19 | `schematic-define` README: ~7 drifted/non-compiling examples, not doctested | `define/README.md:152,246,261,362,425-430,598,651,733-848,865` | |
| 20 | Inline foreign-spec schemas imported as literal `"serde_json::Value"` → re-export fails ref-closure | `define/src/openapi/import/mappings/responses.rs:157` | **FIXED** — export emits an inline `Any` schema for path-shaped type names |
| 21 | 21 clippy warnings in generated crate (`&self.x` in `format!`, `{+param}` branch) | `gen/src/codegen/request_structs/shared.rs:122` | **FIXED** — `{+param}` branch no longer emits a redundant `&` |
| 22 | Two divergent module-path resolvers (Rust vs OpenAPI/Postman filenames) | `gen/src/output/assemble/helpers.rs:33-37` vs `definitions/src/lib.rs:248-252` | |

---

## 11. Bottom Line

The user-facing questions, answered directly:

- **What works well?** The pipeline mechanics (determinism, atomic writes, formatting), the auth model's layering, the JSON REST happy path end to end, form-field vocabulary, import name sanitization, Postman auth handling, WS transport runtime.
- **What is awkward?** Everything about volume: 11 lines per endpoint of unaided struct literal, string-wired types with string-vs-string validation, seven parallel API lists, dual legacy/modern auth fields, vendor names in the neutral crate, and a model whose gaps (streaming, errors, optional auth, enums) get patched as comments, prose, and deliberate wrong-paths in the definitions.
- **What's not supported?** Streaming, typed errors, per-status shapes, response content types, typed path params, per-endpoint auth override, deprecation, rate limits — all additively fixable. Polymorphic bodies, status-discriminated responses, serializable definitions, and WS/REST unification require vocabulary v2.
- **How well does generated code represent the definition?** Rust: high fidelity for JSON+auth+headers+paths, silent zero fidelity for form bodies, param metadata, pagination structure, and `ApiKeyParam`. OpenAPI: real schemas but no parameters, no enums for ~30% of properties, no errors — browsable, not generatable. Postman: parameter-faithful but body-empty. WebSocket: no external representation at all.
- **Highest-leverage moves:** builders + typed schema references (ergonomics), silent-drop → validation-error conversion plus un-ignoring the compile test (safety), `Endpoint.params` emission + the schemars bridge fix (OpenAPI credibility), and the `--api <one>` cleanup fix (operational safety).
