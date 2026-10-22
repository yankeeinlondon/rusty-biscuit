---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-15-schematic-definition-vocabulary/spec.md"
plan: "schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md"
implemented_by: "codex/gpt-6.1-sol"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - schematic/features/2026-10-15-schematic-definition-vocabulary/plan.md
docs_created_during_phase_1:
  - schematic/features/2026-10-15-schematic-definition-vocabulary/implementation-log.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/contracts.md
  - schematic/features/2026-10-15-schematic-definition-vocabulary/inventory.md
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
  Phase 1 is complete on the reviewed working tree; no source files changed.
  TypeDesc privately stores TypeRef and a Rust path, with no serde/structural
  slot or context identity; ModelCatalog has no serde/context attachment.
  See contracts.md and inventory.md for actual interfaces, matrix, and callers.
  PD4 is adopted as a default: keep .auth(policy) and add an optional
  endpoint EnvMapping (present replaces the API mapping; absent reuses only
  compatible API sources). The spec's pre-authorizations are granted,
  including the breaking Basic/OAuth2/query scope. Bitbucket/EMQX use
  .authentication(AuthStrategy::Basic, ...), not basic_env; a payload Basic
  variant needs the R12-permitted mechanical authoring edits and an R10 flag.
  Strict bundle preflight must distinguish absent/null; raw RestApi snapshots
  currently contain null options, so bundle writing needs a matching contract.
  Preserve the pre-existing uncommitted definition-API repairs and original
  bare snapshots/generated trees. All area gates pass; four unchanged ordinary
  L1 tests exceeded five seconds under the full suite (details below).
---

# Implementation Log for 2026-10-15-schematic-definition-vocabulary (6 phases)

## Phase 1

### Scope, starting state, and decisions

Phase 1 is evidence and inventory only. No implementation behavior, parser, schema, template, shipped definition, generated artifact, dependency, or test is changed. Existing tests provide the baseline; the next phases' behavior-to-test mapping is recorded in `contracts.md` before implementation begins. No regression is being repaired by this phase, so no new red/green test is appropriate.

The implementation log existed as an empty file at entry. The worktree was already dirty with definition-API review repairs (20 tracked files and three untracked review/log files); these are preserved and excluded from this phase's source-file metadata. HEAD is `a8842e0f9935121543adf650a8ff1eaa5503d97b`; committed tree is `21a263cce41e8c8c16e6a57e6e4a3674b639997f`. The reproducible baseline is this HEAD plus those existing repairs, not HEAD alone. No commit, staging, push, live provider call, CI change, or lifecycle move is authorized or performed.

Adopt PD1–PD3 and PD5–PD10 as planning defaults: validated metadata only; ParamDef typed paths; absent deprecation defaults false; same-context structural registration; complete ordered Basic EnvLists; canonical version-1 JSON bundles; query styles and enum wire spellings; grant-specific OAuth payloads retaining Required/Supported PKCE; unconditional query migration. PD4 is provisional. No author ruling on independent endpoint credential sources was found in the supplied spec/plan or prerequisite review records. The prompt authorizes all independent Phase 1 preparation. It does not settle that public API choice or explicitly confirm the future breaking changes listed under pre-authorizations. Later implementation must resolve those conditions, without blocking this read-only inventory.

GitHub adoption remains limited to rate-limit header names, epoch-second resets, and optional Retry-After; no provider-wide retry statuses or attempts. Although `basic_env(username, passwords)` can keep its signature, shipped Bitbucket and EMQX do not use it: they call `.authentication(AuthStrategy::Basic, ...)` at definitions/src/bitbucket/mod.rs:218 and definitions/src/emqx/mod.rs:157. Turning Basic into a payload variant cannot preserve those exact expressions. Flag this R10/R12 conflict: later implementation needs the narrowly permitted mechanical Basic authoring migration (and existing pattern-match test repairs), with byte-identical generated output, rather than an endpoint rewrite. OpenAI authoring remains unchanged. No spike is needed.

### Prerequisites and contract refresh

- `2026-10-11-schematic-generation-integrity`: completed; review 3 says production ready. Its output-local diagnostics, isolated compiler harness, and artifact comparisons exist.
- `2026-10-12-schematic-runtime-envelope`: completed; review 3 is ready with no human review. `schematic-runtime` owns EndpointSpec, call futures, redaction, redirect/origin rules, and pager request execution.
- `2026-10-13-schematic-pagination`: completed; review 2 is ready. Current provider definitions retain 93 authored pagers. Adoption and sent-default data are available; direct page requests use `fetch_page()`.
- `2026-10-14-schematic-definition-api`: implementation is present and review 3 reports ready, with both earlier findings fixed. Repairs are uncommitted in this working tree. Its spec still has historical `status: human-in-the-loop`, but `implemented: true`, `completed: true`, and `human_review: false`; its handoff mentions author review. This is reviewed working-tree evidence, not a claim of a separate merged commit or author sign-off.
- Reconciled actual interfaces, input outcomes, and future test obligations are in `contracts.md`. The historical claim of a serde structural TypeDesc does not match landed code; the review explicitly accepts the stored TypeRef lowering. Do not reconstruct the prerequisite. Composition/serialization extensions belong in later phases.

### Verification results

- `cd schematic && just migration-check`: PASS, 33 tests. All 18 historical snapshots match; no exception is active. Rust sources plus manifest, OpenAPI, and Postman regenerate in isolated TempDirs and match committed bytes/file sets. Description mutation fails comparison at its exact description and fingerprint leaves. Existing repeated write/read/write/read test passes.
- Exact retained tests: `definition_baseline::changed_description_is_detected_at_its_exact_field`, `definition_baseline::shipped_definitions_round_trip`, `definition_baseline::baseline_catalog_matches_shipped_apis`, `definition_baseline::shipped_json_field_matrix_preserves_required_and_optional_shapes`, all 18 API baseline tests, and `artifact_drift::generated_{rust,openapi,postman}_artifacts_are_up_to_date`.
- New targeted tests: none (no behavior changes). Existing tests are compiled by schematic-gen's declared `l1` target and selected by L1. Filesystem/serde round trips and snapshot mutation are L1/F1; generation through actual writers and Rust compiler consumption is L1/F2 at those boundaries. No live provider F3 evidence is claimed.
- Full `cd schematic && just test`: PASS, 2,287 ordinary L1 tests plus seven fresh-output compiler/request checks and one runtime compile-fail guard (2,295 total). `cd schematic && just lint`: PASS for all five packages. `just check-tier-coverage schematic`: PASS, zero stranded tests. Gates ran once; the migration baseline was a separate focused prerequisite check. No dependent package has been changed by Phase 1; downstream execution is unnecessary for a documentation-only evidence phase. The inherited dirty tree means the whole-tree scope calculator cannot justify skipping the requested area gates.

### Available environments

Loaded the OS skill. Current host is macOS. `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`, and `BUILD_WSL=build-win` are configured; those rigs support later `just cross-check` runs. No connectivity or execution evidence is claimed. Phase 1 adds no OS-sensitive code, so no cross-OS execution is required here.

### Inventory and checkpoint closure

`inventory.md` records all 18 APIs (539 endpoints, 1,818 ordered schema rows), 404 query declarations, 25 required query declarations, 81 enum-bearing declarations, and 93 resolved pagers. The required/enum union spans 106 fields and 63 generated request types. Basic users are Bitbucket and EmqxBasic; no shipped legacy auth or explicit policy contains OAuth2. All affected request references and direct external consumer construction/default/setter/assignment sites are listed by file and line, along with exact manifest package names. `sniff repo packages --package-area schematic --json` confirms schematic-define, schematic-definitions, schematic-gen, schematic-oauth, schematic-runtime, and excluded schematic-schema. Manifests confirm sniff, biscuit-speaks, model-citizen, homelab, eversolo-integration, arcam-amp-integration, sony-receiver-integration, unfolded-integration-helper, and schematic-oauth as the expected external/consumer scope; the inventory includes internal direct edges too.

`contracts.md` supplies R1–R14 requirement-to-evidence mapping, the actual builder/registration/runtime/diagnostic interfaces, field-specific absent/null/empty outcomes, recursive validator/document-shape obligations, an inherited struct-field audit, and a narrow future normalization recipe. Original snapshot files, fingerprints, and tests remain unchanged. No future normalization is enabled before its vocabulary exists. Later bundles need a strict presence-aware wire contract: raw Option serde currently loses explicit-null presence, and raw snapshot nulls must not be mistaken for a valid future bundle preflight contract.

Both Phase 1 plan checkboxes are complete. The baseline/checkpoint is verified on the reviewed working tree; no claim of a separate prerequisite merge or author sign-off is made. The preparation reconciliation is documenting actual TypeDesc/ModelCatalog interfaces and the necessary Basic source-form conflict instead of assuming the draft's proposed carrier/call forms already exist. No source files or skill files were changed by this phase; current runtime and authoring behavior remains unchanged, so no public README/topic/skill update is needed. Only plan.md, this log, contracts.md, and inventory.md were written. The spec is unchanged because no spec frontmatter keys were requested.

### Gate exclusions and timing observations

The full area gate intentionally filters other tiers: ordinary L1 reported 13 exclusions; the separate fresh-output pass reported 235 filtered tests and selected seven; the compile-fail pass reported 342 filtered tests and selected one. These are selection counts, not failed/skipped selected cases. No selected test failed, timed out, or was ignored. No L2/L3/browser/real service test was needed for an evidence-only change. No new or renamed test needs target registration or feature wiring. Dependent packages were inventoried but not executed: no package implementation, generated API, or source test changed in this phase.

Four existing ordinary L1 tests exceeded five seconds under the full gate: `shipped_openai_query_schema_gate_and_reimports` (7.497 s), `artifact_drift::generated_rust_artifacts_are_up_to_date` (6.844 s), `fresh_output_compile::generated_rest_code_references_the_runtime_and_defines_none_of_it` (6.829 s), and `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` (6.955 s). They passed; they remain pre-existing test-budget violations, not newly authored tests or evidence of a fixed timing defect. The artifact drift test took 4.576 s in the earlier focused baseline run. The recipe's explicitly separated Cargo compiler checks also exceed five seconds; their existing tier/placement was preserved. No retries, broadened reruns, or timeout changes were made.

Raw command logs for this run: `/tmp/schematic-vocabulary-phase1-test.log`, `/tmp/schematic-vocabulary-phase1-lint.log`, and `/tmp/schematic-vocabulary-phase1-tiers.log`. No formatter was invoked. Last completed phase: 1. Replacement-policy implementation remains blocked on the recorded credential-source decision; future breaking scope confirmations remain unrecorded and must not be inferred from this preparation-only run.

### Final scope and metadata verification

After saving phase documentation and metadata, ran the required root command:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `change_class: package`; `packages: [schematic-define, schematic-definitions, schematic-gen, test-toolkit, darkmatter]`. The three source packages are inherited definition-API modifications; other selections are test/suite-input scope. Gates were therefore not skipped. This whole-tree scope must not be confused with Phase 1 source ownership: this session wrote only the four listed Markdown records. No additional package source or dependent API changed in this phase.

`md get` read back every requested plan/log frontmatter key successfully, including both review items and the replacement handoff message. `git diff --check` passed. The spec has no requested new metadata and was left unchanged. No background commands or agents remain pending. Phase 1 is implementation complete, ready for review; the human-review handoff records future credential and breaking-scope decisions.
