---
review: schematic/features/2026-05-07-artificial-analysis/review-2.md
spec: schematic/features/2026-05-07-artificial-analysis/spec.md
prior_review: schematic/features/2026-05-07-artificial-analysis/review-1.md
prior_plan: schematic/features/2026-05-07-artificial-analysis/review-plan-1.md
created: 2026-05-07
phases: 4
start_phase: 3
packages:
  - schematic-definitions
  - schematic-gen
  - schematic-schema (workspace-excluded — use --manifest-path)
artifacts_touched:
  - schematic/schema/tests/artificial_analysis_client.rs
test_files_added: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
---

# Review 2 — Remediation Plan

## Goal

Close the single HIGH finding in `review-2.md`:

> **Generated-client tests bypass the typed response contract.** The integration
> tests in `schematic/schema/tests/artificial_analysis_client.rs` deserialize
> responses as `serde_json::Value` rather than the spec-declared response
> structs, so the typed-response side of `LlmModelsResponse`,
> `MediaModelsResponse`, and `CritPtEvaluateResponse` is effectively unverified.

The remediation:

1. Update mock success payloads to match the **declared response structs**
   (`LlmModelsResponse`, `MediaModelsResponse`, `CritPtEvaluateResponse`) — i.e.
   include every required field defined in
   `schematic/definitions/src/artificial_analysis/types.rs`.
2. Add at least one **typed deserialization test per response shape**:
   - `client.request::<LlmModelsResponse>(...)` (with populated `prompt_options`).
   - `client.request::<MediaModelsResponse>(...)` (with `include_categories`).
   - `client.request::<CritPtEvaluateResponse>(...)` (with all four required
     fields: `accuracy`, `timeout_rate`, `server_timeout_count`,
     `judge_error_count`).
3. Keep the existing `serde_json::Value` checks **only** where the test is
   intentionally about raw request serialization or hook behavior (e.g. the
   absent-field `batch_metadata` test, the missing-auth pre-flight test).

In addition, the diff must leave the schematic package areas:

- **All tests passing.**
- **Zero `cargo clippy` warnings or errors** in `schematic-define`,
  `schematic-definitions`, `schematic-gen`, `schematic-oauth`, and the excluded
  `schematic-schema` crate.

The plan is structured as four phases. Phase 1 is the surgical fix for the
review finding. Phase 2 broadens typed-response coverage to the remaining
endpoints. Phases 3-4 enforce the global passing-tests and lint-clean
acceptance criteria across the schematic package area.

## Working Constraints

- Working directory: `/Users/ken/.claudine/worktrees/rusty-biscuit/schematic`
  (the `schematic` worktree of the `rusty-biscuit` monorepo).
- `schematic/schema` is **excluded from the workspace**. Always use
  `--manifest-path schematic/schema/Cargo.toml` for `cargo` commands targeting
  it.
- `schematic-gen` is the only path that may write to `schematic/schema/src/*.rs`,
  `schematic/openapi/*.json`, and `schematic/postman/*.postman_collection.json`.
  Do not hand-edit those files. **Phase 1-2 do not require regeneration** — the
  generated client surface already exposes typed `request::<T>()` and the
  declared response structs are already re-exported via
  `schematic_schema::artificial_analysis::*`.
- Follow the rustdoc convention from `CLAUDE.md`: no `# H1` inside `///`
  blocks, use `## H2` for `Examples` / `Returns` / `Errors` / etc.
- No new `cargo build`/`cargo check`/`cargo test` at the repo root — always
  scope with `-p <pkg>` flags or `--manifest-path`.
- Do not introduce new `#![allow(...)]` or `#[allow(...)]` attributes unless
  matching a pre-existing project-wide convention. Prefer fixing the underlying
  code.

## Reference: Required Response-Struct Fields

The mock payloads in Phases 1-2 must include **every** required (non-`Option`)
field on the response structs. Source of truth:
`schematic/definitions/src/artificial_analysis/types.rs`.

### `LlmModelsResponse`
- `status: i32`
- `prompt_options: PromptOptions`
  - `parallel_queries: i32`
  - `prompt_length: String`
- `data: Vec<LlmModel>` (may be empty `[]`)

A populated `LlmModel` (used for at least one fixture) requires:
`id`, `name`, `slug`, `model_creator { id, name, slug }`,
`evaluations { artificial_analysis_intelligence_index,
artificial_analysis_coding_index, artificial_analysis_math_index, mmlu_pro,
gpqa, hle, livecodebench, scicode, math_500, aime }`,
`pricing { price_1m_blended_3_to_1, price_1m_input_tokens,
price_1m_output_tokens }`, `median_output_tokens_per_second`,
`median_time_to_first_token_seconds`.

### `MediaModelsResponse`
- `status: i32`
- `include_categories: Option<bool>` — present in fixture for the
  `include_categories=true` test, omitted in fixtures for endpoints that do
  not support it.
- `data: Vec<MediaModel>` (may be empty `[]`)

A populated `MediaModel` requires: `id`, `name`, `slug`,
`model_creator { id, name, slug }`, `elo`, `rank`, `ci95`, `appearances`.
`release_date` and `categories` are `Option<...>`. The "categories populated"
fixture (Phase 2) must include a non-empty `categories: [CategoryBreakdown]`
where each entry has `style_category`, `subject_matter_category`, `elo`,
`ci95`, `appearances` (and may include `format_category`).

### `CritPtEvaluateResponse`
- `accuracy: f64`
- `timeout_rate: f64`
- `server_timeout_count: i32`
- `judge_error_count: i32`

The current mock body `{"accuracy": 0.0, "results": []}` is **wrong**: it omits
three required fields and includes an undeclared `results` field. Replace with
a body that matches the declared struct exactly.

## Phase 1 — Fix the Three Tests Called Out by the Review

**Goal:** Make every test currently using `Result<serde_json::Value, _>`
either (a) deserialize into its declared response struct or (b) keep the
`serde_json::Value` typing only because the test is intentionally about
**request-side** behavior. Update mock bodies so they conform to the declared
response structs.

### File touched

- `schematic/schema/tests/artificial_analysis_client.rs`

### Current state classification

| # | Test fn | Today | Phase 1 action |
|---|---|---|---|
| 1 | `list_text_to_image_models_emits_include_categories_query` | `Result<serde_json::Value, _>`, partial mock body | **Type as `MediaModelsResponse`**; populate mock to satisfy required fields |
| 2 | `list_llm_models_uses_no_query_string` | `Result<serde_json::Value, _>`, partial mock body | **Type as `LlmModelsResponse`**; populate mock incl. `prompt_options` |
| 3 | `missing_auth_returns_authentication_required_error` | `Result<serde_json::Value, _>`, no mock | **Keep `serde_json::Value`** — pre-flight error path; no body decoded. Add an inline comment explaining why. |
| 4 | `explicit_api_key_overrides_env_fallback` | `Result<serde_json::Value, _>`, partial mock body | **Type as `LlmModelsResponse`**; populate mock body |
| 5 | `env_api_key_is_used_when_no_explicit_key` | `Result<serde_json::Value, _>`, partial mock body | **Type as `LlmModelsResponse`**; populate mock body |
| 6a | `critpt_evaluate_serializes_populated_batch_metadata` | `Result<serde_json::Value, _>`, mock body has wrong schema (`results: []`) | **Type as `CritPtEvaluateResponse`**; replace mock body with all four required fields |
| 6b | `critpt_evaluate_omits_null_batch_metadata` | `Result<serde_json::Value, _>`, mock body has wrong schema | **Keep `serde_json::Value`** — test is intentionally about **request-side** absent-field serialization; verifying response decode is not the point. **Still fix the mock body** so it does not lie about the contract (use the same valid `CritPtEvaluateResponse` JSON as 6a). Add an inline comment. |

### Concrete changes

1. Open `schematic/schema/tests/artificial_analysis_client.rs`.

2. At the top of the file, extend the `use schematic_schema::artificial_analysis::{...}` import list to include the response types now needed in test bodies:

   ```rust
   use schematic_schema::artificial_analysis::{
       ArtificialAnalysisCritPt, ArtificialAnalysisCritPtRequest, ArtificialAnalysisData,
       ArtificialAnalysisDataRequest, CritPtEvaluateBody, CritPtEvaluateResponse, CritPtMessage,
       CritPtSubmission, EvaluateCritPtRequest, LlmModelsResponse, ListLlmModelsRequest,
       ListTextToImageModelsRequest, MediaModelsResponse,
   };
   ```

   Verify the names exist in the generated module by grepping
   `schematic/schema/src/artificial_analysis.rs` for each of `LlmModelsResponse`,
   `MediaModelsResponse`, `CritPtEvaluateResponse`. They are re-exported from
   the same `pub use types::*` path the existing `CritPtEvaluateBody` import
   uses.

3. Add two small private helpers above test 1 to keep mock bodies DRY and
   in-sync with the spec:

   ```rust
   /// Minimal valid `LlmModelsResponse` JSON: empty `data`, populated
   /// `prompt_options`. Required fields: `status`, `prompt_options`, `data`.
   fn llm_models_response_empty() -> serde_json::Value {
       serde_json::json!({
           "status": 200,
           "prompt_options": {
               "parallel_queries": 1,
               "prompt_length": "medium"
           },
           "data": []
       })
   }

   /// Minimal valid `MediaModelsResponse` JSON: empty `data`. The optional
   /// `include_categories` field is included only when the caller passes it.
   fn media_models_response_empty(include_categories: Option<bool>) -> serde_json::Value {
       let mut value = serde_json::json!({ "status": 200, "data": [] });
       if let Some(include) = include_categories {
           value
               .as_object_mut()
               .expect("object")
               .insert("include_categories".to_string(), serde_json::Value::Bool(include));
       }
       value
   }

   /// Minimal valid `CritPtEvaluateResponse` JSON. Required fields: `accuracy`,
   /// `timeout_rate`, `server_timeout_count`, `judge_error_count`.
   fn critpt_evaluate_response_zero() -> serde_json::Value {
       serde_json::json!({
           "accuracy": 0.0,
           "timeout_rate": 0.0,
           "server_timeout_count": 0,
           "judge_error_count": 0
       })
   }
   ```

   These helpers are the single point at which mock bodies stay aligned with
   the declared structs — if a future spec change adds a required field, only
   one place needs updating.

4. **Test 1 (`list_text_to_image_models_emits_include_categories_query`)**:
    - Replace the mock `set_body_json(...)` argument with
      `media_models_response_empty(Some(true))`.
    - Change the result type:
      ```rust
      let result: Result<MediaModelsResponse, _> = client.request(request).await;
      ```
    - After `assert!(result.is_ok(), ...)`, additionally assert on the
      decoded value:
      ```rust
      let body = result.expect("ok");
      assert_eq!(body.status, 200);
      assert_eq!(body.include_categories, Some(true));
      assert!(body.data.is_empty());
      ```

5. **Test 2 (`list_llm_models_uses_no_query_string`)**:
    - Replace the mock body with `llm_models_response_empty()`.
    - Change the result type:
      ```rust
      let result: Result<LlmModelsResponse, _> = client.request(request).await;
      ```
    - After `assert!(result.is_ok(), ...)`:
      ```rust
      let body = result.expect("ok");
      assert_eq!(body.status, 200);
      assert_eq!(body.prompt_options.parallel_queries, 1);
      assert_eq!(body.prompt_options.prompt_length, "medium");
      assert!(body.data.is_empty());
      ```

6. **Test 3 (`missing_auth_returns_authentication_required_error`)**:
    - Leave `Result<serde_json::Value, _>` as-is. Add an inline comment:
      ```rust
      // Intentionally typed as `serde_json::Value`: this test exercises the
      // pre-flight `AuthenticationRequired` error path, where no HTTP request
      // is made and no response body is decoded. Typed-response decoding is
      // covered by other tests in this file.
      ```

7. **Test 4 (`explicit_api_key_overrides_env_fallback`)** and
   **Test 5 (`env_api_key_is_used_when_no_explicit_key`)**:
    - Replace mock bodies with `llm_models_response_empty()`.
    - Change the result type to `Result<LlmModelsResponse, _>`.
    - After `assert!(result.is_ok(), ...)`, add:
      ```rust
      let body = result.expect("ok");
      assert_eq!(body.status, 200);
      ```
      (The minimal extra typed assertion proves decoding succeeded; the
      header-injection assertion is already enforced by the `header(...)`
      mock matcher.)

8. **Test 6a (`critpt_evaluate_serializes_populated_batch_metadata`)**:
    - Replace the mock body with `critpt_evaluate_response_zero()`.
    - Change the result type to `Result<CritPtEvaluateResponse, _>`.
    - After `assert!(result.is_ok(), ...)`:
      ```rust
      let body = result.expect("ok");
      assert_eq!(body.accuracy, 0.0);
      assert_eq!(body.timeout_rate, 0.0);
      assert_eq!(body.server_timeout_count, 0);
      assert_eq!(body.judge_error_count, 0);
      ```

9. **Test 6b (`critpt_evaluate_omits_null_batch_metadata`)**:
    - Replace the mock body with `critpt_evaluate_response_zero()` so the
      response body actually conforms to the declared `CritPtEvaluateResponse`
      contract.
    - Leave `Result<serde_json::Value, _>` typing in place. Add an inline
      comment:
      ```rust
      // Intentionally typed as `serde_json::Value`: this test asserts on the
      // **request body** the client emits (specifically that
      // `batch_metadata` is omitted when null). Typed-response decoding is
      // covered by `critpt_evaluate_serializes_populated_batch_metadata`.
      ```

### Tests to add in this phase

None new — Phase 1 is a tightening of the existing six tests' contracts. New
tests come in Phase 2.

### Acceptance Criteria

- Tests 1, 2, 4, 5, 6a deserialize into their spec-declared response structs.
- Tests 3 and 6b retain `serde_json::Value` typing and carry an inline
  comment justifying it.
- Every mock success payload is a valid instance of the response struct that
  endpoint declares (i.e. would round-trip through
  `serde_json::from_value::<T>(...)` cleanly), enforced by helper functions
  defined once at the top of the file.
- No regression in test count (still six tests after Phase 1).

### Verification

```bash
cargo test --manifest-path schematic/schema/Cargo.toml \
    --test artificial_analysis_client
```

All six tests must pass. If any decode fails, the helper body is wrong —
re-check against `types.rs`.

### Risks

- **Hidden type drift.** If the generated client renames any of
  `LlmModelsResponse` / `MediaModelsResponse` / `CritPtEvaluateResponse` (e.g.
  via `request_suffix`), the import block will fail to compile. Resolution:
  inspect `schema/src/artificial_analysis.rs` and use the actual generated
  names; do **not** edit the generated file, and do **not** rename in
  `types.rs` to compensate. If a real generator bug is found, escalate as a
  follow-up.
- **`request::<T>()` generic surface.** Phase 1 assumes the existing
  `client.request(request).await` call site already infers the generic from
  the `Result` binding. If the generated `request` method instead takes a
  turbofish only at call-time (e.g. `client.request::<T, _>(...)`), match the
  generated signature exactly. Confirm by grepping
  `schema/src/artificial_analysis.rs` for `pub async fn request`.

---

## Phase 2 — Broaden Typed-Response Coverage

**Goal:** The review's suggested fix is "deserialize at least one
representative request **per response shape** as its concrete type." Phase 1
covers all three shapes via the existing six tests. Phase 2 hardens this with
a small number of additional tests that exercise **populated** typed
deserialization (non-empty data arrays, optional-field handling), so a future
type-shape regression on a populated payload is caught.

### File touched

- `schematic/schema/tests/artificial_analysis_client.rs`

### Tests to add

1. **`list_llm_models_decodes_populated_response`**
    - Mocks `GET /data/llms/models` with a body containing **one** populated
      `LlmModel`. The fixture lives in a new helper
      `llm_models_response_one_populated()` that includes every required
      sub-field of `LlmModel` (see *Reference: Required Response-Struct
      Fields*).
    - Types the result as `Result<LlmModelsResponse, _>`.
    - Asserts: `body.data.len() == 1`; the `LlmModel`'s `id`, `name`,
      `slug`, `model_creator.id`, `pricing.price_1m_input_tokens`, and
      `evaluations.artificial_analysis_intelligence_index` round-trip.

2. **`list_text_to_image_models_decodes_populated_categories`**
    - Mocks `GET /data/media/text-to-image?include_categories=true` with a
      body whose single `MediaModel` carries a non-empty `categories` array
      (one `CategoryBreakdown`, including the optional `format_category`).
    - Types the result as `Result<MediaModelsResponse, _>`.
    - Asserts: `body.include_categories == Some(true)`; `body.data[0].categories`
      is `Some(vec)` with one entry; that entry's `format_category` is
      `Some("portrait".to_string())`.

3. **`list_image_editing_models_decodes_response_without_optional_fields`**
    - Mocks `GET /data/media/image-editing` with a body whose single
      `MediaModel` **omits** the optional `release_date` and `categories`
      fields entirely (i.e. the JSON keys are absent).
    - Types the result as `Result<MediaModelsResponse, _>`.
    - Asserts: the decoded `MediaModel` has `release_date == None` and
      `categories == None`. This proves `Option<...>` handling on the wire,
      complementing the populated case in test 2.

4. **`critpt_evaluate_decodes_realistic_response`**
    - Mocks `POST /critpt/evaluate` with a body using realistic non-zero
      values (e.g. `accuracy: 0.42, timeout_rate: 0.10, server_timeout_count:
      3, judge_error_count: 1`).
    - Types the result as `Result<CritPtEvaluateResponse, _>`.
    - Asserts every field round-trips. Complements Phase 1 test 6a (which
      uses an all-zero payload) by proving non-zero numeric decoding.

Each new test follows the same `#[tokio::test] #[serial_test::serial]` and
`set_env("test-key") / clear_env()` pattern as the existing tests. Helpers
added in Phase 1 are reused for empty fixtures; new populated fixtures live in
new private helper functions next to them.

### Acceptance Criteria

- Four new tests added; final test count in
  `schematic/schema/tests/artificial_analysis_client.rs` is **ten**.
- Each new test deserializes into the spec-declared response struct and
  asserts on at least one decoded field beyond `status`.
- Optional-field handling (`Option<String>`, `Option<Vec<...>>`) is exercised
  in both directions: populated (test 2) and absent (test 3).

### Verification

```bash
cargo test --manifest-path schematic/schema/Cargo.toml \
    --test artificial_analysis_client
```

All ten tests must pass.

### Risks

- **`#[serial_test::serial]` cost.** Phase 2 adds four serially-executed
  tokio tests. Total wall-clock for this test file will go up; this is
  acceptable because env-var safety requires the serialization. Do not
  attempt to remove `#[serial]` to recover parallelism.
- **Fixture drift.** If `types.rs` ever adds a required field, every helper
  in this file must be updated. Mitigation: the helpers are concentrated at
  the top of the file and named explicitly so a `git grep "pub struct
  LlmModel"` change is straightforward to chase down.

### Dependency note

Phase 2 depends on the helper functions defined in Phase 1. Run Phase 1 to
green first; otherwise the new tests will duplicate fixture code.

---

## Phase 3 — Run All Schematic Tests Green

**Goal:** Confirm Phase 1-2 changes leave the wider schematic test suite
passing — no regressions in definitions, gen, define, or oauth, and the schema
crate still compiles and tests cleanly.

### Steps

1. Run the workspace-side schematic tests:

    ```bash
    cargo test -p schematic-define
    cargo test -p schematic-definitions
    cargo test -p schematic-gen
    cargo test -p schematic-oauth
    ```

2. Run the excluded schema crate's full test suite:

    ```bash
    cargo test --manifest-path schematic/schema/Cargo.toml
    ```

3. Re-run the focused review-1 tests to confirm prior remediation still holds:

    ```bash
    cargo test -p schematic-gen --test artifact_drift \
        artificial_analysis_attribution_present_in_all_artifacts
    cargo test -p schematic-definitions \
        registry::tests::registry_key_for_known_apis_matches_table
    cargo test -p schematic-definitions artificial_analysis
    ```

4. If any test fails:

    - **`artificial_analysis_client.rs` tests** — fixture body does not
      satisfy the declared response struct. Compare the JSON to
      `types.rs` field-by-field.
    - **`artifact_drift` failure** — a checked-in artifact has drifted; this
      plan does not modify generated artifacts, so an unexpected drift
      indicates contamination from another change. Investigate before
      regenerating.
    - **Any other failure** — real regression. Bisect against the
      pre-change tip if needed.

### Acceptance Criteria

- All five `cargo test` invocations exit `0`.
- No tests are skipped or marked `ignored` as a workaround.
- The `artificial_analysis_client` test count is exactly **ten**.

### Verification

The verification commands above are themselves the acceptance check. Phase 3
is complete when all of them pass in a clean re-run.

### Dependency note

Phase 3 depends on Phases 1 and 2. Do not run Phase 3 first as a
sanity-check — it will pass against the unmodified tree because the existing
tests are passing-but-unverified-typed (which is exactly what review-2.md
flags).

---

## Phase 4 — Clippy Clean Across the Schematic Package Area

**Goal:** Zero clippy warnings or errors in any of the schematic crates.

### Steps

1. Run clippy on every workspace-included schematic crate:

    ```bash
    cargo clippy --all-targets -p schematic-define -- -D warnings
    cargo clippy --all-targets -p schematic-definitions -- -D warnings
    cargo clippy --all-targets -p schematic-gen -- -D warnings
    cargo clippy --all-targets -p schematic-oauth -- -D warnings
    ```

2. Run clippy on the excluded schema crate:

    ```bash
    cargo clippy --all-targets \
        --manifest-path schematic/schema/Cargo.toml \
        -- -D warnings
    ```

3. **For every reported warning, fix the underlying code, not the lint level.**
    - If the warning lands in **generated** code
      (`schema/src/artificial_analysis.rs` or any other generated module),
      fix the **generator** in `schematic/gen/src/codegen/...` and
      regenerate. Do **not** hand-edit the generated file. Adding
      `#![allow(...)]` is acceptable **only** if the same allow already
      appears in other generated modules as a uniformly-applied convention.
    - If the warning lands in the test file from Phases 1-2, prefer fixing
      the test's code style. Adding new top-of-file `#![allow(...)]`
      annotations beyond the two already present
      (`needless_borrows_for_generic_args`, `field_reassign_with_default`)
      requires a justification comment.
    - Likely candidates worth pre-empting in the new tests:
        - `clippy::needless_pass_by_value` on helper closures
        - `clippy::float_cmp` on `assert_eq!(body.accuracy, 0.42)`
          (use `assert!((body.accuracy - 0.42).abs() < f64::EPSILON, ...)`
          if clippy complains, or pick a value that is exactly
          representable, e.g. `0.5`)
        - `clippy::too_many_lines` on the test file overall (no fix —
          allow only if pre-existing convention)

4. Run formatters to keep style consistent after any fixups:

    ```bash
    cargo fmt --all
    cargo fmt --manifest-path schematic/schema/Cargo.toml
    ```

### Acceptance Criteria

- All five clippy invocations exit `0` with `-D warnings`.
- No new `#[allow(...)]` or `#![allow(...)]` annotations introduced unless
  matching a pre-existing project-wide convention. Each new allow (if any)
  has a comment explaining why.

### Verification

```bash
cargo clippy --all-targets -p schematic-define -- -D warnings
cargo clippy --all-targets -p schematic-definitions -- -D warnings
cargo clippy --all-targets -p schematic-gen -- -D warnings
cargo clippy --all-targets -p schematic-oauth -- -D warnings
cargo clippy --all-targets \
    --manifest-path schematic/schema/Cargo.toml \
    -- -D warnings
```

All five must succeed. The phase is complete when clippy reports zero
warnings and zero errors across the whole schematic area.

### Dependency note

Phase 4 depends on Phases 1-3 (test code must exist and compile before
clippy can lint it). Run last.

---

## Final Goal-Backward Verification

The remediation is complete when **all** of the following hold simultaneously:

1. `cargo test --manifest-path schematic/schema/Cargo.toml --test artificial_analysis_client`
   passes with **ten** tests, of which at least:
    - **Three** typed `LlmModelsResponse` decodes (one empty, one populated,
      plus the auth-fallback variants which also type-check the response).
    - **Three** typed `MediaModelsResponse` decodes (the `include_categories`
      query test; the populated-categories test; the absent-optional-fields
      test).
    - **Two** typed `CritPtEvaluateResponse` decodes (the populated
      `batch_metadata` request test; the realistic non-zero response test).
    - **Two** intentional `serde_json::Value` tests
      (`missing_auth_returns_authentication_required_error` and
      `critpt_evaluate_omits_null_batch_metadata`), each with an inline
      comment explaining why typed decoding is not the point.

2. Every mock success payload across the file is a valid instance of the
   declared response struct (i.e. round-trips through
   `serde_json::from_value::<T>(...)` cleanly).

3. `cargo test -p schematic-define -p schematic-definitions -p schematic-gen -p schematic-oauth`
   passes (split into separate invocations if combined filtering blocks
   the run, per review-1's Verification Run note).

4. `cargo test --manifest-path schematic/schema/Cargo.toml` passes.

5. `cargo clippy --all-targets -- -D warnings` is clean for every crate listed
   in Phase 4.

6. `git diff --stat schematic/` shows changes in **only**:
    - `schematic/schema/tests/artificial_analysis_client.rs` (the test
      tightening from Phases 1-2).
    - **Optional:** any generator/codegen fix in `schematic/gen/src/codegen/...`
      strictly required to keep clippy clean (Phase 4). No regenerated
      artifacts unless a generator change triggered regeneration.
   No unrelated drift.

7. The two HIGH findings from review-1.md remain green
   (attribution regression test and provider-specific generated-client
   integration coverage), per Phase 3 step 3.

## Risks and Open Questions

- **`request::<T>()` generic inference.** If the generated `request` method
  signature is `pub async fn request<R: DeserializeOwned>(&self, req: ...) -> Result<R, SchematicError>`,
  the existing `Result<serde_json::Value, _>` binding will continue to work
  and the new typed bindings will too. If the signature is exotic
  (e.g. takes a turbofish on a different generic parameter), Phases 1-2 will
  need a small wording adjustment. **Confirm by grepping**
  `schema/src/artificial_analysis.rs` for `pub async fn request` before
  starting Phase 1.

- **Generator drift outside this remediation.** The generator is volatile —
  if `schema/src/artificial_analysis.rs` is regenerated mid-flight by a
  parallel change, the import block in the test file may need to track
  renames. Hold the test file's imports authoritative against the **current**
  generated names, not those documented in spec.md.

- **Float assertions and clippy.** `clippy::float_cmp` may fire on
  `assert_eq!(body.accuracy, 0.42)`-style assertions in
  `critpt_evaluate_decodes_realistic_response`. Mitigate by either picking
  values that are exactly representable in `f64` (e.g. `0.5`, `0.25`,
  `0.125`) or by switching to an epsilon-based assertion. The plan prefers
  the former because it is cheaper and reads more clearly.

- **Helper-function drift.** Concentrating mock bodies in helper functions
  is a deliberate trade-off: the helpers must be kept in sync with
  `types.rs` if the spec changes. The alternative (inline JSON in every
  test) is worse — it scatters the same drift risk across ten sites instead
  of three. Accept the trade-off and prefer helpers.

- **Out of scope.** This remediation does **not**:
    - Add any new endpoint coverage beyond the four endpoints already
      exercised by the existing tests (LLM models, text-to-image,
      image-editing for the new optional-field test, CritPt).
    - Modify generated artifacts.
    - Touch `schematic/definitions/src/artificial_analysis/` source.
    - Add a first-class `attribution: Option<String>` field on `RestApi`
      (still out of scope — review-2 explicitly considered the description
      string acceptable in review-1's Coverage Notes).
