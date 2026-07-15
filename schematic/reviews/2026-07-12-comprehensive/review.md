---
created: "2026-07-12T22:33:16-07:00"
agent: claude
yolo: true
---

# Comprehensive Code Review: schematic Package Area

**Date:** 2026-07-12
**Scope:** `schematic-define`, `schematic-definitions`, `schematic-gen`, `schematic-oauth`, `schematic-schema`
**Lines reviewed:** ~105k Rust source (~15k define, ~28k definitions, ~21k gen, ~1.7k oauth, ~41k schema) + ~8k test lines
**Method:** Full-source review of hand-written code; sampled review of generated/definition-data modules; independent verification of every Critical/High finding against source; quality gates run (`cargo clippy`, `cargo nextest`, `cargo fmt --check`, `just lint`).

---

## 1. Executive Summary

The schematic area is a well-architected definition → codegen → runtime pipeline with genuinely strong engineering discipline in its core: the four workspace crates are clippy-clean and fmt-clean, 1653 tests pass in 8s under nextest, public enums are `#[non_exhaustive]`, codegen builds token streams with `quote!`/`syn`/`prettyplease` rather than string concatenation, output is deterministic (`BTreeMap` ordering), file writes are atomic, and doc-comment emission is injection-safe. The WebSocket runtime in `schematic-schema` (correlation, backpressure, capped-jittered reconnect) is correct and meaningfully tested.

Risk is concentrated in three places. First, the **quality gate for `schematic-schema` is currently broken**: the generated `Cargo.toml` template omits `serial_test`, so the crate's test suite does not compile — and the area `justfile` never runs schema tests (or anything for `schematic-oauth`), which is exactly how this shipped. Second, the **OpenAPI import path trusts spec-supplied names**: path/query/field names flow unsanitized into `format_ident!` (generator panic) or into struct fields (`type`, `2fa` → invalid Rust), and the documented `x-schematic` round-trip fails for the common empty-collection case. Third, **`schematic-oauth` has credential-hygiene gaps** (world-readable plaintext token file, `Debug`-derived secret exposure, refresh-token loss against RFC 6749 servers that omit `refresh_token` on refresh).

Overall risk: **medium** — the curated native-definition path is production-ready today; the OpenAPI import path and the OAuth crate are fragile and should be hardened before external use. Biggest strengths: architecture, determinism, test volume on the happy path. Biggest concerns: template-level defects replicated into every generated client, adversarial-input handling in the importer, and credential handling. The good news is structural: most High findings are single-point fixes in an emitter template or one module, not redesigns.

---

## 2. Key Findings

#### [Severity: Critical] `schematic-schema` test suite does not compile — `serial_test` missing from the generated manifest

- **Location:** `schematic/gen/src/cargo_gen.rs:38-40` (`CARGO_TOML_TEMPLATE`), manifesting at `schematic/schema/Cargo.toml:25-26` and `schematic/schema/tests/artificial_analysis_client.rs`
- **Why it matters:** `cargo nextest run --manifest-path schematic/schema/Cargo.toml` and `cargo clippy --all-targets` both fail at compile (11× `E0433: cannot find module or crate serial_test`). No schema test executes on a full run; the ~11 artificial_analysis integration tests are silently dead. The crate's test gate has been broken without anyone noticing because no `just` recipe runs schema tests (see the tooling finding below).
- **Evidence:** `schema/Cargo.toml` `[dev-dependencies]` contains only `wiremock = "0.6"`; the hand-written test file uses `#[serial_test::serial]` 11 times (it mutates `ARTIFICIAL_ANALYSIS_API_KEY`). The manifest is generated ("Do not edit manually") — any hand-added dep is clobbered on `just generate`.
- **Recommendation:** Add `serial_test = "3"` to `CARGO_TOML_TEMPLATE`'s dev-dependencies in `cargo_gen.rs` and regenerate. Do not hand-edit `schema/Cargo.toml`. Then add a schema test step to the area gates (below).
- **Confidence:** high (reproduced by compiler; verified manifest and 14 `serial_test` references directly)

#### [Severity: High] Generator panics on legal OpenAPI specs: `format_ident!` on unsanitized path/query parameter names

- **Location:** `schematic/gen/src/parser.rs:19-36` (`extract_path_params`), `schematic/gen/src/codegen/request_structs/path_params.rs:11`, `query_params.rs:16`, `shared.rs:91`
- **Why it matters:** `extract_path_params` returns the raw substring between `{}` and it is fed directly to `format_ident!`, which panics (`proc_macro2::Ident::new`) on anything that is not a lexically valid Rust identifier. Real-world specs routinely use `{user-id}`, `$top`, `$filter`, `user.name`, `2fa`. `schematic-gen import spec.yaml` aborts with a panic instead of a diagnostic. The import boundary in define stores these names verbatim (`define/src/openapi/import/mappings/parameters.rs:90`).
- **Evidence:** `let field_name = format_ident!("{}", param);` (path_params.rs:11) on the raw `&path[pos..idx]` slice; verified. The WS path already solved this — `ws_codegen/client.rs:38` has a `sanitize_ident` handling non-alphanumerics, leading digits, and empties — the REST path has no equivalent.
- **Recommendation:** Route every identifier through one shared sanitizer (lift `ws_codegen`'s or reuse define's `sanitize_rust_ident` with a snake-case variant) before any `format_ident!`, preserving the wire name via `#[serde(rename)]` / the literal query key. Add import tests for `/x/{user-id}` and a `$top` query param asserting a clean error or a compiling rewrite.
- **Confidence:** high

#### [Severity: High] `x-schematic` extension round-trip fails for empty collections (`skip_serializing_if` without `#[serde(default)]`)

- **Location:** `schematic/define/src/openapi/extensions.rs:55-57, 96-97, 150-151`
- **Why it matters:** `Vec` fields (`headers` on the doc and op extensions, `env_auth` on `AuthExtension`) use `#[serde(skip_serializing_if = "Vec::is_empty")]` with no `#[serde(default)]`. Serde tolerates missing `Option` fields but a missing `Vec` field is a hard deserialize error. The exporter (`export/mod.rs:77-83`) skips empty `headers` — the usual case — so a re-import of schematic's own exported document fails with `missing field 'headers'`. This defeats the module's stated purpose ("round-trip fidelity when importing and re-exporting").
- **Evidence:** Verified all three attribute sites; no `#[serde(default)]` exists anywhere in `extensions.rs`.
- **Recommendation:** Add `#[serde(default)]` beside every `skip_serializing_if` on non-`Option` fields, plus round-trip tests serializing the default/empty extension and deserializing it back.
- **Confidence:** high

#### [Severity: High] Imported struct field names are not sanitized for Rust keywords / leading digits

- **Location:** `schematic/define/src/openapi/import/mappings/schema.rs:239`
- **Why it matters:** Type names go through `sanitize_rust_ident` (keyword-aware), but property names get only `to_snake_case`, which does no keyword/digit/empty handling. A property named `type` yields `rust_field_name == "type" == field_name`, so no `serde(rename)` is recorded and the stored field name is a bare keyword; `2fa`, `self`, and `""` are not expressible even as raw identifiers. Downstream, schematic-gen emits these as struct fields → the generated crate fails to compile. `type` and `self` are extremely common JSON property names.
- **Evidence:** `let rust_field_name = to_snake_case(field_name); let serde_rename = if rust_field_name != *field_name { Some(...) } else { None };` — verified; `to_snake_case` has no keyword table.
- **Recommendation:** Route field names through a keyword/digit/empty-safe snake-case sanitizer; whenever the sanitized name differs from the wire name, always record `serde_rename`.
- **Confidence:** high (the invalid identifier is stored in define; final emission is in gen — mechanism verified at both ends)

#### [Severity: High] `FileTokenStore` writes plaintext OAuth tokens world-readable and non-atomically

- **Location:** `schematic/oauth/src/store.rs:106-116`
- **Why it matters:** `std::fs::write` creates the token file with default mode (0644 under a standard umask on macOS/Linux — readable by every local user); `create_dir_all` yields 0755 parents. Access and refresh tokens are long-lived bearer credentials. The write is also truncate-in-place, so a crash or concurrent reader can observe a truncated/corrupt token file. Windows inherits parent-directory ACLs, which is acceptable only under per-user profile dirs — currently undocumented.
- **Evidence:** `std::fs::write(&self.path, json)` with no permission handling anywhere in the crate; verified by full read of `store.rs`.
- **Recommendation:** On Unix, create via `OpenOptions` with `mode(0o600)` (and `0o700` on the created parent); write temp-file-then-rename (matching the repo's atomic-write convention, and what `schematic-gen`'s own `output/write.rs` already does). On Windows, document reliance on per-user profile ACLs.
- **Confidence:** high

#### [Severity: High] Token refresh discards the stored refresh token when the server omits it

- **Location:** `schematic/oauth/src/manager/mod.rs:167` (`extract_tokens`), consumed at `manager/refresh.rs:45-48`
- **Why it matters:** RFC 6749 §6 allows refresh responses to omit `refresh_token` (the client keeps using the old one) — many providers do exactly this. `extract_tokens` maps the absent field to `None` and the refresh path saves the result verbatim, overwriting the persisted refresh token with `None`. After the first refresh against such a provider, the next expiry returns `AuthenticationRequired` and forces a full re-authorization despite holding a valid refresh token.
- **Evidence:** `refresh_token: response.refresh_token().map(|rt| rt.secret().to_string()),` then `store.save(&new_tokens)?` — verified.
- **Recommendation:** In `refresh_token()`, carry forward `tokens.refresh_token.clone()` when the response has none. Add a wiremock test for a refresh response without `refresh_token`.
- **Confidence:** high

#### [Severity: High] Generated REST clients do not URL-encode path parameters

- **Location:** Template: `schematic/gen/src/codegen/request_structs/shared.rs:84-96` (`generate_path_format`); generated instances e.g. `schematic/schema/src/gitlab/requests.rs:1173`, `schematic/schema/src/gitea/requests.rs:206`, `schematic/schema/src/openai.rs:117`
- **Why it matters:** Path values are interpolated raw (`format!("/projects/{}/repository/tags/{}", self.id, self.tag_name)`), while query *values* are encoded (`urlencoding::encode(v)`, shared.rs:134) and the WS generator encodes path params (verified by `tests/ws_runtime.rs:65`). GitLab tag names with `/`, Gitea file paths, or any value containing `? # %` produce malformed URLs, and caller-supplied values like `../x` or `x?a=1` reshape the request path/query — a request-forgery vector in every generated client.
- **Evidence:** Verified the template's `quote! { let #mut_tok path = format!(#format_str, #(#format_args),*); }` with no encoding of the args, and raw interpolation in generated files.
- **Recommendation:** Percent-encode each path segment in the emitter (with a documented opt-out for intentionally multi-segment params like HuggingFace `repo_id`); regenerate. Also encode query *keys* (currently `format!("{}={}", k, urlencoding::encode(v))`) — low risk today since keys are static, but free to fix in the same pass.
- **Confidence:** high

#### [Severity: High] `cleanup_stale_files` deletes arbitrary directories and `.rs` files under `--output`

- **Location:** `schematic/gen/src/output/mod.rs:396-415`
- **Why it matters:** Any top-level `.rs` file or *any directory* in the output dir whose name is not in the current generation set is removed — directories via `fs::remove_dir_all`, with the `Result` discarded (`let _ =`). `--output` is a free-form CLI path; pointing it at a populated `src/` (with `bin/`, `tests/`, or a hand-written module) silently destroys that content. Failures are also invisible.
- **Evidence:** Verified: `if path.is_dir() && ... && !expected.contains(name) { let _ = fs::remove_dir_all(&path); }`.
- **Recommendation:** Only delete artifacts the generator provably owns — track a manifest of previously generated files or require the `// This code was automatically generated` header before deletion; never `remove_dir_all` a directory not created by this tool; surface removal errors.
- **Confidence:** high

#### [Severity: High] OAuth secrets exposed through derived `Debug`

- **Location:** `schematic/oauth/src/types.rs:9, 27, 53`
- **Why it matters:** `StoredTokens` (`access_token`, `refresh_token`), `OAuth2RuntimeConfig` (`client_secret`), and `AuthorizationSession` (`pkce_verifier`) all derive `Debug` over plain `String` fields. Any `{:?}` log, `dbg!`, or `.unwrap()` panic message prints live credentials. The upstream `oauth2` crate deliberately wraps these in redacting types; this crate unwraps them and re-adds derived `Debug`. Notably, `schematic-define` already has `SensitiveString` with a redacting `Debug` — the sibling crate just doesn't use it. (Related low: `OAuthError::StateMismatch` puts both CSRF state values into its `Display` output.)
- **Evidence:** `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct StoredTokens { pub access_token: String, ... }` — verified.
- **Recommendation:** Hand-write `Debug` impls rendering secrets as `[redacted]`, or adopt `SensitiveString`/`secrecy::SecretString` for token and secret fields (serde stays on the storage type via explicit field access in `FileTokenStore`).
- **Confidence:** high

#### [Severity: Medium] Concurrent `get_valid_token` callers race to refresh (no single-flight)

- **Location:** `schematic/oauth/src/manager/refresh.rs:16-26, 47-48`
- **Why it matters:** The read lock is dropped after `load()`; `refresh_token()` reacquires a write lock only around `save()`. N concurrent callers that all observe an expired token all fire refresh requests. With refresh-token rotation (single-use refresh tokens), the first rotation invalidates the token the others are still presenting — spurious failures — and last-save-wins can persist an already-revoked pair. Without rotation it is a thundering herd against the token endpoint. Note the `RwLock<Box<dyn TokenStore>>` itself buys nothing here: `TokenStore` is already `Send + Sync` with `&self` methods, and the lock is never held across the check→refresh→save sequence it would need to guard.
- **Evidence:** `let store = self.store.read().await; let tokens = store.load()?; drop(store);` then an unguarded async refresh — verified.
- **Recommendation:** Hold a `tokio::Mutex` (or the write half) across load→refresh→save and re-check `is_expired()` after acquiring it, so only the first caller refreshes; drop the now-pointless `RwLock` wrapper or repurpose it as that guard.
- **Confidence:** high (mechanism verified; impact depends on concurrent client usage)

#### [Severity: Medium] Unconditional `#[derive(Default)]` on generated structs breaks compilation for required enum-typed fields

- **Location:** `schematic/gen/src/model_gen.rs:97` (structs) vs `:152-159` (enums derive no `Default`)
- **Why it matters:** Every generated struct derives `Default`, but generated enums do not. A struct with a required field of a generated enum type derives `Default` over a non-`Default` field, which rustc rejects — and the `syn` validation gate is syntax-only, so this passes generation and fails only when `schematic-schema` compiles. The generated doc examples (`..Default::default()`) actively depend on the derive.
- **Evidence:** `// For simplicity, derive Default for all structs ... let derives = quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] };` — verified.
- **Recommendation:** Derive `Default` conditionally (all required fields `Default`), or emit `#[default]` on a designated enum variant, or emit a manual impl. Add a generation test that `cargo check`s output containing a required enum-typed field.
- **Confidence:** high from code; medium that current curated inputs trigger it

#### [Severity: Medium] `validate` reports success on definitions that cannot generate; `syn` gate oversold as a safety guarantee

- **Location:** `schematic/gen/src/validation.rs:104-149`, `schematic/gen/src/pipeline.rs:144-148`, doc claim at `schematic/gen/src/output/mod.rs:24-28`, gate at `output/format.rs:23-26`
- **Why it matters:** `validate_api` checks only request-suffix alphanumerics and body/wrapper name collisions — not identifier validity of endpoint IDs, path/query/field names, i.e. exactly the inputs that later panic (`format_ident!`) or fail downstream compilation (`Default`, keywords). Meanwhile the module docs state "All generated code is validated with `syn` before writing" as a safety guarantee, but `syn::parse2::<syn::File>` validates grammar only. Users get "All validation checks passed" on inputs that cannot be generated.
- **Evidence:** Verified both the narrow check set and the doc claim.
- **Recommendation:** Add an identifier-validity pass to `validate_api` reusing the shared sanitizer; soften the "Safety Guarantee" wording, or add an optional `cargo check` gate for the untrusted import path.
- **Confidence:** high

#### [Severity: Medium] `EndpointSpec::Response` does not steer callers to the right decoder

- **Location:** `schematic/schema/src/gitea/requests.rs:211-214` (`type Response = String`), generic `request<T>` at `schematic/schema/src/gitea/client.rs:270-295`
- **Why it matters:** Text endpoints are correctly served by generated `request_text()` and per-endpoint convenience methods (e.g. `get_repository_content_raw()` routes to `request_text` — verified), but the generic `request<T: DeserializeOwned>()` accepts *any* request convertible into the request enum and always decodes with `response.json::<T>()`. A caller who follows the `EndpointSpec` trait (`Response = String`) and calls `request::<String>(...)` on a raw-content endpoint gets a confusing `SchematicError::Json` on perfectly good responses. The type system holds the information (the associated type) but never uses it to select the decode path.
- **Evidence:** Verified `request_text` exists and the convenience method uses it; verified `request<T>`'s bound is unconstrained by `EndpointSpec`.
- **Recommendation:** Tie decoding to the endpoint type: e.g. a sealed `ResponseKind` associated const/type on `EndpointSpec` that the single `request()` dispatches on, or constrain `request<T>` to JSON-response endpoint markers so text/binary endpoints don't typecheck through the JSON path.
- **Confidence:** high

#### [Severity: Medium] Area `justfile` never tests `schematic-oauth` or `schematic-schema`

- **Location:** `schematic/justfile` (`lint`, `test`, `sanity`, `coverage`, `doctest`, `build` recipe lists)
- **Why it matters:** Every quality recipe iterates only define/gen/definitions. `schematic-oauth` — the credential-handling crate — is absent from *all* recipes; `schematic-schema` gets only `cargo check` via `full`/`install`. This is the direct enabler of the Critical finding: `just all` can be green while the schema test suite doesn't compile. Clippy for oauth happens only because it's a workspace member swept by root-level runs.
- **Evidence:** Gate run confirmed `just lint` passes while schema clippy/tests fail; recipe lists verified.
- **Recommendation:** Add oauth to every recipe list; add a `test-schema` recipe (`cargo nextest run --manifest-path schematic/schema/Cargo.toml`) and wire it into `all`/`full`.
- **Confidence:** high

#### [Severity: Medium] No default timeout on generated `reqwest` clients

- **Location:** Generated constructors, e.g. `schematic/schema/src/openai.rs:278,312`, `anthropic.rs:352,395`
- **Why it matters:** `reqwest::Client::new()` with no timeout means a hung server blocks the returned future indefinitely; the `SchematicError::Http` docs even mention "timeout" as a covered failure though none is configured. The WS runtime, by contrast, has per-request timeouts.
- **Recommendation:** Emit `Client::builder().timeout(...)` with a sensible default and keep `with_client()` as the override; optionally expose `timeout()` on the variant builder.
- **Confidence:** high

#### [Severity: Medium] Gitea auth requires the `token ` scheme prefix baked into the env var

- **Location:** `schematic/schema/src/gitea/client.rs:109-121`; documented at `schematic/definitions/src/gitea/mod.rs:22-26`
- **Why it matters:** The `Authorization` header is set to the raw `GITEA_TOKEN` value, so users must set `GITEA_TOKEN="token <pat>"`. Setting a bare PAT — the natural thing — sends `Authorization: <pat>` and yields an unexplained 401. No other Git-host client here has this shape.
- **Recommendation:** Model the scheme prefix in the auth strategy (e.g. a `value_prefix` on `ApiKey`) so users set only the PAT; at minimum, error clearly when the value lacks the expected prefix.
- **Confidence:** high

#### [Severity: Medium] Sanitization collisions are not deconflicted (schema names and enum variants)

- **Location:** `schematic/define/src/openapi/import/mappings/schema.rs:30-54` (schema names), `:286-299` (string-enum variants)
- **Why it matters:** (a) `name_mapping` is keyed by *sanitized* name; two components that sanitize identically (`user-name`, `user_name` → `UserName`) collapse — loop 2 assigns both the suffixed name, the unsuffixed name is lost, and unrelated `TypeRef::Named("UserName")` refs get rewritten to the wrong target. (b) Enum values differing only in case/separators (`active`/`Active`) sanitize to duplicate variant names — invalid Rust.
- **Recommendation:** Key the mapping by original name and deconflict final names in one pass; deconflict variant names within each enum, preserving wire values in `EnumVariant.value`.
- **Confidence:** medium-high (mechanism verified; requires colliding spec names to fire)

#### [Severity: Medium] `ApiKeyParam` auth is silently dropped by `effective_auth_policy`

- **Location:** `schematic/define/src/auth.rs:266`
- **Why it matters:** `AuthStrategy::ApiKeyParam { .. } => Self::default()` yields an empty `AuthPolicy` — no explicit method, no env fallback — yet the importer produces `ApiKeyParam` for `in: query`/`in: cookie` security schemes (`mappings/mod.rs:126-133`). An imported query/cookie-auth API silently loses its credential wiring with no diagnostic.
- **Evidence:** Verified the match arm.
- **Recommendation:** Represent param-based keys in `AuthPolicy` (or emit a diagnostic on this conversion); at minimum document the limitation on `effective_auth_policy`.
- **Confidence:** high

#### [Severity: Medium] Registry lookup docs badly stale; no cross-list invariant test in `schematic-definitions`

- **Location:** `schematic/definitions/src/registry/lookup.rs:8-22` (docs) vs `:37-56` (code); parallel lists at `lib.rs:224`, `lookup.rs:37`, `lookup.rs:83`, `registry/mod.rs:743`
- **Why it matters:** The doc claims only OpenAI and Samsung Smart TV have registries and lists two supported keys; the `match` dispatches ~18 APIs. Per repo convention, code wins — rewrite the doc. Structurally, four hand-maintained parallel lists (`apis_by_module()`, `get_registry`, `registry_key_for`, the test table) have no sweep test asserting mutual consistency, so a new API added to one list but not another ships silently.
- **Recommendation:** Rewrite the `get_registry` docs; add one integration test driving all invariants off `apis_by_module()` — every API resolves through `registry_key_for` → `get_registry`, passes `validate_completeness`, has unique endpoint IDs and a parseable `base_url`.
- **Confidence:** high

#### [Severity: Medium] Import path (`generate_and_write_standalone`) skips conflict/stale cleanup

- **Location:** `schematic/gen/src/output/mod.rs:343-388` vs `:218, 252-253`
- **Why it matters:** The native `all` path calls `remove_conflicting_paths` + `cleanup_stale_files`; the `import`/`import-asyncapi` path calls neither. Re-importing a module that previously existed in directory form leaves `foo.rs` alongside `foo/` — a module ambiguity that fails compilation. (Any unification must also fix the destructive-cleanup finding above, not spread it.)
- **Recommendation:** Extract shared, ownership-aware cleanup used by both entry points.
- **Confidence:** medium

#### [Severity: Medium] Silent-degradation paper cuts in credential/config plumbing

- **Location:** (a) `schematic/oauth/src/manager/authorization_code.rs:36`; (b) `schematic/schema/src/elevenlabs_ws.rs:259-266` (and sibling WS modules); (c) `schematic/define/src/openapi/extensions.rs:60-62, 105-107, 135-137, 159-161`
- **Why it matters:** Three variants of the same disease. (a) `PkceRequirement::NotUsed | _` — verified that `PkceRequirement` has exactly three variants and is *not* `#[non_exhaustive]`, so the wildcard is redundant today and silently maps any future variant to "no PKCE": a security downgrade with no compiler signal (cross-crate, so no build break either). (b) WS `dial` drops headers whose name/value fail parsing via `if let (Ok, Ok)` — a malformed auth header vanishes and surfaces later as an opaque auth rejection. (c) `From<…Extension> for serde_json::Value` uses `to_value(ext).unwrap_or(Value::Null)` — a serialize bug would silently strip the whole extension.
- **Recommendation:** (a) Drop `| _` so new variants force a decision. (b) Map header parse failures to a `WsError` variant. (c) Use `expect("x-schematic extension is always serializable")` to fail loudly.
- **Confidence:** high

#### [Severity: Low] Clippy debt in generated output: 18× `clippy::option_zip`

- **Location:** Emitter pattern reproduced across providers, e.g. `schematic/schema/src/anthropic.rs:564`, `artificial_analysis/mod.rs:256`
- **Why it matters:** The generated crate is not clippy-clean and has no lint gate, so real warnings will drown in template noise. Fix is one emitter change (`Option::zip`), then regenerate — not `clippy --fix` on output.
- **Confidence:** high

#### [Severity: Low] `write_atomic` overstates durability; fixed temp name races concurrent runs

- **Location:** `schematic/gen/src/output/write.rs:8-14`
- **Why it matters:** Docs claim crash-safety, but there is no fsync of file or parent dir (rename is atomic, not durable), and the temp path is a fixed `path.with_extension("tmp")`, so two concurrent invocations targeting the same output clobber each other's temp file.
- **Recommendation:** Soften the doc or add fsync; use a unique temp suffix.
- **Confidence:** high

#### [Severity: Low] `SensitiveString` docs overstate its guarantees

- **Location:** `schematic/define/src/headers/sensitive.rs:3-8`
- **Why it matters:** Docs claim "secure handling" and that omitting `PartialEq` "prevents timing attacks." It actually provides exactly two things — `Debug` redaction and no `Display` (both genuinely good) — with no zeroize-on-drop and free exposure via `as_str()`/`into_inner()`. Omitting `Eq` prevents comparison; constant-time comparison is what addresses timing.
- **Recommendation:** State precisely what is guaranteed; consider `zeroize` if values persist.
- **Confidence:** high

---

## 3. Rust-Idiomaticity Notes

- **One sanitizer, one snake_case.** There are six divergent `to_snake_case` implementations in schematic-gen alone (`output/assemble/helpers.rs:8`, `codegen/module_docs.rs:253`, `codegen/client/helpers.rs:6`, `codegen/request_structs/shared.rs:12`, `ws_codegen/client.rs:805`, `asyncapi_import.rs:745`) differing exactly where it matters (non-alphanumeric handling) — this fragmentation is the root of the identifier-panic finding. Consolidate into one utility shared with define's `naming.rs`.
- **Parallel hand-maintained registries.** `pipeline.rs` keeps three copies of the API list (`AVAILABLE_APIS` string const, `resolve_api` match, `resolve_all_apis` vec); definitions keeps four. Deriving all of them from `schematic_definitions::apis_by_module()` removes the drift class entirely.
- **`RwLock<Box<dyn TokenStore>>` models the wrong thing** — the lock guards the *box*, never the load→refresh→save critical section. Either it becomes the single-flight guard or it should be deleted (`Box<dyn TokenStore>` alone suffices; the trait is `Send + Sync` with `&self` methods).
- **Sync trait in async context.** `TokenStore` does blocking `std::fs` I/O and `std::sync::Mutex` locking invoked from async methods. Token ops are rare so impact is bounded, but an async trait (or `spawn_blocking` in `FileTokenStore`) would be honest about the boundary.
- **Generated API surface carries unused capabilities:** request structs derive `Deserialize` (only ever serialized outbound) and WS clients expose a `pub headers` field that tests mutate directly — a builder setter would preserve encapsulation. `SchematicError::MissingCredential` appears to be a dead variant (resolve paths return `AuthenticationRequired`); confirm and remove.
- **`build_http_client()` per OAuth operation** creates a fresh `reqwest::Client` (new connection pool) per token call; a lazily-initialized shared client is cheap and idiomatic.
- **Praiseworthy:** `#[doc = #text]` string-literal doc emission (injection-safe); `#[non_exhaustive]` discipline in define; boxing `openapiv3::OpenAPI` inside `OpenApiSource` to keep the enum small; diagnostics-not-panics throughout the importer; deliberate `BTreeMap` ordering for deterministic Postman output.

---

## 4. Testing Gaps

Existing coverage is strong on happy paths (1653 green tests, golden/drift tests for Postman/OpenAPI artifacts, a genuinely good WS runtime suite covering encoding, timeout, disconnect-drains-pending, reconnect). The gaps cluster exactly where the findings live:

**schematic-gen / define importer (adversarial inputs)**
- Import a spec with path param `{user-id}` / `{2fa}` and query params `$top`, `user.name` → assert clean diagnostic, not panic.
- Properties named `type`, `self`, `async`, `2fa`, `""` → valid field ident + `serde(rename)` round-trip.
- Component schemas `user-name` + `user_name` → two distinct model names, refs point at the right ones.
- Enum values `active` + `Active` → distinct variants.
- `x-schematic` round-trips of `SchematicDocExtension::default()`, op extension with empty `headers`, `AuthExtension` with empty `env_auth` (all currently fail deserialization).
- A `StructDef` with a required enum-typed field → generated output passes `cargo check` (catches the `Default` derive break).
- `cleanup_stale_files`: an unrelated subdirectory (`bin/`) and a hand-written `.rs` in the output dir survive a run (currently they are deleted — the test would document the hazard and gate the fix).
- Re-import over a pre-existing directory-form module → no `foo.rs` + `foo/` ambiguity.
- Path templates with stray/unmatched braces and duplicate params (`/{id}/{id}`).

**schematic-schema (runtime, once the test build is fixed)**
- A `text/plain` endpoint exercised through the *generic* `request<T>` path (documents the decoder-misuse hazard) and through `request_text` (happy path).
- Path param containing `/` or space (e.g. GitLab `tag_name = "release/1.0"`) → assert percent-encoding after the emitter fix.
- Slow/hung mock server → assert a timeout fires once a default is configured.
- Gitea: bare PAT vs `token `-prefixed value behavior.

**schematic-oauth**
- `wiremock` is a dev-dependency and completely unused: there are zero mocked happy-path tests for `exchange_code`, `acquire_client_credentials_token`, `refresh_token`, `revoke_token`.
- Refresh response omitting `refresh_token` preserves the stored one (guards the High finding).
- `FileTokenStore` creates the file 0600 / parent 0700 on Unix (post-fix).
- Concurrent `get_valid_token` performs exactly one refresh (post-single-flight).
- `format!("{:?}", StoredTokens {...})` contains no token material (post-redaction).

**schematic-definitions**
- The all-APIs sweep described above (registry/list consistency, `validate_completeness`, endpoint-ID uniqueness, `base_url` parses).

---

## 5. Unsafe Code Review

**No `unsafe` in production code across all five crates.** The only `unsafe` blocks are in test code (`schematic/define/src/headers/builder/tests.rs`, ~20 sites), wrapping `std::env::set_var`/`remove_var` — required since Rust 2024 marked process-environment mutation unsafe due to thread-safety on Unix. These tests run under `serial_test`-style isolation assumptions; the invariant (no concurrent env access) is upheld by test serialization, which is the standard pattern in this repo. The unsafe regions are minimal (single env call each). No unsoundness risk identified.

---

## 6. Prioritized Next Steps

1. **Unbreak the schema quality gate** — add `serial_test = "3"` to `CARGO_TOML_TEMPLATE` dev-deps in `gen/src/cargo_gen.rs`, regenerate, and extend `schematic/justfile` so `lint`/`test`/`all` cover `schematic-oauth` and run schema tests. Everything else relies on this gate existing.
2. **Centralize identifier sanitization** — one keyword/digit/empty-safe sanitizer shared by define's importer (field names) and gen's REST path (all `format_ident!` call sites), with `serde(rename)` preservation; wire it into `validate_api`; add the adversarial-name test set.
3. **Fix the `x-schematic` round-trip** — `#[serde(default)]` on the three `Vec` fields + empty-case round-trip tests.
4. **OAuth credential hygiene pass** — 0600/0700 + atomic token writes, preserve the refresh token on refresh, redact `Debug` on the three secret-bearing types, single-flight the refresh path; add the wiremock suite that regression-guards all four.
5. **Regenerate-once emitter fixes** — percent-encode path segments (and query keys), default `reqwest` timeout, conditional `Default` derive, `Option::zip` clippy pattern. One emitter pass fixes every provider simultaneously.
6. **Make cleanup ownership-aware** — restrict `cleanup_stale_files` to generator-owned artifacts, surface deletion errors, and share the logic with the standalone import path.
7. **Definitions invariant sweep + doc drift** — the all-APIs integration test and the `get_registry` doc rewrite (code is authoritative; docs currently claim 2 of ~18 supported APIs).
