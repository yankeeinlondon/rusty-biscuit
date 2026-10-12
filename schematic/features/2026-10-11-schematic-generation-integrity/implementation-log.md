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
packages:
  - schematic-gen
  - test-toolkit
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
| `duplicate-method-path` (same API, same `endpoint_uniqueness_key`) | ✓ | ✓ | ✓ | `DUPLICATE_METHOD_PATH_SEVERITY` = warning (deviation, below) |
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
| R13 `(method, normalized_path)` key | `::path_parameter_names_do_not_distinguish_routes` (`/items/{id}` = `/items/{name}`, GET ≠ DELETE, `{+repo}`; detection in every output) | pass; severity is warning (deviation above) |
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
