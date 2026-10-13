---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-modern-config/schematic/features/2026-10-12-schematic-endpoint-methods/spec.md"
plan: "schematic/features/2026-10-12-schematic-endpoint-methods/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - schematic/features/2026-10-12-schematic-endpoint-methods/endpoint-inventory.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - schematic/runtime/src/call.rs
    - schematic/runtime/src/client.rs
    - schematic/runtime/src/lib.rs
    - schematic/runtime/src/api_client.rs
    - schematic/runtime/src/deadline.rs
    - schematic/runtime/src/options.rs
    - schematic/runtime/tests/l1/call.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/support.rs
    - schematic/runtime/tests/l1/builder.rs
    - schematic/runtime/tests/l1/client_settings.rs
    - schematic/runtime/tests/l1/credential_resolution.rs
    - schematic/runtime/tests/l1/hooks.rs
    - schematic/runtime/tests/l1/raw.rs
    - schematic/runtime/tests/l1/redirects.rs
    - schematic/runtime/tests/l1/send.rs
    - schematic/runtime/tests/l1/timing.rs
    - schematic/runtime/tests/l1/whole_send_elapsed.rs
    - schematic/runtime/tests/fixtures/compile_fail/request_of_another_api.rs
    - schematic/runtime/tests/fixtures/compile_fail/request_of_another_api.stderr
    - schematic/runtime/tests/fixtures/compile_fail/request_of_another_api_through_the_api_struct.rs
    - schematic/runtime/tests/fixtures/compile_fail/request_of_another_api_through_the_api_struct.stderr
    - schematic/runtime/tests/fixtures/compile_fail/call_error_match_needs_a_wildcard.rs
    - schematic/runtime/tests/fixtures/compile_fail/call_error_match_needs_a_wildcard.stderr
    - schematic/runtime/tests/fixtures/compile_fail/request_change_after_full.rs
    - schematic/runtime/tests/fixtures/compile_fail/request_change_after_full.stderr
    - schematic/gen/src/lib.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/schema/src/lib.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - schematic/gen/src/codegen/endpoint_methods/docs.rs
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/names.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/mod.rs
    - schematic/gen/src/codegen/module_docs.rs
    - schematic/gen/src/codegen/request_structs/mod.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/lib.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/src/output/assemble/lib_rs.rs
    - schematic/gen/src/output/assemble/prelude.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/tests/l1/e2e_generation.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/http_client.rs
    - schematic/gen/tests/l1/partial_generation.rs
    - schematic/schema/src/anthropic.rs
    - schematic/schema/src/artificial_analysis.rs
    - schematic/schema/src/bitbucket.rs
    - schematic/schema/src/bitbucket/mod.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/bitbucket/responses.rs
    - schematic/schema/src/elevenlabs/mod.rs
    - schematic/schema/src/elevenlabs/requests.rs
    - schematic/schema/src/emqx/requests.rs
    - schematic/schema/src/eversolo.rs
    - schematic/schema/src/eversolo/mod.rs
    - schematic/schema/src/eversolo/requests.rs
    - schematic/schema/src/eversolo/responses.rs
    - schematic/schema/src/gitea.rs
    - schematic/schema/src/gitea/mod.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/gitea/responses.rs
    - schematic/schema/src/github.rs
    - schematic/schema/src/github/mod.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/github/responses.rs
    - schematic/schema/src/gitlab.rs
    - schematic/schema/src/gitlab/mod.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/schema/src/gitlab/responses.rs
    - schematic/schema/src/huggingface.rs
    - schematic/schema/src/huggingface/mod.rs
    - schematic/schema/src/huggingface/requests.rs
    - schematic/schema/src/huggingface/responses.rs
    - schematic/schema/src/lib.rs
    - schematic/schema/src/lmstudio.rs
    - schematic/schema/src/ollama.rs
    - schematic/schema/src/openai/mod.rs
    - schematic/schema/src/openai/requests.rs
    - schematic/schema/src/prelude.rs
    - schematic/schema/src/samsung_smart_tv.rs
    - schematic/schema/src/unfolded_circle_core_rest.rs
docs_updated_during_phase_3:
    - schematic/docs/generation/validation.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/schema/tests/artificial_analysis_client.rs
    - schematic/schema/tests/github_client.rs
    - schematic/schema/tests/huggingface_client.rs
    - schematic/schema/tests/non_json_endpoints.rs
    - schematic/schema/src/anthropic.rs
    - schematic/schema/src/artificial_analysis.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/elevenlabs/requests.rs
    - schematic/schema/src/emqx/requests.rs
    - schematic/schema/src/eversolo/requests.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/schema/src/huggingface/requests.rs
    - schematic/schema/src/lmstudio.rs
    - schematic/schema/src/ollama.rs
    - schematic/schema/src/openai/requests.rs
    - schematic/schema/src/samsung_smart_tv.rs
    - schematic/schema/src/unfolded_circle_core_rest.rs
docs_updated_during_phase_4:
    - schematic/docs/generation/validation.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
    - schematic-runtime
    - schematic-gen
    - schematic-schema
---

# Implementation Log for 2026-10-12-schematic-endpoint-methods (8 phases)

## Phase 1

Phase 1 is reconnaissance and the R8 "before" measurement. **No source file changed.** The only files written are this log, the plan's checkboxes and frontmatter, the spec's frontmatter, and a new sidecar, `endpoint-inventory.md`, in this feature directory.

### Starting state (read this before Phase 2)

- The runtime-envelope repair work is **uncommitted** in this worktree: 121 modified or untracked paths at the start of this phase, including the untracked `schematic/runtime/src/api_client.rs` and modified `schematic/schema/src/*`, `schematic/gen/src/*`, and `schematic/runtime/src/*`. `HEAD` is `7aae49e68`. Everything below, the baseline included, describes the **working tree**, not `HEAD`. Do not revert or stash those files.

### Task 1.1: Skills

- Loaded `schematic-define` and `os`. `rust-testing`, `rust`, and `rust-devops` concern writing code, tests, and CI, none of which happens in this phase; load them in Phase 2.
- OS notes for later phases: always use absolute `cd` targets (the `os` skill records `CDPATH` sending `cd <area>` into the main checkout). Phase 1 touched no paths or `#[cfg]` code. The generated call types are plain Rust with no OS-specific surface, and the R8 numbers are macOS-only by design (ruling 11).

### Task 1.2: Runtime facts

| Plan assumption | Finding | Ruling impact |
| --- | --- | --- |
| `TransportError` carries request metadata | Yes: `pub request: Box<RequestMeta>`, plus `kind` and `elapsed` (`runtime/src/error.rs`). | Ruling 1 needs no `TransportError` change. `request_meta()` returns `&*self.request` for `Transport` and `&meta.request` for `Api { meta, .. }`. |
| Request types can be `Clone` | Every REST request type already derives `Debug, Clone, Default, Serialize, Deserialize` (`gen/src/codegen/request_structs/derives.rs`, `form.rs`; `shared::FormFile` is `Clone`). Only WebSocket helpers lack `Clone`. `EndpointSpec` itself has no `Clone` bound (`Send + 'static`). | Ruling 5 holds. Put `R: Clone` on `build()` and on `Clone for Call`, not on `EndpointSpec`. |
| `Client<A>` is cheap to borrow | Yes. `Client<A>` is `Arc<Settings>` plus `Arc<OnceLock<..>>`, and hooks are `Send + Sync`. The generated API struct is a tuple struct with a **private** field (`pub struct GitHub(Client<GitHubApi>)`), so an endpoint method must be emitted in the API's module or a child module. | Ruling 4: a boxed `IntoFuture` borrowing `&'a Client<A>` needs `Client<A>: Sync` and a `Send` future. Nothing asserts this today; add a compile-time `Send` assertion for `Call`'s future in Phase 2. |
| ABI is `2` in both places | Yes: `runtime/src/lib.rs:96` `ABI`, and `gen/src/lib.rs:93` `RUNTIME_ABI`, guarded by the `runtime_abi_tests` module, which parses the runtime source. | Task 2.4 as planned. |
| `send` shape | `Client::send(&self, request: R, options: &CallOptions) -> impl Future` boxes `send_unboxed`. `api_client!` forwards `send`, `send_raw`, and `check_link`. `CallOptions::anonymous()` is an associated **constructor**, while `credential(..)` and `deadline(&Deadline)` are `self`-consuming methods. | Ruling 2: the call type owns its `CallOptions`, and its future body is `async move { client.send_unboxed(request, &options).await }`. The call type's `anonymous()` sets `CallAuth::Anonymous` through a crate-internal setter, because the public `CallOptions::anonymous()` is a constructor. |
| `.build()` error type | `InvalidArgument` today surfaces as `TransportErrorKind::InvalidArgument(..)`. `EndpointSpec::into_parts(self)` consumes the request. | Ruling 5: `.build()` calls `self.request.clone().into_parts()` and maps the `Err` to `InvalidArgument`. |

Call sites of `send(..)` or `CallOptions` (lines matching `.send(` or `CallOptions`, so they include other `send`s): runtime src 19, runtime tests 169, gen src 32, gen tests 51, schema src 215 (mostly generated doc examples), schema tests 59, `schematic/docs` 36, schematic READMEs 18, `.claude/skills` 67.

Surprises, recorded rather than acted on:

1. `gen/src/codegen/api_struct/{constructors,fields,request_method,variant}.rs` and the whole `gen/src/codegen/client/` directory are **not declared as modules** (`codegen/mod.rs` declares `api_struct`, `module_docs`, `request_structs`, `shared`, `traits`; `api_struct/mod.rs` declares `auth`, `helpers`, `tests`). They are dead files left from the pre-envelope generator. They must not seed the reserved-name list (for example, `with_client` is not a real method). Deleting them is out of this phase's scope. Raise it at review.
2. Response types reach each generated module by `pub use schematic_definitions::<api>::*;`. A local item shadows a glob import **silently**, so a generated call type that shares a name with a definitions type would hide that type with no compile error. Phase 3's name resolver must check call-type names against the glob-imported definitions names, not only against generated items. No shipped name collides today (see Task 1.3).

### Task 1.3: Generator-input inventory

The full per-endpoint table (id, method name, HTTP method, required `new` arguments in order, optional setters, call-type name) is in [`endpoint-inventory.md`](./endpoint-inventory.md). It was computed from the generated `schematic/schema/src`, whose `EndpointSpec` impls count 547, matching the parse.

| API (module) | Endpoints |
| --- | --- |
| Anthropic | 4 |
| ArtificialAnalysisData / ArtificialAnalysisCritPt (`artificial_analysis`) | 6 / 1 |
| Bitbucket | 15 |
| ElevenLabs | 42 |
| EmqxBasic / EmqxBearer (`emqx`) | 36 / 38 |
| Eversolo | 24 |
| Gitea | 16 |
| GitHub | 16 |
| GitLab | 19 |
| HuggingFaceHub | 26 |
| LmStudio | 6 |
| OllamaNative / OllamaOpenAI (`ollama`) | 11 / 4 |
| OpenAI | 265 |
| SamsungSmartTv | 7 |
| UnfoldedCircleCoreRest | 11 |
| **18 APIs** | **547** (410 optional setters) |

Collisions under D2 and ruling 7, checked against the reserved call-type methods (`full`, `build`, `anonymous`, `credential`, `deadline`, `pages`, `fetch_page`, `collect_all`) and the real API-struct inherent methods (`new`, `definition`, `basic_auth`, and every `api_client!` method: `base_url`, `timeout`, `connect_timeout`, `proxy`, `add_root_certificate`, `danger_accept_invalid_certificates`, `user_agent`, `default_header`, `transport`, `credentials`, `api_key`, `env_order`, `cross_origin`, `pre_response_json`, `mutate_response`, `build`, `send`, `send_raw`, `check_link`):

- setter vs shared call method (`_param`): **none**
- endpoint method vs API-struct method (`_endpoint`): **none**
- endpoint method name is a Rust keyword: **none**
- two endpoints with one snake-case name in one API (ruling 7): **none**

**The expected rename list for the shipped definitions is empty.** Task 4.3 pins an empty list. The D2 rules are therefore exercised only by the Phase 3 fixture (Task 3.7).

Findings the rules do not cover, for Phase 3 and the author:

- **Call-type naming across APIs that share a module.** EMQX (Basic and Bearer) and Ollama (Native and OpenAI) each put two APIs in one module, with the same endpoint ids, separated only by `request_suffix` (`ListClientsBasicRequest` and `ListClientsBearerRequest`). A plain `{EndpointId}Call` name collides 36 times in `emqx` and twice in `ollama` (`EmbeddingsCall`, `ListModelsCall`). **Ruling added (13): the call type's name is the request type's name with its trailing `Request` replaced by `Call`** (`ListClientsBasicCall`, `ListModelsNativeCall`, `GetSystemInfoCoreRestCall`), and the request name itself is unique per module today. Under that rule no call-type name collides with a generated item or with a glob-imported definitions type. OpenAI has many `*ToolCall` types, but none equals `{Endpoint}Call`.
- **Endpoints with no `new()`.** 74 request types have no `new()` and are built with `Default` (for example, `ListModelsRequest`). Their endpoint method takes no arguments and starts from `Default::default()`. Another 47 have a zero-argument `new()`. Either way, 121 endpoint methods take no arguments.
- **Ugly but legal names from OpenAI operation ids.** The imported OpenAI spec has some operation ids without casing, so `to_snake_case` produces `compactconversation`, `deleteorganizationspendlimit`, and `createan_a_p_ikeyforaserviceaccount` (from `CreateanAPIkeyforaserviceaccount`). These are valid and do not collide. Fixing them means changing the import (a definitions change, which is a non-goal). Raise it at review.
- Maximum required arguments: 4 (`Bitbucket.GetSource`).

### Task 1.4: R8 baseline

**Date:** 2026-10-13. **Host:** the development Mac (Apple M4 Max, 16 cores, 128 GiB), macOS (Darwin 27.2.0). **Toolchain:** rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1. **Tree:** `7aae49e68` plus the uncommitted runtime-envelope repair (see Starting state). **Profile:** `dev` (debug). `schematic/schema` is excluded from the workspace, so it builds with Cargo's default `dev` profile, not the workspace's reduced-debuginfo profile.

**Method.** The host's `~/.cargo/config.toml` sets `rustc-wrapper = "kache"`, which would turn a "clean" build into a cache restore. Every measurement therefore disables the wrapper and uses a fresh target directory outside the repository. This counts as "delete the build output" without touching the shared `schematic/schema/target`.

```sh
# from the repository root
# clean (run twice, each into a new empty directory)
/usr/bin/time -p env RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= CARGO_TARGET_DIR=/tmp/r8-baseline/target \
  cargo build --color=never --manifest-path schematic/schema/Cargo.toml --timings
# incremental (twice): touch one source file, rebuild into the same target dir
touch schematic/schema/src/lib.rs
/usr/bin/time -p env RUSTC_WRAPPER= CARGO_BUILD_RUSTC_WRAPPER= CARGO_TARGET_DIR=/tmp/r8-baseline/target \
  cargo build --color=never --manifest-path schematic/schema/Cargo.toml
```

Per-crate times come from the `UNIT_DATA` table in `cargo-timing.html`.

| Measurement | Run 1 | Run 2 | Mean |
| --- | --- | --- | --- |
| Clean build, whole command (wall) | 41.83 s | 45.00 s | **43.4 s** |
| …of which the `schematic-schema` unit | 6.08 s | 6.45 s | **6.3 s** |
| …of which the `schematic-definitions` unit (critical path, not changed by this feature) | 28.52 s | 31.36 s | 29.9 s |
| Incremental rebuild after `touch src/lib.rs` (wall) | 1.74 s | 1.77 s | **1.76 s** |

The two clean runs differ by 7.6%, inside ruling 11's 10%, so no third run.

**OpenAI separately (265 endpoints), measured by compiling the module in isolation.** A throwaway probe crate at `/tmp/r8-openai-probe` copies `schema/src/openai/` and `schema/src/shared.rs`, renames the package to `schematic-schema-openai-probe`, points its path dependencies at the worktree, and gets a `lib.rs` holding only the ABI assertion, `pub use schematic_runtime;`, `pub mod shared;`, and `pub mod openai;`. OpenAI depends on nothing else in the crate (`crate::shared` only). It was built into the warm `/tmp/r8-baseline/target2`, so its timing is its own unit only:

| OpenAI probe | Run 1 | Run 2 |
| --- | --- | --- |
| Unit time, from scratch (no incremental state) | 3.40 s | 3.92 s |
| Incremental rebuild after `touch src/lib.rs` (wall) | 1.06 s | — |

OpenAI is therefore about 60% of the `schematic-schema` unit.

**Generated crate size (`schematic/schema/src`).**

| Scope | Files | Lines | Bytes |
| --- | --- | --- | --- |
| Whole `src` | 30 | 41,610 | 1,516,000 |
| REST only (excluding `*_ws.rs` and `ws_shared.rs`) | — | 37,494 | 1,342,153 |
| `openai/` | 3 | 19,703 | 716,506 |
| Debug `libschematic_schema.rlib` | — | — | 43 MB |
| Debug OpenAI probe rlib | — | — | 19 MB |

**Which number D3's 25% applies to (for Phase 4 and the author).** D3 says "a clean debug `cargo build -p schematic-schema`". Read literally, that is the whole command's wall time (43.4 s), so a 25% increase would be about 10.9 s. But this feature changes only the `schematic-schema` unit (6.3 s), and the whole command is dominated by dependencies the feature never touches. On the unit alone, 25% is about 1.6 s. The two readings can reach different D3 branches. Phase 4 must report both, apply the literal (whole-command) reading as the branch decision, and say so explicitly if the unit-only reading would have chosen another branch. That case is the "measurement result that the D3 rule does not cover" the spec asks a reviewer to flag. Ruling 14 records this.

### Task 1.5: Consumers

`Cargo.toml` files outside `schematic/` that depend on `schematic-schema` are `biscuit-speaks/lib`, `sniff/lib`, `homelab/lib`, `homelab/eversolo-integration`, `homelab/arcam-amp-integration`, `homelab/sony-receiver-integration`, and `homelab/unfolded-integration-helper`. None depends on `schematic-runtime` directly.

Files that use generated REST sends (`CallOptions` count in parentheses):

- `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs` (2)
- `homelab/lib/src/eversolo.rs` (2), `homelab/lib/src/samsung_tv.rs` (2), `homelab/lib/src/rest.rs` (generic helper, check for R6)
- `sniff/lib/src/remote/{github,gitlab,gitea,bitbucket}.rs` (11, 7, 9, 9), `sniff/lib/src/remote/envelope.rs` (generic helper, check for R6)

The homelab `*-integration*` crates show `.send(` matches, but those appear to be WebSocket and channel sends. Phase 6 should confirm per file.

**`model-citizen` is not a consumer of generated clients.** It depends only on `schematic-definitions` (`model-citizen/lib/Cargo.toml:39`) and uses its response types (`ShowModelResponse` and similar) in `scanner/ollama.rs` and `scanner/lmstudio.rs`, without `schematic-schema` or `send`. Task 6.4 should record "none".

### Rulings adopted

Rulings 1–12 in the plan are adopted as written, with the Task 1.2 confirmations above. Their recorded alternatives:

1. Struct variant `Api { error, meta }`. *Alternative:* add `meta` to `ApiError`, which changes `Response`'s outcome type. Rejected in the plan.
2. `send(request)` with no options. *Alternative:* keep `send(request, &CallOptions)` beside a new entry point, which leaves two shapes.
3. Separate `FullCall`. *Alternative:* a type-state parameter on `Call`, which complicates every signature.
4. Boxed future. *Alternative:* an `impl Future` type alias (unstable `type_alias_impl_trait`).
5. `.build()` encodes and discards. *Alternative:* cache `RequestParts`, which needs `RequestParts: Clone` and a second send path.
6. One reserved-name const with a drift test. *Alternative:* derive names from the runtime by reflection, which is not possible at generation time.
7. Duplicate snake name is a hard error. *Alternative:* numeric suffixes, which are unstable across definition edits. No shipped definition triggers it.
8. Setters delegate to `with_*`.
9. Methods for every REST endpoint.
10. Shared-helper move decided in Task 3.5.
11. One host, debug, two runs per number. *Open question for the author:* rust-analyzer latency and release-profile builds were not measured.
12. Baseline taken here.

New in this phase:

13. **Call-type name = request type name with `Request` replaced by `Call`** (Task 1.3). *Alternative:* `{EndpointId}Call` plus collision renames, which would rename 38 types in `emqx` and `ollama`. Rejected because the request name already carries the per-API suffix, so following it keeps the two names visibly paired. The Phase 3 resolver still checks the result against generated items and glob-imported definitions names, because of the silent-shadowing hazard.
14. **D3 branch decision uses the whole-command clean wall time**, the literal reading of D3. Phase 4 also reports the `schematic-schema` unit time and flags it if the two readings disagree (Task 1.4).

### Requirement-to-test mapping

Phase 1 changes no behavior, so it adds no tests. The inventory script and the probe crate live outside the repository (`/tmp/r8-scripts`, `/tmp/r8-openai-probe`); the results above are their record. The gates for this phase are `just test` and `just lint` in the `schematic` area, run to confirm the untouched baseline is green (results below).

### Gates run

- `just test` in `schematic/`: 1882 passed, 12 skipped. The slow suites also passed: `schematic-gen::l1 fresh_output_compile::slow_*` (6 of 6) and `schematic-runtime::l1 compile_fail::slow_compile_fail_cases_report_their_intended_diagnostic` (1 of 1).
- `just lint` in `schematic/`: exit 0 (schematic-gen, schematic-definitions, schematic-oauth, schematic-runtime, and the rest of the area).
- No pre-existing failures. Nothing skipped beyond nextest's 12 tier-skipped tests.
- Cross-OS runs: not needed. Phase 1 changed no code.

### Checkpoint 1

- Rulings are recorded (1–12 adopted, 13–14 added), the baseline numbers exist, and the expected rename list is written down (empty).
- No source file changed: only this log, `plan.md`, the spec's frontmatter, and the new `endpoint-inventory.md`.
- Status: Phase 1 implementation complete, ready for review.

## Phase 2

Phase 2 adds the runtime call types: `Call`, `FullCall`, `CallError`, and `CallFuture` in a new `schematic/runtime/src/call.rs`. `Client::send(request)` now returns a `Call`, `api_client!` forwards it, and the ABI is now 3. Skills loaded: `rust-testing` conventions (L1 placement through `tests/l1/main.rs`, no tier markers in names) and `os` from Phase 1. No `#[cfg]` or path code changed.

### What was built

- **`Call<'a, A, R>`** holds `&'a Client<A>`, the request, and a `CallOptions`. It has these methods:
    - `anonymous()`, `credential(..)`, and `deadline(..)`: the last credential mode set wins.
    - `map_request(f)`: replaces the request and keeps the settings. Phase 3's generated setters use it.
    - `full()` returns a `FullCall`.
    - `build()` (needs `R: Clone`) encodes a clone of the request with `into_parts()` and discards the result (ruling 5). It returns `Result<Self, InvalidArgument>`.

  `IntoFuture` boxes `send_unboxed` into `CallFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>`. It maps `Outcome::Failure` to `CallError::Api` and keeps the metadata, and maps a `TransportError` to `CallError::Transport`. The `dyn … + Send` coercion compiles for every `R`, so it is the compile-time `Send` assertion Phase 1 asked for.
- **`FullCall<'a, A, R>`** has the same per-call methods and `build()`, and awaits to `Result<Response<..>, TransportError>`. It has no `map_request` and no `full`, so a setter after `.full()` does not compile (ruling 3).
- `Clone` and `Debug` are written by hand with method- and impl-level bounds: `Clone` needs only `R: Clone` and `Debug` only `R: Debug`, and `IntoFuture` needs neither. Both types are `#[must_use]`.
- **`CallError<E>`** is `#[non_exhaustive]` with `Transport(TransportError)` and `Api { error: ApiError<E>, meta: Box<ResponseMeta> }`.
    - `request_meta()` serves both variants.
    - It implements `From<TransportError>`, `Display`, and `Error` (for `E: Debug + 'static`), and derives `Clone` and `PartialEq`.
    - No `api()` or `transport()` accessors were added (Task 2.1 defers them to Phase 6, and only if a consumer needs them).
- **`Client::send(request) -> Call<'_, A, R>`** (ruling 2). `send_unboxed` is now `pub(crate)` and is the body of both futures. `send_raw` and `check_link` are unchanged, and `send_raw` still takes `&CallOptions`.
- **`api_client!`** forwards `send(request) -> $crate::Call<'_, $tag, R>`.
- **ABI**: `schematic_runtime::ABI` and `schematic_gen::RUNTIME_ABI` are both 3. Neither doc comment lists changes, so the comments are unchanged.

### Departures from the plan (recorded here, spec untouched)

1. **`CallError::Api` stores `meta: Box<ResponseMeta>`, not `meta: ResponseMeta`.** `ResponseMeta` holds two `HeaderMap`s and a `RequestMeta`. Unboxed, every `Result<T, CallError<E>>` would be large, and consumers' functions returning it would trip clippy's `result_large_err`. This follows `TransportError`, which already boxes its `request`. Field access (`meta.status`) reads the same through `Box`.
2. **`Display` does not repeat the inner error; `source()` returns it.** The Rust error-handling guideline is to either show the source's text or return it as `source`, never both. Otherwise `anyhow`'s `{:#}` prints the same text twice. The two forms:
    - `Transport` prints `"{endpoint_id} call failed before a response arrived"`.
    - `Api` prints `"{endpoint_id} call failed: {ResponseMeta}"`, for example `call call failed: 401 Unauthorized from GET http://…/item`.

   Both are credential-free, because they are built only from the endpoint id and the redacted `ResponseMeta` `Display`. **Author:** if the preference is a transparent `Display` (the inner text) with `source()` forwarding the inner error's own source, it is a two-line change.
3. **`Call::map_request` is public** (plan: unspecified). Phase 3's generated setters need some public way to change a call's request from another crate. One documented method is simpler than a `#[doc(hidden)]` back door, and it is also usable from generic (R6) code.
4. **`schematic/schema/src/lib.rs` was regenerated in this phase**, not Phase 4. `schematic-gen`'s `slow_real_apis_compile_offline` byte-compares fresh output with the committed tree, so `just test` failed once the generator emitted ABI 3. I ran the real generator: `cargo run -p schematic-gen -- generate --api all --output schema/src --openapi-out openapi --postman-out postman`, which is `just generate` without its desktop notification. The only change is the ABI marker and its assertion message in `lib.rs`; OpenAPI and Postman outputs were byte-identical. The schema library therefore builds against the new runtime. Phase 4 still regenerates for the endpoint methods.
5. **`schematic-gen`'s tests were migrated in this phase.** They call `send(request, &CallOptions)` inside the generated test programs in `fresh_output_compile.rs`, and `e2e_generation.rs` pinned the literal `ABI: u32 = 2`. Both are part of the area's `just test`. Generator *source* is unchanged apart from `RUNTIME_ABI` and the `//!` example in `gen/src/lib.rs`. The generated docs strings in `module_docs.rs` and `assemble/{lib_rs,prelude}.rs` still show the old shape; that is Task 3.6.

### Known breakage carried into later phases

- **Consumers do not compile until Phase 6.** `cargo check` shows:
    - `biscuit-speaks` (lib): 4 errors.
    - `homelab` (lib): 4 errors.
    - `sniff` with `--all-features`: 78 errors. The default features compile, because the remote clients are feature-gated.

  Every error is a `send(request, &CallOptions)` call or a use of its `Response` result. Removing the options parameter (ruling 2) makes this unavoidable. The plan migrates them to endpoint methods in Phase 6. Until then a workspace-wide build or a push hook that compiles these packages fails.
- **`schematic-schema`'s own tests** (`github_client`, `huggingface_client`, `artificial_analysis_client`, `non_json_endpoints`) do not compile, for the same reason. Task 4.2 migrates them.
- **`gen/tests/l1/http_client.rs`** asserts that generated API code contains no `.send(`. Phase 3's endpoint methods will call `self.0.send(..)`, so Phase 3 must narrow that assertion.

### Task 2.5: Migrating the runtime tests

The 169 call sites were rewritten mechanically: `send(X, &CallOptions::new()<chain>)` became `send(X)<chain>.full()`, and `&CallOptions::anonymous()` became `.anonymous()`. This keeps each existing test's envelope assertions unchanged. The scripts are outside the repository (`/tmp/p2-scripts`).

Hand edits:

- `credential_resolution.rs`: the `send` helper now returns a `FullCall`, so each caller chains its own settings. The precedence loop holds an `Option<Credential>` instead of a `CallOptions`.
- `send_futures_stay_small_enough_to_join_many` now measures `Call::into_future()` and `FullCall::into_future()`. After the mechanical rewrite it would have measured the `FullCall` struct, not a future.
- `support::Call` gained a hand-written `Clone` and `Debug`, because derives would bound the codec parameters.

The tests for credentials precedence, anonymous mode, deadlines, and error evidence keep their own files and assertions.

The two `request_of_another_api*` compile-fail fixtures use `send(GiteaListRepos).await`. Their regenerated `.stderr` still leads with the intended `E0271` "`Api == Github`" mismatch; the compiler now also reports that the mismatched `Call` is not awaitable.

### Requirement-to-test mapping

The boundary is the runtime crate's public API, called from a separate test crate (`schematic-runtime::l1`) against wiremock or a raw-socket mock. That is fidelity F2, the same as the existing runtime suites, at tier L1. The compile-time contracts use trybuild.

| Behavior | Test (`schematic/runtime/tests/…`) |
| --- | --- |
| `.await` returns the data on 2xx | `l1/call.rs::awaiting_a_call_returns_the_data_of_a_2xx_reply` |
| Declared error and malformed 2xx become `CallError::Api` with status, request id, body, and `request_meta()` | `call::a_declared_error_or_an_undecodable_body_is_an_api_error_with_its_response_kept` |
| Interrupted body becomes `CallError::Api` (`BodyInterrupted`) with the partial bytes | `call::a_body_cut_off_after_the_head_is_an_api_error_with_the_bytes_that_arrived` |
| Connection failure becomes `CallError::Transport` (`Connect`) with `request_meta()` | `call::a_connection_failure_is_a_transport_error_with_the_request_kept` |
| `.full()` returns the envelope (status, headers, request id) for success and error replies, and `TransportError` when no head arrives | `call::full_returns_the_envelope_for_a_success_and_for_an_error_reply` |
| `.build()` refuses an invalid argument on `Call` and `FullCall` with **zero** requests; the same check runs at `.await`; a valid `build()` sends nothing | `call::build_refuses_an_invalid_argument_before_any_request_is_sent` |
| A built call (and a built `FullCall`) awaited twice makes two requests with two results, and a clone keeps per-call settings | `call::a_built_call_sent_twice_makes_two_requests_and_two_results` |
| `anonymous` and `credential` on `Call` and `FullCall`, with the last mode winning, checked against the header actually sent and `meta.request.auth` | `call::per_call_settings_behave_on_call_and_full_alike_and_the_last_credential_mode_wins` |
| An expired deadline is a transport error and sends nothing; a live deadline counts completed sends through both awaits | `call::an_expired_deadline_is_a_transport_error_on_call_and_full_and_nothing_is_sent` |
| `map_request` replaces the request and keeps the settings | `call::map_request_replaces_the_request_and_keeps_the_settings` |
| `Call` and `FullCall` are `Send` before the await | `call::a_call_can_move_to_another_thread_before_it_is_awaited` |
| `Display` is credential-free for both variants, `source()` downcasts to the inner `ApiError` or `TransportError`, and `request_meta()` is redacted | `call::call_error_display_names_the_call_without_secrets_and_its_source_is_the_inner_error` |
| A downstream wildcard match compiles; `From<TransportError>` | `call::call_error_is_matched_with_a_wildcard_arm_and_converts_from_a_transport_error` |
| A downstream match **without** a wildcard fails (`#[non_exhaustive]`) | `fixtures/compile_fail/call_error_match_needs_a_wildcard.rs` (E0004) |
| After `.full()` the request cannot change and `.full()` cannot repeat; per-call settings still chain | `fixtures/compile_fail/request_change_after_full.rs` (E0599 ×2) |
| A request of another API is refused through `Client::send` and `api_client!`'s `send` | existing `request_of_another_api*.rs`, migrated |
| The boxed futures stay one pointer | `l1/send.rs::send_futures_stay_small_enough_to_join_many` (now covers `Call` and `FullCall`) |
| Generated `lib.rs` records ABI 3 | `schematic-gen::l1 e2e_generation::generated_files_exist_and_have_expected_structure`, and `runtime_abi_tests` (gen and runtime constants agree) |
| Committed schema tree equals fresh generator output | `schematic-gen::l1 fresh_output_compile::slow_real_apis_compile_offline` |

The Input Robustness Matrix does not apply: no parser or configuration reader changed.

### Gates run

- `just test` in `schematic/`: 1895 passed and 12 skipped (tier-skipped, as in Phase 1). `test-fresh-output` passed 6 of 6, and `test-compile-fail` passed 1 of 1, covering all 9 fixtures.
- `just lint` in `schematic/`: exit 0.
- `cargo build --manifest-path schematic/schema/Cargo.toml`: builds.
- Not run, and expected to fail until later phases: the schema crate's tests and the consumer packages (see "Known breakage").
- Cross-OS: not run. The change adds no `#[cfg]`, path, or process code. The new tests use the same loopback mocks and `closed_port_url()` as the existing runtime suites, which already run on every CI OS. A refused loopback connect on Windows takes longer than on macOS, but stays well inside the 5 s budget per test, as in the existing suites.

### Checkpoint 2

- `just test` and `just lint` pass in the `schematic` area. That covers more than the plan's checkpoint asked: the generator tests were migrated rather than left failing.
- Status: Phase 2 implementation complete, ready for review.

## Phase 3

Phase 3 adds the generator side: every REST endpoint now gets an inherent method on its API struct and a generated call type. The work is in a new `schematic/gen/src/codegen/endpoint_methods/` module, wired into `output/assemble`. Skills used: `rust-testing` conventions (L1 placement, no tier markers, repository reads through `include_str!` and `CARGO_MANIFEST_DIR` joins) and `os` from Phase 1. No `#[cfg]`, path-comparison, or process code changed.

### What was built

- **`endpoint_methods/names.rs`: the name resolver (Task 3.1).** `resolve_names(api)` returns, for each endpoint:
    - the method name;
    - the request and call type names;
    - each setter, with its field, location, and parameter;
    - the rename list (`Rename`: API, endpoint, target, from, to, reason);
    - the remaining conflicts (`NameConflict`).

  `endpoint_method_renames(apis)` returns the flat list that Task 4.3 pins. The rules:
    - The method name is the endpoint id put through `sanitize_rust_field_ident`, the request-field sanitizer, so a keyword becomes `type_` and is listed as `NotAnIdentifier`.
    - A method named like an API-struct method gets the `_endpoint` suffix.
    - A setter is named after its request field, so it matches `with_<field>`. A setter named like a call-type method gets the `_param` suffix.
    - A name that two endpoints still share, or that two setters of one call type still share, is a conflict.
- **Reserved names (ruling 6), in five consts:**

  | Const | Contents |
  | --- | --- |
  | `SHARED_CALL_METHODS` | `anonymous`, `credential`, `deadline`, `full`, `build` |
  | `PLANNED_CALL_METHODS` | `pages`, `fetch_page`, `collect_all` |
  | `CALL_TYPE_TRAIT_METHODS` | `clone`, `clone_from`, `fmt`, `into_future` |
  | `API_CLIENT_METHODS` | the 19 `api_client!` methods |
  | `API_STRUCT_METHODS` / `API_STRUCT_TRAIT_METHODS` | `new`, `definition`, `basic_auth` / `clone`, `clone_from`, `fmt`, `default` |

  Drift tests prove three of them against the code:
    - `API_CLIENT_METHODS` equals the `pub fn`s of the `api_client!` macro, read from `runtime/src/api_client.rs` with `include_str!`.
    - `API_STRUCT_METHODS` equals the inherent `fn`s that `generate_api_struct` emits for a Basic-auth API.
    - Every `SHARED_CALL_METHODS` entry is a method of the runtime `Call`, and of `FullCall` (except `full`), parsed from `runtime/src/call.rs`.

  Trait-method names are reserved because an inherent `clone(self, v)` or `default(&self)` would shadow the trait method in method-call syntax.
- **Diagnostics (Task 3.2).**
    - New `Feature::EndpointMethodRename` (`endpoint-method-rename`). `validate_rust_client` reports each rename as a **warning**, with one line per rename naming the API, endpoint, old and new name, and reason.
    - Conflicts are `generated-name-collision` **errors**.
    - `check_group_type_names` now also claims each call-type name, so these cases are errors too:
        - two APIs of one module whose call types collide;
        - a body or response type named like a call type.
- **`endpoint_methods/docs.rs`: the rustdoc (Task 3.3, R7).** The method doc has:
    - the description, verbatim, multi-line, and omitted with no blank line when it is empty;
    - a `` `METHOD /path` `` line;
    - a "Returns a [`XCall`]" sentence;
    - `## Arguments`, listing the name, type, and role of each required argument, and omitted when there are none;
    - `## Optional values`, linking each setter to `XCall::setter` with its wire name, location, type, and description collapsed to one line, and omitted when there are none.

  Setter docs reuse the same summary. There is no H1, and no format-string narration.
- **`endpoint_methods/mod.rs`: emission (Task 3.4).** `generate_endpoint_methods(api)` emits each call type and **one** `impl {Api} { … }` block with every endpoint method. The call type is:

  ```rust
  #[derive(Clone, Debug)]
  #[must_use = "a call sends nothing until it is awaited"]
  pub struct XCall<'a>(schematic_runtime::Call<'a, Tag, XRequest>);
  ```

    - Setters are `Self(self.0.map_request(|request| request.with_field(value)))`, taking the `with_*` argument type (ruling 8).
    - Forwards for `anonymous`, `credential`, `deadline`, `full` (which returns the runtime `FullCall`, per ruling 3), and `build`.
    - `IntoFuture` uses `Output = <Call<..> as IntoFuture>::Output` and `IntoFuture = schematic_runtime::CallFuture<'a, Self::Output>`.
    - The endpoint method is `XCall(self.0.send(XRequest::new(args…)))`, or `XRequest::default()` when there are no required arguments. Its arguments are the path parameters in path order, then `body`, which is `new`'s order (R1).

  No WebSocket code is touched. `request_body_type_name` was extracted from `generate_request_struct_with_options` so the request struct and the endpoint method agree on the body type.
- **Assembly (Task 3.6).** `api_requests` appends the endpoint methods after the requests, so in a split module they land in `requests.rs`. That file now imports the API struct beside its tag (`use super::{GitHub, GitHubApi};`). A child module may reach the struct's private `.0`.
- **Docs strings (Task 3.6).**
    - `module_docs.rs`'s example, `lib_rs.rs`'s quick start, and `prelude.rs` now show endpoint methods, through a new `example_invocation` helper.
    - The `lib.rs` "Results" section describes `CallError` and `.full()`.
    - The prelude also re-exports `Call`, `CallError`, and `FullCall`, but not the call types, following the request-type pattern.
    - The `schematic-gen` crate docs (`gen/src/lib.rs`) show the call type and the method.

### Decisions and departures (spec untouched)

1. **Task 3.5, shared helpers: not touched.** The call types use only `schematic_runtime` items. `shared.rs` is byte-identical after regeneration, so `encode_cookie_value`, `combine_cookie_headers`, and `FormFile` stay where they are.
2. **Method names use `schematic_define`'s `to_snake_case` (through `sanitize_rust_field_ident`), not `schematic-gen`'s.** Phase 1's inventory used the generator's `to_snake_case`. D2 says to reuse the request-field sanitizer, and that sanitizer is built on the definitions crate's snake case, which keeps acronyms together. For ordinary ids (`ListReleases` → `list_releases`) the two agree. They differ only on the uncased OpenAI ids Phase 1 flagged, for example `CreateanAPIkeyforaserviceaccount` → `createan_apikeyforaserviceaccount`, not `createan_a_p_ikeyforaserviceaccount`. A re-run of `validate` over the shipped definitions reports **zero** renames and **zero** collisions, so the expected rename list is still empty.
3. **Trait-method names are reserved too** (`clone`, `clone_from`, `fmt`, `default` on the API struct; `clone`, `clone_from`, `fmt`, `into_future` on call types). Ruling 6 named only inherent methods. Without these, an endpoint or parameter named `default` or `clone` would compile but shadow the trait method.
4. **A keyword endpoint id is listed as a rename** (`NotAnIdentifier`). A keyword *setter* is not listed, because the setter takes the request field's existing name (`type_`, matching `with_type_`), so nothing a caller already uses changes.
5. **`schematic/schema/src` was regenerated in this phase** with `cargo run -p schematic-gen -- generate --api all --output schema/src --openapi-out openapi --postman-out postman` (`just generate` without its notification). `slow_real_apis_compile_offline` compares fresh output with the committed tree byte for byte, so `just test` would fail otherwise. OpenAPI and Postman outputs are byte-identical.
6. **Six modules crossed the generator's 2,000-line split threshold and became directories:** `bitbucket`, `eversolo`, `gitea`, `github`, `gitlab`, and `huggingface`. For example, `github.rs` became `github/{mod,requests,responses}.rs`. Public paths do not change, because `mod.rs` re-exports `requests::*` and `responses::*`, so `schematic_schema::github::GitHub` and `…::github::ListReleasesRequest` still resolve. Task 4.1's churn review should expect this layout change.
7. **Generated size roughly doubled.** This is for Phase 4's R8 measurement; Phase 3 did not time any builds:

   | Scope | Phase 1 baseline | Now |
   | --- | --- | --- |
   | Whole `schema/src` | 41,610 lines, 1,516,000 bytes, 30 files | 78,634 lines, 3,188,410 bytes, 42 files |
   | REST only | 37,494 lines, 1,342,153 bytes | 74,518 lines, 3,014,563 bytes |
   | `openai/` | 19,703 lines, 716,506 bytes | 38,438 lines, 1,598,362 bytes |

   Most of the growth is rustdoc, plus five forwards and an `IntoFuture` impl per endpoint. A debug `cargo build --manifest-path schematic/schema/Cargo.toml` of the regenerated crate succeeds, and `cargo clippy --manifest-path schematic/schema/Cargo.toml --lib -- -D warnings` is clean. The crate's own tests still do not compile; that is the known breakage Task 4.2 fixes.
8. **No validation-time check against glob-imported definitions names.** The generator cannot see every `schematic-definitions` type, so Phase 1's silent-shadowing hazard is guarded by an L1 test over the shipped definitions source instead (`no_shipped_call_type_shares_a_name_with_a_definitions_type`). A body or response type named like a call type is still a validation error.

### Test changes required by the new surface

These old assertions forbade what this phase adds. Each now asserts the new contract.

- `gen/tests/l1/http_client.rs::module_has_no_send_path_of_its_own` forbade `.send(` and `fn get_items`. It now requires that the only `.send(` calls are the two endpoint methods' `self.0.send(..)`, and it pins both methods' signatures and bodies.
- `gen/tests/l1/e2e_generation.rs` previously used `assert_no_send_methods`. It is now `assert_endpoint_methods`: no `async fn` or `fn request`, and each named endpoint method returns its call type.
- `fresh_output_compile.rs::RUNTIME_DEFINITIONS` forbade `fn variant`, meaning the old API-struct method. OpenAI now has a legitimate `variant` setter, so the entry is narrowed to `fn variant(&self`. `generated_rest_code_…` reads `github/requests.rs` (the split layout) and asserts that `list_releases(&self, owner, repo) -> ListReleasesCall<'_>` exists.
- `output::tests::split_module_public_api_is_accessible` expects `use super::{LargeApi, LargeApiApi};`.
- `validation::tests::group_members_whose_emitted_names_collide_name_both_apis` now expects two collisions, the request struct and the call type.

### Requirement-to-test mapping

Two boundaries are involved:

- **Generator output:** tokens from the `schematic-gen` library, tested in-crate at fidelity F1, tier L1.
- **The generated crate compiled and run against the real runtime:** `fresh_output_compile`'s `slow_` tests, which build a fresh crate outside the checkout with Cargo, run its child nextest against wiremock, and check a set of misuse cases with `cargo check`. That is fidelity F3, L1 tier. These tests run in `just test-fresh-output`, part of the area's `just test`.

| Behavior | Test |
| --- | --- |
| Method = snake-case id; call type follows the request name; no conflicts for an ordinary API | `codegen::endpoint_methods::tests::methods_are_snake_case_ids_and_call_types_follow_the_request_name` |
| Request suffix carries into the call type (`BasicRequest` → `…BasicCall`, non-`Request` suffix) | `…::a_request_suffix_carries_into_the_call_type_name` |
| Setters keep field names; `full`/`build`/`pages` → `_param`; keyword field → `type_` | `…::setters_keep_field_names_and_shared_call_method_names_gain_param` |
| Exact rename list with reasons, API, endpoint, target | `…::the_rename_list_names_every_rename_with_its_reason` |
| Keyword endpoint id → `type_`, listed | `…::a_keyword_endpoint_id_takes_the_field_sanitizer_suffix_and_is_listed` |
| Every API-struct and trait method name → `_endpoint` | `…::every_api_struct_and_trait_method_name_is_renamed` |
| Two endpoints sharing a method name (including after an `_endpoint` rename) is a conflict naming both | `…::two_endpoints_with_one_method_name_are_a_conflict_naming_both` |
| A renamed setter colliding with another field is a conflict | `…::a_renamed_setter_that_meets_another_field_is_a_conflict` |
| Rename list across APIs | `…::endpoint_method_renames_spans_every_api` |
| Renames are warnings (exact rendered lines); conflicts are errors | `…::renames_are_warnings_and_conflicts_are_errors_of_the_rust_output` |
| Call type vs referenced type collision; call types of two module members | `…::a_call_type_name_shared_with_a_referenced_type_is_a_collision`, `…::call_types_of_two_apis_in_one_module_collide_only_without_distinct_suffixes` |
| Required args in `new` order (path, then body); `default()` when none; method sends nothing | `…::an_endpoint_method_takes_the_required_values_in_new_order_and_sends_nothing` |
| Call type shape: newtype, derives, `must_use`, every setter location, forwards, `IntoFuture` | `…::a_call_type_wraps_the_runtime_call_with_setters_and_shared_forwards` |
| Inherent method sets are exactly setters plus shared methods, and the API struct gains exactly the endpoint methods | `…::each_call_type_has_exactly_its_setters_and_the_shared_methods`, `…::an_api_with_no_endpoints_gains_an_empty_impl_and_no_call_types` |
| R7 docs: description (with Markdown characters, multi-paragraph), route, arguments, setters | `…::method_docs_carry_description_route_arguments_and_setters` |
| Body argument documented; empty sections omitted; no blank line for a missing description | `…::method_docs_list_the_body_and_omit_empty_sections`, `…::a_missing_description_leaves_no_leading_blank_line` |
| Setter and call-type docs | `…::setter_and_call_type_docs_name_the_parameter_and_the_method` |
| Single-file and split assembly (split: `requests.rs` imports the API struct) | `…::a_module_holds_the_call_types_and_the_endpoint_methods`, `…::a_split_module_puts_call_types_and_methods_beside_the_requests_and_imports_the_api_struct`, `output::tests::split_module_public_api_is_accessible` |
| Docs examples use endpoint methods | `…::example_invocation_passes_path_parameters_and_the_body`, `codegen::module_docs::tests::example_section_*` |
| Reserved names are true to the code (ruling 6) | `…::api_client_methods_are_exactly_the_methods_api_client_emits`, `…::api_struct_methods_are_exactly_the_methods_generate_api_struct_emits`, `…::shared_call_methods_are_methods_of_the_runtime_call` |
| No shipped call type shadows a definitions type | `…::no_shipped_call_type_shares_a_name_with_a_definitions_type` |
| Every shipped API generates endpoint methods; GitHub has `list_releases(owner, repo)` | `fresh_output_compile::generated_rest_code_references_the_runtime_and_defines_none_of_it` |
| The whole shipped crate compiles and equals the committed tree | `fresh_output_compile::slow_real_apis_compile_offline` |
| **Task 3.8.** Only the API struct imported: an endpoint method, setters, per-call settings, `.await`, `.full()`, `.build()`, and `Clone` all compile, with no trait import | `slow_generated_types_reject_misuse_at_compile_time` (`endpoint_methods_control` target, `ENDPOINT_METHODS_CONTROL`) |
| A setter after `.full()` fails (E0599 naming `limit`); awaiting `.full()` yields a `Response`, not the data (E0308) | `MISUSES` entries "an endpoint's own setter after full" and "the data, not the envelope, after full" |
| End to end against wiremock: setters reach the query in order, a per-call `anonymous` reaches the send, `.await` returns the data, `.full()` returns the envelope, a 404 is `CallError::Api` with status and body | child test `an_endpoint_method_sends_its_values_and_awaits_to_the_data` (in `slow_fixture_apis_compile_and_send_offline`) |
| End to end, renamed names (`new_endpoint`, `full_param`, `type_`) send under their wire names; a built call sent twice makes two requests | child test `renamed_endpoint_methods_and_setters_send_under_their_wire_names` (new fixture API `FixtureNames`) |
| **Task 3.9.** Partial generation still passes at ABI 3; a group recorded as ABI 2 is refused | `partial_generation` suite (unchanged cases pass), new `refuses_modules_generated_against_abi_2` |

The Input Robustness Matrix does not apply: no parser, manifest reader, or configuration loader changed. The new code reads `RestApi` values the definitions crate builds in Rust. The passive corpus test over every shipped artifact is `slow_real_apis_compile_offline`, which generates all 18 shipped APIs, compares them byte for byte with the committed tree, and compiles them. Validation over the same corpus reports no renames or collisions.

### Gates run

- `just test` in `schematic/`: 1923 passed, 12 skipped (tier-skipped as before). `test-fresh-output` passed 6 of 6, and `test-compile-fail` passed 1 of 1.
- `just lint` in `schematic/`: exit 0.
- `cargo clippy --manifest-path schematic/schema/Cargo.toml --lib -- -D warnings`: clean. `--all-targets` is not run, because the schema tests are the known Phase 4 breakage.
- Not run, still expected to fail: `schematic-schema`'s own tests (Task 4.2) and the consumers biscuit-speaks, homelab, and sniff (Phase 6).
- Cross-OS: not run. The new code generates and compares strings only, with no `#[cfg]`, process, or path-comparison logic. The one new path read (`../definitions/src`, walked with `read_dir`) and the split-layout lookup (`github/requests.rs`) go through `snapshot_tree`, which already joins components with `/` on every OS. The fresh-output tests run on every CI OS's build job as before.

### Repository state note

While this phase's gates were running, an outside process (the pipeline's separate commit step) staged the working tree and committed it. This session ran no `git add` or `git commit`. The Phase 2 and Phase 3 work is now in these commits on `feat/modern-config`:

- `138358330` (runtime)
- `d694c7f5c` (generator)
- `97c5949a3` (regenerated schema)
- `f9328b142` (validation docs)
- `ce12aed8b` (Phase 2 planning close)

`5bfe641f5`, where the phase started, is the base for the file lists in this frontmatter. Only this log and the plan and spec frontmatter edits are uncommitted.

### Checkpoint 3

- `just test` and `just lint` pass for the `schematic` area, including `schematic-gen`.
- Generating the fixture APIs produces a crate that compiles against the new runtime and sends correctly through endpoint methods. All 18 shipped APIs also generate, compile, and are committed.
- Status: Phase 3 implementation complete, ready for review.

## Phase 4

Phase 4 confirms the regenerated `schematic-schema`, migrates its tests to endpoint methods, pins the rename list, and takes the R8 "after" measurement. The measurement found a large, avoidable cost in the generated code. A one-attribute generator fix removed most of it, and the crate was regenerated with that fix. Skills used: `rust-testing` (L1 placement, F-rungs) and `os` (no OS-specific code touched). Base for this phase's file lists: `74a74f453`.

### Task 4.1: Regenerate and review churn

- `cargo run -p schematic-gen -- generate --api all --output schema/src --openapi-out openapi --postman-out postman` (that is, `just generate` without its desktop notification) left the tree byte-identical to `97c5949a3`, the Phase 3 regeneration.
- The churn review of `97c5949a3` against its parent:
    - In the non-split modules, the only removed lines are module-doc examples (`client.send(XRequest::default(), &CallOptions::new())`), the matching `use …Request` doc lines, the `use super::XApi;` import that became `use super::{X, XApi};`, the prelude's old runtime re-export list, and the ABI 2 constant and its message.
    - In the six modules that were split into directories (`bitbucket`, `eversolo`, `gitea`, `github`, `gitlab`, `huggingface`), a line-set comparison of the old single file against the union of the new `mod.rs`, `requests.rs`, and `responses.rs` shows exactly 2 lines missing per module. Both are the old doc-example lines.
    - Nothing else was removed. Additions are call types, endpoint methods, docs, and re-exports.
- Later in this phase the crate was regenerated once more for the `#[inline]` fix (see "Departure" below). That diff is exactly 547 inserted lines, one `#[inline]` per endpoint, across 15 files.

### Task 4.2: Schema tests migrated

All four REST test files now use endpoint methods. `ws_runtime.rs` has no REST sends. The patterns:

| Need | Form used |
| --- | --- |
| Decoded data | `client.get_model("x").await` |
| Envelope metadata (request id, status, headers, auth source) | `client.get_repository(o, r).full().await` |
| Failure as data | `match client.x().await { Err(CallError::Api { error, meta }) => … }` |
| Per-call credential | `.credential(Credential::access_token(..))` |
| Anonymous call | `.anonymous()` |
| Request objects (R6) | `client.send(ListPullRequestsRequest::new(..).with_state(..))` in `endpoint_method_and_generic_send_of_its_request_agree`; `send_raw(request, &CallOptions::anonymous())` in `non_json_endpoints.rs` (unchanged, because `send_raw` is outside this spec) |

Tests were renamed only where the old name described the removed shape: `anonymous_call_options_send_no_credential_despite_client_and_env` is now `anonymous_call_sends_no_credential_despite_client_and_env`.

Where an envelope test now also asserts the awaited `CallError`, its mock was widened from `.expect(1)` or `.up_to_n_times(1)` to 2.

### Task 4.3: Rename list pinned

`gen/tests/l1/output_validation.rs::live_definitions_endpoint_method_renames_are_pinned` runs over `resolve_all_apis()`, the shipped definitions. It checks:

- **Control:** there are 18 REST APIs, every API resolves one name per endpoint, none is empty, and none has a conflict. An empty rename list is therefore a result, not a skipped input.
- The rendered `endpoint_method_renames` lines equal `SHIPPED_ENDPOINT_METHOD_RENAMES`, which is **empty** as Phase 1 predicted.
- `validate_rust_client` reports exactly as many `endpoint-method-rename` warnings as the list has, and no `generated-name-collision`.

**Mutation check:** with a bogus entry in the const, the test fails (`left: []`). The const was then restored. `docs/generation/validation.md` now says the shipped list is pinned by a test.

### Task 4.4: Build, test, and lint

- `just test-schema`: 39 passed.
- `just lint-schema` (clippy `--all-targets -D warnings`): clean.
- `cargo test --doc` on the schema manifest: 0 doctests, because the examples are `text` blocks.

### Task 4.5: R8 "after"

**Method.** The host, toolchain, profile, and commands are the same as in Task 1.4: `RUSTC_WRAPPER` is disabled, each clean run builds into a fresh `CARGO_TARGET_DIR` outside the repository, and per-unit times come from `--timings` `UNIT_DATA`.

The host was heavily shared during this phase. Load averages were 11–30 on 16 cores, from other sessions. A first unpaired attempt gave clean runs of 74.2 s, 53.6 s, and 52.4 s, which disagree by more than 10%. The baseline was therefore **re-measured back to back** with the after tree under the same load:

- A throwaway detached worktree was created at `5bfe641f5`. Its `schema/src` is exactly Phase 1's 41,610 lines and 1,516,000 bytes.
- Base and after were built interleaved (base, after, base, after).
- The worktree was then removed. No `git stash` was used.

The paired numbers replace the unpaired ones.

**Before the fix (generation as committed in Phase 3), paired, load about 11–12:**

| Measurement | Base | After | Change |
| --- | --- | --- | --- |
| Clean build, whole command (wall) | 42.88 s, 43.04 s | 52.23 s, 52.52 s | **+21.9%** |
| `schematic-schema` unit | 6.14 s, 6.05 s | 15.91 s, 15.21 s | **+155%** |
| Incremental after `touch src/lib.rs` | 1.80 s, 1.78 s | 2.84 s, 2.73 s | +56% |
| OpenAI probe unit, from scratch | 2.87 s, 2.92 s | 7.56 s, 7.67 s | +163% |
| Debug `libschematic_schema.rlib` | 42.8 MB | **254.9 MB** | ×6.0 |
| Debug OpenAI probe rlib | 19.4 MB | 127.6 MB | ×6.6 |

**Root cause.** Each generated call type has a concrete, non-generic `impl IntoFuture`. Its body instantiates the runtime's generic `Call::into_future`, and through it the whole `send_unboxed` async state machine, once per endpoint inside `schematic-schema`: 547 copies, compiled whether or not anyone calls them. Before this feature, the generic `send` was instantiated only in a dependent, and only for the endpoints that dependent used.

Marking the generated `into_future` `#[inline]` moves that instantiation back to the awaiting dependent. Probe experiment on a copy of the OpenAI module:

| Probe variant | Unit time | rlib |
| --- | --- | --- |
| as generated in Phase 3 | 8.5 s, 8.7 s | 127.6 MB |
| `#[inline]` on `into_future` only | 4.0 s, 5.0 s | 32.7 MB |
| `#[inline]` on every call-type method | 5.1 s, 7.4 s (noisy) | 23.7 MB |

Inlining `into_future` alone recovers most of the cost, so only that line changed (see "Departure" below).

**After the fix (final generated code), paired.** Rounds 3–4 are the decision pair. Their unchanged `schematic-definitions` unit took 30.1–31.0 s, matching Phase 1's 29.9 s, so load inflated them least.

| Measurement | Base | After | Change |
| --- | --- | --- | --- |
| Clean build, whole command (wall), rounds 3–4 | 44.48 s, 46.47 s | 47.69 s, 47.36 s | **+4.5%** |
| `schematic-schema` unit, rounds 3–4 | 6.39 s, 6.32 s | 8.51 s, 8.51 s | **+34%** |
| `schematic-schema` unit, rounds 1–2 (load 15–28) | 6.62 s, 6.95 s | 12.04 s, 12.12 s | +78% |
| Incremental after `touch src/lib.rs`, 3 pairs | 1.74 s, 1.69 s, 1.75 s | 2.28 s, 2.20 s, 2.18 s | +28% |
| OpenAI probe unit (the `#[inline]`-only probe above, which is identical to the generated module) | 2.9 s | 4.0–5.0 s | +38% to +72% |
| Debug `libschematic_schema.rlib` | 42.8 MB | 69.0 MB | +61% |

Round 1–2 wall times are not reported. Load rose to 28 during them, and one base run (59.1 s) was slower than both after runs, so they cannot carry a comparison.

**Source size (final):**

| Scope | Lines | Bytes | Change in lines |
| --- | --- | --- | --- |
| Whole `schema/src` (42 files) | 79,181 | 3,196,068 | +90% |
| REST only | 75,065 | — | +100% |
| `openai/` | 38,703 | — | +96% |

These are Phase 3's figures plus the 547 `#[inline]` lines (265 of them in `openai/`).

**D3 branch: A (25% or less).** Under ruling 14, the whole-command clean build decides: +4.5% with the final code, and +21.9% even before the fix. The `schematic-schema` unit alone is +34% in the quietest pair, which is over 25%, so **the unit-only reading would choose branch B**. This is the disagreement ruling 14 said to flag. It is raised for the author in the spec's `human_review_items`. Phase 5 should not start until the author confirms the branch.

Open questions for the author (ruling 11): release-profile builds and rust-analyzer latency were not measured. Neither was a consumer's dependent build: with `#[inline]`, the send state machine for each awaited endpoint compiles in the consumer, as it did before this feature.

### Departure from the plan (spec untouched)

**`#[inline]` on the generated `into_future`.** The plan's Phase 4 regenerates and measures; it does not change emission. The measurement showed that the Phase 3 emission compiled 547 unused async state machines. A one-attribute change in `gen/src/codegen/endpoint_methods/mod.rs` fixes the cause without changing behavior, so it was made here rather than carried into the D3 decision as an inflated number:

- A `//` comment at the attribute in the generator explains why. It is not emitted into the generated code.
- The token test `a_call_type_wraps_the_runtime_call_with_setters_and_shared_forwards` now expects `#[inline]`.
- The crate was regenerated and every gate re-run.
- The F3 compile test (`slow_real_apis_compile_offline`) re-proved that the committed tree equals fresh output.

### Requirement-to-test mapping

Boundaries:

- The generated crate's public API against a wiremock server, through the real runtime. These are integration tests of the generated crate: fidelity F3 (real generated code, real HTTP stack, local fake peer), tier L1, run by `just test-schema`.
- Generator output over the shipped definitions: F1, L1.

| Behavior | Test |
| --- | --- |
| Endpoint method plus `.full()` exposes request id, status, headers, endpoint id, method, and URL | `github_client::success_metadata_carries_request_id_status_and_headers` |
| An awaited failure is `CallError::Api` with kind, exact body, request id, status, and endpoint | `github_client::failure_metadata_carries_request_id_status_and_headers`, `huggingface_client::not_found_is_an_envelope_failure_with_status_and_body`, `artificial_analysis_client::missing_credential_sends_anonymously_and_returns_the_401` |
| An undecodable 2xx is `CallError::Api` (`MalformedSuccess`), not a transport error | `github_client::undecodable_success_body_is_malformed_success_with_raw_bytes` |
| A hook refusal on a 200 is `CallError::Api` (`Hook`) with the original body | `huggingface_client::refusing_hook_yields_hook_failure_with_original_body` |
| Precedence through call-type setters: per-call `.credential` beats client beats env (primary, then secondary) beats anonymous | `github_client::credential_precedence_per_call_client_env_anonymous` |
| `.anonymous()` sends no credential despite client and env, for both `.full()` and plain `.await` | `github_client::anonymous_call_sends_no_credential_despite_client_and_env` |
| Redirect policy unchanged through endpoint methods | `github_client::redirects_follow_the_same_origin_policy` |
| R6: the generic `send(request)` and the endpoint method reach the same URL (setters equal `with_*`) and decode the same data | `github_client::endpoint_method_and_generic_send_of_its_request_agree` |
| `.build()` sends nothing; a built call is `Clone`; each awaited copy sends once | `github_client::a_built_call_sends_nothing_until_awaited_and_sends_once_per_copy` |
| Text, binary, and empty codecs decode through endpoint methods (`()` for empty, both `.await` and `.full()`); a setter (`pagelen`) reaches the query | `non_json_endpoints::{text_endpoint_decodes_into_string, binary_endpoint_decodes_into_bytes, empty_endpoint_decodes_into_unit}` |
| `send_raw` unchanged on request objects | `non_json_endpoints::send_raw_returns_status_headers_and_body_unchanged` |
| Query and body serialization through endpoint methods and setters | `artificial_analysis_client::*` (query string present or absent, JSON body, null-field omission) |
| Rename list pinned for the shipped definitions | `output_validation::live_definitions_endpoint_method_renames_are_pinned` |
| `#[inline]` emitted on `into_future` | `codegen::endpoint_methods::tests::a_call_type_wraps_the_runtime_call_with_setters_and_shared_forwards` |
| Committed tree equals fresh generation, and compiles | `fresh_output_compile::slow_real_apis_compile_offline` |

`.build()` refusing an invalid argument is not exercised in `schematic-schema`. No shipped endpoint's encoding can fail from caller input, because path, query, and header values are strings or numbers, and bodies are typed structs that always serialize. It is covered at the runtime level (Phase 2, Task 2.6).

The Input Robustness Matrix does not apply: no parser, manifest, or configuration reader changed. The passive corpus test over every shipped artifact is `slow_real_apis_compile_offline`, plus the new pinned-list test.

### Gates run

- `just test` in `schematic/`: 1924 passed, 12 skipped (tier-skipped, as in earlier phases). `test-fresh-output` passed 6 of 6, and `test-compile-fail` passed 1 of 1.
- `just lint` in `schematic/`: exit 0.
- `just test-schema`: 39 passed. `just lint-schema`: clean.
- Cross-OS: not run. The changes are wiremock tests with no paths, processes, or `#[cfg]`, an `#[inline]` attribute, and a test over in-memory definitions. CI covers Linux and macOS on the pull request.
- Not run, still expected to fail: the consumers biscuit-speaks, homelab, and sniff (Phase 6).
- No pre-existing failures.

### Checkpoint 4

- The schema crate builds, its tests pass, the rename-list test passes, and both measurements and the D3 branch (A, pending the author's confirmation) are recorded.
- Status: Phase 4 implementation complete, ready for review.

## Phase 5

Phase 5 applies the D3 decision that Task 4.5 measured. **Branch A was taken: chained setters kept; no feature flags needed.** No source, docs, or skill files changed in this phase. Base for this phase: `74a74f453` plus the uncommitted Phase 4 tree.

### Branch decision

- **Rule applied:** ruling 14, the literal reading of D3. The whole-command clean debug `cargo build -p schematic-schema` decides.
- **Numbers behind it** (Task 4.5, paired rounds 3–4, final generated code with `#[inline]` on `into_future`):

| Measurement | Base | After | Change | Branch by this reading |
| --- | --- | --- | --- | --- |
| Clean build, whole command (wall) | 44.48 s, 46.47 s | 47.69 s, 47.36 s | **+4.5%** | A |
| `schematic-schema` unit alone | 6.39 s, 6.32 s | 8.51 s, 8.51 s | +34% | B |

- **Final call style:** chained setters on the generated call type. For example, `gh.list_releases("o", "r").per_page(10).await?`. No per-API Cargo features and no `{Endpoint}Options` struct.

### The author has not confirmed the branch yet

Phase 4 raised the two-reading disagreement in the spec's `human_review_items` and said Phase 5 should wait for the author. This headless session found no answer: `human_review` was still `true`, and the spec carried no decision. Phase 5 went ahead with A for these reasons:

- A is what the plan's own adopted rule (ruling 14) picks, and Phase 4's recommendation agrees.
- A changes no code, so it costs nothing to reverse. If the author picks B, branch B (Tasks 5.B.1–5.B.4) can still run as written. Its only interaction with Phase 6 is the consumer `Cargo.toml` feature lines in Task 5.B.2, plus consumer call code if the 5.B.4 options-struct fallback were ever needed.

The review item stays open in the spec, reworded to say that A has been applied and what switching to B would now cost.

### Branch B tasks: not taken

- Task 5.B.1 (per-API features): not taken.
- Task 5.B.2 (switch consumers to `default-features = false`): not taken.
- Task 5.B.3 (measure biscuit-speaks dependent build): not taken.
- Task 5.B.4 (options-struct fallback): not taken.

### Requirement-to-test mapping

Phase 5 changed no behavior, so no test was added. The behavior that the branch keeps is already pinned:

| Kept behavior | Existing test |
| --- | --- |
| Chained setters and shared per-call methods on each generated call type, with `#[inline]` on `into_future` | `codegen::endpoint_methods::tests::a_call_type_wraps_the_runtime_call_with_setters_and_shared_forwards` |
| Committed `schema/src` equals fresh generation and compiles | `fresh_output_compile::slow_real_apis_compile_offline` |
| Setters reach the wire through endpoint methods | `schematic-schema` tests `github_client::endpoint_method_and_generic_send_of_its_request_agree`, `artificial_analysis_client::*`, `non_json_endpoints::*` |
| Schema manifest has no feature table (branch B absent) | Not asserted. Nothing generates features, so a test would only pin the absence. |

The Input Robustness Matrix does not apply: no parser or configuration reader changed.

### Gates run

- `just test` in `schematic/`: exit 0. L1 1924 passed, 12 skipped (tier-skipped, as in earlier phases). `test-fresh-output` 6 of 6 passed, and `test-compile-fail` 1 of 1 passed.
- `just test-schema`: exit 0, 39 passed.
- `just lint` in `schematic/`: exit 0.
- `just lint-schema`: exit 0.
- Cross-OS: not run. No code changed.
- Still expected to fail, not run: the consumers biscuit-speaks, homelab, and sniff (Phase 6).
- No pre-existing failures.

### Checkpoint 5

- The log shows the branch (A), the numbers behind it, and the final call style (chained setters).
- Schema tests and generator tests pass.
- Status: Phase 5 implementation complete, ready for review. The author's confirmation of branch A is still open in the spec's `human_review_items`.
