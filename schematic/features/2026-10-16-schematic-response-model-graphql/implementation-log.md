---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-16-schematic-response-model-graphql/spec.md"
plan: "schematic/features/2026-10-16-schematic-response-model-graphql/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - schematic-define
  - schematic-gen
  - schematic-runtime
  - schematic-schema
  - schematic-definitions
  - sniff
human_review: false
source_files_during_phase_2:
  - schematic/define/src/builders.rs
  - schematic/define/src/bundle.rs
  - schematic/define/src/components.rs
  - schematic/define/src/error_table.rs
  - schematic/define/src/graphql.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/openapi/export/info.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/pagination.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/registration.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/bundle_support/mod.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_bundles.rs
  - schematic/define/tests/fixtures/bundles/fixture.json
  - schematic/define/tests/graphql_definitions.rs
  - schematic/define/tests/response_vocabulary.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
docs_updated_during_phase_2:
  - schematic/docs/definition-builders.md
  - schematic/docs/io/definition-bundles.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_2:
  - schematic/docs/responses-and-errors.md
  - schematic/docs/graphql.md
skills_files_updated_during_phase_2:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_3:
  - schematic/runtime/Cargo.toml
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/codec.rs
  - schematic/runtime/src/endpoint.rs
  - schematic/runtime/src/error.rs
  - schematic/runtime/src/graphql.rs
  - schematic/runtime/src/lib.rs
  - schematic/runtime/src/pager/request.rs
  - schematic/runtime/src/response.rs
  - schematic/runtime/src/shapes.rs
  - schematic/runtime/src/stream.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_non_json_endpoint.stderr
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_stream_endpoint.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_a_stream_endpoint.stderr
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_mixed_alternatives.rs
  - schematic/runtime/tests/fixtures/compile_fail/mutator_on_mixed_alternatives.stderr
  - schematic/runtime/tests/l1/decode_boundary.rs
  - schematic/runtime/tests/l1/envelope.rs
  - schematic/runtime/tests/l1/error_routes.rs
  - schematic/runtime/tests/l1/graphql.rs
  - schematic/runtime/tests/l1/main.rs
  - schematic/runtime/tests/l1/shapes.rs
  - schematic/runtime/tests/l1/streams.rs
  - schematic/runtime/tests/l1/support.rs
  - schematic/runtime/tests/l1/timing.rs
docs_updated_during_phase_3:
  - schematic/docs/dependencies.md
  - schematic/docs/graphql.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/deadlines-and-timeouts.md
  - schematic/docs/runtime/hooks.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_4:
  - schematic/define/src/builders.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/responses.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import.rs
  - schematic/define/src/openapi/import/mappings/classify.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/responses.rs
  - schematic/define/src/openapi/import/query_paths.rs
  - schematic/define/src/openapi/mod.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/response_import_matrix.rs
  - schematic/define/tests/response_openapi.rs
  - schematic/define/tests/response_vocabulary.rs
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/client/helpers.rs
  - schematic/gen/src/codegen/client/methods.rs
  - schematic/gen/src/codegen/client/mod.rs
  - schematic/gen/src/codegen/client/shared_auth.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/endpoint_methods/names.rs
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/graphql.rs
  - schematic/gen/src/codegen/mod.rs
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/src/codegen/request_structs/endpoint_spec.rs
  - schematic/gen/src/codegen/responses.rs
  - schematic/gen/src/codegen/variant.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/export/postman/item.rs
  - schematic/gen/src/export/vocabulary.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/common/components.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/l1/response_adapters.rs
  - schematic/runtime/src/api_client.rs
  - schematic/runtime/src/client.rs
  - schematic/runtime/src/endpoint.rs
  - schematic/runtime/src/error.rs
  - schematic/runtime/src/graphql.rs
  - schematic/runtime/src/lib.rs
  - schematic/runtime/src/pager/request.rs
  - schematic/runtime/src/response.rs
  - schematic/runtime/tests/l1/envelope.rs
  - schematic/runtime/tests/l1/error_routes.rs
  - schematic/runtime/tests/l1/graphql.rs
  - schematic/runtime/tests/l1/graphql_address.rs
  - schematic/runtime/tests/l1/main.rs
  - schematic/runtime/tests/l1/support.rs
  - schematic/runtime/tests/l1/whole_send_elapsed.rs
  - schematic/schema/src/lib.rs
docs_updated_during_phase_4:
  - schematic/docs/definition-builders.md
  - schematic/docs/generation/generated-clients.md
  - schematic/docs/generation/validation.md
  - schematic/docs/graphql.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/export-postman.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/building-clients.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_5:
  - Cargo.lock
  - schematic/define/src/builders.rs
  - schematic/define/src/components.rs
  - schematic/define/src/lib.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/src/openapi/export/responses.rs
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/import/builder.rs
  - schematic/define/src/openapi/import/mappings/classify.rs
  - schematic/define/src/openapi/import/mappings/mod.rs
  - schematic/define/src/openapi/import/mappings/responses.rs
  - schematic/define/src/pagination/resolve.rs
  - schematic/define/src/pagination/validate.rs
  - schematic/define/src/prelude.rs
  - schematic/define/src/request.rs
  - schematic/define/src/response.rs
  - schematic/define/src/types.rs
  - schematic/define/tests/builders.rs
  - schematic/define/tests/bundle_support/mod.rs
  - schematic/define/tests/cross_origin_policy.rs
  - schematic/define/tests/definition_bundles.rs
  - schematic/define/tests/fixtures/bundles/fixture.json
  - schematic/define/tests/openapi_tests.rs
  - schematic/define/tests/pagination_support/mod.rs
  - schematic/define/tests/pagination_tables.rs
  - schematic/define/tests/response_import_matrix.rs
  - schematic/define/tests/response_openapi.rs
  - schematic/define/tests/response_vocabulary.rs
  - schematic/definitions/Cargo.toml
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/anthropic/types/mod.rs
  - schematic/definitions/src/anthropic/types/streaming.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/elevenlabs/endpoints/speech.rs
  - schematic/definitions/src/elevenlabs/mod.rs
  - schematic/definitions/src/elevenlabs/types.rs
  - schematic/definitions/src/elevenlabs/types/common.rs
  - schematic/definitions/src/elevenlabs/types/speech.rs
  - schematic/definitions/src/emqx/mod.rs
  - schematic/definitions/src/eversolo/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/mod.rs
  - schematic/definitions/src/huggingface/mod.rs
  - schematic/definitions/src/lmstudio/mod.rs
  - schematic/definitions/src/lmstudio/types.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/ollama/types/completion.rs
  - schematic/definitions/src/ollama/types/models.rs
  - schematic/definitions/src/ollama/types/openai.rs
  - schematic/definitions/src/openai_pagination.rs
  - schematic/definitions/src/openai/mod.rs
  - schematic/definitions/src/registry/typed.rs
  - schematic/definitions/src/samsung_smart_tv/mod.rs
  - schematic/definitions/src/unfolded_circle/core_rest/mod.rs
  - schematic/definitions/tests/authoring_surface.rs
  - schematic/gen/src/codegen/endpoint_methods/tests.rs
  - schematic/gen/src/codegen/module_docs.rs
  - schematic/gen/src/codegen/pagination/tests.rs
  - schematic/gen/src/codegen/request_structs/endpoint_spec.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/codegen/responses.rs
  - schematic/gen/src/definition_gen.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/item.rs
  - schematic/gen/src/export/postman/tests.rs
  - schematic/gen/src/export/vocabulary.rs
  - schematic/gen/src/output/assemble/api.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/common/components.rs
  - schematic/gen/tests/l1/definition_baseline.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/pagination_validation.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/postman_golden.rs
  - schematic/gen/tests/l1/query_param_detection.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/gen/tests/l1/response_adapters.rs
  - schematic/gen/tests/l1/vocabulary_exports.rs
  - schematic/openapi/anthropic.json
  - schematic/openapi/bitbucket.json
  - schematic/openapi/elevenlabs.json
  - schematic/openapi/lmstudio.json
  - schematic/openapi/ollama.json
  - schematic/openapi/openai.json
  - schematic/postman/elevenlabs.postman_collection.json
  - schematic/postman/lmstudio.postman_collection.json
  - schematic/postman/ollama.postman_collection.json
  - schematic/postman/openai.postman_collection.json
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama.rs
  - schematic/schema/src/ollama/mod.rs
  - schematic/schema/src/ollama/requests.rs
  - schematic/schema/src/ollama/responses.rs
  - schematic/schema/src/openai/requests.rs
  - schematic/schema/tests/non_json_endpoints.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/envelope.rs
  - sniff/lib/tests/l1/remote_providers.rs
docs_updated_during_phase_5:
  - schematic/docs/definition-builders.md
  - schematic/docs/generation/generated-clients.md
  - schematic/docs/generation/validation.md
  - schematic/docs/io/definition-bundles.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/io/import-openapi.md
  - schematic/docs/io/openapi-extensions.md
  - schematic/docs/responses-and-errors.md
  - schematic/docs/runtime/codecs.md
  - schematic/docs/runtime/README.md
  - schematic/docs/runtime/sending-and-responses.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/schematic-define/schematic/definitions.md
  - .claude/skills/schematic-define/schematic/gen.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_6:
  - schematic/define/src/openapi/extensions.rs
  - schematic/define/src/openapi/export/mod.rs
  - schematic/define/src/openapi/export/paths.rs
  - schematic/define/tests/response_openapi.rs
  - schematic/gen/tests/l1/response_adapters.rs
  - schematic/schema/tests/streaming_providers.rs
  - schematic/openapi/github.json
  - schematic/openapi/openai.json
docs_updated_during_phase_6:
  - schematic/docs/io/openapi-extensions.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/schematic-define/SKILL.md
message_to_agent: |-
  Phase 6 is complete; read "## Phase 6" in this log (findings and review agenda).
  - Phase 5 was NOT committed before Phase 6 ran, so the working tree holds both phases; the scope calculator sees both.
  - `x-schematic` now carries document `cross_origin` (only when not `SameOrigin`), `request_id_header`, and operation `cross_origin` (endpoint override). Struct literals of `SchematicDocExtension`/`SchematicOpExtension` need these members.
  - Mock-server tests through the regenerated clients live in `schematic/schema/tests/streaming_providers.rs`; its `HeldServer` (raw TCP, holds the body until the test saw the first item) is reusable for any incremental-delivery check. These run via `just test-schema` only (CI does not run the schema crate).
  - OpenAI `BetaResponseStreamEvent` (importer output, untagged) decodes most events as the audio variants; logged, not fixed. Do not build GraphQL/pagination tests that depend on OpenAI stream event variants.
  - Shipped-definition export tests are in `gen/tests/l1/response_adapters.rs` (`shipped_*`); building the full OpenAI API costs ~1.2 s, so trim to the endpoints you need.
---

# Implementation Log for 2026-10-16-schematic-response-model-graphql (8 phases)

## Phase 1

### Outcome: blocked at the prerequisite gate

Plan Rule 1 requires the run to stop with a recorded block if a required
prerequisite surface is absent. The spec's "Dependencies and ordering" section
says `2026-10-15-schematic-definition-vocabulary` (spec 4b) must be **landed
first**. On 2026-10-21 it is not, so this run stops after Wave 1's
prerequisite inventory. No source, test, generated artifact, doc page, or skill
changed.

Only the **Prerequisite inventory** task is checked off in the plan. Its
deliverable is this section. The following tasks are deliberately left
unchecked:

- **Consumer inventory** and **Provider inventory**: 4b Phases 2–5 rewrite the
  builders, auth, OAuth2, query fidelity, and dependent call sites these
  inventories would count. Counts taken now would be stale before Phase 2
  could use them.
- **Baseline capture**, **Timeout spike**, and **Contract freeze**: the plan
  says Wave 2 runs only "after Wave 1 clears the gates". The bundle-format
  matrix columns cannot be frozen because no bundle loader exists.

### Prerequisite inventory (tree state 2026-10-21, branch `feat/better-schematics`)

| Prerequisite | Spec state | Required surface | Present in code? |
| --- | --- | --- | --- |
| `2026-10-11-schematic-generation-integrity` (spec 1) | `implemented: true`, closed | validation structure, R42 diagnostics format | yes (not re-verified in detail; refresh with the spec) |
| `2026-10-12-schematic-runtime-envelope` (spec 2) | `implemented: true`, closed | envelope, typed send, `EndpointSpec`, codecs, hook marker, `send_raw` | **yes**: `schematic/runtime/` (`schematic-runtime`). `EndpointSpec` at `runtime/src/endpoint.rs:29`, `ErrorBody` at `endpoint.rs:65`, `Decode` at `codec.rs:33`, hook marker `SupportsHooks` at `hooks.rs:41`, `ErrorKind<E>` at `response.rs:140` (exists), `NoDeclaredError` at `response.rs:251`, `send_raw` at `client.rs:660`, transport builder `SharedTransportBuilder` at `transport.rs:34` |
| `2026-10-13-schematic-pagination` (spec 3) | `implemented: true`, closed | pagination model, Relay preset | yes: `runtime/src/pager/` (`Paginated`, `CursorKind`, `ResumeAfter`/`ResumeBefore`), `define/src/pagination/selector.rs`. Whether a named Relay preset exists, or only a Relay-capable selector, was not checked; check during refresh (D-12) |
| `2026-10-14-schematic-definition-api` (spec 4a) | `implemented: true`, `status: human-in-the-loop`, 3 review iterations, still in `features/` (not closed) | builders, `DefinitionContext`, typed registration, `.returns`/`.errors` placeholders | **yes, partly placeholder**: `DefinitionContext` at `define/src/registration.rs:121` (`register`, `register_for`, `register_as`). `Success(ApiResponse)` newtype at `define/src/builders.rs:48`. `ErrorSet { _private: () }` at `builders.rs:61`. `EndpointBuilder::errors(self, _: ErrorSet)` at `builders.rs:123` discards its argument (the placeholder this spec fills). `Endpoint::response: ApiResponse` still at `define/src/types.rs:395` |
| `2026-10-15-schematic-definition-vocabulary` (spec 4b) | `implemented: false`, `status: draft-spec`; **Phase 1 of 6 complete**, plan `phase: 2` | self-contained definition bundle (4b Phase 4); rate-limit/deprecated/auth-override vocabulary (4b Phases 2–3) that R26's `x-schematic` must carry; builder integration (4b Phase 2) | **no**: no bundle reader or writer in `schematic-define` (no `bundle` symbol in `define/src`). No rate-limit or deprecation metadata in `define/src`. `cross_origin` and `request_id_header` exist already (from spec 2/4a) |

Other facts gathered:

- 62 Rust files outside `target/` still name `ApiResponse` (a raw `grep -l`
  count, not the plan's consumer inventory).
- The uncommitted change to
  `features/2026-10-14-schematic-definition-api/implementation-log.md` already
  in the working tree is someone else's work. It was not touched.

### Why this is blocking rather than a defaulted choice

Under the rulings section of the run prompt, a block needs all three of these:

1. The specification offers no way to proceed without 4b. It says "Requires,
   landed first", and plan Rule 1 says to stop.
2. No option is consistent with the spec: R44 and the definition-bundle input
   matrices need 4b's bundle, and R26 needs 4b's metadata vocabulary. The plan
   forbids building those here ("this plan does not silently build those
   features"; "Unsupported bundle formats are not added to satisfy this plan").
3. A wrong guess is costly. Phase 2 edits `schematic-define` builders, stored
   types, and the serialized shape that 4b Phases 2–4 are about to change. Two
   features rewriting the same public API and serialized format would cost far
   more to reconcile than a review diff.

This spec is also still `refresh_required: true`. Its "Refresh notes" list
outcomes from 4a and 4b review 1 as "the largest source of change". Those
outcomes do not exist yet.

### Authorizations and defaults

- Pre-authorizations: confirmed by the run (plan Rule 2). **None were used**:
  nothing was regenerated, deleted, or migrated, and no dependency changed.
- Defaults D-4–D-31: **not adopted yet**. Rule 3 makes their adoption subject
  to the prerequisite review outcomes, which do not exist. Record them in the
  rerun's Phase 1.

### Gates

There are no source changes, only this log and the plan's checkbox and
frontmatter. The scope calculator was run after every edit was saved:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `packages: []`, `change_class: documentation`. As the run prompt
allows, `just test` and `just lint` were **skipped**. No dependent-package
tests or cross-OS checks apply, because no code changed. The `schematic-define`
skill needed no update.

### Rerun on 2026-10-21: still blocked

Phase 1 was invoked again without its prerequisite having moved. Before doing
any work, the gate was re-checked against the tree (HEAD `c135102a4`, the
previous run's Phase 1 close; clean working tree):

| Check | Result |
| --- | --- |
| `2026-10-15-schematic-definition-vocabulary` spec | `implemented: false`, `status: draft-spec`, `refresh_required: true` (unchanged) |
| its plan | `phase: 2` (Phase 1 of 6 complete, unchanged) |
| bundle reader/writer in `schematic/define/src` | none (no `bundle` symbol) |
| endpoint rate-limit / deprecation vocabulary | none (the only `deprecated` hits are OpenAPI parameter/operation fields in `openapi/export` and `openapi/import`) |
| this spec | `status: draft-spec`, `refresh_required: true` (unchanged) |
| 4a placeholders | unchanged: `Success(ApiResponse)` at `define/src/builders.rs:48`, `ErrorSet { _private: () }` at `builders.rs:61`, `errors(self, _: ErrorSet)` at `builders.rs:123`, `Endpoint::response: ApiResponse` at `define/src/types.rs:395` |

The three blocking conditions recorded above still hold, so this run stops at
the same point. No source, test, generated artifact, doc page, or skill
changed; only this log's rerun section and its `message_to_agent` were
updated. The prerequisite inventory table above is still accurate as of this
rerun. The remaining Wave 1 and Wave 2 tasks stay unchecked for the reasons
already given.

**Process note for the author:** rerunning Phase 1 cannot clear this gate.
The pipeline should not reinvoke this plan until 4b has landed and been
reviewed and this spec has been refreshed (`refresh_required: false`).

**Rerun gates.** The changed files are this log and the plan's frontmatter.
The scope calculator, run after every edit was saved
(`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`),
returned `packages: []` and `change_class: documentation`, so `just test` and
`just lint` were skipped as the run prompt allows. No dependent tests, cross-OS
checks, or skill updates apply.

### Rerun on 2026-10-22: prerequisite landed, still blocked on its review and this spec's refresh

The prerequisite has moved since the 2026-10-21 runs, so the gate was
re-evaluated rather than assumed closed. Tree: HEAD `ea86eb8ab`, branch
`feat/better-schematics`. The working tree already had uncommitted changes to
`features/2026-10-15-schematic-definition-vocabulary/spec.md` and an untracked
`review-1.md` there. Both belong to someone else and were not touched.

| Check | Result on 2026-10-22 |
| --- | --- |
| 4b code | **landed**: Phases 2–6 committed (`13c168975`, `69e819c97`, `c00790c57`, `cf1a27e80`, `ea86eb8ab`). `DefinitionBundle` with `to_json`/`from_json` and `format_version` at `schematic/define/src/bundle.rs:48`; `RateLimit` at `define/src/metadata.rs:99`, `RetryGuidance` at `metadata.rs:286` |
| 4b spec | `implemented: true`, `review_iterations: 1`, still `status: draft-spec` and `refresh_required: true` |
| 4b review 1 | a triage of decisions the author must make. No implementation review was done: `human_review: true`, **`human_resolved: false`**, and six open decisions (listed below) |
| 4a | `status: human-in-the-loop` (unchanged) |
| specs 2 and 3 | closed in `features/_completed/` (unchanged) |
| 4a placeholders this spec fills | unchanged: `Success(ApiResponse)` at `define/src/builders.rs:48`, `ErrorSet { _private: () }` at `builders.rs:61`, `errors(self, _: ErrorSet)` at `builders.rs:125`, `Endpoint::response: ApiResponse` at `define/src/types.rs:466` |
| this spec | `status: draft-spec`, **`refresh_required: true`** (unchanged) |
| Rust files outside `target/` naming `ApiResponse` | 64 (raw `grep -l`, up from 62) |

**Why this is still blocking.** Plan Rule 1's narrow "surface absent" condition
no longer holds: the runtime, builders, and bundle all exist. Two other
conditions still stop the run:

1. **This spec has not been refreshed.** Its Refresh notes say "Before its
   plan runs, refresh it", and they name the outcomes of 4a and 4b review 1
   as "the largest source of change". Those review outcomes do not exist yet.
   Refreshing the spec is an authoring step that comes before this plan, not a
   Phase 1 task. The plan says it is "a plan for the tree after the
   prerequisite reviews".
2. **4b review 1 is waiting on the author, and three of its six open decisions
   set inputs that this phase's Contract freeze must lock down:**
    - the version-1 bundle layout (schemas and bindings kept together in
      `api.schemas`, or separate maps). It decides R44 and the
      definition-bundle input-robustness columns.
    - the precedence for endpoint auth replacement, and whether anonymous calls
      may bypass it. It decides what R26's `x-schematic` extension carries.
    - whether to keep the retry-guidance vocabulary. It changes R26's field
      list.

    The other three decisions (omitting OpenAI query parameters, the importer's
    explode default, and OAuth client authentication) affect the baseline the
    Baseline capture task would compare against.

The rule "conform to what landed" in the spec's Dependencies section was
considered. It does not cover this case, because what landed is still waiting
on author decisions that may change it. Locking down contracts that the author
may reject in 4b is the costly wrong guess the blocking test describes. Phase 1
takes inventories and freezes contracts for every later phase, so redoing it
after the 4b rulings costs less than redoing Phases 2–8.

**What did not change.** No source, test, generated artifact, doc page, or
skill changed. No pre-authorization was used. Defaults D-4–D-31 are still
not adopted. The Consumer inventory, Provider inventory, Baseline capture,
Timeout spike, and Contract freeze tasks remain unchecked. The prerequisite
inventory table above is superseded by this section's table for 4b.

**What unblocks it:** (1) the author records rulings on the six 4b review 1
decisions (`human_resolved: true`); (2) any code change those rulings require
lands; (3) this spec is refreshed from the 4a/4b outcomes and set to
`refresh_required: false`. Then rerun Phase 1 from the start.

**Gates.** See the scope-calculator result recorded below.

The scope calculator was run after all edits were saved
(`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`).
It returned `packages: []` and `change_class: documentation`. The working tree
it saw included the two pre-existing 4b feature files. `just test` and
`just lint` were therefore skipped. No dependent tests, cross-OS checks, or
skill updates apply. Only this log changed in this run; the plan and spec were
not edited.

### Rerun on 2026-10-22 (fourth run): unchanged, still blocked

The gate was checked again at HEAD `9a50aad7b`. Nothing that unblocks it has
changed since the previous run:

- 4b review 1 still has `human_resolved: false`. Its `## Human rulings`
  section still says "Pending", and its six decisions are still open.
- The uncommitted 4b log entry in the working tree records the 4b
  review-to-implement pass. It says that pass had no findings to implement and
  that it is still waiting on the author. It contains no rulings, and this run
  did not touch it.
- This spec is still `refresh_required: true` and `status: draft-spec`.

The reasons in the previous section still apply, and so does the unblock path:
rulings recorded, any code they require landed, this spec refreshed and set to
`refresh_required: false`, then Phase 1 rerun from the start. No source, test,
doc, plan, spec, or skill file changed, and no plan task was checked off. Only
this log changed.

**Gates.** The scope calculator was run after the edit
(`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`).
The result is recorded below.
It returned `packages: []` and `change_class: documentation`, so `just test`
and `just lint` were skipped.

## Refresh on 2026-10-22

The four Phase 1 runs above stopped at the prerequisite gate and did no Phase 1
work. `2026-10-15-schematic-definition-vocabulary` has since been implemented,
and its review 1 rulings are recorded (all six accept the implemented
behavior). The spec and plan were refreshed against the landed code:

- the spec now has `refresh_required: false`, and a reader's note of 2026-10-22
  at its top records the landed builders, 4b metadata, the version-1 bundle,
  the baselines, runtime `ABI = 5`, the counts, and OpenAI `GetResponse`
  (`BetaResponse`)
- plan Rule 1 now states that the prerequisites have landed and lists their
  symbols and locations, and the Phase 1 "Prerequisite inventory" task is
  unchecked again

Phase 1 runs from the start. The prerequisite snapshot tables in the sections
above are historical; the next run's inventory replaces them.

## Phase 1

Run of 2026-10-22, after the refresh. Tree: HEAD `37095c7ee`, branch
`feat/better-schematics`. Implementer: `claude/opus`. The working tree already
had uncommitted edits to `prompts/_reviews/feature-review.md`,
`features/2026-10-15-schematic-definition-vocabulary/review-1.md`, and this
feature's spec, plan, and log (the refresh). None of them was reverted.

**Outcome:** every Phase 1 task is complete. Every prerequisite the plan
names is present. Some have moved, have different shapes, or are missing a
preset; those differences are listed below and are conformed to, not blocking.
No source, test, generated artifact, doc page, or skill changed; Phase 1 is
inventory, baseline, and contract freeze only.

### Wave 1 — Prerequisite inventory

All paths are under `schematic/`. "Δ" marks a difference from the plan or
spec. Each one is conformed to (plan Rule 1).

| Surface | Landed form |
| --- | --- |
| `Success` | `define/src/builders.rs:48`, `#[non_exhaustive] pub struct Success(ApiResponse)` with a private field; `json(TypeDesc)`, `text()`, `binary()`, `empty()` at `:51-54` |
| `ErrorSet` | `builders.rs:61`, `{ _private: () }`, only `none()`; `EndpointBuilder::errors(self, _: ErrorSet)` at `:125` stores nothing; no shipped definition calls `.errors(` |
| `.returns*` | `.returns(Success)` `:113` counts `successes` and overwrites `endpoint.response`; `.returns_json::<T>(ctx)` `:116` registers through `ctx.register_for`; `.returns_text/_binary/_empty` `:122-124` |
| second success | Δ rejected at `build()` (`builders.rs:176`, `DefinitionError::InvalidDefinition`, "several success shapes arrive with response model v2"), not when `.returns` is called. Phase 2 removes this rule |
| `ApiResponse` | Δ in `define/src/response.rs:49`, not `types.rs`: `Json(TypeRef)`, `Text`, `Binary`, `Empty`; externally tagged serde (`{"Json":{"Named":"X"}}`) |
| `Endpoint` | `types.rs:447`; `response: ApiResponse` at `:466`; `pagination: Option<pagination::Collection>` `:486`; `rate_limit`, `retry_guidance`, `deprecated`, `auth: Option<EndpointAuth>`; `Option` members use `serde_presence::non_null` (absent allowed, null rejected) |
| `RestApi` | `types.rs:107`; `rate_limit` `:205`, `retry_guidance` `:209`, `effective_*` `:326/:333` |
| `EndpointAuth` | Δ in `define/src/auth.rs:354`, not `metadata.rs`: `Public` or `Policy { policy, env_mapping }` |
| `RateLimit`, `RetryGuidance` | `define/src/metadata.rs:99`, `:286` |
| `DefinitionContext` | `define/src/registration.rs:154` (`types_in`), `register_nominal :167`, `register :173`, `register_for :177`, `register_as :185`; all return `Result<TypeDesc, DefinitionError>` |
| `DefinitionBundle` | `define/src/bundle.rs:48`, `BUNDLE_FORMAT_VERSION = 1` `:23`. Loader `from_json :179` runs `strict_parse :221` (duplicate key, trailing content), `check_version :269`, `reject_nulls :299` (except schema payloads), typed deserialize, `check_members :333` (diff against re-serialization; `OMITTED_DEFAULTS :325`), then `validate()`. Δ **JSON only**, so the bundle matrices below are JSON-only (plan Rule 12) |
| bundle matrix | `define/tests/definition_bundles.rs:490` `bundle_json_matrix()`, cells at `:249`, helpers in `define/tests/bundle_support/mod.rs` |
| x-schematic | `define/src/openapi/extensions.rs`: `SchematicDocExtension :44` (`module_path, request_suffix, env_mapping, headers, rate_limit, retry_guidance`), `SchematicOpExtension :99` (`request, response: ApiResponse, headers, rate_limit, retry_guidance`) |
| pagination | Δ **no Relay preset**. `PageStrategy::{PageNumber, Offset, Cursor, NextLink}` (`define/src/pagination/strategy.rs:468`), `PageStrategy::cursor(param, CursorSource) :834`, `cursor_leg :838`, `InputParam::Body(JsonPath)` `:16`. Body-path injection exists (`runtime/src/pager/request.rs:284`; empty body starts from `{}`, non-JSON is rejected). D-12's `relay_forward` preset is therefore new Phase 7 builder sugar over these, not a new strategy |
| runtime | `runtime/src/lib.rs:108` `ABI = 5`; `RequestParts` `endpoint.rs:80` (`query: Vec<(String, QueryValue)>`); `EndpointSpec` (`Api`, `Success`, `Failure: ErrorBody`, `Decoder`, `ENDPOINT_ID`, `METHOD`, `DECLARED_ERROR_STATUSES: RangeInclusive<u16> = 400..=599`, `into_parts`); `ErrorBody` `endpoint.rs:65` (`DECLARED`, `decode_body`); `ErrorKind<E>` `response.rs:140`, `#[non_exhaustive]`, 8 variants (`Declared, Undeclared, MalformedError, MalformedSuccess, BodyInterrupted, RedirectRefused, RedirectFailed, Hook`); `NoDeclaredError` `:251`; `Response<T,E> { meta, outcome }` `:27`; `Decode` `codec.rs:33` with `Json<T> :242`, `Text :279`, `Binary :291`, `Empty :303`; `SupportsHooks` `hooks.rs:41` (only `Json<T>`); `send_raw` `client.rs:734` |
| runtime streaming | `LiveBody` `codec.rs:60` with crate-private `StreamBounds { idle, deadline }` `:74` and public `into_streaming()` `:113`; set at `client.rs:1384` (`idle = expiration.timeout`, `deadline = expiration.deadline_at`). No stream codecs, no frame limit, no public idle setting: all new work in Phase 3 |
| managed builder | `Client::new(..)` `client.rs:265`: `base_url, timeout` (default 30 s, `:50`), `connect_timeout, proxy, add_root_certificate, danger_accept_invalid_certificates, user_agent, default_header, transport, credentials, api_key, cross_origin, build`; per call `credential, anonymous, deadline` (`options.rs:21`) |
| baselines | `gen/tests/l1/definition_baseline.rs`: `adoptions() :39` (1 entry, github `rate_limit`), `normalize_historical :59`, `apply_form_explode_default :107`, `EXPLAINED :12` empty; fixtures `gen/tests/fixtures/definition-baseline/*.json` (18). `gen/tests/l1/artifact_drift.rs` byte-compares fresh Rust/OpenAPI/Postman against `schema/`, `openapi/`, `postman/`. `gen/tests/l1/definition_bundles.rs` plus `define/tests/definition_bundles.rs` |
| stale `gen/schematic` | Δ already gone (no directory, nothing in `git ls-files`). The optional deletion pre-authorization is not needed |
| recipes | `schematic/justfile` `test` and `lint` include `schematic-runtime`; `test-compile-fail` runs the runtime's trybuild `slow_` tests |
| leftover | Δ `gen/src/codegen/error.rs:288` `generate_response_kind_type()` is uncalled, and `request_enum.rs:179` / `client/methods.rs:189` still use a `response_kind()`; committed `schema/` has no `ResponseKind`. This is dead generator code. Phase 4/5 removes it when it switches those readers (R12 says nothing builds against the guard) |

**4b review 1 rulings** (`human_resolved: true`):

1. per-endpoint env mappings and the precedence per-call, then client, then env are accepted; an explicit anonymous call may bypass a replacement policy.
2. unsupported optional query inputs are omitted with diagnostics.
3. the version-1 bundle envelope is accepted.
4. the OpenAI importer's `explode=true` default is accepted.
5. OAuth `client_auth` behavior is kept.
6. rate-limit and retry metadata are kept.

None of them changes this spec's inputs.

**Authorization status** (plan Rule 2, separate from defaults): the
pre-authorizations were confirmed by the author on 2026-10-21. **None was
used in Phase 1**: nothing was regenerated, deleted, migrated, or added as a
dependency.

### Wave 1 — Consumer inventory

Counting method: `grep` for `ApiResponse` and `\.response\b` in `.rs` files
outside `target/`, minus name-only false positives
(`AssistantsApiResponseFormatOption`, UC `ApiResponseMessage`, two doc
comments). Endpoints were counted by builder starts plus `.returns_*` calls per
directory and cross-checked against the 18 baseline JSON fixtures
(`.definition.endpoints[].response`).

- **Readers:** 59 files name `ApiResponse`, and 3 more read only `.response`.
  - define:
    - model: `response.rs`, `types.rs`
    - builder: `builders.rs`
    - components: `components.rs:172`
    - pagination: `pagination/resolve.rs:97,122,158`, `pagination/validate.rs:365`
    - OpenAPI export: `openapi/export/responses.rs:14` `map_responses`, `export/paths.rs:189,195`
    - OpenAPI import: `openapi/import/mappings/responses.rs:244`, `import/builder.rs:222,1046`
    - x-schematic: `extensions.rs:105`
    - the bundle has no direct match; it serializes through serde.
  - gen:
    - success type and decoder: `codegen/request_structs/endpoint_spec.rs:25`
    - `request_enum.rs:179`
    - `client/mod.rs:67-70`
    - `client/helpers.rs:53-86`
    - definition emitter: `definition_gen.rs:188,212`
    - validator: `validation.rs:458,868`
    - Postman diagnostics: `export/vocabulary.rs:101`
    - module docs tests.
  - unit tests in gen `output`, `test_utils`, `client`, `endpoint_methods`, `pagination`, `request_structs`, and `export/postman`.
  - gen L1 tests in 11 files and define tests in 5 files.
  - `#[cfg(test)]` assertions in 13 definitions `mod.rs` files, plus `registry/typed.rs:99` and `definitions/tests/authoring_surface.rs`.
  - runtime and oauth: 0 hits.
- **Endpoints:** 18 REST APIs, **539** resolved endpoints (503 literal builder
  sites; `emqx/mod.rs:230` `build_common_endpoints` is shared by both EMQX APIs).
  By variant: Json 475, Text 9, Binary 20, Empty 35.

  | API | Endpoints | Json | Text | Bin | Empty |
  | --- | --- | --- | --- | --- | --- |
  | anthropic | 4 | 4 | 0 | 0 | 0 |
  | artificial-analysis (2 APIs) | 7 | 7 | 0 | 0 | 0 |
  | bitbucket | 15 | 13 | 0 | 2 | 0 |
  | elevenlabs | 42 | 36 | 0 | 6 | 0 |
  | emqx (2 APIs) | 74 | 54 | 2 | 0 | 18 |
  | eversolo | 24 | 24 | 0 | 0 | 0 |
  | gitea | 15 | 14 | 1 | 0 | 0 |
  | github | 16 | 15 | 1 | 0 | 0 |
  | gitlab | 19 | 19 | 0 | 0 | 0 |
  | huggingface | 26 | 23 | 3 | 0 | 0 |
  | lmstudio | 6 | 4 | 0 | 2 | 0 |
  | ollama (2 APIs) | 15 | 6 | 0 | 7 | 2 |
  | openai | 258 | 248 | 0 | 1 | 9 |
  | samsung-smart-tv | 7 | 2 | 1 | 0 | 4 |
  | unfolded-circle-core-rest | 11 | 7 | 1 | 2 | 1 |

  (The EMQX row is the resolved count for both APIs; its literal split is 27/1/0/10.)

  **Correction to the "roughly 530 untouched" estimate:** the four migrated
  APIs plus the two named endpoints are 69 endpoints (15+6+4+42+2), so **470
  endpoints are untouched**. Within those four APIs, only the endpoints the
  provider inventory below lists change their declarations. The other
  endpoints in those directories keep their call sites; "touched" here means
  the directory may be edited, not that every endpoint changes.
- **Streaming request fields (`pub stream`):**
  - Ollama `GenerateBody`, `ChatBody`, `PullModelBody`, `PushModelBody`, `CreateModelBody`, `OpenAIChatCompletionRequest`, `OpenAICompletionRequest`
  - LM Studio `ChatBody`
  - Anthropic `CreateMessageBody`
  - OpenAI `CreateCompletionRequest2`, `CreateImageRequest2`, `EditImageBodyJsonParam`; the CreateTranscription form field; the GetResponse query param.
- **Endpoints declared Binary that stream:**
  - Ollama native Generate, Chat, PullModel, PushModel, CreateModel
  - Ollama OpenAI ChatCompletions, Completions
  - LM Studio Chat, DownloadModel
  - ElevenLabs StreamSpeech

  Anthropic CreateMessage is declared Json although it can stream.
- **Hooks:** no definition declares a response hook (all `hook` hits are
  domain "webhook" data). `SupportsHooks` exists only for `Json<T>`. **D-26
  will remove nothing.**
- **Stranded Ollama types:** 13 structs plus one alias, not ten. They have no
  `Registration` impl or `JsonSchema` derive and appear in no `module_group`.
  - `completion.rs`: `GenerateResponse`, `GenerateStreamChunk`, `ChatResponse`, `ChatStreamChunk`
  - `models.rs`: `PullModelProgress`, alias `PushModelProgress`, `CreateModelProgress`
  - `openai.rs`: `OpenAIChatCompletionResponse`, `OpenAIChatCompletionChoice`, `OpenAICompletionResponse`, `OpenAICompletionChoice`, `OpenAIChatStreamChunk`, `OpenAIStreamChoice`, `OpenAIDelta`

  No completions stream chunk type exists yet.
- **Dependents.** Nine crates depend on `schematic-schema`: `sniff`,
  `biscuit-speaks`, and the homelab crates `homelab` (lib),
  `eversolo-integration`, `arcam-amp-integration`,
  `sony-receiver-integration`, and `unfolded-integration-helper` (plus their
  shared workspace use). `model-citizen` depends on `schematic-definitions`,
  not on the schema crate. `homelab/cli` and `homelab/server` have no direct
  schematic dependency. Uses that touch a migrated endpoint:
  - **sniff** calls Bitbucket `get_source` (`sniff/lib/src/remote/bitbucket.rs:176,360`; it decodes raw bytes by hand as `PaginatedResponse<SourceEntry>` or text). **This is the one forced dependent call site** expected when R32a changes `GetSource`'s success type to an enum.
  - **biscuit-speaks** uses ElevenLabs `create_speech`, `create_sound_effect`, and `list_voices` (`lib/src/providers/cloud/elevenlabs.rs:455,899,695`). Their declared shape stays Binary or JSON, and the Rust type is unchanged. `StreamSpeech` is not used.
  - **model-citizen** uses `ollama::{ShowModelBody, ShowModelResponse}` and `lmstudio::{ListModelsResponse, ModelInfo}` (definition types only; unaffected).
  - **homelab crates:** eversolo, samsung, and UC integration types only.
  - **Zero references:** generated `ollama`, `lmstudio`, `anthropic`, and OpenAI `GetResponse`/`BetaResponse` outside `schematic/`.
- **sniff GraphQL (R43 inputs):**
  - document: `sniff/lib/src/remote/check_status.rs` `batch_query(count) :37`, with aliased `p{i}:pullRequest(number:$n{i})` and fragment `F`.
  - variables: `batch_variables :51` builds `{owner, name, n0…}`.
  - transport: `focused.rs:534/556/580`.
  - fixtures: the eight files in `sniff/lib/tests/fixtures/remote/graphql_check_status/` (`success`, `passed`, `changed_head`, `null_rollup`, `partial_errors`, `request_wide_errors`, `rate_limit`, `permission_failure`), read with `include_str!` in `sniff/lib/tests/l1/pr_check_status.rs:22-33`.

### Wave 1 — Provider inventory and evidence

Provider documentation was read on **2026-10-22** (documentation pages and
the providers' published OpenAPI/SDK sources only; no provider API was
called). In-repo research labels were not taken as framing evidence (plan
Rule 10).

**Evidence (URL, what it establishes):**

- **Anthropic.**
  - <https://platform.claude.com/docs/en/build-with-claude/streaming> documents:
    - SSE when `"stream": true`.
    - the event flow `message_start`, `content_block_start/delta/stop`, `message_delta`, a final `message_stop`, and any number of `ping` events.
    - delta types `text_delta`, `input_json_delta`, `thinking_delta`, and `signature_delta`.
    - an `error` event `{"type":"error","error":{"type","message"}}`.
    - "new event types may be added".
  - <https://platform.claude.com/docs/en/api/errors> gives the HTTP error body `{"type":"error","error":{type,message},"request_id"}` and the status list 400/401/402/403/404/409/413/429/500/504/529.
  - **Not documented:** the literal response media type `text/event-stream`.
- **Ollama native.**
  - <https://github.com/ollama/ollama/blob/main/docs/api/streaming.mdx> establishes `application/x-ndjson` when streaming and `application/json` when `stream:false`.
  - `docs/api.md` (same repo) establishes a final `done:true` object carrying `eval_count`, `total_duration`, and the rest; pull/push/create progress `StatusEvent {status, digest, total, completed}` with a final `{"status":"success"}` and **no `done` field**; and a single JSON object for `stream:false` on pull/push/create.
  - `docs/api/errors.mdx` establishes a mid-stream `{"error": …}` NDJSON line with an unchanged status.
- **Ollama OpenAI-compatible.** <https://docs.ollama.com/api/openai-compatibility> lists streaming for both endpoints. SSE and `[DONE]` are **not documented** by Ollama.
- **LM Studio.**
  - <https://lmstudio.ai/docs/developer/rest/chat>: SSE when `stream` is true, **default `false`**.
  - <https://lmstudio.ai/docs/developer/rest/streaming-events>: `event:`/`data:` frames named `chat.start`, `model_load.*`, `prompt_processing.*`, `reasoning.*`, `tool_call.*`, `message.*`, `error`, and `chat.end`. The stream "concludes with `chat.end`", which carries the aggregated result. The `error` event is `{"type":"error","error":{type,message,code?,param?}}`.
  - <https://lmstudio.ai/docs/developer/rest/download>: `POST /api/v1/models/download` returns **a JSON download-job object** (`job_id`, `status`, …), **not SSE**.
- **ElevenLabs.**
  - The `text-to-speech/stream`, `stream-with-timestamps`, `convert`, sound-effects, history-audio, and voice-sample reference pages:
    - default `output_format` is `mp3_44100_128`.
    - **no response media type is stated** for any audio endpoint ("Streaming audio data", "File download").
    - with-timestamps is "a stream of JSONs" with snake_case fields `audio_base64`, `alignment`, and `normalized_alignment`.
    - 422 `{"detail":[{loc,msg,type}]}`.
  - The official Python SDK (`elevenlabs-python` `text_to_speech/raw_client.py`) reads with-timestamps with `iter_lines()` + `json.loads`: line-delimited in practice. This is SDK evidence, not docs.
- **OpenAI Responses.**
  - <https://github.com/openai/openai-openapi/blob/main/openapi.json>: `GET /responses/{id}` `stream` is "streamed … using server-sent events", and the 200 lists `application/json` and `text/event-stream`.
  - `[DONE]` is documented only for chat/legacy completions and Assistants, not for Responses.
  - Terminal events `response.completed`/`failed`/`incomplete`; `error` event `{type, code, message, param, sequence_number}`.
- **Bitbucket.**
  - <https://dac-static.atlassian.com/cloud/bitbucket/swagger.v3.json> (`/repositories/{workspace}/{repo_slug}/src/{commit}/{path}`):
    - a file's Content-Type "is derived from the filename extension", plus `Content-Disposition: attachment` and an ETag.
    - a directory is a paginated JSON listing (`pagelen`, `values`, `page`, `size`; items `commit_directory`/`commit_file`).
    - `format=meta` returns file metadata JSON.
    - no header marks file versus directory.
    - the spec itself declares only `application/json` for 200.

**Endpoint migration table** (Phase 5 input; types under `definitions/src/<api>/`):

| Endpoint | Today | New success shapes (wire tag) | End | Provider error events / HTTP errors | Notes |
| --- | --- | --- | --- | --- | --- |
| Ollama `Generate` POST /api/generate | `returns_binary()` `ollama/mod.rs:77` | `Stream` NDJSON of `GenerateStreamChunk` (`application/x-ndjson`, note "returned when `stream` is true or omitted"); `Json` `GenerateResponse` (`application/json`, note "returned when `stream` is false") | `FieldEquals { done = true }` + keep | mid-stream `{"error"}` line: item type becomes chunk-or-error (R11c); no API-level errors (D-19) | register both types; enum-name collision, see Contract freeze |
| Ollama `Chat` POST /api/chat | `:82` | NDJSON `ChatStreamChunk` + JSON `ChatResponse` (same tags and notes) | `done = true` + keep | same | same collision |
| Ollama `PullModel` / `PushModel` / `CreateModel` | `:102/:107/:122` | NDJSON `PullModelProgress` / `PushModelProgress` / `CreateModelProgress` (`application/x-ndjson`) + JSON status reply (`application/json`, `stream:false`) | `FieldEquals { status = "success" }` + keep (documented final line; there is no `done` field) | `{"error"}` line as an item variant | the JSON status type is the same `StatusResponse`-shaped struct; add it if absent |
| Ollama OpenAI `ChatCompletions` / `Completions` | `:172/:177` | JSON `OpenAIChatCompletionResponse` / `OpenAICompletionResponse` (`application/json`, the default when `stream` is omitted) + SSE `OpenAIChatStreamChunk` / new completions chunk type (`text/event-stream`) | `DataLineEquals("[DONE]")` consumed, as R30 decides. **No Ollama doc confirms `[DONE]`**; it is kept because R30 rules it, and flagged as unverified | none documented | add the completions chunk type |
| LM Studio `Chat` POST /api/v1/chat | `returns_binary()` `lmstudio/mod.rs:78` | JSON reply (`application/json`, default since `stream` defaults to false; a response type must be added) + SSE event enum covering the documented events (`text/event-stream`) | `EventNamed("chat.end")` **+ keep** (documented terminator carrying the aggregated result) | `error` event as an item variant | the definition comment says `stream` defaults to true, but the docs say false: fix the doc comment in Phase 5 |
| LM Studio `DownloadModel` POST /api/v1/models/download | `:101` | **JSON `DownloadModelResponse`** (`lmstudio/types.rs:324`), not SSE | — | — | **deviation from R31's premise**, see "Review agenda" below |
| Anthropic `CreateMessage` POST /messages | `returns_json::<MessageResponse>` `anthropic/mod.rs:87` | JSON `MessageResponse` (`application/json`, note "stream false or omitted") + SSE `MessageStreamEvent` (`text/event-stream`, note "stream true"); add the event types | `EventNamed("message_stop")` consumed | `error` event is an item variant; `ping` is an item variant; unknown future event types decode as `Malformed` items and the stream continues (R10) | no shared error type exists, so no API-level errors (D-19); the generated name `MessageStreamEvent` also exists in the OpenAI module (separate module, no collision) |
| ElevenLabs `StreamSpeech` POST …/stream | `returns_binary()` `elevenlabs/endpoints/speech.rs:15` | `Stream` Bytes, no declared content type (exports `application/octet-stream`) | ConnectionClose | — | R5/R33: no audio media type is documented, so none is declared (conservative choice allowed by R5) |
| ElevenLabs `StreamSpeechWithTimestamps` | `returns_json::<SpeechWithTimestampsResponse>` `:25` | NDJSON of `SpeechWithTimestampsResponse` (SDK evidence of line framing) | ConnectionClose | — | the definition uses camelCase (`audioBase64`), but the docs use snake_case `audio_base64`: fix it in Phase 5 (ElevenLabs is authorized) and log it |
| ElevenLabs CreateSpeech, CreateSoundEffect, GetHistoryItemAudio, DownloadHistoryItems, GetVoiceSampleAudio | Binary | unchanged single Binary shape, no declared content type | — | — | R5 evidence absent; `output_format` query params are not declared on any TTS endpoint today (nothing to preserve; adding them is new scope, logged only) |
| Bitbucket `GetSource` GET …/src/{commit}/{path} | `returns_binary()` `bitbucket/mod.rs:133` | `Directory`: JSON `PaginatedResponse<SourceEntry>` (`application/json`); `File`: Binary fallback (R3c) | — | — | **R32a finding:** no wire tag separates a committed `.json` file (extension-derived `application/json`) from a listing; `Content-Disposition: attachment` is the only file-only header and is not a spec tag. Keep the shape set; record the limitation in the `Directory` note. sniff call sites `bitbucket.rs:176,360` change |
| OpenAI `GetResponse` GET /responses/{response_id} | `returns_json::<BetaResponse>` `openai/mod.rs:7593` | JSON `BetaResponse` (note "`stream` omitted or false") + SSE `BetaResponseStreamEvent` (registered `mod.rs:1688`; note "`stream` is true") | ConnectionClose (no Responses terminator documented; `response.completed` etc. are items) | `BetaResponseErrorEvent` is already a variant | uses the OpenAI `ErrorResponse` only if added at endpoint level; not added (R24) |

**Streaming or multi-shape endpoints outside the migration (logged, not
edited, D-19):**

- OpenAI `CreateChatCompletion`, `CreateCompletion`, `CreateResponse`, `CreateSpeech` (`stream_format`), `CreateTranscription`, `CreateImageEdit`, and `CreateImage` (all with registered stream event types except `CreateCompletion`).
- OpenAI video content download (JSON `Value`, described as streamed).
- UC core-rest backup (binary archive).
- OpenAI Assistants stream event types are registered with no endpoint.

### Wave 2 — Baseline capture

- **Feature identifier:** `2026-10-16-schematic-response-model-graphql`; plan
  `plan.md`; implementer `claude/opus`.
- **Isolated regeneration:** `just migration-check`: **38/38 passed** (it runs
  `definition_baseline::*` and `artifact_drift::*`, which regenerate Rust,
  OpenAPI, and Postman into temporary directories and byte-compare them with
  `schema/`, `openapi/`, and `postman/`). There is no unexplained drift.
- **R24 capture:** the untouched-source and public-type baseline is commit
  **`37095c7ee`**, where `schema/src` and `definitions/src` are committed and
  clean. Rather than copy files, later phases compare against it:
  - definitions outside the migration:
    `git diff --stat 37095c7ee -- schematic/definitions/src ':!schematic/definitions/src/ollama' ':!schematic/definitions/src/lmstudio' ':!schematic/definitions/src/anthropic' ':!schematic/definitions/src/elevenlabs'`
    must list only `bitbucket/mod.rs` (GetSource) and `openai/mod.rs`
    (GetResponse) and their tests.
  - public types: `git diff 37095c7ee -- schematic/schema/src` must change
    only the migration list recorded in Phase 5.
- **Field-specific comparison allow-list** for `definition_baseline` (Phase 5 implements it):
  - `response`: a mechanical rewrite of every historical `{"Json":X}`/`"Text"`/`"Binary"`/`"Empty"` into the one-shape v2 form, applied by `normalize_historical`, so unchanged endpoints compare equal.
  - new members `errors`, `no_declared_errors`, and API `errors`/`graphql`: absent in shipped definitions, so they need no allowance.
  - the endpoints in the migration table: one named `adoptions()` entry each, with a reason.

  Anything else is drift.
- **Baseline L1:**
  - `just test` from `schematic/` (define with all features, gen, definitions, oauth, runtime, plus fresh-output and compile-fail): **2482/2482 passed, 15 skipped**; fresh-output 9/9; compile-fail 1/1.
  - `just test-schema`: **55/55 passed**.
  - Seven tests were marked SLOW (over 5 s) while three read-only inventory agents shared the host: `import_normalization::shipped_openai_query_schema_gate_and_reimports`, `artifact_drift::generated_rust_artifacts_are_up_to_date` (1.9 s alone in `migration-check`), three `e2e_generation::*`, `fresh_output_compile::generated_rest_code_references_the_runtime_and_defines_none_of_it`, and `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`. These were pre-existing and are recorded as load-sensitive; none failed.

### Wave 2 — Timeout spike: skipped, R11a already proved by runtime evidence

The landed transport has **no whole-body timeout** that would kill a healthy
stream:

- reqwest is built with only `connect_timeout` (`runtime/src/transport.rs:143`); `api_client.rs:58` forwards to the runtime's per-request setting.
- the header wait is `tokio::time::timeout_at(expiration.at, dispatch)` (`client.rs:917`).
- after headers, `LiveBody::into_streaming()` switches to `StreamBounds { idle: per-request timeout, deadline: shared deadline }`.

Existing L1 tests already prove the three spike outcomes:

- time to headers: `runtime/tests/l1/timing.rs:162` `a_stalled_head_is_a_timeout_whose_text_redacts_the_query`.
- a stream outliving the send expiration while it delivers, then expiring idle: `:484` `a_stream_outlives_the_send_expiration_while_it_keeps_delivering_then_times_out_when_idle`.
- the shared deadline bounding a stream that never idles: `:516` `the_shared_deadline_bounds_a_stream_that_never_goes_idle`.
- first item before body end: `:453`.

**Ruling: no experiment.**

**Smallest integration change (Phase 3):** today `LiveBody::chunk()` restarts
the idle clock at every **transport read** (`Instant::now() + idle`,
`codec.rs:137`). Plan Rule 7 wants the clock to start at headers and restart at
each **yielded item**. Phase 3 replaces the per-read restart with an idle
anchor:

- set when the head arrives (in `with_stream_bounds`).
- moved by the stream codec when it yields an item (a crate-visible `LiveBody::mark_item()` or equivalent).
- each read waits until `min(anchor + idle, deadline)`.

**Chosen behavior to test:**

1. A frame that is already buffered is yielded without a read, so pending buffered items are never affected by idle timing; yielding one moves the anchor.
2. Bytes that drip in without completing a frame do **not** extend the idle bound.
3. `tokio::time::timeout_at` polls the inner read before its timer, so bytes the server already sent are read even when the caller polled late. Application delay therefore never manufactures an idle timeout while data is waiting, and it never extends the shared deadline.
4. Idle expiry and deadline expiry become distinct `StreamError` variants (today both are `BodyReadError::TimedOut`).

### Wave 2 — Contract freeze

Names are Proposed unless the spec marks them Decided; Phase 2+ may rename a
**builder method** if the code requires it, and must log that. Stored members
and serialized forms below are frozen: a change to them is logged as a
deviation.

**Stored definition model (`schematic-define`; externally tagged serde like
today's `ApiResponse`; every type derives
`Debug, Clone, PartialEq, Serialize, Deserialize`; new public enums are
`#[non_exhaustive]`):**

| Rust | Serialized path | Rules |
| --- | --- | --- |
| `Endpoint::response: Vec<SuccessShape>` | `endpoints[i].response` (array, required, non-empty) | the member keeps its name so the baseline diff is confined to `response` (R22) |
| `SuccessShape { shape, content_type, statuses, json_tag, fallback, note }` | `response[j].shape` required; `content_type` (string, absent means none; `non_null`); `statuses` (array of u16, absent means `[]` = no restriction; skipped when empty); `json_tag` (absent means none); `fallback` (bool, absent means false; skipped when false); `note` (string, absent means none) | add `statuses`/`fallback` to `OMITTED_DEFAULTS`; explicit `null` is rejected for every member |
| `ResponseShape` | `{"Json":<TypeRef>}`, `"Text"`, `"Binary"`, `"Empty"`, `{"Stream":{"framing":…,"end":…,"keep":bool}}` | `keep` absent means false (consumed); skipped when false |
| `StreamFraming` | `{"Sse":<TypeRef>}`, `{"Ndjson":<TypeRef>}`, `"Bytes"` | Bytes uses the shape's `content_type` |
| `StreamEnd` (D-4) | `{"DataLineEquals":"[DONE]"}`, `{"EventNamed":"message_stop"}`, `{"FieldEquals":{"field":"done","value":true}}`, `"ConnectionClose"` | `end` is **required** in serialized form (no published default; the builder default is ConnectionClose) |
| `JsonTag` (R3b) | `{"Field":{"name":"type","value":<scalar>}}`, `"Array"`, `"Object"` | stored explicitly; the builder writes an inferred Array/Object only when the endpoint has two or more JSON shapes sharing status and media, so bundles never need the Rust registry |
| `ErrorEntry { status, shape, content_type }` | `{"status":…,"shape":…,"content_type"?}` | `shape` is a `ResponseShape` other than `Stream` (R16) |
| `StatusMatch` (D-6) | `{"Exact":404}`, `{"Range":{"lo":400,"hi":499}}` | `StatusMatch::class(4)` builds `400..=499`; `lo <= hi`, both in 400–599 |
| `RestApi::errors: Vec<ErrorEntry>` | `errors` (absent means `[]`, no API defaults; skipped when empty) | |
| `Endpoint::errors: Vec<ErrorEntry>`, `Endpoint::no_declared_errors: bool` | `errors` (absent means `[]`, inherit), `no_declared_errors` (absent means false) | opt-out plus entries is a validation error (R17); `ErrorSet::none()` sets the flag (R24) |
| `RestApi::graphql: Option<GraphQlSurface>` | `graphql` (absent means no GraphQL surface; `non_null`) | |
| `GraphQlSurface { address: Vec<AddressRule>, operations: Vec<GraphQlOperation> }` | `graphql.address` required non-empty; `graphql.operations` (absent means `[]`) | |
| `AddressRule` (D-10) | `{"ReplaceSuffix":{"suffix":"/api/v3","with":"/api/graphql"}}`, `{"Append":"/graphql"}` | |
| `GraphQlOperation { id, description, kind, document, variables, data, operation_name, errors, no_declared_errors, pagination }` | `kind`: `"Query"`/`"Mutation"`; `variables`: `"Unit"` or `{"Typed":<TypeRef>}` (required); `data`: `TypeRef` (required); `operation_name` absent means omitted on the wire, while `""` is preserved; `pagination`: the existing `pagination::Collection` form | D-9 constructors `GraphQlOperation::query(id, document)` / `::mutation(..)` |

**Builder surface (Proposed, widening 4a in place; every existing call
compiles unchanged):**

- `.returns(Success)` may repeat; the `successes > 1` rule at `builders.rs:176` is replaced by R1/R3 validation.
- `Success::{json(desc), text(), binary(), empty()}` gain `Success::{sse(desc), ndjson(desc), bytes()}`.
- modifiers `.content_type(..)`, `.status(u16)`, `.tagged(name, value)`, `.json_array()`, `.json_object()`, `.fallback()`, `.note(..)`, `.ends_at_data_line(..)`, `.ends_at_event(..)`, `.ends_when_field(path, value)`, and `.keep_end()`.
- `ErrorSet::none()` plus `ErrorSet::new().on(StatusMatch, ErrorShape)`. `ApiResponse` survives only until Phase 5 switches storage, then is removed (spec Design "Replaced": logged here; no sugar is kept, because `Success` already is the sugar).

**Runtime interfaces (`schematic-runtime`, `ABI` 5 → 6 when the generated
code first needs them):**

- `ErrorKind::UnexpectedContentType` (new variant of the `#[non_exhaustive]` enum, multi-shape selection only).
- `StreamError` (`#[non_exhaustive]`): `Malformed { raw: Bytes }`, `EndedEarly`, `Interrupted { pending: Bytes }`, `IdleTimeout`, `DeadlineExceeded`, `FrameLimitExceeded { limit: usize, prefix: Bytes }` (prefix bounded).
- Codecs `Sse<T, End>`, `Ndjson<T, End>`, and `ByteStream`, where `End` is a generated marker carrying a `const` end spec; outputs `ItemStream<T>: Stream<Item = Result<T, StreamError>> + Send + 'static` and `ByteStream: Stream<Item = Result<Bytes, StreamError>>`. None implements `SupportsHooks`.
- Multi-shape selection is a runtime function over a generated `const` route table (status set, normalized media, JSON tag, fallback, empty) returning the chosen alternative, or `UnexpectedContentType`.
- Error lookup is a runtime function over a generated `const` table of `(StatusMatch, alternative)` returning `Declared`, `MalformedError`, or `Undeclared`.
- Managed builder setting **`max_frame_bytes(usize)`**, default `DEFAULT_MAX_FRAME_BYTES = 16 MiB` (D57 item 17; the name is Proposed).
- `GraphQl<D>` (implements `SupportsHooks`), `GraphQlFailure<D> { errors: Vec<GraphQlError>, partial_data: Option<D> }`, `GraphQlError { message, path: Option<Vec<PathSegment>>, locations: Option<Vec<Location>>, extensions: Option<Value>, other: Map }`, and `GraphQlOperationError<D, H> { GraphQl(..), Http(H) }`.

**Generated names:**

- success enum `<EndpointId>Response` (D-15); variants are author-named or derived from the shape kind (`Json`, `Text`, `Binary`, `Empty`, `Stream`, `JsonProblem`-style by content type).
- failure enum `<EndpointId>Error` (D-23).
- R3b groups and Bitbucket name their variants (`Directory`, `File`).
- **Known collision:** Ollama's `GenerateResponse` and `ChatResponse` (the buffered-reply types, unregistered today) equal the D-15 enum names of `Generate` and `Chat`. Phase 2's naming validation must report this, and Phase 5 resolves it by registering those two types under non-colliding schema names (for example `GenerateCompletion`, `ChatCompletion`). They are not public generated types today, so no existing public type changes. D-15 stays as written.
- No other migrated endpoint collides (checked: `CreateMessageResponse`, `ChatCompletionsResponse`, `CompletionsResponse`, `GetSourceResponse`, `GetResponseResponse`, `PullModelResponse`, `PushModelResponse`, `CreateModelResponse`, `StreamSpeechWithTimestampsResponse` do not exist). LM Studio `Chat` stays single-module distinct from Ollama.

**Diagnostics (frozen wording pattern):** every validation and import diagnostic
names `API <name>`, `endpoint <id>` or `operation <id>`, and the offending
statuses, media types, or member path. GraphQL export skips (R42) read
`<API> operation <id>: GraphQL operation skipped in <OpenAPI|Postman> output`
and are warnings, never errors.

**Bundle format outcomes:** version stays 1 (spec refresh, adopted). The new
members ride the existing strict loader. The default outcome for **every**
cell is: a located `BundleError` (JSON pointer), no definition. The only
exceptions, whose outcome is a successful load:

- `response[j].statuses` absent or `[]`: no restriction.
- `fallback` and `keep` absent: false.
- `content_type`, `json_tag`, and `note` absent: none.
- API/endpoint `errors` absent or `[]`: no defaults or inherit, while still inheriting API entries.
- `no_declared_errors` absent: false.
- `graphql` absent: no surface.
- `graphql.operations` absent or `[]`: no operations.
- `operation_name` absent: omitted.
- `operation_name: ""`, a `DataLineEquals("")`, and a `FieldEquals` string value `""`: allowed as data.
- `document: ""`: sent unchanged.
- `variables: "Unit"`: unit.

Explicit `null` is rejected everywhere outside schema payloads (existing
`reject_nulls`). Dangling `TypeRef`s in success/error/stream/variables/data are
rejected by `validate()` with API/endpoint-or-operation/type context.

**Frozen matrix columns:** these are the plan's tables, expanded to the exact
serialized members above. The plan's per-column outcomes stand, with the
exceptions just listed:

- Success: `response`, `response[j].shape`, `.content_type`, `.statuses`, `.statuses[k]`, `.json_tag`, `.json_tag.Field.name`, `.json_tag.Field.value`, `.fallback`, `.note`.
- Stream: `.shape.Stream.framing`, `framing.Sse`/`framing.Ndjson` (TypeRef), `.shape.Stream.end`, `end.DataLineEquals`, `end.EventNamed`, `end.FieldEquals.field`, `end.FieldEquals.value`, `.shape.Stream.keep`.
- Errors: API `errors`, endpoint `errors`, `errors[k].status.Exact`, `errors[k].status.Range.lo`, `.Range.hi`, `errors[k].shape`, `errors[k].content_type`, `no_declared_errors`.
- GraphQL: `graphql`, `graphql.address`, `address[k].ReplaceSuffix.suffix`, `.ReplaceSuffix.with`, `address[k].Append`, `graphql.operations`, `operations[k].id`, `.kind`, `.document`, `.variables`, `.variables.Typed`, `.data`, `.operation_name`, `.errors`, `.no_declared_errors`, `.pagination` (pagination's own inner members reuse the existing pagination matrix unchanged).
- Compatibility checks run through `validate()`: data/event end on NDJSON or Bytes, field end on Bytes, `keep` with ConnectionClose, a status outside 200–299 in `statuses`, an error range outside 400–599 or with `lo > hi`, two fallbacks, a fallback with `content_type`, and a fallback on a single-shape endpoint.

**Real-tool positive fixtures and test locations:**

| Matrix | Positive fixture | Walker location | Feature |
| --- | --- | --- | --- |
| OpenAPI JSON + YAML (two loaders: `serde_json` and `serde_yaml_ng`, both through `openapi/import/query_paths.rs::check_text` then `openapiv3`) | the Schematic-exported OpenAPI for a fixture API with JSON+SSE, discriminator `oneOf`, a `*/*` fallback, 204 empty, and exact plus class errors; exported by the real exporter in the test, then YAML re-serialized | new `define/tests/response_import_matrix.rs`; the response checks extend `check_text`, the loader that already rejects duplicate keys | `openapi` (`all-features = true` in `[package.metadata.ci.tests]`) |
| Definition bundle JSON | the committed bundle-tool fixture under `define/tests/fixtures/bundles/`, regenerated from `bundle_support`'s typed API extended with the new members | extend `bundle_json_matrix()` in `define/tests/definition_bundles.rs` | none |
| Shipped bundles | the real `schematic-definitions` (four migrated APIs, R32a/R32b) | `gen/tests/l1/definition_bundles.rs` | none |

**R1–R44 ledger** (task means plan phase/wave; "define", "runtime", and "gen"
are package L1 through `just test` from `schematic/`):

| Req | Task | Public test boundary | Verify |
| --- | --- | --- | --- |
| R1, R3, R3a, R3b, R3c, R23 | P2 W4 Shape validation; P3 W5 Response dispatch | define validation (built `RestApi` / `DefinitionError`); runtime mock-server envelope | `just test` |
| R2, R20 | P4 W7 REST generation | gen output assertions plus fixture crate compile | `just test` (`test-fresh-output`) |
| R4, R5 | P4 W7 Consumer adapters; P6 W12 OpenAPI | gen export over the real ElevenLabs API | `just test` |
| R6–R11, R11a, R11b | P3 W5 Stream codecs; W6 | runtime mock server (`tests/l1/`), Send/ownership compile checks | `just test`, `just test-compile-fail` |
| R11c | P3 W6; P6 W13 | runtime SSE error-event item; generated provider clients | `just test` |
| R12 | P3 W5/W6; P4 W8 | trybuild compile-fail (runtime `slow_`), definitions source scan, global-hook mock | `just test-compile-fail`, `just test` |
| R13a | P2 W4; P4 W7; P6 W13 | define duplicate rule; gen rustdoc/OpenAPI note; Anthropic mock records `stream` | `just test` |
| R16–R19 | P2 W3/W4; P3 W5 | define unit (lookup and overlap diagnostics); runtime mock | `just test` |
| R21, R27 | P4 W7; P6 W12 | gen export plus `openapiv3` parse | `just test` |
| R22, R24, R25 | P5 W9/W11 | `migration-check`, `artifact_drift`, `git diff 37095c7ee` capture, schema compile and lint | `just migration-check`, `just test-schema`, `just lint-schema` |
| R26 | P4 W7; P6 W12 | define serialization of `SchematicOpExtension` | `just test` |
| R28 | P4 W7; P6 W12 | define public import, JSON and YAML matrices | `just test` |
| R29 | P6 W12 | gen Postman goldens | `just test` |
| R30–R33 (incl. R32a/b) | P5 W10; P6 W13 | definitions tests plus generated-client mocks | `just test` |
| R34 | P5 W11; P8 W17 | `just test sniff` (Bitbucket `get_source` call sites); `biscuit-speaks`, `model-citizen`, and homelab builds (no forced change expected) | root `just test {pkg}` |
| R35–R39, R41 | P2 W3/W4; P3 W5; P4 W7; P7 W15 | define unit, trybuild for wrong variables, generated-client mocks | `just test`, `just test-compile-fail` |
| R40 | P7 W14 | generated pager two-page mock; compile-fail for unauthored knob | `just test`, `just test-compile-fail` |
| R42 | P6 W12 | gen export diagnostics | `just test` |
| R43 | P7 W14 | gen test-only operation over sniff's eight fixtures (`include_str!` in each `#[test]`; `sniff/lib/src/remote/check_status.rs` declared in gen `[package.metadata.ci.tests] source-inputs` if read) | `just test` |
| R44 | P2 W3; P5 W9; P7 W15 | define/gen bundle round trip and matrix, byte-identical reserialization | `just test` |

No feature flags beyond define's `openapi` are needed; runtime trybuild cases
are `slow_` L1 run by `test-compile-fail`.

### Adopted defaults (D-4–D-31; D-1–D-3 withdrawn)

Each is adopted as the spec writes it unless noted. The phase listed is where
it takes effect; evidence is the test named in the ledger.

| Default | Choice | Alternatives considered | Reason | Phase |
| --- | --- | --- | --- | --- |
| D-4 | `StreamEnd::{DataLineEquals, EventNamed, FieldEquals, ConnectionClose}`, builder `.ends_at_data_line/.ends_at_event/.ends_when_field`, implicit close | a single string matcher | spec default | 2 |
| D-5 | `StreamError` as frozen above; idle and deadline expiry split into `IdleTimeout` / `DeadlineExceeded` | a single `TimedOut` | R11a requires the distinction | 3 |
| D-6 | `Exact` / `Range { lo, hi }`, `class(n)`; OpenAPI `4XX` imports as a class; `default` gets a diagnostic | `RangeInclusive` serde `{start,end}` | named members read better in bundles | 2 |
| D-7 | specificity across the merged table; identical match at endpoint replaces API | endpoint-first | spec default; stays on the review agenda | 2 |
| D-8 | 2xx success, else failure; declared matches 400–599 | — | spec default | 2 |
| D-9 | `GraphQlOperation::query/mutation` | — | spec default | 2 |
| D-10 | `Vec<AddressRule>`; GitHub preset `[ReplaceSuffix(/api/v3 → /api/graphql), Append(/graphql)]` | — | spec default | 2 |
| D-11 | client-builder full-URL override checked against cross-origin policy | — | spec default | 4 |
| D-12 | `relay_forward(root)` / `.with_edges()` / backward leg as **new preset sugar** over `PageStrategy::cursor` + `InputParam::Body` (no Relay preset landed) | a new strategy variant | reuse the landed model | 7 |
| D-13 | no batching; out of scope | — | spec default | — |
| D-14 | Content-Type selection with 204-empty first, R3a statuses, R3b tags, R3c fallback | — | spec default | 3 |
| D-15 | `<EndpointId>Response`; Ollama buffered types renamed to avoid collision | rename the enum | keeps the default and changes no public type | 4/5 |
| D-16 | no content-type check for single-shape; SSE/NDJSON do not require a matching Content-Type | — | spec default | 3 |
| D-17 | Decided: `Bytes` framing | — | — | 3 |
| D-18 | Decided: consumed by default, `keep` delivers; applied to Ollama `done`, Ollama pull-family `status=success`, and LM Studio `chat.end` | — | evidence above | 5 |
| D-19 | migrate the four APIs plus R32a/R32b; others logged above; **no API-level default error lists added** (none of Ollama, LM Studio, Anthropic, or ElevenLabs has a shared error type; adding one to OpenAI would change 258 failure types, against R24) | add Anthropic errors | D-19 as written | 5 |
| D-20 | errors stay endpoint-level on import | hoisting | spec default | 4/6 |
| D-21 | `operationName` omitted unless set | — | spec default | 4 |
| D-22 | codecs and dispatcher in runtime | in generated code | spec default | 3 |
| D-23 | `<EndpointId>Error` | — | spec default | 4 |
| D-24 | serde serialization in `x-schematic` | — | spec default | 4 |
| D-25 | class ranges as `4XX`/`5XX` only when lookup is preserved, else expanded | — | spec default (agenda item) | 4/6 |
| D-26 | hooks removed and logged; **inventory found none to remove** | — | evidence above | 5 |
| D-27 | `()` omits `variables` | — | spec default | 3/4 |
| D-28 | test-only parity definition, no sniff edit | — | spec default | 7 |
| D-29 | exporters skip GraphQL with a warning | — | spec default | 4/6 |
| D-30 | error `Debug` per D22; GraphQL errors not redacted | — | spec default | 3 |
| D-31 | one text or binary fallback, no content type, exported `*/*` | wildcard ranges | spec default | 2 |

### Review agenda entries raised in Phase 1

1. **LM Studio `DownloadModel` is not a stream (deviation candidate, R31).**
   R31 says both `// SSE streaming` endpoints become SSE shapes, but LM Studio's
   docs say download returns a JSON job object polled through a status
   endpoint.
   - Adopted (default per plan Rule 10): declare it as JSON
     `DownloadModelResponse` and log it.
   - Alternative: SSE with connection close, with no evidence.
   - Stand-in test: the Phase 5 definition test asserts the JSON shape.
2. **LM Studio `Chat` needs two shapes.** `stream` defaults to false, so the
   default reply is JSON, and a lone SSE shape would misdecode it.
   - Adopted: JSON plus SSE (R13a), ending at `chat.end` with `keep`.
   - Also fix the doc comment that says the default is true.
3. **Ollama OpenAI-compatible `[DONE]`** is ruled by R30 but not documented by
   Ollama. Kept because the spec decides it; flagged as unverified provider
   behavior.
4. **Ollama pull/push/create** end at `status == "success"` with `keep`. R30
   names only `done=true`, but these have no `done` field; ConnectionClose
   would hide truncation.
5. **ElevenLabs**: no audio media type is documented, so none is declared (R5
   conditional). With-timestamps becomes NDJSON on SDK evidence, and its
   camelCase field names contradict the provider's snake_case (a definition
   defect to fix in Phase 5).
6. **Bitbucket R32a**: no wire tag separates a `.json` file from a listing
   (only `Content-Disposition: attachment` differs, and it is not a spec
   tag). Keep the shape set and note the limitation.
7. **Anthropic unknown event types** become `Malformed` items (stream
   continues); the provider says new types "may be added".
8. **Counts**: 470 untouched endpoints (not "roughly 530"); 14 stranded
   Ollama types (not ten).
9. **D-15 collision** with Ollama `GenerateResponse` / `ChatResponse`,
   resolved by renaming the unregistered types (see Contract freeze).
10. **Dead `ResponseKind` generator code** contradicts the spec's statement that
    spec 2 removed it; removed with its readers in Phase 4/5.

### Phase 1 gates

See "Phase 1 gate result" below for the scope calculator's output.

### Phase 1 gate result

The scope calculator was run from the repository root after all edits (log,
plan checkboxes, frontmatter) were saved:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `packages: []`, `change_class: documentation`. The gates could be
skipped, but both were run on this unchanged source tree anyway:

- `just test`: 2482/2482 passed, plus fresh-output 9/9 and compile-fail 1/1 (baseline run above).
- `just lint`: clean.

No dependent-package tests apply (no code changed). No cross-OS check applies
(no platform-sensitive change). The `schematic-define` skill needed no update:
nothing it describes changed in Phase 1.

## Phase 2

Run of 2026-10-22. Tree: HEAD `37095c7ee` plus the Phase 1 refresh edits
already staged (not touched). Implementer: `claude/opus`. All work was done in
the main session, sequentially: the stored types, rules, and tests share three
files (`builders.rs`, `types.rs`, `components.rs`), so parallel agents would
have contended for them. No subagent ran.

### What landed

**Stored vocabulary (`schematic-define`), frozen forms unchanged from the
Phase 1 contract freeze:**

- `response.rs`: `ResponseShape` (`Json`, `Text`, `Binary`, `Empty`,
  `Stream { framing, end, keep }`), `StreamFraming` (`Sse`, `Ndjson`, `Bytes`),
  `StreamEnd` (`DataLineEquals`, `EventNamed`, `FieldEquals { field, value }`,
  `ConnectionClose`), `JsonTag` (`Field { name, value }`, `Array`, `Object`),
  `SuccessShape { shape, content_type, statuses, json_tag, fallback, note }`,
  `default_media_type`, `normalize_media_type`, and
  `validate_success_shapes`. All new public enums and `SuccessShape` are
  `#[non_exhaustive]`; all derive `Debug, Clone, PartialEq, Eq, Serialize,
  Deserialize`. `ApiResponse` is unchanged; `SuccessShape::legacy_response()`
  and `From<ApiResponse> for SuccessShape` convert between the two.
- `error_table.rs` (new): `StatusMatch` (`Exact`, `Range { lo, hi }`,
  `exact`/`range`/`class`/`contains`/`width`, `Display` as `404` or
  `400-499`), `ErrorEntry { status, shape, content_type }`, `ErrorTable`
  (`resolve`, `lookup`, `entries`), and `validate_error_entries`.
- `graphql.rs` (new): `GraphQlSurface { address, operations }`,
  `AddressRule` (`ReplaceSuffix`, `Append`, `github()` preset),
  `OperationKind`, `GraphQlVariables` (`Unit`, `Typed`), `GraphQlOperation`
  (frozen members), and `GraphQlOperationBuilder` with typestate for
  `describes`, `variables`/`no_variables`, and `data`.
- New stored members, all omitted at their default so every shipped
  definition serializes byte-identically: `RestApi::errors`,
  `RestApi::graphql` (`non_null`), `Endpoint::errors`,
  `Endpoint::no_declared_errors`. Resolvers `RestApi::effective_errors` and
  `RestApi::effective_operation_errors`. Seam `Endpoint::success_shapes()`
  (today one shape from `response`).
- `DefinitionError::InvalidOperation { api, operation, message }`, rendered
  `{api} operation {id}: {message}`, for operation rules and unresolved
  operation references.
- Builders: `Success` gains `sse`, `ndjson`, `bytes`, `content_type`,
  `status`, `tagged`, `json_array`, `json_object`, `fallback`, `note`,
  `ends_at_data_line`, `ends_at_event`, `ends_when_field`, `keep_end`.
  New `ErrorShape::{json, text, binary, empty}.content_type(..)`;
  `ErrorSet::new().on(StatusMatch, ErrorShape)` beside `ErrorSet::none()`.
  `RestApiBuilder::graphql(rules)` and `.operations(ops)`. The prelude
  exports `AddressRule`, `ErrorShape`, `GraphQlOperation`, `StatusMatch`.
- Bundles: `OMITTED_DEFAULTS` gains `errors`, `no_declared_errors`,
  `operations`, `statuses`, `fallback`, `keep`. References in API errors,
  endpoint errors, operation variables, data, and operation errors are
  checked by `validate_references` (so bundle load rejects a dangling one).
- `schematic-gen` validation: GraphQL operations join the module group's
  generated naming space (request struct `{id}{suffix}` and call type), their
  data, variables, and error types are checked for shadowing, operation ids
  must be Rust identifiers, and the D-15/D-23 enum names `{id}Response`
  (several success shapes) and `{id}Error` (several distinct error shapes in
  the effective table) are emitted names. Endpoint error types are now
  "referenced types" too, so a registered `GetError` shadowed by the
  `GetError` failure enum is a `generated-name-collision`.

**Rules (validation lives beside the types; endpoint build and API build
both run them, so bundle load runs them too):**

- R1/R3/R3a/R3b/R3c/R23 in `validate_success_shapes`: nonempty set; concrete
  media types (case and parameters ignored, wildcards rejected); empty shape
  has no media type; statuses in 200–299 and unique per shape; in a
  multi-shape set, 204 belongs to the one empty shape only; per candidate set
  (each declared status, then the status-less shapes), shapes sharing a
  normalized media type must be JSON separated by a common discriminator
  field with distinct scalar values, or exactly one array and one object;
  one text/binary fallback, without a content type, never on a single shape;
  JSON tags only on JSON shapes, top-level names, scalar values; stream end
  compatibility (data/event only on SSE, field on SSE/NDJSON, bytes only
  connection close, `keep` needs a terminator, field paths without fan-out,
  nonempty event names, scalar values; `DataLineEquals("")` and `""` field
  values allowed).
- R16–R18 in `error_table.rs`: matches within 400–599 and ordered; no stream
  error shape; valid media; no identical match twice at one level; no
  equal-width overlapping ranges at one level or in the resolved table;
  opt-out plus entries rejected (R17); lookup exact, then narrowest range,
  across the merged table, an identical endpoint match replacing the API
  entry in place (D-7).
- R35/R39/R40 in `graphql.rs`: nonempty ids/descriptions; unique operation
  ids, distinct from endpoint ids; typed variables must be a named type or
  map; error policy as for endpoints; address needs a rule, each a `/`-led
  path of nonempty segments without query or fragment; a base URL with a
  query or fragment is rejected when a GraphQL surface exists; pagination
  must be explicit, not next-link, without follow-up/session/alternates, with
  an item type and typed variables, and every pager input a body path under
  `variables`.

### Adopted defaults and decisions

- **adopted default: success shapes are validated now and stored in Phase 5**
  (source: plan "Phases 2–4 are additive", Phase 5 "widen builder lowering in
  place"; spec Suggested phases item 5). `Endpoint::response` stays
  `ApiResponse`. Endpoint build validates the whole shape set, then stores a
  lone shape without modifiers and refuses anything else with
  `... are stored once Endpoint::response holds response model v2 shapes`.
  Nothing is dropped silently. Alternative rejected: a temporary second
  stored member, which the plan forbids ("Do not introduce a persistent
  second public response system") and which would perturb every baseline.
- **adopted default: declared errors and the GraphQL surface are stored now.**
  They are additive members omitted at their defaults, so no baseline,
  artifact, or shipped bundle changes; storing them lets Phase 2 apply the
  frozen bundle matrix to them (plan Wave 4: "apply frozen matrix cases for
  these types"). Until Phases 4/7 generate from them, the generated client
  ignores them; no shipped definition declares any.
- **adopted default: JSON tag inference.** `Success::json(desc)` infers
  `Array` for a `Vec` and `Object` for a map from the `TypeRef`;
  `returns_json::<T>` also infers `Object` when `T`'s schemars schema has
  `"type": "object"`. An inferred tag is applied only when another JSON shape
  shares the shape's media type and an applicable status (empty status sets,
  or intersecting ones); a declared tag is never replaced. `Success::json`
  with a nominal struct infers nothing because a `TypeDesc` carries no schema;
  authors use `.json_object()` (source: R3b "inferred from the registered type
  when it is a sequence or a struct").
- **adopted default: API-level `default_errors(ErrorSet::none())` adds
  nothing** (source: plan Input Robustness Matrix, "an empty API error table
  is not endpoint opt-out"); the existing call site in
  `define/tests/builders.rs` keeps its meaning.
- **adopted default: a multi-shape 204 rule.** A non-empty shape declaring 204
  or an empty shape declaring another status is rejected only when the
  endpoint has several shapes (R3: empty is selected by 204; R3a: single-shape
  statuses are documentation).
- **decision: `ErrorSet` and `ErrorTable` have no `Default`.** The
  `public_default_set_equals_documented_allow_list` guard forbids new public
  `Default` impls; `ErrorSet::new()` is written out with
  `#[allow(clippy::new_without_default)]`.
- **decision: `DefinitionError::InvalidOperation`** is a new variant so
  operation diagnostics read `operation {id}` (frozen diagnostic pattern)
  rather than `endpoint operation {id}`. No match on `DefinitionError` outside
  `schematic-define` is exhaustive over its variants.
- **decision: reference-check order.** API error references are checked before
  endpoint references, so the existing bundle cell "schemas empty" now
  reports `Fixture endpoint <api>: unresolved schema reference "Problem"`
  (the fixture's API errors name `Problem`). Updated that cell.
- **deviation (builder):** none of the frozen builder names changed. Added
  `ErrorShape` (the freeze named `ErrorShape` as the `.on` argument without
  defining it) and `RestApiBuilder::graphql`/`operations` (the freeze named
  no API-level GraphQL builder).

### Retired test assertion

`define/tests/cross_origin_policy.rs::endpoints_support_cross_origin_override_and_no_graphql_export`
asserted that the crate root exports no GraphQL name. The runtime-envelope
log records it as frozen until "spec 5 (GraphQL primitive)", which is this
spec. Its GraphQL half was removed and the test renamed
`endpoints_support_cross_origin_override`; its cross-origin assertions are
unchanged.

### Requirement-to-test mapping (Phase 2 scope)

All tests are L1 (no tier marker), compiled by default `tests/*.rs` targets
of `schematic-define` (autotests on; `all-features = true` in its CI
metadata) or by `schematic-gen`'s lib unit tests. Boundary: the crate's public
API (built `Endpoint`/`RestApi`, `DefinitionError` text, `DefinitionBundle`
load results), fidelity F1 in-process; no CLI, filesystem, or network
boundary is crossed by this phase's behavior.

| Req | Behavior | Test |
| --- | --- | --- |
| R1 | empty set, concrete media, wildcard rejection, empty shape has no media | `define/tests/response_vocabulary.rs::{a_shape_set_must_be_nonempty_and_each_media_type_concrete, media_types_normalize_case_and_parameters_and_reject_wildcards}` |
| R3 | distinct media separable; same normalized media (default vs declared, case/params) ambiguous; one empty shape | `::{shapes_with_distinct_media_types_or_json_plus_empty_are_selectable, shapes_sharing_a_normalized_media_type_are_ambiguous}` |
| R3a | 200/202 JSON separable; same status+media fails; 199/300 and duplicates fail; 204 rules; single-shape statuses are docs | `::declared_statuses_separate_json_shapes_and_are_bounded` |
| R3b | discriminator, array/object; duplicate values, mixed tags, missing tags, two arrays, three shapes, non-JSON tag, nested name, non-scalar/null value | `::json_tags_separate_shapes_sharing_status_and_media_type`; inference through builders: `::several_shapes_are_validated_at_build_with_inferred_json_tags` |
| R3c | one fallback; second, single-shape, with content type, JSON fallback rejected | `::one_text_or_binary_fallback_takes_unmatched_media_types` |
| R23 | stream end/framing compatibility, keep, field path and value | `::stream_end_conditions_must_suit_their_framing`; builder misuse (`ends_*`/`keep_end` on non-streams) in `::builder_keeps_single_plain_successes_and_refuses_what_storage_cannot_hold` |
| R13a | `(method, path)` duplicate rule unchanged | existing `gen/tests/l1/output_validation.rs::same_api_duplicate_route_blocks_every_output` (not modified) |
| R16 | domain, ordering, class(3), stream error shape, empty with media, wildcard, duplicate match | `::error_entries_are_bounded_typed_and_unambiguous_with_context` |
| R17 | overlay replaces identical match, opt-out empties, opt-out + entries fails at endpoint build and bundle load | `::{error_tables_overlay_api_defaults_and_resolve_by_specificity, declared_errors_reach_the_effective_table_and_references_are_checked, error_entries_are_bounded_typed_and_unambiguous_with_context}` |
| R18 | lookup matrix (exact > narrower > wider, endpoint range vs API exact); equal-width overlap at one level and across levels with API/endpoint/status context | same three tests |
| R24 | single plain successes still store as before; unused members omitted; `ErrorSet::none()` serializes `no_declared_errors: true` | `::{builder_keeps_single_plain_successes_and_refuses_what_storage_cannot_hold, new_definition_members_are_omitted_until_used_and_keep_their_frozen_forms}`; existing `builders.rs` and `definition_serde.rs` suites |
| R35 | typestate (compile_fail doctest on `GraphQlOperationBuilder`), registration, unregistered data/variables, unsupported map key, ids, variables shape, empty/invalid document builds | `define/tests/graphql_definitions.rs::{an_operation_registers_its_types_and_belongs_to_the_api, an_unregistered_data_or_variables_type_is_a_validation_error_naming_the_operation, operation_facts_ids_and_variables_shape_are_checked}` |
| R35 naming | operation vs endpoint request/call names, data type shadowing, operation id identifier, `{id}Error` enum shadowing | `gen/src/validation.rs::tests::{an_operation_shares_the_endpoint_naming_space, a_failure_enum_exists_for_several_distinct_error_shapes_and_may_shadow_a_type}` |
| R39 | address rules (6 invalid paths × 2 rules), empty list, query base URL | `graphql_definitions.rs::address_rules_are_ordered_segment_paths_on_a_query_free_base` |
| R40 | Relay cursor into `variables.*`; query/body-root/`variables`-only/size inputs, inherit, unpaginated, missing item type, next-link, bad selector, unit variables | `::relay_pagination_writes_variables_and_reads_from_data` |
| R37 (policy) | operation error overlay and opt-out | `::an_operation_error_policy_overlays_api_defaults_or_opts_out` |
| R44 | frozen JSON forms, read back, null rejection, required `end`; bundle round trip byte-identical; matrix | `response_vocabulary.rs::new_definition_members_are_omitted_until_used_and_keep_their_frozen_forms`, `graphql_definitions.rs::graphql_members_keep_their_frozen_serialized_forms`, `define/tests/definition_bundles.rs::{committed_fixture_is_the_bundle_tool_output, bundle_files_round_trip_byte_equal_through_repeated_writes_and_reads, bundle_json_matrix}` |

**Deferred to the phase that owns the boundary (not Phase 2 gaps):** the
compile-time "wrong variables type at the call site" check needs generated
request code (plan Wave 8 lists it); stream item-type reference checks
(R23 "missing registered item type") and the success-shape bundle columns run
once `Endpoint::response` stores shapes (Phase 5, "Update self-contained
bundle loading"); runtime selection, lookup decode, and GraphQL wire behavior
are Phases 3–4.

### Input robustness matrix (definition bundle, JSON only)

The fixture (`define/tests/bundle_support/mod.rs`, committed as
`define/tests/fixtures/bundles/fixture.json`, regenerated by the bundle tool
path and its literals reviewed: names only, no secrets) now carries API
errors (class 5 JSON, exact 404 text), endpoint errors (exact 404 empty,
range 420–429 text/html), an opted-out endpoint, the GitHub address rules,
a paginated query with typed variables, data, operation name, and errors,
and an opted-out mutation with unit variables and an empty document.
Component order is unchanged up to `Thing` so existing index-based cells hold.

Default outcome for every cell: a located `BundleError`, no definition.
Exceptions (load): API `errors` absent or `[]`; endpoint `errors` absent or
`[]` (still inherits); `no_declared_errors` absent or `false`; `graphql`
absent; `operations` absent or `[]`; `operation_name` absent or `""`;
`document` `""` or unparsable; `pagination` absent. 101 cells were added to
`bundle_json_matrix()`; each row in the freeze's error and GraphQL column
lists has its control (a consequence check through `effective_errors`,
`effective_operation_errors`, or the loaded value), absent, null, wrong type,
one/all invalid members where the field is a collection, empty, nested
required members (`status`, `lo`, `hi`, `shape`, `suffix`, `with`, `Append`,
`id`, `kind`, `document`, `variables`, `data`), semantic rules (domain,
reversed, equal-width overlap at both levels, duplicate match, stream error
shape, dangling and unregistered-nested references, opt-out with entries,
pager input placement, item type), and duplicate keys in the error, GraphQL,
operation, and opt-out objects. Trailing content and other document shapes
are shared by the one loader and already covered by the existing document
cells. Five existing cells changed only their expected line numbers or the
first unresolved name (noted above).

**Smell grep** over the changed readers (`response.rs`, `error_table.rs`,
`graphql.rs`, `types.rs` new members, `bundle.rs`): every `#[serde(default)]`
on a new load-bearing member is either a `Vec` whose absence and `[]` mean
the same (documented above) paired with `OMITTED_DEFAULTS`, a `bool` whose
absence means `false`, or an `Option` paired with `non_null` so `null` is
rejected; no `filter_map(.. as_str())`, `unwrap_or_default()`, or `.ok()`
on a parsed value was added. `SuccessShape::media_type` uses
`unwrap_or_else` only for a declared type that validation has already
rejected (it is never reached for a stored, valid shape).

### Finalized public names and construction seams (for Phases 3–5)

- Read success shapes through `Endpoint::success_shapes()`; Phase 5 makes it
  return the stored `Vec<SuccessShape>` and folds stream item references into
  `Endpoint::json_references()`.
- Lower a shape with `SuccessShape::media_type()` (normalized, `None` for
  empty and fallback) and `response::default_media_type`.
- Effective errors: `RestApi::effective_errors(&endpoint)` /
  `effective_operation_errors(&operation)` → `ErrorTable::{entries, lookup}`.
  Distinct-shape counting for D-23 compares `(shape, content_type)`, as in
  `gen/src/validation.rs::endpoint_enum_names`.
- Builder lowering: `EndpointBuilder::build()` (the `ok_or_else` refusal) is
  the one line Phase 5 replaces with storing `shapes`.

### Phase 2 gates

The scope calculator, run from the repository root after all edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `packages: [schematic-define, schematic-gen, test-toolkit, darkmatter]`,
`change_class: package`. So both gates ran. `darkmatter` is selected because
its schema-corpus tests read `.claude/skills/*/SKILL.md`; `test-toolkit`
because its archive guard scans every test source.

- `just lint` (schematic): clean, after one clippy fix
  (`cloned_ref_to_slice_refs` in a new test).
- `just test` (schematic): **2505 passed, 15 skipped** (baseline 2482; the
  15 skips are the same environment-gated tests), fresh-output 9/9,
  compile-fail 1/1, exit 0. No test was reported slow.
- `just check-tier-coverage schematic`: 0 stranded.
- In-area dependent `just test-schema`: 55/55 passed (one reported slow
  under the generated crate's compile; it passed).
- `model-citizen` (depends on `schematic-definitions`, 153 tests, all run):
  153/153 passed.
- `darkmatter`, the tests that read shipped skills
  (`shipped_schema_corpus`, `schema_roots`, `schema_phase_validation`,
  `current_root*`): 55/55 passed.
- `test-toolkit::archive_path_guard`: `every_allowlist_entry_still_names_a_live_site`
  passed; **`no_archive_executed_target_bakes_in_a_producer_path` fails,
  pre-existing**: it reports `env!("CARGO_MANIFEST_DIR")` at
  `schematic/definitions/tests/authoring_surface.rs:5` and
  `schematic/gen/tests/l1/definition_baseline.rs:301`, two files this phase
  did not touch (`git diff HEAD` on both is empty). Left for the owner of those
  tests; not fixed here (scope).
- Dependents of `schematic-schema` (sniff, biscuit-speaks, homelab crates):
  not run. No generated artifact changed (`artifact_drift` and
  `definition_baseline` are in the green area suite), so their inputs are
  byte-identical.
- Cross-OS check: not run. The change is in-memory data modeling and
  validation with no path, process, or platform code.

The `schematic-define` skill was updated (builder surface, storage rule,
declared errors, GraphQL, `OMITTED_DEFAULTS`).

## Phase 3

Run of 2026-10-22. Tree: HEAD `6f030fd26` (Phase 2 committed). Implementer:
`claude/opus`. All work was done in the main session, sequentially: the
three codec families share `codec.rs`, `client.rs`, `lib.rs`, and the one
`l1` test binary, so a half-written file from one parallel agent would have
broken every other agent's build. No subagent ran.

### What landed (`schematic-runtime`)

- **Timing (spike outcome).** `LiveBody` now carries an **idle anchor**: set
  when the head arrives (`with_stream_bounds`) and moved by a streaming codec
  through the new public `LiveBody::mark_item()`. A streamed read waits until
  `min(anchor + per-request timeout, shared deadline)`, so bytes that drip in
  without completing an item no longer extend the idle bound. A buffered
  frame is yielded without a read. `tokio::time::timeout_at` polls the read
  before its timer, so a late poll never manufactures a timeout while data
  waits. The deadline's expiry on a streamed body is the new
  `BodyReadError::DeadlineExceeded`; the idle bound stays `TimedOut`.
- **`stream.rs` (new).** `StreamError { Malformed { raw }, EndedEarly,
  Interrupted { pending }, IdleTimeout, DeadlineExceeded,
  FrameLimitExceeded { limit, prefix } }` (frozen names), `is_terminal()`;
  `Scalar`, `EndCondition { DataLineEquals, EventNamed, FieldEquals { path,
  value }, ConnectionClose }`, the `StreamEnd` marker trait (`CONDITION`,
  `KEEP`), `UntilClose`; codecs `Sse<T, E = UntilClose>`,
  `Ndjson<T, E = UntilClose>`, and `ByteStream` (codec and output); output
  `ItemStream<T>` (fused `Stream` plus inherent `next()`), all `Send +
  'static`, none `SupportsHooks`. SSE follows the WHATWG interpretation (LF,
  CRLF, CR, comments, joined `data:`, BOM, `id`/`retry` ignored, incomplete
  event at EOF discarded). NDJSON: LF/CRLF, blank lines skipped, final line
  at EOF decoded. `FRAME_LIMIT_PREFIX_BYTES = 1024`.
- **Frame limit.** `DEFAULT_MAX_FRAME_BYTES` (16 MiB), managed builder
  `Client::max_frame_bytes(usize)` (frozen name), `LiveBody::{max_frame_bytes,
  with_max_frame_bytes}`; zero is refused at validation with the new
  `BuildError::InvalidMaxFrameBytes`.
- **`shapes.rs` (new).** `ShapeKind`, `JsonTag { Field { name, value },
  Array, Object }`, `Route` (`#[non_exhaustive]`, `const` builders `new`,
  `media`, `statuses`, `tag`, `fallback`), trait `Alternatives` (`ROUTES`,
  `decode_alternative(index, LiveBody)`), marker `JsonAlternatives`, codec
  `Shapes<A>` (`SupportsHooks` only for `A: JsonAlternatives`). Selection
  mirrors define: 204 → the empty route; candidates = routes declaring the
  status, else status-less ones; normalized `Content-Type` match; several
  matches → JSON tag read from the body; no match → fallback. Anything left
  over is the new `ErrorKind::UnexpectedContentType` (status, headers, and
  whole body kept). The global hook runs only when the chosen route is JSON.
- **Error table.** `EndpointSpec::ERRORS: &'static [ErrorRoute]` replaces
  `DECLARED_ERROR_STATUSES` (default: `400–599 → alternative 0`, the old
  behavior); `StatusMatch { Exact, Range { lo, hi } }` (`contains`, `width`,
  `Display`), `ErrorRoute { status, alternative }` with
  `ErrorRoute::lookup` (exact, then narrowest). `ErrorBody::decode_body`
  now takes `(alternative, body)`; runtime impls for `String`, `Bytes`, `()`,
  and `serde_json::Value`.
- **2xx-declared failures.** `DecodeFailure::declared(failure, retained)`
  (kind `DecodeFailureKind::Declared`, type-erased payload) and
  `ErrorBody::from_declared` (default `None`); `classify_reply` turns an
  accepted payload into `ErrorKind::Declared(E)` and a refused one into
  `MalformedSuccess`.
- **`graphql.rs` (new).** `GraphQl<D>` (`SupportsHooks`, overrides
  `decode_hooked`), `GraphQlError { message, path, locations, extensions,
  other }`, `PathSegment`, `Location` (1-based, validated),
  `GraphQlFailure<D> { errors, partial_data }`,
  `GraphQlOperationError<D, H> { GraphQl, Http }` (`ErrorBody` with
  `DECLARED = H::DECLARED` and `from_declared`), and
  `graphql::request_body(document, variables, operation_name)`.

### Adopted defaults and decisions

- **decision: the error table lives on `EndpointSpec`, not on the failure
  type** (source: contract freeze "generated `const` table of `(StatusMatch,
  alternative)`"). Two endpoints sharing one error type can have different
  tables. `DECLARED_ERROR_STATUSES` is removed rather than kept beside it
  (no generated code, pager aside, read it; the pager forwards `ERRORS`).
- **decision: a 2xx body's declared failure travels type-erased**
  (`DecodeFailure::declared` + `ErrorBody::from_declared`). Alternatives
  rejected: an associated-type default on `Decode`/`EndpointSpec` (unstable
  Rust), and a new required associated type (it would break the committed
  generated crate before Phase 4 regenerates it). The payload is private to
  `DecodeFailure`, which keeps its `Clone`/`PartialEq` derives (payload
  equality is identity).
- **decision: the codec keeps the spec's name `GraphQl<D>`** rather than
  `GraphQl<D, H>`; `GraphQlOperationError<D, H>` downcasts the payload.
- **decision: the runtime end-condition trait is named `StreamEnd`**, as is
  define's stored enum. The runtime does not depend on define, and generated
  code names the runtime one by path (`schematic_runtime::StreamEnd`).
- **adopted default: a fallback does not take a missing or invalid
  `Content-Type`** (source: R3 "Missing, invalid, conflicting, or unmatched
  response content types produce `UnexpectedContentType`"; R3c "whose media
  type no other shape matches"). A missing header is not a media type.
- **adopted default: an SSE event that the body cuts off before its blank
  line is discarded** (source: R11b names the WHATWG interpretation as the
  framing reference). A `data: [DONE]\n` without its blank line therefore
  ends early. A name-only SSE event (no `data:`) is dispatched only to match
  an `EventNamed` terminator, otherwise skipped, as WHATWG does.
- **adopted default: GraphQL member nullability.** `"errors": null` is a
  malformed wrapper (R37 "malformed wrapper fields"); an entry's `"path":
  null` or `"locations": null` reads as absent; `data` that is null or absent
  beside errors gives `partial_data: None`.
- **decision: a body read failure outranks `UnexpectedContentType`.** When a
  multi-shape reply must be read to decide (tag) or to keep as evidence and
  the read breaks, the outcome is `BodyInterrupted` with the bytes received.
- **decision: `graphql::request_body` lands now.** R35 makes "a non-object
  value fails before transmission" runtime behavior, and the runtime's
  module docs keep generated code thin; Phase 4 emits calls to it.
- **decision: `ABI` stays 5.** Contract freeze: "`ABI` 5 → 6 when the
  generated code first needs them" (Phase 4). Generated code reads none of
  the changed items (`DECLARED_ERROR_STATUSES` and `ErrorBody` are unused in
  `schema/src`), so the committed crate is unaffected.
- **behavior change:** a streamed body's shared-deadline expiry is now
  `BodyReadError::DeadlineExceeded` (was `TimedOut`), and a streaming
  codec must call `mark_item()` for the idle bound to restart. The existing
  `timing.rs` test-only `Lines` decoder now calls it, and
  `the_shared_deadline_bounds_a_stream_that_never_goes_idle` asserts the new
  variant.
- **mechanical:** `Exchange::Final` boxes its `LiveBody` (clippy
  `large_enum_variant` after `LiveBody` grew); the runtime's `serde`
  dependency enables `derive` (GraphQL error entries); the raw-socket
  `serve` helper moved from `timing.rs` to `support.rs` for reuse.

### Requirement-to-test mapping (Phase 3 scope)

All tests are L1, in the `schematic-runtime` `l1` binary (`autotests =
false`; each new file is declared in `tests/l1/main.rs`). Boundary: the
crate's public API: `Decode::decode` on a built `LiveBody` (F1) for framing
and end matrices, and the managed send's envelope over loopback sockets or
wiremock (F2) for selection, timing, ownership, errors, GraphQL, and hooks.
Compile-fail cases are `slow_` trybuild fixtures run by
`just test-compile-fail`.

| Req | Behavior | Test |
| --- | --- | --- |
| R6 | `Send`/owned outputs; usable after the client drops | `streams::{streams_are_owned_and_send, a_stream_outlives_its_client_and_dropping_it_releases_the_connection}` |
| R7, R11 | envelope before the body ends; first item before the server finishes; delayed first item | `streams::{the_envelope_and_first_item_arrive_before_the_server_finishes, a_delayed_first_item_and_a_real_connection_break_mid_frame}` |
| R8 | data line consumed (`[DONE]`), event consumed and kept, field kept (Ollama `done` with `eval_count`), nested field consumed, close | `streams::each_end_condition_finishes_cleanly_consuming_or_keeping_its_terminator` |
| R9 | ended early exactly once; close never ends early; truncated line then ended early; cut-off SSE event | `streams::a_body_that_ends_before_its_terminator_reports_ended_early_once` |
| R10 | malformed keeps raw bytes and continues; `[DONE]` not malformed; malformed kept terminator ends; non-2xx never a stream | `streams::{a_malformed_item_keeps_its_bytes_and_the_stream_continues, an_error_status_on_a_stream_endpoint_takes_the_error_path_and_skips_the_global_hook}` |
| R11a | 1 item/s outlives a 2 s timeout; silent and dripping streams time out idle; shared deadline is its own error; pre-head expiry (existing) | `streams::{a_stream_that_delivers_every_second_outlives_a_two_second_timeout, a_silent_stream_and_one_that_only_drips_partial_bytes_both_time_out_idle, the_shared_deadline_ends_a_healthy_stream_with_its_own_error}`; `timing::a_stalled_head_is_a_timeout_whose_text_redacts_the_query` |
| R11b | every split point incl. inside UTF-8, CRLF/CR, comments, joined data, BOM; NDJSON blank lines and EOF line; interrupted with pending bytes and no ended-early (injected and real socket); frame limit (LiveBody and client builder, bounded prefix, bytes unaffected, zero refused); drop releases the connection | `streams::{sse_events_decode_the_same_whatever_the_chunk_boundaries, ndjson_lines_decode_the_same_whatever_the_chunk_boundaries, a_transport_break_is_interrupted_with_the_pending_frame_and_never_ended_early, a_delayed_first_item_and_a_real_connection_break_mid_frame, a_frame_over_the_limit_ends_the_stream_with_a_bounded_prefix, a_client_frame_limit_bounds_an_unterminated_event_and_zero_is_refused, a_byte_stream_yields_chunks_in_order_and_reports_a_break}` |
| R11c | provider error event is `Ok(typed item)` | `streams::a_provider_error_event_is_a_typed_item_not_a_stream_error` |
| R3 | JSON vs SSE by media type, case and parameters ignored; 204 empty without `Content-Type`; missing, invalid, wildcard, conflicting, unmatched → `UnexpectedContentType` with status, headers, raw bytes; same type repeated agrees | `shapes::{json_and_a_stream_are_chosen_by_media_type_ignoring_case_and_parameters, missing_invalid_conflicting_or_unmatched_content_types_are_unexpected_with_evidence}` |
| R3a | 200 record vs 202 job; other 2xx to status-less shapes; unmatched status → unexpected; chosen-but-undecodable → `MalformedSuccess` | `shapes::declared_statuses_choose_between_json_shapes_and_others_fall_to_status_less_shapes` |
| R3b | discriminator values, unknown value, missing field, non-JSON, matched-but-malformed; array vs object with leading whitespace; scalar → unexpected | `shapes::json_tags_choose_by_discriminator_or_by_array_versus_object` |
| R3c | `image/png` → fallback, `application/json` → JSON; missing header not taken | `shapes::a_fallback_takes_a_media_type_no_other_shape_matches` |
| R3 single, R4 | single shape never checks `Content-Type`; binary with a mismatched type succeeds | `shapes::a_single_shape_endpoint_never_checks_the_content_type` |
| R12 | stream codecs and mixed alternatives refuse a mutator (compile-fail); all-JSON alternatives run global hook then mutator, refusal keeps original bytes; global hook only on a JSON choice; never on a stream or non-2xx | `tests/fixtures/compile_fail/{mutator_on_a_stream_endpoint, mutator_on_mixed_alternatives}.rs`; `shapes::{an_all_json_enum_runs_a_typed_mutator_after_the_global_hook_and_keeps_original_bytes_on_refusal, the_global_hook_runs_only_when_the_chosen_shape_is_json}`; `streams::an_error_status_on_a_stream_endpoint_takes_the_error_path_and_skips_the_global_hook` |
| R19 | exact beats ranges, narrower beats wider, order-independent; declared per alternative; undecodable → `MalformedError`; unmatched and opted-out → `Undeclared`; raw body kept; single-shape runtime failure types | `error_routes::{lookup_prefers_an_exact_status_then_the_narrowest_range, the_matched_entry_decodes_its_own_shape_and_everything_else_keeps_its_body, the_runtime_failure_types_decode_text_bytes_empty_and_any_json_under_the_default_table}` |
| R35, R36 (runtime part) | body members, unit omits `variables`, `operationName` only when set (empty kept), invalid document unchanged, non-object variables refused | `graphql::the_request_body_carries_the_document_unchanged_and_variables_only_as_an_object` |
| R37 | success; empty errors; 201/206 keep status; errors with undecodable/null/absent data; partial data; 13 malformed wrapper/entry/data cases; 403 → HTTP path (declared and opted out); GraphQL errors declared after opt-out; `into_result` is `Err` | `graphql::{the_wrapper_is_classified_before_typed_data, a_permission_failure_takes_the_http_error_path_not_the_wrapper, graphql_errors_stay_declared_after_an_http_opt_out, sniff_error_fixtures_are_declared_graphql_failures_keeping_github_type}` |
| R38 | sniff fixtures `success`, `passed`, `changed_head`, `null_rollup`, `partial_errors`, `request_wide_errors`, `rate_limit`, `permission_failure` (each `include_str!` inside its `#[test]`); path segments, locations, extensions, GitHub `type` in `other` | `graphql::{sniff_success_fixtures_decode_as_typed_data, sniff_error_fixtures_are_declared_graphql_failures_keeping_github_type, a_permission_failure_takes_the_http_error_path_not_the_wrapper, the_wrapper_is_classified_before_typed_data}` |
| R41 | global hook sees the wrapper once per 2xx, never non-2xx; mutator typed `&mut D`, runs only failure-free, never on partial data; refusal keeps original bytes | `graphql::the_global_hook_sees_the_wrapper_once_and_the_mutator_only_failure_free_data` |

**Deferred to the phase that owns the boundary:** R7's "one through a
generated fixture client", R2/R20 generated types, the wrong-variables
compile error (needs generated requests), and R12's "no definition carries
such a hook" scan are Phase 4/5 items (plan Waves 7, 8, 10).

### Input robustness (wire readers)

The new readers are HTTP wire protocols, not configuration, but the GraphQL
wrapper has load-bearing fields, so its shapes were walked through the
public envelope (`graphql::the_wrapper_is_classified_before_typed_data`).
Default outcome: `MalformedSuccess` with the raw body. Exceptions: `errors`
absent or `[]` means no errors; `data` absent or null beside errors gives no
partial data; entry `path`/`locations`/`extensions` absent or null mean
absent. Covered cells: `errors` absent, null, wrong type (object), one
invalid entry among valid, the only entry invalid, empty; `message` absent
and wrong type; `path` element negative and fractional; `locations` line 0;
`data` absent, null, wrong type; wrapper not an object; body not JSON.
Duplicate wrapper keys are last-wins (`serde_json` `Value`), accepted for a
wire protocol whose JSON grammar allows them. **Smell grep** over
`stream.rs`, `shapes.rs`, `graphql.rs`, `endpoint.rs`: the `.ok()` uses are
`ErrorBody for Value` (None → `MalformedError`, the documented result),
`partial_data` (decode failure → `None`, R37's rule), and the tag reader
(not JSON → no tag match → `UnexpectedContentType`, R3b's rule); no
`#[serde(default)]`, `unwrap_or_default()`, or `filter_map(.. as_str())`.

### Phase 3 gates

The scope calculator, run from the repository root after all edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `packages: [schematic-runtime, test-toolkit, darkmatter,
schematic-define, schematic-gen]`, `change_class: package`. So both gates ran.

- `just lint` (schematic): clean, after one fix: clippy
  `large_enum_variant` on `Exchange` (boxed `LiveBody`).
- `just test` (schematic): **2541 passed, 15 skipped** (Phase 2: 2505; the
  15 skips are the same environment-gated tests), fresh-output 9/9,
  compile-fail 1/1 (25 trybuild cases, three snapshots new or changed), exit
  0. No test was reported slow. The slowest new test,
  `streams::a_stream_that_delivers_every_second_outlives_a_two_second_timeout`,
  takes about 3 s by contract (R11a's one item per second past a 2 s
  timeout), inside the 5 s budget.
- During development, two real defects surfaced and were fixed before the
  gate: the `unfold`-backed streams panicked when polled after their end (now
  fused), and the NDJSON framer kept a stale scan offset after its EOF flush.
- `just check-tier-coverage schematic`: 0 stranded.
- In-area dependent `just test-schema`: 55/55 passed (one slow under the
  generated crate's compile; it passed).
- Dependents of `schematic-schema` (which re-exports the runtime):
  `cargo check --tests` for `homelab`, `sniff`, `biscuit-speaks`,
  `eversolo-integration`, `arcam-amp-integration`,
  `unfolded-integration-helper`, `sony-receiver-integration`, and
  `model-citizen`: clean. Behavior tests that send through a generated client
  (the default error table and buffered codecs changed underneath them):
  `sniff` remote tests 75/75, `homelab` 49/49, `biscuit-speaks` ElevenLabs
  48/48. None of them names a changed item (`DECLARED_ERROR_STATUSES`,
  `ErrorBody::decode_body`, `BodyReadError`).
- `darkmatter` tests that read shipped skills (`shipped_schema_corpus`,
  `schema_roots`, `schema_phase_validation`, `current_root*`): 55/55.
- `test-toolkit::archive_path_guard`: `every_allowlist_entry_still_names_a_live_site`
  passed; **`no_archive_executed_target_bakes_in_a_producer_path` fails,
  pre-existing and unchanged from Phase 2**: it reports
  `schematic/definitions/tests/authoring_surface.rs:5` and
  `schematic/gen/tests/l1/definition_baseline.rs:301`, which this phase did
  not touch. No new test file is reported (the new repository reads are
  `include_str!` inside each `#[test]`).
- Cross-OS check: not run. The change has no path, process, console, or
  `#[cfg]` code; the new socket tests use dynamic loopback ports and the same
  patterns as the existing `timing.rs` suite, which CI runs on every OS.

Docs updated: `runtime/codecs.md` (streams, shapes, error table, GraphQL,
hook implementers), `runtime/deadlines-and-timeouts.md` (idle bound per
item, distinct expiry errors, sequence diagram), `runtime/sending-and-responses.md`,
`runtime/hooks.md`, `runtime/README.md` (planned list now covers only
generation), `responses-and-errors.md` and `graphql.md` (new "Decoding a
reply" section), and `dependencies.md` (serde `derive`, futures-util use).
The `schematic-define` skill's streaming note now names the runtime codecs
the generator will target.

## Phase 4

Run of 2026-10-22. Tree: HEAD `6f030fd26` plus the uncommitted Phase 3
runtime work (still in the working tree, so the scope calculator below also
sees Phase 3's files). Implementer: `claude/opus` as coordinator, with three
foreground subagents on disjoint files (REST response codegen; consumer
adapters; runtime GraphQL address and `ErrorBody<A>`), then GraphQL codegen,
the ABI bump, and fixture integration in the main session.

### Decisions taken before the waves

- **decision: a hidden test seam `Endpoint::shapes_preview`** (define
  `types.rs`). `Endpoint::response` still holds one `ApiResponse`, so no
  definition can carry several shapes or a stream until Phase 5. The field is
  `#[doc(hidden)]`, `#[serde(skip)]`, set by no builder, and read only by
  `Endpoint::success_shapes()` when nonempty. It lets generator and exporter
  tests drive multi-shape and stream endpoints through the real pipeline
  (validation, request structs, module assembly, fixture crates) now, and
  Phase 5 deletes it when `response` becomes the shape list (the test
  fixtures then assign `response` instead). Alternative rejected: a
  `generate_*_with_shapes` entry point in the generator, which would bypass
  validation and module assembly and leave a test-only public function in
  production code. It is not a second response system: nothing serializes,
  builds, or exports it outside tests.
- **deviation (frozen form, additive): `SuccessShape::variant`** (string,
  absent means none, `non_null`) and the builder `Success::variant(name)`.
  D-15 says "the author may name variants in the definition, and does so for
  R3b groups and R32a (`Directory`, `File`)", but the Phase 1 freeze gave the
  stored shape no member to hold that name. Validation: an UpperCamelCase
  ASCII identifier other than `Self`, unique within the endpoint, and refused
  on a single shape (which generates no enum), mirroring the R3c fallback
  rule. `legacy_response()` refuses a named shape. Test:
  `define/tests/response_vocabulary.rs::an_author_named_variant_is_an_identifier_unique_to_a_multi_shape_endpoint`.
- **removed dead generator code** (Phase 1 review agenda item 10):
  `gen/src/codegen/client/` (5 files), `codegen/request_enum.rs`,
  `codegen/error.rs`, `codegen/variant.rs`. None was declared in
  `codegen/mod.rs`, so none compiled; they were the last readers of the
  removed `ResponseKind` and would have needed v2 edits for nothing.
- **finding and decision: `ErrorBody` becomes `ErrorBody<A>` (generic over the
  API marker).** Registered JSON types live in `schematic-definitions`
  (`schema/src/anthropic.rs` re-exports `schematic_definitions::anthropic::*`),
  so the generated crate cannot write `impl ErrorBody for
  ::schematic_definitions::…::Problem`: Rust's orphan rule forbids it, although
  the runtime docs promised that "generated code implements it for registered
  JSON types". With the API marker (a generated, local type) as a trait
  parameter, `impl ErrorBody<Anthropic> for ::schematic_definitions::…::Problem`
  is legal, so R20's "the failure type is the matched shape's type" holds
  without a wrapper. Alternatives rejected: a local newtype or one-variant enum
  per endpoint (contradicts R20), and a second associated failure-decoder
  type on `EndpointSpec` (larger change to every generated impl and to the
  GraphQL composite).

**Tree note.** The pipeline committed Phase 3 while this run was underway
(`d4037bbe6`, `8a10f5298`, `44b6878b3`). Those commits hold no Phase 4 edit
(checked: HEAD has no `ErrorBody<A>`, `AddressRule` in the runtime, or
`## Phase 4` here), so the working tree is exactly Phase 4's change.

### What landed

**Wave 7: REST generation** (`gen/src/codegen/responses.rs`, new; wired in
`request_structs/endpoint_spec.rs` and `output/assemble/api.rs`). The
generator reads only `Endpoint::success_shapes()` and
`RestApi::effective_errors`:

- one shape keeps today's `Json<T>`/`Text`/`Binary`/`Empty`; an SSE/NDJSON
  stream is `ItemStream<T>` with `Sse<T, End>`/`Ndjson<T, End>`; a byte stream
  is `ByteStream`. `End` is `UntilClose` for a consumed connection close, else
  a generated `{Id}End` (`{Id}{Variant}End` inside an enum) implementing
  `StreamEnd` (`FieldEquals` paths split on `.`, values mapped to `Scalar`).
- several shapes generate `{Id}Response` (D-15) deriving `Debug`, variants in
  declaration order with the note as rustdoc, named by `SuccessShape::variant`,
  else the kind, else kind plus media subtype (`JsonProblem`); still-equal
  names are a `generated-name-collision` diagnostic asking for
  `Success::variant`. `impl Alternatives` (routes: normalized media, statuses,
  tags, fallback) and `impl JsonAlternatives` only when every shape is JSON;
  `Decoder = Shapes<{Id}Response>`.
- failures: an empty or opted-out table keeps `NoDeclaredError` and emits no
  `ERRORS` (so every shipped endpoint is byte-identical); one distinct shape is
  its own type (R20); several give `{Id}Error` (D-23) with an
  `ErrorBody<{Api}Api>` impl dispatching on the route's alternative.
  `api_error_body_impls` emits one `ErrorBody<{Api}Api>` per registered JSON
  error type per API module (`ErrorBody<A>` decision above).
- validation: end-marker and operation failure-enum names join the module's
  naming space; stream item types are "referenced types" (role "stream
  item"); `failure_enum_needed` decides `{Id}Error` for endpoints and
  operations alike.

**Wave 7: GraphQL generation** (`gen/src/codegen/graphql.rs`, new). One
request struct per operation, `{id}{suffix}`: typed variables as its one field
taken by `new`, or a unit struct with `new()`; `DOCUMENT` and `OPERATION_NAME`
constants; `EndpointSpec` with `METHOD = POST`, `ADDRESS = Address::GraphQl`,
`Accept: application/json`, `Decoder = GraphQl<D>`, `Failure =
GraphQlOperationError<D, H>` where `H` comes from the same failure codegen
over `effective_operation_errors`. `definition()` gains `.graphql(&[..])` for
an API with a surface. No endpoint method or convenience wrapper.

**Runtime support (subagent)**, needed by the generated GraphQL and error code:

- `ErrorBody<A>` (generic over the API marker); runtime impls are blanket over
  `A`; `EndpointSpec::Failure: ErrorBody<Self::Api>`.
- `Address { Base, GraphQl }` and `EndpointSpec::ADDRESS` (default `Base`).
  A GraphQL-address request goes to the resolved GraphQL URL through the one
  shared assembly step (typed send, `send_raw`, `check_arguments`, the pager),
  with the endpoint's cross-origin check as for an absolute URL.
- `graphql::{AddressRule, AddressError, derive_address}` (R39: segment-wise,
  trailing slash ignored, proxy prefix kept, query/fragment base refused,
  first match wins); `ApiDefinition::graphql(rules)`;
  `Client::graphql_url(url)` with `BuildError::InvalidGraphQlUrl { reason }`
  (reason only, never the URL). The address is derived at build from the
  effective base URL.
- The generated API-struct macro (`api_client!`) now forwards `graphql_url`
  and `max_frame_bytes`; both joined `API_CLIENT_METHODS`. **Gap closed:**
  Phase 3 added `Client::max_frame_bytes` but a generated API struct could not
  reach it.
- `ABI` 5 → 6 (runtime `ABI`, gen `RUNTIME_ABI`): generated code now names
  `ErrorBody<A>`, `Address`, `AddressRule`, the stream codecs, and
  `Alternatives`. `schematic-schema` regenerated with the generator
  (`cargo run -p schematic-gen -- generate --api all …`, the `just generate`
  command without its desktop notification); the only diff is the two ABI
  lines of `schema/src/lib.rs`. OpenAPI and Postman artifacts unchanged.
  `gen/tests/l1/e2e_generation.rs` now reads the ABI from `RUNTIME_ABI`
  instead of the literal `5`.

**Wave 7: consumer adapters (subagent)**:

- OpenAPI export: production now goes through the v2 mapping
  (`openapi/export/responses.rs::map_responses(shapes, &ErrorTable)` from
  `paths.rs::map_operation`), byte-identical for every shipped definition
  (`artifact_drift` green). Per-status merging, `oneOf` + `discriminator`,
  array/object `oneOf`, `*/*` fallback, notes in descriptions, byte streams as
  binary strings, SSE/NDJSON item schemas, resolved-table error keys with
  class expansion (D-25), success and error keys disjoint. The binary schema
  is now `StringFormat::Binary` (serializes identically, and survives an
  `openapiv3` parse).
- `x-schematic`: an operation-level `success` member when shapes carry facts
  the standard fields lose (omitted for a lone legacy-representable shape),
  and the declared error levels (`errors` on the document and operations,
  `no_declared_errors`).
- OpenAPI import: `openapi::classify_responses(api, endpoint, &Responses,
  &RefResolver) -> ResponseClassification { successes, errors, diagnostics }`
  implements R28 and is tested directly; **production import stays legacy**
  (an endpoint cannot store several shapes yet). Differences Phase 5 will see
  on switching: SSE becomes a stream (today Binary plus an info diagnostic); no
  representable success becomes an import error (today Empty plus a warning);
  error statuses become endpoint entries (today ignored); several media types
  keep one shape each (today one by preference). Phase 5 must also extend
  `OpenApiImport::remap_reference_names` to shapes and error entries.
- Parser-boundary checks for the response fields `classify_responses` reads
  were added to the existing `check_text` loader; they are live (existing
  import tests and shipped corpora pass).
- Postman: the first success shape's content type (`first_success_content_type`),
  a description line only when that shape declares a content type or streams.
- R42/D-29: OpenAPI and Postman skip GraphQL operations with a nonfatal
  `graphql-operation` warning, `<API> operation <id>: GraphQL operation skipped
  in <OpenAPI|Postman> output` (new `Feature::GraphQlOperation`).
- `definition_gen.rs` emits v2 builder calls (`Success::sse(..)`, modifiers,
  `ErrorSet::new().on(..)`, `ErrorSet::none()`) when an endpoint carries v2
  facts; the legacy `returns_*` path otherwise (committed outputs unchanged).

**Wave 8: fixture integration.** `gen/tests/l1/fresh_output_compile.rs` gains
two fixture APIs generated, compiled offline, and run against wiremock in the
fixture crate:

- `FixtureShapes` (via `shapes_preview`): `CreateMessage` JSON + SSE `[DONE]`
  with notes and a 404 text overlay; `StreamChunks` NDJSON `done = true` kept;
  `Download` byte stream `audio/mpeg`, opted out; `GetSource` `Directory` JSON +
  `File` binary fallback; `Submit` `Record` 200 / `Job` 202 with a 409
  `Conflict` overlay; API default `5XX` → `Problem`.
- `FixtureGraphQl`: GitHub address rules on a `/api/v3` base, `CheckStatus`
  (typed variables, operation name, 403 → `Problem`), `Viewer` (no
  variables), `Rename` mutation (HTTP opt-out), and a REST `GetUser`.
- `tests/common/components.rs` now also registers components for preview
  shapes, API error entries, and GraphQL operation references.

### Adopted defaults and decisions (Phase 4)

- **adopted default: failure variant collisions get a position suffix**
  (`Problem`, `Problem2`) after the media-subtype suffix, rather than a
  diagnostic (source: D-23 "named after the shape's type"; error shapes have no
  author-naming facility, so a diagnostic would leave the author with no fix).
  Success-variant collisions are a diagnostic because `Success::variant` exists.
- **adopted default: a lone JSON error shape whose Rust type the runtime
  already decodes differently** (`String`, `bytes::Bytes`, `()`,
  `NoDeclaredError`) generates the `{Id}Error` enum so the body is read as JSON;
  `serde_json::Value` is used directly (the runtime's impl decodes JSON).
  Source: R20, constrained by the runtime impls of Phase 3.
- **adopted default: D-25 expansion.** A class range that an exact or narrower
  entry supersedes in part is expanded into its exact statuses (a `4XX` default
  plus an exact `404` exports 100 keys). OpenAPI already lets an exact key take
  precedence over `4XX`, so keeping `4XX` + `404` would also preserve lookup.
  **Review agenda:** whether to keep the class key in that case.
- **decision: `x-schematic` carries the declared error levels**, not the
  resolved table (the `responses` map already shows the resolved table); stream
  framing/end live only in the operation-level `x-schematic.success`.
- **decision: import declares success statuses only when the statuses carry
  different shape sets**, ignoring a body-less 204; inline `oneOf` members and
  array/object split members import as `serde_json::Value` (as
  `reference_from_schema` already does).
- **decision: GraphQL operations get no endpoint method** on the API struct;
  the request value goes through `send` (R36 "the existing generic send").
- **decision: an invalid author variant name falls back to `Shape{n}` in
  codegen** instead of panicking; define validation rejects such names first.
- **deferred (Phase 7, not a Phase 4 gap):** the generated pager for a
  paginated operation. Phase 4 prepared it: the runtime pager forwards
  `R::ADDRESS` (`graphql_address::every_page_of_a_graphql_address_endpoint_reaches_that_address`).
- **known limitations recorded for Phase 5:** `check_pagination` and the
  Postman composition warning still read `endpoint.response`; a multi-shape
  paginated endpoint is not handled until storage switches.
- **mechanical:** the misuse label "ErrorSet only supports none" is now "ErrorSet
  has no status shorthand" (it has `new`/`on` since Phase 2; the case still
  fails as intended).

### Requirement-to-test mapping (Phase 4 scope)

Tiers: every test below is L1 except the `slow_` fresh-output guards, which
`just test` runs through `just test-fresh-output` (they invoke Cargo, so they
stay off archive consumers, as before). New files are declared:
`gen/tests/l1/main.rs` (`mod response_adapters;`),
`runtime/tests/l1/main.rs` (`mod graphql_address;`); the two new
`define/tests/*.rs` files compile under define's autotests (`all-features` in
its CI metadata). No name carries a tier marker by accident.

Boundaries and fidelity: generator output (F1, token assertions parsed by
`syn`) for naming and shape; **the real generated crate compiled offline and
run against wiremock (F2)** for R2/R7/R12/R19/R20/R36–R39 behavior; the
public define importer/exporter (F1, in-process, including `openapiv3`
re-parse); the runtime client against loopback mocks (F2).

| Req | Behavior | Test |
| --- | --- | --- |
| R2 | one shape keeps its type/codec; streams are `ItemStream`/`ByteStream`; several shapes are `{Id}Response` | `gen codegen::responses::tests::{a_single_plain_shape_keeps_its_type_and_codec_and_declares_no_error, a_single_sse_stream_consuming_its_data_line_terminator_names_a_generated_end_marker, a_kept_ndjson_field_terminator_splits_its_dotted_path, a_byte_stream_and_a_connection_close_stream_need_no_end_marker, scalars_keep_their_json_kind, json_plus_sse_is_an_enum_whose_variant_docs_carry_the_notes}`; compiled and run: `fresh_output_compile::slow_fixture_apis_compile_and_send_offline` (child tests `a_multi_shape_endpoint_returns_the_variant_its_content_type_selects`, `single_stream_endpoints_deliver_items_and_bytes_as_they_are_framed`) |
| R3, R3a–R3c (generated) | routes carry media, statuses, tags, fallback; JSON + empty; unexpected content type through a generated client | `responses::tests::{json_shapes_told_apart_by_status_are_all_json_alternatives, tagged_json_groups_route_by_discriminator_or_json_kind, a_binary_fallback_has_no_media_type_and_disables_json_alternatives, json_plus_empty_has_a_unit_variant_chosen_by_204}`; child tests `a_multi_shape_endpoint_returns_the_variant_its_content_type_selects`, `named_fallback_and_status_shapes_select_their_variants` |
| D-15 naming | author variant names; kind, then media subtype; collision diagnostic; `variant` validation and serialized form | `responses::tests::derived_variant_names_take_the_media_subtype_only_when_kinds_repeat`; `gen validation::tests::success_variants_left_colliding_by_derived_names_ask_for_author_names`; `define/tests/response_vocabulary.rs::an_author_named_variant_is_an_identifier_unique_to_a_multi_shape_endpoint` |
| R8/R11 (generated) | `[DONE]` consumed; NDJSON `done` kept as last item; ended early through the generated client; byte chunks in order | child test `single_stream_endpoints_deliver_items_and_bytes_as_they_are_framed` |
| R12 | `JsonAlternatives` only for all-JSON; mutator refused on NDJSON, byte stream, text, binary, JSON+stream; all-JSON enum and GraphQL data accept one; mutator runs on a generated all-JSON enum | `slow_generated_types_reject_misuse_at_compile_time` (five new misuses plus the control); child test `named_fallback_and_status_shapes_select_their_variants` |
| R13a | notes reach variant rustdoc and the OpenAPI response description | `responses::tests::json_plus_sse_is_an_enum_whose_variant_docs_carry_the_notes`; `define/tests/response_openapi.rs::every_shape_of_a_status_merges_into_one_response_object_with_its_notes` |
| R19, R20 | inherited/overlaid/opted-out tables; one shape is its type with one deduplicated `ErrorBody<Api>` impl; several give `{Id}Error` with correct alternatives; exact beats range through a generated client; malformed, undeclared, opt-out | `responses::tests::{inherited_api_errors_with_one_json_shape_fail_as_that_type, an_endpoint_overlay_with_a_second_shape_generates_the_failure_enum, an_opted_out_endpoint_declares_no_error_and_no_table, a_lone_error_shape_uses_the_runtime_body_unless_the_runtime_decodes_its_type_otherwise, repeated_error_variant_names_take_the_media_subtype_then_a_position, error_body_impls_are_one_per_distinct_type}`; `output::assemble::api::tests::a_json_failure_type_shared_by_endpoints_gets_one_error_body_impl_per_api`; `validation::tests::{a_stream_end_marker_joins_the_generated_names, a_lone_json_string_error_needs_the_failure_enum}`; child test `declared_errors_decode_by_the_most_specific_status_and_opt_outs_stay_undeclared`; runtime `graphql_address::a_registered_type_of_another_module_decodes_a_declared_http_error` |
| R21, R27, D-25, D-31 | per-status merge, `oneOf`/discriminator, array/object, `*/*`, byte stream binary, class kept vs expanded, keys disjoint, `openapiv3` re-parse (JSON and YAML) | `define/tests/response_openapi.rs::{json_shapes_sharing_status_and_media_become_a_one_of_naming_the_discriminator, array_and_object_json_shapes_become_a_one_of_without_a_discriminator, byte_streams_export_as_binary_strings_under_each_declared_status, a_class_range_stays_4xx_unless_an_exact_entry_supersedes_part_of_it, the_exported_document_parses_back_through_openapiv3_as_json_and_yaml}`; define unit `export::responses::tests::{a_class_range_superseded_in_part_expands_to_its_exact_statuses, a_range_that_is_not_a_class_exports_its_exact_statuses}`; `gen/tests/l1/response_adapters.rs::the_grouped_openapi_document_merges_shapes_and_parses_back` |
| R26 | `x-schematic` carries tags, fallback, notes, variant, stream end, declared errors | define unit `extensions::tests::response_vocabulary_round_trips_through_both_extensions`; `response_openapi.rs::x_schematic_carries_declared_error_levels_and_no_success_for_a_plain_shape` |
| R28 | media families, stream without schema, `oneOf` separable or not, endpoint-level errors and diagnostics, status declaration, no-success error, export→import round trip | `response_openapi.rs::{import_classifies_each_normalized_media_family, a_stream_without_a_named_item_schema_decodes_values_until_close_with_a_diagnostic, a_one_of_separates_by_discriminator_or_array_versus_object_and_otherwise_imports_as_one_schema, errors_stay_endpoint_level_and_unrepresentable_responses_are_reported, success_statuses_are_declared_only_when_statuses_carry_different_shapes, an_operation_without_a_representable_success_is_an_import_error, export_then_import_of_json_sse_fallback_and_errors_yields_the_same_shapes_and_entries}` |
| R29 | Postman records the first shape's content type | `response_adapters.rs::postman_records_the_first_success_shapes_telling_content_type` |
| R35 (call site), R36 | wrong variables type fails to compile; POST, `Accept`/`Content-Type` JSON, body members, unit omits `variables`, `operationName` only when set | `slow_generated_types_reject_misuse_at_compile_time` ("GraphQL variables of another type"); `gen codegen::graphql::tests::{typed_variables_are_the_request_field_and_reach_the_body_with_the_operation_name, an_operation_without_variables_omits_them_and_its_name_by_default}`; child test `graphql_operations_post_their_document_to_the_address_the_rules_derive` |
| R35 naming | operation failure enum joins the naming space | `validation::tests::an_operation_shares_the_endpoint_naming_space` |
| R37 (generated) | GraphQL errors with GitHub `type`, 403 typed HTTP failure, opt-out keeps GraphQL errors declared | `codegen::graphql::tests::the_failure_composes_graphql_errors_with_the_effective_http_table`; child test `graphql_errors_and_http_errors_reach_the_composite_failure` |
| R39, D-11 | derivation table; `/api/v3` and root bases for REST and GraphQL; caller base override moves it; override beats rules; invalid override fails at build; foreign override refused under same-origin, sent under `allow_origins`; no address fails before send; pager pages reach it; generated rules and `graphql_url` | runtime `graphql_address::{derivation_takes_the_first_matching_rule_over_path_segments, rest_and_graphql_endpoints_reach_their_addresses_under_both_bases, a_graphql_url_override_beats_the_rules, an_invalid_override_or_an_underivable_base_fails_at_build, a_foreign_override_is_refused_under_same_origin_and_sent_when_its_endpoint_allows_it, a_graphql_send_with_neither_rules_nor_override_fails_before_sending, every_page_of_a_graphql_address_endpoint_reaches_that_address}`; `codegen::graphql::tests::address_rules_join_the_api_definition_in_order_only_for_a_graphql_surface`; child test `graphql_operations_post_their_document_to_the_address_the_rules_derive` |
| R41 (generated) | a mutator for an operation is typed `&mut D` | misuse control in `slow_generated_types_reject_misuse_at_compile_time` |
| R42 | both exporters skip operations with a nonfatal warning | `response_adapters.rs::both_exporters_skip_graphql_operations_with_a_warning` |
| definition source | importer emits v2 builder calls | gen unit `definition_gen::tests::emits_builder_calls_reproducing_response_model_v2_facts` |
| no regression | every shipped definition's Rust, OpenAPI, and Postman output unchanged except the ABI lines | `artifact_drift`, `definition_baseline`, `fresh_output_compile::slow_real_apis_compile_offline`, `import_overlay` (all in `just test`) |

**Deferred to the phase that owns the boundary:** R43's sniff fixtures through
a generated operation and R40's generated pager are Phase 7 (Wave 14);
switching production import onto `classify_responses` and storing shapes are
Phase 5.

### Input robustness matrix (OpenAPI response fields, JSON and YAML)

Two loaders (`serde_json`, `serde_yaml_ng`), both through `check_text` and
then `openapiv3`. Default outcome for every cell: an import error naming the
operation and member path; nothing imported. Columns: `responses`,
`content`, `schema`, response `$ref`, `oneOf`,
`discriminator.propertyName` (and `mapping`). Rejected rows (32 cells, each
format): null, wrong type, one invalid member, all members invalid, empty
where empty is not allowed, duplicates by spelling (`4XX`/`4xx`, media-type
case), exact textual duplicate keys (JSON; YAML's parser already rejects them),
and trailing content. Exceptions (accepted, checked through
`classify_responses`): a response with no `content` or empty `content` is
Empty; absent or empty `schema` is `serde_json::Value` plus a diagnostic; an
inline response is used directly; an absent discriminator is a single schema
plus a diagnostic; empty `responses` is "no success", an import error; a
media range gets a diagnostic. A control row proves the unedited fixture
imports. Tests: `define/tests/response_import_matrix.rs::{json_response_matrix,
yaml_response_matrix, json_exact_duplicate_keys_are_rejected_for_every_column}`.
Production import keeps today's behavior on these cells until Phase 5.

**Smell grep** (subagent, re-checked for the new codegen): the `#[serde(default)]`
members added are the write-only `x-schematic` members, each paired with its
skip-when-default serialization; `unwrap_or` in `check_text` and `classify.rs`
only labels messages; `split(';').next().unwrap_or_default()` cannot reach its
default; no `.ok()` or `filter_map(.. as_str())` on a load-bearing parse. The
generated `ErrorBody` impl's `serde_json::from_slice(body).ok()` is the
runtime's documented "`None` is `MalformedError`" contract.

### Review agenda entries raised in Phase 4

1. **`SuccessShape::variant`** is a new stored member beyond the Phase 1
   freeze (additive, omitted when unset). D-15 needs somewhere to keep author
   variant names; confirm the member name and the "only on several shapes"
   rule.
2. **`ErrorBody<A>`** replaces `ErrorBody` (orphan rule; see the decision
   above). Confirm the trait shape before Phase 5 adds the first shipped error
   tables.
3. **D-25 expansion** of a partly superseded class (100 exact keys for `4XX` +
   `404`) versus keeping `4XX` beside the exact key.
4. **Failure variant collisions** take a position suffix (`Problem2`); error
   shapes have no author-naming facility.
5. **The `shapes_preview` test seam** lives in `schematic-define` until Phase
   5; confirm it is removed there.

### Phase 4 gates

The scope calculator, run from the repository root after all edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `packages: [schematic-define, schematic-gen, schematic-runtime,
test-toolkit, darkmatter]`, `change_class: package`. So both gates ran, once,
after every subagent had finished.

- `just lint` (schematic): clean.
- `just test` (schematic): **2599 passed, 15 skipped** (Phase 3: 2541; the 15
  skips are the same environment-gated tests), fresh-output 9/9 (the fixture
  test now also builds and runs `FixtureShapes` and `FixtureGraphQl`; the
  misuse test has the five new misuses), compile-fail 1/1, exit 0. No test
  was reported slow, timed out, or leaky. Before the gate the two slow guards
  were run alone: `slow_fixture_apis_compile_and_send_offline` (92 s, all
  child tests passed first time) and
  `slow_generated_types_reject_misuse_at_compile_time` (40 s).
- `just check-tier-coverage schematic`: 0 stranded.
- In-area generated crate: `just test-schema` 55/55 (one slow under the
  generated crate's compile, as before); `just lint-schema` clean.
- Subagent-scoped runs (before the gate): runtime `l1` 400 passed and
  `just test-compile-fail` 1/1; define full suite 796/796; gen lib 483 then
  488 with the GraphQL tests; the l1 `artifact_drift`, `definition_baseline`,
  `output_validation`, `e2e_generation`, `postman_*`, `openapi_*`,
  `import_overlay`, `definition_bundles`, and `response_adapters` groups
  green; clippy `-D warnings` clean on all three crates.
- Dependents of `schematic-schema` / `schematic-definitions`, serially:
  `cargo check --tests` clean for `sniff`, `biscuit-speaks`, `homelab`,
  `eversolo-integration`, `arcam-amp-integration`,
  `unfolded-integration-helper`, `sony-receiver-integration`, and
  `model-citizen`. Behavior tests that send through a generated client (the
  `ErrorBody<A>` trait and ABI changed underneath them): `sniff` remote tests
  75/75, `homelab` 49/49, `biscuit-speaks` ElevenLabs 48/48, `model-citizen`
  153/153. None of them names `ErrorBody`, `EndpointSpec`, or the ABI.
- `darkmatter` tests that read shipped skills (`shipped_schema_corpus`,
  `schema_roots`, `schema_phase_validation`, `current_root*`): 55/55.
- `test-toolkit::archive_path_guard`: `every_allowlist_entry_still_names_a_live_site`
  passed; **`no_archive_executed_target_bakes_in_a_producer_path` fails,
  pre-existing and unchanged since Phase 2**:
  `schematic/definitions/tests/authoring_surface.rs:5` and
  `schematic/gen/tests/l1/definition_baseline.rs:301`, neither touched by this
  phase. No new file is reported.
- Cross-OS check: not run. No path, process, console, or `#[cfg]` code
  changed; GraphQL address derivation works on parsed `url::Url` path
  segments; the new fixture-crate tests use the existing fresh-output harness
  (which CI already runs on every OS) and wiremock on dynamic ports.
- Nothing was staged or committed (the three `git rm` deletions were unstaged
  immediately; they remain plain working-tree deletions). `cargo fmt` was not
  run.

Docs updated: `responses-and-errors.md` (new "Generated success and failure
types"; `.variant`; stored form), `graphql.md` (new "Generated operation
requests"; generated rules and `graphql_url`; planned markers removed),
`generation/generated-clients.md` (`Success`/`Failure` from shapes and tables;
operations), `runtime/codecs.md` ("What generated code emits" replaces
"Planned"), `runtime/README.md`, `runtime/sending-and-responses.md`,
`definition-builders.md`; by subagents `runtime/codecs.md` (`ErrorBody<A>`),
`runtime/building-clients.md` (`graphql_url`), `io/export-openapi.md`,
`io/openapi-extensions.md`, `io/export-postman.md`, `io/import-openapi.md`,
and `generation/validation.md`. The `schematic-define` skill (`SKILL.md`,
`schematic/gen.md`) now describes the generated v2 codecs, the test seam,
`variant`, and the removed dead files.

## Phase 5

Run of 2026-10-22. Tree: HEAD `4aba5e94c` (Phase 4 committed), clean.
Implementer: `claude/opus` as coordinator. Wave 9 (the storage switch) ran in
the main session because every consumer shares `schematic-define`'s core
files; Wave 10's four provider migrations ran as parallel foreground
subagents on disjoint provider directories; Wave 11 ran in the main session.

### Wave 9 — Storage switch (main session)

- `Endpoint::response` is now `Vec<SuccessShape>`, serialized as an array under
  the historical member name (frozen form). `ApiResponse` is **removed**
  (contract freeze: "survives only until Phase 5 … then is removed"; the
  spec's "Proposed: yes, as builder sugar only" is answered by `Success`,
  which already is the sugar). Thin constructors `SuccessShape::{json,
  json_type, json_vec_type, text, binary, empty}` replace `ApiResponse`'s for
  hand-built test literals; `SuccessShape::plain()` replaces
  `legacy_response()` (the lone non-stream shape without facts: what every
  pre-v2 endpoint stored).
- Builder lowering stores the validated shape list (`EndpointBuilder::build`);
  the "stored once `Endpoint::response` holds response model v2 shapes"
  refusal is gone. Explicit `ErrorSet::none()` versus an omitted policy is
  unchanged (`no_declared_errors`).
- `Endpoint::success_shapes()` returns `&[SuccessShape]` (the stored list);
  the hidden `Endpoint::shapes_preview` seam is deleted and every test that
  set it assigns `response`. `Endpoint::json_references()` now includes SSE
  and NDJSON item types, so API build and bundle load reject a dangling
  stream item reference (R23 "missing registered item type").
- Pagination (`define/src/pagination/{resolve,validate}.rs`,
  `gen` `check_pagination`) reads the endpoint's **sole** JSON shape; an
  endpoint with several shapes is never an inferred list, and an authored
  collection on one reports `the response is not a single JSON shape` (was
  `the response is not JSON`).
- Postman's composition warning (`gen/src/export/vocabulary.rs`) reads the
  first success shape, the one Postman describes (R29).
- `x-schematic`: `SchematicOpExtension::response` is now a `ResponseShape`
  (the first shape). Its serialized form for JSON/Text/Binary/Empty equals the
  removed `ApiResponse`'s, so committed OpenAPI artifacts are byte-identical
  for every untouched endpoint; `success` still lists every shape when
  `response` cannot say everything.
- **OpenAPI import now runs `classify_responses` in production**
  (`mappings::map_operation` takes the API name; the legacy `map_responses`,
  `map_response_ref`, and `ResponseContentClass` are deleted). An operation
  without a representable success is an import error for that operation
  (R28), reported as the existing "Failed to map operation" diagnostic.
  `OpenApiImport::remap_reference_names` now renames JSON success, stream
  item, request, and JSON error references.
- `definition_gen` emits the plain `returns_*` call for a lone plain shape and
  `Success::…` builder chains otherwise (unchanged behavior, now off the
  stored list).

#### Finding and decision: the OpenAI definition is importer output

`definitions/src/openai/mod.rs` is written by `just import-openai`, and
`gen/tests/l1/import_overlay.rs::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`
requires a re-import to reproduce it byte for byte. The spec does not
anticipate this. Switching production import to `classify_responses` (plan
Wave 9; Phase 4's message) makes a re-import declare:

- JSON or binary + SSE on six streaming endpoints (`CreateSpeech`,
  `CreateTranscription`, `CreateChatCompletion`, `CreateImageEdit`,
  `CreateImage`, `CreateResponse`), which D-19 leaves unmigrated, and whose
  D-15 enum `CreateChatCompletionResponse` collides with the registered type
  of that name, so generation would fail;
- endpoint error entries (400/404 `Error`, `ErrorResponse`, `BetaError`) on
  14 endpoints, changing their failure types against R24 ("only a migration
  that deliberately opts into declared errors changes that endpoint's failure
  type");
- binary-first shape lists on `RetrieveVideoContent`, `GetSkillContent`, and
  `GetSkillVersionContent` (binary + JSON), where the old importer chose JSON;
- binary `application/sdp` on `CreateRealtimeCall`, which the old importer did
  not recognize and declared `Empty` with a "no suitable response" warning.

**Correction made during the phase:** the multi-shape endpoint the import
produces is `CreateResponse` (POST `/responses`), not `GetResponse`. The
vendored `specs/openai/openapi.yaml` lists only `application/json` for
`getResponse`; OpenAI's current published document (Phase 1 evidence) lists
both. So R32b's stream cannot come from the import.

**Decision (R24 and D-19 are decided; R28 and the plan's import switch are
decided; the import invariant is a repository rule):** commit the re-imported
`openai/mod.rs` (`types.rs` is unchanged) and scope it in the hand-written
overlay `definitions/src/openai_pagination.rs`, which the importer never
writes: every imported endpoint keeps one shape, the one the old importer
chose (the first JSON shape, else the first), and no errors; then
`stream_get_response` authors R32b: `GetResponse` keeps its JSON
`BetaResponse` and gains the SSE `BetaResponseStreamEvent` shape, each with a
note (the `stream` query parameter selects the reply, R13a). Migrating
another OpenAI endpoint later is a function in the same overlay.

Alternatives rejected:

- accept the whole re-import: changes 20 unmigrated endpoints' generated types
  (R24, D-19) and fails generation on the D-15 collision;
- keep the legacy importer for OpenAI: a second import path, which the plan
  forbids ("do not introduce a persistent second public response system"),
  and R28 requires the v2 mapping;
- hand-edit `openai/mod.rs`: the next `just import-openai` would undo it, and
  the import-invariant test fails.

The one unmigrated endpoint whose type still changes is `CreateRealtimeCall`
(`()` → `Bytes`): reproducing the old `Empty` would mean re-creating an
importer defect (the SDP answer was discarded). It is listed as a migration
with its reason. **Review agenda:** confirm the overlay narrowing, and
whether the OpenAI streaming endpoints should be migrated in a follow-up (each
needs an author-chosen enum or variant name for D-15).

`GetResponse` (R32b): JSON `BetaResponse` + SSE `BetaResponseStreamEvent`,
ending at connection close (no Responses terminator is documented; source:
<https://github.com/openai/openai-openapi/blob/main/openapi.json>, whose
`GET /responses/{id}` 200 lists `application/json` and `text/event-stream`
and documents terminal `response.completed`/`failed`/`incomplete` events,
which are items). Test:
`openai_pagination::tests::get_response_returns_json_or_server_sent_events_and_every_other_endpoint_keeps_one_shape`
(GetResponse's two shapes and notes; `CreateChatCompletion`,
`CreateResponse`, `CreateSpeech`, and `RetrieveVideoContent` keep the old
importer's shape; every other endpoint has one shape and no errors).

#### Definition baselines (R22)

`gen/tests/l1/definition_baseline.rs` implements the Phase 1 allow-list:

- `lower_single_responses` rewrites every historical single `response` into
  `[{"shape": …}]` and recomputes that endpoint's fingerprint as `snapshot`
  does, so unchanged endpoints compare equal; a list is left in place.
- `RESPONSE_MIGRATIONS` names one `(api, endpoint, reason)` per deliberate
  migration; only that endpoint's `response`/`errors` and fingerprint may
  differ, and an entry that allows no difference fails as stale.
- `MIGRATED_COMPONENTS` names one `(api, component, reason)` per component a
  migration registers or corrects; those registry rows are compared nowhere
  else, and an entry naming no current component fails as stale.
- Tests: `a_historical_single_response_is_lowered_and_a_changed_one_is_still_reported`
  (a historical `Text` mutated to `Binary` is still reported at
  `…/response/0/shape` and the fingerprint) and the extended
  `historical_normalization_lowers_basic_and_unused_usernames_only` (the
  lowering touches only `response` and fingerprints, and equals
  `[{"shape": raw}]`). `changed_description_is_detected_at_its_exact_field`
  still passes, so an unrelated description mutation is still detected.

#### Bundle matrix (success-shape columns, deferred from Phase 2)

`define/tests/bundle_support/mod.rs` gains `Converse` (JSON with note and
variant `Whole`, SSE `Event` ending at event `done` kept, binary fallback
`Attachment`), `Submit` (two JSON shapes tagged `kind`, a byte stream
`audio/mpeg`, an empty shape at 204), and `Follow` (NDJSON ending at field
`kind = "end"`, status 200); the committed fixture was regenerated by the
bundle-tool path (only those additions and the `response` array form
differ; literals reviewed, no secrets). `bundle_json_matrix()` gains 87
cells over the frozen success and stream columns. Default outcome: a located
error, nothing loaded. Exceptions that load: `statuses` absent or `[]`;
`fallback` and `keep` absent or `false`; `content_type`, `note`, `variant`
absent; `note: ""`; `DataLineEquals("")`; tag and terminator value `""`.
Covered per column: control, absent, null, wrong type, one/every invalid
element (collections), empty, nested required members, semantic rules
(wildcard and shared media, empty-shape and fallback content types, status
domain and duplicates, 204 on a body, missing/repeated/non-scalar tags, tag
on a stream, fallback on JSON, variant identifier/uniqueness/single shape,
dangling SSE and NDJSON item references, terminator/framing compatibility,
`keep` at connection close), and a duplicate key. Five existing cells changed
only their expected line numbers. Smell grep over the changed readers: no new
`#[serde(default)]`, `unwrap_or_default()`, `.ok()`, or
`filter_map(.. as_str())` on a load-bearing member.

#### Import remap test

`define/tests/openapi_tests.rs::a_component_that_yields_its_name_is_renamed_in_success_stream_and_error_references`:
Petstore's `CreatePetRequest` yields its name to the `CreatePet` request
struct; an SSE alternative and a 400 error naming it now import as
`CreatePetRequest2` and the imported API's references resolve (without the
remap extension the stream item would dangle).

### Wave 10 — Bitbucket `GetSource` (R32a, main session)

`GetSource` declares `Directory` (JSON `PaginatedResponse<SourceEntry>`) and
`File` (binary fallback), each with a note. **R32a finding:** no header tells
a file from a directory (Phase 1: only `Content-Disposition: attachment`,
not a spec wire tag), and worse than the spec anticipates: every member of
`PaginatedResponse<T>` defaults, so a committed JSON *object* file (for
example `package.json`, served as `application/json`) decodes **successfully
as an empty `Directory`**, not as `MalformedSuccess`; only a non-object JSON
file fails to decode. The `Directory` note records the limitation. Shape set
kept as the spec directs. Test: `bitbucket::tests` (`GetSource` shapes,
variant names, fallback, the note's limitation text).

**Forced dependent edits (R34), sniff** (`sniff/lib/src/remote/bitbucket.rs`,
`sniff/lib/src/remote/envelope.rs`):

- directory walk (`collect_directory_entries`): matches
  `GetSourceResponse::Directory(listing)`; a `File` reply is a `RemoteApi`
  error naming the path (the tree then degrades to unavailable, as an
  undecodable listing did before). Type change at this call site:
  `bytes::Bytes` → `GetSourceResponse`. The hand decoder
  `decode_directory_listing` and its two unit tests are removed: the runtime
  decodes the listing now.
- file read (`get_file_content`): reads through the generated client's
  `send_raw` with the same `GetSourceRequest`, classified by the new
  `Replies::raw` (error status as for a call; cut-off body or unfollowed
  redirect without a status), so a file's bytes arrive whatever media type
  Bitbucket sends. This keeps sniff's behavior and is not exposed to the R32a
  limitation above.
- New sniff L1 tests in `sniff/lib/tests/l1/remote_providers.rs`:
  `a_committed_json_file_is_read_as_its_raw_text` (an `application/json`
  `package.json` arrives whole) and
  `a_json_reply_that_is_not_a_listing_leaves_the_tree_unavailable` (a JSON
  array → `MalformedSuccess` → tree unavailable). The existing
  `a_malformed_directory_listing_leaves_the_tree_unavailable_not_empty`
  (`text/plain` body) now exercises the `File` branch.

### Wave 10 — Provider definitions (parallel subagents)

Each subagent owned one provider directory, verified with scoped commands
only (its module's `definitions` tests, `schematic-gen validate --api <key>`,
clippy, and its `definition_baseline` case, expected to fail until the
coordinator added the allow-list entries). Provider evidence is the Phase 1
"Provider inventory and evidence" (read 2026-10-22; no provider was called).

**Anthropic** (`definitions/src/anthropic/`; new `types/streaming.rs`):

- `CreateMessage`: JSON `MessageResponse` (note "returned when `stream` is
  false or omitted") + SSE `MessageStreamEvent` (note "returned when `stream`
  is true"), `EventNamed("message_stop")` consumed. `stream` stays an ordinary
  request field. Generated enum `CreateMessageResponse` (no collision).
- `MessageStreamEvent` (`#[serde(tag = "type")]`): `MessageStart`,
  `ContentBlockStart`, `ContentBlockDelta` (`TextDelta`, `InputJsonDelta`,
  `ThinkingDelta`, `SignatureDelta`), `ContentBlockStop`, `MessageDelta`,
  `MessageStop`, `Ping`, `Error { error: StreamErrorDetail }` (R11c). New
  public types: `MessageStreamEvent`, `ContentBlockDelta`, `StreamMessage`
  (the `message_start` payload, whose `stop_reason` is null, so
  `MessageResponse` could not be reused), `MessageDelta`, `MessageDeltaUsage`,
  `StreamErrorDetail`.
- Recorded as unsupported (decode as `Malformed` items, stream continues,
  R10): unknown future event types; content blocks `ContentBlock` does not
  model (`server_tool_use`, `web_search_tool_result`, `redacted_thinking`,
  the server `fallback` block; the JSON reply has the same limitation);
  `citations_delta` (a real delta not on the streaming page). Not modeled:
  beta `input_transformations`.
- Tests: `anthropic::tests::create_message_returns_json_or_message_stream`;
  `anthropic::types::streaming::tests::{message_start_decodes_with_null_stop_reason,
  message_start_decodes_without_usage, content_block_start_decodes_text_tool_use_and_thinking,
  content_block_delta_decodes_every_delta_kind, content_block_stop_decodes,
  message_delta_decodes_with_and_without_usage, message_stop_and_ping_decode_as_unit_variants,
  error_event_decodes_as_typed_variant, unknown_event_type_fails_to_decode}`.

**LM Studio** (`definitions/src/lmstudio/`):

- `Chat`: JSON `ChatResult` (note "returned when `stream` is false or
  omitted"; named to avoid the D-15 enum `ChatResponse`) + SSE
  `ChatStreamEvent` (note "returned when `stream` is true"),
  `EventNamed("chat.end")` **kept** (adopted default, Phase 1 agenda item 2:
  the terminator carries the aggregated result).
- `ChatStreamEvent` (`#[serde(tag = "type")]`): all 19 documented events by
  name (`chat.start`, `model_load.{start,progress,end}`,
  `prompt_processing.{start,progress,end}`, `reasoning.{start,delta,end}`,
  `tool_call.{start,arguments,success,failure}`, `message.{start,delta,end}`,
  `error`, `chat.end`); `error` is a typed variant and not a terminator (the
  docs say `chat.end` still follows). Unknown events → `Malformed` items.
- `DownloadModel`: one JSON `DownloadModelResponse` — **deviation from R31's
  premise** (Phase 1 agenda item 1; the provider documents a JSON job object
  polled through `GetDownloadStatus`). `DownloadModelResponse` is now
  registered and its fields match the docs (`job_id` optional, `status`
  required, new optional `total_size_bytes`, `started_at`, `completed_at`);
  nothing else used it. Its description drops the false "with SSE progress"
  (explained baseline leaf). Doc comments fixed: `ChatBody.stream` defaults
  to **false** (was documented as true); `ChatBody`/`DownloadModelBody` no
  longer claim SSE.
- **Public types removed:** `lmstudio::ChatStreamChunk`, `lmstudio::ChatUsage`
  (unregistered, unused, and describing a chunk format LM Studio does not
  send; superseded by `ChatStreamEvent`). New: `ChatResult`, `ChatStreamEvent`,
  `ChatOutputItem`, `ChatStats`, `InvalidToolCallMetadata`,
  `ToolProviderInfo`, `ChatStreamErrorDetail`. `model-citizen`'s
  `ListModelsResponse`/`ModelInfo` unchanged. Not added (request scope):
  `DownloadModelBody.quantization`.
- Tests: `lmstudio::tests::{chat_returns_json_result_or_sse_events_ending_at_kept_chat_end,
  download_model_returns_one_json_job_object, chat_result_decodes_non_streaming_reply,
  chat_stream_event_decodes_progress_and_delta_events, chat_stream_event_decodes_tool_call_events,
  chat_stream_event_decodes_error_into_typed_variant, chat_stream_event_decodes_chat_end_aggregated_result,
  chat_stream_event_rejects_unknown_event_type, download_model_response_decodes_job_and_already_downloaded}`.

**ElevenLabs** (`definitions/src/elevenlabs/`):

- `StreamSpeech`: one byte stream (`Success::bytes()`), no content type (R5:
  no audio media type is documented), connection close.
- `StreamSpeechWithTimestamps`: one NDJSON stream of
  `SpeechWithTimestampsResponse`, connection close (line framing rests on the
  official Python SDK's `iter_lines()` + `json.loads`; the docs say only "a
  stream of JSONs").
- **Definition defect fixed:** `SpeechWithTimestampsResponse` and
  `HttpAlignment` used camelCase wire names; the provider sends snake_case
  (`audio_base64`, `normalized_alignment`, `character_start_times_seconds`,
  …). Rust field names are unchanged; the non-streaming
  `CreateSpeechWithTimestamps`, which shares the type, decodes the provider's
  replies correctly now. `WebSocketAlignment` (the WebSocket API's camelCase
  type) is untouched. `biscuit-speaks` uses none of these.
- Unchanged (R5 evidence absent): CreateSpeech, CreateSoundEffect,
  GetHistoryItemAudio, DownloadHistoryItems, GetVoiceSampleAudio (one binary
  shape, no content type; default `output_format` `mp3_44100_128`).
  `output_format` query parameters are declared on no TTS endpoint; adding
  them is new scope (logged, not done). The 422 `{"detail": …}` error shape is
  not declared (D-19).
- Discrepancy recorded: `schematic/docs/research/elevenlabs-api.md` labels
  audio streaming `text/event-stream` (not evidence of SSE, R33) and uses
  camelCase field names; the research page was not edited.
- Tests: `elevenlabs::tests::{audio_endpoints_store_one_binary_shape_without_content_type,
  stream_speech_stores_one_byte_stream_without_content_type,
  stream_speech_with_timestamps_stores_one_ndjson_stream_ending_at_close}`;
  `elevenlabs::types::tests::{speech_with_timestamps_decodes_snake_case_line,
  speech_with_timestamps_rejects_camel_case_spelling}` (camelCase
  `audioBase64` fails: the field is required; a camelCase
  `normalizedAlignment` is ignored and leaves the field `None`, recorded as
  observed behavior).

**Ollama** (`definitions/src/ollama/`):

- Native `Generate`/`Chat`: NDJSON `GenerateStreamItem`/`ChatStreamItem`
  (note "returned when `stream` is true or omitted"),
  `FieldEquals { done = true }` **kept**, plus JSON `GenerateCompletion`/
  `ChatCompletion` (note "returned when `stream` is false"); selected by
  `Content-Type` (`application/x-ndjson` vs `application/json`, unverified on
  the wire, as R30 says). `stream` stays an ordinary request field.
- Native `PullModel`/`PushModel`/`CreateModel`: NDJSON `PullModelStreamItem`
  (pull and push; `PushModelProgress` stays an alias) /
  `CreateModelStreamItem`, `FieldEquals { status = "success" }` **kept**
  (adopted default, Phase 1 agenda item 4: these lines have no `done`), plus
  JSON `StatusResponse` for `stream: false`.
- OpenAI-compatible `ChatCompletions`/`Completions`: JSON
  `OpenAIChatCompletionResponse`/`OpenAICompletionResponse` (note "returned
  when `stream` is false or omitted") + SSE `OpenAIChatStreamChunk`/new
  `OpenAICompletionStreamChunk` (note "returned when `stream` is true"),
  `DataLineEquals("[DONE]")` consumed — **unverified provider behavior**
  (R30 rules it; no Ollama doc confirms `[DONE]`).
- R11c: each native item type is an untagged enum whose `Error(StreamErrorLine
  { error })` variant is tried first; every chunk/progress type has required
  fields, so no chunk swallows an error line, and an error line never matches
  the terminator. The OpenAI-compatible SSE documents no error event:
  recorded as unsupported. If Ollama closes right after an error line, the
  stream reports `EndedEarly` after the error item (reasoned, not tested).
- D-15 collision resolved as the contract freeze directs: **public types
  renamed** `ollama::GenerateResponse` → `GenerateCompletion`,
  `ollama::ChatResponse` → `ChatCompletion` (no Rust use outside the module).
  Variant names stay derived (`Stream`, `Json`). All 13 stranded structs are
  now registered and reachable from shapes; the `PushModelProgress` alias is
  not registered separately. New: `StreamErrorLine`, the four `*StreamItem`
  enums, `StatusResponse`, `OpenAICompletionStreamChunk`. New registrations
  are appended after `OpenAIModel`, so no existing registry index moved.
- Not done (narrow change): the chunk types still omit `load_duration`,
  `prompt_eval_duration`, `eval_duration`, and `done_reason`, which serde
  ignores, so they are not delivered on the kept final line. Logged as a
  follow-up candidate.
- Tests: `ollama::tests::{native_generate_endpoint, native_chat_endpoint,
  native_pull_model_endpoint, native_push_model_endpoint, native_create_model_endpoint,
  openai_chat_completions_endpoint, openai_completions_endpoint,
  module_group_preserves_endpoint_and_registry_inventory}` (full stored shapes);
  serde: `generate_stream_item_decodes_a_text_chunk`,
  `generate_stream_item_decodes_the_final_line_with_statistics`,
  `generate_stream_item_decodes_an_error_line_as_the_error_variant`,
  `chat_stream_item_decodes_chunk_final_and_error_lines`,
  `pull_model_stream_item_decodes_progress_success_and_error_lines`,
  `create_model_stream_item_decodes_progress_and_error_lines`,
  `status_response_decodes_the_buffered_reply`,
  `openai_completion_stream_chunk_decodes_an_sse_data_payload`,
  `openai_chat_stream_chunk_decodes_an_sse_data_payload`.

Baseline allow-list entries for every migration above (endpoints and
components, each with its reason) are in `definition_baseline.rs`; the one
explained leaf is LM Studio `DownloadModel`'s description.

### Wave 11 — Regeneration and dependents (main session)

**Regeneration** (pre-authorized; `cargo run -p schematic-gen -- generate
--api all --output schema/src --openapi-out openapi --postman-out postman`,
the `just generate` command without its desktop notification). Changed:
`schema/src/{anthropic.rs, bitbucket/requests.rs, elevenlabs/requests.rs,
lmstudio.rs, openai/requests.rs}`, and `schema/src/ollama.rs` replaced by the
generator's split `schema/src/ollama/` directory; `openapi/{anthropic,
bitbucket, elevenlabs, lmstudio, ollama, openai}.json`; `postman/{elevenlabs,
lmstudio, ollama, openai}.postman_collection.json`. Every other API's
artifacts are byte-identical. A second generation run produced no further
change.

**Generator change found by `just lint-schema`:** a generated success enum
holding a large reply (`GetResponseResponse::Json(BetaResponse)`) trips
`clippy::large_enum_variant`. The generator now emits
`#[allow(clippy::large_enum_variant)]` on its success and failure enums
(`gen/src/codegen/responses.rs`, with the reason at the attribute): a variant
holds a whole reply built once per call, and boxing would only make the
caller's type worse.

**Changed public types (R24), the migration list.** Compared per endpoint
(`type Success`/`type Failure` of every `EndpointSpec` impl) between `HEAD`
and the regenerated `schema/src`; no `Failure` type changed anywhere:

| API | Endpoint | Success before → after |
| --- | --- | --- |
| Ollama native | `Generate` | `Bytes` → `GenerateResponse` (NDJSON `GenerateStreamItem` stream or JSON `GenerateCompletion`) |
| Ollama native | `Chat` | `Bytes` → `ChatResponse` |
| Ollama native | `PullModel` | `Bytes` → `PullModelResponse` |
| Ollama native | `PushModel` | `Bytes` → `PushModelResponse` |
| Ollama native | `CreateModel` | `Bytes` → `CreateModelResponse` |
| Ollama OpenAI | `ChatCompletions` | `Bytes` → `ChatCompletionsResponse` |
| Ollama OpenAI | `Completions` | `Bytes` → `CompletionsResponse` |
| LM Studio | `Chat` | `Bytes` → `ChatResponse` (JSON `ChatResult` or SSE `ChatStreamEvent`) |
| LM Studio | `DownloadModel` | `Bytes` → `DownloadModelResponse` |
| Anthropic | `CreateMessage` | `MessageResponse` → `CreateMessageResponse` |
| ElevenLabs | `StreamSpeech` | `Bytes` → `ByteStream` |
| ElevenLabs | `StreamSpeechWithTimestamps` | `SpeechWithTimestampsResponse` → `ItemStream<SpeechWithTimestampsResponse>` |
| Bitbucket | `GetSource` | `Bytes` → `GetSourceResponse { Directory, File }` |
| OpenAI | `GetResponse` | `BetaResponse` → `GetResponseResponse` |
| OpenAI | `CreateRealtimeCall` | `()` → `Bytes` (importer correction, see above) |

Public definition types re-exported by the schema crate also changed:
renamed `ollama::GenerateResponse` → `GenerateCompletion` and
`ollama::ChatResponse` → `ChatCompletion` (the generated enums now take those
names); removed `lmstudio::ChatStreamChunk` and `lmstudio::ChatUsage`;
`lmstudio::DownloadModelResponse` fields corrected; ElevenLabs
`SpeechWithTimestampsResponse`/`HttpAlignment` wire names corrected (Rust
names unchanged); new stream event and item types per provider above.

**Hand-written schema test updated:** `schema/tests/non_json_endpoints.rs::binary_endpoint_decodes_into_bytes`
now matches `GetSourceResponse::File(body)` (an `image/png` reply selects the
binary fallback); the bytes assertion is unchanged.

**Generator tests updated for the new truth:**
`gen/tests/l1/e2e_generation.rs::elevenlabs_binary_endpoints_generate_correctly`
asserts `StreamSpeech` as `ByteStream` and `StreamSpeechWithTimestamps` as
`Ndjson<SpeechWithTimestampsResponse, UntilClose>`; its shared
`assert_endpoint_methods` allows the one async fn generated code may define,
a success enum's `Alternatives::decode_alternative`, while still refusing
any send method.

**Forced dependent edits found** (R34): only sniff (see Wave 10 Bitbucket
above). `biscuit-speaks` (ElevenLabs `create_speech`, `create_sound_effect`,
`list_voices`: types unchanged), `model-citizen` (Ollama
`ShowModelBody`/`ShowModelResponse`, LM Studio `ListModelsResponse`/`ModelInfo`:
unchanged), and the homelab crates (EverSolo, Samsung, UC: unchanged) needed
no edit.

**Pre-existing guard fixed in passing** (this phase edits both files): the
`test-toolkit::archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`
failure recorded since Phase 2 named `env!("CARGO_MANIFEST_DIR")` in
`schematic/definitions/tests/authoring_surface.rs` and
`schematic/gen/tests/l1/definition_baseline.rs`. Both now use
`biscuit_test_harness::manifest_dir!()`; `schematic-definitions` gains
`biscuit-test-harness` as a dev-dependency (pre-authorized test dependency
and lockfile change; `Cargo.lock` gains only that edge).
`schematic/docs/dependencies.md` has no `schematic-definitions` section at
all, so none was added for one dev-dependency. The source-scan guard's
forbidden list now names `SuccessShape::` (a hand-built shape bypasses the
builders) instead of the removed `ApiResponse::json`.

**Second fresh-output fix:** `fresh_output_compile::slow_real_apis_compile_offline`
hard-coded `ollama.rs`; Ollama's generated module group is now large enough
that the generator writes it as `ollama/` (`mod.rs`, `requests.rs`,
`responses.rs`), so the test edits `ollama/mod.rs`.

### Adopted defaults and decisions (Phase 5)

- **decision: `ApiResponse` removed** (contract freeze), thin
  `SuccessShape` constructors added for hand-built fixtures; `plain()`
  replaces `legacy_response()`.
- **decision: `SchematicOpExtension::response` is a `ResponseShape`** (the
  first shape), keeping committed OpenAPI `x-schematic` output byte-identical.
- **decision: pagination reads a sole JSON shape** (a multi-shape endpoint is
  never a list; authored collections on one are rejected).
- **decision: OpenAI overlay narrows the v2 import** and authors R32b (see the
  finding above). Review agenda.
- **decision: `CreateRealtimeCall` `()` → `Bytes` accepted** as an importer
  correction, not re-created. Review agenda.
- **decision: sniff reads Bitbucket files through `send_raw`** rather than the
  typed enum, so the R32a `.json` limitation does not reach sniff.
- **decision: generated success/failure enums allow
  `clippy::large_enum_variant`.**
- **adopted default: LM Studio `chat.end` kept; Ollama pull/push/create end at
  `status = "success"` kept; LM Studio `DownloadModel` is JSON** (Phase 1
  agenda items 1, 2, 4).
- **adopted default: ElevenLabs declares no audio content type** (R5: none
  documented); with-timestamps is NDJSON on SDK evidence (R33).
- **adopted default: Ollama buffered types renamed** `GenerateCompletion`,
  `ChatCompletion` (contract freeze resolution of the D-15 collision).
- D-26: no hooks existed to remove (Phase 1 inventory); none removed.

### Requirement-to-test mapping (Phase 5 scope)

Tiers: all L1 (no tier marker in any new name), compiled by declared
targets: define `tests/*.rs` (autotests; `all-features` in CI metadata),
gen `tests/l1/` (declared in `main.rs`; no new file), `schematic-definitions`
lib unit tests, sniff `tests/l1/remote_providers.rs` behind the `remote`
feature (in sniff's `[package.metadata.ci.tests] features`), and the
generated crate's `schema/tests/`. Boundaries: the define crate's public API
(F1); the definitions crate's built APIs (F1); the real generator over the
real definitions with `syn`-checked output and the fresh-output compile
(F2); sniff's provider through wiremock (F2).

| Req | Behavior | Test |
| --- | --- | --- |
| R1, R22 (storage) | builder stores every shape with its facts, in order; repeated success adds a shape; inseparable repeats rejected | `define/tests/response_vocabulary.rs::{builder_stores_every_success_shape_with_its_facts, several_shapes_are_validated_at_build_with_inferred_json_tags}`; `define/tests/builders.rs::repeated_success_and_invalid_required_facts_report_errors` |
| R22 (serialized form) | `response` is a list; the pre-v2 form is rejected; dangling/composition/empty-key references in the new form | `define/tests/builders.rs::{deserialized_definitions_are_checked_by_the_same_resolution, transferred_components_and_references_input_matrix}` |
| R22 (baselines) | historical single responses lowered; named migrations only; unrelated mutations still reported | `gen/tests/l1/definition_baseline.rs` (18 API cases, `a_historical_single_response_is_lowered_and_a_changed_one_is_still_reported`, `historical_normalization_lowers_basic_and_unused_usernames_only`, `changed_description_is_detected_at_its_exact_field`) |
| R23 | stream item references checked at API build and bundle load | bundle cells `sse item dangling`, `ndjson item dangling` |
| R24 | untouched endpoints keep types; changed public types equal the migration list; single plain shape output unchanged | `artifact_drift::*`, `fresh_output_compile::slow_real_apis_compile_offline`, the per-endpoint comparison in Wave 11, `just test-schema`, `just lint-schema` |
| R25 | regenerated, idempotent, compiles, lints | generation run twice (no change), `just test-schema` 55/55, `just lint-schema` clean |
| R28 (production) | import runs `classify_responses`; remap reaches stream items and errors | `define/tests/openapi_tests.rs::a_component_that_yields_its_name_is_renamed_in_success_stream_and_error_references`; `define/src/openapi/import/builder.rs` tests; `response_import_matrix::*` now through production; `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` |
| R29 | Postman composition warning reads the first shape | `gen/tests/l1/postman_*`, `vocabulary_exports` (unchanged expectations, green) |
| R30, R11c | Ollama shapes, ends, keep, notes; error lines typed | `ollama::tests::*` and the `ollama::types` serde tests listed above |
| R31, R11c | LM Studio shapes; 19 events; error typed; DownloadModel JSON | `lmstudio::tests::*` listed above |
| R32, R11c | Anthropic JSON + SSE; every event; ping/error typed; unknown → malformed | `anthropic::tests::create_message_returns_json_or_message_stream`, `anthropic::types::streaming::tests::*` |
| R32a | Bitbucket `Directory`/`File`; limitation recorded; sniff reads files raw and walks listings | `bitbucket::tests` (GetSource); `schema/tests/non_json_endpoints.rs::binary_endpoint_decodes_into_bytes`; sniff `remote_providers::bitbucket_tests::{a_committed_json_file_is_read_as_its_raw_text, a_json_reply_that_is_not_a_listing_leaves_the_tree_unavailable, a_malformed_directory_listing_leaves_the_tree_unavailable_not_empty, get_file_content_returns_the_raw_bytes_as_text}` |
| R32b | GetResponse JSON + SSE with notes; OpenAI scope narrowing | `openai_pagination::tests::get_response_returns_json_or_server_sent_events_and_every_other_endpoint_keeps_one_shape` |
| R33 | ElevenLabs byte stream and NDJSON; snake_case | `elevenlabs::tests::*`, `elevenlabs::types::tests::*`; `e2e_generation::elevenlabs_binary_endpoints_generate_correctly` |
| R34 | dependents compile and pass | see gates below |
| R44 | bundle round trip with success shapes; matrix | `define/tests/definition_bundles.rs::{committed_fixture_is_the_bundle_tool_output, committed_fixture_loads_and_exports_like_the_built_definition, bundle_files_round_trip_byte_equal_through_repeated_writes_and_reads, bundle_json_matrix}`; `gen/tests/l1/definition_bundles.rs` over the shipped (migrated) definitions |

**Deferred to the phase that owns the boundary (plan):** mock-server tests
through the regenerated provider clients (Ollama NDJSON/SSE incremental
delivery and final statistics, Anthropic variants by `Content-Type`,
Bitbucket `Directory`/`File` selection with a JSON listing and a
`text/x-rust` body, OpenAI `GetResponse` variants, ElevenLabs first audio
chunk before completion) are Phase 6 Wave 13; OpenAPI/Postman export
assertions over the migrated APIs are Phase 6 Wave 12.

### Review agenda entries raised in Phase 5

1. **OpenAI overlay narrowing** of the v2 import (and whether to migrate the
   OpenAI streaming endpoints later; each needs author-chosen D-15 names).
2. **`CreateRealtimeCall` `()` → `Bytes`**, an importer correction outside the
   named migration list.
3. **R32a is worse than the spec says:** a JSON *object* file decodes as an
   empty `Directory` (all `PaginatedResponse` members default); options for
   later: make `values` required (changes every Bitbucket list type's
   decoding), or a Bitbucket-specific tag such as `pagelen`.
4. **LM Studio public types removed** (`ChatStreamChunk`, `ChatUsage`) and
   `DownloadModelResponse` fields corrected.
5. **Ollama chunk statistics:** `load_duration`, `prompt_eval_duration`,
   `eval_duration`, `done_reason` are not modeled, so the kept final line
   drops them.
6. **Earlier phases' sniff evidence:** the "sniff remote tests 75/75" of
   Phases 3 and 4 ran without sniff's `remote` feature, which gates
   `remote_providers` and the generated-client call sites; with
   `--features remote` the same filter runs 297 tests (all passing now).

### Phase 5 gates

Run once, after every subagent had finished, from `schematic/` unless noted:

- `just lint`: clean (after fixing clippy findings in this phase's code:
  `useless_vec`, `needless_borrow`, `into_iter` on a slice).
- `just test`: **2630 passed, 15 skipped** (the same environment-gated
  skips; Phase 4: 2599), fresh-output **9/9**, compile-fail **1/1**, exit 0.
  The first run failed only `fresh_output_compile::slow_real_apis_compile_offline`
  (hard-coded `ollama.rs`, fixed above); the rerun passed in full. That rerun
  overlapped a cold compile and marked 16 tests slow; rerun alone, eight at
  once, the slowest (`artifact_drift::generated_rust_artifacts_are_up_to_date`,
  `import_overlay::…`, `shipped_openai_query_schema_gate_and_reimports`,
  `generated_rest_code_references_the_runtime_and_defines_none_of_it`, TLS
  builder tests) all finish in 1.7–3.9 s: load-sensitive, as Phase 1 recorded.
- `just check-tier-coverage schematic`: 0 stranded.
- `just test-schema`: **55/55** (the pre-existing trybuild pager guard is
  slow by design); `just lint-schema`: clean.
- define doctests (`cargo test --doc -p schematic-define --all-features`):
  167 + 14 passed (not part of `just test`; run because doc examples changed).
- Dependents, serially: `cargo check --tests --all-features` clean for
  `biscuit-speaks`, `model-citizen`, `homelab`, `eversolo-integration`,
  `arcam-amp-integration`, `sony-receiver-integration`,
  `unfolded-integration-helper`; `cargo clippy -p sniff --features remote
  --tests -D warnings` clean. Tests: **sniff `--features remote`** remote,
  Bitbucket, and envelope tests **297/297**; `model-citizen` 153/153 (all);
  `biscuit-speaks` ElevenLabs 48/48; `homelab` 49/49.
- Scope-calculator extras: `darkmatter` skill-reading tests
  (`shipped_schema_corpus`, `schema_roots`, `schema_phase_validation`,
  `current_root*`) 55/55; `test-toolkit::archive_path_guard` 2/2 (green for
  the first time since Phase 2, see above).
- Cross-OS check: not run. No path-handling, process, console, or `#[cfg]`
  production code changed; the test path edits use `manifest_dir!()` joins
  and a `/`-separated relative join that `Path::join` accepts on Windows.
- Nothing was staged or committed; `cargo fmt` was not run.

Scope calculator, run from the repository root after every edit (including
this log, the plan, and their frontmatter):

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `packages: [schematic-define, schematic-definitions, schematic-gen,
sniff, test-toolkit, darkmatter]`, `change_class: package`. So both gates ran
(above); `just lint` was re-run on the final tree after the archive-guard
fix: clean.

## Phase 6

Run of 2026-10-22. Tree: HEAD `4aba5e94c` plus the **uncommitted Phase 5
work** (Phase 5 was not committed before this run, so the working tree and
the scope calculator below see both phases' files). Implementer:
`claude/opus` as coordinator. The R26 extension change and regeneration ran
in the main session. Wave 12 (define tests; gen tests over the shipped
definitions) and Wave 13 (provider mock tests) ran as three subagents on
disjoint files.

**Deviation from the plan's ordering:** Wave 13 ran concurrently with Wave
12 instead of after it. It depends only on Phase 5's regenerated clients, not
on Wave 12's tests, and the files are disjoint. The subagents also ran in the
background rather than the foreground, because of a coordinator dispatch
slip. The coordinator waited for all three and read every result before it
continued, so no work was abandoned.

### R26 gap closed: `cross_origin` and request-id in `x-schematic`

R26 requires the extension to carry endpoint `cross_origin` and request-id.
Neither was serialized (the Phase 4 wiring carried the response vocabulary,
the error levels, and the 4b rate-limit and retry members only). Added:

- `SchematicDocExtension::cross_origin: CrossOrigin`, written only when it is
  not the `SameOrigin` default (absent reads back as the default), and
  `SchematicDocExtension::request_id_header: Option<String>`.
- `SchematicOpExtension::cross_origin: Option<CrossOrigin>` holds the
  endpoint override and is absent when the endpoint inherits.
- Filled from `RestApi`/`Endpoint` in `openapi/export/{mod,paths}.rs`.
- Deprecation and the endpoint auth override stay in standard OpenAPI
  (`deprecated`, `security`, `securitySchemes`), as the spec's 2026-10-22
  refresh of R26 adopts.

**adopted default: omit the default policy** (source: R26 names the members
but not their form). Writing `SameOrigin` on every document would change
all 18 committed OpenAPI artifacts for no information; absent and default
are equivalent on read. Regenerated with the pre-authorized
`cargo run -p schematic-gen -- generate --api all --output schema/src
--openapi-out openapi --postman-out postman`. The **only new artifact
diffs** are one `"request_id_header"` line each in `openapi/github.json`
(`x-github-request-id`) and `openapi/openai.json` (`x-request-id`). No
shipped API sets a non-default policy or an endpoint override. `schema/src`
and Postman are unchanged. Docs: `docs/io/openapi-extensions.md` (both field
tables, plus the note that deprecation and auth are standard members).
Skill: `.claude/skills/schematic-define/SKILL.md` (extension member lists).

### Wave 12 — Export/import checks

**OpenAPI coverage** (`define/tests/response_openapi.rs`; gen
`tests/l1/response_adapters.rs` over the shipped definitions):

- Every success and error kind exports its media key and schema: JSON,
  text with and without a type (`text/plain` default), binary with and
  without a type (`application/octet-stream` default), empty at 204, SSE,
  NDJSON, and byte stream as a binary string, with framing and end in
  `x-schematic.success`. Error entries cover exact empty, binary, text, and
  JSON, plus class defaults. A class is expanded where an exact entry
  supersedes part of it and kept elsewhere. Success and error keys are
  disjoint (R21). The document parses back through `openapiv3` from JSON and
  YAML.
- The R26 integration test exports a built API and asserts doc and op
  `cross_origin`, `request_id_header`, `rate_limit`, and `retry_guidance`;
  standard `deprecated`, `security` (`[]` for a public endpoint, absent when
  inheriting) and `securitySchemes`; and nothing of these inside
  `x-schematic`. A default-policy API writes neither member.
- Shipped APIs: ElevenLabs audio endpoints and `StreamSpeech` export one 200
  under `application/octet-stream` as a binary string, `StreamSpeech` with a
  `Bytes` stream in `x-schematic`. `StreamSpeechWithTimestamps` exports
  `application/x-ndjson` with the item `$ref`, and every declared query
  parameter survives export (R5). Anthropic `CreateMessage` has JSON + SSE
  with both notes; Bitbucket `GetSource` has JSON + `*/*` (binary); Ollama
  `Generate`/`Chat` have NDJSON + JSON and `ChatCompletions` has JSON + SSE;
  OpenAI `GetResponse` has JSON + SSE. Each document is re-parsed through
  `openapiv3` from JSON and YAML.

**Finding (R5):** no ElevenLabs TTS or audio endpoint declares a query
parameter (`output_format` is not modeled; Phase 5 logged adding it as new
scope). The R5 parameter assertion compares declared with exported names, so
today it compares two empty lists. It will catch a dropped parameter once one
is declared.

**Import coverage** (`define/tests/response_openapi.rs`):

- A full public `OpenApiImport` round trip runs from the exported document as
  JSON and as YAML. It covers JSON + SSE + `*/*` fallback, a tagged JSON
  group exported as `oneOf` + `discriminator` (`$ref` members, so names and
  tag values survive), binary `audio/mpeg`, and text `text/markdown`.
  Imported shapes equal the originals. The imported API's `errors` is empty,
  and every status from 400 to 599 looks up the same endpoint-level shape
  and content type (D-20, no hoisting). Re-export gives the same responses
  map.
- Import reads only the responses map, by design: a non-close SSE end comes
  back as `ConnectionClose`, a byte stream comes back as `Binary`, and variant
  names are not restored, because reading `x-schematic` back is out of scope
  (R26). The test asserts this, and that the re-exported `content` is
  identical.
- Already covered by Phase 4 and confirmed: the `default`, 1xx, 3xx, and
  multiple-error-media diagnostics, 204 → empty, no-success rejection,
  deduplication across 200/201, inline `oneOf` → `serde_json::Value`, and the
  JSON and YAML matrices (`response_import_matrix.rs`, whose fixture already
  is the real exporter's document; unchanged).

**Finding (round trip):** where an exact error supersedes part of a class,
export expands the class into exact keys described `Declared error
400-499`. Import makes each an exact entry, so a second export describes
them `Declared error 418`. Keys, content, and schemas are identical, and the
test asserts the difference. **Review agenda:** decide whether this matters.

**Postman coverage** (`gen/tests/l1/response_adapters.rs`): over the
shipped collections, Ollama `Generate` (NDJSON first) records `Response
content type: application/x-ndjson`, while JSON-first `ChatCompletions` and
Anthropic `CreateMessage` record nothing (R29). `postman_golden.rs` needed
no change: its fixtures are hand-built and still green. R42's GraphQL skip
was already asserted by `both_exporters_skip_graphql_operations_with_a_warning`
for API, operation, feature, target, warning severity, and an output that
still builds; it was confirmed rather than extended.

### Wave 13 — Shipped runtime checks

**Provider verification** (`schema/tests/streaming_providers.rs`, new, 10
tests through the regenerated `schematic-schema` clients). Incremental
delivery is proven with `HeldServer`, a raw TCP server on port 0. It sends
the head and the first body part, then waits on a `oneshot` that the test
fires only after it has received the first item; an `AtomicBool` confirms
the rest was unsent at that point. A buffering client would fail on the 5 s
bound instead of hanging. In hold mode the connection stays open after the
terminator, which proves the stream ended at the terminator and not at
close. Each test takes about 0.05 s and every call is `.anonymous()`.

| Test | Proves |
| --- | --- |
| `ollama_generate_ndjson_is_delivered_incrementally_and_keeps_its_final_line` | R30, R8, R11: incremental NDJSON; the kept `done:true` line is last, with `eval_count`, `prompt_eval_count`, `total_duration`; ends while the connection is open; R13a `"stream":true` sent |
| `ollama_generate_mid_stream_error_line_is_a_typed_item` | R11c: `{"error":…}` is `GenerateStreamItem::Error`, and the stream continues; R13a omitted `stream` is absent from the body |
| `ollama_generate_json_reply_selects_json_variant` | R30: `application/json` gives `GenerateResponse::Json`; `stream:false` sent as set |
| `ollama_chat_completions_sse_is_delivered_incrementally_and_done_is_consumed` | R30, R8: incremental SSE; `[DONE]` ends the stream and is not yielded |
| `anthropic_json_reply_selects_json_variant_and_sends_stream_as_set` | R32, R13a: JSON variant for `stream:false` and for omitted; bodies sent as set |
| `anthropic_sse_reply_delivers_events_in_order_and_consumes_message_stop` | R32, R11c, R8: ordered events including `ping` and a typed `error`; `message_stop` consumed and nothing after it yielded |
| `bitbucket_get_source_selects_directory_or_file_by_media_type` | R32a: JSON listing gives `Directory` with decoded entries; `text/x-rust` gives `File` with exact bytes |
| `openai_get_response_json_reply_selects_json_variant` | R32b: JSON variant |
| `openai_get_response_sse_ends_cleanly_at_connection_close` | R32b: SSE events yielded and a clean end at close; `?stream=true` sent |
| `elevenlabs_stream_speech_yields_audio_before_the_body_completes` | R33: the first audio chunk is observed before the server sends the second; the bytes concatenate exactly |

The definition tests for every migrated endpoint (Phase 5's
`ollama::tests`, `lmstudio::tests`, `anthropic::tests`, `elevenlabs::tests`,
`bitbucket::tests`, `openai_pagination::tests`, and the serde tests for the
LM Studio framing and end, and the ElevenLabs timestamps and media) ran
inside `just test` and pass. No hook-removal or provider-evidence entry
changed (D-26: no hooks existed).

**Finding: OpenAI `BetaResponseStreamEvent` decodes most events as the wrong
variant.** The importer generates it (`definitions/src/openai/types.rs`) as a
`#[serde(untagged)]` enum whose variants take `type_: String` and accept
unknown fields, so serde picks the first variant whose required fields are
present:

- `{"type":"response.output_text.delta","item_id":"i","output_index":0,
  "content_index":0,"delta":"hi","sequence_number":3,"logprobs":[]}` decodes
  as `BetaResponseAudioDeltaEvent`;
- `{"type":"response.completed","sequence_number":4,"response":{…}}` decodes
  as `BetaResponseAudioDoneEvent`, and the `response` payload is lost.

R32b's shape (JSON or SSE by `Content-Type`) works, and the test uses
`response.audio.delta` and `response.audio.done`, which decode correctly; a
code comment there says why. The defect is in how the importer generates
every OpenAI `oneOf` without a discriminator, not in this spec's response
model. Fixing it (a `type` literal check, or internal tagging, in the import
type generator) changes the whole imported OpenAI type file, which is new
scope; it is logged and left undone. **Review agenda.**

**Test-placement observation:** `schematic/schema` is outside the Cargo
workspace, and CI never runs `just test-schema` (only the fresh-output guard
fetches its lockfile). These tests therefore run locally only, like the
existing `non_json_endpoints.rs`, `github_client.rs`, and `pagination.rs`
beside them. The plan says "through regenerated clients", which is the
committed `schema` crate; the CI-run alternative (child tests in
`fresh_output_compile`'s 18-API crate) would add a full test-profile build of
every API to the CI build job. **Review agenda:** whether CI should run
`test-schema`.

### Requirement-to-test mapping (Phase 6 scope)

Tiers: all L1, and no new name carries a tier marker. Each test is compiled
by a declared target: define `tests/response_openapi.rs` (autotests,
`openapi` feature, which CI enables through `all-features`), gen
`tests/l1/response_adapters.rs` (already declared in `l1/main.rs`), define
unit tests in `extensions.rs`, and `schema/tests/streaming_providers.rs`
(the generated crate's autotests, run by `just test-schema`; see the
observation above). Boundaries: the define public exporter and importer
(F1, in-process, with an `openapiv3` re-parse); the real definitions through
the gen exporters (F2); the real regenerated clients against loopback mocks
(F2).

| Req | Test |
| --- | --- |
| R5, R33 (export) | `response_adapters::shipped_elevenlabs_audio_exports_octet_stream_and_keeps_query_parameters` |
| R21, R27 | `response_openapi::every_success_and_error_kind_exports_its_media_key_and_schema`; `response_adapters::{shipped_anthropic_create_message_exports_json_and_sse_with_both_notes, shipped_bitbucket_get_source_exports_json_and_a_fallback, shipped_ollama_streams_export_beside_their_json_shapes, shipped_openai_get_response_exports_json_and_sse}` |
| R26 | `extensions::tests::cross_origin_and_request_id_are_written_only_when_declared`; `response_openapi::x_schematic_carries_cross_origin_request_id_and_guidance_while_deprecation_and_auth_stay_standard`; `artifact_drift::*` (the two regenerated lines) |
| R28 | `response_openapi::{public_import_of_the_exported_json_and_yaml_keeps_shapes_content_types_and_endpoint_errors, import_reads_only_the_responses_map_so_stream_ends_and_byte_streams_come_back_plain}`, plus the Phase 4 tests listed under Import coverage |
| R29 | `response_adapters::shipped_postman_collections_record_the_first_success_content_type` |
| R42 | `response_adapters::both_exporters_skip_graphql_operations_with_a_warning` (Phase 4, confirmed) |
| R30, R8, R11, R11c, R13a | the five Ollama tests in `streaming_providers.rs` |
| R32, R13a, R11c | the two Anthropic tests |
| R32a | `bitbucket_get_source_selects_directory_or_file_by_media_type` |
| R32b | the two OpenAI tests |
| R33 (runtime) | `elevenlabs_stream_speech_yields_audio_before_the_body_completes` |

**Input robustness:** no reader changed in this phase. The two new
`x-schematic` members are write-only, because import does not read the
extension (R26), so the matrix does not apply. The OpenAPI JSON and YAML
response matrices from Phase 4 still pass through production import. Smell
grep over the change: the one new `#[serde(default)]` pairs with a
skip-when-default serializer on a write-only member, and adds no
`unwrap_or_default()`, `.ok()`, or `filter_map(.. as_str())`.

### Review agenda entries raised in Phase 6

1. The OpenAI `BetaResponseStreamEvent` mis-decoding, which comes from the
   importer's untagged `oneOf` generation (finding above).
2. CI does not run `schematic/schema` tests, so the Wave 13 provider tests
   are local-only.
3. Error descriptions change on an export, import, export round trip after
   D-25 expansion (keys and schemas are identical).
4. The adopted default that `x-schematic` omits a `SameOrigin` API policy.

### Phase 6 gates

Run once, after every subagent had finished, from `schematic/` unless noted:

- `just lint`: clean.
- `just test`: **2641 passed, 15 skipped** (the same environment-gated
  skips; Phase 5: 2630), fresh-output **9/9**, compile-fail **1/1**, exit 0.
  The 13 tests marked slow are the load-sensitive set Phase 5 recorded
  (`artifact_drift`, `import_overlay`, `e2e_generation`, the split-output
  tests, `shipped_openai_query_schema_gate_and_reimports`, the TLS builder
  test). The host load average was 36–85, partly from other sessions. No new
  test is among them.
- `just check-tier-coverage schematic` (repository root): 0 stranded.
- `just test-schema`: **65/65** (55 + 10 new); `just lint-schema`: clean.
- Subagents' scoped runs (before the gates): define `response_openapi`,
  `response_import_matrix`, `openapi_tests`, and `cross_origin_policy` 51/51;
  define `extensions::` 30/30; gen `response_adapters`, `postman_golden`, and
  `artifact_drift` 23/23; define and gen clippy `-D warnings` clean.
- Dependents: no Rust type a dependent uses changed. `schema/src` is
  byte-identical, and every `SchematicDocExtension`/`SchematicOpExtension`
  user is inside `schematic/define`. sniff, biscuit-speaks, model-citizen,
  and the homelab crates were therefore not re-run; Phase 5's runs stand.
- Cross-OS check: not run. No path, process, console, or `#[cfg]`
  production code changed. The new schema tests use loopback TCP only, and CI
  does not run them.
- Nothing was staged or committed; `cargo fmt` was not run.

Scope calculator, run from the repository root after every edit (including
this log, the plan, and their frontmatter):
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
python3 scripts/ci/affected_scope.py`. Result: `packages: [schematic-define,
schematic-definitions, schematic-gen, sniff, test-toolkit, darkmatter]`,
`change_class: package`. That is identical to Phase 5's result, because the
uncommitted Phase 5 files dominate it. Both gates were required and ran
(above). The sniff, test-toolkit, and darkmatter cells come from Phase 5
files; Phase 6 changed nothing those packages read.
