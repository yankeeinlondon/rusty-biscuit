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
