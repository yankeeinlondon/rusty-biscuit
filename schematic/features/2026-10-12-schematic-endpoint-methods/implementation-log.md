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
packages: []
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
