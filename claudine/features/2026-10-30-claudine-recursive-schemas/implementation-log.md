---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-30-claudine-recursive-schemas/spec.md"
plan: "claudine/features/2026-10-30-claudine-recursive-schemas/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine
    - darkmatter
    - darkmatter-cli
    - dmls
human_review: false
source_files_during_phase_2:
    - claudine/lib/src/composition/lifecycle/tests/mixed_span.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/tests/l1/claudine_schema_parity.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - darkmatter/justfile
    - darkmatter/lib/examples/lifecycle_schema_generator.rs
    - darkmatter/lib/src/effects/mod.rs
    - darkmatter/lib/src/effects/types.rs
    - darkmatter/lib/src/lifecycle/action_shape.rs
    - darkmatter/lib/src/lifecycle/actions/tests.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/operands.rs
    - darkmatter/lib/src/lifecycle/schema_projection.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/lifecycle/source_map.rs
    - darkmatter/lib/src/lifecycle/tests.rs
    - darkmatter/lib/src/lifecycle/tests/overloads.rs
    - darkmatter/lib/src/markdown/coercion/engine.rs
    - darkmatter/lib/src/markdown/coercion/mod.rs
    - darkmatter/lib/src/markdown/coercion/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/deferred.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/json_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/lifecycle_event_binding.rs
    - darkmatter/lib/tests/l1/lifecycle_generated_operands.rs
    - darkmatter/lib/tests/l1/lifecycle_operand_validation.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/schema_nested_template_deferral.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/mutations.yaml
docs_updated_during_phase_2:
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/state-management/side-effects.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/side-effects.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/schema.md
message_to_agent: |-
    Read the log's "## Phase 3", especially D16, D17, D19.
    (1) Claudine's schemas are now recursive and select Darkmatter's typed
    per-verb forms. Payload: 0.66 MB claudine.yaml / 0.70 MB sequence.yaml
    (claudine_schemas_embedded::embedded_trigger_payloads_resolve_within_two_megabytes).
    The resolver still inlines every NON-recursive named type at each
    reference, so do not add references to bundle/expression/loop casually.
    (2) Shared corpus for Wave 7: claudine/lib/tests/fixtures/claudine-schemas/
    (new: depth6-wrong-verb, depth5-guarded-bundle, generated-verb-*,
    optional-nonnullable-null, ensure-file-overloads, variadic-and-or) and the
    four valid container controls
    claudine/lib/tests/fixtures/sequence-schema/containers-depth4-*.md
    (D17: invalid container cells are one in-test edit of a control; the
    manifest's three invalid container fixture files were not created).
    depth4-completion.md for DMLS is not created yet; it is yours.
    (3) D19: the lifecycle parser does not type-check literal operands
    (pr_list: three is S reject, P accept, R reject). Launch still blocks
    via the schema verdict. D16: launch pre-validation judges a malformed
    {{ … }} as written (darkmatter expression::is_malformed_template);
    DMLS's validate_with_options(Defer) and compose's gate still set any
    template problem aside, so check what DMLS reports for
    generated-verb-malformed-template when you assert CLI/DMLS parity.
    (4) Docs for schemas.md were already corrected in Phase 3; Phase 5 still
    owns the full guidance pass (Mermaid passive-validation flow, authoring
    command, examples).
source_files_during_phase_3:
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - claudine/schemas/sequence-types.yaml
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/claudine_schema_parity.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/lib/tests/l1/container_schema_recursion.rs
    - claudine/lib/tests/fixtures/claudine-schemas/depth5-guarded-bundle.md
    - claudine/lib/tests/fixtures/claudine-schemas/depth6-wrong-verb.md
    - claudine/lib/tests/fixtures/claudine-schemas/ensure-file-overloads.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-convertible-text.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-bad-sibling.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-extra-argument.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-deferred-beside-unknown-parameter.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-malformed-template.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-typed-value.md
    - claudine/lib/tests/fixtures/claudine-schemas/generated-verb-wrong-type.md
    - claudine/lib/tests/fixtures/claudine-schemas/optional-nonnullable-null.md
    - claudine/lib/tests/fixtures/claudine-schemas/shell-in-initialize-depth3-then.md
    - claudine/lib/tests/fixtures/claudine-schemas/variadic-and-or.md
    - claudine/lib/tests/fixtures/sequence-schema/level4-invalid.md
    - claudine/lib/tests/fixtures/sequence-schema/containers-depth4-seq-seq.md
    - claudine/lib/tests/fixtures/sequence-schema/containers-depth4-seq-group.md
    - claudine/lib/tests/fixtures/sequence-schema/containers-depth4-group-seq.md
    - claudine/lib/tests/fixtures/sequence-schema/containers-depth4-group-group.md
    - darkmatter/lib/src/markdown/compose/expression/deferred.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/execution-flow.md
    - claudine/schemas/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/timeline.md
    - .claude/skills/darkmatter/schema.md
---

# Implementation Log for 2026-10-30-claudine-recursive-schemas (5 phases)

## Phase 1

Phase 1 is readiness and contracts. It changed no production code, schema,
fixture, or test. Its outputs are this log section and the archived spike
source `spike/deferred_operand_spike.rs`, which is not compiled from this
location.

Method: three read-only or bounded subagents ran in parallel (prerequisite
seams, fixture contracts, deferred-operand spike), and their key claims were
spot-checked by reading the code
(`darkmatter/lib/src/lifecycle/action_shape.rs:381-425`,
`lifecycle/executor.rs:926-985`, `lifecycle/signatures.rs:185-210`,
`schemas/partials/mutations.yaml`, and the `code_link`/`pr_list` overloads in
`functions.yaml`).

### Wave 1: prerequisites verified by delivered APIs and tests

All four prerequisites (`2026-10-21-claudine-schemas`,
`2026-10-27-darkmatter-lifecycle-ownership`, `2026-10-28-schema-enhancements`,
`2026-10-29-recursive-schema-types`) carry `implemented: true`. The working
tree still holds uncommitted review-cycle edits from
`2026-10-29-recursive-schema-types`; its `review-3.md` reports no findings.
These are treated as current.

#### Path map

| Seam | Location |
|---|---|
| Reserved import namespace `@darkmatter:<stem>` | `darkmatter/lib/src/markdown/schemas/embedded.rs:213-221`; resolver `resolve.rs:2161-2172` (never touches disk) |
| Disk `@file` imports | `resolve.rs:2185-2207` (`named_type_file_path`); bundle-only imports `resolve.rs:2208-2213`, `embedded.rs:292` |
| Darkmatter shipped manifest | `SHIPPED`, `embedded.rs:162-203`; pinned by `the_shipped_table_registers_exactly_the_catalog_its_partials_and_policy_schema` (`:427`). It holds `lifecycle`, `md-lifecycle`, and `functions`, but **not** `mutations.yaml` |
| Claudine bundle manifest | `CLAUDINE_SCHEMA_FILES`, `claudine/lib/src/composition/schema/bundle.rs:18-26` (7 files; trigger list `:30`); registered at `composition/schema/mod.rs:561` |
| Hard-coded copies of the Claudine file list | `claudine/lib/tests/l1/claudine_schemas.rs:394`; `darkmatter/cli/tests/l1/claudine_schema_bundle.rs`; `darkmatter/dmls/tests/l1/claudine_schema_discovery.rs:29-35`; `claudine/cli/src/completion/schema_completion/tests.rs:1132`; `darkmatter/lib/tests/l1/shipped_schema_corpus.rs:154-260` |
| Typed signature API | `markdown/compose/expression/catalog/mod.rs`: `expression_function_catalog()` `:221`, `expression_function_descriptors()` `:230`, `function_descriptors(name)` `:236`, `ExpressionFunctionDescriptor.function_type` `:122` |
| Binder | `call_binding/registration.rs:82` `builder_from_types`, `:41` `RegistrationError::FunctionValued`; `call_binding/bind.rs:179` `bind_call`, `:206` `AmbiguousOverload`; runtime registry `functions/mod.rs:270`, `call_function` → `bind_call` `:795` |
| Passive design-time call check | `call_binding/call_check.rs:741,839`; used by DMLS (`dmls/src/diagnostics/call_check.rs:60`) and compose, **not** by lifecycle operands |
| Side-effect descriptors | `darkmatter/lib/src/effects/catalog.rs:25` (`EffectDescriptor`: signature string, no parameter types), `EFFECT_DESCRIPTORS` `:61-220`, `effect_descriptors()` `:229` |
| Template parser | `ExpressionFinder::find_all_plain`, `expression/lexer.rs:328`, with `parser.rs:683` `parse`; the lifecycle operand reader already uses the pair (`action_shape.rs:381`) |
| Deferral | `Defer`/`BindOptions { defer, max_depth }`/`bind_with`, `markdown/coercion/mod.rs:90,94,134`; used only by `schemas/coerce.rs:171-176,345-351` (top-level pending keys) |
| Conversion boundary | `markdown/coercion/{engine,shape,union,error}.rs`; `schemas/coerce.rs:268` `coerce_frontmatter`, `:316` `validation_copy_with_plan` |
| Parser host profiles | `darkmatter/lib/src/lifecycle/parse.rs:59-70` `LifecycleParseOptions`, `:76-105` `UnknownVerbPolicy`, `:110` `MAX_CONDITIONAL_DEPTH = 5`; md host `md_host.rs:90-115` (`IgnoreUnprovided`); Claudine `composition/lifecycle/mod.rs:112-115` (`Reject`), `registry.rs:30` `claudine_registry()` |
| Recursive schema accessors | `EffectiveSchema::definition(&DefKey)` `schemas/mod.rs:1034` (`definitions()` `:1023`); `DefKey` `simplified/definitions.rs:30`; `SchemaError::NonDescendingCycle` `errors.rs:195` |
| DMLS recursive editor | `darkmatter/dmls/src/providers/{definition,frontmatter}.rs`, `overlay/named_types.rs` |
| Stage timings | `darkmatter/cli/src/commands/schema/validate.rs:124,285` (`-vv`); `schemas/timings.rs:132` `ValidationTimings`; DMLS `schema.slow_validation_ms` `router.rs:1044` |

#### Capability evidence (existing tests)

| Capability | Tests |
|---|---|
| Tuples | `darkmatter/lib/tests/l1/schema_grammar_descriptors.rs::tuple_forms_parse`, `::tuples_validate_positions_omission_and_length`; `function_call_binding.rs::tuples_follow_the_declared_structure` |
| `\|` unions | `schema_grammar_descriptors.rs::union_operator_parses_everywhere_a_type_is_written`, `::union_operator_and_yaml_list_are_the_same_definition` |
| Function-catalog parsing | `expression/catalog/tests.rs::the_embedded_catalog_loads_with_unique_display_keys`, `::semantically_duplicate_overloads_fail_catalog_load`; `schema_grammar_descriptors.rs::a_document_schema_that_reaches_a_function_fails_to_load_naming_the_path` |
| Recursive `$ref`/`$defs` | `recursive_schema_types.rs::mutual_recursion_in_one_file_stores_each_definition_once`, `::a_two_file_cycle_validates_depth_six_and_records_both_files` |
| Non-descending cycles | `recursive_schema_types.rs::an_alias_cycle_is_rejected_naming_each_reference`, `::a_cycle_through_a_function_signature_is_rejected` |
| Positioned errors | `recursive_schema_types.rs::a_self_recursive_type_validates_depth_eight_and_positions_a_bad_leaf`; `recursive_schema_reporting.rs::a_depth_seven_failure_is_reported_once_at_its_path_and_position_with_the_selected_arm_description` |
| Event profiles | `claudine/lib/tests/l1/claudine_schemas.rs::initialize_stacks_are_typed_three_levels_deep_without_shell`, `::event_stacks_are_typed_three_levels_deep_and_loose_below`, `::document_cancelled_rejects_forbidden_actions` |
| Closed Claudine vs tolerant md | `darkmatter/lib/tests/l1/lifecycle_schema_parity.rs::the_open_arm_rejects_every_md_verb_and_admits_every_unprovided_verb`, `::shell_never_escapes_initialize_through_a_bundle_or_the_open_arm`; `claudine_schema_parity.rs::each_event_admits_exactly_the_registry_action_types` |
| Strict launch validation | `claudine/lib/tests/l1/claudine_schemas_embedded.rs::invalid_claudine_controls_fail_the_launch_and_are_not_dropped`, `::authored_optional_is_dropped_while_a_claudine_control_fails` |
| Embedded vs disk origin | `claudine_schemas_embedded.rs::disk_import_never_falls_back_to_an_embedded_file`, `::embedded_lifecycle_exports_resolve_without_disk_schemas`; `darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs::a_disk_trigger_never_imports_from_the_bundle` |
| DMLS recursive descent | `darkmatter/dmls/tests/l1/recursive_schema_editor.rs::completion_offers_the_definition_at_recursion_levels_one_two_and_three`, `::completion_and_hover_open_only_the_definitions_on_the_cursor_path` |
| Stage timings | `darkmatter/cli/tests/l1/schema_validate_timings.rs::double_verbose_adds_a_stage_report_to_stderr_only` |

#### The 2 MB size convention (Rule 6)

- **Existing assertion:**
  `darkmatter/lib/tests/l1/recursive_schema_frozen_fixture.rs::recursive_fixture_resolves_to_at_most_two_megabytes`.
  It asserts `serde_json::to_string(&resolved.json_schema).len() <= 2_000_000`
  (compact UTF-8, `$defs` included).
  - Its root is a wiring map of six events plus `loop` onto the frozen
    `lifecycle-recursive.yaml`.
  - Measured: 430,195 bytes.
- **Claudine's payload has no size test yet.** Phase 3 ("Verify Embedded
  Payload") adds one using the same expression.
- **Root to measure:** each embedded Claudine trigger payload (`claudine.yaml`
  and `sequence.yaml`), resolved through `CLAUDINE_SCHEMA_FILES` alone. That
  covers the complete transitive bundle, including nested `task`/`group`
  references, with no user schema and no document baseline.
  - The assertion holds per trigger and for both together.
  - Every exported type is also resolved on its own, for correctness only.

#### Findings that later phases must close

None of these is a blocker for human review; each falls inside scope the plan
already assigns.

1. **No typed side-effect parameter metadata exists.**
   - `EffectDescriptor` has only a signature string.
   - `partials/lifecycle.yaml` types every effect as `side-effect-verb: unknown`.
   - `mutations.yaml` (not embedded, read by no library code) types only `set`.
   - The "shared action parameter definitions" the spec attributes to the
     ownership prerequisite were never delivered. The gap covers 12 generated
     effect verbs (13 signatures; `set` is the lifecycle mapping).
   - Phase 2 "Complete Effect Metadata" creates them (decision D1). This is
     excluded estimate cost.
2. **Side-effect verbs have no event-time argument binding.**
   - `dispatch_side_effect` (`executor.rs:926-985`) evaluates the operands,
     checks only that they are present, and converts with `scalar_string`.
   - Expression-function verbs are bound by the runtime `bind_call`
     (`functions/mod.rs:795`) before their handler runs.
   - Phase 2 binds effect operands with the same binder before dispatch.
3. **`bind_call` cannot defer.** `bind_argument` (`bind.rs:346-352`) calls
   `bind` without `BindOptions`, so a deferred operand fails like `"three"`.
4. **Frontmatter deferral is per top-level key.** `counts: ["1", "{{ x }}",
   "three"]` validates with `pending=["counts"]`, so `"three"` is never
   checked. Operands need child-level deferral, which `bind_with` already does.
5. **`builder_from_types` registrations diverge from the runtime registry.**
   Such a registration misses the registry's source exclusions and numeric
   recoveries (`length(true)` binds, but runtime rejects it as
   `SourceExcluded`). The validator must use the runtime registration, which
   is crate-private today (`functions/mod.rs:684`).
6. **The `Defer` alias is `'static`.** It cannot borrow a recorder or scope.
   The spike had to use an owned `Rc`.
7. **The executor's `is_template_operand` is a substring test.**
   `executor.rs:1624` uses `text.contains("{{")`; the spec forbids a substring
   test for recognizing deferred operands. The spike also observed, without
   proving it, that mixed-text string operands such as `"n={{ fm.count }}"`
   reach `invoke_expression_function` without `evaluate_operand`. Phase 2
   confirms or clears this while wiring the event-time check.
8. **The parser merges overloads** (`signatures.rs:197-211`). It takes
   long-form names from the longest overload and makes the extra parameters
   an optional tail. Effects:
   - `code_link`/`link`: `(file)` | `(target, desc)` becomes `[target, desc?]`,
     so `{file: …}` is P-rejected and `{target: x}` is P-accepted.
   - `pr_list`/`cicd_list`: the stable sort keeps the last one-parameter
     overload's name.
   - `branch_exists_on_remote` becomes `[branch?, remote?]`.
   - See decision D3.
9. **`parse_signature` (`signatures.rs:61`) treats a call as variadic only for
   a bare `...`.** A rest parameter with a fixed prefix would parse wrongly.
   No catalog function has one today (`and`/`or` are rest-only). The generator
   reads `FunctionType`, not display strings, so it is unaffected.
10. **The spike generator cannot run on a real checkout.**
    `claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py`
    reads `darkmatter/docs/schemas/expression-functions.yaml`, which no longer
    exists, and regex-scans `effects/catalog.rs`. It runs only in its L1
    test's synthetic workspace. It is reference material and is left
    untouched.
11. **Stale comment in `merge_signatures`** (`signatures.rs:188-190`). It cites
    `frontmatter(file)`/`frontmatter(file, prop)` as effect overloads, but
    `frontmatter` is an expression function. Phase 2 rewrites
    `merge_signatures` (D3) and must fix or delete the comment there. It was
    not edited here, because Phase 1 changes no source.

### Wave 2: fixture contracts

#### Generated verb inventory

- **Admission** is `is_known_lifecycle_verb` (`darkmatter/lib/src/lifecycle/signatures.rs:231-242`), the union of:
  - the communication channels;
  - `shell`;
  - `LIFECYCLE_CONTROL_VERBS` (`stop`, `skip`, `error`, `proxy`, `retry`, `resume`, `defer`, `break`, `prep`);
  - the effect verbs;
  - **exact canonical** expression-function names.
- **Counts:**
  - 116 expression functions (124 overload descriptors).
  - 13 effect verbs (14 descriptors). `set` is excluded, leaving 12 generated.
  - **128 generated verbs** and 21 hand verbs.
- **Host filtering:** none of the catalog's functions. Every function is
  admitted under Claudine's `Reject` profile. The md host ignores only the
  host-provided verbs (`say`, `speak`, `effect`, `notify`, `retry`, `resume`,
  `proxy`, `prep`, `break`, `defer`), none of which is generated.
- **Aliases:**
  - The about 70 expression-function aliases (`isstring`, `haskey`, …) are
    **not** lifecycle verbs, because admission is exact and case-sensitive.
    Completion lists canonical names only.
  - `speak` is a separate channel documented as an alias of `say`. Both in one
    dictionary is P `LifecycleSayConflict`; S has no exclusivity (S pass, P
    reject).
- **Collisions and winners** (recorded for the parity test):
  - **Lifecycle `set`** (a destination map; no long form, no `no_error`) wins
    over effect `set(key, value)` and `mutations.yaml` `set`. The parser
    routes `set` first (`parse.rs:954-955`, `:840-867`, `:931-937`).
  - **Loop-root `max`** (an iteration control, `LOOP_ITERATION_KEYS`
    `parse.rs:114-122`) wins over catalog `max(a, b)` at the `loop:` root.
    Inside `gate:`, `max` is the function (`claudine_schemas.rs:494-505`).
  - No other name collides. All 116 function names were checked against the
    hand verbs and the `when`/`then`/`else`/`no_error`/loop keys.
- **Overloads** (7 functions plus 1 effect):
  - `code_link` ×2, `link` ×2, `frontmatter` ×2, `validate_schema` ×2
  - `branch_exists_on_remote` ×3
  - `pr_list` ×2 (`query: object` | `count: number(integer; min(1); max(100))`)
  - `cicd_list` ×2
  - effect `ensure_file` ×2
- **Optional parameters (7):** `round(x, fallback?)`, `number(x, fallback?)`,
  `remote_vendor(remote?)`, `ipv4(filter?)`, `ipv6(filter?)`,
  `ping(address, timeout?)`, `ping_under(address, timeout, attempts?)`.
- **Variadic:** `and`, `or` (`...values: unknown | null`), rest-only. A long
  form is P `LifecycleExpressionFunctionKeyValueUnsupported`.
- **Zero-argument generated verbs (6):** `and`, `or`,
  `branch_exists_on_remote`, `ipv4`, `ipv6`, `remote_vendor`. These are
  exactly the generated members of `bare-verb`.
- **Helper types used by parameters:**
  - `agentic-cli` (`has_agentic_cli`) and `ip-address` (`ping`, `ping_under`).
  - `recent-commits` is only a return type, so it needs no inlining.
  - Rule 9's list of three is therefore two for parameters; the drift test
    covers the two that appear.
- **Callback parameters:** 0. `RegistrationError::FunctionValued` rejects
  them, so the exclusion class is empty.

#### Exported types and manifests

- **Darkmatter `partials/lifecycle.yaml`** exports:
  - `interpolation`, `count`
  - `<verb>-short`/`<verb>-long` for every hand verb
  - `set-short`, `side-effect-verb`, `expression-function-verb`
  - `communicate`, `flow`, `shell`, `expression`, `bundle`, `initialize-bundle`
- **Darkmatter `md-lifecycle.yaml`** exports `md-*` categories and the
  unrolled `md-stack`/`md-item-1..3`/`md-conditional-1..3`/`md-stack-2..3`
  plus the `md-initialize-*` twins.
- **Claudine `action.yaml`** exports `interpolation`, `count`, `communicate`,
  `flow`, `shell`, `expression`, `bundle`, `initialize-bundle`,
  `cancelled-stop`, `cancelled-bundle`.
- **Claudine `lifecycle.yaml`** exports:
  - `bare-verb`, `loose-body`
  - `stack`, `item-1..3`, `conditional-1..3`, `stack-2..3`
  - the `initialize-*` twins
  - `loop-action`, `loop-action-entry`, `loop`
  - `cancelled-stack`, `cancelled-item-1..6`, `cancelled-conditional-1..5`,
    `cancelled-stack-2..6`
- **Claudine `sequence-types.yaml`** exports `steps`, `step`, `task`, `group`
  and their `-2`/`-3` twins. Container events use `loose-body` (38 references)
  except `cancelled-stack`.
- **Triggers:** `claudine.yaml` and `sequence.yaml` are `kind: schema-trigger`
  and reference `stack`, `initialize-stack`, `loop`, and `steps`.
- **Public names that survive:** `stack`, `initialize-stack`, `loop`,
  `cancelled-stack`, `steps`, `task`, `group`, and every `action.yaml` export.
  The numbered types and `loose-body` are removed (spec criterion 1, 10).
- **Generated keys today:**
  - Claudine `action.yaml`: 128 keys in each of `expression`, `bundle`,
    `initialize-bundle`, and `cancelled-bundle`.
  - Claudine `lifecycle.yaml`: 127 in `loop` (all but `max`).
  - Darkmatter `partials/lifecycle.yaml`: `expression`, `bundle`, and
    `initialize-bundle`.
  - Every value is `unknown`.
  - Alignment tests:
    - `claudine_schemas.rs::generated_verb_keys_equal_the_darkmatter_catalogs`
      (`:460`). It asserts each value is `{}` plus a description, so it must
      change to compare values.
    - `::hand_verbs_equal_the_parser_inventory`
    - `::bare_verb_enum_equals_what_the_parser_accepts_as_a_bare_item`
    - `::bundle_keys_are_the_union_…`, `::bundle_values_are_the_category_short_forms`
    - `claudine_schema_parity.rs::claudine_action_types_are_darkmatters_neutral_definitions`
- **Manifests for new transitive references:** `SHIPPED` (Darkmatter) and
  `CLAUDINE_SCHEMA_FILES` (Claudine), plus the five hard-coded copies listed
  in the path map.

#### Generator location and invocation

Decided as D2 below:

- **Projection:** a Darkmatter library projection under
  `darkmatter/lib/src/lifecycle/`.
- **Entry point:** `darkmatter/lib/examples/lifecycle_schema_generator.rs`.
  It prints by default; `--write` rewrites the generated types in
  `darkmatter/schemas/partials/lifecycle.yaml`, resolving the path through
  `CARGO_MANIFEST_DIR`.
- **Recipe:** `regen-lifecycle-schema` in `darkmatter/justfile`, wrapping
  `cargo run -p darkmatter --example lifecycle_schema_generator -- --write`.
- **Checkout-relative invocation:** `just -f darkmatter/justfile
  regen-lifecycle-schema` from the root, or `just regen-lifecycle-schema` in
  `darkmatter/`.
- **Drift test:** a new `darkmatter/lib/tests/l1/` test comparing the full
  checked-in structure (short, long, and bundle projections, inlined helper
  bodies) with the projection.
- This mirrors the existing `expression_doc_generator` example,
  `just regen-expr-doc`, and its drift test.

#### Configuration formats and loaders

- **Markdown YAML frontmatter** is the only lifecycle source both S and P
  judge.
  - Loader: Darkmatter frontmatter, which rejects duplicate keys at every
    level (`markdown/frontmatter.rs:1162,1222`).
  - Parser: `parse_lifecycle_config` → `darkmatter::lifecycle::parse_lifecycle_config_with`.
- **External sequence/group sources** are loaded by
  `claudine/lib/src/composition/sequence/data.rs::load_document`.
  - Formats: YAML, JSON, JSON5 with `DuplicateScan`, and JSONL/NDJSON.
  - Their container events reach P only (`sequence/preflight/mod.rs:694,991,1204`).
  - Schema validation of such a source covers its state portion only
    (`sequence/formal.rs:194-230`).
  - This feature does not change that loader or add S for those files
    (spec: "A referenced YAML file is still checked by the task parser during
    preparation"). So no new per-format table is required; existing
    `DuplicateScan` coverage stands.
- **TOML** does not feed lifecycle stacks.

#### Existing fixture verdicts (S = schema, P = parser)

Source: the `assert_ac5` tables in `claudine/lib/tests/l1/claudine_schemas.rs:905-1110`,
`claudine/lib/tests/l1/sequence_schema.rs:69-223`, and
`darkmatter/cli/tests/l1/claudine_schema_bundle.rs`. Fixtures are in
`claudine/lib/tests/fixtures/claudine-schemas/` unless stated.

| Fixture / test | Current S / P | Planned S / P | Reason |
|---|---|---|---|
| depth1, depth3, depth5 | pass / pass | pass / pass | — |
| depth6 | pass (loose body) / reject at level 6 | pass (recursive) / reject at `success[0].then[0]…when` | S now passes by typing, not by looseness |
| depth6-wrong-verb (new) | — | reject (key, path, position) / not asserted for ordering | criterion 4 |
| depth5-guarded-bundle (new) | — | pass / reject (level 6) | criterion 4: a guarded bundle inside level 5 |
| root-dictionary, bundle-in-then, loop-gate, bare-stop | pass / pass | pass / pass | — |
| long-forms | pass / pass | pass / pass | re-verify under typed `set_frontmatter`, `append_line`, `file_exists` |
| shell-in-initialize-list / -then / -root-dictionary | reject / reject | reject / reject | — |
| **shell-in-initialize-below-typed-levels** | **pass** / reject | **reject** / reject | initialize recursion types every depth. Also changes `darkmatter/cli/tests/l1/claudine_schema_bundle.rs::parser_only_defects_pass_schema_validation`, which must drop this row |
| shell-expansion-in-initialize-set | pass / reject | pass / reject | `set` value stays open; the parser and runtime backstop remain |
| shell-in-start, message-shell-bundle-with-when | pass / pass | pass / pass | — |
| when-with-then-and-sibling, misspelled-parameter, misspelled-verb, misspelled-verb-in-then, long-form-in-bundle, long-form-in-root-dictionary | reject / reject | reject / reject | — |
| **generated-verb-arbitrary-value** (`set_frontmatter: [a, b, c]`) | pass / pass | pass / pass, **renamed** `generated-verb-typed-value` | Three strings satisfy the typed `(file, prop, value)` tuple. The fixture's stated contract ("value left to the parser") becomes false, so it is renamed and a new `generated-verb-wrong-type` (reject / reject) carries the typing claim |
| start-number, loop-number | reject / reject | reject / reject | — |
| agent-empty-list, prompt-with-malformed-sequence | reject / not judged | unchanged | — |
| empty-frontmatter, unrelated-ticket, null-identifiers, controls-beside-ticket, linked-prompt | unchanged | unchanged | — |
| `claudine_schemas.rs::assert_typed_levels` (used by the three `*_typed_three_levels_deep*` tests) | bad leaf S-valid at depths 3, 5, 6 | rejected at every depth | unroll removed; the tests are renamed off "three levels" |
| `claudine_schemas.rs::cancelled_rejects_a_conditional_deeper_than_the_parser_accepts` | S reject at level 6 | S pass, P reject | the schema encodes no depth limit (spec "parser-only") |
| `claudine_schemas.rs::grandchild_cancelled_is_left_to_the_task_parser` | S pass | S reject | containers recurse (criterion 10) |
| `sequence_schema.rs` `deeper-than-typed-levels.md` | S pass | S reject at the level-4 `with` and the level-5 `group.execution` | containers recurse |
| `sequence-schema/example-*` | pass | pass | container events use hand verbs only |
| Frozen prompts (`claudine/{lib,cli}/tests/fixtures/frozen_prompts/…`, `darkmatter/{cli,lib}/tests/fixtures/frozen_prompts/…`) | pass | pass | Generated verbs appear only as `ensure_file: "{{ … }}"`, a deferred whole-value operand |
| Spike copies (`_completed/2026-10-20-lifecycle-grammar/spike/fixtures/`) | not read by tests | untouched | historical; `darkmatter/lib/tests/fixtures/recursive_schema_frozen/` stays byte-equal |

#### Landed contracts recorded before any fixture changes

| Contract | P (evidence) | S today | S planned |
|---|---|---|---|
| `then: []` / `else: []` / `{}` / `null` | reject: "an explicit `then` must contain at least one action" (`parse.rs:703-735`; `lifecycle/tests.rs:139`) | `[]` pass | `[]` pass (typed empty list); stays a P check |
| Root bundle at event root | accepted in canonical order; `{}` is an empty stack; `when`/`then`/`else` inside is rejected (`parse.rs:737-765`) | `{when, …}` pass | unchanged (P-only rule) |
| Single-verb dictionary long form | accepted (`parse.rs:957-966`) | long form in a multi-verb root dictionary rejected | unchanged; the single-action long form stays an S alternative |
| Zero-argument `null` / `[]` / bare string | 0 arguments (`action_shape.rs:58-63`); `break`/`prep` reject a non-empty list | `null \| unknown[](max(0))` | the generated zero-argument forms follow P |
| Literal YAML `null`, object, or array as one operand | **reject**: "use a whole-value `{{ null }}`" (`action_shape.rs:381-425`); long-form object values also reject (`parse.rs:1069-1077`) | pass (untyped) | follows the parameter type (D4); P rejection unchanged |
| `speak` + `say` in one dictionary | reject `LifecycleSayConflict` | pass | pass (P-only) |
| Event `null` vs absent | `null` is an empty stack; `cancelled: null` rejects; `loop: null` is skipped | null allowed by the optional-property allowance | unchanged; tested as separate cells |
| `loop.gate: []` / `{}` / `null` | `[]` is an empty gate; `{}` and `null` reject ("write `gate: []`") | no evidence | `[]` pass; `{}` pass (bundle arm); `null` per the optional allowance. P rejection stays P (recorded cells) |
| `no_error: null` | reject "must be a boolean, got null" | likely pass (optional allowance) | recorded cell; unchanged |
| `set: {}` | accept (empty set) | pass | pass |

### Wave 2: deferred-integration spike

Source: `spike/deferred_operand_spike.rs`. It ran once in-crate as
`darkmatter/lib/src/markdown/compose/expression/spike_deferred_operand.rs`
with `cd darkmatter && cargo nextest run -p darkmatter --lib -E
'test(spike_)' --no-capture` and gave **5 passed**. The temporary module and
its `mod` line were removed afterward; `git diff --stat` matched the pre-spike
baseline.

The spike used `pr_list` (`(query: object)` | `(count: number(integer; min(1);
max(100)))`) for the numeric case and `min(a, b)` as a positional list for the
structured case.

| Operand | Runtime `bind_call` | `bind_with` + template-parser defer |
|---|---|---|
| `3` | overload 1, exact | same |
| `"3"` | overload 1, converted | same |
| `"three"` | rejected at `arg[0]` | rejected |
| `"{{ fm.count }}"` | **rejected** (no defer path) | `Deferred { overloads: [0, 1] }`, nothing evaluated |
| `min: [1, "{{ fm.x }}"]` | — | `Deferred { overloads: [0] }` |
| `min: ["three", "{{ fm.x }}"]` | — | rejected at `arg[0] a`; the literal sibling is still checked |
| `number[]` `["1", "{{ fm.x }}", "three"]` | — | only `[2]` fails |

- **Optional but non-nullable** (`ping`'s `timeout?: number`): the compiled
  parameter has `is_optional() = true` and `admits_null() = false`, and the
  runtime rejects `ping(addr, null)` with `NullNotAdmitted`. The schema
  system's optional property accepts `{timeout: null}`, so the projection must
  not use a plain optional property for such a parameter (D5).
- **Overload selection:**
  - `bind_call` prefers exact over converted (`"3"` against `number | string`
    picks the string overload).
  - It reports `AmbiguousOverload { exact }` when two overloads tie.
  - A deferred operand leaves several overloads possible, so a passive
    validator returns "deferred; overloads [i, j] remain" rather than
    ambiguity. Selection happens at event time.
- **Malformed templates:**
  - `"{{ 1 + }}"` is a parse error with a span, reported as a diagnostic.
  - `"{{ fm.count"` is not a template under the existing scanner (an unclosed
    opener is ignored). It is judged as a literal (D6).
- **Passivity:** the spike called only scanners, the parser, the coercion
  engine, the binder, and schema validation. No evaluator, handler, host
  probe, or effect was involved.

**API decision.** The landed APIs express the projection once three
Darkmatter-owned extensions exist. No second evaluator or conversion policy
is needed.

1. `FunctionRegistration::bind_call_with(args, &BindOptions)`, threaded
   through `bind_signature`/`bind_argument`; `bind_call` delegates to it.
2. A passive operand validator over the **runtime** registration, built on
   one template-parser defer predicate:
   `validate_operands(verb, args) -> Bound | Deferred{overloads} | Rejected(paths)`.
   It also needs a public `function_registration(name)` accessor and a
   non-`'static` `Defer<'a>` alias.
3. Binding of side-effect operands through the same binder before
   `dispatch_side_effect`, using the D1 metadata.

Added cost, excluded from the spec estimate: about 9–12 engineer-hours for
the three extensions, plus about 0.5 day for the effect metadata in D1.

### Input Robustness Matrix: concrete columns

The default outcome for every invalid shape is a located rejection through
the public S verdict (CLI, DMLS, launch) or the P/R error with its authored
`LifecyclePath`; nothing runs and nothing is published. The plan's tables
stand with these exceptions and concrete keys:

- **Event columns:**
  - Document: `initialize`, `start`, `blocked`, `success`, `failure`,
    `finalize`, `cancelled`, and `loop.gate`.
  - Container (task, group, nested inline sequence): the same seven events.
  - Exceptions: `cancelled: null` is a P reject; other events treat `null` as
    an empty stack; `loop: null` is skipped.
- **Branch columns:** ordinary `then`/`else`, `initialize` `then`/`else`, and
  `cancelled` `then`/`else`.
  - Explicit empty (`[]`) is S pass and P reject; `else` absent is legal.
  - `cancelled` conditionals use literal `then`/`else` keys after migration.
- **Operand columns:**
  - every parameter of every overload in the 128-verb inventory, on its
    positional, long-form, and bundle projection, enumerated by the
    parameterized matrix test from the projection itself (not a hand list);
  - lifecycle `set`;
  - `no_error`.
- **Additional exceptions to the default:**
  - **Literal YAML `null`/object/array as an operand:** S follows the
    parameter type (D4); P rejects with the `{{ … }}` hint. These are two
    separately recorded cells.
  - **Convertible numeric text** (`"3"`) is not a wrong-type case.
  - **Deferred operands:** S accepts, R binds after evaluation.
  - **Duplicate YAML keys:** rejected by the frontmatter loader before any
    field; tested once per loader. Markdown frontmatter is the only S/P
    loader for this feature.

### Shared fixture manifest

New fixtures go beside the existing ones so that the CLI, DMLS, and launch
tests read the same files. Source positions are taken from the authored file
by the test (line and column of the offending key) rather than duplicated
here.

- `claudine/lib/tests/fixtures/claudine-schemas/`:
  - `depth6-wrong-verb.md`
  - `depth5-guarded-bundle.md`
  - `generated-verb-typed-value.md` (renamed from `generated-verb-arbitrary-value.md`)
  - `generated-verb-wrong-type.md` (`pr_list: three`)
  - `generated-verb-convertible-text.md` (`pr_list: "3"`)
  - `generated-verb-deferred.md` (`pr_list: "{{ fm.count }}"`, plus a deferred
    child in a positional list)
  - `generated-verb-deferred-beside-unknown-parameter.md`
  - `generated-verb-deferred-beside-extra-argument.md`
  - `generated-verb-deferred-beside-bad-sibling.md`
  - `generated-verb-malformed-template.md` (`"{{ 1 + }}"`)
  - `optional-nonnullable-null.md` (`ping: [addr, null]` / long-form `timeout: null`)
  - `ensure-file-overloads.md`
  - `variadic-and-or.md`
  - `depth4-completion.md` (DMLS completion/hover at depth 4 in `then`/`else`)
- `claudine/lib/tests/fixtures/sequence-schema/`:
  - `containers-depth4-{seq-seq,seq-group,group-seq,group-group}.md`
  - `cancelled-forbidden-actions-depth4.md`
  - `container-initialize-shell.md`
  - `container-event-malformed-argument.md`

### Design Decisions

**D1. Typed side-effect metadata lives in `darkmatter/schemas/partials/mutations.yaml`.**
- **Decision:** Phase 2 writes one catalog-grammar `function(...)` entry per
  effect overload (13 signatures, `set` already present) in `mutations.yaml`,
  embeds it, and exposes it through a Darkmatter library accessor. A coverage
  test fails when an `EFFECT_DESCRIPTORS` signature has no typed entry. A
  removed-entry fixture proves that the test detects the gap.
- **Alternatives considered:**
  - Typed parameter fields on `EffectDescriptor` in Rust.
  - Adding effects to `functions.yaml`.
  - A generator-local table, which the spec forbids.
- **Why:**
  - `mutations.yaml` is already Darkmatter-owned and already types `set` in
    the catalog grammar.
  - `FunctionRegistration::builder_from_types` binds catalog-grammar types
    directly (spec reconciliation), so one binder serves validation and
    runtime.
  - `functions.yaml` holds expression functions, and putting effects there
    would make them callable in expressions.
- **Downsides:**
  - A second catalog file must stay in step with `EFFECT_DESCRIPTORS`. The
    coverage test enforces this.
  - It is embedded through the accessor, not the shipped partial imports, so
    on-disk partial copies are unaffected.

**D2. The generator is a Darkmatter example plus a `just` recipe, with a drift test.**
- **Decision:** as given under "Generator location and invocation" above. It
  reads `expression_function_descriptors()`, the D1 accessor, and the
  parser's inventory functions (`all_lifecycle_verbs`, control verbs,
  channels), and fails on a new ambiguous collision.
  - Claudine keeps its per-verb selection keys.
  - Each key references the generated per-verb type through
    `@darkmatter:lifecycle`.
  - `generated_verb_keys_equal_the_darkmatter_catalogs` changes to compare
    referenced values as well as names.
- **Alternatives considered:**
  - A `claudine-gen` subcommand.
  - A Python script.
  - A build-script generation step.
- **Why:**
  - Darkmatter owns shared shapes.
  - The example, recipe, and drift test pattern already exists
    (`expression_doc_generator`, `just regen-expr-doc`).
  - Builds embed checked-in YAML.
  - `claudine/gen` would have to reach into Darkmatter shapes it does not own.
- **Downsides:** adding a catalog function still needs one line per Claudine
  selection type. The parity test names the missing line.

**D3. Generated long forms and argument checks are per overload.** This
replaces the merged long-form names.
- **Decision:**
  - The generator emits one long-form arm per non-variadic overload, using
    that overload's catalog parameter names.
  - Phase 2 changes the parser's long-form binding and arity check to select
    an overload by the same rule as `bind_call`, instead of
    `merge_signatures`.
  - Planned P verdict changes, recorded for review:
    - `code_link`/`link` `{file: x}`: P reject → accept.
    - `code_link`/`link` `{target: x}` without `desc`: P accept → reject
      (`desc` is required in `(target, desc)`).
    - `pr_list`/`cicd_list` long forms with the earlier overload's name:
      P reject → accept.
    - `branch_exists_on_remote` `{remote: x}` alone: P accept → reject (no
      overload takes `remote` without `branch`).
- **Alternatives considered:**
  - Keep the parser's merged names and project them into S. This breaks the
    spec's "one arm per supported overload" and "never combine unrelated
    overloads".
  - Emit per-overload S arms but keep the merged P. S and P would then
    disagree on the same call.
- **Why:**
  - The spec requires "the same overload-selection rules at validation and
    execution". It also explicitly "replaces any older merged-signature
    approximation for generated argument checks".
  - The narrowed calls bind no catalog overload, so the runtime rejects them
    anyway (`bind_call`). They were never executable.
- **Downsides:** a document that relied on a merged-only long form now fails
  at parse rather than at the event. Phase 5's corpus check must confirm no
  shipped example uses one. If a documented form would be lost, the spec's
  open-questions rule applies.

**D4. Literal null and collection operands: S follows the parameter type; P keeps its rejection.**
- **Decision:** S accepts a literal YAML `null` only where the parameter
  admits null, and a literal object or array only where the type is one. The
  parser's existing rejection of literal `null`/object/array operands, which
  carries the `{{ … }}` hint, is unchanged.
- **Alternatives considered:**
  - Make S reject every literal null or collection to mirror P.
  - Relax P to accept literals.
- **Why:**
  - Spec: "accepts explicit null only if its type does".
  - Rule 4 preserves the grammar.
  - The spec states that schema acceptance never proves executability.
  - Launch still blocks, because P runs before launch.
- **Downsides:** the editor accepts `has_agentic_cli: null`, but launch
  rejects it with a hint. Recorded as an S-pass/P-reject cell.

**D5. Optional non-nullable parameters project as omittable but not null-admitting.**
- **Decision:**
  - For a tuple slot or long-form property whose parameter is optional and
    does not admit null, the generator emits a form that rejects an explicit
    null. The value is closed to `null`: either a type that excludes null
    under the schema system's null rule, or the omitted-property form with
    `nullable: false` semantics, whichever the landed grammar supports.
  - Phase 2 proves it with the `ping` `timeout: null` fixture.
- **Alternatives considered:** a plain optional property, which inherits the
  automatic null allowance.
- **Why:** the spike shows that the runtime rejects `ping(addr, null)` while
  the optional property accepts it (Rule 5).
- **Downsides:** if the grammar has no non-nullable optional spelling, Phase 2
  needs a small Darkmatter schema-grammar extension. This is Darkmatter-owned
  and logged as added cost.

**D6. Template recognition uses the existing scanner and parser unchanged.**
- **Decision:**
  - An operand is deferred when `ExpressionFinder::find_all_plain` finds a
    span and `parse` accepts it.
  - A span that fails to parse is a positioned diagnostic.
  - An unclosed `{{` is not a template under the existing scanner, so it is
    judged as a literal.
  - Phase 2 also replaces the executor's substring `is_template_operand`
    with the same predicate.
- **Alternatives considered:** extend the scanner to diagnose an unclosed
  `{{`.
- **Why:**
  - The spec forbids a new template grammar and a substring test.
  - An unclosed opener is ordinary text everywhere else in Darkmatter.
- **Downsides:** `pr_list: "{{ fm.count"` gets a type error rather than a
  syntax error.

**D7. The size check measures the embedded Claudine trigger payloads.**
- **Decision:** as given under "The 2 MB size convention".
- **Alternatives considered:** measure the frozen spike root, or a merged
  document.
- **Why:** the spec requires the Claudine payload in isolation from user
  schemas and the document baseline.
- **Downsides:** none found.

### Estimate impact

The baseline is 2.75–4.25 engineer-days (spec). Added, excluded cost
discovered in Phase 1:
- effect metadata (D1): about 0.5 day;
- deferred-operand integration (three extensions): about 9–12 hours;
- per-overload parser binding (D3): about 2–3 hours.

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gave
  `packages: [claudine, darkmatter, darkmatter-cli, dmls, test-toolkit]`,
  `change_class: package`. The non-empty list comes from the uncommitted
  `2026-10-29-recursive-schema-types` edits in the tree, not from Phase 1, so
  the gates were run rather than skipped.
- **`just lint` (claudine):** passed.
- **`just test` (claudine):** 10,697 run, 10,697 passed (11 slow), 9 skipped,
  in 231 s.
- **Dependent-package tests:** none were run. Phase 1 changed no source, so
  there is no changed symbol whose dependents could be affected.
- **Cross-OS check:** none was run, because Phase 1 has no OS-level risk.
- **Requirement-to-test mapping:** Phase 1 adds no tests by design. Its
  verdict table and fixture manifest above are the mapping that Phases 2–4
  implement. The spike's 5 disposable tests passed once and are archived, not
  registered.

## Phase 2

Phase 2 projects typed action arguments: typed side-effect metadata (Wave 3),
operand validation with deferral (Wave 3), and the maintained schema
generator (Wave 4).

Method: the coordinator landed two small shared seams first, so parallel
subagents editing the `darkmatter` crate would not collide on them, then ran
subagents on disjoint files in two rounds. Round 1 (parallel): typed effect
metadata (A), the deferring call binder and passive operand validator (B),
and child-level template deferral in schema validation (C). Round 2: the
per-overload parser and event-time effect binding (D) beside the Darkmatter
generator (E1), then the Claudine selection files, fixtures and parity tests
(E2). Round 1's three subagents were started as background tasks by mistake;
the coordinator waited for all three before starting round 2, so nothing was
abandoned.

### Shared seams landed by the coordinator

- `darkmatter::markdown::coercion::Defer<'a>` is now
  `dyn Fn(&[PathSegment], &Value) -> bool + 'a`, and `BindOptions<'a>.defer`
  is `Option<&'a Defer<'a>>` (Phase 1 finding 6). `schemas/coerce.rs::convert`
  now borrows its pending set instead of cloning it into an owned closure.
- `darkmatter/lib/src/markdown/compose/expression/deferred.rs`:
  `is_deferred_template(&str)`, `is_deferred_value(&Value)` and
  `defer_templates` (the `BindOptions.defer` predicate). A value is deferred
  when `ExpressionFinder::find_all_plain` finds at least one span and every
  span parses (D6). This is the only template test the phase uses.

### Design Decisions

**D8. Schema-side deferral is child-level, below the top-level key, through the engine's defer predicate.**
- **Decision:** a recognized template nested anywhere below a top-level
  frontmatter key is deferred in every schema validation entry point
  (`validate_with_positions`, phase validation used by Claudine's launch
  preflight, and `validate_with_options` under both pending policies).
  Validator problems at a deferred value's path or below are dropped; a
  problem at an ancestor (a union or tuple failure reported on the parent) is
  dropped only when the conversion engine, binding with the same predicate,
  finds no failure at or below that ancestor. A top-level key that merely
  contains a nested template is no longer wholesale-pending for dropping
  problems; whole top-level template values keep the existing `PendingValue`
  contract unchanged.
- **Alternatives considered:**
  - Add a `| interpolation` arm to every generated operand type, as the
    hand-written `count` type does. Rejected: it is a regular-expression
    test, not the template parser (spec, D6), it misses mixed-text templates,
    and plan Rule 9 names the `BindOptions.defer` predicate.
  - Defer templates at every depth including top-level values. Rejected: it
    would change `PendingPolicy::Report`, which keeps top-level problems.
  - Keep per-top-level-key pending and type operands anyway. Rejected: a
    single template in `success` hides every other problem in that stack in
    DMLS, and the launch path (`validate_for_phase`) has no deferral at all,
    so `pr_list: "{{ fm.count }}"` would fail launch.
- **Why:** the spec requires that a template defers only the affected child
  while literal siblings and the collection's known shape stay checked, with
  no substring test and no evaluation; the engine already walks unions,
  tuples and objects with a per-path predicate.
- **Downsides:** nested templates in ordinary (non-lifecycle) frontmatter are
  deferred too. Compose renders those before its own post-render validation,
  so the visible change is that DMLS and launch no longer report a nested
  template as a type error, which it never was.

**D9. The `md` host profile keeps its tolerant pattern key for generated verbs.**
- **Decision:** `md-lifecycle.yaml`'s `md-expression` pattern key and the
  `md-bundle` open arm stay `unknown`; typed generated verbs are projected
  into Darkmatter's neutral `partials/lifecycle.yaml` and selected by
  Claudine's closed profile.
- **Alternatives considered:** type the md profile's generated verbs too.
- **Why:** plan Rule 2 preserves Darkmatter's tolerant baseline, and the spec
  targets Claudine's closed profile; the md host ignores verbs it does not
  provide.
- **Downsides:** `md` documents get no generated argument typing in S; the
  parser and the event-time binding still check them.

**D10. Claudine selects the typed per-verb forms in Phase 3's recursive families, not on the unrolled Phase 2 families.**
- **Decision:** Darkmatter's neutral `partials/lifecycle.yaml` carries the
  generated typed forms now. Claudine's `action.yaml` and `lifecycle.yaml`
  keep the legacy untyped `side-effect-verb` / `expression-function-verb`
  values for generated verbs until Phase 3 Wave 5 builds the recursive
  families. That wave's plan text already says those families "reference
  supported bare-action enums and generated short/long forms", and adopting
  the forms is the first step of that task. The mechanical edit was applied,
  measured, and reverted. Its patch was kept at
  `/tmp/rb-phase2/claudine-typed-selection.patch` (not durable). It is a plain
  substitution:
  - `expression`: `<verb>: [<verb>-short@darkmatter:lifecycle, <verb>-long@darkmatter:lifecycle]` (`and`/`or` short only)
  - bundles: `<verb>: <verb>-short@darkmatter:lifecycle`
  - `loop` loose keys: short only (see below)
- **Evidence (subagent E2):**

  | Measure (debug build) | Before | After the edit | Ratio |
  |---|---|---|---|
  | `claudine.yaml` resolved payload | 2,825,073 B | 11,227,312 B | 4.0x |
  | `sequence.yaml` resolved payload | 2,402,696 B | 10,074,214 B | 4.2x |
  | First launch validation in a fresh process | 0.6 s | 4.0–7.4 s | 6.5–12x |

  - The resolver (`resolve.rs::expand_imports`) stores only cycle members in
    `$defs` and inlines every other named type at each reference.
  - The three-level unroll copies `expression`/`bundle` into every numbered
    item: one `stack` grew from 363 KB to 1.43 MB.
  - Every L1 test that spawns `claudine` would pay the first-call cost, which
    breaks the 5 s budget.
- **Alternatives considered:**
  - Adopt now and accept slow launches until Phase 3. Rejected: every user
    launch and much of the L1 suite would regress.
  - Make the resolver store every repeated named type in `$defs`. Rejected
    for this phase: it is a broad resolver change with a large blast radius,
    outside this plan.
  - Shrink the generated forms. Rejected: even with no long forms the unroll
    multiplies the bundle.
- **Why:** Phase 3 stores each family once as `$defs` (Rule 11). E2's
  estimate for the typed payload after Phase 3 is 0.65–0.8 MB.
- **Downsides:**
  - Until Phase 3, Claudine documents get typed operands only from the parser
    (P) and event-time binding (R), not from schema validation (S).
  - `claudine_schema_parity.rs::claudine_action_types_are_darkmatters_neutral_definitions`
    now compares hand verbs, key sets and closedness exactly, and ignores
    generated values. `generated_verb_keys_equal_the_darkmatter_catalogs`
    still pins those values as untyped. Phase 3 restores full equality and
    switches that test to compare values with
    `LifecycleSchemaProjection::shipped()` (D2).
  - The Claudine fixture set from the Phase 1 manifest (`generated-verb-*`,
    `optional-nonnullable-null`, `ensure-file-overloads`,
    `variadic-and-or`) needs typed Claudine selection, so it moves with the
    adoption. The same S verdicts are proven now at the Darkmatter level
    (`lifecycle_generated_operands.rs`) against the same generated types.
- **Loop keys (E2's finding):** the parser accepts a long form at the loop
  root only when it is the loop's single loose verb
  (`parse.rs::parse_loop_concerns` → `parse_bundle`). The schema cannot
  express that condition, so loop keys select short forms only, as the hand
  verbs in the loop do.

**D11. Every "set aside for later rendering" filter is path-precise and shares one rule: `schemas::awaits_rendering`.**
- **Decision:** compose's pre-shell gate (`PendingDeferral::defers_problem`),
  `validate_with_options(Defer)` (DMLS), and Claudine's launch pre-validation
  (`is_composition_independent`) set a problem aside only when all of these
  hold:
  - the authored value at the problem's own path (or its nearest existing
    ancestor) holds a template;
  - the schema did not already accept that template as `deferred`
    (`ValidationReport::deferred`, a new field);
  - the problem is not an unknown or missing key on a collection.

  Whole-value `$( … )` keys, provisional keys, caller-owned keys and `Final`
  keep their key-level rules.
- **Alternatives considered:**
  - Keep key-level dropping. Rejected: one template in `success` hid every
    other mistake in that stack.
  - Path-precise with ancestor fallback only, as first built by subagent G.
    Rejected: `{pr_list: "{{ fm.count }}", no_eror: true}` and a closed
    long form with a bogus key beside a deferred field still passed.
  - Drop ancestor fallback entirely. Rejected: a union or length failure on
    a list holding an unmarked template that compose renders would fail
    compose before rendering.
- **Why:**
  - Rendering changes values, never keys.
  - A template the schema already accepted leaves only genuine problems
    around it.
  - All three callers now give one verdict (spec: `md`, DMLS and Claudine
    share one verdict).
- **Downsides:** the change is visible beyond lifecycle frontmatter.
  `validate_with_options(Defer)` now drops a template's own problem at an
  unmarked place, as compose does. Three rows of
  `schema_nested_template_deferral.rs` changed their Defer-policy
  expectation (see "Changed test expectations"). `ValidationReport` gained a
  public field.

**D12. `is_template_operand` keeps a parity clause beside the shared template predicate.**
- **Decision:** an operand is a template when `is_deferred_template` accepts
  it, or when the scanner finds an interpolation literal (`{{{ x }}}`) or a
  literal token in it.
- **Alternatives considered:** apply D6 literally (`is_deferred_template`
  alone).
- **Why:** subagent B ran the old interpolation path. `"{{{ count }}}"`
  rendered to `"{{ count }}"`, and `"{{!data:abc}}"` failed as a malformed
  literal token. Under the predicate alone, both would arrive verbatim: a
  silent runtime change. Unclosed `{{` text behaves the same either way, and
  so does escaped text.
- **Downsides:** one extra clause beside the shared predicate. A malformed
  span (`"{{ 1 + }}"`) is a literal at runtime, but the parser rejects it
  first, so authored YAML cannot reach that case.

**D13. Generated scalar short forms take only the first parameter's scalar types; collection parameters get a `template-operand` arm.**
- **Decision:**
  - A scalar short form takes only the first parameter's scalar types. An
    `unknown` first parameter becomes string, number or boolean.
  - A parameter that takes a collection gets a `template-operand` arm,
    `string(pattern([^\s\S]); deferred)`: no literal matches it, and a
    template is deferred.
  - `null` is a short form only for overloads that accept zero arguments.
- **Alternatives considered:** follow D4 literally for scalar forms, so a
  literal mapping or list is accepted wherever the parameter type allows it.
- **Why:**
  - A mapping value is the long form and a list is positional, so neither is
    ever one argument.
  - Under D4, `pr_list: {state: open}` would pass through an `object` arm and
    turn off long-form typo checking.
  - D4 still holds inside tuple slots and long-form properties
    (`has_agentic_cli: [null]` passes S).
  - Without the `template-operand` arm, `as_json: "{{ fm.items }}"` would fail
    S. The parity test also showed `pr_list("{{ x }}")` matching the wrong
    overload.
- **Downsides:** one more named type in the generated file.

**D14. D5 (optional and non-nullable) needed no grammar addition.**
- **Decision:**
  - Long forms emit one closed arm per subset of an overload's optional
    parameters, with the parameters present in that subset marked
    `required`.
  - An optional tuple slot already rejects `null` unless its type admits it.
  - A required parameter whose type admits `null` would be omittable as a
    plain property, so its arms count their keys with `min-keys(n)`, plus a
    separate `no_error: boolean(required)` arm.
- **Alternatives considered:** a new "optional but not null" grammar
  constraint (D5's fallback).
- **Why:** the existing grammar expresses both cases; the largest expansion
  today is two arms per overload.
- **Downsides:** slightly larger long forms. Added cost: none.

**D15. A side-effect argument-binding failure is an evaluation failure.**
- **Decision:** `dispatch_side_effect` binds the evaluated operands through
  `effect_registration(verb).bind_call` before any engine call. A failure is
  `ActionFailure::Evaluation` carrying `ExpressionError::Call`, the same path
  an expression-function binding failure takes. Dispatch uses the bound
  (converted) values. `ensure_file` picks its form from the bound overload,
  not from the argument count.
- **Alternatives considered:** a dispatch-class failure, which `no_error`
  suppresses.
- **Why:** an argument that does not bind is a defect in what the author
  wrote, which is exactly what evaluation failures report. The existing rule
  is that evaluation failures always halt.
- **Downsides:** `no_error: true` does not suppress a wrong-typed effect
  argument. A test pins this.

### Work completed

**Wave 3: Complete Effect Metadata (subagent A)**
- `darkmatter/schemas/partials/mutations.yaml` types all 14
  `EFFECT_DESCRIPTORS` signatures in the catalog grammar. `ensure_file` has
  two overloads. Categories match the descriptor categories.
- Each type follows runtime behavior. For example, `merge_frontmatter.obj` is
  `object` because the engine rejects anything else; text payloads are
  `string | object | unknown[] | null` because `scalar_string` writes any
  value; path, key and URL parameters are `string`.
- One deliberate narrowing: a `null`, array or object path or key argument
  now fails at binding. Before, it became empty or JSON text.
- The file is embedded through `darkmatter::effects::{effect_function_catalog,
  effect_function_descriptors, effect_registration}`
  (`darkmatter/lib/src/effects/types.rs`), not through `SHIPPED`.
- Coverage tests:
  - `every_effect_descriptor_has_exactly_one_typed_overload`
  - `coverage_check_reports_a_removed_typed_entry` (removes `append_line`)
  - `coverage_check_reports_a_typed_overload_with_no_descriptor`
  - seven further type and registration tests

**Wave 3: Integrate Operand Validation (subagents B, C, F, G and the coordinator)**
- **Binder:**
  - `FunctionRegistration::bind_call_with` and `select_overload_with`
    (`OverloadSelection::{Selected, Deferred}`).
  - `BoundCall::deferred_arguments`.
  - Public `functions::function_registration(name)` returns the RUNTIME
    registration, so source exclusions apply: `length(true)` is rejected.
  - `bind_call` is unchanged.
- **Passive validator:** `darkmatter::lifecycle::{validate_operands,
  validate_operands_with, validate_expression_function_operands,
  OperandVerdict}` (`lifecycle/operands.rs`).
- **Schema-side deferral (D8):**
  - Built by C, then scoped by F to schema-marked places: a `deferred`
    constraint, `x-darkmatter-deferred`, `Shape::Deferrable`, and
    `bind_with_marked`.
  - Both translators agree; `coercion_translator_conformance.rs` was
    extended to prove it.
  - C's first version deferred every nested template. That accepted the
    content-policy row `file_changed_leading_template`
    (`content_policy: ["FileChanged({{HOME}}/x, @fp)"]`), which must be
    rejected. F's marker restored the rejection.
- **Caller filters:** made path-precise and shared through
  `awaits_rendering` (D11).
- **Executor:**
  - `is_template_operand` now uses the shared predicate (D12).
  - Phase 1 finding 7 was confirmed and fixed (subagent D):
    expression-function operands are evaluated with `evaluate_operand`, so
    `length: "n={{ fm.count }}"` with `count: 3` gives 3, not 16. `and`/`or`
    still short-circuit.
- **Parity test** (`lifecycle_generated_operands.rs`, subagent E1):
  - The table covers `code_link`, `link`, `frontmatter`, `validate_schema`,
    `branch_exists_on_remote`, `pr_list`, `cicd_list`, `ensure_file`,
    `and`, `or`, `min` and `ping`, each as list, `null` and scalar
    spellings.
  - The binder's verdict and overload (or set of open overloads) equal the
    generated arms that accept the value.
  - No shipped overload set has two same-arity overloads that both need
    conversion. A fixture (`pick(x: number) | pick(x: boolean)` with `"1"`)
    records the one real disagreement: the binder reports
    `AmbiguousOverload`, while the schema accepts both arms.

**Wave 4: Adapt Schema Generator (subagents D, E1, E2)**
- **D3, per-overload parser:**
  - `merge_signatures`, `Signature` and `parse_signature` are removed, and
    the stale doc comment (finding 11) went with them. Overloads are read
    from the typed catalogs; finding 9 now works by construction.
  - `VerbSignature`/`Overload`/`select_named`. Long-form argument indexes in
    `source_map.rs` follow the selected overload.
- **Generator:**
  - `darkmatter/lib/src/lifecycle/schema_projection.rs`
    (`LifecycleSchemaProjection`).
  - Entry point `darkmatter/lib/examples/lifecycle_schema_generator.rs`
    (prints; `--write` rewrites only the four `# BEGIN GENERATED` regions of
    `darkmatter/schemas/partials/lifecycle.yaml`).
  - Recipe `just regen-lifecycle-schema` in `darkmatter/`
    (`just -f darkmatter/justfile regen-lifecycle-schema` from the root).
  - Output is byte-stable across runs.
- **What it generates:**
  - 128 verbs (116 functions and 12 effects; lifecycle `set` excluded).
  - Helper types `agentic-cli` and `ip-address` are inlined, with a drift
    test against `named_type`.
  - Recorded collisions: `set` (lifecycle `set` wins) and loop-root `max`.
  - A new collision with a hand verb or reserved key fails generation, as
    does missing effect metadata. The callback exclusion class is empty.
  - `side-effect-verb` and `expression-function-verb` are kept only because
    the frozen measurement inputs (`recursive_schema_golden`, the bench
    `claudine-schemas` copy) name them, and those inputs must stay
    byte-equal.
- **Claudine adoption:** moved to Phase 3 (D10).

### Parser (P) verdict changes made deliberately (D3)

| Call | Before | After | Test (`lifecycle::tests::overloads`) |
|---|---|---|---|
| `code_link`/`link` `{file: x}` | reject | accept | `link_long_form_with_file_alone_selects_the_one_parameter_overload` |
| `code_link`/`link` `{target: x}` | accept | reject (`desc` required) | `link_long_form_with_target_requires_desc` |
| `pr_list`/`cicd_list` long form with the other overload's name | reject | accept | `list_long_forms_accept_either_overloads_parameter_name` |
| `branch_exists_on_remote` `{remote: x}` | accept | reject | `branch_exists_on_remote_long_form_needs_branch_with_remote` |

No shipped prompt or fixture used a merged-only long form. The Claudine
lifecycle suites passed unchanged (1,013 targeted tests).

### Changed test expectations (each deliberate)

- `darkmatter/lib/tests/l1/meta_schema_phase3.rs::semantic_type_descriptors_are_authoritative`:
  the `type-definition` and `schema` types now accept `deferred`, which every
  type accepts.
- `darkmatter/lib/tests/l1/schema_nested_template_deferral.rs`, three rows
  (`a_marked_union_arm_defers_where_an_unmarked_union_would_fail`,
  `a_marked_place_inside_a_recursive_type_defers_at_every_depth`,
  `malformed_template_is_judged_as_written`):
  - Under `validate_with_options(Defer)`, a problem on an unmarked template's
    own path is now set aside, as compose's gate does (D11).
  - Plain `validate`, `Report` and launch still judge it.
- `claudine/lib/tests/l1/claudine_schema_parity.rs::claudine_action_types_are_darkmatters_neutral_definitions`:
  generated values are ignored until Phase 3 (D10).
- The `lifecycle/actions/tests.rs` merged-signature tests were replaced by
  per-overload tests (D3).
- `claudine/lib/src/composition/lifecycle/tests/mixed_span.rs`: moved to the
  new signature API; the values are unchanged.

### Requirement-to-test mapping

| Requirement (plan Phase 2) | Tests |
|---|---|
| Effect metadata coverage + removed-entry detection | `effects::types::tests::{every_effect_descriptor_has_exactly_one_typed_overload, coverage_check_reports_a_removed_typed_entry, coverage_check_reports_a_typed_overload_with_no_descriptor}` |
| `3` / `"3"` pass, `"three"` fails by path and position | `lifecycle_operand_validation::pr_list_*`; `schema_nested_template_deferral::{numeric_text_operand_converts, non_numeric_text_operand_fails, a_failing_operand_is_located_on_its_line}`; `lifecycle_generated_operands` |
| Deferred scalar/structured pass S; literal siblings, arity, unknown keys still judged | `schema_nested_template_deferral::*` (tuple, structured, misspelled-verb rows); `lifecycle_operand_validation::{min_with_a_deferred_operand_checks_its_literal_sibling, arity_is_never_deferred, array_operand_defers_only_its_template_children, object_operand_defers_only_its_template_properties}`; Claudine `pre_validate_surfaces_{literal_sibling_of_nested_template, literal_sibling_of_deferred_operand, unknown_key_beside_a_template, a_union_failure_beside_an_accepted_template}`, `pre_validate_accepts_a_marked_template_operand` |
| Incompatible evaluated value fails R at the event | `lifecycle_event_binding.rs` (`merge_frontmatter: [doc, "{{ 5 }}"]` fails before dispatch, document unchanged, names `obj`; `ensure_file: [path, 3]` writes `3`; `no_error` does not suppress) |
| Passivity (no evaluation, probe, effect during S) | `lifecycle_operand_validation::validation_consults_only_the_deferral_predicate` (counting `Defer` over `ping`); `schema_nested_template_deferral::template_is_never_evaluated` |
| Optional non-nullable vs null-admitting | `lifecycle_operand_validation::optional_non_nullable_parameter_rejects_null_and_admits_omission`; `lifecycle_generated_operands` (`ping` positional and long `timeout: null` fail; required-but-nullable parameter) |
| Overload parity binder ↔ generated schema | `lifecycle_generated_operands` parity table + coverage guard + ambiguity fixture |
| Generator: full-structure drift, determinism, missing metadata, collisions | `lifecycle_schema_generation.rs` (11 tests) + `schema_projection` unit tests |
| `ensure_file` overloads, `and`/`or`, legit `unknown`, invalid/missing/extra/unknown parameters | `lifecycle_generated_operands`, `lifecycle::tests::overloads` |
| Template predicate, not a substring test | `lifecycle_operand_validation::{template_operand_uses_the_shared_template_predicate, malformed_and_unclosed_templates_are_judged_as_literals, mixed_text_template_is_deferred}`; `deferred.rs` doctest |
| Finding 7 | `executor/tests.rs` + `lifecycle_event_binding.rs` (mixed-text operand, typed whole-value operand, `and`/`or` short-circuit) |
| Content-policy and compose "defers nothing" contracts kept | `content_policy_editor_schema` (unchanged), `dmls::content_policy_baseline` (unchanged), `schema_nested_template_deferral::template_at_an_unmarked_*` |

Fidelity: every test is an in-process L1 test against the crate's public
result: a `ValidationReport` with positions, the launch pre-validation
verdict, `OperandVerdict`, and lifecycle playback against a sandboxed
effect engine. No CLI spawn was needed, because no CLI wiring changed.

**Input robustness matrix:** the per-format CLI wiring rows and the full
Claudine fixture walk belong to Phases 3–4 (D10). The parameter-level field
shapes are walked once per validator:
- absent and missing required;
- explicit `null` (optional non-nullable vs null-admitting);
- wrong type: the whole value, one element, every element;
- empty collections.

The validators covered are the binder (`lifecycle_operand_validation`) and
the generated schema (`lifecycle_generated_operands`). The code-smell grep
over touched readers found no `#[serde(default)]`, `unwrap_or_default` or
`.ok()` on a load-bearing parse. `decode_literal_tokens(..).ok()` predates
this phase and falls back to the raw value, which is then judged.

### Known gaps carried forward (not blockers)

- **Category `verb: null` (subagent E1):**
  - Each verb key in a category type is an optional property, so `null`
    passes S; the parser rejects it.
  - This already held for hand verbs. The generated short types themselves
    reject `null`.
  - Recorded as an S-pass / P-reject cell, like D4. Closing it would need
    "optional but not null" on category keys.
- **Payload budget:** the Claudine payload was already 2.8 MB before this
  phase, because of the unroll. Phase 3 must measure it first (D7, D10).

### Gates

- **Scope calculator:**
  - Command: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`.
  - Result: `packages: [claudine, darkmatter, test-toolkit, claudine-cli, dmls]`,
    `change_class: package`.
  - The gates were therefore required, and they were run.
- **`claudine`:**
  - `just test`: 10,703 run, 10,703 passed (27 slow), 9 skipped.
  - `just lint`: passed. The only warning is the existing `ld: __eh_frame
    section too large` linker note.
  - `just check-tier-coverage claudine` (root): 0 stranded.
- **`darkmatter`** (changed directly):
  - `just test`: 12,087 run, 12,087 passed, 10 skipped. This includes `dmls`
    and `darkmatter-cli`.
  - `just lint`: passed.
  - `just doctest`: 230 passed.
  - `just check-tier-coverage darkmatter`: 0 stranded.
  - An earlier run had one failure,
    `dmls::content_policy_baseline::every_shared_declaration_publishes_the_baseline_verdict_at_its_anchor`.
    It exposed the DMLS/compose inconsistency that D11 fixed; the test was
    not edited.
- **Dependents:** `cargo check` of every other package that depends on
  `darkmatter` passed:
  - `messenger`, `messenger-cli`, `research`, `biscuit-icon-cli`,
    `biscuit-speaks-cli`, `playa-cli`, `sniff-cli`.

  Then the schema-exercising subsets ran: `messenger` `research_corpus`
  (1 passed) and `research` schema/frontmatter/validate tests (83 passed).
- **Cross-OS:** not run. The phase changed no path, process or OS-specific
  code; the work is schema, binder and parser logic.
- **Process note:** no test failure was load-sensitive. Subagents ran only
  scoped nextest filters; the full gates ran once each, with no subagent
  running.
- **Edits after the gates:** two doc edits landed after the full gates ran:
  `darkmatter/docs/inline/schema-validation.md` (the D11 refinement) and
  `claudine/docs/topics/state-management/side-effects.md` (the typed effect
  catalog). The only test that reads the first, `schema_phase_validation`,
  was rerun: 19 passed. `just doctest` was rerun: 230 passed. A stray
  `cargo test -p darkmatter --doc` was also typed during that check; its
  output was discarded, and `just doctest` is the recorded result.

## Phase 3

Phase 3 replaces the unroll with recursive families (Wave 5) and proves the
embedded payload and the grammar (Wave 6). The coordinator did the work
directly, without subagents: every edit touched the same five schema files
and the same three test files, so parallel agents would have collided.

### Step 1: typed generated verbs, then measure (D10, D7)

- The D10 substitution was applied first, with a one-off script over
  `claudine/schemas/action.yaml` and the `loop` keys of `lifecycle.yaml`:
  - `expression.<verb>: [<verb>-short@darkmatter:lifecycle, <verb>-long@darkmatter:lifecycle]`,
    short only for `and`/`or` (no long form);
  - every bundle (`bundle`, `initialize-bundle`, `cancelled-bundle`) and
    every loose `loop` key: `<verb>-short@darkmatter:lifecycle`.
- The recursive families followed (below). The payload was measured before
  any test was changed, with the new size test
  (`claudine_schemas_embedded::embedded_trigger_payloads_resolve_within_two_megabytes`,
  the D7 convention: compact `serde_json` of the effective schema, which
  also holds the built-in baseline):

  | Payload | Phase 2 (untyped, unrolled) | Typed, unrolled (D10 probe) | Phase 3 (typed, recursive) |
  |---|---|---|---|
  | `claudine.yaml` (`start: []`) | 2,825,073 B | 11,227,312 B | **660,284 B** |
  | `sequence.yaml` (`sequence: []`) | 2,402,696 B | 10,074,214 B | **704,904 B** |
  | both triggers | — | — | **761,010 B** |

  The 2 MB limit holds with about 2.6x headroom. The Phase 2 estimate was
  0.65–0.8 MB.

### Wave 5: recursive families

- `claudine/schemas/lifecycle.yaml`:
  - `stack`/`item`/`conditional` (ordinary events and `loop.gate`);
  - `initialize-stack`/`initialize-item`/`initialize-conditional` (no
    `shell` arm at any depth);
  - `cancelled-stack`/`cancelled-item`/`cancelled-conditional`, with literal
    `when`/`then`/`else` keys (the pattern key and its `min-keys: 2` are
    gone).
  - Removed: `loose-body`, `item-1..3`, `conditional-1..3`, `stack-2..3`,
    the `initialize-*-N` twins, `cancelled-item-1..6`,
    `cancelled-conditional-1..5`, `cancelled-stack-2..6`.
  - Every cycle crosses a list item (`item[]`) or a property
    (`then`/`else`), so no `NonDescendingCycle`.
- `claudine/schemas/sequence-types.yaml`: `steps`/`step`/`task`/`group`
  only. A task's `sequence:` is `steps` and a group's `tasks:` is `task[]`,
  so both recurse. Every container event is typed with its family, and
  `loop` is the shared `loop` type at every depth. Scalar steps and the
  string file-reference arms of `sequence:` and `group:` are kept.
- `claudine/schemas/action.yaml`: the header no longer claims untyped
  generated values. `claudine/schemas/README.md` updated.
- No new schema file, so `CLAUDINE_SCHEMA_FILES` and its hard-coded copies
  are unchanged; disk origins and the no-fallback rule are unchanged.

### Wave 6: embedding and grammar evidence

- **Embedded payload** (`claudine_schemas_embedded.rs`):
  - the size test above, which also asserts non-empty local `$defs`, each
    family present by `DefKey` (`DefNamespace::Embedded("claudine/<file>")`),
    and 12 retired names absent by `DefKey` (never by `$defs` key, Rule 11);
  - export probes rewritten for the recursive names, now including
    `cancelled-stack`/`-item`/`-conditional` and `step`;
  - new `broken_disk_recursive_family_never_falls_back_to_an_embedded_file`
    (a disk `lifecycle.yaml` whose `stack` names an undeclared `item` is a
    load failure; the embedded `item` never supplies it).
- **Grammar** (`claudine_schemas.rs`):
  - `assert_typed_at_every_depth` (replaces `assert_typed_levels`) for
    `start`, `initialize`, and `loop.gate`: depths 1, 3, 5, 6, 8, each in
    `then` and in `else`; a bad leaf and a malformed long form rejected at
    every depth; at depths 1 and 4 a conditional with a sibling verb, an
    `else` without `then`, a sibling `no_error` rejected, and a guarded
    bundle and a bare verb accepted.
  - `ac5_generated_operands`: the Phase 1 manifest fixtures (below), S and P.
  - `depth6_wrong_verb_is_positioned_at_the_verb`: the problem carries the
    verb's line and a column.
- **Containers** (new `lib/tests/l1/container_schema_recursion.rs`, 14
  tests): see D17.
- **DMLS** (`darkmatter/dmls`): `nested_span_tests::lifecycle_inventory_matches_the_authored_claudine_schema`
  now walks the three named conditionals instead of `conditional-1..3`;
  `sequence_tests::nested_shape_consumes_array_index_against_item_type`
  descends `initialize` through the recursive definition table (a
  `Definitions` lookup over `ResolvedSchema::definitions`) instead of
  `NoDefinitions`, and its stale `item-1[]` comment was corrected.

### Design Decisions

**D16. Launch pre-validation judges a malformed template as written.**
- **Decision:** `is_composition_independent` (Claudine) no longer sets a
  problem aside when the authored value holds a `{{ … }}` span that fails to
  parse. Darkmatter gained `expression::is_malformed_template` for that
  test, beside `is_deferred_template`.
- **Alternatives considered:**
  - Keep setting it aside, because the parser rejects the operand anyway.
    The passive-parity test then fails (`passive_parity_generated_verb_malformed_template`):
    plain validation (DMLS, `md schema validate`) rejects while the launch
    schema verdict accepts.
  - Change Darkmatter's shared rule for compose and `validate_with_options(Defer)`
    too. Compose renders frontmatter and reports a malformed template itself,
    with its span, so its gate does not need it; widening the change was not
    needed for this phase.
  - Implement the span scan in Claudine. Rejected: template recognition is
    Darkmatter-owned (plan Rule 2).
- **Why:** `deferred.rs` already states that a malformed template "is never
  deferred and is judged as written", AC8 says malformed templates fail
  passively, and no rendering can turn one into a valid value.
- **Downsides:** a malformed template in a typed launch-checked field now
  reports a schema mismatch before composition would have reported the
  parse error.

**D17. Criterion 10's container cells are generated from four shipped controls.**
- **Decision:** a container path X→Y alternates kinds from the first level
  (seq→group is sequence, group, sequence, group) to container depth 4, with
  all seven events on the innermost container. The controls are shipped as
  `tests/fixtures/sequence-schema/containers-depth4-{seq-seq,seq-group,group-seq,group-group}.md`,
  and each test asserts the generator reproduces its control byte for byte.
  Every other cell is one edit to a control, made in the test.
- **Alternatives considered:** one fixture file per cell (the Phase 1
  manifest also named `cancelled-forbidden-actions-depth4.md`,
  `container-initialize-shell.md`, `container-event-malformed-argument.md`).
  About 140 near-identical files, and the three named ones would cover one
  path each.
- **Why:** one edit per cell from a real control is what the robustness rules
  ask for, and the generator covers all four paths. Phase 4's CLI and DMLS
  tests can read the four controls and the `claudine-schemas/` invalid
  fixtures.
- **Downsides:** the three named invalid fixtures do not exist; a Phase 4
  test that wants an invalid container document on disk writes it from the
  same edit.

**D18. One `bare-verb` enum serves the ordinary and initialization families.**
- **Decision:** keep the shared `bare-verb` (the spec sketch named an
  `initialize-bare-verb`).
- **Alternatives considered:** a separate enum per family.
- **Why:** no bare verb is a shell, so the two profiles' bare sets are equal;
  `bare_verb_enum_equals_what_the_parser_accepts_as_a_bare_item` keeps the
  enum equal to the parser's set. `skip` (initialize-only at parse time) was
  already S-pass/P-reject in ordinary events before this phase.
- **Downsides:** none new.

**D19. A wrong-typed literal operand is S-reject, P-accept, R-reject.**
- **Decision:** record the parser's actual verdict; do not add literal type
  checks to the parser.
- **Alternatives considered:** make the parser bind literal operands (the
  Phase 1 table planned P reject for `generated-verb-wrong-type`).
- **Why:** the parser checks arity, long-form names and literal-shape rules;
  types are bound at the event (D15). Launch still blocks, because the S
  launch verdict rejects the literal, and the event-time binder rejects an
  evaluated value. Adding a second binding site to the parser would
  duplicate D15.
- **Downsides:** a direct parser caller (`parse_lifecycle_config`) accepts
  `pr_list: three`; `ac5_generated_operands` pins that.

**D20. The `loop` type stays a non-recursive named type.**
- **Decision:** `loop` is inlined at its three references (document, task,
  group); it is not made a definition.
- **Alternatives considered:** force it into `$defs`.
- **Why:** the resolver stores only cycle members; `loop` is not one, and the
  measured payload is well inside the limit.
- **Downsides:** each new reference to `loop` costs its full typed size
  (about 128 typed short forms).

### Fixture verdict changes (each deliberate)

| Fixture / test | Before (S / P) | After (S / P) | Reason |
|---|---|---|---|
| `shell-in-initialize-below-typed-levels` → renamed `shell-in-initialize-depth3-then` | pass / reject | **reject** at `/initialize/0/then/0/then/0/then/0` / reject | `initialize-*` recurses; the old name described the removed bound |
| `generated-verb-arbitrary-value` → renamed `generated-verb-typed-value` | pass / pass | pass / pass | three strings satisfy `(file, prop, value)`; the old name's claim ("value left to the parser") is false |
| `depth6` | pass (loose body) / reject | pass (typed) / reject | the schema has no depth limit |
| `sequence-schema/deeper-than-typed-levels` → renamed `level4-invalid` | pass | **reject** at the level-4 `with` and the level-5 `group` (`execution`) | containers recurse |
| `cancelled_rejects_a_conditional_deeper_than_the_parser_accepts` → `cancelled_conditional_depth_is_the_parsers_alone_to_limit` | level-6 conditional S reject | S pass, P reject; a level-6 `retry` is still S reject | the schema encodes no depth limit |
| `grandchild_cancelled_is_left_to_the_task_parser` → `grandchild_cancelled_is_typed` | S pass | S reject at `/sequence/0/sequence/0/sequence/0/cancelled/0` | containers recurse |
| `task_when_and_container_loop_schema_shapes` `deeper-loop-max-zero` | S pass | S reject; new row `deeper-group-loop-gate` (S reject) | `loop` is typed at every depth |
| `cancelled` `{ when, else }` without `then` | S pass (`min-keys: 2`) | S reject | literal `then` is required (spec) |
| `assert_typed_levels` users | bad leaf S-valid at depth ≥ 3 | rejected at every depth | unroll removed; tests renamed `*_at_every_depth` |
| `darkmatter/cli` `parser_only_defects_pass_schema_validation` | rows `depth6`, `shell-in-initialize-below-typed-levels`, `generated-verb-arbitrary-value` | rows `depth6`, `depth5-guarded-bundle`, `shell-expansion-in-initialize-set` | the dropped rows are no longer parser-only defects |

New fixtures (`claudine/lib/tests/fixtures/claudine-schemas/`), with S / P:

| Fixture | S | P |
|---|---|---|
| `depth6-wrong-verb` | reject at the sixth-level item, naming `sya`, positioned | reject (on `success[0].then[0]…`; ordering not asserted) |
| `depth5-guarded-bundle` | pass | reject (level 6) |
| `generated-verb-wrong-type` (`pr_list: three`) | reject at `/start/0/pr_list` | accept (D19) |
| `generated-verb-convertible-text` (`"3"`, `["1", 2]`) | pass | accept |
| `generated-verb-deferred` (scalar, positional, long form) | pass | accept |
| `generated-verb-deferred-beside-unknown-parameter` | reject at `/start/0/pr_list` naming `limit` | reject |
| `generated-verb-deferred-beside-extra-argument` | reject at `/start/0/min/2` | reject |
| `generated-verb-deferred-beside-bad-sibling` | reject at `/start/0/min/0` | accept (D19) |
| `generated-verb-malformed-template` | reject at `/start/0/pr_list` (and at launch, D16) | reject |
| `optional-nonnullable-null` | reject at `/start/0/ping/1` and `/start/1/ping/timeout` | reject (`{{ null }}` hint) |
| `ensure-file-overloads` (both overloads, scalar, positional, long) | pass | accept |
| `variadic-and-or` | pass | accept |

`depth4-completion.md` (manifest) belongs to Phase 4's DMLS task and was not
created here.

### Other test changes

- `claudine_schema_parity::claudine_action_types_are_darkmatters_neutral_definitions`:
  full equality restored; `without_generated_values` removed.
- `claudine_schemas::generated_verb_keys_equal_the_darkmatter_catalogs`:
  every generated value is typed (not `{}`), and each authored key in
  `expression`, the three bundles, and `loop` names exactly the projected
  per-verb types of `LifecycleSchemaProjection::shipped()`.
- `bundle_values_are_the_category_short_forms` and
  `action_objects_are_closed_and_data_maps_stay_open`: a generated verb has
  one long-form arm per overload and optional subset, and its object-typed
  parameters are catalog data; hand verbs keep the old assertions.
- `every_file_loads_and_every_export_resolves`: the recursive export names.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| Recursive families, no numbered/loose type (AC1) | `embedded_trigger_payloads_resolve_within_two_megabytes` (`DefKey` presence/absence); `every_file_loads_and_every_export_resolves`; `embedded_lifecycle_exports_resolve_without_disk_schemas` |
| Embedded resolution, every export, three event families, loop gate (AC1) | `embedded_{action,lifecycle,linking,sequence_types}_exports_resolve_without_disk_schemas`; `embedded_payloads_apply_without_disk_schemas` |
| 2 MB limit, nested task/group in the payload (AC1) | `embedded_trigger_payloads_resolve_within_two_megabytes` |
| No embedded fallback for missing/broken disk imports | `disk_import_never_falls_back_to_an_embedded_file`, `broken_disk_recursive_family_never_falls_back_to_an_embedded_file` |
| Typed generated verbs selected by Claudine (AC2) | `generated_verb_keys_equal_the_darkmatter_catalogs`, `claudine_action_types_are_darkmatters_neutral_definitions`, `ac5_generated_operands` |
| Restrictions at depths 1/3/5, both branches, bundles, bare verbs, gate, long forms, typos, mixtures, init shell in an unselected branch (AC3) | `event_stacks_are_typed_at_every_depth`, `initialize_stacks_exclude_shell_at_every_depth`, `loop_gate_is_typed_at_every_depth`, `ac5_*` |
| `set` shell expansion stays a parser/runtime check | `ac5_shell_placement` (`shell-expansion-in-initialize-set`: S pass, P reject); runtime backstop tests unchanged |
| Depth fixtures (AC4) | `ac5_depths_and_stack_shapes` (`depth5`, `depth6`, `depth6-wrong-verb`, `depth5-guarded-bundle`), `depth6_wrong_verb_is_positioned_at_the_verb`, `cancelled_conditional_depth_is_the_parsers_alone_to_limit` |
| Launch/DMLS schema parity for every fixture | `passive_parity_*` (one per AC5 fixture, 44 total), `passive_parity_tests_cover_every_ac5_fixture` |
| Recursive containers (AC10) | `container_schema_recursion::*` (four paths × every-event / cancelled / profiles-and-depth-counts, plus scalar steps and references), `every_container_level_is_typed`, `grandchild_cancelled_is_typed`, `task_when_and_container_loop_schema_shapes`, the `*_cancelled_rejects_forbidden_actions*` suites |
| Malformed template fails passively at launch too (D16) | `passive_parity_generated_verb_malformed_template`; `is_malformed_template` doctest |

Fidelity: in-process L1 against public results (`ValidationReport` with
paths and positions, Claudine's `pre_validate_schema`, the task parser's
`resolve_sequence_plan`). No CLI wiring changed in this phase; Phase 4 owns
the `md schema validate`, DMLS, and launch-process evidence.

**Input robustness matrix:** no reader, loader, or configuration parser
changed. The schema is the field validator for event stacks, so its field
shapes were walked once through it (`*_at_every_depth`, AC5 tables,
container cells): absent events (controls omit them), explicit `[]`, wrong
whole type (`start-number`, `loop-number`), one invalid element, and every
invalid element at depth. The duplicate-key and trailing-content rows belong
to the frontmatter loader, which is unchanged and already tested. Code-smell
grep over touched readers: none touched.

### Docs and skills updated in this phase

The behavior change made several current pages false, so they were corrected
now rather than left for Phase 5 (Phase 5 still owns the full guidance pass,
including the passive-validation Mermaid flow and the authoring command):
`claudine/docs/topics/schemas.md` (depth, containers, parser-only list,
generated verbs), `claudine/docs/topics/execution-flow.md` (§9),
`claudine/schemas/README.md`, `.claude/skills/claudine/{SKILL.md,cli-commands.md,timeline.md}`,
`.claude/skills/darkmatter/schema.md`.

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `packages: [claudine, darkmatter, darkmatter-cli, dmls, test-toolkit, claudine-cli]`,
  `change_class: package` (the tree also holds uncommitted Phase 2 work), so
  every gate ran.
- **`claudine`:** `just test`: 10,733 run, 10,733 passed (2 slow), 9 skipped,
  165 s. `just lint`: passed (only the existing `ld: __eh_frame` linker
  note). `just check-tier-coverage claudine`: 0 stranded.
- **Darkmatter (changed directly: `deferred.rs`, a DMLS unit test, a
  darkmatter-cli test):** `just lint` passed; `just doctest` 231 passed
  (includes the new `is_malformed_template` doctest). Targeted nextest over
  every Darkmatter test that reads Claudine's schemas or fixtures
  (`dmls`: `sequence_tests`, `nested_span`, `host_globals`,
  `lifecycle_event_views`, `claudine_schema_discovery`; `darkmatter-cli`:
  `claudine_schema_bundle`, `schema_validate`; `darkmatter`:
  `base_schema_end_to_end`, `shipped_schema_corpus`, `trigger_spelling_guard`,
  `recursive_schema_frozen_fixture`, `deferred`,
  `schema_nested_template_deferral`): 307 run, all passed after the two DMLS
  fixes. The full Darkmatter suite was not run: the only Darkmatter source
  change is an additive public function.
- **Load-sensitive failures:** none.
- **Cross-OS:** not run. The change is schema YAML, an additive string
  predicate, and in-process tests over temp-dir fixtures; no path, process,
  or OS-specific code changed.
- **Frozen fixtures:** `darkmatter/lib/tests/fixtures/recursive_schema_golden/`
  and the bench copy were not touched (`recursive_schema_frozen_fixture`
  passed).
