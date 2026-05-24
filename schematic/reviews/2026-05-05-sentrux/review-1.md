---
title: Schematic Sentrux Structural Review
date: 2026-05-05
package_area: schematic
quality_signal: 0.72
coupling_score: 0.18
cycle_count: 0
max_depth: 1
complex_fn_count: 40
total_import_edges: 147
cross_module_edges: 96
suggestions: 23
suggestions_critical: 1
suggestions_urgent: 8
---

# Schematic Package Area – Sentrux Review

Baseline source: `schematic/.sentrux/baseline.json`.

Headline numbers (whole package area):

| Metric                  | Value   | Read                                                                   |
| ----------------------- | ------- | ---------------------------------------------------------------------- |
| `quality_signal`        | 0.72    | Mid-band — improvable.                                                 |
| `coupling_score`        | 0.18    | Low — healthy.                                                         |
| `cycle_count`           | 0       | Excellent — no circular dependencies (Martin 2003 acyclicity is met).  |
| `max_depth`             | 1       | Very shallow Lakos depth — risk of "flat-and-wide" instead of layered. |
| `complex_fn_count`      | 40      | Dominant negative signal across the area.                              |
| `total_import_edges`    | 147     | —                                                                      |
| `cross_module_edges`    | 96      | 65% cross-module ratio — Newman modularity is weak.                    |
| `god_file_count`        | 0       | Sentrux threshold not tripped, but multi-thousand-line files exist.    |
| `hotspot_count`         | 0       | —                                                                      |

How this maps to the five requested metrics:

1. **Modularity (Newman 2004)** — 65% of import edges cross module boundaries. Modules do not cluster well; many crates re-export through flat root modules. Highest leverage: `schematic-gen` (single big `output.rs` shared by every code-path) and `schematic-schema` (22 sibling modules with no grouping by transport).
2. **Acyclicity (Martin 2003)** — `cycle_count = 0`. Nothing critical; only one *latent* risk in `schematic-define` worth a guardrail.
3. **Depth (Lakos 1996)** — `max_depth = 1`. Counter-intuitively low: most public surface is re-exported at the crate root, so the layered "level graph" Lakos describes is collapsed. The fix is to introduce intentional layers (e.g. a `core` submodule in `define`, a `pipeline` in `gen`).
4. **Equality (Gini)** — File-size distribution is highly concentrated (e.g. `define/headers.rs` 2 005 LOC, `gen/output.rs` 2 166 LOC, `definitions/huggingface/types.rs` 2 587 LOC, `schema/emqx.rs` 6 352 LOC vs sibling files in the hundreds). High Gini → most logic lives in a few files.
5. **Redundancy (Kolmogorov)** — Hand-written redundancy is low; the generator emits structural redundancy (per-API request/response patterns in `schema/` and per-provider `types.rs` in `definitions/`). The fix is in the generator (shared traits / blanket impls), not in the generated output.

Suggestions are grouped by package below and ordered by priority.

---

## schematic-define

Files inspected: `auth.rs` (561), `headers.rs` (2 005), `models.rs` (951), `oauth.rs` (257), `params.rs` (1 561), `request.rs` (883), `response.rs` (197), `schema.rs` (187), `types.rs` (745), `websocket.rs` (1 279), `openapi/` subtree (~6 KLOC).

Sentrux signal: `define` is the central depended-upon crate. Its size imbalance and broad re-export surface drive the area-wide `cross_module_edges = 96`.

### `urgent`: Split `headers.rs` (2 005 LOC) by responsibility

**Problem.** `define/src/headers.rs` mixes four distinct concerns: `SensitiveString` (security primitive), `EnvList` / `ApiKeyEnv` / `EnvMapping` (env-var resolution), `Headers` builder (HTTP), and `HeaderError` (errors). This is the largest non-generated file in the area and a Gini-equality outlier; every consumer of any one type pulls a transitive edge to all four concerns. It also concentrates four sets of complex functions in one file, contributing to `complex_fn_count = 40`.

**Files touched.** `schematic/define/src/headers.rs`, `schematic/define/src/lib.rs` (re-exports), and any `crate::headers::*` import sites in `auth.rs`, `types.rs`, `request.rs`, `params.rs`, `websocket.rs`.

**Fix.** Convert `headers.rs` from a flat file into a submodule:

```text
schematic/define/src/headers/
├── mod.rs            # re-exports, public surface unchanged
├── sensitive.rs      # SensitiveString
├── env.rs            # EnvList, ApiKeyEnv, EnvMapping
├── builder.rs        # Headers
└── error.rs          # HeaderError
```

Keep the public re-exports in `lib.rs` byte-identical so downstream crates (`gen`, `oauth`, `definitions`, the generated `schema`) are unaffected. This raises Newman modularity (smaller, cohesive units) and improves the file-size Gini coefficient without changing the dependency graph.

### `urgent`: Split `params.rs` (1 561 LOC) along query / pagination boundaries

**Problem.** `define/src/params.rs` carries `EndpointParams`, `ParamDef`, `QueryParamType`, `ParamStyle`, `PaginationStyle`, and `PaginationResponse` — two unrelated topics (parameter definition vs. pagination semantics) in one file.

**Files touched.** `schematic/define/src/params.rs`, `schematic/define/src/lib.rs`.

**Fix.** Extract pagination types into `define/src/pagination.rs` (or a `params/pagination.rs` submodule). Re-export from `lib.rs` to preserve the public API. This both reduces the file's contribution to the Gini imbalance and gives `gen` and `definitions` a narrower import target when only pagination is needed.

### `urgent`: Reorganise `lib.rs` re-exports into layered groups

**Problem.** `define/src/lib.rs` flattens 30+ types at the crate root. Combined with `max_depth = 1`, this is the biggest reason Lakos depth never grows: every consumer imports from `schematic_define::*`, so the level graph collapses to one. It also masks Newman clusters.

**Files touched.** `schematic/define/src/lib.rs`, `schematic/define/src/prelude.rs`.

**Fix.** Keep the flat re-exports for backward compatibility, but additionally expose intentional layers consumers should prefer:

```rust
// lib.rs
pub mod core {
    pub use crate::auth::*;
    pub use crate::types::*;
    pub use crate::request::*;
    pub use crate::response::*;
    pub use crate::schema::*;
}
pub mod transport {
    pub use crate::headers::*;
    pub use crate::params::*;
    pub use crate::websocket::*;
}
pub mod model { pub use crate::models::*; }
```

Then migrate `gen`, `oauth`, and `definitions` to import from these submodules. Lakos depth grows from 1 to 2–3 without breaking anything.

### `important`: Add an `acyclicity` guardrail between `auth`, `headers`, `oauth`

**Problem.** `cycle_count = 0` today, but `auth.rs` and `headers.rs` already share types (`SensitiveString`, `EnvMapping`), and `oauth.rs` references both. As more OAuth flows land, it is easy to introduce `headers → auth` edges that close a triangle. This is a Martin-2003 risk, not a current violation.

**Files touched.** `schematic/define/src/auth.rs`, `schematic/define/src/headers.rs`, `schematic/define/src/oauth.rs`.

**Fix.** Add a `.sentrux/rules.toml` rule (or a CI doc-comment policy) declaring the intended layer order: `headers (lowest) → auth → oauth (highest)`. Codify it with a one-line architectural rule so a regression is caught at the next `sentrux scan`:

```toml
[[layer]]
name = "define-internal"
order = ["headers", "auth", "oauth"]
```

### `important`: Move OpenAPI extension structs out of feature-gated module

**Problem.** `define/src/openapi/extensions.rs` (471 LOC) defines `SchematicSchemaExtension`, the document- and operation-level extension structs. The skill notes the schema-level struct is **defined but not emitted**. That is dead structural surface (Kolmogorov redundancy): code with no consumer.

**Files touched.** `schematic/define/src/openapi/extensions.rs`, callers in `schematic/gen/src/openapi_output.rs`.

**Fix.** Either wire `SchematicSchemaExtension` into the exporter (preferred — gives a real consumer) or delete it and the supporting types until needed. Track the decision in `schematic/docs/io/openapi-extensions.md`.

### `nice-to-have`: Compact the `openapi/import/` sub-tree

**Problem.** `define/src/openapi/import/` has four files (`diagnostics.rs`, `naming.rs`, `mappings.rs` 1 284 LOC, `resolver.rs`). Newman-wise this is fine, but `mappings.rs` is the largest single file in the import path and likely contributes ≥10 of the 40 complex functions.

**Files touched.** `schematic/define/src/openapi/import/mappings.rs`.

**Fix.** Group mappings by OpenAPI element kind (`mappings/schema.rs`, `mappings/parameters.rs`, `mappings/responses.rs`) so each file targets one OpenAPI concept. No public-API change.

---

## schematic-definitions

Files inspected: 17 provider sub-modules under `definitions/src/`, plus `lib.rs` (245), `prelude.rs` (68), `registry.rs` (1 216).

Sentrux signal: redundancy and Gini concentration. Each provider has a `mod.rs` (endpoint definitions) and `types.rs` (request/response models). The largest `types.rs` is `huggingface/types.rs` (2 587 LOC).

### `urgent`: Split provider `types.rs` files larger than ~1 200 LOC

**Problem.** `huggingface/types.rs` (2 587), `elevenlabs/types.rs` (2 035), `gitlab/types.rs` (1 411), `bitbucket/types.rs` (1 365), `github/types.rs` (1 151), and `ollama/types.rs` (1 149) concentrate many model types per provider. This drives Gini inequality and (because every endpoint references several types from one giant file) inflates `cross_module_edges`.

**Files touched.** Each `definitions/src/<provider>/types.rs` listed above.

**Fix.** Split by API resource. Example for HuggingFace:

```text
definitions/src/huggingface/
├── mod.rs
└── types/
    ├── mod.rs           # re-export everything
    ├── models.rs        # model-discovery request/response
    ├── datasets.rs
    ├── repos.rs
    ├── spaces.rs
    └── shared.rs        # cross-cutting types
```

Each split file becomes the unit a Newman cluster can form around. Keep `pub use types::*` in `mod.rs` so the generator and `schema` crate are unaffected.

### `important`: Lift duplicated provider scaffolding into a generator macro or trait

**Problem.** Every provider sub-module repeats the same shape: `define_api()` → builds `RestApi` → declares endpoints inline. This is Kolmogorov-redundant: ~17 nearly-identical `define_api` shells. Today this is hand-edited; tomorrow it diverges.

**Files touched.** All `definitions/src/<provider>/mod.rs`.

**Fix.** Add a small builder helper in `define` (e.g. `RestApi::builder("Name")`) plus a documentation-by-convention pattern, **or** push more responsibility into the generator so providers describe data only. This reduces both the per-file LOC and the chance of drift between providers.

### `important`: Make `registry.rs` (1 216 LOC) data-driven

**Problem.** `definitions/src/registry.rs` likely enumerates every provider explicitly (one match arm or one slot per provider). That is a single point that grows linearly with `N_providers`, contributing to the file-size Gini.

**Files touched.** `schematic/definitions/src/registry.rs`.

**Fix.** Replace explicit enumeration with `inventory::submit!` or a `phf` map keyed by API name; let each provider sub-module register itself. New providers no longer touch `registry.rs`. Smaller diffs, cleaner Newman boundaries.

### `nice-to-have`: Add an `lib.rs` re-export policy doc-comment

**Problem.** `definitions/src/lib.rs` (245 LOC) re-exports many symbols; the rules for *which* are stable public types are implicit.

**Files touched.** `schematic/definitions/src/lib.rs`.

**Fix.** Add a module-level `//!` policy block (e.g. "only types backed by an OpenAPI/Postman source appear at the crate root; provider-internal helpers stay behind their `<provider>::` path"). Documentation, not code change.

---

## schematic-gen

Files inspected: `lib.rs` (86), `main.rs` (1 185), `output.rs` (2 166), `postman_output.rs` (1 951), `openapi_output.rs` (1 087), `asyncapi_import.rs` (856), `model_gen.rs` (636), `cargo_gen.rs` (413), `validation.rs` (411), `inference.rs` (275), `import_pipeline.rs` (245), `parser.rs` (130), `errors.rs` (60), `codegen/` subtree (~6.5 KLOC across 9 files), `ws_codegen/` subtree (~3 KLOC), `export/` subtree (~700 LOC).

Sentrux signal: this crate is the dominant source of `complex_fn_count`. `output.rs`, `postman_output.rs`, `openapi_output.rs`, `codegen/api_struct.rs` (1 381), `codegen/client.rs` (1 320), `codegen/request_structs.rs` (2 172), and `main.rs` are all >1 KLOC of imperative codegen.

### `critical`: Carve `output.rs` (2 166 LOC) into a pipeline

**Problem.** `gen/src/output.rs` is the top-of-pipeline module: it imports 11+ symbols from `codegen::`, owns `OutputOptions`, the WebSocket helper table, the snake-case helper, and final-file assembly + atomic write. It is simultaneously the highest-fan-in *and* highest-fan-out file in `gen`, and almost every cross-module edge in the crate touches it. It is also the leading contributor to `complex_fn_count`.

**Files touched.** `schematic/gen/src/output.rs`, `schematic/gen/src/lib.rs`, `schematic/gen/src/main.rs` (call sites).

**Fix.** Decompose into a small pipeline of single-purpose modules:

```text
gen/src/output/
├── mod.rs           # re-exports + public entry: write_api_module(...)
├── options.rs       # OutputOptions
├── ws_modules.rs    # WS_DEFINITION_MODULES table + helpers
├── assemble.rs      # generate_module_tokens(api, opts) -> TokenStream
├── format.rs        # syn validate + prettyplease formatting
└── write.rs         # atomic temp-file write
```

Each stage takes a typed input and returns a typed output. The result:

- `complex_fn_count` drops because long monolithic functions become short, named ones.
- Newman modularity rises (a clear cluster: assemble → format → write).
- Lakos depth grows because `lib.rs` no longer re-exports everything from a single root.

```rust
// gen/src/output/mod.rs
pub use options::OutputOptions;

pub fn write_api_module(api: &RestApi, dir: &Path, opts: OutputOptions)
    -> Result<(), GeneratorError>
{
    let tokens = assemble::module(api, &opts)?;
    let formatted = format::pretty(tokens)?;
    write::atomic(dir, &api.module_filename(), &formatted)
}
```

### `urgent`: Split `postman_output.rs` (1 951 LOC) by resource type

**Problem.** Same shape as `output.rs`: one file owns Postman collection assembly end-to-end. It is the second-largest file in `gen` and re-implements many of the patterns in `openapi_output.rs` (1 087 LOC).

**Files touched.** `schematic/gen/src/postman_output.rs`, `schematic/gen/src/openapi_output.rs`.

**Fix.** Extract a shared trait:

```rust
trait ExportFormat {
    fn render_collection(api: &RestApi) -> Result<String, GeneratorError>;
    fn extension() -> &'static str;
}
```

with implementations in `gen/src/export/postman.rs` and `gen/src/export/openapi.rs`. Move per-resource emitters (request body, auth block, path params) into `export/body.rs`, `export/auth.rs`, `export/path_params.rs` — files that already exist but are tiny (~120–320 LOC) and underused. Reduces Kolmogorov redundancy between the two exporters.

### `urgent`: Break `codegen/request_structs.rs` (2 172 LOC) by struct shape

**Problem.** This is the single largest file in `gen` and a dominant source of complex functions. It generates request-struct ASTs for *every* shape the generator supports (single-param, body-only, multipart, urlencoded, paginated, …) in one place.

**Files touched.** `schematic/gen/src/codegen/request_structs.rs`.

**Fix.** Split per request shape under `codegen/request_structs/`:

```text
codegen/request_structs/
├── mod.rs        # public entry: generate_request_struct_with_options(...)
├── single.rs     # `From<&str>` / `From<String>` ergonomics
├── body.rs       # `From<BodyType>` ergonomics
├── multipart.rs  # FormData/file fields
├── urlencoded.rs
├── paginated.rs
└── shared.rs
```

The dispatch in `mod.rs` becomes a small `match` on `ApiRequest` / `Endpoint::params` instead of a 2 KLOC monolith.

### `urgent`: Slim `main.rs` (1 185 LOC)

**Problem.** `gen/src/main.rs` is doing CLI parsing, orchestration, **and** real codegen logic. CLI binaries should be thin. A 1 185-LOC `main.rs` indicates the public API of the *library* half (`lib.rs` is only 86 LOC) is too narrow.

**Files touched.** `schematic/gen/src/main.rs`, `schematic/gen/src/lib.rs`.

**Fix.** Move every non-CLI function out of `main.rs` into the library (`lib.rs` re-exporting from new submodules `gen::commands`, `gen::pipeline`). Leave `main.rs` with `clap` parsing + a small `commands::dispatch(args)` call. Same effect on Newman modularity as the `output.rs` split.

### `important`: Promote `codegen::api_struct` (1 381) and `codegen::client` (1 320) to sub-modules

**Problem.** Both are siblings of the smaller files in `codegen/` and individually exceed the rest combined. Same Gini imbalance pattern.

**Files touched.** `schematic/gen/src/codegen/api_struct.rs`, `schematic/gen/src/codegen/client.rs`.

**Fix.** Convert each to a directory with one file per generated emitter (e.g. `client/methods.rs`, `client/builders.rs`, `client/variants.rs`). The public function signatures (`generate_api_struct`, etc.) remain unchanged; they become entry points in `mod.rs`.

### `important`: Reduce `complex_fn_count = 40` with extracted helpers

**Problem.** Sentrux flagged 40 complex functions across the area. Most live in `gen/`. Without specific function names from the scan, the leading suspects (by file size + responsibility) are large `match`-on-`Schema`/`TypeRef` arms in `model_gen.rs` (636), `inference.rs` (275), and `codegen/request_structs.rs`.

**Files touched.** `schematic/gen/src/model_gen.rs`, `schematic/gen/src/inference.rs`, `schematic/gen/src/codegen/request_structs.rs`.

**Fix.** Adopt a small refactor pattern:

1. Replace large `match` arms whose bodies exceed ~10 lines with a function call.
2. Replace nested `if let` ladders with `Option::map` / `?` chains.
3. Move per-arm helper closures into private free functions.

Re-run `sentrux scan` after each crate; aim for `complex_fn_count < 25`.

### `nice-to-have`: Document the `import_pipeline → output` data flow

**Problem.** The flow `import_pipeline.rs → parser.rs → inference.rs → model_gen.rs → codegen/* → output.rs` is real but undocumented; new contributors must read all 17 KLOC to discover it. This is a Lakos-depth observability problem.

**Files touched.** `schematic/gen/src/lib.rs`.

**Fix.** Add an ASCII diagram in the `lib.rs` `//!` block showing the stages. Documentation only, but it is the cheapest way to make the (currently flat) layered structure visible.

---

## schematic-oauth

Files inspected: `lib.rs` (32), `error.rs` (40), `manager.rs` (575), `store.rs` (223), `types.rs` (135).

Sentrux signal: this is the **best-shaped** crate in the area. Small (≈1 000 LOC), 5 files, even distribution, clean public API. Acyclicity, depth, and Gini all look good.

### `important`: Hide the `oauth2` crate behind a thin internal module

**Problem.** `manager.rs` (575) likely re-exports or thinly wraps `oauth2` types directly. If `oauth2` versions diverge from `reqwest` expectations elsewhere in the workspace (`reqwest 0.12` vs `homelab` 0.13.2 per memory), an upgrade ripples through every caller. This is a redundancy-of-coupling risk, not a Newman violation today.

**Files touched.** `schematic/oauth/src/manager.rs`, `schematic/oauth/src/types.rs`.

**Fix.** Add `oauth/src/external.rs` that owns every `use oauth2::*` and republishes only the types `manager.rs` needs. Future `oauth2` upgrades touch one file. Same pattern that the `homelab` integrations skill recommends for upstream HTTP libs.

### `nice-to-have`: Move `OAuth2RuntimeConfig` validation into a constructor

**Problem.** `types.rs` exposes plain structs; if any field invariant exists (URL well-formedness, scope non-empty), it is enforced at use-site, not construction-site. This is a Lakos-depth opportunity: validation belongs lower in the level graph than usage.

**Files touched.** `schematic/oauth/src/types.rs`.

**Fix.** Replace `pub struct OAuth2RuntimeConfig { … pub fields }` with a private struct + a `OAuth2RuntimeConfig::new(...) -> Result<Self, OAuthError>`. Existing callers update once; downstream gains compile-time guarantees.

---

## schematic-schema

Files inspected: 22 generated `*.rs` files including `emqx.rs` (6 352), `elevenlabs.rs` (3 777), `ollama.rs` (2 823), `gitlab.rs` (2 700), `huggingface.rs` (2 638), `github.rs` (2 533), `eversolo.rs` (2 401), `bitbucket.rs` (2 345), `gitea.rs` (2 321), `unfolded_circle_core_rest.rs` (1 692), `samsung_smart_tv.rs` (1 387), `lmstudio.rs` (1 350), `unfolded_circle_core_ws.rs` (1 266), `anthropic.rs` (1 192), `openai.rs` (1 099), `unfolded_circle_integration_ws.rs` (804), `elevenlabs_ws.rs` (705), `samsung_smart_tv_remote_ws.rs` (595), `unfolded_circle_dock_ws.rs` (372), `shared.rs` (335), `ws_shared.rs` (303), `prelude.rs` (92), `lib.rs` (120).

Sentrux signal: extreme Gini concentration (`emqx.rs` is ~70× larger than `prelude.rs`) and high redundancy (every per-API file repeats the same generated shape). **Important caveat:** this crate is auto-generated by `schematic-gen` and the `schematic/schema/Cargo.toml` is excluded from the workspace. Fixes belong upstream in the generator.

### `urgent`: Have the generator emit per-resource sub-modules instead of one file per API

**Problem.** `schema/src/emqx.rs` is 6 352 LOC. Even if Sentrux's god-file threshold is not tripped, IDE/rust-analyzer load and incremental compile times for any change to a single endpoint are dominated by this file.

**Files touched (in the generator).** `schematic/gen/src/output.rs`, `schematic/gen/src/codegen/client.rs`. The output target is `schematic/schema/src/<api>/`.

**Fix.** Emit:

```text
schema/src/emqx/
├── mod.rs            # client struct + method dispatch
├── requests.rs       # request structs
├── responses.rs      # response types (or split per resource)
└── auth.rs           # variant() / variant_with() builders
```

`mod.rs` re-exports everything currently re-exported from `emqx.rs`, so consumers see no API change. After regeneration, `lib.rs`'s `pub mod emqx;` keeps working. Improves both Gini and Newman scores in one pass.

### `important`: Group sibling modules by transport

**Problem.** `lib.rs` declares 22 `pub mod` siblings; REST and WebSocket modules are mixed. The "Available REST APIs" / "Available WebSocket Definitions" tables in the doc-comment already describe this grouping conceptually, but the module tree does not reflect it.

**Files touched (in the generator).** `schematic/gen/src/output.rs` (the section that emits `lib.rs`).

**Fix.** Emit:

```rust
// generated lib.rs
pub mod rest {
    pub mod anthropic;
    pub mod openai;
    // ...
}
pub mod ws {
    pub mod elevenlabs_ws;
    pub mod unfolded_circle_core_ws;
    // ...
}
// keep flat re-exports for back-compat for one release:
pub use rest::*;
pub use ws::*;
```

Lakos depth on the consumer side rises from 1 to 2; Newman modularity rises because the import graph now shows the transport split.

### `important`: Factor cross-API patterns into `shared.rs` / `ws_shared.rs`

**Problem.** `shared.rs` (335) and `ws_shared.rs` (303) already exist and host `SchematicError` and a few traits, but every per-API file still re-emits identical patterns: pagination iterators, `From<&str>` impls on single-param requests, `#[must_use]` on async fns, `DOCS_URL` consts. This is Kolmogorov-redundant generation.

**Files touched (in the generator).** `schematic/gen/src/codegen/api_struct.rs`, `schematic/gen/src/codegen/request_structs.rs`, `schematic/gen/src/codegen/client.rs`.

**Fix.** Promote the repeated patterns to traits in `schema/src/shared.rs`:

```rust
pub trait DocsUrl { const DOCS_URL: &'static str; }
pub trait Paginated { type Cursor; fn cursor(&self) -> Option<Self::Cursor>; }
```

Generate `impl DocsUrl for OpenAI { const DOCS_URL: &'static str = "..."; }` instead of inlining the constant in each module. Reduces total schema LOC, raises Newman cohesion, and lowers Kolmogorov complexity of the generated bundle.

### `nice-to-have`: Add `#[allow(clippy::too_many_lines)]` only at module scope, never per-fn

**Problem.** As generated files shrink (after the previous fixes), per-function `#[allow]` annotations may become misleading.

**Files touched (in the generator).** `schematic/gen/src/codegen/*`.

**Fix.** Emit any clippy escape hatches at the top of `mod.rs` (or omit them entirely once functions are short enough). No behaviour change; cleaner reads.

---

## Summary

The structure-of-the-area story Sentrux is telling:

- **Acyclicity** is excellent — preserve it with one rule (define-internal layer order).
- **Depth** is artificially low because crate roots flatten everything; layered submodules in `define` and `gen` would lift it without API churn.
- **Modularity** suffers from a few oversize files acting as fan-in/fan-out hubs (`define/headers.rs`, `gen/output.rs`, `gen/postman_output.rs`, `gen/codegen/request_structs.rs`, `definitions/<provider>/types.rs`, `schema/emqx.rs`).
- **Equality** is the same problem from a Gini lens — split the giants.
- **Redundancy** is strongest in the *generator's output*, so the highest-leverage fix is shared traits in `schema/shared.rs` plus per-resource emission, not changes to the generated files themselves.

The highest-ROI single change is the **critical** one: carve `gen/src/output.rs` into a pipeline. It directly attacks `complex_fn_count`, raises Newman modularity, lifts Lakos depth, and is a precondition for the other generator-side fixes that improve `schema/`.
