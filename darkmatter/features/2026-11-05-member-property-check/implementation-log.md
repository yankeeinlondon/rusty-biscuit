---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-member-property-check/spec.md"
plan: "darkmatter/features/2026-11-05-member-property-check/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
message_to_agent: |-
    Phase 1 changed no source. The R0 check revised three rules in plan.md;
    read the revised D5, D7, E4, M5 and M12 text before coding:
    - M5: SiteSchema::Template is now a struct variant,
      `Template { template: Option<DocumentTemplate>, unresolved: Box<UnresolvedSlots> }`.
      Get the pending template with
      `unresolved.pending.as_ref().and_then(|p| p.template())`, which returns
      borrows (clone the SchemaRoot and the Arc<DefinitionTable>).
    - M12 (Phase 3): SchemaOutcome has only Ready/Failed. Put the template in a
      new `SlotStatus.template` field (dmls/src/overlay/mod.rs:241). In the
      same change, fix the doc comment on `SlotStatus::template_shape`, which
      wrongly says the shape excludes slots.
    - D5 (Phase 5): a library composition with no session attaches a resolving
      SessionMode::Full session (compose/pipeline/mod.rs:79-82). Use a caller
      SessionMode::Passive session for the "without resolution" AC3 case.
    - E4: `try` evaluates through evaluate_recovery
      (compose/expression/mod.rs:1026), not evaluate_lazy_function.
    - Shared slot fixtures: darkmatter/lib/tests/dynamic_slots_fixtures/mod.rs.
      Slot work counter: Work::SlotResolution (counted per slot evaluated).
    Baseline before any change: just test 16118 passed / 10 skipped; just lint clean.
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
