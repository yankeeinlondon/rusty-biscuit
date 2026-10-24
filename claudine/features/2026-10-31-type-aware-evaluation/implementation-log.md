---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-31-type-aware-evaluation/spec.md"
plan: "claudine/features/2026-10-31-type-aware-evaluation/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
human_review: false
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/raw.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/lifecycle/error.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/registry.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/cli/tests/l1/compose_lifecycle.rs
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/expression_scopes.rs
    - claudine/lib/src/composition/error/lifecycle_authoring.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/registry.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/call_check.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/call-check.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
message_to_agent: |-
    Phase 2 is done; read "## Phase 2" in this log, especially its decisions.
    The projection is DeclaredTypes::for_schema(Option<&EffectiveSchema>) (document,
    triggers, RawDeclarations of the merged JSON, layered_definitions); every consumer
    now builds it. Shape::union is the single normalization (canonical order prints
    "number | string"). Call/operator types come from call_check::expression_type,
    whose ternary and `||` operands already go through TypeScope::assume; Phase 3
    extends facts there, not a second path. Lifecycle surfaces are call-checked by
    darkmatter::lifecycle::reject_incompatible_calls (unguarded on purpose, P2-D5).
    Host globals are typed via DeclaredTypes::with_global_catalog and
    EventRegistry::register_global_catalog (Claudine registers claudine_globals()).
    Two guards list DeclaredTypes/TypeScope::unguarded sites with reasons: darkmatter
    lib/tests/l1/type_scope_ownership_guard.rs and claudine lib/tests/l1/type_scope_ownership_guard.rs;
    new sites need entries in both where scanned. Keep SiteScope's schema boxed (P2-D8):
    the nested-composition depth test sits near the 2 MiB stack limit.
---

# Implementation Log for 2026-10-31-type-aware-evaluation (9 phases)

## Phase 1

Phase 1 changes no product code. It records the dependency audit, the surface
and legacy-flag inventories, the criterion traceability table, the Spike S1
classification, and the "before" performance baseline. All findings were taken
from the worktree at `817792c35` (2026-10-24) on macOS.

### Dependency audit

**Verdict: passed.** Every seam the plan names from
`2026-10-28-schema-enhancements`, `2026-10-27-darkmatter-lifecycle-ownership`,
and `2026-10-29-recursive-schema-types` exists. Rule 2's stop condition does
not apply. `L` = `darkmatter/lib/src/markdown`.

| Seam | Status | Location | Signature / notes |
|---|---|---|---|
| Catalog `returns` | yes | `darkmatter/schemas/partials/functions.yaml` (`kind: schema`, `categories:`) | e.g. `is_array`: `function( parameters(x: unknown \| null); returns(boolean); category(Type Predicates); …)` |
| Catalog loader | yes | `L/compose/expression/catalog/mod.rs:221`, `:256` | `pub fn expression_function_catalog() -> &'static FunctionCatalog` (`LazyLock` over `include_str!`); entry field `pub returns: &'static str`; strict loader `pub(crate) fn load_catalog(` |
| `ArgumentCategory` | yes | `L/compose/expression/call_check.rs:55` | `pub enum ArgumentCategory { Compatible, NullableMatch, Uncertain, Unknown, Incompatible }`, numbered 1–5 by `number()` |
| `ArgumentType` / `::of` | yes | `call_check.rs:106`, `:122` | `pub enum ArgumentType { Literal(Value), Typed(ScopeShape), Unknown }`; `pub fn of(arg: &SpannedExpr, scope: &TypeScope<'_>) -> Self`. **Calls and operators type `Unknown` today** (only variables and constant paths are subjects). |
| `classify_argument` | yes | `call_check.rs:176`, `:182` | `pub fn classify_argument(source: &ArgumentType, parameter: &CompiledParameter) -> ArgumentCategory`; `classify_argument_with_definitions(…)` |
| `check_call` / `CallCheck` | yes | `call_check.rs:708`, `:719`, `:580` | `pub fn check_call(name, arguments: &[ArgumentType], span, argument_spans) -> Option<CallCheck>` delegates to `check_call_with_definitions(…, definitions: &DefinitionTable)` with an empty table |
| `check_expression_calls` / `check_prepared_calls` | yes | `call_check.rs:945`, `:978` | `pub fn check_expression_calls(expr: &SpannedExpr, site: &TypeScope<'_>) -> Vec<CallCheck>`; `pub fn check_prepared_calls(prepared: &PreparedValue, site: &TypeScope<'_>) -> Vec<PreparedCallCheck>` |
| Passive literal classification | yes | `call_check.rs:755–767`, `:839`; `call_binding/bind.rs:211` | all-literal calls go through `registration.bind_call(values)` |
| Composition rejection | yes | `L/compose/call_precheck.rs:56`, `:232`; `L/types.rs:457`; `L/compose/expression/error.rs:379` | `check_document(…) -> Result<(), PrecheckFailure>`; `MarkdownError::rejected_call()`; `ExpressionError::CallCheck(Box<CallCheck>)` |
| `[expressions] strict` | yes | `darkmatter/dmls/src/config/mod.rs:184`, `:285` | `pub struct ExpressionsConfig { pub strict: bool }` on `DmlsConfig.expressions` |
| DMLS call codes | yes | `darkmatter/dmls/src/diagnostics/codes.rs:164`, `:167` | `EXPRESSION_INCOMPATIBLE_CALL = "dm.expression.incompatible_call"`, `EXPRESSION_UNCERTAIN_CALL = "dm.expression.uncertain_call"` (DMLS-owned today; Phase 6 moves them) |
| DMLS call adapter | yes | `darkmatter/dmls/src/diagnostics/call_check.rs:36`, `:50`, `:59` | `severity(category, strict)`, `declared_types(ctx)`, `call_diagnostics(expr, site, strict)` |
| Projection | yes | `L/compose/expression/scope/projection.rs:153–192`, `:304` | `pub fn top_level_types(&self) -> BTreeMap<String, Shape>`; `new(Option<&SimplifiedSchema>)`; `with_definitions(&'s DefinitionTable)` (**a table, not a lookup**); `definitions()`; `with_static_frontmatter`; `with_host_globals(&[&str])`; `with_binding_view(&BindingView)` |
| `with_definitions` callers | yes | — | `L/compose/call_precheck.rs:64`; `darkmatter/dmls/src/diagnostics/call_check.rs:54`; `darkmatter/dmls/src/providers/dsl.rs:419`; `L/compose/directive_targets.rs:105`; `L/compose/shell_expansion/site.rs:516`; `L/compose/preflight/collect.rs:1378`; `claudine/lib/src/composition/sequence/preflight/mod.rs:1609`, `:2013`; `darkmatter/lib/src/lifecycle/shell_template.rs:195` |
| `lifecycle_declared_types` | yes (moved) | `darkmatter/lib/src/lifecycle/shell_template.rs:188` | `pub fn lifecycle_declared_types<'a>(schema: Option<&'a EffectiveSchema>, registry: &'a EventRegistry) -> DeclaredTypes<'a>`; Claudine re-exports it (`claudine/lib/src/composition/lifecycle/mod.rs:49`) |
| Registration facts | yes | `L/compose/expression/call_binding/registration.rs:14`, `:67`, `:136`, `:141` | `EvaluationMode { Eager, Lazy }`; builder `reject_sources(parameter, &[ValueKind])`, `numeric_recovery(subject, fallback)`; `FunctionRegistration { name, overloads, evaluation, recovery }` |
| Narrowing-fact registration slot | **different** | `L/compose/expression/scope/facts.rs:20`, `:34`, `:44` | No fact field on `FunctionRegistration`. Built-in facts are the private static table `PREDICATE_FACTS: &[(&str, PredicateFact)]` (`Family`, `Null`, `Empty`, `FileExists`) applied by `assume(..)`. See decision D1. |
| `BindingView` | yes | `L/compose/expression/binding.rs:247` | `builder(scope)`, `baseline()`, `global(root)`, `globals()`, `classify_root(root)` |
| Global catalogs | yes | `L/schemas/catalog.rs:185–265`; `L/schemas/embedded.rs:165` | `GlobalCatalog::darkmatter()`, `from_documents`, `from_file`, `from_host_file(path, &FileResolutionContext)` |
| Claudine host globals | yes | `claudine/lib/src/composition/schema/globals.rs:23`, `:37`, `:52` | `HOST_CATALOG_TEXT`, `claudine_globals()`, `host_global(name)`; DMLS `validation_view(event, catalogs)` (`darkmatter/dmls/src/overlay/event_views.rs:63`) |
| `TypeExpr::Ref(DefKey)` | yes | `L/schemas/simplified/types.rs:271` | — |
| `EffectiveSchema::definitions()` / `definition(&DefKey)` | yes | `L/schemas/mod.rs:1026`, `:1037` | `definitions()` returns `&Arc<DefinitionTable>` (the document table only); `definition(&DefKey) -> Option<&Definition>` searches every layer |
| `DefinitionLookup` | yes | `L/schemas/simplified/query.rs:346` | `fn lookup_definition(&self, key: &DefKey) -> Option<&Definition>`, implemented for `DefinitionTable` |
| Path / subtype helpers | yes | `L/schemas/frontmatter_shape.rs:116`; `L/schemas/simplified/subtype.rs:98` | `property_def_at_path_with_definitions(root, frontmatter, path, &DefinitionTable)`; `is_subtype_with_definitions(a, b, &DefinitionTable)` |
| `Shape::Ref` in call check | yes | `call_check.rs:350`, `:395`, `:478` | an unresolved target gives `Fit::Unknown` |
| `AbsenceScope` name list | yes | `L/compose/expression/absence.rs:50`, `:84`; `functions/mod.rs:674`; `functions/predicates.rs:16` | `ABSENCE_PREDICATES` = `is_null`/`isnull`, `is_empty`/`isempty` (the list Phase 6 removes) |
| `dm.expression.undeclared_property` | yes | `darkmatter/dmls/src/diagnostics/codes.rs:151` | warning |
| `file_exists_fn` / `fileexists` | yes | `L/compose/expression/functions/fs.rs:162`; `functions/paths.rs:6`; `resolve_ctx.rs:330`, `:342` | catalog `parameters(value: unknown \| null); returns(boolean); fallible;` |
| Host function registration | **absent** | `L/compose/expression/functions/mod.rs:150–172`, `:300`; `functions/test_hooks.rs:77` | The registry is a static `LazyLock` from the embedded catalog plus `BINDING_GROUPS`. The only runtime hook is test/feature-gated `test_hooks::register`. `load_catalog` is `pub(crate)`. Claudine registers no expression function today. See decision D1. |

### Surface inventory: consumers of expression analysis

Marks: **move** = a read-only scan that must move onto a Darkmatter library
API; **library** = already the shared library (a Phase 2–6 target);
**n/a** = not variable visibility or typing.

| File | Role | Mark |
|---|---|---|
| `L/compose/expression/absence.rs` | `AbsenceScope`, `static_variable_reads`, `branch_scope`; the catalog-name rule `call_argument → is_absence_predicate` | library (Phase 6 replaces the rule) |
| `L/compose/expression/functions/predicates.rs:16` | `ABSENCE_PREDICATES` name list | library (removed in Phase 6) |
| `L/compose/expression/binding.rs` | binding model, reserved namespaces | library |
| `L/compose/expression/semantics.rs` | operator/truthiness descriptor catalogs (no AST walk) | n/a |
| `L/compose/expression/lint.rs` | nested `{{…}}` in literals (syntax) | n/a |
| `L/compose/expression/prepared.rs` | `prepare_value`, `validate_prepared`, `validate_expression` | library |
| `L/compose/expression/path_projection.rs` | filesystem path projection for `relative`/`dirname` | n/a |
| `L/compose/expression/call_check.rs` | compatibility API; `ArgumentType::of` types calls/operators `unknown` | library (Phase 2) |
| `L/compose/expression/scope/{projection,mod,facts,shape}.rs` | `DeclaredTypes`, `TypeScope`, guard algebra | library (Phases 2, 3) |
| `L/compose/call_precheck.rs` | composition walk; category 5 only | library |
| `L/compose/interpolation/evaluator.rs` | runtime evaluator | n/a |
| `L/compose/directive_targets.rs` | target nullability via `TypeScope` (own `DeclaredTypes` build) | library |
| `L/compose/unknown_identifiers.rs` | absence candidates → advisories | library |
| `darkmatter/lib/src/lifecycle/validate.rs` | `walk_stack`, `walk_action`, `walk_with_value`, `event_surfaces` (successor of Claudine's deleted `validate.rs`) | library |
| `darkmatter/dmls/src/diagnostics/call_check.rs` | thin severity adapter | library adapter |
| `darkmatter/dmls/src/diagnostics/frontmatter.rs` | frontmatter expression diagnostics through the library walk | library adapter |
| `darkmatter/dmls/src/overlay/expressions.rs:1357` | `undeclared_property_findings` via `static_variable_reads`; local dash-key walk for a quick fix only | library adapter (dash walk n/a) |
| `darkmatter/dmls/src/providers/dsl.rs:406` | `with_site_scope`: own `DeclaredTypes` + `BodyGuards` | library adapter |
| `darkmatter/dmls/src/config/mod.rs` | `ExpressionsConfig.strict`; no `file_exists` setting | n/a |
| `claudine/lib/src/composition/looping/config.rs:393–498` | `extract_control_variables`, `collect_value_template_identifiers`, `collect_identifiers` (local `Expr::*` walk; lifts `fm.<head>`; local reserved-root test `is_reserved_identifier`) | **move** |
| `claudine/lib/src/composition/lifecycle/binding_checks.rs:42` | `validate_no_err_in_no_error_events` (library `prepare_value`/`validate_prepared`) | library adapter |
| `claudine/lib/src/composition/lifecycle/binding_checks.rs:95` + `bindings.rs:423–692` | `find_removed_loop_read`, `removed_loop_read*`, `reject_removed_loop_reads_*` (local `Expr` walk for retired loop names) | **move** |
| `claudine/lib/src/composition/lifecycle/bindings/rename.rs:23` | `read`/`static_path` (local walk for retired sequence globals) | **move** |
| `claudine/lib/src/composition/sequence/preflight/shape.rs:510`, `:522` | `first_target_identity_root`, `target_identity_path_in_expr` (local walk for agent/model late roots) | **move** |
| `claudine/lib/src/composition/lifecycle/bindings.rs:456` | `approval_diagnostics` (library reads filtered by late namespaces) | library adapter |
| `claudine/lib/src/composition/lifecycle/prewarm/classify.rs:114–262` | readiness classification; roots from `static_variable_reads`, local `contains_call` | n/a (decision D3) |
| `claudine/lib/src/composition/dispatch/matcher.rs:123` | condition-vs-regex heuristic | n/a |
| `claudine/lib/src/composition/sequence/preflight/containers.rs:27`, `sequence/task/mod.rs:1376–1410` | shell literal rewriting | n/a |

Places that build their own `DeclaredTypes` (they all switch to the
all-layer definition lookup in Phase 2): `L/compose/call_precheck.rs:62`,
`L/compose/directive_targets.rs:98`, `darkmatter/lib/src/lifecycle/shell_template.rs:188`,
`L/compose/shell_expansion/site.rs:515`, `L/compose/preflight/collect.rs:1373`,
`L/compose/shell_expansion/template/typing.rs:298` (`project_hole`: bare
top-level properties only, everything else `string`),
`L/compose/shell_expansion/template/grammar.rs:176` (`DeclaredTypes::new(None)`),
`L/compose/expression/absence.rs:43` (`UNTYPED`),
`darkmatter/dmls/src/diagnostics/call_check.rs:50`, `darkmatter/dmls/src/providers/dsl.rs:417`, `:489`,
`darkmatter/dmls/src/providers/frontmatter.rs:3120`,
`claudine/lib/src/composition/sequence/preflight/mod.rs:1606`, `:2010`.

**Criterion 8 gap.** No lifecycle action expression is call-checked today:
only `call_precheck.rs` and DMLS call `check_expression_calls`. See decision D2.

### Legacy flag inventory

| Flag | Occurrences | Files (all / live) | clap definition | Accepted by |
|---|---|---|---|---|
| `--allow-missing-hyperlinks` | 6 | 4 / 1 | `darkmatter/cli/src/args/command.rs:154` | compose |
| `--allow-missing-image-refs` | 4 | 4 / 1 | `command.rs:158` | compose |
| `--allow-missing-transclusions` | 8 | 4 / 1 | `command.rs:162` | compose |
| `--allow-any-missing-reference` | 9 | 5 / 1 | `command.rs:166` | compose |
| `--allow-ctx-override` | 23 | 9 / 4 | `command.rs:170` | compose |
| `--allow-invalid-frontmatter-assignment` | 19 | 9 / 3 | `command.rs:174` | compose |
| `--allow-reassigned-frontmatter-property` | 13 | 7 / 3 | `command.rs:178` | compose |
| `--allow-shell-timeout` | 41 | 24 / 13 | `command.rs:198` | compose |
| `--allow-host` | 100 | 42 / 22 | `command.rs:211` (`Vec<String>`) | compose |
| `--no-baseline-schema` | 115 | 54 / 23 | `command.rs:98` (clean), `:186` (compose), `args/target.rs:79` (schema validate) | clean, compose, schema validate |
| `--no-schema-triggers` | 67 | 31 / 24 | `command.rs:102`, `:190`, `target.rs:91` | clean, compose, schema validate |
| `--no-trigger-schemas` | 72 | 44 / 4 | not a clap arg; `RENAMED_FLAGS` in `darkmatter/cli/src/main.rs:136` | none (usage error already) |
| `--strict-style` | 89 | 34 / 12 | `darkmatter/cli/src/args/cli.rs:233` (top-level `Cli`) | default `md FILE` (`darkmatter/cli/src/render.rs:134`) |

"Live" excludes `*/features/**`, `*/fixes/**`, `*/reviews/**`, and
`docs/research/**`. Baseline conflicts: `command.rs:93` (clean), `:181`
(compose), `target.rs:73` (schema validate, on `--schema`). Explicit
`md render` (`command.rs:16–38`) declares none of these flags.

Live files by kind:

- **CLI definitions and commands:** `darkmatter/cli/src/args/{command,cli,target}.rs`,
  `darkmatter/cli/src/commands/{compose,clean}.rs`, `commands/schema/validate.rs`,
  `darkmatter/cli/src/main.rs`, `darkmatter/cli/src/render.rs`.
- **Completions, justfiles, scripts, examples:** none. Completions are
  generated by clap; `darkmatter/cli/tests/l1/schema_trigger_rename.rs::completions_offer_only_the_new_flag`
  is the only completion test.
- **Library messages/doc strings:** `L/types.rs`, `L/compose/shell_expansion/types.rs`,
  `L/compose/shell_blocks/types.rs`, `L/compose/frontmatter_shell_expansion/suffix.rs`,
  `L/compose/context/options.rs`, `L/compose/remote.rs`, `L/compose/icmp.rs`,
  `L/compose/expression/functions/network.rs`, `L/compose/pipeline/mod.rs`,
  `L/schemas/clean.rs`, `L/block/hr_parser.rs`, `L/style/parse.rs`,
  `darkmatter/lib/benches/schema_validation.rs`.
- **Schemas/fixtures:** `darkmatter/schemas/partials/functions.yaml`,
  `darkmatter/lib/tests/fixtures/recursive_schema_frozen/{action,lifecycle,lifecycle-recursive}.yaml`.
- **Docs:** `darkmatter/cli/README.md`; `darkmatter/docs/cli/{compose,clean}.md`;
  `darkmatter/docs/inline/{fm-shell-expansion,preflight-checks,schema-validation,shell-expansion}.md`;
  `darkmatter/docs/lsp/features.md`; `darkmatter/docs/rendering/{disclosure,hr,style}.md`;
  `darkmatter/docs/topics/{caching,darkmatter-expressions,layouts-and-style,markdown-formatting,remote-url-references,transclusion}.md`;
  `darkmatter/docs/topics/schemas/definition.md`;
  `darkmatter/docs/topics/state-management/context-variables.md`;
  `darkmatter/docs/transclusion/block-transclusion.md`.
- **Tests:** `darkmatter/cli/tests/l1/{claudine_schema_bundle,clean_schema,compose_base_schema,compose_dasherized_identifiers,compose_icmp,compose_refs_and_missing,compose_remote_caching,compose_schema,compose_shell_template,compose_shell,compose_state_set,layout_style_frontmatter,literal_glob_hint,schema_robustness_matrix,schema_trigger_rename,schema_triggers,schema_validate_baseline_selection,schema_validate_shipped_artifacts,schema_validate_timings,schema_validate,standalone_baseline}.rs`;
  `darkmatter/lib/tests/l1/{clean_counters,trigger_spelling_guard}.rs`.
- **Other packages and skills:** `.claude/skills/darkmatter/{SKILL,compose,frontmatter,remote-cache,schema}.md`,
  `.claude/skills/claudine/research-contracts.md`,
  `content-policy/docs/topics/policy-lifecycle.md`, `renderable/README.md`,
  `renderable/docs/layout-and-style.md`, `prompts/_add/add-expressions.md`
  (internal prompt folder).
- **Historical (not changed):** about 125 files under feature/fix/review
  snapshots and `docs/research/`.

### Traceability: acceptance criteria → tests

Abbreviations: `LT` = `darkmatter/lib/tests/l1/`, `DT` = `darkmatter/dmls/tests/l1/`,
`CT` = `darkmatter/cli/tests/l1/`, `CL` = `claudine/lib/src/composition/`.
Later phases add the new test names to the last column.

| # | Existing coverage today | Required addition (phase) | Added test(s) |
|---|---|---|---|
| 1 | `LT/design_time_call_check.rs::a_mixed_root_union_projects_nothing` (Phase B behavior; changes) | root-union fold, order independent (P2) | `LT/variable_type_projection.rs::root_unions::the_motivating_table_folds_every_arm`, `::root_unions::the_fold_is_independent_of_arm_and_key_order`, `::root_unions::arms_that_differ_fold_to_a_union_with_each_arms_optionality` |
| 2 | `LT/expression_type_scope.rs::type_predicates_narrow_a_declared_union_on_both_sides` (partial) | every normalization row (P2) | `LT/variable_type_projection.rs::normalization::every_row_of_the_table`, `::normalization::presence_markers_do_not_split_a_member`, `::normalization::a_property_level_union_normalizes_like_a_fold` |
| 3 | `LT/unknown_identifier_warning.rs::a_baseline_schema_property_is_known`, `::a_matched_trigger_schema_property_is_known` (presence only) | all-layer + raw JSON Schema projection (P2) | `LT/variable_type_projection.rs::layers::every_layer_of_the_effective_schema_projects`, `::layers::a_raw_json_schema_exposes_its_enumerable_properties`, `::layers::a_raw_root_union_folds_its_arms`, `::layers::a_referenced_raw_schema_types_its_properties` |
| 4 | `LT/expression_type_scope.rs::constant_paths_and_indices_are_subjects_and_computed_ones_are_not` (subjects only) | `meta.count` → `number \| null` (P2) | `LT/variable_type_projection.rs::nested_paths::a_member_of_an_optional_object_may_be_null` |
| 5 | `LT/global_catalog.rs::err_declares_exactly_the_runtime_fields`, `::timing_declares_the_runtime_fields_as_optional_integers`, `::ctx_and_current_share_one_shape_projected_by_the_context_descriptors`; `DT/host_globals.rs::host_global_hover_comes_from_the_host_catalog_file` | reserved/host global types in `DeclaredTypes`; unavailable → no type (P2) | `LT/variable_type_projection.rs::globals::reserved_roots_take_their_catalog_shapes`, `::globals::an_unavailable_global_has_no_type`, `::globals::a_host_global_takes_its_host_catalog_type`; `CL/lifecycle/tests/call_check.rs::claudine_globals_take_their_host_catalog_types` |
| 6 | `LT/design_time_call_check.rs::the_worked_example_rows`; `DT/expression_call_diagnostics.rs::rows_a_to_e_by_default` | rows f, i category 1 (P2) | `LT/variable_type_projection.rs::call_typing::worked_example_rows_f_and_i_are_exact`, `::call_typing::operators_and_calls_are_typed_from_their_operands`, `::call_typing::an_overloaded_call_unions_its_candidate_returns`; `LT/design_time_call_check.rs::categories::the_worked_example_rows` (rows f, i now category 1); `DT/expression_call_diagnostics.rs::settings::rows_f_and_i_are_silent_under_both_settings` |
| 7 | `LT/design_time_call_check.rs::projection_is_deterministic_and_reads_only_the_schema`; `LT/absent_property_contract.rs::*` | instrumented passivity + unchanged state (P2) | `LT/variable_type_projection.rs::passivity::projection_reads_no_file_and_runs_no_handler`, `::passivity::null_materialization_is_unchanged` |
| 8 | none (lifecycle actions are not call-checked) | lifecycle `min(flag, 5)` rejection parity (P2, decision D2) | `LT/variable_type_projection.rs::lifecycle_calls::a_lifecycle_call_is_rejected_exactly_like_the_body_call`, `::lifecycle_calls::a_compatible_or_untyped_lifecycle_call_is_not_rejected`; `CT/compose_lifecycle.rs::a_provably_wrong_lifecycle_call_rejects_before_any_event`; `CL/lifecycle/tests/call_check.rs::a_lifecycle_call_is_rejected_like_the_body_call` |
| 9 | `LT/design_time_call_check.rs::guards_narrow_the_argument`; `DT/expression_call_diagnostics.rs::guards_narrow_rows_g_and_h`; `DT/expression_scopes.rs::expression_branch_rows_agree_with_the_library` | composition assertion on extended scope (P3) | — |
| 10 | `LT/expression_type_scope.rs::a_conjunction_proves_each_operand_on_its_true_side_only`, `::a_ternary_guard_joins_its_branch_routes`; `DT/expression_scopes.rs::hover_inside_a_body_block_guard_shows_the_library_type` | category 1/2 for `x ?`, `x &&`, `::block when` (P3) | — |
| 11 | `LT/expression_type_scope.rs::type_predicates_narrow_a_declared_union_on_both_sides`; `DT/type_scope_ownership_guard.rs::*` | true-iff vs true-implies registry; `is_file_type`, host facts, shell hole (P3) | — |
| 12 | `LT/expression_type_scope.rs::a_child_scope_shares_its_parent_and_never_changes_it` | before/after state fixture (P3) | — |
| 13 | none (`is_file_type` absent); biscuit-file `parse.rs` unit tests `empty_string_rejected`, `https_url_detected_as_url_kind` | `is_file_type` corpus with failing stubs (P4) | — |
| 14 | `LT/path_function_contracts.rs::file_exists_inspects_its_argument_without_converting_it`; `LT/context_function_contracts.rs::file_exists_inspects_its_argument_without_converting_it` | `""`/`https://` false without I/O; `file://` corpus (P4) | — |
| 15 | `DT/expression_call_diagnostics.rs::file_exists_takes_a_number_silently` (adjacent) | always-false literal warning (P4) | — |
| 16 | `LT/design_time_call_check.rs::the_worked_example_rows` (category 4 machinery) | guarded `frontmatter` category 1 (P4) | — |
| 17 | `LT/function_binding_registry.rs::every_registered_function_has_a_row` (built-ins) | host predicate, catalog, never-invoked counter (P5) | — |
| 18 | DMLS `config/mod.rs` unit tests (strict only) | assert no `file_exists` setting (P4) | — |
| 19 | `DT/call_check_agreement.rs::composition_and_the_editor_reject_the_same_documents`; `DT/recursive_call_check.rs::the_editor_reports_exactly_the_recursive_call_composition_rejects` | identical fields across composition, report-json, DMLS (P6, P8) | — |
| 20 | none | category 3 warning; `--deny all` fails (P6, P7) | — |
| 21 | `LT/unknown_identifier_warning.rs::every_suppression_row_warns_exactly_where_evaluation_reads_an_unhandled_root`, `::a_handled_absence_is_silent_and_renders_the_fallback`; `DT/expression_scopes.rs::undeclared_property_findings_match_the_library_branch_facts`; `absence.rs` unit tests | `\| null` type rule; no name list (P6) | — |
| 22 | `DT/no_side_effects.rs::expression_validation_runs_no_actions_shells_file_effects_or_lazy_providers` | instrumented report privacy/passivity (P6, P8) | — |
| 23 | none | precedence and conflict usage errors (P7) | — |
| 24 | `CT/compose_shell.rs::test_compose_allow_shell_timeout_emits_warning`; `CT/compose_refs_and_missing.rs::test_compose_scalar_ctx_with_allow_override_succeeds`; `CT/compose_state_set.rs::test_set_overlay_permissive_reassigned_warns_and_rightmost_wins`, `::test_set_overlay_permissive_invalid_warns`; `CT/compose_remote_caching.rs::test_compose_remote_allowed_host_fetches_url` (no missing-reference allow tests exist) | port each to `--allow <id>`; `--allow all` grants no host/shell (P7) | — |
| 25 | `CT/schema_trigger_rename.rs::the_old_flag_is_a_usage_error_naming_the_new_one`, `::completions_offer_only_the_new_flag`; `LT/trigger_spelling_guard.rs::no_active_file_spells_a_renamed_trigger_form` | every removed flag on every command; spelling guard (P7) | — |
| 26 | `CT/layout_style_frontmatter.rs::style_strict_style_fails_on_unknown_key`, `::style_strict_style_fails_on_deprecated_key`, `::style_strict_style_accepts_markdown_cleanup_rules` | `--deny style`; info never blocks (P7) | — |
| 27 | `LT/compose_call_precheck.rs::a_proven_bad_call_rejects_before_any_expression_runs`, `::a_bad_body_call_rejects_before_frontmatter_runs` | stop-before-effect / no rollback under `--deny` (P7) | — |
| 28 | `CT/schema_trigger_rename.rs::compose_reports_an_invalid_trigger_field_by_location_and_composes_nothing` (partial) | no stdout document in all modes; no viewer/file publish (P7, P8) | — |
| 29 | none | report-json success/failure schema (P8) | — |
| 30 | this log (baseline below) | "after" sample + explanation (P8) | — |
| 31 | `LT/expression_type_scope.rs::bare_and_fm_reads_share_one_document_binding_unless_a_host_root_shadows_it`, `::an_unavailable_global_stays_unavailable_under_any_guard`; `LT/host_globals_compose.rs::reading_an_unavailable_global_is_a_typed_binding_error_in_body_and_frontmatter` | type-level bare/`fm` agreement (P2) | `LT/variable_type_projection.rs::precedence::bare_and_fm_paths_agree_unless_a_host_global_shadows_the_bare_one` |
| 32 | `LT/expression_type_scope.rs::constant_paths_and_indices_are_subjects_and_computed_ones_are_not`; `LT/recursive_schema_typed_consumers.rs` | item type + null; terminating recursive path (P2) | `LT/variable_type_projection.rs::nested_paths::constant_indices_project_the_item_type`, `::nested_paths::a_path_through_a_recursive_type_terminates` |
| 33 | `LT/expression_type_scope.rs::a_disjunction_proves_what_both_routes_prove`, `::negation_swaps_sides_and_parentheses_keep_facts`; `LT/compose_call_precheck.rs::a_dead_ternary_branch_is_checked` | re-assert on extended scope (P3) | — |
| 34 | `LT/expression_type_scope.rs::a_write_invalidates_its_aliases_ancestors_and_descendants_only`, `::a_join_restores_only_incoming_facts_no_branch_could_have_written`; `CL/lifecycle/executor/tests/guarded_shell_sites.rs::a_set_before_the_shell_ends_the_guard_proof`; `CL/lifecycle/executor/tests/scope_site_conformance.rs::lifecycle_set_shell_negated_guard` | lifecycle guard → `set` → later call, runtime and DMLS (P3) | — |
| 35 | `CL/lifecycle/executor/tests/scope_site_conformance.rs::file_exists_alias`; `LT/scope_site_conformance.rs::file_exists_guards_are_analyzed_without_running_a_handler` | context-free `is_file_type`; passive validator; `fileexists` parity (P4) | — |
| 36 | `LT/design_time_call_check.rs::criterion_36_pairs`, `::null_is_accepted_only_by_an_explicit_null_arm` | `\| null` suppression incl. host and overloads (P6) | — |
| 37 | `LT/function_binding_registry.rs::*`; `LT/function_catalog_parity.rs` | registration errors; `.facts.json` matrix (P5) | — |
| 38 | `CT/clean_schema.rs::test_baseline_schema_flags_conflict`; `CT/schema_validate_baseline_selection.rs::no_baseline_schema_conflicts_with_schema_flag`; `CT/schema_trigger_rename.rs::the_new_flag_disables_triggers_on_compose_validate_and_clean` | registry mappings, unknown/repeated ids, host denial, precedence (P7) | — |
| 39 | none | report-json failure kinds, transcluded positions, non-compose rejection (P8) | — |

### Spike S1: biscuit-file structural-validation gap

Read from `biscuit-file/lib/src/file_reference/parse.rs` (**P**),
`resolve.rs` (**R**), `darkmatter/…/functions/fs.rs` (**FS**), and
`resolve_ctx.rs` (**RC**). Results were confirmed with the installed `md`
(built 2026-10-23, same code paths) in a scratch repository.
`FileReference::new` (`file_reference/mod.rs:615`) is `parse::parse` (P:9),
purely lexical with no I/O. `file_exists` today: a non-string is `false`; an
`is_remote_url` string (RC:330) is fetched; anything else goes through
`normalize_path_arg` (RC:342) → `FileReference::new` → resolve.

| Input | `FileReference::new` today | `file_exists` today | Required `is_file_type` | Gap |
|---|---|---|---|---|
| `./docs/spec.md` | Ok ExplicitRelative | local resolve | true | none |
| `/tmp/spec.md` | Ok Absolute | local (Windows: `ForeignAbsolutePath`, R:196) | true | none |
| `\Users\ken\data\foobar.json` | Ok ImplicitRelative (P:212) | local → false | true | none |
| `@docs/spec.md`, `%@spec.md` | Ok Magic (recursive for `%`) | local | true | none |
| `vault:notes/spec.md` | Ok Vault | `VaultNotConfigured` error (R:938) | true | none |
| `https://example.com/spec.md`, `http://…` | Ok Url (P:93) | remote fetch / policy | true | none |
| `42` (string), `hello world` | Ok ImplicitRelative | local → false | true | none |
| `""` | Err InvalidSyntax (P:12) | **error, not false** | false | darkmatter must return `false` (P4) |
| `https://`, `http://` | **Ok Url** (no authority check) | collapse turns it into `https:/` → UnsupportedScheme error | false | **URL authority check** |
| `&docs/spec.md`, `^docs/spec.md` | Ok RepositoryRoot / RepositoryScoped | local | true | none |
| `~/spec.md`, `~` | Ok Home | local | true | none |
| `{{HOME}}/x` (env template; grammar `{{NAME}}`, `NAME` in `[A-Z0-9_]+`) | Ok + EnvVar (P:248–291) | local; unset var errors | true | none |
| `${HOME}/x`, `$HOME/x` | Ok, literal text | local → false | true (legal file name) | none |
| `{{HOME/x`, `{{}}`, `{{home}}`, `{{env.X}}` | Err InvalidSyntax (P:260/266/273) | error | false | none |
| `a\0b.md` | **Ok ImplicitRelative** | reaches `fs::metadata` (R:363) → error | false | **NUL check** |
| `ftp://x/y`, `mailto:a@b`, `s3://b/k` | Err UnsupportedScheme | error | false | none |
| `file://foo/bar`, `file:///tmp/spec.md` | Err UnsupportedScheme | adapter strips `file://` → local | true (through adapter) | none (adapter stays in Darkmatter) |
| `file://` | Err UnsupportedScheme | adapter → `""` → error | false | none |
| `\\server\share\x`, `C:\x\y.md` | Ok Absolute | macOS: `ForeignAbsolutePath` | true | none |
| `//server/share/x` | Ok Absolute | **adapter collapses to `/server/share/x`** | true | adapter defect (Darkmatter, D5) |
| `C:x`, `\\?\C:\x` | Err | error | false | none |
| `@`, `&`, `^`, `@/`, `%`, `%%x`, `!x`, `~bob/x` | Err (P:19, 53, 112, 146–161, 197) | error | false | none |
| `vault:`, `vault::` | **Ok Vault, empty payload** | error / probes vault root | false | **empty vault payload check** |
| `vault:/etc/hosts` | **Ok Vault, rooted payload** | joins as absolute, escaping the vault root | false | **rooted vault payload check** (D4) |
| `@@x` | Ok Magic, payload `@x` (documented valid, `mod.rs:646`) | local | true | none |
| `%https://example.com/x` | **Ok Url, recursive** | `RemoteNotLocal` at resolve (R:88) | false | **recursive-URL check** |
| `https:///x` | Ok Url | url crate makes host `x` → fetch | false (D4) | covered by authority check |

**Structural checks to add to biscuit-file** (Phase 4, all in `parse::parse`
/ `detect_kind`, so `FileReference::new` itself is the passive validator):

1. **NUL rejection** at the top of `parse` (next to the empty check, P:12):
   `InvalidSyntax` for any `\0`, every kind.
2. **URL authority check** in the URL branch (P:93–95): a non-empty literal
   authority after `scheme://`, validated with the same `url::Url::parse` +
   host check resolve already does (R:407–417). An authority that is a
   `{{NAME}}` template is accepted when non-empty; variables are never read.
3. **Recursive URL rejection:** `%` combined with a URL (`%https://…`) is
   `InvalidSyntax` at parse time.
4. **Vault payload checks:** an empty payload (`vault:`, `vault::`) and a
   rooted payload (`is_rooted_magic_payload`, P:229) are `InvalidSyntax`.

**`file://` adapter** (`normalize_path_arg`, RC:342–360; an engine-local copy
at `darkmatter/lib/src/effects/verbs.rs:329–345`): strips one leading
`file://`, then collapses every run of `/`. The collapse violates spec §3's
"do not collapse separators in a way that changes UNC or URL meaning"
(`//server/share/x` and `file:////server/share/x` lose their UNC meaning).
`file_exists` routes URLs with Darkmatter's own `is_remote_url`, separately
from `FileReference`'s `Url` kind. The required Phase 4 order is:
adapter → `FileReference::new` (on `Err` return `false`) → route on the
reference's `Url` kind for the remote fetch, otherwise resolve locally.

### Performance baseline ("before")

- **Host:** Apple M4 Max, 16 cores, 128 GiB, macOS 27.2. Load average 7–9
  during measurement (other sessions on the host).
- **Build:** `cargo build --release -p darkmatter-cli --bin md -p dmls --bin dmls`
  at `817792c35`, binaries in `target/release/`.
- **Input:** `darkmatter/cli/tests/fixtures/frozen_prompts/prompts/plan.md`, a
  frozen copy of the repository's planning prompt (5.6 KB, 16 `{{ … }}`
  sites). It has a root-union `$schema` (the shape criterion 1 changes),
  `||` and `?:` expressions, and lifecycle stacks with expressions. It
  composes successfully with `spec=claudine/features/2026-10-31-type-aware-evaluation/spec.md`.
- **Composition:** `hyperfine -N --warmup 3 --runs 20 "target/release/md compose <input> spec=<spec>"`
  → **797.9 ms ± 5.5 ms** (range 787.1–807.1 ms; user 765 ms, system 219 ms).
  `--perf` prints no report for this document (it takes the lifecycle path),
  so the wall-clock figure includes context capture and I/O.
- **Schema stages (checking vs other work):** `md schema validate -vv <input>`
  (exits 1 without `spec`, but the timings are still reported): resolve
  8.3 ms, compile 28.5 ms, validate 48.3 ms, total **85.5 ms**.
- **DMLS analysis:** `python3 -I claudine/features/2026-10-31-type-aware-evaluation/perf/lsp_drive.py target/release/dmls <input> . 10`
  (stdio LSP; workspace root = repo; didOpen → `publishDiagnostics`; 0
  diagnostics). First publish: median **250.1 ms** (min 245.2, max 274.8).
  The second publish (schema compile cache hit, which is mostly the
  expression and diagnostic work) followed after **41.8–48.5 ms**. DMLS's own
  `schema validation complete` log for the first publish: resolve 27–29 ms,
  compile 27–30 ms, validate 45–48 ms, total **101–107 ms**.
- Phase 8 reruns exactly these three commands on the same input and host.

### Design Decisions

- **D1. The dependency gate passes; the fact slot and host registration are
  this spec's work.**
  - *Decision:* The static `PREDICATE_FACTS` table (`scope/facts.rs`) is
    `TypeScope`'s fact registry. Phase 3 turns it into facts carried on
    `FunctionRegistration`, and Phase 5 adds the public host registration
    path (function catalog loading through the strict `load_catalog`, plus
    registration). Work does not stop under Rule 2.
  - *Alternatives:* stop and report these as missing Phase B seams.
  - *Why:* spec §2 says new facts register "into the same registry built-ins
    use", which today is that table. Spec §5 makes host registration this
    spec's deliverable ("through the same Rust registration API the built-ins
    use"). Neither is a Phase B outcome; Phase B's deliverables (catalog
    `returns`, categories, call check, strict, projection) are all present.
  - *Downsides:* Phases 3 and 5 carry the registry-opening work the plan's
    audit list described as a pre-existing slot.
- **D2. Criterion 8 wires the call check into Darkmatter's lifecycle walk.**
  - *Decision:* Phase 2 runs `check_expression_calls` from
    `darkmatter/lib/src/lifecycle/validate.rs`'s action walk, under
    `lifecycle_declared_types`, so `md` and Claudine hosts both get it.
  - *Alternatives:* a Claudine-side check in its lifecycle parser.
  - *Why:* lifecycle validation moved into Darkmatter
    (`claudine/lib/src/composition/lifecycle/validate.rs` no longer exists),
    and rule 3 forbids a second implementation.
  - *Downsides:* Claudine lifecycle fixtures whose actions hold a provably
    bad call start failing; such a fixture holds a real bug.
- **D3. Which Claudine scans move (spec §5's last paragraph).**
  - *Decision:* Phase 5 moves every Claudine walk that decides which roots
    or paths an expression reads onto Darkmatter's reads API
    (`static_variable_reads` / `binding::is_reserved_namespace`, extended
    there if needed): `looping/config.rs::collect_identifiers` and its
    callers; `lifecycle/bindings.rs::removed_loop_read*` with
    `binding_checks.rs::find_removed_loop_read` (the successor of the
    deleted `validate.rs` scan); `bindings/rename.rs::read`/`static_path`;
    and `sequence/preflight/shape.rs::target_identity_path_in_expr`.
    `prewarm/classify.rs` already takes its roots from
    `static_variable_reads`, and `dispatch/matcher.rs` is a grammar heuristic,
    so both stay.
  - *Alternatives:* move only the two files the spec names.
  - *Why:* the spec's rule is "Claudine keeps no independent definition of
    variable visibility or types", and the spec's two files have since
    moved or split. Every walk listed defines visibility locally.
  - *Downsides:* a wider Claudine diff in Phase 5.
- **D4. Structural verdicts for edge spellings S1 surfaced.**
  - *Decision:*
    - `https:///x` (empty literal authority) is malformed, so `is_file_type`
      is `false`.
    - `vault:`, `vault::`, and rooted vault payloads (`vault:/etc/hosts`)
      are malformed.
    - `%https://…` is malformed.
    - `@@x` stays valid (it is documented grammar).
    - `${X}`/`$X` stay literal file-name text.
  - *Alternatives:* accept `https:///x` because the `url` crate normalizes
    it; leave rooted vault payloads to resolve.
  - *Why:* spec §3 makes malformed reference syntax, including an empty URL
    authority, `false`. Reserved malformed forms are false rather than
    implicit local paths. A rooted vault payload escapes its root, as a
    rooted magic payload does, and the parser already rejects that one.
  - *Downsides:* `vault:/x` and `%https://…` become parse errors where they
    used to fail at resolution; error text changes for those inputs.
- **D5. The `file://` adapter stops collapsing separators.**
  - *Decision:* Phase 4 changes `normalize_path_arg` (and its copy in
    `effects/verbs.rs`) to strip the `file://` prefix only. The structural
    check then runs on the unmodified remainder.
  - *Alternatives:* keep the collapse and special-case UNC.
  - *Why:* spec §3 forbids collapsing in a way that changes UNC or URL
    meaning. The collapse is also why `https://` reaches `UnsupportedScheme`
    today instead of a structural rejection.
  - *Downsides:* a path with doubled slashes (`a//b`) is no longer
    normalized by Darkmatter. Resolution treats it the same on every OS.
- **D6. Baseline input and method.**
  - *Decision:* the frozen `plan.md` prompt, release builds, hyperfine for
    composition, and the checked-in stdio driver
    (`perf/lsp_drive.py` in this feature directory) for DMLS.
  - *Alternatives:* a live `prompts/` file (it may change between phases); a
    Criterion bench (does not cover DMLS).
  - *Why:* a frozen input guarantees the same bytes in Phase 8. It exercises
    root unions, `||`, `?:`, and lifecycle expressions. The driver measures
    the editor path without a client.
  - *Downsides:* composition time is dominated by context capture and I/O;
    the checking share is visible only through the schema stage timings and
    DMLS's cache-hit republish.
- **D7. Legacy-flag migration has no static completion files.**
  - *Decision:* Phase 7 updates clap definitions only for completions, and
    extends `completions_offer_only_the_new_flag`-style tests to every
    migrated command. `--strict-style` lives on the top-level `Cli`
    (default `md FILE` only), and `md render` declares none of the flags
    today, so `--deny style` and the verbs are added there by the registry.
  - *Alternatives:* none.
  - *Why:* the inventory found no static completion, justfile, script, or
    example uses.
  - *Downsides:* none.

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter"]`, `change_class: package`. `plan.md` and this log
  are test inputs of two `darkmatter` L1 tests (`--resolved-plan` filter:
  `shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively`,
  `shipped_schema_corpus::shipped_markdown_named_type_references_resolve`).
  `perf/lsp_drive.py` alone selects nothing (`documentation`). The gates
  were therefore **not** skipped.
- **Narrowed darkmatter cell:** `cargo nextest run -p darkmatter --test l1 -E 'test(=shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively) | test(=shipped_schema_corpus::shipped_markdown_named_type_references_resolve)'`
  → 2 passed.
- **`claudine/` `just test`:** 11445 passed (13 slow), 9 skipped, 296.6 s.
- **`claudine/` `just lint`:** passed.
- **`just check-tier-coverage claudine`:** passed, 0 stranded.
- No pre-existing or load-sensitive failures. No cross-OS check: Phase 1
  changed no product code.
- **Requirement-to-test mapping:** Phase 1 changes no behavior, so it adds
  no tests. The traceability table above maps every criterion to the
  existing tests and to the phase that adds the missing ones.
- **Non-product file created:** `claudine/features/2026-10-31-type-aware-evaluation/perf/lsp_drive.py`
  (measurement driver for criterion 30; not package source).

## Phase 2

Phase 2 completes the variable-type projection (spec §1; criteria 1–8, 31,
32). Everything lands in the one projection, `DeclaredTypes`
(`darkmatter/lib/src/markdown/compose/expression/scope/projection.rs`), and
in `TypeScope`'s `Shape`. No consumer walks schema syntax itself.

### What changed

- **Union normalization** (`scope/shape.rs`): `Shape::union` is the one
  operation every projection, root-union fold, scope join, and operator type
  uses. `unknown` absorbs (including `null`). Members are deduplicated by
  type identity: presence markers, timing, annotations, and descriptions are
  stripped from top-level members, because a shape carries presence in
  `nullable`. Members are sorted canonically by their serialized type text.
  `Shape::of_type` is a type's values with no optionality (a `returns(...)`);
  `Shape::from_definition` adds `null` for an optional property;
  `Shape::to_definition` converts back for `DeclaredType::Property`.
- **Layers** (`projection.rs`): `DeclaredTypes::for_schema(Option<&EffectiveSchema>)`
  reads the authored SimplifiedSchema, then the trigger shapes (later
  wins), then `RawDeclarations` of the merged JSON Schema, which carries the
  baseline and every raw layer. A root union folds per arm; an arm that does
  not declare a key takes the lower layers' declaration, else `unknown`.
  Arms that agree exactly keep the authored definition (constraints
  included) for shell-hole validators.
- **Definitions across layers**: `EffectiveSchema::layered_definitions()`
  merges the document's table with each trigger layer's (`DefinitionTable::absorb`,
  cached in a `OnceLock`). `DeclaredTypes::with_definitions` keeps its
  `&DefinitionTable` parameter (decision P2-D2).
- **Raw JSON Schema** (`scope/raw.rs`, `RawDeclarations`, cached by
  `EffectiveSchema::raw_declarations()`): literal `properties` through
  root `anyOf`/`oneOf` (folded) and `allOf` (intersected); `type`, `const`,
  `enum`, `items`, the two file formats; `$ref` and everything else are
  `unknown`.
- **Nested paths**: `DeclaredTypes::projected` walks constant members and
  indices through inline objects, arrays, tuples, and recursive references
  (one definition opened per segment, alias chains bounded by the table
  size). `DeclaredType::NestedProperty` is removed: a typed path is
  `Property(def)`, an untyped one `NoProjection`.
- **Reserved roots and host globals**: `ctx`/`env`/`err`/`timing` take
  Darkmatter's catalog shapes; a host global takes its host catalog's shape
  (`DeclaredTypes::with_global_catalog`, `EventRegistry::register_global_catalog`,
  registered by Claudine's `claudine_registry()` and set on the sequence
  preflight sites). An available global's root is present, an
  execution-dependent one nullable, and an unavailable one has no type.
  `ctx` members keep their descriptor presence.
- **Call and operator typing** (`call_check.rs::expression_type`, public):
  call = union of the `declared_returns` (new `CompiledSignature` field) of
  every overload the argument count selects; `x || y` and `c ? a : b` as the
  spec states, with operands typed under `TypeScope::assume`; anything else
  `unknown`. `ArgumentType::of` reads it.
- **Lifecycle call check (criterion 8, decision D2)**:
  `darkmatter::lifecycle::reject_incompatible_calls` checks every surface
  `walk_stack` yields, from authored text, with each event's binding view.
  The error is the new `LifecycleError::LifecycleCallRejected` (with
  `rejected_call()`), and its Claudine twin is
  `CompositionError::LifecycleCallRejected` (mapped, rendered, coded
  `composition.lifecycle_invalid`, and in the JSON-detail and property
  groups). md calls it from `md compose` before the lifecycle run, against
  `darkmatter::markdown::compose::effective_schema_for` (new public helper,
  the pipeline's own schema assembly). Claudine calls it in `prepare.rs`
  right after the launch schema resolves (direct and inline preparation).
- **Callers moved to `for_schema`**: composition precheck, directive
  targets, shell-site discovery (`preflight/collect.rs`) and fill
  (`shell_expansion/site.rs`, which now stores the boxed `EffectiveSchema` so
  discovery and fill type identically), `lifecycle_declared_types`, DMLS
  call diagnostics and site scopes, and Claudine's sequence preflight.

### Design Decisions

- **P2-D1. Canonical member order is the serialized type text.**
  - *Decision:* `Shape::union` sorts members by `serialize_property_atom` of
    the stripped member, so messages print `number | string`.
  - *Alternatives:* keep authored order (not order independent, which
    criterion 2 requires); rank by value family first.
  - *Why:* the annex requires one canonical order. Text order is total,
    deterministic, and needs no second ranking table.
  - *Downsides:* messages and hovers no longer echo the authored order. Ten
    existing tests that pinned `string | number` were updated.
- **P2-D2. Layered definitions are one merged table, not a lookup trait
  object.**
  - *Decision:* `EffectiveSchema::layered_definitions()` returns one
    `DefinitionTable` holding the document's definitions and every trigger
    layer's. `with_definitions` keeps `&DefinitionTable`.
  - *Alternatives:* widen `with_definitions`, `check_call_with_definitions`,
    `classify_argument_with_definitions`, subtyping, and the call binder's
    translator to `&dyn DefinitionLookup`.
  - *Why:* a `DefKey` names one definition wherever it is reached, so the
    merge resolves exactly what `EffectiveSchema::definition(&DefKey)` does
    (spec §11's requirement). It needs no change to the call-binding and
    subtype APIs, and it is computed once per schema.
  - *Downsides:* one table clone per effective schema with trigger
    definitions (none when no trigger has any).
- **P2-D3. The baseline is projected from the merged JSON Schema.**
  - *Decision:* layers that `EffectiveSchema` holds only as JSON (the
    baseline, raw document or trigger schemas) project through
    `RawDeclarations` of `json_schema`.
  - *Alternatives:* store the baseline's SimplifiedSchema on
    `EffectiveSchema`.
  - *Why:* a configured baseline may itself be raw JSON. The merged JSON is
    "the final EffectiveSchema after resolution and merging", which the spec
    names, and it needs no new stored state.
  - *Downsides:* baseline types are the conservative raw projection (for
    example `date` is `string`). A sound supertype never proves a call
    falsely valid or invalid.
- **P2-D4. A root-union arm that lacks a property takes the lower layers'
  declaration.**
  - *Decision:* an arm with no declaration of a key contributes the
    trigger's or the raw layer's declaration of it, and only otherwise
    `unknown`.
  - *Alternatives:* always `unknown` for a missing arm declaration.
  - *Why:* the merged schema merges the baseline and triggers into every
    arm, so the arm does declare the property at runtime.
  - *Downsides:* the raw fallback is the fold over every merged arm, which
    is a supertype of that arm's own.
- **P2-D5. Lifecycle rejection ignores guards.**
  - *Decision:* `reject_incompatible_calls` types each surface at an
    unguarded site. It has an allowlist entry in
    `type_scope_ownership_guard.rs`.
  - *Alternatives:* narrow each surface by its enclosing `when`.
  - *Why:* narrowing only removes alternatives, and an impossible branch
    keeps its incoming type. A call that is category 5 unguarded is category
    5 under every guard, so the verdict is identical. Guard-sensitive
    warnings (categories 2 to 4) on lifecycle surfaces are Phase 3/6 work.
  - *Downsides:* none for rejection.
- **P2-D6. A new lifecycle twin rather than reusing `MarkdownError`.**
  - *Decision:* `LifecycleError::LifecycleCallRejected` carries the
    `ExpressionError::CallCheck` as its source, and Claudine's
    `CompositionError` twin mirrors it.
  - *Alternatives:* raise a `MarkdownError::Interpolation` from the
    lifecycle layer.
  - *Why:* lifecycle validation errors are the `LifecycleError` family. The
    twin rule (`lifecycle_authoring.rs`) keeps one mapping, and the verdict
    and message are the body's (`rejected_call()`).
  - *Downsides:* one more twin pair to render.
- **P2-D7. An available host global's root is present.**
  - *Decision:* `Availability::Available` removes `null` from the root's
    type, `ExecutionDependent` keeps it, and `Unavailable` has no type.
  - *Alternatives:* use the catalog definition's own optionality, which is
    always optional.
  - *Why:* availability is the host's statement that the binding exists in
    that scope.
  - *Downsides:* `err.msg` now proves present in failure events; presence
    consumers see that evidence.
- **P2-D8. `SiteScope` boxes its schema.**
  - *Decision:* `shell_expansion::site::SiteScope` holds
    `Option<Box<EffectiveSchema>>`.
  - *Why:* storing the effective schema inline grew the pipeline's
    stage-dispatch frame (`run_inline_pre_operation`) by 1 KB per
    nested-composition level. With that,
    `nested_composition::self_composition_reaches_the_depth_limit` overflowed
    a 2 MiB debug test thread. Measured with lldb: HEAD already needed 2.0–2.1
    MB for that test, so it sits within 5% of the default stack. That margin
    is a pre-existing risk, recorded here, not introduced by this phase.
  - *Downsides:* one allocation per document with shell blocks.

### Requirement-to-test mapping

See the traceability table in Phase 1 ("Added test(s)" column, criteria 1–8,
31, 32). New file `darkmatter/lib/tests/l1/variable_type_projection.rs`
(declared in `tests/l1/main.rs` under `effects-instrumentation`, which CI
enables for the library), plus
`darkmatter/cli/tests/l1/compose_lifecycle.rs::a_provably_wrong_lifecycle_call_rejects_before_any_event`,
`darkmatter/dmls/tests/l1/expression_call_diagnostics.rs::settings::rows_f_and_i_are_silent_under_both_settings`,
and `claudine/lib/src/composition/lifecycle/tests/call_check.rs` (two tests).
Passivity (criterion 7) is proven with a deleted fixture tree as a failing
file-access stub and handler-entry counters, not return values alone.

Updated existing tests (behavior changed by design):
`design_time_call_check.rs` (rows f, i now category 1; bare `env` is an
`object`, so `min(env, 5)` is category 5; canonical order),
`expression_type_scope.rs` (canonical order; `NestedProperty` removed),
`directive_target_analysis.rs` (a raw-JSON-only and a disagreeing root-union
`optional` are now typed, so nullable), `dmls/tests/l1/expression_scopes.rs`
(canonical order), `type_scope_ownership_guard.rs` (allowlist and baseline
snippet for the lifecycle site), and `lifecycle_authoring.rs` (one more
twin).

### Gates

- **Scope calculator:** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit", "claudine-cli"]`,
  `change_class: package`. Gates were run.
- **darkmatter library** (`cargo nextest run -p darkmatter --features effects-instrumentation`, L1 filter):
  9514 passed, 4 skipped (before the last lifecycle test additions; those
  and the module they extend were rerun and pass).
- **darkmatter CLI** (`-p darkmatter-cli --features test-fixtures`, L1): 1250 passed.
- **DMLS** (`-p dmls --features effects-instrumentation`, L1): 1375 run, one
  canonical-order expectation updated, then `expression_scopes` and
  `expression_call_diagnostics` (22) rerun and pass.
- **`claudine/` `just test`:** 11447 passed (14 slow), 9 skipped.
- **`claudine/` `just lint`:** passed. **`darkmatter/` `just lint`:** passed.
- **`just check-tier-coverage claudine`** and **`darkmatter`:** 0 stranded.
- **Workspace compile:** `cargo check --workspace` clean. The other
  darkmatter dependents (`research`, `messenger`, the `*-cli` crates) do not
  compose documents, so no further dependent tests were selected.
- **Failures found and fixed during the phase:** the stack overflow
  (P2-D8), Claudine's `type_scope_ownership_guard` (its declared-layer
  matcher now recognizes `DeclaredTypes::for_schema` as well as `new`, and
  the lifecycle site has a reviewed `Unguarded` entry), and
  `test_placement::repository_test_placement` (inline tests in
  `lifecycle_authoring.rs` exceeded 300 lines; the new helper was compacted).
  No load-sensitive or pre-existing failures.
- **Cross-OS:** not run. The changes are pure type computation over
  in-memory schema values plus temp-dir fixtures that follow existing
  portable patterns, with no path comparison or OS-specific code.
