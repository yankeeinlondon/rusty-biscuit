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
human_review: false
message_to_agent: |-
    Phase 1 changed no code; read the log's "## Phase 1" first. It records the
    path map, the fixture verdict table (S and P recorded separately), and
    decisions D1-D7, which are settled.
    Key facts for Phase 2:
    (1) No typed side-effect metadata exists anywhere. D1 creates it as
    catalog-grammar `function(...)` entries in
    darkmatter/schemas/partials/mutations.yaml, read through a Darkmatter
    accessor, with a coverage test against EFFECT_DESCRIPTORS.
    (2) `bind_call` has no defer path. Add `bind_call_with(args,
    &BindOptions)` and a passive operand validator over the RUNTIME
    registration (builder_from_types misses source exclusions and
    recoveries). The `Defer` alias is 'static; make it `Defer<'a>`.
    (3) `dispatch_side_effect` (darkmatter/lib/src/lifecycle/executor.rs
    ~926) binds no types. Bind effect operands before dispatch, and replace
    the substring `is_template_operand` (~1624) with the
    ExpressionFinder+parse predicate (D6).
    (4) D3: argument checks and long forms become per overload, replacing
    merge_signatures. The planned P verdict changes for
    code_link/link/pr_list/cicd_list/branch_exists_on_remote are listed in D3.
    Fix the stale merge_signatures doc comment there.
    (5) The parser rejects literal YAML null/object/array operands (D4 keeps
    that; S follows the parameter type).
    The archived spike is spike/deferred_operand_spike.rs; its header gives
    the in-crate path and command to rerun it if needed.
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
