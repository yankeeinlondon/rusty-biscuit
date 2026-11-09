---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-member-property-check/spec.md"
plan: "darkmatter/features/2026-11-05-member-property-check/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/expression/member_check.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/src/markdown/compose/policy.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/dynamic_schema_slots.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/member_check.rs
    - darkmatter/cli/src/args/policy.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/collections.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/undeclared_member.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
decisions_for_review:
    - |-
        Library compositions are never run without a slot session: the pipeline
        creates a resolving one when the caller sets none. The plan assumed the
        no-session case could leave a schema unresolved, and its acceptance test
        for "composition without resolution" was written for that. I changed the
        test to attach a passive slot session, which is the only way the library
        leaves a schema template unresolved during a normal composition. That
        composition fails with dm.schema.slot_pending, and the member warning is
        expected among the failure's earlier warnings. Alternative: drop the
        "without resolution" composition case and test only the editor. That is
        less coverage. If the reviewer prefers it, only the one Phase 5 test case
        changes.
    - |-
        The plan's site_schema_for returned Template(Option<DocumentTemplate>)
        and had the editor overlay keep that template in a new SchemaOutcome
        variant. The landed overlay reports a pending schema as
        SchemaOutcome::Failed(UnresolvedSlots) plus a SlotStatus record, and its
        slot jobs and slot diagnostics need that error. So I changed the
        library's Template variant to carry both the template and the
        UnresolvedSlots error. The overlay rewraps the error into the existing
        Failed outcome and stores the template in a new SlotStatus.template
        field, so the schema is assembled once.
        Alternatives: (a) call effective_for and then site_schema_for a second
        time for pending documents only, which repeats schema assembly; or (b) a
        library helper that builds the template from an UnresolvedSlots error
        alone, which cannot cover a document whose own schema is concrete but
        whose trigger payload is pending. Choosing (a) would change only the
        overlay assembly and the SiteSchema enum shape (Phase 2/3 code).
    - |-
        The member check types the base of a selector a little more widely than
        the plan's rule. The plan typed a postfix or bracket base with the
        shared expression typer, which knows nothing about a constant index or
        member applied to a function result, so
        load_data("h.yaml", "{ name: string }[]")[0].nmae was never reported.
        I added a small local rule in the member check: a constant index or
        member on a value with no stable variable path follows the same
        member/item projection a variable path uses. The shared expression
        typer (and so the call check) is unchanged. Alternative: put the rule
        into the shared typer, so the call check would also type
        load_data(...)[0] arguments; that changes call-check verdicts and was
        out of scope. If the reviewer prefers the shared typer, move base_type
        from member_check.rs into call_check.rs::expression_type.
    - |-
        The plan asked for a test that a value outside the literal members of
        a pending enum(...) is not rejected by the call check. The call check
        never compares enum members at all (a host parameter typed
        enum(macOS, Linux) accepts an enum(Windows) argument), so no input is
        rejected for that reason even with a concrete schema. The test instead
        shows the rule that matters: with the concrete enum(Windows) the call
        check judges the argument as a string (upper(os) compatible,
        min(os, 1) uncertain), and with the pending enum(Windows, {{ 123 }})
        every such call is "type unknown", never judged from the literal
        written beside the slot. Nothing else would change if the reviewer
        wants a different assertion.
    - |-
        SiteSchema keeps the plan's shape, Effective(Option<EffectiveSchema>)
        holding the resolved schema by value, with a clippy
        large_enum_variant allowance. Boxing it would save stack space when a
        template is returned, but adds an allocation on the common path and
        changes the signature every later phase uses. Switching to a box only
        touches SiteSchema, DeclaredTypes::for_site, and their callers.
    - |-
        The editor's pending-template test holds the slot job (the server's
        background schema resolution) rather than the registry gate the plan
        named. Holding the slot job is what keeps the first resolved schema
        from being published, and it is how the existing slot tests observe
        that state; holding the registry gate delays trigger discovery,
        which this document does not use. The test also proves that no slot
        was evaluated while it ran. Alternative: also add a registry-gate
        variant; it would add a test without covering a different state.
    - |-
        The editor keeps the pending document template itself, not the
        library's whole site-schema answer, so the library gained a small
        constructor, DeclaredTypes::for_template, and its for_site uses it for
        the template case. The plan had the editor call for_site; that would
        mean storing the slot error a second time beside the template. If the
        reviewer prefers for_site, the editor's slot record would hold a
        SiteSchema instead of the template; only the overlay and
        declared_types change.
    - |-
        Lifecycle stacks stay without member warnings in the editor, as the
        plan says. The shared frontmatter check served both ordinary values
        and lifecycle-stack expressions, so it now has two entry points: one
        with member warnings and one (for lifecycle stacks, including an
        expression-typed value inside a stack) with call findings only.
        Alternative: report members in lifecycle stacks too; composition does
        not observe those reads, so the editor and composition would then
        differ on another surface.
message_to_agent: |-
    Phase 3 wired the member check into DMLS. What Phase 4+ needs to know:
    - Library: DeclaredTypes::for_template(Option<&DocumentTemplate>) is new;
      for_site's template arm delegates to it.
    - DMLS: call_check::call_diagnostics(expr, text, site, strict) carries
      member warnings; call_check::stack_call_diagnostics is call-only and is
      used for every lifecycle-stack expression (FrontmatterCalls::check_stack).
      The overlay's SlotStatus.template holds the pending document template
      (from overlay/schema.rs::assemble's AssemblyError), and
      declared_types falls back to it when no bundle is served.
    - Tests: dmls/tests/l1/undeclared_member.rs covers every editor surface
      (Phase 5 AC7 parity tests can reuse its every_surface document shape).
    - member_check.rs's expr_nodes still carries the
      cfg_attr(not(test), expect(dead_code)) attribute: remove it when the
      composition observer (M14) calls it.
    - Gates after Phase 3: see the Phase 3 close in the log.

---

# Implementation Log for 2026-11-05-member-property-check (7 phases)

## Phase 1

### Wave 2 — Baseline (2026-11-09)

Run in `darkmatter/` before any change (the working tree already carried the
uncommitted expression-recovery changes listed in git status):

- `just test`: **pass** — 16118 tests run, 16118 passed, 10 skipped (137.6 s);
  no L1 test at or above 5.0 s.
- `just lint`: **pass** (clippy, format check, zed-dmls wasm32-wasip2 build).

No pre-existing failures to report.

### Wave 1 — Rebase verification (R0) (2026-11-09)

Both earlier features have landed in this worktree (dynamic-schema-types
committed; expression-recovery's last phase is uncommitted in the tree). One
entry per assumption; paths under `darkmatter/lib/src/markdown/` unless noted.

- **D1 — holds.** Slots live only in `Constraint::Dynamic`
  (`simplified-schema/lib/src/simplified/types.rs:1158`) and
  `Constraint::DynamicValue` (`:1164`). Substitution rewrites constraints only
  (`simplified-schema/.../slots/substitute.rs:167`, `:420`, `:534`), so
  properties, pattern keys, inline-object bodies and the definition table are
  unchanged. Two notes: constraint lists can be nested (an inline-object body or
  a pattern-key definition can itself carry a slot; the structure is still
  complete), and `PendingSchema::without_properties` (`pending.rs:116`) trims a
  *trigger payload* template's redeclared top-level properties
  (`schemas/mod.rs:1094`). M5 reads the document layer, so neither changes M4/M5.
- **D2 — holds.** `SchemaError::UnresolvedSlots(Box<UnresolvedSlots>)`
  (`simplified-schema/.../errors.rs:391`); `UnresolvedSlots { slots, pending:
  Option<PendingSchema>, .. }` (`:206`). The accessor is
  `PendingSchema::template() -> Option<(&SchemaRoot, &Arc<DefinitionTable>)>`
  (`slots/pending.rs:217`), passive; it returns borrows, so `DocumentTemplate`
  clones the root and the `Arc`. DMLS already reads it (`dmls/src/slot_jobs.rs:539-546`).
- **D3 — holds.** The session is a field, `DarkmatterSchemas.slot_session`
  (`schemas/mod.rs:263`, builder `with_slot_session` `:465`, `with_slot_pins_only`
  `:491`). With no session `resolve_template` (`:498-511`) returns the original
  `UnresolvedSlots`; the document layer falls back to it at `:874`, trigger
  payloads at `resolve_pending_payload` (`:1155-1177`). Lowering rejects a
  leftover slot (`reject_unresolved`, `simplified-schema/.../slots.rs:162`).
  `PreparedSchemas::effective_for` (`compose/schema_validation.rs:241-249`) maps
  every error to `None`.
- **D4 — holds (lazily).** `prepare_schemas` only attaches the session
  (`schema_validation.rs:423-427`); the first resolution happens inside
  `run_call_precheck`'s `effective_for` (`compose/pipeline/mod.rs:445`, `:1057`),
  which pins one snapshot per template, so the precheck sees the resolved
  schema. If resolution fails (approval required) the precheck sees `None` and
  the error surfaces later, at schema validation.
- **D5 — differs.** A library composition is never sessionless:
  `run_root_pipeline_collecting` attaches `SlotSession::for_request` in
  `SessionMode::Full` when none is set (`compose/pipeline/mod.rs:79-82`), so a
  pure-literal pending schema resolves. A template stays unresolved in
  composition only under a caller-attached `SessionMode::Passive` session
  (fails at validation, reported as `dm.schema.slot_pending`,
  `report_diagnostics.rs:341-352`), a `ReferenceCheck` session, or a pins-only
  discovery pass (both defer, `defers_template`, `schema_validation.rs:795-802`).
  **Revised:** R0's D5 row and the Phase 5 `ac3_pending_template` test now name
  a `Passive` session for the "without resolution" case, which fails with
  `dm.schema.slot_pending` and carries the member warning among the failure's
  earlier warnings. Closing note in the plan updated to match.
- **D6 — holds.** `Shape::of_type` (`compose/expression/scope/shape.rs:263`) keeps
  `Dynamic` constraints and does not panic; `enum_members` (`:577`) matches only
  `Constraint::Members`, so there is no finite member list. Note: a
  `literal({{ … }})` atom has no literal value, so its family is every family
  (still conservative).
- **D7 — holds; the overlay shape differs from M12's guess.** `SchemaOutcome` is
  `Ready(Option<Arc<SchemaBundle>>) | Failed(Arc<SchemaError>)`
  (`dmls/src/overlay/mod.rs:74`). A pending schema is
  `Failed(UnresolvedSlots)` plus `DocumentOverlay.slots: Option<SlotStatus>`
  (`SlotStatus { refreshing, unresolved, template_shape }`, `:241-252`), and
  `template_shape` holds only the root shape, with no definitions. Assembly
  calls `schemas.effective_for(&md)?` at `dmls/src/overlay/schema.rs:1048` under
  `SlotUse` (`:962`). `stale` (`overlay/mod.rs:198`) still means last-good
  frontmatter tree. `declared_types` (`dmls/src/diagnostics/call_check.rs:59-61`)
  types only `Ready(Some)`. **Revised M12 and M5:** `SiteSchema::Template`
  now carries `{ template, unresolved: Box<UnresolvedSlots> }`. The overlay
  rewraps `unresolved` into the existing `Failed` outcome and stores `template`
  in a new `SlotStatus.template` field, so the slot jobs and slot diagnostics
  are unchanged and the schema is assembled once. (See decisions_for_review.)
- **D8 — holds.** Grants gate reads only (`schemas/slots/grants/gate.rs:1-16`);
  a literal slot reads nothing. Precedent: `tests/l1/dynamic_schema_slots.rs:119`
  resolves `default({{ 1 + 2 }})` with `SlotSession::new(DataRuntime::new())`.
  Shared fixtures exist at `lib/tests/dynamic_slots_fixtures/mod.rs` (M21 reuse).
  The slot work counter is `Work::SlotResolution`
  (`darkmatter/lib/src/work_counters.rs:94`), counted per slot evaluated
  (`slots/host.rs:105`); usable for the Phase 2 "no slot evaluator runs" test.
- **E1 — holds.** `try` is `FunctionBinding::lazy("try")`
  (`compose/expression/functions/mod.rs:275`, `LazyKind::Recover` `:284`); no
  parser special case. `walk_static` (`absence.rs:546-555`), `branch_scope`
  (`:463-474`) and `check_expression_calls` (`call_check.rs:1561`) see both args.
- **E2 — holds.** `expression_type` (`call_check.rs:204`) → `try_type`
  (`:319-330`): the union of both arms, keeping `null`; an authored `[]` adds
  nothing when the other arm is unknown or accepts `[]`.
- **E3 — holds.** `AbsenceScope::call_argument` (`absence.rs:120-133`) excludes
  `is_recovery` (`:156-159`); `try` declares no `narrows(...)`
  (`darkmatter/schemas/partials/functions.yaml:566-573`).
- **E4 — holds, different function.** `try` evaluates through
  `evaluate_recovery` (`compose/expression/mod.rs:1026-1046`), not
  `evaluate_lazy_function` (`:984`, `unreachable!` for `try` at `:999`). Both
  arguments use the same `&mut` observer, never reset, so reads before a
  recovered failure stay recorded. M14's citation is updated.
- **E5 — holds.** `callback_item` (`call_check.rs:1793-1802`) types the source
  with `expression_type` and projects its entry shape.

**Cited evidence.** Every symbol in M1–M19 still exists. The moved line
numbers are corrected in the plan. The notable relocations are
`literal_index_key` (now `compose/expression/fm_namespace.rs:61`),
`member_shape`/`arm_member` (`projection.rs:622-673`), `expression_type`
(`call_check.rs:204`), `effective_for_with_override` (`schemas/mod.rs:838`), and
the DMLS frontmatter caller (`diagnostics/frontmatter.rs:1199`, inside
`FrontmatterCalls::check`). Phases 2–7 also cite some line numbers; those were
not all re-verified, so treat them as approximate.

**Drift found (not fixed here).** The doc comment on
`SlotStatus::template_shape` (`dmls/src/overlay/mod.rs:249-251`) says the
shape keeps "every constraint that is not a slot". `slot_jobs::template_shape`
clones the template shape with the slots still in place. Phase 3 edits this
struct, so it is handed to Phase 3 rather than adding a source change to this
phase.

### Phase 1 close (2026-11-09)

- Gate-skip check, run from the repository root after all edits:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  returned `packages: [biscuit-file, darkmatter, darkmatter-cli, test-toolkit,
  darkmatter-lsp]`, `change_class: package`. Those packages come from source
  changes already uncommitted in the tree before this phase (the
  expression-recovery work and biscuit-file); this phase edited only `plan.md`
  and this log. Gates are not skippable, so both were run again:
  - `just test` (darkmatter/): pass, 16118 passed, 10 skipped, none ≥ 5 s.
  - `just lint` (darkmatter/): pass.
- No source file changed, so there were no dependent-package tests to run and
  no cross-OS check.
- Requirement-to-test mapping: Phase 1 delivers no behavior, only the R0
  verification and the baseline, so no test was added.
- The plan's `packages` key was left as it was (darkmatter, darkmatter-cli,
  darkmatter-lsp). It records the plan's scope, and replacing it with this
  phase's empty list would lose that. This log's `packages` is `[]`.
- Darkmatter skill: no change needed (no behavior changed).
- Status: Phase 1 complete; ready for Phase 2.

## Phase 2

### Wave 3 — Declaration verdict, absence member reads, code and registry (2026-11-09)

- **M1**: `UNDECLARED_MEMBER_CODE` (`dm.expression.undeclared_member`) in
  `expression/diagnostics.rs`, re-exported from `expression/mod.rs`.
- **M3**: one row in `COMPOSE_DIAGNOSTIC_CODES` after the undeclared-property
  row; `cli/src/args/policy.rs` test
  `the_undeclared_member_code_is_deniable_on_compose_only` (Compose `--deny`
  parses it as `PolicySelector::Code`, the long help lists it, Render
  rejects it).
- **M4**: `MemberDeclaration` + `declares_member` in `scope/projection.rs`
  (private `arms_declare`/`arm_declares` walk, alias budget as
  `arm_member`); `member_shape` is now `pub(crate)`; both re-exported from
  `scope/mod.rs`. Tests: `scope::tests::member_declarations` (every type-table
  row, optional member, pattern-key, array arm, bare `object`, `unknown`,
  passive `Imported`; a recursive `Ref` declared/undeclared and unknown with a
  missing table) and `scope::projection::tests::an_exhausted_alias_budget_is_unknown`
  (a present definition with a spent budget is unknown). A real alias *cycle*
  cannot be built: the resolver rejects alias cycles, so the budget test calls
  the private walk with budget 0 on a real table.
- **M7**: `walk_static` now writes into a private `StaticReads` sink
  (`variables` + `members`); `static_variable_reads*` and
  `static_callback_reads` return `variables` exactly as before. New
  `StaticMemberRead`, `static_member_reads`, `variable_root_segments`, and
  `AbsenceScope::handles_expr`. One record per selector: each variable
  segment after its binding (`fm.<key>` / `local.<name>` take two segments),
  each postfix name segment (prefix-precise `handles_expr` per segment), and a
  string-literal bracket key. Module doc updated. Tests:
  `absence::tests::member_reads::*` (8 tests: every absence-table row,
  postfix/bracket forms, sibling vs descendant facts, default gate scope,
  `try` operands, namespace wrappers, callback members, and a
  variable-inventory corpus with spans). The plan's
  `AbsenceScope::handles_absence()` accessor was not needed:
  `handles_expr` includes the direct-position flag like `handles` does.

### Wave 4 — Findings and template declarations (2026-11-09)

- **M2/M6/M8/M9/M10**: new `expression/member_check.rs`: `MemberFinding`
  (`message`, `diagnostic`), `expression_member_findings`,
  `expression_member_diagnostics`, and the crate-private `spanned_nodes` /
  `expr_nodes` enumerators. Records of one node are taken together, the base
  typed under `branch_scope` at the node, stopping at the first undeclared or
  unknown step. `expr_nodes` is unused outside tests until composition wires
  it, so it carries `#[cfg_attr(not(test), expect(dead_code, …))]`, which
  turns into an error once it is used and must then be removed.
- **M5**: `DocumentTemplate`, `SiteSchema`, `DarkmatterSchemas::site_schema_for`
  in `schemas/mod.rs`. `effective_for_with_override` now delegates to the
  private `site_schema_with_override` and turns `Template { unresolved, .. }`
  back into `Err(SchemaError::UnresolvedSlots(unresolved))`, so its behavior
  is unchanged and the work happens once. `Template` is returned at exactly
  two points: the document `$schema` resolution (template = the pending
  template's root and definitions) and the trigger-layer resolution (template
  = the concrete document layer's SimplifiedSchema root, or `None`).
  `DeclaredTypes::for_site` in `scope/projection.rs`.
  `SiteSchema` carries `#[allow(clippy::large_enum_variant)]` (the
  `Effective` variant holds an `EffectiveSchema` by value, as the plan's
  signature and `effective_for` do).
- Tests: `l1/dynamic_schema_slots.rs::site_schemas::*`:
  `a_pending_document_schema_types_its_sites_from_the_template` (pending
  inline `$schema` → `Template { template: Some(..) }`, `server` is a closed
  object, `server.nmae` reports; `Work::SlotResolution` unchanged under
  `work-counters`; `effective_for` still `Err(UnresolvedSlots)`; concrete →
  `Effective(Some)`; no schema → `Effective(None)`) and
  `a_pending_enum_is_not_judged_by_its_literal_members` (see decision below).

### Wave 5 — Library classifier tests (2026-11-09)

`lib/tests/l1/member_check.rs` (declared in `l1/main.rs`), 13 tests through the
public API: `type_table_rows`, `access_forms`, `a_typed_function_return`,
`paths_and_keys`, `namespaces_and_host_globals_are_not_checked`,
`contracts`, `named_references_in_the_site_definitions`, `try_typing`,
`narrowing`, `shadowing`, `spans`, `diagnostic_fields`, `absence_table`.
`ordinals_mirror_erase` lives as a unit test in `member_check.rs` because the
enumerators are crate-private; it compares the erased `spanned_nodes` sequence
with `expr_nodes` node for node over a corpus covering every node kind.

Findings while writing them:

- A string-literal index node's span already includes its quotes
  (`server["nmae"]` → 7..13), as M2 wanted.
- `expression_type` gives `unknown` for a constant index or member on a
  value with no stable subject (`load_data(…)[0]`), so
  `load_data("h.yaml", "{ name: string }[]")[0].nmae` was silent. The
  member check now types such bases with a local `base_type` that follows
  constant steps through `member_shape` (see decisions).
- No built-in expression function returns a closed object (`http_post` is a
  lifecycle mutation, not dispatchable), so the typed-return case uses a
  host registration, as the plan allowed.
- A "cyclic alias" contract cannot be authored (the resolver rejects alias
  cycles); the bounded-alias behavior is covered by the unit test above.

### Requirement-to-test mapping

| Behavior | Test(s) |
| --- | --- |
| Type table (M4) | `scope::tests::member_declarations::the_type_table_decides_each_verdict`; `l1 member_check::type_table_rows` |
| Named references, missing table, budget (M4) | `scope::tests::member_declarations::a_reference_is_checked_through_its_definition`; `scope::projection::tests::an_exhausted_alias_budget_is_unknown`; `l1 member_check::named_references_in_the_site_definitions` |
| Absence table (M7) | `absence::tests::member_reads::member_reads_follow_the_absence_table`; `l1 member_check::absence_table` |
| Facts: siblings, descendants, gates, `try` (M7, E3) | `absence::tests::member_reads::{a_fact_covers_its_descendants_and_never_a_sibling, a_gate_scope_handles_nothing, try_arguments_are_ordinary_reads}` |
| Variable inventory unchanged (M7) | `absence::tests::member_reads::static_variable_reads_are_unchanged_by_member_records` + every pre-existing `absence` test |
| Access forms, eligible bases (M6) | `l1 member_check::{access_forms, a_typed_function_return, namespaces_and_host_globals_are_not_checked}` |
| No cascade, keys, computed keys, indexes (M8) | `l1 member_check::paths_and_keys` |
| Contracts and `try` typing (M8, E2, E5) | `l1 member_check::{contracts, try_typing}` |
| Narrowing and invalidation | `l1 member_check::narrowing` |
| Callback shadowing, node ordinals per site | `l1 member_check::shadowing` |
| Spans (M2) | `l1 member_check::spans` |
| Diagnostic fields and message (M1, M9) | `l1 member_check::diagnostic_fields` |
| Ordinal parity (M10) | `member_check::tests::ordinals_mirror_erase` |
| Registry and `--deny` (M3) | `cli args::policy::tests::the_undeclared_member_code_is_deniable_on_compose_only` |
| Template declarations, passivity (M5) | `l1 dynamic_schema_slots::site_schemas::*` |

Fidelity: everything here is library-internal and passive (no CLI, file
write, or terminal boundary in this phase), so F1 L1 tests at the public API
are the right rung; the DMLS and composition surfaces get their end-to-end
tests in Phases 3–5.

### Phase 2 close (2026-11-09)

- Gate-skip check (repository root, after all edits):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [darkmatter, darkmatter-cli, test-toolkit]`,
  `change_class: package`. Gates are not skippable; both were run.
- `just lint` (darkmatter/): pass, after two fixes it reported
  (`clippy::large_enum_variant` on `SiteSchema`, allowed with a reason;
  `iter_cloned_collect` in a test).
- `just test` (darkmatter/): pass, 16146 run, 16146 passed, 10 skipped
  (baseline 16118; +28 new tests). Two DMLS L1 tests were reported SLOW
  (5.14 s / 5.23 s):
  `claudine_editor_parity::the_shared_corpus_valid_depth_fixtures_raise_no_schema_diagnostic`
  and `content_policy_baseline::every_shared_declaration_publishes_the_baseline_verdict_at_its_anchor`.
  Rerun alone they take 2.35 s and 2.41 s; neither touches code changed in
  this phase (DMLS does not call the member check yet). Recorded as
  load-sensitive, not changed.
- The existing `undeclared_property` and `static_variable_reads` tests pass
  unchanged.
- Dependent packages (targeted, serially): `claudine` tests matching
  prewarm/bindings/type_scope_ownership/lifecycle_call/host_catalog/
  collection_host_compat (the consumers of `static_variable_reads`): 160
  passed; `claudine-gen` input/schema tests (the consumer of
  `effective_for_with_override`): 15 passed.
- Cross-OS: not run. The phase adds no path, process, or OS-specific code;
  the new tests build paths from temp dirs through
  `biscuit_file::canonicalize_simplified`.
- Darkmatter skill: not changed in this phase. Phase 6 owns the skill and
  docs updates for this feature (expressions.md, compose.md, dmls.md).
- No pre-existing failures.
- Status: Phase 2 complete; ready for Phase 3.

## Phase 3

### Wave 6 — Choke point, code, and template layer (2026-11-09)

- **M12 (template layer)**: `dmls/src/overlay/schema.rs::assemble` now calls
  `DarkmatterSchemas::site_schema_for` instead of `effective_for` and returns
  `Result<Option<SchemaBundle>, AssemblyError>`. `AssemblyError { error,
  template }` (with `From<SchemaError>`, so every `?` in `assemble` is
  unchanged) carries the document template beside the rewrapped
  `SchemaError::UnresolvedSlots(unresolved)`. The overlay
  (`overlay/mod.rs`) keeps its slot handling exactly as before
  (`find_unresolved`, `template_shape`, job demand) and stores the template in
  the new `SlotStatus.template: Option<Arc<DocumentTemplate>>`. The slot
  worker's `resolve_slot_templates` maps the failure back to its
  `SchemaError`. `SchemaOutcome` gains no variant.
- **`call_check::declared_types`**: the bundle's effective schema (fresh or
  last-good) when a bundle exists, else `SlotStatus.template`, else untyped.
  The template arm needs a `DeclaredTypes` borrowed from an
  `Arc<DocumentTemplate>` (no `SiteSchema` is kept), so the library gained
  `DeclaredTypes::for_template(Option<&DocumentTemplate>)`, and
  `DeclaredTypes::for_site`'s template arm now delegates to it (one rule, two
  holders).
- **M11**: `call_diagnostics(expr, text, site, strict)` appends
  `expression_member_diagnostics(expr, text, site)` after the call findings,
  independent of `strict`. Callers: `providers/dsl.rs` (body interpolation:
  `interpolation.text`; directive `when=`: `value.value`),
  `providers/collections.rs` (`authored.site.text`), and
  `diagnostics/frontmatter.rs` (`FrontmatterCalls::check(expr, text)` for
  interpolations and `expression`-typed values).
- **Lifecycle stacks stay call-only.** `FrontmatterCalls::check` was also the
  unguarded path for lifecycle-stack expressions
  (`stack_expression_diagnostics`) and for an `expression`-typed value inside a
  stack that has no guarded surface. Both now use the new
  `FrontmatterCalls::check_stack` → `call_check::stack_call_diagnostics`
  (call findings only); the `expression`-typed path picks it when
  `lifecycle.stack_event(&path)` is set. Module doc updated.
- **M13**: `code::EXPRESSION_UNDECLARED_MEMBER` delegating to
  `UNDECLARED_MEMBER_CODE`, and a `WARNING` row in `codes.rs`'s severity
  ladder table.

### Wave 7 — DMLS tests (2026-11-09)

New `dmls/tests/l1/undeclared_member.rs` (declared in `l1/main.rs`), F2/L1
through the in-memory LSP session; each asserts line, start, end, source,
severity, message, and null `data` types:

| Plan item | Test |
| --- | --- |
| `reports_on_every_surface` (7 surfaces, control with `name` first) | `reports_on_every_surface` |
| `ac1_editor` (three rows, `[]`, declared control, computed contract) | `ac1_editor` |
| `all_branches` (unchosen ternary, callback over a `[]` value) | `all_branches` |
| `positions` (UTF-16: `é` = 1 unit, `😀` = 2; body edit shifts lines) | `positions` |
| `strict_independent` (identical member warnings; control: strict changes call findings) | `strict_independent` |
| `pending_template` (AC3, slot job held, `slot_pending` shown, no slot run) | `pending_template` |
| refresh with last-good snapshot, reported once | `pending_template_refresh_reports_once` |
| `lifecycle_unchanged` (stack value silent, value outside the stack warns) | `lifecycle_unchanged` |
| `no_side_effects` | `diagnostics::call_check::tests::member_diagnostics_never_read_data_or_enter_a_handler` (beside `diagnostics_never_enter_a_handler`, `effects-instrumentation`) |

- Mutation check: with `declared_types` reverted to bundle-only,
  `pending_template` fails at the template assertion; restored.
- The M21 schema (slot inside the inline object string) works as written.
- `pending_template` holds the slot job (`start_with_slot_gate`), not the
  registry gate: the slot job is what publishes the first snapshot, and the
  held job is how the existing slot tests observe "before any snapshot is
  published" (see decisions).

### Phase 3 close (2026-11-09)

- Gate-skip check (repository root, after all edits):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [darkmatter, darkmatter-cli, darkmatter-lsp, test-toolkit]`,
  `change_class: package`. Gates are not skippable; both were run.
- `just lint` (darkmatter/): pass, first run.
- `just test` (darkmatter/): pass, 16155 run, 16155 passed, 10 skipped
  (Phase 2 close: 16146; +9 new tests). No SLOW or flaky test this run.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependent packages: none to run. No package outside `darkmatter/` depends
  on `darkmatter-lsp`, and the only library change is the additive
  `DeclaredTypes::for_template` (with `for_site` delegating to it, same
  result); no code outside the area names either symbol.
- Cross-OS: not run. No path, process, or OS-specific code changed; the new
  tests build paths from temp dirs through `biscuit_file::canonicalize_simplified`
  and `url::Url::from_file_path`.
- Docs and the Darkmatter skill: unchanged in this phase; Phase 6 owns
  `dmls/docs/diagnostics.md` and the skill for this feature.
- No pre-existing failures.
- Status: Phase 3 complete; ready for Phase 4.
