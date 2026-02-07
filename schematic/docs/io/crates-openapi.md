# OpenAPI crates for Schematic

## Schematic refresher (from repo READMEs)

- schematic-define: data-driven REST/WebSocket API definition types (RestApi, Endpoint, AuthStrategy, ApiRequest/ApiResponse, Schema).
- schematic-definitions: concrete API definitions that use schematic-define primitives.
- schematic-gen: generator CLI/library that transforms definitions into Rust client code; validates collisions and formats output.
- schematic-schema: generated client crate consumed by external libs.

## Crate reviews

### openapiv3
Functionality

- Data structures for OpenAPI 3.0.x; serde-friendly models for JSON/YAML.
- Build, parse, and serialize OpenAPI docs.

#### Main use cases

- Parse an existing OpenAPI 3.0 spec into Rust for analysis or transformation.
- Programmatically emit OpenAPI 3.0 docs.

Cargo features (default: none)

- `skip_serializing_defaults` (off by default): omit default-valued fields to reduce output size; avoid if consumers expect explicit defaults or you want fully explicit docs.

Gotchas and workarounds

- No validation layer; it models the spec only. Add your own validation pass or use a linter in CI.
- Many nodes are `ReferenceOr<T>`; you must resolve `$ref` yourself to avoid missing schemas.
- 3.0.x only. If you need 3.1 features (JSON Schema 2020-12), plan for a different crate or a custom extension layer.

### utoipa
Functionality

- Proc-macro driven, code-first OpenAPI generation.
- OpenAPI types and builders (OpenAPI 3.1).
- Framework integration helpers (actix-web, axum, rocket) and UI helpers via companion crates.

#### Main use cases

- Generate OpenAPI docs from Rust server handlers and DTOs.
- Serve Swagger UI/Redoc via companion crates.

Cargo features (default: `macros`)

- `macros` (default): enables derive/path macros; required for most usage.
- `actix_extras`: auto-parse actix-web path/query metadata; use only with actix-web.
- `axum_extras`: axum-specific parsing and helpers; use only with axum.
- `rocket_extras`: rocket-specific parsing and helpers; use only with rocket.
- `auto_into_responses`: enables auto response derivation for IntoResponses; use if you prefer inferred responses; avoid if you want fully explicit response specs.
- `chrono`: schema support for chrono types; enable if DTOs use chrono.
- `time`: schema support for time crate; enable if DTOs use time.
- `jiff_0_2`: schema support for jiff types; enable if DTOs use jiff.
- `decimal`: treat rust_decimal as string; use if you want decimal-as-string in docs.
- `decimal_float`: treat rust_decimal as number; use if you want numeric representation.
- `uuid`: schema support for uuid types; enable if DTOs use uuid.
- `ulid`: schema support for ulid types; enable if DTOs use ulid.
- `url`: schema support for url::Url; enable if DTOs use url.
- `smallvec`: map SmallVec to array types; enable if you use smallvec in DTOs.
- `indexmap`: schema support for IndexMap; enable if DTOs use it.
- `rc_schema`: schema support for Rc/Arc; enable if DTOs use them and serde rc is enabled.
- `repr`: support serde-repr style enums; enable if you use repr enums.
- `openapi_extensions`: helper traits for building request bodies and components; enable for convenience APIs.
- `config`: enables utoipa-config for global settings; use for centralized configuration.
- `debug`: add extra Debug derives to OpenAPI structures; useful for debugging output.
- `non_strict_integers`: allow nonstandard int formats like int8/uint32; use only if your API relies on them.
- `preserve_order`: preserve field order in schemas; use if diff stability matters.
- `preserve_path_order`: preserve path order in output; use if output diffs matter.
- `serde_norway`: internal serialization helper; usually only needed with `yaml`.
- `yaml`: emit YAML via serde_norway; enable only if you need YAML output.

Gotchas and workarounds

- Requires explicit registration of paths/types; no automatic discovery. Use helper crates like utoipauto or maintain a central `#[openapi(paths(...))]`.
- Some generic types are not supported as schema parameters; implement `ToSchema` manually for complex cases.
- Swagger UI 404 in debug builds with utoipa-swagger-ui due to RustEmbed defaults; build in release or enable its `debug-embed` feature.
- External types need manual `ToSchema` or wrapper types.

### okapi
Functionality

- OpenAPI 3.0 model types and tooling.
- Integrates with schemars to generate JSON Schema and OpenAPI components.
- Used by rocket_okapi and related ecosystems.

Main use cases

- Build or modify OpenAPI documents programmatically.
- Produce OpenAPI docs from schemars JsonSchema derivations.

Cargo features (default: none)

- `impl_json_schema`: enable schemars `JsonSchema` impls; use if you need to derive schemas for your types.
- `preserve_order`: preserve insertion order in maps; use if output stability/diffing matters.

Gotchas and workarounds

- Focused on schema modeling, not full client generation; pair it with a generator for client code.
- Limited high-level helpers; expect more manual OpenAPI object construction.
- Order is unstable without `preserve_order` if you rely on diff stability.

### rocket_okapi
Functionality

- OpenAPI generation and serving for Rocket apps.
- Bridges Rocket routes, request guards, and responses into OpenAPI via okapi/schemars.

Main use cases

- Rocket-specific OpenAPI generation and Swagger/RapiDoc serving.

Cargo features (default: `preserve_order`)

- `preserve_order` (default): stable ordering for schemas and maps; keep on for diff-friendly output.
- `swagger`: enable Swagger UI serving; disable if you do not serve docs.
- `rapidoc`: enable RapiDoc UI; disable if unused.
- `msgpack`, `mtls`, `secrets`, `uuid`: enable if you use the corresponding Rocket features.
- `rocket_db_pools`, `rocket_sync_db_pools`, `rocket_dyn_templates`, `rocket_ws`: enable if you use these Rocket subsystems.

Gotchas and workarounds

- Rocket-specific; not useful for non-Rocket or codegen-only pipelines.
- Output can vary based on enabled Rocket features; keep feature set minimal to control dependencies.

### paperclip

#### Functionality

- OpenAPI tooling and code generation for Rust, heavily focused on actix-web.
- Supports OpenAPI v2 and v3, route macros, and spec hosting.

#### Main use cases

- Actix-web apps that want OpenAPI docs and codegen.
- Mixed server/client generation in a single crate.

#### Cargo features (default: none)

- `v2`: enable OpenAPI v2 support; avoid if you only need v3.
- `v3`: enable OpenAPI v3 support; use for modern specs.
- `paperclip-actix`: actix-web integration; enable only for actix.
- `paperclip-macros`: derive and route macros; enable if using macro-based annotations.
- `actix-base`: base actix integration; required by actix2/3/4 features.
- `actix2`, `actix3`, `actix4`: pick the exact actix version you use; avoid others.
- `actix2-nightly`, `actix3-nightly`, `actix4-nightly`: only for nightly toolchains.
- `actix-files`, `actix-identity`, `actix-multipart`, `actix-session`: enable only if your app uses those actix features.
- `actix3-validator`, `actix4-validator`: validator integration for actix; enable only if you use validator.
- `path-in-definition`: include route path in macro definition; use for explicit path control.
- `codegen`: enable codegen support and templates; use when generating code artifacts.
- `cli`: enable codegen CLI (structopt, logging, git2); use only if you need CLI tooling.
- `cli-ng`: enable next-gen CLI components; use only if you are on the newer CLI path.
- `paperclip-ng`: enable next-gen components; use only if you are adopting paperclip-ng APIs.
- `openapiv3-paper`: use the paperclip fork of openapiv3; required for v3 features.
- `swagger-ui`, `rapidoc`: enable UI hosting; disable if you do not serve docs.
- `reqwest`: enable client-side generation helpers; use if generating clients.
- `serde_qs`: query string serialization; enable for complex query params.
- `http`: enable http types integration; use if your code references http types.
- `url`: URL type support; enable if DTOs use url::Url.
- `uuid`, `uuid0`, `uuid1`: UUID type support; choose the uuid version you use.
- `chrono`: chrono type support; enable if DTOs use chrono.
- `jiff01`: jiff type support; enable if DTOs use jiff 0.1.
- `rust_decimal`: rust_decimal type support; enable if DTOs use it.
- `camino`: camino path types; enable if used.
- `regex`: regex support in codegen/validation; enable if your schemas use regex patterns.
- `tinytemplate`, `heck`, `log`, `env_logger`, `git2`, `structopt`: internal CLI/codegen dependencies; enable only with `cli` or `codegen` paths.

#### Gotchas and workarounds

- Feature matrix is large and can be brittle; keep enabled features minimal and version-aligned with actix.
- Project notes that some capabilities are still under active development; test generated output carefully.
- If you only need spec modeling (not actix integration), openapiv3 or okapi are usually simpler.

### progenitor

#### Functionality

- Client generator for OpenAPI 3.0.x.
- Macro, build.rs, or CLI-based generation.
- Can also generate a CLI and httpmock helpers.

#### Main use cases

- Generate Rust clients from existing OpenAPI specs.
- Use builder or positional API styles for client ergonomics.

#### Cargo features (default: `macro`)

- `macro` (default): enables `generate_api!` macro; use for simple in-crate generation.
- Disable default features if you only want the build.rs or CLI flow and do not want macro deps.

#### Gotchas and workarounds

- Best supported for Dropshot-generated specs; may fail on unusual specs. Validate and patch specs when needed.
- Requires extra deps for specific formats (chrono, uuid, regex, websocket); add these when spec uses those types.
- Generates its own client style; not aligned with Schematic's request struct conventions.

### openapi-model-generator
Functionality

- CLI and library to generate Rust models from OpenAPI 3.0 specs.
- Supports JSON/YAML input, schema composition (allOf/oneOf/anyOf), and vendor extensions for Rust type overrides.

#### Main use cases

- Generate DTOs from OpenAPI to pair with a custom client.
- Use as a preprocessing step to build model types before client generation.

#### Cargo features (default: none)

- None.

#### Gotchas and workarounds

- Models only; no endpoint/client generation.
- Heavily driven by schema structure; verify naming output and consider vendor extensions (`x-rust-type`, `x-rust-attrs`) for customization.
- Documentation coverage is thin; rely on CLI tests or sample specs to validate output.

## Comparison matrix

| Crate | OpenAPI version | Primary role | Parse spec | Emit spec | Client generation | Best fit for Schematic export | Best fit for Schematic import |
| --- | --- | --- | --- | --- | --- | --- | --- |
| openapiv3 | 3.0.x | Spec model types | Yes | Yes | No | High | High |
| utoipa | 3.1 | Code-first spec generation | No | Yes | No | Low | Low |
| okapi | 3.0 | Spec model + schemars integration | Partial | Yes | No | Medium | Medium |
| rocket_okapi | 3.0 | Rocket spec generation/serving | No | Yes | No | Low | Low |
| paperclip | 2.0 + 3.0 | Actix-centric spec/codegen | Partial | Yes | Limited | Low | Low |
| progenitor | 3.0.x | Client generator | Yes | No | Yes | Low | Medium |
| openapi-model-generator | 3.0 | Model generator | Yes | No | No | Low | Medium |

## Recommendations for Schematic

### 1) Export OpenAPI during generation stage
Best choice: openapiv3.

Why

- Schematic is already data-driven (RestApi, Endpoint, Schema), which maps cleanly to OpenAPI objects.
- openapiv3 is a lightweight AST with serde, so we can build the spec directly from definitions and serialize to JSON/YAML.
- Minimal dependencies and no macro integration.

How it would fit

- Add an OpenAPI export phase alongside codegen: map RestApi to OpenAPI, Endpoint to Operation, and Schema to components.schemas.
- Optional: enable `skip_serializing_defaults` to shrink output if we only want explicit fields.

### 2) Import OpenAPI and generate a schematic/schema client
Best choice: openapiv3 as the parser, with openapi-model-generator as optional assistance.

Why

- openapiv3 gives a strongly-typed AST for OpenAPI 3.0 specs, which we can transform into schematic-define primitives.
- openapi-model-generator can produce DTOs from components.schemas, but Schematic still needs a custom mapping layer for endpoints and request/response wiring.

How it would fit

- Parse spec to `openapiv3::OpenAPI`.
- Convert paths/operations to RestApi + Endpoint, map parameters/body/response to ApiRequest and ApiResponse.
- Generate schema types either via custom schema extraction (staying in Schematic) or via openapi-model-generator for DTOs, then reference those types in the generated Schema values.

Fallback option

- If we want a direct client generator without aligning with Schematic conventions, progenitor can generate clients from OpenAPI directly, but it bypasses Schematic's data model and generator pipeline.
