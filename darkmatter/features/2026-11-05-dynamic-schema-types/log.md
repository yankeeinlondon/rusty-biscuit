---
feature: "2026-11-05-dynamic-schema-types"
---

# Review-to-Implement Log: Dynamic Schema Types

## Implementation of Review Findings #1

> **started at:** 2026-11-08T11:49:32-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- the review has no `human_in_the_loop` items, no blocked findings, and no recurrence; every finding is repairable this cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Several slot paths perform reads before approval | pending | |
| R1-F1.1 | Inline enum slot: approval-required before data bytes are read | pending | |
| R1-F1.2 | Inline suggestion slot: same | pending | |
| R1-F1.3 | Inline literal slot: same | pending | |
| R1-F1.4 | Inline default slot: same | pending | |
| R1-F1.5 | Inline minimum slot: same | pending | |
| R1-F1.6 | Inline maximum slot: same | pending | |
| R1-F1.7 | Inline pattern slot: same | pending | |
| R1-F1.8 | Imported data contract: authorize each import before opening it | pending | |
| R1-F1.9 | Schema-validation expression: read permitted page, stop before opening its unapproved schema | pending | |
| R1-F1.10 | Glob listing `find_files`: directory approval before enumeration | pending | |
| R1-F1.11 | `find_first_index`: directory approval before listing | pending | |
| R1-F1.12 | Last-index call: directory approval before listing | pending | |
| R1-F1.13 | Direct data read control: approval-required, no content observation (retain) | pending | |
| R1-F1.14 | Frontmatter reader control (retain) | pending | |
| R1-F1.15 | `try_frontmatter` control (retain) | pending | |
| R1-F1.16 | `markdown_title` control (retain) | pending | |
| R1-F1.17 | `markdown_body_empty` control (retain) | pending | |
| R1-F2 | Overridden trigger slots still evaluate | pending | |
| R1-F2.1 | Document overrides trigger property: success without asking/reading | pending | |
| R1-F2.2 | Later trigger overrides earlier trigger: same | pending | |
| R1-F2.3 | Unmatched payload control: exit 0, no read (retain) | pending | |
| R1-F2.4 | Effective dynamic payload control: approval-required (retain) | pending | |
| R1-F2.5 | Preflight and runtime use the same surviving-slot selection (consumer: validation, DMLS) | pending | |
| R1-F3 | Inline and trigger slot failures lose authored locations | pending | |
| R1-F3.1 | Compose inline schema: original document + authored slot range | pending | |
| R1-F3.2 | Schema-validate inline schema: same | pending | |
| R1-F3.3 | Compose trigger-inline payload: slot range in trigger file | pending | |
| R1-F3.4 | Schema-validate trigger-inline payload: same | pending | |
| R1-F3.5 | Compose referenced schema control (retain) | pending | |
| R1-F3.6 | Schema-validate referenced schema control (retain) | pending | |
| R1-F3.7 | Compose imported named type control (retain) | pending | |
| R1-F3.8 | Schema-validate imported type control (retain) | pending | |
| R1-F3.9 | DMLS inline slot diagnostic: positioned, consuming shared source mapping | pending | |
| R1-F4 | Unsaved data buffers are ignored outside YAML and JSON | pending | |
| R1-F4.1 | JSON5 unsaved buffer accepted | pending | |
| R1-F4.2 | JSONL unsaved buffer accepted | pending | |
| R1-F4.3 | TOML unsaved buffer accepted | pending | |
| R1-F4.4 | CSV unsaved buffer accepted | pending | |
| R1-F4.5 | TSV unsaved buffer accepted | pending | |
| R1-F4.6 | YAML control (retain) | pending | |
| R1-F4.7 | JSON control (retain) | pending | |
| R1-F5 | Stale completion candidates have no stale marker | pending | |
| R1-F5.1 | Completion during held refresh identifies stale values | pending | |
| R1-F5.2 | Hover control (retain) | pending | |
| R1-F5.3 | Diagnostic status control (retain) | pending | |

### Progress

- starting the work on 'R1-F1: Several slot paths perform reads before approval' at 11:50:02
        - defect class: an effectful read or listing reachable from slot evaluation performs its I/O before the slot grant gate judges the target
        - discovery: the library `compose_with_warnings` path alone does not reproduce R1-F1.1–7 (its in-pipeline gate fails first); the read happens in the reference check `md compose` runs before composing (`ComposeOptions::for_reference_check` graph preparation), whose frontmatter interpolation evaluated the inline `$schema` subtree; pre-flight (`collect.rs`) already excluded `$schema`, the pipeline did not
                - the new tests run the reference check, then the composition, as `md compose` does
        - sibling sites checked
                - dispatch table input readers: `load_data`, the `frontmatter` family, `markdown_*` (already gated before reading: R1-F1.13–17 clean); `validate_schema`, `find_files`, `find_first_index`, `find_last_index` (defective); path functions, index arithmetic, `has_skill`/`has_local_skill`, local `file_exists` make existence observations only (clean, confirmed by grep: the only I/O sites in `functions/` are `load_markdown`'s read, `find_index_endpoint`'s `read_dir`, and `find_files`' `list_files`)
                - schema opens while a slot is active: contract imports and `validate_schema`'s schema both read through `simplified_schema::resolve::buffers::read`
                - frontmatter passes walking `$schema`: pass 1 and pass 2 (both through `interpolate_frontmatter_impl`), pre-flight best effort (already excluded)
                - process slot cache: `SlotSession::serves` reread (rehashed) a cached entry's inputs before judging grant coverage (new row R1-F1.18)
                - pre-flight target naming: a listing was collected as the argument path (`<folder>/*.md`, `<folder>/review-1.md`), so the offered folder grant never covered the listing; and the post-call check named a `find_files` observation by its relative pattern, so no grant could allow it (new row R1-F1.19)
                - render-time "did you mean" sibling suggestions (`file_suggestions.rs`) list a folder when a not-found diagnostic is rendered; not reached during slot evaluation and not in the observation log; left unchanged (R1-F1.20)
        - repairs
                - `frontmatter_interpolation.rs`: `interpolate_frontmatter_impl` always excludes `$schema` (`with_schema_excluded`), covering every pass and caller
                - `simplified-schema` `resolve::buffers::guarded`/`unguarded`/`ReadGuard`: a host guard asked before every schema source read (buffer or disk); a refusal fails the read as `PermissionDenied`
                - `ResolutionContext::gate_schema_reads`: runs a schema resolution under a guard calling the slot gate; used by `DataRuntime::prepare_contract` (now takes the `ResolutionContext`; a refused preparation is not memoized) and `validate_schema`
                - `find_files` authorizes each pattern root, `find_*_index` the existing parent folder, before enumerating; the post-call check judges a `Glob` observation by its pattern's roots (`glob_listing_roots`)
                - `targets.rs::listed_folder`: pre-flight names the folder a listing enumerates
                - `session::resolve_contract_slots` evaluates nested template slots under `buffers::unguarded`, so they keep their own gates only
                - `SlotSession::serves` judges grant coverage before rehashing inputs; new work counter `SlotInputRehash`
                - docs: `docs/topics/schemas/schema-slots.md` "What a slot may read"; `simplified-schema/docs/topics/selecting-schema-files.md`; skills `darkmatter/requests.md`, `darkmatter/schema.md`, `simplified-schema/SKILL.md`
        - tests (all failed before the repair, pass after)
                - `darkmatter/lib/tests/l1/dynamic_schema_slot_read_order.rs`: one per R1-F1.1–12, controls R1-F1.13–17, `a_granted_enum_resolves_through_the_slot`, `a_persisted_folder_grant_lets_each_listing_run`, `the_folder_grant_pre_flight_offers_covers_each_listing`
                - `darkmatter/cli/tests/l1/dynamic_schema_slots.rs::compose_reads_nothing_for_an_unapproved_inline_slot`
                - `session_tests.rs::reuse::an_uncovered_cache_entry_is_refused_before_its_inputs_are_reread`
                - `simplified-schema` `buffers::tests::a_guard_is_asked_before_every_read_and_a_refusal_reads_nothing`, `an_unguarded_scope_suspends_the_guards_around_it`
        - orchestrator check of R1-F1.20 (sibling "did you mean" hints): clean. `suggest_sibling_files` is called only from `errors/blocks.rs::interpolation_block`, which renders `MarkdownError::Interpolation` (document expressions, outside Q5); no code under `lib/src/markdown/schemas/` produces that error, and `$schema` is now excluded from interpolation, so a slot failure never reaches the listing
- work completed for 'R1-F1: Several slot paths perform reads before approval' at 12:21:28
- starting the work on 'R1-F2: Overridden trigger slots still evaluate' at 12:21:28
        - defect class: a layer's slots are resolved or prompted for before layer precedence decides which properties survive
        - sibling sites checked
                - runtime trigger-layer resolution (`mod.rs::resolve_trigger_layers`): resolved every slot of each matched payload, whatever the document or a later trigger redeclared (R1-F2.1, R1-F2.2); fixed
                - pre-flight collection (`pending_templates_for`): filtered questions against the document only, never a later trigger, and judged a union document by any arm's names; fixed and now shares the runtime selection (R1-F2.5)
                - document `$schema` layer resolution: highest layer, never replaced, so all its slots are effective; clean
                - baseline / catalog / kind definitions: refuse slots; clean
                - every downstream consumer goes through those two sites: `md schema validate` and `md clean` (`approve_slot_reads` then `effective_for`), compose pre-flight (`compose_slot_approvals`), the reference check, DMLS (`overlay/schema.rs` `effective_for` + `templated` from `pending_templates_for`; slot jobs run `effective_for` with a session); fixed through the shared selection (R1-F2.6 to R1-F2.8)
                - slot cache key (`slots/cache.rs::resolution_key`): would have keyed a pruned resolution like the whole one when the dropped property held no slot; fixed by adding the omitted names to the trigger template identity (R1-F2.9)
                - pending reporting: a payload whose only slots were discarded still raised `UnresolvedSlots` without a session, so DMLS called the schema templated; fixed: such a payload resolves through `resolve_without_slots` and is not listed by `pending_templates_for` (R1-F2.10)
                - named-type imports: a slot in a stored definition that only discarded properties reach was still collected (`collect_slots` walks every definition); fixed: pruning drops unreached definitions (R1-F2.11)
                - `md schema triggers` `pending: true`: a fact about the trigger file, not a document's layering; left unchanged
        - repairs
                - `simplified-schema` `PendingSchema::without_properties` (drops named top-level properties and the definitions only they reach before resolution; `omitted_properties`, `is_union_arm`, `resolve_without_slots`); `slots::omitting` scope consumed by the replayed entry's lowering in `apply_active`; root-union arm sites call `apply_active_to_arm`, which marks the template `union_arm`; `DefinitionTable::retain`; `atom_refs` now also walks `Constraint::Property` declarations
                - `darkmatter` `mod.rs`: `declared_by_json` / `declared_by_template` / `declared_by_payload` and the one selection `discarded_by_higher_layers`, used by `pending_templates_for` and `resolve_trigger_layers`; `resolve_trigger_layers` collects every matched payload first, then resolves each template without what higher layers redeclare; `resolve_pending_payload` adds the omitted names to the cache identity and resolves a slot-less survivor without the session
                - docs: `darkmatter/docs/topics/schemas/schema-slots.md` "A replaced property's slots never run"; `simplified-schema/docs/topics/language/slots.md` "Resolving only the properties that survive"; skills `darkmatter/schema.md`, `simplified-schema/SKILL.md`
        - tests (each failed with the selection disabled, except the controls)
                - `darkmatter/lib/tests/l1/dynamic_schema_slot_precedence.rs` (new): compose R1-F2.1–4, an earlier trigger never discards a later one, surviving sibling slot, root-union every-arm rule, named-type definition dropped / kept, pre-flight asks nothing / asks once, no pending template, validation valid and asks nothing, no `UnresolvedSlots` without a session, cache identity across an edit dropping the override (fails when the omitted names leave the identity)
                - `darkmatter/cli/tests/l1/dynamic_schema_slots.rs`: `md compose` exit 0 with empty read set and `md schema validate` valid for R1-F2.1 and R1-F2.2, `md clean` no findings, approval-required control
                - `darkmatter/dmls/tests/l1/schema_slots_editor.rs`: no slot diagnostic for a redeclared trigger slot; approval-required control
                - `simplified-schema/lib/tests/l1/dynamic_slots.rs::surviving_properties` (4 tests)
        - decisions
                - prune inside the library replay (an `omitting` scope) rather than resolve and discard: discarded properties never reach substitution, so no placeholder value and no lazy validation are needed
                - a union layer replaces a property only when every arm declares it, matching `merge_layer`'s per-arm copy
                - pre-flight cannot see the other arms of a root-union document that is itself a template (the template is the first slotted arm), so it treats that document as declaring nothing; runtime uses the resolved JSON and is exact. Residual: pre-flight may over-ask in that one case; nothing is read
                - a matched payload whose resolution fails on demand (programmatic registries only) now fails before an earlier template resolves, because every matched payload is collected first
        - follow-up: root-union template document pre-flight (R1-F2.12)
                - defect: a root-union document `$schema` that is itself a template reported only the arm whose slot stopped resolution, and `declared_by_template` treated a union arm as declaring nothing, so pre-flight asked about (and `pending_templates_for` listed) a trigger slot whose property every arm redeclares, while runtime, reading the resolved `anyOf`, discarded it. This closes the residual recorded in the R1-F2 decisions above
                - repair (library): `resolve.rs::pending_union_arm` intercepts the template error at every root-union arm site (inline and referenced arms, document and standalone unions) and marks the template (`Template::mark_union_arm`) with the names every arm declares: arms already lowered from their JSON `properties`, the pending arm from its template, later inline arms from their parsed shape, later referenced arms as they resolve (a template's unevaluated). No slot is evaluated. `apply_active_to_arm` no longer marks the arm itself; the union does, once it knows every arm. New public `PendingSchema::declared_properties()`
                - also fixed: a referenced root-union arm that was a template was reported as a whole single-object template, so a host could have treated that one arm's properties as the union's (and `without_properties` would have pruned that arm alone); it is now marked as a union arm like an inline one
                - repair (darkmatter): `declared_by_template` reads `declared_properties()`, so `discarded_by_higher_layers` gets the same document names before and after resolution
                - tests (red with the fix disabled, except the controls): `dynamic_schema_slot_precedence.rs::template_union_document` (pre-flight asks nothing and reads nothing when every arm redeclares `os`; only the document template is pending; composition succeeds with a pure `enum({{ ['Linux', 'Windows'] }})` slot and the trigger data unread; control: one arm only, pre-flight asks once about `a.trigger.yaml`); `simplified-schema` `dynamic_slots.rs::surviving_properties::a_root_union_arm_template_declares_what_every_arm_declares` (first/later slotted arm, a referenced arm that does and does not declare `os`, a single object)
                - docs: removed the pre-flight limitation bullet from `darkmatter/docs/topics/schemas/schema-slots.md` and stated the exact rule; `simplified-schema/docs/topics/language/slots.md` documents `declared_properties()`; skills `darkmatter/schema.md`, `simplified-schema/SKILL.md` updated
                - decisions: compute the names at the union, when the template error is met, rather than in Darkmatter from the raw `$schema` value (which would be a second reader of arms and miss referenced arms); read later inline arms as parsed (import expansion never adds a top-level property) and resolve later referenced arms, matching what runtime's `declared_by_json` sees in each `anyOf` arm, including empty for an arm that is itself a union
                - verification: darkmatter `just test dynamic_schema_slot_precedence` 21/21, `just test slot` 288/288, `just test trigger` 650/650, `just lint` clean; simplified-schema `just test` 1640/1640 + 1437/1437 minimal, `just lint` clean
- work completed for 'R1-F2: Overridden trigger slots still evaluate' at 12:56:55
- starting the work on 'R1-F3: Inline and trigger slot failures lose authored locations' at 12:56:55
