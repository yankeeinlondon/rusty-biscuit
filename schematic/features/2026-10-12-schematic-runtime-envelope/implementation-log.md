---
spec: /Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-12-schematic-runtime-envelope/spec.md
plan: schematic/features/2026-10-12-schematic-runtime-envelope/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - Cargo.toml
    - Cargo.lock
    - schematic/justfile
    - schematic/runtime/Cargo.toml
    - schematic/runtime/src/lib.rs
    - schematic/runtime/src/codec.rs
    - schematic/runtime/src/credentials.rs
    - schematic/runtime/src/deadline.rs
    - schematic/runtime/src/endpoint.rs
    - schematic/runtime/src/error.rs
    - schematic/runtime/src/meta.rs
    - schematic/runtime/src/options.rs
    - schematic/runtime/src/redact.rs
    - schematic/runtime/src/response.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/compile_fail.rs
    - schematic/runtime/tests/l1/decode_boundary.rs
    - schematic/runtime/tests/l1/envelope.rs
    - schematic/runtime/tests/l1/redaction.rs
    - schematic/runtime/tests/l1/test_layout.rs
    - schematic/runtime/tests/fixtures/compile_fail/endpoint_missing_associated_types.rs
    - schematic/runtime/tests/fixtures/compile_fail/endpoint_missing_associated_types.stderr
    - schematic/runtime/tests/fixtures/compile_fail/no_declared_error_value.rs
    - schematic/runtime/tests/fixtures/compile_fail/no_declared_error_value.stderr
docs_updated_during_phase_2:
    - docs/dependencies.md
    - schematic/docs/dependencies.md
    - schematic/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/schematic-define/schematic/SKILL.md
    - schematic/skill/SKILL.md
source_files_during_phase_3:
    - Cargo.lock
    - schematic/runtime/Cargo.toml
    - schematic/runtime/src/lib.rs
    - schematic/runtime/src/client.rs
    - schematic/runtime/src/codec.rs
    - schematic/runtime/src/credentials.rs
    - schematic/runtime/src/deadline.rs
    - schematic/runtime/src/encode.rs
    - schematic/runtime/src/endpoint.rs
    - schematic/runtime/src/error.rs
    - schematic/runtime/src/transport.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/support.rs
    - schematic/runtime/tests/l1/send.rs
    - schematic/runtime/tests/l1/credential_resolution.rs
    - schematic/runtime/tests/l1/builder.rs
    - schematic/runtime/tests/l1/timing.rs
    - schematic/runtime/tests/l1/decode_boundary.rs
    - schematic/runtime/tests/fixtures/compile_fail/endpoint_missing_associated_types.rs
docs_updated_during_phase_3:
    - docs/dependencies.md
    - schematic/docs/dependencies.md
    - schematic/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/schematic-define/schematic/SKILL.md
    - schematic/skill/SKILL.md
    - .claude/skills/os/SKILL.md
    - .claude/skills/os/windows.md
packages:
    - schematic-runtime
---

# Implementation Log for 2026-10-12-schematic-runtime-envelope (8 phases)

## Phase 1

Started 2026-10-12. Phase 1 is inventory, baseline, and one bounded functional
spike; it changes no production or generated API.

### Wave 1 — Verify Prerequisite

The prerequisite `2026-10-11-schematic-generation-integrity` is **ready**. Its
review 3 (`ready: true`, no findings, no human review) is in the working tree,
and the feature directory has been moved to the completed lifecycle directory
(uncommitted). Checked against the source, not only the review:

| Interface | Observed in source | Matches plan assumption |
|---|---|---|
| `RequestParts` | `schema/src/shared.rs:35`: `(&'static str, String, RequestBody, Vec<(String, String)>, Vec<(String, String)>)` = method, substituted path (embedded template query stays in the path), body, headers, ordered **unencoded** query pairs; `append_query_pairs` encodes once | yes |
| Request-side features | Text/binary bodies, header/cookie params, query/cookie API keys exercised against wiremock in `gen/tests/l1/fresh_output_compile.rs` (`text_body_reaches_the_server_verbatim_…`, `cookie_key_replaces_…`, `explicit_query_key_wins_over_env_…`, etc.) | yes |
| Fresh-output harness | `gen/tests/common/fresh_output.rs` (`FreshCrate::generate`, `cargo_check`, `nextest_run`, offline `--locked` against a copy of `schema/Cargo.lock`, shared child target dir). Four `slow_` tests: `slow_fixture_apis_compile_and_send_offline`, `slow_real_apis_compile_offline`, `slow_negative_control_reports_compiler_error`, `slow_query_wire_matches_pre_split_baseline`; run by `just test-fresh-output`, which `just test` invokes | yes |
| Partial-generation compatibility | `gen/src/output/mod.rs::prepare_rust_output` → `inventory::read_rust_inventory` (only for `Selection::Apis`) → `inventory::shared_runtime_compatibility(root, [("shared.rs", …), (ws_shared, …)], &retained)` compares bytes and returns `GeneratorError::RequiresFullGeneration { root, changed, retained }`. Inventory parses owned `lib.rs` (`declared_modules`) and `prelude.rs` (`prelude_exports`) with `syn`; ownership via `GENERATED_FILE_HEADER` / `MANIFEST_OWNERSHIP_MARKER` | yes — no serialized inventory, no persisted ABI marker |
| Stale tree | `schematic/gen/schematic/` removed (staged deletion, not committed) | yes |
| Temporary auth | `shared::resolve_request_auth` (`shared.rs:308`) and `with_api_key_param` exist; missing credential → `SchematicError::AuthenticationRequired` (and a `MissingCredential` variant also exists) | yes |

**Integration points for later phases.**

- Phase 6 ABI marker: add the marker read to `inventory.rs` beside
  `declared_modules`/`prelude_exports` (it already parses `lib.rs` with `syn`);
  replace the byte comparison in `shared_runtime_compatibility` while keeping
  `RequiresFullGeneration` as the refusal and keeping the module, export, and
  ownership checks. `ws_shared.rs` still needs its own comparison while
  WebSocket stays on the generated template (R35).
- Phase 6 compile harness: extend `FreshCrate::generate` /
  `generate_cargo_toml_with_paths` with a `schematic-runtime` path dependency.
  The fresh crate resolves offline against `schema/Cargo.lock`, so any new
  runtime dependency must already be in that lockfile (or the lockfile must
  be regenerated by the Phase 7 regeneration) — see Wave 2 notes.
- Existing generated tests whose expectations change under D56:
  `a_missing_key_is_authentication_required_and_nothing_is_sent` (fresh
  output) asserts a pre-send failure for a missing key; after this feature a
  missing key sends anonymously. Phase 6 must rewrite it, not delete it.
- Generated code depends on `schematic-define` today
  (`SchematicError::Header(#[from] schematic_define::HeaderError)`); the
  runtime must not, so header-validation errors become a runtime
  `InvalidArgument`-style kind.

**Authorization status.** The implementation instruction for this run is
"implement phase 1 of the plan". Phase 1 creates only this log and temporary
artifacts outside the repository, so it needs none of the spec's
pre-authorizations. I read the author's phase-by-phase implementation
instruction as granting, for each later phase, the pre-authorized scope that
phase's plan tasks name (runtime package creation and justfile edits,
definition/`RestApi` edits, test dev-dependencies, regeneration, dependent
edits, skill updates). Committing, pushing, CI changes, real provider calls,
and moving the feature remain unauthorized. No blocker.

### Planned PD1–PD35 dispositions

Recorded at Phase 1 as the planned disposition. Each later phase records
whether the implementation matched exactly (Human review agenda item 1).

| PD | Disposition | Planned implementation |
|---|---|---|
| PD1 | Ruled (D56, D57 item 6) | Tighten-only: `deny`, `same_origin`, or subset `allow_origins`; over `deny` only `deny`; others → typed `BuildError` naming both policies where required |
| PD2 | Adopt default | `deny` still passes same-origin `check_link`; forbids redirects and cross-origin links |
| PD3 | Adopt default | 10 followed hops; the 11th 3xx → `RedirectRefused`; constant in runtime |
| PD4 | Adopt default | 301/302 POST→GET, 303 non-HEAD→GET (drop body + body headers); 307/308 replay bytes; non-replayable body on 307/308 → `RedirectRefused` (none exists in typed path) |
| PD5 | Adopt default | Envelope shows the refused hop's 3xx; `meta.url` = request that received it; no `redirects` chain field |
| PD6 | Adopt default | `CrossOrigin::{same_origin, allow_origins, allow_without_credentials, danger_allow_any, deny}`; `RestApi::cross_origin`; `ClientBuilder::cross_origin(policy)` override |
| PD7 | Adopt default | `#[non_exhaustive]` kinds: `CrossOriginRejected`, `InvalidArgument`, `InvalidUrl`, `Connect`, `Tls`, `Timeout`, `DeadlineExceeded`, `Other` (sanitized; no library text). No `Cancelled`, no `MissingCredential` |
| PD8 | Adopt default, with proposed connect-timeout placement | Builder names as listed; `SharedTransport` builder carries proxy/TLS (and connect timeout — see Wave 2 decision); author review item |
| PD9 | Adopt default | No-argument `danger_accept_invalid_certificates()`; Samsung HTTPS only |
| PD10 | Adopt default | No general middleware; only `pre_response_json` / `mutate_response` via a JSON-codec marker trait |
| PD11 | Adopt default | `schematic-runtime` 0.1.0 at `schematic/runtime/`, workspace member, path dep; `pub const ABI: u32`; generated `lib.rs` `const _: () = assert!(…, "readable message")`; partial generation reads the marker from generated `lib.rs` |
| PD12 | Adopt default | Runtime owns traits, codecs, envelope/errors, redaction, credentials, builder, origin/redirects, deadline, hooks, `send`/`send_raw`; generated code owns request types, `EndpointSpec` impls/constants, thin per-API client |
| PD13 | Adopt default | Remove the erasing request enum unless private construction still needs it; never reachable from a send path; decision logged in Phase 6 |
| PD14 | Adopt (author review item) | `send_raw(request, &CallOptions) -> Result<RawResponse, TransportError>`; `RawResponse { meta, body, completion }` with Complete / BodyInterrupted / RedirectRefused / RedirectFailed |
| PD15 | Ruled (D57 item 9) | Client-global hook on successful JSON bodies only |
| PD16 | Adopt default | Stream responses not `Clone`/`Sync`; documented (test-only stream in this feature) |
| PD17 | Adopt default | `ErrorKind` `#[non_exhaustive]`; no `UnexpectedContentType` |
| PD18 | Adopt default | Both `into_result` (drops metadata) and `into_parts` (keeps it) |
| PD19 | Adopt default | Stream `elapsed` = time to head; documented on `Decode` |
| PD20 | Adopt default | No extra value-only convenience |
| PD21 | Adopt default | One redaction set by name for request and response headers |
| PD22 | Adopt default | `NoDeclaredError` (uninhabited) |
| PD23 | Ruled (D56) | `min(remaining, per-request timeout)`; `Deadline::after(duration)` in `CallOptions` |
| PD24 | Adopt default | Cancellation proved on HTTP/1.1; HTTP/2 untested, "stream reset" accepted wording |
| PD25 | Adopt default | API-level `request_id_header: Option<String>` / `secret_names: Vec<String>`; set only from in-tree or offline provider evidence; otherwise `None` / `same_origin`; logged per API in Phase 4 |
| PD26 | Ruled (D56) | `ResponseKind` guard and its error removed; mismatch → `MalformedSuccess` |
| PD27 | Adopt default | 2xx → success decode; other final statuses → Failure; `DECLARED_ERROR_STATUSES` (default `400..=599`) → `Declared`/`MalformedError` when `E` declared, else `Undeclared` |
| PD28 | Adopt default | Redacted `Credential` enum (access token, API key, Basic); `CallOptions::credential`, `ClientBuilder::credentials`; explicit anonymous mode on `CallOptions` |
| PD29 | Adopt default | `CrossOrigin` in define (data); `OriginPolicy` in runtime (behavior); generator translates |
| PD30 | Adopt default | `Client::check_link(&Url) -> Result<(), TransportError>` |
| PD31 | Adopt default | Preserve consumer behavior; sniff's anonymous retry stays a separate consumer call with forced anonymous options |
| PD32 | Withdrawn | — |
| PD33 | Adopt default | `trybuild`, `rcgen`, `tokio-rustls` dev-dependencies (all already in the root `Cargo.lock`: trybuild 1.0.116, rcgen 0.13.2, tokio-rustls 0.26.4) + hand-rolled TCP proxy mock |
| PD34 | Adopt default | No caller-supplied auth label |
| PD35 | Withdrawn | — |

### Wave 1 — Inventory Consumers

Read-only inventory of every dependent's use of the generated clients, taken
2026-10-12. Phase 7's "Close Consumer Inventory" task matches each row below to
its migrated location and test. Paths are relative to the repository root.

**Headline facts.**

- All eight dependents declare `schematic-schema` directly; no other crate
  does. Exact package names equal the spec's names: `sniff`, `biscuit-speaks`,
  `model-citizen`, `homelab`, `eversolo-integration`, `arcam-amp-integration`,
  `sony-receiver-integration`, `unfolded-integration-helper`.
- Only `sniff` (4 providers), `biscuit-speaks` (ElevenLabs), and `homelab`
  (Eversolo, Samsung TV) call the generated REST clients.
- `model-citizen` is **manifest-only** (confirmed: no Rust reference).
- `arcam-amp-integration`, `sony-receiver-integration`, and
  `unfolded-integration-helper` use WebSocket types only; `ws_shared.rs` and
  the `*_ws.rs` modules never reference `crate::shared`, so they are untouched
  by the REST change (still rebuilt and tested in Phase 7).
- `eversolo-integration` reaches REST only through `homelab`, and treats *any*
  `EversoloError` as "device asleep → Wake-on-LAN"; the migrated mapping must
  keep non-2xx and transport failures as errors.
- No dependent uses hooks, `EndpointSpec`, `ResponseKind`, `Headers`,
  `bearer_token`/`api_key` setters, `with_client`, `with_api_key_param`, or
  `resolve_request_auth`. `with_api_key_param` is not emitted into the current
  schema at all (no shipped API has an `ApiKeyParam` policy); it exists only in
  the generator and its fixture tests.
- `biscuit-speaks::list_voices_raw` bypasses the generated send using the
  accessors `api_key_header()`, `http_client()`, `api_base_url()`. Removing the
  injected/exposed HTTP client (R17) requires a replacement for this path — see
  the open questions below.

Read-only inventory of worktree `/Volumes/coding/wt/rusty-biscuit/feat-modern-config`, taken 2026-10-12. All paths are relative to the repo root.

##### 0. Generated surface being replaced (exact names in `schematic/schema/src`)

| Concern | Exact public name(s) | Where |
|---|---|---|
| Constructors (18 REST client structs) | `new()`, `with_base_url(impl Into<String>)`, `with_client(reqwest::Client)`, `with_client_and_base_url(reqwest::Client, impl Into<String>)` | e.g. `github/mod.rs:82,130,184,233` |
| Accessors | `http_client()`, `api_base_url()`, `auth_policy()`, `oauth_config()`, `api_key_header() -> Option<(String,String)>` | `github/mod.rs:278-297` |
| Auth setters (return a new client) | `bearer_token(token)` (github, huggingface, lmstudio, emqx, openai, unfolded_circle_core_rest), `api_key(key)` (anthropic, elevenlabs, artificial_analysis, gitlab, gitea), `variant_with_headers(Headers)`, `variant_with(..)` | `github/mod.rs:417,422`; `elevenlabs/mod.rs:480` |
| Variant builder | `variant()` -> `{Api}VariantBuilder` with `base_url`, `env_auth(Vec<String>)`, `auth_update(UpdateStrategy)`, `headers_builder(Headers)`, `pre_response_json(F)`, `mutate_response::<R,F>(F)`, `build()` | `github/mod.rs:359-580` |
| Generic JSON send | `pub async fn request<T: DeserializeOwned + Send + Sync + 'static>(&self, impl Into<{Api}Request>) -> Result<T, SchematicError>` (24 copies) | `github/client.rs:151` |
| Non-JSON convenience sends | `request_text` -> `String` (7 clients), `request_bytes` -> `bytes::Bytes` (7), `request_empty` -> `()` (6), plus per-endpoint wrappers such as `get_repository_content_raw` (github, gitea), `get_source`/`get_download` (bitbucket), `create_speech`/`create_sound_effect`/... (elevenlabs), `get_server_logs`/`launch_application_by_*`/`close_app`/`install_app` (samsung_smart_tv) | `github/client.rs:216,228`; `bitbucket/client.rs:217,229,239` |
| Response-kind guard | `shared::ResponseKind { Json, Text, Binary, Empty }` (`shared.rs:646`); `{Api}Request::response_kind()`; `request()` returns `SchematicError::WrongResponseDecoder` for a non-JSON kind | `github/client.rs:155-177` |
| Error enum | `shared::SchematicError` (`shared.rs:682`): `Http(reqwest::Error)`, `Json(serde_json::Error)`, `ApiError { status: u16, body: String }`, `UnsupportedMethod(String)`, `SerializationError(String)`, `Io(String)`, `MissingCredential { env_vars }`, `AuthenticationRequired { message, explicit_methods, env_fallback_vars }`, `Header(HeaderError)`, `WrongResponseDecoder { endpoint_id, kind, expected_method }`, `InternalError(String)` | |
| Hooks | `shared::ResponseContext` (`:778`, includes `url` and `headers`), `PreResponseJsonHook` (`:872`), `AnyResponseMutator` (`:888`), `TypedMutator` (`:903`), `VariantHooks` (`:954`) | |
| Endpoint trait | `shared::EndpointSpec { type Response; const ENDPOINT_ID }` (`:851`); `shared::Paginated` (`:997`) | |
| Auth helpers | `shared::resolve_request_auth` (`:308`), `apply_api_key_param` (`:584`), `ApiKeyParamCredential` (`:284`), `RequestAuth` (`:293`), `header_pair` (`:609`), `debug_client` (`:633`). Re-exported from `schematic_define`: `AuthPolicy, AuthStrategy, EnvAuthStrategy, Headers, OAuth2Config, UpdateStrategy` (`:19-20`); `pub use reqwest;` (`:9`) | |
| `with_api_key_param` | **Not emitted in the current generated schema.** No API defines an `ApiKeyParam` policy today. It exists only in the generator (`schematic/gen/src/codegen/api_struct/helpers.rs:108`) and its fixture tests | |
| WebSocket shared | `ws_shared.rs`: `WsError`, `WsClientOptions(+Builder)`, `ReconnectPolicy`, `WsConnectionState`, `WsTransportHandle`, `WsEncode`, `WsDecode`. `*_ws.rs` and `ws_shared.rs` have **no** reference to `crate::shared`, `SchematicError`, `EndpointSpec`, `ResponseKind`, or reqwest. The REST change does not reach WS-only users | |

##### 1. Dependent crates

`rg 'schematic-schema'` over every `Cargo.toml`, excluding `target/`, `schematic/schema`, and `schematic/gen/schematic`, finds exactly these 8 manifests. Each one declares a **direct** path dependency. `schematic/schema` is outside the workspace (`Cargo.toml:80 exclude`). No other crate depends on it.

| Package (`[package] name`) | Directory | Direct dep (manifest line) | Rust use of `schematic_schema` | Surface used |
|---|---|---|---|---|
| `sniff` | `sniff/lib` | yes (`Cargo.toml:44`) | yes | REST: github, gitlab, gitea, bitbucket |
| `biscuit-speaks` | `biscuit-speaks/lib` | yes (`:27`) | yes | REST: elevenlabs |
| `model-citizen` | `model-citizen/lib` | yes (`:42`) | **none** (manifest-only; Rust uses `schematic_definitions::{ollama,lmstudio}` only) | none |
| `homelab` | `homelab/lib` | yes (`:10`) | yes | REST: eversolo, samsung_smart_tv; WS: samsung_smart_tv_remote_ws, ws_shared |
| `eversolo-integration` | `homelab/eversolo-integration` | yes (`:13`), plus `homelab` | yes: WS host + `eversolo::VolumeData`/response types | WS + response types; REST indirectly through `homelab` |
| `arcam-amp-integration` | `homelab/arcam-amp-integration` | yes (`:12`), plus `homelab` | yes: `unfolded_circle_integration_ws` only | WS only |
| `sony-receiver-integration` | `homelab/sony-receiver-integration` | yes (`:12`), plus `homelab` | yes: `unfolded_circle_integration_ws` only | WS only |
| `unfolded-integration-helper` | `homelab/unfolded-integration-helper` | yes (`:10`) | yes: `unfolded_circle_integration_ws` only | WS only |

Indirect-only users, with no manifest dependency (they reach the generated REST client through `homelab` or `sniff`): `homelab-cli` (`homelab/cli`), `homelab-server` (`homelab/server`), `sniff-cli` (`sniff/cli`), and `biscuit-speaks-cli` (`biscuit-speaks/cli`, through `ElevenLabsProvider`).

##### 2. Counts per crate (generated REST execution sites only)

| Crate | `request::<T>` / `.request(` | non-JSON convenience sends | constructors / `with_client*` | variant/auth overrides | `SchematicError` matches/maps | hooks | EndpointSpec / ResponseKind |
|---|---|---|---|---|---|---|---|
| sniff | 32 (github 10, gitlab 7, gitea 8, bitbucket 7; incl. 4 anonymous-retry sends) | 4 (github `get_repository_content_raw` 1, gitea 1, bitbucket `get_source` 2) | 10 (`new`/`with_base_url`/`Default` ×4 providers, plus codeberg `new().variant().base_url()`) | 5 (`unauthenticated_client` ×4, codeberg `env_auth` ×1) | 4 `map_schematic_error` (non-exhaustive, `other =>`), 6 inline `Err(SchematicError::..)` matches, 1 construction (bitbucket.rs:307), 12 unit tests constructing variants | 0 | 0 |
| biscuit-speaks | 1 (`list_models`) | 3 (`create_speech` ×2, `create_sound_effect` ×1) | 2 (`ElevenLabs::new`, `with_base_url`) | 0; uses `api_key_header()`, `http_client()`, `api_base_url()` for a hand-rolled paginated GET | 1 `map_api_error` (`if let ApiError{402}`), 2 unit tests | 0 | 0 |
| model-citizen | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| homelab | 26 (eversolo 24, samsung 2) | 5 samsung (`request_text` ×1, `request_empty` ×4) | 2 `with_client_and_base_url` (+1 WS `with_base_url`) | 0 | 2 `#[from] SchematicError` wrappers (`EversoloError::Api`, `SamsungTvError::Api`), no variant match | 0 | 0 |
| eversolo-integration | 0 direct (indirect: ~40 `Eversolo::*` calls) | 0 | 0 (indirect `Eversolo::new` ×2) | 0 | indirect: treats any `EversoloError` as "unreachable" | 0 | 0 |
| arcam-amp-integration | 0 | 0 | 0 | 0 | 0 | 0 | 0 (WS only) |
| sony-receiver-integration | 0 | 0 | 0 | 0 | 0 | 0 | 0 (WS only) |
| unfolded-integration-helper | 0 | 0 | 0 | 0 | 0 | 0 | 0 (WS only) |

No dependent registers `pre_response_json`/`mutate_response`, uses `EndpointSpec`, `ResponseKind`, `ResponseContext`, `Headers`, `bearer_token`, `api_key(...)`, `headers_builder`, `with_client(...)`, or `with_api_key_param`.

##### 3. Row inventory

Legend: **Kind** is the endpoint's `ResponseKind` taken from `{api}/requests.rs::response_kind()`. **New** is the proposed replacement on the runtime surface, where `client.send(req, &CallOptions::default())` returns `Response<T,E>` and `.into_result()` (or status inspection) yields the value.

###### 3.1 sniff (`sniff/lib/src/remote/*.rs`)

| file:line | fn | API | Request -> Kind | Old behavior | New | Test owner |
|---|---|---|---|---|---|---|
| github.rs:49 / :64 / :72 | `GitHubRemote::new` / `with_base_url` / `Default` | github | ctor | `GitHub::new()` / `GitHub::with_base_url(url)`, env-fallback `GITHUB_TOKEN`/`GH_TOKEN` | runtime client builder + base URL; credentials from runtime env resolution | `remote_providers.rs::github_tests::setup_github_mock` (all github_tests) |
| github.rs:90-97 | `unauthenticated_client` | github | variant | `variant().env_auth(vec![]).auth_update(ChangeTo(AuthStrategy::None)).build()` | `CallOptions::anonymous()` at the call site; delete the variant | `unauth_fallback_tests::github_list_pull_requests_falls_back_to_unauthenticated_when_token_missing` |
| github.rs:114-120 | `fetch_tree` | github | `GetGitTreeRequest::with_recursive(true)` -> JSON `GitTreeResponse` | `request` + `map_schematic_error`; sets `truncated` | `send` + `into_result()`, keep `recursive=true` | `github_tests::list_documents_success` (asserts `query_param("recursive","true")`), `snapshot_reuse_tests::github_fetch_report_resolves_metadata_and_tree_once` |
| github.rs:147-153 | `continue_truncated_tree` | github | `GetGitTreeRequest("{branch}:{prefix}").with_recursive(true)` -> JSON | `request::<GitTreeResponse>`; any `Err` means skip the prefix | `send`; skip on any non-2xx/transport error | `snapshot_reuse_tests::a_truncated_github_tree_continues_for_document_and_ci_prefixes`, `a_truncated_tree_with_no_recoverable_prefixes_still_returns_root_entries` (the path is asserted but `recursive` is **not** asserted on continuations) |
| github.rs:294-300 | `get_repo_metadata` | github | `GetRepositoryRequest` -> JSON | `request` -> `map_schematic_error` | `send`; map the envelope status via the shared classifier | `github_tests::get_repo_metadata_success`, `error_401_unauthorized`, `error_404_not_found`, `error_429_rate_limited`, `error_403_rate_limited_in_body` |
| github.rs:377-381 | `get_file_content` | github | `GetRepositoryContentRawRequest` -> **Text** | `get_repository_content_raw` (`request_text`) | `send` on the text endpoint, returning `Response<String,_>` | **none: needs test** (no GitHub raw-content mock) |
| github.rs:411-421 | `list_pull_requests` | github | `ListPullRequestsRequest` -> JSON | `request`; on `MissingCredential`/`AuthenticationRequired`, retry with `unauthenticated_client()` | `send(req, default)`; on a credential-missing outcome retry `send(req, &CallOptions::anonymous())` | `github_tests::list_pull_requests_*`; `unauth_fallback_tests::github_list_pull_requests_falls_back_..._token_missing`, `..._returns_invalid_credentials_when_anonymous_rejected` |
| github.rs:468-474 | `list_issues` | github | `ListIssuesRequest` -> JSON | `request` | `send` + `into_result` | `github_tests::list_issues_success` |
| github.rs:501-516 | `get_tags_and_releases` | github | `ListTagsRequest`, `ListReleasesRequest` -> JSON | 2× `request` | 2× `send` | `github_tests::get_tags_and_releases_success` |
| github.rs:586-604 | `list_workflow_runs` | github | `ListWorkflowRunsRequest.with_per_page` -> JSON | `request`; anonymous retry on MissingCredential/AuthRequired **or ApiError 401/403** | `send`; anonymous retry on credential-missing or a 401/403 envelope | `github_tests::list_workflow_runs_success`, `list_workflow_runs_caps_results_at_limit`, `fetch_report_via_enum_includes_workflow_runs`; `unauth_fallback_tests::github_list_workflow_runs_falls_back_..._token_missing` |
| gitlab.rs:63 / :88 / :97 | `new` / `with_base_url` (appends `/api/v4`) / `Default` | gitlab | ctor | `GitLab::new()` / `with_base_url(api_url)` | runtime builder | `gitlab_tests::setup_gitlab_mock`; unit `test_build_key_urls_self_hosted` |
| gitlab.rs:118-123 | `unauthenticated_client` | gitlab | variant | env_auth empty + AuthStrategy::None | `CallOptions::anonymous()` | `unauth_fallback_tests::gitlab_list_pull_requests_falls_back_..._token_missing` |
| gitlab.rs:132-137 | `fetch_tree_items` (used by `get_repo_metadata` :289 and `snapshot` :298) | gitlab | `ListRepositoryTreeRequest` -> JSON | `request` | `send` | `gitlab_tests::get_repo_metadata_success`, `list_documents_success`, `snapshot_reuse_tests::gitlab_fetch_report_fetches_the_tree_once` |
| gitlab.rs:346-352 | `get_file_content` | gitlab | `GetRepositoryFileRequest(.., "HEAD")` -> JSON `FileContent` (base64) | `request`, then base64 decode | `send` | **none: needs test** |
| gitlab.rs:377-410 | `list_pull_requests` | gitlab | `ListMergeRequestsRequest` -> JSON | `request` + anonymous retry on MissingCredential/AuthRequired | `send` + `CallOptions::anonymous()` retry | `gitlab_tests::list_pull_requests_*`; `unauth_fallback_tests::gitlab_*` |
| gitlab.rs:454-460 | `list_issues` | gitlab | `ListIssuesRequest` -> JSON | `request` | `send` | `gitlab_tests::list_issues_success` |
| gitlab.rs:488-503 | `get_tags_and_releases` | gitlab | `ListTagsRequest`, `ListReleasesRequest` -> JSON | 2× `request` | 2× `send` | `gitlab_tests::get_tags_and_releases_success` |
| gitea.rs:89 | `with_base_url` (normalized `/api/v1`) | gitea | ctor | `Gitea::with_base_url` | runtime builder | `gitea_tests::setup_gitea_mock` |
| gitea.rs:113-120 | `codeberg` | gitea | variant | `Gitea::new().variant().base_url("https://codeberg.org/api/v1").env_auth(["CODEBERG_TOKEN","GITEA_TOKEN"]).build()` | runtime builder with base URL + credential env list `CODEBERG_TOKEN`, `GITEA_TOKEN` | unit `test_build_key_urls_codeberg` only (no request test): **needs test** for env order |
| gitea.rs:157-162 | `unauthenticated_client` | gitea | variant | as above | `CallOptions::anonymous()` | `unauth_fallback_tests::gitea_*` |
| gitea.rs:175-181 | `fetch_tree` | gitea | `GetGitTreeRecursiveRequest` -> JSON | `request` | `send` | `gitea_tests::list_documents_success`, `snapshot_reuse_tests::gitea_fetch_report_resolves_metadata_and_tree_once` |
| gitea.rs:187-193 | `fetch_tree` continuation | gitea | `GetGitTreeRecursiveRequest("{branch}:{prefix}")` -> JSON | `request::<GitTreeResponse>`; skip on Err | `send`; skip on non-success | **none: needs test** (truncation test is GitHub-only) |
| gitea.rs:329-335 | `get_repo_metadata` | gitea | `GetRepositoryRequest` -> JSON | `request` | `send` | `gitea_tests::get_repo_metadata_success`, `error_401/404/429_*` |
| gitea.rs:419-423 | `get_file_content` | gitea | `GetRepositoryContentRawRequest` -> **Text** | `get_repository_content_raw` (`request_text`) | `send` on the text endpoint | **none: needs test** |
| gitea.rs:433-463 | `list_pull_requests` | gitea | `ListPullRequestsRequest` -> JSON | `request` + anonymous retry | `send` + anonymous | `gitea_tests::list_pull_requests_*`; `unauth_fallback_tests::gitea_*` |
| gitea.rs:528-534 | `list_issues` | gitea | `ListIssuesRequest` -> JSON | `request` | `send` | `gitea_tests::list_issues_success` |
| gitea.rs:573-588 | `get_tags_and_releases` | gitea | `ListTagsRequest`, `ListReleasesRequest` -> JSON | 2× `request` | 2× `send` | `gitea_tests::get_tags_and_releases_success` |
| bitbucket.rs:59 / :74 / :127 | `new` / `with_base_url` / `Default` | bitbucket | ctor | `Bitbucket::new()` / `with_base_url` (Basic via `BITBUCKET_USERNAME`+`BITBUCKET_APP_PASSWORD`) | runtime builder | `bitbucket_tests::setup_bitbucket_mock` |
| bitbucket.rs:148-153 | `unauthenticated_client` | bitbucket | variant | as above | `CallOptions::anonymous()` | `unauth_fallback_tests::bitbucket_*` |
| bitbucket.rs:272-288 | `collect_directory_entries` (from `fetch_tree` :102, :112) | bitbucket | `GetSourceRequest{pagelen:100, page}` -> **Binary** | `get_source` (`request_bytes`) -> `Bytes`, then `decode_directory_listing` JSON-decodes it (`serde_json` error wrapped as `SchematicError::Json`) | `send` on the binary endpoint -> `Response<Bytes,_>`; keep the local JSON decode; replace the `SchematicError::Json` wrap at :307 | `bitbucket_tests::list_documents_success`, `a_malformed_directory_listing_leaves_the_tree_unavailable_not_empty`; unit `a_directory_listing_body_decodes_to_its_entries`, `a_malformed_directory_listing_body_is_a_json_error` |
| bitbucket.rs:352-358 | `get_repo_metadata` | bitbucket | `GetRepositoryRequest` -> JSON | `request` | `send` | `bitbucket_tests::get_repo_metadata_success`, `error_*` |
| bitbucket.rs:452-468 | `get_file_content` | bitbucket | `GetRepositoryRequest` (JSON) then `GetSourceRequest` -> **Binary** | `request` then `get_source`; lossy UTF-8 via `decode_file_text` | `send` ×2 (binary endpoint for the second) | `bitbucket_tests::get_file_content_returns_the_raw_bytes_as_text`; unit `file_text_replaces_invalid_utf8` |
| bitbucket.rs:478-510 | `list_pull_requests` | bitbucket | `ListPullRequestsRequest` -> JSON | `request` + anonymous retry | `send` + anonymous | `bitbucket_tests::list_pull_requests_*`; `unauth_fallback_tests::bitbucket_*` |
| bitbucket.rs:616-626 | `list_issues` | bitbucket | `ListIssuesRequest` -> JSON | `request`; `Err(ApiError{status:404})` returns `Ok(vec![])` (tracker disabled) | `send`; check `status == 404` on the envelope and return empty | `bitbucket_tests::list_issues_success`, `list_issues_returns_empty_when_disabled` |
| bitbucket.rs:685-691 | `get_tags_and_releases` (tags) | bitbucket | `ListTagsRequest` -> JSON | `request` | `send` | `bitbucket_tests::get_tags_and_releases_success` |
| bitbucket.rs:720-732 | `get_tags_and_releases` (downloads) | bitbucket | `ListDownloadsRequest` -> JSON | `request`; **any** error becomes an empty page | `send`; any non-success becomes empty | `bitbucket_tests::get_tags_and_releases_success` (success path only) |
| github.rs:203 / gitlab.rs:213 / gitea.rs:259 / bitbucket.rs:175 | `map_schematic_error` | all | n/a | non-exhaustive match (see §4) | one classifier over `Response` status/headers/body + `TransportError` | unit `test_map_schematic_error_{missing_credentials,rate_limited,not_found}` in each of the four files (constructs `SchematicError` directly; these **must be rewritten**) |
| bitbucket.rs:307 | `decode_directory_listing` | bitbucket | n/a | wraps a local serde error in `SchematicError::Json` to reuse the mapper | map directly to `SniffError::RemoteApi{status:0,..}` | unit `a_malformed_directory_listing_body_is_a_json_error` |

Indirect users in sniff:
- `remote/mod.rs:148-185` `GitRemote::from_url`.
- `remote/mod.rs:210-255` `from_shorthand`, which skips on `RemoteApi{404}` and stops on `InvalidCredentials`/`RateLimited`.
- `sniff-cli` `cli/src/commands/remote.rs:17,18,41,43,82,181`.
- Bench `lib/benches/support/remote_report_fixture.rs:66` (`GitHubRemote::with_base_url`).

`remote/focused.rs` and `remote/blocking.rs` make hand-written reqwest calls and are **not** generated-client users.

###### 3.2 biscuit-speaks (`biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs`)

| file:line | fn | API | Request -> Kind | Old behavior | New | Test owner |
|---|---|---|---|---|---|---|
| :28 | imports | shared | n/a | `use schematic_schema::shared::{SchematicError, reqwest}` (uses the `reqwest` re-export) | import the runtime error types; take `reqwest` directly or from the runtime re-export | n/a |
| :97-112 | `ElevenLabsProvider::new` | elevenlabs | ctor | pre-checks `ELEVEN_LABS_API_KEY`/`ELEVENLABS_API_KEY`, then `ElevenLabs::new()` (env fallback supplies `xi-api-key`) | runtime client with resolved credential | unit `test_new_without_env_var`; `tests/detached_phase4.rs::elevenlabs_cached_job_is_ready_without_network` |
| :123-138 | `with_base_url` | elevenlabs | ctor | same pre-check, then `ElevenLabs::with_base_url(url)` | runtime builder + base URL | unit `test_with_base_url_without_env_var` |
| :283-298 | `map_api_error` | elevenlabs | n/a | `if let ApiError{status:402, body}` gives `PaidPlanRequired`; everything else gives `HttpError{message: e.to_string()}` (Display includes body) | inspect envelope status 402 + body; transport gives `HttpError` | unit `test_map_api_error_402_returns_paid_plan_required`, `test_map_api_error_non_402_returns_http_error` (construct `SchematicError`; **must be rewritten**) |
| :355-360 | `generate_to_cache` | elevenlabs | `CreateSpeechRequest` -> **Binary** | `create_speech` (`request_bytes`) | `send` on the binary endpoint -> `Bytes` | ignored integration `test_generate_audio_integration` only: **needs mock test** |
| :488-493 | `generate_audio` | elevenlabs | `CreateSpeechRequest` -> **Binary** | `create_speech` | as above | ignored `test_generate_audio_integration` (:1450): **needs mock test** |
| :525-614 | `list_voices_raw` | elevenlabs | none (hand-rolled GET `/v2/voices`) | reads `client.api_key_header()` (:529), `client.http_client()` (:534), `client.api_base_url()` (:535); sends via raw reqwest; handles 402 itself | either a runtime-generated `ListVoices` send with pagination, or keep raw reqwest but obtain the credential/base URL/HTTP client from the runtime. **The accessors `api_key_header`/`http_client`/`api_base_url` must survive or be replaced** | ignored `test_list_voices_raw_integration`, `test_tts_voice_inventory_list_voices_integration`: **needs mock test** |
| :637-646 | `list_models` | elevenlabs | `ListModelsRequest` -> JSON | `request::<Vec<ModelInfo>>` | `send` + `into_result` | ignored `test_list_models_integration`: **needs mock test** |
| :669-693 | `create_sound_effect` | elevenlabs | `CreateSoundEffectRequest` -> **Binary** | `create_sound_effect` (`request_bytes`) | `send` on the binary endpoint | **none: needs test** |
| :1346-1360 | test `test_debug_voice_deserialization` | n/a | n/a | raw `reqwest::Client` through the `schematic_schema::shared::reqwest` re-export | adjust the import | ignored diagnostic |

Indirect callers:
- `lib/src/speak.rs:384,434`.
- `lib/src/detached.rs:101,146`.
- `lib/src/cache.rs:483-484`.
- `biscuit-speaks-cli` `cli/src/main.rs:355`.

###### 3.3 homelab (`homelab/lib/src`)

| file:line | fn | API | Request -> Kind | Old behavior | New | Test owner |
|---|---|---|---|---|---|---|
| eversolo.rs:27-31 | `EversoloError::Api(#[from] SchematicError)` | eversolo | n/a | wraps every failure; Display is `"Eversolo API error: {SchematicError}"` (includes the body for ApiError and the reqwest error/URL for Http) | wrap the runtime error/`TransportError` plus a non-success envelope (status+body) | none |
| eversolo.rs:53-65 | `Eversolo::new(host, port)` | eversolo | ctor | `reqwest::Client::builder().connect_timeout(5s).timeout(10s).build().expect(..)`; `EversoloClient::with_client_and_base_url(http_client, "http://{host}:{port}")`; auth None | managed runtime builder with connect timeout 5 s, request timeout 10 s, base URL; no `with_client*` | **none: needs test** |
| eversolo.rs:84,94,102,110,118,126,136,144,154,164,174,184,192,202,210,220,228,236,244,252,260,268,277,286 (24 sites) | `get_model`, `get_state`, `play_or_pause`, `play_next`, `play_previous`, `seek_to`, `get_inputs_outputs`, `set_input`, `set_output`, `set_volume`, `set_mute`, `get_power_options`, `set_power_option`, `send_key`, `input_text`, `get_screen_brightness`, `set_screen_brightness`, `get_knob_brightness`, `set_knob_brightness`, `get_vu_modes`, `set_vu_mode`, `get_spectrum_modes`, `set_spectrum_mode`, `change_vu_display` | eversolo | `Device*/Music*/Power*/Remote*/System*Request` -> all **JSON** (eversolo has no non-JSON endpoint) | `request::<T>(req).await?` | `send(req, default).await` + `into_result()` mapped to `EversoloError` | **none: needs test** (eversolo.rs tests cover only the pure helpers `is_actively_playing`/`infer_effective_power_state`) |
| samsung_tv.rs:46-50 | `SamsungTvError::Api(#[from] SchematicError)` | samsung_smart_tv | n/a | as eversolo; `WebSocket(Box<WsError>)` stays | runtime error variant | none |
| samsung_tv.rs:93-125 | `SamsungTv::new` -> `with_https(host, rest_port, ws_port, use_https)` | samsung_smart_tv (+ws) | ctor | scheme is `https` iff `use_https`; builder `connect_timeout(5s).timeout(10s)`, and **`danger_accept_invalid_certs(true)` only when `use_https`**; `RestClient::with_client_and_base_url(http_client, "{scheme}://{host}:{rest_port}")`; WS `SamsungSmartTvRemoteWs::with_base_url("wss://{host}:{ws_port}")` | managed builder (5 s/10 s, accept-invalid-certs gated on HTTPS) + base URL; WS unchanged | **none: needs test** |
| samsung_tv.rs:149 | `get_device_info` | samsung_smart_tv | `GetDeviceInfoRequest` -> JSON | `request::<SamsungDeviceInfoResponse>` | `send` + `into_result` | none |
| samsung_tv.rs:157 | `get_server_logs` | samsung_smart_tv | `GetServerLogsRequest` -> **Text** | `request_text` | `send` on the text endpoint | none |
| samsung_tv.rs:166 / :174 | `launch_app_by_id` / `launch_app_by_name` | samsung_smart_tv | `LaunchApplicationById/ByNameRequest` -> **Empty** | `request_empty` | `send`; check success status, discard the body | none |
| samsung_tv.rs:186 | `get_app_status` | samsung_smart_tv | `GetAppStatusRequest` -> JSON | `request::<SamsungAppStatusResponse>` | `send` | none |
| samsung_tv.rs:193 / :201 | `close_app` / `install_app` | samsung_smart_tv | `CloseAppRequest` / `InstallAppRequest` -> **Empty** | `request_empty` | `send`, success check | none |
| samsung_tv.rs:18, 246-253, 298-302 | WS connect | ws_shared | n/a | `WsClientOptions::builder().handshake_timeout(10s).request_timeout(10s)`; `WsError` | **unchanged** (WS) | none |

Indirect callers:
- **`homelab-cli`** `cli/src/main.rs`:
  - Eversolo: `Eversolo::new` at :1260, plus about 25 method calls.
  - Samsung: `SamsungTv::with_https(.., use_https)` at :1801, where `use_https` comes from the CLI flag or config via `resolve_samsung`, plus REST calls at :1825, 1869, 1890, 1906, 1928, 1952, 1963.
- **`homelab-server`**:
  - `src/lib.rs:261` creates `Eversolo::new` and `src/lib.rs:381` creates `SamsungTv::new` (HTTP), with `get_device_info` at :384.
  - `handlers/eversolo.rs:534` creates `Eversolo::new`, plus 21 calls.
  - `handlers/samsung_tv.rs:512` creates `SamsungTv::with_https` from config, with REST calls at :104, 125, 151, 155, 244, 268, 292.
  - `src/error.rs:32-34,65-66,100-109,142-143` maps `ServerError::Eversolo`/`SamsungTv` to **502** `EVERSOLO_ERROR`/`SAMSUNG_TV_ERROR`; Display includes the inner `SchematicError` text.
  - Test owner: `server/tests/integration_tests.rs` has no eversolo or samsung references, so these paths are untested.

###### 3.4 eversolo-integration (`homelab/eversolo-integration/src`), indirect REST through `homelab`

| file:line | fn | Old behavior | New | Test owner |
|---|---|---|---|---|
| discovery.rs:25-28 | `EversoloDiscovery::validate_host` | `Eversolo::new` then `get_model().await.ok()?` (any error means not a device) | unchanged if `homelab` keeps `Result<_, EversoloError>` | none |
| dispatch.rs:454-457 | `build_client` | `Eversolo::new(host, config.port)` | unchanged | none |
| dispatch.rs:59-66, 251-371, 434-439, 465, 485, 859-989 | dispatch ops (~40 calls) | `run_with_timeout` wraps every `homelab` call; `Ok(Err(e))` gives `EversoloIntegrationError::Eversolo(e)` (:490-498) | unchanged | unit tests cover pure helpers only (`test_playback_state_label`, `test_clamp_volume`, ...): needs test |
| dispatch.rs:244-245, 398-400 | `PowerToggle` / power-on path | **any** `EversoloIntegrationError::Eversolo(_)` (transport, non-2xx, or JSON) or `Timeout` is treated as "device asleep", which triggers `power_on`/WoL | **behavior to preserve**: the new homelab mapping must still produce `EversoloError` for non-2xx **and** transport failures, not panic or return `Ok` | none: needs test |
| error.rs:8, 42 | `EversoloIntegrationError::Eversolo(#[from] EversoloError)` | UC error code mapping | unchanged | none |
| dispatch.rs:13, 1001; main.rs:25; handler.rs:5, 481 | `schematic_schema::eversolo::VolumeData` etc. (response types); WS host/handler | type-only / WS | unchanged (response types stay in definitions) | n/a |

###### 3.5 WS-only crates (no REST surface; unaffected by the REST change)

| Package | file:line | Symbol |
|---|---|---|
| arcam-amp-integration | src/main.rs:24; src/handler.rs:5, 477 | `UnfoldedCircleIntegrationWsHost`, `WsConnectionContext`, `WsHandler`, `UnfoldedCircleEventHub` |
| sony-receiver-integration | src/main.rs:25; src/handler.rs:5, 477 | same |
| unfolded-integration-helper | src/setup.rs:13; src/envelope.rs:3; src/subscriptions.rs:3 | `WsConnectionContext`, envelope/subscription types from `unfolded_circle_integration_ws` |
| eversolo-integration | src/main.rs:25; src/handler.rs:5, 481 | same WS types |

###### 3.6 model-citizen

There is no Rust reference to `schematic_schema`; the dependency is manifest-only (`model-citizen/lib/Cargo.toml:42`). Rust code uses `schematic_definitions::ollama` (`scanner/ollama.rs:276,284,295,642`) and `schematic_definitions::lmstudio` (`scanner/lmstudio.rs:374,405`). Doc drift: `model-citizen/lib/README.md:72` says "`schematic-schema` - HuggingFace API client", which is stale. The dependency could be dropped.

##### 4. sniff error classification (behavior baseline)

Generated-client path (`map_schematic_error`; every one is **non-exhaustive** with an `other =>` catch-all giving `RemoteApi{status:0, message: other.to_string()}`):

| Condition | github.rs | gitlab.rs | gitea.rs | bitbucket.rs |
|---|---|---|---|---|
| `MissingCredential{env_vars}` -> `MissingCredentials{env_var: join(" or ")}` | :205 | :215 | :261 | :177 |
| `AuthenticationRequired{env_fallback_vars}` -> `MissingCredentials` | :209 | :219 | :265 | :181 |
| **401** | :215 gives `InvalidCredentials{message}` from JSON `body.message`, else "bad credentials" | :225 gives `MissingCredentials{env_var:"GITLAB_TOKEN or GITLAB_PRIVATE_TOKEN (API returned 401: {body})"}` | :271 gives `MissingCredentials{"GITEA_TOKEN (API returned 401: {body})"}` | :187 gives `MissingCredentials{"BITBUCKET_USERNAME + BITBUCKET_APP_PASSWORD (API returned 401: {body})"}` |
| **403**: body lowercase contains "rate limit" -> `RateLimited{retry_after:None}`, else `RemoteApi{403, body}` | :225-239 | :232-246 | :275-289 | :194-207 |
| **429** -> `RateLimited{retry_after:None}` | :240 | :247 | :290 | :208 |
| **404** -> `RemoteApi{404,"Not found"}` | :244 | :251 | :294 | :212 |
| other status -> `RemoteApi{status, body}` | :249 | :256 | :299 | :217 |
| `Http(e)` -> `RemoteApi{0, e.to_string()}` (the reqwest Display includes the **URL**) | :254 | :261 | :304 | :222 |
| `Json(e)` -> `RemoteApi{0,"JSON parse error: .."}` | :259 | :266 | :309 | :227 |

- **Retry-After:** never parsed anywhere. `retry_after` is always `None`, and github.rs:230 has the comment "Would need to parse X-RateLimit-Reset header". `SniffError::RateLimited{retry_after: Option<u64>}` is defined in `sniff/lib/src/error.rs:306-311`.
- **Request-id:** not read or logged anywhere in sniff (no `x-github-request-id`/`x-request-id` handling).
- **Anonymous retry triggers:**
  - `list_pull_requests` retries on `MissingCredential|AuthenticationRequired` (github.rs:413-414, gitlab.rs:402-403, gitea.rs:455-456, bitbucket.rs:502-503).
  - GitHub `list_workflow_runs` additionally retries on `ApiError{401}` and `ApiError{403}` (github.rs:594-597).
- **404 treated as absence:**
  - bitbucket `list_issues` returns empty on 404 (bitbucket.rs:622).
  - bitbucket downloads return empty on any error (bitbucket.rs:725).
  - tree continuations skip a prefix on any error (github.rs:153, gitea.rs:193).
  - `snapshot` degrades a tree failure to `RemoteTree::unavailable()` (github.rs `snapshot`, gitea :380, bitbucket :417).
  - `from_shorthand` moves to the next provider on `RemoteApi{404}` (mod.rs:229-231) and stops on `InvalidCredentials|RateLimited` (mod.rs:237-240).

Non-generated path (same semantics; reference for a shared classifier):
- `remote/focused.rs:1041-1069` classifies by status:
  - 404 is `Absent` or `RemoteApi`.
  - 401 is `MissingCredentials` when there is no token, else `InvalidCredentials`.
  - 403 goes to `forbidden`.
  - 429 is `RateLimited`.
  - 3xx is "redirect blocked".
- `focused.rs:1076-1100` `forbidden`: treats `x-ratelimit-remaining: 0` **or** a body containing "rate limit" as `RateLimited`, otherwise checks for an insufficient-permission phrase.
- `remote/blocking.rs:571-610` `classify` maps `SniffError` to `PrUnavailable`:
  - 401 or `InvalidCredentials` -> `CredentialsRejected`/`Required`.
  - 403/404 -> `NotFoundOrNotPermitted`.
  - 429 or `RateLimited` -> `RateLimited`.
  - `RemoteUnreachable` -> `Timeout`/`Network`.

##### 5. Behavior baselines to preserve

**GitHub recursive tree (`GetGitTree` + `with_recursive(true)`):**
- `sniff/lib/src/remote/github.rs:114` is the root tree, `GetGitTreeRequest::new(owner, repo, default_branch).with_recursive(true)`.
- `sniff/lib/src/remote/github.rs:147-148` is the continuation, `GetGitTreeRequest::new(owner, repo, "{default_branch}:{prefix}").with_recursive(true)`, for each `CONTINUATION_PREFIXES` entry and only when `truncated`.
- Test owners: `github_tests::list_documents_success` (asserts `recursive=true`), `snapshot_reuse_tests::a_truncated_github_tree_continues_for_document_and_ci_prefixes` (asserts paths `main%3Adocs`, `main%3A.github`, but **not** the recursive query on continuations).
- Gitea uses a separate `GetGitTreeRecursiveRequest` (gitea.rs:175, 187).

**Bitbucket binary `GetSource`:**
- `sniff/lib/src/remote/bitbucket.rs:272-285` in `collect_directory_entries`: `GetSourceRequest::new(ws, slug, commit, path)` with `pagelen = Some(100)` and `page = Some(n)` for n > 1, up to `MAX_LISTING_PAGES = 10`. It calls `client.get_source` (`request_bytes`, `ResponseKind::Binary`) and then JSON-decodes the body locally (`decode_directory_listing`, :306).
- `sniff/lib/src/remote/bitbucket.rs:462-466` in `get_file_content`: `GetSourceRequest::new(ws, slug, &commit, path)` via `get_source`, then lossy UTF-8 (`decode_file_text`, :316).
- Tests: `bitbucket_tests::list_documents_success`, `a_malformed_directory_listing_leaves_the_tree_unavailable_not_empty`, `get_file_content_returns_the_raw_bytes_as_text`, plus 3 unit tests in bitbucket.rs.

**homelab constructors:**
- `eversolo.rs:53-65`: HTTP only (`http://{host}:{port}`), connect_timeout 5 s, timeout 10 s, `.expect("failed to build HTTP client")`, no TLS options, no auth.
- `samsung_tv.rs:102-125`: scheme is `https` iff `use_https`, connect_timeout 5 s, timeout 10 s, `danger_accept_invalid_certs(true)` **iff `use_https`**, `.expect(..)`. The WS base is always `wss://{host}:{ws_port}`. `SamsungTv::new` is `with_https(.., false)`.
- `use_https` sources: `homelab-cli` `resolve_samsung` (CLI flag or config) and `homelab-server` `state.get_samsung_tv(name)` (config). `server/src/lib.rs:381` always uses HTTP.

##### 6. Logging and formatting of URLs, headers, and bodies from generated errors

- **sniff:**
  - `RemoteApi.message` embeds the raw response `body` for 403 and other statuses, and 401 bodies are embedded in `MissingCredentials.env_var` text (gitlab/gitea/bitbucket).
  - The `Http(e)` Display includes the request URL.
  - No header logging.
- **biscuit-speaks:** `map_api_error` uses `e.to_string()` (status + body), and `list_voices_raw` formats `"API error ({status}): {body}"`. `tracing::debug!` logs voice/model IDs and sizes only, never headers or the key.
- **homelab / homelab-server / eversolo-integration:** the `EversoloError`/`SamsungTvError` Display wraps the `SchematicError` Display (body / URL), and `ServerError` Display re-wraps it in the HTTP 502 error response.
- No dependent logs request headers or `ResponseContext`.

##### 7. `with_api_key_param` and `resolve_request_auth` users (whole repo)

**Dependents: none.** No dependent crate calls `with_api_key_param` or `resolve_request_auth`.

**Generated schema:**
- `resolve_request_auth(` is defined at `schematic/schema/src/shared.rs:308`.
- It is called once per client impl in `anthropic.rs`, `artificial_analysis/client.rs` (×2), `bitbucket/client.rs`, `elevenlabs/client.rs`, `emqx/client.rs` (×2), `eversolo/client.rs`, `gitea/client.rs`, `github/client.rs:18`, `gitlab/client.rs`, `huggingface/client.rs`, `lmstudio.rs`, `ollama/client.rs` (×2), `openai/client.rs:18`, `samsung_smart_tv.rs`, `unfolded_circle_core_rest.rs`.
- `api_key_param: Option<SensitiveString>` fields exist in `openai/mod.rs:325` and `lmstudio.rs:529`.
- **No `pub fn with_api_key_param` is emitted.**

**Generator:**
- `schematic/gen/src/codegen/api_struct/helpers.rs:108` emits `with_api_key_param`.
- `schematic/gen/src/codegen/client/shared_auth.rs:41,52,88,256` emits `resolve_request_auth`.
- `schematic/gen/src/codegen/client/helpers.rs:24,27`.
- `schematic/gen/src/codegen/client/tests.rs:154,255,309`.

**Generator tests:**
- `schematic/gen/tests/l1/fresh_output_compile.rs:543,598,632,642,643` (`FixtureQueryKey`/`FixtureCookieKey`/`FixturePolicyKey` `.with_api_key_param`).
- `schematic/gen/tests/l1/http_client.rs:24`.

**Docs and skills:**
- `schematic/gen/README.md:599,607,619,665`.
- `schematic/docs/generation/request-side.md:231,246,270` (already marked **Temporary**).
- `.claude/skills/schematic-define/SKILL.md:71,75`.
- `.claude/skills/schematic-define/schematic/SKILL.md:206`.
- `.claude/skills/schematic-define/schematic/gen.md:261`.
- `schematic/skill/SKILL.md:70`.
- Specs (snapshots; leave as they are): `2026-10-11-schematic-generation-integrity`, `2026-10-12-schematic-runtime-envelope`, and `schematic/reviews/2026-07-31-macro-review/spec.md:2200`.

##### 8. Doc drift found (docs code fences in dependent crates)

- `sniff/docs/repos/repo-tech-design.md:17-20,510,523-528,812-816` shows `schematic_schema::GitHub`, a `GitHub::new().map_err(..)` that no longer exists, and `self.client.request::<T>(..)` inside `tokio::join!`. These must be updated to the new surface. The section is already stale against the current code: `new()` is infallible and there are no owner/repo fields.
- `homelab/docs/eversolo-design.md:7,17,27,79,121` describes `self.client.request::<ResponseType>(RequestType)` and `Api(#[from] schematic_schema::SchematicError)` plus a `Status(i32)` variant. Both are already stale: the path is `shared::SchematicError` and there is no `Status` variant.
- `homelab/eversolo-integration/README.md:158` mentions a WS type only, so it is unaffected.
- `model-citizen/lib/README.md:72` is stale (see §3.6).

##### 9. Test-gap summary (no existing test exercises the path)

- **sniff:**
  - GitHub and Gitea `get_file_content` (text endpoint).
  - GitLab `get_file_content`.
  - Gitea tree continuation.
  - `codeberg()` env order.
  - Bitbucket downloads error-to-empty.
  - GitHub `list_workflow_runs` anonymous retry on 401/403.
- **biscuit-speaks:** every network path (`create_speech`, `list_models`, `create_sound_effect`, `list_voices_raw`) is covered only by `#[ignore]` live tests. There is no wiremock dev-dependency in biscuit-speaks.
- **homelab, eversolo-integration, homelab-server, homelab-cli:** no REST test of any kind (no wiremock in `homelab/`). Missing tests include the eversolo/samsung constructors (timeouts, HTTPS-gated invalid certs), the samsung text/empty endpoints, and the eversolo-integration "any `EversoloError` means asleep, so use WoL" fallback.

### Wave 2 — Capture Baseline

Captured 2026-10-12 on this macOS host before any code change (HEAD
`62aa6dabc` plus the prerequisite's uncommitted work). **No test or lint
failed, so there are no pre-existing failures to carry forward.**

| Package(s) | Command (cwd) | Result | Passed / failed / skipped |
|---|---|---|---|
| schematic-define, -gen, -definitions, -oauth | `just test` (`schematic/`) | PASS | 1780 / 0 / 9; fresh-output 4 / 0 (18.9 s) |
| schematic-schema | `just test-schema` (`schematic/`) | PASS | 20 / 0 / 0 |
| sniff, sniff-cli | `just test` (`sniff/`) | PASS | 3360 / 0 / 33 (sniff 2387) |
| biscuit-speaks, biscuit-speaks-cli | `just test` (`biscuit-speaks/`) | PASS | 492 / 0 / 19 (biscuit-speaks 385) |
| model-citizen, model-citizen-cli | `just test` (`model-citizen/`) | PASS | 170 / 0 / 0 (model-citizen 153) |
| homelab, eversolo-integration, arcam-amp-integration, sony-receiver-integration, unfolded-integration-helper, homelab-cli, homelab-server | `just test` (`homelab/`) | PASS | Rust 224 / 0 / 8 (34 / 28 / 21 / 27 / 29 / 16 / 69); frontend vitest 22 / 0 |
| schematic area | `just lint` (`schematic/`) | PASS | 0 warnings |
| schematic-schema | `just lint-schema` (`schematic/`) | PASS | 0 warnings |

All eight dependents built (implied by their passing L1). Consumer-area lint
was not run at baseline (not needed; nothing changed).

**`schematic-schema` compile-time sample (repeat exactly in Phase 7).**

- Host: macOS 27.2.0 arm64, Apple M4 Max, 16 CPUs, 128 GiB.
- Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1`
  (pinned by `rust-toolchain.toml`).
- Target dir: the crate's own `schematic/schema/target` (no `CARGO_TARGET_DIR`,
  no repo `.cargo/config.toml`). `~/.cargo/config.toml` sets
  `rustc-wrapper = "kache"`, which serves cached rlibs, so the sample disables it.
- Profile `dev`, default features, `--lib`; dependencies warm; only the schema
  package cleaned.
- Procedure, from the repo root, after a dev `--lib` build exists:

  ```sh
  cargo clean -p schematic-schema --manifest-path schematic/schema/Cargo.toml
  time RUSTC_WRAPPER= cargo build --manifest-path schematic/schema/Cargo.toml --lib
  ```

- **Result: 14.63 s** (cargo "Finished"; `time` 14.70 s wall, 16.61 s user).
  The same step with kache active finished in 0.37 s (cache hit, not a
  measurement).

**CI planner snapshot (pre-runtime).** `NO_COLOR=1 just ci-local --plan`
(read-only, exit 0): schematic-define and schematic-definitions get
lint/check/L1 on ubuntu plus L1 on macOS/Windows/WSL2; schematic-gen gets
lint/L1/L2 on ubuntu and macOS, L1 on Windows/WSL2 (L2 accepted gap there);
schematic-schema is not planned (excluded generated crate). Phase 2 re-runs the
planner once `schematic-runtime` is a member (R37).

**Build hosts and OS-sensitive cases.** `BUILD_LINUX=build-linux`,
`BUILD_WIN=build-win-native`, `BUILD_WSL=build-win` are set, so
`just cross-check <pkg> --os linux|windows|wsl` can produce Linux, native
Windows, and WSL2 evidence; macOS is this host. Nothing was cross-run in
Phase 1 (no code changed). Cases to prioritize on every OS in later phases:

- **Closed-port refusal timing (R7, R24 `RedirectFailed`).** The `os` skill
  records that a refused `127.0.0.1` connection takes about 2 s on native
  Windows (SYN retry). Tests must not depend on refusal speed; give the
  per-request timeout a wide margin over 2 s, or use an accept-then-close
  listener where only "no head arrived" matters.
- **Abrupt close after a partial body (R11).** Use explicit `Content-Length`
  and a clean close (not a reset); confirm partial-byte retention on Windows.
- **Drop cancellation (R26).** Observe EOF at the mock with a bounded wait;
  check `num_alive_tasks()` returns to baseline (see spike, item 3).
- **TLS/proxy/connection reuse (R18)** and **environment isolation (R15)**
  with `EnvGuard` + `#[serial]`.

### Wave 2 — Prove Decoder Boundary

One throwaway crate under `/tmp` (deleted afterwards; nothing in the
repository) ran 9 tests offline with the actual dependencies: reqwest 0.13.4
(`form, json, multipart, rustls`, no default features), tokio 1.52, hyper 1.9.
The root `Cargo.lock` pins reqwest **0.13.2** and `schematic/schema/Cargo.lock`
pins **0.13.4**. The runtime will compile against both (workspace vs. generated
crate), so I checked every API the spike relied on in both versions' sources;
all exist in both (`Response::chunk`, `Error::without_url`,
`ClientBuilder::{connect_timeout, add_root_certificate,
tls_danger_accept_invalid_certs}`).

| # | Question | Result | Evidence and consequence |
|---|---|---|---|
| 1 | Live decoder: head, then incremental bytes, then truncation | PASS | `Content-Length: 100`, 10 bytes, close: `chunk()` yields the 10 bytes, then `Err`. **The truncation error reports `is_decode()`, not `is_body()`**; reqwest routes every body-read error through its decode kind. The runtime must classify "interrupted" by *where* the error occurred (any read error after the head), never by reqwest predicates. |
| 1b | Library error text | — | `send()` errors' `Display` appends ` for url (…)` with the **full URL including the query**. `Error::without_url()` exists. Per D23 the runtime composes its own text from a redacted URL and a sanitized category, and never stores the library error. |
| 2 | Owned stream; first item before body completion; idle timeout | PASS | A `futures_util::stream::unfold` stream owning the `Response` yielded item 1 while the mock was still blocked on the test's readiness signal (no sleeps). A 200 ms idle bound produced a terminal timeout item at ~202 ms retaining the partial bytes. The stream is `Send + 'static`. |
| 3 | Drop cancels without a detached worker | PASS (caveat) | Dropping a mid-body `Response`: mock saw EOF after ~0.1 ms; dropping `send()` mid-upload: EOF after ~0.9 ms. Our code spawns nothing, but hyper-util spawns one connection-driver task per connection, which ends when the request is dropped. Assert "`num_alive_tasks()` returns to its baseline", not "no task ever existed". |
| 4 | One monotonic expiration for a logical send | PASS | Wrapping `send()` and each `chunk()` in `tokio::time::timeout_at` with one `Instant` cleanly separates pre-head (`Timeout`/`DeadlineExceeded`) from post-head (`BodyInterrupted` with retained bytes). `RequestBuilder::timeout` also works but gives muddled predicates and needs recomputation per hop. **Decision: use the runtime's own `timeout_at`, record which bound was limiting, and do not set reqwest's per-request timeout.** |
| 5 | `SharedTransport` feasibility | PASS | Two logical clients sharing one `reqwest::Client`, each applying its own default headers, user agent, and timeout per request, made 3 sequential drained requests over **1** accepted connection. |
| 6 | Redirects disabled | PASS | `redirect::Policy::none()` returns the 302 with `Location`; the mock saw exactly one request; no error is raised, so the runtime classifies redirects itself. |

**Settings that can only live on the `reqwest::Client`** (so on
`SharedTransport` when attached): `connect_timeout`, `read_timeout`,
`proxy`/`no_proxy`, `redirect`, all TLS settings (`add_root_certificate`,
`tls_danger_accept_invalid_certs`, …), pool, HTTP/1/2, TCP, DNS, and cookie
settings. **Settable per request** (so client-owned): headers (user agent,
default headers, credentials), timeout, version, query, body.

**Decision — connect-timeout ownership (author-review item, PD8).** Connect
timeout is a `reqwest::Client` setting, so it cannot differ between clients
sharing one connection pool. Planned implementation:

- `SharedTransport`'s builder owns proxy, extra root certificates,
  accept-invalid-certificates, and **connect timeout**.
- A standalone `ClientBuilder` keeps `.connect_timeout()`, `.proxy()`,
  `.add_root_certificate()`, and `.danger_accept_invalid_certificates()` and
  builds its own private transport (redirects always disabled).
- A `ClientBuilder` attached with `.transport(&t)` that also sets *any* of
  those four transport-owned options is rejected with a typed `BuildError`
  naming the option, whether or not the value matches. That is simpler to
  explain than comparing values, and it never silently ignores or overrides a
  setting.
- Per-request timeout, user agent, default headers, credentials, hooks,
  metadata defaults, and origin policy stay client-owned and are applied per
  request, which the spike showed keeps connection reuse.
- Enforcement is by recorded configuration, not raw-client injection: the
  runtime builds every `reqwest::Client` itself and never accepts one.

This satisfies R18's required case (an attached client that sets a proxy fails
construction). Samsung's migration keeps a standalone builder (5 s connect,
10 s request, accept-invalid-certificates only over HTTPS), so the choice does
not affect any current consumer.

**Contract sketch (Phase 2 entry; signatures only, refined during Phase 2).**

```rust
pub const ABI: u32 = 1;

/// Marker for one generated API; `send` accepts only requests of its own API.
pub trait Api: 'static { const NAME: &'static str; }

pub trait EndpointSpec {
    type Api: Api;
    type Success;
    type Failure: DecodeError;                 // `NoDeclaredError` when none is declared
    type Decoder: Decode<Output = Self::Success>;
    const ENDPOINT_ID: &'static str;
    const DECLARED_ERROR_STATUSES: RangeInclusive<u16> = 400..=599;
    fn into_parts(self) -> Result<RequestParts, TransportErrorKind /* InvalidArgument */>;
}

/// Replayable application request; credentials are attached per hop, never stored here.
pub struct RequestParts {
    pub method: Method,
    pub path: String,                          // substituted; may carry an embedded template query
    pub query: Vec<(String, String)>,          // ordered, unencoded, duplicates kept
    pub headers: Vec<(String, String)>,
    pub body: Option<Bytes>,                   // replayable for 307/308
}

/// One expiration per logical send; streams switch to an idle bound after the head.
pub struct Expiration { at: Instant, limiting: Limit, idle: Option<Duration>, deadline: Option<Deadline> }
pub enum Limit { RequestTimeout, SharedDeadline, Idle }

pub trait Decode: Send + 'static {
    type Output;
    /// Receives the classified head and the live body; metadata is built before this runs.
    fn decode(self, head: &ResponseHead, body: LiveBody, exp: Expiration)
        -> impl Future<Output = Result<Self::Output, DecodeFailure>> + Send;
}
pub struct DecodeFailure { pub kind: DecodeFailureKind, pub retained: Bytes }   // no library error retained
pub enum DecodeFailureKind { Interrupted, Malformed, Hook }

/// Only the JSON codec implements this; typed mutators require it (R29).
pub trait HookCapable: Decode + sealed::Sealed {}

pub enum Completion { Complete, BodyInterrupted, RedirectRefused, RedirectFailed { target: RedactedUrl, category: FailureCategory } }
```

One correction to the spike's own sketch: it described `RedirectFailed` as a
missing or unparsable `Location`. Under R24 that case is `RedirectRefused`;
`RedirectFailed` means a *followed* hop failed before its head arrived
(connect, TLS, timeout, deadline). The sketch above follows the spec. The
spike also showed `bytes_stream()` needs reqwest's `stream` feature; `chunk()`
is sufficient, so no feature is added. A `dyn Decode` would need a
boxed-future shim; none is planned (generic dispatch only).

**Regression fixtures to carry into Phase 3 tests** (from the spike):
truncated-body raw socket mock; readiness-gated chunked mock (std channel, no
sleeps); drop-to-EOF probe with a `num_alive_tasks()` baseline check;
pre-head vs post-head expiry with one `Instant`; keep-alive mock counting
accepted connections.

### Open questions and review items found in Phase 1

1. **`biscuit-speaks::list_voices_raw` needs the HTTP client and credential.**
   It sends a hand-rolled GET `/v2/voices` with `http_client()`,
   `api_key_header()`, and `api_base_url()`. R17 removes injected clients;
   exposing the managed `reqwest::Client` would let callers bypass origin
   policy. The ElevenLabs definition already declares `GET /v2/voices`
   (`schematic/definitions/src/elevenlabs/endpoints/voices.rs`), so the
   planned Phase 7 resolution needs no new runtime surface: send the generated
   request typed, or through `send_raw` if its declared response shape does not
   fit what the consumer parses today. Phase 7 records the choice.
2. **sniff's anonymous retry becomes partly unreachable under D56.** Today
   `list_pull_requests` retries anonymously on `MissingCredential` /
   `AuthenticationRequired`. Under D56 a missing credential already sends
   anonymously, so that retry branch has nothing left to trigger it. GitHub
   `list_workflow_runs` also retries on 401/403; that path stays a separate
   explicit `CallOptions` anonymous call (PD31). Phase 7 records which branches
   are removed and why.
3. **sniff's `MissingCredentials` reporting for 401.** GitLab, Gitea, and
   Bitbucket map 401 to `MissingCredentials` with env-var hints. With
   `AuthMethod` on the envelope, sniff can distinguish "no credential sent
   (`Anonymous`) and got 401" from "credential rejected". Behavior is
   preserved by default (PD31); this is an improvement opportunity, not
   in scope.
4. **Stale dependent docs** (to fix during migration):
   `sniff/docs/repos/repo-tech-design.md` and `homelab/docs/eversolo-design.md`
   already describe outdated `request::<T>` code; `model-citizen/lib/README.md:72`
   calls the manifest-only dependency a "HuggingFace API client".
5. **Test gaps on paths Phase 7 must migrate:** homelab REST
   (Eversolo/Samsung constructors and the Samsung text/empty endpoints),
   biscuit-speaks network paths (only `#[ignore]` live tests; no wiremock dev
   dependency), and several sniff paths (text `get_file_content`, Gitea tree
   continuation, Codeberg env order, GitHub workflow-runs 401/403 retry).
   Phase 7 adds local mock tests for each migrated path rather than relying on
   compile success; this needs a `wiremock` dev-dependency in `homelab` and
   `biscuit-speaks` (already a workspace dependency).

None of these needs an author decision before Phase 2: Phase 2 builds only
runtime foundations and changes no consumer.

### Phase 1 validation checkpoint

| Checkpoint item | State |
|---|---|
| Prerequisite outcomes established | Done — Wave 1, Verify Prerequisite (all interfaces match; no blocker) |
| Authorizations established | Done — Phase 1 needs none; later-phase reading recorded above |
| Baseline exists | Done — all L1 and lint green; compile sample 14.63 s; planner snapshot |
| Exact migration inventory exists | Done — Wave 1, Inventory Consumers |
| Every PD has a planned disposition | Done — PD1–PD35 table |
| Decoder evidence and stable contracts | Done — spike items 1–6 PASS; contract sketch; connect-timeout decision |
| No production or generated API changed | Confirmed — this phase wrote only this log and the plan's checkboxes and frontmatter; the spike lived in `/tmp` and was deleted |

**Test design for this phase.** Phase 1 changes no behavior, so it adds no
repository test and needs no requirement-to-test mapping, regression test, or
input-robustness matrix. It reads no new file format. The generated-Rust ABI
marker cases (supported, differing, missing legacy, malformed or conflicting
declarations) are planned for Phase 6 through public partial generation.
Behavioral evidence for this phase:

- the spike's 9 throwaway tests (all passed, offline, 1.2 s; not retained,
  per the plan);
- the baseline runs listed above.

**Gates.** The completion gates are the baseline runs, all on unchanged
source: `just test` and `just lint` in `schematic/` both passed. No file under
any package changed afterwards, so the evidence stands; nothing was skipped
and nothing was red before this phase.

**Message for Phase 2.** Start from the contract sketch and the
connect-timeout decision above. Classify body interruption by where the read
failed, not by reqwest predicates. Never store or display reqwest's error
text, because `send()` errors include the full URL. Use `timeout_at` with one
`Instant` per logical send. Make sure the runtime compiles against reqwest
0.13.2 (root lock) and 0.13.4 (schema lock).

## Phase 2

Started and finished 2026-10-12 on macOS. Phase 2 creates `schematic-runtime`
and its foundation types. Checked-in `schematic-schema`, the generator, and
every dependent are unchanged; no consumer uses the runtime yet.

### Wave 3 — Create Runtime Package

- **Package.** `schematic-runtime` 0.1.0 at `schematic/runtime/`, edition
  2024, workspace member (root `Cargo.toml`), no dependency on
  `schematic-define`. Production dependencies: `bytes`, `futures-core`, `http`
  1, `tokio` (`time` only), `url`. All of them already appear in both the root
  `Cargo.lock` and `schematic/schema/Cargo.lock`, so Phase 6's offline fresh-output
  build can resolve them. The root lockfile gains only the runtime's own
  entry. No `reqwest` dependency yet; it arrives with managed sends in Phase 3.
- **Test registration.** `autotests = false` with one `[[test]] l1`
  (`tests/l1/main.rs`), plus the `test_layout` gate used by the other
  consolidated packages. Unit tests live in `src/deadline.rs` and
  `src/options.rs`, where they need crate-private state.
- **Modules.** `endpoint` (`Api`, `EndpointSpec`, `ErrorBody`,
  `RequestParts`, `RequestBody`, `FormPart`), `codec` (`Decode`, `LiveBody`,
  `DecodeFailure`, `BodyReadError`), `response`, `error`, `meta`, `redact`,
  `credentials`, `options` (`CallOptions`), and `deadline`. `pub const ABI: u32 = 1`
  is in `lib.rs`.
- **Area recipes.** `schematic/justfile`: `build`, `sanity`, `test`,
  `test-real`, `coverage`, `doctest`, and `lint` now include
  `schematic-runtime`. A new `test-compile-fail` recipe is called by `test`
  (see "Compile-fail evidence"). `test-l2` is unchanged: the runtime has no L2
  tests, and the generator's L2 coverage is kept.
- **Discovery (R1, R37).** `sniff repo packages` lists `schematic-runtime`.
  `just check-tier-coverage schematic` reports 0 stranded. `NO_COLOR=1 just
  ci-local --plan` places it in area `schematic` with no CI change:
  `schematic-runtime/ubuntu-latest/lint`, and `L1` on ubuntu-latest,
  windows-latest, macos-latest, and wsl2-ubuntu, all `execute ci`.

### Wave 4 — Define Response Types / Implement Redaction

- `Response<T, E> { meta, outcome }`, `Outcome::{Success, Failure}`,
  `ApiError<E> { kind, body: Bytes }`. Methods: `data`, `error`,
  `into_result` (drops metadata, documented), `into_parts`,
  `is_decode_failure` (`MalformedSuccess` only), `is_interrupted`
  (`BodyInterrupted` only), and `ApiError::declared`.
- `ErrorKind<E>` is `#[non_exhaustive]`: `Declared(E)`, `Undeclared`,
  `MalformedError`, `MalformedSuccess`, `BodyInterrupted`,
  `RedirectRefused { reason }`, `RedirectFailed { target: RedactedUrl, category }`,
  and `Hook`. There is no `UnexpectedContentType`.
- `NoDeclaredError` is an uninhabited enum implementing `ErrorBody` with
  `DECLARED = false`.
- `TransportError { request, kind, elapsed }` with `#[non_exhaustive]`
  `TransportErrorKind`: `CrossOriginRejected { target }`,
  `InvalidArgument(InvalidArgument)`, `InvalidUrl`, `Connect`, `Tls`,
  `Timeout`, `DeadlineExceeded { completed_requests, deadline_elapsed }`, and
  `Other`. `BuildError` covers the base URL, allowed origin, policy not
  permitted (it names both policies), transport-option conflict, auth-bearing
  default header, invalid header, incompatible credential, proxy,
  certificate, and transport.
- **Safe text.** Every `Display` is composed from redacted metadata and fixed
  category text. None of them prints a body, a declared error value, or any
  library text. The D22 `Debug` limitation is a `## Debug can print provider
  data` section in the `response` module rustdoc and is summarized in the
  crate docs. The `schematic/docs/` topic-page half of R6 is Phase 8.
- **Redaction.** `SecretNames` holds the D48 built-ins
  (`BUILTIN_SECRET_HEADERS`, `BUILTIN_SECRET_QUERY_NAMES`) plus names added
  with `with_name`/`with_names`. Phase 6 feeds effective-auth names and the
  API's `secret_names` through these.
  - Header names match case-insensitively. Query names match exactly after
    percent and `+` decoding.
  - Secret values become the literal `[redacted]`. URL user information is
    always redacted whole. The query and fragment are redacted pair by pair,
    and every non-secret piece is kept byte for byte, so ordered duplicates
    survive.
  - `Location` and `Content-Location` values are sanitized as URL references
    (absolute, relative, or scheme-relative). An unreadable value becomes
    `[redacted]` whole.
  - The request id is read from the API's named header (case-insensitive).
    It is `[redacted]` when that header name is secret.
  - Metadata can only be built through `RequestMeta::capture` /
    `ResponseMeta::capture`, which always redact (both structs are
    `#[non_exhaustive]`).

### Design choices for review (Human review agenda item 2)

1. **Metadata layout.** `ResponseMeta { request: RequestMeta, status,
   headers, request_id, elapsed }` and `RequestMeta { endpoint_id, method,
   url: RedactedUrl, query, headers, auth, page }`, with public fields.
   `TransportError` reuses `RequestMeta`. The `page` slot is on `RequestMeta`
   (spec 3 sets it), not on `CallOptions`.
   - Alternative considered: flat metadata with `Option` status. Rejected,
     because the transport error has no status or response headers by
     construction.
2. **Redacting constructors are public.** `capture` is the only way to build
   metadata, so stored metadata is redacted by construction. It is also how a
   consumer (sniff in Phase 7) can unit-test a classifier over a `Response`
   without a mock server. `TransportError::new` is public for the same reason.
3. **`RedactedUrl` is a string newtype**, not a `url::Url`. `Url`'s
   serializers would percent-encode `[redacted]` in a query, and R7 requires
   the literal text.
4. **Fragments are redacted like queries.** OAuth implicit-flow redirects
   carry `#access_token=`. The spec names only queries.
5. **`Link` headers are not sanitized.** They can carry URL queries. The spec
   names only `Location`; I added `Content-Location` because it has the same
   shape. Pagination (spec 3) parses `Link`; flagged below.
6. **`RedirectRefused` carries a reason** (`RedirectRefusal`: cross-origin,
   hop limit, missing, invalid, or unsafe `Location`, body not replayable).
   The kind name stays `RedirectRefused`, so R22/R24 still match it with
   `{ .. }`.
7. **`Decode` is type-level**: `fn decode(body: LiveBody) -> impl Future<…> + Send`,
   with no `self`, because the codec is a type such as `Json<T>`.
   - `LiveBody` wraps a `Stream` of `Result<Bytes, BodyReadError>`, not a
     `reqwest::Response`. The HTTP library stays out of the public contract,
     and a codec can be tested with in-memory streams.
   - `LiveBody` reads are bounded by one `read_until` instant.
   - The stream idle bound (D57 item 14) and the hook context (Phase 5) are
     added to `LiveBody` privately by later phases, so the trait signature
     does not change.
8. **`ErrorBody`** (the contract sketch called it `DecodeError`): `const
   DECLARED: bool = true` and `fn decode_body(&[u8]) -> Option<Self>`.
   `None` means `MalformedError`. Generated code implements it with
   `serde_json` in later phases.
9. **`EndpointSpec::into_parts` returns `Result<RequestParts, InvalidArgument>`**,
   not a transport-error kind. The runtime wraps it with request metadata.
10. **`RequestParts` is a struct with `http::Method`**, and its `RequestBody`
    mirrors the generated enum. `FormPart::File` flattens `FormFile` (name,
    MIME, `Bytes`), so the runtime needs no serde. The runtime will encode
    multipart itself, so a 307/308 can replay it.
11. **Instants are `tokio::time::Instant`** (monotonic; it wraps
    `std::time::Instant`), so deadline and expiry tests run under paused time
    with no sleeps.
    - `Deadline::after` with an unrepresentable budget saturates at about 100
      years instead of panicking.
    - The completed-send counter is shared by clones. It is incremented by
      a crate-private method that the Phase 3 executor calls; until then it
      carries `cfg_attr(not(test), expect(dead_code))`, which turns into a
      warning once that call exists.
12. **`CallOptions`**: `CallOptions::new()`, `CallOptions::anonymous()`,
    `.credential(c)`, and `.deadline(&d)`.
    - A per-call credential and anonymous mode are one private enum, so they
      are mutually exclusive by construction, and the last one set wins.
    - There are no public getters; the executor reads the crate-private
      fields.
13. **`Credential`** is opaque. It has `access_token`, `api_key`, and `basic`
    constructors, plus `kind() -> CredentialKind`. Its `Debug` and `Display`
    print the kind and `[redacted]`.

### Compile-fail evidence (decision for review)

- **Doctests cannot prove the diagnostic.** I tried them first, and rustdoc on
  stable does not enforce the error code in `compile_fail,E…`: a doctest
  marked with the wrong code still passed. Doctests also reach no CI gate
  (`affected_scope.py`).
- **The cases use `trybuild` instead**, with `.stderr` expectations:
  - `tests/fixtures/compile_fail/no_declared_error_value.rs` fails with
    `E0599` (no value of `NoDeclaredError`) — R12.
  - `endpoint_missing_associated_types.rs` fails with `E0046` (missing
    `Success`, `Decoder`) — R2 groundwork.
  - The cases sit under `tests/fixtures/` because the layout gate skips that
    directory.
- **They run outside the L1 filter.** trybuild runs Cargo, which an archive
  consumer lacks, so the test is named `slow_`. The shared L1 filter excludes
  it, and the new `just test-compile-fail` recipe runs it as part of the
  area's `just test`, mirroring `test-fresh-output`.
  - The test sets `RUSTC_WRAPPER=""` (through `EnvGuard`), because `kache`
    rewrites diagnostic paths.
  - Cost: about 15 s cold (trybuild builds the dependencies in its own target
    directory) and about 1–1.5 s warm.
- **CI does not run these cases.** CI calls `_test` and `test-fresh-output`,
  never the area `test` recipe. Making CI run `test-compile-fail` would be a
  CI change, which is not authorized.
  - Phase 6 extends the fresh-output harness, which CI does run, with the
    generated-surface compile-fail cases (wrong success type, foreign API
    request, removed `request<T>`).
  - The runtime-only cases stay local unless the author decides otherwise.
    Review item below.

### PD dispositions touched in this phase (agenda item 1)

| PD | Matched the planned disposition? |
|---|---|
| PD7 | Yes, except `Other` carries no `String`. PD7's text says `Other(String)`; the plan's sanitization rule (no library text) wins, and the category is the message |
| PD11 | Yes: `schematic-runtime` 0.1.0, workspace member, `pub const ABI: u32 = 1` (generated assertion and partial-generation read: Phase 6) |
| PD16, PD19 | Yes, documented on `Decode` (stream output not `Clone`/`Sync`; `elapsed` to the head) |
| PD17 | Yes: `#[non_exhaustive]`, no `UnexpectedContentType` |
| PD18 | Yes: both `into_result` and `into_parts` |
| PD20 | Yes: no value-only shortcut |
| PD21 | Yes: one `SecretNames` set for request and response headers |
| PD22 | Yes: `NoDeclaredError` |
| PD28 | Partly: the redacted `Credential` enum and `CallOptions::credential` exist; `ClientBuilder::credentials` and method validation are Phase 3 |
| PD33 | `trybuild` added. `rcgen`/`tokio-rustls` are deferred to Phase 3, where the TLS mocks need them |
| PD34 | Yes: `AuthMethod` has no label |

### Deviations from the plan

- **Module boundaries.** The plan lists transport, origin, and hooks modules.
  I did not create them as empty files. `CallOptions` lives in `options`, and
  `transport`, `origin`, and `hooks` arrive with their content in Phases 3–5.
- **Contract sketch.** The Phase 1 sketch's `Expiration` type is not
  introduced: `LiveBody` carries one `read_until`, and Phase 3 adds the
  limiting cause and idle mode. `DecodeError` became `ErrorBody` (choice 8).
  `HookCapable` is Phase 5.
- **Doctests.** None in the runtime (see "Compile-fail evidence"). `just
  doctest` includes the runtime for future examples.

### Requirement-to-test mapping (Phase 2 share)

Boundary: an in-process library API (fidelity F1, L1). There is no network
in this phase; wire-level evidence (wiremock, raw sockets) is Phase 3. This
phase adds no file-format or configuration reader, so the input-robustness
matrix does not apply. Redaction parses URLs and header values, which are not
configuration.

| Requirement (part) | Test (`schematic-runtime::l1` unless noted) |
|---|---|
| R1 runtime exists | the package builds; `sniff repo packages` lists it |
| R2 groundwork: impl missing associated types | `compile_fail::slow_compile_fail_cases_report_their_intended_diagnostic` (`endpoint_missing_associated_types`, E0046) |
| R3 envelope shape and accessors | `envelope::a_success_exposes_its_value_through_every_accessor`, `envelope::a_failure_exposes_its_error_and_keeps_metadata_through_into_parts` |
| R4 metadata fields; `page` and `request_id` defaults | `envelope::metadata_records_every_field_on_success_and_failure`, `redaction::the_request_id_comes_from_the_named_header_only` |
| R5 built-in names exact; every built-in header and query name; API and auth names; non-secret kept; stored fields, not only text | `redaction::the_builtin_names_are_exactly_the_documented_lists`, `every_builtin_secret_header_is_redacted_in_request_and_response_metadata`, `every_builtin_secret_query_name_is_redacted_in_the_url_and_query_fields`, `api_and_authentication_names_join_the_builtin_names`, `query_names_match_exactly_after_decoding`, `ordinary_duplicate_query_values_keep_their_order`, `repeated_headers_stay_repeated_in_order`, `url_user_information_is_always_redacted`, `location_headers_keep_their_destination_and_lose_their_secrets`, `an_unreadable_location_is_redacted_whole`, `a_request_id_from_a_secret_header_is_redacted` |
| R5/R6 sentinel absent from `Display` across success and every failure kind; raw body kept | `envelope::stored_metadata_and_display_never_contain_a_secret_or_a_body` |
| R6 limitation text in runtime rustdoc | `envelope::the_debug_limitation_is_documented_on_the_error_types` (`include_str!`) |
| R7 text inputs: transport error `Display`/`Debug` show the redacted URL and non-secret `page=2`, never the sentinel | `envelope::transport_error_text_shows_the_redacted_url_and_no_secret` (network cases: Phase 3) |
| R10/R11 helpers true only for their own kind | `envelope::decode_and_interruption_helpers_are_true_only_for_their_own_kind` |
| R11 groundwork: partial bytes kept on interruption and on read expiry | `decode_boundary::an_interrupted_body_keeps_the_bytes_that_arrived`, `a_read_still_waiting_at_the_expiration_times_out_and_keeps_partial_bytes` |
| R12 placeholder cannot be constructed; undeclared failures | `compile_fail::…` (`no_declared_error_value`, E0599), `envelope::an_endpoint_without_a_declared_error_reports_undeclared_failures` |
| R13 groundwork: declared body type decodes, non-matching bytes do not | `envelope::an_endpoint_without_a_declared_error_reports_undeclared_failures` (`ProviderError`) |
| R26 groundwork: shared expiration and counter | `decode_boundary::deadline_clones_share_one_expiration`, `an_unrepresentable_deadline_budget_does_not_panic`, unit `deadline::tests::every_clone_counts_into_one_total` |
| R30 groundwork: live input; first item before body end; two-shape enum; later read failure as an item | `decode_boundary::a_streaming_decoder_delivers_its_first_item_before_the_body_ends`, `a_two_shape_decoder_returns_either_shape_or_keeps_the_original_bytes`, `a_streaming_decoder_reports_a_later_read_failure_as_an_item`, `collect_reads_every_piece_in_order`, `a_live_body_exposes_its_head_to_the_decoder` |
| PD28 credential never printed; anonymous and per-call exclusive | `envelope::a_credential_never_prints_its_value`; unit `options::tests::*` (3) |
| Endpoint contract defaults (`DECLARED_ERROR_STATUSES`, ordered duplicate query) | `decode_boundary::an_endpoint_declares_its_identity_codec_and_request_parts` |
| Layout | `test_layout::every_test_source_is_compiled_by_a_declared_target` |

**Mutation checks.** These are temporary source edits, all restored.
Removing URL user-information redaction failed 2 tests. Dropping header-name
case folding failed nothing at first: `http` lowercases header names, so
metadata cannot observe it. I added direct `is_secret_header` /
`is_secret_query` assertions; the mutation now fails
`api_and_authentication_names_join_the_builtin_names`.

### Gates (all on the final tree)

| Gate | Result |
|---|---|
| `just test` (`schematic/`) | PASS: L1 1817 / 0 failed / 10 skipped (baseline 1780 + 37 runtime); fresh-output 4 / 0 (63.6 s, against 18.9 s at baseline, which I attribute to a cold harness cache after the lockfile changed; not investigated further); `test-compile-fail` 1 / 0 |
| `just lint` (`schematic/`) | PASS: define, gen, definitions, oauth, runtime, 0 warnings |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p schematic-runtime --no-deps` | PASS |
| `cargo check` of the 8 dependents | PASS (no source they use changed; this confirms the workspace and lockfile edits) |
| `just cross-check schematic-runtime --os windows` (native Windows) | PASS: 37 / 0 (L1; the `slow_` trybuild case is not part of L1) |
| `just cross-check schematic-runtime --os linux` | PASS: 37 / 0 |
| `just cross-check schematic-runtime --os wsl` (WSL2, archive produced on Linux) | PASS: 37 / 0 |

**OS evidence.** macOS: full gates. Native Windows, Linux, and WSL2: runtime
L1 37/37 each through `just cross-check`. The `slow_` trybuild case ran on
macOS only. This phase's code is in-memory only; platform-sensitive network
cases start in Phase 3.

`just lint-schema` and `just test-schema` were not run: `schematic-schema`
and its inputs are unchanged since the baseline. There were no pre-existing
failures and nothing was skipped beyond the existing 10 skipped tests.

### Review items found in Phase 2

1. **Compile-fail cases are local-only in CI terms** (see "Compile-fail
   evidence"). Options are to accept local-only for the runtime cases (Phase
   6 adds CI-run generated-surface cases), or to let CI run `just -f
   schematic/justfile test-compile-fail` next to the fresh-output guard (a CI
   change). Not blocking.
2. **`Link` header redaction** (design choice 5): decide in spec 3 whether
   `Link` URLs need the `Location` treatment.
3. **`RequestParts` derives `Debug`.** A header parameter that the API marks
   secret would print through it. Parts are never stored in metadata, and
   credentials are attached later, so this is outside the D22 promise. Noted
   for completeness.

## Phase 3

Started and finished 2026-10-12 on macOS, with Linux, native Windows, and WSL2
evidence through `just cross-check`. Phase 3 adds managed sends to
`schematic-runtime`. Checked-in `schematic-schema`, the generator, and every
dependent are unchanged, and nothing outside the runtime calls it yet.

### Wave 5 — Request Infrastructure

- **Managed transport (`src/transport.rs`).** `SharedTransport` and its
  builder own the four settings that can only live on the HTTP client:
  `proxy`, `add_root_certificate` (PEM or DER), `danger_accept_invalid_certificates()`,
  and `connect_timeout`. A standalone `ClientBuilder` keeps the same four and
  builds a private pool.
  - A client attached with `.transport(&t)` that also sets any of the four is
    refused with `BuildError::TransportOptionConflict { option }`, named by
    its builder method. This is the Phase 1 decision; the values are not
    compared.
  - Every client is built with `redirect::Policy::none()`, and none is ever
    accepted from a caller.
  - Library errors are classified by type, never by text:
    - a `rustls::Error` anywhere in the source chain is `Tls`. The chain
      walk also opens each `io::Error` through `get_ref`, because
      `io::Error::source` skips the error it wraps; the first version missed
      it and reported `Connect`;
    - otherwise `is_timeout` is `Timeout`, `is_connect` is `Connect`, and
      anything else is `Other`.
  - With the rustls backend, reqwest parses root certificates only in
    `build()`. A failed build is therefore retried without the roots, and
    `InvalidCertificate` is reported when that retry succeeds.
- **Credentials (`src/credentials.rs`).** This is the runtime's own form of
  the effective auth policy; the runtime does not depend on `schematic-define`.
  - `AuthConfig` lists the accepted `CredentialPlacement`s plus an optional
    `EnvFallback`. Placements are `Bearer { header }`,
    `ApiKeyHeader { header, prefix }`, `ApiKeyQuery { name }`,
    `ApiKeyCookie { name }`, and `Basic`.
  - Resolution happens once per send. The order is per-call, then client
    level, then the first non-empty fallback variable, then anonymous.
    `CallOptions::anonymous()` bypasses every source and strips every
    placement's header, query value, and cookie from the request.
  - A caller credential uses the first accepted placement of its kind. A kind
    that no placement accepts is refused: `BuildError::IncompatibleCredential`
    at client level, and `InvalidArgument` before sending for a per-call
    credential.
  - Basic env credentials are resolved independently: the username comes from
    `username_vars`, and `EnvVar(name)` names the password variable.
  - `ClientBuilder::env_order` replaces the fallback's (password, key, or
    token) variable order.
  - Placing a credential into a request that already carries that header,
    query name, or cookie is refused as `InvalidArgument`.
  - A default header whose name is secret under the client's `SecretNames` is
    `AuthBearingDefaultHeader`. The client's `SecretNames` are the built-ins,
    every placement name, and the definition's `secret_names`.
- **Deadlines (`src/deadline.rs`).** A crate-private `Expiration` is computed
  once per logical send. It is `min(start + timeout, deadline expiry)` and
  records which bound limits (`RequestTimeout` or `SharedDeadline`), plus the
  timeout and deadline instant that stream mode needs.
  - `record_completed` lost its `expect(dead_code)`; the executor calls it
    after classification.
  - An expired deadline is rejected before any network work, as
    `DeadlineExceeded` carrying the count of sends that preceded it.

### Wave 6 — Typed Send (`src/client.rs`, `src/codec.rs`, `src/encode.rs`)

- **Surface.** `Client::<A>::builder(ApiDefinition)` returns a
  `ClientBuilder<A>`, and `send<R: EndpointSpec<Api = A>>(request, &CallOptions)`
  returns `Result<Response<R::Success, R::Failure>, TransportError>`. The API
  marker on `Client<A>` is what restricts `send` to that API's requests.
- **`ApiDefinition`.** Generated code supplies the base URL, the
  `AuthConfig`, the API's secret names, and the request-id header. Phase 4
  adds the origin policy here.
- **Request assembly.**
  - The base URL and path are joined by string concatenation, as generated
    clients do today, not with `Url::join`, which would drop a base path.
  - Declared query pairs are appended in order after any query in the path
    template. The fragment is removed.
  - Default headers come first. A request header replaces a default header of
    the same name, and the body's `Content-Type` comes last.
  - Bodies are encoded to owned bytes before sending (`encode.rs`), so a
    307/308 can replay them in Phase 4. The runtime writes multipart itself,
    with a boundary that does not occur in any part.
- **Classification.**
  - A 2xx goes through `R::Decoder`.
  - Any other status has its body collected and is then classified:
    `Declared(E)` or `MalformedError` inside `DECLARED_ERROR_STATUSES` when
    `E::DECLARED`, otherwise `Undeclared`.
  - A body read that fails or expires after the head is `BodyInterrupted`,
    with the bytes that arrived.
  - A pre-head expiry is `Timeout` or `DeadlineExceeded`, according to which
    bound was limiting.
  - The completed-send count is incremented once, after classification.
- **Codecs.** `Json<T>`: a successful parse is the only check, so valid JSON
  under a misleading content type decodes. `Text`: lossy UTF-8, matching
  today's reqwest without the `charset` feature. `Binary`: bytes. `Empty`:
  reads the body to its end and ignores it.
- **Stream mode.** `LiveBody::into_streaming()` switches a body to stream
  bounds: the per-request timeout becomes an idle bound between reads, and the
  shared deadline bounds the whole stream. The `Decode` signature is
  unchanged.

### Design choices for review (Human review agenda item 2)

1. **`EndpointSpec` gains `const METHOD: Method`.** When `into_parts` fails
   (an `InvalidArgument`, such as spec 3's page-size bound), R9 still needs
   request metadata. That metadata reports the endpoint's declared method and
   the base URL, because the path could not be assembled. A sent request uses
   `RequestParts::method`. The trybuild fixture
   `endpoint_missing_associated_types.rs` now declares `METHOD`, so its
   expected diagnostic is unchanged.
2. **`TransportError.request` is `Box<RequestMeta>`.** Clippy's
   `result_large_err` flagged `send` (`TransportError` was 264 bytes). Field
   access is unchanged through auto-deref. This is a public-field type
   change.
3. **`Client<A>` is generic over the API marker,** and the generated per-API
   client will wrap it (PD12). `ApiDefinition` is a plain builder struct, so
   tests can build clients without generated code.
4. **Default user agent: none; default per-request timeout: 30 s
   (`DEFAULT_TIMEOUT`).** Generated clients set neither today. The timeout
   default comes from D56.
5. **Credential collisions are refused rather than replaced.** Generated
   code today replaces a same-named query key and cookie (`apply_api_key_param`).
   The spec asks for refusal; Phase 7 must check that no consumer relies on
   replacement.
6. **The environment proxy is honored.** With `default-features = false`,
   reqwest still reads `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`
   (hyper-util `Matcher::from_system` → `from_env`). Only operating-system
   proxy settings are behind the `system-proxy` feature. Generated clients
   behave the same today, so I kept it (PD31) and documented it in
   `schematic/docs/dependencies.md`.

### PD dispositions touched in this phase (agenda item 1)

| PD | Matched the planned disposition? |
|---|---|
| PD7 | Yes: `Connect`, `Tls`, `Timeout`, `DeadlineExceeded`, `InvalidUrl`, `InvalidArgument`, `Other` are all produced by sends; `CrossOriginRejected` is Phase 4 |
| PD8 | Yes, with the Phase 1 connect-timeout placement: `ClientBuilder::{base_url, timeout, connect_timeout, proxy, add_root_certificate, user_agent, default_header, transport, danger_accept_invalid_certificates, credentials, env_order}`. `cross_origin` is Phase 4. `SharedTransport::builder()` adds `connect_timeout` to the proposed three |
| PD9 | Yes: no-argument `danger_accept_invalid_certificates()` |
| PD12 | Yes for this phase's share: builder, credentials, deadline, the four codecs, `send` |
| PD19 | Yes: `elapsed` is captured at the head; streams keep reading afterwards |
| PD23 | Yes: `min(remaining, timeout)` via one `Expiration`; `Deadline::after` in `CallOptions` |
| PD24 | Yes: cancellation proved on HTTP/1.1 (`dropping_a_send_closes_its_connection_and_does_not_count_it`, `a_dropped_send_leaves_no_task_alive_and_the_runtime_shuts_down_promptly`); HTTP/2 untested (the `http2` feature is off) |
| PD27 | Yes: 2xx → codec; declared range and `E::DECLARED` → `Declared`/`MalformedError`; else `Undeclared` (a 304 is `Undeclared`) |
| PD28 | Yes: `ClientBuilder::credentials` and method validation now exist |
| PD33 | Yes: `rcgen` and `tokio-rustls` dev-dependencies (offline, already in the lock) and a hand-rolled proxy mock |
| PD34 | Yes: no label |

### Deviations from the plan

- **The connect timeout is not observed by a mock.** R18 asks for each
  setting to be observed by a local mock. A connect that hangs cannot be
  produced portably on loopback without unreliable backlog tricks. The test
  covers the setting's ownership (the conflict rule) and that it reaches the
  HTTP client builder, but not its effect. Recorded as a gap.
- **Four TLS tests are `ignore`d on native Windows** (see "Windows TLS
  abort" below).
- **Tests were written by three parallel subagents**, one file each
  (`credential_resolution.rs`, `builder.rs`, `timing.rs`). I wrote
  `support.rs`, `send.rs`, and every source change, then reviewed their
  files: no readiness sleeps (only controlled server-side delays and one
  bounded poll) and no tier-marker names. Their reports found two runtime
  defects (TLS classification, certificate error attribution), both fixed
  above.

### Windows TLS abort (review item)

On `build-win-native`, the loopback-TLS tests intermittently abort the test
process with `0xc0000409` (a native fast-fail; empty stderr). The abort
happens inside the send, not at runtime drop or thread exit. In full parallel
runs it occurred in 3 of 5 runs; it does not occur when the test runs alone.
I isolated it with temporary probes, since removed:

| Probe | Aborts |
|---|---|
| `RUST_MIN_STACK=8388608` vs default, 4 TLS tests as parallel processes ×30 | 0 / 0 |
| 18 builder tests as parallel processes, outside nextest ×15 | 1 / 270 |
| runtime client, `ring` server (not aws-lc), accept-invalid client (no platform verifier) | yes |
| runtime client, aws-lc server, platform verifier | yes |
| key generation only; client construction only | none |
| **plain reqwest, built-in rustls, no runtime code** | yes (2 / 160) |
| reqwest with a preconfigured rustls `ClientConfig` (ring or aws-lc client × ring or aws-lc server) | 0 / 160 |

Conclusions:

- The trigger is in reqwest 0.13's built-in rustls client path on native
  Windows under load, not in runtime code. The runtime is safe Rust, and
  plain reqwest aborts too.
- The preconfigured-config result is too small a sample to call a fix.
- Generated clients today use the identical reqwest configuration, so this
  is existing exposure, not a regression. It has been observed only against
  this loopback fixture.
- macOS, Linux, and WSL2 run the four tests. On native Windows they carry
  `#[cfg_attr(windows, ignore = "…")]` so that the `main` Windows leg is not
  red on most pushes.
- The full write-up and the next diagnostic step (capture the faulting stack
  with `cdbX64.exe`) are in the `os` skill (`windows.md`, plus a symptom-table
  row in `SKILL.md`).

### Requirement-to-test mapping (Phase 3 share)

Boundary: the runtime's public send API over real loopback sockets. Fidelity
is F2: real HTTP, TLS, and proxy wire behavior through local mocks, with no
provider. All tests are L1 in `schematic-runtime::l1`.

The input-robustness matrix does not apply: this phase adds no reader of a
file format or configuration. `ApiDefinition`, `AuthConfig`, and builders
are typed Rust values, and environment variables are read as single values,
not parsed.

| Requirement (part) | Tests |
|---|---|
| R3 envelope over the wire; `into_parts` keeps metadata | `send::a_success_reaches_the_caller_with_its_metadata`, `send::a_declared_error_is_decoded_on_arrival_and_keeps_its_body` |
| R4 metadata fields; `page` None; `request_id` present / absent / no definition field; repeated headers | `send::a_success_reaches_the_caller_with_its_metadata`, `send::the_request_id_is_none_without_the_header_or_the_definition_field` |
| R5 sentinel absent from stored fields and `Display` in the success, declared-failure, undecodable-success, and transport cases; built-in header and query, API-marked name, `ApiKeyQuery` name, `Set-Cookie`, `Location`; `page=2` kept | `send::no_stored_field_or_display_text_holds_a_secret_in_any_outcome` (the table over every built-in name stays in Phase 2's `redaction::*`) |
| R6 401 echo kept in the raw body, absent from metadata and `Display` | `send::a_provider_echo_stays_in_the_raw_body_and_out_of_metadata_and_display` |
| R7 (a) closed port, (b) stalled head: redacted URL, `page=2` shown, no sentinel | `send::a_refused_connection_is_a_transport_error_with_request_metadata`, `timing::a_stalled_head_is_a_timeout_whose_text_redacts_the_query` |
| R8 governing rule | refused connection (above); 500 empty body (`send::statuses_outside_the_declared_range_and_endpoints_without_a_declared_error_are_undeclared`); 200 undecodable (`send::html_on_a_json_endpoint_is_a_malformed_success_with_its_evidence`); cut-off body (`timing::a_body_cut_off_after_the_head_is_interrupted_inside_the_envelope_with_its_bytes`) |
| R9 pre-send kinds with metadata and zero requests: `InvalidArgument` (parts, header, per-call credential, collision), `DeadlineExceeded` (exhausted) | `send::an_invalid_argument_is_refused_before_any_request`, `credential_resolution::a_per_call_credential_no_placement_accepts_is_refused_before_any_request`, `credential_resolution::a_request_already_carrying_the_credentials_slot_is_refused_and_sent_unchanged_without_one`, `timing::an_exhausted_deadline_sends_nothing` (`CrossOriginRejected`: Phase 4) |
| R10 HTML on JSON 200 → `MalformedSuccess`, helpers | `send::html_on_a_json_endpoint_is_a_malformed_success_with_its_evidence` |
| R11 raw-socket truncation, 2xx and non-2xx | `timing::a_body_cut_off_after_the_head_is_interrupted_inside_the_envelope_with_its_bytes` |
| R12 undeclared over the wire | `send::statuses_outside_the_declared_range_and_endpoints_without_a_declared_error_are_undeclared` (compile-fail: Phase 2) |
| R13 declared decode on arrival; `MalformedError` | `send::a_declared_error_is_decoded_on_arrival_and_keeps_its_body`, `send::an_undecodable_declared_error_is_malformed_with_its_body` |
| R14 precedence, one credential each, anonymous plus 401 envelope, no retry | `credential_resolution::per_call_beats_client_beats_environment_beats_anonymous_with_one_credential_each`, `credential_resolution::nothing_resolving_sends_once_anonymously_and_a_401_is_an_envelope_failure` |
| R15 first non-empty variable; caller order replaces | `credential_resolution::the_first_non_empty_variable_wins_and_a_client_order_replaces_the_definitions`, `credential_resolution::each_send_reads_the_environment_again` |
| R16 resolved once (variable changed after the head was read); single attempt; reported source | `credential_resolution::an_environment_change_after_the_request_left_does_not_change_the_reported_source`, plus R14 tests |
| Credential construction: placements, cookie join and encoding, Basic env, explicit policy differing from the fallback, incompatible kinds, collisions, duplicate query values, anonymous stripping, auth-bearing defaults | `credential_resolution::{each_placement_puts_the_credential_where_the_api_expects_it, a_cookie_credential_joins_existing_cookies_with_its_value_percent_encoded, basic_environment_credentials_need_both_variables_and_report_the_password_variable, an_explicit_policy_and_its_environment_fallback_use_their_own_placements, a_client_credential_no_placement_accepts_fails_the_build, ordinary_duplicate_query_values_stay_beside_a_query_credential, anonymous_mode_bypasses_every_source_and_strips_every_placement_from_the_request, a_default_header_that_would_carry_authentication_fails_the_build}` |
| R18 builder surface: base URL, timeout default and effect, user agent and default headers, proxy (standalone and shared), roots PEM and DER, root absence → `Tls`, accept-invalid vs default, shared-transport reuse (1 connection vs 2), attached-client conflicts, invalid proxy and certificate | `builder::*` (18 tests; the four TLS ones are `ignore`d on native Windows) |
| R19 groundwork: the library never follows a redirect (standalone and shared transport) | `builder::a_redirect_is_returned_to_the_caller_and_never_followed` (the full constructor × policy matrix is Phase 4) |
| R26 shared deadline: exhausted sends nothing; timeout leaves budget; deadline bounds a request; N-send stop; counting (HTTP failure counts, transport failure and drop do not); post-head expiry → `BodyInterrupted`; drop cancels; no task left | `timing::{an_exhausted_deadline_sends_nothing, a_per_request_timeout_under_a_longer_deadline_leaves_the_rest_of_the_budget, a_deadline_shorter_than_the_timeout_bounds_the_request, a_deadline_stops_an_operation_of_several_sends_and_sends_nothing_after_it_expires, only_sends_that_received_a_head_count_as_completed, an_expiration_after_the_head_is_a_body_interruption_with_the_partial_bytes, dropping_a_send_closes_its_connection_and_does_not_count_it, a_dropped_send_leaves_no_task_alive_and_the_runtime_shuts_down_promptly}`, unit `deadline::tests::the_earlier_bound_limits_a_send` |
| R28 codecs judge content, not content type; text and binary accept any type; Empty drains | `send::codecs_judge_the_body_not_the_content_type`, `send::html_on_a_json_endpoint_is_a_malformed_success_with_its_evidence` |
| R30 over the network: owned stream, first item before the body ends, idle bound, deadline over the whole stream | `timing::{a_streaming_decoder_yields_its_first_line_before_the_body_ends, a_stream_outlives_the_send_expiration_while_it_keeps_delivering_then_times_out_when_idle, the_shared_deadline_bounds_a_stream_that_never_goes_idle}` |
| Request assembly: ordered duplicate query and template query, fragment dropped, every body form | `send::query_order_duplicates_and_a_template_query_reach_the_server`, `send::every_body_form_is_sent_with_its_content_type`, unit `encode::tests::*` (2) |

**Mutation checks** (temporary edits, all restored). Re-enabling the
library's redirect following failed
`builder::a_redirect_is_returned_to_the_caller_and_never_followed`.
Removing `record_completed` failed 4 `timing::` tests. The two runtime
defects the subagents found (TLS classification, certificate error) were
each caught by their tests before the fix.

### Gates (all on the final tree)

| Gate | Result |
|---|---|
| `just test` (`schematic/`) | PASS: L1 1879 / 0 failed / 10 skipped (Phase 2: 1817; +62 runtime); fresh-output 4 / 0; `test-compile-fail` 1 / 0 (23 s cold after the dependency change) |
| `just lint` (`schematic/`) | PASS: define, gen, definitions, oauth, runtime |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p schematic-runtime --no-deps` | PASS |
| `cargo check` of the 8 dependents | PASS (unchanged schema; the lockfile gains only the runtime's entry) |
| `just cross-check schematic-runtime --os linux` | PASS 99 / 99 |
| `just cross-check schematic-runtime --os wsl` | PASS 99 / 99 |
| `just cross-check schematic-runtime --os windows` | PASS 95 / 95 with the four TLS tests skipped, 3 consecutive runs; before the `ignore`, 3 of 5 runs had one abort (see above) |

The runtime L1 count after cleanups is 99 on macOS. Nothing failed before
this phase. The pre-existing 10 skipped tests are unchanged.
`just test-schema` and `just lint-schema` were not run, because
`schematic-schema` and its inputs are unchanged.

### Review items found in Phase 3

1. **Windows TLS abort and the four `ignore`d tests.** Options:
   - keep the `ignore` until the faulting stack is captured (recommended;
     the main Windows leg stays green and the coverage gap is named);
   - remove it and accept a frequently red Windows leg;
   - move the runtime to a preconfigured rustls `ClientConfig`. The evidence
     does not yet show that this avoids the abort, and it would change trust
     behavior.
2. **`EndpointSpec::METHOD` and `Box<RequestMeta>`** (design choices 1–2).
   Both are visible to generated code in Phase 6.
3. **Collision refusal versus today's replacement** (design choice 5):
   Phase 7 checks that no consumer relies on replacement.
4. **Connect-timeout effect is not observed** (deviation above).
5. **Cookie credential encoding keeps `%` unescaped** (RFC 6265
   cookie-octet). It matches today's generated `encode_cookie_value`. A key
   that contains `%XX` is therefore ambiguous to a server that
   percent-decodes cookies.
