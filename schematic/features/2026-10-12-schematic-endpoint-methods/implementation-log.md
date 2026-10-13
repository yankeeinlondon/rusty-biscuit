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
