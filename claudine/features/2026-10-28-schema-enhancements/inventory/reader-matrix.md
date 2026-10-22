# Phase 1 Inventory: Reader Fields, Formats, and Matrix Expansion

Snapshot of `e116c6acc`, taken 2026-10-21. Path shorthands:

- `lib` = `darkmatter/lib/src`
- `schemas` = `lib/markdown/schemas`
- `expr` = `lib/markdown/compose/expression`

**Where the behavior claims come from:**

- Code reading.
- Existing tests, cited where they exist.
- Throwaway probes run against the already-built `target/debug/md` (built 2026-10-21) in `/tmp`, using `md schema validate --no-baseline-schema`, `md get`, `md set`, and `md compose`. No files in the repo were changed.

**Cell codes** follow plan.md: E, K, C, V, N/A.

- "**now**" means existing code already decides the cell, so it can be tested today.
- "**blocked: X**" names the plan task that has to land before the cell can be tested.

## (a) YAML SimplifiedSchema: standalone and catalog envelopes

### Loaders

All of these readers parse with `serde_yaml_ng::Value`. None of them reads JSON.

A `.json` schema file always goes to the raw loader (`schemas/resolve.rs:723-725`). A JSON-spelled envelope works only as YAML flow syntax inside a `.yaml` file; `lib/tests/l1/schema_embedded_trigger_bundles.rs:609` covers that case. **Do not add a JSON envelope column.**

| Reader | Entry point | Notes |
|---|---|---|
| Standalone document, in both the pure (`$schema`) and tagged (`kind`/`types`) forms | `parse_standalone_schema_document`, `schemas/simplified/standalone.rs:123` | Calls `source::parse_standalone_schema_payload_with_source`, then `parse_schema_declaration` (`simplified/mod.rs:86`) and `parse_yaml_schema` (`mod.rs:117`). Each named type goes through `parse_property_definition_at_depth` (`mod.rs:314`). Consumed by `resolve.rs:729` (`parse_yaml_referenced_file`), `:970`, and `:2177`. |
| Global variable catalog | `GlobalCatalog::load`, `schemas/catalog.rs:299` | Expects `kind: schema`, `description`, and a `$schema` map of globals. This is not the function catalog. |
| Expression function catalog (current) | `parse_expression_function_catalog`, `expr/catalog/parser.rs:158` | Reads `darkmatter/docs/schemas/expression-functions.yaml`, embedded at `catalog/mod.rs:484-491`. Deserializes a serde `Raw*` model with `deny_unknown_fields`, then validates against its own `$schema`. |
| Replacement function catalog | **none yet** | `darkmatter/schemas/partials/{functions,mutations}.yaml` (drafted under `claudine/docs/schemas/partials/`) needs a strict loader modeled on `GlobalCatalog::load`. Blocked on the Phase 4 Catalog replacement. |

### Load-bearing fields

Fields read today:

- **Tagged envelope:** `kind` (exactly `"schema"`) and `types` (a mapping). No other top-level key is allowed (`standalone.rs:149-163`).
- **Pure envelope:** `$schema` is the sole key. It may be a mapping, a root-union sequence, or a reference scalar.
- **`types.<name>`:** one of
  - a type-expression string,
  - null, meaning the `null` type (`mod.rs:321-324`, documented at `darkmatter/docs/topics/schemas/definition.md:666`),
  - a mapping (an inline object),
  - a sequence (a union).

Fields this feature adds, all owned by Wave 2 Grammar descriptors:

- the top-level `categories:` list, whose entries are `Name [-> description]`;
- function signatures inside `types.<canonical>`, using `parameters`, `returns`, `category`, `fallible`, and repeated `example`;
- tuple slots;
- `|` unions;
- `category(X)` on any type;
- the file form of examples.

The old catalog fields `order`, `literals`, and `nullable` are dropped.

### Current behavior for each cell

Pinned tests already exist: `TAGGED_CELLS` in `lib/tests/l1/schema_yaml_spellings.rs:96-155` (seed files in `lib/tests/fixtures/schema_yaml_spellings/`) and `lib/tests/l1/global_catalog.rs`.

| Shape | `kind` | `types` | named type | `categories` | root `$schema` |
|---|---|---|---|---|---|
| Positive | catalog usable: **now** for the global catalog; **blocked: Catalog replacement** for functions | **now** | **now** for data types; **blocked: Grammar** for function types | **blocked: Grammar** | **now** |
| Absent | the classifier *declines* (`Ok(None)`); the document then falls through to raw JSON Schema and **loads** (probe: `title: 5` passes). The plan wants E, which is reachable only through an explicit catalog loader. **Ruling R5** | E (`SchemaDocument`) **now** | missing canonical E: **blocked: Catalog replacement** (registry parity) | unrestricted: **blocked: Grammar** | declines to raw schema (documented) **now**; E only through an explicit load |
| Null | declines, same as Absent | E **now** | **the `null` type (documented)**, but the plan says E. **Ruling R5** | **blocked: Grammar** | E **now** |
| Wrong type | declines (`kind: 5`, a list) | E **now** (number, list, scalar reference) | number or bool gives E **now** | **blocked: Grammar** | `5` gives E **now**; `[string]` is read as a reference arm, giving E "unresolved" **now** |
| One element wrong | N/A (scalar) | an invalid named type gives E **now** | `[string, 5]` gives E **now** | **blocked: Grammar** | E **now** |
| Every element wrong | N/A (scalar) | E **now** | E **now** | **blocked: Grammar** | E **now** |
| Empty | `kind: ""` declines | `types: {}` is accepted **now**; "catalog incomplete" E is **blocked: Catalog replacement** | `""` gives E **now** | `[]` (a declared list) is **blocked: Grammar**; duplicate or empty names: **Ruling R2** | `{}` is valid **now** |
| Duplicate key | E **now**, but the error is wrong. `standalone.rs:127-130` swallows the YAML error into `Ok(None)`, and the raw re-parse then reports `SchemaError::AmbiguousReferenced` ("ambiguous schema reference") | same | same | same once added | same |
| Trailing content / second document | same: E with the AmbiguousReferenced message | same | same | same | same |

Two more cells:

- **An extra top-level key such as `categories:`** gives E "unsupported keys" **now**. The Grammar task must relax this for `categories:` only (spec.md:405).
- **The old function catalog** already gives E for each of these: a duplicate key or second document (`InvalidYaml`), an unknown field, an empty `functions` list (`EmptyCatalog`), and an empty category or description (`EmptyValue`).

### Smell hits in (a)

| Hit | Verdict |
|---|---|
| `standalone.rs:127-130`: `Err(_) => return Ok(None)`, which behaves like `.ok()` on the parse | **Real defect.** Two consumers then drop the error: `.ok().flatten()` at `dmls/src/diagnostics/named_types.rs:344` and `dmls/src/overlay/named_types.rs:231`, and the same pattern at `resolve.rs:2239`. Through them a malformed `kind: schema` file is silently ignored. The error is also mislabeled. The new catalog loader must not reuse the classifier's `Option`. |
| `catalog.rs:425-438`: `is_global_catalog` and `declares_fm` use `is_ok_and` and `.ok()` | **Real defect, out of scope.** `host_catalogs` silently skips a malformed schema-root catalog. Report it; do not fix it in this feature. |
| `expr/catalog/parser.rs:101,129,130,148`: the `Option` fields `description`, `returns.type`, `returns.array`, and `reason` | **Minor.** These treat null the same as absent. The reader is deleted in Phase 4; do not copy the pattern. |
| `standalone.rs:151`: `filter_map(YamlValue::as_str)` over keys | Benign, because `map.len() != 2` still rejects non-string keys. The categories change edits this exact code: keep it error-complete. |
| `catalog.rs:258-263`: `.ok()…filter_map(as_str)…unwrap_or_default()` | Benign. It is a pre-scan, and `load()` re-validates. |
| `parser.rs`: `#[serde(default)]` at :100, 108, 120-124, 134-138 | Benign. These are the documented absent-means-false or empty fields. |
| `grammar.rs:279`: `lex.parse::<f64>().ok()?`; `grammar.rs:1948` | **Real (numeric).** Numbers in type expressions round silently. This feeds the numeric-ingress spike. Owned by Grammar plus Numeric (see ownership.md section 3). |

## (b) DMLS `.dmls.toml` and editor JSON settings

### Loaders

| Side | Entry point | Parser | Notes |
|---|---|---|---|
| File | `DmlsConfig::load_file`, `dmls/src/config/mod.rs:285` | `toml` 1.0: `toml::from_str::<Table>`, then `try_into::<DmlsConfig>` | `discover` (:323) skips a file it cannot load and records an error diagnostic. |
| Editor | `ConfigState::apply_client_settings` (:516), then `recompute` (:569) | The already-decoded `serde_json::Value` from `DidChangeConfigurationParams` (`router.rs:2581`, `parse_params` :2827) | Deep merge of overlay over file, then `from_value::<DmlsConfig>`. |
| Wire | `lsp-server` 0.8 transport | `serde_json` | Malformed JSON stops the transport and the server exits (`dmls/tests/l1/stdio_subprocess.rs:309-329`). |

**`workspace/configuration` is never requested.** `darkmatter/docs/lsp/configuration.md:119` says so explicitly. The only consumer of `supports_workspace_configuration` (`capabilities.rs:236,310`) is the capability profile. The spec's "workspace/configuration" (spec.md:1002, 1129, criterion 40) therefore refers to the existing `didChangeConfiguration` channel. See **Ruling R4**.

### Fields

- **Existing sections, all with struct-level `#[serde(default)]`:** `workspace`, `wiki`, `schema`, `style`, `shell`, `hints`, `code_actions`, `symbols`, `semantic_tokens`, and `diagnostics`.
- **`formatting`** is read separately through `normalize_formatting` (:647).
- **Added by this feature:** an `expressions` table (`ExpressionsConfig`, `#[serde(default)]`) with `strict: bool`, default `false`.

### Current behavior, as it would apply to `expressions` and `strict`

These cells are inferred from the generic paths every other section goes through today. The same mechanism is pinned by tests for `formatting`, `diagnostics.debounce_ms`, and `semantic_tokens.enable`.

| Shape | TOML `expressions` | TOML `strict` | JSON `expressions` | JSON `strict` | Settings root / `dmls` |
|---|---|---|---|---|---|
| Positive | **blocked: Editor diagnostics** (the field does not exist) | same | same | same | wrapped and bare both work **now** (`config_state_bare_section_settings`, mod.rs tests) |
| Absent | `#[serde(default)]` gives false. **Documented absence: allowed** | same | merge leaves the file value | same | `{}` or `{"dmls": null}` clears the overlay **now** |
| Null | N/A (TOML has no null) | N/A | null replaces the section, deserialization fails, and the file layer applies with an error, matching the plan | `bool` from null fails, giving the file layer with an error | `null` clears the overlay **now** (`null_settings_clear_the_editor_overlay`) |
| Wrong type | `expressions = 5` or `[]`: `try_into` fails, the whole file is rejected, and defaults or the next root apply **now** | `"yes"` gives the same | file layer, error | file layer, error | a non-object keeps the previous *overlay* with an error **now** (`non_object_settings_keep_the_previous_overlay`) |
| One / every element wrong | collection instead of table: E (whole file) | collection instead of bool: E | file layer | file layer | same |
| Empty | `[expressions]` (empty) gives the default false | `strict = ""` gives E (whole file) | `{}` keeps the file fields | `""` gives the file layer | `{}` clears the overlay |
| Duplicate key | `toml` 1.0 rejects it: E **now** (pinned for formatting at `dmls/tests/l1/workspace_formatting_settings.rs:297,364`) | same | **last-wins at the transport, pinned**: `duplicate_json_member_names_are_resolved_before_the_loader` (`workspace_formatting_settings.rs:1001`) | same | same |
| Trailing / invalid | E **now** (`:306,365`) | same | the transport rejects it and the server exits (`stdio_subprocess.rs:309`) | same | same |

Two caveats apply to the whole table:

- **One bad leaf discards everything.** A failing merge discards the *whole* non-formatting overlay (`recompute` :583-596). Tests for "invalid overlay: file layer" should also assert that an unrelated valid overlay key is lost too. That is the current contract. Changing it to per-section isolation would be a new decision.
- **Unknown keys are ignored.** Outside `[formatting]`, an unknown key is silently ignored. This is documented at `configuration.md:73`, and no section uses `deny_unknown_fields`. So `[expressions] strcit = true` is accepted and leaves strict false. See **Ruling R6**.

### Smell hits in (b)

| Hit | Verdict |
|---|---|
| `#[serde(default)]` on `DmlsConfig` and every section (`mod.rs:88-246`) | Benign: absence here is documented to mean the default. `ExpressionsConfig` should follow the same pattern. Null is still rejected because the fields are not `Option`. |
| `Option<PathBuf>` fields `workspace.root`, `wiki.wiki_root`, and `shell.policy_path` | Intentional: null clears them (documented at `configuration.md:178-181`). Do **not** make `strict` an `Option`, or null would become "absent" and contradict the plan's E cell. |
| `normalize_formatting` `unwrap_or_default()` (:658) | Benign: `None` means already rejected and reported. |
| `u16::try_from(columns).ok()` (:783) | Benign: it is range-filtered first. |

## (c) Raw JSON Schema loader and translator

### Loaders

| Format | Entry point | Parser | Duplicate key | Trailing content |
|---|---|---|---|---|
| `.json` | `parse_raw_json_schema`, `schemas/resolve.rs:1150` | `serde_json::from_slice` | **last-wins, silently.** Probe: a duplicated `"n"` (string, then integer) validated as integer. | E, but `map_err(\|_\|)` turns it into `AmbiguousReferenced` and drops the detail |
| YAML raw schema (any non-`.json` file that is not a SimplifiedSchema envelope) | `parse_yaml_referenced_file`, `resolve.rs:729`, which parses at :759 | `serde_yaml_ng` | E (mislabeled as AmbiguousReferenced) | E for multiple documents (mislabeled) |

How the loader is reached and what it builds:

- **Sources:** a frontmatter `$schema` value or a local file reference. `classify_schema_reference` (`reference.rs:64`) rejects `http(s)`. Built-ins are embedded YAML (`embedded.rs:163-183`).
- **Validator:** `jsonschema` 0.55 with `default-features = false` (`lib/Cargo.toml:141`), using `Draft202012` (`validate.rs:504`).
  - Meta-schema validation is on, so a malformed keyword fails when the validator is built (`SchemaError::BuildValidator`).
  - No retriever is configured, so a remote `$ref` gives E at build time (probe).
- **`serde_json` features in this build:** `std`, `float_roundtrip`, and `raw_value`. Neither `preserve_order` nor `arbitrary_precision` is enabled.

Because YAML raw schemas **are** supported, the plan's "repeat for supported YAML raw schema documents" applies: every keyword column needs a YAML-format pass as well as the JSON one.

### Fields

What Darkmatter's own code reads today:

- **For conversion** (`coerce.rs:162-240`): `type` (single string only; arrays are ignored at :191), `anyOf`, `const`, `format`, `items`, `properties`, and `x-darkmatter-*`.
- **For file-mode selection** (`compose/schema_validation.rs:922-975`): `properties`, `allOf`, `anyOf`, and `oneOf`.
- **In `rewrite.rs`** (:263, :271): `type`.

Every other keyword in the plan's tables is read only by `jsonschema`.

What this feature adds (Wave 4 Schema adapter):

- conversion through `$ref`, `$defs`, `definitions`, `$id`, `$anchor`, `oneOf`, `allOf`, `patternProperties`, `additionalProperties`, `prefixItems`, `type` arrays, and `multipleOf`;
- lossless numeric admission for `minimum`, `maximum`, the exclusive bounds, `multipleOf`, `enum`, `const`, and `default` (Wave 2 Numeric foundations);
- the custom formats.

### Cell status

- **now:** all of the following are E at validator build, through the meta-schema:
  - for every keyword: explicit null, a wrong whole type, wrong elements, and trailing content;
  - for every Absent row: the dialect default applies;
  - an unresolved or remote `$ref`;
  - an empty `anyOf`, `oneOf`, `allOf`, or `type` array.

  Validation-only positive controls (`allOf`, `not`, `if`/`then`/`else`, `dependent*`, `propertyNames`, `contains`, `unevaluated*`, `$dynamicRef`, and so on) are testable **now** through `md schema validate`.
- **Conflicts with the plan table**, all from the Draft 2020-12 meta-schema (probed):
  - **`prefixItems: []` is E.** The meta-schema's `schemaArray` sets `minItems: 1` (probe: "[] has less than 1 item"). The plan says "empty list valid". **Ruling R7.**
  - **`enum: []` loads.** The 2020-12 meta-schema has no `minItems` on `enum`, so the schema loads and every instance then fails. The plan says "[] E". **Ruling R7.**
  - **A duplicate key is K (last-wins) for `.json`.** The plan says E. **Ruling R3**, then Schema adapter.
- **blocked: Wave 3 Conversion core and Wave 4 Schema adapter:** the conversion positive controls for `$ref`, `$defs`, `definitions`, `$id`, `$anchor`, `oneOf`, `patternProperties`, `additionalProperties`, `prefixItems`, `type` arrays, `items`, `properties`, and `multipleOf`.
- **blocked: Wave 2 Numeric foundations:** lossless numeric tokens in the bounds and `multipleOf`, and in numeric `enum`, `const`, and `default`.

### Smell hits in (c)

| Hit | Verdict |
|---|---|
| `resolve.rs:759` and `resolve.rs:1152`: `map_err(\|_\|)` maps to `AmbiguousReferenced` | **Real defect, error quality.** The cell's outcome is E, but the error has the wrong class and no detail (line and column, "duplicate key"). Fix it in Schema adapter (resolve.rs should be assigned to that task). |
| `coerce.rs:597`: `serde_json::from_str::<Number>(s).ok()` | **Real defect.** Without `arbitrary_precision`, an integer above u64 silently becomes an `f64`, which contradicts the comment at :592-596. Wave 2 Numeric foundations replaces it. Fix the comment then. |
| `coerce.rs:50` `.ok()` on the arm validator; `coerce.rs:620,632`; `format.rs:313`; `validate.rs:614,1108,1179,1241,1421`; `schema_validation.rs:148,597,745,798,860,929,963` | Benign. Either the schema was already meta-validated, or `None` is the signal and the main build reports the error. |
| `#[serde(default)]`, or `Option` used where absent and null must differ | None in these readers. Raw schemas stay `serde_json::Value`. |

## (d) Frontmatter and numeric value loading

### Loaders

Frontmatter is **YAML only**, between `---` fences:

- `parse_frontmatter`, `lib/markdown/frontmatter.rs:332`
- then `parse_yaml_with_fallbacks` (:584)
- then `parse_yaml_map_rejecting_duplicates` (:644)

The path goes through `serde_yaml_ng::Value`, then `from_value` into `FrontmatterMap`. Neither JSON nor TOML frontmatter exists, so the JSON column of the frontmatter matrix applies only to the JSON *value-loading surfaces* below.

Duplicate keys are rejected recursively, and the expression and shell fallbacks keep that error (:594, :625). Tests: `frontmatter.rs:1173` and `darkmatter/cli/tests/l1/get_set_rm.rs:212`. A second document is E.

**JSON value-loading surfaces:**

| Surface | file:line | Parser | Duplicate key | Trailing content | Changed by this feature? |
|---|---|---|---|---|---|
| `json` content format (`format: json` / `json` type) | `schemas/format.rs:260` | `serde_json::from_str(..).is_ok()` | **accepted** (probe) | E | Yes: Wave 4 custom formats and criterion 25 |
| `yaml` content format | `format.rs:255` | `biscuit_file::Yaml` | E | E for multiple documents | Yes: Wave 4 |
| Raw schema `.json` | `resolve.rs:1150` | `serde_json` | last-wins | E (mislabeled) | Yes: Wave 4 |
| `md set <prop> <value>` | `darkmatter/cli/src/commands/frontmatter.rs:75` | `serde_json`, falling back with `unwrap_or_else` to a string | last-wins | **silently stored as a string** | Only if the numeric seam changes it |
| `md compose --state` / `--set` | `darkmatter/cli/src/commands/compose.rs:260,274` | `Json5` (json_five) | last-wins | E | Only if the numeric seam changes it |
| Compose shorthand `key=value` | `compose.rs:86` | `Json5`, falling back on `Err(_)` to a string | last-wins | becomes a string | same |
| `::file … set=` | `lib/markdown/transclusion/parser.rs:381` | `Json5` | last-wins | E | same |
| `::file … replace=` | `transclusion/parser.rs:401` | `serde_json` | last-wins | E | same |

### Numbers today

These feed the numeric-ingress spike.

- YAML numbers become `serde_json::Number` (i64, u64, or f64) through `from_value`.
- What the probes showed:

  | Input | Result |
  |---|---|
  | `9007199254740993` | kept exactly as data, but `{{ n + 0 }}` renders `…992` |
  | `18446744073709551616` | the **whole frontmatter parse fails** ("integer as u128") |
  | `1e400` | loads as the string `"1e400"` |
  | `1e-400` | silently becomes `0.0` |
  | `.inf` | silently becomes `null` |
  | `02134` | stays a string |

- Expression literals are read by `expr/lexer.rs:890` (`read_number`). It accepts only `digits[.digits]` (no exponent, no leading `.`) and parses straight to `f64`.

### Fields

Today:

- the document property paths declared by the effective schema;
- the conversion paths listed in (c);
- the `x-darkmatter` formats.

This feature adds:

- numeric admission on every number;
- tuple slots, which lower to `prefixItems`;
- conversion through union arms, pattern keys, and `$ref`;
- `numberlike` and `boolish` preservation through containers;
- optional tuple null as E.

### Cell status

| Archetype | now | blocked |
|---|---|---|
| Required `n` / optional `maybe` | Absent, Null, Duplicate (YAML E), Trailing (E), exact integer K | exactness, overflow, and underflow (**Wave 2 Numeric**). The whole-parse failure on u128 and the silent `.inf` → null and `1e-400` → 0 are current defects for the spike. |
| `zip` (numberlike) / `flag` (boolish) | preservation K and presence | containers (**Wave 3/4**) |
| `values` (typed list) | `[]` K; scalar E | numeric text elements C at an index (**Wave 3/4**) |
| `meta` (typed object) | `{}` K; presence | nested C (**Wave 3/4**) |
| `config` (content target) | a native mapping serializes to JSON text **now**; malformed text E | duplicates inside textual JSON (**Ruling R3**) |
| Tuple slots, required variants, pattern-key values, whole-value unions | — | **Grammar (W2), then Schema adapter (W4)** |
| Every JSON-surface duplicate cell | — | **Ruling R3** |

### Smell hits in (d)

| Hit | Verdict |
|---|---|
| `darkmatter/cli/src/commands/frontmatter.rs:75` falls back with `unwrap_or_else` to a string | Behavior is documented as "JSON or literal", but **malformed JSON with trailing content becomes a string with no warning**. Not changed by this feature unless the numeric seam is routed through it. Report it. |
| `compose.rs:86`: `Err(_)` gives a string | Same class. Documented shorthand. |
| `lib/markdown/render/metadata_codec.rs:33` `.ok()`, cache manifests, mermaid theme | Out of scope (not schema or frontmatter). |
| `frontmatter.rs:569` | Benign. |

## Matrix fixtures: sources from real producers

| Need | Candidate | Provenance |
|---|---|---|
| Shipped SimplifiedSchema catalog | `darkmatter/schemas/darkmatter.yaml`, `darkmatter/schemas/partials/*.yaml` | shipped artifacts, loaded byte-for-byte in `lib/tests/l1/global_catalog.rs` |
| Current function catalog | `darkmatter/docs/schemas/expression-functions.yaml` | shipped and embedded |
| Draft replacement catalog | `claudine/docs/schemas/partials/{functions,mutations}.yaml` | hand-authored draft. It does **not** load today: `lib/tests/l1/shipped_schema_corpus.rs:215-223` marks it Draft because `parameters(...)` is an unknown constraint |
| Serializer output | `lib/tests/fixtures/schema_yaml_spellings/*.serialized.yaml` | `serde_yaml_ng` serializer, checked by `serializer_captures_are_serde_yaml_ng_output` |
| `md schema` output | `md schema detect <md…> [--format yaml\|json]` on `lib/tests/fixtures/detect/*/inputs` | real producer. Existing tests only check substrings (`darkmatter/cli/tests/l1/schema_detect.rs`). Library snapshots: `lib/tests/l1/snapshots/l1__schemas_convert_snapshots__*.snap`. `--format json` is the **only real producer of a raw JSON Schema** found |
| "Current `md frontmatter` output" | **no `md frontmatter` subcommand exists.** Use `md get <file> [props] --yaml` (or `--json5`/`--toml`/`--raw`/`--compact`) and `md set` | real producer. Tests: `darkmatter/cli/tests/l1/get_set_rm.rs` |
| DMLS real client exchange | `dmls/tests/fixtures/editor_neovim/{init.lua:58,probe.lua:164}` (Neovim sends `didChangeConfiguration` with `settings = {}`, L2 only); wire-level L1 in `dmls/tests/l1/stdio_subprocess.rs:309`; `lsp_session.rs:109,1092,3705` | Neovim is the real client, but its payload is empty, so a strict-setting payload has to be authored in the same wire shape |
| DMLS control fixtures | `dmls/tests/fixtures/formatting/{dmls.toml,settings.json}` | hand-authored controls. The precedent harness is `dmls/tests/l1/workspace_formatting_settings.rs`, a complete TOML and JSON one-edit matrix to extend for `expressions` |
| Documented `.dmls.toml` settings | `darkmatter/docs/lsp/configuration.md` (code blocks at :24, :157, :193) | documentation |
| Raw JSON Schema fixtures | `lib/tests/fixtures/validate/json_schema_ref/schema.json`, `dmls/tests/fixtures/suggest_constraint/raw-schema.json` | **hand-written, no producer provenance.** Capture one from `md schema detect --format json` for provenance, and keep the hand-written ones for keyword coverage (`$ref`/`$defs`/`oneOf`), which `detect` does not emit |
| Existing schema robustness harness | `darkmatter/cli/tests/l1/schema_robustness_matrix.rs` (controls: `docs/schemas/env.yaml`, `tests/fixtures/schema-triggers/schemas/sequence.yaml`), `cli/tests/fixtures/schema_validate_baseline/*` | precedent to reuse |
