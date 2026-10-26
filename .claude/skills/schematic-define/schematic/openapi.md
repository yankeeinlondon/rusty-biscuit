# OpenAPI Support

Schematic supports bidirectional OpenAPI 3.x integration: import OpenAPI specs to generate Rust clients, or export existing API definitions to OpenAPI format.

**Feature Gate**: OpenAPI functionality requires the `openapi` feature in `schematic-define`.

## Import OpenAPI Specs

Transform any OpenAPI 3.x specification into a type-safe Rust client.

### CLI Usage

```bash
# Basic import
schematic-gen import --input petstore.yaml --output generated/src

# With custom API name
schematic-gen import --input api.json --api-name MyPetStore --output src

# Strict mode (fail on warnings)
schematic-gen import --input api.yaml --output src --strict

# Dry run (preview without writing)
schematic-gen import --input api.yaml --output src --dry-run
```

### Import Options

| Option | Description |
|--------|-------------|
| `--input` | Path to OpenAPI spec file (JSON or YAML) |
| `--api-name` | Override API name (default: derived from spec title) |
| `--module-path` | Override module path for generated code (wins over `x-schematic.module_path` only when given) |
| `--output` | Output directory for generated code |
| `--dry-run` | Preview generated code without writing files |
| `--strict` | Fail on any WARN or ERROR diagnostic (INFO never fails) |
| `--definitions-out` | Write a definitions-crate module (`mod.rs` with `define_{api}_api()` and the fallible `module_group()`, plus `types.rs`) instead of a standalone client |
| `--overlay <RUST_PATH>` | With `--definitions-out`: a hand-written `fn(RestApi) -> RestApi` the factory applies, so additions survive a re-import (OpenAI: `crate::openai_pagination::apply`) |
| `--exclude-path <PREFIX>` | Drop endpoints by path prefix (repeatable) |
| `--request-id-header <HEADER>` | Set `RestApi::request_id_header`; overrides `x-schematic.request_id_header` only when given (`just import-openai` passes `x-request-id`) |

### Library API

```rust
use schematic_define::openapi::{OpenApiImport, OpenApiSource};
use schematic_gen::import_pipeline::{run_import, ImportOptions};

// Option 1: High-level pipeline
let options = ImportOptions {
    input: "api.yaml".to_string(),
    api_name: Some("MyApi".to_string()),
    module_path: None,
    output: "generated/src".to_string(),
    definitions_out: None,
    overlay: None,
    exclude_paths: vec![],
    request_id_header: None,
    dry_run: false,
    strict: false,
    verbose: true,
};
let result = run_import(&options)?;

// Option 2: Low-level builder
let source = OpenApiSource::from_file("api.yaml")?;
let result = OpenApiImport::new(source)
    .api_name("MyApi")
    .prefer_json()
    .strict()
    .build()?;

println!("Generated {} endpoints, {} models",
    result.api.endpoints.len(),
    result.models.models.len());
```

`result.api` carries one schema component per imported model and passes
`validate_references()`; an operation whose JSON body or response names an
undefined schema fails the import with `OpenApiError::Definition`. See
`schematic/docs/io/import-openapi.md`.

### Responses

Every 2xx response becomes a success shape (`text/event-stream` an SSE
stream, `application/x-ndjson`/`application/jsonl` an NDJSON stream of the
schema's component) and every exact `400`-`599`, `4XX`, or `5XX` response an
endpoint error entry (never hoisted to the API, unless `x-schematic.errors`
restores the API defaults). Stream end conditions cannot be read from
standard OpenAPI, so imported streams end at connection close unless the
operation's `x-schematic.success` restores them. Bodies and responses with no named schema
become `serde_json::Value` (`TypeRef::Primitive(Json)`); render that type path
with `syn` parsing, never `format_ident!`.

A JSON `oneOf` separable by `discriminator` or array/object becomes one tagged
shape per member whether inline or behind a schema `$ref` (chains resolved).
Tags use the same values as the union's dispatching `Deserialize` (mapping
key, single `enum` value, component name); each shape's `variant` is the
member's component name (`Array`/`Object` when unnamed). An inseparable
referenced union keeps `Json(Named)` with a warning; error statuses keep one
shape (a referenced union stays named). The OpenAI overlay
(`definitions/src/openai_pagination.rs`) restores `GetConversationItem` to
`ConversationItem` so the public type does not change.

### Imported builder assemblies

`--definitions-out` emits fallible `Endpoint`/`RestApi` builder code and a
public `module_group() -> Result<Vec<RestApi>, DefinitionError>` over one
ordered `DefinitionContext`; catalog factories unwrap it. Nominal generated
models implement `Registration`; the emitter bounds stack use with small
endpoint helpers and batched registrations. The importer normalizes
query-bearing paths (OpenAI's seven beta operations share their ordinary ids
with an optional `beta` query input), rejects GET bodies, duplicate keys, and
malformed input. It never clamps or narrows a value rule: numeric keyword
text is captured from the source before `openapiv3` parses it (an
out-of-`i64` bound gets an `i64` stand-in only so the document parses), a
stored component schema keeps a value no JSON number holds exactly in an
`x-schematic-exact` member that `openapi::serialize` writes back, Boolean
3.0 exclusive bounds become numeric ones, and `const` is kept. Rules land in
`ParamDef`/`FormField`/`request_constraints`; see
`schematic/docs/io/import-openapi.md#value-constraints`. A malformed or
contradictory rule (`schema_rule_problems`: low bound above high, empty
interval, `multipleOf <= 0`, empty `enum`, invalid pattern) in any parameter,
body, form field, or component fails the import in either mode
(`OpenApiError::Validation` at the keyword; first one recorded via
`RefResolver::reject`). Only a rule that does not fit its input's type is
dropped, one keyword at a time, with a WARN. The source capture also records
the 3.1 subschema keywords `openapiv3` drops (`DROPPED_KEYWORDS`: `if`,
`prefixItems`, `contains`, ...) so they are warned, not lost: a parameter,
form field, or body rule warns its own, and the builder warns every other
captured pointer (components, nested body and form schemas, responses).
`--definitions-out` re-emits them: endpoint rules as `Constraints` builder
calls (numbers as text), model rules as `#[schemars(extend(..))]` read from
the stored component schema (never native `length`/`range`, which drop
silently on mismatched types): per field, and per model for root and
extra-property (`additionalProperties`) rules. The Rust type picks where a
rule goes, and never whether it stays: `format` (annotation), map values and
array items (over the Schemars slot schema, `$ref` for a model), a rule
beside a model `$ref` (written on the field, so Schemars' `RemoveRefSiblings`
makes a one-member `allOf` that `unenforceable_rules` and the value-rule
plans treat as the reference), and every nested `properties`/`items`/
`additionalProperties` rule of a `serde_json::Value` field, untyped. Beneath
a `serde_json::Value` field, a `$ref` to a model with rules stays a
reference (written by a `schema_with` function calling
`generator.subschema_for::<Model>()`, so the model is registered wherever the
type is reached). The two wrappers the value-rule plans pass through are
rebuilt in source shape when their member has rules: a nullable alternative
(two-member `anyOf`, exactly one member `admits_only_null`, copied as
written) and a one-member `allOf` without a sibling `$ref`; nested wrappers
recurse. Any other `allOf`/`anyOf`/`oneOf`/`not` with a rule or ruled
`$ref` beneath it is kept too (`RuleSource::composed_members`): every
member in source order and count (pointers match the direct import), each
rebuilt free-form plus its `type`/`nullable` and a `$ref` even to an
unruled model (`composition_structure`); one with no rule beneath is left
out. Plans never enter it, so the rebuilt API errors on its rules, or warns
and discloses them with `accept_unchecked_compositions()`, as the direct
import does. Component-root compositions too: a `oneOf`/`anyOf` enum's
variants get their member's rules (`variant_rules`; `extend`/`schema_with`
on the variant, a null-only member's `type: "null"` beside a ruled variant;
Schemars writes `anyOf`, so a `oneOf` component's pointer reads `/anyOf/i`)
and a flattened `allOf` struct gets the source `allOf` as a container
`extend` (`alias_attribute`, model members as literal `$ref`s; the merged
properties stay, so a member's rule is also enforced there, or reported a
second time when that property is itself beneath an unchecked composition:
rebuilt findings are a documented superset of the direct import's, e.g.
OpenAI chat-completion and create-response +8 warnings each). A
rebuilt nullable `anyOf [{$ref}, null]` registers byte-identical to
Schemars' own `Option<Model>`, so `definition_baseline` undoes it, and the
generic compositions, only on fields the importer types as free-form
(through plain aliases too), plus the union-variant and `allOf`-struct root
rules (`NominalCompositions`). `--definitions-out` fails the
import (`RuleSource::finish`, before writing) for a ruled plain alias
reached again through a free-form value, and for a plain-alias JSON body
whose rules reach past root and item rules (`properties`,
`additionalProperties`, a composition, a ruled `$ref`), since `Constraints` cannot hold
them; the fix is naming the item/object schema. Export writes every rule in 3.0 spelling,
including an adopted page size's strategy bounds. An imported page-size
parameter's source bounds must agree with the overlay's strategy
(`openai_pagination::align_page_size_bounds`). Acceptance:
`define` `constraint_preservation` (import/export) and `gen`
`constraint_preservation::slow_definitions_out_rebuilds_every_rule_through_registration`.
OpenAI is regenerated through `just -f schematic/justfile import-openai`,
never edited by hand;
`import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`
fails when the committed module and a fresh import differ.

### Type Mappings

| OpenAPI Type | Rust Type |
|--------------|-----------|
| `string` | `String` |
| `string` + `format: date-time` | `String` (or chrono types) |
| `integer` | `i64` |
| `integer` + `format: int32` | `i32` |
| `number` | `f64` |
| `boolean` | `bool` |
| `array` | `Vec<T>` |
| `object` | Named struct |
| `object` + `additionalProperties` | `HashMap<String, T>` |
| `oneOf`/`anyOf` | Enum; decoded by its `discriminator` when every member has a tag value, else untagged |
| `$ref` | Named type reference |

### Diagnostics

Import produces diagnostics for potential issues:

```rust
pub enum DiagnosticSeverity {
    Info,   // Informational messages
    Warn,   // Non-blocking issues
    Error,  // Blocking issues (fail in strict mode)
}

pub struct OpenApiDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub location: String,  // RFC 6901 pointer, e.g. `#/x-schematic/auth`
}
```

Strict mode fails on any WARN or ERROR, never on INFO (pinned by
`build_strict_mode_*` in `import/builder.rs` and
`strict_import_passes_an_info_only_document`).

## Export to OpenAPI

Generate OpenAPI 3.0.3 specs from existing Schematic API definitions.

### CLI Usage

```bash
# Export as JSON (default)
schematic-gen generate --api openai --openapi-out specs/

# Export as YAML
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml

# Generate all with OpenAPI export
schematic-gen generate --api all --openapi-out specs/
```

### Library API

```rust
use schematic_define::openapi::{export, ExportOptions, ExportFormat};
use schematic_definitions::openai::define_openai_api;

// Components come from `api.schemas`; there is no separate registry argument.
let api = define_openai_api();

let options = ExportOptions::new()
    .with_version("1.0.0")
    .with_format(ExportFormat::Yaml);

let openapi_doc = export(&api, &options)?;
```

Component schemas convert faithfully to OAS 3.0.3 (`define/src/openapi/export/conversion.rs`;
rules and examples in `schematic/docs/io/export-openapi.md#how-component-schemas-convert`):

- Both schemars dialects convert: root `nullable`, nested 2020-12 `type: [T, "null"]`,
  `const` (→ one-value `enum`), `{type: "null"}` members.
- Nullable `$ref`/composition → `anyOf: [X, {type: object, nullable: true, enum: [null]}]`;
  an `enum` keeps its members (a nullable type never adds `null`; only a merged
  `anyOf` member beside a null alternative does); several types → `anyOf` (never `oneOf`).
- A bare `$ref` stays bare; one with siblings becomes `allOf: [{$ref}]` + siblings.
- `"default": null` is dropped (openapiv3 reparse would lose it anyway).
- An inexpressible keyword fails export with `OpenApiError::UnsupportedFeature`
  and its JSON pointer. Export also runs `validate_schema_closure` first.
- Gates: `schematic-definitions` tests `schema_fidelity` (zero lost/typed-loss
  positions over 18 registries) and `schema_values` (jsonschema accepts/rejects).

### Standard import mappings

- Path-item `parameters` merge into each operation (identity: name + `in`,
  header names case-insensitive; operation wins in place). Local
  `#/components/parameters` refs resolve first; unresolved or remote refs are
  ERROR at their pointer, never fetched.
- Typed path params import only when some path param is non-string,
  described, or deprecated (then every placeholder, template order). A plain
  undescribed string path param imports as an undeclared placeholder.
- Operation/parameter `deprecated`, `info.version` (into `RestApi::version`)
  are read. No `servers` → WARN, base URL `https://api.example.com`.
- ERROR: empty `info.version`/`servers[0].url`/`externalDocs.url`, a
  non-required path param, empty `requestBody.content`, `apiKey` with empty
  `name`. `security: null` fails the import.
- Multipart: `encoding.<part>.contentType` → file `accept` (comma-split) or a
  JSON part; `minItems`/`maxItems` → file counts. A primitive part (or a
  component/composition of primitives) is text silently; an object-valued part
  without JSON encoding is WARN and stays text; unresolved part refs ERROR;
  encoding `headers`/`style`/`explode`/`allowReserved` WARN.
- Auth: root `security` is authoritative (none/one/several → anonymous /
  `AuthStrategy` / `AuthPolicy`); absent root `security` falls back to the
  first component scheme. Operation `security`: absent inherits, `[]` public,
  equal to root (no op details) inherits, otherwise `EndpointAuth::Policy`.

### Schematic Extensions

Export writes `x-schematic` at the document root and on each operation, and
the importer **reads it back** (`import/extension.rs` decodes,
`import/reconcile.rs` applies). Writer shapes: `SchematicDocExtension`,
`SchematicOpExtension`, `AuthDetail`, `ParamDetail` (serialize-only;
`EXTENSION_VERSION = 1`).

```yaml
x-schematic:                     # document
  version: 1                     # only here; absent = 1; anything else disables all extensions (one WARN)
  module_path: gitea
  env_auth: ["GITEA_TOKEN"]      # env var names, never credentials
  auth: { apiKeyAuth: { value_prefix: "token " } }   # or { bearer: true }; nothing else allowed
  secret_names: ["x-session"]
  # also: request_suffix, env_mapping, headers, rate_limit, retry_guidance,
  #       errors, cross_origin, request_id_header
paths:
  /models/{repo_id}:
    get:
      x-schematic:               # operation
        response: { Json: { Named: Model } }   # legacy, always written
        path_template: /models/{+repo_id}      # only when the path has {+name}
        params: { query: { api-key: { secret: true }, type: { rust_name: kind } } }
        # also: request (form bodies only), success, errors, no_declared_errors,
        #       headers, rate_limit, retry_guidance, cross_origin,
        #       auth + env_mapping (only with an operation `security` override)
```

Reader rules:

- Each member decodes on its own. Unknown member → INFO. Malformed, failing
  `validate_api`'s checks, or explicit `null` → WARN at an RFC 6901 pointer
  (`#/x-schematic/<member>…`, `#/paths/<escaped>/<method>/x-schematic/<member>…`),
  member dropped, siblings kept. Allowed nulls: `env_mapping`'s
  `bearer_token`/`basic_user`/`basic_pass`/`api_key`, form field
  `description`, `Files.min`/`max` (the writer emits them).
- Standard wins: `success`/`response`/`errors` are re-exported with
  `map_responses` and compared with the standard `responses` (descriptions
  ignored); mismatch → WARN, standard-derived value kept. Conflicting document
  `errors` are withdrawn as a whole with one WARN. A form `request` only fills
  `accept`, file counts, and a JSON part's reference. Legacy JSON/text/binary
  `request` is compared only.
- Auth details apply only to an `apiKey` header scheme the relevant
  `security` applies; else WARN. Fixed headers exported as single-value-enum
  header parameters are recognized and not re-imported as parameters.
- Does not round-trip: pagination (`default_pagination`, endpoint
  `pagination`, `pager_managed`), GraphQL, component Rust paths (rebuilt as
  `{module_path}::{Name}`), the explicit/env-fallback split of a hand-written
  `AuthPolicy`, OAuth2 details beyond standard flows, grouped documents beyond
  the first API's metadata and `info.title`, a declared undescribed
  plain-string path param.

Export side: root `security` has one requirement per method of
`effective_auth_policy()`; multipart writes `encoding` and file
`minItems`/`maxItems`; `explode: true` is written only for non-`form` styles;
JSON shapes hold the stored `TypeRef`, never a Rust path. GraphQL operations
are not exported (`graphql-operation` warning).

Tests that pin behavior: `define/tests/openapi_extension_matrix.rs` (every
member × absent/null/wrong/empty, JSON and YAML),
`openapi_standard_matrix.rs`, `openapi_import_parameters.rs`,
`openapi_multipart_import.rs`, `response_openapi.rs`, and
`gen/tests/l1/extension_round_trip.rs` (every committed `schematic/openapi/`
artifact imports strictly and re-exports with only listed differences). Docs:
`schematic/docs/io/openapi-extensions.md`.

Fixed headers (`.header(name, value)`, API or endpoint) also export as
standard parameters: one required header parameter per operation with
`schema: {type: string, enum: [value], default: value}`, endpoint over API by
case-insensitive name. Left out, and reported as a `fixed-header` warning
(`openapi::header_gaps` lists the same): a header the operation's effective
security sends, `Accept`/`Content-Type` values its content does not declare,
`Authorization`, explicit reserved-name parameters, and a fixed value an
explicit header parameter's schema rejects (a compatible one merges into it:
required, value as default). GitHub's `Accept: application/vnd.github+json`
and Gitea's raw-content `Accept` are the pinned shipped gaps. See
`schematic/docs/io/export-openapi.md#fixed-headers`.

## Feature Configuration

In `Cargo.toml`:

```toml
[dependencies]
schematic-define = { path = "../define", features = ["openapi"] }
```

The `openapi` feature enables:
- `openapi` module with import/export functions
- OpenAPI 3.x parsing via `openapiv3` crate
- YAML support via `serde_yaml`
