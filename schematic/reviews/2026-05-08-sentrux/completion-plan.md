# Sentrux Review Completion Plan

**Review:** [review-1.md](./review-1.md) (27 suggestions, 2026-05-08)
**Plan created:** 2026-05-09

## Completed Items (10)

| # | Severity | Suggestion | Completed |
|---|----------|-----------|-----------|
| 1 | critical | Remove request_struct stubs (`multipart.rs`, `paginated.rs`, `urlencoded.rs`) | deleted, mod declarations removed |
| 2 | critical | Remove `ws_codegen` placeholder helpers (`codec.rs`, `routing.rs`) | deleted, callers cleaned up |
| 3 | urgent | Deprecate `openapi_output.rs` / `postman_output.rs` shims | `#[deprecated]` + `#[doc(hidden)]` applied |
| 4 | urgent | Drop `unfolded_circle_core_rest` compatibility shim | shim deleted; generator uses `definitions_module_path()` mapping |
| 5 | important | Drop `core` / `transport` / `model` façade modules in `schematic-define` | 28-line block removed from `lib.rs` |
| 6 | important | Split `schematic-oauth` `manager.rs` by OAuth flow | `manager/` directory with 4 per-flow files |
| 7 | nice-to-have | Add `prelude` to `schematic-oauth` | `prelude.rs` added, re-exported in `lib.rs` |
| 8 | important | Promote shared client-impl boilerplate into `shared.rs` | duplicated patterns lifted into shared helpers |
| 9 | — | Regenerate all schemas after generator changes | `just -f schematic/justfile generate` completed |
| 10 | — | Verify all tests pass | 379 unit + 249 doc tests green |

---

## Remaining Work (17 items across 5 phases)

Phases are ordered by dependency and risk. Each phase is independently
testable — run `cargo test -p schematic-define -p schematic-definitions -p schematic-gen`
and `cargo check -p schematic-schema` after each phase.

---

## Phase 1: `schematic-define` file splits

These are the highest-value cognitive-load reductions. Each is a
self-contained file-to-directory promotion with no cross-crate impact.

### 1A. Split `openapi/export.rs` → `export/` directory (1,510 LOC)

**Current:** `define/src/openapi/export.rs`
**Target:**
```
openapi/
  export.rs                # public export() entry + re-exports (~100 LOC)
  export/
    info.rs                # map_info, map_servers
    paths.rs               # map_paths, map_operation, extract_path_params
    request_body.rs        # map_request_body, map_form_fields_to_schema
    responses.rs           # map_responses
    security.rs            # map_security, map_security_requirements
    components.rs          # map_components, SchemaRegistryLike
    validate.rs            # validate_ref_closure + visit_* helpers
```

**Scope:** ~1,510 lines relocated. Tests stay in per-file `#[cfg(test)]` blocks.
**Risk:** Low — mirrors existing `openapi/import/` pattern.
**Verify:** `cargo test -p schematic-define`, doctest still resolves `schematic_define::openapi::export::*`.

### 1B. Extract `OpenApiImport` builder → `openapi/import/builder.rs` (1,198 LOC)

**Current:** `define/src/openapi/import.rs` (~553 impl + ~645 tests)
**Target:**
```
openapi/
  import.rs                # façade: pub use builder::*, re-exports
  import/
    (existing: diagnostics, naming, resolver, mappings/)
    builder.rs             # OpenApiImport struct + 17 methods
    auth.rs                # map_auth_strategy + helpers
```

**Scope:** Builder impl block moved to `builder.rs`; auth mapping to `auth.rs`; `import.rs` becomes re-export façade.
**Risk:** Low — `import/` directory already exists with submodules.
**Verify:** `cargo test -p schematic-define`, doctest for `OpenApiImport::new()`.

### 1C. Split `headers/builder.rs` → `builder/` module tree (1,249 LOC)

**Current:** `define/src/headers/builder.rs`
**Target:**
```
headers/
  builder/
    mod.rs                 # Headers struct, accept/content_type/header/remove/build
    auth.rs                # use_bearer_token*, use_basic_auth, use_api_key
    env.rs                 # from_env, from_env_with, try_from_env, from_env_internal
    validate.rs            # validate_header_name
    tests.rs               # ~640 lines of unit tests (#[cfg(test)])
```

**Scope:** Split by concern; tests centralized.
**Risk:** Low — public API unchanged, only internal module structure.
**Verify:** `cargo test -p schematic-define`, all Headers doctests.

### 1D. Split `websocket.rs` → `websocket/` directory (1,279 LOC)

**Current:** `define/src/websocket.rs`
**Target:**
```
websocket/
  mod.rs                   # pub re-exports + WebSocketApi/Endpoint + tests
  lifecycle.rs             # ConnectionLifecycle, MessageSchema, MessageDirection
  params.rs                # ConnectionParam, ParamType
  hints.rs                 # AuthFlowHints, CorrelationHints, HeartbeatHints,
                           # FrameFormat, RequestIdType, *Hints
```

**Scope:** 14 public types distributed across 4 files. `mod.rs` re-exports everything.
**Risk:** Low — `pub use websocket::*` still works from `lib.rs`.
**Verify:** `cargo test -p schematic-define`, WebSocket doctests.

---

## Phase 2: `schematic-gen` file splits

These are the largest files in the generator and the biggest contributors
to the Gini equality penalty in that crate.

### 2A. Split `export/postman.rs` → `postman/` directory (1,936 LOC)

**Current:** `gen/src/export/postman.rs` (~999 impl + ~937 tests)
**Target:**
```
export/
  postman.rs               # write_postman, write_postman_grouped (entry points, ~80 LOC)
  postman/
    mod.rs                 # internal re-exports
    collection.rs          # build_postman_collection, build_postman_collection_grouped
    item.rs                # build_item, build_folder_item
    request.rs             # build_request, url/header/body generation
    auth.rs                # auth blocks (bearer/basic/api-key), auth_variables
    variables.rs           # collection-variables, path/query interpolation
    examples.rs            # example response generation
    tests.rs               # unit tests (#[cfg(test)])
```

**Scope:** ~1,936 lines split by concern.
**Risk:** Medium — Postman export has integration tests in `gen/tests/postman_golden.rs` and `gen/tests/postman_artifact_validation.rs`. Run those.
**Verify:** `cargo test -p schematic-gen`, all postman golden + artifact validation tests.

### 2B. Split `codegen/request_structs/mod.rs` → submodules (1,863 LOC)

**Current:** `gen/src/codegen/request_structs/mod.rs` (~567 impl + ~1,296 tests)
**Target:**
```
codegen/request_structs/
  mod.rs                   # public API, re-exports (~50 LOC)
  body.rs                  # generate_request_struct_body
  shared.rs                # generate_request_struct_shared (no params)
  single.rs                # single-param request structs
  derives.rs               # derive attribute generation
  path_params.rs           # path parameter extraction + new() methods
  query_params.rs          # query parameter fields + builder methods
  into_parts.rs            # into_parts + from_parts generation
  endpoint_spec.rs         # generate_endpoint_spec
  tests.rs                 # test module (#[cfg(test)])
```

**Scope:** ~567 impl lines split by generation concern; ~1,296 test lines to `tests.rs`.
**Risk:** Medium — this is the codegen hot path. Thorough syntax-validation test needed.
**Verify:** `cargo test -p schematic-gen`, `just -f schematic/justfile generate && cargo check -p schematic-schema`.

### 2C. Split `codegen/client.rs` → `client/` directory (1,320 LOC)

**Current:** `gen/src/codegen/client.rs` (~647 impl + ~673 tests)
**Target:**
```
codegen/
  client/
    mod.rs                 # generate_client_struct + re-exports
    methods.rs             # generate_request_method, generate_request_method_with_suffix
    helpers.rs             # generate_error_type, generate_request_parts_type
    tests.rs               # test module
```

**Scope:** `client.rs` promoted to directory following `api_struct/` pattern.
**Risk:** Medium — `client.rs` is imported from `output/assemble.rs` and `commands.rs`.
**Verify:** `cargo test -p schematic-gen`.

### 2D. Split `codegen/api_struct/mod.rs` into submodules (1,041 LOC)

**Current:** `gen/src/codegen/api_struct/mod.rs` (~562 impl + ~479 tests)
**Target:**
```
codegen/api_struct/
  mod.rs                   # generate_api_struct entry point (~80 LOC)
  auth.rs                  # (existing) auth strategy generation
  helpers.rs               # (existing) helper generation
  fields.rs                # generate_client_fields, env_auth fields
  constructors.rs          # generate_constructors (new, with_base_url, with_client)
  request_method.rs        # generate_request_method integration
  variant.rs               # variant builder + UpdateStrategy hooks
  tests.rs                 # test module
```

**Scope:** Parent `mod.rs` reduced to entry point + re-exports.
**Risk:** Low-Medium — `api_struct/` already has submodules; extending the pattern.
**Verify:** `cargo test -p schematic-gen`.

---

## Phase 3: `schematic-definitions` provider splits

These are the highest-LOC refactorings in the area. Each provider is
independent and can be done as a standalone PR.

### 3A. Split largest `types.rs` files by resource

Split providers exceeding 1,000 LOC in `types.rs` into `types/` directories
with one file per resource family:

| Provider | Current LOC | Resource Split |
|----------|------------|----------------|
| huggingface | 2,587 | models, datasets, spaces, repos, inference |
| elevenlabs | 2,035 | voices, speech, sound_generation, voice_library, workspace |
| gitlab | 1,411 | projects, merge_requests, issues, releases, pipelines |
| bitbucket | 1,365 | repos, pull_requests, issues, tags, workspaces |
| github | 1,151 | repos, pull_requests, issues, releases |
| ollama | 1,149 | completion, models, misc |
| anthropic | 1,069 | messages, tools, batches |
| emqx | 1,012 | auth, clients, topics, plugins, subscriptions |

**Pattern:**
```
provider/
  mod.rs
  types.rs            # façade: pub use types::* re-exports
  types/
    resource_a.rs
    resource_b.rs
```

**Scope:** ~12,879 total LOC redistributed across ~35-40 new files.
**Risk:** Low per-provider — pure data types with no logic. Public import paths preserved via façade `types.rs`.
**Verify:** `cargo test -p schematic-definitions`, `cargo check -p schematic-schema`.

### 3B. Split largest `mod.rs` endpoint catalogs by resource

For providers exceeding 1,000 LOC in `mod.rs`, extract per-resource
endpoint groups:

| Provider | Current LOC | Endpoint Groups |
|----------|------------|----------------|
| emqx | 1,224 | auth, clients, topics, subscriptions, plugins, alarms, stats |
| elevenlabs | 1,119 | voices, speech, sound_generation, voice_library, workspace, history |
| gitlab | 1,038 | projects, merge_requests, issues, releases, pipelines |
| github | 967 | repos, pull_requests, issues, releases |

**Pattern:**
```
provider/
  mod.rs              # define_*_api() + endpoints.extend(endpoints::resource::all())
  endpoints/
    resource_a.rs     # pub fn all() -> Vec<Endpoint> { ... }
    resource_b.rs
```

**Scope:** ~4,348 total LOC redistributed.
**Risk:** Low — each `endpoints::resource::all()` returns `Vec<Endpoint>`, easily tested.
**Verify:** `cargo test -p schematic-definitions`, compare generated output before/after.

### 3C. Cap `registry.rs` by extracting per-provider registration (1,220 LOC)

**Current:** `definitions/src/registry.rs`
**Target:**
```
registry/
  mod.rs               # SchemaRegistry struct + public API (~200 LOC)
  builders.rs          # register_*() helper methods
  providers/
    openai.rs          # add_openai_types(&mut SchemaRegistry)
    anthropic.rs
    elevenlabs.rs
    ...                # one file per provider (~50-100 LOC each)
```

**Scope:** 1,220 LOC split. Central `mod.rs` stays under 300.
**Risk:** Medium — registry is used by OpenAPI export, Postman export, and strict completeness tests.
**Verify:** `cargo test -p schematic-gen` (includes `openapi_strict_completeness` tests).

---

## Phase 4: Generator output pipeline improvements

These changes affect what gets emitted into `schematic-schema` and
therefore have the widest blast radius. Do these after the generator's
own structure is clean (Phases 1-3).

### 4A. Emit per-resource submodules per API (schematic-schema)

**Current:** One `.rs` file per `RestApi` (e.g., `emqx.rs` at 6,302 LOC)
**Target:** One directory per API:
```
schema/src/
  emqx/
    mod.rs             # client struct, re-exports (~200 LOC)
    requests.rs        # *Request structs + request enum (~1,500 LOC)
    responses.rs       # response type re-exports (~500 LOC)
    client.rs          # impl block with HTTP methods (~2,000 LOC)
```

**Scope:** Changes `output/assemble.rs` and `output/mod.rs` in `schematic-gen`.
         Requires updating `lib.rs` and `prelude.rs` generation too.
         All ~17 generated REST API modules affected.
**Risk:** High — changes generated code shape. Requires thorough regeneration + `cargo check`.
**Verify:** `just -f schematic/justfile generate && cargo check -p schematic-schema`, all e2e tests.

### 4B. Reduce `output/mod.rs` (896 LOC) and `output/assemble.rs` (799 LOC)

**Current:** Cross-cutting orchestration and assembly in two large files.
**Target:**
```
output/
  mod.rs               # generate_and_write entry; thin façade (~100 LOC)
  pipeline.rs          # validate → format → write orchestration
  files.rs             # per-file naming, path resolution, cleanup
  assemble/
    mod.rs             # re-exports + get_module_path helpers
    api.rs             # API struct + variant assembly
    requests.rs        # request enum + per-endpoint structs
    client.rs          # client struct generation
    lib_rs.rs          # lib.rs assembly
    prelude.rs         # prelude.rs assembly
    docs.rs            # module doc comment generation
    ws.rs              # WS module assembly
```

**Scope:** 1,695 LOC redistributed across ~10 focused files.
**Risk:** Medium-High — output pipeline is the integration point for all generation.
**Verify:** Full regeneration + `cargo check -p schematic-schema` + all `schematic-gen` tests.

### 4C. Audit WS client duplication → `ws_shared.rs`

**Current:** 5 WS files totaling ~3,488 LOC with duplicated envelope/codec/lifecycle code.
**Target:** Pull common patterns into `ws_shared.rs` (currently 298 LOC).
**Scope:** Survey + extract; estimated 200-400 LOC reduction per WS file.
**Risk:** Medium — WS codegen is less tested than REST.
**Verify:** `cargo test -p schematic-gen` (ws_codegen tests), `cargo check -p schematic-schema`.

---

## Phase 5: Nice-to-have polish

Low-risk, low-urgency improvements that can be picked up opportunistically.

### 5A. Split `models.rs` (951) and `params.rs` (931) per shape

Promote to directories with one type-family per file (`structs`, `enums`,
`aliases`, `type_ref`). Brings them in line with `headers/` and `openapi/`
patterns.

### 5B. Make `apis_by_module()` data-driven

Replace the hardcoded `vec![...]` in `definitions/src/lib.rs` with
per-provider registration (e.g., `inventory` crate or manual macro).
Same shape as `registry.rs` per-provider split in 3C.

### 5C. Consolidate test utilities in `schematic-gen`

Promote shared `Endpoint`, `RestApi`, and `ApiResponse` fixture builders
from `test_utils.rs` and inline test helpers into a shared
`test_utils::fixtures` module.

### 5D. Add generation-provenance comments with content hash

Each generated file gets:
```rust
// generated-by: schematic-gen 0.4.2
// source-hash:  b3:7f0c2a91...
```
Requires blake3 of the canonical `RestApi` definition. Enables CI drift
detection and idempotent regeneration.

### 5E. Verify `prelude.rs` coverage is automatic

Ensure `schematic/schema/src/prelude.rs` is auto-generated or build-script
emitted so new providers don't require manual edits.

### 5F. Fold tiny `error.rs` (40 LOC) and `types.rs` (135 LOC) into `lib.rs` in `schematic-oauth`

Defer until after `manager/` split settles (already done). Only if files
remain this small.

---

## Execution Strategy

### Dependency order
- Phase 1 and Phase 2 are **independent** of each other and can run in
  parallel or in any order.
- Phase 3 is **independent** of Phases 1-2. Provider splits can be done
  in any order (one PR per provider is fine).
- Phase 4 **depends on** Phases 1-2 being complete — the output pipeline
  should be clean before changing what it emits.
- Phase 5 is **independent** of everything else.

### Recommended session ordering

1. **Session 1:** Phase 1A + 1B (`schematic-define` openapi splits)
2. **Session 2:** Phase 1C + 1D (`schematic-define` headers + websocket)
3. **Session 3:** Phase 2A (postman split — largest single-file in gen)
4. **Session 4:** Phase 2B (request_structs split)
5. **Session 5:** Phase 2C + 2D (client + api_struct splits)
6. **Session 6:** Phase 3C (registry split — unblocks 3A/3B patterns)
7. **Session 7-10:** Phase 3A + 3B (one provider per session, largest first)
8. **Session 11:** Phase 4A (per-resource submodules — biggest impact)
9. **Session 12:** Phase 4B + 4C (output pipeline + WS audit)
10. **Opportunistic:** Phase 5 items as fill-in work

### Testing checkpoint after each session
```bash
cargo test -p schematic-define -p schematic-definitions -p schematic-gen
just -f schematic/justfile generate
cargo check -p schematic-schema
```
