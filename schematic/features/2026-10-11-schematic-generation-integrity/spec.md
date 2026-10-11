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
depends_on: []
refresh_required: false
related: []
---

# Schematic generation integrity

## Problem

`schematic-gen` turns API definitions into the `schematic-schema` client crate, OpenAPI documents, and Postman collections. Today the generator can destroy work silently, drop declared request data silently, and publish an invalid document. Each item below is verified against the parent review or the code unless marked **unverified**.

1. **Single-API generation deletes the other clients.** `generate --api <one>` against the real output directory builds its expected-file set from only the requested API and removes every generator-owned file not in it. The parent measured 24 files and directories shrinking to 10, and `lib.rs` is rewritten consistently, so the result still compiles and nothing signals the loss (`schematic/gen/src/output/mod.rs`, `collect_expected_files` and `cleanup_stale_files`). `schematic/gen/README.md` documents the invocation as normal usage and the `generate-one` recipe in `schematic/justfile` runs it.
2. **Declared request data is dropped without a diagnostic.**
    - Text and binary request bodies (`ApiRequest::Text`, `ApiRequest::Binary`) produce no body field, and `into_parts` returns `RequestBody::Empty` for them (`gen/src/codegen/request_structs/into_parts.rs`). The generated runtime enum `RequestBody` already has a `Raw { content_type, bytes }` variant (`schematic/schema/src/shared.rs`); whether the generated client sends it with its `Content-Type` is **unverified**.
    - `AuthStrategy::ApiKeyParam` (key in query or cookie) is never injected. Header resolution is header-only, `headers_satisfy_fallback` returns `false` for it, so such an API cannot authenticate and reports a misleading "authentication required" error (`gen/src/codegen/client/helpers.rs`). The legacy env mapping also drops it (`schematic/define/src/types.rs`, `legacy_env_mapping_for`).
    - `EndpointParams.header` and `EndpointParams.cookie` have no readers in the generator.
    - Live counts verified by the parent on 2026-10-11: text bodies 0, binary bodies 0, `ApiKeyParam` 0, header or cookie parameters 0, multipart about 29. The July figure "six endpoints POST empty bodies" is a different measure and is not reconciled with "about 29". No shipped definition uses the zero-count features, so tests need purpose-built fixture definitions.
3. **Query parameters are formatted into the path string.** `generate_path_format` (`gen/src/codegen/request_structs/shared.rs`) builds one `path` string containing the query, and `RequestParts` is `(method, path, body, headers)` (`schematic/schema/src/shared.rs`). Later work must redact secret query values from URLs and error text, which needs the query pairs kept separate from the path.
4. **`openapi/ollama.json` has duplicate `operationId` values** (`Embeddings` and `ListModels`, each on two paths), which OpenAPI 3.0 forbids. The grouped merge (`gen/src/export/openapi.rs`) deduplicates on `(path, method)` and never checks ids. Verified by a duplicate scan of the committed file on 2026-10-11.
5. **Validation is incomplete and not per output.** `validate_api` (`gen/src/validation.rs`) checks identifiers and body-type collisions but not duplicate endpoint ids or duplicate `(method, path)`. Nothing validates the whole batch for each output before writing.
6. **Nothing on a normal test run compiles freshly generated code.** `generated_code_compiles` and the clippy test in `gen/tests/l1/e2e_generation.rs` are `#[ignore]`d. `just lint-schema` compiles the committed crate, which is not the same as compiling what the generator emits now.
7. **A stale git-tracked duplicate output tree** exists at `schematic/gen/schematic/` (five tracked files under `schema/`, residue of a wrong-cwd run). It is easy to mistake for real output.

## Goal and non-goals

**Goal.** After this spec, regenerating any subset of APIs is safe and honest; every declared request-side input the supported APIs can send reaches the generated Rust client; the generator never publishes a duplicate id or an invalid batch; and a normal test run proves freshly generated code compiles offline in isolation.

**Non-goals (owned by other specs or later work):**

- The runtime package, the response envelope, typed send, credential precedence, redirects, deadline, and redaction implementation (spec 2). This spec only separates query parameters so redaction can be built later.
- Pagination (spec 3), definition API and builders (spec 4), response model v2, selectors, and GraphQL (spec 5), WebSocket (spec 6), fidelity and polish including feature flags, the module-path resolver unification, and the generated-docs honesty pass (spec 7).
- Query-parameter `required`, style, `explode`, and enum fidelity (spec 4).
- Any edit to the roughly 519 endpoint literals in the definitions (openai alone has 265), except the one narrow migration in R33 (the Gitea `?recursive=true` endpoint), or to a dependent crate's source (sniff, biscuit-speaks, model-citizen, the homelab crates). This spec is otherwise generator-only.

## Background: decisions this spec implements

Provenance: parent review `schematic/reviews/2026-07-31-macro-review/spec.md`, section 15. Restated in full so the implementer does not need to open it.

- **D9. Partial generation (confirmed 2026-10-10).** Regenerating one API regenerates its whole resolved module group, keeps every other module, rebuilds shared files and exports from the retained inventory, deletes only stale files it can prove it owns, and fails before writing anything if the inventory cannot be established safely. Example: selecting `OllamaNative` also regenerates `OllamaOpenAI` because they share a module, and keeps the other 16 APIs. Full generation may remove obsolete owned modules; unowned files stay protected. Atomic writes protect individual files, not a multi-file update, so failures must be reported without claiming the whole crate was updated. An incompatible shared-runtime change requires full generation or an actionable error before any mutation; preserving files while removing their exports does not satisfy the contract. Rejected: refusing single-API generation unless the output is isolated; removing the single-API option.
- **D10. Request-side feature support (confirmed 2026-10-10).** Implement, rather than reject, every input type the supported APIs can send: text and binary request bodies with content type, API key in query or cookie (`ApiKeyParam`), and header and cookie parameters, each tested against local mock servers. Backstop kept: per-output validation. The whole batch for each output is validated before writing; an error names the API, endpoint, feature, and output; one output's gap never blocks another output. Invalid Rust type syntax is an error; valid generic types and qualified paths (`Vec<Model>`, `serde_json::Value`) stay accepted. Wire parameter names are preserved while generated Rust identifiers are sanitized and checked for collisions. Repeated base and path segments alone are not proof of a bad URL; validate the declared composition rule, do not reject a legitimate `/api/api` path by substring.
- **D33. Landing (confirmed 2026-10-11).** The D10 features are generator-only work in landing 1, with no definition edits.
- **D49 item 1. Isolated compile check (confirmed 2026-10-11).** Freshly generated code is compiled in an isolated temporary crate with no live provider requests and no change to the checkout. It guards every later generator change and runs on a normal test run; compile-only checks are L1 (use the `slow_` prefix when the testing guide requires), not L2 merely because they are slow. Removing `#[ignore]` is an implementation choice, not the success criterion.
- **D49 item 6.** The "six endpoints" and "about 29 multipart" figures are unreconciled; the live count is established when D33 is implemented.
- **D23 (prerequisite only).** Network-error text will keep the URL with secret query values masked; that needs the generator to keep query parameters out of the path string (S6 / P7). This spec delivers the split; the masking belongs to spec 2.
- **D36 (forward hook only).** In spec 5, two endpoints on the same method and path will be told apart by a selector, making uniqueness `(method, path, selector)`. This spec's duplicate check must be shaped so the selector can be added, and must not implement a selector.
- **D40 (relevant fact).** An `ApiKeyParam`'s name will be a secret automatically; the generated client must therefore never print the key value.
- **D50 spec 1 scope.** Generation integrity: D9, per-output validation, D49 compile check, duplicate `operationId` fix, D33, query-parameter split, and deletion of the stale tree (a pre-authorization the author must confirm, item 2). Generator only apart from the R33 migration; no dependent crate changes.
- **D20 (whole-work rule).** Every defect-ledger row is closed or recorded as an accepted gap. This spec closes rows 1 (text and binary bodies), 2 (`--api <one>` deletion), 5 (`ApiKeyParam`), 6 (duplicate `operationId`), 16 (`#[ignore]`d compile test), and 18 (stale tree).

## Requirements

Each requirement is testable. "L1" means `just test` in `schematic/`; "L2" means `just test-l2`, which must never take window focus. Nothing here needs L2 except where stated; none does by default.

### Partial generation (D9)

**R1. Whole module group.** Generating a subset of APIs regenerates the entire resolved module group of every selected API. Selecting `OllamaNative` alone regenerates `OllamaOpenAI` too (they share a module; **unverified** that the current definitions share one module, the parent asserts it) and leaves the other 16 modules byte-identical.
Validated by: L1 integration test in `schematic-gen` that generates all APIs into a temp directory, copies it, runs a one-API generation into the copy, and compares file contents.

**R2. Shared files rebuilt from the retained inventory.** After a partial run `lib.rs`, `prelude.rs`, and every other generated shared file and export list name all 18 APIs (or, more generally, all retained plus regenerated APIs), and the crate compiles.
Validated by: the R1 test asserting the export lists, plus the R15 compile check on the resulting directory.

**R3. Deletion only of provably owned stale files.** Within the regenerated groups, a stale file the writer provably owns (the existing generated-header and owned-directory proofs) is deleted; an unowned file in the same directory survives; nothing outside the regenerated groups is deleted by a partial run. A full run may remove obsolete owned modules (today's behavior).
Validated by: L1 test in `schematic-gen/src/output/mod.rs` tests or `tests/l1` with a planted owned stale file, a planted hand-written file, and a hand-written `bin/` directory.

**R4. Unsafe inventory fails before any write.** If the existing output's inventory cannot be established (for example a corrupt or hand-edited `lib.rs`), a partial run returns an actionable error naming the cause and telling the user to use full generation or an isolated output directory, and the output directory is byte-identical afterward.
Validated by: L1 test that snapshots every file's bytes and modification-independent hash before and after.

**R5. Incompatible shared-runtime change.** If a partial run would change a shared file (for example `shared.rs`) in a way that retained, unregenerated modules may not match, the run fails before any write with an error that requires full generation. The detection rule is a planning default (see Planning-time details, default 4).
Validated by: L1 test that alters the stored `shared.rs` and asserts the error and an untouched directory.

**R6. Honest failure reporting.** If a write fails midway, the returned error lists the files already written and the files not written, the CLI exits non-zero, and no message claims the crate was updated. All validation, code assembly, syntax check, and formatting finish before the first write so a failure mid-run can only be an I/O failure.
Validated by: L1 test that injects a write failure (read-only target file or directory on Unix; an equivalent that works on Windows, such as a directory occupying a target filename) and asserts the error content. The injection must be portable to macOS, Linux, Windows, and WSL2 (see `os` skill).

**R7. OpenAPI and Postman follow the same group rule.** A partial run rewrites OpenAPI and Postman artifacts only for the regenerated groups and leaves the rest untouched. (Default 22.)
Validated by: L1 test over `--openapi-out` and `--postman-out` temp directories.

### Per-output validation (backstop of D10)

**R8. Whole batch, per output, before writing.** For each selected output (Rust client, OpenAPI, Postman) the generator validates the entire selected batch before writing that output. If any output has a blocking error, no file of that output is written.
Validated by: L1 test with a batch of one valid and one invalid API asserting zero files written for the failing output.

**R9. Diagnostics name four things.** Every validation diagnostic names the API, the endpoint, the feature, and the output it concerns (for example `api=Foo endpoint=UploadFile feature=binary-request-body output=openapi`). Format and machine readability are planning defaults (default 1, 2).
**This spec owns the validation-diagnostic format and its severities (`error` blocks the output, `warning` is reported and does not block) and exposes them as a public, reusable surface (the diagnostic type, a severity enum, and the per-output validator entry points) so that spec 6 (WebSocket) maps its validation onto them rather than inventing a second format.** Each diagnostic carries its severity in the rendered line and in the serialized form.
Validated by: L1 unit tests on the diagnostic type, the severity, and the rendered text; a test that a downstream caller outside `validation.rs` can construct and emit both severities.

**R10. Outputs are independent.** A feature one output lacks fails validation for that output only; the others still generate. A Rust-client gap never blocks OpenAPI or Postman, and the reverse. After R21 to R27 land, no feature in D10's list is a Rust-client gap, so the test uses a deliberately unsupported fixture feature (default 3) to prove the mechanism.
Validated by: L1 test with a test-only unsupported-feature marker asserting that Rust generation reports the diagnostic while OpenAPI and Postman still write.

**R11. Type syntax and identifiers.** Invalid Rust type syntax in a request or response `type_name` or `module_path` is a validation error; `Vec<Model>`, `serde_json::Value`, and qualified paths stay accepted. Wire parameter names are preserved while generated identifiers are sanitized and checked for collisions, including header and cookie parameter names added by R25 and R26. **Never reject by substring:** a repeated base or path segment is not itself an error (a legitimate `/api/api` stays accepted). Only an exact duplicated segment produced by the declared base-plus-path composition rule (for example a base ending in `/v1` joined to a path beginning `/v1`) may be reported, and then as a `warning`, never an `error`. Add checks only where a gap is found; record each in the implementation log.
Validated by: L1 tests in `gen/src/validation.rs` (accept and reject tables, plus a `/api/api` path accepted).

**R12. Duplicate endpoint ids.** Two endpoints with the same id in one API fail validation; so do two endpoints whose ids collide inside one generated module group (default 6).
Validated by: L1 unit tests in `gen/src/validation.rs`.

**R13. Duplicate `(method, path)` with a selector hook.** Two endpoints in one API with the same HTTP method and the same path (compared after normalizing path-parameter names) fail validation. The uniqueness key is a function of `(method, path, selector)` where `selector` is always `None` today; no selector field or syntax is added (D36 belongs to spec 5).
Validated by: L1 unit tests, including a test that the key function accepts a `None` selector and is the single place a later spec extends.

### Duplicate `operationId` (ledger row 6)

**R14. Unique ids in every exported document.** The grouped OpenAPI merge produces a document in which every `operationId` is unique. The first occurrence keeps its id; a later operation with a clashing id and a different `(path, method)` is renamed by the default rule (default 7), deterministically. After merging, a final scan fails the export if any duplicate remains. Per-API (ungrouped) documents are checked by the same scan.
Validated by: L1 unit test on the merge in `gen/src/export/openapi.rs` with two member APIs sharing ids; L1 artifact test (beside `openapi_strict_completeness.rs` / `artifact_drift.rs`) that scans every committed `schematic/openapi/*.json` for duplicate ids.

**R15. Committed artifacts regenerated.** `schematic/openapi/ollama.json` is regenerated by the generator so it contains no duplicate ids; no other OpenAPI file changes in content except as R14's rename rule demands. This is produced by running the generator, never by hand.
Validated by: the R14 artifact test, and `artifact_drift` passing.

### Isolated fresh-output compile check (D49)

**R16. Isolated crate, offline, checkout untouched.** A test generates the client crate into a temporary directory outside the checkout, with a generated `Cargo.toml`, and compiles it with no network access to crates registries or providers (`cargo check --offline` or equivalent; mechanism is default 8). The checkout is unchanged afterward, proven without depending on a clean worktree (the cargo target directory is build output and is exempt).
Validated by: L1 test in `schematic-gen` (a new file under `gen/tests/l1/`), using `slow_` in the test name so `sanity` skips it. Does not need `just test-l2`. Before generating, the test hashes every tracked and untracked file of the checkout (excluding the cargo target directory and git metadata; for example the file list from `git ls-files --cached --others` plus their content hashes), and after the run it recomputes and compares; alternatively it runs the generator against a copied tree and compares the copy's inputs. A dirty worktree is a normal precondition: the assertion is before-versus-after equality, never `git status` being empty or unchanged text.

**R17. Runs on a normal test run.** The check is not `#[ignore]`d; `just test` runs it. The pre-existing `#[ignore]`d `generated_code_compiles` and clippy tests in `gen/tests/l1/e2e_generation.rs` are either superseded by it and removed, or retained and still runnable through `just test-e2e`; the choice and the `test-e2e` recipe consequence are logged (default 9).
Validated by: `just test` output listing the new test; the test-audit filters in the `rust-testing` skill (`tier-filters.md`) classify it as slow L1, not L2.

**R18. Negative control.** A deliberately broken generator output (for example a test hook that injects a type error into one emitted module) makes the check fail with the compiler error shown.
Validated by: L1 test using the same helper with a broken input and `#[should_panic]`-style or result assertion on the failure.

**R19. Coverage.** The check compiles (a) a small fixture crate set with purpose-built definitions covering JSON, multipart form, URL-encoded, text body, binary body, each auth strategy (Bearer, ApiKey header, Basic, `ApiKeyParam` query, `ApiKeyParam` cookie, OAuth where the generator supports it), header parameters, cookie parameters, and query parameters; and (b) the full set of 18 real APIs (default 17).
Validated by: the same L1 test file; assertions list the covered fixture names.

**R20. Determinism, no checkout rewrite.** The check also compares the fresh 18-API output with the committed `schematic/schema/src` and `schematic/openapi`, `schematic/postman` content and fails with a diff summary if they differ, without writing into the checkout (this is the drift check `artifact_drift` partly provides; reuse it rather than duplicate if it already covers the same ground, and log which).
Validated by: L1 test comparing in-memory or temp output with committed files.

### Request-side features (D10, D33)

All of R21 to R27 are exercised against purpose-built fixture definitions through a local mock HTTP server that records the raw request. The mock server must be local and must not bind a fixed port (bind port 0). `wiremock` is the repo-standard choice (see the `wiremock` skill); confirm it is already a dev-dependency of `schematic-gen` or `schematic-schema` before adding one, and log any dependency added (update `schematic/docs/dependencies.md`).

**R21. Text request body.** An endpoint declaring `ApiRequest::Text { content_type }` generates a request struct with a `body: String` field; `into_parts` yields the body verbatim under that content type; the server sees the exact text and the declared `Content-Type`.
Validated by: L1 codegen assertion in `gen/src/codegen/request_structs/tests.rs` plus a mock-server runtime test. The runtime test runs in a compiled fixture crate (reuse the R16 mechanism) or in the generated `schematic-schema` crate's tests; the plan chooses (default 10).

**R22. Binary request body.** Same for `ApiRequest::Binary { content_type }` with a `body: Vec<u8>` field; the server sees the exact bytes and the declared `Content-Type`, including bytes that are not valid UTF-8.
Validated by: as R21.

**R23. `ApiKeyParam` injection.** For an API whose auth is `AuthStrategy::ApiKeyParam { name, location }`: with `location` query, the key is appended to the query string under `name` on every request (merged with the endpoint's own query parameters, never replacing them); with `location` cookie, the server sees `Cookie: name=value` (merged with any cookie parameters). The key is percent-encoded where the location requires it.
Validated by: mock-server runtime test.

**R24. `ApiKeyParam` credential source and errors.** **Temporary:** the explicit `with_api_key_param` setter and the generator-local env scan are interim mechanisms; spec 2 owns credentials (D11) and removes both. The key can be supplied explicitly (default 16) or resolved from the API's `env_auth` list, first non-empty variable winning, with the env fallback mapping carrying the parameter-style key. The silent drop at `legacy_env_mapping_for` is closed in the way default 15 states. When no key is available and auth is required, the existing "authentication required" error is returned, and when a key is available it is no longer returned. `headers_satisfy_fallback` is consistent with this.
Validated by: mock-server runtime tests (explicit key, env key, empty-then-nonempty env list, no key) using `serial_test` for env mutation; L1 unit tests in `schematic-define` if the mapping changes there.

**R25. Header parameters.** Each `EndpointParams.header` entry becomes a request-struct field and `with_*` builder, sent under its exact declared wire name; only set values are sent; endpoint headers declared on the endpoint still win over API headers as today.
Validated by: codegen test and mock-server runtime test.

**R26. Cookie parameters.** Each `EndpointParams.cookie` entry becomes a request-struct field sent in one `Cookie` header (`a=1; b=2`) under its declared name, merged with an `ApiKeyParam` cookie without duplicating the header.
Validated by: mock-server runtime test.

**R27. The key never appears in debug output.** With a sentinel key, `format!("{:?}", client)` (and the request struct) does not contain it, for header, query, and cookie strategies. This is the scope of D10 only; the broader redaction contract (D22, D23, D40) is spec 2.
Validated by: L1 sentinel test.

### Query-parameter split (S6 / P7)

**R28. Query pairs separate from the path.** `into_parts` returns the path (path parameters substituted, no query string) and the query pairs as a separate collection, preserving array parameters as repeated keys, and the generated client applies them when building the URL. `RequestParts` changes accordingly (default 12). Header, cookie, and `ApiKeyParam` query additions go through the same separation, so a later redaction layer can mask exactly the secret pairs.
Validated by: codegen tests in `gen/tests/l1/query_params_codegen.rs`, `path_substitution.rs`, `http_client.rs` updated for the new shape.

**R29. Wire equivalence.** For every existing endpoint with query parameters, the URL that reaches the server (path plus encoded query, parameter order, array encoding, encoding of reserved characters) is identical before and after the split. Evidence is a mock-server test over representative endpoints plus the unchanged expectations of the existing query tests, not only string comparison of generated code.
Validated by: L1 mock-server tests comparing against golden request lines captured from the pre-change generator (capture them in the first phase, before changing any code).

### Cleanup, regeneration, and scope

**R30. Stale tree deleted.** `schematic/gen/schematic/` (five tracked files under `schema/`) is deleted from git. No workspace member, CI input, or test reads it (the root `Cargo.toml` `exclude` list names only `schematic/schema`). References outside spec and plan snapshots are updated: `checks/cargo-machete.txt` lists it; treat that file by default 18.
Validated by: `git ls-files schematic/gen/schematic` is empty; `grep` finds no non-snapshot reference; `just test` and `just lint` pass.

**R31. Regeneration only through the generator; generator-only blast radius.** `schematic-schema` is regenerated with `just generate` (never hand-edited) after the generator changes land; the committed diff is explained entirely by R28 (query split) and R14 (ids), plus any feature the live definitions already use. No file in a dependent crate (sniff, biscuit-speaks, model-citizen, the homelab crates) changes, and each still builds and passes its tests. A search for `into_parts` and `RequestParts` outside `schematic/` was run while drafting (excluding `schematic/gen` and `schematic/schema`) and found none; the implementer repeats it before changing `RequestParts`. If a dependent breaks, stop and record it (this contradicts the spec's premise) rather than editing the dependent.
Validated by: `just test-schema`, `just lint-schema`, and `just test {pkg}` from the repo root for each dependent package (list them with `sniff repo packages`; the dependents named by the parent are sniff, biscuit-speaks, model-citizen, and the five homelab crates).

**R32. Live feature counts recorded.** The implementer measures, from the definitions at hand, how many endpoints use text bodies, binary bodies, multipart bodies, `ApiKeyParam`, header parameters, and cookie parameters, and records the numbers in `implementation-log.md` next to the parent's figures, including a note reconciling or not reconciling "six" against "about 29" (D49 item 6).
Validated by: a small test or script output pasted in the log; there is no pass/fail assertion beyond the numbers being recorded. The endpoint literal count is about 519 (openai alone 265); record the measured total next to it.

**R33. `?` in a path template is a hard error.** Parent recommendation 3 lists `?` in path templates among the silent drops to turn into `validate_api` errors: a query parameter smuggled into a path string cannot be separated, redacted, or typed. A path template containing `?` fails validation with an `error` diagnostic (feature `query-in-path`, default 20) for every output. The one shipped case, Gitea `GetGitTreeRecursive` (`/repos/{owner}/{repo}/git/trees/{sha}?recursive=true`), is migrated to a proper declared query parameter `recursive` in the same change that lands the check, so the tree stays green; the committed Gitea schema, OpenAPI, and Postman artifacts are regenerated and the diff is explained by this migration. This is the only endpoint-definition edit this spec makes (Pre-authorizations item 9).
Validated by: L1 unit tests in `gen/src/validation.rs` (a `?` path is rejected, a clean path accepted); a Gitea definition test that no endpoint path contains `?` and that `recursive` is a query parameter; the wire-equivalence test (R29) extended to show the migrated endpoint still sends `?recursive=true`.

**R34. Runtime ABI marker hook.** Spec 2 adds a runtime ABI marker (D39, P11). This spec defines the interface the partial-generation compatibility check reads now: a single constant in the generator (for example `RUNTIME_ABI_MARKER_SOURCE`, an `Option` of the marker the retained `shared.rs` or runtime declares) that the generator compares between the retained output and the one being assembled. While spec 2 has not landed the marker is absent, and the check degrades to the interim default 4 (byte-diff of `shared.rs`). When the marker exists the marker comparison replaces the byte-diff; spec 2 changes the constant's source, not the check.
Validated by: L1 test with the marker absent (byte-diff behavior, R5) and a test with a fixture marker present on both sides (equal markers proceed, unequal markers abort before any write).

## Design

Decided means a parent decision fixes it. Proposed is this spec's suggestion; the implementer may adopt a better one and logs the change.

| Symbol (package) | What it does | Status |
| --- | --- | --- |
| Whole-group regeneration in `output::generate_and_write_all` (`schematic-gen`, `src/output/mod.rs`) | Resolves the selected apis to module groups, regenerates each group completely, keeps others | Decided (D9) |
| Inventory reader (`schematic-gen`, new function in `src/output/`) | Establishes the retained module set from the existing output before any write; any failure aborts | Decided as a requirement (D9); mechanism Proposed (default 3) |
| Ownership proofs `is_generator_owned_rs_file`, `is_generator_owned_dir`, `remove_conflicting_paths` (`schematic-gen`) | Keep the existing proof-of-ownership checks | Decided (D9: "retain its existing proof-of-ownership checks") |
| Two-phase writer (`schematic-gen`) | Phase 1 builds, validates, formats everything in memory; phase 2 writes group files, then shared files, then `prelude.rs` and `lib.rs` last, reporting written and unwritten files on failure | Proposed |
| `ValidationDiagnostic` and a `Feature` enumeration (`schematic-gen`, `src/validation.rs` or new `src/diagnostics.rs`) | One diagnostic with api, endpoint, feature, output, message; `Display` and `Serialize` | Fields Decided (D10); type Proposed |
| Per-output validators `validate_rust_client`, `validate_openapi`, `validate_postman` (`schematic-gen`) | Validate the whole selected batch for one output before that output writes | Behavior Decided (D10); names Proposed |
| `validate_api` additions: duplicate ids, duplicate `(method, path, selector)` (`schematic-gen`) | R12, R13 | Rules Decided; code shape Proposed |
| `endpoint_uniqueness_key(endpoint) -> (Method, NormalizedPath, Option<Selector>)` (`schematic-gen`) | The single hook spec 5 extends; selector is `None` now | Proposed |
| OpenAPI merge id uniqueness in `export/openapi.rs` (`schematic-gen`) | Rename clashing later ids, then verify none remain | Requirement Decided; rename rule Proposed (default 7) |
| `fresh_output` test helper and `slow_` tests (`schematic-gen`, `tests/l1/`) | Generate to a temp crate, `cargo check --offline`, compare with committed output | Requirement Decided (D49); mechanism Proposed |
| Text/binary body fields and `into_parts` arms (`schematic-gen`, `codegen/request_structs/{body,into_parts}.rs`) | R21, R22; emit `RequestBody::Raw` | Decided (D10, D33); field types Proposed |
| `ApiKeyParam` handling in `codegen/client/helpers.rs` and `codegen/api_struct/` (`schematic-gen`) | Inject key into query or cookie, env resolution, fix `headers_satisfy_fallback` | Decided (D10); structure Proposed |
| `legacy_env_mapping_for` in `define/src/types.rs` (`schematic-define`) | Possible additive change only if default 15 permits | Proposed |
| `with_api_key_param` and the generator-local `ApiKeyParam` env scan (`schematic-gen`) | R23, R24 | **Temporary**: spec 2 owns credentials (D11) and removes both |
| Diagnostic severities and public validator surface (`schematic-gen`) | Owned here; spec 6 (WebSocket) maps its validation onto the same type, severities, and entry points | Decided as ownership; shape Proposed |
| `query-in-path` check and Gitea `GetGitTreeRecursive` migration (`schematic-gen`, `schematic-definitions`) | R33 | Decided (parent recommendation 3); migration is the single definition edit |
| Runtime ABI marker constant (`schematic-gen`) | R34 | Interface Decided here; marker supplied by spec 2 (D39) |
| Header and cookie parameter fields and builders (`schematic-gen`, `codegen/request_structs/`) | R25, R26 | Decided (D10, D33) |
| `RequestParts` and `into_parts` query separation (`schematic-gen` template for `schematic-schema/src/shared.rs`, and `codegen/request_structs/{into_parts,shared}.rs`) | R28 | Decided as a requirement (S6, D23 prerequisite); shape Proposed (default 12) |
| Deleted tree `schematic/gen/schematic/` | R30 | Decided (D50); deletion authority is owned by this spec only |

## Suggested phases

The plan may refine this. Each phase ends with `just test` and `just lint` green in `schematic/` and nothing committed.

1. **Baseline and safety net.** Capture golden request lines (R29) and the live feature counts (R32) from the current generator. Add the isolated compile-check helper and its negative control (R16 to R18), initially compiling the fixture set and the 18 APIs as they are. This phase guards every later generator change (the reason D49 puts it first).
2. **Validation foundation.** `ValidationDiagnostic`, per-output validators, duplicate id and `(method, path)` checks with the selector hook, type and identifier checks (R8 to R13), the `?`-in-path error with the Gitea migration (R33), and the diagnostic severities exposed for spec 6.
3. **Duplicate `operationId`.** Merge fix, final scan, artifact test, regenerate `openapi/ollama.json` (R14, R15).
4. **Partial generation.** Inventory reader, two-phase writer, group-only cleanup, shared-file rebuild, runtime compatibility rule and the ABI marker hook (R34), failure reporting, OpenAPI and Postman group rule, README and `generate-one` docs (R1 to R7).
5. **Query split.** Change `RequestParts` and the client URL building; wire equivalence against the golden lines (R28, R29). Run `just generate` and inspect the schema diff.
6. **Request-side features.** Text and binary bodies; header and cookie parameters; then `ApiKeyParam` injection, env mapping, and Debug safety (R21 to R27), each with fixtures and mock-server tests.
7. **Cleanup and close.** Delete `schematic/gen/schematic/` (R30), full regeneration, dependent builds (R31), docs, skills, and README drift pass, implementation log finalized.

## Acceptance criteria and definition of done

Done means the agent reaches "implementation complete, ready for review". The agent never runs `just complete` and never moves this directory to `_completed`.

- Every requirement R1 to R34 has a passing validation or, for R32, a recorded measurement.
- Builds and tests pass on macOS, Linux, native Windows, and WSL2 per repo rules. The implementer runs what the current host can and records, per the `os` skill, which hosts produced evidence and which did not; it does not claim an OS is untestable without loading that skill. CI runs only the environments its schedule assigns; the agent does not change CI scope.
- `just test`, `just lint`, `just lint-schema`, `just test-schema`, and `just test-l2` (no window focus, no new L2 tests expected) pass in `schematic/`.
- `schematic-schema` was regenerated by `just generate`; no generated file was hand-edited. `just check-drift` (or the `artifact_drift` test) is clean.
- Every dependent crate builds and its tests pass (sniff, biscuit-speaks, model-citizen, the five homelab crates), with no source change in any of them.
- Documentation drift pass done: `schematic/README.md`, `schematic/gen/README.md` (the single-API usage text), `schematic/docs/` topic pages for the changed behavior (partial generation, validation diagnostics, request-side features, query separation; write a developer-facing page with examples and a Mermaid diagram of the generation flow, and do not link the page to this spec), `schematic/docs/dependencies.md` if a crate was added, and the affected skills (`schematic/skill/`, `.claude/skills/` entries for schematic if present, **unverified** which exist). Code comments and `///` docs on every changed symbol were reviewed per the repo's comment-drift rule; where a comment and the code disagreed, the code was treated as correct and the drift was logged.
- No commits by the agent unless the plan says otherwise. If the plan does say so, commit rules in the repo `CLAUDE.md` apply (author identity, signed, no agent attribution, `git verify-commit HEAD`).
- `implementation-log.md` exists, is current, and covers every item of the Human review agenda.

## Pre-authorizations the author must confirm before the unattended run (D50 item 8)

These are requests, not grants. They would otherwise pause an unattended run, they come from the parent's D50 list (item 8) and from this spec's content, and none is in force until the author confirms it when approving the plan; the plan restates them. Items marked **conditional** apply only if the stated condition holds.

1. Regenerate `schematic-schema`, `openapi/`, and `postman/` with `just generate` (a large tree diff is expected and acceptable).
2. Delete the git-tracked stale tree `schematic/gen/schematic/` (R30). **This spec alone owns that deletion; no other spec carries any deletion authorization for it or any other tree.**
3. `checks/cargo-machete.txt` (outside the area): keep the default (default 18): **conditional**, edit it only if it is a generated file with a regeneration recipe; otherwise leave it and log the stale line as a finding. This is a narrow exception to "no edits outside the area" and needs the author's explicit confirmation.
4. **Conditional:** add a dev-dependency to `schematic-gen` or `schematic-schema` for the mock server or `serial_test` only if it is not already present; update `docs/dependencies.md`.
5. Run the offline compile check in a temp directory outside the checkout, and create files under the system temp directory and the cargo target directory.
6. Change the public shape of the generated `RequestParts` and `into_parts` in `schematic-schema` (R28). No dependent uses them (verified by search while drafting).
7. Remove or rename `#[ignore]`d tests in `gen/tests/l1/e2e_generation.rs` and adjust the `test-e2e` recipe (R17).
8. **Conditional:** an additive field or function in `schematic-define` only if default 15 allows.
9. Edit the single Gitea `GetGitTreeRecursive` endpoint definition (and its `definitions/README.md` row) to declare `recursive` as a query parameter (R33).

Not pre-authorized, so the implementer stops and logs rather than proceeds: editing any dependent crate; changing CI configuration or scope; creating the `schematic-runtime` package; touching the roughly 519 endpoint literals in the definitions beyond the single Gitea endpoint migration in R33 (Pre-authorizations item 9); adding a selector syntax; committing, unless the plan says so.

## Dependencies and ordering

- `depends_on: []`. This is first in order (D50).
- Specs 2 to 7 depend on this one: spec 2 needs the query split (D23), the `ApiKeyParam` key being a distinct query pair or cookie (D40 redaction), and the compile check as the guard for every later generator change. Spec 5 extends the uniqueness key with a selector (D36).
- Interaction with later specs: spec 2 replaces client injection, the envelope, and the send path, and will rework `RequestParts` again. Keep R28 minimal so that rework is cheap. Spec 7 owns feature flags, which interact with partial generation (D9); this spec does not add flags.

## Planning-time details assigned to this spec

The parent's P7 (query-parameter separation) is wholly owned here, and the diagnostics-format part of P10 is owned here. The remaining defaults fill gaps the parent leaves open for D9, D10, D33, and D49. Each default is adopted by the implementer if the author has not ruled before planning, and logged.

1. **Diagnostics format (P10 part).** Default if not ruled before planning: a `ValidationDiagnostic { api, endpoint, feature, output, message }` rendered by `Display` as one line `api=<api> endpoint=<endpoint> feature=<feature> output=<output>: <message>`.
2. **Machine readability (P10 part).** Default if not ruled before planning: the diagnostic type also derives `serde::Serialize`; no new CLI flag; the CLI prints the `Display` lines to stderr and exits non-zero. Spec 7 can add a flag.
3. **Inventory source for partial generation.** Default if not ruled before planning: the retained inventory is the set of module files and directories in the existing output that carry the generator-owned proof, matched to the in-process definitions by resolved module path; the existing `lib.rs` is parsed only to confirm the module list and a parse failure aborts. Exports for retained modules are re-emitted from the in-process definitions, not copied from old text. Unclassifiable entries cause an abort, not a skip.
4. **"Incompatible shared-runtime change" rule (the runtime package check belongs to spec 2).** Default if not ruled before planning: a partial run aborts with a "run full generation" error when the newly assembled `shared.rs` differs byte-for-byte from the one on disk and at least one retained module is not being regenerated. An identical `shared.rs` lets the partial run proceed.
5. **Write order and reporting.** Default if not ruled before planning: write group modules first, then `shared.rs`, then `prelude.rs`, then `lib.rs` last; on I/O failure return the lists of written and unwritten paths.
6. **Duplicate endpoint id scope.** Default if not ruled before planning: checked within each API and across all APIs sharing a generated module group.
7. **Merged `operationId` rename rule.** Default if not ruled before planning: the first occurrence in deterministic merge order keeps its id; a later clash is renamed by prefixing the owning API's name in PascalCase (for example `OllamaOpenAIListModels`), and if that clashes too, a numeric suffix is appended. The rename is applied only in the grouped merge, not in per-API documents.
8. **Compile-check mechanism.** Default if not ruled before planning: write a generated `Cargo.toml` and sources to a `tempfile` directory, copy `schematic/schema/Cargo.lock` as the lockfile if one exists, run `cargo check --offline --locked` (fall back to `--offline` if `--locked` fails only because the lock needs a new entry, and log it), with the target directory under the workspace target directory in a `schematic-fresh-output` subfolder so dependencies build once. If a host's cargo cache cannot satisfy `--offline`, the test fails with an explanatory message and does not silently skip.
9. **Disposition of the `#[ignore]`d e2e compile tests.** Default if not ruled before planning: keep `generated_code_compiles` only if it tests something the new check does not; otherwise remove it and the clippy duplicate (clippy already runs in `just lint-schema`) and point `test-e2e` at remaining ignored tests or delete the recipe if none remain.
10. **Where request-side runtime tests run.** Default if not ruled before planning: in an isolated fixture crate built by the R16 helper (so they exercise freshly generated code), using a mock server; the 18 real APIs are not given new runtime tests.
11. **Text and binary field types.** Default if not ruled before planning: the request struct field is `body: String` for text and `body: Vec<u8>` for binary, and `into_parts` emits `RequestBody::Raw { content_type, bytes }`.
12. **`RequestParts` shape.** Default if not ruled before planning: keep a tuple and add a fifth element `Vec<(String, String)>` for query pairs, the path element no longer containing `?`. Header parameters join the existing headers element; cookie parameters join as one `Cookie` header entry.
13. **Cookie serialization.** Default if not ruled before planning: `name=value` pairs joined by `; ` in declaration order, values not percent-encoded unless they contain characters illegal in a cookie value, in which case percent-encode them.
14. **Header and cookie parameter typing.** Default if not ruled before planning: reuse the query parameter type mapping (`QueryParamType` to Rust type), generate `Option<T>` fields with `with_*` builders, and do not honor `required` (query-parameter `required`, style, `explode`, and enum fidelity are spec 4).
15. **`ApiKeyParam` env fallback mapping.** Default if not ruled before planning: generator-local and **temporary** (spec 2 owns credentials and removes it). The generator reads the API's `env_auth` list for an `ApiKeyParam` strategy and emits the env names directly into the generated client, leaving `EnvMapping` in `schematic-define` unchanged (64 construction sites of `EnvMapping {` were counted outside generated code, **unverified** how many use `..Default::default()`). The doc comment at `legacy_env_mapping_for` is updated to say where parameter-style keys are resolved.
16. **Explicit `ApiKeyParam` key.** Default if not ruled before planning: generated clients for an `ApiKeyParam` API get a builder method `with_api_key_param(value)` storing the key in a field excluded from `Debug`. This is new public surface that is **temporary**: spec 2's credential work (D11) owns credentials and removes it; flag it for review 1.
17. **Compile-check scope.** Default if not ruled before planning: two `slow_` tests, one over the fixture set and one over the full 18-API output.
18. **`checks/cargo-machete.txt`.** An explicit author-confirmed pre-authorization item (item 3). Default if not ruled before planning: if the file has a regeneration recipe (look in the root `justfile` and `just/` recipes), regenerate it; if not, leave it and log the stale line as a finding for the author.
19. **Docs page location.** Default if not ruled before planning: `schematic/docs/generation/` with one page for partial generation and validation and one for request-side features.
20. **Feature identifiers.** Default if not ruled before planning: kebab-case strings from a `Feature` enum (`text-request-body`, `binary-request-body`, `api-key-param`, `header-parameter`, `cookie-parameter`, `duplicate-endpoint-id`, `duplicate-method-path`, `invalid-type-syntax`, `duplicate-operation-id`, `query-in-path`).
21. **Naming of new tests and files.** Default if not ruled before planning: `gen/tests/l1/fresh_output_compile.rs`, `gen/tests/l1/partial_generation.rs`, `gen/tests/l1/request_side_features.rs`; tests use `slow_` only where they invoke cargo. Keep to the `rust-testing` skill's layout rules (`test_layout.rs` enforces some).
22. **Partial generation for OpenAPI and Postman.** Default if not ruled before planning: artifacts are written only for regenerated groups, named by the group's resolved module path as today. The two divergent module-path resolvers are spec 7's, so if they disagree for some name (the `PagerDutyApi` case) log it and do not fix it here.

## Risks and failure modes

- **Partial writes.** A multi-file update is not transactional. Mitigation: all work precedes the first write; the write order puts `lib.rs` last; failures are reported with written and unwritten lists (R6). A crash between writes can still leave a mixed tree; a rerun of the same command must converge (test it: run the partial generation twice).
- **False ownership.** Deleting a file the generator did not write loses user work. Mitigation: only the existing proofs; tests plant hand-written files (R3).
- **Offline compile unavailable.** A host whose cargo cache lacks the generated crate's dependencies cannot pass `--offline`. This is **unverified** on native Windows and WSL2 CI hosts. A silent skip would void the guard, so the test fails loudly; log the first host where it fails.
- **Compile time.** Compiling the 18 APIs is slow (parent spike measured about 11 s for a synthetic 550-endpoint trait, not a real compile). Use a shared target directory and `slow_`.
- **Query split alters the wire.** Percent-encoding or ordering may change when `reqwest` builds the query. Mitigation: R29 golden lines captured before the change.
- **`ApiKeyParam` surface churn.** The explicit-key builder (default 16) may be replaced by spec 2. Flagged for review 1.
- **Cookie and query merging.** An `ApiKeyParam` cookie and cookie parameters must share one `Cookie` header; a bug yields two headers that servers treat differently.
- **Env mutation in tests.** Env-based key tests race; use `serial_test`.
- **Hidden dependents.** A crate outside the searched set could use `into_parts` or `RequestParts`. The search covered `.rs` files repo-wide outside `schematic/gen` and `schematic/schema`; the implementer repeats it.
- **Fixture drift.** Fixtures exercise features no shipped definition uses, so the generated 18-API output may not change at all for R21 to R27; do not mistake an unchanged schema diff for missing work.
- **Parent gaps (see Refresh notes and the Human review agenda).** The "incompatible shared-runtime change" rule and the inventory source are underspecified in the parent.

## Process

These specs are built for the following loop, confirmed by the author:

1. Each spec has a 1:1 plan. The plan is implemented end to end, phase by phase, **without a human in the loop**.
2. Then **review 1**: time is set aside for the author to review decisions made during implementation and any open design questions.
3. Then the **next** spec and plan are refreshed from the outcomes.
4. Fix and review cycles repeat until the reviewer deems the spec production ready.

An agent's terminal state is "implementation complete, ready for review". Only the author moves a spec to `_completed`; an agent never does and never runs `just complete`.

Because no human is available while the plan runs, this spec (a) is self-contained: decisions are restated above with D-numbers and the parent path as provenance; (b) gives a default for every remaining planning-time detail, each flagged "Default if not ruled before planning", which the implementer adopts and logs; and (c) lists a Human review agenda below. **The implementer writes each agenda item into `implementation-log.md` as it occurs, not at the end.** If a requirement cannot be met as written, the implementer records why, takes the safest alternative that leaves the tree green, and continues; it does not stop to ask unless an item on the "not pre-authorized" list in the confirm-before-the-run section is the only way forward.

### Human review agenda

Record every one of these in `implementation-log.md` for review 1:

- Every default adopted (defaults 1 to 22), with the value actually used and any change from the stated default.
- Every design choice not dictated by this spec (names, module layout, error types, test file layout, fixture content).
- Every deviation from a requirement or from the Proposed design, with the reason.
- Every open design question found, including:
    - whether the inventory reader and the "incompatible shared-runtime change" rule are adequate once spec 2 introduces a runtime package version check (D39);
    - the explicit `ApiKeyParam` key setter's public surface (default 16);
    - the `RequestParts` shape choice (default 12) and what spec 2 will need;
    - the disposition of the `#[ignore]`d e2e tests (default 9);
    - whether the offline compile check worked on every host that was available, and which hosts were not available;
    - the measured feature counts and whether "six" and "about 29" reconcile (R32);
    - the compile-check run time added to `just test`;
    - any generated-file diff in `schematic-schema` not explained by R14 and R28;
    - any finding in `checks/cargo-machete.txt` or other references to the deleted tree;
    - the interim D9 inventory default (default 3), confirmed or replaced;
    - the interim incompatible-shared-runtime default (default 4, `shared.rs` byte-diff) and how it will hand over to the spec 2 runtime ABI marker (R34);
    - `with_api_key_param` (default 16), a temporary public surface for spec 2 to remove;
    - whether the offline compile check works on Windows and WSL2 (not only macOS and Linux), recorded per host;
    - verification that the generated client sends `RequestBody::Raw` with its declared `Content-Type` (the unverified claim in Problem item 2), with the mock-server evidence.
- Every dependency added, every file touched outside `schematic/`, and every dependent crate build result.
- Any comment-versus-code drift found and how it was resolved (code assumed correct).

## Refresh notes

None: first in order. `refresh_required` is `false`; `depends_on` stays `[]`.

Later specs may be affected by this spec's outcomes, recorded here so the refresh of specs 2 to 7 knows where to look: the final `RequestParts` shape and the explicit `ApiKeyParam` setter (spec 2); the `endpoint_uniqueness_key` hook (spec 5); how partial generation treats shared files and the "incompatible shared-runtime change" rule (spec 2's runtime-package version check and spec 7's feature flags, which parent question 11 says interact with D9); and the diagnostics format and severities (spec 6 maps WebSocket validation onto them; spec 7 may add a machine-readable flag).

Interim items later specs replace, recorded here for their refresh:

- `with_api_key_param` and the generator-local `ApiKeyParam` env scan are **temporary**; spec 2 owns credentials (D11) and removes them.
- The partial-generation compatibility check reads the runtime ABI marker interface defined by R34; until spec 2 (D39, P11) adds the marker it uses the interim `shared.rs` byte-diff default. Spec 2 supplies the marker and the check then switches without a change to this spec's interface.
