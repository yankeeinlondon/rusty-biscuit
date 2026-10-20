---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/fixes/2026-10-16-type-narrowing/spec.md"
plan: "claudine/fixes/2026-10-16-type-narrowing/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - claudine
human_review: false
message_to_agent: |-
    Phase 3 changed the shared template API only; every caller still passes `TypeScope::unguarded`. Read "## Phase 3" in this log, especially "Departures and decisions". Phase 2's notes (a) `x != ''` proves presence and (b) `DeclaredTypes` does not model `with_document_fallbacks` or the origin-kind filter still apply.

    What Phase 4 must thread:
    - `CandidateScope::build(.., types: &TypeScope, evaluate)` and `CommandTemplate::project_types(&TypeScope)` take the site's scope. Darkmatter builds it in `preflight/collect.rs::DiscoverySiteContext::build` (discovery) and `shell_expansion/site.rs::SiteScope::site` (runtime). Both must get the SAME guards for a site: the approval shape follows a guard's refinement, so a runtime site typed without the discovery guard will not match its approval.
    - Claudine builds unguarded scopes in `lifecycle/shell_template.rs::typed_template` and `sequence/preflight/mod.rs::prepare_site`. Its own null checks (`composition/preflight.rs::resolve_shell_command_expr`, `sequence/preflight/mod.rs::resolve_approval_time`) do not consult a scope yet. Retime a guard-proven missing hole there to `HoleTiming::Guarded { guard, roots: vec![] }` (use `CommandTemplate::placeholders_mut`), and use `HoleTiming::is_late()` where "not read at approval" is meant.
    - Barrier semantics: `resolve_late_values` sweeps only `Deferred` and never `Guarded`. Claudine's `PreparedShellSite::reads_late_value` and the `sequence_lifecycle_template` flip still match `Deferred` only; that is intended for barriers. A `Guarded` hole is read by `fill_template` at the reached action.
    - Label guards with `assume_at(.., GuardOrigin::new("<authored path>.when"))`; plain `assume` labels with `Expr::to_string()` (`and(a, b)`).
    - `fill_template` now reads each distinct expression once per call. A `RejectReason::GuardRefinement` refusal is a `TemplateValueRejected` and must stay unsuppressible like the other value refusals.
    - `darkmatter/docs/inline/preflight-checks.md` has a "Guarded values (planned)" subsection. Remove the planned marker when the sites are wired.
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/expressions.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/tests/l1/shell_template_guards.rs
    - darkmatter/lib/tests/l1/shell_template_execution.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/lib/tests/l1/main.rs
    - claudine/lib/src/composition/lifecycle/shell_template.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/preflight.rs
docs_updated_during_phase_3:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/expressions.md
---

# Implementation Log for 2026-10-16-type-narrowing (6 phases)

## Phase 1

Phase 1 is inventory and contract work. It changed no production code, test,
or fixture. All paths below are relative to the repository root. Line numbers
are as of this phase's checkout (HEAD `563326f84`); later phases should search
by symbol name rather than trust them.

### Baseline ownership (plan rule 1)

- The `2026-10-27-darkmatter-lifecycle-ownership` extraction has landed only
  its Phase 1: test-only traces and prototype coverage (`12509a77e`,
  `240eb04dc`, `563326f84`). `darkmatter/lib/src/lifecycle/` does not exist.
  **The lifecycle preflight walk is still Claudine's**
  (`claudine/lib/src/composition/preflight.rs`), so this fix builds against
  that owner. That spec's log already records that type narrowing is not
  built yet.
- The `2026-10-23-shell-template-approval` prerequisite is implemented
  (`implemented: true`, in `_completed`). Its APIs exist:
  - `CommandTemplate`, `HoleTiming`, `project_types`, `FilledChain`,
    `parse_authored`
  - `ShellPreApprovals`, `DeferredShellRoots`, `ShellCandidate`,
    `CandidateScope::build`
  - `project_hole`, `rule_covers`
  - `fill_template`, `resolve_late_values`, `LateValues`,
    `DocumentShellValues`
  - Claudine's `harness/shell.rs::{validate_and_approve_template_site,
    template_rule_covers, prompt_resolved_site}`
- No `TypeScope` exists anywhere yet.

### Decision-site inventory (Wave 1 · Map decision sites)

**Static presence and type decisions to replace.** These are the five sites
in the spec's table. Each is a "competing classifier" that the guard test
(Phase 5) must name.

| # | Site | Current decision | Guards honored |
|---|---|---|---|
| 1 | `darkmatter/lib/src/markdown/compose/expression/absence.rs` `AbsenceScope` (`pub(crate)`, `condition_guard`, `RootGuard`) and `static_variable_reads` / `walk_static` | absence handling (warning policy); its branch rule is root-only, and only a bare `Variable` / `Paren(Variable)` ternary condition becomes a guard | ternary with a bare variable only. `&&` / `and()`, `!`, comparisons and `file_exists` give nothing |
| 2 | `darkmatter/lib/src/markdown/compose/directive_targets.rs` `narrowed_expression_paths` → `collect_narrowed_paths`, and `classify_target_nullability` | positive facts from `x`, `!!x`, `x != null`, `x != ""`, `file_exists(x)` (case-insensitive, **no alias handling**), and `and(...)` | body blocks only (via `analyze_directive_targets`, which DMLS uses alone) |
| 3 | `darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs` `CandidateScope::build` (≈:345) | `evaluate(&expr) == Some(Value::Null)` → `ShellExpansionError::MissingValue`; `None` means unknown and defers | none |
| 4 | `claudine/lib/src/composition/preflight.rs` `resolve_shell_command_expr` (≈:845/930); also `sequence_lifecycle_template` (≈:1001/1079) through `resolve_late_values` | `read.is_null()` → `MissingValue`, wrapped as `LifecycleShellResolution` | none |
| 5 | `claudine/lib/src/composition/sequence/preflight/mod.rs` `Loader::prepare_site` (≈:1520) → `resolve_approval_time` (≈:1750, null check ≈:1795) | `read.is_null()` → `MissingValue`, wrapped as `SequenceShellResolution` | none |

**Discrepancy found:** walkers 1 and 2 disagree on `&&`. The parser lowers
`a && b` to `FunctionCall "and"` (`expression/parser.rs::parse_logical_and`).
`directive_targets` handles that form; `AbsenceScope` does not. `||` with a
truthy left operand parses to `Fallback`, and `and`/`or` are lazy bindings in
the registry. `TypeScope::assume` must therefore treat `FunctionCall
{name: and|or}` (matched through the registry, case-insensitively) and
`Fallback` as the logical operators. Do not add a new AST node.

**Runtime value checks.** These are legitimate owners. The guard test must
permit them.

- `template/fill.rs::resolve_hole` (≈:337): Null → `HoleError::Missing`;
  container → `NotScalar`; NUL → `EmbeddedNul`; `!validator.accepts` →
  `DeclaredType`. `HoleError::into_shell_error` maps these to
  `MissingValue` / `TemplateValueRejected` / `ExpressionEvaluation`.
- `functions/mod.rs::is_null` and `is_empty_fn`, `file_exists_fn`
  (≈:1638; a null argument gives `false`, a non-string errors, a URL fetches),
  and the evaluator's `observe_missing`.
- `compose/directive_targets.rs::rewrite_directive_targets`: the runtime
  null-target skip and warning. It uses no static facts.

**Structural relation.** `markdown/schemas/simplified/subtype.rs::is_subtype(&PropertyAtom,
&PropertyAtom) -> bool`. Its only caller is
`shell_expansion/template/matching.rs::rule_covers`. It ignores presence and
constraints.

**Declared projection.** `template/typing.rs::project_hole(expression,
Option<&SimplifiedSchema>, host_globals) -> HoleType`:

- It types only a bare `x` or `fm.x`.
- A root union counts as a declaration only when all its arms agree.
- Everything else is `HoleType::fallback` (`BroadString`, `FallbackScalar`).
- It reads `EffectiveSchema.simplified` (authored schema only). Presence comes
  from `PropertyDef::is_required()` / `admits_null()`
  (`schemas/simplified/types.rs`), which is the documented single presence
  calculation.

**Descriptor authority.** `compose/context/catalog.rs::context_variable_descriptors()`
(`required`, `default`, `generated`). Today only `classify_target_nullability`
consumes it.

**Binding model.**

- `expression/binding.rs`: `ResolvedBinding`, `RootClass`, `Availability`,
  `BindingView::classify_root`, `unavailable_read` →
  `BindingError::Unavailable`.
- Reserved namespaces are `fm`, `ctx`, `env` and `current`.
- `seq`, `loop`, `group`, `err` and `timing` are Claudine host globals,
  declared in `claudine/lib/src/composition/lifecycle/bindings.rs::binding_view_in`.
- Bare versus `fm.` resolution is `EffectiveState::get`, and
  `fm_namespace.rs::strip_fm_prefix`.

**Construction paths and callers.**

- Darkmatter document discovery:
  - `compose/preflight/collect.rs::collect_shell_commands[_with_graph]` and
    `collect_frontmatter_shell_commands`, which feed `collect_recursive` and
    then `DiscoverySiteContext::build` (≈:1346) and `CandidateScope::build`.
  - `shell_expansion/site.rs` (≈:490) calls `CandidateScope::build` with
    `&|_| None`.
  - Discovery is **condition-blind** and never evaluates page blocks.
  - The body `::block when=` text exists only in `page_blocks` regions
    (`page_blocks/types.rs` `options.when_expr`;
    `parse_page_blocks[_in]`). At runtime, `page_blocks/engine.rs::render_body`
    deletes false blocks before Interpolation and ShellExpansion run, so
    **body scopes must be built at discovery from `scan_darkmatter_blocks` /
    `parse_page_blocks` body spans**, the same way `analyze_directive_targets`
    builds them.
- Claudine lifecycle preflight:
  - `resolve_lifecycle_shell_commands_in_sequence` (`pub(crate)`, ≈:629)
    walks every signal with the flattened `lifecycle/actions.rs::for_each_action_mut`.
    That walk recurses `then` then `otherwise` and passes no guard.
  - Inline guarded actions are already parsed into a `LifecycleConditional`
    with an empty `otherwise` (`lifecycle/parse.rs` ≈:528).
  - Callers are `prepare.rs::effective_surface` (from `prepare_direct_on`,
    `prepare_inline_on` and `preflight_bootstrap_shell`) and
    `resolve_graph_shell_approvals`.
  - The secondary, read-only flattened sites are `lifecycle_shell_candidates`,
    `for_each_lifecycle_shell_site`, `settle_lifecycle_shell_modes` and
    `collect_lifecycle_shell_commands[_for]`.
- Claudine sequence preflight:
  - `shell:` → `resolve_shell_commands`, then `prepare_site`.
  - `with:` → `record_prompt_reference`, which keeps late keys in
    `deferred_with`.
  - Task `when:` is stored as an unparsed `Option<String>` (`load_task`).
    Preflight never parses it today.
  - A container's stack shells go through
    `shape::collect_stack_shell_sites`, a raw-JSON, condition-blind walk.
- Late-value caches and barriers:
  - `PrepareOptions.{deferred_shell_roots, validate_deferred_shell_values,
    document_shell_values}`
  - `prepare.rs::validate_document_shell_late_values`
  - `sequence/preflight::resolve_task_late_values`, called from
    `TaskExecution::late_values` inside `validate_task`, which is the
    task-start barrier
- Runtime dispatch:
  - The lifecycle guard is `lifecycle/executor.rs::execute_nodes` →
    `when_matches`.
  - A shell site runs `run_shell_action` → `run_shell_template`. That calls
    `fill_template`, then `check_filled_chain`, then `run_structured`.
  - Set values run `run_set_shell_values` → `ShellRunner::run_values`.
  - A sequence task runs `task/mod.rs::run_shell` →
    `task/templates.rs::prepare_primary_template` → `run_structured`. Its
    guard is `evaluate_task_guard` → `StackExecutionContext::evaluate_task_guard`.
- DMLS:
  - `dmls/src/overlay/expressions.rs::undeclared_property_findings` uses the
    library's `static_variable_reads`, keeps `!handles_absence` reads, and
    flags both branches.
  - Hover is `hover_markdown_from`, which is not guard-aware.
  - `providers/dsl.rs::transclusion_diagnostics` uses `analyze_directive_targets`
    and `is_narrowed`.
  - `diagnostics/nested_span.rs::lifecycle::is_predicate` classifies
    predicates by path shape only.
  - DMLS reimplements no guard algebra. It must keep calling the library.
- Other `static_variable_reads` consumers that are **not** presence
  decisions: `expression/prepared.rs::validate_expression`, Claudine
  `lifecycle/bindings.rs::approval_diagnostics` (late-root refusal), and
  `lifecycle/prewarm/classify.rs` (readiness). Leave them as they are.

### Scope contract (Wave 1 · Define scope contract)

This is the proposed API. Later phases may refine the names, but any change
to the semantics is recorded here as a departure.

- **Module.** `darkmatter/lib/src/markdown/compose/expression/scope/`
  (`mod.rs` holds `TypeScope`; `facts.rs` holds `assume`, joins and
  invalidation; `projection.rs` holds the shared declared layer). It is
  re-exported from `compose::expression`, beside `static_variable_reads`.
- **Declared layer.** `DeclaredTypes<'s>` is built once per document/owner
  from `Option<&SimplifiedSchema>` (`EffectiveSchema.simplified`), the static
  frontmatter, `context_variable_descriptors()`, and the site's
  `BindingView` / host globals.
  - `project_hole`'s bare/`fm` lookup and `classify_target_nullability`'s
    descriptor and concrete-value rules move behind it.
  - `project_hole` keeps its signature and delegates, so `rule_covers` and
    grammar callers are unchanged.
  - Effective schemas are never rebuilt per hole.
- **`TypeScope<'d>`.** It is immutable, `Clone`, and has **no `Default`**.
  - `TypeScope::unguarded(&'d DeclaredTypes)`
  - `scope.assume(&Expr, Polarity) -> TypeScope`, the only fact producer
  - `scope.type_of(&Subject) -> ScopedType`
  - `scope.invalidate(&WriteSet) -> TypeScope`
  - `TypeScope::join(&a, &b) -> TypeScope`
  - Facts live in a persistent parent-sharing chain (`Rc` link), so a nested
    region costs one node.
- **Subjects.** `Subject { binding: BindingId, path: Vec<Member> }`, where
  `Member` is `Field(String) | Index(usize)` and contains constants only.
  - `BindingId` comes from `BindingView::classify_root`: document property,
    reserved namespace, or host global with its `Availability`.
  - `x` and `fm.x` map to the same `BindingId::Document("x")` only when no
    host global shadows `x`.
  - A dynamic index or a function result yields no subject, so the scope
    learns nothing from it.
- **`ScopedType`.** It holds the declared `Option<PropertyDef>` (or unknown),
  a refinement (union arms, literals, family set), `non_null: bool` (the
  internal unknown-minus-null fact, which is not a schema type), and the
  provenance (the `Expr` span of the guard and the owner/source). The adapter
  methods are:
  - `proves_present()`
  - `approval_type()`, which is the narrowed atom with the broad-string
    fallback applied **after** the lookup
  - `validators()`, which returns both the original declaration and the
    refinement
- **Polarity.** `Polarity::{True, False}`. `!` swaps it. `and`/`or`/`Fallback`
  and the ternary follow the spec's §2 rules. Predicate facts are looked up
  through the canonical registry (`functions::{BINDING_GROUPS,
  ABSENCE_PREDICATES}` aliases, case-insensitive like `is_absence_predicate`)
  via a small fact table keyed by canonical name. **No second registry**: the
  table is validated against `joined_bindings()` in a unit test.
- **Equality.** `scalar_string` and the both-null rule are reused from
  `expression/mod.rs`. A numeric literal narrows only when formatting it is
  unique.
- **Writes.** `WriteSet::{Static(Vec<Subject>), AllDocument}`. A static write
  invalidates the destination and every ancestor and descendant, through the
  canonical alias. A dynamic or unknown write invalidates all document facts.
  A join keeps possible writes.
- **Approval adapter (the contract shared by Phases 3 and 4).**
  - `CandidateScope` gains a required `scope: &TypeScope` field.
    `CandidateScope::build` reads `scope.type_of` for each hole.
  - When the approval-time value is null and the hole's subject
    `proves_present()` from a guard fact (not from the declaration), the hole
    becomes `HoleTiming::Deferred` with a new
    `DeferReason::Guarded { site, expression }` beside today's root-based
    reason. It is `TemplateRequired`.
  - **Ownership:** `candidate.rs` owns `HoleTiming`/`DeferReason`
    (placeholder timing definitions). `fill.rs` consumes them.
  - `resolve_late_values` skips `Guarded` holes. `fill_template` validates
    `Guarded` holes and refined holes at the reached action, against both the
    original declaration and the refinement.
  - `DeferredShellRoots` is unchanged and gains no document keys.
- **Runtime errors.** The public error enums (`ShellExpansionError`,
  `HoleError`, `CompositionError`) keep their current variants.

### Acceptance-to-test map (Wave 1 · Map acceptance evidence)

| AC | Owner (phase) | Public result asserted | Location |
|---|---|---|---|
| 1 | P5 invocation | the frozen prompt with `commit_message` absent passes preflight after template approval; a false guard launches nothing; `'msg'` with dirty files gives exactly one argv entry `msg` | `claudine/cli/tests/l1/` new file, using `claudine-argv-recorder` copied in as `git` (portable; pattern in `lifecycle_shell_templates.rs`) |
| 2 | P3/P5 | the unguarded command fails with `MissingValue` | extend `claudine/cli/tests/l1/shell_missing_values.rs` |
| 3 | P3 fill + P5 CLI | a `set` to null before the branch fails that action with `MissingValue`; nothing launches; earlier actions keep their effects | Claudine lib `lifecycle/executor/tests/shell_templates.rs` and CLI L1 |
| 4 | P5 matrix | guard and equality cases at every applicable site | `darkmatter/lib/tests/l1/` matrix plus Claudine lib preflight tests |
| 5 | P2 subtype | the four `is_subtype` examples | `darkmatter/lib/tests/l1/` (beside `schemas_core_type_values.rs`) or `subtype.rs` unit tests |
| 6 | P3 candidate | `is_string(target)` gives a `{{ string }}` shape | `darkmatter/lib/tests/l1/shell_template_discovery.rs` |
| 7 | P4/P5 DMLS | DMLS type equals library type per supported site | `darkmatter/dmls/tests/l1/` |
| 8 | P5 guard | a forbidden snippet and an unallowlisted `unguarded` call are rejected; `is_null` at runtime is permitted | new guard in `claudine/lib/tests/l1/` (models: `lifecycle_lift_ready_guard.rs`, `messaging_spawn_guard.rs`) plus its Darkmatter counterpart |
| 9 | P3/P5 | `when: x` does not defer `y` or an unguarded `x`; a non-null declaration with an actual null fails | Darkmatter template tests and CLI `shell_missing_values.rs` |
| 10 | P4/P5 | a false guarded task does not fail at the task-start barrier; an ordinary late value still does | `claudine/cli/tests/l1/sequence_shell_templates.rs` / `task_guards.rs` |
| 11 | P2 algebra | differential cases against the real evaluator (`evaluate`) | `darkmatter/lib/tests/l1/` new scope test |
| 12 | P2 invalidation + P3/P5 | alias and nested-join invalidation; a present value whose type changes before dispatch is a template-value refusal | Darkmatter scope tests; Claudine executor tests |
| 13 | P2/P5 | `assume(file_exists(..))` uses no handler and does no I/O; an unavailable global keeps `BindingError::Unavailable` | Darkmatter scope tests, plus an `effects-instrumentation` assertion where available |
| 14 | P6 | the existing regressions still pass | existing approval, blacklist and recovery tests (named in P5) |

**Site enum for the Phase 5 matrix** (`ScopeSite`, test-only, exhaustive):

| Variant | Guard sources that apply |
|---|---|
| `BodyShell` | body `::block when=` (true side; nested) and expression operators |
| `BodyShellBlock` | body `::block when=` (true side; nested) and expression operators |
| `TransclusionTarget` | body `::block when=` (true side; nested) and expression operators |
| `FrontmatterShell` | expression operators only; **inapplicable**: lifecycle and body guards |
| `LifecycleShell` | `when`/`then`/`else`, five levels deep |
| `LifecycleInlineGuard` | `when`/`then`/`else`, five levels deep |
| `LifecycleSetShell` | `when`/`then`/`else`, five levels deep, plus `set` invalidation |
| `SequenceTaskShell` | task `when:` using pre-`with` bindings |
| `SequenceTaskWith` | task `when:` using pre-`with` bindings |

**Planned, not claimed:** nested-union and discriminant projection,
function-result narrowing, host types for `seq`/`loop`/`group`, argument
binding (`2026-10-28`), and conversion-aware `file_exists`.

**Fixtures.** No new fixture is needed. The motivating prompt is already
frozen at `claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md`
(the guard is at :83 and `git commit -m "{{commit_message}}"` at :90), and
`common/prompt_staging.rs::stage_frozen_prompts` stages it. No test reads
`prompts/` live.

**Finding for Phase 5 (AC 1).** Both the shipped and the frozen prompt now
declare a top-level default `commit_message: ""`, added in `b47947b1c` on
2026-10-15. An empty string is present (`shell_missing_values.rs::an_empty_string_value_is_present_and_runs`),
so the motivating failure does **not** reproduce with the prompt as shipped.
The AC 1 test must stage the frozen prompt and make one edit: remove that
default line (or set it to `null`). That is the control-plus-one-edit cell.
The default looks like a workaround for this bug; whether to remove it from
the shipped prompt once this lands is the author's call, and it is not part
of this fix.

### Input Robustness Matrix applicability (plan rule 8)

Not applicable. Phase 1 changes no loader, deserializer, or field validator.
The planned scope analyzes typed schema projections and expression and
lifecycle regions. No exception was found that requires a loader change.

### Gates

- Phase 1 changed no file that a package reads. The only changed files are
  this log and `plan.md` (checkboxes and frontmatter).
- I ran the CI scope calculator after all edits:

  ```sh
  git status --porcelain --untracked-files=all | awk '{print $NF}' \
    | xargs python3 scripts/ci/affected_scope.py
  ```

  Result: `packages: ["darkmatter"]`, `change_class: "package"`. The
  selection reason is "changed test input read by package(s) darkmatter".
  `darkmatter/lib/tests/l1/shipped_schema_corpus.rs` reads every spec, plan
  and log frontmatter under `claudine/fixes` and `claudine/features`. Every
  changed file triggers it, including the pre-existing uncommitted
  `2026-10-27-darkmatter-lifecycle-ownership/implementation-log.md` and
  `spec.md` edits, which this phase did not make.
- Because `packages` is non-empty, the gates were **not** skipped:
  - `cargo nextest run -p darkmatter --test l1 -E 'test(shipped_schema_corpus)'`:
    8 passed.
  - `just test` in `claudine/`: 10479 passed, 9 skipped, exit 0 (226 s).
  - `just lint` in `claudine/`: exit 0.
- There were no pre-existing or load-sensitive failures.
- `just check-tier-coverage` was not run. Phase 1 adds no tests, so tier
  placement cannot have changed.
- No cross-OS check was run, because nothing that compiles changed.
- The `claudine` skill needs no update. No architecture or workflow changed;
  the skill gets its update when the scope lands (Phase 6).

## Phase 2

Phase 2 built the shared analysis core in Darkmatter. No consumer reads the
scope yet (that is Phases 3 and 4). The one behavior a user can observe is the
widened structural relation in shell-rule coverage. One agent did all four
tasks serially: they share the new module, and the plan makes Wave 3 serial.

### What landed

**Subtype relation** (`darkmatter/lib/src/markdown/schemas/simplified/subtype.rs`)

- `is_subtype` now also accepts:
  - a `literal` in its own scalar family (`string`, `number`, or `boolean`)
  - a string `literal(v)` in an `enum` that lists `v`
  - an `enum` that is a subset of another `enum`
  - a non-array union on either side, compared arm by arm and independent of
    arm order (every source arm must be covered; one destination arm must
    cover a non-union source)
- What it still refuses: `literal(4)` ⊆ `enum('4')`, `unknown` as proof of
  anything, and array promotion (`alphanumeric[]` ⊆ `string[]`). Arrays
  still compare by identity.
- The new `is_definition_subtype(&PropertyDef, &PropertyDef)` lifts the
  relation to whole definitions through `flattened_alternatives`.
- **Observable effect:** `rule_covers` now lets a saved `{{ enum(a,b,c) }}`
  rule cover a hole declared `enum(a, b)` or `literal('a')`. This is sound,
  because every value the declaration admits passes the rule. The saved rule
  grammar is unchanged, and non-string holes still need `{{ string }}`
  (`template/tests.rs::rule_coverage_follows_the_structural_relation_for_string_like_holes`).

**Type scope** (new `darkmatter/lib/src/markdown/compose/expression/scope/`,
re-exported from `compose::expression`)

- `projection.rs`:
  - `DeclaredTypes<'s>` is the shared declared layer, built once per owner
    from `Option<&SimplifiedSchema>` (`EffectiveSchema.simplified`).
    `.with_static_frontmatter(&Value)` adds the static values, and
    `.with_host_globals(&[&str])` or `.with_binding_view(&BindingView)` adds
    the host roots.
  - `Subject` and `BindingId` (`Document` / `Namespace` / `Global`), plus
    `Member` (`Field` / `Index`). `x`, `fm.x` and `fm['x']` share
    `Document("x")` unless a host root shadows bare `x`.
  - Function results, computed indexes, bare `fm`, `current.*` (lazily
    refreshed) and the `null` spelling have no subject.
  - `DeclaredType` is one of `Property` / `Undeclared` / `NoProjection` /
    `NestedProperty` / `Context{present}` / `Namespace` /
    `Host(Availability)`.
  - `classify_variable` and `declaration` (the root-union agreement rule) are
    the single lookup. **`template::project_hole` now delegates to it**; its
    signature and behavior are unchanged (all existing template and
    discovery tests pass).
- `shape.rs`:
  - `Shape` is either `Unknown { non_null }` or `Arms { arms, nullable }`.
    Optional means nullable, by `PropertyDef::is_required`, which is the
    shared presence rule; no new presence calculation was written.
  - `ScopedType` exposes `declared()`, `shape()`, `proves_present()`,
    `guard_presence()`, `presence_evidence()`, `refinements()`,
    `was_written()`, `admits(&Value)` and `narrowed_definition()`.
  - `PresenceEvidence` is one of `Declaration` / `Descriptor` /
    `StaticValue` / `Guard(GuardOrigin)`.
- `facts.rs`:
  - The guard algebra behind `TypeScope::assume`, which is the only fact
    producer.
  - `PREDICATE_FACTS` is keyed by canonical registry name. A unit test checks
    that every entry is a registered one-argument binding.
  - Aliases and case resolve through the new
    `functions::canonical_function_name` (with `accepts_call_arity`), which
    matches exactly as `evaluate_function` does.
- `mod.rs`:
  - `TypeScope<'d>` is immutable, `Clone`, and has **no `Default`**.
    `TypeScope::unguarded`, `assume`, `assume_at(.., GuardOrigin)`,
    `type_of`, `type_of_expr`, `invalidate(&WriteSet)` and
    `TypeScope::join`.
  - Facts are a persistent `Arc` chain, so a child shares its parent.
  - `WriteSet` is `Paths(Vec<Subject>)` or `AllDocument`.
    `WriteSet::destination(path)` gives `Paths` for a static (optionally
    `fm.`-prefixed) dotted destination and `AllDocument` for anything
    interpolated or computed.

**Passive by construction.** No scope path calls `evaluate`, a function
handler, or I/O. `admits` uses the schema validator only; it is for runtime
refinement checks and is not used during scope construction.

### Rules as implemented

- Truthiness. The true side removes null and the always-falsy literals; the
  false side learns nothing. `!` swaps sides and parentheses are transparent.
- `and(…)`, `&&`, `or(…)`, `||` (`Fallback`) and ternaries follow the spec's
  section 2 route rules through one `short_circuit` helper. Each operand is
  checked under the earlier operands' continuing facts, and the
  short-circuit routes are joined.
- `is_string` / `is_number` / `is_array` / `is_object` narrow both sides,
  through the alternatives' runtime families. From unknown they yield the
  bare family type (`string`, `number`, `object`, `unknown[]`).
- `is_null` is true-iff. The false side of `is_empty` proves presence, and
  its true side learns nothing.
- The true side of `file_exists` is a present string, and its false side is
  nothing. That is today's strict handler; the conversion question in the
  spec's Open Question is still future work.
- `==` / `!=` use `scalar_string` text and the both-null rule:
  - A literal operand narrows on either side, including reversed operands.
  - Another subject narrows only when its values are a finite literal set.
    The unequal side narrows only against a single present value.
  - Container operands narrow nothing.
- Numbers narrow to a literal only when exactly one number formats as the
  text: non-integral values, or integers below 2^53. A text no number
  formats as (`'4.0'`, `'+4'`, `'-0'`) excludes the numeric alternatives.
- Constrained types (`email`, `number(min(1))`) keep their alternative and
  are not rewritten to a literal.
- A contradiction (no value survives) keeps the incoming type and records no
  presence evidence.
- Joins and writes:
  - A join keeps what both routes know. Guard evidence survives only when
    both routes have it.
  - Every write on either route is replayed after the shared ancestor, so a
    possible write inside an inner branch still invalidates after an outer
    join.
  - A branch's own facts end with it.
  - A write removes static-value evidence as well.

### Departures from the specification (recorded per its instruction)

1. **The false side of `x == ''` (and the true side of `x != ''`) excludes
   null.**
   - What the spec table says: "`string` without a presence proof".
   - What the evaluator does: `null == ''` is **true**, because null
     compares as empty text and both-null does not apply. So `x != ''` is
     exactly `x != null`.
   - Why this side: plan rule 5 makes the evaluator the authority, and
     `directive_targets::narrowed_expression_paths` already treats
     `x != ""` as proving presence. Phase 4 must not regress that when it
     replaces the walker.
   - Evidence: the mutation check below and the `optional_strings`
     differential row.
2. **The false side of `!is_empty(x)` proves presence**, although the spec
   table does not list it. It is handler-backed: `is_empty(null)` is true.
3. **A comparison against unknown with a nonempty literal records only
   presence.** `x == 'a'` on unknown gives `unknown(present)`, with no
   family. The spec forbids inferring a family; non-null is the internal
   presence fact it allows.
4. **Discriminant narrowing of a parent object is not implemented.** A guard
   on `x.kind` narrows the path `x.kind` only. This is planned with nested
   union projection, as the spec allows.
5. **`DeclaredTypes` does not apply `classify_target_nullability`'s
   origin-kind filter** (`Document | ReferencedFile`), because
   `EffectiveSchema.simplified` is already the authored layer only.
   **`with_binding_view` does not model `HostGlobals::with_document_fallbacks`.**
   Phase 4 should confirm both when it moves the directive-target and
   Claudine consumers onto the layer.

### Tests (requirement → evidence)

| Requirement | Test |
|---|---|
| AC 5 (the four structural examples) | `darkmatter/lib/tests/l1/schemas_core_type_values.rs::narrowed_literals_and_unions_satisfy_their_parameter_types_structurally`; unit rows in `subtype.rs` (`literals_are_subtypes_of_their_scalar_family_only`, `string_literals_are_subtypes_of_enums_that_list_them`, `enum_subsets_are_subtypes`, `unions_compare_arm_by_arm_independent_of_order`, `unknown_is_never_proof_of_a_concrete_type`, `definitions_compare_every_leaf_alternative`) |
| Shell-rule compatibility | `shell_expansion/template/tests.rs::rule_coverage_follows_the_structural_relation_for_string_like_holes`; all existing `template::tests` and `shell_template_discovery` tests |
| Scope model (optional vs required, shadowing, bare/`fm` identity, constant paths, unknown projections, shared parents, descriptors and static values, unavailable globals) | `darkmatter/lib/tests/l1/expression_type_scope.rs`, the "declared layer" section (6 tests) |
| AC 6 at the core, and the spec's narrowing example | `type_predicates_narrow_a_declared_union_on_both_sides` |
| AC 11 (null vs empty, both-null, container/string, reversed operands, predicate aliases, optional declarations, both-side unions, numeric formatting) | `equality_narrows_to_literals_of_every_scalar_family` (every spec table row), `equality_keeps_constraints_and_numeric_families_it_cannot_pin`, `equality_against_unknown_or_a_container_proves_no_scalar_family`, `equality_between_two_subjects_narrows_against_a_finite_other_side`, `two_nulls_are_neither_equal_nor_unequal`, and the **differential** module `every_guard_side_admits_every_value_the_evaluator_sends_there::*` (10 declarations × 39 guards × 19² sample values through the real `evaluate`; `every_differential_declaration_has_a_test` guards the macro list) |
| AC 12 at the core (alias/nested-join invalidation) | `a_write_invalidates_its_aliases_ancestors_and_descendants_only`, `a_join_restores_only_incoming_facts_no_branch_could_have_written`, `a_join_keeps_what_both_branches_learned_and_drops_one_sided_facts`, `a_static_destination_writes_its_path_and_any_other_destination_writes_everything` |
| AC 13 at the core | `file_exists_proves_a_present_string_without_touching_the_filesystem` (the static value is an unreachable URL: the fact holds because no handler runs), `an_unavailable_global_stays_unavailable_under_any_guard` |
| The registry is not duplicated | `expression/scope/tests.rs` (3 unit tests) |
| Motivating guard | `a_conjunction_proves_each_operand_on_its_true_side_only` and the `scope` module doctest |

- **Fidelity:** F1 (in-process, public crate API), tier L1. No CLI,
  filesystem, or persistence boundary is crossed: the core is pure analysis.
  The CLI and invocation evidence for AC 1–3, 9 and 10 belongs to Phases 3
  through 5.
- **Mutation check:** making `x == v` drop null for `v == ''` failed the
  differential rows (`x == '' is True for x=null … excludes null`). The
  change was reverted.
- **Speed:** the differential test took about 1.0 s as one test, so it was
  split per declaration with `macro_rules!` (slowest row 0.4 s).
- **Input Robustness Matrix:** not applicable. No loader, deserializer or
  field validator changed (plan rule 8).

### Docs and skills

- `darkmatter/docs/topics/schemas/definition.md` documents the widened
  relation, the new `is_definition_subtype` API, and the rule-coverage
  consequence.
- `.claude/skills/darkmatter/schema.md` has an updated `is_subtype` bullet.
- `.claude/skills/darkmatter/expressions.md` has a new "Type scope"
  paragraph.
- The `claudine` skill is unchanged, because Claudine code did not change.

### A partial commit was made during this phase

- While this phase was running, commit `3c05cfe60` ("feat(darkmatter): add
  structural expression type narrowing", authored as Ken Snyder) recorded 9
  of this phase's files exactly as they were on disk:
  - `subtype.rs`, `schemas/mod.rs` and `simplified/mod.rs`
  - `functions/mod.rs`
  - `scope/facts.rs`, `scope/projection.rs` and `scope/shape.rs`
  - `template/tests.rs`
  - `tests/l1/schemas_core_type_values.rs`
- This agent did not commit or stage anything.
- The rest of the phase is still uncommitted: `scope/mod.rs`,
  `scope/tests.rs`, `expression/mod.rs` (the `mod scope` wiring),
  `typing.rs`, the new L1 file and `tests/l1/main.rs`, plus the docs and
  skills.
- **That commit is not self-contained.** It adds scope files that no `mod`
  declaration compiles yet, and two `pub(crate)` registry helpers that
  nothing calls yet, so it alone would likely fail `-D warnings` lint. The
  next commit of this phase's uncommitted files completes it.
- Every gate below ran on the full working tree, which contains both parts.

### Gates

- Scope calculator. The working tree alone is no longer the phase's change
  set, so I ran it over the working tree **plus** the files of `3c05cfe60`:

  ```sh
  { git status --porcelain --untracked-files=all | awk '{print $NF}'; \
    git show --name-only --format= 3c05cfe60; } | sort -u \
    | xargs python3 scripts/ci/affected_scope.py
  ```

  Result: `packages: ["darkmatter", "test-toolkit"]`,
  `change_class: "package"`. `test-toolkit` has a lint row only, for its
  `archive-path-guard` companion suite. Because `packages` is non-empty,
  no gate was skipped.
- `just lint` (darkmatter): exit 0. It first failed on
  `clippy::large_enum_variant` for the scope's `Step::Fact`; the fix was to
  box the `ScopedType`.
- `just test` (darkmatter), first run: 10552 passed and 1 failed with
  fail-fast. The failure was
  `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  (a watcher registration count, 3 vs 2). It **passed 3/3 when run alone**,
  so it is recorded as **load-sensitive and unrelated**: DMLS does not reach
  the new code. Rerun of the whole suite: **10672 passed**, 10 skipped,
  exit 0.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Doctests for the touched items (`is_subtype`, `is_definition_subtype`,
  `is_string_like`, `HoleType::display_type`, the `scope` module): 5 passed.
- Dependent package `claudine`, the only consumer of `rule_covers` and
  `project_hole`:
  - `just test`: **10479 passed**, 9 skipped, exit 0.
  - `just lint`: exit 0.
- `test-toolkit`: `just archive-path-guard` passed (2 tests).
- No other package glob-imports `compose::expression` or `schemas`, so the
  new public names cannot collide.
- Cross-OS check: **not run**. The change is pure in-memory analysis with no
  paths, processes, terminals or `cfg` code, so it has no notable OS risk.
  CI's Linux, macOS and nightly Windows/WSL2 cells cover it.

## Phase 3

Phase 3 changed the shared shell-template API in Darkmatter so that it can
carry guard evidence. No consumer passes a guarded scope yet: every caller
passes `TypeScope::unguarded`, so composition and Claudine behave as before,
apart from the one-read-per-expression change below. Phase 4 threads the real
scopes. One agent did both tasks serially, because they share `HoleTiming`,
`HoleType`, and the template tests.

### What landed

**Scope shell candidates** (`template/typing.rs`, `template/mod.rs`,
`template/candidate.rs`, `template/grammar.rs`)

- `project_hole(expression, &TypeScope)` and
  `CommandTemplate::project_types(&TypeScope)` now require a scope. The old
  `(schema, host_globals)` arguments live in the scope's `DeclaredTypes`.
- `HoleType` gains `refinement: Option<Box<HoleRefinement>>`
  (`definition` + `guards: Vec<GuardOrigin>`). It is set when the scope's
  `ScopedType::is_refined()` and `narrowed_definition()` gives a type. The
  approval type follows the refinement: `ApprovalType::for_declaration` of the
  narrowed definition. The broad-string fallback still applies afterwards, so
  a non-string narrowing approves as `{{ string }}`. The original validator
  (`HoleValidator`) and provenance (`TypeProvenance`) are unchanged.
  `display_type` shows `(validated as …)` from the refinement when there is
  one.
- New `HoleTiming::Guarded { guard: GuardOrigin, roots: Vec<String> }` and
  `HoleTiming::is_late()`. `CandidateScope` loses its `schema` and
  `host_globals` fields. `CandidateScope::build(.., types: &TypeScope, ..)`
  makes a hole `Guarded` in two cases:
  - an approval-time hole that evaluates to `null` and whose own expression
    has `guard_presence()` at the site (`roots` empty);
  - a late-root hole whose expression has `guard_presence()` (`roots` keeps
    the late roots).
  Either makes the site `TemplateRequired`, so a non-template command still
  fails with `TemplateGrammar`. Without guard evidence, a `null` is still
  `MissingValue`. The guarded hole keeps its authored expression, and
  `resolved` stays `None`.
- `CommandTemplate::placeholders_mut()` is new and public; hosts that retime
  holes can use it.
- `DeferredShellRoots` is unchanged and gains no document keys.

**Validate live holes** (`template/fill.rs`)

- New `RejectReason::GuardRefinement`. After the declaration check passes,
  `fill_template` checks `HoleType::refinement`. A failure is
  `HoleError::Rejected` → `TemplateValueRejected`, with `declared_type` written
  `"<declaration> narrowed to <refinement>"`. A `null` is still `Missing`, and
  the declaration is checked first.
- `fill_template` now evaluates each distinct expression once per call, and
  that checked value fills every position the expression holds (spec §6:
  "use the same checked value for dispatch"). This is the only behavior change
  visible without a guarded scope. The existing test
  `every_segment_is_filled_once_before_the_chain_runs` now expects
  `["a", "b"]` instead of `["a", "b", "a"]`.
- `resolve_late_values` still sweeps only `HoleTiming::Deferred`, so a
  `Guarded` hole never reaches an unconditional barrier. For a deferred hole
  it checks the declaration and records whether the refinement holds
  (`LateHole.refinement_holds`). `fill_template` enforces that verdict only
  when the command is reached, so a branch that does not run cannot fail on
  its narrowing at the barrier.
- Ownership as the Phase 1 contract set it: `candidate.rs` decides timing and
  `fill.rs` consumes it.

**Compile-only adjustments outside the template module**

- `darkmatter/.../preflight/collect.rs::DiscoverySiteContext::build` and
  `shell_expansion/site.rs::SiteScope::site` build
  `DeclaredTypes::new(schema).with_host_globals(..)` and an unguarded scope.
  The lazy-schema behavior is kept: the schema is assembled only for a site
  with holes.
- Claudine `lifecycle/shell_template.rs::typed_template` and
  `sequence/preflight/mod.rs` (`prepare_site`) build an unguarded scope over
  their existing schema and host globals.
- `composition/preflight.rs::sequence_lifecycle_template` timing flip: a
  `Guarded` hole stays `Guarded`, so it is never validated at preflight.

### Departures and decisions

1. **`Guarded` covers late-root holes too.** The Phase 1 contract named only
   discovery-null holes. Spec §6 and AC 10 say a guarded sequence late value
   must not fail at the task-start barrier, so a late hole that a guard proves
   present is also `Guarded`. It keeps its `roots`, and it is read live at the
   reached action, not from the step-entry snapshot. An ordinary late hole is
   unchanged.
2. **A refinement is not checked at an unconditional barrier.** The barrier
   checks the declaration and records the refinement verdict. The reached
   command enforces it (see `LateHole`).
3. **A refined hole does not force template mode** when its value is present
   at discovery. An exact approval still fixes the resolved bytes, and those
   bytes are what that mode authorizes. A template, prefix, or automatic run
   fills through `fill_template` and so checks the refinement.
4. **Guard labels.** `TypeScope::assume` labels a guard with
   `Expr::to_string()` (`and(ctx.dirty_files, commit_message)`). Hosts that
   know the authored location should use `assume_at(.., GuardOrigin::new("success[4].when"))`.
5. **`HoleRefinement` is boxed.** An inline refinement grew `ArgPart::Hole`
   past clippy's `large_enum_variant` threshold.

### Tests (requirement → evidence)

| Requirement | Test |
|---|---|
| Guarded missing hole waits as a template, keeps its authored expression, is not offered exact, and is authorized only by its shape (AC 1 core) | `template::tests::guarded_discovery::a_guard_proven_missing_value_waits_for_its_command_as_a_template` |
| Unguarded / false-side / non-proving guards still fail (AC 2, AC 9) | `…::a_missing_value_no_guard_proves_still_fails_discovery` (no guard, the false side, `!x`, `\|\|`, `is_null`, an unrelated guard) |
| A guard on `x` does not defer `y` (AC 9) | `…::a_guard_on_one_value_does_not_defer_another` |
| A non-null declaration does not excuse an actual null (AC 9) | `…::a_required_declaration_does_not_excuse_an_actual_null` |
| A value present at discovery stays approval-time / AnyMode | `…::a_guarded_value_present_at_discovery_stays_approval_time` |
| Grammar refusal and variable-program refusal still apply | `…::a_guarded_missing_value_in_a_non_template_command_fails_its_grammar` (`Pipe`, `ProgramWordHole`) |
| Guarded late values skip the barrier; ordinary ones keep it (AC 10 at the API) | `…::only_unguarded_late_values_meet_the_barrier` |
| AC 6: `is_string(target)` → `{{ string }}`, validated as the narrowed type | `l1 shell_template_guards::a_type_predicate_guard_approves_its_hole_as_the_narrowed_type` |
| Saved rule grammar kept; narrower enum round-trips; the whole-enum rule covers it | `shell_template_guards::a_narrowed_enum_approves_in_the_saved_rule_grammar` |
| AC 12: a present value that changed type is refused; the declaration is checked first | `shell_template_guards::a_live_value_outside_its_guard_refinement_is_refused` |
| AC 3 (template half): null at the reached command is `MissingValue` | `shell_template_guards::a_null_when_the_command_is_reached_is_missing` |
| Whole-chain refusal | `shell_template_guards::a_refused_value_in_any_segment_refuses_the_whole_chain` |
| One checked value fills every position | `shell_template_guards::one_checked_value_fills_every_position_of_its_expression`, updated `shell_template_execution::every_segment_is_filled_once_before_the_chain_runs` |
| The barrier checks declarations only; `Guarded` is skipped | `shell_template_guards::the_barrier_leaves_guard_checks_to_the_reached_command` |
| Existing exact/prefix/template, blacklist, and recovery regressions (AC 14) | unchanged `shell_template_*`, `shell_missing_values`, and Claudine executor/sequence tests, all in the gates below |

- **Fidelity:** F1, tier L1. The candidate tests are crate-internal because
  `CandidateScope` is `pub(crate)`. The projection, fill, and barrier tests
  use the public template API. Nothing crosses a CLI, filesystem, or
  persistence boundary that this phase changed. Invocation evidence (AC 1–3
  through the CLI) belongs to Phase 5, once Phase 4 threads guards to real
  sites.
- **Mutation check:** making the refinement check always pass failed
  `a_live_value_outside_its_guard_refinement_is_refused` and
  `the_barrier_leaves_guard_checks_to_the_reached_command`. The change was
  reverted.
- **Input Robustness Matrix:** not applicable. No loader, deserializer, or
  field validator changed (plan rule 8).
- New file `darkmatter/lib/tests/l1/shell_template_guards.rs` is declared in
  `tests/l1/main.rs`. No tier marker is used.

### Docs and skills

- `darkmatter/docs/inline/preflight-checks.md`:
  - Step 1 of the template checks now says one checked value per expression,
    plus the refinement check.
  - A new "Guarded values (planned)" subsection, marked planned until Phase 4
    threads guards to the sites.
- `.claude/skills/darkmatter/compose.md` describes the `CandidateScope::build`
  scope requirement, `HoleTiming::Guarded`, the refinement step in
  `fill_template`, and the barrier rule.
- `.claude/skills/darkmatter/expressions.md` describes the new
  `project_hole(expr, &TypeScope)` and `HoleType::refinement`.
- The `claudine` skill is unchanged: Claudine's behavior did not change.

### Gates

- Scope calculator, run after all edits:

  ```sh
  git status --porcelain --untracked-files=all | awk '{print $NF}' \
    | xargs python3 scripts/ci/affected_scope.py
  ```

  Result: `packages: ["claudine", "darkmatter", "test-toolkit", "claudine-cli"]`,
  `change_class: "package"`. Because `packages` is non-empty, no gate was
  skipped. The working tree also still holds Phase 2's uncommitted files, so
  the scope includes them.
- `just lint` (darkmatter): exit 0. The first run failed on
  `clippy::large_enum_variant` (`ArgPart::Hole`). Boxing `HoleRefinement`
  fixed it.
- `just test` (darkmatter): **10686 passed**, 10 skipped, exit 0.
  - The topic doc was edited while that suite ran. The two binaries' tests
    that read docs (`inline_document_text`, `schema_phase_validation`) were
    rerun afterwards: 23 passed.
- `just check-tier-coverage darkmatter`: 0 stranded.
- `just test` (claudine): **10479 passed**, 9 skipped, exit 0.
- `just lint` (claudine): exit 0.
- `just check-tier-coverage claudine`: 0 stranded.
- `shipped_schema_corpus` was rerun after the frontmatter edits, because it
  reads this log's and the plan's frontmatter: 8 passed.
- `test-toolkit` `just archive-path-guard`: 2 passed.
- Dependent packages:
  - No crate outside `darkmatter` and `claudine` names `project_hole`,
    `project_types`, `HoleTiming`, `CandidateScope`, or `RejectReason`.
  - `cargo check --workspace --tests` is clean.
  - The darkmatter area's `just test` already covers DMLS and `darkmatter-cli`.
- There were no failures, pre-existing or load-sensitive.
- Cross-OS check: **not run**. The change is in-memory analysis and argument
  filling, with no path, process, terminal, or `cfg` code, so there is no
  notable OS risk. CI covers Linux and macOS, and the nightly covers Windows
  and WSL2.
