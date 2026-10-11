---
created: "2026-07-31T12:45:56-07:00"
agent: claude
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
status: finalized-spec
reviewed: true
reviewed_by: "codex/gpt-6.1-sol"
reviewed_on: "2026-10-09"
review_iterations: 0
clarified: true
clarified_by: "claude/sonnet"
review_note: "the clarification process served as a review"
needs_rulings: false
references:
    spikes/pagination/research.md: "Catalog of how real HTTP APIs paginate (page number, offset, cursor, link-following, keyset, snapshot sessions, and awkward cases), with each claim tagged [V] verified from an official page or [M] from memory."
    spikes/pagination/inventory.md: "How every list endpoint of the 18 REST APIs in schematic-definitions really paginates, which endpoints declare pagination today (35, all page-number), and which declare none (for example OpenAI's 61), with a confidence tag per row."
    spikes/pagination/design.md: "The proposed pagination model: four strategy kinds plus the general dimensions, builder authoring examples, the generated-client surface, the coverage matrix against the inventory and outside patterns, and open questions."
    spikes/pagination/prototype/: "A throwaway 56-test standalone crate (not part of the workspace) proving the model expresses every inventoried strategy and 10 outside patterns; evidence for the design, not shippable code."
    spikes/typed-send/findings.md: "Results of the typed-send spike (2026-10-11): D3 typed request and the D7 envelope work in Rust, per-question verdicts, measured compile cost, and the semantic details that need author rulings."
    spikes/typed-send/: "A throwaway standalone prototype crate (not part of the workspace) implementing the D3/D7 shape with 28 fixture tests, 5 ported sniff call-site tests, and 8 compile-fail cases; evidence only, not shippable code."
    spikes/pager-surface/findings.md: "Results of the pager-surface spike (2026-10-11): one generic pager whose knobs are compile-time checked, plus measured deadline, cancel, host-policy, loop-detection, early-drop, and binary-size behavior."
    spikes/pager-surface/: "A throwaway standalone prototype crate (not part of the workspace) with 31 runtime tests and 21 trybuild cases (19 compile-fail, 2 pass) for the D14 pager surface; evidence only, not shippable code."
---

# Schematic Definition-and-Generation Review

**Date:** 2026-07-31
**Scope:** How APIs are defined with `schematic-define` primitives, and how `schematic-gen` converts those definitions into Rust client code (`schematic-schema`), OpenAPI documents (`schematic/openapi/`), and Postman collections (`schematic/postman/`).
**Method:** Four parallel full-source reviews (definition primitives; the 18 REST + 5 WebSocket definitions in `schematic-definitions`; Rust codegen and generated output; OpenAPI/Postman export-import), each with independent file:line verification, cross-checked against each other and spot-verified in the main review. Regeneration was verified byte-identical against the committed output; the generated crate `cargo check`s clean. Historical plain-text paths below are relative to `schematic/`; Markdown links resolve from this document.

> **Reader's note — inline review, 2026-10-09.** Sections 1–8 record the July investigation; their counts, line numbers, and regeneration results are historical evidence, not a fresh certification of the current tree. Sections 9–10 distinguish resolved work from remaining recommendations. Sections 12–15 correct the runtime comparison, define implementation boundaries, group the work (13a), and record decisions D1–D8 confirmed by the author on 2026-10-09 (D9–D16 followed on 2026-10-10; see the next note) alongside the questions still open (15). Contracts in section 13 that carry the label "Proposed (pending ratification)" were drafted in this review and are not yet confirmed by the author. The July full regeneration and compilation were not repeated for this document review.

> **Reader's note, 2026-10-10.** The author confirmed eight more decisions, D9-D16 (section 15): partial generation, request-side feature support, credentials, the credential host policy, a shared deadline, the pagination model, no public `Default` on definition types, and full GraphQL support. They ratify or replace most blocks labelled "Proposed (pending ratification)" in section 13; those blocks now say which decision covers them. Only the complete-list rule remains Proposed. D16 is **new scope** beyond what this document previously included. A pagination spike backs D14; its files are listed under `references:` in the frontmatter and live in `./spikes/pagination/`. Where earlier text is superseded, a dated note says so; the original text is kept.
>
> **Reader's note, 2026-10-10 (second round).** The author confirmed five more decisions, D17-D21 (section 15): GraphQL operations are authored as strings in the definition (D17, narrows D16); a fourth landing carries fidelity and hygiene work (D18, amends D5); pagination moves into landing 1 (D19, amends D5 and the D14 placement); the whole-work definition of done includes retiring sniff's hand-written provider client (D20); and the response envelope carries a fixed set of request metadata with redaction (D21, resolves part of D7's open field list). Wherever earlier text says "three landings", places the pagination definition model in landing 2, calls the whole-work definition of done open, or lists the envelope's field set as open, a dated note says what changed and the original text is kept. On two points the author chose against this review's recommendation (D17: this review had recommended query files validated against a provider schema; D19: this review had recommended pagination entirely in landing 2); both decisions record the rejected option.
>
> **Reader's note, 2026-10-11 (spike results).** Two prototype spikes ran against D3, D7, D13, and D14: `typed-send` and `pager-surface`. Their results are recorded under "Spike results (2026-10-11)" in section 15. They validated the decided shapes and surfaced new details. **No new ruling is decided by that section**: every shape in it is labelled "Recommended by spike; not yet ratified by the author", and the new rulings are in the Open Questions. Dated notes were added to D3, D7, D12, D13, D14, D21, section 13a, and section 14; the original text is kept.

> **Reader's note, 2026-10-11 (rulings D22-D29).** After the spike results, the author confirmed eight more decisions, D22-D29 (section 15): the credential-free promise is narrowed (D22); network-error text keeps the URL with secrets masked (D23); failures before anything is sent are the outer `TransportError` (D24); an undecodable 2xx reply and a cut-off body are failures inside the envelope (D25, D26); paging sessions are closed only through a scoped `with_session` helper (D27); the cross-origin policy is authored in the API definition and governs redirects and followed next-page links (D28, which amends D4 and D12); and GraphQL errors and partial data inside an HTTP 200 reply go on the failure side (D29). The spike "recommended shapes" are now ratified **only where one of these rulings covers them**; each ratification is stated in "Spike results (2026-10-11)". The previous note's sentence that no new ruling is decided there is superseded for those items.
>
> **Limitation to know before relying on redaction (D22).** Only Schematic's stored metadata and `Display` output are guaranteed free of credentials. A provider's decoded error body is printed as received by `Debug`, so a provider that echoes a credential in an error message can leak it through `Debug` logging.
>
> **Reader's note, 2026-10-11 (second): rulings D30-D48.** The author confirmed nineteen more decisions, D30-D48 (section 15). They answer all twelve rulings the frontmatter used to list, so `needs_rulings` is now `false` and the `required_rulings` list is removed. In short:
>
> - **Where work lands.** Landing 1 builds the redirect-following runtime and the API-level `cross_origin` policy field (D30); request-side features are generator-only work in landing 1 (D33); GraphQL, its error handling, and the per-endpoint `cross_origin` override land in landing 2 (D30). WebSocket is landing 3 and fidelity is landing 4, which also gets the single generated API list, required constant headers as OpenAPI parameters, and an AsyncAPI exporter (D46, D47).
> - **Runtime shape.** A new hand-written runtime package holds the shared runtime and the generated clients depend on it (D39). Generated clients no longer accept an injected `reqwest::Client`; a Schematic-managed builder replaces it, so the cross-origin rule always holds (D31). A refused redirect is a failure inside the envelope; a refused next-page link is an outer `TransportError` (D32). `send_raw` is the escape hatch (D45).
> - **Pagination.** Landing 1 adds no field to the 253 endpoint literals (D42, which removes the cost D19 had accepted): `Vec<T>` on a GET endpoint is a list automatically (D43), and a table of exceptions on each API definition covers the rest until landing 2. `collect_all` exists and never reports a partial list as complete (D41).
> - **Response model.** Same-address streaming and non-streaming endpoints are two endpoints distinguished by a selector (D36); the stream end condition is declared (D37); error shapes map from status or range with most-specific-wins (D38).
> - **GraphQL.** It lives in `schematic-define` and `schematic-gen`, reuses the REST machinery (D34), and derives its address from the client's base address with a caller override (D35).
> - **Definition and envelope details.** Typed registration naming (D44), secret marking (D40), and the envelope's no-declared-error type, request-id field, auth-method values, and built-in secret names (D48).
>
> What remains open is only planning-time detail, listed under "Remaining planning-time details (open)" in section 15. Two things this review found while recording the rulings are noted where they matter: D31's evidence ("no crate calls `with_client`") is incomplete, because two homelab call sites call `with_client_and_base_url` (see D31); and D41's list of incomplete reasons has no entry for an invalid or missing continuation (see D41).

> **Landings at a glance (current as of 2026-10-11; details in section 13a).** This document keeps the word "landing" for one shippable batch of work. In plain terms:
>
> - **Landing 1, "safety and shared runtime":** fixes the dangerous behaviors and ships the new hand-written runtime package: the response envelope, typed sends, credentials, deadlines, redirect and cross-origin handling, pagination, and the request-side features. It edits the 18 API definitions (two new fields each) but none of the 253 endpoint definitions.
> - **Landing 2, "definition vocabulary":** rewrites the 253 endpoint definitions once, into builders with typed registration, and adds the new definition vocabulary (response model v2, per-endpoint overrides, GraphQL).
> - **Landing 3, "WebSocket":** fixes and reworks the WebSocket definitions and generated clients.
> - **Landing 4, "fidelity and hygiene":** OpenAPI, Postman, and AsyncAPI output quality, the single generated API list, per-API feature flags, and housekeeping that needs no second pass over the definitions.
>
> Terms used in this document: the **envelope** is the `Response<T, E>` value every send returns (it holds status, headers, and either the decoded success or a failure); **`TransportError`** is the outer error for "no response head arrived"; a **selector** is a request-body field value (such as `stream=true`) that tells two endpoints on the same address apart.

> **Correction, 2026-10-11:** the count of 253 endpoint literals, and every ratio derived from it, is stale. The verified count is about 519 (see the correction in section 2). Read "253" in this place as "about 519". The landing 2 rewrite is the largest in that sense; D52 splits spec 4 in two because of it.

> **How to read this document if you are new to the repository.** `schematic-define` holds the primitives an author uses to describe an API (`RestApi`, `Endpoint`, and so on). `schematic-definitions` holds the 18 REST and 5 WebSocket API descriptions written with those primitives. `schematic-gen` turns descriptions into Rust client code, OpenAPI documents, and Postman collections. `schematic-schema` is the generated Rust client crate. Sections 1-11 are the July review, 12 is an addendum, 13-14 are contracts and tests, and 15 holds the decisions and open questions.

---

## 1. Executive Summary

The pipeline's core competencies are real: deterministic byte-identical regeneration, `quote!`/`syn`/`prettyplease` codegen with atomic ownership-gated writes, a layered auth model (`AuthMethod` / `EnvAuthStrategy` / `AuthPolicy`) that is genuinely better thought out than most, correct `#[non_exhaustive]` discipline, env fallback chains as data, and export-side `$ref`-closure validation that provably holds across all 15 artifacts. For a JSON-in/JSON-out REST API with bearer auth, the system works end to end and the generated client is pleasant to consume.

The honest overall verdict, though, is that **the definition vocabulary is significantly richer than what the generators consume, and the generators are more trustworthy than the definitions deserve.** Three structural problems dominate:

1. **Ceremony without safety.** Defining an endpoint costs a fixed ~11-line struct literal (253 of them, no builder, no `Default`, no macro), yet the thing that ceremony should buy — a verified link between the definition and the Rust types it names — does not exist. Type wiring is by hand-typed string, validated string-against-string. The crate currently survives on discipline: a systematic sweep found zero dangling references, but also found the type system being bent to fit strings (`pub type Commit = CommitInfo;` exists solely so a string literal resolves) and the only test that would catch a bad name (`generated_code_compiles`) is `#[ignore]`d.

> **Correction, 2026-10-11:** the count of 253 endpoint literals, and every ratio derived from it, is stale. The verified count is about 519 (see the correction in section 2). Read "253" in this place as "about 519".

2. **Silent fidelity loss, concentrated in exactly the features the vocabulary advertises.** Form/multipart request bodies generate clients that POST **no body at all** (6 endpoints today). `Endpoint.params` never reaches OpenAPI output (0 query parameters across all 15 exported specs, while the same data *does* reach Postman). `ApiKeyParam` auth generates clients that never send the key. Pagination's 12 fields of structure are read as one boolean. Every one of these fails silently — compiles, exports, looks done.

3. **Two of the four output targets are materially weaker than they appear.** The OpenAPI documents are browsable but not generatable (~30% of schema properties export as untyped `{}`, no error responses, required headers only in `x-schematic`), and the documented `x-schematic` round-trip is fiction — the importer never reads the extension. WebSocket definitions produce no external artifact at all, and the generated WS clients are transport-rich but type-empty and never authenticate from env.

None of this requires a ground-up redesign. The single highest-leverage change is ergonomic (builders and typed schema references, section 9, recommendations 1–2); the single most urgent changes are converting remaining silent drops into validation errors (section 9, recommendation 3) and fixing destructive single-API regeneration (section 7.4).

> **Update, 2026-10-11:** Several of the fixes recommended here are now decided. Endpoint and API builders with no public `Default` are D15 (landing 2); typed schema registration is D2 and D44; the form, text, binary, and `ApiKeyParam` request gaps are implemented rather than turned into errors (D10, D33, landing 1); pagination gets a real model and generated pagers (D14, D19, D42); streaming and typed errors are in scope (D6); OpenAPI, Postman, and AsyncAPI quality is landing 4 (D18, D47); and the destructive single-API regeneration is replaced by whole-module-group regeneration (D9).

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

> **Correction, 2026-10-11:** the endpoint literal count is about 519, not 253. `grep -c 'oauth_scopes: None'` over `definitions/src` gives 519; a boundary-anchored `Endpoint {` match gives 577, which includes tests and helpers. Per module: openai 265, elevenlabs 43, emqx 39, gitlab 34, github 29, gitea 28, huggingface 28, bitbucket 27, eversolo 26, ollama 15, unfolded_circle 12, artificial_analysis 8, samsung_smart_tv 7, lmstudio 6, registry 6, anthropic 4. The median and the lines-per-endpoint ratio above were derived from 253 and are stale; the 27,638-line and 43,325-line totals were not re-measured.

Response and request types are wired **by name**: `json_type("RepositoryInfo")` has no compile-time relationship to `pub struct RepositoryInfo`, and `Vec<T>` responses are the literal string `"Vec<RepositoryInfo>"` (`define/src/response.rs:125`). A parallel `SchemaRegistry` maps hand-typed name strings to `schemars` schemas (`.register::<RepositoryInfo>("RepositoryInfo")`), and `validate_completeness` checks endpoint strings against registry strings — string vs. string, both written by the same hand (`definitions/src/registry/mod.rs:238-267`). The upstream test says it plainly: *"content does not matter, only the name does"* (`gen/tests/openapi_strict_completeness.rs:122-124`).

> **Correction, 2026-10-11:** the strict-completeness test lives at `schematic/gen/tests/l1/openapi_strict_completeness.rs`, not `gen/tests/openapi_strict_completeness.rs`.

> **Update, 2026-10-11:** Builders and constructors for `Endpoint` and `RestApi`, without a public `Default`, are decided in D15 and land in landing 2, together with the single rewrite of the 253 literals (D5, D42). Typed registration replaces the hand-typed name strings (D2); defaults come from the type, `Vec<T>` reuses its element's registration, and aliases are explicit (D44).

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

> **Correction, 2026-10-11:** `oauth_scopes: None` appears about 519 times, not 253 (see the correction in section 2); the other per-line counts in this paragraph were taken against the same stale literal count and are not re-measured. "A feature no definition uses" needs a qualifier: `Endpoint::oauth_scopes` has no generator consumer, but it is copied into the `x-schematic` extension at `define/src/openapi/export/paths.rs:88` and emitted by the importer at `gen/src/definition_gen.rs:178-196`.

### 4.2 Dual sources of truth in the core types

`RestApi` carries both the legacy fields (`auth`, `env_auth`, `env_username`) and their successors (`auth_policy`, `env_mapping`), reconciled at runtime (`define/src/types.rs:289-307`). Real definitions set **both** — `definitions/src/openai/mod.rs:72,113-116` writes `OPENAI_API_KEY` twice, and the `env_auth` copy is silently ignored. Basic auth's password lives positionally in `env_auth[0]`, a contract that exists only in prose (`define/src/auth.rs:116-117`).

> **Correction, 2026-10-11:** `RestApi::auth_policy` does have a codegen consumer: `effective_auth_policy()` at `gen/src/codegen/api_struct/mod.rs:77` and `helpers.rs:7`, and `constructors.rs` emits it. No definition sets it. So it is not dead surface in the generator; it is unused by definitions.

### 4.3 Everything is written three times

The struct literal, a hand-maintained Markdown endpoint table in the module doc, and a unit test asserting the literal says what it says (e.g. `definitions/src/emqx/mod.rs:37-50` + `:387-677`). The registry adds four more hand-maintained parallel lists of the same 18 API names across `definitions/src/lib.rs`, `registry/lookup.rs`, and `gen/src/pipeline.rs` — **seven total, already drifted**: `gen/tests/openapi_strict_completeness.rs:176-192` lists 16 of 18, so both ArtificialAnalysis APIs are silently skipped by that gate today. Adding one API requires ~13 mandatory edits across two crates.

> **Update, 2026-10-11:** The seven parallel API lists are to be replaced by one generated list in landing 4 (D46); until then the drift described here remains.

> **Correction, 2026-10-11:** the "seven parallel API lists" are six runtime lists plus a test table: `definitions/src/lib.rs:224` (`apis_by_module`); `registry/lookup.rs:41-56` and `:87-103`; `gen/src/pipeline.rs:21` (`AVAILABLE_APIS`), `:49-65`, and `:94-114`; and the 16-name table in `gen/tests/l1/openapi_strict_completeness.rs:176-193`. The table at `definitions/src/registry/mod.rs:744` is a candidate seventh.

### 4.4 Vendor knowledge leaked into the neutral layer

`PaginationStyle::github()/gitlab()/bitbucket()/gitea()` live in `schematic-define` (`define/src/pagination.rs:72-128`) — and `gitlab()` is literally `Self::github()`. A new page-number API must either call a competitor's constructor or edit the primitive crate. The WS generator hard-codes `UnfoldedCircleEventHub` as the host struct name for every API and an Unfolded-Circle auth envelope as the generic success response (`gen/src/ws_codegen/host.rs:66`, `shared.rs:436-444`).

> **Update, 2026-10-11:** Moving the vendor pagination constructors out of `schematic-define` is landing 4 (D18), and the `PaginationStyle` type itself is replaced by the pagination model of D14. The WebSocket generator's hard-coded host name and auth envelope belong to the WebSocket landing (D8, D46).

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

> **Update, 2026-10-11:** Streaming is now in scope: a success shape can be a stream with SSE or NDJSON framing (D6), same-address streaming and non-streaming endpoints are two endpoints told apart by a selector (D36), and the stream end is declared (D37). Typed error bodies and per-status shapes are in scope too: the API declares a default error list that endpoints refine (D38), and endpoints with no declared error body report error replies as `Undeclared` (D48). Response content types are part of the same vocabulary (D6).

4. **Typed path parameters.** `EndpointParams` has `query`/`header`/`cookie` but no `path`; `{owner}` exists only as a substring recovered by string-scraping (`gen/src/parser.rs:29`). Untyped, undescribed, unvalidated — nothing checks declared params against the template, or that two endpoints don't share `(method, path)`.
5. **Per-endpoint auth override, deprecation, rate limits, retry policy.** No way to mark one endpoint public on an authenticated API; no `deprecated` field anywhere (export hardcodes `deprecated: None`); zero rate-limit/retry vocabulary despite `PaginationResponse::LinkHeader` proving response-header semantics are modelable.
6. **Feature-usable but dead surface.** `AuthStrategy::ApiKeyParam` (query/cookie keys) exists but: legacy env mapping silently drops it (`define/src/types.rs:334`), `EnvMapping` has no field that can hold it, and no codegen ever injects the key (§6.1). `Endpoint::oauth_scopes` and `RestApi::auth_policy` have zero users and zero codegen consumers. 253 lines of `oauth_scopes: None` buy nothing.

> **Update, 2026-10-11:** Typed path parameters, per-endpoint auth override, `deprecated`, and rate-limit and retry vocabulary are in scope and assigned to landing 2 (D18). `ApiKeyParam` auth is implemented in the generator in landing 1 (D10, D33).

## 5b. Not Supportable Without Redesign

> **Reader's note, 2026-10-09.** The title records the July assessment. Decision D1 (section 15) says nothing is deferred, so this redesign work is in scope. Item 2 (the `Endpoint::response` change) is decided as D6 and lands in landing 2 (D5). Item 4 (REST/WS unification) is narrowed by D8: WebSocket reuses the REST parameter, auth-policy, env-mapping, and header types, but full unification is rejected for this work. Items 1, 3, and 5 (polymorphic `oneOf` bodies, serializable `RestApi`, OAuth2 per-grant payloads) are in scope under D1 but have no landing yet; they are listed as an open assignment in section 13a. Earlier drafts said these "remain unscheduled"; that contradicted D1 and is withdrawn.

> **Update, 2026-10-11:** Landing is now assigned: polymorphic `oneOf` bodies, serializable definitions, and OAuth2 per-grant payloads (items 1, 3, 5) go to landing 2 (D18), so they are no longer an open assignment. Item 4's REST/WS vocabulary sharing is the WebSocket landing, landing 3 (D8, D46).

1. **Polymorphic (`oneOf`) bodies from hand-written definitions.** `TypeRef` models `OneOf`/`AnyOf`/`AllOf`, but `ModelCatalog` is unreachable from `RestApi` — it exists only as a sibling output of the importers. A hand-authored definition cannot attach structural models; it can only name Rust types. Bridging `Schema` (a name) to `TypeRef` (a structure) is the redesign.
2. **Status-discriminated responses + typed errors together** mean changing the type of `Endpoint::response`, which breaks the vendor-extension format, both exporters, the importer, and all 253 endpoint literals. Doable, but it is v2 of the vocabulary.
3. **Serializing a hand-authored `RestApi`.** `RestApi`/`Endpoint`/`EndpointParams`/`ParamDef` are not `Serialize` (while `WebSocketApi` is — with no stated principle for the split), and the by-name `Schema` contract means a serialized definition still isn't self-contained.
4. **WebSocket as a peer of REST.** The WS model forked the vocabulary (`ConnectionParam`/`ParamType` vs `ParamDef`/`QueryParamType` — split over a name clash, acknowledged in a doc-comment apology at `define/src/params/param_def.rs:47-48`), lacks `AuthPolicy`/`EnvMapping`/headers entirely, and the generator collapses auth to three WS-specific cases. Unifying means one `ApiDefinition` with a transport discriminant — a rewrite of both trees.
5. **OAuth2 grants beyond the three enumerated.** `OAuth2Config` is a flat struct where every illegal grant/URL combination is representable; per-grant payloads are the fix and a breaking change.

---

## 6. Fidelity: Definition → Generated Rust

**Preserved faithfully:** auth strategy/policy as reified constructor literals (including full OAuth2 config), env-mapping fallback chains actually consulted at request time, API- and endpoint-level headers with correct endpoint-wins merge, percent-encoded path params with `{+param}` opt-out, query params as `Option<T>` + `with_*` builders, response *kind* discriminants with a runtime guard, response *types* parsed via `syn` (so `Vec<T>` works), descriptions into docs.

**Lost or degraded, ranked by damage:**

> **Stale-text note, 2026-10-09.** Item 1 below is partly out of date: form and URL-encoded request bodies are now emitted; text and binary request bodies are still dropped (section 10, defect 1). Item 8's panic on non-identifier type names is fixed (defect 4); malformed type syntax still lacks validation.
>
> **Superseded in part, 2026-10-10.** Item 1 (text and binary bodies) and item 2 (`ApiKeyParam`) and the dropped header and cookie parameters in item 3 are now decided as features to implement, not errors to raise (D10). Item 4 (pagination as an empty marker trait) is superseded by the pagination model of D14; the generator still ignores all pagination data today.

> **Update, 2026-10-11:** The request-side gaps in items 1 to 3 (text and binary bodies, `ApiKeyParam`, header and cookie parameters) are generator-only work in landing 1 (D33); required query parameters, `ParamStyle`, `explode`, and enums are vocabulary work in landing 2 (D18). Pagination (item 4) is built in landing 1 with no new field on the 253 endpoint literals (D19, D42).

1. **Form/multipart/urlencoded/text/binary request bodies vanish silently.** `generate_body_field` emits nothing for non-JSON variants — the comment says "The generated code will handle these differently" (`gen/src/codegen/request_structs/body.rs:24-27`) but `into_parts` emits `body: None` for everything non-JSON (`into_parts.rs:20-28`). ElevenLabs' `AddVoiceSample` — a file-upload endpoint with a `FormField` definition — generates a request struct with no audio field that POSTs an empty body (`schema/src/elevenlabs/requests.rs:676-705`). Six endpoints affected today. No error, no diagnostic. **This is the single worst fidelity failure in the pipeline.**

> **Update, 2026-10-11:** "Six endpoints" is a July figure and is partly out of date (form and URL-encoded bodies are fixed since). The counts verified on 2026-10-11 are: text bodies 0, binary bodies 0, `ApiKeyParam` 0, header or cookie parameters 0, multipart about 29. The "six" and the "about 29" are two measures that have not been reconciled. The live count is established when D33 is implemented (D49).

> **Correction, 2026-10-11:** the ten `FormData` request literals in `definitions/src/openai/mod.rs` are current; text and binary request literals are 0. The July "six endpoints" figure is unverifiable.
2. **`ApiKeyParam` emits clients that never authenticate.** Header resolution is header-only; `headers_satisfy_fallback` returns `false` for it, so such an API would be permanently un-authable with a misleading "Authentication required" error (`gen/src/codegen/client/helpers.rs:128-217`).
3. **Query-param `required`, `ParamStyle`, `explode`, and `Enum` are all discarded.** Everything becomes `Option<String>`-ish: required params silently optional (runtime 400 instead of compile error), arrays always `form/explode=true`, GitLab's 11-value status enum collapses to `String` (`gen/src/codegen/request_structs/shared.rs:50-176`). `EndpointParams.header`/`.cookie` have zero readers.

> **Update, 2026-10-11:** Query-parameter `required`, style, `explode`, and enum fidelity change the definition vocabulary and are assigned to landing 2 (D18).

4. **Pagination becomes an empty marker trait.** All 12 fields of `PaginationStyle` reduce to `impl Paginated for X {}` where `Paginated` has no methods — and its doc describes a `fetch_all_pages` helper that does not exist (`gen/src/codegen/traits.rs:86-102`, `schema/src/shared.rs:376`). `response_pagination` has zero readers anywhere.
5. **The declared response type is discarded at the call site.** `request<T>` is fully generic; `client.request::<WrongType>(ListModelsRequest::default())` compiles. `EndpointSpec::Response` types only the mutator hooks.

> **Update, 2026-10-11:** The free generic `request<T>` is removed from the public surface; each request type declares its response and its send is typed by it (D3), returning the `Response<T, E>` envelope (D7). `send_raw` is the checked escape hatch (D45).

6. **Generated documentation lies.** 478 doc code fences, all ` ```text `, zero compiled. The flagship quick-start in three places calls `client.list_models()` — a method that does not exist (convenience methods are only generated for *non-JSON* endpoints; `gen/src/output/assemble/lib_rs.rs:246-254` vs `codegen/client/helpers.rs:239`). The same block emits an `AuthStrategy::ApiKey` literal missing its required `value_prefix` field. 283 stray `///;` lines. Combined modules (emqx) lose nearly all docs.
7. **No feature flags.** Every consumer compiles all 18 REST clients + 5 WS modules + reqwest + tokio + tungstenite (`gen/src/cargo_gen.rs:14-41`). Four crates depend on this.

> **Update, 2026-10-11:** Per-API feature flags in the generated crate are landing 4 (D18).

8. **`Schema::module_path` is dead on the REST path** — documented as used (`define/src/schema.rs:98-112`), never read; and a non-identifier `type_name` on a *request* (`ApiRequest::json_type("Vec<Foo>")`) panics the generator via `format_ident!` while the response path handles it via `syn`. `validation.rs` never checks `schema.type_name`.

**Failure-mode summary:** identifier validation for API/endpoint/param names is loud and good; type-name errors are loud-but-late (rustc errors in a workspace-excluded crate whose compile test is `#[ignore]`d); body drops, param drops, and auth drops are fully silent.

---

## 7. Fidelity: Definition → OpenAPI and Postman

### 7.1 OpenAPI export

> **Stale-text note, 2026-10-09.** The first bullet (`Endpoint.params` dropped, 0 query parameters) is fixed: operation parameters now export (see the status update before the defect ledger in section 10). The other bullets were not re-verified.

Component schemas are **real JSON Schema**, not stubs — the strongest part of the export. But:

- **`Endpoint.params` is dropped entirely** — `map_operation` hardcodes `parameters: vec![]` (`define/src/openapi/export/paths.rs:83`). Verified across all 15 artifacts: **0 query parameters anywhere**, while the Postman exporter reads the same data and emits them. 52 endpoints with params, 27 with pagination, invisible to every OpenAPI consumer. A GitHub client generated from `openapi/github.json` cannot paginate.

> **Update, 2026-10-11:** "27 with pagination" is a stale July figure. The pagination inventory counts 35 endpoints with declared pagination, all page-number: gitea 8, github 9, gitlab 9, bitbucket 9 (D49).

> **Correction, 2026-10-11:** 35 declared pagination endpoints is right. The raw `with_pagination` count of 27 missed the Bitbucket `bitbucket_pagination()` helper, which wraps the same call.
- **~30% of schema properties export as untyped `{}`.** The schemars→openapiv3 bridge (`definitions/src/registry/conversion.rs:86-204`) handles no `enum`, no `oneOf/anyOf/allOf`, no `additionalProperties`, and — critically — no type arrays, so every `Option<T>` (`{"type": ["string","null"]}`) and every Rust enum falls to `SchemaKind::Any`. Measured: bitbucket 109/216 properties untyped, huggingface 99/227, elevenlabs 73/233. Every string-enum value domain in the catalog is absent from the published contracts.
- **Required headers never become header parameters.** Anthropic's mandatory `anthropic-version` lives only in `x-schematic`; a generically-generated client gets HTTP 400.
- **One status code per operation, no error models, one security scheme max**, `value_prefix` discarded, unsupported OAuth grants silently emit *no* scheme (the document then claims the API is unauthenticated), per-endpoint `oauth_scopes` never become operation `security`.
- **Spec violation shipped:** `openapi/ollama.json` has duplicate `operationId`s (`Embeddings`, `ListModels` each on two paths) — the grouped merge dedupes on `(path, method)` but never checks ids (`gen/src/export/openapi.rs:215-237`).
- **Nested-type `$ref`s can dangle**: registry emission is register-list-driven, but schemars parents `$ref` nested types that were never registered (`anthropic::Usage`, `elevenlabs::SampleModel`…). `validate_completeness` structurally cannot see this — it checks top-level names only.

> **Update, 2026-10-11:** The schemars bridge fixes are landing 4 (D18). Required constant headers as OpenAPI parameters are also landing 4 (D46), and error responses and per-content-type responses come from the response vocabulary of D6 and D38. Same-address endpoints that differ by a body field merge into one operation listing both content types (D36).

### 7.2 The x-schematic round-trip is fiction

The extension structs are emitted at document and operation level and **read by nothing** — there are zero `x-schematic` references anywhere under `define/src/openapi/import/`. The importer hardcodes `env_mapping: None, headers: vec![], version: None, …` (`import/builder.rs:339-354`). Three files claim round-trip fidelity (`extensions.rs:4-5`, `export/mod.rs:48-49`, `docs/io/openapi-extensions.md:3`); no round-trip test exists. Two of the four extension structs (`SchematicSchemaExtension`, `AuthExtension`) are never constructed in production at all, and `AuthExtension`'s rustdoc claims an embedding that doesn't exist. Additional import losses: path-item-level parameters silently dropped (a very common OpenAPI idiom in foreign specs), multipart bodies dropped to `request: None`, inline schemas become the literal type name `"serde_json::Value"` — which then makes `import → export` **error out** on the ref-closure check for most hand-written specs. `strict` mode fails on INFO diagnostics and is unusable on realistic input.

### 7.3 Postman

More faithful than OpenAPI on params and headers, with careful auth-variable and mixed-auth-grouping work. But: **every JSON body is the literal string `"{}"`** (the schema registry is never passed in; `postman/examples.rs` is a 5-line placeholder), no test scripts, no saved example responses, no environment files, OAuth2 collapses to `bearer`, `Text`/`Binary` bodies lose content types, and header/cookie params are ignored. Collections are browsable, not runnable.

> **Update, 2026-10-11:** Reading `x-schematic` on import (or striking the round-trip claim) and Postman example bodies are landing 4 (D18). Header and cookie parameters now reach the generated Rust client (D10, D33), which makes it a gap in the Postman and OpenAPI outputs only.

### 7.4 Pipeline hazards

- **`generate --api <one>` against the real output directory deletes the other seventeen clients.** Cleanup builds its expected-file set from only the requested API, then removes every generator-owned file not in it — empirically verified (24 files/dirs → 10). `lib.rs` is rewritten consistently, so it even compiles afterward and nothing screams (`gen/src/output/mod.rs:257-297,482-506`; `gen/README.md:38` documents this invocation as normal usage).
  *Note, 2026-10-10:* the required behavior is now decided as D9 (section 15); the defect itself is not yet fixed.
- A stale, git-tracked duplicate output tree exists at `gen/schematic/schema/` — residue of a wrong-cwd run, easy to mistake for real output.

> **Correction, 2026-10-11:** `schematic/gen/schematic/` still exists with 5 tracked files, and no Cargo path references it.
- Two divergent module-path resolvers (`gen/src/output/assemble/helpers.rs:33-37` applies a suffix heuristic; `definitions/src/lib.rs:248-252` only lowercases) — an API named `PagerDutyApi` without explicit `module_path` would land in `pagerduty.rs` but `pagerdutyapi.json`.

---

## 8. WebSocket Assessment

The five WS definitions are **five dialects**: identical auth handshakes expressed as prose in one module and as `AuthFlowHints` in its sibling; `integration_ws` marks all six messages `Bidirectional` not because they are but to trip a generator heuristic (with a test asserting the contortion); ElevenLabs' WS types don't use `schematic_define` at all. `CorrelationHints` and `HeartbeatHints` have zero users; the generator instead string-matches messages literally named `"RequestEnvelope"` (`gen/src/ws_codegen/plan.rs:188-206`), so Samsung's declared `RequestIdType::String` is dead.

> **Correction, 2026-10-11:** ElevenLabs' WebSocket definition does use `schematic_define::WebSocketApi` (`definitions/src/elevenlabs/mod.rs:189`); only its payload types do not. Homelab WebSocket client consumers: `samsung_tv.rs` (Samsung WS), `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, and `unfolded-integration-helper` (`unfolded_circle_integration_ws`). `homelab/cli` and `homelab/server` have none.

Generated WS clients have a genuinely good transport core (supervisor loop, correlation, backoff, even a host side) but: the typed message lists are computed and **no emitter reads them** — the surface is `send(serde_json::Value)` / `next_event() -> Option<Result<Value, _>>`; **WS auth is a no-op** — `Headers::build()` never consults the emitted `EnvMapping`, so no generated WS client ever sends an env-derived credential, while the Dock module's docs advertise `UCR_DOCK_TOKEN`; the reconnect loop's attempt counter never resets after success (supervisor dies after N *lifetime* reconnects) and lifecycle `BOS` is never re-sent, so a "reconnected" stream is protocol-dead while reporting `Ready`; `receive_timeout` is accepted and ignored; WS plan diagnostics — including errors — are discarded wholesale, so a failing definition produces no file and no message.

Externally, **WebSocket definitions are invisible**: no AsyncAPI export exists, and the AsyncAPI *importer* builds a full `WebSocketApi` + catalog and then discards both, writing only a count summary. Five WS APIs, zero artifacts.

Two concrete defects worth fixing regardless of any redesign: the shipped `ws://remote.local/ws/ws` URL (base_url ends in the endpoint path; `definitions/src/unfolded_circle/core_ws/mod.rs:182,144` → `schema/src/unfolded_circle_core_ws.rs:106` — found independently by two reviewers), and a dormant guaranteed `E0308` in the Bearer host template (trailing `;` making an `else` arm yield `()` against a `Result` tail, `gen/src/ws_codegen/shared.rs:526-534`) that survives because `validate_code` is grammar-only and its unit test asserts substrings.

> **Update, 2026-10-11:** WebSocket fixes and the typed message enums are landing 3 (D8, D46), and WebSocket reuses the REST parameter, auth-policy, env-mapping, and header types. An AsyncAPI exporter is in scope in landing 4 (D47), so the "no external artifact" statement is expected to stop holding after that landing.

---

## 9. How the Ergonomics Should Improve (ranked)

> **Reader's note, 2026-10-09.** This section is the July ranking and is kept as written. Under decision D1 every recommendation below is in scope; the ranking is not a sequence (landing order is decided in D5; see section 13a). Recommendations 2 and 12 were refined by decisions D2 and D3 in section 15, and recommendation 3's "unsupported text/binary" list is as stated in section 13. Recommendations 5 and 6 are superseded in shape by D6, and recommendation 13 by D8. Two sentences here are the review's own proposals, not author decisions: recommendation 1's "do not add a public `Default`" and recommendation 2's "do not enforce equality" with `schema_name`/`type_name`; both are listed for ratification in section 15.
>
> **Update, 2026-10-10.** Recommendation 1's "do not add a public `Default`" is now decided (D15); the second half of that proposal and recommendation 2's "do not enforce equality" are subsumed by D2 and no longer stand as separate proposals. Recommendation 3's "error until implemented" for text and binary bodies, `ApiKeyParam`, and header and cookie parameters is replaced by implementing them (D10); the per-output validation in that recommendation remains as a backstop. Recommendation 14's "consume `PaginationStyle`'s payload or shrink it" is answered by D14, which replaces `PaginationStyle` with the model in the pagination spike.
>
> **Stale-text note.** Recommendation 7 is done for operation parameters (required constant headers remain open). Recommendation 10 is partly stale: it points to the grouped-module contract in section 13, which is still a proposal, and the destructive `--api <one>` cleanup (defect 2) is not fixed. Recommendation 3 already notes that form and URL-encoded bodies are supported.
>
> **Update, 2026-10-10 (D18, D19, D17).** The landing of several recommendations is now decided. Recommendations 8 (schemars bridge) and 9 (`x-schematic` round-trip), the feature-flag part of 10, and the README-doctest and vendor-pagination-move parts of 14 go to the fourth landing (D18). Recommendation 7's remaining item (required constant headers as OpenAPI parameters) is not named by D18 and stays unassigned. Recommendation 11 (one generated list of APIs, replacing seven) was not assigned by the author and remains an open assignment. Recommendation 14's "consume `PaginationStyle`'s payload or shrink it" is answered by D14, and D19 moves that work (primitives and generated pagers) into landing 1; the vendor-constructor move still waits for landing 4. Recommendation 14's other items follow D18: `env_auth[0]` retirement and `deprecated` in landing 2, `ConnectionParam`/`ParamType` collapse with the WebSocket work (D8). Recommendations 1 and 2 (builders, typed references) stay in landing 2. D17 adds no recommendation to this list; it narrows the GraphQL scope that D16 opened.

> **Update, 2026-10-11:** Recommendation 2's open naming rules are closed by D44. Recommendation 7's remaining item and recommendation 11 are assigned to landing 4 (D46). Recommendations 5 and 6 are refined by D36 (streaming and non-streaming endpoints on one address), D37 (declared stream end), and D38 (default error list with most-specific-wins mapping); recommendation 12's generic `request<T>` is replaced by typed send and `send_raw` (D3, D45).

The directory name says "macro review," so the headline question first: **should definition authoring get a macro?** Recommendation: **builders first, macro optional on top.** A `define_api!` proc-macro could collapse the ceremony, but it would trade struct-literal debuggability and IDE support for a bespoke syntax, and the deeper problems (string wiring, silent drops) are not macro-shaped. Builders + typed references fix the same ceremony additively, keep rustc as the error surface, and leave room for a thin declarative macro later if wanted. A `macro_rules!` sugar layer over builders is cheap once builders exist.

1. **Builders/constructors for `Endpoint` and `RestApi`** (`Endpoint::get("GetRepository", "/repos/{owner}/{repo}").returns_json::<RepositoryInfo>()`…), `impl Into<String>` everywhere, required fields supplied to constructors; optional fields defaulted by builders. Do not add a public `Default` that presents an empty API or endpoint as usable. Collapses the median literal from 11 lines to 2–4, deletes ~1,000 `.to_string()` calls and all 253/268/201 `None`/`vec![]` lines, and — because callers stop writing literals — reduces the number of callers affected by future field additions. Adding builders is additive; adding fields to these public structs still breaks existing literals inside crates that can construct them. Migrate those callers with any later field change.
2. **Bind type references to types.** A typed authoring method must connect a Rust type, its path in the generated client, and its exported schema in one operation. `JsonSchema::schema_name()` names a schema component; it does not necessarily name an accessible Rust type, especially for aliases and containers. `std::any::type_name::<T>()` is diagnostic text, not a stable serialization or code-generation contract. Do not enforce equality with either. Registration ownership is decided (D2, section 15: an explicit definition context); the naming rules remain open there.
3. **Turn silent drops into hard `validate_api` errors** until implemented: unsupported text/binary request bodies (form and URL-encoded bodies are now supported), `ApiKeyParam` (§6.2), invalid Rust type syntax or module paths, unsupported header/cookie parameter handling, duplicate endpoint ids, duplicate `(method, path)`, `?` in path templates, ambiguous base-URL/path composition. Valid paths such as `serde_json::Value` and types such as `Vec<Model>` must remain accepted. Validate independently for each requested output target before writing; OpenAPI support must not be rejected merely because the Rust client lacks a feature.
4. **Verify freshly generated code compiles.** `just lint-schema` already checks the committed generated crate. A separate check must generate into an isolated temporary crate and compile it without changing the checkout or making live provider requests. Keep compile-only checks in L1, using the `slow_` prefix when required by the testing guide; see section 14. Removing `#[ignore]` is an implementation choice, not the success criterion.
5. **Model streaming** — `ApiResponse::Stream { framing: Sse | Ndjson, item: Schema }` — and give `Binary`/`Text` responses the `content_type` their request twins already have. Un-strands ten Ollama types, fixes the buffering `stream_speech`, stops the exporter misreporting audio endpoints.
6. **Make `ApiResponse` a status→schema map** (or add `errors: Vec<(u16, Schema)>` as the v1.5 compromise). Unlocks typed errors everywhere: generated clients, OpenAPI error responses, Postman examples.
7. **Emit `Endpoint.params` and required constant headers as real OpenAPI parameters** (`export/paths.rs:83`) — declared operation parameters now export through `map_parameters`. Required constant headers remain a separate task; preserve endpoint-over-API precedence and represent a fixed header value using a single-value enum. Parameter emission alone does not establish that every exported schema is usable by third-party generators.
8. **Fix the schemars→openapiv3 bridge** (type arrays → `nullable`, `enum`, `oneOf/anyOf/allOf`, `additionalProperties`), plus focused tests that distinguish deliberate arbitrary JSON from accidental loss of type constraints. `serde_json::Value`, unconstrained foreign schemas, and explicit free-form objects are valid; a blanket ban on `SchemaKind::Any` would reject supported inputs. Recovers the ~30% untyped-property hole.
9. **Read `x-schematic` on import or strike the round-trip claim**; test semantic preservation for the supported subset rather than byte equality: ordering and normalization may change. Document extension version handling, standard-field precedence, and diagnostics for unsupported extension data before promising a complete round-trip. Delete or wire the two dead extension structs.
10. **Fix `--api <one>` destructive cleanup** using the grouped-module contract in section 13, delete the stale `gen/schematic/` tree, unify the two module-path resolvers, feature-gate the generated crate per API.
11. **Generate the registry and API lists** from one source (build script or macro over `apis_by_module()`), collapsing the seven drifted parallel name lists to one.
12. **Generated-docs honesty pass:** emit ` ```no_run ` instead of ` ```text `, generate typed convenience methods for JSON endpoints and explicitly address the existing generic `request<T>` entry point (convenience methods alone do not prevent callers from requesting the wrong type; D3 in section 15 removes it from the public surface), fix the `ApiKey` doc literal, drop `///;`.
13. **WS follow-through:** emit the already-computed typed message enums, resolve env auth at connect time (or fail loudly when `env_auth` is declared), replace the `"RequestEnvelope"` string-match with an explicit `responds_with` link on `MessageSchema`, fix the reconnect counter + lifecycle replay, surface WS diagnostics, fix the dormant Bearer-template `E0308`.
14. **`schematic-define` hygiene:** `#![doc = include_str!("../README.md")]` so the ~7 currently-broken README examples become compiling doctests; move vendor pagination constructors to `schematic-definitions`; collapse `ConnectionParam`/`ParamType` into `ParamDef`/`QueryParamType`; either consume `PaginationStyle`'s payload (a real `fetch_all_pages` needs exactly those fields) or shrink it; retire `env_auth[0]`-as-password by making `Basic` carry its env shape; add `deprecated` to `Endpoint`/`ParamDef`.

---

## 10. Defect Ledger

Concrete bugs found during this review, independent of any design opinion:

> **Status update (OpenAI import work; source checked 2026-10-09).** Form and URL-encoded bodies are fixed; text/binary bodies remain unsupported. Valid Rust type paths no longer trigger the reported identifier panic, but malformed type syntax still needs validation. The inline-schema export and clippy findings were previously recorded as fixed. The generated-code compilation concern is partly addressed: the generated crate is now
> compiled and clippy-clean as part of `just lint-schema`. Other July findings below are not all re-verified by this inline review; an empty status cell means unresolved in this ledger, not confirmed present today.
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
| 1 | Non-JSON request bodies silently dropped; 6 endpoints POST empty bodies | `gen/src/codegen/request_structs/body.rs:24-27`, `into_parts.rs:20-28` | **PARTLY FIXED** — `RequestParts` carries `shared::RequestBody`; form and URL-encoded bodies are emitted, but text/binary request fields remain absent |
| 2 | `generate --api <one>` deletes all other generated clients | `gen/src/output/mod.rs:257-297,482-506` | |
| 3 | Shipped wrong URL `ws://remote.local/ws/ws` | `definitions/src/unfolded_circle/core_ws/mod.rs:182` → `schema/src/unfolded_circle_core_ws.rs:106` | |
| 4 | Generator panic on non-identifier request `type_name` / `module_path` | `gen/src/codegen/request_structs/body.rs:16`, `output/assemble/lib_rs.rs:29` | **FIXED** — all type-name sites route through `type_name_to_tokens` (syn-based) |
| 5 | `ApiKeyParam` clients never send the key; env fallback silently dropped | `gen/src/codegen/client/helpers.rs:128-217`, `define/src/types.rs:334` | |
| 6 | Duplicate `operationId`s in `openapi/ollama.json` (OpenAPI 3.0 violation) | `gen/src/export/openapi.rs:215-237` | |
| 7 | Nested reference completeness must be checked beyond endpoint names; export closure validation already rejects unresolved references, so this is not evidence of currently published dangling references | `definitions/src/registry/mod.rs:244-257` (can't see them) | |
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

> **Correction, 2026-10-11:** the row 15 path is `schematic/gen/tests/l1/openapi_strict_completeness.rs:176-193` (16 names).

> **Update, 2026-10-11:** Dispositions decided after the ledger was written: rows 1 and 5 (text and binary bodies, `ApiKeyParam`) are implemented in landing 1 (D10, D33); row 2 by whole-module-group regeneration (D9); rows 3 and 9 to 12 by the WebSocket landing (D8, D46); rows 13, 14, and 19 in the fidelity landing 4 (row 16, the isolated compile check, is safety work under D1) (D18); row 15 by the single generated API list in landing 4 (D46). D20 requires every row closed or recorded as an accepted gap.

> **Update, 2026-10-11 (D49):** the remaining loose rows are assigned: row 16 (isolated compile check) to landing 1, row 22 (module-path resolvers) to landing 4, and row 8 (generated docs) to landing 4. D20 requires each row closed or recorded as an accepted gap, which is why each needed a landing.

---

## 11. Summary of the July Investigation

The user-facing questions, answered directly:

- **What works well?** The pipeline mechanics (determinism, atomic writes, formatting), the auth model's layering, the JSON REST happy path end to end, form-field vocabulary, import name sanitization, Postman auth handling, WS transport runtime.
- **What is awkward?** Everything about volume: 11 lines per endpoint of unaided struct literal, string-wired types with string-vs-string validation, seven parallel API lists, dual legacy/modern auth fields, vendor names in the neutral crate, and a model whose gaps (streaming, errors, optional auth, enums) get patched as comments, prose, and deliberate wrong-paths in the definitions.

> **Correction, 2026-10-11:** the "seven parallel API lists" are six runtime lists plus a test table; see the correction in section 4.3. The 11-lines-per-endpoint figure rests on the stale 253 count.
- **What's not supported?** Streaming, typed errors, per-status shapes, response content types, typed path params, per-endpoint auth override, deprecation, rate limits — all additively fixable. Polymorphic bodies, status-discriminated responses, serializable definitions, and WS/REST unification require vocabulary v2.
- **How well does generated code represent the definition?** Rust: high fidelity for JSON+auth+headers+paths, silent zero fidelity for form bodies, param metadata, pagination structure, and `ApiKeyParam`. OpenAPI: real schemas but no parameters, no enums for ~30% of properties, no errors — browsable, not generatable. Postman: parameter-faithful but body-empty. WebSocket: no external representation at all.
- **Highest-leverage moves:** builders + typed schema references (ergonomics), silent-drop → validation-error conversion plus un-ignoring the compile test (safety), `Endpoint.params` emission + the schemars bridge fix (OpenAPI credibility), and the `--api <one>` cleanup fix (operational safety).

> **Update, 2026-10-11:** The first two moves are now decided: builders (D15) and typed registration (D2, D44) are landing 2, and the silent-drop fixes are implemented rather than turned into errors (D10, D33). Streaming, typed errors, per-status shapes, and content types are covered by the response vocabulary (D6); typed path parameters, auth override, deprecation, and retry are landing 2 (D18). The remaining items (polymorphic bodies, serializable definitions) are in scope too (D1, D18), so "require vocabulary v2" no longer means deferred.

---

## 12. Addendum (2026-10-09): runtime gaps found while comparing sniff's two provider stacks

This comparison informs `2026-10-09-consistent-use-schematic`. Sniff’s hand-written [FocusedProviderClient](../../../sniff/lib/src/remote/focused.rs) supplies consumer behavior that generated `schematic-schema` clients do not fully expose. The table distinguishes missing shared capabilities, existing configuration, and consumer-owned behavior; they are not all generator defects. Historical file:line references should be located by symbol when lines have moved.

| # | Gap | Evidence |
|---|---|---|
| A1 | **Errors discard response headers.** `request` returns `Err` before the caller sees headers on a non-2xx status, and `ApiError` carries only `status` and `body`, so callers lose evidence needed to help distinguish rate limiting from permission failures, and `Retry-After` is lost. Status and headers may still be ambiguous; do not promise a definitive classification for every 403. | `schema/src/shared.rs` (`SchematicError`), `schema/src/github/client.rs:263-270` |
| A2 | **Consumer-owned 404 meaning.** Schematic returns a status-bearing `ApiError`; sniff’s GitHub adapter maps 404 to "Not found". Callers can already match status 404. A convenience policy may improve ergonomics, but no transport redesign is needed to distinguish absence from failure. | `sniff/lib/src/remote/github.rs:203-250` |
| A3 | **No credential evidence.** The client resolves env variables internally and does not report which variable (or anonymous) each request was sent with; consumers also cannot tell a host-bound token from a provider token. | `env_auth` / `resolve_request_headers` |
| A4 | **Credential order is configurable today.** The generated variant builder accepts `env_auth` and rebuilds the environment mapping when it changes. The remaining requirement is preserving caller-selected order together with credential-source evidence and scope. Explicit `headers_builder` overrides take precedence; test that combination rather than assuming the environment list wins. | [variant builder in schematic-gen](../../gen/src/codegen/api_struct/variant.rs) |
| A5 | **Anonymous retry in sniff adapters.** The sniff adapters build an `unauthenticated_client()` fallback, so a rejected token can be retried anonymously and the evidence becomes ambiguous. | `sniff/lib/src/remote/github.rs:82-92` |
| A6 | **Redirect configuration requires a custom client today.** Default reqwest redirects apply; the only escape is injecting a whole `reqwest::Client` via `with_client`. | `schema/src/github/mod.rs:83-86,181` |
| A7 | **No exact-host policy.** Nothing checks that an endpoint host equals the remote's host before a credential is attached. Verified 2026-10-09: no such check exists in the generated runtime. | `schema/src/shared.rs`, generated `client.rs` files |
| A8 | **No shared deadline.** The default client has a fixed 30 s per-request timeout; one operation spanning several pages cannot share one budget. | `schema/src/github/mod.rs:83-86` |
| A9 | **Some branch filters are missing.** GitHub’s generated PR list already has `page`, `per_page`, `state`, `sort`, and `direction`; it lacks `head`. Inventory GitLab, Gitea, and Bitbucket separately before adding source-branch filters, since provider wire names and capabilities differ. | `schema/src/github/requests.rs:281-296`, `gitlab/requests.rs:452-460` |
| A10 | **No GraphQL surface.** Nothing in `schematic/` references GraphQL. A GraphQL operation is expressible as a `POST` with a JSON body (document plus variables). Caveat: GitHub Enterprise REST lives under `/api/v3` and GraphQL under `/api/graphql`, so the bases differ and one definition needs a runtime base override (see Open Questions). | recursive search of `schematic/` |

Preserving status, response headers, and the raw body must happen before any typed decoding, and it must hold in the success case too (success also exposes headers), because pagination needs them. Typed error bodies are in scope (D1, D6), but decision D7 makes the result envelope carry this evidence alongside any typed body.

Related fact for D3: `EndpointSpec` in the `schematic-schema` shared runtime already has an associated `Response` type, with 550 generated impls, so D3's typed send is cheaper than earlier text implied: the type-to-endpoint link exists; what is missing is a send method that uses it. Complete-list handling needs pagination behavior beyond the existing marker trait. Shared request policy belongs in Schematic; provider classification, discovery, and explicit anonymous retry decisions belong in consumers.

> **Dated note, 2026-10-10.** Gaps A3-A5 are answered by D11 (credential evidence and order, single attempt, no anonymous retry), A7 by D12 (exact-origin policy), A8 by D13 (shared deadline), and A10 by D16 (the author chose full GraphQL support, which is wider than the "expressible as a JSON POST" reading above). A9 (branch filters) is untouched. A1-A2 and A6 are covered by D7 and D4 as before. D17 narrows D16 (GraphQL operations are strings; Schematic does not validate them), and D21 fixes the envelope metadata that A1 and A3 asked for.

> **Dated note, 2026-10-11.** A6 (redirect configuration) and A7 (exact-host policy) are now governed by D28: the cross-origin policy is authored in the definition, covers redirects and followed next-page links, and defaults to `same_origin`. It replaces the caller-configured shape in D4 and D12.

> **Dated note, 2026-10-11 (second round).** D31 removes `with_client` injection entirely, so A6 ("the only escape is injecting a whole `reqwest::Client`") is answered differently from D28's first reading: there is no injected client, and the Schematic-managed builder covers proxy, TLS roots, timeouts, user agent, default headers, a shareable pool, and a base URL for tests.

---

## 13. Implementation Boundaries and Decided Contracts

This document is a review with recommendations. **Reader's note, 2026-10-09:** the author decided (D1, section 15) that the first implementation takes the largest cut and defers nothing. In scope are: safety fixes (destructive generation, silent data loss, validation errors, isolated compile check, duplicate `operationId`); the runtime features of this section; typed API wiring (D2, D3); endpoint and API builders; streaming responses and response content types; status-to-schema responses and typed error bodies; WebSocket fixes and redesign work; and OpenAPI/Postman fidelity. An earlier draft of this section called builders, streaming, richer responses, and WebSocket redesign "separate changes"; that is superseded. Preserving HTTP error evidence and configuring credentials still do not technically require streaming, richer response schemas, or WebSocket work, so they are not prerequisites of those items, but all are now in scope. Packaging and ordering are decided by D5 (three landings, section 13a); the whole-work definition of done is open. *Note, 2026-10-10:* D18 adds a fourth landing, D19 moves pagination into landing 1, and D20 defines the whole-work definition of done (section 15). Open questions in section 15 must be settled before the implementation they affect.

> **Status of the contracts below (updated 2026-10-10).** D1-D16 (section 15) are confirmed. On 2026-10-10 the author ratified or replaced every block that was labelled "Proposed (pending ratification)" except one: the complete-list rule under "Complete pagination is an explicit promise", which stays **Proposed**. Each block below says which decision covers it. Where a block was replaced, the replaced text is removed and the decision holds the current wording.

> **Update, 2026-10-11 (second round).** The one remaining "Proposed" block, the complete-list rule, is ratified by D41. No block in this section is still pending ratification.

### Partial generation preserves the rest of the client crate

**Ratified by D9 (2026-10-10).** Generating one API into an existing client crate updates its entire resolved module group, preserves unrelated modules, and regenerates the crate’s shared files and exports from the retained API inventory. For example, selecting `OllamaNative` must retain `OllamaOpenAI`, since both share a module. Resolve the selection and validate the resulting inventory before writing. An incompatible shared-runtime change requires full generation or an actionable error before any mutation; preserving files while removing their exports does not satisfy this contract.

The [output writer](../../gen/src/output/mod.rs) in `schematic-gen` must limit stale-file deletion to successfully regenerated groups and retain its existing proof-of-ownership checks. Full generation may remove obsolete owned modules. Unowned files remain protected. If the output inventory cannot be established safely, require an isolated output directory or full generation. Atomic writes protect individual files; they do not make a multi-file update transactional, so failures must be reported without claiming the whole crate was updated successfully.

### Validation reflects each output’s capabilities

**Ratified in part and narrowed by D10 (2026-10-10).** D10 replaces "error until implemented" with implementing the request-side features the supported APIs can use (text and binary bodies, `ApiKeyParam`, header and cookie parameters). What remains of this contract is the backstop: a declared feature must either reach the selected output or produce a diagnostic identifying the API, endpoint, feature, and unsupported target. Validate the whole selected batch for each output before writing it, and one output's gap never blocks another output. Invalid Rust syntax is an error; valid generic types and qualified paths remain supported. Wire parameter names are preserved while generated Rust identifiers are sanitized and checked for collisions. Repeated base/path segments alone are not proof of a bad URL; validate the declared composition rule rather than rejecting a legitimate `/api/api` path by substring.

**Reader’s note:** this replaces blanket rejection of every non-JSON request body and every non-identifier type name. Those recommendations would now reject working forms and imported `serde_json::Value` types. *Superseded 2026-10-10:* the earlier sentence "unsupported text/binary bodies receive explicit errors until implemented" no longer stands; D10 implements them in the generated Rust client, and the error remains only for an output that still lacks a feature.

### HTTP evidence and request policy do not depend on schema redesign

The [SchematicError generator](../../gen/src/codegen/error.rs) in `schematic-gen` defines errors shared by `schematic-schema` clients.

**HTTP evidence (decided by D7, section 15).** Status, response headers, and the raw body are preserved before any typed decoding is attempted. A malformed, undeclared, or unknown error body must not replace that evidence with only a JSON parsing error; the raw body stays in the envelope's error. Success also exposes response headers, which pagination needs. Every send returns the `Response<T, E>` envelope defined in D7, so the metadata is present in both the success and the error case. Only a failure with no HTTP response at all (DNS, connection refused, timeout, credential-host check rejection, redirect denial) is the outer `Err(TransportError)`, and that `Err` still carries the request-side metadata. Downstream error matches change with this shape: sniff has exhaustive error matches that will require edits. Redaction rules and the exact metadata field list are open (D7). *Update, 2026-10-10:* D21 fixes the metadata field list and the redaction rules; a few details stay open (D21).

*Update, 2026-10-11 (D22-D26).* The rule for where a failure lives is now: **an envelope exists whenever an HTTP response head was received; the outer `TransportError` means no response head arrived.** Failures before anything is sent (missing credential, page size above the maximum, origin rejected, invalid argument) are `TransportError` kinds that still carry request metadata (D24). A 2xx reply that cannot be decoded is a failure inside the envelope with its real status and kind `MalformedSuccess` (D25); a body cut off after the headers is a failure inside the envelope with kind `BodyInterrupted` and the partial bytes (D26). The credential-free promise covers stored metadata and `Display` only; a decoded error body prints as received in `Debug` (D22). Network-error text keeps the URL, with secret query values shown as `[redacted]` (D23). The list above that puts "credential-host check rejection, redirect denial" among the no-response cases is refined by D24 and D28 (see the host boundary note below).

*Update, 2026-10-11 (second round: D32, D39, D48).* One test places every refusal: **did this call receive an HTTP response?** (D32). Where the list above names "redirect denial" as a no-response case, it is superseded: a redirect refused by the policy did receive a 3xx head, so it is a failure inside the envelope. The "SchematicError generator in `schematic-gen`" named above is superseded for the shared runtime types: the envelope, `TransportError`, redirect and cross-origin logic, deadline, pager, and redaction live in a new hand-written runtime package (D39). For an endpoint with no declared error body, error replies arrive as kind `Undeclared` with status, headers, and raw body (D48).

**Credential-source metadata and credential order: ratified and extended by D11 (2026-10-10).** The auth method reported in the envelope is per-call, client-level, an environment variable's name, or anonymous; never a value. Credentials are resolved once per request. Schematic never retries a rejected credential, anonymously or otherwise. The precedence rules and the single-attempt rule are in D11.

**Credential-host boundary: ratified by D12 (2026-10-10).** Scheme, host, and effective port must equal the configured base origin or a declared additional exact origin, for the first request and every followed link. Redirect enforcement uses D4: policy-enabled requests run on a Schematic-managed transport with redirects disabled, and a redirect is an explicit error. Checking only the initial URL would not enforce a boundary on redirects. Domain semantics such as “this provider’s 404 means absent” remain in consumers.

*Amended by D28, 2026-10-11.* The boundary is now authored in the API definition as a cross-origin policy with five values (`same_origin`, `allow_origins`, `allow_without_credentials`, `allow_any`, `deny`), defaulting to `same_origin`, and it governs redirects as well as followed next-page links. The exact scheme+host+port comparison and the credential-attachment rule above survive. The sentence about D4 ("redirects disabled, and a redirect is an explicit error") is superseded: redirects are disabled in the HTTP client and followed by Schematic according to the policy. Callers may tighten the policy, never loosen it.

*Amended again, 2026-10-11 (D31, D32).* D31 removes client injection, so the policy cannot be bypassed by a caller-built client, and the text above about the "existing client-injection API" no longer applies. D32 sets where a refusal lands: a redirect the policy refuses (a 3xx head was received) is a failure inside the envelope, kind `RedirectRefused`, with the redirect status and `Location` visible; a next-page link the policy refuses before any request for it starts is the outer `TransportError`, kind `CrossOriginRejected`.

**Shared monotonic deadline: ratified by D13 (2026-10-10).** Opt-in, one deadline per operation across all pages and follow-up requests; the existing per-request default is unchanged for callers who do not opt in.

### Complete pagination is an explicit promise

The pagination model itself (strategies, authoring, generated surface) is decided in D14. Only the auto-collecting "all items" helper below is still **Proposed (pending ratification)**; the author has not individually ratified it.

**Proposed (pending ratification):** a helper advertised as returning a complete list succeeds only after observing the provider’s terminal-page condition. A page bound, repeated continuation, invalid continuation, HTTP error, or expired deadline returns an explicit incomplete result carrying the reason and the items collected so far, never a successful partial list. Page limits are caller-visible. Provider-specific continuation decoding may differ, but every page goes through the shared client policy. Low-level single-page methods remain available. (That a next-page URL passes the same credential boundary as the first request is no longer part of this proposal; it is decided in D14 item 7 and D12.)


**Ratified by D41, 2026-10-11.** A `collect_all` helper exists. It succeeds only after the provider's last page is observed; otherwise it returns an explicit `Incomplete` error whose reason is one of `PageLimitReached`, `RepeatedContinuation`, `HttpError`, `DeadlineExceeded`, or `CrossOriginRejected`, carrying the items gathered so far. The page limit is caller-visible. The text above is kept as the proposal that was ratified; the reason list is D41's. One reason the proposal named, an invalid continuation, is not in D41's list (open under "Remaining planning-time details").

### GraphQL

**Replaced by D16 (2026-10-10).** An earlier draft here proposed that a JSON POST could carry a GraphQL document, that the GitHub public and Enterprise base URLs differ (REST under `/api/v3`, GraphQL under `/api/graphql`), and that batching, partial errors inside HTTP 200 responses, and provider interpretation belong in the consumer, with "no generic GraphQL compiler proposed". The last part is withdrawn: the author chose full GraphQL support (typed queries and GraphQL-aware error extraction). The URL difference remains a fact the design must handle. See D16 for the decision and its open questions.

*Narrowed by D17, 2026-10-10.* The author chose to author operations as strings in the definition, with typed variables and typed response data; Schematic does not parse or validate GraphQL. "Typed queries" in D16 therefore means typed variables and typed response data, not a query checked against a provider schema. The Enterprise versus public URL difference remains a design fact (D17, open details).

*Closed in part by D29, 2026-10-11.* Errors and partial data inside an HTTP 200 reply go on the envelope's failure side as `Declared(GraphQlFailure { errors, partial_data })`, with status 200 and the raw body kept.

*Closed by D34 and D35, 2026-10-11 (second round).* GraphQL lives inside `schematic-define` (an operation primitive) and `schematic-gen` (generated into the same client crate), reusing the envelope, credentials, cross-origin rule, deadline, and pagination (D34). The GraphQL address comes from a rule in the definition (GitHub.com: base plus `/graphql`; Enterprise base ending `/api/v3`: replace that ending with `/api/graphql`), and the caller may override it (D35). The GraphQL primitive and D29's error handling land in landing 2 (D30).


---

## 13a. Work Packages and Ordering

Decision D1 (section 15) puts a very large body of work in scope. This section groups it so each group can be discussed, estimated, and verified on its own. The packages (P1-P6) are a description. **The sequencing is decided by D5:** three landings, below. The whole-work definition of done and several work-item assignments are still open (see the end of this section).

> **Update, 2026-10-10.** D18 adds a fourth landing, D19 moves the pagination model into landing 1, and D20 defines the whole-work definition of done. The text below keeps the three-landing wording of D5; the amendments follow the landings table.

| Package | Contents | Source sections |
|---|---|---|
| P1. Safety | Partial-generation cleanup; validation errors for unsupported features; isolated compile check of fresh output; duplicate `operationId` fix; the other silent-drop and defect-ledger fixes that need no vocabulary change | 6.1-6.2, 7.4, 9 (3, 4, 10), 10, 13 |
| P2. Runtime evidence and policy | The `Response<T, E>` envelope (D7); typed send (D3); redirect-as-error transport (D4); error headers and body; success headers; credential-source metadata; exact-host credential boundary; shared deadline; honest complete-pagination; partial-generation contract (D9); request-side features (D10); credential precedence and single attempt (D11); host policy (D12); shared deadline (D13); pagination runtime (D14) | 12, 13, D3, D4, D7, D9-D14 |
| P3. Typed definition API and builders | Definition context with typed registration (D2); endpoint and API builders without public `Default` (D15); pagination definition model (D14); GraphQL authoring model (D16, placement open) | 2, 9 (1, 2), D2, D14-D16 |
| P4. Response model v2 | Success set plus error set (D6); response content types; streaming (SSE/NDJSON) | 5 (1-3), 5b (2), 9 (5, 6), D6 |
| P5. OpenAPI and Postman fidelity | schemars-to-openapiv3 bridge; `x-schematic` round-trip (or strike the claim); Postman example bodies; required headers as parameters; exporter and importer changes for P3/P4 | 7, 9 (7-9) |
| P6. WebSocket | Fixes (defects 3, 9-12); typed message emission; explicit link replacing the `RequestEnvelope` string match; reuse of REST parameter, auth-policy, env-mapping, and header types (D8) | 8, 9 (13, 14), D8 |

*Note, 2026-10-10.* P3's pagination definition model moves to landing 1 (D19). P3's GraphQL line is narrowed by D17 (string operations); its landing is still open. P5 splits: the exporter and importer parts that depend on P3/P4 stay in landing 2 (D5); the schemars bridge, `x-schematic` round-trip, and Postman example bodies go to landing 4 (D18). The vocabulary-changing items that no package named are assigned to landing 2 (D18, table below).

### Current landing assignments (after D30-D48, 2026-10-11)

This is the current assignment. The tables below it ("Decided landings (D5)", "Placement of D9-D16") and the dated notes under them are kept as history; where they say OPEN, this section holds the ruling.

| Landing | In plain terms | Contents now |
|---|---|---|
| 1 | Safety and shared runtime. Edits the 18 API definitions, not the 253 endpoint definitions | P1 safety. New hand-written runtime package (D39) holding the envelope, `TransportError`, redirect and cross-origin logic, shared deadline, pager, and redaction. Typed send and `send_raw` (D3, D45). Credentials (D11), partial generation (D9), shared deadline (D13). Managed HTTP builder; no injected client (D31). Redirect-following runtime and the API-level `cross_origin` field on the 18 API definitions (D28, D30). Request-side features, generator-only (D10, D33). Pagination runtime, automatic `Vec<T>` list recognition on GET endpoints, API default strategy, and the per-API exceptions table (D14, D19 as amended by D42, D43). `collect_all` (D41). Scoped `with_session` (D27). Secret marking and error-text redaction (D23, D40, D48). Generator work: query parameters separated from the path (S6). Isolated compile check of freshly generated code (D49). The `ResponseKind` runtime guard follows the response model (D49) |
| 2 | Definition vocabulary. Rewrites the 253 endpoint definitions once | D2 definition context with the D44 naming rules; D15 builders. Response model v2 (D6) with the selector (D36), stream end condition (D37), and error mapping (D38). Per-endpoint `cross_origin` override (D30). GraphQL primitive and D29 error handling (D34, D35). Each pagination exception moves from the API table onto its endpoint and the table is removed (D42). The vocabulary-changing items assigned by D18. The exporter and importer parts of P5 that depend on these. The decision on `Endpoint::oauth_scopes` and `RestApi::auth_policy`, wired in or removed, is made here (D49) |
| 3 | WebSocket | P6 (D8) |
| 4 | Fidelity and hygiene | The D18 list (schemars bridge, `x-schematic` round trip, Postman example bodies, README doctests, vendor pagination presets out of the neutral crate, per-API feature flags) plus the single generated API list and required constant headers as OpenAPI parameters (D46) and the AsyncAPI exporter (D47) ; the module-path resolver unification and the generated-docs honesty pass (D49) |

Order: landing 3 (WebSocket) before landing 4 (fidelity) (D46).

> **Update, 2026-10-11 (D52).** Landing 2 = specs 4a, 4b, and 5 (spec 4 is split). The row for landing 2 above is otherwise unchanged. D52 resolves parent recommendation 14 as "shrink = remove": the old pagination types are removed and the 35 `with_pagination` endpoints converted, in spec 4a.

| Decision | Landing (current) | State |
|---|---|---|
| D10 request-side features | 1 | Ruled by D33; generator-only, no definition edits |
| D12 / D28 cross-origin | Runtime and API-level field in 1; per-endpoint override in 2 | Ruled by D30; details are planning-time |
| D16 / D17 / D29 GraphQL | 2 | Ruled by D30; home is `schematic-define` plus `schematic-gen` (D34) |
| D19 pagination | 1 | Amended by D42: no new field on the 253 literals |
| D21 / D7 envelope details | 1 | Ruled by D48 |
| D23-D27 | 1 | Follow the envelope and pagination runtime |
| D31 managed builder, no injection | 1 | Ruled |
| D39 runtime package | 1 (this review's reading: the envelope, redirect runtime, and pager are all landing 1, and D39 puts them there) | Ruled; package name is a working name |
| D41 `collect_all` | 1 (with the pager) | Ruled |
| D36, D37, D38 response model details | 2 | Ruled; part of D6 |
| D44 typed registration naming | 2 | Ruled; part of D2 |
| D45 `send_raw` | 1 | Ruled |
| D46 order, single API list, constant headers | 3 then 4; list and headers in 4 | Ruled |
| D47 AsyncAPI exporter | 4, after WebSocket reuses the REST types | Ruled |
| D49 isolated compile check of fresh output (ledger row 16, rec. 4) | 1 | Ruled |
| D49 `ResponseKind` runtime guard | 1, follows D3, D6, D7 | Ruled; kept, changed, or removed as the model requires |
| D49 module-path resolver unification (ledger row 22, rec. 10) | 4 | Ruled |
| D49 generated-docs honesty pass (ledger row 8, rec. 12) | 4 | Ruled |
| D49 `oauth_scopes` and `auth_policy`: wired in or removed | Decided during 2; the choice is open | Landing ruled, choice open |

**Cost accounting after D42.** D19 had accepted a known cost: each of the 253 literals would be touched in landing 1 (a collection marker) and again in landing 2 (builders). D42 removes it: landing 1 touches the 18 API definitions only (D30 adds the `cross_origin` field; D42 adds the pagination default and exceptions table), so landing 2 is the single pass over the 253 endpoints. Landing 1 is still large (this review's reading; no size or timing expectation is stated by the author).

> **Correction, 2026-10-11:** the count of 253 endpoint literals, and every ratio derived from it, is stale. The verified count is about 519 (see the correction in section 2). Read "253" in this place as "about 519". The cost argument gets stronger, not weaker: D42 still removes the landing 1 touch, but the single landing 2 pass is about 519 literals, which D52 splits across specs 4a and 4b.

### Decided landings (D5)

| Landing | Contents | Why this grouping |
|---|---|---|
| 1 | P1 safety, plus P2 runtime evidence and policy (including D3 typed send, D4 transport, and the D7 envelope), plus D9, D10, D11, D12, D13, and the pagination runtime of D14 | Needs no new definition vocabulary; the envelope and typed send change every dependent call site once. |
| 2 | P3 typed definition API and builders (including the D15 builders), the pagination definition model of D14, the GraphQL authoring model of D16 unless ruled otherwise, P4 response model v2, and the exporter and importer parts of P5 that depend on them | One pass over the 253 endpoint definitions instead of two. |
| 3 | P6 WebSocket (D8) | Depends on the REST vocabulary settling first. |
| 4 (D18, added 2026-10-10) | Fidelity and hygiene that does not change the definition vocabulary: OpenAPI schemars-bridge fixes, `x-schematic` round-trip, Postman example bodies, README doctests, moving vendor pagination presets out of the neutral crate, and per-API feature flags in the generated crate | None of these needs a second pass over the 253 definitions, so they can follow landing 2. Whether this runs before or after landing 3 is **OPEN**; "3" and "4" are labels, not a ruled order. |

**Known cost of this packaging.** Landing 1 changes dependent crates once (the new envelope and typed send). Landing 2 changes them again, but only where response shapes change (D6). That second round is accepted as the price of landing 1 shipping without waiting for the vocabulary rewrite.

**Amendments, 2026-10-10.**

- **Landing 4 (D18).** Added after landing 2. The relative order of landing 3 (WebSocket) and landing 4 (fidelity) is open; this document calls them 3 and 4 unless the author rules otherwise.
- **Pagination in landing 1 (D19).** The pagination definition primitives and the generated pagers move from landing 2 to landing 1, so sniff gets real pagers immediately. The "Why this grouping" cell for landing 1 ("needs no new definition vocabulary") no longer holds in full: landing 1 now adds pagination primitives and one field (the collection marker) to each of the 253 endpoint literals.
- **Known cost grows.** Each of the 253 literals is touched in landing 1 (the collection marker) and rewritten again to builders in landing 2. The landing 2 claim of "one pass over the 253 endpoint definitions" now describes the builder rewrite only. Landing 1 also grows in size.
  - *Superseded 2026-10-11 by D42:* the collection marker is no longer added to the 253 literals in landing 1; the cost described here is removed.
- **Landing 2** otherwise keeps its D5 contents, minus the pagination model; its GraphQL authoring line is open (landing and home of the primitive, D17), and it gains the vocabulary-changing items listed under "Work not yet assigned" below.

**Risk notes from the spikes, 2026-10-11.** Evidence is from throwaway prototypes on one host (macOS), reported and not re-run by this review; none of it is a decision.

- **Landing 1 compile cost is measured and small for the trait, not for wrappers.** On a synthetic 550-endpoint input, the new `Endpoint` trait cost about the same as today's `EndpointSpec` (11.70 s versus 11.54 s); 550 calls to a generic `send` added about 7 s (18.54 s). So the generator should not emit a wrapper method per endpoint. Incremental rebuild time and real-schema compile time were not measured (the host was overloaded), so landing 1 has no compile-time evidence beyond this.
- **Landing 1 binary size.** The pager adds about 1.77 MB to a release binary at 157 endpoints; most of it is a second deserializer per item. A fix is proposed, not measured.
- **Landing 1 call sites change more than D3 says.** Pre-send failures, interrupted bodies, and 2xx decode failures each need a home in the envelope (Open Questions), and each moves sniff's exhaustive matches. These are decided before the migration, not during it.
- **Landing 1 generator work grows.** The generator must separate query parameters from the path (needed for redaction), and the request enum that D3 hoped to remove stays for `into_parts` and per-client scoping.
- **Landing 2 hooks.** Hooks on text, binary, and stream endpoints become compile errors in the new design; any existing definition that has one fails to build and needs a fix.
- **Update, 2026-10-11 (D22-D29).** Rulings D24-D26 decide the three envelope cases above before the migration, as intended. **Landing 1 grows again** if D28 is placed there: Schematic must follow redirects itself (hop limit, per-hop origin checks, credential stripping, method and body rules for 301/302/303/307/308), which D4 had set aside as too complex. D23 makes the query-parameter split (S6) a prerequisite generator task. D27 removes work this review had expected (no manual open/close, no drop-warning path).
- **Update, 2026-10-11 (D30-D48).** D30 places the redirect-following runtime and the API-level cross-origin field in landing 1, so the landing-1 growth flagged above is accepted. D31 adds the managed HTTP builder and removes client injection. D39 adds a new runtime package ("one more package and coordinated versions with the generator", the author's stated cost). D42 removes the 253-literal touch that D19 had added. D33 adds generator-only request-side work. The earlier statement that the request enum stays "for `into_parts` and per-client scoping" is now a planning detail (D45): whether the erasing enum remains is left to planning.
- **Untested, so still risks:** HTTP/2; Linux, Windows, and WSL2; a real provider; the offset strategy; partitions; rust-analyzer latency.

### Placement of D9-D16 (added 2026-10-10)

The author's instruction was: D9, D10, D11, D12, D13, and the pagination runtime go in landing 1; the pagination definition model, the D15 builders, and the GraphQL authoring model go in landing 2 unless ruled otherwise; GraphQL placement overall is an open assignment.

| Decision | Landing | State |
|---|---|---|
| D9 partial generation | 1 | Placed |
| D10 request-side features (text and binary bodies, `ApiKeyParam`, header and cookie parameters) | 1 (per D5, unless the author rules otherwise) | **OPEN**: the landing for D10 is not ruled |
| D11 credentials | 1 | Placed |
| D12 credential host policy | 1 | Placed; builder name and custom-client interaction open |
| D13 shared deadline | 1 | Placed |
| D14 pagination runtime (pager, link following through the host policy, deadline use, envelope headers) | 1 | Placed; see the dependency note below |
| D14 pagination definition model (`schematic-define` primitives, authoring in `schematic-definitions`, generated knobs) | 2, then **1** (D19, 2026-10-10) | Moved to landing 1 by D19; originally placed in landing 2 unless ruled otherwise |
| D15 builders, no public `Default` | 2 | Placed unless ruled otherwise |
| D16 GraphQL authoring model | 2 unless ruled otherwise | **OPEN**: GraphQL placement overall (definition crate, new crate, or consumer) is not assigned. D17 (2026-10-10) chose string-authored operations; the primitive's home and landing are still open |
| D17 GraphQL string operations (narrows D16) | Open (1 or 2) | **OPEN**: home of the primitive and its landing |
| D18 fourth landing (fidelity and hygiene) | 4 | Placed; order relative to landing 3 is **OPEN** |
| D19 pagination model and generated pagers | 1 | Placed; how the inferred default is expressed on a literal is **OPEN** |
| D20 whole-work definition of done | All landings | Placed |
| D21 envelope metadata and redaction | 1 (the envelope is landing 1 per D5) | Placed; request-id field, auth-method representation, and secret-marking granularity **OPEN** |
| D22 echoed secrets (narrows D21's promise) | Follows D7/D21 (landing 1) | Not separately ruled; this review's reading. Documentation and criteria change only |
| D23 URL in network-error text | Follows D7/D21 (landing 1) | Not separately ruled; depends on the query-parameter split (S6) |
| D24 pre-send failures | Follows D7 (landing 1) | Not separately ruled; moves sniff's "auth required" case |
| D25, D26 undecodable 2xx and cut-off body | Follow D7 (landing 1) | Not separately ruled |
| D27 scoped `with_session` only | Follows the pagination runtime (landing 1, D14/D19) | Not separately ruled; closure shape **OPEN** |
| D28 cross-origin policy in the definition | **OPEN**: the redirect-following runtime follows D4/D12 (landing 1, this review's reading); the `schematic-define` primitive is vocabulary and its landing (1 or 2) is not ruled | **OPEN** |
| D29 GraphQL failure with partial data | Open, with GraphQL (D17) | **OPEN**: GraphQL landing is not ruled |

*Update, 2026-10-11 (second round).* Every row above that says OPEN is now ruled; see "Current landing assignments" at the top of this section: D10 is landing 1 (D33); D16/D17/D29 GraphQL is landing 2 in `schematic-define` and `schematic-gen` (D30, D34); D19's "how the inferred default is expressed" is answered by D42 (an exceptions table on the API definition, no field on literals); D21's open fields are answered by D48 and D40; D28's redirect runtime and API-level field are landing 1 and the per-endpoint override is landing 2 (D30); D12's "builder name and custom-client interaction" is answered by D31 (no injected client).

**Dependency flagged by this review, not decided.** The pagination runtime ships in landing 1 but the pagination definition model that tells it what to do ships in landing 2. Until landing 2, nothing authored in `schematic-definitions` can drive the runtime, and the generator ignores all pagination data today. Landing 1 can still be tested with hand-built strategy values, the way the spike prototype does, but the author should confirm that this split is intended, since the generated pager surface (D14 item 3) only exists once the definitions are migrated. A related effect: the 35 existing declared page-number endpoints and the OpenAI endpoints get no generated pagination until landing 2. *Superseded 2026-10-10:* D19 moves the pagination definition model into landing 1, so this split no longer exists; the cost is recorded in the amendments after the landings table.

### Work not yet assigned to a landing (all OPEN)

D1 says nothing is deferred, so each item below is in scope and must be assigned. None is assigned today; do not read an absence from a package as a drop.

> **Update, 2026-10-10.** D18 assigned most of these items; the last column records the assignment. The heading's "(all OPEN)" is the original state. Still open: the single generated list of APIs, and the P5 item that D18 did not name (required constant headers as OpenAPI parameters).

| Item | Source | Natural neighbor (a suggestion, not a decision) | Assignment (D18, 2026-10-10) |
|---|---|---|---|
| Typed path parameters | 5 (4) | P3 | Landing 2 |
| Per-endpoint auth override, `deprecated`, rate limits, retry policy | 5 (5), 9 (14) | P3 or P4 | Landing 2 |
| Query-param `required`, `ParamStyle`/`explode`, and enum handling | 6 (3) | P3 | Landing 2 |
| Feature flags per API in the generated crate | 6 (7), 9 (10) | P1 | Landing 4 |
| Single registry and API-list generation (replacing the seven lists) | 4.3, 9 (11) | P3 | **OPEN**: not assigned by the author |
| README doctests (`include_str!` for the `schematic-define` README) | 9 (14), defect 19 | P1 | Landing 4 |
| Vendor pagination constructors moved to `schematic-definitions` | 4.4, 9 (14) | P3 | Landing 4 |
| Retiring `env_auth[0]` as the Basic-auth password | 4.2, 9 (14) | P3 | Landing 2 |
| Polymorphic `oneOf` bodies from hand-written definitions | 5b (1) | P3 or P4 | Landing 2 |
| Serializable `RestApi` | 5b (3) | P3 | Landing 2 |
| OAuth2 per-grant payloads | 5b (5) | P3 | Landing 2 |
| P5 work not tied to P3/P4: schemars bridge, `x-schematic` round-trip, Postman example bodies | 7, 9 (8-9) | landing 1, 2, or 3 | Landing 4 |

*Update, 2026-10-11 (second round).* The two items left unassigned are assigned to landing 4 by D46: the single generated list of APIs (replacing the seven parallel lists) and required constant headers as OpenAPI parameters. The relative order is landing 3 (WebSocket) then landing 4 (fidelity). D47 adds the AsyncAPI exporter to landing 4.

> **Correction, 2026-10-11:** the "seven lists" are six runtime lists plus a test table (see the correction in section 4.3).

Not listed above because they already sit in a package: the generated-docs honesty pass (9, item 12, with P2's typed send) and WebSocket diagnostics (P6).

*Update, 2026-10-11 (D49).* The remaining loose items are assigned; the table below extends the one above and is the current record.

| Item | Source | Assignment (D49, 2026-10-11) |
|---|---|---|
| Isolated compile check of freshly generated code | 9 (4), ledger row 16 | Landing 1 (it guards every later generator change) |
| Unify the two divergent module-path resolvers | 9 (10), ledger row 22 | Landing 4 |
| Generated-docs honesty pass (`no_run` fences, correct quick-start, no `///;` lines, typed convenience methods where applicable) | 9 (12), ledger row 8 | Landing 4 |
| `Endpoint::oauth_scopes` and `RestApi::auth_policy`: wire in or remove | 4.2, 7.1 | Decided during landing 2 while the definitions are rewritten; the choice is **open**, its landing is fixed |
| `ResponseKind` runtime guard in the generated client | 6, 3 | Landing 1, with the new response model (D3, D6, D7); kept, changed, or removed as that model requires |

### Dependencies derivable from the current code and decisions

These follow from how the pieces are built today. They are constraints on any ordering, not an ordering.

- **P4 and P3 touch the same code.** Changing the type of `Endpoint::response` (section 5b, item 2) breaks the vendor-extension format, both exporters, the importer, and all 253 endpoint literals. Rewriting those literals once, through builders and typed registration, is cheaper than rewriting them twice. P4 should be considered together with P3.
- **D3 and D7 constrain P4.** A typed request declares exactly one success type `T` and one failure type `E` in the `Response<T, E>` envelope. Streaming and multiple success shapes must be expressible as `T` (D6: a plain type, or a small generated enum when several success shapes exist). The stream's concrete Rust type is open.
- **P2's evidence contract must survive P4.** Preserving status, headers, and body does not depend on typed error schemas (section 13), but P4 changes response shapes that the P2 envelope already wraps. This is the known double change to dependents recorded above.
- **P5's Postman example bodies depend on P3.** Examples are generated from registered schemas; typed registration (D2) is what makes the registry authoritative.
- **P5's exporter work overlaps P4.** Error responses, response content types, and streaming media types all appear in the OpenAPI exporter.
- **P6's reuse of REST types depends on P3 and P4.** D8 makes WebSocket reuse the REST parameter, auth-policy, env-mapping, and header types; doing that before the REST vocabulary settles would repeat the work. This is why it lands last.
- **P1 has no dependency on the others** for its core items, except that the isolated compile check (P1) is the verification mechanism later packages would lean on.
- **D4 is part of P2** but changes the client construction surface that P3's typed requests also use.

### Still open in this section

Sequencing and packaging are decided (D5). The author has not decided:

- The definition of done for the whole body of work, as distinct from the per-landing and per-package criteria in section 14.
- The landing assignment of every item in "Work not yet assigned to a landing" above.
- The landing for D10, and the placement of GraphQL (D16), per the table above.
- Whether the landing-1 pagination runtime and the landing-2 pagination definition model should stay split (see the dependency note above).

*Update, 2026-10-10.* Closed by D18-D20: the whole-work definition of done (D20); the landing of every previously unassigned item except the single generated list of APIs (D18); the pagination split (D19, both halves in landing 1). Still open: the landing for D10; the GraphQL placement and landing (D17); the single generated list; required constant headers as OpenAPI parameters; and the relative order of landing 3 (WebSocket) and landing 4 (fidelity).

*Update, 2026-10-11.* D22-D27 follow the decisions they amend and add no landing question of their own beyond D28 (the landing of the redirect-following runtime and of the `schematic-define` primitive) and D29 (with GraphQL). Both are still open.

*Update, 2026-10-11 (second round).* Everything listed as open in this section is ruled: the landing for D10 (landing 1, D33); GraphQL placement and landing (landing 2, in `schematic-define` and `schematic-gen`, D30, D34); the single generated list and required constant headers (landing 4, D46); the order of landings 3 and 4 (D46); D28 and D29 landings (D30). The remaining details are planning-time and listed in section 15.

*Update, 2026-10-11 (D49).* The loose items that sat outside every package or landing (isolated compile check, module-path resolvers, generated-docs honesty pass, `oauth_scopes`/`auth_policy`, the `ResponseKind` guard) are assigned by D49; see the table after "Work not yet assigned to a landing". Only the wire-in-or-remove choice for `oauth_scopes` and `auth_policy` stays open, and its landing is fixed.

### Plan structure (D50)

*Added 2026-10-11, confirmed by the author (option A of three).* This spec is the parent (decisions and evidence). Seven child specs carry the work, in order, each with a 1:1 plan and each leaving the repo green: (1) generation integrity, (2) runtime and envelope, (3) pagination, (4) definition API, (5) response model v2 and GraphQL, (6) WebSocket, (7) fidelity and polish. Landing 1 = specs 1-3; landing 2 = specs 4-5; landing 3 = spec 6; landing 4 = spec 7. Details, rejected options, and pre-authorizations are in D50.

*Added 2026-10-11, confirmed by the author (D51).* All seven child specs are drafted now (option B), and each one runs through the same per-spec process: implement every phase of its plan without a human in the loop; "review 1", where the author reviews the decisions made during implementation and any open design questions; refresh the next spec and its plan from the outcomes of review 1; then loop fix and review cycles until the reviewer deems the spec production ready. Specs 2-7 are therefore drafts (`refresh_required: true`), refreshed after the previous spec's review 1; drafting them now serves visibility and dependency checking, not a commitment to their final text. The child specs are `2026-10-11-schematic-generation-integrity`, `-schematic-runtime-envelope`, `-schematic-pagination`, `-schematic-definition-api`, `-schematic-response-model-graphql`, `-schematic-websocket`, and `-schematic-fidelity-polish`, under `schematic/features/`. Details are in D51.

> **Update, 2026-10-11 (D52).** Spec 4 is split in two: 4a `2026-10-11-schematic-definition-api` and 4b `2026-10-11-schematic-definition-vocabulary`. The plan is now **eight** specs: 1 generation-integrity, 2 runtime-envelope, 3 pagination, 4a definition-api, 4b definition-vocabulary, 5 response-model-graphql, 6 websocket, 7 fidelity-polish. Landing 1 = specs 1-3; landing 2 = specs 4a, 4b, 5; landing 3 = spec 6; landing 4 = spec 7. D53 and D54 record related rulings. The "seven" in this subsection and in D50 and D51 is the original count.

The P1-P12 planning-time items in "Remaining planning-time details" (section 15, not the packages P1-P6 above) map onto the specs as follows. Where an item spans several topics, each part is listed.

| Item | Topic | Owning spec |
|---|---|---|
| P1 | Cross-origin details (D28, D30, D32) | 2 (runtime, API-level field); the per-endpoint override scope part goes to 4 |
| P2 | `TransportError` kinds | 2 |
| P3 | Managed builder, call-site migration, middleware hooks | 2 |
| P4 | Runtime package name, versioning, contents | 2 |
| P5 | Envelope and `send_raw` details | 2 |
| P6 | Deadline arithmetic; `with_session` shape | 2 (deadline); 3 (`with_session`) |
| P7 | Query parameters separated from the path | 1 |
| P8 | Pagination details | 3 |
| P9 | Selector, end conditions, error mappings, GraphQL names | 5 for D34-D38; 4 for D44 registration; request-id marker and `.secret()` forms (D40, D48): owner to assign |
| P10 | Mixed planning details | 6 (explicit response link); 7 (AsyncAPI, feature flags, `x-schematic` round trip, Postman example values); 1 (diagnostics format for validation errors) |
| P11 | Size of landing 1 | Answered by D50 (landing 1 is specs 1-3) |
| P12 | `oauth_scopes` and `auth_policy` choice | 4 |

> **Update, 2026-10-11 (D52, D54): revised P1-P12 mapping.** Spec 4 is now 4a and 4b. **No P1-P12 item moves wholly to 4b**, because 4b's content (typed path parameters, per-endpoint auth override, `deprecated`, rate-limit and retry vocabulary, `env_auth[0]` retirement, `oneOf` bodies, serializable definitions, OAuth2 grants, query-parameter fidelity) is vocabulary the P-items do not list. The rows that named spec 4 now read as follows. Rows not listed are unchanged.
>
> | Item | Owning spec now |
> |---|---|
> | P1 | 2 (runtime, API-level field); the per-endpoint override scope goes to 4a |
> | P9 | 5 for D34-D38; 4a for D44 registration; request-id marker and `.secret()` forms (D40, D48) are assigned by D54: semantics in spec 2, builder sugar and per-parameter `.secret()` in spec 4a |
> | P12 | 4a (assumption by this edit: the `oauth_scopes` field is removed or kept as part of the literal rewrite, which 4a owns; not stated by the author) |

---

## 14. Verification and Completion Criteria

Implement in `schematic-define`, `schematic-definitions`, and `schematic-gen`, then regenerate `schematic-schema`; do not hand-edit generated clients. Use the monorepo Test Toolkit and the package-area `just test` and `just lint` recipes. Compile-only tests are L1 checks, not L2 merely because they are slow. Actual L2 terminal tests use `just test-l2`; browser tests follow the browser-tier workflow. Neither may take window focus. Do not introduce CI cells or alter CI scope as part of this document review.

Each work package (section 13a) is complete when its relevant checks demonstrate the criteria below, and each landing (D5) is accepted when the packages it contains are complete (see "Per-landing acceptance" below). **Completion of the whole body of work is not defined by this document**; the whole-work definition of done is open (section 13a). *Superseded 2026-10-09/10:* D20 defines it, and its criteria appear under "Criteria for D17-D21" below. Passing every bullet here would show each listed behavior, not that the review's findings are exhausted.

> **Update, 2026-10-11 (D52-D54).** The work is carried out by eight child specs (D52 splits spec 4). References to "landing 2" criteria below are carried by specs 4a, 4b, and 5. D53 records an accepted gap against D49 item 3; D54 records ownership clarifications.

> **Scope of "debug or display output" in the criteria below (D22, 2026-10-11).** Wherever a criterion says a sentinel token must be absent from debug and display output, it holds for stored metadata and for `Display` only. It is **not** guaranteed for `Debug` of a decoded provider error body, which prints as received. Criteria D10, D11, and D21 below are read with that exception, and the D22-D29 criteria state the testable form.

### Safety, runtime, and OpenAPI criteria

- Partial generation of a shared-module API keeps its sibling and unrelated clients exported and compilable; stale owned files in the selected group are removed, and unowned files survive.
- Unsupported features fail before output mutation. Valid type paths and supported form bodies still generate. Supported parameter names keep their wire spelling without Rust-name collisions.
- Fresh output compiles in an isolated crate without network calls to providers. Include representative JSON, form, URL-encoded, and supported auth configurations; compare deterministic output without rewriting the checkout.
- Local mock HTTP responses preserve headers and body on errors, including malformed JSON. Credential-source metadata agrees with the selected credential; explicit auth and environment overrides obey documented precedence.
  *Note, 2026-10-10:* the precedence rules are now fixed by D11 and the host and deadline criteria below refine the next bullet (D12, D13).
- Policy-enabled calls reject disallowed initial and continuation URLs before credentials are sent. Multiple pages consume one deadline, and bounded or failed pagination cannot report completeness.
- OpenAPI conversion preserves nullable types, enums, composed schemas, and free-form JSON deliberately. Supported extension round-trips compare meaning, not serialized byte order.

### Criteria for items brought into scope on 2026-10-09

Each is phrased so a test could decide it. Where a test cannot be written without an owner decision, that is stated.

- **Redirect as error (D4).** Against two local mock servers, a policy-enabled request whose first server answers with a redirect returns an explicit redirect error, and the second server records zero requests, so no credential reaches it. A non-policy client built with `with_client` keeps its current redirect behavior.
  *Superseded in part, 2026-10-11 (D28):* redirects are followed or refused according to the definition's cross-origin policy (default `same_origin`), not always refused. The zero-requests-to-the-second-server check now applies whenever the policy refuses that server; see the D28 criteria below. What a custom injected client does is open.
  *Superseded again, 2026-10-11 (D31):* the clause "a non-policy client built with `with_client` keeps its current redirect behavior" no longer applies, because `with_client` is removed and every client is Schematic-managed. What a custom injected client does is closed (there is none).
- **Typed requests (D3).** A `compile_fail` doctest or compile-fail test shows that sending a generated request type cannot yield a response type other than the one it declares, and that the free generic `request<T>` is absent from the public API of the generated crate. The migration of the dependents (sniff, biscuit-speaks, the five homelab crates, and model-citizen if it uses the crate; about 27 request call sites outside sniff) is verified by their own existing builds and tests. The criterion for a custom-decoding path cannot be written until its shape is decided (open).
  *Update, 2026-10-11 (D45):* the custom-decoding path is `send_raw` (status, headers, bytes, JSON convenience); typed sends stay strict. Criteria are in "Criteria for D30-D48" below. Whether the erasing request enum remains is left to planning.
- **Typed registration (D2).** Registering a type through the context yields a descriptor whose generated Rust path compiles in the isolated crate, and an unregistered type referenced by an endpoint is a validation error. Cases for aliases, external types, and container paths cannot be specified until the naming rules are decided (open).
  *Update, 2026-10-11 (D44):* the naming rules are decided; the cases for aliases, external types, and containers are in "Criteria for D30-D48" below.
- **Builders.** For every definition migrated to builders, the built value is equal to the literal it replaces (equality of the exported OpenAPI and generated Rust output is a usable substitute if `Endpoint`/`RestApi` do not implement `PartialEq`; whether to add that trait is not decided). A builder cannot construct an endpoint missing a required field (compile-fail test).
- **Streaming (SSE/NDJSON).** A local mock server sends several events or lines with deliberate delays, and the test asserts that the first decoded item is observed before the mock finishes sending. Malformed items, premature close, and a response that is not the declared framing each return a distinct error that retains status and headers. Whether a stream item type and a terminal marker such as `[DONE]` are modeled per framing is not decided, so the terminal-event criterion is not yet testable.
  *Update, 2026-10-11 (D37):* the terminal-event criterion is now testable; see D37 below.
- **Response content types.** Exported OpenAPI for the ElevenLabs audio endpoints reports the declared media type rather than `application/octet-stream`, and import followed by export preserves it.
- **Status-to-schema responses and typed error bodies.** A mock returns a declared 4xx body and the client exposes it typed, together with status and headers; an undeclared status or an undecodable body still yields the raw status, headers, and body. OpenAPI export lists every declared status. Which statuses count as errors, and the shape of the generated result type, are open.
  *Update, 2026-10-11 (D38, D48):* the status-to-error mapping and the "no declared error" form are decided; see below.
- **WebSocket fixes.** The five concrete defects are individually testable: the `ws://remote.local/ws/ws` URL is corrected (assertion on the generated URL), the reconnect attempt counter resets after a successful connect and the lifecycle-open message is re-sent (mock server with forced drops), the Bearer host template compiles in the isolated crate, env-derived credentials are sent at connect time (mock server inspects the handshake), and a failing WebSocket plan produces a visible diagnostic.
- **WebSocket reuse of REST types (D8).** A WebSocket definition declares its parameters, auth policy, env mapping, and headers with the same Rust types the REST definitions use, and `ConnectionParam`/`ParamType` no longer exist (a compile-level check: the old types are absent from the public API; the five definitions build and generate with the shared types). Full REST/WS unification is rejected, so no criterion exists for it. **An AsyncAPI exporter has no criterion**: whether it is in scope is open (section 15).
  *Update, 2026-10-11 (D47):* the AsyncAPI exporter is in scope, in landing 4; its criterion is under D47 below.
- **Response model v2 (D6).** An endpoint declaring several success shapes generates a small enum; one declaring a single shape generates a plain type (generated-output assertion). A declared error body decodes on demand from the error. Not testable until owner decisions land: the streaming terminal marker, same-URL streaming and non-streaming endpoints, and status-code-to-error mapping.
  *Update, 2026-10-11 (D36, D37, D38):* the three "not testable until owner decisions land" items are decided; see below.
- **Envelope metadata (D7).** With a local mock server, one success request and one declared-error request each return an envelope whose metadata holds request headers, response headers, query parameters, HTTP status, and auth method used. `data()` returns `Some` and `error()` returns `None` on success, and the reverse on failure; `into_result()` agrees. A malformed or undeclared error body still yields status, headers, and the raw body inside the envelope's error. [Superseded 2026-10-10: D21 fixes the field list; see the D21 criteria below.] The exact field list is open, so a test of "all fields" cannot be written yet.
- **Redaction (D7).** Using a sentinel token value, no debug or display output, and no metadata field, of any envelope, error, or transport error contains the token, in the success, error, and transport-failure cases; the auth method records only anonymous, an environment variable name, or the caller's label. [Superseded in part 2026-10-10: D21 defines the built-in list and definition-marked secrets; see the D21 criteria below.] Which headers and query-string parameters count as secret (for example `ApiKeyParam`) is open, so the redaction test is only partly specifiable: it can assert the sentinel is absent but cannot assert which non-credential headers stay visible.
  *Amended by D22, 2026-10-11:* "no debug or display output" is read as stored metadata and `Display`; `Debug` of decoded error bodies is explicitly excluded.
- **Transport failure keeps request metadata (D7).** Against an unreachable port, a timeout, a credential-host rejection, and a redirect denial, the outer `Err(TransportError)` carries the request headers (redacted), query parameters, and auth method used.
  *Refined, 2026-10-11 (D24, D28):* a failure before anything is sent (missing credential, page size above the maximum, origin rejected, invalid argument) is also an outer `TransportError` with a specific kind. A "redirect denial" is governed by the D28 policy; whether a refused redirect after a 3xx head arrived is an envelope or a `TransportError` is open.
  *Refined again, 2026-10-11 (D32):* a redirect the policy refuses (a 3xx head arrived) is a failure inside the envelope, kind `RedirectRefused`, not a `TransportError`; a refused next-page link is the outer `TransportError`, kind `CrossOriginRejected`. See D32 below.
- **Per-landing acceptance (D5).** Landing 1: the P1 and P2 criteria above pass; dependents build against the new envelope and typed send. Landing 2: the P3, P4, and dependent P5 criteria pass and the 253 definitions are migrated in one pass; dependents build again where response shapes changed. Landing 3: the P6 criteria and the D8 reuse criterion pass. What shows the whole of D1 is done is not defined (open).
  *Superseded 2026-10-10 (D18, D19, D20):* there are four landings. Landing 1 additionally passes the D14 pagination criteria that do not depend on builders, plus the collection marker on the 253 literals (D19). Landing 4 (D18) passes the fidelity and hygiene criteria (OpenAPI bridge, `x-schematic`, Postman, README doctests, vendor presets moved, feature flags). The order of landings 3 and 4 is open. The whole of D1 is done only when D20's definition holds.
  *Updated 2026-10-11 (D30, D42, D46):* landing 1 passes the D14 pagination criteria that do not depend on builders and **no longer** includes a collection marker on the 253 literals (D42). Landing 1 additionally passes the redirect-following and API-level `cross_origin` criteria (D30). Landing 2 passes the per-endpoint override, GraphQL, response-model, and builder criteria, and the 253 definitions are migrated in one pass. Landing 3 is WebSocket and landing 4 is fidelity, in that order (D46).

> **Correction, 2026-10-11:** the count of 253 endpoint literals, and every ratio derived from it, is stale. The verified count is about 519 (see the correction in section 2). Read "253" in this place as "about 519".
- **Postman example bodies.** For each JSON-bodied endpoint with a registered schema, the generated body is valid JSON that validates against that schema. Whether example values should come from schema defaults, schema examples, or generated placeholders is not decided, so "meaningful example" cannot be tested.
- **`x-schematic` round-trip.** For the supported subset, export then import then export yields semantically equal documents; the supported subset must be listed before this is complete (open).

### Criteria for D9-D16 (added 2026-10-10)

Each is phrased so a test could decide it. **NEEDS RULING** marks a criterion that cannot be written until the author decides the named detail. Mock servers are local; nothing here contacts a real provider. Where a criterion says "sentinel token", use a distinctive fake credential value and assert it never appears in any output.

**D9 partial generation.**

- Generating `OllamaNative` alone into a copy of the full output keeps `OllamaOpenAI` (same module group) and all 16 other APIs exported and compiling; `lib.rs` and the shared files list all 18.
- A stale file that the writer provably owns, inside the regenerated group, is deleted; an unowned file in the same directory survives.
- If the existing inventory cannot be established (for example, a corrupt or hand-edited `lib.rs`), the run fails before writing anything: the directory is byte-identical afterward.

**D10 request-side feature support.** Each runs the generated client against a local mock server that records the raw request.

- Text body: the server sees the declared `Content-Type` and the exact text. Binary body: the server sees the exact bytes and the declared `Content-Type`.
- `ApiKeyParam` as a query parameter: the server sees the key in the query string. As a cookie: the server sees the `Cookie` header. In both cases the key never appears in debug output (sentinel token).
- Header parameters and cookie parameters declared on an endpoint arrive with the declared names and values.
- Per-output backstop: a definition using a feature that one output lacks fails validation naming API, endpoint, feature, and output, before any file is written, and the other outputs still generate.
- Regression: the verified counts (JSON bodies about 312, multipart about 29, URL-encoded 0, text 0, binary 0, `ApiKeyParam` 0, header or cookie parameters 0, binary and text responses 33 and 17) are a baseline. Because no current definition uses the zero-count features, tests need purpose-built fixture definitions, not the shipped 18.
- **NEEDS RULING:** none for behavior; the landing is open (section 13a).
  *Closed 2026-10-11 (D33):* landing 1; the work is generator-only and edits no definition.

**D11 credentials.**

- Precedence: with sentinel tokens set at every level (per call, client level, an environment variable, and none), the mock server sees exactly one credential per request, in the order per-call, client-level, environment, anonymous. Removing the winner makes the next one win.
- Environment scan: with several variables listed, the first non-empty one is sent (an empty variable is skipped); a caller-supplied ordered list at client level replaces the definition's order.
- Resolved once: changing the environment variable between resolution and the response does not change the auth method reported in the envelope.
- Single attempt: when the mock server answers 401, the server has received exactly one request. No second request is sent with another key or without one.
- Reporting: the envelope's auth method is, respectively, per-call, client-level, the variable's name, or anonymous, and never the value (assert the sentinel is absent from envelope, error, and debug text).
- **NEEDS RULING:** the exact representation of the reported auth method (the D7 field list is open).
  *Closed 2026-10-11 (D48):* `AuthMethod = PerCall | ClientLevel | EnvVar(name) | Anonymous`, never a value.

**D12 credential host policy.**

- With the policy enabled and the URL's scheme, host, or effective port different from the base origin and not in the extra-origins list, the call returns a `TransportError` and the mock server records zero requests. The error carries request metadata.
- A declared extra origin (exact match) receives the credential; a sibling subdomain and a differing port do not (no wildcard or domain matching).
- A next-page link or followed link to another host is rejected before any send, including when the first page succeeded.
- A client built without the policy performs no host check (today's behavior preserved).
- **NEEDS RULING:** behavior when the policy is enabled and a custom HTTP client was injected; the policy's builder name (a test needs the name); whether a definition may carry known extra origins.

  *Superseded in part, 2026-10-11 (D28):* the policy now comes from the definition (default `same_origin`) and always applies, so "a client built without the policy performs no host check" and "whether a definition may carry known extra origins" no longer apply (the definition carries them through `allow_origins`). The builder name is now the naming question of D28. The injected-client question remains open. The first-request and next-link rejection checks above remain, with the outer `TransportError` kind `CrossOriginRejected` (D24).
  *Closed 2026-10-11 (D31):* the injected-client question is moot because injection is removed.

**D13 shared deadline.**

- Across N pages from a mock server that delays each page, an overall deadline smaller than the sum of delays stops the operation: no request is sent after the deadline, and `DeadlineExceeded` reports the completed pages and elapsed time.
- Each request receives the remaining time, not the full per-request timeout (the mock records the effective timeout it was given or the time at which it gave up).
- Dropping the operation mid-flight cancels the in-flight request (the mock sees the connection close) and no task keeps running (no detached work; a runtime shutdown completes promptly).
- Without a deadline, the existing 30 s per-request default applies unchanged.
- Monotonic clock: a test that moves the wall clock is not portable; verify with a controllable clock source if one is introduced, otherwise by inspection. (Partly testable.)

**D14 pagination.**

- Unauthored knob does not compile: for an endpoint with no page-size rule, a `compile_fail` test shows `.page_size(..)` is absent; likewise `.after(..)`, `.page(..)`, `.offset(..)`, `next()` when the matching strategy is not authored, and an unpaginated list endpoint has no pager at all.
- Inference and defaults: an endpoint returning `Vec<T>` is recognized as a root-array list and, with no API default, is unpaginated; a wrapper list marked with `Collection::at(path)` finds items under the path; an API default applies to every list endpoint; an override and an opt-out each replace the default for one endpoint.
  *Updated 2026-10-11 (D42, D43):* inference applies to GET endpoints only; before landing 2 an API-level exceptions table (keyed by endpoint id) expresses wrapper lists, opt-outs, and overrides, with no field on the endpoint literal.
- Coverage: a test authors the list endpoints of all 18 REST APIs with the model (as the prototype's 16 inventory tests do) and asserts the resolved strategy and knob list for each. Count check: about 157 collection endpoints, 35 declared today.
- Every inventoried strategy and the 10 outside patterns pass against mock servers (the prototype has 56 tests at the model level; the pager itself is not prototyped). Six of the 10 outside patterns were checked from memory and are **unverified** (X API v2, WordPress REST, OData `$skiptoken` in its non-Graph form, Azure Cosmos DB, Salesforce REST query, Reddit listings); do not treat their tests as proof of the provider's real behavior.
- Page size above the author-declared max returns a typed error before any request is sent.
- Link following goes through the D12 host policy: a next link to another host is rejected unsent. *Updated 2026-10-11 (D28): the policy is the definition's cross-origin policy; under `same_origin` a link to another host is rejected unsent.*
- Labeled alternates: the model represents them and a definition using one validates. **NEEDS RULING:** whether the generated client exposes them (the design recommended modeling now, generating later; the author's ruling says only that the model supports them).
- Sessions: a mock scroll or PIT server sees open, keep-alive, and close requests. *Closed by D27, 2026-10-11:* only the scoped `with_session` exists; it closes on return, error, and panic, there is no manual `open()`/`close()` and no drop warning, and cancelling the caller's whole future leaves the session to the server's own timeout. See the D27 criteria below. (Original text: "closing at the end of iteration and on an explicit `close()` is testable. **NEEDS RULING:** behavior on early drop.")
- The complete-list helper is **not testable** until the author ratifies it. If ratified as proposed: it returns success only after the mock server's end condition was observed; a repeated continuation, an invalid continuation, an HTTP error, a page bound, or an expired deadline returns an explicit Incomplete result with the reason and the items so far.
  *Closed 2026-10-11 (D41):* ratified; the criteria are under D41 below. The reason list is D41's (`PageLimitReached`, `RepeatedContinuation`, `HttpError`, `DeadlineExceeded`, `CrossOriginRejected`).
- Not modelable, so no test: the two OpenAI vector-store endpoints (provider documents no continuation request) and EMQX `ListAuthUsers` (wrong response type in the definition). Job creation then polling (Salesforce Bulk, BigQuery) belongs to a different feature.

**D15 no public `Default` on definition types.**

- A `compile_fail` test shows `Endpoint::default()` and `RestApi::default()` do not compile from outside the crate; builders cannot build an endpoint missing a required fact.
- Generated request types in `schematic-schema` still implement `Default` (existing generated tests keep compiling).

**D16 GraphQL.**

- Envelope surfacing: a mock GraphQL server answers HTTP 200 with `{"errors": [...]}`; the envelope reports the GraphQL errors on its error side or in its metadata (which one is open), with status 200 and the raw body retained. *Closed by D29, 2026-10-11: the envelope reports GraphQL errors on its failure side with partial data; see the D29 criteria below.*
- A typed query decodes a successful response into its declared type. *Refined 2026-10-10 (D17):* the operation is a string held in the definition, its variables are a typed value serialized into the request, and the response data decodes into the declared type; see the D17 criteria below.
- Public GitHub versus Enterprise: with two mock bases (`/api/v3` for REST, `/api/graphql` for GraphQL), the same definition reaches the right URL for each.
- **NEEDS RULING (not testable yet):** partial data plus errors (is it success, failure, or both); how queries are authored; where the feature lives; the exact error-side semantics. No test should be written for these before the author rules. *Update 2026-10-10:* how queries are authored is ruled by D17 (strings, no build-time validation); the other items remain open. No criterion in this document assumes build-time query validation. *Update 2026-10-11: partial data plus errors and the error-side semantics are ruled by D29 (failure side). How queries are authored is ruled by D17. Where the feature lives is still open.*
  *Update 2026-10-11 (D30, D34, D35):* where the feature lives is ruled (`schematic-define` plus `schematic-gen`); the landing is 2; the URL difference is ruled by D35.

### Criteria for D17-D21 (added 2026-10-10)

Each is phrased so a test could decide it. **NEEDS RULING** marks a criterion that cannot be written until the author decides the named detail. Mock servers are local. Where a criterion says "sentinel token", use a distinctive fake credential value and assert it never appears in any output.

**D17 GraphQL authoring.**

- The operation string held in the definition is sent exactly as written; the mock server records the document and the variables as separate parts of the JSON body.
- Typed variables: a variables value of the wrong type fails to compile (`compile_fail`); a correct one serializes to the variables the mock records.
- Typed response data: a mock answering HTTP 200 with a `data` object decodes into the declared type.
- Errors in a 200 body: a mock answering HTTP 200 with an `errors` list yields an envelope whose status is 200, whose raw body is retained, and which reports the GraphQL errors (error side or metadata: **NEEDS RULING**). *Closed by D29, 2026-10-11: error side, with partial data.*
- No GraphQL parsing or validation: a syntactically invalid document is sent unchanged, and the server's error is what the caller sees. Nothing rejects it at build time or before sending.
- Both bases: the same definition reaches `/api/graphql` for GraphQL and `/api/v3` for REST against two mock bases (carried over from D16).
- **NEEDS RULING:** partial data plus errors; where the primitive lives; which landing; how typed variables and response data bind to the D2 registration context. *Update 2026-10-11: partial data plus errors is ruled by D29; the rest stays open.*
  *Update 2026-10-11 (D30, D34, D35):* all items are now ruled: home (D34), landing 2 (D30), address (D35), and binding through the D2 definition context (D34).

**D18 fourth landing.**

- Each item named for landing 4 has a criterion elsewhere in this section or below: schemars-bridge and `x-schematic` (OpenAPI criteria above), Postman example bodies (above), and the three below.
- README doctests: the `schematic-define` README examples compile as doctests, so a drifted example fails a normal test run.
- Vendor presets moved: `schematic-define`'s public API has no `github()`, `gitlab()`, `bitbucket()`, or `gitea()` pagination constructor, and `schematic-definitions` provides them (compile-level check).
- Feature flags: building the generated crate with a single API's feature compiles and exports only that API (isolated-crate check). **NEEDS RULING:** the flag granularity (per API or per module group; D9 shows a module can hold two APIs) and the default feature set.
- Landing 2 contains none of these items, and the vocabulary-changing items (typed path parameters, per-endpoint auth override, `deprecated`, rate limits and retry, `env_auth[0]` retirement, polymorphic `oneOf` bodies, serializable definitions, OAuth2 per-grant payloads, query-parameter `required`/style/enum fidelity) are each covered by the landing 2 criteria or by a new one written when the vocabulary is designed (these items have no criteria today).
- **NEEDS RULING:** the order of landings 3 and 4; the assignment of the single generated list of APIs. No criterion depends on either answer yet.

**D19 pagination in landing 1.**

- At the end of landing 1, the D14 criteria that do not depend on builders pass: unauthored knobs fail to compile, all 18 REST APIs' list endpoints resolve to the expected strategy, a page size above the declared maximum returns a typed error before any request, and link following goes through the D12 policy.
- Every one of the 253 endpoint literals carries the collection marker and the definitions build; the generated pagers exist for each authored list endpoint.
  *Superseded 2026-10-11 (D42):* no field is added to the 253 literals; the criterion becomes: the 253 endpoint literals are unchanged by landing 1, and `Vec<T>` GET endpoints resolve as lists through inference plus the exceptions table.
- sniff obtains real pagers from the generated clients without hand-looping on envelope headers (verified through D20's sniff criterion).
- At the builder rewrite in landing 2, the generated Rust and exported OpenAPI for the 253 endpoints are unchanged, except for intended changes recorded in the implementation log (the same equality substitute as the builder criterion above).

> **Correction, 2026-10-11:** the count of 253 endpoint literals, and every ratio derived from it, is stale. The verified count is about 519 (see the correction in section 2). Read "253" in this place as "about 519".
- **NEEDS RULING:** how the inferred `Vec<T>` default (D14) is expressed on a literal, since landing 1 precedes builders. The criterion for inference on literals cannot be written until then.
  *Closed 2026-10-11 (D42, D43).*

**D20 whole-work done.**

- All four landings pass their section 14 criteria.
- After each landing, every dependent crate builds and passes its tests: sniff, biscuit-speaks, model-citizen, and the five homelab crates (`homelab/lib`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`).

> **Correction, 2026-10-11:** dependents of `schematic-schema` are eight crates by path dependency: `homelab/{eversolo-integration, arcam-amp-integration, unfolded-integration-helper, lib, sony-receiver-integration}`, `sniff/lib`, `biscuit-speaks/lib`, and `model-citizen/lib` (manifest dependency only; no Rust reference).
- Every defect-ledger row in section 10 and every recommendation in section 9 is either closed or recorded as an accepted gap in the implementation log (decidable by reading the log: no row or recommendation is missing).
- sniff retires its hand-written GitHub and GitLab client (`FocusedProviderClient`), uses the generated clients, and passes sniff's own tests. Verification: no remaining reference to `FocusedProviderClient` in the sniff tree, and sniff's tests pass.
- The author closes the work; an agent's terminal state is "implementation complete, ready for review". This ties completion of this spec to the sniff migration, which has its own spec (`2026-10-09-consistent-use-schematic`).
- Not testable by a tool: whether an accepted gap is acceptable. That is the author's judgment.

**D21 envelope metadata.**

- With a local mock server, a success request, a declared-error request, and a paginated operation each return an envelope carrying: status, HTTP method, full URL, request headers, response headers, query parameters, auth method used, and elapsed time. Page number is present only when the call is part of a paginated operation. Provider request id is present when the definition names that header and absent otherwise.
- URL redaction: a sentinel value in a secret query parameter appears in the envelope's URL and query parameters as `[redacted]`.
- Header redaction: sentinel values in `Authorization`, `Cookie`, and `Set-Cookie` appear as `[redacted]`, as do the common API-key headers and any header or query parameter the definition marks secret. A non-secret header keeps its real value.
- Real values are never stored: the sentinel is absent from every envelope field, debug output, and display output, not merely masked at print time (inspect the stored fields).
  *Amended by D22, 2026-10-11:* the sentinel must be absent from stored metadata and from `Display`. It is explicitly **not** guaranteed for `Debug` of decoded error bodies (a provider that echoes a credential prints it there). See the D22 criteria below.
- The auth method records per-call, client-level, the environment variable's name, or anonymous, and never the value.
- Elapsed time is present on success, declared error, and transport-failure results.
- **NEEDS RULING:** the definition field that names the request-id header; the representation of the auth method; whether secret marking is per parameter or per auth strategy; the exact list of "common API-key headers".
  *Closed 2026-10-11 (D48, D40):* `request_id_header("...")`; `AuthMethod` as above; secret marking is automatic from auth strategies, an explicit `.secret()` marker, and the built-in list; the built-in names are listed in D48. Whether response headers beyond `Set-Cookie` share the redaction remains a planning detail.

### Criteria for D22-D29 (added 2026-10-11)

Each is phrased so a test could decide it. **NEEDS RULING** marks a criterion that cannot be written until the author decides the named detail. Mock servers are local; nothing here contacts a real provider. "Sentinel token" means a distinctive fake credential value.

**D22 echoed secrets (amends the D21 sentinel criterion).**

- Stored metadata and `Display`: the sentinel is absent from every stored envelope field and from the `Display` output of the envelope, the error, and the transport error, in the success, declared-error, undecodable-success, and transport-failure cases.
- **Explicitly not guaranteed, so not a criterion:** `Debug` output of decoded error bodies. A test documents the limitation instead of denying it: a mock answers 401 with the sentinel in its body; the test asserts the raw body keeps it (evidence is preserved) and that metadata and `Display` do not contain it. It makes no assertion about `Debug` of the decoded body.
- Documentation: the limitation appears in the generated crate's docs for the error types and in the `docs/` topic page (checkable by searching for it).

**D23 URL in network-error text.**

- Sentinel-key test: a request carries a secret query parameter whose value is the sentinel. The mock port is closed (connection refused) and, separately, the mock stalls past the timeout. In both cases the sentinel appears in no error text (the transport error's `Display`, and its `Debug`, which holds no provider body: this review's reading), and no stored field.
- The error text still shows the URL, with the secret parameter's value as `[redacted]` and a non-secret parameter (for example `page=2`) visible.
- The HTTP library's raw URL does not appear anywhere in the text (the secret check above covers it; the library's message is otherwise kept).
- The secret set is the D21 set: the built-in list plus anything the definition marks secret.

**D24 pre-send failures.**

- Missing credential: the call returns the outer `TransportError` with kind `MissingCredential`, carries request metadata, and the mock records zero requests.
- Page size above the declared maximum, an origin rejected by policy, and an invalid argument each return the outer `TransportError` with its kind and zero requests recorded.
- sniff's former "auth required" branch compiles and passes against the `TransportError` kind (verified by sniff's own build and tests).
- **NEEDS RULING:** the full kind list and which kind each pre-send case takes; a test cannot name a kind that is not ruled.

**D25 undecodable 2xx.**

- A mock answers a JSON endpoint with status 200, `Content-Type: text/html`, and a login page. The result is a Failure inside the envelope: status 200, kind `MalformedSuccess`, raw body and headers kept, `is_decode_failure()` true, and `into_result()` is `Err`.
- `is_decode_failure()` is false for a declared 4xx and for a successful decode.

**D26 cut-off body.**

- A mock sends headers (status 200, a `Content-Length` larger than what follows), writes part of the body, and closes the connection. The result is a Failure inside the envelope: kind `BodyInterrupted`, status and headers present, partial bytes equal to what was sent, `is_interrupted()` true.
- `is_interrupted()` is false for every other kind.
- Boundary of the governing rule: a connection refused before any response head arrives is the outer `TransportError`, not an envelope.

**D27 scoped sessions.**

- `with_session` closes the session on normal return, when the closure returns an error, and when it panics (the mock records exactly one close request in each case).
- No manual `open()` or `close()` exists on the generated pager, and no drop warning is emitted (a compile-level check of the public API; the prototype's `.close` knob case is removed).
- Cancellation is documented, not asserted closed: cancelling the caller's whole future leaves the mock's session open until its own lease expires, and no task keeps running (consistent with D13).

**D28 cross-origin policy.** One matrix over two mock origins (the base A and another origin B), run twice: once with B reached by a redirect and once with B reached by a next-page link.

- `same_origin`: B receives zero requests; a same-origin hop is followed.
- `allow_origins([B])`: B is followed and receives the credential; an unlisted origin (a sibling subdomain or a different port) receives zero requests.
- `allow_without_credentials`: B is followed and receives **no** credential. Every credential form is absent: the `Authorization` header, cookies, and a query-string key (this review's reading of "strip credentials"). A same-origin hop keeps its credential.
- `allow_any`: B is followed and receives the credential. The opt-in is explicit and clearly named (a compile-level check that no default or typo reaches it).
- `deny`: no redirect is followed and no cross-origin link is followed (zero requests beyond the first). **NEEDS RULING:** whether same-origin next-page links are followed under `deny`.
- Default: a definition that sets nothing behaves exactly as `same_origin`.
- Tighten-only: a caller override to `deny` takes effect; a caller attempt to loosen beyond the definition (for example `allow_any` over `same_origin`) is rejected. **NEEDS RULING:** whether rejection is a compile error or a runtime error, and the ordering among the five values.
- Hop-by-hop: a chain A to A to B is re-checked at each hop. **NEEDS RULING:** the hop limit and the method and body rules for 301, 302, 303, 307, and 308.
- Per-endpoint override: an endpoint-level policy replaces the API-level one for that endpoint only. **NEEDS RULING:** the override scope.
- Injected custom HTTP client: **NEEDS RULING** (refuse the policy, or accept it unchecked); no test until ruled.
- A refusal before sending is the outer `TransportError` with kind `CrossOriginRejected` and zero requests to the refused origin. **NEEDS RULING:** the shape when a redirect response head had already arrived (envelope with the 3xx status, or `TransportError`).
  *Update, 2026-10-11 (D30, D31, D32):* the injected-client item is closed (no injection, D31). The shape question is closed: a refused redirect (3xx head received) is a failure inside the envelope, kind `RedirectRefused`; a refused next-page link is the outer `TransportError`, kind `CrossOriginRejected` (D32). The redirect runtime and API-level policy are landing 1; the per-endpoint override is landing 2 (D30). The remaining NEEDS RULING items in this block (`deny` and same-origin links, the ordering among the five values, rejection form, hop limit, method and body rules, override scope) are planning-time details.

**D29 GraphQL errors and partial data.**

- A mock GraphQL server answers HTTP 200 with `data` holding a pull request and `errors` naming a forbidden check-status field. The result is a Failure inside the envelope: `Declared(GraphQlFailure)` with the errors and `partial_data` (the pull request), status 200, and the raw body kept. `?` and `into_result()` return `Err`.
- A 200 with `data` and no `errors` decodes as success.
- A 200 with `errors` and no `data` is a failure with `partial_data` absent (this review's reading of the shape).

### Criteria for D30-D48 (added 2026-10-11, second round)

Each is phrased so a test could decide it. Mock servers are local; nothing here contacts a real provider. "Sentinel token" means a distinctive fake credential value. A line marked **planning detail** names a detail the author left to planning, so the exact assertion is fixed then.

**D30 landing of cross-origin and GraphQL.**

- Landing 1 passes the whole D28 matrix (two mock origins, redirect and next-page link) with the policy set on the API definition. All 18 API definitions carry the `cross_origin` field; one that sets nothing behaves as `same_origin`.
- In landing 1 an endpoint cannot override the policy and no GraphQL operation primitive exists (a compile-level check of the public API); both exist after landing 2.
- Landing 2: an endpoint-level policy replaces the API-level one for that endpoint only (mock matrix again); the GraphQL criteria below pass.

**D31 no injected HTTP client.**

- The generated clients have no constructor that takes a `reqwest::Client` (`with_client` and `with_client_and_base_url` are absent from the public API; a compile-level check).
- Builder tests, each against a local mock: a proxy setting routes the request through a mock proxy; a custom root certificate lets a local TLS mock be reached and its absence fails; timeouts, user agent, and default headers are observed by the mock; two clients built from one pool share a connection (the mock counts connections); a test base URL redirects calls to the mock.
- No public path produces a client that follows redirects by itself (the cross-origin tests above pass for every constructor).
- The existing callers of the removed constructors build and pass their tests against the builder (see D31 for the two known call sites). **Planning detail:** the builder surface for accepting invalid certificates, which one call site uses today.

**D32 refused redirect or link.**

- A mock answers 302 to origin B under `same_origin`: the result is a failure inside the envelope with status 302, the `Location` value visible in the response headers, kind `RedirectRefused`; B records zero requests; `into_result()` is `Err`.
- A mock's first page carries a next-page link to origin B: the next call returns the outer `TransportError` with kind `CrossOriginRejected`; no request for that link was started.
- The rule is stated and tested as one question: if the call received an HTTP response, the result is an envelope; if not, it is the outer `TransportError`. **Planning detail:** what a refused redirect shows when the refusal happens at a later hop of a chain.

**D33 request-side features in landing 1.**

- The D10 criteria (text body, binary body, `ApiKeyParam` in query and cookie, header and cookie parameters) pass in landing 1 using purpose-built fixture definitions.
- Landing 1 changes no shipped endpoint definition for them (a diff of the 253 endpoint literals shows no change for these features).

**D34 GraphQL home.**

- The GraphQL operation primitive is in `schematic-define` and its generated client is in the same client crate as REST (no new package for GraphQL; a workspace package-list check).
- A GraphQL call through a mock shows the same credential resolution, a cross-origin refusal under `same_origin`, the shared deadline, and the envelope as a REST call. Pagination applies to a GraphQL operation that declares it. **Planning detail:** the pagination vocabulary for GraphQL connections.
- Typed variables and typed data bind through the D2 definition context: a variables value of the wrong type fails to compile, and a data type that was never registered is a validation error.

**D35 GraphQL address.**

- With base `https://api.github.com`, the GraphQL address is that base plus `/graphql`; with an Enterprise base ending in `/api/v3`, that ending is replaced by `/api/graphql` (two mock bases).
- A caller override wins over the rule.

**D36 same-address streaming and non-streaming.**

- Two endpoints on the same method and path, each declaring a selector (for example `stream=false` and `stream=true`), validate; two endpoints with the same (method, path, selector) fail validation; two endpoints on the same method and path with no selectors fail validation.
- Each generated request sends its selector value in the body (the mock records it).
- OpenAPI export produces one operation whose response lists both `application/json` and `text/event-stream`.

**D37 stream end.**

- With the end condition set to each form, a mock stream ends cleanly and the end marker is not delivered as an item: data line equals `[DONE]`; event named `message_stop`; field equal (`done=true`); connection close (the default).
- A mock that closes the connection before the declared end yields an explicit "ended early" error item, distinct from a clean end and from malformed items.

**D38 error shape mapping.**

- A mock answers a status matched by an exact entry, a range entry, and the API default: the exact entry wins over the range, and the range over the default.
- An endpoint can add an entry, override an entry, and opt out with the no-declared-errors form; an opted-out endpoint's 4xx is kind `Undeclared` with status, headers, and raw body.
- A status matched by nothing is kind `Undeclared` and keeps the raw body. OpenAPI export lists every declared status.

**D39 runtime package.**

- A hand-written runtime package exists; generated clients depend on it; the generator no longer emits the envelope, `TransportError`, redirect logic, deadline, pager, or redaction as template text into `schematic-schema` (a search of generator output for those items finds only references).
- **Planning detail:** how the generator and the runtime package check that their versions are compatible (also the interplay with D9's "incompatible shared-runtime change").

**D40 secret marking.**

- A query parameter named by an `ApiKeyParam` is redacted without any explicit marker.
- A header and a parameter marked with `.secret()` in the definition are redacted; a built-in well-known name is redacted without a marker; an unmarked, unknown name is shown.

**D41 `collect_all`.**

- `collect_all` returns success only after the mock's last page is observed.
- Each of `PageLimitReached`, `RepeatedContinuation`, `HttpError`, `DeadlineExceeded`, and `CrossOriginRejected` yields an `Incomplete` error that carries the reason and the items gathered so far. The page limit can be read and set by the caller. Single-page methods remain.
- **Planning detail:** an invalid or missing continuation has no reason in D41's list; the mapping is fixed at planning.

**D42 landing-1 pagination shape.**

- Landing 1 leaves the 253 endpoint literals unchanged; each of the 18 API definitions carries a default strategy and an exceptions table.
- A GET endpoint typed `Vec<T>` is a list; a list inside a wrapper, an opt-out, and an override each work through the table; validation rejects an exception id that matches no endpoint.
- Landing 2: each exception sits on its endpoint, the table is gone, and the generated Rust and exported OpenAPI are unchanged except for intended changes recorded in the implementation log.

**D43 automatic list recognition.**

- A POST returning an array is not a list unless the exceptions table says so; a GET returning `Vec<T>` is, with no table entry.

**D44 typed registration naming.**

- Defaults: the schema name and Rust path come from the type and the module the API states once. `Vec`, `Option`, and `HashMap` reuse the element's registration; `serde_json::Value` registers no component; an explicit alias resolves; two types that collide on a name are reported when the definition is built.
- **Planning detail:** how a `HashMap` key type and nested containers register.

**D45 `send_raw`.**

- `send_raw` returns status, headers, and bytes (and a JSON convenience) and passes through the same credential resolution (sentinel), cross-origin policy, deadline, and redaction as typed sends. Typed sends still fail to compile for a wrong response type.

**D46 order, single list, constant headers.**

- Landing 3 criteria do not depend on landing 4 and the reverse. A single generated list of APIs replaces the seven parallel lists (adding an API to the single list is the only registration step). A required constant header appears as an OpenAPI parameter.

> **Correction, 2026-10-11:** six runtime lists plus a test table, not seven; see the correction in section 4.3.

**D47 AsyncAPI exporter.**

- The exporter, in landing 4, exports the five WebSocket definitions using the REST types D8 shares; the existing AsyncAPI importer reads the result back into an equivalent WebSocket definition (a suggested check by this review). **Planning detail:** AsyncAPI version and the supported subset.

**D48 envelope details.**

- An endpoint with no declared error body: a declared-error value cannot be constructed (compile-level check on the uninhabited placeholder type), and a 4xx arrives as kind `Undeclared` with status, headers, and raw body.
- `request_id_header("x-github-request-id")` makes `meta.request_id` `Some(value)` when the mock sends that header and `None` otherwise; with no such definition field it is `None`.
- The reported auth method is exactly one of `PerCall`, `ClientLevel`, `EnvVar(name)`, `Anonymous`.
- Built-in redaction: the headers `Authorization`, `Proxy-Authorization`, `Cookie`, `Set-Cookie`, `X-Api-Key`, `X-Auth-Token`, `Api-Key` and the query parameters `api_key`, `apikey`, `access_token`, `token`, `key`, `signature` show `[redacted]`; extra names from D40 are added to that set.

### Criteria for D49 (added 2026-10-11)

- **Isolated compile check (landing 1).** Freshly generated output is compiled in an isolated crate with no network access (offline build against vendored or locally resolved dependencies); a failure to compile fails the test. Validation: a deliberately broken generator output fails it. It runs on a normal test run, not as an `#[ignore]`d test.
- **One module-path resolver (landing 4).** A single resolver serves Rust, OpenAPI, and Postman. A test feeds `PagerDutyApi`-style names (an API name with no explicit `module_path`) and asserts that Rust, OpenAPI, and Postman all resolve to the same file name.
- **Generated docs honesty (landing 4).** Generated documentation code fences are `no_run` and compile as doctests; no doc mentions a method that does not exist (the quick-start's convenience calls are checked against the generated client); no `///;` lines remain. Validation: a doc-compile test over the generated crate, and a grep for `///;`.
- **`oauth_scopes` and `auth_policy` (landing 2).** The choice is open, so no criterion fixes it. Whichever is chosen, the field is either read by generator or exporter code and covered by a test, or it no longer exists; a field that is neither read nor removed fails the review.
- **`ResponseKind` guard (landing 1).** Its disposition (kept, changed, or removed) follows the response model; the criterion is that the generated client's behavior on a response of an unexpected kind is covered by a test under the new model, or the guard is gone and the test list says so.

### Spike notes on the criteria (2026-10-11)

Evidence from the two prototypes (reported, not re-run by this review) sharpens these criteria. None changes a decision.

*Update, 2026-10-11.* Rulings D22-D29 resolve several notes below: the envelope sentinel limit and the `reqwest::Error` leak (D22, D23); failures before any request is sent (D24); GraphQL errors in a 200 (D29); the followed-link-when-policy-off case (D28); and the session-pager case (D27, which removes the manual `close()` the prototype tested). The notes are kept as the evidence behind those rulings; the criteria are in "Criteria for D22-D29" above.

- **Typed requests (D3).** The compile-fail criterion is demonstrable: the prototype's trybuild cases show a wrong success type, a wrong mutator type, a foreign request, an impl with a mismatched codec or missing associated type, and a missing free generic `request<T>` all failing to compile with readable errors. A hook on a text, binary, or stream endpoint is a compile error. Add a criterion that the cross-API error stays readable at 550 endpoints (the prototype's list is truncated at 550 variants); that cannot be tested until the real generator exists.
- **Envelope metadata and redaction (D7, D21).** The sentinel criterion "absent from any debug or display output of any envelope, error, or transport error" **cannot be met as written** when a provider echoes the token in an error body and that body is decoded into a typed error whose `Debug` prints it. It is testable only for stored metadata and `Display`, unless decoded bodies are hidden in `Debug` (Open Questions). A second criterion is needed: no stored value comes from `reqwest::Error`'s `Display`, since that includes the full URL.
- **Handling when `E` is not declared (D7).** Testable if the recommended uninhabited type is adopted: a compile-level check that a declared-error value cannot be constructed for such an endpoint, and a runtime check that a 4xx still yields status, headers, and raw body.
- **GraphQL errors in a 200 (D16, D17).** If the recommended placement is adopted, the "error side or metadata" criterion becomes: the envelope's failure side reports the GraphQL errors and any partial data, with status 200 and the raw body kept. This remains **NEEDS RULING** until the author accepts it.
- **Transport failure keeps request metadata (D7).** Add a case for a failure before any request is sent (missing credential): the criterion cannot say whether that is the outer `Err` or a failure inside the envelope until it is ruled.
- **D12 host policy.** The prototype covers differing port, scheme, and host on followed links. Add a case for a followed link when the policy is off, once the always-check question is ruled.
- **D13 deadline and cancel.** Evidence is HTTP/1.1 only. Add a session-pager case: a session opened and then the future cancelled leaves the server-side session open until its lease expires, so the criterion "no task keeps running" holds but "session is closed" does not.
- **D14 pagination.** The "unauthored knob does not compile" criterion has a working encoding (19 compile-fail and 2 pass cases for `.page_size`, `.page`, `.after`, `.before`, `.prev`, `.keep_alive`, `.close`, an unpaginated endpoint's pager, and a knob after `.pages()`). Out-of-range page size errors in the builder before any request. Stall cases (repeated cursor, cycle, repeated link, missing or invalid continuation) each return an explicit error and a failed pager stays failed. The "early drop" criterion remains **NEEDS RULING**. Add a binary-size budget only if the author wants one (the prototype measured about 1.77 MB at 157 endpoints; no budget is proposed here).

Update READMEs, current `schematic/docs/` topic pages, and affected skills alongside behavior changes. Topic pages explain behavior without linking back to this review. Record departures in the implementation log; the author controls lifecycle closure. Packages must continue to compile and work on macOS, Linux, native Windows, and WSL2 using the repository's existing checks. No performance spike is required by this review: it proposes no quantified performance claim that needs one. Streaming and shared-deadline work may introduce latency or memory expectations; none is stated here, and if implementation reveals a specific cost worth measuring, request an author decision rather than expanding hosts or statistical thresholds.

## 15. Decisions and Open Questions

### Decisions

D1-D4 were confirmed by the author on 2026-10-09 and D5-D8 the same day. D7 corrects one sentence of D3. D9-D16 were confirmed on 2026-10-09/10 and follow D8. Rejected alternatives are kept briefly so the reasoning is recoverable. D17-D21 were confirmed on 2026-10-10 and follow D16. D22-D29 were confirmed on 2026-10-11 after the spikes; D22 narrows D21, D28 amends D4 and D12, and D29 closes a D16/D17 detail.

*Added 2026-10-11 (second round).* D30-D48 were confirmed on 2026-10-11 after D22-D29. They answer the twelve rulings the frontmatter listed. D31 amends D4 and D12; D39 settles the envelope's home (D7); D42 amends D19; D36-D38 settle D6's open details; D44 settles D2's naming; D45 settles D3's custom-decoding path; D46 and D47 settle the open landing order and the AsyncAPI question (D8).


#### D1. Scope of the first implementation: the largest cut

**Decided:** everything in sections 5, 5b, 7, 8, 9, 12, and 13 is in scope; nothing is deferred. This covers (a) safety fixes (partial-generation cleanup, validation errors for unsupported features, isolated compile check of fresh output, duplicate `operationId`); (b) the sniff-driven runtime features of sections 12 and 13 (error headers and body, success headers, credential-source metadata, exact-host credential boundary, shared deadline, honest complete-pagination); (c) typed API wiring (D2, D3); and also endpoint and API builders; streaming responses (SSE/NDJSON) and response content types; status-to-schema responses and typed error bodies (the "vocabulary v2" change to the type of `Endpoint::response`); WebSocket fixes and redesign work (section 8, recommendation 13, and the REST/WS vocabulary unification of section 5b, which the author has not narrowed); and OpenAPI/Postman fidelity (schemars bridge, `x-schematic` round-trip, Postman example bodies).

**Rationale:** the author prefers to treat the review as one body of work rather than leave known fidelity gaps for later. How that work is ordered and packaged is open (section 13a).

#### D2. Typed schema registration: an explicit definition context

**Decided:** `schematic-define` provides a definition context that is passed through each definition function in `schematic-definitions`. One call registers a type's schema and returns a descriptor carrying a validated generated Rust path. Containers such as `Vec<T>` reuse their element registrations.

**Rationale:** it is the only option that directly connects the schema record and the Rust path record, which the current string-versus-string checks cannot.

- Pros: ownership is clear, no hidden global state, aliases and imported names can be represented explicitly.
- Cons: every definition function gains a context parameter or return value.
- Rejected: descriptors that carry schema data or a schema-producing callback (complicates serialization and equality, still needs separate Rust naming); a deriving proc-macro (new macro crate, and external types and aliases still need an explicit contract; may be reconsidered once the ordinary API has settled the naming rules); deferring.
- Open details: naming rules for aliases, external types, and container paths.
  - *Closed 2026-10-11 (D44).* Defaults come from the type; containers reuse the element's registration; `serde_json::Value` registers no component; aliases are explicit; collisions are reported at build.

#### D3. Request method: typed request only

**Decided:** each generated request type declares its response type, and its primary send method is typed by it. The free generic `request<T>` is removed from the public surface.

> **Correction by D7.** The original text said the send method "returns exactly the declared response type." That wording is withdrawn: the author does not want metadata excluded. Every send instead returns the `Response<T, E>` envelope defined in D7, where `T` is the declared success type. The "typed request only" decision itself stands.

**Rationale:** the project has no established external users, and the goal is enforceable typing; convenience methods alone would leave the wrong-type call compiling. `EndpointSpec` in the shared runtime already has an associated `Response` type with 550 generated impls, so the type link exists today and this change is cheaper than earlier text implied.

- Consequences: every call site changes (sniff; about 27 request call sites in other crates; the dependents are listed under D5); the generator must preserve concrete endpoint types rather than erase them into an enum.

> **Correction, 2026-10-11:** about 60 `.request(` / `request::<` call sites by raw grep, not 27 and not filtered to generated clients: sniff remote github 10, gitea 8, bitbucket 8, gitlab 7; homelab `eversolo.rs` 24, `samsung_tv.rs` 2; biscuit-speaks `elevenlabs.rs` 1. `with_client_and_base_url` is called at `samsung_tv.rs:120` and `eversolo.rs:62`.
- Rejected: typed conveniences plus a named unchecked decoding method (leaves a public escape); keeping `request<T>` with conveniences only (does not fix the defect).
- Open details: whether a separate path for custom decoding exists and what it looks like. **Interaction with D1:** streaming responses and status-to-schema responses must fit the typed-request design (the associated response type must be able to express them). How is open.

*Spike note, 2026-10-11 (typed-send).* The prototype shows the typed-request design works, including streaming and GraphQL, through one generic send with no separate `send_stream` (reported, not re-run by this review). That closes the "can the associated response type express streaming and several success shapes" half of the interaction with D1. Still open: the custom-decoding path (not tested); and the erasing request enum, which the prototype found is still needed for `into_parts` and per-client scoping, so "preserve concrete endpoint types" does not remove it (Open Questions). Hooks on text, binary, or stream endpoints become compile errors; today they are silently never run. See "Spike results (2026-10-11)".
*Update, 2026-10-11 (second round).* The custom-decoding path is `send_raw` (D45): status, headers, and bytes, with a JSON convenience, through the same credentials, cross-origin rule, deadline, and redaction; typed sends stay strict. Whether the erasing request enum remains is left to planning (D45).

#### D4. Redirects: a Schematic-managed transport for policy-enabled requests

**Decided:** requests that opt into the credential-boundary policy use a Schematic-managed transport built by the generated runtime with redirects disabled. A redirect becomes an explicit error. A supported builder path for TLS, proxy, and timeout settings is required. The existing client-injection API stays for callers who do not opt into the policy.

**Rationale:** denying redirects is sufficient for the provider migration and removes credential-boundary ambiguity that an arbitrary injected client would leave.

- Rejected: an explicit caller-supplied transport contract (enforcement relies on caller configuration); manual redirect following with per-hop policy checks (method rewriting, body replay, and redirect limits; reconsider only if a concrete consumer needs approved redirects).
- Open details: the builder configuration surface.

> **Amended by D28, 2026-10-11.** The rejection of "manual redirect following" no longer holds. The author ruled that the cross-origin policy (five values, authored in the definition) governs redirects, and values such as `allow_origins` and `allow_without_credentials` require Schematic to follow redirects itself, hop by hop. D4's "redirects disabled in a managed client" becomes **redirects disabled in the HTTP client, followed by Schematic according to the policy**. The requirement for a supported builder path for TLS, proxy, and timeout settings stands. The existing client-injection API conflicts with an always-present policy: reqwest fixes redirect behavior when the client is built, so an injected client cannot be policed (open under D28).
> **Amended again by D31, 2026-10-11.** "The existing client-injection API stays for callers who do not opt into the policy" is withdrawn: injection is removed and a Schematic-managed builder replaces it (D31). Because every client is Schematic-managed, the open question in the last sentence above is closed.

#### D5. Packaging: three landings

**Decided:** the work ships in three landings.

1. **Landing 1:** P1 safety plus P2 runtime evidence and policy, including D3 typed send, D4 transport, and the D7 envelope.
2. **Landing 2:** P3 typed definition API and builders, P4 response model v2 (D6), and the exporter and importer parts of P5 that depend on them, as a single pass over the 253 endpoint definitions.

> **Correction, 2026-10-11:** the count of 253 endpoint literals, and every ratio derived from it, is stale. The verified count is about 519 (see the correction in section 2). Read "253" in this place as "about 519".
3. **Landing 3:** P6 WebSocket (D8).

**Rationale:** safety and the shared runtime need no new definition vocabulary, so they can ship first; the vocabulary rewrite touches every definition once; WebSocket reuses the settled REST types.

- Pros: each landing is reviewable on its own; the 253 definitions are rewritten once; unsafe behavior is fixed earliest.
- Cons: dependents change twice (known cost). Landing 1 changes them once, for the envelope and typed send. Landing 2 changes them again, but only where response shapes change.
- Dependents of `schematic-schema` (verified by `Cargo.toml` search, 2026-10-09): path dependency of `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib` (no Rust references to `schematic_schema` found there), and five homelab crates (`homelab/lib`, `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, `unfolded-integration-helper`). About 27 request call sites sit outside sniff, and sniff has exhaustive error matches.

> **Correction, 2026-10-11:** dependents are eight crates by path dependency (the five homelab crates are `eversolo-integration`, `arcam-amp-integration`, `unfolded-integration-helper`, `lib`, `sony-receiver-integration`; plus `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib`, the last with a manifest dependency only). About 60 raw `.request(` / `request::<` call sites, not 27: sniff remote github 10, gitea 8, bitbucket 8, gitlab 7; homelab `eversolo.rs` 24, `samsung_tv.rs` 2; biscuit-speaks `elevenlabs.rs` 1.
- Rejected alternatives: not recorded in the confirmation. Alternatives such as one large change were not stated by the author, and none is invented here.
- Open details: the whole-work definition of done; whether P5 work not tied to P3/P4 (schemars bridge, `x-schematic`, Postman examples) belongs in landing 1, 2, or 3; and the landing of every item currently assigned to no package (listed in section 13a). Because D1 defers nothing, each must be assigned.

> **Amended 2026-10-10 (D18, D19, D20).** D18 adds a fourth landing (fidelity and hygiene) after landing 2 and assigns most of the open work items; D19 moves the pagination model and generated pagers from landing 2 into landing 1, so the claim above that landing 1 "needs no new definition vocabulary" and that the 253 definitions are rewritten once no longer hold in full; D20 supplies the whole-work definition of done. The heading ("three landings") and the text above are kept as decided on 2026-10-09.
> **Amended 2026-10-11 (D30, D33, D42, D46).** D30 places the redirect runtime and the API-level `cross_origin` field in landing 1 and the per-endpoint override and GraphQL in landing 2; D33 places the request-side features in landing 1; D42 removes the touch of the 253 literals from landing 1 that D19 had added; D46 orders landing 3 (WebSocket) before landing 4 (fidelity). Section 13a, "Current landing assignments", holds the current table.

#### D6. Response vocabulary (response model v2): success set plus error set

**Decided:** each endpoint declares one or more success shapes and a separate list of error shapes. A success shape is one of: JSON with a type, text, binary, empty, or a stream with SSE or NDJSON framing and an item type, each with an optional content type. The generated success type is a plain type for the common case, or a small generated enum when several success shapes exist. Declared error bodies decode on demand from the error. This replaces the type of `Endpoint::response`. It is breaking for both exporters, the importer, and all 253 definitions.

**Rationale:** it is the smallest model that covers the streaming, content-type, per-status, and typed-error gaps in section 5 and still yields one `T` and one `E` for the D7 envelope.

- Pros: the common case stays a plain type; error bodies are typed without losing raw evidence.
- Cons: breaking change across definitions, exporters, and the importer; a generated enum for multi-shape endpoints.
- Rejected alternatives: not recorded in the confirmation; none is invented here.
- Open details:
  - The streaming terminal marker (for example `[DONE]`).
  - How to express same-URL streaming and non-streaming endpoints (Ollama and Anthropic choose by a body field). The earlier draft forbids duplicate `(method, path)`, and a reviewer noted that two endpoint ids on one path conflict with that rule and with the OpenAPI exporter's `(path, method)` merge. This is an **open design detail and is not decided here**.
  - How error shapes map status codes and ranges.
  - *Closed 2026-10-11:* the end marker by D37, same-URL endpoints by D36, status-to-error mapping by D38. The entries are below.

#### D7. Result and error envelope (corrects D3's wording)

**Decided:** every send returns one envelope `Response<T, E>`, where `T` is the endpoint's declared success type and `E` its declared failure type. It is a struct with public metadata fields plus an inner enum (`Success(T)` or `Failure(E)`), read through `data()` and `error()` accessors that return `Option`, plus `into_result()`. Metadata is present in both the success and the error case: request headers (secret values redacted; never the token), response headers, query parameters, HTTP status, and the auth method used (anonymous, an environment variable name, or a caller-supplied label; never the credential value). A failure with no HTTP response at all (DNS, connection refused, timeout, credential-host check rejection, redirect denial) is the outer `Err` of `Result<Response<T, E>, TransportError>`, and that `Err` still carries the request-side metadata. Any HTTP status, including malformed or undeclared error bodies, is reported inside the envelope's error with the raw body retained.

**Rationale:** the author does not want metadata excluded from the result; evidence is needed in both outcomes (pagination headers on success, rate-limit headers on failure).

- Pros: one shape for every call; headers and status are always available; the raw body is never lost.
- Cons: every call site changes; callers must unwrap an envelope even for the simple case.
- Rejected alternatives: returning exactly the declared type with no metadata (the withdrawn D3 wording).
- Open details:
  - The exact metadata field list and names beyond those above (URL, elapsed time, request id, attempt or page number).
  - Redaction rules: which headers are secret, how redacted values appear, and query-string secrets such as `ApiKeyParam`.
  - The envelope's home crate: a shared runtime generated into `schematic-schema`, or a hand-written crate.
  - Whether a lighter "value only" convenience exists.
  - Exact handling when `E` is not declared.

*Update, 2026-10-11 (D24, D25, D26, D29).* The sentence above that makes credential-host rejection and redirect denial part of the outer `Err` is refined. The governing rule (D26): **an envelope exists whenever an HTTP response head was received; the outer `TransportError` means no response head arrived.** Failures before anything is sent (missing credential, page size above the maximum, origin rejected, invalid argument) are `TransportError` kinds with request metadata (D24). An undecodable 2xx is a failure inside the envelope with its real status, kind `MalformedSuccess` (D25). A cut-off body is a failure inside the envelope, kind `BodyInterrupted`, with status, headers, and partial bytes (D26). GraphQL errors in a 200 are `Declared(GraphQlFailure { errors, partial_data })` on the failure side (D29). Still open: the handling when `E` is not declared. Where a refused redirect or next link falls under the rule is also open (D28).

*Update, 2026-10-10 (D21).* The exact metadata field list and the redaction rules above are decided by D21. The envelope's home crate, a value-only convenience, and the handling of an undeclared `E` stay open here, as do D21's own open details.

*Spike note, 2026-10-11 (typed-send).* The envelope shape works in the prototype. Two open details gain a **recommended, unratified** answer: handling when `E` is not declared (an uninhabited `NoDeclaredError` type, so a declared-error value cannot exist; the raw body is still kept), and where GraphQL errors inside an HTTP 200 appear (the failure side, as a declared GraphQL failure carrying the errors and any partial data, with status 200 and the raw body kept). The prototype also found cases D7 does not yet cover: a 2xx body that fails to decode, a body that breaks after headers arrive, and failures before any request is sent. These are Open Questions, not decisions.
*Update, 2026-10-11 (second round).* The envelope's home crate is the new hand-written runtime package (D39). Handling when `E` is not declared is an uninhabited placeholder, so error replies arrive as kind `Undeclared` (D48). A value-only convenience is not ruled and stays a planning detail.

#### D8. WebSocket extent: fixes plus shared building blocks

**Decided:** fix the defects (wrong URL, reconnect counter reset, lifecycle replay, env auth, dropped diagnostics, the dormant Bearer-template compile error); emit the typed message enums; replace the `RequestEnvelope` string match with an explicit link; and make WebSocket reuse the REST parameter, auth-policy, env-mapping, and header types, replacing `ConnectionParam`/`ParamType`. `WebSocketApi` stays its own type with its own generator, and the five definitions are normalized. It lands last (landing 3).

**Rationale:** it removes the forked vocabulary and the worst defects without rewriting both trees.

- Pros: one vocabulary for parameters, auth, and headers; bounded scope.
- Cons: WebSocket and REST remain separate definition types.
- Rejected: full REST/WS unification under one definition type, for this work. An AsyncAPI exporter is **not decided**.
- Open details: whether the AsyncAPI exporter was in scope under D1 "everything" (a question for the author); the naming and shape of the explicit response link that replaces `RequestEnvelope`.
- *Closed 2026-10-11 (D47):* the AsyncAPI exporter is in scope, in landing 4, after WebSocket reuses the REST types. The shape of the explicit response link stays a planning detail.

### Decisions D9-D16 (confirmed by the author, 2026-10-09 and 2026-10-10)

The author confirmed these across 2026-10-09 and 2026-10-10. Each has a one-sentence rationale, rejected alternatives where the author chose among options, and the details still open. Where the confirmation did not record pros and cons, "Consequences" gives this review's reading; those lines are labelled and are not author statements.

#### D9. Partial generation

**Decided (ratifies the section 13 partial-generation contract):** regenerating one API regenerates its whole resolved module group, keeps every other module, rebuilds shared files and exports from the retained inventory, deletes only stale files it can prove it owns, and fails before writing anything if the inventory cannot be established safely.

**Rationale:** it removes the destructive `generate --api <one>` behavior (defect 2) without making single-API generation unusable.

- Example: selecting `OllamaNative` also regenerates `OllamaOpenAI` because they share a module, and keeps the other 16 APIs.
- Consequences (this review's reading): a single-API run may touch more than one API's files; the failure path leaves the directory untouched, so an unsafe run is loud rather than destructive.
- Rejected: refusing single-API generation unless the output is isolated; removing the single-API option.
- Open details: none recorded in the confirmation.

#### D10. Request-side feature support

**Decided (replaces section 13's "unsupported text/binary bodies receive explicit errors until implemented" and section 9 recommendation 3's "error until implemented" for these features):** Schematic should support every input type the supported APIs can include rather than reject it. Implement in the generated Rust client: text and binary request bodies (with content type), API-key-in-query and cookie auth (`ApiKeyParam`), and header and cookie parameters, each tested with local mock servers.

**Rationale:** the author's principle is to support what the supported APIs can send, not to turn a gap into an error.

- Backstop kept: per-output validation still applies. The whole batch for each output is validated before writing, an error names the API, endpoint, feature, and output, and one output's gap never blocks another output.
- Verified counts in the current definitions: JSON bodies about 312, multipart about 29, URL-encoded 0, text 0, binary 0, `ApiKeyParam` 0, header or cookie parameters 0, binary responses 33, text responses 17. No shipped definition exercises the zero-count features, so the tests need fixtures. (The July "six endpoints" figure in section 6 counted a different thing, the endpoints whose body was then dropped; the two numbers are not reconciled here.)

> **Correction, 2026-10-11:** the ten `FormData` literals in `openai/mod.rs` are current; text and binary request literals are 0; the July "six endpoints" figure is unverifiable.
- Consequences (this review's reading): more generator work in landing 1 and more surface to keep consistent with OpenAPI and Postman output, in exchange for no silent or loud gap for these inputs.
- Rejected: erroring until implemented as the end state (it stays only as the per-output backstop).
- Open: which landing. Landing 1 per D5 unless the author rules otherwise.
- *Closed 2026-10-11 (D33):* landing 1; generator-only work.

#### D11. Credentials

**Decided (ratifies and extends the credential-source contract):** callers can pass credentials programmatically at two levels, (1) client level, set once and used by all later requests, and (2) per call, overriding the client level.

**Rationale:** the author wants one predictable credential per request and evidence of which one was used.

- **Precedence:** per call, then client level, then environment policy, then anonymous.
- **Environment policy** applies only when nothing was set programmatically. The definition's ordered list of environment variable names is scanned once before sending, the first non-empty variable is selected, and exactly that one credential is sent. Callers may supply their own ordered list at client level.
- **Single attempt.** Schematic only ever attempts one credential. There is no chain of different keys and no automatic retry, including no silent anonymous retry, if it is rejected. A consumer may make a separate anonymous call and keep both results.
- **Reporting.** Every response (the envelope metadata of D7) reports the auth method used: per call, client level, the environment variable's name, or anonymous. It never reports the value. Credentials are resolved once per request, so the report matches what was sent.
- Example: `GITHUB_TOKEN` empty and `GH_TOKEN` set, no programmatic credential: `GH_TOKEN` is sent and the envelope says `GH_TOKEN`. If GitHub answers 401, one request has been made.
- Consequences (this review's reading): sniff's current anonymous fallback (gap A5) moves to the consumer; behavior that depended on a hidden retry changes.
- Rejected: exactly one environment variable per API; the environment off unless enabled; an opt-in anonymous fallback.
- Open details: none recorded in the confirmation beyond the D7 field list.

#### D12. Credential host policy

**Decided (ratifies the credential-host boundary contract):** credentials attach only if the URL's scheme, host, and effective port equal the configured base origin, or equal one of an optional list of additional exact origins the client declares (for example `https://uploads.github.com`). There are no wildcards and no domain matching. It applies to the first request and to every next-page or followed link. A mismatch is an outer `TransportError` (D7) returned before anything is sent, carrying request metadata. A client built without the policy performs no host check (today's behavior).

**Rationale:** a credential must not reach a host the caller did not name, including through a link the server supplied.

- Redirects: policy-enabled requests use the D4 transport with redirects disabled.
- Open details: the interaction with an injected custom HTTP client when the policy is enabled; the policy's builder name; whether a definition may carry known extra origins.

*Spike note, 2026-10-11 (pager-surface).* In the prototype, a followed link that differed in port, scheme, or host was rejected with zero requests reaching the other server, and a declared extra origin was followed (reported, not re-run). The spike recommends checking followed links against the origin **even when the policy is off**, which differs from D12's "a client built without the policy performs no host check". That is not decided; it is an Open Question.

> **Amended by D28, 2026-10-11 (supersedes the caller-configured shape above).** The cross-origin policy is authored in the API definition (per API, overridable per endpoint) and the generated client follows it. It governs redirects and followed next-page links. Five values: `same_origin`, `allow_origins([...])`, `allow_without_credentials`, `allow_any`, `deny`; default `same_origin`. The exact scheme+host+port comparison and the "additional exact origins" concept survive as `same_origin` and `allow_origins`; the credential-attachment rule remains (credentials attach only to allowed origins). A client built with no policy now gets the definition's policy rather than "no check", so the spike's "always origin-check followed links" recommendation is **superseded** and the question of what a policy-off client does is **closed**. A refusal before sending is the outer `TransportError` kind `CrossOriginRejected` (D24). Open details are under D28.
> **Amended again by D31, 2026-10-11.** The builder-name and custom-client questions are closed: no injected client exists, so the definition's policy always applies.

#### D13. Shared deadline

**Decided (ratifies the shared-deadline contract):** opt-in. A caller sets one overall deadline for an operation (all pages and follow-up requests) on a monotonic clock. Each request gets the remaining time. Once the time is spent, no further request is sent and a `DeadlineExceeded` error reports the completed pages and the elapsed time. Dropping the call cancels in-flight work and spawns no detached tasks. The existing 30 s per-request default is unchanged for callers who do not opt in.

**Rationale:** one operation that spans many pages needs one budget (gap A8), and callers who do not ask for it should see no change.

- Rejected: a per-request timeout only; an always-on default operation deadline.
- Open details: none recorded in the confirmation.

*Spike note, 2026-10-11 (pager-surface).* Evidence for the cancel and deadline criteria, HTTP/1.1 only (reported, not re-run): with 150 ms pages and a 400 ms budget the third request was cut off, the error reported 2 completed pages and the elapsed time, and nothing was sent afterwards; each request got the remaining budget; dropping the operation closed the server-side connection and the live task count went from 1 to 0. HTTP/2 was not tested; the spike suggests accepting "stream reset" as the HTTP/2 form of "connection close" in the cancel criterion (unratified). New details this raises, all open: per-request time as "remaining" versus "min(remaining, 30 s)", and whether closing a session pager gets time after the deadline. Also, a future being cancelled does not close a server-side session (see D14 note).

#### D14. Pagination

**Decided (ratifies the author's principles; supersedes the earlier vague "pagination helper" text and the old proposal for a complete-list helper).** The author confirmed these eight points:

1. **Primitives, authored by the definition.** Pagination is definable entirely with primitives in `schematic-define` and authored in `schematic-definitions`. It describes what the provider actually offers, and the author chooses among the mechanisms the provider offers. Schematic does not choose on the provider's behalf. Example: GitHub offers both a page parameter and a `Link` header, so the author picks.
2. **Lists are unpaginated by default.** An endpoint returning an array is a list. A response typed `Vec<T>` is recognized as a root-array list automatically (the author chose inference). A list inside a wrapper, for example `{"data": [...]}`, is marked with a `Collection::at(path)`-style marker. An API may declare a default strategy that applies to all its list endpoints, and any endpoint can override it or opt out.
3. **The generated client exposes only authored knobs.** In `schematic-schema` there is no page size, offset, cursor, or `next()` unless the definition provides that strategy. A missing knob is a compile error, not a runtime error.
4. **The model.** The design is [design.md](spikes/pagination/design.md): four strategy kinds (page number, offset, cursor, next-link) plus general dimensions: where inputs go; how to detect the end; item location with nesting; page-size rules; direction; sessions with open, keep-alive, renew, and close; composite cursors with tiebreakers; the follow-up request shape; mid-iteration page-size changes; terminal values; 0-based and 1-based offsets; and partitioned parallel iteration.
5. **General mechanisms, not fitted examples.** The author ruled that every pattern the research found must be handled by a general mechanism. The prototype passes 56 tests covering every inventoried strategy and 10 outside patterns. Six of those 10 were checked from memory, not fetched, and are **unverified** (X API v2, WordPress REST, OData `$skiptoken` in its non-Graph form, Azure Cosmos DB, Salesforce REST query, Reddit listings). The other four were fetched (Salesforce Bulk 2.0 through secondary sources, since the Salesforce page returned 403).
6. **Page sizes above the author-declared max return an error.** This is the default; the author did not veto it.
7. **Link following goes through the host policy of D12.**
8. **Labeled alternate strategies are supported in the model.**

**Rationale:** the current `PaginationStyle` is read as one boolean and the generator ignores the rest; the author wants one model that describes the provider honestly and a generated client that cannot be asked for something the provider does not offer.

Example of the authoring shape (from the spike, names illustrative until implemented):

```rust
// An API default (Gitea), inherited by every list endpoint
Strategy::page_number("page", MoreSignal::link_rel("next"))
    .page_size(PageSize::new("limit").server_default(30).max(50))
// A wrapped list that opts out
Endpoint { collection: Some(Collection::at("tree").unpaginated()), .. }
```

**Inventory facts (spike, 2026-10-09; confidence tags are in [inventory.md](spikes/pagination/inventory.md)):**

- About 157 collection endpoints across the 18 REST APIs.
- 35 declare pagination today, all page-number: Gitea 8, GitHub 9, GitLab 9, Bitbucket 9. (The inventory's cross-cutting notes say "27 declared" in one place; that is stale against its own table, which sums to 35. Section 7.1's "27 with pagination" is the July figure.)

> **Correction, 2026-10-11:** 35 is right; the raw `with_pagination` count of 27 missed the Bitbucket `bitbucket_pagination()` helper.
- OpenAI's 61 list endpoints declare none; it expresses pagination as ordinary query parameters.
- The generator ignores all pagination data today.
- Declared defaults differ from provider defaults (GitHub 100 vs 30, GitLab 100 vs 20, Bitbucket 50 vs 10, Gitea 50 vs 30). The new model records server defaults as documentation only.
- Not modelable, both provider or definition gaps: two OpenAI vector-store endpoints (the provider documents no continuation request) and EMQX `ListAuthUsers` (the definition has the wrong response type).
- Out of the pagination model: long-running job creation followed by polling (Salesforce Bulk, BigQuery). That belongs to a different feature.

- Consequences (this review's reading): the 35 existing declarations and `PaginationStyle` are replaced in landing 2 (*note 2026-10-10: landing 1 under D19; the builder rewrite of the same endpoints stays in landing 2*); every list endpoint of all 18 APIs needs a decision (inherit, override, or opt out); the model is proven only at the data-model level, since the async pager and the generator are not prototyped.
- Rejected alternatives are listed in design.md section 14 (for example one variant per provider, role-tagged input bags, and a global "adjustable size" flag); the author's confirmation did not add others.
- **Note on a divergence from the spike's recommendation:** design.md Q1 recommended explicit marking with a warning for unmarked `Vec<T>` responses, because inference would wrongly paginate a root-array write result such as Unfolded Circle `UploadResource`. The author chose inference. That risk is real and is recorded as open below.
- **Note on a possible overlap:** the prototype's `NextLink` carries its own link-trust setting (same origin by default, or an allow-list). The author's item 7 says links go through the D12 policy. D12 is opt-in, so whether a client without the policy follows a cross-host link, and whether the strategy's own setting survives, is open below.
- Open details:
  - Whether an auto-collecting "all items" helper exists and what it guarantees. The earlier proposed rule (success only after the end condition is observed, otherwise an explicit Incomplete result with the reason and the items so far) is **Proposed**; the author has not individually ratified it.
  - Behavior when a pager that holds a server-side session is dropped early. Rust has no async drop. The spike's recommendation is `#[must_use]`, a leak warning on drop, and also a scoped API that always closes; not ruled. *Closed by D27, 2026-10-11: only a scoped `with_session` is generated; no manual open/close and no drop warning.*
  - Verifying each provider's real pagination before authoring (a preparation task; the spike marks several rows unverified).
  - Inference limits: whether `Vec<T>` inference applies to non-GET endpoints, and how an inferred write result opts out when an API default exists.
  - Link trust: the D12 / `NextLink` overlap above. *Closed by D28, 2026-10-11: one cross-origin policy, authored in the definition, governs redirects and next-page links.*
  - Whether the generated client exposes labeled alternates (the model supports them; generation is not ruled).
  - *Added 2026-10-10 (D19):* how the inferred `Vec<T>` default is expressed on an endpoint literal in landing 1, before builders exist.

*Spike note, 2026-10-11 (pager-surface).* Point 3 ("only authored knobs; a missing knob is a compile error") is validated in a prototype that uses **one generic pager** for all four strategy kinds, with each knob guarded by its own trait bound (19 compile-fail and 2 pass cases; reported, not re-run). The pager's engine is not generic; the generator emits about 20 lines of constants and empty marker impls per endpoint. This narrows the generated-surface part of D14 item 3 to a concrete recommended encoding (unratified; see "Spike results"). The prototype also measured the early drop of a session pager, which D14 listed as open: scoped `with_session` closes on return, error, and panic; a drop warning leaks unless the caller calls `close()`; neither closes if the caller's future is cancelled. The spike recommends generating both. Still unratified. Cost found: at 157 endpoints the pager adds about 1.77 MB to a release binary, mostly a second copy of each item's deserializer (the reduction is proposed, not measured). The pager itself is therefore now prototyped, so the sentence "the async pager and the generator are not prototyped" in the Consequences bullet is out of date for the pager; the generator remains unprototyped.

*Update, 2026-10-11 (D27, D28).* The early-drop question is ruled by D27: **only** the scoped `with_session` is generated, and the spike's recommendation to generate both a drop warning and `with_session` was not adopted. The prototype's explicit `close()` therefore has no counterpart in the design. Link following is ruled by D28 (the definition's cross-origin policy).
*Update, 2026-10-11 (second round: D41, D42, D43).* The complete-list helper is ratified by D41. The inference limits are ruled: inference applies to GET endpoints only (D43), and in landing 1 the list shape is expressed by the automatic `Vec<T>` rule plus an API-level table of exceptions, not by a field on each literal (D42). The pager-lifecycle question that spiked remains as ruled by D27. The Consequences bullet above that says the 35 declarations are replaced in landing 2 is unchanged by D42 for the model itself; only the way landing 1 expresses exceptions changes.

#### D15. No public `Default` on definition types

**Decided (ratifies the first half of section 9 recommendation 1's proposal):** `Endpoint`, `RestApi`, and related definition types get constructors and builders, with no public `Default`. Required facts go in constructors, so an unusable definition cannot be written. Generated request types in `schematic-schema` keep their `Default`. The second half of that proposal (do not enforce equality between `schema_name` or `type_name` and Rust type names) is subsumed by D2 and is removed as a separate proposal.

**Rationale:** a `Default` endpoint or API presents an empty, unusable value as if it were valid.

Today an endpoint is an 11-line struct literal:

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

With a builder it is about 3 lines (method names illustrative; the real names, and how the response type binds through the D2 context, are not decided):

```rust
Endpoint::get("GetRepository", "/repos/{owner}/{repo}")
    .describes("Get repository metadata including default branch")
    .returns_json::<RepositoryInfo>(),
```

- Consequences (this review's reading): the constructor must take every required fact, so the set of required facts for `Endpoint` and `RestApi` needs to be fixed during landing 2.
- Rejected: a public `Default`; `Default` only behind a test feature.
- Open details: none recorded in the confirmation; naming of builder methods follows D2's open naming work.

> **Correction, 2026-10-11:** `EndpointParams` derives `Default` (`define/src/params/mod.rs:86`). D15 covers `Endpoint` and `RestApi` only, so `EndpointParams` is explicitly allow-listed unless spec 4a rules otherwise (D52).

#### D16. GraphQL

**Decided (replaces the section 13 proposal that said "no generic GraphQL compiler is proposed"):** the author chose **full GraphQL support**: typed queries, and GraphQL-aware error extraction in the response envelope. Errors that arrive inside an HTTP 200 body must be surfaced on the envelope's error side or in its metadata; the exact semantics are open.

**This is new scope.** Before this decision the spec's boundary stopped at "a GraphQL operation can be sent as a JSON POST and the consumer interprets it". Typed queries and envelope-level GraphQL error handling go beyond that boundary, and they add work to a body of work that D1 already made very large.

**Narrowed by D17 (2026-10-10).** The author chose to author operations as strings with typed variables and typed response data. Schematic does not parse or validate GraphQL, so "typed queries" above means typed variables and response data, not a query checked against a provider schema. The open item "how a query is authored" is closed; the other open items below remain open.

**Rationale:** the author wants GraphQL providers (GitHub is the motivating case) supported by the generated client rather than left to each consumer.

- Rejected: nothing GraphQL-specific; only a per-endpoint base URL rule.
- Open (each needs its own ruling):
  - How a query is authored: raw strings, typed documents, or schema-driven code generation. *Closed by D17, 2026-10-10: strings in the definition.*
  - Where it lives: `schematic-define`, a new crate, or the consumer.
  - Handling of partial data plus errors in one response (success, failure, or both in the envelope). *Closed by D29, 2026-10-11: failure side, with partial data.*
  - The GitHub.com versus Enterprise difference: REST under `/api/v3`, GraphQL under `/api/graphql`, so one definition needs a base override.
  - Which landing (section 13a: landing 2 unless ruled otherwise; placement open).
  - *Closed 2026-10-11 (D30, D34, D35):* landing 2; `schematic-define` plus `schematic-gen`; the address is derived by a rule with a caller override.

### Decisions D17-D21 (confirmed by the author, 2026-10-10)

The author confirmed these on 2026-10-10 after reviewing options. Each entry gives the decision, a rationale, the options with condensed pros and cons, the rejected alternatives, and the details still open. The confirmation recorded which option the author chose and which were rejected; lines labelled "this review's reading" are this review's condensation of the options and are not author statements.

#### D17. GraphQL authoring (narrows D16)

**Decided:** the author chose option A of three. A GraphQL operation is a string held in the definition (a `GraphQlOperation`-style primitive; the name is illustrative until implemented), with typed variables and typed response data. GraphQL errors that arrive inside an HTTP 200 body are surfaced in the response envelope (D7, D21). Schematic does not parse or validate GraphQL: the string is sent as written and the provider's server is the validator.

**Rationale (this review's reading):** it gives GraphQL providers such as GitHub a generated, typed call without making Schematic own a GraphQL parser, a committed copy of each provider's schema, or a second generator.

Options offered:

- **A (chosen): queries are strings in the definition.** Pros (this review's reading): smallest surface; no GraphQL tooling in the build; works with any provider. Cons: a typo in a query, or a mismatch between a query and its response type, is found at run time, not at build time.
- **B (rejected; this review had recommended it): query files validated against a committed provider schema, with generated types.** Pros: invalid queries and mismatched types fail at build time. Cons: each provider's schema must be committed and refreshed, and the build needs a GraphQL parser, validator, and type generator.
- **C (rejected): whole-API generation from an imported GraphQL schema.** Pros: complete coverage of a provider's graph, as OpenAPI import does for REST. Cons: the largest scope; in effect a second generator pipeline.

**Consequences:** "typed" in D16 narrows to variables and response data. Because nothing validates the operation, the response type an author declares is a promise the server can break; that failure surfaces as a decode error in the envelope.

**Open details:**

- Where the primitive lives: `schematic-define`, a new crate, or the consumer.
- Partial data plus errors in one response: success, failure, or both. *Closed by D29, 2026-10-11: failure side as `Declared(GraphQlFailure { errors, partial_data })`.*
- GitHub.com versus Enterprise: REST under `/api/v3` and GraphQL under `/api/graphql`, so one definition needs a URL override.
- Which landing (landing 2 unless ruled otherwise, per D16).
- *Closed 2026-10-11 (D30, D34, D35):* the primitive's home, landing, and the GitHub.com versus Enterprise address are all ruled.

#### D18. Four landings (amends D5)

**Decided:** a fourth landing is added after landing 2, for fidelity and hygiene work that does not change the definition vocabulary:

- OpenAPI schemars-bridge fixes
- the `x-schematic` round-trip
- Postman example bodies
- README doctests
- moving vendor pagination presets out of the neutral crate
- per-API feature flags in the generated crate

Items that change the vocabulary stay in landing 2: typed path parameters; per-endpoint auth override, `deprecated`, rate limits, and retry; retiring `env_auth[0]` as the Basic-auth password; polymorphic `oneOf` bodies; serializable definitions; OAuth2 per-grant payloads; and query-parameter `required`, style, and enum fidelity. WebSocket remains its own landing. The relative order of the WebSocket landing and the new fidelity landing is **open**; this document calls them landing 3 (WebSocket) and landing 4 (fidelity) unless the author rules otherwise.

**Rationale (this review's reading):** the vocabulary-changing items belong with the one pass over the 253 definitions; the fidelity items can follow without a second pass, which keeps landing 2 from absorbing every remaining item.

Options offered:

- **A fourth landing for fidelity and hygiene (chosen).** Pros: landing 2 stays focused on the vocabulary; each landing remains reviewable. Cons: one more landing, with another round of dependent builds under D20.
- **Everything into landing 2 (rejected).** Pros: no extra landing. Cons: landing 2 becomes the largest and least reviewable change.
- **The author assigning each item individually (rejected).** Pros: maximum control. Cons: many separate rulings; the author chose a rule instead.

**Open details:**

- The order of landing 3 and landing 4.
- The single generated list of APIs (replacing the seven parallel lists) was not assigned; it remains an open assignment.

> **Correction, 2026-10-11:** six runtime lists plus a test table, not seven.
- Not named by D18 and therefore unassigned: required constant headers as OpenAPI parameters (section 9, recommendation 7).
- *Closed 2026-10-11 (D46):* the single generated list and required constant headers go to landing 4; landing 3 (WebSocket) precedes landing 4.

#### D19. Pagination in landing 1 (amends D5 and the D14 placement)

**Decided:** the pagination model (definition primitives) and the generated pagers land in landing 1, so sniff gets real pagers immediately.

**Known cost, stated plainly.** Each of the 253 endpoint literals gets one extra field in landing 1 (the collection marker) and is rewritten again to builders in landing 2. Landing 1 therefore also grows. The landing 2 claim of a single pass over the 253 definitions describes the builder rewrite only.

**Rationale (this review's reading):** sniff's provider migration (D20) needs pagers, and landing 1 is where the sniff-facing runtime ships.

Options offered:

- **Pagination in landing 1 (chosen).** Pros: sniff gets generated pagers at once; the D14 model is exercised early. Cons: a second touch of all 253 literals; a larger landing 1.
- **Pagination entirely in landing 2, with sniff hand-looping on envelope headers (rejected; this review had recommended it).** Pros: landing 1 stays smaller; the 253 literals are touched once. Cons: sniff keeps hand-written paging loops until landing 2.

**Open details:** because `Vec<T>` inference marks lists (D14) and landing 1 predates builders, how the inferred default is expressed on a literal.
*Amended 2026-10-11 by D42.* The known cost stated above is removed: landing 1 adds no field to the 253 literals. The inferred default is expressed by automatic `Vec<T>` recognition for GET endpoints (D43), an API-level default strategy, and one table of exceptions keyed by endpoint id on the API definition (18 places); validation rejects an id that matches no endpoint; in landing 2 the builders move each exception onto its endpoint and the table is removed.

#### D20. Whole-work done

**Decided:** the work is done when all of the following hold:

1. All four landings pass their section 14 criteria.
2. Every dependent crate (sniff, biscuit-speaks, model-citizen, and the five homelab crates) builds and passes its tests after each landing.

> **Correction, 2026-10-11:** eight crates by path dependency; see the correction under D5.
3. Every defect-ledger item (section 10) and every recommendation (section 9) is closed or recorded as an accepted gap in the implementation log.
4. sniff retires its hand-written GitHub and GitLab client (`FocusedProviderClient`) and uses the generated clients, passing sniff's own tests.

The author closes the work.

**Rationale (this review's reading):** items 1-3 show the review's findings were handled; item 4 proves the generated clients can replace a real hand-written stack.

Options offered:

- **The four-part definition including the sniff proof (chosen).** Pros: a concrete, checkable end state; proves the generated clients on a real consumer. Cons: it ties this spec's completion to the sniff migration, which has its own spec (`2026-10-09-consistent-use-schematic`); a delay or defect there holds this work open.
- **Items 1-3 without the sniff proof (rejected).** Pros: this work could close independently. Cons: leaves the question "do the generated clients replace sniff's client?" unanswered.
- **No whole-work definition (rejected).** Pros: none beyond less up-front work. Cons: "done" stays a matter of opinion.

**Open details:** none recorded in the confirmation.

#### D21. Envelope metadata (resolves part of D7's open field list)

**Decided:** the envelope carries:

- status
- HTTP method
- full URL, with secrets in the query redacted
- request headers, redacted
- response headers
- query parameters
- auth method used: per-call, client-level, environment variable name, or anonymous; never the value
- elapsed time
- page number, when the call is part of a paginated operation
- the provider's request id, when the definition names that header

**Redaction:** a built-in list (`Authorization`, `Cookie`, `Set-Cookie`, common API-key headers) plus any header or query parameter the definition marks secret. Redacted values appear as `[redacted]`, and real values are never stored in the envelope.

**Rationale (this review's reading):** pagination, rate-limit diagnosis, support requests, and credential audits each need evidence beyond status and headers; storing only redacted values keeps the envelope safe to log.

Options offered:

- **The full set above (chosen).** Pros: one envelope serves pagination, diagnosis, and audit. Cons: more fields to keep stable; redaction must be correct for every one.
- **A minimal set without URL, method, timing, page number, and request id (rejected).** Pros: smaller, simpler surface. Cons: callers rebuild the missing evidence themselves, inconsistently.

**Open details:**

- The definition field that names the request-id header.
- The representation of the auth method (also open under D11).
- Whether secret marking is per parameter or per auth strategy.
- Raised by this review, not by the author: the exact "common API-key headers" list, and whether response headers other than `Set-Cookie` are covered by the same redaction (the list includes `Set-Cookie`, a response header, which suggests yes).

*Spike note, 2026-10-11 (typed-send).* The prototype stored redacted URL, headers, and query in the envelope and kept stored metadata and `Display` clean. It found two limits on the sentence "real values are never stored" and on criterion "the sentinel is absent from every ... debug output": (1) if a provider echoes the credential in an error body, the decoded error type's `Debug` shows it, so the guarantee cannot cover decoded bodies unless they are hidden in `Debug` or the guarantee is narrowed to stored metadata; (2) `reqwest::Error`'s `Display` includes the full URL, so it must never be stored (today's `SchematicError::Http` would leak a query-string API key). The generator must also split query parameters out of the request parts (today they are formatted into the path) for redaction to work. All three are Open Questions; D21 is not changed.

*Update, 2026-10-11 (D22, D23).* Limits (1) and (2) are ruled. For (1), D22 narrows the promise: stored metadata and `Display` are guaranteed credential-free; decoded error bodies are not. For (2), D23 keeps the URL in network-error text, composed from Schematic's own redacted URL plus the library's message with its raw URL removed. The third item (the generator must split query parameters out of the request parts) remains open as S6 and is now a prerequisite for D23.
*Update, 2026-10-11 (second round: D40, D48).* The open details are ruled: the request-id header is named by `request_id_header(...)`; the auth method is `PerCall | ClientLevel | EnvVar(name) | Anonymous`; secret marking is automatic from auth strategies, an explicit `.secret()` marker, and a built-in list of well-known names (D40), whose contents are fixed by D48. Whether response headers other than `Set-Cookie` share the redaction is not ruled (planning detail).

### Decisions D22-D29 (confirmed by the author, 2026-10-11)

The author confirmed these on 2026-10-11 after reviewing the spike results and the options this review offered. Each entry gives the decision, a rationale, the options offered with the one chosen, the rejected alternatives, and the details still open. Lines labelled "this review's reading" are this review's condensation or inference and are not author statements. Where the confirmation recorded no open details, the entry says so; anything this review added is labelled "raised by this review".

> **Known limitation, stated up front (D22).** After D22, the credential-free guarantee covers only Schematic's **stored metadata** and its **`Display` output**. A provider's decoded error body is printed as received by `Debug`. If a provider echoes a credential in an error message (for example a 401 whose body says `Bad credentials for token sk-abc123`), logging the error with `{:?}` can leak that credential. Do not log `Debug` output of error values at a level or to a destination that is not allowed to hold credentials. This limitation must be repeated in the generated crate's documentation for the error types and in the `docs/` topic page for the envelope.

#### D22. Echoed secrets: narrow the promise (amends D21 and D7)

**Decided:** the author chose option A of three. Only Schematic's **stored metadata** (the envelope's URL, headers, query parameters, auth method, and the rest of D21's fields) and its **`Display`** output are guaranteed free of credentials. A provider's decoded error body is printed as received in `Debug`. A provider that echoes a credential in an error message can therefore leak it through `Debug` logging. The raw body stays in the error as evidence (D7); Schematic does not alter it.

**Amendment to D21's sentinel criterion:** the sentinel token must be absent from stored metadata and from `Display`. It is **explicitly not guaranteed** for `Debug` of decoded error bodies. D21's sentence "real values are never stored in the envelope" stays true for the metadata Schematic itself records; it makes no claim about a provider's body.

**Rationale (this review's reading):** hiding or rewriting a provider's body would remove or distort the evidence D7 exists to keep, and no rewrite can reliably catch a credential that the provider has transformed.

Options offered:

- **A (chosen): narrow the promise to stored metadata and `Display`.** Pros (this review's reading): keeps the body exactly as received; the promise Schematic makes is one it can actually keep. Cons: a real leak path remains through `Debug` logging of decoded error bodies, and callers must know it.
- **B (rejected; this review had recommended it): hide error bodies in `Debug`, printing only the kind and the size.** Pros: no credential can reach `Debug` through a body. Cons: `Debug` output becomes less useful when debugging provider errors. The author preferred seeing the body.
- **C (rejected): scrub the exact credential value from bodies.** Pros: keeps most of the body readable. Cons: breaks the "raw body" evidence, and misses echoes the provider has transformed (encoded, truncated, or partially masked).

Example: a mock server answers 401 with `{"message":"Bad credentials for token <sentinel>"}`. The envelope's metadata and `Display` never contain the sentinel; the raw body does, and `{:?}` on the decoded error prints it.

**Open details:** none recorded in the confirmation.

#### D23. URL in network-error text: keep it, with secrets masked (resolves S2)

**Decided:** the author wants the URL **kept** in network-error text for debugging, with secrets masked. Schematic composes the error text itself from (1) its own redacted URL, in which the values of secret query parameters show as `[redacted]` (which parameters are secret comes from D21: the built-in list plus anything the definition marks secret), and (2) the HTTP library's message with the library's raw URL removed. The library's own error text, which includes the full URL, is never stored or shown. A sentinel-key test must prove that a query-string key never appears in any error text.

Example (wording illustrative): `error sending request for https://api.example.com/v1/items?api_key=[redacted]&page=2: connection refused`.

**Rationale:** a network failure is the case where the URL helps most, and today's `SchematicError::Http` would print a query-string API key through the library's `Display`.

Options offered:

- **Compose the text from Schematic's redacted URL plus the library message without its URL (chosen).** Pros (this review's reading): the URL stays visible for debugging; secrets are masked by the same rules as the envelope. Cons: Schematic must strip the library's URL from its message reliably, and the sentinel test must guard that.
- **Convert to Schematic's own error kinds only (rejected).** The URL would be lost from the text.
- **Put the URL only in a field (rejected).** The text a person reads in logs would lack it.
- **Keep the library's text as-is (rejected).** It includes the raw URL and so any query-string secret.

**Consequence (raised by this review):** the redaction depends on the generator separating query parameters from the path (S6, still open as generator work); D23 makes that split a prerequisite, not an option.

**Open details:** none recorded in the confirmation.

#### D24. Failures before anything is sent: the outer `TransportError` (resolves S3)

**Decided:** failures that happen **before anything is sent** are the outer `TransportError`, with specific kinds (for example `MissingCredential`, `CrossOriginRejected`, `InvalidArgument`, `Timeout`), and they still carry request metadata. The cases named were: a missing credential, a page size above the declared maximum, a host or origin rejected, and an invalid argument. **Consequence:** sniff's "auth required" case moves to the `TransportError` kind.

**Rationale (this review's reading):** with no request sent there is no status or response headers to put in an envelope, and the envelope rule (D26) is then simple to state.

Options offered:

- **Outer `TransportError` with specific kinds (chosen).** Pros (this review's reading): one place for everything that happened before a response; request metadata is still available. Cons: sniff's exhaustive matches change.
- **A separate third error layer (rejected).** More types for callers to handle.
- **Failures inside the envelope with no status (rejected).** An envelope would exist without a response, which weakens "envelope means a response was received" (D26).

**Open details (raised by this review):** the complete kind list and the exact kind for each pre-send case (the confirmation gave examples only; in particular which kind a page size above the maximum takes). Whether a timeout belongs in this group when part of the request was already sent is also not stated.

#### D25. Undecodable 2xx: a failure inside the envelope, status kept (resolves S5)

**Decided:** a 2xx reply whose body cannot be decoded is a **Failure inside the envelope** with the real status kept (for example 200), kind `MalformedSuccess`, the raw body and headers kept, plus an `is_decode_failure()` helper.

Example: a corporate proxy intercepts the call and returns its HTML login page with status 200. The caller sees status 200, kind `MalformedSuccess`, the HTML in the raw body, and `is_decode_failure()` returns true, so the caller can tell "the server sent something else" from "the API said no".

**Rationale (this review's reading):** an envelope exists because an HTTP response arrived (D26); the evidence (status, headers, body) is the most useful thing for diagnosing a proxy or a changed API.

Options offered:

- **Failure inside the envelope with status kept (chosen).** Pros (this review's reading): all evidence retained; a 200 that decodes to nothing cannot be mistaken for success. Cons: a 2xx is now on the failure side, so `into_result()` returns `Err` for it, which surprises callers who equate failure with a 4xx or 5xx.
- **Outer `TransportError` (rejected).** Would hide the status and headers.
- **A success carrying raw bytes (rejected).** `?` would pass an undecoded reply as success.

**Open details:** none recorded in the confirmation.

#### D26. Cut-off body: a failure inside the envelope (resolves S4)

**Decided:** a reply whose body is cut off after the headers arrived is a **Failure inside the envelope**, kind `BodyInterrupted`, with the status, headers, and the **partial bytes** received, plus an `is_interrupted()` helper for retry code.

**Governing rule (ratified with D25):** an envelope exists whenever an HTTP response head (status line and headers) was received. The outer `TransportError` means **no response head arrived**.

**Rationale (this review's reading):** retry logic needs the status and headers (for example `Retry-After`) even when the body breaks, and the rule gives one test for where a failure lives.

Options offered:

- **Failure inside the envelope (chosen).** Pros (this review's reading): evidence retained; the rule is mechanical. Cons: sniff currently treats the same event as a transport error, so its matches change.
- **Outer `TransportError` carrying partial response details (rejected).** Would make `TransportError` sometimes mean "a response arrived", blurring the rule.

**Open details (raised by this review):** none from the confirmation. One case the rule does not settle is listed in Open Questions: a redirect or next-page link that the cross-origin policy (D28) refuses after a redirect response head has already arrived.

#### D27. Abandoned paging sessions: a scoped helper only (closes the D14 early-drop detail)

**Decided:** the author chose option B. The generator emits **only** a scoped helper, `with_session(|pager| async { ... })`, which always closes the server-side session when the closure returns, errors, or panics. There is **no manual `open()`/`close()`** and **no drop-warning path**. (The pager-surface spike and this review had recommended generating both; the author decided against.)

Example (names illustrative until implemented):

```rust
client.list_events().with_session(|pager| async move {
    while let Some(page) = pager.next().await? { /* use page */ }
    Ok(())
}).await?;
// The session is closed here whether the closure returned, failed, or panicked.
```

**Known consequences, stated plainly:**

- Flows that need a session held across several functions (open in one place, keep paging in another, close later) **cannot use sessions**.
- If the caller's whole task is **cancelled externally**, nothing can send the close call (Rust has no async drop, and D13 forbids detached work). The **server's own timeout is the backstop**. Example: Elasticsearch's default limit is 500 open scroll contexts; a program that abandons sessions faster than their lease expires can hit that limit.

**Rationale (this review's reading):** one way to use a session is harder to misuse than two, and a drop warning cannot actually close a session.

Options offered:

- **A (rejected; recommended by the spike and this review): generate both the scoped helper and a manual open/close with a drop warning.** Pros: supports sessions held across functions. Cons: the manual path leaks unless the caller remembers to close it, and a warning does not close anything.
- **B (chosen): the scoped helper only.** Pros: a session cannot be forgotten in normal use. Cons: the two consequences above.

**Open details:** how `with_session` is shaped for an async closure that borrows the pager. Raised by this review: whether closing a session gets time after the D13 deadline has expired (S10 remains open).

#### D28. Cross-origin policy lives in the API definition (amends D4 and D12)

**Decided:** the policy for following a request to a **different origin** is authored in the API definition and the generated client follows it. It is a `schematic-define` primitive, set per API and overridable per endpoint (for example a `cross_origin(...)` builder; the name is illustrative). It governs **both redirects and followed next-page links**. This supersedes the caller-configured shape in D4 and D12.

The author chose all five values:

| Value | Meaning |
|---|---|
| `same_origin` | Follow only to the same origin. Anything else is refused. |
| `allow_origins([...])` | Follow to the listed exact origins (scheme, host, effective port); credentials are also sent to those. |
| `allow_without_credentials` | Follow to any origin; strip credentials when the origin differs. |
| `allow_any` | Follow anywhere **with** credentials. A risky escape hatch: it requires an explicit, clearly named opt-in. |
| `deny` | No redirects and no followed cross-links. |

- **Default** when the definition says nothing: `same_origin`.
- **Caller override:** callers can **tighten** the policy (for example to `deny`) but never loosen it beyond what the definition says.
- Example (names illustrative): GitHub's definition says `allow_origins(["https://uploads.github.com"])`; a caller that does not want that can set `deny` for one client, and cannot set `allow_any`.

**Known cost, stated plainly.** D4 rejected "manual redirect following" as too complex. These values require Schematic's runtime to follow redirects itself, hop by hop: a hop limit; re-checking each hop's origin against the policy; stripping credentials when leaving the origin; and method and body rules for 301, 302, 303, 307, and 308. D4's "redirects disabled in a managed client" therefore becomes **"redirects disabled in the HTTP client, followed by Schematic according to the policy."** This is real added work in the runtime, in the same landing as the transport.

**Relationship to D12.**

- The exact scheme+host+port comparison and the "additional exact origins" concept survive, as `same_origin` and `allow_origins`.
- D12's credential-attachment rule remains: credentials attach only to allowed origins.
- A client built with no policy now gets the **definition's** policy (default `same_origin`) instead of "no check". D12's "a client built without the policy performs no host check" no longer holds.
- A refusal before sending is the outer `TransportError` with kind `CrossOriginRejected` (D24).

**Rationale (this review's reading):** the people who know which origins an API legitimately uses (for example GitHub's upload host) are the definition's authors, and putting the rule there makes it reviewable in one place and the same for every consumer.

Options offered:

- **Policy authored in the definition, with the five values (chosen).** Pros (this review's reading): one reviewable source; a safe default. Cons: the redirect-following runtime described above.
- **Caller-configured only (rejected).** This was the D4 and D12 shape: enforcement depended on every caller configuring it.
- Separate policies for redirects and for followed links were not offered or discussed. One policy governs both.

**Open details:**

- Per-endpoint override scope.
- The redirect hop limit.
- Method and body rules for 301, 302, 303, 307, and 308.
- Interaction with an injected custom HTTP client: reqwest fixes the redirect policy when the client is built, so an injected client cannot be policed. The choices are to refuse the policy with such a client or to accept it unchecked.
- Naming.
- How the policy appears in the generated client's public API.
- Raised by this review: the order of "tighter than" among the five values is not a simple line (for example, `allow_origins` sends credentials to listed origins, while `allow_without_credentials` reaches any origin but sends none), so the tighten-only rule needs a defined comparison. Whether a caller may narrow an `allow_origins` list to a subset is also unstated.
- Raised by this review: whether `deny` still follows same-origin next-page links. The ruling says "no followed cross-links", which reads as cross-origin only; same-origin paging would otherwise stop working.
- *Update, 2026-10-11 (D30, D31, D32):* landing 1 holds the redirect-following runtime and the API-level policy; the per-endpoint override is landing 2 (D30). The injected-client question is closed by D31. The shape of a refusal is ruled by D32. The other items above remain planning-time details.

#### D29. GraphQL errors and partial data: the failure side (closes the D16/D17 open detail)

**Decided:** errors and partial data inside an HTTP 200 GraphQL reply go on the **failure** side as `Declared(GraphQlFailure { errors, partial_data })`, with status 200 and the raw body kept. `?` and `into_result()` therefore cannot silently ignore them, and partial data is available deliberately to the caller who asks for it.

Example: a pull request loads but its check-status field is forbidden to the token. The server answers 200 with the pull request in `data` and an entry in `errors` for the check-status path. The caller sees a failure whose `partial_data` holds the pull request and whose `errors` explains the forbidden field.

**Rationale (this review's reading):** GraphQL servers report field-level problems with a success status, so a success-side design lets a caller read half a result without noticing.

Options offered:

- **Failure side with errors and partial data (chosen).** Pros (this review's reading): impossible to ignore by accident; partial data is still reachable. Cons: a reply that is mostly good is on the failure side, so callers who want it must match on the kind.
- **Success side with an `errors()` accessor (rejected).** `?` would pass a partial result as success.
- **A per-operation strict or lenient choice (rejected).** More surface, and an easy wrong default.

**Open details:** none recorded in the confirmation. The remaining GraphQL details (home of the primitive, the GitHub.com versus Enterprise URL, the landing, binding to the D2 context) stay open under D17.
*Update, 2026-10-11 (D30, D34, D35).* The remaining GraphQL details are ruled: landing 2 (D30), home in `schematic-define` and `schematic-gen` (D34), address rule plus caller override (D35), and binding of typed variables and data through the D2 context (D34).

### Decisions D30-D48 (confirmed by the author, 2026-10-11, second round)

The author confirmed these on 2026-10-11 after the D22-D29 round. They answer the twelve rulings the frontmatter used to list. Each entry gives the decision, a rationale, the options offered and rejected with a one-line reason, consequences, and the details left to planning. **Convention:** the decisions and the rejected options come from the author's confirmation. The rationale and the one-line reasons for rejection are this review's reading unless a line says otherwise; the author's own stated reasons are marked as such. Consequences and remaining details are this review's reading, except where they restate the author's confirmation.

#### D30. Stage for cross-origin and GraphQL

**Decided:** landing 1 builds the redirect and link-following runtime (D28) and the API-level `cross_origin` policy field (18 API definitions touched). The per-endpoint `cross_origin` override, the GraphQL primitive, and D29's error handling land in landing 2.

**Rationale (this review's reading):** the credential boundary must exist from the first runtime, while overrides and GraphQL are definition vocabulary that fits the builder pass. The chosen option is the one this review recommended.

Options offered:

- **Runtime and API-level field in landing 1; override and GraphQL in landing 2 (chosen).**
- **Both in landing 2, with `same_origin` hard-wired in landing 1 (rejected).** Reason (reading): landing 1 would ship a boundary no definition can configure, so an API such as GitHub's upload host could not state its origins until landing 2.
- **Both fully in landing 1 (rejected).** Reason (reading): it pulls all of GraphQL, a new feature, into an already large landing.

**Consequences:** landing 1 includes real redirect-following work (hop limit, per-hop origin checks, credential stripping, method and body rules); 18 API definitions gain one field; until landing 2 an endpoint cannot differ from its API's policy.

**Remaining details (planning):** per-endpoint override scope; the ordering among the five values for the tighten-only rule; redirect hop limit; method and body rules for 301, 302, 303, 307, and 308; whether `deny` still follows same-origin next-page links; naming; how the policy appears in the generated client's public API.

#### D31. No injected HTTP client (amends D4 and D12)

**Decided:** the generated clients' `with_client(reqwest::Client)` injection is **removed**. A Schematic-managed builder covers proxy, TLS root certificates, timeouts, user agent, default headers, a shareable connection pool, and a base URL for tests. The cross-origin rule therefore always holds.

**Rationale:** reqwest fixes redirect behavior when a client is built, so an injected client cannot be policed. Evidence cited by the author: no crate in the repo calls `with_client` on the generated clients (grep, 2026-10-11).

Options offered:

- **Remove injection; managed builder (chosen).**
- **Keep injection under a name where the caller promises redirects are off (rejected).** Reason (author): unverifiable.
- **Keep injection and document that the policy does not apply (rejected).** Reason (reading): it recreates the bypass the cross-origin rule exists to close.

**Consequences:** cases needing middleware or exotic network settings wait for builder support or hooks (the author's statement). `with_client` and `with_client_and_base_url`, which the generator emits today, are removed; D4's "existing client-injection API stays" is withdrawn.

> **Evidence note by this review, 2026-10-11 (not a change to the ruling).** The grep covered the name `with_client`. The sibling constructor `with_client_and_base_url` is called on generated clients at `homelab/lib/src/eversolo.rs` (line 62) and `homelab/lib/src/samsung_tv.rs` (line 120), both passing a hand-built `reqwest::Client` with a 5-second connect timeout and a 10-second request timeout; the Samsung call site also calls `danger_accept_invalid_certs(true)` when HTTPS is used. Both must move to the builder. The D31 builder list names TLS root certificates but not "accept invalid certificates", so the Samsung call site is not covered by the list as recorded. This is left to planning (below), not decided here.
>
> **Author ruling after the evidence note, 2026-10-11.** Asked again with the corrected evidence, the author confirmed that D31 stands (removal of client injection), and that the Schematic-managed builder gains: a base URL, timeouts, and an explicitly named, opt-in "accept invalid certificates" setting for local devices such as the Samsung TV (self-signed certificates on a home network). The two homelab call sites (`eversolo.rs`, `samsung_tv.rs`) are migrated to the builder in landing 1. The cross-origin rule still always holds. Rejected: a constructor taking a caller-built client under a "redirects disabled" promise; injection only for clients Schematic can verify. Remaining builder details (names, surface) stay in planning item P3.

**Remaining details (planning):** the builder's exact surface and names; whether and how an accept-invalid-certificates setting is offered (the Samsung call site needs one or a different solution); the later hooks mechanism for middleware; migration of the two homelab call sites.

#### D32. Refused redirect or link

**Decided:** one test governs: **did this call receive an HTTP response?** A redirect refused by the policy (a 3xx was received) is a failure **inside the envelope**, with the redirect status and `Location` visible, kind `RedirectRefused`. A refused next-page link (no request for it was started) is the **outer `TransportError`**, kind `CrossOriginRejected`.

**Rationale (this review's reading):** it keeps D26's governing rule mechanical and keeps the evidence (status, `Location`) when a response did arrive. Closes R2.

Options offered:

- **Redirect inside the envelope; link outer (chosen).**
- **Both outer (rejected).** Reason (reading): it would hide the 3xx status and `Location` that a received response carries.
- **Both inside the envelope (rejected).** Reason (reading): no response head exists for a refused link, so an envelope would exist without a response, which weakens D26.

**Consequences:** `ErrorKind` gains `RedirectRefused`; D24's list of pre-send `TransportError` cases applies to links, not to redirects; the earlier list that put "redirect denial" among the no-response cases is superseded.

**Remaining details (planning):** what the envelope shows when the refusal happens at a later hop of a redirect chain (the last hop's response is the natural reading).

#### D33. Request-side features in landing 1 (settles D10's landing)

**Decided:** text and binary request bodies, API key in query or cookie auth, and header and cookie parameters are **generator-only** work (no definition edits) and land in landing 1.

**Rationale (this review's reading):** no shipped definition uses them (D10), so they need no pass over the definitions.

Options offered: landing 1 (chosen); landing 2 (rejected; reading: it would delay features sniff-style consumers can use now for no saving, since no definition edit is involved); landing 4 (rejected; reading: the same, and landing 4 follows WebSocket).

**Consequences:** landing 1 generator work grows; tests need purpose-built fixtures (D10).

**Remaining details:** none recorded.

#### D34. GraphQL home

**Decided:** GraphQL lives inside `schematic-define` (a GraphQL operation primitive) and `schematic-gen` (generated into the same client crate), reusing the envelope, credentials, cross-origin rule, deadline, and pagination. Typed variables and data bind through the D2 definition context.

**Rationale (this review's reading):** it reuses every guarantee the REST path already has, without a second package to keep in step.

Options offered: inside `schematic-define` and `schematic-gen` (chosen); a new GraphQL package (rejected; reading: a second runtime and generator to keep consistent with REST); a generic typed POST only, with conventions left to the consumer (rejected; reading: it would not deliver the typed GraphQL D16 chose).

**Consequences:** landing 2 (D30); D17 (strings, no validation) and D29 (failure side) stand.

**Remaining details (planning):** the primitive's name; the pagination vocabulary for GraphQL connections; batching (not addressed by any decision).

#### D35. GraphQL address

**Decided:** the definition carries a rule that derives the GraphQL address from the client's base address (GitHub.com: append `/graphql`; an Enterprise base ending in `/api/v3`: replace that ending with `/api/graphql`), **and** the caller may override it.

**Rationale (this review's reading):** the rule gives the right default for both GitHub flavors; the override covers hosts the rule cannot know.

Options offered: rule plus caller override (chosen); rule only (rejected; reading: no escape for an unusual host); caller-supplied only (rejected; reading: every consumer would repeat GitHub's knowledge).

**Consequences:** closes D17's address detail.

**Remaining details (planning):** how the rule is represented in the definition and how it recognizes the Enterprise base.

#### D36. Same-address streaming and non-streaming (resolves a D6 open detail)

**Decided:** two endpoint definitions on the same method and path, each declaring a **selector** (a request-body field value, for example `stream=false` versus `stream=true`). The uniqueness rule becomes (method, path, selector). OpenAPI export merges them into one operation whose response lists both content types (`application/json`, `text/event-stream`).

**Rationale (this review's reading):** Ollama and Anthropic choose by a body field; this keeps two typed endpoints while keeping the OpenAPI exporter's one-operation-per-(path, method) merge.

Options offered: two endpoints with a selector (chosen); one endpoint with an enum response (rejected; reading: it would hide the stream/non-stream choice from the type); one endpoint with named variants (rejected; reading: a new concept on top of D6's success set).

**Consequences:** the validator's duplicate rule changes; the exporter merges selector-distinguished endpoints.

**Remaining details (planning):** selector syntax; naming of the two generated request types; how the merged OpenAPI request schema shows the selector.

#### D37. Stream end (resolves a D6 open detail)

**Decided:** the definition declares the end condition from a small vocabulary: **data line equals X** (OpenAI `[DONE]`), **event named X** (Anthropic `message_stop`), **field equals X** (Ollama `done=true`), or **connection close** (the default). A stream that closes before its declared end yields an explicit "ended early" error item.

**Rationale (this review's reading):** the three real providers each end differently, and a vocabulary avoids hard-coding one.

Options offered: a declared end condition (chosen); hard-code `[DONE]` (rejected; reading: wrong for two of the three providers); no end rule (rejected; reading: a cut-off stream would look like a clean end).

**Consequences:** the definition vocabulary grows; the stream item type gains an "ended early" error.

**Remaining details (planning):** exact names of the end-condition forms and the shape of the "ended early" item.

#### D38. Error shape mapping (resolves a D6 open detail)

**Decided:** the API declares a default error list of (status or range → shape); endpoints add, override, or opt out (a `no_declared_errors` form); the most specific wins (exact status over range over default); an unmatched status is an `Undeclared` failure that keeps the raw body.

**Rationale (this review's reading):** most providers share one error shape per API with a few exceptions.

Options offered: API default plus endpoint changes (chosen); per-endpoint lists only (rejected; reading: repeats the shared shape on every endpoint); one error shape per endpoint (rejected; reading: cannot express different shapes per status).

**Consequences:** supersedes the spike's endpoint-level status range (S8).

**Remaining details (planning):** range syntax and the treatment of overlapping ranges.

#### D39. Runtime home

**Decided:** a new hand-written package (working name `schematic-runtime`) holds the shared runtime: the response envelope and metadata, `TransportError`, redirect-following and cross-origin logic, the shared deadline, the pager, and redaction. Generated clients depend on it and stay thin.

**Rationale (this review's reading):** hand-written, testable code instead of template text; one place for the guarantees every client shares.

Options offered: a new package (chosen); keep generating the runtime as template text into `schematic-schema` (rejected; reading: logic that must be correct would be tested only through regenerated text); put it in `schematic-define` (rejected; reading: the definition layer would depend on HTTP behavior). **Cost (author):** one more package and coordinated versions with the generator.

**Consequences:** settles D7's home-crate question; D9's "shared runtime change" now includes a version check between generator and runtime package; this review reads landing 1 as the landing that creates the package.

**Remaining details (planning):** final name; versioning; exactly which items (the `Endpoint` trait, codecs) live in the runtime versus generated code.

#### D40. Secret marking

**Decided:** secrets are marked automatically from auth strategies (for example an `ApiKeyParam`'s name), by an explicit `.secret()` marker on any header or parameter in the definition, and by a built-in list of well-known names.

**Rationale (this review's reading):** auth strategies catch the common case, the marker catches provider-specific secrets, and the list catches well-known names.

Options offered: all three (chosen); auth strategies plus the built-in list only (rejected; reading: no way to mark an unusual secret); explicit marker only (rejected; reading: every definition author must remember it).

**Consequences:** the redaction set is the union of the three sources.

**Remaining details (planning):** the marker's exact form on headers versus parameters.

#### D41. `collect_all` helper (ratifies the previously Proposed rule)

**Decided:** a `collect_all` helper exists. It succeeds only after the provider's last page is observed; otherwise an explicit `Incomplete` error carries the reason (`PageLimitReached`, `RepeatedContinuation`, `HttpError`, `DeadlineExceeded`, `CrossOriginRejected`) and the items gathered so far. The page limit is caller-visible.

**Rationale (this review's reading):** a helper advertised as complete must not return a partial list as a success.

Options offered: the helper with `Incomplete` (chosen); no helper (rejected; reading: every consumer rewrites the loop); a helper returning items plus a completeness flag (rejected; reading: the flag is easy to ignore).

**Consequences:** the earlier proposal named "invalid continuation" as a reason; D41's list has none for it.

**Remaining details (planning):** which reason an invalid or missing continuation takes (or whether a reason is added); the default page limit.

#### D42. Landing-1 pagination shape (amends D19; removes its known cost)

**Decided:** landing 1 adds **no** new field to the 253 endpoint literals. A response typed `Vec<T>` is recognized as a list automatically. The API definition declares a default pagination strategy and one table of exceptions keyed by endpoint id (a list inside a wrapper, opt-outs, overrides), in 18 places. Validation rejects an exception id that matches no endpoint. In landing 2 the builders move each exception onto its endpoint and the table is removed.

> **Correction, 2026-10-11:** the count of 253 endpoint literals, and every ratio derived from it, is stale. The verified count is about 519 (see the correction in section 2). Read "253" in this place as "about 519".

**Rationale (this review's reading):** it gives sniff real pagers in landing 1 without touching the 253 literals twice.

Options offered: the table (chosen); the collection field on all 253 literals now and again in landing 2 (rejected; that was D19's accepted cost).

**Consequences:** D19's "known cost" is removed; landing 2 is the only pass over the 253 endpoints.

**Remaining details (planning):** validation messages; the table's form and how it interacts with the API default.

#### D43. Automatic list recognition

**Decided:** only **GET** endpoints are recognized as lists automatically. Other methods (a POST search, writes that return arrays) are lists only if the exceptions table says so.

**Rationale (this review's reading):** it removes the risk, recorded under D14, of inferring a list from a write result such as Unfolded Circle's `UploadResource`.

Options offered: GET only (chosen); all methods with an opt-out (rejected; reading: a write result would be paginated by default).

**Consequences:** closes D14's inference-limits detail.

**Remaining details:** none recorded.

#### D44. Typed registration naming (resolves D2 open details)

**Decided:** defaults come from the type itself (schema name from the type; Rust path from a module the API states once, for example `types_in(...)`). Containers (`Vec`, `Option`, `HashMap`) reuse their element's registration. Free-form `serde_json::Value` registers no component. Aliases are stated explicitly (for example `ctx.alias::<Commit, CommitInfo>()`). Name collisions are reported when the definition is built.

**Rationale (this review's reading):** it removes the double typing of names that D2 identified, with explicit escapes for the odd cases.

Options offered: defaults plus explicit aliases (chosen); the author writes both names every time (rejected; reading: it keeps the string-versus-string weakness); a derive macro per type (rejected; D2 had deferred macros).

**Consequences:** fixes the naming D15's builder methods depend on.

**Remaining details (planning):** nested container and `HashMap` key registration; the format of collision reports.

#### D45. Raw escape hatch (resolves D3's open detail)

**Decided:** a clearly named `send_raw` returns status, headers, and bytes (with a JSON convenience) through the same credentials, cross-origin rule, deadline, and redaction. Typed sends stay strict. Whether the erasing request enum remains is an implementation detail left to planning (the spike found it still needed for `into_parts` and per-client scoping).

**Rationale (this review's reading):** an escape that goes through the same guarantees is safer than an unchecked decoder.

Options offered: `send_raw` (chosen); no escape hatch (rejected; reading: consumers would reach for a second client); a per-call decoder override (rejected; reading: it reopens the unchecked-type problem D3 closed).

**Consequences:** closes the custom-decoding detail; S13 narrows to the enum question.

**Remaining details (planning):** the signature; the erasing enum.

#### D46. Stage order

**Decided:** WebSocket is landing 3 and fidelity and polish is landing 4. The single generated API list (replacing seven parallel lists) and required constant headers as OpenAPI parameters go to landing 4.

> **Correction, 2026-10-11:** six runtime lists plus a test table, not seven; see the correction in section 4.3.

**Rationale (this review's reading):** WebSocket reuses the REST types once they settle (D8); the two items had been unassigned.

Options offered: WebSocket then fidelity (chosen); fidelity before WebSocket (rejected; reading: no dependency favors it); no committed order (rejected; reading: leaves the plan open).

**Consequences:** closes the order question and the two unassigned items.

**Remaining details:** none recorded.

#### D47. AsyncAPI exporter

**Decided:** in scope, in landing 4, after WebSocket reuses the REST types.

**Rationale (this review's reading):** D1 deferred nothing, and WebSocket definitions are otherwise invisible to external tools.

Options offered: in scope in landing 4 (chosen); in landing 3 (rejected; reading: the REST types it relies on settle in landing 2 and WebSocket reuse lands with landing 3); out of scope (rejected; reading: contradicts D1).

**Consequences:** closes D8's AsyncAPI question; landing 4 grows.

**Remaining details (planning):** AsyncAPI version and supported subset.

#### D48. Envelope details (resolves D7 and D21 open details)

**Decided:**

- An endpoint with no declared error body uses an uninhabited placeholder error type, so a `Declared` error cannot exist; error replies arrive as kind `Undeclared` with status, headers, and raw body.
- The definition names the provider's request-id header (for example `request_id_header("x-github-request-id")`); `meta.request_id` is `Some` or `None` accordingly.
- `AuthMethod = PerCall | ClientLevel | EnvVar(name) | Anonymous` (never a value).
- Built-in secret names: headers `Authorization`, `Proxy-Authorization`, `Cookie`, `Set-Cookie`, `X-Api-Key`, `X-Auth-Token`, `Api-Key`; query parameters `api_key`, `apikey`, `access_token`, `token`, `key`, `signature`. The set is extensible per D40.
- Redacted values show `[redacted]`.

**Rationale (this review's reading):** it settles the fields that D21 left open so the envelope and the redaction test have fixed content.

Options offered: the author's confirmation recorded the chosen shapes; no rejected options were recorded.

**Consequences:** the list includes `Set-Cookie`, a response header, so the redaction covers at least that response header.

**Remaining details (planning):** whether other response headers share the redaction; the placeholder type's name; a value-only convenience (unruled).

### Decision D49 (confirmed by the author, 2026-10-11)

#### D49. Loose-item assignments

**Decided:**

1. The isolated compile check of freshly generated code (recommendation 4, ledger row 16) lands in **landing 1**.
2. Unifying the two divergent module-path resolvers (ledger row 22, recommendation 10) lands in **landing 4**.
3. The generated-docs honesty pass (ledger row 8, recommendation 12: `no_run` fences, a correct quick-start, no `///;` lines, typed convenience methods where applicable) lands in **landing 4**.
4. Whether `Endpoint::oauth_scopes` and `RestApi::auth_policy` are wired in or removed is **decided during landing 2**, while the definitions are rewritten. The choice itself is still open; its landing is fixed.
5. The `ResponseKind` runtime guard in the generated client follows the new response model (D3, D6, D7) in **landing 1**: it is kept, changed, or removed as that model requires.
6. Two stale July figures get correcting notes: section 7.1's "27 with pagination" (the inventory counts 35 endpoints with declared pagination, all page-number: gitea 8, github 9, gitlab 9, bitbucket 9) and section 6 item 1's "six endpoints POST empty bodies" (a July figure, partly fixed; the verified 2026-10-11 counts are text 0, binary 0, `ApiKeyParam` 0, header or cookie parameters 0, multipart about 29; the two measures are not reconciled, and the live count is established when D33 is implemented).

D1 and D20 require every ledger row closed or recorded as an accepted gap. That is why each of these items needed a landing: an unassigned row cannot be closed.

**Rationale (one sentence each):**

1. The compile check guards every later generator change, so it must exist before them.
2. The resolver unification is fidelity work that does not change the definition vocabulary, which is landing 4's scope.
3. The docs pass is fidelity and hygiene work on generator output, which is landing 4's scope.
4. The two fields live on the definitions that landing 2 rewrites, so the choice is cheapest there.
5. The guard checks response kinds, which the response model defines, so it moves with that model.
6. Stale figures mislead planning and estimates, so they are corrected in place with dated notes.

**Options offered:**

- **Assign each item to a landing now (chosen).** Pros: D20 can be checked row by row; nothing waits on planning. Cons: items 1 and 5 add to the already large landing 1 (P11).
- **Leave the items unassigned until planning (rejected).** Pros: none beyond less up-front work. Cons: D20 cannot be checked against unassigned rows.

**Consequences:** landing 1 gains the isolated compile check and the guard decision; landing 4 gains the resolver unification and the docs pass; landing 2 carries one more decision (item 4).

**Open details:** the wire-in-or-remove choice for `oauth_scopes` and `auth_policy` (item 4).


### Decision D50 (confirmed by the author, 2026-10-11)

#### D50. Plan structure: this spec is the parent of seven child specs

**Decided (option A of three):**

1. This review spec stays as the **parent**: it holds the decisions and the evidence. The work is carried out by **seven child specs**, run strictly in the order below, each with a 1:1 plan.
2. Each plan is implemented end to end without human review, then goes through a review/fix cycle until it is deemed production ready.
3. Each child spec must leave the whole repo green: it builds on macOS, Linux, native Windows, and WSL2 (repo rules), and every dependent crate builds and passes its tests. Each ends at "implementation complete, ready for review". An agent never moves a spec to `_completed`; the author does.
4. Child specs live under `schematic/features/YYYY-MM-DD-name/`, dated at creation.
5. The seven, in order:
    1. **Generation integrity.** D9 partial generation, per-output validation, D49 fresh-output compile check, duplicate `operationId` fix, D33 request-side features (text and binary bodies, `ApiKeyParam`, header and cookie parameters), query-parameter split. Generator only; no dependent crate changes.
    2. **Runtime and envelope.** New `schematic-runtime` package (D39); typed send and the wrapper (D3, D7, D21, D48); credentials (D11); Schematic-managed builder replacing client injection (D31), including migrating the homelab `eversolo` and `samsung_tv` call sites; cross-origin runtime plus the API-level policy field (D28, D30, D32); shared deadline (D13); `send_raw` (D45); the D49 `ResponseKind` guard disposition. This is the big breaking change: all dependents migrate.
    3. **Pagination.** Model in `schematic-define` (D14, D42, D43), generated pagers in the runtime, `collect_all` (D41), `with_session` (D27). Depends on spec 2.
    4. **Definition API.** Builders (D15); typed registration (D2, D44); per-endpoint cross-origin override (D30); the vocabulary additions assigned to landing 2 (typed path parameters, per-endpoint auth override, `deprecated`, rate limit and retry vocabulary, `env_auth[0]` retirement, `oneOf` bodies, serializable definitions, OAuth2 grants, query-parameter fidelity); the `oauth_scopes`/`auth_policy` choice (D49). **Condition:** the builders' surface (for example `.returns(...)` and `.errors(...)`) must be shaped in spec 4 for spec 5's response vocabulary, so the 253 definitions are not rewritten twice.
    5. **Response model v2 and GraphQL.** Success and error sets (D6, D36, D37, D38); GraphQL (D17, D29, D34, D35). Depends on spec 4.
    6. **WebSocket.** D8, D46. Independent of REST internals once spec 4 lands.
    7. **Fidelity and polish.** OpenAPI bridge, `x-schematic` round trip, Postman example bodies, AsyncAPI exporter (D47), feature flags, single API list and constant headers (D46), module-path resolver unification and docs honesty pass (D49), README doctests, vendor pagination move.
6. **Outside the seven:** sniff retiring its hand-written GitHub and GitLab client (D20) is tracked by sniff's own spec (`sniff/fixes/2026-10-09-consistent-use-schematic`) and starts after spec 3. It counts toward whole-work done.
7. **Mapping to landings:** landing 1 = specs 1, 2, 3; landing 2 = specs 4, 5; landing 3 = spec 6; landing 4 = spec 7.
8. **Before each plan runs**, its spec needs a short pass to settle its planning-time details (the P1-P12 items in Open Questions, mapped in section 13a), and the author must pre-authorize the steps that would otherwise pause an unattended run: regenerating `schematic-schema`; creating the `schematic-runtime` package; deleting the stale `schematic/gen/schematic` tree; editing dependent crates (sniff, biscuit-speaks, model-citizen, the five homelab crates); and, for spec 4, rewriting the 253 endpoint definitions.

**Rationale:** the blast radius of each unreviewed run stays small, and each spec leaves the tree green.

**Options offered:**

- **A. Seven child specs under this parent (chosen).** Pros: small blast radius per unreviewed run; each spec leaves the tree green. Cons: more specs and more review cycles; a cross-spec condition on the builders (spec 4 for spec 5).
- **Four specs, one per landing (rejected).** Landings 1 and 2 are too large for an unreviewed run.
- **Five specs, merging 1+2 and 4+5 (rejected).** The merged specs are still too large for an unreviewed run.

**Consequences:** D5's landings stay as the grouping and ordering; the seven specs are the unit of planning and execution. P11 (size of landing 1) is answered by the split of landing 1 into specs 1, 2, and 3.

**Open details:** child spec names and dates are assigned at creation. The owner of the P items marked "owner to assign" in section 13a is not yet decided.

> **Update, 2026-10-11 (D52).** Item 5 now lists eight specs: item 4 (Definition API) is split into 4a and 4b. Item 7 reads: landing 1 = specs 1, 2, 3; landing 2 = specs 4a, 4b, 5; landing 3 = spec 6; landing 4 = spec 7. The pre-authorization to rewrite endpoint definitions in item 8 is for spec 4a and covers about 519 literals, not 253. The "condition" in item 4 (builders shaped for spec 5's response vocabulary) binds spec 4a. The original text above is kept.

### Decision D51 (confirmed by the author, 2026-10-11)

#### D51. Draft all seven child specs now, and the per-spec process

**Decided (option B of three):**

1. **All seven child specs are drafted now.** They are `schematic/features/2026-10-11-schematic-generation-integrity`, `-schematic-runtime-envelope`, `-schematic-pagination`, `-schematic-definition-api`, `-schematic-response-model-graphql`, `-schematic-websocket`, and `-schematic-fidelity-polish`, in the order of D50.
2. **Per-spec process.** For each child spec, in order:
    1. Implement all phases of its plan, phase by phase, without a human in the loop.
    2. **Review 1.** Time is deliberately set aside for the author to review the decisions made during implementation and any open design questions.
    3. Refresh the **next** spec and its plan from the outcomes of review 1.
    4. Loop fix and review cycles until the reviewer deems the spec "production ready".
3. **Specs 2-7 are drafts.** Their frontmatter carries `refresh_required: true`. Each is refreshed after the previous spec's review 1, so drafting all seven now is for visibility and dependency checking, not a commitment to their final text.
4. **Every child spec carries a "Process" section and a "Human review agenda".** The unattended implementer logs decisions to `implementation-log.md` so review 1 has them.
5. **Every planning-time detail has a stated default.** The implementer adopts the default and logs the choice, so no run pauses for a question.

**Rationale:** drafting all seven exposes cross-spec dependencies early, while the refresh step keeps later specs from being frozen on assumptions that review 1 may overturn.

**Options offered:**

- **A. Draft only spec 1 now and the rest later (rejected).**
- **B. Draft all seven now (chosen).**
- **C. Stop after the parent (rejected).**

**Consequences:** D50's seven specs and order are unchanged; D51 adds the drafting and the process. Drafts of specs 2-7 are not authoritative until refreshed.

**Open details:** how review 1 findings are fed to the next spec (the refresh) is performed on the author's instruction at that time.

### Decisions D52-D54 (confirmed by the author, 2026-10-11, after three read-only reviews of the child specs)

#### D52. Spec 4 is split into two specs

**Decided:**

1. **Spec 4a keeps the directory `2026-10-11-schematic-definition-api`.** It holds: builders; typed registration; the rewrite of all endpoint literals; pagination exceptions moved onto endpoints; removal of the old pagination types and conversion of the 35 `with_pagination` endpoints (this resolves parent recommendation 14: shrink = remove); the OpenAI importer definition (`definition_gen.rs`) emitting builder source; the disposition of `Schema::module_path`; serde derives on `ParamDef`, `QueryParamType`, and `ParamStyle`; the per-endpoint cross-origin override; and builder sugar for `request_id_header` and `.secret()`.
2. **Spec 4b is a new directory, `2026-10-11-schematic-definition-vocabulary`.** It holds the vocabulary additions as new builder methods, so no definition is rewritten again: typed path parameters; per-endpoint auth override; `deprecated`; rate-limit and retry vocabulary (metadata only, no runtime retry); `env_auth[0]` retirement (Basic auth carries its own env shape); polymorphic `oneOf` bodies; serializable definitions; OAuth2 per-grant payloads; and query-parameter `required`, `style`, `explode`, and enum fidelity in the Rust generator.
3. **The order is now eight specs:** 1 generation-integrity, 2 runtime-envelope, 3 pagination, 4a definition-api, 4b definition-vocabulary, 5 response-model-graphql, 6 websocket, 7 fidelity-polish. Landing 2 = specs 4a, 4b, 5. D50's list and mapping are updated by dated notes; the original text is kept.
4. **Constraint on 4a.** Its registration descriptors must be able to carry optional structural type information (needed for `oneOf`) and must derive serde from the start. The author estimated about a 1-in-3 chance that 4b would otherwise force changes to 4a's registration types.

**Rationale:** the count of endpoint literals is about 519, not 253, so a mechanical rewrite plus design-heavy vocabulary is too large for one unattended run.

**Options offered:**

- **Split into 4a and 4b (chosen).**
- **Keep spec 4 whole (rejected).**
- **Move serializable definitions and `oneOf` into 4a (rejected).**

**Consequences:** the D50 pre-authorization to rewrite the endpoint definitions applies to spec 4a. The D50 condition that the builder surface is shaped for spec 5's response vocabulary binds spec 4a. The P1-P12 mapping is revised in section 13a. `EndpointParams` derives `Default` and is allow-listed under D15 unless spec 4a rules otherwise.

**Open details:** none recorded in the confirmation.

#### D53. Typed convenience methods are an accepted gap

**Decided:** D49 item 3 ("typed convenience methods where applicable") is amended. No per-endpoint wrapper methods are generated. The generated docs and quick-start show `client.send(Request::new(..))`. This is recorded as an **accepted gap** against D49 item 3 and recommendation 12.

**Rationale:** the typed-send spike measured about 7 s of extra compile time at 550 wrapper instantiations; the typed request already carries the response type; wrappers only help the no-filter case.

**Options offered:**

- **No wrappers; accepted gap (chosen).**
- **Wrappers behind an optional Cargo feature (rejected).**
- **Wrappers always (rejected).**

**Consequences:** the docs honesty pass (spec 7) does not add convenience methods; the gap is listed in the final ledger table (D20).

#### D54. Ownership clarifications

**Decided (each is one bullet with its owner spec):**

- Removal of the old pagination types and conversion of the 35 endpoints: **spec 4a**.
- Credentials, including `ApiKeyParam`: **spec 2**. Spec 1's `with_api_key_param` and local env scan are temporary and are removed by spec 2.
- `request_id_header` and `secret_names` semantics, and API-level fields: **spec 2**. Builder sugar and the per-parameter `.secret()` marker: **spec 4a**.
- Hook mechanism, `ResponseKind` guard disposition, and `ErrorKind` additions: **spec 2** owns the mechanism; **spec 5** extends it and declares `UnexpectedContentType`.
- Diagnostics format and severity: **spec 1** (spec 6 maps onto it).
- Deletion of the stale `gen/schematic` tree: **spec 1 only**.
- Adding `schematic-runtime` to the area's `just` recipes, and naming `test-schema` as a gate: **spec 2**.
- Frozen tests of earlier specs are written as positive tests and retired or updated by the later spec: **spec 4a** retires spec 2 and 3 ones; **spec 5** owns baselines, the allow-list, and serde derives on its new types.
- A `?` in a path template is a hard error: **spec 1**.
- `Schema::module_path` is dead on the REST path: **spec 4a**.
- Loss of the emqx combined-module docs: **spec 7**.
- The optional-auth variant (Ollama), legacy-versus-modern auth dual-source cleanup, invented HuggingFace stub shapes, and orphaned types: **accepted-gap entries** recorded in spec 7's final ledger table (D20).
- The runtime trait that carries `Success`, `Failure`, and `Decode` is named `EndpointSpec`, for continuity with the existing generated trait and to avoid a collision with the `schematic-define` struct `Endpoint`. This is a **default, flagged for review 1**, not a final ruling.


### Spike results (2026-10-11)

Two throwaway prototype crates, kept under `./spikes/`, tested decisions D3, D7, D13, and D14. They are evidence, not shippable code, and not part of the workspace. Their own write-ups are `spikes/typed-send/findings.md` and `spikes/pager-surface/findings.md`. Test counts below were checked against the test files on 2026-10-11 (file and case counts only); that the tests **pass**, and all measurements, are the spike runs' reports and were **not re-run** by this review. Everything under "Recommended by spike" is **Recommended by spike; not yet ratified by the author.** Nothing here is a decision.

> **Update, 2026-10-11 (after D22-D29).** The same day, the author ruled on several of these recommendations. A shape is **ratified only where a ruling named below covers it**; everything else under "Recommended by spike" remains unratified. The two paragraphs headed "Ratification status" list which is which.

#### Typed-send spike

**Validated.** The decided design (D3 typed request only; D7 envelope `Result<Response<T, E>, TransportError>` with an inner success-or-failure outcome) works. One generic `send` covers every fixture, including streaming and GraphQL, so no separate `send_stream` is needed. Tests: 28 fixture, 5 ported sniff call-site, 1 ergonomics, 1 trybuild test with 8 compile-fail cases, 7 doctests (6 compile-fail, 1 positive). Wrong types fail to compile with readable errors. Compile cost of the new trait is negligible against today's (11.70 s versus 11.54 s for 550 endpoints; 550 calls to `send` add about 7 s, so do not generate a wrapper per endpoint; incremental builds not measured). The ported sniff call site is shorter and clearer than today's, and can read headers, which today's generated path cannot (`retry_after` is always `None` today).

**Recommended by spike; not yet ratified by the author** (compact Rust shapes; names illustrative):

```rust
trait Endpoint {
    type Success;                    // no serde bound; for a stream, an item stream
    type Failure: ErrorBody;         // or the uninhabited NoDeclaredError
    type Decode: Decode<Self::Failure, Output = Self::Success>; // async: decode(resp, ctx)
    const ENDPOINT_ID: &'static str;
    const DECLARED_ERROR_STATUSES: RangeInclusive<u16>;         // default 400..=599
}
struct Response<T, E> { meta: ResponseMeta, outcome: Outcome<T, E> }
enum   Outcome<T, E>  { Success(T), Failure(ApiError<E>) }
struct ApiError<E>    { kind: ErrorKind<E>, body: Bytes }       // raw body always kept
#[non_exhaustive]
enum   ErrorKind<E>   { Declared(E), Undeclared, MalformedError, MalformedSuccess,
                        UnexpectedContentType, BodyInterrupted, Hook }
struct ResponseMeta   { request: RequestMeta, status, headers, elapsed, request_id }
struct RequestMeta    { endpoint_id, method, url, headers, query /* all redacted */,
                        auth: AuthMethod, page }
struct TransportError { request: RequestMeta, kind, elapsed }
enum   AuthMethod     { PerCall, ClientLevel, EnvVar(name), Anonymous }
```

Codecs: `Json<T>`, `Text`, `Binary`, `Empty`, `Sse<T>`, `Ndjson<T>`, `GraphQl<D>`. A multi-shape success is a generated enum that implements `Decode` itself. GraphQL is a blanket impl over a per-operation trait (string document, typed `Variables`, typed `Data`).

Recommended semantics: `E` not declared is `NoDeclaredError` (uninhabited); GraphQL errors inside a 200 go on the **failure** side as `Declared(GraphQlFailure { errors, partial_data })` with status 200 and the raw body kept (so `?` cannot silently ignore them); `elapsed` for a stream measures time to headers and mid-stream errors are distinct items; hooks on text, binary, and stream endpoints are compile errors.

Friction found: a 2xx body that fails to decode arrives as a failure with status 200 (check `MalformedSuccess` first; the spike adds `is_decode_failure()`); `into_result` drops success metadata (use `into_parts`); `CallError<E>` is generic over the endpoint's error type, so mixing endpoints needs an own error enum; `pre_response_json` is client-global and also sees the GraphQL wire body; a GraphQL response mutator must run on typed `data`, not the `{data, errors}` wrapper; stream responses are not `Clone` or `Sync`.

**Ratification status, 2026-10-11 (typed-send).** *Ratified by a ruling:* failures before any request is sent are the outer `TransportError` (D24, matching the spike's choice; the kind list is open); `MalformedSuccess` with the real status kept, plus `is_decode_failure()` (D25); `BodyInterrupted` with status, headers, and partial bytes, plus `is_interrupted()`, and the rule that an envelope exists whenever a response head arrived (D26); GraphQL errors in a 200 on the failure side as `Declared(GraphQlFailure { errors, partial_data })` with status 200 and the raw body kept (D29); and never storing the HTTP library's own error text, with the URL kept in redacted form (D23). The sentinel limit the spike found for decoded error bodies is resolved by narrowing the promise (D22). *Still unratified:* the `Endpoint` trait and codec shapes, `NoDeclaredError`, the remaining `ErrorKind` variants and `#[non_exhaustive]` (S17), the `AuthMethod` representation, `into_result` versus `into_parts`, hooks as compile errors, and the meaning of `elapsed` for streams.

*Update, 2026-10-11 (second round).* Further ratified: `NoDeclaredError` (an uninhabited placeholder; the name is illustrative) and kind `Undeclared` for error replies (D48, D38); the `AuthMethod` representation (D48); the redirect-refusal kind `RedirectRefused` (D32). `send_raw` (D45) answers the custom-decoding path. Whether the erasing request enum remains is left to planning. Still unratified: the `Endpoint` trait and codec shapes, the remaining `ErrorKind` variants and `#[non_exhaustive]` (S17), `into_result` versus `into_parts`, hooks as compile errors, and `elapsed` for streams.

**Not tested:** real providers and the generator; D12 beyond redirect denial; D13; real pagers; multipart, form, and binary request bodies; HTTP/2; non-UTF-8 headers; stream cancel on drop; the custom decoding path (D3 open); same-URL streaming and non-streaming endpoints (D6 open); Linux, Windows, WSL2; real schematic-schema compile time; rust-analyzer latency; incremental compile time. The trybuild `.stderr` snapshots contain `kache`-remapped paths.

#### Pager-surface spike

**Validated.** D14's rule "only authored knobs exist; a missing knob is a compile error" holds with **one generic `Pager<E>` (configuration) and `Pages<E>` (iteration)** for all four strategy kinds (page number, cursor, next-link, session). Tests: 31 runtime tests (8 consecutive passing runs reported) and 1 trybuild test covering 21 cases (19 compile-fail, 2 pass). Of nine checks, eight with pass criteria passed; the ninth (early drop of a session pager) was measured only. clippy raised one warning, in the rejected encoding.

**Recommended by spike; not yet ratified by the author:**

```rust
// Each knob carries its own bound, so a missing knob is E0277 with a custom message.
impl<E> Pager<E> {
    fn page_size(self, n: u32) -> Self where E: PageSizeKnob;   // #[diagnostic::on_unimplemented]
    fn page(self, n: u32)      -> Self where E: PageNumbered;   // blanket capability trait,
}                                                               // #[diagnostic::do_not_recommend]
// Per endpoint the generator emits ~20 lines: constants plus empty marker impls, no method bodies.
```

Rejected by the spike: impl-level bounds (E0599 ignores the custom message) and type-level flag parameters (flags can drift from per-endpoint config; errors name type parameters). Also recommended: split `PageStrategy` variants into named structs and emit a constant projection; always origin-check followed links even with the D12 policy off; widen the stall rule to any repeated continuation; generate **both** a drop warning and a scoped `with_session`, and document `with_session` as the main path; decode pages through the same `from_slice` path as unpaginated calls (proposed, not measured).

**Ratification status, 2026-10-11 (pager-surface).** *Ratified by a ruling:* the scoped `with_session` as the way to use a session (D27). The recommendation to generate **both** a drop warning and `with_session` is **not** adopted: the author chose the scoped helper only, with no manual `open()`/`close()`. "Always origin-check followed links even with the D12 policy off" is **superseded** by D28 (the definition's policy always applies, default `same_origin`). *Still unratified:* the one-generic-pager encoding and its per-knob bounds (D14 item 3 itself stands as decided), named structs for `PageStrategy` variants and the constant projection (S12), widening the stall rule, and decoding pages through `from_slice` (S11).

*Update, 2026-10-11 (second round).* The generic pager is hand-written in the runtime package (D39) rather than emitted as template text; its encoding remains the unratified spike recommendation. `collect_all` is ratified (D41). Still unratified: the one-generic-pager encoding and per-knob bounds, S11, S12, and the stall-rule widening.

**Measured.** Deadline: 150 ms pages against a 400 ms budget; the third request was cut off; the error reported 2 completed pages and elapsed time; each request got the remaining budget; nothing was sent afterwards. Drop-cancel (HTTP/1.1 only): the server saw the connection close and live tasks went 1 to 0. Host policy: link differing in port, scheme, or host rejected with zero requests to the other server; a declared extra origin followed. Out-of-range page size errors before any request. Stalls (repeated cursor, cycle, repeated link, missing or invalid continuation) each return an explicit error, and a failed pager stays failed. Early drop of a session pager: `with_session` closes on return, error, and panic; a drop warning leaks unless the caller calls `close()`; **neither closes when the caller's future is cancelled** (only a detached spawn would, which D13 forbids), and `#[must_use]` does not help. Binary size: at 157 endpoints the pager adds about 1.77 MB to a release binary; about 253 IR lines of its own generic code per endpoint; 74% of added IR is a second copy of each item's deserializer, because pages decode through `serde_json::from_value`. Error messages read like "`Ep1` has no authored page size", and the implementor list is cut to 8 at 157 endpoints.

**Not tested:** real providers and generator; the offset strategy; partitions and advanced dimensions; HTTP/2; rust-analyzer completion; Linux, Windows, WSL2. The `.stderr` snapshots contain `kache`-remapped paths.

### Open Questions

*Refreshed 2026-10-11 (second round, D30-D48).* The author's rulings D30-D48 answered the twelve rulings the frontmatter listed. Answered items are listed under "Closed by rulings (2026-10-11, second round: D30-D48)" and carry a dated note in the lists below, which are otherwise kept as history. **The only open items are the planning-time details under "Remaining planning-time details (open)"**, near the end of this section. The older refresh paragraphs below say "nothing is decided" about their own moment; read them with this paragraph.

*Refreshed 2026-10-11 (rulings D22-D29).* The author's rulings closed spike questions S1-S5, S7, and S9 and part of items 3, 5, 6, and 7 below. Those are moved to "Closed by rulings (2026-10-11)" ahead of "Closed by spikes", and the numbered items carry dated notes saying what remains. Open details raised by the rulings are in "Raised by the 2026-10-11 rulings". Nothing else is decided.

*Update, 2026-10-11 (D52-D54).* The plan has eight child specs (spec 4 is split into 4a and 4b). D52-D54 record the split, an accepted gap against D49 item 3, and ownership clarifications. Where the P items below name "spec 4", read 4a unless the section 13a update says otherwise.

*Refreshed 2026-10-11 (spikes).* Added a block of spike-raised questions after the numbered list ("Raised by the 2026-10-11 spikes"), merged some into existing items (marked there), and added a "Closed by spikes" list. Nothing below is decided.

*Refreshed 2026-10-10.* Nothing below has been decided. Sequencing is decided (D5, amended by D18 and D19). The contracts that were "pending ratification" are now D9-D13 and D15, so the old first question is closed except for the complete-list rule (question 5). Questions that D9-D16 answered are removed; their remaining details moved here. Ranked by how much of the work each shapes.

*Refreshed again 2026-10-10, after D17-D21.* Closed: how a GraphQL query is authored (D17), the whole-work definition of done (D20), the pagination split between landings (D19), and the envelope's field list and redaction rules (D21, with details below). The list is re-ranked.

1. **Landing order and unassigned items (D5, D18, 13a).** The order of landing 3 (WebSocket) and landing 4 (fidelity); the single generated list of APIs; the landing for D10; the GraphQL landing; required constant headers as OpenAPI parameters (not named by D18).
   *Closed 2026-10-11 (second round): landing order and the unassigned items by D46, the D10 landing by D33, the GraphQL landing by D30.*
2. **Pagination on literals in landing 1 (D19, D14).** How the inferred `Vec<T>` default is expressed on an endpoint literal before builders exist; whether inference covers non-GET endpoints and how an inferred write result opts out of an API default. Also the size of landing 1, which now holds D3, D4, D7, D9-D14, D19, and D21 (raised by this review, not by the author; no size or timing expectation is stated).
   *Closed 2026-10-11 (second round): D42 and D43. The size-of-landing-1 remark is carried as P11.*
3. **GraphQL remaining details (D16, D17).** The home of the primitive (`schematic-define`, a new crate, or the consumer); partial data plus errors; the GitHub.com versus Enterprise URL (`/api/graphql` versus `/api/v3`); the landing; how typed variables and response data bind to the D2 registration context.
   *Update 2026-10-11: partial data plus errors is closed by D29 (failure side). The rest stays open.*
   *Closed 2026-10-11 (second round): home by D34, landing by D30, address by D35, binding by D34.*
4. **Response model v2 details (D6).** The streaming terminal marker; how to express same-URL streaming and non-streaming endpoints given the `(method, path)` uniqueness rule and the exporter's merge; how error shapes map status codes and ranges.
   *Closed 2026-10-11 (second round): D36, D37, D38.*
5. **Complete-list helper and pager lifecycle (D14).** Whether an auto-collecting "all items" helper exists and its guarantee (the proposed rule is still **Proposed**); what happens when a sessioned pager is dropped early. *Merged with spike question S9 (2026-10-11): the spike recommends generating both a drop warning and a scoped `with_session`; cancellation of the caller's future still leaks the session until the server lease ends.*
   *Update 2026-10-11: the early-drop half is closed by D27 (scoped `with_session` only). The complete-list helper and its guarantee remain open.*
   *Closed 2026-10-11 (second round): the complete-list helper is ratified by D41.*
6. **Envelope details (D7, D11, D21).** *Merged with spikes (2026-10-11): "handling when `E` is not declared" and "where GraphQL errors in a 200 body appear" now have spike recommendations (an uninhabited `NoDeclaredError`; the failure side with partial data), still unratified, so they stay open.* The definition field that names the request-id header; the representation of the auth method; per-parameter versus per-auth-strategy secret marking; the "common API-key headers" list and whether response headers share the redaction; the envelope's home crate; a value-only convenience; handling when `E` is not declared; where GraphQL errors in a 200 body appear.
   *Update 2026-10-11: where GraphQL errors in a 200 appear is closed by D29. Still open: the request-id field, the auth-method representation, secret-marking granularity, the common API-key header list and response-header redaction, the home crate, a value-only convenience, and handling when `E` is not declared.*
   *Closed 2026-10-11 (second round): request-id field, auth method, and the API-key header list by D48; secret-marking granularity by D40; home crate by D39; handling when `E` is not declared by D48. Still open as planning details: response-header redaction beyond `Set-Cookie` and a value-only convenience (P5).*
7. **Host policy details (D12, D4, D14).** The interaction between the policy and an injected custom HTTP client (`with_client`); the policy's builder name; whether a definition may carry known extra origins (such as `https://uploads.github.com`); whether a client without the policy follows cross-host next links, and how that squares with `NextLink`'s own trust setting in the pagination spike.
   *Update 2026-10-11: reshaped by D28. Closed: whether a client without the policy follows cross-host next links (the definition's policy always applies, default `same_origin`), the builder-versus-definition question (the definition carries it, including known extra origins through `allow_origins`), and the `NextLink` trust overlap. Still open: the injected custom HTTP client, naming, and how the policy appears in the generated public API (see "Raised by the 2026-10-11 rulings").*
   *Closed 2026-10-11 (second round): the injected client by D31. Naming and the generated public API remain (P1).*
8. **Typed registration sub-design (D2).** Naming rules for aliases, external types, and container paths; how the existing `SchemaRegistry` and the seven drifted name lists are replaced or fed. D15's builder method names depend on this.
   *Closed 2026-10-11 (second round): D44. Nested container and `HashMap` key details remain (P9).*
9. **Request methods and transport (D3, D4).** Whether and how custom decoding stays possible after `request<T>` is removed; the builder surface for TLS, proxy, and timeouts.
   *Closed 2026-10-11 (second round): custom decoding by D45; the builder surface by D31 (exact names remain, P3).*
10. **Pagination details beyond the principles (D14).** Whether labeled alternates are exposed by the generated client; whether the six unverified outside patterns need checking before they count as evidence.
   *Still open as planning details (P8).*
11. **Feature-flag shape (D18).** Granularity (per API or per module group) and the default feature set; interplay with partial generation (D9).
   *Still open as a planning detail (P10).*
12. **WebSocket scope (D8).** Whether an AsyncAPI exporter was in scope under D1 "everything", and what the explicit response link looks like.
   *Closed in part 2026-10-11 (second round): the AsyncAPI exporter is in scope, in landing 4 (D47). The explicit response link remains (P10).*
13. **OpenAPI and Postman specifics (landing 4).** The supported subset and version handling for the `x-schematic` round-trip; where Postman example values come from.
   *Still open as planning details (P10).*
14. **Diagnostics format.** How validation errors identify API, endpoint, feature, and output (D10 requires all four); whether the format is machine-readable.
   *Still open as a planning detail (P10).*

#### Raised by the 2026-10-11 spikes (ranked by impact)

Evidence is in "Spike results (2026-10-11)". Each is open; the "spike suggests" text is a recommendation, not a decision. Items that duplicate an existing question are merged into it above and marked there.

- **S1.** *Closed by D22 (see "Closed by rulings").*
- **S2.** *Closed by D23 (see "Closed by rulings").*
- **S3.** *Closed by D24 (see "Closed by rulings"); the kind list is a new open detail.*
- **S4.** *Closed by D26 (see "Closed by rulings").*
- **S5.** *Closed by D25 (see "Closed by rulings").*
- **S6. Query parameters must be separable (D21, generator).** The generator has to split query parameters out of the request parts (today they are formatted into the path) so they can be redacted. Needs confirming as landing 1 generator work. *Update 2026-10-11: D23 (URL kept in error text with secrets masked) depends on this split, so it is now a prerequisite generator task for landing 1 (this review's reading). Still not ruled as a landing item.*
- **S7.** *Superseded by D28 (see "Closed by rulings").*
- **S8. Declared error statuses on the endpoint (D6).** The spike puts a status range on the endpoint (default 400 to 599). Merges into question 4 (error shapes and status mapping).
  *Closed 2026-10-11 (second round): D38 maps status or range to shape at the API and endpoint level.*
- **S9.** *Closed by D27 (see "Closed by rulings").*
- **S10. Deadline arithmetic and session close (D13).** Each request gets "remaining time" or "min(remaining, 30 s)"; and whether closing a session gets time after the deadline expires.
- **S11. Pager decode path and binary size (D14).** Decode pages through the same `from_slice` path as unpaginated calls (proposed, not measured) to avoid about 1.77 MB at 157 endpoints, or accept the cost.
- **S12. `PageStrategy` variants as named structs, with a constant projection (D14).** Shape of the generated surface; touches the D14 model.
- **S13. Retain the erasing request enum (D3).** The spike found it is still needed for `into_parts` and per-client scoping. Confirms or limits D3's "preserve concrete endpoint types". Related to question 9 (custom decoding path, still untested).
  *Narrowed 2026-10-11 (second round): D45 leaves whether the enum remains to planning (P5).*
- **S14. Client-global `pre_response_json` (D3 hooks).** It sees every response, including the GraphQL wire body. Keep, scope per endpoint, or remove.
- **S15. Stream responses are not `Clone` or `Sync`.** Whether that is acceptable for sniff-style consumers; also the `[DONE]` marker, which the spike hard-codes (merges into question 4).
  *The `[DONE]` half is closed by D37; the `Clone`/`Sync` half remains (P5).*
- **S16. HTTP/2 cancel wording (D13).** Accept "stream reset" as the HTTP/2 form of "connection close". HTTP/2 itself was not tested.
- **S17. Mark `ErrorKind` `#[non_exhaustive]`.** The spike recommends it; the prototype does not.

#### Raised by the 2026-10-11 rulings (ranked by impact)

Each is open. Items marked "raised by this review" were not stated by the author.

- **R1. Injected custom HTTP client under D28.** reqwest fixes redirect behavior at client build time, so an injected client cannot be policed. Refuse the policy (reject such a client) or accept it unchecked.
  *Closed 2026-10-11 (second round): D31.*
- **R2. A refused redirect or link after a response head arrived (D26, D28; raised by this review).** The governing rule says an envelope exists whenever a head was received, but D24 makes a rejected origin a pre-send `TransportError`. For a redirect refused by the policy, the 3xx head did arrive. Which shape applies is unstated.
  *Closed 2026-10-11 (second round): D32.*
- **R3. The landing of D28 and D29.** D28 adds a redirect-following runtime and a `schematic-define` primitive; D29 rides with GraphQL placement. Neither landing is ruled.
  *Closed 2026-10-11 (second round): D30.*
- **R4. Per-endpoint override scope and the tighten-only comparison (D28).** What an endpoint may override, and (raised by this review) how "tighter than" orders the five values, including whether a caller may narrow an `allow_origins` list.
- **R5. How the policy appears in the generated client's public API, and its naming (D28).**
- **R6. `deny` and same-origin next-page links (D28; raised by this review).** The ruling reads "no followed cross-links"; same-origin paging presumably still works, but it is not stated.
- **R7. `with_session` shape (D27).** How an async closure borrows the pager.
- **R8. `TransportError` kinds (D24; raised by this review).** The full list and which kind each pre-send case takes.
- **R9. Redirect hop limit, and method and body rules for 301, 302, 303, 307, and 308 (D28).**

#### Closed by rulings (2026-10-11)

These were open and the author ruled on 2026-10-11. Unlike "Closed by spikes", each answer is a decision.

- **S1. Credential echo versus D21's redaction promise.** D22: narrow the promise to stored metadata and `Display`; decoded error bodies print as received in `Debug`. Rejected: hide bodies in `Debug`; scrub the credential from bodies.
- **S2. `reqwest::Error` leaks the URL.** D23: keep the URL in the error text, composed from Schematic's redacted URL plus the library message without its raw URL; a sentinel-key test must prove no query-string key appears.
- **S3. Failures before any request is sent.** D24: the outer `TransportError` with specific kinds, still carrying request metadata. sniff's "auth required" case moves to the kind.
- **S4. Body interrupted after headers arrive.** D26: a failure inside the envelope, kind `BodyInterrupted`, with status, headers, and partial bytes, plus `is_interrupted()`. Governing rule: an envelope exists whenever a response head arrived.
- **S5. A 2xx body that fails to decode.** D25: a failure inside the envelope with the real status, kind `MalformedSuccess`, plus `is_decode_failure()`.
- **S7 and the "followed links when the policy is off" question (question 7).** D28: the cross-origin policy is authored in the definition and always applies (default `same_origin`). The "always origin-check" recommendation is superseded.
- **S9 and the early drop of a session pager (question 5, in part).** D27: only a scoped `with_session`; no manual open/close and no drop warning.
- **Partial data plus errors in one GraphQL response (question 3, in part).** D29: failure side as `Declared(GraphQlFailure { errors, partial_data })`.
- **Where GraphQL errors in a 200 body appear (question 6, in part).** D29: the failure side.
- **Whether a definition may carry known extra origins (D12 open detail).** D28: yes, through `allow_origins([...])`.

#### Closed by rulings (2026-10-11, second round: D30-D48)

These were open and the author ruled on 2026-10-11. Each answer is a decision.

- **Landing of the cross-origin runtime and of GraphQL error handling (R3).** D30: runtime and API-level field in landing 1; per-endpoint override, GraphQL, and D29 in landing 2.
- **Injected custom HTTP client under D28 (R1, question 7, question 9 in part).** D31: injection removed; Schematic-managed builder.
- **A refused redirect or link (R2).** D32: redirect refused (3xx received) is a failure inside the envelope, kind `RedirectRefused`; a refused link is the outer `TransportError`, kind `CrossOriginRejected`.
- **Landing for the D10 features (question 1).** D33: landing 1, generator-only.
- **GraphQL home, landing, address, and binding (question 3).** D34, D30, D35.
- **Response model v2 details (question 4, S8, S15 in part).** D36 (same-address endpoints), D37 (stream end, including `[DONE]`), D38 (error mapping).
- **Envelope details (question 6).** D39 (home crate), D48 (no declared error, request-id field, auth-method values, built-in secret names), D40 (secret marking granularity).
- **Complete-list helper (question 5).** D41.
- **Pagination on literals and inference limits (question 2).** D42, D43.
- **Typed registration naming (question 8).** D44.
- **Custom decoding path (question 9, S13 in part).** D45.
- **Order of landings, single generated list, constant headers (question 1).** D46.
- **AsyncAPI exporter (question 12 in part).** D47.

#### Closed by D49 (2026-10-11)

These loose items had no landing. The author assigned them on 2026-10-11.

- **Isolated compile check (recommendation 4, ledger row 16).** D49: landing 1.
- **Two module-path resolvers (recommendation 10, ledger row 22).** D49: landing 4.
- **Generated-docs honesty pass (recommendation 12, ledger row 8).** D49: landing 4.
- **`ResponseKind` runtime guard.** D49: landing 1, following the response model.
- **Landing of the `oauth_scopes` and `auth_policy` decision.** D49: landing 2. The choice itself stays open (P12 below).
- **Stale July figures (section 7.1, section 6 item 1).** D49: correcting notes added.

#### Closed by spikes (2026-10-11)

These were open and the spikes answer them **in the prototypes**. "Closed" means the question has evidence; the author has not ratified any answer, and the generator and real providers remain untested.

- **Can one generic send serve streaming and GraphQL (D1, D3 interaction)?** Yes, without a separate `send_stream`.
- **Does the associated-type design make wrong-type calls a compile error with readable messages (D3)?** Yes, eight compile-fail cases.
- **Is the typed-trait compile cost acceptable at 550 endpoints (section 13a)?** Yes for the trait; a wrapper method per endpoint costs about 7 s extra, so avoid it. Incremental cost unmeasured.
- **Can a missing pagination knob be a compile error with one pager type (D14 item 3)?** Yes (19 compile-fail, 2 pass).
- **Does deadline-with-cancel work (D13)?** Yes on HTTP/1.1.
- **Does link following reject a foreign host before sending (D12, D14 item 7)?** Yes, differing port, scheme, and host.
- **Are stalls and loops detected (D14)?** Yes, and a failed pager stays failed.
- **Can the pager's cost be bounded (D14)?** Measured, not bounded: about 1.77 MB at 157 endpoints, with a proposed reduction.

#### Remaining planning-time details (open)

These are the only open items. None is a ruling the author still owes under D30-D48; each is a detail the rulings left to planning, or something this review raised while recording them. Where a line says "unratified", it is a spike recommendation nobody has decided. This review did not turn any of them into a decision.

Cross-origin (D28, D30, D32):

- **P1.** Per-endpoint override scope; the ordering among the five values for the tighten-only rule, and whether a caller may narrow an `allow_origins` list (R4); naming and the generated public API (R5); whether `deny` still follows same-origin next-page links (R6); redirect hop limit and the method and body rules for 301, 302, 303, 307, and 308 (R9); whether a tighten-only violation is a compile error or a runtime error; what the envelope shows when a refusal happens at a later hop of a redirect chain.

Runtime and envelope (D24, D31, D39, D45, D48):

- **P2.** `TransportError` kinds: the full list and the kind each pre-send case takes (R8).
- **P3.** Managed builder surface and names (D31); an accept-invalid-certificates setting for the Samsung TV call site, or another solution; migration of `homelab/lib/src/eversolo.rs` and `homelab/lib/src/samsung_tv.rs`; the later hooks mechanism for middleware.
- **P4.** Runtime package: final name; versioning against the generator; which items live in it (the `Endpoint` trait and codecs versus generated code); interplay with D9's incompatible shared-runtime rule.
- **P5.** Whether the erasing request enum remains (S13); `send_raw`'s signature; client-global `pre_response_json` (S14); whether stream responses not being `Clone` or `Sync` is acceptable (S15); `ErrorKind` `#[non_exhaustive]` (S17, unratified); `into_result` versus `into_parts`; `elapsed` for streams; a value-only convenience; whether response headers other than `Set-Cookie` share the redaction; the placeholder error type's name.
- **P6.** Deadline arithmetic: "remaining" versus "min(remaining, 30 s)", and whether closing a session gets time after the deadline expires (S10); accepting "stream reset" as the HTTP/2 form of "connection close" in the cancel criterion (S16); the shape of `with_session` for an async closure that borrows the pager (R7).
- **P7.** Query parameters separated from the path in the generator (S6): confirmed as a prerequisite for D23 by this review's reading, not separately ruled as a landing item.

Pagination (D14, D41, D42):

- **P8.** Which `Incomplete` reason an invalid or missing continuation takes, or whether a reason is added; the default page limit; the exceptions table's form and validation messages; decoding pages through the same `from_slice` path (S11, unratified); `PageStrategy` variants as named structs with a constant projection (S12, unratified); widening the stall rule (unratified); whether the generated client exposes labeled alternates; verifying the six unverified outside patterns before they count as evidence.

Response model, registration, GraphQL (D6, D34-D38, D44):

- **P9.** Selector syntax and the naming of the two generated request types (D36); exact names of the end-condition forms and the "ended early" item (D37); range syntax and overlap handling in error mappings (D38); nested container and `HashMap` key registration and the format of collision reports (D44); the GraphQL primitive's name, the representation of the address rule, and the pagination vocabulary for GraphQL connections (D34, D35); the request-id marker and `.secret()` forms (D40, D48).

Landing 3 and 4 content (D8, D18, D46, D47):

- **P10.** The explicit response link that replaces the WebSocket `RequestEnvelope` match (D8); AsyncAPI version and supported subset (D47); feature-flag granularity (per API or per module group) and default set, with partial generation (question 11); the supported subset and version handling for the `x-schematic` round trip and where Postman example values come from (question 13); the diagnostics format for validation errors (question 14).

Raised by this review (not by the author):

- **P11.** Landing 1 is now the largest landing: it holds the runtime package, the redirect runtime, the managed builder, request-side features, and pagination. No size or timing expectation is stated; the author may want a planning-time check on whether it stays one landing.

#### Items for review 1 (collected from the child-spec drafts)

These are decisions the child-spec drafters flagged for the author's review 1. **None is decided.** Each default shown is the drafter's default, which an unattended run would adopt and log.

- Spec 5: the final `done=true` item from Ollama (default: consume it; delivering it is the alternative).
- `Bytes` framing beyond D6.
- REST versus WebSocket behavior when no credential resolves (default: fail before sending with `MissingCredential`, unless the client was built anonymous).
- WebSocket never follows redirects (stricter than `same_origin`).
- Whether the tighten-only rule is a partial order.
- `min(remaining, per-request timeout)` versus remaining time under a deadline.
- Spec 3: the "adopt and hide" hazard.
- `auth_policy` kept and `oauth_scopes` removed (the `oauth_scopes`/`auth_policy` choice of D49 item 4; see the correction in section 2 on `auth_policy`'s generator consumer).
- The `ResponseKind` guard removed.
- The name `EndpointSpec` for the runtime trait (D54).

- **P12.** Whether `Endpoint::oauth_scopes` and `RestApi::auth_policy` are wired in or removed (D49). The decision is made during landing 2; the landing is fixed, the choice is not.

### Preparation tasks needing author permission

These are tasks, not design questions. The implementer cannot take them without the author's explicit approval.

- Regenerate `schematic-schema` (generated output is never hand-edited, but regeneration changes a large tree).
- Delete the stale git-tracked `gen/schematic/` tree.
- Verify the pagination of each provider the spike marks unverified (the **U** rows in [inventory.md](spikes/pagination/inventory.md), and the six outside patterns checked from memory), before authoring them, since a validator cannot catch a wrongly named signal.
- Fix the EMQX `ListAuthUsers` response type (the definition declares a root array where the provider returns `{data, meta}`), so the endpoint inherits the EMQX default.
- Decide whether the spike directory `./spikes/pagination/`, including the throwaway prototype crate, is committed. It is untracked today, and the prototype is not part of the workspace; the author decides whether it is kept, moved, or discarded.
- Confirm GraphQL expressibility (A10) with a local mock test. D16 widens this from "a JSON POST can carry it" to full support, so this check is now only the first step; the base-URL caveat is recorded in D16. D17 (2026-10-10) chose string-authored operations, which is what such a mock test would exercise first.
- Verify D14 inventory and counts against the current tree when authoring begins (the spike is a 2026-10-09 snapshot).
- Any other destructive or wide-scope action found during implementation.
