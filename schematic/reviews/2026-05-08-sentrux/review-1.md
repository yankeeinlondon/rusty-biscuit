---
title: Schematic Quality Review (Sentrux)
date: 2026-05-08
reviewer: claude-opus-4-7
baseline: .sentrux/baseline.json
suggestions: 27
suggestions_critical: 2
suggestions_urgent: 7
---

# Schematic Quality Review (Sentrux)

This review evaluates the `schematic` package area against five quality
metrics — **modularity**, **acyclicity**, **depth**, **equality**, and
**redundancy** — and proposes prioritized improvements per package.

## Methodology Note

The Sentrux MCP tools (`scan`, `health`, `dsm`, `check_rules`, `test_gaps`,
`git_stats`) require interactive permission approval that is not available in
this non-interactive session. The CLI is similarly gated. The review therefore
combines:

- The pre-computed baseline at `.sentrux/baseline.json`
  (`quality_signal=0.72`, `cycle_count=0`, `max_depth=1`,
  `coupling_score=0.18`, `total_import_edges=147`, `cross_module_edges=96`,
  `complex_fn_count=40`).
- Direct source inspection: file inventory, line counts (a coarse but
  effective proxy for the Gini equality metric), `mod` graph,
  presence of stub/placeholder files (Kolmogorov-style redundancy), and
  inter-crate dependency direction.

The baseline confirms there are **no cycles** anywhere in the area and the
import-graph is shallow (`max_depth=1`), so all cycle/depth recommendations
below are preventative — keeping things that already pass from sliding.
The dominant signal is **inequality** (a few enormous files concentrating
the complexity) and **localized redundancy** (placeholder modules that
exist purely to be re-routed elsewhere). Cross-module coupling at
`96/147 ≈ 65%` of edges is also notable and drives the modularity targets
below.

To regenerate the live metrics once permissions are granted:

```bash
sentrux scan schematic/define
sentrux scan schematic/definitions
sentrux scan schematic/gen
sentrux scan schematic/oauth
sentrux scan schematic/schema
```

---

## schematic-define

**Files:** 31 · **LOC:** ~12,635 · **Notable size outliers:**
`openapi/export.rs` (1,510) · `websocket.rs` (1,279) · `headers/builder.rs`
(1,249) · `openapi/import.rs` (1,198) · `models.rs` (951) · `params.rs`
(931) · `request.rs` (883)

The crate has the largest equality (Gini) problem in the package area: a
small set of files concentrates the implementation while many siblings sit
under 250 lines. The `openapi` subsystem is healthy structurally
(`import/` is split into `diagnostics`, `naming`, `resolver`, `mappings/*`)
but the parent `import.rs` and `export.rs` files still carry the bulk.

### `urgent`: Split `openapi/export.rs` into per-aspect submodules

`schematic/define/src/openapi/export.rs` (1,510 lines) holds eight
different mapping concerns — `Info`, `Servers`, `Paths`, `Operations`,
`RequestBody`, `Responses`, `SecurityRequirements`, `Components` — plus
the recursive `validate_ref_closure` walker. This is the single largest
contributor to the Gini equality penalty in `define`.

Mirror the structure already used by `openapi/import/`:

```
openapi/
  export.rs                # public `export()` entry point + re-exports
  export/
    info.rs                # map_info, map_servers
    paths.rs               # map_paths, map_operation, extract_path_params
    request_body.rs        # map_request_body, map_form_fields_to_schema
    responses.rs           # map_responses
    security.rs            # map_security, map_security_requirements
    components.rs          # map_components, SchemaRegistryLike
    validate.rs            # validate_ref_closure + visit_* helpers
```

Each new file becomes ~150–250 lines with one clear responsibility, the
parent stays under 200 lines, and tests can move to per-concern `mod
tests` blocks.

### `urgent`: Extract `OpenApiImport` builder out of `import.rs`

`schematic/define/src/openapi/import.rs` (1,198 lines, ~553 impl + ~645
tests) still embeds the `OpenApiImport` builder inline alongside source
parsing, validation, auth-strategy mapping, and operation extraction even
though the directory submodules already exist. The builder has 17
methods on a single impl block.

Move the builder into `openapi/import/builder.rs` and the
auth-strategy mapping into `openapi/import/auth.rs`. The parent
`import.rs` then becomes a façade re-exporting public types.

### `important`: Split `headers/builder.rs` into a small module tree

`schematic/define/src/headers/builder.rs` (1,249 lines) interleaves
Headers builder methods, three auth strategies, env-fallback resolution,
header-name validation, and ~640 lines of unit tests. Extract:

```rust
// headers/builder/auth.rs    — use_bearer_token{,_with_header}, use_basic_auth, use_api_key
// headers/builder/env.rs     — from_env, from_env_with, try_from_env, from_env_internal
// headers/builder/validate.rs — validate_header_name
// headers/builder.rs         — Headers struct, accept/content_type/header/remove/build
```

Tests can live in a dedicated `headers/builder/tests.rs` to keep the
implementation files lean.

### `important`: Split `websocket.rs` into a `websocket/` module

`schematic/define/src/websocket.rs` (1,279 lines) contains
`WebSocketApi`, `WebSocketEndpoint`, `ConnectionLifecycle`,
`MessageSchema`, `MessageDirection`, `ConnectionParam`, `ParamType`,
`AuthFlowHints`, `CorrelationHints`, `HeartbeatHints`, `FrameFormat`,
`RequestIdType`, `WebSocketRuntimeHints`, and `WebSocketEndpointHints` —
14 public types plus ~625 lines of tests. Promote to a directory:

```
websocket/
  mod.rs            # public re-exports + WebSocketApi/Endpoint
  lifecycle.rs      # ConnectionLifecycle, MessageSchema, MessageDirection
  params.rs         # ConnectionParam, ParamType
  hints.rs          # AuthFlowHints, CorrelationHints, HeartbeatHints,
                    # FrameFormat, RequestIdType, *Hints
```

### `important`: Drop the `core` / `transport` / `model` façade modules

`schematic/define/src/lib.rs:177-201` defines three façade modules that
re-export `crate::*` glob-style. They duplicate the top-level
`pub use` re-exports already declared at lines 152–171, are not
referenced elsewhere in the area, and add ambiguity (`crate::model::*`
vs `crate::models::*` — the singular form is a re-export of the plural,
which is itself a sibling module). Removing them tightens the public
surface and avoids the dual-namespace confusion.

```rust
// Delete:
pub mod core { pub use crate::auth::*; ... }
pub mod transport { pub use crate::headers::*; ... }
pub mod model { pub use crate::models::*; }
```

If grouped imports are valuable, replace with `mod groups { ... }` that
is `#[doc(hidden)]` and clearly marked as for documentation only — but
verify external callers first.

### `nice-to-have`: Split `models.rs` and `params.rs` per shape

`schematic/define/src/models.rs` (951) defines `ModelCatalog`,
`ModelDef`, `StructDef`, `EnumDef`, `EnumVariant`, `TypeAlias`,
`FieldDef`, `TypeRef`, `PrimitiveType`. `params.rs` (931) similarly
bundles `EndpointParams`, `ParamDef`, `QueryParamType`, `ParamStyle`.
Promoting both to directories with one type-family per file (`structs`,
`enums`, `aliases`, `type_ref`) brings them in line with the
`headers/` and `openapi/` patterns and reduces the equality outlier
even if no implementation moves.

---

## schematic-definitions

**Files:** 42 · **LOC:** ~29,785 · **Notable size outliers:**
`huggingface/types.rs` (2,587) · `elevenlabs/types.rs` (2,035) ·
`elevenlabs/mod.rs` (1,119) · `gitlab/types.rs` (1,411) ·
`gitlab/mod.rs` (1,038) · `bitbucket/types.rs` (1,365) ·
`registry.rs` (1,220) · `emqx/mod.rs` (1,224)

Each provider follows a uniform `mod.rs` (endpoint definitions) +
`types.rs` (request/response shapes) split, which is good for modularity.
The Gini problem is intra-provider: the largest providers exceed 2,000
LOC in a single `types.rs`, while `openai/types.rs` is 96 lines —
roughly a 27× spread.

### `urgent`: Drop the `unfolded_circle_core_rest` compatibility shim

`schematic/definitions/src/lib.rs:180-187` re-exports
`crate::unfolded_circle::core_rest::*` under a top-level
`unfolded_circle_core_rest` module purely so generated code can resolve
the `module_path` value through a flat name. This is the textbook
Kolmogorov-style redundant node — the same content reachable via two
paths.

Fix in the generator (`schematic-gen`) to emit either the nested path or
to canonicalize on the flat path, then delete the shim. While the shim
is small, it is the kind of dual-path that drives modularity penalties
in the DSM and is easy to forget when adding sibling APIs (Dock, Core
WS, Integration WS — all under `unfolded_circle::*` with no flat
aliases, suggesting the inconsistency is unintended).

### `important`: Split the largest provider `types.rs` files by resource

`huggingface/types.rs` (2,587), `elevenlabs/types.rs` (2,035),
`gitlab/types.rs` (1,411), `bitbucket/types.rs` (1,365),
`github/types.rs` (1,151), `ollama/types.rs` (1,149),
`anthropic/types.rs` (1,069), and `emqx/types.rs` (1,012) all bundle
unrelated resource families (e.g., for Bitbucket: repositories +
pull-requests + issues + tags + workspaces). Promote to a `types/`
directory with one file per resource:

```
elevenlabs/
  mod.rs
  types.rs            # façade re-exports (preserve existing import paths)
  types/
    voices.rs
    speech.rs
    sound_generation.rs
    voice_library.rs
    workspace.rs
```

Same approach for HuggingFace (models, datasets, spaces, repos),
GitLab (projects, MRs, issues, releases), GitHub (repos, PRs, issues,
releases), Bitbucket (repos, PRs, issues, tags), Anthropic (messages,
tools, batches), EMQX (auth, clients, topics, plugins).

### `important`: Mirror the same split on the largest `mod.rs` files

`emqx/mod.rs` (1,224), `elevenlabs/mod.rs` (1,119),
`gitlab/mod.rs` (1,038), `github/mod.rs` (967), and the EMQX/Bitbucket
endpoint catalogs concentrate `define_*_api()` definitions in a single
function with hundreds of inline `Endpoint` literals. Extract per-resource
endpoint groups into private free functions in matching submodules:

```rust
// elevenlabs/mod.rs
mod endpoints {
    pub mod voices;
    pub mod speech;
    pub mod sound_generation;
}

pub fn define_elevenlabs_rest_api() -> RestApi {
    let mut endpoints = Vec::with_capacity(40);
    endpoints.extend(endpoints::voices::all());
    endpoints.extend(endpoints::speech::all());
    endpoints.extend(endpoints::sound_generation::all());
    RestApi { /* ... */ endpoints, /* ... */ }
}
```

This brings the single-function complexity down (currently the largest
contributors to the `complex_fn_count=40` baseline) and makes per-resource
review and diffing dramatically easier.

### `important`: Cap `registry.rs` by extracting per-shape registration

`schematic/definitions/src/registry.rs` (1,220 lines) acts as a single
central catalog for every JSON Schema in every provider. As the catalog
grows, this file is on track to be the largest single source file in the
crate. Split into a small `registry/` directory:

```
registry/
  mod.rs              # SchemaRegistry struct + public API
  builders.rs         # register_*() builder helpers
  providers/
    openai.rs         # registry-additions for OpenAI types
    anthropic.rs      # registry-additions for Anthropic types
    ...
```

Each provider's registration code lives next to its types-of-record,
removing the pull on `registry.rs`. The crate-level facade in
`registry/mod.rs` stays small.

### `nice-to-have`: Make `apis_by_module()` data-driven

`schematic/definitions/src/lib.rs:231-266` hard-codes the list of 18
APIs in a single `vec![...]`. Each new provider requires editing this
list (and may be silently omitted on additions, as the test surface is
limited). Replace with a constructor list registered per-provider, e.g.
via `inventory` or a manual macro that walks each provider module's
exported `define_*_api` symbols.

---

## schematic-gen

**Files:** 56 · **LOC:** ~20,750 · **Notable size outliers:**
`export/postman.rs` (1,936) · `codegen/request_structs/mod.rs` (1,866) ·
`codegen/client.rs` (1,320) · `export/openapi.rs` (1,087) ·
`codegen/api_struct/mod.rs` (1,041) · `output/mod.rs` (896) ·
`output/assemble.rs` (790) · `pipeline.rs` (712)

`schematic-gen` carries the **highest redundancy load** in the area.
There are five files that are pure stubs or backward-compat shims and
two `ws_codegen` placeholder modules that emit empty `TokenStream`s. It
also has the worst single-file equality outliers and likely contributes
significantly to the `coupling_score=0.18` baseline through
cross-module imports out of `output/`, `codegen/`, and `export/`.

### `critical`: Remove or land the placeholder request_struct stubs

`schematic/gen/src/codegen/request_structs/multipart.rs` (4 lines),
`paginated.rs` (4 lines), and `urlencoded.rs` (4 lines) each contain
only a doc comment ending in *"reserved for future shape-specific
handling"*. They are declared in
`codegen/request_structs/mod.rs:14-19` and contribute to module-graph
fan-in without providing any implementation. This is exactly the
Kolmogorov-redundancy pattern Sentrux flags.

Two acceptable resolutions:

1. **Delete** them and remove the `mod` declarations. Add them back
   when shape-specific handling is actually implemented.

2. **Land** the implementation now — multipart and url-encoded bodies
   already have specialized handling inline in
   `request_structs/mod.rs`; relocating that code to the named files
   would justify their existence.

```rust
// codegen/request_structs/mod.rs — option 1 (delete)
- mod multipart;
- mod paginated;
- mod urlencoded;
  mod body;
  mod shared;
  mod single;
```

### `critical`: Remove or implement the `ws_codegen` placeholder helpers

`schematic/gen/src/ws_codegen/codec.rs` (`generate_codec_helpers`) and
`schematic/gen/src/ws_codegen/routing.rs`
(`generate_routing_helpers`) both return an empty `TokenStream::new()`.
Their doc comments say codec generation "is handled inline by the shared
module's blanket impls" and routing "will be generated when typed
envelope structs are available". They are both module-level public
functions and consume real edges in the graph.

```rust
// ws_codegen/codec.rs — currently:
pub fn generate_codec_helpers() -> TokenStream {
    TokenStream::new()
}
```

Either delete both files (and any callers, which currently pass the
empty stream through unchanged) or move the inline blanket-impl
generation into them so they actually carry the helper code their
names promise. Deletion is the lower-risk option until the AsyncAPI
import work that motivates routing exists.

### `urgent`: Decide on `openapi_output.rs` / `postman_output.rs` shims

`schematic/gen/src/openapi_output.rs` (12 lines) and
`postman_output.rs` (8 lines) are *backward-compatibility shims* that
each consist of `pub use crate::export::{openapi,postman}::*;`.
`lib.rs:73,77,83-84` re-exports both the shim modules and the canonical
`export::*` modules — so every public symbol is reachable through three
distinct paths.

If no external consumer still depends on `schematic_gen::openapi_output`
or `schematic_gen::postman_output`, delete the shims and the
corresponding `pub mod` declarations. If they must remain for one
release cycle, mark them `#[deprecated]` and scope the shim with a
single `#[doc(hidden)] pub use ...;` to flatten the alias.

### `urgent`: Split `export/postman.rs`

`schematic/gen/src/export/postman.rs` (1,936 lines, ~999 impl + ~937
tests) is the largest single file in `schematic-gen`. It generates
Postman Collection v2.1 JSON for every shape of request — collections,
folders, items, request URLs, query/header/body sections, auth blocks,
variable interpolation, examples — all in one module. Promote to a
`postman/` directory:

```
export/
  postman.rs              # write_postman, write_postman_grouped (entry points)
  postman/
    collection.rs         # top-level collection + info block
    item.rs               # request/folder item generation
    request.rs            # url, method, header, body sections
    auth.rs               # auth blocks (bearer/basic/api-key)
    variables.rs          # collection-variables + path/query interpolation
    examples.rs           # example responses
```

### `urgent`: Split `codegen/request_structs/mod.rs`

`schematic/gen/src/codegen/request_structs/mod.rs` (1,866 lines) holds
~567 lines of generation logic and ~1,299 lines of unit tests in a
single file. Move tests to a sibling `tests.rs` (or
`request_structs/mod_test.rs` for path stability) and split the
remaining generation logic by concern: `derives`, `path_params`,
`query_params`, `into_parts`, `endpoint_spec`. The current file
defines 12 free functions covering all of these.

### `important`: Split `codegen/client.rs` and `codegen/api_struct/mod.rs`

`codegen/client.rs` (1,320 lines, ~647 impl + ~673 tests) and
`codegen/api_struct/mod.rs` (1,041 lines, ~562 impl + ~479 tests)
are the next-largest offenders. The `api_struct` module already has
an `auth.rs` and `helpers.rs` submodule, but the parent `mod.rs`
remains the bulk holder; lift the major impl groups into named
submodules (`fields.rs`, `constructors.rs`, `request_method.rs`).
For `client.rs`, mirror the existing `client/` directory split
(`methods.rs`, `helpers.rs`) by moving the `generate_client_struct`
top-level glue out of the file root.

### `important`: Reduce the `output/mod.rs` and `output/assemble.rs` mass

`output/mod.rs` (896) and `output/assemble.rs` (790) both contain
cross-cutting orchestration logic (validation, formatting, write
dispatch). These are likely the source of much of the cross-module
edge fan-out shown in `cross_module_edges=96`. Split orchestration
from concrete writers:

```
output/
  mod.rs           # generate_and_write entry; thin façade
  pipeline.rs      # validate → format → write orchestration
  files.rs         # per-file naming, path resolution
  assemble/
    api.rs         # api struct + variant + client assembly
    requests.rs    # request enum + per-endpoint structs
    types.rs       # type aliases / re-exports for response types
    docs.rs        # module doc comment generation
```

### `nice-to-have`: Consolidate test utilities

`schematic/gen/src/test_utils.rs` (230 lines) is `#[cfg(test)]` only
but lives in `lib.rs`. Several subsystems carry parallel fixture-builder
patterns inline. Promote shared `Endpoint`, `RestApi`, and `ApiResponse`
fixture builders into a shared `test_utils::fixtures` module to halve
the test-side LOC duplication and reduce churn when definitions evolve.

---

## schematic-oauth

**Files:** 5 · **LOC:** ~1,005 · **Notable size outliers:**
`manager.rs` (575) · `store.rs` (223) · `types.rs` (135) ·
`error.rs` (40) · `lib.rs` (32)

This is the smallest crate in the area and the cleanest by every
metric. The Gini concentration in `manager.rs` is moderate: it carries
~57% of the crate's LOC. There are no stubs, no shims, and the public
surface is well-organized.

### `important`: Split `manager.rs` by OAuth flow

`schematic/oauth/src/manager.rs` (575 lines) implements four distinct
flows (Authorization Code + PKCE, Client Credentials, Refresh,
Revocation) plus storage glue inside one impl block on `OAuth2Manager`.
Each flow is conceptually independent and has its own test surface.
Splitting reduces cognitive load and isolates the regions where future
changes are likely.

```
manager/
  mod.rs                  # OAuth2Manager struct + constructor
  authorization_code.rs   # begin_authorization, exchange_code (+ PKCE)
  client_credentials.rs   # acquire_client_credentials_token
  refresh.rs              # get_valid_token, refresh_token
  revocation.rs           # revoke_token
```

### `nice-to-have`: Add a `prelude` mirroring sibling crates

Both `schematic-define` and `schematic-schema` expose a
`prelude` module for ergonomic glob imports.
`schematic-oauth` does not. Adding `prelude` with `OAuth2Manager`,
`OAuthError`, `MemoryTokenStore`, `FileTokenStore`, `TokenStore`, and
`StoredTokens` re-exported brings the public-surface ergonomics in
line with the rest of the area at trivial cost.

### `nice-to-have`: Fold tiny `error.rs` and `types.rs` into `lib.rs`

At 40 and 135 lines, these files add module-graph nodes for very small
type definitions. If they continue to grow this is fine; if they remain
this small, consolidating into `lib.rs` (or `prelude.rs`) reduces
the import-edge fan-out without losing readability. Defer until
post-`manager.rs` split so the file count is rebalanced.

---

## schematic-schema

**Files:** 24 · **LOC:** ~41,580 · **Notable size outliers:**
`emqx.rs` (6,302) · `elevenlabs.rs` (3,749) · `ollama.rs` (2,771) ·
`gitlab.rs` (2,663) · `huggingface.rs` (2,591) · `eversolo.rs` (2,378) ·
`github.rs` (2,475) · `gitea.rs` (2,269) · `bitbucket.rs` (2,303) ·
`artificial_analysis.rs` (2,234) · `anthropic.rs` (1,161)

`schematic-schema` is **fully generated code** (`lib.rs:1` —
*"This code was automatically generated by schematic-gen. Do not edit
manually"*). Per-file LOC is therefore a property of the **generator**,
not the schema crate. The Gini-style equality penalty here is severe
(all top-10 files exceed 1,500 LOC, with `emqx.rs` at 6,302 LOC) but
it can only be addressed by changing what `schematic-gen` emits.

### `urgent`: Have the generator emit per-resource submodules per API

The generator currently writes one file per `RestApi` definition. With
APIs like EMQX, ElevenLabs, and Ollama containing 30+ endpoints and
hundreds of types, a single file is the wrong unit. Update
`schematic-gen`'s `output/` pipeline to emit a directory per API:

```
emqx/
  mod.rs            # client struct, variant builder, prelude re-exports
  requests.rs       # *Request structs and the unified request enum
  responses.rs      # *Response structs (today re-exported from definitions)
  client.rs         # impl block with the request/request_bytes/etc methods
```

This brings each generated file into the 500–1,500 LOC range typical
of hand-written modules elsewhere in the area, dramatically improves
compile parallelism for `schematic-schema`, and reduces the equality
spread without changing the generator's semantics.

### `important`: Promote shared client-impl boilerplate into `shared.rs`

`schematic/schema/src/shared.rs` (327 lines) already exists as a
common module. Inspect generated files for repeated patterns
(variant builder, env-auth resolution glue, error-mapping match arms)
and lift the truly-identical helpers into trait default methods or
free functions in `shared`. Each removed line is duplicated across
~17 generated modules, so the redundancy multiplier is large.

### `important`: Audit cross-API duplication in WS clients

`elevenlabs_ws.rs` (657), `unfolded_circle_core_ws.rs` (1,176),
`unfolded_circle_dock_ws.rs` (352),
`unfolded_circle_integration_ws.rs` (750), and
`samsung_smart_tv_remote_ws.rs` (553) likely share a substantial
amount of envelope/codec/lifecycle boilerplate. `ws_shared.rs` (298)
is the existing landing spot — pull common WS request/response
plumbing in and let the per-API files focus on message schemas.

### `nice-to-have`: Add a generation-provenance comment with a hash

Each generated file should carry the source `RestApi` definition's
content hash (e.g., blake3 of the post-canonicalization definition)
and the generator version. This makes drift detection trivial in CI
and makes regeneration idempotent under noop changes — directly
addressing the redundancy metric (no-op regenerations stop producing
diffs and the file becomes a true mirror of the definition).

```rust
// emqx.rs (top of file)
// generated-by: schematic-gen 0.4.2
// source-hash:  b3:7f0c2a91...      // blake3(define_emqx_basic_api() canonical form)
```

### `nice-to-have`: Verify `prelude.rs` covers every API

`schematic/schema/src/prelude.rs` (98 lines) is the glob-import
target. Each new generated module should be picked up automatically
by an `include!`-driven preamble or a build-script-emitted
`prelude.rs` so adding a provider does not require manual edits to a
hand-curated file. This is the same gap as
`schematic_definitions::apis_by_module()` and has the same fix shape.
