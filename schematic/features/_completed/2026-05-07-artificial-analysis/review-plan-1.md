---
review: schematic/features/2026-05-07-artificial-analysis/review-1.md
spec: schematic/features/2026-05-07-artificial-analysis/spec.md
original_plan: schematic/features/2026-05-07-artificial-analysis/plan.md
created: 2026-05-07
phases: 5
packages:
  - schematic-definitions
  - schematic-gen
  - schematic-schema
artifacts:
  - schematic/openapi/artificial_analysis.json
  - schematic/postman/artificial_analysis.postman_collection.json
  - schematic/schema/src/artificial_analysis.rs
  - schematic/schema/tests/artificial_analysis_client.rs
---

# Review 1 — Remediation Plan

## Goal

Close the two HIGH findings from `review-1.md` and ship a clean diff:

1. **Attribution to <https://artificialanalysis.ai/> must surface in every public artifact**
   that downstream consumers can see — the generated rustdoc on
   `schematic/schema/src/artificial_analysis.rs`, the exported
   `schematic/openapi/artificial_analysis.json`, and the
   `schematic/postman/artificial_analysis.postman_collection.json` collection — not
   just the source-only `//!` comment in `schematic/definitions/src/artificial_analysis/mod.rs`.

2. **Provider-specific generated-client integration coverage** must exist under
   `schematic/schema/tests/`, exercising the actual HTTP behaviour of both generated
   clients (`ArtificialAnalysisData` and `ArtificialAnalysisCritPt`) against a
   `wiremock` mock server — mirroring the shape of
   `schematic/schema/tests/huggingface_client.rs`.

In addition, the diff must leave the schematic package areas:

- **All tests passing.**
- **Zero `cargo clippy` warnings or errors** in `schematic-define`, `schematic-definitions`,
  `schematic-gen`, `schematic-oauth`, and the excluded `schematic-schema` crate.

The plan is structured as five phases. Phases 1-3 address the review findings.
Phases 4-5 enforce the global passing-tests and lint-clean acceptance criteria.

## Working Constraints

- Working directory: `/Users/ken/.claudine/worktrees/rusty-biscuit/schematic` (the
  `schematic` worktree of the `rusty-biscuit` monorepo).
- `schematic/schema` is **excluded from the workspace**. Always use
  `--manifest-path schematic/schema/Cargo.toml` for `cargo` commands targeting it.
- `schematic-gen` is the only path that may write to `schematic/schema/src/*.rs`,
  `schematic/openapi/*.json`, and `schematic/postman/*.postman_collection.json`. Do
  not hand-edit those files. If a regeneration produces unexpected drift, fix the
  upstream definitions or generator code, then re-run.
- Follow the rustdoc convention from `CLAUDE.md`: no `# H1` inside `///` blocks, use
  `## H2` for `Examples` / `Returns` / `Errors` / etc., and the documented section
  ordering.

## Phase 1 — Make Attribution Surface in Generated Artifacts

**Goal:** Every consumer of any generated artifact sees the attribution requirement.

### Strategy

Out of the two paths suggested by the review, pick the **already-supported** one:
embed the attribution sentence directly into the `description` field of **both**
`define_artificial_analysis_data_api()` and `define_artificial_analysis_critpt_api()`.
This requires zero generator changes and propagates cleanly through:

- The Rust `//!` rustdoc on `schema/src/artificial_analysis.rs` (the generator
  emits each `RestApi.description` as part of the grouped module doc header).
- The OpenAPI `info.description` in `schematic/openapi/artificial_analysis.json`
  (the grouped exporter uses the first API's description as the document
  description, so attribution will appear at the top).
- The Postman `info.description` in
  `schematic/postman/artificial_analysis.postman_collection.json` (the grouped
  Postman exporter concatenates per-API descriptions).

A separate "add attribution as a first-class API field" change is **out of scope**
for this remediation — the description-field path is sufficient to satisfy the
terms-driven requirement and avoids cross-cutting generator surgery.

### Steps

1. Edit `schematic/definitions/src/artificial_analysis/mod.rs`:
    - Update the `description` for `define_artificial_analysis_data_api()` to:
      `"Artificial Analysis free data API: LLM and media-model benchmark catalogs. Attribution: as required by the Artificial Analysis API terms, all usage must include attribution to https://artificialanalysis.ai/."`
    - Update the `description` for `define_artificial_analysis_critpt_api()` to:
      `"Artificial Analysis CritPt benchmark: submit code-generation results for evaluation. Attribution: as required by the Artificial Analysis API terms, all usage must include attribution to https://artificialanalysis.ai/."`
    - Keep the existing module-level `//!` doc block (with its `## Attribution`
      section) unchanged. The two paths reinforce each other.

2. Regenerate the typed schema crate for both APIs:

    ```bash
    cargo run -p schematic-gen -- \
        --api artificial-analysis-data \
        --output schematic/schema/src
    cargo run -p schematic-gen -- \
        --api artificial-analysis-critpt \
        --output schematic/schema/src
    ```

3. Regenerate the OpenAPI grouped artifact:

    ```bash
    cargo run -p schematic-gen -- \
        openapi-export-grouped \
        --module artificial_analysis \
        --output schematic/openapi
    ```

    If the CLI surface differs, fall back to the project's standard regeneration
    recipe (check `schematic/gen/justfile` and the area `justfile` for
    `gen-openapi` / `gen-postman` / `regen-all` recipes — use whichever the
    repo currently exposes for grouped exports).

4. Regenerate the Postman grouped artifact (same notes as step 3).

5. Manually inspect the three regenerated artifacts and confirm the attribution
   sentence is present at the top of each:

    - `schematic/schema/src/artificial_analysis.rs` — top `//!` block.
    - `schematic/openapi/artificial_analysis.json` — `info.description`.
    - `schematic/postman/artificial_analysis.postman_collection.json` — `info.description`.

### Acceptance Criteria

- All three artifacts contain the literal substring `https://artificialanalysis.ai/`.
- The committed source change is limited to two `description` strings in
  `schematic/definitions/src/artificial_analysis/mod.rs` plus the regenerated
  artifacts.

### Verification

```bash
# Source compiles after description edits.
cargo check -p schematic-definitions

# Schema crate still compiles after regeneration.
cargo check --manifest-path schematic/schema/Cargo.toml

# Attribution presence sanity checks.
grep -F "https://artificialanalysis.ai/" \
    schematic/schema/src/artificial_analysis.rs
grep -F "https://artificialanalysis.ai/" \
    schematic/openapi/artificial_analysis.json
grep -F "https://artificialanalysis.ai/" \
    schematic/postman/artificial_analysis.postman_collection.json
```

Each `grep` must match at least once. If any match is missing, do not proceed —
fix the relevant generator path or regenerate the missing artifact before moving
on.

---

## Phase 2 — Add an Attribution Regression Test

**Goal:** Prevent silent attribution loss in any future regeneration.

### Strategy

The review explicitly calls for an "artifact/content regression test." Place it in
`schematic-gen` because the `gen/tests/artifact_drift.rs` test infrastructure
already does exactly this for other artifacts. Add a focused test that asserts
the literal attribution URL appears in all three generated Artificial Analysis
artifacts.

### Steps

1. Open `schematic/gen/tests/artifact_drift.rs` (or create
   `schematic/gen/tests/artificial_analysis_attribution.rs` if dropping a single
   test alongside an unrelated file is awkward — match local convention).

2. Add a Rust integration test that:

    - Reads each artifact path as a `&str` via `std::fs::read_to_string`.
    - Asserts each contains `"https://artificialanalysis.ai/"`.
    - Uses descriptive `assert!` messages so a failure points at the artifact
      and at this remediation requirement.

   Sketch:

    ```rust
    /// Regression: the Artificial Analysis API terms require that all usage
    /// include attribution to <https://artificialanalysis.ai/>. The generated
    /// rustdoc, OpenAPI grouped document, and Postman collection MUST surface
    /// that attribution so downstream consumers see it.
    #[test]
    fn artificial_analysis_attribution_present_in_all_artifacts() {
        for (label, path) in [
            (
                "schema rustdoc",
                "../schema/src/artificial_analysis.rs",
            ),
            (
                "OpenAPI grouped doc",
                "../openapi/artificial_analysis.json",
            ),
            (
                "Postman grouped collection",
                "../postman/artificial_analysis.postman_collection.json",
            ),
        ] {
            let contents = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("read {label} at {path} failed: {e}"));
            assert!(
                contents.contains("https://artificialanalysis.ai/"),
                "{label} ({path}) is missing the required Artificial Analysis \
                 attribution. Regenerate the artifact and confirm the API \
                 description carries the attribution sentence."
            );
        }
    }
    ```

   **Path note:** confirm the relative path resolution used by other tests in
   `artifact_drift.rs` (some use `CARGO_MANIFEST_DIR`-anchored paths). Match the
   local pattern exactly to avoid CI surprises.

### Acceptance Criteria

- A single dedicated test asserts attribution presence in all three artifacts.
- The test fails clearly (with the artifact name) if any one is missing the
  attribution sentence.

### Verification

```bash
cargo test -p schematic-gen artificial_analysis_attribution_present_in_all_artifacts
```

Must pass.

---

## Phase 3 — Add Provider-Specific Generated-Client Integration Tests

**Goal:** Cover the user-observable HTTP behaviour of both generated clients
against a mock HTTP server, closing the second HIGH review finding.

### Strategy

Create `schematic/schema/tests/artificial_analysis_client.rs`, modelled closely
after the existing `schematic/schema/tests/huggingface_client.rs`. Use
`wiremock` (already a `dev-dependency` of `schematic-schema` — verify with
`grep wiremock schematic/schema/Cargo.toml`; if absent, add it to
`[dev-dependencies]` together with `tokio` features needed for `#[tokio::test]`).

### Required Test Coverage

The following cases are required by the review and constitute the acceptance
criteria for this phase:

1. **Data API — endpoint with `include_categories=true`**
    Use `ListTextToImageModelsRequest::new().with_include_categories(true)`.
    Mock `GET /data/media/text-to-image?include_categories=true`. Assert the
    generated client hits that exact path-and-query.

2. **Data API — endpoint without query params**
    Use `ListLlmModelsRequest::default()`.
    Mock `GET /data/llms/models`. Assert path is hit with no query string.

3. **Data API — missing-auth error behaviour**
    Ensure `ARTIFICIAL_ANALYSIS_API_KEY` is **unset** (use
    `std::env::remove_var` inside an `unsafe` block, matching the Hugging Face
    test's `setup_test_env` style). Construct a client without `.api_key(...)`.
    Drive a request and assert it fails with the appropriate
    `SchematicError::*` variant emitted by the generator when no API key is
    available. (Inspect the actual generated `api_key_header()` /
    `request(...)` paths to determine the precise error variant — pick the
    one the generator currently produces; do not invent one.)

4. **Data API — explicit `.api_key(...)` injection**
    Set `ARTIFICIAL_ANALYSIS_API_KEY=""` (empty) and instead call
    `.api_key("explicit-key")` on the client builder. Mock the endpoint with
    `header("x-api-key", "explicit-key")` matcher and assert success. This
    proves explicit override beats env fallback.

5. **Data API — env fallback**
    Set `ARTIFICIAL_ANALYSIS_API_KEY=env-key`. Do **not** call `.api_key(...)`.
    Mock the endpoint with `header("x-api-key", "env-key")` matcher and
    assert success.

6. **CritPt API — body serialization**
    Build a `CritPtEvaluateBody` with:
        - One `CritPtSubmission` populated with a non-trivial
          `generation_config` (`serde_json::json!({"temperature": 0.3, "top_p": 0.9})`)
          and a `messages` vec of one `CritPtMessage`.
        - A non-null `batch_metadata` (e.g. `serde_json::json!({"run_id": "abc"})`).
    Mock `POST /critpt/evaluate` with `wiremock::matchers::body_json(...)` to
    assert the exact serialized payload. Confirm the
    `skip_serializing_if = "serde_json::Value::is_null"` rule is respected by
    also adding a second test where `batch_metadata` is `Value::Null` and the
    mock asserts the field is absent from the wire body (use
    `body_partial_json` or a custom matcher — match the Hugging Face test's
    style).

### Steps

1. Verify `wiremock` and `tokio` (with `macros` and `rt-multi-thread`) are
   listed under `[dev-dependencies]` of `schematic/schema/Cargo.toml`. If any
   are missing, add them with the same versions used by other schema tests
   (check `schema/Cargo.toml` first; do **not** introduce a new minor
   version).

2. Create `schematic/schema/tests/artificial_analysis_client.rs` with the
   six tests described above. Re-use a `setup_test_env` style helper for
   environment management; ensure cleanup so tests do not leak env state.

3. Add the same `#![allow(...)]` lints at the top as `huggingface_client.rs`
   if needed to keep clippy clean for generator-style ergonomic patterns
   (`needless_borrows_for_generic_args`, `field_reassign_with_default`).
   Justify any additional `#![allow(...)]` with a comment.

4. Use the typed `ArtificialAnalysisData::with_base_url(&mock_server.uri())`
   and `ArtificialAnalysisCritPt::with_base_url(&mock_server.uri())`
   constructors — these exist in the generated client (verified on the
   current artifact: `schema/src/artificial_analysis.rs` lines 481 and 1441).

5. Use the typed request enums where they exist:
   `ArtificialAnalysisDataRequest::ListLlmModels(...)` etc., as the Hugging
   Face test does. The generated request enums are at
   `schema/src/artificial_analysis.rs` lines 323 and 1338.

### Acceptance Criteria

- The new test file compiles via `cargo check --manifest-path schematic/schema/Cargo.toml`.
- All six tests pass.
- The tests cover: query-string serialization with and without
  `include_categories`, missing-auth failure, explicit `.api_key(...)` override,
  env-var fallback, and CritPt JSON body serialization with both populated and
  null `batch_metadata`.
- The tests do **not** depend on network connectivity; everything is mocked
  via `wiremock`.

### Verification

```bash
cargo test --manifest-path schematic/schema/Cargo.toml \
    --test artificial_analysis_client
```

All tests in the new file must pass. Re-run with `--nocapture` if any failure
needs diagnosis.

---

## Phase 4 — Run All Schematic Tests Green

**Goal:** Confirm the changes from Phases 1-3 leave the wider schematic test
suite passing — no regressions in definitions, gen, define, or oauth, and the
schema crate still compiles and tests cleanly.

### Steps

1. Run the workspace-side schematic tests:

    ```bash
    cargo test -p schematic-define
    cargo test -p schematic-definitions
    cargo test -p schematic-gen
    cargo test -p schematic-oauth
    ```

2. Run the excluded schema crate tests:

    ```bash
    cargo test --manifest-path schematic/schema/Cargo.toml
    ```

3. If any test fails:

    - If it is **the artifact_drift test**, the failure indicates a stale
      checked-in artifact. Re-run regeneration commands from Phase 1 step 2-4
      and recommit the regenerated artifact.
    - If it is the new attribution test, double-check Phase 1 step 5 grep
      results — at least one artifact is missing the attribution sentence.
    - Any other failure is a real regression. Bisect against the pre-change
      tip if needed.

### Acceptance Criteria

- All five `cargo test` invocations exit `0`.
- No tests are skipped or marked `ignored` as a workaround.

### Verification

The `Phase 4 — Verification` block is the union of the test commands above.
The phase is complete when all of them pass in a clean re-run.

---

## Phase 5 — Clippy Clean Across the Schematic Package Area

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
    - If a warning lands in **generated** code (`schema/src/artificial_analysis.rs`
      or any other generated module), fix the **generator** in
      `schematic/gen/src/codegen/...` and regenerate. Do **not** hand-edit the
      generated file. Add `#![allow(...)]` to the generated module **only** if
      the same allow already appears in other generated modules (i.e., it is a
      pre-existing convention applied uniformly).
    - If the warning lands in the new test file from Phase 3, prefer fixing
      the test's code style. Add a top-of-file `#![allow(...)]` only if it
      mirrors the same lint allow used by `huggingface_client.rs`.
    - If the warning lands in `schematic-definitions` (e.g. unused imports
      from earlier wiring or a description string that triggers a clippy
      `doc_markdown` complaint about the URL), fix it directly.

4. Run `cargo fmt --all` and `cargo fmt --manifest-path schematic/schema/Cargo.toml`
   to keep formatting consistent — especially after any regenerated code.

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

All five must succeed. The phase is complete when clippy reports zero warnings
and zero errors across the whole schematic area.

---

## Final Goal-Backward Verification

The remediation is complete when all of the following hold simultaneously:

1. `grep -F "https://artificialanalysis.ai/" schematic/schema/src/artificial_analysis.rs schematic/openapi/artificial_analysis.json schematic/postman/artificial_analysis.postman_collection.json` matches in **all three** files.
2. `cargo test -p schematic-gen artificial_analysis_attribution_present_in_all_artifacts` passes.
3. `cargo test --manifest-path schematic/schema/Cargo.toml --test artificial_analysis_client` passes with **at least six** distinct test cases covering the matrix from Phase 3.
4. `cargo test -p schematic-define -p schematic-definitions -p schematic-gen -p schematic-oauth` passes (split into separate invocations if combined filtering blocks the run, per the review's Verification Run note).
5. `cargo test --manifest-path schematic/schema/Cargo.toml` passes.
6. `cargo clippy --all-targets -- -D warnings` is clean for every crate listed in Phase 5.
7. `git diff --stat schematic/` shows changes in: the two definition descriptions, the regenerated artifacts, the new attribution test, the new client integration test, and any generator/codegen fixes required to keep the artifacts attribution-bearing and clippy-clean. **No unrelated drift.**

## Risks and Open Questions

- **Description-field path may be too coarse.** If the project later prefers a
  first-class `attribution: Option<String>` field on `RestApi`, this remediation
  becomes a stepping stone rather than the final design. That refactor is
  explicitly out of scope here — the review accepts either path and this one is
  cheaper.
- **`schematic-gen` regenerate CLI surface.** The exact `cargo run -p schematic-gen --` flags used in Phase 1 may differ from the current main subcommand layout
  (the original plan assumes `--api <key> --output <dir>`). If the CLI has
  evolved, defer to whatever recipe the repo currently exposes (`just regen-*`
  in `schematic/gen/justfile` or the area's root `justfile`). **Use the repo's
  canonical recipe and do not invent flags.**
- **Generator-emitted clippy warnings.** If clippy flags warnings that originate
  inside generated code, the fix lives in `schematic/gen/src/codegen/...`. That
  may expand the scope of Phase 5 beyond a pure remediation. If it grows large
  enough to merit its own review, split it into a follow-up rather than blocking
  this remediation — but do not silence the warning in generated output as a
  shortcut.
- **`wiremock` body matchers and absent-field assertions.** `wiremock` does not
  ship a dedicated "field-must-be-absent" matcher. The CritPt null-batch_metadata
  test will likely require a custom `Match` implementation or an
  inspect-after-the-fact assertion using a recorded request. Match the style
  used elsewhere in `schematic/schema/tests/`.
- **Env-var test isolation.** The `setup_test_env` pattern in
  `huggingface_client.rs` uses `unsafe { std::env::set_var(...) }`. Tests that
  manipulate `ARTIFICIAL_ANALYSIS_API_KEY` must run serialized or use a
  per-test env guard to avoid cross-test contamination. Confirm the local
  testing convention in the schema crate before assuming `#[tokio::test]` on
  parallel-by-default.
