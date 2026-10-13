---
total_phases: 8
created: 2026-10-13
phase: 1
agent: claude/sonnet
yolo: true
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
source_files_during_phase_6:
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - homelab/lib/src/eversolo.rs
    - homelab/lib/src/rest.rs
    - homelab/lib/src/samsung_tv.rs
    - sniff/lib/src/remote/bitbucket.rs
    - sniff/lib/src/remote/envelope.rs
    - sniff/lib/src/remote/gitea.rs
    - sniff/lib/src/remote/github.rs
    - sniff/lib/src/remote/gitlab.rs
    - sniff/lib/tests/l1/remote_providers.rs
docs_updated_during_phase_6:
    - homelab/docs/eversolo-design.md
    - sniff/docs/repos/repo-tech-design.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/sniff/remote-and-repository.md
source_files_during_phase_7:
    - schematic/runtime/src/api_client.rs
docs_updated_during_phase_7:
    - schematic/README.md
    - schematic/schema/README.md
    - schematic/runtime/README.md
    - schematic/gen/README.md
    - schematic/docs/runtime/README.md
    - schematic/docs/runtime/sending-and-responses.md
    - schematic/docs/runtime/building-clients.md
    - schematic/docs/runtime/credentials.md
    - schematic/docs/runtime/deadlines-and-timeouts.md
    - schematic/docs/runtime/metadata-and-redaction.md
    - schematic/docs/runtime/codecs.md
    - schematic/docs/generation/request-side.md
docs_created_during_phase_7:
    - schematic/docs/runtime/calling-an-api.md
skills_files_updated_during_phase_7:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
    - .claude/skills/schematic-define/schematic/headers.md
    - .claude/skills/homelab/SKILL.md
source_files_during_phase_8: []
docs_updated_during_phase_8:
    - schematic/docs/headers/README.md
    - schematic/define/README.md
    - schematic/definitions/README.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
source_code:
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
    - schematic/gen/src/codegen/endpoint_methods/docs.rs
    - schematic/gen/src/codegen/endpoint_methods/mod.rs
    - schematic/gen/src/codegen/endpoint_methods/names.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/mod.rs
    - schematic/gen/src/codegen/module_docs.rs
    - schematic/gen/src/codegen/request_structs/mod.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/output/assemble/api.rs
    - schematic/gen/src/output/assemble/lib_rs.rs
    - schematic/gen/src/output/assemble/prelude.rs
    - schematic/gen/src/output/mod.rs
    - schematic/gen/src/validation.rs
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
    - schematic/schema/src/lmstudio.rs
    - schematic/schema/src/ollama.rs
    - schematic/schema/src/openai/mod.rs
    - schematic/schema/src/openai/requests.rs
    - schematic/schema/src/prelude.rs
    - schematic/schema/src/samsung_smart_tv.rs
    - schematic/schema/src/unfolded_circle_core_rest.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/schema/tests/artificial_analysis_client.rs
    - schematic/schema/tests/github_client.rs
    - schematic/schema/tests/huggingface_client.rs
    - schematic/schema/tests/non_json_endpoints.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - homelab/lib/src/eversolo.rs
    - homelab/lib/src/rest.rs
    - homelab/lib/src/samsung_tv.rs
    - sniff/lib/src/remote/bitbucket.rs
    - sniff/lib/src/remote/envelope.rs
    - sniff/lib/src/remote/gitea.rs
    - sniff/lib/src/remote/github.rs
    - sniff/lib/src/remote/gitlab.rs
    - sniff/lib/tests/l1/remote_providers.rs
documentation:
    - schematic/features/2026-10-12-schematic-endpoint-methods/endpoint-inventory.md
    - schematic/docs/generation/validation.md
    - homelab/docs/eversolo-design.md
    - sniff/docs/repos/repo-tech-design.md
    - schematic/README.md
    - schematic/schema/README.md
    - schematic/runtime/README.md
    - schematic/gen/README.md
    - schematic/docs/runtime/README.md
    - schematic/docs/runtime/sending-and-responses.md
    - schematic/docs/runtime/building-clients.md
    - schematic/docs/runtime/credentials.md
    - schematic/docs/runtime/deadlines-and-timeouts.md
    - schematic/docs/runtime/metadata-and-redaction.md
    - schematic/docs/runtime/codecs.md
    - schematic/docs/generation/request-side.md
    - schematic/docs/runtime/calling-an-api.md
    - schematic/docs/headers/README.md
    - schematic/define/README.md
    - schematic/definitions/README.md
completed_phase: 8
implemented: true
packages:
    - schematic-runtime
    - schematic-gen
    - schematic-schema
    - biscuit-speaks
    - sniff
    - homelab
    - schematic-define
    - schematic-definitions
---

# Schematic endpoint methods implementation plan

## Summary

Today a caller reaches an endpoint through `client.send(Request::new(..).with_x(..), &CallOptions::new()).await?.into_result()?`. This work adds, for every generated REST API struct, one inherent method per endpoint that returns a generated call type. The call type chains optional values and per-call overrides, and sends on `.await`.

The work has five parts:

1. **Runtime** (`schematic-runtime`): a combined `CallError<E>`, a generic call type with `.await`, `.full()`, and `.build()`, and the generic `send` returning that call type.
2. **Generator** (`schematic-gen`): naming and collision resolution (D2), one generated call type and one method per endpoint, and rustdoc (R7).
3. **Regeneration and measurement**: regenerate `schematic-schema`, then record compile cost before and after (R8, D3).
4. **Consumers** (R9): biscuit-speaks, homelab, and sniff move to endpoint methods.
5. **Docs** (R10): one new `schematic/docs/` page, plus drift fixes to existing pages and skills.

### Definition of success

- `gh.list_releases("o", "r").per_page(10).await?` compiles with only `GitHub` imported and returns `Vec<Release>` with one `?`.
- `.anonymous()`, `.credential(..)`, `.deadline(..)`, `.full()`, and `.build()?` work on every endpoint call type and on the runtime's generic call type, with identical behavior.
- A built call is `Clone` and can be sent twice. `.build()` does no network work and refuses an invalid argument.
- Name collisions are detected by the generator and reported in a diagnostic. A test pins the rename list for the shipped definitions.
- Every endpoint method's rustdoc carries description, HTTP method and path, required arguments, and optional setters.
- The R8 numbers and the D3 branch taken are recorded in `implementation-log.md`.
- No in-repo consumer uses `send(request, &CallOptions)`, except where generic code needs R6.
- The docs page exists, and no page in `schematic/docs/` describes the removed call shape.
- Per-package `just test` and `just lint` pass. The agent's terminal state is **implementation complete, ready for review**.

### Not in scope

- WebSocket APIs, pagination, value constraints, and endpoint-definition changes (spec non-goals).
- Any edit to the spec, sibling specs, or the lifecycle directories. Departures from the spec go in `implementation-log.md` and the docs.
- Commits. Never run `cargo fmt`.

### Input Robustness Matrix

Not applicable. This work generates code and adds runtime types. It adds no parser, manifest or lockfile reader, or config loader.

## Phase 1: Baseline, rulings, and reconciliation

Nothing generated changes in this phase, so the baseline is clean.

### Necessary Rules

The spec's decisions D1–D4 are not reopened. The rulings below fill gaps the spec leaves. Adopt each default and log it in `implementation-log.md` with its alternative. Items marked **author** are worth a look at review, but they do not block work.

| # | Topic | Gap | Ruling to adopt |
| --- | --- | --- | --- |
| 1 | `CallError` metadata (**author**) | D1 says `Api(ApiError<E>)` and `request_meta()` serves both variants. `ApiError` holds only `kind` and `body`, and `Response::into_result` discards `ResponseMeta`. | Make the `Api` variant carry the metadata next to the error. Use a struct variant `Api { error: ApiError<E>, meta: ResponseMeta }`, keeping `ApiError` itself unchanged so `Response` is unaffected. Confirm in Task 1.2 that `TransportError` already carries request metadata. If not, add it there. `request_meta()` returns the shared `RequestMeta`. |
| 2 | Generic `send` shape (R6) | The spec says the generic entry returns a runtime call type. It does not say what happens to the `&CallOptions` parameter. | `Client::send(request)` takes no options and returns `Call<'_, A, R>`. Per-call settings chain on it. Remove the `&CallOptions` parameter from `send`, and migrate every caller and test in this change. `CallOptions` stays as the call type's internal state. `send_raw` and `check_link` are unchanged (outside the spec). |
| 3 | `.full()` type | `.full()` changes the awaited output, so it cannot return `Self`. | The runtime has `Call<'a, A, R>` (awaits to `Result<R::Success, CallError<R::Failure>>`) and `FullCall<'a, A, R>` (awaits to `Result<Response<..>, TransportError>`). Both have the shared per-call methods and `.build()`. Per D4, a generated call type's `.full()` forward returns the runtime `FullCall`, so setters are unavailable after `.full()`. Document "put `.full()` last". |
| 4 | Future type | `IntoFuture` needs a named `IntoFuture` type. | Use a boxed pinned future. This matches today's `send`, which boxes its state machine. Keep `send_unboxed` as the body. |
| 5 | `.build()` cost | The spec does not say whether `.build()` keeps the encoded parts. | `.build()` encodes the request once to prove it succeeds and discards the result, taking `&self`'s clone. `.await` re-encodes. This is simple, and encoding is cheap relative to a network call. `.build()` takes `self` and returns `Result<Self, InvalidArgument>`. Requires request types to be `Clone`. Verify this in Task 1.2. The type-constraints hook is a placeholder step that runs after encoding. |
| 6 | Reserved names (D2) | "Any method later specs add" is open-ended. | Keep one reserved-name const in the generator. List the shared per-call methods (`full`, `build`, `anonymous`, `credential`, `deadline`) and the names pagination is known to add (`pages`, `fetch_page`, `collect_all`). Seed the API-struct list from the methods `api_client!` and `generate_api_struct` emit. A test proves each reserved name is a real inherent method, so the list cannot drift silently. |
| 7 | Endpoint method naming | A snake-case conversion could produce two identical method names for two endpoint ids. | Treat it as a collision. Apply a numeric-free rule: a second duplicate fails generation with a diagnostic naming both ids. Do not auto-rename these. Confirm in Task 1.3 whether any shipped definition triggers it. |
| 8 | Optional-value setter types | Setters should accept the same conversions the request's `with_` methods accept. | The call type's setter delegates to the existing `with_*` method, so the argument type is identical to today's. Required arguments use the types `{Endpoint}Request::new` takes. |
| 9 | Endpoints without a success type | Some endpoints are streamed, binary, or empty. | Endpoint methods exist for every REST endpoint, with `Success` as the request type declares it. The codec decides the data type. Streaming and `send_raw` use stay on the generic path. |
| 10 | Shared request helpers (spec Note) | Whether to move `encode_cookie_value`, `combine_cookie_headers`, and `FormFile` into the runtime. | Decide in Task 3.5, after seeing whether the call-type work touches `shared.rs`. If it does not, leave them and log "not touched". Moving them is outside the critical path. |
| 11 | Measurement scope (R8) | Spec asks for clean and incremental builds of `schematic-schema`, OpenAI separately, plus generated size. | Use one host (the development Mac), debug profile, and one run per number, plus a second run only if the first two disagree by more than 10%. Do not widen to other hosts, release builds, or rust-analyzer latency. List any wider measurement as an open question in the log, for the author. |
| 12 | Measurement baseline | R8 needs a "before" taken before generator output changes. | Take it in this phase (Task 1.4). |

### Spikes

None. The spec's own figures and the D3 rule already answer the feasibility question. R8 is a required measurement, not a spike. It runs once as a "before" (Task 1.4) and once as an "after" (Task 4.5), and it is not repeated elsewhere.

### Wave 1: Reconcile and baseline (parallel)

- [x] **Task 1.1: Load skills.** Load `schematic-define`, `rust-testing`, `rust`, `os`, and `rust-devops`. Load `biscuit-file` only if a path conversion is needed.
    - Note any OS-specific constraint before touching path or cfg code. Generated files and docs paths must stay cross-platform.
- [x] **Task 1.2: Verify runtime facts.**
    - Confirm these hold in `schematic/runtime/src`: `TransportError` carries request metadata (or how to add it), request types can derive `Clone`, `Client<A>` is cheap to borrow (`&'a Client<A>`), and the ABI constant is `2` in both `runtime/src/lib.rs` and `gen/src/lib.rs`.
    - Check which `Client`, `send`, and `CallOptions` call sites exist in `schematic/` (`runtime/tests`, `gen/tests`, `schema/tests`, `schema/src`, docs).
    - Record any surprise as a ruling update in the log.
- [x] **Task 1.3: Inventory generator inputs.**
    - List every endpoint id across all 18 shipped REST definitions, with its snake-case name, its required arguments, and its optional-value names.
    - Compute the collisions D2 and ruling 7 define. This is the expected rename list that the Phase 4 test pins.
    - Check whether any API struct already has an inherent method named like an endpoint.
- [x] **Task 1.4: Take the R8 baseline.**
    - On the development Mac: delete the build output, then run a clean debug `cargo build --manifest-path schematic/schema/Cargo.toml`. Time it.
    - Touch one source file and rebuild for the incremental time.
    - Report OpenAI (265 endpoints) separately, and report how by timing the module in isolation or from `cargo build --timings`. State which.
    - Record the size of the generated crate: bytes and lines of `schematic/schema/src`.
    - Write all of this to `implementation-log.md` under "R8 baseline", with the date, host, and commands.
- [x] **Task 1.5: Locate consumers.**
    - Confirm the consumer list from `grep` of `Cargo.toml` files: `biscuit-speaks/lib`, `sniff/lib`, `homelab/lib`, and the `homelab/*-integration*` crates.
    - Confirm whether `model-citizen` still consumes `schematic-schema`. Today it appears not to. Record the result.

### Checkpoint 1

- Rulings are recorded in the log, the baseline numbers exist, and the expected rename list is written down.
- No source file has changed.

## Phase 2: Runtime call types

Everything here is in `schematic-runtime`. It must not depend on `schematic-define`.

### Wave 2: Types (parallel)

- [x] **Task 2.1: CallError.**
    - Add `CallError<E>` in the runtime `error` module (or a new `call_error` module re-exported from `lib.rs`).
    - Shape per D1 and ruling 1: `#[non_exhaustive]`, `Transport(TransportError)`, `Api { error: ApiError<E>, meta: ResponseMeta }`.
    - Implement `Display` and `std::error::Error` with `source()` returning the inner error.
    - Implement `request_meta()`, and add convenience accessors (`api()`, `transport()`) only if a consumer in Phase 6 needs them.
    - `Display` must be credential-free, following the crate's redaction rule.
    - Prerequisite: Task 1.2's finding on `TransportError` metadata.
- [x] **Task 2.2: Call and FullCall.**
    - Add `Call<'a, A, R>` and `FullCall<'a, A, R>` in a new `call.rs` module, holding `&'a Client<A>`, the request, and `CallOptions`.
    - Add the shared methods on both: `anonymous()`, `credential(..)`, `deadline(..)`, and `build()`. `Call` also has `full()`.
    - Implement `IntoFuture` for each, boxed per ruling 4.
    - `Call`'s output maps `Outcome::Failure` to `CallError::Api` with the response metadata retained, and a `TransportError` to `CallError::Transport`.
    - `Call` and `FullCall` are `Clone` when the request is. Bounds must be method-level where possible, so `IntoFuture` does not require `Clone`.
    - Preserve the timing contract in the spec's Refresh notes: elapsed time covers body reading, decoding, hooks, and redirects, and a returned live stream stops at decoder return. The adapters add no timing of their own.

### Wave 3: Wire in (sequential after Wave 2)

- [x] **Task 2.3: Change `Client::send`.**
    - Per ruling 2, `send(request)` returns `Call`. Keep the boxed state machine as the future's body.
    - Update `api_client!` so the generated API struct's `send` forwards the same way.
    - Update all runtime-internal callers and doc examples.
- [x] **Task 2.4: ABI bump.** Raise `ABI` to `3` in `runtime/src/lib.rs` and the matching constant in `gen/src/lib.rs`, because generated code can observe the change. Update both constants' doc comments if they list changes.
- [x] **Task 2.5: Migrate runtime tests.** Move `runtime/tests/l1` and doc tests from `send(request, &CallOptions)` to the new shape. Keep a test per prior behavior: credentials precedence, anonymous, deadline, and error evidence.

### Wave 4: Runtime tests (parallel, after Wave 3)

- [x] **Task 2.6: Behavior tests (L1).** Use the mock-server pattern the existing runtime tests use.
    - `.await` returns the data on 2xx. On a declared error, a malformed body, and an interrupted body, it returns `CallError::Api` with `request_meta()` populated.
    - On a connection failure it returns `CallError::Transport`.
    - `.full()` returns the envelope with status, headers, and request id, including for error replies.
    - `.build()` refuses an invalid argument with no request reaching the mock server (assert zero hits).
    - A built call sent twice produces two requests and two results.
    - `anonymous`, `credential`, and `deadline` behave as they did through `CallOptions`: last-set credential mode wins, and an expired deadline yields a transport error.
- [x] **Task 2.7: `CallError` tests (L1).** `Display` has no secret values, `source()` chains, `request_meta()` works on both variants, and a downstream `match` with a wildcard arm compiles (non-exhaustive).

### Checkpoint 2

- `just test` and `just lint` pass in the `schematic` runtime package area.
- The generator is not yet rebuilt against ABI 3. It is expected to fail its own ABI check until Phase 3, so the generator and `schema` tests are not part of this checkpoint.

## Phase 3: Generator

All of it is in `schematic/gen/src/codegen` and `output/assemble`.

### Wave 5: Components (parallel)

- [x] **Task 3.1: Name resolver.**
    - New module under `codegen/api_struct` (or `codegen/endpoint_methods`).
    - Input: an `Endpoint` set for one API. Output: for each endpoint, the method name, the call-type name, per-setter names, and a list of renames with reason.
    - Rules per D2 and rulings 6 and 7: `_param` for a setter colliding with a shared method, `_endpoint` for a method colliding with an API-struct method, the existing request-field sanitizer for keywords, and a hard error for two endpoints that share a snake-case name.
    - The reserved-name const lives here, with the drift test from ruling 6.
- [x] **Task 3.2: Diagnostics.**
    - Emit every rename through the generator's existing diagnostic format (`gen/src/diagnostics.rs`), one line per rename, naming the API, endpoint id, old name, new name, and reason.
    - Make the rename list retrievable programmatically so the Phase 4 test can pin it.
- [x] **Task 3.3: Rustdoc builder.**
    - Build the method docs from the endpoint definition (R7): description, `METHOD /path`, a required-arguments list with types, and the optional setters the call type exposes.
    - Follow the repo's rustdoc conventions: no H1, `##` sections, and no quoted format strings or glyph narration.
    - Handle a missing description without emitting an empty line.

### Wave 6: Emission (after Wave 5)

- [x] **Task 3.4: Call type and method emission.**
    - For each REST endpoint, emit `{Endpoint}Call<'a>`: a newtype over the runtime `Call<'a, Api, {Endpoint}Request>`.
    - Emit one setter per optional request value, delegating to the request's `with_*` method (ruling 8), using names from Task 3.1.
    - Emit one-line inherent forwards for `anonymous`, `credential`, `deadline`, `full`, and `build` (D4), and an `IntoFuture` impl that delegates.
    - Emit the endpoint method on the API struct: the same required arguments in the same order as `{Endpoint}Request::new` (R1), returning the call type. It sends nothing.
    - Place call types in each endpoint's existing module, so the API struct's `impl` block only gains the methods. Re-export call types from the API module and the prelude following the request types' pattern.
    - Do not emit anything for WebSocket APIs.
    - Keep request types and `{Endpoint}Request::new` public (R6).
- [x] **Task 3.5: Shared helpers decision.**
    - Check whether Task 3.4 touches the generated `shared.rs` helpers.
    - If it does, move `encode_cookie_value`, `combine_cookie_headers`, and `FormFile` into the runtime per the spec's note, keep re-exports for generated code, and update the partial-generation byte comparison for `shared.rs` accordingly.
    - If it does not, change nothing and log "not touched".
- [x] **Task 3.6: Assembly and docs strings.** Wire the new emission into `output/assemble` (including `lib_rs.rs` and `prelude.rs`) and update `module_docs.rs` text that describes the call shape.

### Wave 7: Generator tests (parallel, after Wave 6)

- [x] **Task 3.7: Output assertions (L1).** Against a fixture API with:
    - required arguments in order and optional setters;
    - a setter named `full`, a method named `new`, and a keyword-named field;
    - rustdoc content (all four R7 items);
    - a doc string with Markdown characters.
    - Assert generated tokens and the rename list. Extend `codegen/api_struct/tests.rs` and `request_structs/tests.rs` in the existing style.
- [x] **Task 3.8: Compile test.**
    - Extend `gen/tests/l1/fresh_output_compile.rs`, or add a `trybuild` case, so a source file importing only the fixture API struct calls an endpoint method, chains a setter, and awaits.
    - This proves no trait import is needed.
    - Include a negative case: awaiting after `.full()` yields a `Response`, and chaining a setter after `.full()` fails to compile.
- [x] **Task 3.9: Partial generation.** Confirm `partial_generation.rs` tests still pass with ABI 3, and add a case showing modules generated against ABI 2 are refused.

### Checkpoint 3

- `just test` and `just lint` pass for `schematic-gen`.
- Generating the fixture API produces a crate that compiles against the new runtime.

## Phase 4: Regenerate `schematic-schema`

### Wave 8: Regenerate and verify

- [x] **Task 4.1: Regenerate.** Run the area's generate recipe (`just` in `schematic/`, near line 211 of its justfile) for all APIs. Review the diff for unexpected churn outside call types, methods, and re-exports.
- [x] **Task 4.2: Fix schema tests.** Migrate `schematic/schema/tests/*.rs` (GitHub, Hugging Face, Artificial Analysis, non-JSON endpoints) from `send(request, &CallOptions)` to endpoint methods. Keep at least one test per path that must remain on request objects (R6).
- [x] **Task 4.3: Pin the rename list.** Add a test (in `schematic-gen` L1, driven from the shipped definitions) asserting the exact rename list from Task 1.3. A new collision then fails the test and shows in review.
- [x] **Task 4.4: Build and lint.** Run the schema recipes from the `schematic` justfile for test and clippy. These use the generated manifest because `schematic-schema` is excluded from the workspace.
- [x] **Task 4.5: R8 "after".**
    - Repeat Task 1.4 exactly: same host, commands, and profile.
    - Compute the percentage increase for clean and incremental builds, OpenAI alone, and crate size.
    - Apply D3 to the clean build: **25% or less** goes to Phase 5 branch A, and **more than 25%** goes to branch B.
    - Log the numbers and the branch.

### Checkpoint 4

- The schema crate builds, its tests pass, the rename list test passes, and the log holds both measurements and the D3 branch.

## Phase 5: Apply the D3 decision

The branch is chosen by Task 4.5. Exactly one branch runs. The unused branch's tasks are recorded as "not taken".

### Branch A: Increase is 25% or less

- [x] **Task 5.A.1: Close.** Log "chained setters kept; no feature flags needed". Nothing else changes. Go to Phase 6.

### Branch B: Increase is more than 25%

- [x] **Task 5.B.1: Per-API features.** *(not taken: Phase 5 took branch A)*
    - Add one Cargo feature per generated API to `schematic-schema`, default all on. Make the generator write the features and gate each API module and its prelude re-exports.
    - Update `gen/src/cargo_gen.rs`, the schema manifest, and tests that assert the manifest.
    - Regenerate and rebuild.
- [x] **Task 5.B.2: Switch consumers.** *(not taken: Phase 5 took branch A)* Set `default-features = false` and the needed features in each consumer's `Cargo.toml`: biscuit-speaks (`elevenlabs`), sniff (`gitlab` and others it uses), and the homelab crates. Update each consumer's `docs/dependencies.md` if feature use changes.
- [x] **Task 5.B.3: Measure the heaviest consumer.** *(not taken: Phase 5 took branch A)* Measure the clean debug dependent build of biscuit-speaks with ElevenLabs only, against its own baseline. Get that baseline by building biscuit-speaks with the pre-change generated tree (a throwaway worktree or a stash-free checkout of the baseline commit; do not use bare `git stash`).
    - If the increase is 25% or less, keep chained setters and log it.
    - Otherwise do Task 5.B.4.
- [x] **Task 5.B.4: Options-struct fallback (only if needed).** *(not taken: Phase 5 took branch A)* Change Task 3.4's emission so the method takes `{Endpoint}Options { .. }` and keep the shared per-call methods. Update tests, regenerate, and re-run the Wave 8 checks. Log that the author should confirm this reading experience.

### Checkpoint 5

- The log shows the branch, the numbers behind it, and the final call style.
- Schema tests and generator tests pass.

## Phase 6: Migrate consumers (R9)

### Wave 9: One consumer per task (parallel)

Each task runs in its own subagent and edits only its package. Each migrates from request objects plus `send` to endpoint methods, except where generic code needs R6. Each subagent must treat the session as non-interactive and must not commit.

- [x] **Task 6.1: biscuit-speaks.** `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs`. Replace `CallOptions`, `EndpointSpec`, and `ApiError` handling with `CallError` matching. Keep its error mapping equivalent. Update `biscuit-speaks/docs/dependencies.md` if imports change.
- [x] **Task 6.2: sniff.** `sniff/lib/src/remote/gitlab.rs` and the other `remote` clients. The `FocusedProviderClient` may be generic over requests, so keep R6 there and say why in a comment only if non-obvious.
- [x] **Task 6.3: homelab.** `homelab/lib/src/{rest,samsung_tv,eversolo}.rs` and the `eversolo-integration`, `arcam-amp-integration`, `sony-receiver-integration`, and `unfolded-integration-helper` crates, where they use the generated clients. These files already have uncommitted edits in the working tree. Read the current state before editing and do not revert unrelated changes.
- [x] **Task 6.4: Others.** Handle any consumer Task 1.5 found, such as `model-citizen`, if still a consumer. Otherwise record "none". *(none: `model-citizen` depends only on `schematic-definitions`)*

### Wave 10: Per-package validation (parallel, after Wave 9)

- [x] **Task 6.5: Test and lint.** For each migrated package area, run `just test` and `just lint`.
    - L2 is not needed. If a consumer's L2 suite exercises these clients, run `just test-l2` without taking window focus.
- [x] **Task 6.6: Old pattern gone.** `grep` the repo, excluding `target` and the generated crate, for `CallOptions::new()` and `.send(` next to generated request types. Every remaining hit must be generic code (R6) or a comment explaining it.

### Checkpoint 6

- All consumers build and pass their tests, and the grep audit is clean.

## Phase 7: Documentation and drift

### Wave 11: Docs (parallel)

- [x] **Task 7.1: New page.**
    - Add `schematic/docs/runtime/calling-an-api.md` and link it from `schematic/docs/runtime/README.md`.
    - Write for a developer new to the repository. Lead with what the reader can do.
    - Cover creating a client, client-wide settings (base URL override, `api_key`, timeout, proxy), calling an endpoint with required and optional values, per-call overrides, `.full()`, `.build()?`, and when to use request objects.
    - Use the GitHub and OpenAI examples from the spec, a compact example per rule, and a Mermaid diagram of the call lifecycle (method, chained settings, `.build()`, `.await` or `.full()`, result or `CallError`).
    - Follow repo doc rules: kinded frontmatter if the surrounding pages use it, and **never name or link a feature or fix**.
- [x] **Task 7.2: Fix existing pages.** Update every page that shows `send(request, &CallOptions)` or `into_result()` as the primary path: `docs/runtime/sending-and-responses.md`, `building-clients.md`, `credentials.md`, `deadlines-and-timeouts.md`, `hooks.md`, `metadata-and-redaction.md`, `origins-and-redirects.md`, and `docs/generation/request-side.md`. Add `CallError` to the error documentation. These pages have uncommitted edits, so edit surgically.
- [x] **Task 7.3: READMEs and dependencies.** Update `schematic/README.md`, `schematic/schema/README.md`, and `schematic/docs/dependencies.md` if any crate or feature changed (Branch B).
- [x] **Task 7.4: Skills.** Update `.claude/skills/schematic-define` (and nested `schematic` pages) so the described call shape and `CallError` are current. Add D2 collision behavior where the skill describes generated names. Update the `os` skill only if an OS-specific fact was learned.
- [x] **Task 7.5: Comment pass.** For every symbol whose behavior changed (`Client::send`, `api_client!`, `CallOptions`, `ABI`, module docs), fix its `///`, `//!`, and inline comments per the Drift Maintenance rules. Report any drift found and how it was resolved.

### Checkpoint 7

- Every doc statement matches the code. No docs page names a feature or fix.

## Phase 8: Final validation and close-out

### Wave 12: Validation (parallel)

- [x] **Task 8.1: Package suites.** Run `just test` and `just lint` in `schematic`, `biscuit-speaks`, `sniff`, and each homelab crate. Run the schema crate's recipes.
- [x] **Task 8.2: CI scope check.** Run `just ci-local --plan` and read the executing cells. A `schematic/schema` change reaches its dependents, so confirm no package is scheduled unexpectedly. Do not push.
- [x] **Task 8.3: Spec coverage audit.** Walk R1–R10 and D1–D4 against the code and tests and tick each in `implementation-log.md` with the evidence (file, test name, or measurement).

### Wave 13: Close (sequential)

- [x] **Task 8.4: Implementation log.** Finalize `implementation-log.md`: rulings adopted, deviations from the spec (for example the struct-variant `Api` in ruling 1), R8 numbers, D3 branch, the shared-helpers choice, the rename list, and open questions for the author (any wider measurement).
- [x] **Task 8.5: Hand off.** State "implementation complete, ready for review". Do not commit, move the spec, or run `just complete`.

### Final checkpoint

- All task boxes are checked or marked "not taken" with a reason.
