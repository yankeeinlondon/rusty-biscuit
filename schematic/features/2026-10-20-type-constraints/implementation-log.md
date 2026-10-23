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
packages: []
human_review: false
message_to_agent: |-
  Phase 1 was research only; read the "## Phase 1" section of this log before
  starting, especially "Frozen decisions" and "Contract check". Key facts that
  differ from the plan text:
  - ABI is 7 today, so Rule 7 raises it to 8 (runtime/src/lib.rs ABI and
    gen/src/lib.rs RUNTIME_ABI together).
  - Use num-bigint 0.4.x (already in Cargo.lock) for the Decimal mantissa;
    exponent bound +/-10000. Add the dependency stanza in Wave 4.
  - The GitHub pager anchor is schematic-schema
    pagination::github_page_size_outside_the_authored_bounds_is_refused_before_sending
    (sizes 0 and 101), plus runtime/tests/l1/pager_rules.rs
    a_size_outside_the_declared_bounds_is_refused_naming_the_bound_with_zero_requests.
    `just test-schema -E 'test(..)'` breaks on the parentheses; run
    `cargo nextest run --manifest-path schema/Cargo.toml -E ...` instead.
  - Stored components mix dialects: the root is OpenAPI 3.0 shaped
    (nullable: true, enum:[null]) and nested definitions are 2020-12 shaped
    (type:[T,"null"], {"type":"null"}, const). The evaluator and plan compiler
    must accept both; treat anyOf as Option only via admits_only_null
    (define/src/openapi/export/conversion.rs).
  - Order object keys explicitly in the evaluator; never rely on serde_json Map
    iteration order (preserve_order is off today but could be unified on later).
  - The working tree carries unrelated uncommitted fidelity-polish edits in 12
    files (listed in the log); merge with them, never revert them.
  Full spike evidence is in spikes/ next to this log.
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
