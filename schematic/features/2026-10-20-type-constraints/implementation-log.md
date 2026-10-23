---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/spec.md"
plan: "schematic/features/2026-10-20-type-constraints/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - schematic/features/2026-10-20-type-constraints/spikes/s1-lossless-numbers.md
    - schematic/features/2026-10-20-type-constraints/spikes/s2-schemars-output.md
    - schematic/features/2026-10-20-type-constraints/spikes/s3-body-encode-reuse.md
skills_files_updated_during_phase_1: []
packages:
    - schematic-runtime
    - schematic-gen
    - schematic-schema
    - schematic-define
    - schematic-definitions
human_review: false
message_to_agent: |-
  Phase 3 built schematic_define::constraints; read "## Phase 3" in this log.
  Facts Phase 4 relies on:
  - ParamDef.constraints, FormField.constraints, and Endpoint.request_constraints
    are Option<Box<Constraints>> (boxed: inline overflowed the debug stack of the
    imported OpenAI builders). Builders take a plain Constraints.
  - Serialized keys are snake_case (min_length, allowed_values, constant,
    pattern_unenforced, ...); numeric limits are JSON strings of decimal text.
    Export must map them to OpenAPI keywords, without narrowing.
  - An imported pattern outside the enforced subset must be stored with
    Constraints::unenforced_pattern (pattern_unenforced: true); .pattern() on one
    is a definition error. admit_pattern(source) classifies, and
    PatternAdmission::Enforced { translated } is what Phase 5 compiles.
  - gen/src/definition_gen.rs param_def_tokens emits `constraints: None`; Phase 4
    must emit the real rules (as a builder call or Some(Box::new(..))). The
    committed definitions/src/openai/mod.rs was patched to that form; the
    import_overlay test pins two older re-import differences (info.version, the
    realtime session part), so after `just import-openai` keep those pins in
    mind rather than assuming every diff is yours.
  - FieldDef has no constraint field yet; Wave 10 owns that shape.
  - pagination::adopted_page_size(api, ep) gives the adopted page-size param's
    effective bounds for export; constraints::unenforceable_rules(api, body) is
    for Phase 5.
source_files_during_phase_2:
    - Cargo.lock
    - schematic/gen/src/lib.rs
    - schematic/runtime/Cargo.toml
    - schematic/runtime/src/lib.rs
    - schematic/runtime/src/validate/mod.rs
    - schematic/runtime/src/validate/decimal.rs
    - schematic/runtime/src/validate/eval.rs
    - schematic/runtime/src/validate/pattern.rs
    - schematic/runtime/src/validate/plan.rs
    - schematic/runtime/src/validate/violation.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/validate_decimal.rs
    - schematic/runtime/tests/l1/validate_eval.rs
    - schematic/runtime/tests/l1/validate_report.rs
    - schematic/runtime/tests/fixtures/compile_fail/path_segment_holds_only_static_names.rs
    - schematic/runtime/tests/fixtures/compile_fail/path_segment_holds_only_static_names.stderr
    - schematic/runtime/tests/fixtures/compile_fail/violation_holds_no_submitted_value.rs
    - schematic/runtime/tests/fixtures/compile_fail/violation_holds_no_submitted_value.stderr
    - schematic/schema/Cargo.lock
    - schematic/schema/src/lib.rs
docs_updated_during_phase_2:
    - docs/dependencies.md
    - schematic/docs/dependencies.md
    - schematic/docs/runtime/README.md
    - schematic/runtime/README.md
docs_created_during_phase_2:
    - schematic/docs/runtime/validation.md
skills_files_updated_during_phase_2:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
    - .claude/skills/schematic-define/schematic/responses.md
source_files_during_phase_3:
    - Cargo.lock
    - schematic/schema/Cargo.lock
    - schematic/define/Cargo.toml
    - schematic/define/src/builders.rs
    - schematic/define/src/bundle.rs
    - schematic/define/src/constraints/check.rs
    - schematic/define/src/constraints/mod.rs
    - schematic/define/src/constraints/number.rs
    - schematic/define/src/constraints/pattern.rs
    - schematic/define/src/constraints/registered.rs
    - schematic/define/src/lib.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/openapi/export/parameters.rs
    - schematic/define/src/openapi/import/mappings/mod.rs
    - schematic/define/src/openapi/import/mappings/responses.rs
    - schematic/define/src/pagination.rs
    - schematic/define/src/pagination/lower.rs
    - schematic/define/src/pagination/validate.rs
    - schematic/define/src/params/mod.rs
    - schematic/define/src/params/param_def.rs
    - schematic/define/src/registration.rs
    - schematic/define/src/request.rs
    - schematic/define/src/types.rs
    - schematic/define/tests/builders.rs
    - schematic/define/tests/definition_serde.rs
    - schematic/define/tests/endpoint_literal_surface.rs
    - schematic/define/tests/pagination_constraints.rs
    - schematic/define/tests/registered_model_rules.rs
    - schematic/define/tests/value_constraints.rs
    - schematic/definitions/src/openai/mod.rs
    - schematic/gen/src/codegen/api_struct/tests.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/pagination/tests.rs
    - schematic/gen/src/codegen/request_structs/tests.rs
    - schematic/gen/src/definition_gen.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/tests/l1/endpoint_auth_output.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/gen/tests/l1/postman_runnable.rs
    - schematic/gen/tests/l1/query_params_codegen.rs
    - schematic/gen/tests/l1/typed_path_output.rs
docs_updated_during_phase_3:
    - docs/dependencies.md
    - schematic/docs/dependencies.md
    - schematic/docs/definition-builders.md
    - schematic/docs/generation/validation.md
    - schematic/docs/io/definition-bundles.md
    - schematic/docs/pagination.md
    - schematic/docs/typed-registration.md
    - schematic/docs/websocket-definitions.md
docs_created_during_phase_3:
    - schematic/docs/value-constraints.md
skills_files_updated_during_phase_3:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/define.md
    - .claude/skills/schematic-define/schematic/pagination.md
---

# Implementation Log for 2026-10-20-type-constraints (8 phases)

## Phase 1

Phase 1 is research only: rulings, spikes, baselines, and the touchpoint
inventory. No package source changed. The spikes' full evidence lives in
`spikes/` beside this log (`s1-lossless-numbers.md`, `s2-schemars-output.md`,
`s3-body-encode-reuse.md`), because Phase 4 and Phase 5 need it and the scratch
crates under `/tmp` do not survive.

### Prerequisites and queue position

- `2026-10-14-schematic-definition-api` and `2026-10-12-schematic-endpoint-methods`
  have landed. The four later Schematic features (vocabulary, response-model-graphql,
  websocket, fidelity-polish) have also landed, so this plan runs last, as ruled.
- The working tree has uncommitted fidelity-polish follow-up edits in 12 files
  (`define/src/components.rs`, `define/src/graphql.rs`, `define/src/models/type_ref.rs`,
  `define/src/openapi/import/{builder.rs,mappings/schema.rs}`,
  `define/tests/openapi_multipart_import.rs`, `docs/io/{export-postman,import-openapi}.md`,
  `gen/src/export/postman/{graphql,request}.rs`, `gen/tests/l1/postman_runnable.rs`,
  and the fidelity-polish spec), plus untracked fidelity-polish logs and reviews.
  None is part of this feature, and Phase 1 did not touch them. The baselines below
  include them. Later phases editing those files must merge with these edits, not
  revert them.
- Typed registration and the pager contract both exist, so there is no
  evidence-based blocker.

### Contract check (spec "current contracts" against the code as it is now)

| Contract | State | Location / note |
| --- | --- | --- |
| `FieldDef` | OK | `define/src/models/structs.rs:27`; `deny_unknown_fields`, `serde_presence::non_null`; no constraints. Rustdoc and `gen/src/model_gen.rs` tests build it as a struct literal, so a new field breaks those literals. |
| `ParamDef` | OK | `define/src/params/param_def.rs:26`; builders at `:152-167`; no min/max; `QueryParamType::Enum` is the only value-domain rule. Typed path params are `ParamDef`s (`ParamDef::path`, `EndpointBuilder::path_param`). |
| `SchemaComponent` | OK | `define/src/components.rs:16`, `schema: serde_json::Value`. `define/src/schema.rs` `Schema` is WebSocket-only, as the spec says. |
| `EndpointSpec::into_parts` / `Call::build` | OK | `runtime/src/endpoint.rs:70`; `runtime/src/call.rs:134` → `client.check_arguments` (`runtime/src/client.rs:859`); `InvalidArgument` at `runtime/src/error.rs:110`. |
| Pager page-size check | OK | `runtime/src/pager/engine.rs:395-421` (`plan_managed`). It runs after `effective_size()`, and only when no followed link carries the size or the caller adjusted it. The problem text is "the page size is above the declared maximum, {max}". It surfaces through `pager/request.rs:171` as `InvalidArgument`. Retry resizing clears "the page size " problems (`engine.rs:247`). Define side: `pagination::PageSize` (`define/src/pagination/strategy.rs:158`), validated at `pagination/validate.rs:480-488`; GitHub `.max(100)` at `definitions/src/github/mod.rs:71`. |
| Importer clamp | OK | `define/src/openapi/import/normalize.rs:50` `clamp_numeric_bounds` (public, re-exported at `openapi/import.rs:61`, with a doctest), called at `import/builder.rs:297`. |
| Importer retention | OK, but weaker than the spec implies | `mappings/schema.rs:82-99` `source_component` re-serializes the *parsed* `openapiv3` schema, so the "retained source" is already clamped and widened to f64. |
| Exporter | OK, partly constraint-aware already | `define/src/openapi/export/conversion.rs`: copies the keyword list (`:29-34`), lowers `const` to `enum`, lowers a numeric exclusive bound to bound + `true` (`:151-160`, an error if the same-side inclusive bound is also present), and keeps constraints on nullable. It then deserializes into `openapiv3::Schema` (`:70`), a lossy step (S1). |

Discrepancies (adaptations, not blockers):

1. **The GitHub regression anchor is not `.page_size(500)`.** Nothing calls that; it
   appears only in a doc comment (`runtime/src/pager/session.rs:98`). The real anchor is
   `schematic-schema` `pagination::github_page_size_outside_the_authored_bounds_is_refused_before_sending`
   (`schema/tests/pagination.rs:174`, sizes 0 and 101, zero requests). Runtime
   equivalents: `runtime/tests/l1/pager_rules.rs:321`
   `a_size_outside_the_declared_bounds_is_refused_naming_the_bound_with_zero_requests`
   and `runtime/tests/l1/pager_collect.rs:319`. Later phases keep these as the anchors.
2. **`ABI` is 7, not 4.** Rule 7 therefore raises it to **8**, in both
   `runtime/src/lib.rs:127` `ABI` and `gen/src/lib.rs:98` `RUNTIME_ABI`. The unit test
   `the_generator_emits_the_abi_of_the_workspace_runtime` cross-checks the two.
3. **WebSocket handshake parameters reuse `EndpointParams`/`ParamDef`**
   (`define/src/websocket/mod.rs:201-205`), so a constraint authored on `ParamDef` can
   reach a WebSocket definition, which the spec puts out of scope. Adopted default:
   a constraint on a WebSocket handshake parameter is a located definition error.
   Source: Non-goals ("WebSocket message validation"), plus "Generation must fail
   for a supported request rule it cannot enforce" (Definition errors versus
   unsupported rules). Rejecting is consistent with Rule 8 and easy to relax later.
4. **The definition bundle is JSON-only** (`define/src/bundle.rs`, `DefinitionBundle::from_json`).
   The plan's "JSON/YAML/TOML" serialized-definition loader row therefore reduces to one
   JSON loader. Its YAML-specific cells (`k:` null, second document) do not apply; the
   JSON cells still apply: duplicate key, trailing content, and an old snapshot without
   the metadata. Couplings Phase 3 must handle:
   (a) new members omitted at their default must be listed in `OMITTED_DEFAULTS`
   (`bundle.rs:335`); (b) `reject_nulls` (`bundle.rs:307`) rejects an explicit null
   outside `/api/schemas/N/schema`, so an `enum` or `const` holding JSON `null`
   in parameter or form metadata needs an exemption like `is_schema_payload`
   (`:301`), because null there is data (plan Input Robustness exception).
   Matrix tests live in `define/tests/definition_bundles.rs` (`bundle_json_matrix`).
5. **The importer drops parameter constraints** (`openapi/import/mappings/parameters.rs:225`
   `map_schema_to_param_type` keeps only type and enum), and **the exporter emits none**
   (`openapi/export/parameters.rs:75` `param_schema`). Both are Phase 4 work.
6. **Postman examples are already constraint-aware** (`gen/src/export/examples.rs`,
   `ExampleSource`: const/enum, min/maxItems, numeric bounds including exclusive and
   `multipleOf`, string length; each value checked with `jsonschema`). Postman parameter
   values are empty variables, and `x-schematic` `ParamDetail` carries no constraints.
   Constraints travel through standard schema keywords, so there is no second channel
   to keep consistent. Phase 4 and Phase 7 must check that newly exported constraints
   (for example patterns) still yield examples that `jsonschema` accepts.
7. **The generation guard harness uses a real local wiremock server, not a mock
   transport** (`gen/tests/l1/fresh_output_compile.rs` + `gen/tests/common/fresh_output.rs`).
   Adopted default: "zero transport sends" is asserted as zero requests received
   by that server, matching existing tests (source: spec Validation and delivery,
   "Reuse the existing generation guard").

### Touchpoint inventory (located, not edited)

- **`--definitions-out` emitter:** flag at `gen/src/main.rs:139`, handled in
  `gen/src/import_pipeline.rs:160-176`. Models come from `gen/src/model_gen.rs`:
  `generate_models_with` (`:50`), `json_schema_derive` (`:137`), `generate_field`
  (`:180`), which emits only `serde(rename)` and `skip_serializing_if`.
  Definitions come from `gen/src/definition_gen.rs`: `generate_definition_module`
  (`:41`), `endpoint_tokens` (`:211`), `form_field_tokens` (`:405`),
  `param_def_tokens` (`:435`), `query_param_type_tokens` (`:464`). Nothing in
  `gen/src` writes `#[schemars(...)]` yet. Recipe: `just import-openai`.
- **Builders:**
  - `ParamDef` (`params/param_def.rs:152-167`).
  - `EndpointParams::with_query_param(s)` (`params/mod.rs:120,153`).
  - `EndpointBuilder` (`define/src/builders.rs`): `header_param` 241, `cookie` 242,
    `params` 243, `query` 244, `path_param` 248, `request_json::<T>` 226,
    `request_form` 233, `request_urlencoded` 234, `request_text*` 235-236,
    `request_binary*` 237-238, `pagination` 239.
  - `FormField` (`define/src/request.rs`): `text` 143, `file` 163, `files` 205,
    `files_with_constraints` 237, `json` 262, `optional` 281.
- **ABI assertions:**
  - generated `lib.rs` const assert (`gen/src/output/assemble/lib_rs.rs:169-182`);
  - partial-generation check (`gen/src/output/inventory.rs` `ABI_MARKER` 138,
    check around 450-471);
  - fresh-crate test `slow_a_runtime_of_another_abi_fails_the_build_with_a_readable_message`
    (`gen/tests/l1/fresh_output_compile.rs:2902`).
- **Recipes** (`schematic/justfile`):
  - `test` (71): define (all features), gen, definitions, oauth, and runtime, plus
    `test-fresh-output` and `test-compile-fail`.
  - `test-schema` (98): a separate workspace, `schema/Cargo.toml`. Its `*args`
    do not survive quoting, so `just test-schema -E 'test(..)'` fails with a shell
    syntax error. Run filtered tests with
    `cargo nextest run --manifest-path schema/Cargo.toml -E ...`.
  - `doctest` (146) and `doctest-schema` (157); `lint` (196); `lint-schema` (106);
    `check-drift` (183); `migration-check` (192).
  - `generate` (236), `generate-one` (247), `generate-openapi` (256),
    `generate-postman` (265), `import-openai` (220).
- **Generation guard:** add a fixture API in `fresh_output_compile.rs`. Touch
  `fixture_apis()` (465), `FIXTURE_TYPES` (122), `FIXTURE_COVERAGE` (47),
  `REQUEST_SIDE_TEST` (875) and `REQUEST_SIDE_TESTS` (2189), and `FIXTURE_ENV`
  (2180) if the fixture needs environment variables. The main test is
  `slow_fixture_apis_compile_and_send_offline`.
- **Artifacts regeneration will touch:**
  - `schema/src/**` (15 REST clients; `lib.rs` carries the ABI);
  - `openapi/*.json` (15) and `postman/*` (15 pairs);
  - imported definitions `definitions/src/openai/{mod,types}.rs`;
  - fixtures `define/tests/fixtures/bundles/fixture.json`,
    `gen/tests/fixtures/definition-baseline/*.json` (historical snapshots, which must
    read the new metadata as absent), and `gen/tests/fixtures/{postman,wire_baseline}`.
  - `asyncapi/*.json` (5) should not change.
- **Dependent packages (test scope):**
  - `schematic-schema` (separate workspace), which brings in `sniff`,
    `biscuit-speaks`, `homelab`, `eversolo-integration`, `arcam-amp-integration`,
    `unfolded-integration-helper`, and `sony-receiver-integration`;
  - `schematic-definitions`, used by `model-citizen`.
  - Nothing outside `schematic/` depends directly on define, gen, or runtime.

### Spike S1: lossless number path (`spikes/s1-lossless-numbers.md`)

| Stage | `9223372036854775808` | `1e30` | `0.1` |
| --- | --- | --- | --- |
| `serde_json::Value` (JSON) | exact (u64) | f64 → `1e+30` | f64, prints `0.1` |
| `serde_yaml_ng` → `Value` | exact (u64) | f64 | f64 |
| `clamp_numeric_bounds` (key at start of a line) | **lost**: rewritten to `9223372036854775807`, even under `type: number` | untouched | untouched |
| `clamp_numeric_bounds` (minified JSON) | untouched (key must start a line) | untouched | untouched |
| `openapiv3` parse, `type: integer` | **falls back to `SchemaKind::Any`** with an f64; the integer type is lost too | `Any`, f64 | `Any`, f64 |
| `openapiv3` parse, `type: number` | `NumberType` f64 | f64 | f64 |
| `openapiv3` render (JSON/YAML) | **lost**: `9.223372036854776e+18` | `1e+30` | `0.1` |

Further findings:

- More than about 17 significant digits collapse at every serde stage
  (`0.10000000000000000001` → `0.1`), and `100000000000000000000` → `1e+20`.
- A value beyond the u64 range in YAML is a parse error (`invalid type: integer … as u128`).
- `1e30` and `0.1` survive only because the shortest-form printer recovers the text.

**Decision: strategy (b), carry the exact decimal text out-of-band** (the plan's
default). `arbitrary_precision` would not help, because `openapiv3` itself
narrows to i64/f64 and `serde_yaml_ng` yields only u64/i64/f64/i128; no workspace
manifest enables it either.

- **Capture:** in `parse_source` (`import/builder.rs:236`), from the raw content,
  *before* `normalize`. Build a table from JSON pointer to decimal text for each
  numeric constraint keyword.
  - JSON: `serde_json::value::RawValue` (the `raw_value` feature is already unified
    on in the workspace; `schematic-define` must declare it explicitly).
  - YAML: needs a parser that exposes scalar source text. `serde_yaml_ng` does not.
    Candidates: `yaml-rust` 0.4.5 is already in `Cargo.lock` (via syntect) but is
    unmaintained (RUSTSEC-2024-0320), so the adopted default is a maintained
    source-preserving parser chosen in Phase 4 Wave 9. Prefer `yaml-rust2` or
    `saphyr`, and log the choice there (source: Rule 3, "final dependency choice
    … logged").
- **Export:** patch the rendered schema from the sidecar after `openapiv3`
  (`conversion.rs:70` / `options.rs:83-90`) and never trust `openapiv3` numerics:
  for JSON, swap exact numbers back in as `RawValue`; for YAML, patch the scalar text.
- **Phase 4 also has to handle:** an integer schema with any non-i64 bound becomes
  `SchemaKind::Any`, so the mapper must not lose integer-ness for those;
  `clamp_numeric_bounds` (public, with a doctest) is retired for constraint
  keywords; and schemars-emitted `u64::MAX` exports as `1.8446744073709552e+19` today.

### Spike S2: Schemars output (`spikes/s2-schemars-output.md`, with fixture source)

- **Settings:**
  - `registration.rs:231` uses `SchemaSettings::openapi3()` /
    `into_root_schema_for`. Transforms reach only the root, so a stored component's
    **root is OpenAPI 3.0 shaped while nested definitions stay draft 2020-12 shaped**.
    Both dialects appear in one component.
  - `TypeRef::Optional` is `anyOf:[inner, {"type":"null"}]` plus the openapi3 transforms
    (`models/type_ref.rs:206`).
- **Expressibility:**

  | Keyword | Native attribute | Note |
  | --- | --- | --- |
  | `minLength`/`maxLength`, `minItems`/`maxItems` | `length(min, max)` | |
  | `pattern` | `regex(pattern = ..)` or `pattern(..)` | |
  | `minimum`/`maximum` | `range(min, max)` | |
  | `format` | `email`, `url` (→ `uri`), `ip` | other formats need `extend("format" = ..)` |
  | item-level rules | `inner(length / range / regex / pattern)` | |
  | `exclusiveMinimum`/`exclusiveMaximum`, `multipleOf`, `enum`, `const` | none | **`extend` only** |
  | `uniqueItems` | none | implicit for sets; otherwise `extend` |
  | map-value rules | none | `inner` on `HashMap` is silently dropped; only `extend("additionalProperties" = {..})` |

  `#[validate(..)]` and `#[garde(..)]` are also read.
- **Silent drops:**
  - A native rule on a type it does not match vanishes with no diagnostic: `length`
    on `Option<Struct>` or a `$ref`, `range` on `String`, `inner` on a map.
    **The `--definitions-out` emitter must therefore emit `extend` (or explicit
    schema metadata) for any rule the native attribute cannot attach.** A
    round-trip test must assert that the rebuilt schema still has the rule.
  - `extend` is never type-checked, and it overrides native rules.
- **Large numbers through `extend`:**
  - an unsuffixed literal is `i32`, so the emitter must suffix it (`…u64`, `…i64`);
  - a value beyond `u64`/`i64` panics at schema generation (`u128`);
  - a float literal loses precision.
  - So exact bounds beyond the i64/u64 range, and decimals that f64 cannot
    represent exactly, need **explicit schema metadata, not attributes**. This is
    the spec's "explicit schema metadata where attributes cannot express them" route.
- **Nullable-shape catalogue** (the full table is in the spike file):
  - Inline `Option<String|int|Vec>` puts its constraints next to `type`, as
    `nullable: true` at the root or `type: [T, "null"]` in nested definitions.
  - `Option<$ref>` is `anyOf:[{$ref}, {"enum":[null],"nullable":true}]` at the root
    and `anyOf:[{$ref}, {"type":"null"}]` nested.
  - An untagged enum is a non-nullable `anyOf` of 2 or more non-null members.
  - **Traversal rule:** an `anyOf` is an Option only when it has exactly two members
    and one of them admits only null; reuse `admits_only_null` (`conversion.rs:312`).
    Never treat a bare `anyOf`/`oneOf` as nullable.
- **Implicit primitive bounds** are not authored constraints: `u8` gives `0..255`,
  `u32`/`u64` give `minimum: 0`, `NonZeroU32` gives `minimum: 1`, and `char` gives
  length 1. Enforcing them is redundant but harmless; generated rustdoc and
  diagnostics should not present them as authored rules (Phase 5 note).
- `#[serde(rename)]` keys a property by its wire name, and constraints follow it.
  `#[serde(flatten)]` hoists the flattened properties, with their constraints, into
  the parent. A recursive root is `"$ref": "#"`.
- Correction to the S2 report: its claim that the exporter copies `exclusiveMinimum`
  unchanged is wrong. `conversion.rs:151-159` already lowers it (as S1 found). The
  correction is recorded in the spike file.

### Spike S3: body encode reuse (`spikes/s3-body-encode-reuse.md`)

- `RequestBody::Json` already carries a serialized `String`, and the runtime sends it
  unchanged (`runtime/src/encode.rs:23-26`). Generated `into_parts` emits
  `serde_json::to_string(&self.body)` (`gen/src/codegen/request_structs/into_parts.rs:47-53`).
- **Design for Phase 5 (no runtime encoder change):**
  1. For a constrained JSON body, generated `into_parts` does
     `let v = serde_json::to_value(&self.body)`; a failure keeps the existing
     "cannot be serialized as JSON" `InvalidArgument`.
  2. It evaluates the plan: parameters first, then `v`. Violations become one
     `InvalidArgument` before `RequestParts` exists.
  3. It sends `RequestBody::Json(serde_json::to_string(&v)?)`.

  An unconstrained body keeps today's tokens, so its bytes are identical. Every
  send path goes through `into_parts`: `.build()` (`client.rs:859-872`); await,
  `.full()` and `send_raw` (`client.rs:948` → `prepare` `:960` → `:1077-1080`, before
  credentials or I/O); `client.send` (it builds a `Call`); and pager requests. That
  gives one insertion point and zero sends. The runtime additions are just the
  `validate` module plus `impl From<Violations> for InvalidArgument`.
- **Byte differences for constrained bodies only:**
  - Object keys are sorted. `preserve_order` is off in normal builds; it reaches the
    workspace only through a tree-sitter *build* dependency. That is fragile, so the
    evaluator must order keys itself and never rely on `Map` iteration (this matches
    the spec's "deterministic object-key ordering").
  - `f32` values widen when they pass through `Value` (`0.1f32` → `0.10000000149011612`),
    which would also misjudge `multipleOf`. Committed generated clients use no `f32`
    (0 hits in `schema/src`; the model generator maps `number` to `f64`). Adopted
    default: accept it and document it for user-owned registered bodies (source:
    Rule 1, "the representation is the one sent"). It is on the review agenda.
  - `i128`/`u128` beyond 64 bits fail `to_value` and become the existing serialize error.
  - Overlapping flatten keys collapse to the last one.
- **Gap:** a multipart JSON part that fails to serialize is dropped silently
  (`gen/src/codegen/request_structs/form.rs:174-177`), and `into_form_parts` is
  infallible (`form.rs:66`). Adopted default for Phase 5: a *constrained* JSON
  form field is serialized and checked in `into_parts` (fallible), so it never
  takes the silent-drop path (source: spec Bodies and forms, "Form values are
  checked before URL encoding or multipart assembly"). Fixing the silent drop for
  unconstrained parts is new scope and is left undone; it is on the review agenda.
- **Phase 5 also has to handle:** `validate` must become a reserved setter name on
  generated *request* structs, so that a parameter named `validate` is renamed the way
  `build`/`full` are on calls (`gen/src/codegen/endpoint_methods/names.rs:15`
  `SHARED_CALL_METHODS`).

### Baselines

- `just test` (from `schematic/`, with the uncommitted fidelity-polish edits
  present) passed:
  - main run: 2993 run, 2993 passed, 22 skipped;
  - `test-fresh-output`: 16 passed;
  - `test-compile-fail`: 1 passed.
  - Six were marked SLOW: `import_normalization shipped_openai_query_schema_gate_and_reimports`
    (7.2 s), `openapi_extension_matrix extension_matrix_yaml` (10.6 s), and the four
    runtime `builder::` TLS-handshake tests (9–11 s). They ran while four spike agents
    were loading the host; they are pre-existing and not related to this feature.
- `just lint` passed (exit 0).
- **Pager regression anchor** confirmed today:
  `cargo nextest run --manifest-path schema/Cargo.toml -E 'test(github_page_size_outside_the_authored_bounds_is_refused_before_sending)'`
  → 1 passed.
- **Constraint keywords in committed artifacts** (counted with a JSON walk; the
  plan's 4 vs today's 7 for ABI shows why the figures are derived, not hard-coded):
  - exported `openapi/*.json` (15 files): 2124 total. `format` 1125, `enum` 745,
    `minimum` 243, `maximum` 11, and **no** `minLength`, `maxLength`, `pattern`,
    `minItems`, `maxItems`, `multipleOf`, `exclusive*`, `uniqueItems`, or `const`.
    Today's handwritten definitions carry no Schemars rule attributes (0 `length(`,
    `range(`, `regex(`), so most of the 243 `minimum` are implicit unsigned bounds.
  - import sources:
    - `specs/openai/openapi.yaml`: `minLength` 64, `maxLength` 117, `pattern` 8,
      `format` 261, `minimum` 143, `maximum` 94, `exclusiveMinimum` 7, `minItems` 46,
      `maxItems` 51, `enum` 1480, `const` 2.
    - `specs/unfolded-circle/UCR-core-openapi.yaml`: `minLength` 42, `maxLength` 76,
      `pattern` 9, `format` 102, `minimum` 67, `maximum` 37, `multipleOf` 1,
      `minItems` 1, `maxItems` 2, `enum` 75.
  - So the regeneration diff (Phase 7) will mostly land in `openapi/openai.json`,
    `openapi/unfolded_circle_core_rest.json`, their Postman exports, and
    `definitions/src/openai/types.rs`.

### Frozen decisions (rule table, after the spikes)

- Rule 1 (decided): it stands. Reuse is done entirely in generated `into_parts` (S3);
  there is no runtime encoder change.
- Rule 3 (decided): **adopted default: `num-bigint` 0.4.x for the runtime `Decimal`
  mantissa (source: Rule 3, "reuse a big-integer crate already in `Cargo.lock`")**.
  `num-bigint` 0.4.8 is already locked. The exponent bound is ±10 000. The
  integrator adds the stanza in Phase 2 Wave 4. `schematic-define` needs
  `serde_json`'s `raw_value` feature, and a source-preserving YAML parser chosen in
  Phase 4 (see S1).
- Rule 7 (decided): ABI 7 → **8**; `RUNTIME_ABI` follows.
- Rule 9 (defaulted): **adopted default: the public names as listed in Rule 9
  (source: Rule 9)**. None collides today (no existing `Constraints`, `Violation`,
  `Keyword`, `Decimal`, or `Pattern` in define, runtime, or gen). Added:
  `validate` joins the generated request-struct reserved setter names. Large or
  decimal numeric bounds travel as decimal text in the new constraint metadata, so
  the builders' `&str` route is the lossless one, as Rule 9 already says.
- Rule 10 (defaulted): **adopted default: the pattern admission subset as listed in
  Rule 10 (source: Rule 10)**. Unchanged by the spikes.
- Rule 11 (defaulted): **adopted default: the `Violation { path, keyword, reason }` /
  `Violations` shape (source: Rule 11)**. Unchanged by the spikes.
- Rule 13 (defaulted): **adopted default: no performance spike (source: Rule 13)**.
- Adopted defaults from this phase's findings: WebSocket handshake parameter
  constraints are a definition error (contract discrepancy 3); zero sends are asserted
  through wiremock's received requests (discrepancy 7); `f32` widening is accepted and
  documented (S3); constrained multipart JSON fields are serialized fallibly in
  `into_parts` (S3); the YAML parser choice moves to Phase 4 with a preference for a
  maintained crate (S1).
- No rule is unresolved, and nothing is blocking.

### Review agenda (for the author, after the run)

1. Bodies validated through `Value` sort object keys and widen `f32` (S3). Accepted under
   Rule 1; confirm the documentation is enough.
2. A multipart JSON part that fails to serialize is dropped silently in today's
   generated code (`form.rs:174-177`). This is pre-existing behavior outside this
   feature's scope, so it is left alone except for constrained fields.
3. A YAML source-text parser is a new direct dependency of `schematic-define`
   (Phase 4). `yaml-rust`, which is already locked, is unmaintained.
4. `just test-schema` drops filter arguments that contain parentheses (a quoting
   defect in the recipe). Not fixed here, because it is a justfile change outside
   this feature.

### Gate check

- Phase 1 changed no package source; the files are this log, `plan.md` checkboxes and
  frontmatter, and `spikes/*.md`.
- Scope calculator on the whole working tree
  (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: ["schematic-define", "schematic-gen", "test-toolkit"]`,
  `change_class: package`. The packages come entirely from the pre-existing
  fidelity-polish edits. On this phase's five files alone, the result is
  `packages: []`, `change_class: documentation`.
- Because the whole-tree output is not empty, the gates were **not skipped**. The
  `just test` (exit 0, 2993 + 16 + 1 passed) and `just lint` (exit 0) recorded under
  Baselines ran on this same tree. This phase made no source edits after them, so
  they are this phase's gates. No dependent-package tests were needed, because
  Phase 1 changed no package source. No cross-OS check was needed (no code).
- No skill updated: Phase 1 changed no behavior. The `schematic-define` skill is
  updated in the phases that change authoring.
- Spec frontmatter: this phase's prompt asked for no spec properties, so the spec
  is untouched.
- Requirement-to-test mapping: there are no behavior changes, so there are no
  new tests. Phase 1's evidence is the spike reports, the baseline runs, and the
  pager anchor run listed above.

## Phase 2

Phase 2 builds `schematic_runtime::validate` as a self-contained library and
bumps the runtime ABI. Nothing generated calls it yet. All four files were
written by one implementer (no subagents): they share one design, and the
waves' "disjoint files" split would have cost more coordination than it saved.

### What landed

- `runtime/src/validate/` (`mod.rs`, `decimal.rs`, `violation.rs`, `pattern.rs`,
  `plan.rs`, `eval.rs`), declared `pub mod validate` in `runtime/src/lib.rs`.
  Public names follow Rule 9: `Decimal` (+ `DecimalError`, `MAX_EXPONENT` = 10 000),
  `Violation`, `Violations`, `Keyword`, `PathSegment`, `Plan`, `Pattern`, plus
  `Property`, `Literal`, the sealed `Numeric` trait, and the checks `check_value`,
  `check_str`, `check_number`, `check_array_len`.
- `impl From<Violations> for InvalidArgument` (reason = the aggregate text; no
  public error enum changed).
- `ABI` 7 → 8 (`runtime/src/lib.rs`) and `RUNTIME_ABI` 7 → 8 (`gen/src/lib.rs`).
- Dependencies: `num-bigint = "0.4"` and `regex = "1"` in `runtime/Cargo.toml`. Both were
  already locked (0.4.8, 1.12.3); the root `Cargo.lock` gains two edges and no package.
  `schema/Cargo.lock` gains `num-bigint` 0.4.8 and `num-integer` 0.1.47.

### Design decisions (adaptations and adopted defaults)

- **Plans are `&'static` const data, like `pager::spec`.** `Plan` is built with
  `const fn` builder methods (`Plan::new().min_length(1).items(&ITEM)`), with
  children held as `&'static Plan` and properties as `&'static [Property]`.
  The generator will emit plans as `static`s or associated constants. Nothing
  is built at run time. Verified: const-fn chains promote inside `static`
  initializers, and recursive statics (`static TREE` referring to itself
  through `Plan::reference(&TREE)`) compile.
- **`Pattern` lives in its own named `static`.** A nested `OnceLock` temporary
  inside a static initializer is E0492 (verified with rustc 1.98.1), so a plan
  holds `&'static Pattern`, never a `Pattern` inline. Phase 5's emitter must
  write one `static PATTERN_n: Pattern = Pattern::new("..")` per pattern.
- **Adaptation of Rule 10 ("`\d \w` compiled with Unicode off").** A global
  `RegexBuilder::unicode(false)` refuses ECMAScript-valid patterns that Rule 10
  admits: `[^a-z]` ("pattern can match invalid UTF-8") and `[é-ü]` ("Unicode
  not allowed here"), verified in a scratch crate. The runtime `Pattern`
  therefore compiles its source with the `regex` crate's default syntax, and
  **the generator owns the ASCII meaning**: when Phase 3/5 translate an
  admitted pattern, they write `\d` as `[0-9]` and `\w` as `[0-9A-Za-z_]` (and
  `\D`/`\W` as their negations). Rule 10's admission table is still Phase 3's
  deliverable.
- **Numeric limits are JSON number text** (`.maximum("100")`), parsed into
  `Decimal` at check time with no cache (a cache would need interior
  mutability in nested data; see E0492 above). Limit text that does not parse,
  and a nonpositive `multipleOf`, leave the rule unenforced. The generator
  validates both, so neither occurs in generated code.
- **Floats are read through their shortest round-trip text.** For body values,
  `serde_json::Number::to_string()` is the text that is sent. For typed
  parameters, Rust's `Display` gives the same for both `f32` and `f64`, so
  `0.1_f32` is `0.1` and the `f32`-through-`Value` widening noted in S3 does not
  affect parameters. A NaN or infinity, or a body number beyond the exponent
  bound (possible only if `serde_json/arbitrary_precision` were ever unified
  on), fails every declared numeric rule. It is never read as zero.
- **`nullable()` means the Option wrapper.** `null` passes every rule of a
  nullable plan, including `enum`, `const`, and a reference. Without it, `null`
  is data: it passes the kind rules and must be a member of an `enum` that
  declares `Literal::Null`. Phase 5 decides per schema shape (Phase 1 S2
  catalogue) whether to call `.nullable()`. For a 2020-12 `type: [T, "null"]`
  with an `enum`, Schemars already lists `null` in the `enum`.
- **Report order.** Checks append in call order (generated code calls in
  parameter declaration order). Within a value, its own keywords come in
  `Keyword` order, then object entries in bytewise key order (never `Map`
  iteration order, as Phase 1 required), then array items by index, then a
  referenced plan's rules. Declared properties are ordered by key, not by
  declaration.
- **Paths:** `PathSegment::{Name(&'static str), Index(usize), RedactedKey}`. The
  crate root already exports GraphQL's `PathSegment`, so the new one is
  reachable only as `validate::PathSegment` (Rule 9 already names it that way).
  A redacted key renders as `[redacted]` (`redact::REDACTED`), for example
  `body.labels[[redacted]]`, and an empty path renders as `value`.
- **Reasons** carry fixed text plus declared limits (canonical `Decimal`
  `Display`) and array positions (`items 1 and 3 are equal`). They never quote
  an allowed-value set, a constant, a pattern, or a submitted value.
- **A reference cycle that never descends** stops after 32 hops
  (`MAX_REFERENCE_HOPS`); the generator never emits one.
- **ABI bump timing.** Wave 6 bumps the ABI in this phase. Because the committed
  `schema/src/lib.rs` asserts `ABI == SCHEMATIC_RUNTIME_ABI` and the
  artifact-drift tests compare committed output, the bump requires
  regeneration. The plan's "no generated crate changed yet, so no
  regeneration" did not anticipate that. `just generate` (authorized by
  Rule 12) changed exactly the two ABI lines of `schema/src/lib.rs`; no
  OpenAPI, Postman, or AsyncAPI output changed. Phase 5 must **not** bump the
  ABI again. Rule 7 is one increment for the feature.

### Requirement-to-test mapping

| Requirement (plan Wave 4–6) | Test(s) |
| --- | --- |
| Decimal: integers beyond `u64`/`i64` | `validate_decimal::integers_beyond_u64_and_i64_stay_exact_and_ordered` |
| Decimal: `1` vs `1.0` vs `1e0` | `validate_decimal::equal_values_in_different_spellings_are_equal` |
| Decimal: negatives, ordering | `validate_decimal::ordering_is_mathematical_across_signs_and_scales` |
| Decimal: `0.1` divisor vs `0.3`/`0.35` | `validate_decimal::divisibility_by_a_decimal_divisor_is_exact`, `a_divisor_that_is_not_positive_has_no_answer` |
| Decimal: exponent over the bound | `validate_decimal::the_exponent_bound_is_inclusive_and_refuses_beyond_it` |
| Decimal: grammar (malformed text) | `validate_decimal::text_that_is_not_a_json_number_is_a_syntax_error` |
| Decimal: canonical `Display`, re-parse round trip | `validate_decimal::display_is_canonical_and_switches_to_scientific_notation_for_extremes` |
| Decimal: JSON floats read as sent | `validate_decimal::a_json_float_reads_as_the_text_serde_json_writes` |
| Violations: ordering (declaration → key → index → keyword) | `validate_eval::every_violation_is_reported_in_key_index_and_keyword_order`, `validate_report::parameters_report_in_the_order_they_are_checked`, `validate_report::keywords_use_json_schema_spellings_in_report_order` |
| Violations: redacted map key | `validate_eval::map_values_are_checked_under_a_redacted_key_and_declared_properties_by_name`, `validate_report::paths_render_names_indices_and_redacted_keys` |
| Violations: no constructor accepts a submitted value | compile-fail `violation_holds_no_submitted_value` (E0624) and `path_segment_holds_only_static_names` (E0597), run by `compile_fail::slow_compile_fail_cases_report_their_intended_diagnostic` |
| Violations: no value in `Display`/`Debug`/source/aggregate/`InvalidArgument`/`TransportErrorKind` | `validate_report::no_rendering_of_a_report_contains_submitted_values_or_declared_sets` |
| Violations → `InvalidArgument` text | `validate_report::one_violation_reads_as_a_sentence_and_several_are_counted` |
| Pattern: compile once (counter in test build) | `validate::pattern::tests::a_pattern_compiles_once_however_often_it_is_matched` (lib unit test) |
| Pattern: compile failure is unenforced, no panic, not retried | `validate::pattern::tests::a_source_that_does_not_compile_is_unenforced_and_does_not_panic` |
| Pattern: unanchored search, Unicode input | `validate_eval::patterns_search_unless_anchored_and_handle_unicode_input` |
| Length counts code points (astral, combining) | `validate_eval::string_length_counts_code_points_not_bytes_or_utf16_units` |
| Inclusive/exclusive endpoints | `validate_eval::inclusive_and_exclusive_endpoints` |
| Exact bounds and decimal divisibility in the evaluator | `validate_eval::bounds_beyond_u64_and_decimal_divisors_are_exact`, `reasons_name_the_declared_limit_in_canonical_form` |
| Absent vs empty vs null | `validate_eval::an_empty_string_or_array_is_checked_and_an_absent_property_is_not`, `null_passes_a_nullable_plan_and_is_data_for_an_enum_otherwise`, `a_nullable_reference_admits_null_and_checks_the_target_otherwise` |
| Type-specific rules only on matching kinds | `validate_eval::a_rule_applies_only_to_values_of_its_own_json_kind` |
| `uniqueItems` deep equality (member order, `1` = `1.0`) | `validate_eval::unique_items_compares_objects_regardless_of_member_order_and_numbers_exactly` |
| `enum`/`const` whole-value equality | `validate_eval::enum_and_const_compare_whole_json_values` |
| Recursion over a finite nested value | `validate_eval::recursion_follows_the_finite_value_through_named_plans`, `a_reference_cycle_that_never_descends_terminates` |
| Typed helpers before joining/exploding | `validate_eval::check_str_applies_string_enum_and_const_rules`, `check_number_is_exact_for_every_primitive`, `check_array_len_checks_counts_before_joining` |
| Unparseable limit text is unenforced | `validate_eval::limit_text_that_does_not_parse_leaves_its_rule_unenforced` |
| Doctest per public helper | 8 doctests (`validate` module, `Decimal`, `Pattern`, `Plan`, `check_value`, `check_str`, `check_number`, `check_array_len`) |
| ABI 8 in runtime and generator agree | existing `the_generator_emits_the_abi_of_the_workspace_runtime`; the regenerated `schema` crate builds against ABI 8 |
| GitHub page-size refusal unchanged | `cargo nextest run --manifest-path schema/Cargo.toml -E 'test(github_page_size_outside_the_authored_bounds_is_refused_before_sending)'` → 1 passed (against ABI 8) |

Tier and placement: all new integration tests are L1, in `runtime/tests/l1/`,
declared in `main.rs` (`validate_decimal`, `validate_eval`, `validate_report`).
No name carries a tier marker. The two compile-fail cases are picked up by the
existing glob in `compile_fail.rs`. The `layout` gate covers the new files.

Input Robustness Matrix: Phase 2 adds no file-format or configuration loader.
`Decimal::parse` reads one JSON-number token, and its malformed shapes are
covered by `text_that_is_not_a_json_number_is_a_syntax_error`. The matrix
applies to Phases 3 and 4. Smell grep over `runtime/src/validate/`:
- No `#[serde(default)]`, `unwrap_or_default`, or `filter_map`.
- `.ok()` occurs four times, each deliberate:
  - limit and divisor text that does not parse leaves the rule unenforced
    (documented);
  - `Regex::new(..).ok()` makes a non-compiling pattern unenforced
    (documented, tested);
  - integer/float → `Decimal` text, which always parses for finite values;
  - a body number beyond the bound becomes `None`, which fails every numeric
    rule (it is not dropped).
- The one fallback-to-zero (`Decimal::from_json` returning zero on a parse
  failure) was found by this grep and removed. `from_json` now returns
  `Result`.

### Review agenda (Phase 2 additions)

1. `Decimal` limit text is re-parsed on every check (no cache, because of E0492).
   This is microseconds per limit. Revisit only if a profile shows it.
2. `uniqueItems` compares item pairs (O(n²)). That is fine for request
   payloads; flag it if a definition puts `uniqueItems` on large arrays.

### Gates and verification (all on macOS, this host)

- `just lint` (from `schematic/`): exit 0. The first run flagged
  `clippy::type_complexity` on the bounds table in `eval.rs`, which was
  refactored into a `bound` helper.
- `just _doctest schematic-runtime`: 11 passed (8 new).
- `just test` (from `schematic/`), run twice. The second run was on the final
  tree, after `Decimal::from_json` became fallible, and exited 0. The first
  run's summaries were: main L1 3029 passed (22 skipped; four SLOW TLS
  `builder::` tests, pre-existing), `test-fresh-output` 16 passed
  (`module_features::slow_feature_selections_compile_only_their_module_groups`
  SLOW, pre-existing `slow_` test), `test-compile-fail` 1 passed (now 14
  cases, 2 new).
- Generated crate: `just test-schema` → 152 passed. The pager anchor
  `github_page_size_outside_the_authored_bounds_is_refused_before_sending`
  passed against ABI 8 before that.
- Dependents of the regenerated `schematic-schema`: `cargo check -p
  eversolo-integration -p arcam-amp-integration -p unfolded-integration-helper
  -p homelab -p sony-receiver-integration -p sniff -p biscuit-speaks` → builds.
  No dependent uses `validate`; the ABI assert is the only coupling.
- Narrowed test-input readers, from the scope calculator:
  - `sniff` `cargo_lock_typed_parse_matches_reference_on_workspace_lockfile`
    (reads `Cargo.lock`): 1 passed.
  - `test-toolkit` `the_ci_documentation_states_the_implemented_behavior`
    (reads `docs/dependencies.md`): 1 passed.
  - The whole `test-toolkit` package: 477 passed. It also reads
    `schema/src/lib.rs` through a path scan I could not pin to one test, so I
    ran all of it.
  - `darkmatter` l1 skill readers (`ambient_ctx_capture`,
    `current_root_migration_guard`, `current_root_documentation_contract`,
    `schema_phase_validation`, `schema_roots`, `shipped_schema_corpus`):
    62 passed.
  - `schematic-define` `authoring_guidance` (reads the skill files): ran
    inside `just test`.
- **Pre-existing failure, not caused by this phase:**
  - Failing test: `test-toolkit::archive_path_guard
    no_archive_executed_target_bakes_in_a_producer_path`.
  - It reports `env!("CARGO_MANIFEST_DIR")` at
    `schematic/gen/tests/l1/fixed_header_export.rs:281` (from commit
    `9ddc8861b`) and `schematic/gen/tests/l1/upstream_hygiene.rs:11` (from
    commit `0fb7a4a07`). Both are fidelity-polish test files that Phase 2 did
    not touch.
  - It fails the same way alone. Fixing it is outside this feature's scope,
    so it is logged and left undone. The fix is to use
    `biscuit_test_harness::manifest_dir!()` at both sites. CI's
    archive-scoped guard may flag it on a branch that schedules
    `schematic-gen`.
- Cross-OS: not run. The module is pure computation (no paths, I/O, `cfg`,
  or OS crates). The new trybuild `.stderr` files use the same `:::`
  source-path form as the existing cases.
- Scope calculator, run after all edits:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["schematic-gen", "schematic-runtime", "test-toolkit",
  "darkmatter", "schematic-define", "sniff"]`, `change_class: package`. The
  gates were therefore required, and they ran (above).

## Phase 3

Phase 3 adds the definition-side model: `schematic_define::constraints`, its
attachment to parameters, form fields, and request body roots, definition
checks at endpoint build, API build, bundle load, and generator entry, pattern
admission, page-size lowering, and the registered-model classification helper.
Nothing generated enforces constraints yet (Phase 5).

I wrote the shared model (Wave 7) and the definition checks myself. The two
other Wave 8 tasks ran as parallel foreground subagents with disjoint write
ownership:
- pagination lowering: `define/src/pagination/*`, `docs/pagination.md`;
- registered-model rules: `define/src/constraints/registered.rs`,
  `docs/typed-registration.md`.

### What landed

- `define/src/constraints/` holds four parts:
  - `mod.rs`: `Constraints` (`#[non_exhaustive]`, `Default`, fluent builders
    with the Rule 9 names, plus `unenforced_pattern`), re-exports.
  - `number.rs`: `ConstraintNumber` (decimal text, `From` every integer and
    float type, `&str`, `String`), a private exact `Exact` decimal with `Ord`,
    and `MAX_EXPONENT` = 10 000 (same bound as the runtime `Decimal`).
  - `pattern.rs`: `admit_pattern` → `PatternAdmission::{Enforced { translated },
    Unsupported { reason }, Invalid { reason }}`.
  - `check.rs`: `constraint_problems(&RestApi)` → `Vec<ConstraintProblem>`
    (endpoint, input path, keyword, reason), and the per-endpoint check.
  - `registered.rs` (subagent): `unenforceable_rules(&RestApi, &TypeRef)` →
    `Vec<UnenforceableRule>` for Phase 5.
- New fields, each `Option<Box<Constraints>>`, omitted when absent, with
  `null` rejected:
  - `ParamDef::constraints` (builder `ParamDef::constraints(..)`);
  - `FormField::constraints` (builder `FormField::constraints(..)`);
  - `Endpoint::request_constraints` (builder
    `EndpointBuilder::request_constraints(..)`).
- `DefinitionError::InvalidConstraint { api, problem: Box<ConstraintProblem> }`,
  rendered as `{api} endpoint {id}: {path} `{keyword}` {reason}`. It is
  raised by endpoint build (api `<endpoint>`), API build, and
  `DefinitionBundle::from_json` (all through `validate_endpoint_vocabulary`).
- Generator entry: new `Feature::TypeConstraint` (`type-constraint`), an error
  in `validate_rust_client`, `validate_openapi`, and `validate_postman`. A
  constrained WebSocket handshake parameter is a `websocket-definition` error
  (`ws_codegen/plan.rs` `lower_params`).
- Pagination (subagent):
  - `pagination::page_size_constraints(&PageSize)` and
    `pagination::adopted_page_size(api, endpoint) -> Option<AdoptedPageSize>`
    (location, name, effective constraints);
  - `PaginationError::PageSizeConstraintConflict { param, conflict:
    PageSizeConflict }`, rule `page-size-constraint-conflict`, group
    `pagination-parameter`.
- Bundle: `OMITTED_DEFAULTS` gains `unique_items` and `pattern_unenforced`.
  `reject_nulls` and `omit_nulls` exempt a constraint's `allowed_values` and
  `constant` (`is_constraint_data`), where null is data.
- Dependencies: `regex` 1 and `regex-syntax` 0.8 in `define/Cargo.toml`. Both
  are already locked. Root `Cargo.lock` and `schema/Cargo.lock` gain two edges
  each (`cargo metadata` updated the schema lock).
- Struct-literal fallout: adding three public fields broke 330+ `ParamDef`,
  `FormField`, and `Endpoint` literals in define and gen tests and sources. A
  compiler-driven script added `constraints: None` /
  `request_constraints: None` at each E0063 site, and nothing else.
  - `gen/src/definition_gen.rs` `param_def_tokens` now emits
    `constraints: None` (Phase 4 emits real ones).
  - `definitions/src/openai/mod.rs` was patched to exactly that emitted form
    (287 literals).
  - `just import-openai` confirmed that the file matches the generator,
    except for the two re-import differences `import_overlay` pins
    deliberately (`info.version`, the realtime `session` part), which
    predate this feature. So I kept the patched file and did not adopt the
    whole re-import. Phase 4's re-import must keep honoring those pins.

### Design decisions (adaptations and adopted defaults)

- **Adaptation of Rule 9: `request_constraints`, not `.constraints`, on
  `EndpointBuilder`.** There is no separate body builder. A bare
  `EndpointBuilder::constraints` would read as endpoint-wide, so the
  body-root method is `request_constraints`. `ParamDef::constraints` and
  `FormField::constraints` keep the Rule 9 name.
- **Adopted default: `Constraints::unenforced_pattern(source)` plus a
  serialized `pattern_unenforced: true` (source: spec "Shared helpers",
  imported patterns stay unenforced with a warning; Rule 10).**
  - The data model has to tell an authored pattern (a definition error
    outside the subset) from an imported one (preserved, unenforced).
  - Phase 4's importer sets the mark, and Phase 5 labels it in rustdoc.
  - A later `.pattern(..)` clears the mark.
- **Adopted default: numeric limits serialize as JSON strings** (`"maximum":
  "100"`). Source: Rule 3 and S1, "carry exact decimal text out of band".
  The bundle loader parses through `serde_json::Value`, which would narrow a
  JSON number. A JSON number in that slot is a wrong-type error.
- **Adopted default: serialized keys use snake_case Rust field names**
  (`min_length`, `allowed_values`, `constant`), like every other bundle
  member (source: the bundle's existing convention). OpenAPI spellings appear
  only in diagnostics (`keyword`) and in export (Phase 4).
- **Adopted default: type compatibility.**
  - String rules apply to `String`, `Enum`, `Char`, and text form fields.
  - Numeric rules apply to integer and number types.
  - `format` applies to strings and numbers (for OpenAPI's `int32`/`int64`).
  - Array rules and `items` apply to arrays.
  - `enum`/`const` apply to any type, and each member must be of the
    value's type. `null` is allowed only for an `Optional` or free-form JSON
    value.
  - `Json` parameters and `Primitive(Json)` accept everything.
  - Refused targets, where any rule is an error:
    - a `Named` component, at the root or as array items (Rule 4: Schemars
      attributes are the only source);
    - `OneOf`/`AnyOf`/`AllOf`;
    - `Bytes`;
    - file fields, binary bodies, and form bodies;
    - an endpoint without a body.

  Source: spec "Definition errors versus unsupported rules" (incompatible
  authored types) and Rule 8.
- **Adopted default: an empty `format` name is a definition error.** Unknown
  names are preserved (Input Robustness exception).
- **Adopted default: `FieldDef` gets no constraint field in Phase 3.** Wave 7
  names only parameters, inline body roots, and form fields. Imported model
  declarations are rebuilt through Schemars attributes or explicit schema
  metadata by Phase 4 Wave 10, which owns that shape.
- **Adopted default (pagination, subagent): an adopted page-size parameter
  must either declare no bounds or repeat exactly the strategy's bounds.**
  - That means `minimum = min`, and `maximum = max`, absent exactly when the
    strategy has no max. A one-sided declaration fails.
  - Any other value rule fails, because the pager owns the value and
    `validate()` cannot see it. `format` is allowed.
  - Source: spec "Pagination integration", "conflicting bounds fail
    definition validation"; plan Wave 8.
- **Pattern subset, as implemented** (Rule 10). The table is in
  `schematic/docs/value-constraints.md`, and a test reads it row by row.
  - Admitted, with `\d`/`\w`/`\D`/`\W` rewritten to ASCII classes (nested
    inside brackets): literals, `\xHH`, `\uHHHH`, ordinary escapes, `^`, `$`,
    capturing and `(?<name>)` groups, `(?:..)`, alternation, greedy and lazy
    quantifiers, and bracketed classes.
  - Refused as unsupported:
    - `.`, `\s`, `\b`/`\B`, `\p{..}`, inline flags, and `(?P<..>)`;
    - `\A`/`\z`, `\U`/`\x{..}`, octal, `\a`, and an escaped space;
    - POSIX classes, a `[` inside a class (nested in Rust, a literal in
      ECMAScript), and class set operations;
    - a class starting with `]`, and `{,n}`;
    - anything Rust's parser rejects that ECMAScript may accept (look-around,
      back-references).
  - Classified invalid only when ECMAScript also rejects it: unclosed or
    unopened group, nothing to repeat, decreasing repetition or class
    range, trailing backslash, duplicate group name.
  - An admitted pattern must also compile with `regex::Regex::new` (size
    limits).
- **The definition check reports the first problem** through
  `DefinitionError` (matching every other build check).
  `constraint_problems` returns all of them, and the generator emits one
  diagnostic per problem.
- **The new fields hold `Option<Box<Constraints>>`.** The first full
  `just test` aborted with a stack overflow in `schematic-definitions`
  `openai_pagination::tests::get_response_returns_json_or_server_sent_events_and_every_other_endpoint_keeps_one_shape`.
  An inline `Constraints` (about 400 bytes) grew each of the 287 `ParamDef`
  values in the imported OpenAI builder functions past the debug-build
  stack. Boxing keeps `ParamDef`, `FormField`, and `Endpoint` one pointer
  larger. The builders still take a plain `Constraints`.
- **`DefinitionError::InvalidConstraint` boxes its problem.** Four unboxed
  strings made `DefinitionError` large enough to trip clippy's
  `result_large_err` across `schematic-gen`.

### Requirement-to-test mapping

| Requirement (plan Wave 7–8, robustness section) | Test(s) |
| --- | --- |
| Builders, stored values, every input kind (query, path, array items) | `value_constraints::valid_rules_on_every_input_build_and_are_stored`; 21 builder doctests in `constraints` |
| Bodies and form fields: allowed (text, non-component JSON root, items, text/JSON fields) vs refused (binary, component root, component items, form body, no body, file, files, numeric rule on text field) | `value_constraints::bodies_and_form_fields_take_rules_only_where_a_value_is_constrainable` |
| Every semantic definition error is located (api/endpoint/path/keyword/reason): incompatible type, min>max lengths and counts, empty interval (incl. equal exclusive and mixed), nonpositive `multipleOf`, malformed/NaN/inf/exponent limits, empty enum, wrong enum element (one/every), null enum member on a non-nullable type, wrong const, unsupported and invalid patterns, empty format, item rules on wrong item type | `value_constraints::each_semantic_definition_error_is_located_by_api_endpoint_path_and_keyword` (24 rows, endpoint build) and `a_loaded_rule_located_error_names_the_api_endpoint_path_and_keyword` (bundle load, full message) |
| Exceptions: `pattern: ""`, unknown format kept, unenforced pattern kept, null as enum/const data on JSON, equal inclusive bounds | `value_constraints::exceptions_to_the_default_outcome_are_accepted`, `rules_on_a_json_body_admit_null_only_through_an_optional_root` |
| Field-shape matrix per validator (absent, null, wrong whole type, wrong element/every element, empty), one wired cell per other field | `value_constraints::serialized_field_shapes_per_validator` (47 cells, each from one edit of a canonical bundle) |
| Loader (bundle JSON): control with new metadata, repeated read/write/read, duplicate key, trailing content, explicit default accepted, old snapshot without metadata reads as absent | `value_constraints::bundle_control_round_trips_the_rules_exactly_through_repeated_reads_and_writes`, `bundle_document_shapes_duplicate_key_and_trailing_content_fail`, `bundle_without_constraint_metadata_reads_as_absent` (incl. committed `fixtures/bundles/fixture.json`) |
| `null` in `enum`/`const` preserved as data through write and read (found a writer bug: `omit_nulls` dropped `"constant": null`; fixed) | `value_constraints::null_in_enum_and_const_is_preserved_as_data_through_a_bundle` |
| Lossless large integer and decimal limits in serialized form | `bundle_control_round_trips…` (`"18446744073709551616"`, `"0.1"`) |
| Exact number grammar, ordering, exponent bound | `constraints::number::tests::{spellings_of_one_value_are_equal, ordering_is_mathematical, malformed_text_and_huge_exponents_are_refused}` |
| Pattern admission table (fixture and docs, one source) | `value_constraints::pattern_admission_matches_the_documented_table` (35 rows from `docs/value-constraints.md` via `include_str!`), `translated_digit_and_word_classes_are_ascii_only` |
| Generator entry rejects a rule edited after build, in every output | `gen l1 output_validation::a_contradictory_constraint_edited_after_build_blocks_every_output` |
| Feature id serialization | `gen diagnostics::tests::display_and_serialized_ids_agree_for_every_variant` |
| WebSocket handshake parameter constraints refused (Phase 1 adopted default) | `gen ws_codegen::plan::tests::a_constrained_handshake_parameter_is_an_error_naming_its_location` |
| `Default` allow-list includes `Constraints` | `define builders` Default audit (corpus now includes `constraints/mod.rs`) |
| Pagination lowering and conflicts (agreeing, none, format only, conflicting min, conflicting max, one-sided both ways, max without strategy max, extra keyword, located via API build and bundle load) | `pagination_constraints::*` (13 tests) + `page_size_constraints`/`adopted_page_size` doctests |
| Registered-model rules: Schemars attributes on root, nested, `inner`, renamed, `Option` fields land once in components; shared model stored once; `extend` keywords reach the component; classification empty for nullable/recursive shapes, located for untagged enum, `oneOf`/`allOf`/`not`, unresolved ref | `registered_model_rules::*` (5 tests) + `unenforceable_rules` doctest |

Tier and placement: every new test is L1. `define` auto-discovers
`tests/*.rs`, so the new binaries are `value_constraints`,
`pagination_constraints`, and `registered_model_rules`. The gen test is in
the consolidated `tests/l1/output_validation.rs`, which is already declared.
No name carries a tier marker. The CLI loader wiring cells are not spawned,
because the bundle loader has no CLI subcommand of its own; the
generator-entry wiring is proven in-process through the public
`validate_*` functions.

Input Robustness notes:
- The serialized-definition loader is JSON-only (Phase 1 discrepancy 4), so
  the YAML cells do not apply.
- The OpenAPI loader row is Phase 4.
- Smell grep over `define/src/constraints/` and `define/src/pagination/lower.rs`:
  - `#[serde(default)]` appears without `non_null` only on the two Booleans
    `unique_items` and `pattern_unenforced`, where absent and `false` mean
    the same thing (a documented exception); `null` for them is still a
    type error.
  - Every `Option` member uses `serde_presence::non_null` or `non_null_map`.
  - `unwrap_or_default` appears once (`adopted_page_size`): a parameter
    without constraints is the empty set, which is its meaning.
  - `.ok()` appears three times, all in `registered.rs` `percent_decode`: a
    malformed escape yields `None`, so the reference is reported as
    unresolved, not dropped.
  - No `filter_map(.. as_str())`.

### Review agenda (Phase 3 additions)

1. `EndpointBuilder::request_constraints` replaces Rule 9's "body builder
   `.constraints`" (see above).
2. Numeric limits are JSON strings in bundles. Readers that expect JSON numbers
   must adapt.
3. `enum`/`const` members that are JSON numbers beyond `u64`/`f64` precision
   still narrow in a bundle, because the loader parses through
   `serde_json::Value`. Only the limit keywords are lossless as text. This
   case is rare, and Phase 4's lossless capture may address it for imports.
4. `unenforceable_rules` reports a unit enum's `enum` beneath a composition,
   though the Rust type enforces it. Phase 5 decides whether that is an error
   or a label.
5. `registered.rs` duplicates the private `percent_decode` of `components.rs`;
   making one `pub(crate)` would remove the copy.
6. The CLI-spawn wiring cell for the bundle loader has no CLI entry point to
   spawn. The generator's in-process entry is tested instead.

### Gates and verification (all on macOS, this host)

- `just lint` (from `schematic/`): exit 0 on the final tree.
  - The first run failed: `clippy::result_large_err` across
    `schematic-gen`, because the unboxed `InvalidConstraint` variant grew
    `DefinitionError`. Fixed by boxing the problem.
  - Both subagents had also hit `match_like_matches_macro` in `check.rs`;
    fixed.
- `just test` (from `schematic/`): exit 0 on the final tree. Main L1 run:
  3065 passed, 22 skipped (4 SLOW, the pre-existing TLS `builder::` tests);
  `test-fresh-output` 16 passed; `test-compile-fail` 1 passed.
  - The first run aborted on a stack overflow in `schematic-definitions`
    `openai_pagination::tests::get_response_returns_json_or_server_sent_events_and_every_other_endpoint_keeps_one_shape`.
    This was a real regression from the inline `Constraints`, fixed by
    boxing (see Design decisions). That test then passed alone, and in the
    full rerun.
- `just doctest`: exit 0 (define 201 passed). The first run failed one
  doctest, a `ParamDef` struct literal in the `params` module docs, which
  needed `constraints: None`; fixed.
- Dependents:
  - `just test-schema`: 152 passed. The `schematic-schema` workspace
    resolves `schematic-define`; its lock gained the two `regex` edges.
  - `cargo check -p model-citizen` (depends on `schematic-definitions`):
    builds.
  - `schematic-oauth` ran inside `just test`.
- `just import-openai`: the importer reproduces the patched
  `definitions/src/openai/mod.rs`, except for the two differences
  `import_overlay` pins (see What landed).
- Narrowed test-input readers, from the scope calculator:
  - `sniff`
    `cargo_lock_typed_parse_matches_reference_on_workspace_lockfile`: 1 passed.
  - `test-toolkit` `the_ci_documentation_states_the_implemented_behavior`:
    1 passed.
  - The `darkmatter` l1 skill readers (the same six groups as Phase 2): 63
    passed.
  - The pre-existing `test-toolkit::archive_path_guard` failure recorded in
    Phase 2 concerns fidelity-polish test files this phase did not touch; I
    did not rerun it.
- Scope calculator, run after all edits (whole working tree, which still
  includes the uncommitted Phase 2 work):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["schematic-define", "schematic-definitions", "schematic-gen",
  "schematic-runtime", "test-toolkit", "darkmatter", "sniff"]`,
  `change_class: package`. The gates were required, and they ran (above).
- Cross-OS: not run. The new code is pure computation (decimal text,
  `regex-syntax` AST walk, serde), with no paths, I/O, `cfg`, or OS crates.
  `regex`/`regex-syntax` already build on every supported OS in this
  workspace. Windows, Linux, and WSL2 were not exercised for this phase.
- Spec frontmatter: this phase's prompt asked for no spec properties, so the
  spec is unchanged.
