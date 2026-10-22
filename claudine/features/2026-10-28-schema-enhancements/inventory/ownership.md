# Phase 1 Inventory: API and File Ownership

Snapshot of the tree on `feat/pipeline` at `e116c6acc`, taken 2026-10-21.
Paths are relative to the repository root. `lib` means `darkmatter/lib/src`,
`schemas` means `lib/markdown/schemas`, and `expr` means
`lib/markdown/compose/expression`. Line numbers are from this snapshot.

The plan's rule (plan.md, "Scope"): shared types, dispatch wiring, Cargo
feature changes, and consolidated test registration have **one integrator**.
Parallel subagents own only the files listed for them below.

## 1. Modules the plan names

### Schema conversion: `schemas/coerce.rs` (2,125 lines)

- **Public API:**
  - `CoercionTarget` (:58)
  - `CoercionOutcome` (:105)
  - `coercion_target` (:162)
  - `coerce_frontmatter` (:335)
  - `coerce_frontmatter_with_pending` (:352)
  - `coerce_value_against_schema` (:530)
- **Owns today:** frontmatter conversion rules. It recognizes `type` (a single string only; type arrays are ignored, :191), `anyOf` (boolish, numberlike, nullable, and item unions), `const`, `format` (yaml/json), `items`, `properties`, and `x-darkmatter-*`. It does not read `$ref`, `$defs`, `oneOf`, `allOf`, `patternProperties`, `additionalProperties`, or `prefixItems`. Numeric text is converted at :597.
- **Callers:**
  - `schemas/mod.rs`: `EffectiveSchema::validate` (:959), `validate_with_positions` (:976), and `normalize_frontmatter` (:1253)
  - `example.rs`
  - `format.rs`
  - Outside the crate: **`claudine/gen/src/inputs.rs:292`**
- **Changed by:** Wave 4 Schema adapter, which replaces the rules with the Wave 3 core. Wave 2 Numeric foundations also needs the numeric seam at :597.
- **Proposed owner:** Schema adapter (parallel). Numeric foundations must not edit this file; it hands over an admission function that the adapter calls.

### Schema references: `schemas/reference.rs` (130 lines)

- **Public API:** `classify_schema_reference` (:64), re-exported at `schemas/mod.rs:116`. It rejects `http(s)` references as `RemoteUnsupported`.
- **Callers:** `dmls/src/diagnostics/frontmatter.rs`.
- **Changed by:** Wave 4 Schema adapter (reference identity and dialect).
- **Proposed owner:** Schema adapter (parallel).

### Value rewriting: `schemas/rewrite.rs` (1,156 lines)

- **Public API:**
  - `NormalizationOutcome` (:50, re-exported)
  - `rewrite_eager_file_values` (:75), called from `schemas/mod.rs:1269`
  - It also reads `type` (:263, :271).
- **Changed by:** Wave 4 Schema adapter.
- **Proposed owner:** Schema adapter (parallel).

### Example envelopes: `schemas/example.rs` (493 lines)

- **Public API:**
  - `EXAMPLE_ENVELOPE_YAML` (:75)
  - `validate_example_bytes` (:139)
  - `validate_example_object` (:176)
  - `validate_returns_against_target` (:237)
- **Callers:** `resolve.rs`.
- **Changed by:** Wave 4 Schema adapter. The Wave 5 Catalog replacement *reads* it for the file form of examples.
- **Proposed owner:** Schema adapter (parallel). The catalog task consumes it read-only.

### Content formats: `schemas/format.rs` (1,126 lines)

- **Public API:**
  - the `DARKMATTER_*` constants (:76–140)
  - `register_darkmatter_formats` (:205)
  - `is_pending_expression_value` (:302), used by DMLS `diagnostics/frontmatter.rs`
  - keyword factories (:438–508)
- **Owns today:** the `json` format at :260 uses `serde_json::from_str(..).is_ok()`, which accepts duplicate keys. The `yaml` format at :255 uses `biscuit_file::Yaml`, which rejects them.
- **Changed by:** Wave 4 Schema adapter (custom formats with canonical admission).
- **Proposed owner:** Schema adapter (parallel).

### Simplified schemas: `schemas/simplified/{types,grammar,source,serialize,convert,standalone}.rs`

- **Public API:**
  - `parse_standalone_schema_document` (`standalone.rs:123`)
  - `parse_schema_declaration` (`simplified/mod.rs:86`)
  - `parse_yaml_schema` (`mod.rs:117`)
  - `parse_property_definition_at_depth` (`mod.rs:314`)
  - `grammar::parse_type_expr`
- **Callers in DMLS:**
  - `overlay/mod.rs:410`
  - `overlay/registry.rs:93`
  - `overlay/suggestions.rs:152`
  - `overlay/schema.rs:1217`, `:1314`, `:1369`
  - `overlay/named_types.rs:231`
  - `diagnostics/named_types.rs:344`
- **Changed by:** Wave 2 Grammar descriptors (parallel).
- **Also touched by Wave 2 Numeric foundations:** `grammar.rs:279` (`lex.parse::<f64>().ok()?`) and `grammar.rs:1948` read numbers inside type expressions with no exactness check. This is **a file both parallel Wave 2 tasks would touch.**
- **Proposed owner:** Grammar descriptors. Numeric foundations publishes the admission function. Grammar descriptors calls it at those two sites, after the numeric API is frozen; until then the sites are left alone.
- **Not in any list:** `simplified/mod.rs` (the named-type parser and the null-type rule at :321–324) and `simplified/yaml_scalar.rs`. Grammar descriptors should own them.

### Global catalog: `schemas/catalog.rs` (551 lines)

- **Public API:** `GlobalCatalog::{darkmatter :195, from_documents :210, from_file :227, from_host_file :252, load :299, host_catalogs :396, is_global_catalog :434}`.
- **Owns today:** the global variable catalog (`kind: schema` plus `$schema`). This is **not** the function catalog. Its loader is already strict (catalog.rs:299–361), and it is the model for the new function-catalog loader.
- **Changed by:** no task. It may be touched only if `categories:` must be accepted there; the spec's "any schema file" rule (spec.md:380) suggests it must.
- **Proposed owner:** Grammar descriptors, if needed.

### Raw schema loading: `schemas/resolve.rs` (5,699 lines)

- **Public API:**
  - `resolve_schema` (:133) and `resolve_schema_with_roots` (:149)
  - `load_schema_from_path_in_context` (:708), which dispatches on the file extension at :723
  - `parse_yaml_referenced_file` (:729)
  - `parse_raw_json_schema` (:1150)
- **Owns today:** the raw JSON Schema loader, which reads both `.json` and YAML-spelled raw schemas.
- **Changed by:** the plan assigns this file to **no** task. It is still the raw-schema reader that the matrix and the duplicate-key rule (rule 9) apply to.
- **Proposed owner:** Schema adapter (parallel). Add this file to its ownership list.

### Schema validation: `schemas/validate.rs`

- **Owns today:** builds the `jsonschema` 0.55 validator with `Draft202012` (:504).
- **Changed by:** Wave 4 Schema adapter (full-validator candidate filtering).
- **Proposed owner:** Schema adapter.

### `EffectiveSchema` wiring: `schemas/mod.rs` (4,097 lines)

- **Owns today:** the `EffectiveSchema` coerce and rewrite wiring (:959, :976, :1253, :1269), plus the module's re-exports.
- **Changed by:**
  - Wave 4 Schema adapter (calls into the core)
  - Wave 7 Variable projection (adds the type projection to `EffectiveSchema`)
  - Grammar descriptors (re-exports)
- **Proposed owner:** **Integrator.** It is not in any task's list. Parallel tasks request the export or wiring change they need.

### Composition schema stage: `lib/markdown/compose/schema_validation.rs`

Everything here is `pub(crate)`.

- **Symbols:**
  - `run` (:111)
  - `PreparedSchemas` (:134)
  - `prepare_schemas` (:191)
  - `run_with_registry` (:295)
  - `validate_typed_shell_values` (:504)
  - `verify_projection_stability` (:554)
  - `prepare_caller_projection` (:612)
  - `source_path` (:1560)
- **Owns today:** reads `properties`, `allOf`, `anyOf`, and `oneOf` (:922–975) to select the file mode.
- **Callers:**
  - `compose/pipeline/mod.rs:211–329`
  - `pipeline/phases.rs:42`
  - `unknown_identifiers.rs:18`
- **Changed by:**
  - Wave 4 Schema adapter
  - Wave 8 Composition precheck, which adds a call site after schema resolution. That call site is in the pipeline files, not this file.
- **Proposed owner:** Schema adapter (Wave 4). The Wave 8 Composition precheck owns the pipeline files.

### Function catalog: `expr/catalog/{mod,parser,ast,roots}.rs`

- **Public API:**
  - `DataType` (:38)
  - `ParamType` (:133)
  - `ParamRefinement` (:181)
  - `ReturnValueType` (:245)
  - `ReturnType` (:269)
  - `ExpressionFunctionDescriptor` (:313) and its method `typed_signature` (:345)
  - `expression_function_descriptors` (:502)
  - `generate_expression_function_table` (:516)
  - `reserved_root_descriptors` (re-exported :25)
  - crate-internal: `expression_function_catalog` (:484) and `try_parse_catalog` (:495)
  - the parser: `parse_expression_function_catalog` (`parser.rs:158`)
- **Owns today:**
  - It embeds `darkmatter/docs/schemas/expression-functions.yaml` (113 functions, `kind: expression-function-catalog`) at mod.rs:484–491.
  - It uses a serde `Raw*` model with `deny_unknown_fields`.
- **Changed by:** Wave 5 Catalog replacement, which builds a new loader for `darkmatter/schemas/partials/{functions,mutations}.yaml`.
- **Proposed owner:** Catalog replacement (parallel).
- **Exception:** the descriptor type is a **shared type**. Every consumer in section 2 and `functions/mod.rs` import it. The new descriptor shape should be frozen by the integrator at the start of Wave 5, before the three parallel tasks begin.

### Functions and dispatch: `expr/functions/mod.rs` (6,041 lines) and family files

- **What `mod.rs` holds:**
  - the `FunctionBinding`, `FunctionHandler`, and `EvaluationMode` types (:52–80)
  - `LAZY_BINDINGS` (:179) and `BINDING_GROUPS` (:197–213)
  - `validate_registry` (:246) and the `REGISTRY` LazyLock (:305)
  - `dispatchable_canonical_names` (:2948), `dispatchable_signatures` (:2959), `dispatch_fs` (:3004), and `dispatch` (:3046)
  - **most pure handler bodies** (:549–1400+)
  - **27 context handler bodies**, including:
    - path functions: `absolute`, `relative`, `basename`, `dirname`, `join`, `link`
    - `file_exists`, `has_command`, and the file-index functions
    - skill functions: `has_skill`, `has_local_skill`
    - frontmatter functions: `frontmatter`, `try_frontmatter`
    - `find_files`, the markdown body and title functions, and `validate_schema`
  - the tests, at :3065 onward
- **Family files** hold one `BINDINGS` table each: `strings`, `collections`, `dates`, `predicates`, `paths`, `terminal`, `markdown_docs`, `composition`, and so on.
  - The context-heavy families also hold their bodies: `git`, `network`, `shell`, `cicd`, `pull_requests`, `repository`, `agentic_cli`, `provider`.
- **Changed by:**
  - Wave 5 Pure handlers and Wave 5 Context handlers, which run in parallel
  - Wave 5 Catalog replacement, through `validate_registry` and its descriptor imports
  - Wave 6 Registry integration
- **This is the main conflict file.** Pure and Context handlers would both edit `functions/mod.rs`, and Catalog replacement edits the same file's registry join.
- **Proposed split:**
  - Before Wave 5, the integrator moves the context bodies out of `mod.rs` into their family files. This is a move-only change with no behavior change.
  - Context handlers then own the family files.
  - Pure handlers own the pure bodies left in `mod.rs` and the pure family files.
  - The `FunctionBinding` struct, which gains source exclusions and recovery, plus `BINDING_GROUPS`, `validate_registry`, and `dispatch*`, belong to the **integrator** (Wave 6).

### Call evaluation: `expr/mod.rs`

- **Owns today:**
  - the call-evaluation switch, including lazy `and`/`or` (:807) and calls to `dispatch_fs` (:887) and `dispatch` (:899)
  - the `pub use` re-export block (:86–126)
- **Changed by:**
  - Call binder (new exports)
  - Catalog replacement (descriptor exports)
  - Compatibility API (Wave 7)
  - Registry integration (the evaluation switch)
- **Proposed owner:** **Integrator.** Parallel tasks list the exports they need.
- **Naming hazard:** `expr/binding.rs` already exists. It is the host *variable* binding model (`BindingView`, `RuntimeBinding`). Call binder must choose a different module name, such as `call_binding`, so the two do not collide.

### Prepared expressions: `expr/prepared.rs` (419 lines)

- **Public API:**
  - `AuthoredMode` (:54)
  - `PreparedExpression` (:68)
  - `PreparedValue` (:103)
  - `PreparationError` (:128)
  - `ValidationDiagnostic` (:142)
  - `prepare_value` (:161)
  - `validate_prepared` (:264)
  - `validate_expression` (:284)
  - `evaluate_prepared` (:375)
- **Changed by:** Wave 7 Compatibility API, which walks executable calls through every branch.
- **Proposed owner:** Wave 7 library owner (sequential). Its API is frozen before Wave 8.

### DMLS configuration: `darkmatter/dmls/src/config/mod.rs` (1,135 lines)

- **Public API:**
  - `DmlsConfig` (:247) and `DmlsConfig::{load_file :285, discover :323}`
  - `LoadedConfig` (:351)
  - `ConfigSource` (:376)
  - `ConfigDiagnostic` (:431)
  - `ConfigState` (:462) and `ConfigState::{load :477, effective :493, diagnostics :505, apply_client_settings :516, reload :546}`
  - private: `recompute` (:569)
- **Callers:**
  - `router.rs:1180` (`reload_config`) and `router.rs:1185` (`reload_config_file`)
  - the `DidChangeConfiguration` handler at `router.rs:2581`
- **Changed by:** Wave 8 Editor diagnostics (`expressions.strict`).
- **Proposed owner:** Editor diagnostics (parallel in Wave 8). The Composition precheck does not touch DMLS.

### DMLS expression overlay: `darkmatter/dmls/src/overlay/expressions.rs` and providers

- **Public API in `overlay/expressions.rs`:** `function_descriptors` (:623), `function_descriptor` (:630), and `format_function_block` (:685). It consumes `expression_function_descriptors`.
- **Changed by:**
  - Wave 6 Consumer migration (completion and hover)
  - Wave 8 Editor diagnostics (the diagnostics provider)
- **Proposed owner:** These waves are sequential, so there is no parallel conflict. Consumer migration owns it in Wave 6, and Editor diagnostics owns it in Wave 8.

### Claudine context report: `claudine/cli/src/commands/context/`

- **Where the code is:**
  - The function list renders at `expressions.rs:305` (`render_expressions_functions`).
  - The grouping is **in `mod.rs:67`** (`group_expression_descriptors`), which groups by `desc.category` in order of first appearance and sorts by `desc.order`.
  - The report entry point is `render_expressions_report` (`mod.rs:252`).
  - Examples render through `format.rs:91` (`format_example`), which takes `darkmatter::catalog::Example`.
  - Tests: `context/tests.rs` and `claudine/cli/tests/l1/context_command.rs`.
- **Changed by:** Wave 6 Consumer migration. The plan names only `{expressions,format}.rs`, but **`context/mod.rs` must be in scope** because the grouping by `order` lives there. The new catalog drops `order`; categories follow list order, and items are alphabetical.
- **Proposed owner:** Consumer migration (sequential).

## 2. Catalog consumers missing from the plan

Each of these imports `expression_function_descriptors`, `ExpressionFunctionDescriptor`, or `dispatchable_*`. Consumer migration (Wave 6) must update each one, or the integrator must keep a compatible accessor.

| Consumer | Use |
|---|---|
| `darkmatter/cli/src/commands/schema/about.rs:23,497` | `md schema about` renders signatures |
| `darkmatter/lib/src/catalog/mod.rs:449` | unified Darkmatter catalog |
| `darkmatter/lib/src/lifecycle/signatures.rs` | lifecycle signatures |
| `darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs` | context capture |
| `darkmatter/lib/examples/expression_doc_generator.rs` | example target; compile it when the API changes |
| `darkmatter/lib/tests/l1/predict_conflicts.rs` | test |
| `claudine/lib/tests/l1/claudine_schemas.rs` | test |
| `claudine/gen/src/inputs.rs:292` | calls `coerce_*`. Schema adapter (Wave 4) must keep it compiling |
| `darkmatter/lib/src/lib.rs:43` | crate docs link to the descriptor API |

## 3. Files two parallel tasks would both touch

| File | Tasks (same wave) | Resolution |
|---|---|---|
| `schemas/simplified/grammar.rs` (:279, :1948 numeric parse) | Grammar descriptors and Numeric foundations (W2) | Grammar owns the file. Numeric supplies the admission function. Grammar wires it after the numeric API freezes. |
| `schemas/simplified/convert.rs` (numeric constraint lowering) | Grammar descriptors and Numeric foundations (W2) | Same as above |
| `expr/functions/mod.rs` | Pure handlers and Context handlers (W5), plus Catalog replacement (W5) through `validate_registry` and descriptor imports | Move the context bodies out first (integrator, move only). Freeze the descriptor type before W5. The registry join belongs to the integrator. |
| `expr/mod.rs` (`pub use` block) | Call binder (W4), Catalog replacement (W5), Compatibility API (W7) | Integrator |
| `schemas/mod.rs` (re-exports and `EffectiveSchema` wiring) | Schema adapter and Call binder (W4) | Integrator |
| `expr/lexer.rs` and the evaluator | Numeric foundations (W2). Grammar descriptors does not touch them. | No conflict |
| Workspace `Cargo.toml` (`serde_json` `preserve_order`) | Numeric foundations (W2) | Integrator, per the plan's rule on Cargo feature changes |
| Snapshot and consolidated test registration files (`lib/tests/l1/main.rs`-style registries) | Every parallel task that adds tests | Integrator registers. Tasks add test files only. |
| `darkmatter/schemas/partials/*.yaml` | Catalog replacement (W5). Pure and Context handlers *read* the descriptors. | Catalog replacement owns it. Handlers do not edit YAML. |

## 4. ConfigState: comment drift (plan rule 8)

The plan's claim is **only partly right.** There is drift between comment and code, but it is not in the `ConfigState` docs.

**What the code does** (`darkmatter/dmls/src/config/mod.rs`): `recompute` (:569–597) deep-merges overlay over file. If the result fails to deserialize, it pushes an error that says "the file and default settings remain" (:591) and sets `self.effective = self.file.clone()` (:595). That is the file layer, not the previous effective config.

**The docs agree with the code:**

- The `ConfigState` docblock (:452–460) says "a merge that no longer deserializes keeps the file's sections (with an error diagnostic)". This matches the code.
- `apply_client_settings` (:513–515) says a non-object value "keeps the previous overlay". This is also correct: `self.overlay.take()` at :535 puts the old overlay back. It refers to the overlay, not the effective config.
- `darkmatter/docs/lsp/configuration.md:176–181` says the editor settings are ignored and the file layer is kept. This matches the code.

**The drift is in a test:**

- `test_semantic_tokens_malformed_value_keeps_previous` (:1079) and its comment (:1080–1081) say "the last good (default-enabled) config survives".
- The test passes only because the file layer and the default happen to be the same.
- The neighboring test `test_config_state_invalid_overlay_keeps_file_layer` (:1097) states the real contract.

**Proposed fix:** Wave 8 Editor diagnostics renames that test and rewrites its comment to say "file layer". It should also add one cell where the previous *valid overlay* differs from the file, to pin the file-layer fallback.

**Consequence for `expressions.strict`:** an invalid editor `expressions.strict` throws away the **entire** non-formatting overlay. Other valid overlay keys, such as `semantic_tokens.enable`, revert to the file layer too. This is existing behavior, and the matrix's "invalid overlay: file layer" assumes it. Tests should assert it explicitly.
