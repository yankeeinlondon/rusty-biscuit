---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/fixes/2026-10-16-type-narrowing/spec.md"
plan: "claudine/fixes/2026-10-16-type-narrowing/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 changed no code. The full contract and site inventory are in this log under "## Phase 1"; read it before you edit anything. Five points matter most for Phase 2:

    (1) The parser lowers `a && b` to `FunctionCall "and"`, and `a || b` is either `Fallback` or `or(...)`. `assume` must treat those, matched through the function registry and case-insensitively, as the logical operators. Do not add a new AST node.

    (2) `AbsenceScope` (absence.rs) and `narrowed_expression_paths` (directive_targets.rs) are two guard walkers that already disagree on `&&`. Both get replaced by `TypeScope::assume`. Keep AbsenceScope's warning policy (fallback operands, absence-predicate arguments) as a separate concern.

    (3) `project_hole` reads `EffectiveSchema.simplified` and types only bare `x` / `fm.x`. Put the shared declared layer (`DeclaredTypes`) behind it and keep `project_hole`'s signature, so that `rule_covers` callers do not change.

    (4) Use `PropertyDef::is_required` / `admits_null` for presence. Do not write a new presence calculation.

    (5) For AC 1: both the shipped and the frozen `implement-plan.md` now default `commit_message: ""`, and an empty string counts as present. The AC 1 test must stage the frozen copy and remove that default line.

    The lifecycle walk is still Claudine's (`resolve_lifecycle_shell_commands_in_sequence`, via the flattened `for_each_action_mut`), because the darkmatter-lifecycle-ownership extraction has only landed its Phase 1.
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
