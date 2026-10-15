---
spec: /Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-11-schematic-generation-integrity/spec.md
plan: schematic/features/2026-10-11-schematic-generation-integrity/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
  - schematic/gen/src/cargo_gen.rs
  - schematic/gen/tests/common/mod.rs
  - schematic/gen/tests/common/fresh_output.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
  - schematic/justfile
  - .config/nextest.toml
  - .github/workflows/ci.yml
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_1:
  - schematic/README.md
  - .github/ci/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - schematic/skill/SKILL.md
  - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_2:
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/src/errors.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/src/pipeline.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/output/assemble/mod.rs
  - schematic/gen/src/codegen/client/mod.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/mod.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/e2e_generation.rs
docs_updated_during_phase_2:
  - schematic/gen/README.md
docs_created_during_phase_2:
  - schematic/docs/generation/validation.md
skills_files_updated_during_phase_2:
  - schematic/skill/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_3:
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/src/pipeline.rs
  - schematic/gen/tests/l1/artifact_drift.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/openapi/ollama.json
docs_updated_during_phase_3:
  - schematic/docs/generation/validation.md
  - schematic/docs/io/export-openapi.md
  - schematic/gen/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - schematic/skill/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_4:
  - schematic/gen/src/cargo_gen.rs
  - schematic/gen/src/errors.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/mod.rs
  - schematic/gen/src/output/inventory.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/output/publish.rs
  - schematic/gen/src/pipeline.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/partial_generation.rs
  - schematic/gen/tests/level2/terminal_capture.rs
  - schematic/justfile
docs_updated_during_phase_4:
  - schematic/README.md
  - schematic/gen/README.md
  - schematic/docs/generation/validation.md
docs_created_during_phase_4:
  - schematic/docs/generation/partial-generation.md
skills_files_updated_during_phase_4:
  - schematic/skill/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
  - .claude/skills/schematic-define/schematic/SKILL.md
source_files_during_phase_5:
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/variant.rs
  - schematic/gen/src/codegen/client/methods.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/request_structs/into_parts.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/shared.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/schema/Cargo.lock
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/artificial_analysis/client.rs
  - schematic/schema/src/artificial_analysis/requests.rs
  - schematic/schema/src/bitbucket/client.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/client.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/emqx/client.rs
  - schematic/schema/src/emqx/requests.rs
  - schematic/schema/src/eversolo/client.rs
  - schematic/schema/src/eversolo/requests.rs
  - schematic/schema/src/gitea/client.rs
  - schematic/schema/src/gitea/requests.rs
  - schematic/schema/src/github/client.rs
  - schematic/schema/src/github/requests.rs
  - schematic/schema/src/gitlab/client.rs
  - schematic/schema/src/gitlab/requests.rs
  - schematic/schema/src/huggingface/client.rs
  - schematic/schema/src/huggingface/requests.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama/client.rs
  - schematic/schema/src/ollama/requests.rs
  - schematic/schema/src/openai/client.rs
  - schematic/schema/src/openai/requests.rs
  - schematic/schema/src/samsung_smart_tv.rs
  - schematic/schema/src/shared.rs
  - schematic/schema/src/unfolded_circle_core_rest.rs
docs_updated_during_phase_5:
  - schematic/gen/README.md
  - schematic/docs/generation/validation.md
docs_created_during_phase_5:
  - schematic/docs/generation/request-side.md
skills_files_updated_during_phase_5:
  - schematic/skill/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
  - .claude/skills/schematic-define/SKILL.md
source_files_during_phase_6:
  - schematic/gen/src/codegen/request_structs/body.rs
  - schematic/gen/src/codegen/request_structs/into_parts.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/named_params.rs
  - schematic/gen/src/codegen/request_structs/shared.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/mod.rs
  - schematic/gen/src/codegen/client/helpers.rs
  - schematic/gen/src/codegen/client/methods.rs
  - schematic/gen/src/codegen/client/mod.rs
  - schematic/gen/src/codegen/client/shared_auth.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/api_struct/constructors.rs
  - schematic/gen/src/codegen/api_struct/fields.rs
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/api_struct/mod.rs
  - schematic/gen/src/codegen/api_struct/variant.rs
  - schematic/gen/src/output/assemble/shared.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/define/src/types.rs
  - schematic/schema/src/shared.rs
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/samsung_smart_tv.rs
  - schematic/schema/src/unfolded_circle_core_rest.rs
  - schematic/schema/src/artificial_analysis/mod.rs
  - schematic/schema/src/artificial_analysis/client.rs
  - schematic/schema/src/bitbucket/mod.rs
  - schematic/schema/src/bitbucket/client.rs
  - schematic/schema/src/elevenlabs/mod.rs
  - schematic/schema/src/elevenlabs/client.rs
  - schematic/schema/src/emqx/mod.rs
  - schematic/schema/src/emqx/client.rs
  - schematic/schema/src/eversolo/mod.rs
  - schematic/schema/src/eversolo/client.rs
  - schematic/schema/src/gitea/mod.rs
  - schematic/schema/src/gitea/client.rs
  - schematic/schema/src/github/mod.rs
  - schematic/schema/src/github/client.rs
  - schematic/schema/src/gitlab/mod.rs
  - schematic/schema/src/gitlab/client.rs
  - schematic/schema/src/huggingface/mod.rs
  - schematic/schema/src/huggingface/client.rs
  - schematic/schema/src/ollama/mod.rs
  - schematic/schema/src/ollama/client.rs
  - schematic/schema/src/openai/mod.rs
  - schematic/schema/src/openai/client.rs
  - schematic/schema/src/openai/requests.rs
docs_updated_during_phase_6:
  - schematic/docs/generation/request-side.md
  - schematic/docs/generation/validation.md
  - schematic/gen/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - schematic/skill/SKILL.md
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_7:
  - schematic/gen/src/codegen/request_enum.rs
  - schematic/gen/tests/common/fresh_output.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/artificial_analysis/requests.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/emqx/requests.rs
  - schematic/schema/src/eversolo/requests.rs
  - schematic/schema/src/gitea/requests.rs
  - schematic/schema/src/github/requests.rs
  - schematic/schema/src/gitlab/requests.rs
  - schematic/schema/src/huggingface/requests.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama/requests.rs
  - schematic/schema/src/openai/requests.rs
  - schematic/schema/src/samsung_smart_tv.rs
  - schematic/schema/src/unfolded_circle_core_rest.rs
docs_updated_during_phase_7:
  - schematic/README.md
  - schematic/gen/README.md
  - schematic/docs/dependencies.md
  - schematic/docs/generation/validation.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - schematic/skill/SKILL.md
  - .claude/skills/schematic-define/SKILL.md
  - .claude/skills/schematic-define/schematic/gen.md
  - .claude/skills/os/build-hosts.md
packages:
  - schematic-gen
  - test-toolkit
  - schematic-schema
  - schematic-define
source_code:
  - schematic/gen/src/cargo_gen.rs
  - schematic/gen/tests/common/mod.rs
  - schematic/gen/tests/common/fresh_output.rs
  - schematic/gen/tests/l1/fresh_output_compile.rs
  - schematic/gen/tests/l1/main.rs
  - schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt
  - schematic/justfile
  - .config/nextest.toml
  - .github/workflows/ci.yml
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - schematic/gen/src/diagnostics.rs
  - schematic/gen/src/validation.rs
  - schematic/gen/src/errors.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/src/pipeline.rs
  - schematic/gen/src/output/mod.rs
  - schematic/gen/src/output/assemble/mod.rs
  - schematic/gen/src/codegen/client/mod.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/postman/mod.rs
  - schematic/gen/src/test_utils.rs
  - schematic/gen/tests/l1/output_validation.rs
  - schematic/gen/tests/l1/openapi_strict_completeness.rs
  - schematic/gen/tests/l1/e2e_generation.rs
  - schematic/gen/tests/l1/artifact_drift.rs
  - schematic/openapi/ollama.json
  - schematic/gen/src/output/inventory.rs
  - schematic/gen/src/output/publish.rs
  - schematic/gen/tests/l1/partial_generation.rs
  - schematic/gen/tests/level2/terminal_capture.rs
  - schematic/gen/src/codegen/error.rs
  - schematic/gen/src/codegen/variant.rs
  - schematic/gen/src/codegen/client/methods.rs
  - schematic/gen/src/codegen/client/tests.rs
  - schematic/gen/src/codegen/request_structs/into_parts.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/shared.rs
  - schematic/gen/src/codegen/request_structs/tests.rs
  - schematic/gen/tests/l1/http_client.rs
  - schematic/gen/tests/l1/path_substitution.rs
  - schematic/gen/tests/l1/query_params_codegen.rs
  - schematic/schema/Cargo.lock
  - schematic/schema/src/anthropic.rs
  - schematic/schema/src/artificial_analysis/client.rs
  - schematic/schema/src/artificial_analysis/requests.rs
  - schematic/schema/src/bitbucket/client.rs
  - schematic/schema/src/bitbucket/requests.rs
  - schematic/schema/src/elevenlabs/client.rs
  - schematic/schema/src/elevenlabs/requests.rs
  - schematic/schema/src/emqx/client.rs
  - schematic/schema/src/emqx/requests.rs
  - schematic/schema/src/eversolo/client.rs
  - schematic/schema/src/eversolo/requests.rs
  - schematic/schema/src/gitea/client.rs
  - schematic/schema/src/gitea/requests.rs
  - schematic/schema/src/github/client.rs
  - schematic/schema/src/github/requests.rs
  - schematic/schema/src/gitlab/client.rs
  - schematic/schema/src/gitlab/requests.rs
  - schematic/schema/src/huggingface/client.rs
  - schematic/schema/src/huggingface/requests.rs
  - schematic/schema/src/lmstudio.rs
  - schematic/schema/src/ollama/client.rs
  - schematic/schema/src/ollama/requests.rs
  - schematic/schema/src/openai/client.rs
  - schematic/schema/src/openai/requests.rs
  - schematic/schema/src/samsung_smart_tv.rs
  - schematic/schema/src/shared.rs
  - schematic/schema/src/unfolded_circle_core_rest.rs
  - schematic/gen/src/codegen/request_structs/body.rs
  - schematic/gen/src/codegen/request_structs/named_params.rs
  - schematic/gen/src/codegen/mod.rs
  - schematic/gen/src/codegen/client/helpers.rs
  - schematic/gen/src/codegen/client/shared_auth.rs
  - schematic/gen/src/codegen/api_struct/constructors.rs
  - schematic/gen/src/codegen/api_struct/fields.rs
  - schematic/gen/src/codegen/api_struct/helpers.rs
  - schematic/gen/src/codegen/api_struct/mod.rs
  - schematic/gen/src/codegen/api_struct/variant.rs
  - schematic/gen/src/output/assemble/shared.rs
  - schematic/define/src/types.rs
  - schematic/schema/src/artificial_analysis/mod.rs
  - schematic/schema/src/bitbucket/mod.rs
  - schematic/schema/src/elevenlabs/mod.rs
  - schematic/schema/src/emqx/mod.rs
  - schematic/schema/src/eversolo/mod.rs
  - schematic/schema/src/gitea/mod.rs
  - schematic/schema/src/github/mod.rs
  - schematic/schema/src/gitlab/mod.rs
  - schematic/schema/src/huggingface/mod.rs
  - schematic/schema/src/ollama/mod.rs
  - schematic/schema/src/openai/mod.rs
  - schematic/gen/src/codegen/request_enum.rs
documentation:
  - schematic/README.md
  - .github/ci/README.md
  - schematic/skill/SKILL.md
  - .claude/skills/rust-devops/ci-cd.md
  - schematic/gen/README.md
  - schematic/docs/generation/validation.md
  - .claude/skills/schematic-define/schematic/gen.md
  - schematic/docs/io/export-openapi.md
  - schematic/docs/generation/partial-generation.md
  - .claude/skills/schematic-define/schematic/SKILL.md
  - schematic/docs/generation/request-side.md
  - .claude/skills/schematic-define/SKILL.md
  - schematic/docs/dependencies.md
  - .claude/skills/os/build-hosts.md
completed_phase: 7
implemented: true
---

# Implementation Log for 2026-10-11-schematic-generation-integrity (7 phases)

## Phase 1

### Authorization state (Task 1.2)

- The spec is `status: draft-spec`. No recorded author confirmation of
  pre-authorization items 1–8 exists in the spec, the plan, or the macro
  review (`D50`–`D57`). Item 9 (the R35 CI build-job step) is confirmed by
  D57 item 1.
- This run was started by the author's explicit request to "implement phase 1
  of the plan". Phase 1 cannot exist without item 5 (temporary crates and
  Cargo build output outside the checkout), so item 5 is read as authorized by
  that request **for Phase 1's compile guard only**. This is an interpretation,
  recorded for review 1.
- Item 7 (remove or rename the `#[ignore]`d e2e tests and adjust `test-e2e`) is
  **not** exercised: the two ignored compile/clippy tests are retained and
  `test-e2e` keeps working. See "Default 9" below.
- Items 1, 2, 3, 4, 6, 8 are not needed by Phase 1 and were not exercised.

### Worktree state before any edit

- Branch `feat/modern-config`. Pre-existing changes not made by this run:
  `darkmatter/fixes/2026-10-11-clean-fidelity/spec.md` (modified) and the
  untracked `plan.md` files of the seven schematic feature directories.
- Baseline `just test` in `schematic/` (before any edit): **1708 passed,
  5 skipped, exit 0**. Baseline `just lint`: see "Validation checkpoint".

### Recipe behavior found (Task 1.2)

- `just test` runs `_test_all`; locally that is one nextest run whose L1
  filter (`just/devops.just` `_tier_filter`) **excludes `slow_` tests** unless
  `BISCUIT_L1_INCLUDE_SLOW=1`. `schematic-gen` does not set
  `l1-include-slow`, so a plain `slow_` test would not run under `just test`.
  This conflicts with R17 ("`just test` runs it"); resolved below.
- `just check-drift` runs `cargo test -p schematic-gen artifact_drift -- --ignored`,
  which is stale twice over: the drift tests are no longer `#[ignore]`d, and the
  consolidated `l1` binary must not be run by `cargo test`. Left for the phase
  that owns drift docs; the working invocation is
  `cargo nextest run -p schematic-gen --test l1 -E 'test(/^artifact_drift::/)'`.
- `just test-e2e` runs `cargo test -p schematic-gen --test l1 -- --ignored e2e_generation::`
  (`cargo test`, not nextest). `release` calls it.

### Live registry counts (R32, Task 1.2)

Measured on 2026-10-11 by a read-only traversal of
`schematic_gen::pipeline::resolve_all_apis()` and
`schematic_definitions::apis_by_module()`, compiled as a throwaway crate under
`/tmp` (nothing added to the checkout).

| Measure | Live | Parent figure |
| --- | --- | --- |
| REST APIs | 18 | 18 |
| Module groups | 15 (`ollama` = OllamaNative+OllamaOpenAI, `emqx` = EmqxBasic+EmqxBearer, `artificial_analysis` = Data+CritPt) | — |
| Endpoints | **549** (OpenAI 265) | about 519 (OpenAI 265) |
| JSON request bodies | 152 | — |
| Multipart (`FormData`) bodies | **16** endpoints, 47 fields | about 29 |
| URL-encoded bodies | 0 | — |
| Text bodies | 0 | 0 |
| Binary bodies | 0 | 0 |
| `ApiKeyParam` APIs (legacy `auth`, explicit methods, or env fallback) | 0 | 0 |
| Endpoints with header parameters | **7** | 0 |
| Endpoints with cookie parameters | 0 | 0 |
| Endpoints with declared query parameters | 117 | — |
| POST/PUT/PATCH with no request body | 18 | "six POST empty bodies" |
| Paths containing `?` | 9 | 9 |

Per API: Anthropic 4, ArtificialAnalysisCritPt 1, ArtificialAnalysisData 6,
Bitbucket 16, ElevenLabs 42, EmqxBasic 36, EmqxBearer 38, Eversolo 24, GitHub 17,
GitLab 19, Gitea 16, HuggingFaceHub 26, LmStudio 6, OllamaNative 11,
OllamaOpenAI 4, OpenAI 265, SamsungSmartTv 7, UnfoldedCircleCoreRest 11.

Reconciliation:

- **"six" vs "about 29" does not reconcile** with the live data. Live multipart
  is 16 endpoints (47 fields); neither figure matches. Body-less
  POST/PUT/PATCH is 18 (12 OpenAI, 1 ElevenLabs, 1 EmqxBearer, 3 SamsungSmartTv,
  1 UnfoldedCircleCoreRest), not six. Both parent figures are recorded as
  stale measures of different things.
- The endpoint total is 549, not about 519.
- **Header parameters are not zero.** The seven OpenAI `Beta*` `/responses`
  endpoints declare an `openai-beta` header parameter of type
  `Array(Enum(["responses_multi_agent=v1"]))`. These are the same seven
  endpoints that carry `?beta=true`. When Phase 6 implements header parameters
  (R25), the **real** OpenAI output changes; that diff is explained by "a
  feature the live definitions already use" (R31), not fixture-only work.
- The nine `?` paths match R33 exactly: OpenAI `BetaCreateResponse`,
  `BetaGetResponse`, `BetaDeleteResponse`, `BetaCancelResponse`,
  `BetaCompactconversation`, `BetaListInputItems`, `BetaGetinputtokencounts`;
  Gitea `GetGitTreeRecursive`; GitLab `GetRepositoryFile` (`?ref={git_ref}`,
  a path-parameter placeholder inside the embedded query).

### Consumer and dependent search (Task 1.2)

- `rg -l "into_parts|RequestParts" --type rust` outside `schematic/gen` and
  `schematic/schema`: hits only in unrelated types (`biscuit-file`,
  `darkmatter`, `sniff`'s own `aggregate_view::into_parts`) and the macro-review
  spike under `schematic/reviews/`. **No consumer of the generated
  `RequestParts` or `into_parts`.**
- Packages depending on `schematic-schema` (from their `Cargo.toml`): `sniff`,
  `biscuit-speaks`, `model-citizen`, `homelab`, `eversolo-integration`,
  `unfolded-integration-helper`, `arcam-amp-integration`,
  `sony-receiver-integration`. Plus the stale `schematic/gen/schematic/schema`
  manifest.
- Stale tree `schematic/gen/schematic/`: five tracked files (`schema/Cargo.toml`,
  `schema/src/{lib,openai,prelude,shared}.rs`). The only maintained reference
  outside snapshots is `checks/cargo-machete.txt:10`. **No regeneration recipe
  for `checks/cargo-machete.txt` exists** (`rg cargo-machete justfile just/
  scripts/ .github/` finds nothing), so default 18 resolves to "preserve and log"
  when Phase 7 runs.

### CI inspection (Task 1.2, read-only)

- `ci.yml` `build` job (one per build owner) runs no tests. Its steps end with
  install nextest → build `ci-build` → `scripts/ci/produce-owner.sh` (archive
  production) → publish. Every environment's tests run on archive consumers
  (`_package-ci.yml` native, `_wsl-ci.yml` WSL2).
- Archive consumers' executed and expected listings both come from
  `_tier_filter L1`, with `BISCUIT_L1_INCLUDE_SLOW` taken from the cell's
  `l1_include_slow` (the package's `l1-include-slow`, false for
  `schematic-gen`). So **every `slow_` test of `schematic-gen` is already absent
  from both listings on every consumer** (Linux, macOS, native Windows, WSL2).
- `scripts/ci/completion.py` refuses any extra exclusion predicate beyond the
  canonical tier markers (`schema.CANONICAL_SELECTION`: `slow_`/`perf_` "may"
  be excluded). A new per-test filter would therefore break completion; the
  existing `slow_` exclusion is the only exclusion the contract allows.
- No completion record covers producer-run tests; there is no existing
  "builder-only" test concept.

### Wire baseline (Task 1.1, R29)

- Captured **before any generator edit**, from the current generator's output
  for the real Gitea, GitLab, and OpenAI clients, by
  `fresh_output_compile::slow_query_wire_matches_pre_split_baseline`. The test
  generates all 18 APIs into a temporary crate, adds a test file that sends
  requests to a raw `std::net::TcpListener` on `127.0.0.1:0`, and records the
  literal HTTP request line. A raw listener is used instead of wiremock because
  wiremock reconstructs the URL and could normalize the bytes being compared.
  Clients get a custom base URL pointing at the listener and explicit
  credentials (`test-key`, `test-token`); there is no provider traffic.
- Golden file: `schematic/gen/tests/fixtures/wire_baseline/query_request_lines.txt`
  (LF line endings, enforced by `.gitattributes`). Expectations were copied
  from the first capture, never derived from an encoder.
- Cases: ordered scalars with space, `+`, `%`, Unicode, and reserved
  `&=/?#` (Gitea `ListPullRequests`); an empty value; no query; a legacy
  embedded literal query (`GetGitTreeRecursive`, `?recursive=true`); a base URL
  that carries its own query; a legacy embedded query with a path placeholder
  (GitLab `GetRepositoryFile`, `?ref={git_ref}`); repeated array keys (OpenAI
  `ListConversationItems.include`); and an embedded query plus declared query
  parameters (OpenAI `BetaListInputItems`).
- Pre-split behavior recorded as-is, **not fixed here** (for review 1 and
  the query-split phase):
  - a base URL with a query produces `GET /api/v1?tenant=a%20b/repos/o/r/pulls?page=1`:
    the path is appended after the base URL's query, so the server reads
    `tenant=a b/repos/o/r/pulls?page=1` as one query;
  - `Vec<serde_json::Value>` query arrays are sent JSON-quoted
    (`include=%22a%20b%22`) because the generator calls `Value::to_string()`.
- Remaining declared-query endpoints (117 endpoints in total) are covered by
  the existing generation-pattern tests: `query_params_codegen.rs` (pair
  building, array repetition, `?` vs `&` joining), `path_substitution.rs`
  (path-parameter encoding next to queries), and `http_client.rs`. Every
  declared query uses the same emitted code shape (`query_pairs` + one
  `urlencoding::encode` join), so the eight captured lines exercise each branch
  of that shape.

### Defaults adopted

| Default | Used in Phase 1 | Value actually used |
| --- | --- | --- |
| 8 compile-check mechanism | yes | As stated, with one departure: the temporary lockfile entries are resolved with `cargo update --workspace --offline`, **not** `cargo generate-lockfile --offline`. `generate-lockfile` rebuilds the whole lockfile at the newest cached versions, discarding the copied pins; `update --workspace` adds only the temporary packages and keeps every registry pin. Builds then run `--offline --locked`. Child target: `<outer target>/schematic-fresh-output`, derived from the running test binary's location, so it follows whatever target storage is configured. |
| 9 ignored e2e tests | yes | **Retained** (`generated_code_compiles`, `generated_code_passes_clippy`, `multiple_apis_compile`) because pre-authorization item 7 is unconfirmed. The new guard supersedes the two compile tests (it compiles all 18 APIs, not OpenAI alone), and the clippy test duplicates `just lint-schema` for committed output. `just test-e2e` is unchanged and still uses `cargo test` on the consolidated binary, which the testing rules forbid. Removal and the recipe repair wait for item 7. |
| 10 runtime-test location | yes | The wire-baseline test runs inside a freshly generated crate through nextest, as defaulted. |
| 17 compile-check scope | yes | Two principal guard tests (`slow_fixture_apis_compile_offline`, `slow_real_apis_compile_offline`), the negative control (`slow_negative_control_reports_compiler_error`), and one more for distinct coverage: `slow_query_wire_matches_pre_split_baseline` (R29 needs the real clients, compiled with their dev-dependencies and run). |
| 18 `cargo-machete.txt` | recorded | No regeneration recipe exists; Phase 7 preserves the line and logs it. |
| 21 test file names | yes | `gen/tests/l1/fresh_output_compile.rs`, registered in `l1/main.rs`; the helper is `gen/tests/common/fresh_output.rs`, declared once through `#[path = "../common/mod.rs"] mod common;`. |
| 1–7, 11–16, 19, 20, 22 | no | Adopted as stated in the plan's table; they first apply in Phases 2–7 and are logged there. |

### Compile guard (Task 1.3, R16–R18, R20 partial, R19 partial)

- Helper: `schematic/gen/tests/common/fresh_output.rs` (`FreshCrate`). It
  calls the public entry point `schematic_gen::output::generate_and_write_all`
  into a `tempfile` directory and writes the manifest with the new
  `schematic_gen::cargo_gen::generate_cargo_toml_with_paths`. It copies
  `schematic/schema/Cargo.lock`, runs `cargo update --workspace --offline`
  (temporary entries only), then `cargo check --offline --locked --lib` or
  `cargo nextest run --offline --locked --tests`, with an explicit
  `--manifest-path` and `--target-dir`.
- **Manifest builder fix (outside the test, in `schematic-gen`):**
  `generate_cargo_toml` interpolated paths into TOML basic strings, so a
  Windows root such as `C:\work\schematic` produced `"C:\work\..."`, an
  invalid escape. Both builders now serialize each path with the `toml`
  crate's string writer (`biscuit_file::toml_crate`, already a dependency).
  The default output is byte-identical to the committed `schema/Cargo.toml`,
  pinned by a new unit test.
- Fixture types: generated modules emit
  `pub use schematic_definitions::<module>::*`, and every generated crate also
  emits the WebSocket modules that call the real
  `schematic_definitions::define_*_ws_api()`. Fixture APIs therefore build
  against a stand-in `schematic-definitions` **0.1.1** in the temp dir. It
  re-exports the real 0.1.0 crate (`pub use real::*`) and adds one module of
  fixture types per fixture API. Version 0.1.1 satisfies the generated
  `version = "0.1.0"` requirement and avoids Cargo's lockfile collision
  between two path packages with the same name and version. No fixture
  touches production definitions.
- Child environment: `NEXTEST*`, `CARGO_PKG_*`, `CARGO_BIN_*`,
  `CARGO_TARGET_*_RUNNER`, `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET[_DIR]`,
  `CARGO_BUILD_BUILD_DIR`, `CARGO_MANIFEST_*`, `CARGO_CRATE_NAME`,
  `CARGO_PRIMARY_PACKAGE`, `CARGO_TARGET_TMPDIR` are removed (CI's
  `NEXTEST_PROFILE=ci` would otherwise select a profile the temp crate lacks);
  `CARGO_NET_OFFLINE=true` is set. `RUSTFLAGS` and `RUSTC_WRAPPER` are
  inherited on purpose.
- Serialization and bounds: `.config/nextest.toml` adds test group
  `schematic-fresh-output` (max-threads 1) and a `60s × 15` slow-timeout for
  `binary_id(schematic-gen::l1) & test(/(^|::)slow_/)`. The `ci` profile
  inherits both (verified with `cargo nextest show-config test-groups
  --profile ci`). At the ceiling, nextest kills the test's process group,
  which bounds the child Cargo.
- Checkout isolation: `CheckoutWitness` captures, before and after each
  compile, the relative file list and bytes of `schematic/schema` (excluding
  any `target/`), plus the entry listings of `schematic/`, `schematic/gen/`,
  `schematic/define/`, and `schematic/definitions/`. Nothing else is scanned,
  and no clean worktree is required.
- R20: `slow_real_apis_compile_offline` compares the fresh Rust file list and
  bytes with the committed `schematic/schema/src` before compiling. The
  exported documents reuse `artifact_drift` (OpenAPI and Postman, already
  unconditional). The generated manifest is compared by
  `cargo_gen::tests::default_manifest_matches_the_committed_schema_manifest`.
  A standalone in-process comparison took **12 s** inside the full L1 run,
  over the 5 s L1 budget, so it was folded into the slow test, which generates
  the same output anyway.
- R19 (partial, by design): the fixture set covers bearer, header API key, and
  Basic auth; JSON, multipart, and URL-encoded bodies; scalar and array query
  parameters; and empty, text, and binary responses. OAuth, text and binary
  request bodies, `ApiKeyParam`, and header and cookie parameters join in
  Phase 6. `fixture_coverage_names_every_fixture_api` (ordinary L1) guards
  that `FIXTURE_COVERAGE` and `FIXTURE_TYPES` match the fixture list.

### `just test`, `check-drift`, and the CI build-job step (Tasks 1.3–1.4, R17, R35)

- R17: `schematic/justfile` `test` now runs `_test_all`, then the new
  `test-fresh-output` recipe:
  `cargo nextest run -p schematic-gen --test l1 --no-tests=fail -E 'test(/(^|::)slow_/)'`.
  That is the single explicit selection, shared by local runs and CI.
  `schematic-gen` does **not** declare `l1-include-slow`, deliberately: that
  would put the Cargo-invoking tests on toolchain-free archive consumers.
- `check-drift` was silently selecting nothing: `cargo test … -- --ignored` on
  tests that are no longer ignored. It now runs
  `artifact_drift::` + `fresh_output_compile::slow_real_apis_compile_offline`
  through nextest (6 tests).
- R35: `.github/workflows/ci.yml` `build` job gains "Run the schematic
  fresh-output guard" after "Build the producer tool" and **before** "Produce
  the archives". It runs only when `resolved-plan.json` has a build record with
  `producer == matrix.build` and `package == "schematic-gen"`. It runs
  `cargo fetch --manifest-path schematic/schema/Cargo.toml` (online, CI only;
  the tests themselves stay offline), then `just -f schematic/justfile
  test-fresh-output` under the job's `NEXTEST_PROFILE=ci`. "Keep the
  fresh-output guard report" uploads `target/nextest/ci/test-results.xml` as
  `fresh-output-schematic-gen-<producer>`. `ci-gate` already `needs: build`,
  so a guard failure blocks the merge.
- Archive exclusion: there is no new filter. The consumers' executed and
  expected listings both come from `_tier_filter L1`, which already excludes
  `slow_` for `schematic-gen`. Demonstrated locally:
  `cargo nextest list -p schematic-gen --test l1 -E "$(just _tier_filter L1 schematic-gen)"`
  lists **0** `slow_` tests (only `fixture_coverage_names_every_fixture_api`
  from the new module), while the guard selection lists exactly the four
  `slow_` tests. This holds for native Windows and WSL2, which use the same
  recipe and cell field.
- Contract: new `ci_workflow_contracts::the_build_owner_runs_the_schematic_fresh_output_guard_before_archiving`
  pins the step order, the report artifact, the recipe's selection and
  `--no-tests=fail`, and the absence of `l1-include-slow` in `schematic-gen`.
- **R35 mismatch, left incomplete within the narrow authority:** completion
  records (`completion-{package}-{gate}-{environment}`) cover only cell
  producers. The guard's results are accounted for only by the `build` job's
  pass/fail and its uploaded JUnit, not by a completion record. Adding them
  would change `completion.py` and the report contract, which is beyond
  pre-authorization item 9. Also, when CI reuses `schematic-gen`'s passing
  evidence and plans no build, the guard runs nowhere in CI, and local
  pre-push evidence (`just _test`) does not include `slow_` either.
- CI evidence: no hosted run was triggered (no push is authorized). The
  plan-plus-diff fallback is used: `just ci-local --plan` schedules
  `schematic-gen` L1 on ubuntu-latest, macos-latest, windows-latest, and
  wsl2-ubuntu for this tree, so the guard would run on the Ubuntu and macOS
  build owners for a pull request and on Windows for a `main` push.

### Timing (one host: macOS dev machine, kache rustc wrapper)

| Run | Cache state | Duration |
| --- | --- | --- |
| `slow_fixture_apis_compile_offline` | cold child target (deps in check mode) | 167 s |
| `slow_query_wire_matches_pre_split_baseline` | cold child target for the test profile | 242 s |
| `slow_real_apis_compile_offline` | warm | 18 s alone; 5 s within the guard run |
| `slow_negative_control_reports_compiler_error` | warm | 4–8 s |
| `slow_query_wire_matches_pre_split_baseline` | warm | 23–30 s |
| `just test-fresh-output` (4 tests, serialized) | warm | 15–55 s |
| `just test` (L1 1713 tests + guard) | warm | 83 s total |

A cold CI build owner without kache will pay roughly the cold figures once
per cache miss (the child target sits under `target/`, which `rust-cache`
saves). CI duration is not yet measured.

### Requirement → test mapping (Phase 1)

| Requirement | Test(s) | Result |
| --- | --- | --- |
| R16 isolated, offline, checkout untouched | `fresh_output_compile::slow_fixture_apis_compile_offline`, `::slow_real_apis_compile_offline` (`CheckoutWitness`, `--offline --locked`, crate outside the checkout) | pass |
| R16 TOML-safe manifest on Windows | `cargo_gen::tests::explicit_windows_paths_round_trip_through_toml`, `::explicit_paths_with_quotes_round_trip_through_toml`, `::windows_workspace_root_keeps_backslashes` (fail on the old code: `\w` is an invalid TOML escape) | pass |
| R17 on a normal run, slow L1 | `just test` → `test-fresh-output`; absent from `sanity` (`--lib --bins`, `!slow`) and from the consumers' L1 listing | pass |
| R18 negative control | `fresh_output_compile::slow_negative_control_reports_compiler_error` (asserts failure, `error[E0308]`, and the injected function name in stderr) | pass |
| R19 coverage (supported subset) | `slow_fixture_apis_compile_offline`, `fixture_coverage_names_every_fixture_api`, `slow_real_apis_compile_offline` (asserts 18 APIs) | pass; unsupported features pending Phase 6 |
| R20 determinism | Rust: `slow_real_apis_compile_offline`; manifest: `default_manifest_matches_the_committed_schema_manifest`; documents: existing `artifact_drift::*` | pass |
| R29 baseline | `slow_query_wire_matches_pre_split_baseline` + `tests/fixtures/wire_baseline/query_request_lines.txt` | pass (golden captured pre-change) |
| R32 counts | recorded above | recorded |
| R35 placement | `ci_workflow_contracts::the_build_owner_runs_the_schematic_fresh_output_guard_before_archiving`; listing demonstration above | pass; completion-record gap logged |

Input Robustness Matrix: not applicable. No reader of a file format or
configuration was added. The guard copies a lockfile and lets Cargo parse it,
and the manifest change is a writer.

### Broader gates

- `just test` (schematic): exit 0; L1 **1713 passed, 9 skipped** (was 1708/5;
  +4 `cargo_gen` unit tests, +1 coverage test; the 4 new `slow_` tests are the
  extra skips), then the guard: **4 passed**.
- `just lint` (schematic): exit 0 (clippy `--all-targets -D warnings`; no
  formatter run).
- `just check-tier-coverage schematic`: 0 stranded.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 166 passed.
  `cargo clippy -p test-toolkit --tests --test ci_workflow_contracts -- -D warnings`: clean.
- `just check-drift`: 6 passed.
- Not run: the `scripts/ci/test_*.py` suites (no script under `scripts/ci/`
  changed), and cross-OS runs (below).

### Platform evidence

- macOS (this host): the whole guard passes.
- Linux, native Windows, WSL2: **not run in Phase 1.** `just cross-check`
  runs the package's L1 selection and forwards neither a test filter nor
  `BISCUIT_L1_INCLUDE_SLOW`, so it cannot run `slow_` tests, and raw SSH to
  the rigs is not authorized here. The Windows-specific risks (TOML path
  escaping, writing into a held-open temp file) were addressed in code and
  unit-tested on this host. The first CI run (Ubuntu and macOS owners on a
  pull request, Windows on push) or Task 7.5 must provide this evidence.

### Files touched outside `schematic/`

- `.config/nextest.toml`: test group plus timeout override (repository-wide
  nextest config).
- `.github/workflows/ci.yml`: the R35 build-job steps (pre-authorization 9).
- `.github/ci/README.md`: documents the build-owner exception to "no job owns
  a test suite on CI's behalf".
- `tools/test-toolkit/tests/ci_workflow_contracts.rs`: one contract test
  pinning the placement.
- `.claude/skills/rust-devops/ci-cd.md`: the same exception, for agents.

No dependency was added. `schematic/docs/dependencies.md` is unchanged.

### Comment drift found

- `schematic/README.md` "Testing Requirements" claimed compile coverage via
  `cargo check -p schematic-schema` and told developers to run `cargo test`
  over the packages. It now describes the guard and points at `just test`
  (code assumed correct).
- `check-drift` recipe: its invocation selected no tests (see above); fixed.
- Not changed (unrelated to this phase's behavior, recorded for the Phase 7
  drift pass): `artifact_drift.rs`'s module doc still shows `cargo test` on the
  consolidated binary; `.claude/skills/schematic-define/SKILL.md:173-175` runs
  `cargo test` on `e2e_generation::` tests; `e2e_generation.rs`'s header still
  lists the ignored compilation tests.

### Human review agenda items raised in Phase 1

1. Pre-authorization items 1–8 are unconfirmed. Item 5 was read as authorized
   by the request to implement Phase 1; item 7 was not exercised (the ignored
   e2e tests and `test-e2e` remain).
2. R35's completion-record gap: the guard is accounted for by the `build`
   job's result and an uploaded JUnit artifact, not a completion record, and
   it runs nowhere in CI when `schematic-gen`'s evidence is reused.
3. The stand-in `schematic-definitions` 0.1.1 crate used for fixture types.
4. The `cargo update --workspace --offline` departure from default 8's
   `generate-lockfile`.
5. Four `slow_` tests rather than "two plus a negative control": the
   wire-baseline test is distinct coverage (R29).
6. Live counts that contradict the parent: 549 endpoints, 16 multipart, and
   **7 header-parameter endpoints** (OpenAI `openai-beta`), so Phase 6 will
   change real OpenAI output.
7. Pre-split wire quirks recorded verbatim: a base URL with a query, and
   JSON-quoted `Vec<serde_json::Value>` query items.

## Phase 2

### Authorization state

- No pre-authorization item is exercised by Phase 2: no regeneration of
  committed artifacts, no deletion, no `RequestParts` change, no dependency
  added, no `schematic-define` change, no CI change. Items 1–8 remain
  unconfirmed, as recorded in Phase 1.

### What was built (Tasks 2.1–2.3)

- **`schematic/gen/src/diagnostics.rs` (new, public):** `ValidationDiagnostic
  { severity, api: Vec<String>, endpoint: Option<String>, feature, output,
  message }`, `Severity { Warning, Error }`, `Output { Rust, OpenApi, Postman }`,
  `Feature` (kebab-case ids), constructors `new` (one API) and `for_group`
  (every API of a module group, `endpoint = None`), `is_error`, `has_errors`.
  `Display` renders `severity=<s> api=<a,b> endpoint=<e|<none>> feature=<f>
  output=<o>: <message>`; `Serialize` emits the same ids (`severity`,
  `feature`, `output` as strings, `api` as an array, `endpoint` as null when
  absent).
- **`schematic/gen/src/validation.rs` (rewritten):** whole-batch validators
  `validate_rust_client`, `validate_openapi`, `validate_openapi_registry`,
  `validate_postman`; the single route key `endpoint_uniqueness_key` returning
  `EndpointUniquenessKey { method, normalized_path }`; the severity constants
  `QUERY_IN_PATH_SEVERITY` and `DUPLICATE_METHOD_PATH_SEVERITY`; and
  `pub(crate) ensure_no_errors` used by every public writer.
- **Pipeline (`schematic/gen/src/pipeline.rs`):** `run_generate` now builds a
  batch (one API, or all with `BatchScope::Complete`) and calls the new public
  `generate_batch(apis, scope, opts, registry_for) -> GenerationReport`. Each
  output is validated over the whole batch and published by its own step
  (Rust, then OpenAPI, then Postman); a validation error or a write error fails
  only that output, every requested output is attempted, and the report names
  succeeded and failed outputs. `GenerationReport::into_result` returns
  `GeneratorError::OutputsFailed { succeeded, failed }`, which `main` already
  maps to exit 1. Diagnostics print to stderr as `[WARN]`/`[ERROR]` + the
  rendered line; one `[OK]`/`[FAILED] <output> output` line per output prints
  at the end. `run_validate` (`validate --api`) runs all three validators and
  prints `[PASS]`/`[FAIL]` per output. The Rust step no longer runs before a
  global abort: `Cargo.toml` is written only when the Rust output passes.
- **Same boundary for library callers:** `output::generate_and_write_all`,
  `generate_and_write_standalone`, `export::openapi::write_openapi[_grouped]`,
  and `export::postman::write_postman[_grouped]` call their validator first and
  return `GeneratorError::Validation { output, diagnostics }` without writing.
- **Removed:** `validation::validate_api`, `pipeline::run_validation`,
  `pipeline::missing_schemas_error`, `pipeline::run_generate_all`, and the
  `GeneratorError` variants `NamingCollision`, `InvalidRequestSuffix`,
  `InvalidIdentifier` (no users outside the crate; replaced by diagnostics).
  The OpenAPI registry-completeness error became the diagnostic
  `feature=incomplete-schema-registry output=openapi`, checked for every module
  before any document is written (previously checked module by module, after
  the Rust crate had already been written).

### Checks per output

| Feature id | Rust | OpenAPI | Postman | Severity |
| --- | :-: | :-: | :-: | --- |
| `duplicate-endpoint-id` (raw id twice in one API) | ✓ | ✓ | ✓ | error |
| `duplicate-method-path` (same API, same `endpoint_uniqueness_key`) | ✓ | ✓ | ✓ | `DUPLICATE_METHOD_PATH_SEVERITY` = warning (deviation, below). **Superseded:** error since review-1 fix cycle 1 (ruling 2026-10-12; constant removed) |
| `query-in-path` | ✓ | ✓ | ✓ | `QUERY_IN_PATH_SEVERITY` = warning |
| `invalid-output-filename` (export stem not one safe component; Rust module equal to `lib`/`shared`/`prelude`/`ws_shared`/a WebSocket module) | ✓ | ✓ | ✓ | error |
| `invalid-identifier` (API name, endpoint id, path/query/form names) | ✓ | | | error |
| `invalid-request-suffix` | ✓ | | | error |
| `invalid-module-path` (not one plain identifier; definitions path segments) | ✓ | | | error |
| `invalid-type-syntax` (JSON request/response and form JSON types via `syn::Type`) | ✓ | | | error |
| `generated-name-collision` (request-struct fields, form fields, client convenience methods, module-group type names, referenced type shadowed by a generated item) | ✓ | | | error |
| `incomplete-schema-registry` | | ✓ | | error |
| `text-request-body`, `binary-request-body`, `api-key-param` (through `effective_auth_policy`) | ✓ | | | error |
| `header-parameter`, `cookie-parameter` (not sent) | ✓ | | ✓ | error if `required`, warning otherwise |

### Design choices not dictated by the spec

- **Default 1 (format):** adopted as stated. `api` is a `Vec<String>` so a
  group diagnostic names every contributing API (`api=A,B`); one-API
  diagnostics render a single name. `endpoint=<none>` for API/group scope.
- **Default 2 (machine readability):** adopted. `Serialize` derive, no new
  CLI flag, diagnostics on stderr, nonzero exit only for errors or write
  failures.
- **Default 6 (duplicate id scope):** adopted. Raw ids within one API; across
  a module group, generated Rust names (`{id}{suffix}`, `{id}Form`, `{api}`,
  `{api}Request`, `{api}VariantBuilder`, plus the imported names `Serialize`,
  `Deserialize`, `RequestParts`, `SchematicError`, `Paginated`). Ollama and
  EMQX groups are accepted (`validation::tests::live_shared_groups_are_accepted`).
- **Default 20 (feature ids):** the ten listed ids exist; six were **added**
  because the checks R11 requires need a feature to name:
  `invalid-identifier`, `invalid-request-suffix`, `generated-name-collision`,
  `invalid-module-path`, `invalid-output-filename`,
  `incomplete-schema-registry`. `duplicate-operation-id` exists but is not
  emitted until Phase 3.
- **Output ids:** `rust`, `openapi`, `postman`.
- **Client method collisions** are found by generating the client impl for
  the API (`generate_api_struct` + `generate_request_method_with_suffix`),
  parsing it with `syn`, and counting inherent method names. A hard-coded
  "fixed methods" list would drift, and the fixed set depends on the
  response kinds present (`request` exists only with a JSON endpoint).
- **Unsupported request inputs are now reported, not silently dropped**
  (R10's backstop until Phase 6 implements them): text/binary bodies and
  `ApiKeyParam` are Rust errors (no live user); header and cookie parameters
  are errors when `required`, warnings when optional. The 7 live OpenAI
  `openai-beta` header parameters are optional, so they produce 7 Rust and
  7 Postman warnings and block nothing.
- **Newly found gap:** Postman collections drop `EndpointParams.header` and
  `.cookie` (`export/postman/request.rs` reads only `params.query`). Reported
  by `validate_postman` with the same requiredness rule; not fixed (not in this
  spec's scope).
- **Base-plus-path composition warning not implemented.** R11 says an exact
  duplicated segment "may" be a warning; the spec also calls `/api/api`
  legitimate. A warning that fires on a legitimate composition is noise, so
  no composition check exists; `/api/api/items` is in the accept table.
- **Docs page:** default 19 names one page for "partial generation and
  validation". Phase 2 created `schematic/docs/generation/validation.md`
  (validation only). Phase 4 can add the partial-generation page beside it or
  merge them.
- **`generate --api <one>`** keeps today's behavior for the Rust root
  (`generate_and_write_all` with one API, including its cleanup); OpenAPI and
  Postman are written for that API's module with no stale cleanup
  (`BatchScope::Selection`). Partial-generation safety is Phase 4.

### Deviation from R13 — duplicate method+path is a warning (review item)

> **Withdrawn** by the author's 2026-10-12 ruling on review-1 item 1; R13 is
> now enforced as errors. See "Review 1, ruling 1" at the end of this log.

The live definitions contain two same-API duplicate routes:

| API | Endpoints | Route | How they differ |
| --- | --- | --- | --- |
| GitHub | `GetGitTree`, `GetGitTreeRecursive` | `GET /repos/{owner}/{repo}/git/trees/{tree_sha}` | declared query `recursive` |
| Bitbucket | `ListDirectoryContents`, `GetFileContentRaw` | `GET /repositories/{workspace}/{repo_slug}/src/{commit}/{path}` | JSON vs text response |

R13 says these "fail validation". As errors they would block GitHub and
Bitbucket in all three outputs (the real-API compile guard and `just generate`
would fail), and fixing them needs definition edits or the selector of the
response-model/GraphQL follow-up, neither of which is authorized. The check is
therefore implemented with severity `DUPLICATE_METHOD_PATH_SEVERITY = Warning`
— one constant, so the follow-up that adds selectors raises it in one place.
**Finding:** the committed `openapi/github.json` and `openapi/bitbucket.json`
each hold only one operation on that route (`GetGitTreeRecursive`,
`GetFileContentRaw`); `GetGitTree` and `ListDirectoryContents` are silently
missing from the OpenAPI documents today. The OpenAPI diagnostic says so. R13
is **partially met**: the key, its single function, and the detection are
done; the blocking severity is not.

### Fixture corrections

The Rust boundary rejected three test fixtures whose JSON body type equals the
endpoint's own request struct (a recursive type that could never compile):
`output::tests::make_complex_api` and `test_utils` (`CreateCompletionRequest`
→ `CreateCompletionBody`), and `e2e_generation`'s `Patch` endpoint
(`PatchRequest` → `PatchBody`). `openapi_strict_completeness.rs` copied the
removed `missing_schemas_error` format by hand; it now renders the real
`validate_openapi_registry` diagnostic.

### Comment and doc drift found (code assumed correct)

- `.claude/skills/schematic-define/schematic/gen.md` listed `GeneratorError`
  with `PathBuf` fields and an `OpenApiError` variant that do not exist; now
  matches the code (and the new variants).
- `openapi_strict_completeness.rs` header said the binary exercises
  `validate_completeness` in `run_generate`/`run_generate_all`; rewritten for
  `validate_openapi_registry`.
- `schematic/gen/README.md` validation section described only two checks and
  "generation is aborted"; rewritten for per-output validation.
- Pre-existing rustdoc warnings (not touched): unresolved `ResponseKind`
  (`codegen/error.rs`), `codec`/`routing` (`ws_codegen/mod.rs`), private link
  in `parser.rs`.

### Requirement → test mapping (Phase 2)

| Requirement | Test(s) | Result |
| --- | --- | --- |
| R8 whole batch per output, zero files for a failing output | `output_validation::malformed_rust_type_blocks_only_the_rust_client` (pre-existing Rust files byte-identical, no `Cargo.toml`), `::incomplete_registry_blocks_only_openapi` (no OpenAPI root created although one module was valid) | pass |
| R9 diagnostic scope, severities, render, serialize, outside construction | `diagnostics::tests::*` (5), `output_validation::downstream_code_can_build_and_render_both_severities` (outside the crate) | pass |
| R10 independence both ways, all outputs attempted, named, nonzero exit; warnings succeed | `::malformed_rust_type_blocks_only_the_rust_client` (malformed Rust type → Rust fails; OpenAPI carries `Vec<Thing` as data; Postman written), `::incomplete_registry_blocks_only_openapi`, `::cli_reports_each_output_and_exits_nonzero_when_one_fails` (real binary, exit 1, Postman after the OpenAPI failure), `::cli_warnings_alone_exit_successfully` (real binary, `gitea`, exit 0, one warning per output) | pass |
| R11 type syntax, module path, filenames, identifier collisions, `/api/api` | `validation::tests::accepted_definitions_produce_no_diagnostics_for_any_output` (generic/qualified types, `/api/api`, sanitizing names), `::rust_only_rejections_name_feature_and_endpoint` (29 reject rows; each asserts the feature never reaches the exports), `::export_file_names_must_be_one_safe_component`, `::body_collision_message_suggests_a_rename` | pass |
| R12 duplicate ids and emitted names | `::duplicate_endpoint_id_in_one_api_fails_every_output`, `::group_members_may_share_raw_ids_when_suffixes_differ`, `::group_members_whose_emitted_names_collide_name_both_apis`, `::live_shared_groups_are_accepted` | pass |
| R13 `(method, normalized_path)` key | `::path_parameter_names_do_not_distinguish_routes` (`/items/{id}` = `/items/{name}`, GET ≠ DELETE, `{+repo}`; detection in every output) | pass; severity is warning (deviation above). **Superseded:** error since review-1 fix cycle 1; `same_api_duplicate_route_blocks_every_output` |
| R33 nine warnings, one constant | `output_validation::live_definitions_warn_once_per_output_for_each_embedded_query` (9 named endpoints per output, 27 lines, 9 unique, zero live errors), `validation::tests::query_in_path_is_one_warning_per_output`, `::interim_route_checks_are_warnings` | pass |
| Gap reporting (R10 backstop) | `::api_key_param_is_found_through_the_effective_policy` (explicit method found; policy override hides legacy auth), `::unsent_header_and_cookie_parameters_are_reported_by_requiredness` | pass |

Fidelity: the R8/R10 tests run the public `generate_batch` against real
temporary directories (filesystem boundary) and the two CLI tests spawn the
real `schematic-gen` binary through `bin_exe!` (process boundary); all are
ordinary L1 (each under 1 s). No test invokes Cargo, so none is `slow_`.

Input Robustness Matrix: not applicable. The validators read typed Rust
definitions; no file format or configuration reader was added.

### Broader gates

- `just test` (schematic): exit 0. L1 **1718 passed, 9 skipped**; then the
  fresh-output guard **4 passed** (55 s, warm). The guard's
  `slow_real_apis_compile_offline` now generates the 18 APIs through the
  validating `generate_and_write_all`, so the live batch passes the Rust
  validator.
- `just lint` (schematic): exit 0 (clippy only; no formatter).
- `just check-tier-coverage schematic`: 0 stranded.
- Doc examples of the two new modules: `cargo test --doc -p schematic-gen --
  diagnostics validation`, 2 passed (nextest does not run doctests).
- `cargo doc -p schematic-gen --no-deps`: no new warnings.

### Platform evidence

- macOS (this host): `just test` and `just lint` pass (above).
- Native Windows: `just cross-check schematic-gen --os windows` against the
  local tree: **569 passed, 12 skipped**, including all six
  `output_validation::*` tests (the file-occupying-a-directory write failure,
  exit code 1, and `bin_exe!` spawn behave the same as on macOS). L1 only:
  `cross-check` runs no `slow_` tests, so the fresh-output guard was not run
  there.
- Linux and WSL2: not run in Phase 2. The new code is pure validation plus
  `std::fs` writes already exercised on Windows; CI's pull-request Linux leg
  covers it.

### Files touched outside `schematic/`

- `.claude/skills/schematic-define/schematic/gen.md` (validation section,
  library example, error enum).

No dependency was added.

### Human review agenda items raised in Phase 2

1. R13 deviation: `duplicate-method-path` is a warning
   (`DUPLICATE_METHOD_PATH_SEVERITY`) because GitHub and Bitbucket each
   declare two endpoints on one route; the committed OpenAPI documents silently
   lack `GetGitTree` and `ListDirectoryContents`. Options: keep the warning
   until selectors exist; make it an error for OpenAPI only (blocks
   `github.json`/`bitbucket.json`); or migrate the two definitions.
2. Six feature ids added to default 20's list (above).
3. Unsupported request inputs are reported now: errors for text/binary bodies
   and `ApiKeyParam`, error/warning by `required` for header/cookie
   parameters (Rust and Postman). Phase 6 removes the Rust ones as it
   implements them.
4. Postman drops header and cookie parameters (a gap outside this spec).
5. No base-plus-path composition warning (the spec's "may").
6. Removed public items: `validate_api`, `run_validation`,
   `missing_schemas_error`, `run_generate_all`, and three `GeneratorError`
   variants.

## Phase 3

### Authorization state

- Pre-authorization item 1 (regenerate schema/OpenAPI/Postman) still has no
  recorded author confirmation. R15 is part of Phase 3 and the committed
  `openapi/ollama.json` must match the generator for `artifact_drift` (an
  ordinary L1 test) to pass, so — following Phase 1's reading of item 5 —
  item 1 is read as authorized by the request to "implement phase 3" **for
  this phase's generator-produced artifact refresh only**. Recorded for
  review 1.
- The regeneration ran the exact command of `just generate`
  (`cargo run -p schematic-gen -- generate --api all --output schema/src
  --openapi-out openapi --postman-out postman`) without the recipe's desktop
  notification. Nothing was hand-edited.
- Items 2–8 were not exercised.

### What was built (Tasks 3.1–3.2)

- **`export::openapi` split into build and write.**
  `build_openapi_grouped(apis, module, registry, options) -> OpenAPI`
  validates, exports each member, merges, assigns unique ids, and runs the
  final scan; `write_openapi_document(doc, stem, format, dir)` serializes and
  writes; `write_openapi_grouped` is now the two in sequence (signature
  unchanged). All three plus `operation_id_diagnostics` are re-exported from
  the crate root.
- **Merge ownership.** `merge_documents` records every operation as
  `(path, method) -> [contributing API indexes]` in first-owner order: API
  registry order, the owner's document path order, then the fixed method
  order GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE. A compatible
  operation (equal apart from `security`, the EMQX case) adds its API to the
  existing entry.
- **Incompatible same route (R13, OpenAPI only).** Previously a
  `ConfigError` string; now `GeneratorError::Validation` with one
  `duplicate-method-path` OpenAPI error naming both APIs (`for_group`). The
  Rust and Postman validators are unchanged, so cross-API overlap is not a
  Rust-wide error.
- **Id allocation (R14), `assign_operation_ids`.** Every original id is
  reserved first. Walking in first-owner order, the first claimant keeps its
  id; a later clash becomes `{PascalCase(owner API name)}{id}`, and if that is
  reserved or already assigned, `{…}2`, `{…}3`, …. Per-API documents are never
  renamed (default 7).
- **Links.** Each API gets its own original→final map. A response link inside
  an operation is rewritten through its *owner's* map, so the same raw id in a
  link from another API's document is not touched. `components.links` are not
  rewritten: they come from the seed API, which claims first and so is never
  renamed.
- **Final scan, `operation_id_diagnostics(doc, apis)`.** Public. Reports
  `duplicate-operation-id` (an id declared by two routes, both named) and the
  new `unresolved-link-operation` (a link in an operation response,
  `components.links`, or `components.responses` naming no declared id). Run by
  `build_openapi_grouped` and by the per-API `write_openapi`, and by the
  corpus test over committed files.
- **Pipeline (`publish_openapi`).** Builds every module's document before
  writing any; a merge or scan failure in module N leaves the OpenAPI root
  untouched (previously modules before N were already written — a partial
  publish of a rejected batch). `run_openapi_export_grouped` now takes the
  built document (no external callers; signature changed) and the version
  rule moved to `openapi_export_options`.

### Design choices not dictated by the spec

- **Security-scheme renames now happen before path merging**, on the incoming
  document. The old code rewrote, after merging, every operation on any path
  `path_method_owners` attributed to the incoming API — which also rewrote
  other APIs' operations sharing that path, and left the incoming API's
  security unioned into a compatible seed-owned operation pointing at the
  seed's same-named scheme. The incoming document's top-level `security` is
  now renamed too. No live module has a scheme rename, so no artifact
  changed. Done here because it was the ownership code this task rewrote.
- **New feature id `unresolved-link-operation`** (the seventh beyond default
  20's list) for the link-target half of the final scan, rather than
  overloading `duplicate-operation-id`.
- **Numeric suffixes start at 2** (`FooBList2`), and only on a clash.
- **Incoming `components.links` are still dropped by the merge** (as are
  other incoming component maps besides schemas and security schemes) — a
  pre-existing gap. The exporter never emits links, so no artifact is
  affected; recorded, not fixed.
- **The exporter emits no OpenAPI links and only document-level
  `security`.** Link handling is therefore tested on hand-augmented exported
  documents through the private `merge_documents`; no shipped artifact
  exercises it.

### Artifact change (R15)

`git diff --stat` after regeneration: `schematic/openapi/ollama.json | 4 ++--`
and nothing else (Rust sources and Postman collections byte-identical):

| Route | Before | After |
| --- | --- | --- |
| `POST /v1/embeddings` (OllamaOpenAI) | `Embeddings` | `OllamaOpenAIEmbeddings` |
| `GET /v1/models` (OllamaOpenAI) | `ListModels` | `OllamaOpenAIListModels` |

`OllamaNative` is first in registry order, so `POST /api/embeddings` and
`GET /api/tags` keep `Embeddings` and `ListModels`. Every other live module
keeps every endpoint id unchanged
(`unique_ids_are_unchanged_for_every_live_module`).

### Comment and doc drift found (code assumed correct)

- `docs/io/export-openapi.md` "Grouped Export" claimed servers are a
  "deduplicated union of base URLs", info has "concatenated descriptions",
  and operations carry "per-operation security". The code takes servers and
  info from the first member and the exporter emits document-level security
  only. Rewritten, with the operationId rule.
- `write_openapi_grouped` rustdoc said a path/method collision is a
  `ConfigError`; it is now a `Validation` error, and the merge semantics moved
  to `build_openapi_grouped`.
- The merge's security-rename comment ("the simplest correct approach is to
  rewrite all operations on paths …") described the approximation removed
  above; deleted with it.
- `artifact_drift.rs` module header gave the stale `cargo test` invocation
  (noted in Phase 1); now the nextest selection.

### Requirement → test mapping (Phase 3)

| Requirement | Test(s) | Result |
| --- | --- | --- |
| R13 compatible auth alternatives merge | `export::openapi::tests::compatible_operations_merge_security_without_renaming` (one op, id kept, `basicAuth` + `bearerAuth`) | pass |
| R13 incompatible overlap fails OpenAPI only, no partial publish | `export::openapi::tests::write_openapi_grouped_collides_on_duplicate_method_path` (exact rendered diagnostic, no file), `output_validation::incompatible_group_route_blocks_only_openapi_and_writes_no_document` (public `generate_batch`: valid `inventory` module first, OpenAPI root absent, Rust + Postman written) | pass; by inspection the second would fail on the pre-change code, which wrote `inventory.json` before returning `ConfigError` (not executed against the old code) |
| R14 first keeps, later prefixed | `::later_clashing_ids_take_the_owning_api_prefix` (Ollama shape), `output_validation::grouped_document_renames_a_shared_raw_id` (written file via `generate_batch`) | pass |
| R14 reserve originals / adversarial | `::renames_never_take_an_original_id_declared_later` (`FooC` declares `FooBList` → B gets `FooBList2`; `FooB2List` gets no suffix) | pass |
| R14 determinism, registry order | `::allocation_is_deterministic_and_follows_registry_order` | pass |
| R14 owner-scoped links | `::links_follow_the_rename_of_their_own_document_only` (A's link to `List` stays, B's two links become `FooBList`) | pass |
| R14 final scan, grouped + ungrouped | `::scan_reports_duplicate_ids_and_unresolved_links`, `::scan_checks_component_links_too`; wired into `write_openapi` and `build_openapi_grouped` | pass |
| R14 unique per-API ids unchanged | `::unique_ids_are_unchanged_for_every_live_module` (only the two Ollama renames across every live module with a registry — 15 documents) | pass |
| R15 committed corpus | `artifact_drift::committed_openapi_documents_have_unique_operation_ids` — **failed before regeneration** with exactly the two `ollama` duplicates (`Embeddings`, `ListModels`), passes after; `artifact_drift::generated_openapi_artifacts_are_up_to_date` | pass |

Fidelity/tier: the corpus test reads the real committed files
(`schematic_root().join("openapi")` inside the `#[test]`); the two
`output_validation` tests run the public `generate_batch` against temp
directories (filesystem boundary). All are ordinary L1, each well under 1 s;
none invokes Cargo, so none is `slow_`. All live in already-declared targets
(the lib's unit tests and the `l1` binary's existing modules).

Input Robustness Matrix: not applicable. The corpus test deserializes
committed OpenAPI output with the existing `openapiv3` types to inspect it; no
configuration or manifest reader was added.

### Broader gates

- `just test` (schematic): exit 0. L1 **1730 passed, 9 skipped** (Phase 2:
  1718 + 12 new); fresh-output guard **4 passed** (28.6 s, warm), so the full
  18-API compile and the Rust-source comparison stay green.
- `just lint` (schematic): exit 0 (clippy only).
- `artifact_drift::` nextest selection: 6 passed.
- `cargo doc -p schematic-gen --no-deps`: no warnings in touched files (one
  private intra-doc link introduced mid-phase was removed).

### Platform evidence

- macOS (this host): gates above.
- Linux / native Windows / WSL2: not run. The change is in-memory document
  manipulation plus the existing `fs::write`; the new tests use temp dirs and
  compare JSON values, with no path spelling, line ending, or process
  behavior. Phase 2's Windows run already covered the same write path. CI's
  Linux and macOS pull-request legs cover it.

### Files touched outside `schematic/`

- `.claude/skills/schematic-define/schematic/gen.md` (Validation paragraph).

No dependency was added.

### Human review agenda items raised in Phase 3

1. Item 1 (regeneration) read as authorized by the phase request, for the
   `ollama.json` refresh only.
2. Feature id `unresolved-link-operation` added.
3. Security-scheme renames moved before path merging (behavior fix, no
   artifact change).
4. Incoming `components.links` (and other component maps) are still dropped
   by the grouped merge.
5. `run_openapi_export_grouped`'s public signature changed (takes a built
   document).

## Phase 4

### Authorization state

- No pre-authorization item is exercised by Phase 4: no regeneration of
  committed artifacts (the full-generation check below ran against a copy
  under `/tmp`, then the copy was deleted), no deletion in the checkout, no
  `RequestParts` change, no dependency added, no `schematic-define` change, no
  CI change. Items 1–8 remain unconfirmed.
- Temporary output trees were created only under the system temp directory
  (item 5, already read as authorized in Phase 1).

### What was built (Tasks 4.1–4.4)

- **`output/publish.rs` (new, public).** `Mutation { Write, RemoveFile,
  RemoveDir }`, `MutationPlan` (writes in the preparer's order, then
  deletions), `MutationPlan::publish` → `PublicationReport`, and
  `print_dry_run`. A failed mutation returns the new
  `GeneratorError::PublicationFailed { root, failed, completed, pending,
  source }`, rendered with both lists and the remedy (rerun; if a partial run
  then refuses, run full generation). Writes go through the existing
  `write_atomic`.
- **`output/inventory.rs` (new, crate-private).** Ownership proofs
  (`occupant(path, Proof)` with `Proof::{RustSource, Manifest, Artifact}`),
  using `symlink_metadata` so a link is never followed or proven, and an
  unreadable entry or listing error is never proven owned. The ownership
  helpers previously private in `output/mod.rs` (`is_generator_owned_*`,
  `remove_conflicting_paths`, `cleanup_stale_files`, `collect_expected_files`)
  were replaced by these. `read_rust_inventory` establishes retained groups
  (default 3) and `shared_runtime_compatibility` is the single compatibility
  function (default 4, R34).
- **`output/mod.rs`.** New public `Selection { All, Apis(&[&RestApi]) }`,
  `RustOutputRequest { registry, selection, output_dir, manifest_dir }`,
  `RustOutputPlan { plan, regenerated, retained }`, `prepare_rust_output`,
  and `expand_selection`. `generate_and_write_all` is now prepare(`All`) +
  publish; `generate_and_write` unchanged in meaning. `generate_and_write_standalone`
  (the OpenAPI import writer) uses the same planner. A crate-private
  `prepare_export_output` plans an OpenAPI or Postman root.
- **Pipeline.** `BatchScope::Selection` now carries API names
  (`Selection(Vec<String>)`), and `generate_batch(&registry, &scope, …)`
  treats `apis` as the registry. Each root expands the selection to complete
  groups with its own module resolver (Rust: `get_module_path`; exports:
  `resolve_module_name`), validates that batch, prepares, and publishes. The
  generated `Cargo.toml` is now a planned write inside the Rust plan (it was a
  separate `write_cargo_toml` call after the crate), so a refused or failed
  Rust output writes no manifest. Success lines (`[OK] Generated …`,
  `[OK] Published …`) print only after a successful publication.
- **Renderers split from writers.** `export::openapi::render_openapi_document`
  (now used by `write_openapi_document`) and
  `export::postman::render_postman_module` (single API → that API's
  collection, shared module → grouped collection, as the pipeline chose
  before). Exports are now written atomically (temp + rename) instead of
  `fs::write`.
- **`cargo_gen::MANIFEST_OWNERSHIP_MARKER`**, the template's existing
  `# This file was automatically generated by schematic-gen.` line, now the
  manifest's ownership proof (unit test keeps template and constant in step).
- **New errors.** `UnsafeOutput { root, problems, remedy }` (every problem
  listed; remedy is either "run full generation or an isolated output
  directory" for inventory problems, or "move or remove the listed paths" for
  conflicts) and `RequiresFullGeneration { root, changed, retained }`. All say
  "nothing was changed".

### How a Rust run is prepared (the rules as implemented)

1. Expand the selection to complete module groups through the registry (an
   unknown API name is a `ConfigError`); validate the regenerated APIs.
2. Partial run only: read the inventory unless the root is absent or empty.
   Required: owned, parseable `lib.rs` and `prelude.rs`; every declared REST
   module that is retained present as exactly one owned layout (`m.rs`, or a
   real directory whose `mod.rs` is owned); every owned top-level module
   declared (unless it is in a regenerated group, so a retry after a failed
   run still completes); every declared module known to the current
   definitions or a WebSocket module; prelude exports of every retained group
   equal to `{Api, ApiRequest}` of its current members; no prelude export from
   an undeclared module.
3. `lib.rs` and `prelude.rs` are assembled from the retained + regenerated
   groups **in registry order**, so a partial run over a full tree writes them
   byte-identical to full generation.
4. Compatibility: with any retained group, `shared.rs` and `ws_shared.rs` must
   equal the new content byte-for-byte (a missing file counts as different).
5. Plan writes: regenerated group modules (module-path order), `shared.rs`,
   `ws_shared.rs`, the five WebSocket modules, `Cargo.toml` (when requested),
   `prelude.rs`, `lib.rs`. Every target must be absent or an owned file.
   Deletions: the other layout of each regenerated module (owned only; an
   unowned one is a conflict, left intact), owned stale files inside a
   regenerated directory module, and — full generation only — owned top-level
   entries no module produces. Deletions are deduplicated.
6. Export roots: each planned `<module>.json|yaml` /
   `<module>.postman_collection.json` must be absent or a regular file; a
   complete run deletes regular files with the artifact suffix that no module
   produced; a partial run deletes nothing.

### Design choices not dictated by the spec

- **Default 3 adopted** with one refinement: a directory module's evidence is
  an owned `mod.rs` in a real directory; whole-tree ownership is required
  only to *delete* a directory. A retained split module containing a
  hand-written file is therefore still a valid member (found when the R3 test
  planted `large/hand.rs`).
- **Default 4 adopted.** Runtime-bearing files are `shared.rs` and
  `ws_shared.rs`. The five WebSocket definition modules are regenerated on
  every run (as before), so they never need to stay compatible with an older
  `ws_shared.rs`.
- **Default 5, deviation in deletion order.** Writes run in the stated order
  with `lib.rs` the last *write*; deletions of obsolete paths follow `lib.rs`.
  "Prepare replacements before obsolete-path cleanup" and "`lib.rs` last"
  cannot both hold when a layout changes; this order guarantees a replacement
  exists before its old path goes, and a failure during deletions leaves
  `lib.rs` already describing the new module set with the leftovers listed as
  not done.
- **Default 22 adopted.** Partial OpenAPI/Postman runs write only the
  selected export modules under the current export resolver. For the 18 live
  APIs the two resolvers give the same 15 module names (compared from the
  committed `schema/src` and `openapi/` listings); no `PagerDutyApi`-style
  discrepancy exists in the registry today.
- **Export ownership** is name-based (JSON/YAML carry no header): a regular
  file with the artifact suffix. Full-run export cleanup previously deleted
  *every* unexpected entry (and failed on a subdirectory); it now deletes only
  artifact-named regular files, so `README.md` or a subdirectory in
  `openapi/` survives. Planned artifacts on a directory or symlink are
  conflicts.
- **Export roots need no inventory.** Nothing in them is rebuilt from
  retained content, so a partial export run only checks its own targets. R7's
  "nonempty directory missing required inventory evidence" applies to the
  Rust root, which has shared exports.
- **Full generation now refuses an unowned occupant** of any planned path
  (e.g. a hand-written `lib.rs` or a `Cargo.toml` without the marker); it used
  to overwrite them. Same for the standalone import writer, whose test
  `standalone_import_preserves_hand_written_conflicting_directory` now expects
  `UnsafeOutput` (before, it wrote `foo.rs` beside a hand-written `foo/`,
  which Rust rejects as an ambiguous module).
- **Failure-injection seam.** No test-only hook: the public two-step API
  (`prepare_rust_output`, then `MutationPlan::publish`) lets a test change the
  filesystem between preflight and publication. The tests put an empty
  directory where a planned file goes; the rename onto it fails on macOS,
  Linux, and Windows (verified on native Windows below), with no permission
  bits.
- **Dry run.** Rust prints each planned file's content (`=== path ===`) in plan
  order, plus `would delete …` lines; exports print `[OK] Would write …` per
  file (the L2 dry-run test still finds the module file names). Nothing,
  including the output roots and `Cargo.toml`, is created.
- **R1/R2 real-API evidence lives in the existing slow guard** rather than a
  new `slow_` test. Generating all 18 APIs takes about 1.9 s in a debug build,
  too close to the 5 s L1 budget under CI load for an ordinary test, and the
  guard already generates and compiles them. `slow_real_apis_compile_offline`
  now edits `ollama/requests.rs` and `anthropic.rs`, runs a partial
  generation of `OllamaNative`, asserts `regenerated == ["ollama"]`, 14
  retained groups, and `anthropic.rs` untouched, restores `anthropic.rs`,
  then compares the whole tree with the committed `schema/src` and compiles
  it. No new test name, so the R35 builder selection and archive exclusions
  are unchanged.
- **Removed public items** (no callers outside the crate):
  `pipeline::cleanup_stale_artifacts`, `pipeline::run_postman_export`,
  `pipeline::run_openapi_export_grouped`. Signature change:
  `pipeline::generate_batch` takes `&BatchScope`, and `BatchScope::Selection`
  carries names.

### Comment and doc drift found (code assumed correct)

- `output/mod.rs` module docs claimed atomic writes "prevent partial writes"
  and described cleanup only; rewritten for prepare/publish, ownership, and
  partial generation.
- `schematic/gen/README.md` pipeline diagram said "Clean up stale .rs files",
  and the output section said stale files "are automatically cleaned up";
  both now state the ownership rule and full-only deletion. Its CLI example
  "Generate code for the OpenAI API" now says it regenerates that group and
  keeps the rest.
- `schematic/README.md` and the `generate-one` recipe comment said "generate a
  single API client"; they now say the module group is regenerated and the
  rest kept. `.claude/skills/schematic-define/schematic/SKILL.md` said the
  generator "cleans up stale files"; corrected.
- `docs/generation/validation.md` flow said "Write Rust crate and
  Cargo.toml"; now "prepare and publish", and links the new page.
- **Pre-existing test drift (Phase 2):** the L2 test
  `level2_validate_emits_ansi_styled_ok_on_success` still asserted the
  pre-Phase-2 `validate` output (`[OK]`, "All validation checks passed").
  Phase 4 did not change `run_validate`, and neither asserted string exists
  in the committed `HEAD` pipeline, so the failure predates this phase
  (established by reading `git show HEAD:schematic/gen/src/pipeline.rs`, not
  by running the test on the old tree); Phase 2's gates did not include
  `test-l2`. Updated to
  assert the styled `[PASS]` line for each of `rust`, `openapi`, `postman`.

### Requirement → test mapping (Phase 4)

All in `schematic-gen`'s `l1` binary (`gen/tests/l1/partial_generation.rs`,
registered in `main.rs`) unless noted. They run the public
`prepare_rust_output`/`MutationPlan::publish`, `pipeline::generate_batch`, or
the real `schematic-gen` binary against temporary directories (filesystem
and process boundaries) and compare relative file lists and bytes with
`snapshot_tree`.

| Requirement | Test(s) | Result |
| --- | --- | --- |
| R1 whole group, rest byte-identical | `partial_run_regenerates_the_whole_group_and_keeps_every_other_module` (selecting `AlphaTwo` rewrites the edited `alpha.rs`; edited `beta.rs` kept; tree otherwise equals full generation; second run idempotent); `fresh_output_compile::slow_real_apis_compile_offline` (18 real APIs, `OllamaNative`) | pass |
| R2 shared files from retained inventory; compiles | same two tests (prelude exports all four fixture APIs; real tree equals committed `schema/src` and passes `cargo check --offline --locked`) | pass |
| R3 owned stale deleted; unowned kept; no overwrite; no symlinks | `owned_stale_files_in_a_regenerated_group_go_and_hand_written_content_stays` (split module `large/stale.rs` deleted only when `large` is regenerated; `large/hand.rs`, `large/notes.md`, `bin/main.rs`, `handwritten.rs` kept), `refuses_unowned_shared_file`, `unowned_manifest_is_never_overwritten`, `refuses_directory_at_a_planned_file`, `refuses_layout_conversion_with_unowned_content`, `symbolic_links_are_never_followed` (unix), `output::inventory::tests::{ownership_proofs, symbolic_links_are_never_followed}`, existing `output::tests::cleanup_preserves_hand_written_files_and_foreign_dirs` | pass |
| R4 unsafe inventory fails before any write, actionable | `refuses_corrupt_lib_rs`, `refuses_hand_edited_lib_rs`, `refuses_missing_lib_rs`, `refuses_missing_member`, `refuses_undeclared_owned_module`, `refuses_unknown_declared_module`, `refuses_export_mismatch`, `refuses_conflicting_layouts` (each asserts the cause, "nothing was changed", and byte-identical tree; one `#[test]` per case via `macro_rules!`); `cli_refuses_an_unsafe_partial_run_and_claims_no_update` (real binary: exit 1, remedy names `generate --api all`) | pass |
| R5 / R34 shared-runtime change | `refuses_changed_shared_runtime`, `refuses_changed_websocket_runtime`, `full_generation_recovers_after_a_compatibility_refusal`, `output::inventory::tests::compatibility_passes_identical_runtime_and_refuses_a_change` | pass |
| R6 honest failure reporting, retry, dry run | `failure_during_publication_lists_completed_and_pending_mutations` (failure after preflight: `alpha.rs` written, `lib.rs` still old and listed pending; rerun completes; a `prelude.rs` failure makes the retry refuse with no change and full generation repairs), `output::publish::tests::writes_precede_deletions_and_failure_lists_completed_and_pending`, `dry_run_plans_every_root_and_creates_nothing` (no root, no `Cargo.toml` created; plan starts with the module and ends with `lib.rs`), `unowned_manifest_is_never_overwritten` (manifest inside the boundary), `cli_refuses_…` (no `[OK] Generated` line) | pass |
| R7 OpenAPI/Postman group rule, empty roots, independent roots | `exports_rewrite_only_the_selected_groups`, `each_root_establishes_its_own_safety` (OpenAPI conflict fails only OpenAPI; Rust and Postman write only the `alpha` group), `absent_or_empty_root_gets_only_the_selected_groups` | pass |

Regression evidence: before this phase `generate --api <one>` ran
`generate_and_write_all` with one API and its full cleanup, so the real-API
partial run above would have deleted the other 14 groups and failed the
committed-tree comparison. This was established by reading the replaced code;
the new tests were not executed against the old tree.

Not covered at the CLI level: a mid-publication I/O failure through the
binary (no portable way to inject one between preflight and publication from
outside the process). The library tests above cover the report; the CLI
prints a success line only after `publish` returns `Ok`.

Input Robustness Matrix: not applicable. `read_rust_inventory` parses
generator-written `lib.rs`/`prelude.rs` with `syn` as a consistency check;
any unparseable or unexpected content refuses the run rather than being
defaulted. No configuration, manifest, or lockfile reader was added (the
manifest is checked only for its marker line).

### Broader gates

- `just test` (schematic): exit 0. L1 **1760 passed, 9 skipped** (Phase 3:
  1730); fresh-output guard **4 passed** in 16.5 s (warm cache). Whole recipe
  30 s wall. Slowest new test 1.05 s.
- `just lint` (schematic): exit 0 (clippy only; the shared `_lint` recipe runs
  no formatter). Also `cargo clippy -p schematic-gen --all-targets --features
  terminal-tests -- -D warnings`: clean.
- `just test-l2` (schematic, tmux backend, no window focus): **3 passed**
  after the stale validate assertion above was updated.
- `just check-tier-coverage schematic`: 0 stranded.
- `cargo doc -p schematic-gen --no-deps`: no warnings in touched files (the
  four pre-existing ones from Phase 2 remain).
- Real CLI on a copy of the committed tree: `generate --api all` and then
  `generate --api emqx-basic` (all three outputs) both left `schema/`,
  `openapi/`, and `postman/` byte-identical to the checkout; the partial run
  reported `Generated emqx … (14 written, 0 deleted; kept …14 groups)`, one
  OpenAPI and one Postman file written.

### Platform evidence

- macOS (this host): gates above.
- Native Windows: `just cross-check schematic-gen --os windows` on the local
  tree: **609 passed, 12 skipped**, including the 23 non-unix
  `partial_generation` tests (the directory-at-a-planned-file failure
  injection, the CLI refusal, and the publication-failure retry) and the
  `output::publish`/`output::inventory` unit tests. The symlink tests are
  `#[cfg(unix)]` (creating symlinks on Windows needs Developer Mode or
  elevation); Windows has no symlink evidence. `cross-check` runs no `slow_`
  tests, so the partial run inside the real-API guard ran on macOS only.
- Linux / WSL2: not run in Phase 4. The code uses only `std::fs` calls
  already exercised on macOS and Windows; CI's pull-request Linux leg covers
  it.

### Files touched outside `schematic/`

- `.claude/skills/schematic-define/schematic/gen.md` (library example, error
  enum) and `.claude/skills/schematic-define/schematic/SKILL.md` (one
  sentence on shared-module generation).

No dependency was added (`indexmap` and `syn` were already dependencies).

### Human review agenda items raised in Phase 4

1. Default 3 refinement: a directory module is evidenced by its owned
   `mod.rs`; extra hand-written files inside a retained module are allowed.
   Retained modules whose endpoints changed (but not their exported names)
   are kept stale — the inventory checks names, not content.
2. Default 5 deviation: deletions follow the `lib.rs` write.
3. Full generation and the import writer now refuse unowned occupants of
   planned paths instead of overwriting them (a behavior change beyond partial
   generation).
4. Export ownership is name-based; full export cleanup no longer deletes
   non-artifact files.
5. Public API changes: `BatchScope::Selection(Vec<String>)`,
   `generate_batch(&registry, &BatchScope, …)`; new `output::{Selection,
   RustOutputRequest, RustOutputPlan, prepare_rust_output, expand_selection,
   Mutation, MutationPlan, PublicationReport}`, `render_openapi_document`,
   `render_postman_module`, `MANIFEST_OWNERSHIP_MARKER`, three
   `GeneratorError` variants; removed `cleanup_stale_artifacts`,
   `run_postman_export`, `run_openapi_export_grouped`.
6. Inventory and compatibility adequacy once the runtime/envelope follow-up
   adds a runtime package check: the byte comparison lives only in
   `inventory::shared_runtime_compatibility`, the intended replacement point.

## Phase 5

### Authorization state

- Pre-authorization items 1–8 still have no recorded author confirmation.
  Phase 5 cannot exist without item 6 (change the public shape of the
  generated `RequestParts`/`into_parts`, R28) and its Task 5.2 requires item 1
  (`just generate`). Following Phase 1's and Phase 3's narrow reading, both
  are read as authorized by the request to "implement phase 5" **for this
  phase's query split and its generator-produced refresh of `schema/src`
  only**. Recorded for review 1.
- The regeneration ran the exact command of `just generate`
  (`cargo run -p schematic-gen -- generate --api all --output schema/src
  --openapi-out openapi --postman-out postman`) without the recipe's desktop
  notification. Nothing generated was hand-edited.
- Items 2, 3, 4, 7, 8 were not exercised. Temporary crates under the system
  temp directory and the child target (item 5) are the Phase 1 guard's.

### Consumer search repeated before changing the tuple (R31)

- `rg -l "into_parts|RequestParts" --type rust` outside `schematic/gen` and
  `schematic/schema`: only unrelated same-named items (`biscuit-file`,
  `darkmatter`, `sniff`'s `aggregate_view::into_parts`, `sniff` CLI) and the
  macro-review spike. **No consumer of the generated `RequestParts` or
  `into_parts`.** `ResponseContext` (whose `path` changes meaning, below) has
  no user outside `schematic/` either (`rg ResponseContext|variant_hooks`).

### What was built (Tasks 5.1–5.2)

- **`RequestParts` (default 12 adopted):** still a tuple, with a fifth
  element `Vec<(String, String)>`:
  `(method, path, body, headers, query)`. `query` holds declared query
  parameters, unencoded, in declaration order, one pair per array element.
  Headers stay in element `3`.
- **`into_parts`** (`codegen/request_structs/shared.rs`
  `generate_path_format`): path substitution is unchanged (percent-encoded
  unless `{+name}`); a `?…` written in the path template stays in the path
  verbatim, with its existing escaping. Declared parameters are pushed as
  `(name.to_string(), value.to_string())`; no encoding. `path` is no longer
  `mut`.
- **`shared::append_query_pairs(url, path, pairs)`** (new, emitted by
  `codegen::error::generate_request_parts_type`): the only place query pairs
  are encoded, with the existing `urlencoding::encode` on every name and
  value (spaces stay `%20`; reqwest's form-query encoding is not used). It is
  `pub` so the later redaction layer and hand-written callers can build the
  same URL; a `pub(crate)` helper would also warn as dead code in any crate
  without a REST client.
- **Client URL assembly** (`codegen/client/methods.rs`):
  `let mut url = format!("{}{}", self.base_url, path);
  crate::shared::append_query_pairs(&mut url, &path, &query);`.
- **Separator rule — chosen from `path`, not from `url`.** The golden line
  `base-url-with-query` (`GET /api/v1?tenant=a%20b/repos/o/r/pulls?page=1`)
  shows the pre-split client appended `?` after a base URL that already held
  a query, because the old check looked at the path only. Choosing from the
  full URL would have produced `…/pulls&page=1`, a wire change. R29 requires
  byte equivalence, so the old rule is kept and documented; the pre-split
  oddity itself stays recorded (Phase 1) for the runtime/envelope follow-up.
  A mutation run (separator switched to `url.contains('?')`) failed the
  guard with exactly that difference, then was reverted.
- **`ResponseContext.path`** (hooks) is now the path without declared query
  parameters; `ResponseContext.url` still carries the full URL. Its doc
  comment was updated. This is a visible behavior change for response hooks;
  no hook in the repository reads `path`.

### Requirement → test mapping (Phase 5)

| Requirement | Test(s) | Boundary / rung | Result |
| --- | --- | --- | --- |
| R28 path holds no declared query; pairs separate, unencoded, arrays repeated, legacy suffix kept, headers separate | `fresh_output_compile::slow_query_wire_matches_pre_split_baseline` → child test `into_parts_returns_unencoded_query_pairs_apart_from_the_path` (real generated Gitea `ListPullRequests` with space/`+`/`%`/Unicode/reserved values and an empty value, no-query case, Gitea `GetGitTreeRecursive` and GitLab `GetRepositoryFile` legacy suffixes, OpenAI `ListConversationItems` repeated `include`, OpenAI `BetaListInputItems` legacy suffix + declared pair; asserts no query name among headers) | crate boundary: freshly generated `schematic-schema` compiled offline and run through nextest (F3) | pass |
| R28 codegen shape | `query_params_codegen::{single_query_param_is_collected_apart_from_the_path, multiple_query_params_keep_declaration_order, legacy_path_template_query_stays_in_the_path, endpoint_without_query_params_returns_empty_pairs, path_is_immutable_when_query_params_present, query_values_are_left_unencoded_by_into_parts, array_query_param_generates_loop_for_values, special_char_query_params_are_sanitized_with_serde_rename}`; `path_substitution::path_params_are_substituted_without_declared_query_params`; `http_client::client_appends_query_pairs_once_at_url_assembly` (destructure, base+path, append, then request builder, in that order; only headers reach `merge_headers`); unit `codegen::request_structs::tests::{generates_query_param_in_into_parts, generates_array_query_param}`, `codegen::client::tests::generate_request_method_no_auth` | generated source (F1) | pass |
| R29 wire equivalence | `slow_query_wire_matches_pre_split_baseline`: the 8 raw request lines captured in Phase 1 **before any generator change** (raw `TcpListener`, no HTTP library normalization) are byte-identical after the split; golden file unchanged | process + socket boundary (F3) | pass |
| R29 encoded exactly once, existing queries preserved | child test `append_query_pairs_encodes_once_and_keeps_existing_queries` (base-URL query, legacy path query continues with `&`, empty pair list leaves the URL alone, empty value `a=`, `$top=a%20b+c` → `%24top=a%2520b%2Bc`: encoded once, never decoded); `query_params_codegen::append_query_pairs_encodes_each_name_and_value_once` | F3 / F1 | pass |
| R31 (partial) dependents unaffected | `cargo check --all-targets` of `sniff`, `biscuit-speaks`, `model-citizen`, `homelab`, `eversolo-integration`, `unfolded-integration-helper`, `arcam-amp-integration`, `sony-receiver-integration` against the regenerated crate | compile only; their test runs are Phase 7 | pass |

Regression evidence: the existing codegen tests asserted the old in-path
assembly (`path.push_str(&format!("?{}", query_string))`) and failed (10
tests) after the change until rewritten; the rewritten ones assert the
absence of in-path assembly and encoding. The golden test is the
before/after wire evidence and was shown to fail on a separator regression.

Coverage of the remaining declared-query endpoints (117 in the live
definitions): every one is emitted by the same two code shapes — the
per-parameter `query.push` (scalar or array loop) and the single shared
`append_query_pairs` — and the regenerated diff shows exactly 117 removed
in-path `} else {` separator branches, one per such endpoint. The eight
golden lines exercise each branch of that shape (scalar, array, empty
value, no query, legacy suffix with and without placeholder, base-URL
query). Legacy warning-only paths (R33's nine) keep their previous
escaping and order: two are in the golden set and in the `into_parts`
assertions.

Input Robustness Matrix: not applicable (no reader of a file format or
configuration was added or changed).

Test placement: every new or renamed test is in the `l1` binary
(`autotests = false`, modules already declared). The child tests live inside
the existing `slow_` guard's generated crate, so no new `slow_` name exists
and the R35 builder selection and archive exclusions are unchanged. The outer
test asserts the child run reported both new tests, so they cannot silently
drop out. `just check-tier-coverage schematic`: 0 stranded.

### Regeneration diff (R31, Task 5.2)

- `just generate` changed 27 files under `schema/src` (3907+/3790−); every
  OpenAPI and Postman file is byte-identical (the operation-id repair already
  landed in Phase 3).
- Filtering the diff for lines not about `query`/`path`/`url`/
  `into_parts`/`RequestParts` leaves only the new doc text, the
  `append_query_pairs` body, and the 117 removed `} else {` branches. **No
  change is unexplained by R28.**
- `schematic/schema/Cargo.lock` also changed (14+/48−): Cargo re-resolved the
  excluded crate's lockfile because `biscuit-file`'s manifest now uses
  `globset` instead of `dirs`. `just test-schema`/`just lint-schema` run Cargo
  without `--locked` and rewrote it. Not a generator change; kept so the lock
  matches the current path dependency. For review 1.
- `query-in-path` still reports exactly the nine endpoints per output (R33).

### Design choices not dictated by the spec

- Helper name and placement: `append_query_pairs` in the generated
  `shared.rs`, public, taking the path to choose the separator.
- `ResponseContext.path` follows the path element (no declared query) rather
  than re-concatenating the old path-plus-query string; `url` keeps the full
  request URL. A hook that wanted the query reads `url`.
- Test renames in `query_params_codegen.rs` where the old name described the
  removed behavior: `single_query_param_uses_question_mark` →
  `single_query_param_is_collected_apart_from_the_path`,
  `multiple_query_params_uses_correct_separators` →
  `multiple_query_params_keep_declaration_order`,
  `query_params_with_existing_path_query_uses_ampersand` →
  `legacy_path_template_query_stays_in_the_path`,
  `path_uses_mut_when_query_params_present` →
  `path_is_immutable_when_query_params_present`,
  `generated_code_uses_urlencoding_for_query_values` →
  `query_values_are_left_unencoded_by_into_parts` plus
  `append_query_pairs_encodes_each_name_and_value_once`.
- New docs page `schematic/docs/generation/request-side.md` (default 19's
  request-side page): query separation now, with text/binary bodies, header
  and cookie parameters, and `ApiKeyParam` marked **planned** for Phase 6.

### Comment and doc drift found (code assumed correct)

- `codegen/error.rs` `generate_request_parts_type` docs listed "Optional JSON
  body" (the body has been `RequestBody` for a while); replaced.
- `codegen/request_structs/mod.rs` doc example showed
  `Ok(("GET", path, None, vec![]))` with an unencoded path; corrected to the
  real five-element shape.
- `gen/README.md` sample generated code showed a three-element
  `(&'static str, String, Option<String>)` `into_parts` in five places
  (predates this phase); corrected to `Result<RequestParts, SchematicError>`
  with the current bodies. Added a "Query Parameters" section.
- `.claude/skills/schematic-define/schematic/gen.md` showed
  `RequestParts = (&'static str, String, Option<String>, Vec<…>)`; corrected.
- `ResponseContext.path` doc ("The URL path with parameters substituted")
  now states that declared query parameters are only in `url`.

### Broader gates

- `just test` (schematic): exit 0. L1 **1764 passed, 9 skipped** (Phase 4:
  1760; +4 tests); fresh-output guard **4 passed** in 38.4 s (child rebuild
  of the changed schema crate; the wire test alone takes 11 s warm).
- `just lint` (schematic): exit 0 (clippy only; the shared `_lint` recipe
  runs no formatter). Also `cargo clippy -p schematic-gen --all-targets
  --features terminal-tests -- -D warnings`: clean.
- `just test-schema`: 20 passed. `just lint-schema`: clean.
- `just test-l2` (tmux, no focus): 3 passed.
- `artifact_drift::` ran inside `just test`: pass.

### Platform evidence

- macOS (this host): everything above.
- Linux, native Windows, WSL2: not run in Phase 5. The change is generated
  string handling with no filesystem, path, process, or OS-specific API;
  `cross-check` runs no `slow_` tests, so it could not exercise the child
  wire test anyway. CI's pull-request Linux leg runs the guard on the build
  job (R35). Phase 7 owns per-OS evidence.

### Files touched outside `schematic/`

- `.claude/skills/schematic-define/schematic/gen.md` and
  `.claude/skills/schematic-define/SKILL.md` (tuple shape).

No dependency was added (`urlencoding` was already in the generated
manifest).

### Downstream follow-up notes

- Runtime/envelope follow-up: URL redaction can mask exactly the secret
  pairs in `RequestParts.4` before `append_query_pairs`; `ApiKeyParam` query
  keys (Phase 6) join that same collection. The base-URL-query oddity
  (path appended after the base URL's query) is preserved for byte
  equivalence and is that follow-up's to fix deliberately.
- Definition-API follow-up: migrating the nine `?`-in-path templates to
  declared parameters will move their query from `path` into `query`;
  the bytes stay the same only if the declared order matches the template.

### Human review agenda items raised in Phase 5

1. Items 6 and 1 read as authorized by the phase request (no recorded
   confirmation).
2. Default 12 adopted as stated (fifth tuple element); the new public
   `shared::append_query_pairs` is part of that surface.
3. The separator is chosen from the path, preserving the pre-split
   base-URL-query behavior for byte equivalence.
4. `ResponseContext.path` no longer contains declared query parameters.
5. `schematic/schema/Cargo.lock` re-resolved by Cargo (unrelated
   `biscuit-file` dependency change).

## Phase 6

### Authorization state

- Pre-authorization items 1–8 still have no recorded author confirmation.
  Phase 6's regeneration (Task 6.4) needs item 1; as in Phases 3 and 5 it is
  read as authorized by the request to "implement phase 6" **for this phase's
  generator-produced refresh of `schema/src` only**. The exact command of
  `just generate` was run (without the desktop notification). Nothing
  generated was hand-edited.
- Item 4 (dev-dependency) was **not needed**: the generated manifest already
  carries `wiremock` and `serial_test`, and the fixture crate reuses them.
  No dependency was added anywhere; `docs/dependencies.md` is unchanged.
- Item 8 (`schematic-define`) was **not needed** as an additive change:
  `EnvMapping` and every type are unchanged. Only the doc comment of
  `RestApi::legacy_env_mapping_for` changed (default 15), plus a one-line
  inline comment in the same match arm. Within the area; logged.
- Items 2, 3, 6, 7 were not exercised.

### What was built (Tasks 6.1–6.4)

- **Raw bodies (R21, R22; default 11 adopted).** `ApiRequest::Text` →
  `pub body: String`, `ApiRequest::Binary` → `pub body: Vec<u8>`; both are
  required by `new(...)`, get the usual body-only `From<String>` /
  `From<Vec<u8>>`, and `into_parts` emits `RequestBody::Raw { content_type:
  <declared>, bytes }` (`into_bytes()` for text). The existing send branch was
  unchanged and is now proven by the wiremock tests (bytes and
  `Content-Type`, including empty bodies and non-UTF-8 bytes). The struct doc
  example no longer "imports" `String`/`Vec<u8>` as if they were schema types.
- **Header and cookie parameters (R25, R26; defaults 13, 14 adopted).** New
  `codegen/request_structs/named_params.rs`: `Option<T>` fields (query type
  mapping, `serde(rename)` to the wire name), `with_*` builders, and the
  headers-element assembly. `required` is not enforced. Endpoints **without**
  header/cookie parameters keep the old inline `vec![...]` headers expression
  and the old `into_parts` doc text, so the 500+ real endpoints that have
  none are byte-identical in that method.
  - header: a set parameter does `retain(!eq_ignore_ascii_case)` + `push`, so
    it replaces a static endpoint header of the same name; arrays join with
    `,`; unset leaves the static header.
  - cookie: each set value goes through the generated
    `shared::encode_cookie_value` (RFC 6265 cookie-octets kept, `%` included;
    every other UTF-8 byte `%XX` uppercase; space `%20`); array elements are
    encoded one by one and joined with a literal `,`. They are folded with the
    endpoint's static `Cookie` header by `shared::combine_cookie_headers`.
- **One Cookie header end to end.** `combine_cookie_headers(headers, cookies)`
  pulls every `Cookie` header (case-insensitive) out of a header list, parses
  each into `name=value` pairs, applies `cookies`, and re-inserts one header
  at the first Cookie header's position. Same name (case-sensitive) → value
  replaced in place; new name → appended. With a single Cookie header and
  nothing to add it does nothing (static cookies are never rewritten when
  there is nothing to merge). An unparseable static cookie that must be merged
  fails with `SchematicError::Header(InvalidHeaderValue("Cookie"))`. The
  client's `merge_headers` now keeps **both** API and endpoint `Cookie`
  headers (API first) instead of letting the endpoint one win, then calls
  `combine_cookie_headers` with the credential cookie. Merge order as the plan
  elaborated: API static → endpoint static → set parameters → credential.
- **Header validation before send (R24, R25).** Every merged header is
  parsed through `reqwest::header::HeaderName`/`HeaderValue`
  (`shared::header_pair`) before `req_builder.header(...)`; failure is
  `SchematicError::Header(InvalidHeaderName|InvalidHeaderValue(<name>))`,
  never echoing the value, and no request is sent. Previously reqwest
  deferred the same failure to `send()` as an opaque builder error.
- **`ApiKeyParam` (R23, R24, R27; defaults 15, 16 adopted).**
  - Client field `api_key_param: Option<schematic_define::SensitiveString>`
    (redacting `Debug` even if printed), initialized `None` by all four
    constructors and carried by `variant().build()`.
  - `with_api_key_param(mut self, key) -> Self` is generated only when the
    **effective** policy's explicit methods include `ApiKeyParam`. Consuming
    builder (the `with_` naming of default 16); the other explicit helpers are
    `&self -> Self` clones. Doc-marked temporary.
  - Env scan: when — and only when — the effective policy's `env_fallback`
    is `ApiKeyParam`, the first **non-empty** variable named by the client's
    `env_auth` (the definition's names, emitted literally into the
    constructors; a `variant().env_auth(..)` override is honored). A policy
    without that fallback reads no variable.
  - Resolution (`shared::resolve_request_auth`): explicit credential (auth
    header on `Headers` **or** explicit param key) wins and disables every env
    fallback; otherwise header fallback via `EnvMapping` and param fallback via
    the scan; required policy with neither → existing
    `AuthenticationRequired`, whose `env_fallback_vars` now lists `env_auth`
    for `ApiKeyParam`, and whose explicit-method list names
    "an explicit API key for `<name>` (`with_api_key_param`)".
    `headers_satisfy_fallback` keeps its header-only meaning; the required
    check adds `key.is_none()`, so a resolved param key satisfies an
    `ApiKeyParam` fallback ("Align headers_satisfy_fallback").
  - Injection (`shared::apply_api_key_param`): query key → remove every query
    pair with the same name (case-sensitive) and append, so it is encoded once
    by `append_query_pairs`; cookie key → `encode_cookie_value` and merged last
    into the one Cookie header. A key for a location the client cannot send
    (future `#[non_exhaustive]` variant) resolves as no key → auth required,
    never silently dropped.
  - Client `Debug` (new; clients had none): `shared::debug_client` prints the
    type name, `base_url`, and `auth_policy`, `finish_non_exhaustive`.
    `headers` (which holds explicit bearer/API-key/basic values in plain text)
    and the param key are never printed.
- **Validation (Phase 2's Rust-side gap checks removed, as the Phase 5 note
  required).** `check_auth_support`, `check_request_body_support`, and the
  Rust call of `check_unsent_params` are gone. Replaced by checks of what the
  client now sends verbatim (all `error`, Rust output only):
  - `text-request-body`/`binary-request-body`: content type empty or not
    visible-ASCII/SP/HTAB;
  - `api-key-param`: empty query name, or a cookie name that is not an RFC 9110
    token (read through the effective policy, explicit and fallback);
  - `header-parameter`/`cookie-parameter`: name not an HTTP token, or
    `Array(Array(_))` (no serialization) — rejected, not dropped;
  - `generated-name-collision` now includes header and cookie fields and the
    raw `body` field (`with_*` builders collide exactly when fields do).
  Postman still reports header/cookie parameters it does not carry
  (unchanged); OpenAPI declares them (unchanged).
- **Fixture coverage (R19, Task 6.4).** `fresh_output_compile.rs` fixtures
  grew from 3 to 7 APIs: `FixtureBearer` (+ `SearchWidgets` with static
  `X-Mode`/`Cookie` endpoint headers, 3 header and 3 cookie parameters, API
  static `X-Client` and `Cookie`), `FixtureBasic` (+ `SendNote` text body,
  `UploadBlob` binary body), `FixtureQueryKey` (legacy `ApiKeyParam` query,
  declared `api_key` query parameter that the credential must own),
  `FixtureCookieKey` (`ApiKeyParam` cookie, API static `Cookie: lang=en;
  sid=static`, declared `sid` cookie parameter), `FixturePolicyKey` (legacy
  `auth: None` + explicit-only `auth_policy`, no env fallback),
  `FixtureOAuth` (OAuth2 PKCE metadata). `FIXTURE_COVERAGE` names every
  feature; the existing guard test ties it to the fixture list.
  - `slow_fixture_apis_compile_offline` was **renamed**
    `slow_fixture_apis_compile_and_send_offline`: after the offline
    `cargo check` it adds `tests/request_side.rs` (16 wiremock tests, port 0)
    to the child crate and runs it through offline, locked nextest with
    `FIXTURE_ENV` passed to the child (no process-environment mutation; each
    child test is its own process under nextest anyway). It asserts every one
    of the 16 names appears in the child report; a plain L1 guard
    (`request_side_test_list_names_every_child_test`) ties that list to the
    `async fn`s in the source. Still exactly three principal `slow_` tests
    plus Phase 5's wire test; CI selects `slow_` by regex, so the R35 builder
    selection and archive executed/expected exclusions needed no edit
    (re-reviewed: `justfile` `test-fresh-output`, `.config/nextest.toml`
    override, `ci.yml` build-job step all key on `/(^|::)slow_/`).

### Design choice: auth resolution moved into `shared.rs` (review 1)

The first implementation put the new resolution in each client and grew every
client by ~124 lines, then ~37 after moving the `ApiKeyParam` helpers to
`shared.rs`. Two facts forced a further step:

- `unfolded_circle_core_rest.rs` was **1963** lines against the 2000-line
  `SPLIT_THRESHOLD_LINES`; +37 split it into a directory (a layout conversion
  of a real module for no behavioral reason). The `assemble_modules` split
  rule is `> 2000`.
- Six partial-generation/output tests use a two-API group of near-empty
  clients that was within ~60 lines of the threshold (two clients ≈ 2029
  lines after the change) and started producing a directory.

So the whole per-client auth block (`auth_is_required`,
`accepted_explicit_auth_methods`, `env_fallback_var_names`,
`authentication_required_error`, `apply_env_fallback`,
`headers_satisfy_fallback`, `resolve_request_headers`) moved, unchanged in
logic, into free functions emitted once into `shared.rs`
(`codegen/client/shared_auth.rs`), parameterized by
`(policy, headers, env_auth, api_key_param)`. Each client keeps one call
(`crate::shared::resolve_request_auth(...)`). Result: every client is
**168 lines smaller** than before Phase 6 (anthropic 1306 → 1138;
`unfolded_circle_core_rest` 1963 → 1800), `shared.rs` grows once, and the
whole precedence rule lives in one function the runtime/envelope follow-up
can replace. This is broader than the minimal change and touches every
client's generated code; recorded for review 1.

Finding for the fidelity/polish follow-up: a tiny client is still ~1000
lines (mostly per-client variant builder, request methods, and docs), so a
two-API group of trivial clients sits near the split threshold. The
threshold's interaction with module size is fragile.

### Requirement → test mapping (Phase 6)

| Requirement | Test(s) | Boundary / rung | Result |
| --- | --- | --- | --- |
| R21 text body | child `text_body_reaches_the_server_verbatim_under_its_content_type` (empty and `"line one\nzwei – ünïcödé\r\n"`; method, path, exact body bytes, `Content-Type: text/plain; charset=utf-8`); unit `request_structs::tests::text_body_is_a_string_field_sent_as_raw_bytes_under_its_content_type` | process + socket (F3); generated source (F1) | pass |
| R22 binary body | child `binary_body_reaches_the_server_byte_for_byte` (non-UTF-8 `[00 9F 92 96 FF 0A 0D C3]` and empty; `application/octet-stream`; path param encoded); unit `binary_body_is_a_byte_vector_sent_as_raw_bytes_under_its_content_type` | F3 / F1 | pass |
| R23 query key | child `explicit_query_key_wins_over_env_and_owns_its_query_name` (`q=x%20y&api_key=SENTINEL%20key%26%3D`: declared `api_key` replaced, `q` kept, encoded once, no cookie) | F3 | pass |
| R23 cookie key | child `cookie_key_replaces_the_same_named_cookie_in_place` (`lang=en; sid=SENTINEL%3Bv; theme=dark`: API static, parameter, and credential in one header), `env_cookie_key_is_appended_when_no_cookie_has_its_name` (`theme=t; sid=env-cookie-key`) | F3 | pass |
| R24 sources and errors | child `env_query_key_is_read_from_the_definition_env_names` (definition `env_auth`), `the_first_non_empty_env_variable_supplies_the_key` (`""` then `second-key`), `a_missing_key_is_authentication_required_and_nothing_is_sent` (error variant, `env_fallback_vars`, explicit-method text, zero requests), explicit-wins via the explicit query test (env var set, sentinel sent), `a_policy_without_env_fallback_reads_no_variable` (legacy `auth: None` + explicit-only policy: env var set but unused → `AuthenticationRequired`, zero requests; explicit key → `token=explicit`) | F3 | pass |
| R24 policy in validation | unit `validation::tests::api_key_param_is_found_through_the_effective_policy` (explicit, fallback, and a policy override hiding the legacy field) | F1 | pass |
| R25 header params | child `set_header_parameters_override_static_headers_by_case_insensitive_name` (`x-trace-id: abc 123`, `x-mode: param` replaces static `X-Mode`, `x-ids: 1,22,333`, API `x-client` kept, bearer kept, query untouched), `unset_header_parameters_keep_static_headers_and_send_nothing`, `invalid_header_parameter_values_fail_before_send_without_echoing_them` (CR/LF injection, NUL, DEL: `Header(InvalidHeaderValue("X-Trace-Id"))`, value absent from `Display` and `Debug`, zero requests); unit `header_parameters_become_optional_fields_sent_under_their_wire_names`, `named_parameters_start_unset_in_every_constructor`, `endpoints_without_named_parameters_keep_their_inline_headers` | F3 / F1 | pass |
| R26 cookie params | child `cookies_from_api_endpoint_and_parameters_share_one_header` (exactly one Cookie header `api=1; theme=dark; session=a%20b%3Bc%0D%0AX-Evil:%201; prefs=x%2Cy,%C3%A9,%`; no `x-evil` header), `unset_cookie_parameters_still_merge_api_and_endpoint_cookies` (`api=1; theme=dark; session=static`); unit `cookie_parameters_are_encoded_and_folded_into_one_cookie_header` | F3 / F1 | pass |
| R25/R26 validation | unit `invalid_header_and_cookie_parameters_fail_only_the_rust_client` (bad header name, cookie name with `;`, array of arrays), `rust_only_rejections_name_feature_and_endpoint` (content types with CR/LF and empty, cookie key name, empty query key name, header/query field collision, cookie named `body` beside a raw body), `request_inputs_the_client_sends_pass_rust_validation` (accept table incl. `__Host-id`, `openai-beta`, arrays, both key locations), `postman_reports_unsent_header_and_cookie_parameters_by_requiredness` | F1 | pass |
| R27 Debug | child `debug_output_never_contains_a_credential` (header API key, query key, cookie key, bearer token, env-sourced key; and a request struct) | F3 | pass |
| R19 coverage | `fixture_coverage_names_every_fixture_api`, `request_side_test_list_names_every_child_test`, `slow_fixture_apis_compile_and_send_offline` (7 fixture APIs compile offline; all 16 child tests reported), child `header_key_oauth_and_form_bodies_reach_the_server` (header API key, URL-encoded body bytes `name=a+b`, OAuth bearer); real 18 APIs: `slow_real_apis_compile_offline` | F3 | pass |
| R20 drift after regeneration | `slow_real_apis_compile_offline` (fresh vs committed `schema/src`: identical), `artifact_drift::*` | F3 | pass |
| shared helpers | unit `codegen::client::tests::{shared_auth_handles_all_strategies, shared_auth_bearer_env_fallback, shared_auth_api_key_env_fallback, shared_auth_basic_env_fallback, generate_auth_setup_skips_env_check_when_programmatic_auth_set, generate_request_method_applies_headers, generate_request_method_uses_runtime_auth_matching}`; L1 `http_client::{bearer_token_*, api_key_uses_runtime_auth_matching, basic_auth_uses_reqwest_basic_auth, dual_auth_*}` (now read client + `assemble_shared_module`) | F1 | pass |

Regression evidence: before this phase none of the child tests could
compile (no `body` field, no `with_x_trace_id`, no `with_api_key_param`, no
client `Debug`), and the Phase 2 validator rejected every fixture feature.
Two deliberate mutations, each run through the real guard and then reverted:
removing the credential's query `retain` failed
`explicit_query_key_wins_over_env_and_owns_its_query_name`; letting
`encode_cookie_value` keep spaces failed
`cookies_from_api_endpoint_and_parameters_share_one_header`. The fixture
guard also caught one wrong assumption of mine: `"ünï"` is a **valid** header
value for the HTTP types (RFC 9110 `obs-text`), so the invalid-value cases
use control characters and the docs say non-ASCII is sent as raw UTF-8.

Input Robustness Matrix: not applicable (no reader of a file format or
configuration was added; the Cookie-header parser reads a header built by
the generator and definitions, and its one outcome for malformed input —
error, never drop — is tested).

Test placement: all new tests are in the `l1` binary (modules already
declared) or the `schematic-gen` lib; Cargo-invoking work stays inside the
existing `slow_` fixture test. `just check-tier-coverage schematic`: 0
stranded.

### Regeneration diff (R31, Task 6.4)

- `just generate`: 58 Rust files written; **every OpenAPI and Postman file is
  byte-identical** (no artifact change). `schema/Cargo.lock` is unchanged
  since Phase 5 (14+/48−, the `biscuit-file` re-resolution logged there).
- The Rust diff (shared with Phase 5's uncommitted query split) was
  classified line by line. Phase 6 contributes: the auth block removed from
  every client and added once to `shared.rs`; the `api_key_param` field (+4
  constructor inits, +1 variant line); the `Debug` impl; `header_pair` /
  `combine_cookie_headers` / `apply_api_key_param` call sites; the
  cookie-aware `merge_headers`; and, from a feature the live definitions
  already use, the `openai-beta` header parameter on the seven OpenAI
  `Beta*` endpoints (`with_openai_beta(Vec<String>)`, sent comma-joined).
  Nothing else remained unexplained after filtering.
- Text/binary bodies, cookie parameters, and `ApiKeyParam` have zero live
  users, so they change no real request struct (fixture-only, as the spec's
  "fixture drift" risk anticipated).
- `query-in-path` still reports exactly the nine endpoints per output.
- Dependents compile against the regenerated crate: `cargo check
  --all-targets -p sniff -p biscuit-speaks -p model-citizen -p homelab -p
  eversolo-integration -p unfolded-integration-helper -p
  arcam-amp-integration -p sony-receiver-integration` — clean, no warning.
  Their test runs remain Phase 7's.

### Design choices not dictated by the spec

- Module/helper names: `named_params.rs` (`ParamLocation`,
  `generate_headers_assembly`), `client/shared_auth.rs`
  (`generate_request_auth_helpers`), generated `shared::{RequestAuth,
  ApiKeyParamCredential, resolve_request_auth, apply_api_key_param,
  header_pair, debug_client, encode_cookie_value, combine_cookie_headers}`.
  Public items are `pub` (like `append_query_pairs`) so crates without a
  caller do not warn; `explicit_api_key_param`/`env_api_key_param` and the
  moved auth helpers are private.
- `QueryParamInfo` is reused for header and cookie parameters (documented)
  rather than renamed, to keep the change surgical.
- Header-name/cookie-name grammar in validation: RFC 9110 `token`.
  Content-type grammar: non-empty, visible ASCII + SP + HTAB (stricter than
  the HTTP types, which also accept `obs-text`).
- Array cookie values join with a literal `,` after per-element encoding
  (RFC 6265 does not allow `,` in a cookie value; servers generally accept
  it, and per-element encoding keeps element commas distinguishable). `%` is
  kept literally per default 13, so a literal `%20` and an encoded space are
  indistinguishable to a decoding server; documented as "no server is
  assumed to decode". Both for review 1.
- Explicit-credential semantics: an explicit **parameter** key also disables
  header env fallbacks (and an explicit header credential disables the param
  scan), extending the existing "explicit auth suppresses env fallback" rule
  to the new credential kind.
- Nested arrays in **query** parameters already generate code that does not
  compile (`Vec<_>: Display`); not a Phase 6 feature and no feature id fits,
  so it is recorded here, not fixed (no live definition has one; the full
  guard compiles).
- `HeaderError::InvalidHeaderValue("Cookie")` is reused for an unparseable
  static cookie (no new error variant).

### Comment and doc drift found (code assumed correct)

- `codegen/request_structs/body.rs` module doc said bodies were JSON only and
  carried "Text and Binary bodies are not yet carried"; corrected with the
  code.
- `request_structs/mod.rs` `generate_request_struct` doc listed "a `body`
  field if the endpoint has a request schema"; now names all body kinds.
- `client/methods.rs` `merge_headers` doc now states the Cookie exception;
  `client/helpers.rs` `generate_auth_setup` doc described a per-client helper.
- `RestApi::legacy_env_mapping_for` doc/comment ("the legacy mapping omits
  them") now says where parameter keys are resolved (default 15).
- `docs/generation/request-side.md`: the **planned** section was replaced by
  the shipped behavior (raw bodies, header parameters, cookie serialization
  and merge order, `ApiKeyParam` sources/injection/errors with a Mermaid
  flow, header validation, Debug); intro, flow diagram, and tuple table
  updated. `docs/generation/validation.md`: the "cannot send yet" rows were
  replaced by the new rules. `gen/README.md`: authentication precedence,
  generated-code sample (it showed the old per-client
  `self.apply_env_fallback`), new `ApiKeyParam` subsection, raw bodies, and a
  header/cookie section. `.claude/skills/schematic-define/SKILL.md` said Text
  and Binary bodies "are still not carried"; corrected. `schematic/skill`,
  `schematic-define/schematic/SKILL.md` (strategy table) and `gen.md`
  (`shared.rs` contents) updated.

Pre-existing, not fixed (outside this phase's symbols): `cargo doc -p
schematic-define` with `-D rustdoc::broken_intra_doc_links` fails on
``[`Display`]`` in `define/src/headers/sensitive.rs:8`. The new links in
`legacy_env_mapping_for`'s doc resolve.

### Broader gates

- `just test` (schematic): exit 0. L1 **1773 passed, 9 skipped** (Phase 5:
  1764); fresh-output guard **4 passed** in 102.3 s total (child rebuild of the
  regenerated 18-API crate plus the 7-API fixture crate with its 16 wiremock
  tests; the fixture test alone is ~4.7 s warm).
- `just lint` (schematic): exit 0 (clippy only; `_lint` runs no formatter).
  Also `cargo clippy -p schematic-gen --all-targets --features
  terminal-tests -- -D warnings`: clean.
- `just test-schema`: 20 passed. `just lint-schema` (clippy `-D warnings` on
  the regenerated crate): clean.
- `artifact_drift::` ran inside `just test`: pass.
- `just test-l2` (tmux, no focus): 3 passed.

### Platform evidence

- macOS (this host): everything above.
- Linux, native Windows, WSL2: not run in Phase 6. The changes are generated
  string handling, header parsing through `http` types, and `std::env::var`
  reads; no path, filesystem, process, or console API changed. One
  OS-sensitive detail is passed to the child: `FIXTURE_EMPTY_KEY=""`. On
  Windows an empty variable may read as absent rather than empty; the test
  passes either way (both fall through to the second name), so it proves
  "empty is skipped" only where empty variables exist. `cross-check` runs no
  `slow_` tests; CI's Linux build job runs the guard (R35). Phase 7 owns
  per-OS evidence.

### Files touched outside `schematic/`

- `.claude/skills/schematic-define/SKILL.md`,
  `.claude/skills/schematic-define/schematic/SKILL.md`,
  `.claude/skills/schematic-define/schematic/gen.md`.

### Downstream follow-up notes

- Runtime/envelope follow-up: a query key is the pair named by
  `ApiKeyParamCredential.name` in `RequestParts.4` after
  `apply_api_key_param`, so URL redaction can mask exactly it. Until then the
  key **is** in the request URL, `ResponseContext.url`, and therefore in any
  `reqwest::Error` text (`SchematicError::Http` Display includes the URL).
  `with_api_key_param`, the env scan, and `shared::resolve_request_auth` are
  the surfaces it replaces.
- Definition-API follow-up: header/cookie `required`, style, `explode`, and
  enum fidelity remain unimplemented (default 14).

### Human review agenda items raised in Phase 6

1. Item 1 read as authorized by the phase request (no recorded confirmation);
   items 4 and 8 turned out unnecessary.
2. Default 16 surface: `with_api_key_param(mut self, key) -> Self`
   (consuming), generated only when the effective policy accepts an explicit
   `ApiKeyParam`; key stored as `SensitiveString`. Temporary.
3. Default 15: env scan reads the client's `env_auth` (definition names,
   variant-overridable), first non-empty wins, only under an `ApiKeyParam`
   env fallback. Temporary. `EnvMapping` unchanged.
4. The per-client auth block moved into `shared.rs` (every client −168
   lines, one resolver) — broader than minimal, chosen to keep
   `unfolded_circle_core_rest` below the split threshold and to keep one
   precedence implementation.
5. New client `Debug` impls (clients had none) print base URL and auth
   policy only.
6. Cookie encoding keeps `%`; array cookies join with a literal `,`;
   non-ASCII header values are accepted as `obs-text`.
7. `merge_headers` now keeps the API's `Cookie` header alongside the
   endpoint's (they used to be replaced by it), merged cookie by cookie.
8. A query key is visible in URLs and `reqwest` error text until the
   redaction follow-up.
9. The fixture guard test was renamed (`slow_fixture_apis_compile_offline` →
   `slow_fixture_apis_compile_and_send_offline`); CI selection is by `slow_`
   regex, so no CI file changed.

## Phase 7

### Authorization state

- Pre-authorization items 1–8 still have no recorded author confirmation.
- **Item 2 (delete `schematic/gen/schematic/`) — not exercised; R30 left
  incomplete.** The precondition checks passed (below), but the deletion
  command was **refused by the session's permission layer** ("irreversible
  local destruction"). With no recorded author confirmation of item 2 either,
  the plan's rule applies: the action stays pending and the boundary is
  recorded. No workaround (moving the tree, `git rm`) was attempted. The five
  files are unchanged on disk and in the index.
- Item 3 (`checks/cargo-machete.txt`) — condition not met: the file is a
  captured `cargo machete` console transcript (last touched by an unrelated
  sentrux-baseline commit, `0fdbf0dbb`) and no recipe, script, or workflow
  regenerates it (searched root `justfile`, `just/`, `scripts/`, `.github/`).
  Default 18 therefore keeps the file; the stale line is a finding (below).
- Item 1 (`just generate`) read narrowly, as in Phases 3, 5, and 6, as
  authorized by the request to implement this phase, for the generator-made
  refresh only. The recipe's exact `cargo run` command was used (without its
  desktop notification). Nothing generated was hand-edited.
- Item 7 (ignored e2e tests, `test-e2e`) remains unexercised since Phase 1:
  `e2e_generation.rs` still has three `#[ignore]`d compile/clippy tests and
  `just test-e2e` still uses `cargo test`. Not a Phase 7 task; carried to the
  review agenda.
- Items 4, 6, 8: not needed in Phase 7.

### Task 7.1 — stale tree (R30): checks done, deletion pending

> **Superseded (2026-10-12):** the author authorized deletion (review 1, item 2) and removed the five files with `git rm -r` during the decision session; R30 is complete.

- `git ls-files schematic/gen/schematic` → exactly five files:
  `schema/{Cargo.toml, src/lib.rs, src/openai.rs, src/prelude.rs, src/shared.rs}`.
- Not a workspace member: `cargo metadata --no-deps` lists no manifest under
  `gen/schematic/`; its `path = "../define"` dependencies do not even resolve
  from that location, so it cannot build.
- Not a test or CI input: no reference in `Cargo.toml`, `.github/`,
  `.config/`, `scripts/`, `just/`, any justfile, or `schematic/gen/` sources or
  tests.
- Maintained references (snapshot specs/plans/reviews excluded): only
  `checks/cargo-machete.txt:10`
  (`schematic-schema -- ./schematic/gen/schematic/schema/Cargo.toml: tokio`).
  Remaining mentions are in snapshots (`schematic/reviews/2026-07-31-macro-review`,
  `features/2026-09-20-upgrade-reqwest`, `fixes/2026-10-06-planner-metadata-no-deps`)
  and are left as written.
- **Finding (review):** `checks/cargo-machete.txt` lists the stale tree's
  manifest as an unused-`tokio` report. The file has no regeneration recipe;
  it is preserved unchanged. Once the tree is deleted, that entry describes a
  file that no longer exists; re-capturing the transcript is an author action.
- To finish R30 once item 2 is confirmed: `git rm -r schematic/gen/schematic`
  (or an ordinary delete), then verify absence on disk and five deletion
  entries in the diff. Nothing else depends on it.

### Task 7.2 — behavior-docs audit (drift found and fixed; code assumed correct)

An independent read-only audit compared every current doc with the code;
each claim below was re-verified before editing. The three
`docs/generation/` pages were accurate except one item. No `docs/` page names
or links a feature/fix, and none mentions `gen/schematic/`.

- `gen/README.md`: client struct sample (old `headers: Vec<(String, String)>`,
  no `auth_policy`/`api_key_param`, no `Debug` note); "Ignored slow tests
  compile generated output" (the `slow_*` guard is not ignored and runs in
  `just test`); "no mocked server" and Future Work items "runtime behavior"
  and "multi-API compile" (both exist now); `cargo check -p schematic-schema`
  in two places (the crate is excluded from the workspace, so the command
  fails) → `just test-fresh-output` / `cargo check --manifest-path …`;
  manual-verification paths `elevenlabs.rs` → `elevenlabs/client.rs` and the
  missing `generate` subcommand; dependency block (reqwest 0.12
  `rustls-tls`) → the `cargo_gen.rs` template; output tree (split modules
  are directories, WebSocket modules missing).
- `.claude/skills/schematic-define/schematic/gen.md`: same client struct
  sample; `variant(&self, base_url, env_auth, strategy)` is really
  `variant_with`, and `variant(&self)` returns a `VariantBuilder`;
  `SchematicError` list missing five variants; generator-modules table
  (directories, plus `client/shared_auth.rs` and `request_structs/named_params.rs`);
  output tree.
- `.claude/skills/schematic-define/SKILL.md`: "`Endpoint.params` is still
  dropped by the OpenAPI exporter" — false (`export/paths.rs:99` calls
  `map_parameters`, which emits query, header, and cookie parameters);
  bullet removed.
- `schematic/skill/SKILL.md`: `just gen` → `just generate` (no `gen` recipe).
- `schematic/docs/dependencies.md` (stale before this spec): generated
  crate listed reqwest 0.12 and omitted `schematic-define`,
  `schematic-definitions`, `bytes`, `urlencoding`, `futures-util`,
  `tokio-stream`, `tokio-tungstenite`, and the `wiremock`/`serial_test`
  dev-dependencies; `schematic-gen` omitted `biscuit-file`, `colored`,
  `schematic-definitions`, `serde`, `serde_json`, `openapiv3`, `indexmap`,
  and the `jsonschema` dev-dependency, and listed a `toml` dev-dependency the
  manifest does not have. Rewritten from the manifests; no crate was added
  by this spec.
- `schematic/docs/generation/validation.md`: "WebSocket validation is
  expected to report through…" described unbuilt behavior without a marker;
  now **Planned:** (the validator has no WebSocket path).
- **Code comment drift (generator):** `codegen/request_enum.rs` emitted
  `/// Converts the request into (method, path, body, headers) parts.` on
  every request enum's `into_parts`, a 4-tuple, although `RequestParts` has
  had five elements since Phase 5; its `## Errors` also omitted the
  `SchematicError::Header` the inner cookie-merging `into_parts` can return.
  Fixed in the generator, then regenerated (below).
- Left as is (minor, recorded): `gen/README.md`'s sample shows
  `SchematicError` inside a per-API file (it lives in `shared.rs`), and its
  "DO NOT verify combined shared-module APIs at runtime" line in
  `schematic/README.md` still holds (the group is compile-checked only).

### Task 7.3 — final regeneration (R20, R31)

- First `just generate` run (before the comment fix): **zero diff**; every
  Rust, OpenAPI, and Postman file byte-identical to the committed Phase 6
  output.
- After the `request_enum.rs` comment fix: 15 Rust files changed
  (`schema/src/{anthropic,lmstudio,samsung_smart_tv,unfolded_circle_core_rest}.rs`
  and `requests.rs` of the 11 split modules), 90+/54−, exactly 18 copies of one
  doc-comment hunk (one per request enum; grouped modules hold two). No code
  line changed; OpenAPI and Postman unchanged; `schema/Cargo.toml` and
  `schema/Cargo.lock` unchanged. Explained by comment drift, which is neither
  R14 nor R28 but changes no behavior (recorded for review).
  `unfolded_circle_core_rest.rs` is 1802 lines (split threshold 2000).
- `query-in-path`: 9 endpoints for each of `rust`, `openapi`, `postman`
  (`--dry-run`, unique `api endpoint` pairs per output).
- Gates after regeneration (macOS): `just test` exit 0 (L1 1774 passed, 9
  skipped; guard 4 passed), `just lint` 0, `just lint-schema` 0,
  `just test-schema` 20 passed, `just check-drift` 7 passed (6
  `artifact_drift::` + `slow_real_apis_compile_offline`), `just test-l2` 3
  passed (tmux, no focus change), `cargo clippy -p schematic-gen
  --all-targets --features terminal-tests -- -D warnings` clean,
  `just check-tier-coverage schematic` 0 stranded.

### Task 7.4 — dependent packages (R31)

Discovered through `cargo metadata`: exactly eight workspace packages
depend directly on `schematic-schema`, matching the spec's list (the five
homelab crates are `homelab`, `eversolo-integration`,
`unfolded-integration-helper`, `arcam-amp-integration`,
`sony-receiver-integration`). Root `just test {pkg}`, one at a time, after
the final regeneration. No dependent file changed.

| Package (`just test` selection) | Result |
| --- | --- |
| sniff (+ sniff-cli, `sniff/remote`) | **pass**, 3355 passed, 33 skipped — after a host link failure, see below |
| biscuit-speaks | pass, 492 passed, 19 skipped |
| model-citizen | pass, 170 passed |
| homelab | pass, 224 passed, 8 skipped |
| eversolo-integration | pass, 28 passed, 3 skipped |
| unfolded-integration-helper | pass, 28 passed |
| arcam-amp-integration | pass, 21 passed, 2 skipped |
| sony-receiver-integration | pass, 27 passed, 3 skipped |

sniff's first run failed to **link** (`ld: library 'git2' not found`, search
path `/opt/homebrew/Cellar/libgit2/1.9.7/lib`; installed is `1.9.7_2`): the
Homebrew-upgrade trap the `os` skill already records (`macos.md`). Rerun with
the documented `LIBGIT2_NO_PKG_CONFIG=1` (vendored libgit2) → pass. Host
environment, unrelated to this spec; nothing in sniff changed.

### Task 7.5 — platform evidence (R16–R18, R35)

Hosts declared: `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`,
`BUILD_WSL=build-win`; `BUILD_MACOS` unset. Both runs used
`scripts/cross-check.sh` on the working tree (`+656 changed files` over
`origin/feat/modern-config` 3ec131376); no receipt was published (the tested
tree is uncommitted), so none of this is CI-reusable evidence.

| Environment | L1 (archive/CI mode, `slow_` excluded) | Fresh-output guard, native child Cargo |
| --- | --- | --- |
| macOS (this host) | `just test`: 1774 passed | 4/4 pass (warm, ~16–23 s) |
| Linux `build-linux` | 624 passed, 12 skipped (archive from an ubuntu-latest-style build) | **4/4 fail — host Cargo cache gap**: `failed to download cc v1.2.52 … --offline was specified`. The guard failed loudly, as R16/default 8 require; **not proven natively on Linux** |
| native Windows `build-win-native` | 622 passed, 12 skipped (2 unix-only tests absent) | **4/4 pass**, 169 s (cold child target) |
| WSL2 `build-win` | 624 passed, 12 skipped (archive mode, builder target hidden — CI's WSL2 shape) | **4/4 pass** natively in the guest, 139 s (cold) |

- Guard command: `./scripts/cross-check.sh schematic-gen --os all --features
  terminal-tests 'fresh_output_compile::slow_'` (a build flag switches
  `cross-check` to plain nextest so `slow_` tests can be selected).
- Linux: filling the cache needs `cargo fetch --manifest-path
  schematic/schema/Cargo.toml` on that host (what CI's build job does before
  the guard). That needs a remote shell, which this session is not
  authorized to open outside `cross-check`; left unproven. This is the first
  host where offline compile was unavailable (spec risk "Offline compile
  unavailable").
- **Test-helper change prompted by this evidence (small, beyond the plan's
  task list):** the failure printed the command and Cargo's stderr but not
  what to do. `tests/common/fresh_output.rs` now has `OFFLINE_CACHE_HINT` /
  `offline_cache_hint(stderr)`; `CargoRun::assert_success` and the negative
  control's assertion prepend `hint: … run cargo fetch --manifest-path
  schematic/schema/Cargo.toml …` when Cargo reports `but --offline was
  specified`. New L1 test
  `fresh_output_compile::offline_cache_misses_name_the_fetch_that_fills_the_cache`
  uses build-linux's exact stderr (positive), a compiler error, and empty
  stderr (negative). No production code changed. `schematic/README.md`'s
  existing fetch note mentions the hint; the `os` skill's `build-hosts.md`
  records the cross-check guard command and the build-linux cache gap.
- R35 (CI): no hosted run contains this spec's work (latest `ci.yml` run on
  the branch is 2026-10-09, before Phase 1; nothing pushed, no run or label
  created to find out). Fallback evidence, re-read this phase: the build
  job's "Run the schematic fresh-output guard" step (`ci.yml` ~684) runs
  `cargo fetch --manifest-path schematic/schema/Cargo.toml` then
  `just -f schematic/justfile test-fresh-output` **before** "Produce the
  archives" (~709) whenever the owner builds schematic-gen, fails the job on
  failure, and uploads its JUnit report; archive consumers exclude the tests
  through the shared L1 filter's `slow_` exclusion in both executed and
  expected listings (`.config/nextest.toml`, `scripts/ci/consolidation.py`
  `MARKER_TIERS`), and the WSL archive run above (624 tests, no `slow_`)
  shows the same. `ci_workflow_contracts::the_build_owner_runs_the_schematic_fresh_output_guard_before_archiving`
  (Phase 1) guards the order. **Unproven until the first hosted run:** the
  guard's CI duration and the completion records of a real run.

### Requirement → test mapping (Phase 7)

| Requirement | Test / evidence | Boundary / rung | Result |
| --- | --- | --- | --- |
| R30 stale tree deleted | precondition checks above | filesystem | ~~**incomplete** — deletion refused; item 2 unconfirmed~~ **complete** (2026-10-12, author ruling on review 1 item 2) |
| R31 generator-only regeneration | `just generate` diff (doc-only, 18 hunks), `slow_real_apis_compile_offline` (fresh = committed), `artifact_drift::*` | process + filesystem (F3) | pass |
| R31 dependents | root `just test` of all 8 dependents (table above) | crate boundary (F3) | pass |
| R16 fails loudly offline | build-linux run (4 loud failures, no skip); `offline_cache_misses_name_the_fetch_that_fills_the_cache` | process (F3) / F1 | pass |
| R16–R18 on Windows/WSL2 | native guard runs on build-win-native and build-win | process, per OS (F3) | pass |
| R35 placement | workflow re-read + Phase 1 contract test; WSL archive listing | CI config (F1) | pass (fallback); hosted run unproven |
| comment drift fix | `slow_real_apis_compile_offline` (fresh generation = committed, which now carries the corrected doc) | F3 | pass |

Input Robustness Matrix: not applicable (no reader of a file format or
configuration was added).

### Timing (this phase)

- `just test` (macOS, warm): L1 ~2.5 s of test time; fresh-output guard 16–23
  s for its four tests (fixture test ~5 s, real-API ~2–7 s, wire ~7–9 s,
  negative control ~1.6 s).
- Cold child target, other hosts: native Windows 169 s, WSL2 139 s for the
  four tests (fixture test 110–140 s of that).
- CI build-job duration of the guard: not measured (no hosted run).

### Defaults 1–22 as finally used

| Default | Final value |
| --- | --- |
| 1 diagnostics format | As stated; `api` is a list so group diagnostics name every API (`api=A,B`); `endpoint=<none>` for API/group scope. |
| 2 machine readability | As stated: `Serialize`, no new flag, stderr, nonzero only for errors/write failures. |
| 3 inventory | As stated, refined: a directory module is evidenced by an owned `mod.rs`; whole-tree ownership is needed only to delete it. |
| 4 shared-runtime rule | As stated; runtime-bearing files are `shared.rs` and `ws_shared.rs`. |
| 5 write order | Writes in the stated order, `lib.rs` last write; **deviation:** deletions of obsolete paths follow `lib.rs`. |
| 6 duplicate id scope | As stated (raw ids per API; emitted names across a group). |
| 7 operationId rename | As stated; observed renames: Ollama `Embeddings`, `ListModels`. |
| 8 compile mechanism | As stated except `cargo update --workspace --offline` instead of `cargo generate-lockfile --offline` (keeps the copied pins). Offline failure now also prints a fetch hint (Phase 7). |
| 9 ignored e2e tests | **Not applied:** the three `#[ignore]`d tests and `cargo test`-based `just test-e2e` remain (item 7 unconfirmed). |
| 10 runtime-test location | As stated: fixture crate built by the helper, wiremock/raw TCP, nextest. |
| 11 text/binary types | As stated (`String`, `Vec<u8>` → `RequestBody::Raw`). |
| 12 `RequestParts` | As stated: 5-tuple, query pairs fifth. |
| 13 cookie serialization | As stated; `%` kept literally; array elements encoded then joined by `,`. |
| 14 header/cookie typing | As stated; `required`/style/explode/enum not honored. |
| 15 `ApiKeyParam` env | As stated (temporary): first non-empty `env_auth` var, only under an `ApiKeyParam` env fallback; `EnvMapping` unchanged. |
| 16 explicit key | `with_api_key_param(mut self, key) -> Self`, `SensitiveString` storage, generated only when the effective policy accepts it (temporary). |
| 17 guard scope | Two principal tests + negative control + the distinct wire-baseline test (four `slow_`); fixture test renamed `slow_fixture_apis_compile_and_send_offline` in Phase 6. |
| 18 `cargo-machete.txt` | No regeneration recipe → preserved; stale line recorded. |
| 19 docs | Three pages: `docs/generation/{validation,partial-generation,request-side}.md` (split instead of one combined page). |
| 20 feature ids | The ten listed plus six added (`invalid-identifier`, `invalid-request-suffix`, `generated-name-collision`, `invalid-module-path`, `invalid-output-filename`, `incomplete-schema-registry`). `duplicate-method-path` is a **warning** (R13 deviation). |
| 21 test files | `fresh_output_compile.rs`, `partial_generation.rs` as named; request-side tests went into `fresh_output_compile.rs` (child crate) and existing modules rather than a separate `request_side_features.rs`. |
| 22 partial exports | As stated; both module resolvers give the same 15 names for the live APIs. |

### Acceptance ledger R1–R35 (final)

| Req | Evidence (phase) | Status |
| --- | --- | --- |
| R1–R2 | `partial_generation::partial_run_regenerates_the_whole_group_and_keeps_every_other_module` and the real-tree partial compile (4) | pass |
| R3–R7 | `partial_generation::{owned_stale_files_…, refuses_*, failure_during_publication_lists_completed_and_pending_mutations, exports_rewrite_only_the_selected_groups, each_root_establishes_its_own_safety}` (4) | pass |
| R8–R10 | `output_validation::{malformed_rust_type_blocks_only_the_rust_client, incomplete_registry_blocks_only_openapi, …}`, `diagnostics::tests::*` (2) | pass |
| R11–R12 | `validation::tests::*` accept/reject tables, group name collisions (2) | pass |
| R13 | `path_parameter_names_do_not_distinguish_routes`; OpenAPI merge/collision tests (2, 3) | pass with **deviation**: same-API duplicate route is a warning. **Superseded (review-1 fix cycle 1):** met as written; error in every output, see "Review 1, ruling 1" |
| R14–R15 | `export::openapi::tests::*` allocation/link/scan tests; `artifact_drift::committed_openapi_documents_have_unique_operation_ids` (3) | pass |
| R16–R18 | `slow_fixture_apis_compile_and_send_offline`, `slow_real_apis_compile_offline`, `slow_negative_control_reports_compiler_error`, `CheckoutWitness` (1, 6); native macOS/Windows/WSL2 runs (7) | pass on macOS, Windows, WSL2; **Linux native unproven** (host cache gap) |
| R19 | `fixture_coverage_names_every_fixture_api`, `request_side_test_list_names_every_child_test`, fixture guard (6) | pass |
| R20 | `slow_real_apis_compile_offline` (fresh = committed), `artifact_drift::*` (1, 7) | pass |
| R21–R27 | 16 child wiremock tests in the fixture crate + unit/validation tests (6) | pass |
| R28–R29 | `slow_query_wire_matches_pre_split_baseline` (pre-change golden), `query_params_codegen::*` (1, 5) | pass |
| R30 | stale tree deletion | ~~**incomplete** (pending item 2)~~ **complete** (2026-10-12) |
| R31 | regeneration diffs explained (3, 5, 6, 7); 8 dependents tested (7) | pass |
| R32 | measured counts (1) | recorded; "six"/"about 29" do not reconcile |
| R33–R34 | `live_definitions_warn_once_per_output_for_each_embedded_query` (9 per output), compatibility refusal/recovery tests (2, 4) | pass |
| R35 | workflow step + contract test (1), placement re-read (7) | pass by fallback; hosted run not yet available |

### Files touched outside `schematic/` (Phase 7)

- `.claude/skills/os/build-hosts.md` (cross-check command for `slow_`
  guard tests; build-linux cache gap).
- `.claude/skills/schematic-define/SKILL.md`,
  `.claude/skills/schematic-define/schematic/gen.md` (drift).
- No dependency added; no CI file changed; no dependent crate changed.

### Human review agenda — consolidated for review 1

Phase-specific items are in each phase's section; this is the full agenda.

1. **Authorization:** items 1–8 were never confirmed. Item 1 was read as
   granted by each phase request (Phases 3, 5, 6, 7); item 2 (delete
   `schematic/gen/schematic/`) was **refused by the permission layer** and is
   pending, so R30 is incomplete (superseded 2026-10-12: the author deleted
   it; R30 is complete); item 3's condition was not met; item 7 was
   never exercised (ignored e2e tests and `cargo test`-based `test-e2e`
   remain); items 4 and 8 were unnecessary; item 5 was used (temp crates,
   child target dir); item 6 was used in Phase 5.
2. `checks/cargo-machete.txt` names the stale tree's manifest; no recipe
   regenerates it.
3. R13 deviation: same-API duplicate method+path is a warning (GitHub
   `GetGitTree`/`GetGitTreeRecursive`, Bitbucket `ListDirectoryContents`/`GetFileContentRaw`).
4. Default 5 deviation (deletions after `lib.rs`); default 3 refinement;
   default 8 `cargo update` departure; default 19 split into three pages;
   six added feature ids (default 20); default 21 file placement.
5. Inventory reader and byte-for-byte shared-runtime rule: adequate for now;
   both are the interim the runtime/envelope follow-up replaces with a
   persisted compatibility marker.
6. Temporary surfaces for the runtime/envelope follow-up:
   `with_api_key_param`, the `env_auth` scan, `shared::resolve_request_auth`,
   and the 5-tuple `RequestParts` (query pairs are a separate element, so
   redaction can find the `ApiKeyParam` pair).
7. Auth resolution moved into generated `shared.rs` (every client −168
   lines); new client `Debug` impls; `merge_headers` now keeps API and
   endpoint `Cookie` headers and merges them cookie by cookie.
8. Cookie encoding keeps `%`; array cookies join with `,`; non-ASCII header
   values pass as `obs-text`.
9. A query API key is visible in URLs and `reqwest` error text until the
   redaction follow-up; nested query arrays generate non-compiling code (no
   live user).
10. Offline compile per host: macOS, native Windows, WSL2 pass; **build-linux
    failed for a cache gap** (needs `cargo fetch` there); CI's guard duration
    and completion records await the first hosted run.
11. R32: 549 endpoints (not ~519); multipart 16 endpoints (not "six" or
    "about 29"); 7 live header parameters (OpenAI `openai-beta`).
12. `query-in-path` stays a warning with nine endpoints per output; the
    definition-API follow-up owns raising it and migrating them.
13. Generated diffs: Phase 3 (Ollama ids), Phase 5 (query split, lockfile
    re-resolution), Phase 6 (auth move, `openai-beta`), Phase 7 (one
    doc-comment fix in 18 request enums). Nothing unexplained.
14. Postman does not carry header/cookie parameters (reported as warnings);
    the split threshold sits close to small two-API groups.
15. Phase 7 test-helper addition (offline fetch hint) went slightly beyond
    the task list; small and test-only.

### Status

Implementation complete, ready for review — **with R30 incomplete** (stale
tree deletion pending author confirmation of item 2) and two evidence gaps
recorded (native Linux guard run, first hosted CI run). Lifecycle location
unchanged; nothing committed or staged.

> **Superseded (2026-10-12):** R30 is complete (author deletion, review 1 item 2), and R13 is enforced as errors per the review 1 item 1 ruling (see "Review 1, ruling 1").

## Implementation of Review Findings #1

> **started at:** 2026-10-12T08:27:04-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-11-schematic-generation-integrity/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-11-schematic-generation-integrity/review-1.md' indicated that the specification is **production ready**!
- the specification file used to define the functional/non-functional target of all this work can be found at '/Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-11-schematic-generation-integrity/spec.md'
- caveat on the readiness signal: the review's frontmatter sets `ready: true` with `findings: []`, but its body states that it stopped at the human-decision check and that **production readiness has not been assessed** (no implementation comparison, no tests run)
- while the review found this feature to be production ready, it did have findings worth looking at for follow on work:
        - the review recorded no code findings or observations; the follow-on work comes from the two human rulings dated 2026-10-12:
        - Item 1 — R13 enforced as written (duplicate `method + path` routes become `Severity::Error`): **not yet implemented**
                - `DUPLICATE_METHOD_PATH_SEVERITY` (still `Severity::Warning` in `schematic/gen/src/validation.rs`) and its warning behavior are to be removed
                - GitHub `GetGitTree` and `GetGitTreeRecursive` merge into one `GetGitTree` with an optional `recursive` query parameter
                - Bitbucket `ListDirectoryContents` and `GetFileContentRaw` merge into one source endpoint returning `Binary`
                - regenerate with `just generate`, update the docs and skills that describe the warning (e.g. `schematic/docs/generation/validation.md`), and log the departure from the spec's D36 forward hook
        - Item 2 — deletion of the stale `schematic/gen/schematic/` tree (R30): **done** during the decision session (`git rm -r`, five files, uncommitted); R30 is now complete
        - deferred items listed under "Deferred work and verification limits": three ignored end-to-end tests and the `cargo test`-based `test-e2e` recipe, Postman header/cookie parameters, the native Linux guard run (offline cache gap), and the first hosted CI run
- refer to the review file -- /Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-11-schematic-generation-integrity/review-1.md -- for more details

### Review 1, ruling 1 — R13 enforced as errors (2026-10-12)

The author's 2026-10-12 ruling on review-1 item 1 withdraws the D36 forward
hook and the `DUPLICATE_METHOD_PATH_SEVERITY` warning recorded in Phase 2
("Deviation from R13"). An endpoint is `method + path` permanently; no
selector ever joins `endpoint_uniqueness_key`. The spec is left as written
(snapshot); this section records the departure from its selector hook.

- **Validation (`gen/src/validation.rs`):** `DUPLICATE_METHOD_PATH_SEVERITY`
  is removed; `check_routes` reports a same-API duplicate route as
  `Severity::Error` in all three outputs, with no exception list. The OpenAPI
  "document keeps only one operation" consequence clause is gone (nothing is
  written now); the message tells the author to merge the two endpoints.
  `QUERY_IN_PATH_SEVERITY` remains the only route-check severity constant
  (R33 specifies a warning). The test `interim_route_checks_are_warnings` is
  now `query_in_path_severity_is_a_warning`.
- **GitHub:** `GetGitTree` and `GetGitTreeRecursive` merged into one
  `GetGitTree` with an optional `recursive` boolean query parameter,
  response `GitTreeResponse`. 17 → 16 endpoints.
- **Bitbucket:** `ListDirectoryContents` (JSON `PaginatedResponse<SourceEntry>`)
  and `GetFileContentRaw` (`Text`, `Accept: text/plain`) merged into
  `GetSource`, returning `Binary`, keeping Bitbucket pagination (`page`,
  `pagelen`) and no `Accept` override (it would turn a directory listing
  into an error or text). Id choice: Bitbucket names this resource `src`
  ("Get file or directory contents"); `GetSource` names the resource rather
  than one of the two shapes, so it stays correct when the response-model
  follow-up turns `Binary` into a `Directory | File` enum. `SourceEntry` and
  its registry entry are kept for that follow-up and for callers decoding a
  listing. 16 → 15 endpoints.
- **Gitea `GetGitTreeRecursive`** (`?recursive=true` in its path) is
  untouched: its raw path differs from `GetGitTree`'s, so generation reports
  no duplicate route for it (only its existing `query-in-path` warning).
- **Regeneration (`just generate`):** exit 0, `[OK]` for all three outputs,
  zero `duplicate-method-path` lines; warnings are unchanged (27
  `query-in-path`, 7 Postman `header-parameter`). Generated diff, beyond the
  files already modified by earlier phases: `schema/src/github/{requests,mod}.rs`
  (`GetGitTreeRecursiveRequest` removed, `GetGitTreeRequest` gains
  `recursive: Option<bool>`), `schema/src/bitbucket/{requests,client,mod}.rs`
  (`ListDirectoryContentsRequest` and `GetFileContentRawRequest` replaced by
  `GetSourceRequest`; the `get_file_content_raw` text helper replaced by a
  `get_source` bytes helper; `Bitbucket::request_text` is no longer generated
  because no Bitbucket endpoint returns `Text`),
  `openapi/github.json` (the route's one operation is now `GetGitTree`, its
  `recursive` parameter optional), `openapi/bitbucket.json` (`GetSource`,
  `application/octet-stream`, pagination parameters), and the two matching
  Postman collections (one request each instead of two). The two operations
  previously missing from the OpenAPI documents (`GetGitTree`,
  `ListDirectoryContents`) are now described by the merged operation.
- **Premise contradiction (dependent crate):** the ruling states nothing
  outside `schematic/` uses the four endpoints. A repository search found
  that `sniff` does, behind its `remote` feature (enabled by `sniff-cli`,
  `worktree`, `darkmatter`, and `dmls`):
  `sniff/lib/src/remote/github.rs` (`GetGitTreeRecursiveRequest`, lines 114
  and 148) and `sniff/lib/src/remote/bitbucket.rs`
  (`ListDirectoryContentsRequest` line 272, `GetFileContentRawRequest` and
  `get_file_content_raw` lines 438–441); `cargo check -p sniff --features
  remote` fails with 5 errors. Per R31 ("if a dependent breaks, stop and
  record it ... rather than editing the dependent") and because the ruling
  authorizes only the two definition edits, `sniff` was **not** edited. This
  needs an author decision. `sniff/docs/repos/repo-tech-design.md` names the
  old endpoints too.
- **Tests:** `validation::tests::path_parameter_names_do_not_distinguish_routes`
  asserts the error in every output; new
  `validation::tests::same_route_in_different_apis_is_not_a_duplicate`;
  new `output_validation::same_api_duplicate_route_blocks_every_output`
  (`/things/{id}` and `/things/{name}` through `generate_batch`: all three
  outputs fail and no output root is created); new
  `output_validation::live_definitions_declare_no_duplicate_routes` (every
  API `resolve_all_apis` returns, all three validators). Definition tests:
  GitHub `git_tree_endpoint_has_optional_recursive_query_param`, Bitbucket
  `source_endpoint_serves_directories_and_files_as_binary`, pagination and
  count tests updated.
- **Class sweep** ("a validation rule the spec requires as an error is
  emitted below that severity, or carries a per-rule severity constant or
  exception list"): checked `check_routes` (duplicate id: error;
  `query-in-path`: warning, as R33 specifies), every other `Severity` site in
  `validation.rs` (all errors except `check_unsent_params`),
  `export/openapi.rs` (group route collision, duplicate `operationId`,
  unresolved link: all errors), the Postman validator, and the WebSocket
  planner (`WsSeverity`, predates this spec and is owned by the WebSocket
  follow-up). No sibling fixed. One candidate is flagged rather than
  changed: Postman's optional header/cookie parameters are warnings
  (`check_unsent_params`); R10 does not set a severity for an optional input
  an output cannot carry, the choice was disclosed in Phase 2 and on the
  review-1 agenda (item 14), and raising it would block the OpenAI Postman
  collection (7 `openai-beta` parameters).
- **Docs and skills:** `docs/generation/validation.md` (severity table and
  the route section rewritten with a merge example), `README.md` and
  `definitions/README.md` (endpoint counts, Bitbucket table),
  `gen/README.md`, `skill/SKILL.md`, `.claude/skills/schematic-define/SKILL.md`
  (GitHub count, which already read 14), `.claude/skills/schematic-define/schematic/gen.md`.
- **Gates:** `just test` (schematic) exit 0 — L1 1776 passed, 9 skipped;
  fresh-output guard 4 passed (1 slow, 152 s). `just lint` (schematic) exit 0.
