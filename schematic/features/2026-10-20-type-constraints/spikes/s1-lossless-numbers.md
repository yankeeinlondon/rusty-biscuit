# Spike S1 – Lossless number path (host: macOS, 2026-10-23)

Scratch crate: /tmp/tc-spike-s1 (serde_json =1.0.149, openapiv3 =2.2.0, serde_yaml_ng =0.10.0, yaml-rust =0.4.5,
`normalize.rs` included verbatim via `#[path]`). All rows are observed output, not inferred.

## 1. Loss table (`maximum` and `multipleOf` behave identically)

| Stage | 9223372036854775808 (2^63) | 1e30 | 0.1 |
|---|---|---|---|
| serde_json::Value round trip (JSON) | exact (u64; `as_u64` Some, `as_i64` None) | text changes to `1e+30`; stored as f64 | f64; renders `0.1` |
| serde_yaml_ng -> serde_json::Value | exact (u64) | f64, renders `1e+30` | f64, renders `0.1` |
| `clamp_numeric_bounds`, key at line start (YAML or pretty JSON) | **LOST: rewritten to 9223372036854775807**, 1 warning, for `type: number` too | untouched (not an integer literal) | untouched |
| `clamp_numeric_bounds`, minified/inline JSON (`{"type":..,"maximum":..}`) | untouched (key must start the line) | untouched | untouched |
| openapiv3 parse, `type: integer` | not i64, so falls through to **AnySchema f64** (9.223372036854776e18) | Any, f64 | Any, f64 |
| openapiv3 parse, `type: number` | NumberType f64 | f64 | f64 |
| openapiv3 serialize, JSON / YAML | **LOST in text**: `9.223372036854776e+18` / `9.223372036854776e18` (reads back as ...776000) | `1e+30` / `1e30` | `0.1` |
| importer path, integer, after clamp | IntegerType i64 = i64::MAX (off by one) | Any f64 | Any f64 |

Extra probes: `0.10000000000000000001` becomes `0.1` at every serde stage (JSON Value, YAML, openapiv3). `100000000000000000000`
becomes `1e+20` in serde_json; via YAML, > u64 / < i64::MIN errors (`invalid type: integer ... as u128/i128`).
`1e30` and `0.1` only seem to survive because ryu prints the shortest repr. The f64 behind them is not 10^30 or 1/10, and
any value with more than about 17 significant digits is silently changed.

## 2. Code locations
- Pre-pass: `define/src/openapi/import/normalize.rs:50` (`clamp_numeric_bounds`). `:94` `match_key` requires the key at line start.
  `:98` integer-literal-only check. `:104` skips values that fit i64. `:108-112` clamps to i64::MIN/MAX whatever the schema `type`.
- Parse: `define/src/openapi/import/builder.rs:236` `parse_source`. Each text arm runs `Self::normalize` (`:250/:260/:270/:281`),
  then `query_paths::check_text` (`:251..`, builds a throwaway `serde_json::Value`, `query_paths.rs:281`), then a direct parse into
  `openapiv3::OpenAPI`: **serde_yaml_ng** for Path/Yaml/Bytes (`:252/:272/:283`), **serde_json** for Json (`:262`).
  `OpenApiSource::Document` (`:288`) arrives already typed, so its loss happened earlier.
- Retained source components: `define/src/openapi/import/mappings/schema.rs:89`
  `serde_json::to_value(schema_ref)` from the **already parsed openapiv3 schema**, stored as `SchemaComponent.schema: serde_json::Value`
  (`define/src/components.rs:25`). The retained copy is therefore post-clamp and post-f64. It is not raw text.
- openapiv3 2.2.0 `src/schema.rs`: raw fields are `serde_json::Number` (`:84-92`). IntegerType `minimum/maximum/multiple_of: Option<i64>`,
  `exclusive_*: bool` (`:680-692`, chosen only if `is_i64`, `:532`). NumberType all `Option<f64>`, `exclusive_*: bool` (`:661-673`).
  AnySchema `Option<f64>` with `exclusive_*: Option<bool>` (`:593-607`).
- Export: `define/src/openapi/export/components.rs:34` -> `conversion.rs:62` `convert_component_schema`, a Value->Value convert
  (numeric 2020-12 `exclusiveMinimum` becomes bound plus `true`, `:151`), then `serde_json::from_value::<openapiv3::Schema>` (`:70`).
  Rendering: `define/src/openapi/options.rs:83-90` (`serde_json::to_string_pretty` / `serde_yaml_ng::to_string` of `openapiv3::OpenAPI`).
  A schemars `maximum: 18446744073709551615` exports as `1.8446744073709552e+19`.

## 3. Strategy: (b) out-of-band exact decimal text. Default confirmed.
- (a) is rejected. No workspace Cargo.toml enables `arbitrary_precision`. Enabling it would also not be enough: openapiv3 converts to
  i64/f64 on its own (`:207,232-236`) whatever serde_json does, and serde_yaml_ng hands numbers to visitors as u64/i64/f64/i128.
  The loss is in openapiv3 and the YAML visitor, not just in `Value`.
- (b) capture: in `parse_source`, from the **raw `content` before `Self::normalize`**. That is the only point where the text is still
  exact. Build a pointer-keyed sidecar `{json_pointer -> decimal text}` for `minimum/maximum/exclusiveM*/multipleOf` (and
  enum/const if Rule 3 needs them), using parsers already in the dependency graph:
  - JSON: `serde_json::value::RawValue`. The `raw_value` feature is **already unified** in the workspace (`schematic/definitions`,
    `homelab/unfolded-integration-helper`). Recurse via `Map<String, Box<RawValue>>` / `Vec<Box<RawValue>>`. Observed exact for all
    probes, including `0.10000000000000000001`.
  - YAML: `yaml-rust` 0.4.5 is in Cargo.lock (via syntect/serde_yaml). `Yaml::Real(String)` kept the exact text of 2^63, 1e30, 0.1,
    `0.10000000000000000001` and 1e20 in the probe. Adding a direct dep on it (or on another event-level YAML parser) is the only
    new dependency. serde_yaml_ng exposes no scalar text.
  - The sidecar then follows the schema into `source_component` (`schema.rs:89`) and into the new constraint metadata as decimal
    strings. A clamp warning is no longer a value change for the metadata.
- (b) patch: after `serialize` (`options.rs:83`), or better on the `serde_json::Value` of each component before
  `from_value` (`conversion.rs:70`), keep a component-pointer sidecar. Rewrite the rendered numeric token at that pointer to the
  decimal text. For JSON output that is easy: serialize the document to `serde_json::Value`, write the exact numbers as `RawValue`, and
  print. For YAML output it means a text patch on the emitted scalar.
- num-bigint 0.4.8 is already in Cargo.lock, so it covers Rule 3's `Decimal` mantissa.

## 4. Surprises
1. The clamp is broader than its own doc says. Values in (i64::MAX, u64::MAX] parse fine through serde_yaml_ng and openapiv3, which
   fall back to AnySchema. Only values beyond u64/i64::MIN hit the i128 error. The clamp also rewrites `type: number` bounds, and it
   misses minified JSON (key not at line start).
2. `type: integer` with any non-i64 bound (2^63, 1e30, 0.1) silently becomes `SchemaKind::Any`, not IntegerType. Mapping code
   that matches on `Type::Integer` will lose the bounds and the integer kind.
3. 2^63 is exactly representable in f64, yet the rendered text `9.223372036854776e18` names a different decimal. The value is lost
   by rendering, not by storage.
4. The "retained source component" is a re-serialization of openapiv3 (`schema.rs:89`), not source text.
