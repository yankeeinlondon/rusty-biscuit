---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-27-shell-site-placeholder-parse-once/spec.md"
plan: "claudine/features/2026-10-27-shell-site-placeholder-parse-once/plan.md"
implemented_by: "claude/sonnet"
started_phase: 1
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scaling.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - darkmatter
    - claudine
decisions_for_review:
    - |-
        PreparedPlaceholder stores its own expression offset instead of reading it from the retained failure.
        The plan said each occurrence stores the range, trimmed text, offset and a Result of prepared expression or error.
        I kept the offset as a separate field on the occurrence so a failed occurrence still reports where its text starts,
        rather than adding a field to the public PreparationError struct (which would break any outside constructor).
        Alternative: extend PreparationError with an offset; costs an API-breaking field. Redo cost if reversed: small, confined to prepared.rs.
    - |-
        The collection exposes occurrences through a small PlaceholderView type (re-exported with the view type) that translates
        spans and offsets by the view start, plus placeholder_at(span) and has_literal_syntax().
        The plan fixed only view(range) and said iteration translates coordinates; it did not name the item type or the helper methods.
        Alternative: return translated copies of PreparedPlaceholder; that would clone failures and the erased-tree cache.
        Redo cost if reversed: Phase 2 consumers only use these three methods.
    - |-
        Presence-lookup and unguarded-scope guard tests were adjusted rather than the new code reshaped.
        The type-scope ownership guards keyed their allowlists on function names that Phase 2 split: `Parser::take_hole` became a shared `SchemaFree::scope`, and `placeholder_guard` now delegates to `placeholder_guard_expr`.
        I updated Darkmatter's allowlist entry and made Claudine's guard accept `placeholder_guard_expr` wherever it requires `placeholder_guard`, so Phase 3 can call the parsed form.
        Alternative: have Phase 3 keep calling the text form (defeats the parse-once goal). Redo cost if reversed: two small edits in the guard tests.
    - |-
        Distinct-hole and malformed static-analysis count tests mirror the analysis calls in a test helper instead of calling `prepare_site` directly.
        The plan asked for a helper that builds the collection and runs the same static helpers without evaluation; `prepare_site` also evaluates, so its count cannot isolate the static passes.
        The helper (`static_parses` in preflight/tests/scaling.rs) repeats the call sequence, so it can drift from `prepare_site`; the exact 6,000 full-preflight count is the guard against that.
        Alternative: expose a pub(crate) analysis function used by both; cleaner but changes production structure beyond the plan. Redo cost if reversed: moderate, confined to prepare_site and the helper.
    - |-
        A conditional `set` value is rejected inside prepare_site (`Err(TemplateGrammarError::Conditional)`) rather than by the old `authored_shell_template` wrapper.
        The plan said to extract first and build the template with the parsed counterpart; it did not say where the conditional check lives.
        I kept the observable behavior identical (same error variant, same precedence) by checking `conditional` before building the template.
        Alternative: add a `parse_authored_command` helper in Darkmatter; avoids duplicating the rule but widens Darkmatter's API. Redo cost: small.
source_files_during_phase_4: []
docs_updated_during_phase_4:
    - claudine/docs/topics/pre-flight-checks.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/topics/expression-engine.md
docs_created_during_phase_4:
    - claudine/features/2026-10-27-shell-site-placeholder-parse-once/implementation.md
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
source_code:
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/tests/l1/type_scope_ownership_guard.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scaling.rs
documentation:
    - claudine/docs/topics/pre-flight-checks.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/topics/expression-engine.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
    - claudine/features/2026-10-27-shell-site-placeholder-parse-once/implementation.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-10-27-shell-site-placeholder-parse-once (4 phases)

## Phase 1

- Wave 1 / Retain occurrence syntax: added `PreparedInterpolation`, `PreparedPlaceholder`, `PreparedInterpolationView`, `PlaceholderView`, `validate_interpolation` in `darkmatter/lib/src/markdown/compose/expression/prepared.rs`; re-exported from `expression/mod.rs`.
  `scan_leaf` now shares a lazy `parse_occurrences` helper with the new collection (first-error short circuit preserved: counter test shows `prepare_value` parses up to and including the first failure). `validate_prepared` and `validate_interpolation` share `expression_diagnostics`.
- Unit tests (`prepared.rs::interpolation_tests`, 12 tests incl. 3 under `work-counters`): mixed valid/invalid retention, `prepare_value` first-error compatibility, repeated text at distinct spans, cached erasure, no placeholders / triple braces / literal tokens, Unicode view offsets, view literal-syntax containment, non-boundary range panic, dead-branch diagnostics at source coordinates, failure suppresses diagnostics, exact parse counts (construction = occurrences incl. failures; views/erasure/validation = 0).
- Wave 1 / Characterize precedence: `claudine/lib/src/composition/sequence/preflight/tests.rs::static_analysis_characterization` (15 tests) run against the unchanged route and passing: malformed alone / beside `err` (both orders) / beside target identity (before, after, between; dotted and static-bracket) / unrelated ctx+env; first-task program hole exact vs later-task grammar refusal; optional invalid template exact vs required refusal; late read beside malformed; missing value and empty/zero/false presence; padded + Unicode `set` spans (template spans in extracted-command bytes, diagnostic spans in raw-wrapper bytes); accepted suffixes; late-namespace span `None`; dead-branch unknown function; malformed `set` wrapper.
- Input Robustness Matrix: not applicable (text grammar, per plan rule 13).
- Gates (run once, no subagents running): `just lint` and `just test` in `darkmatter/` (12,986 passed, 10 skipped) and in `claudine/` (11,782 passed, 15 skipped); `just check-tier-coverage` claudine and darkmatter: 0 stranded. Lint output shows only a pre-existing linker `__eh_frame` warning. `work-counters` is enabled by both recipes, so the counted tests ran.
- Dependent packages: the Darkmatter change is additive (new types and one new function; `prepare_value` behavior unchanged and covered by the first-error test), and Claudine, its only consumer in this feature, passed in full. No other dependent was run.
- No cross-OS check run: no OS-specific code or paths added.
- Skills/docs: none changed in Phase 1; documentation is Wave 5.

## Phase 2

- Wave 2 / Add parsed consumers (darkmatter): `grammar.rs` now has one shared `build` body driven by a typing callback; `parse_authored` (lazy text typing, unchanged parse behavior) and new `parse_authored_prepared(view)` (typing from retained syntax, fallback `NotBareProperty` for failed occurrences, `InterpolationLiteral` from `view.has_literal_syntax()`). `typing.rs::project_hole` delegates to new `project_hole_expr`. `CommandTemplate::project_types_prepared` matches holes by span. `DeferredShellRoots::late_roots_of` delegates to `late_roots_of_expr` (same traversal order, adjacent dedup). `placeholder_guard` delegates to `placeholder_guard_expr`. `authored_shell_command` extracted in `frontmatter_shell_expansion.rs` (span computed from slice pointer offsets into the original value; no suffix validation); `authored_shell_template` delegates; both exported from `compose/mod.rs`.
- Wave 2 / Adapt host policy (claudine): `approval_diagnostics_prepared` (normal diagnostics via `validate_interpolation`, then the unchanged late-namespace append pass, span None); `approval_diagnostics` constructs the collection and delegates. `first_target_identity_root_prepared` skips failed occurrences, never suppressed by them; text entry delegates.
- Tests: `template/tests.rs::prepared_parity` (8 tests: grammar results for templates and every rejection incl. Unicode/literal tokens/empty, fallback typing for failed occurrence, schema validators + guard refinements via project_types parity, repeated text by span, padded/Unicode `set` extraction with span in original bytes, extraction vs template API for conditional/suffix/non-shell, late-root order, presence guard parity guarded/unguarded); `bindings/tests.rs` (diagnostic parity incl. dead branches, unknown function, malformed suppression; first-error parity); `preflight/tests.rs::prepared_target_identity_scan_matches_the_text_scan` (malformed beside valid read before/after, dotted vs static bracket, computed index, unrelated ctx/env).
- Guard fallout: both `type_scope_ownership_guard` tests keyed on the split functions; updated (see decisions_for_review). Introduced private `SchemaFree` in grammar.rs as the single schema-free `TypeScope::unguarded` site.
- Gates (once, serial, no subagents): `just lint` and `just test` in `darkmatter/` (12,994 passed, 10 skipped) and `claudine/` (11,785 passed, 15 skipped); lint shows only the pre-existing linker `__eh_frame` warning. No tier markers added, so `check-tier-coverage` not rerun. The affected-scope command was not used to skip anything: source files changed, gates run.
- Input Robustness Matrix: not applicable (text grammar). No cross-OS check: no OS-specific code or paths.
- Subagents: none used.

## Phase 3

- Wave 3 / Wire site preparation: `prepare_site` now extracts the authored command with `authored_shell_command`, keeps the `!raw.contains("{{")` early path, then builds one `PreparedInterpolation::new(raw)` and two views (raw `0..len`, authored `command.span`). `refuse_unapprovable_reads` takes the collection and calls `approval_diagnostics_prepared` and `first_target_identity_root_prepared`. The template is `parse_authored_prepared(authored_view)` + `project_types_prepared`; `mark_late_holes` takes the authored view and correlates holes by span via `late_roots_of_expr`; the Deferred->Guarded retime and missing-value guard lookup use `placeholder_guard_expr` (missing values use the first raw occurrence with the same trimmed text); the failed-grammar fallback holes iterate the raw view. Order of mode selection and error branches unchanged. Collection is dropped on every return and never stored. Doc comment of `mark_late_holes` had a stray merged docblock (the `overlay_derived` summary sat above it); fixed while rewriting that function.
- Source search: no `parse`, `prepare_value`, `project_hole`, `late_roots_of` (text), or `placeholder_guard` (text) call remains in `prepare_site` or its helpers. `placeholder_guard` text remains only in `guard_proves_whole_value` (frontmatter set path, out of scope).
- Wave 4 / Prove work counts (`preflight/tests/scaling.rs`, work-counters): `shell_preflight_parses_exactly_steps_times_holes_times_six_at_10_and_100_steps` (60 and 6,000, with `assert_ten_hole_graph`), `the_parse_count_follows_the_hole_count` (4 steps x 3 holes = 72), `static_analysis_parses_each_occurrence_once` (8 cases: repeated text 3, deferred 2, failed grammar beside late read 1, malformed beside valid 2, malformed beside late 2, plain/triple-brace/data-token 0). Snapshot deltas of existing counters.
- Wave 4 / Preserve observable behavior: Phase 1 characterization tests (15) and the existing preflight, executor and CLI cases pass unchanged against the new route. Added `prepared_site_behavior` in `preflight/tests.rs`: placeholder commands in setup, teardown, `on_error:` and a padded `set` value resolve; a template prepared once re-reads `fixture.md` (`draft` then `ready`) on each fill through `LookupEvaluator` and a document resolution context (retained syntax does not freeze values). Existing executor tests `a_shape_approved_shell_fills_from_the_state_current_when_it_runs` / `an_exact_approval_runs_the_stamped_bytes_and_never_rereads_the_value` and CLI `a_later_task_is_approved_only_by_its_shape` retained as the resolved-byte and CLI proofs.
- Requirement-to-test: parse count -> scaling tests above; precedence/errors/presence/coordinates -> `static_analysis_characterization`; lifecycle positions + no value freezing -> `prepared_site_behavior`; CLI wiring -> existing `sequence_shell_templates` L1.
- Gates (once, serial, no subagents): `just lint` in `claudine/` clean; `just test` in `claudine/`: 11,789 passed, 15 skipped (one test >5 s on the loaded full run, `sequence_task_providers::empty_hint_setter_list_uses_the_missing_provider_gate`, unrelated, pre-existing slow under full-suite load). Counter tests ran (both recipes enable `work-counters`). Darkmatter untouched this phase, so its gates were not rerun; no dependents of claudine's changed private code. No tier markers added, no OS-specific code, so no cross-OS check.
- Input Robustness Matrix: not applicable (text grammar). Subagents: none used. Docs and skills are Phase 4 (Wave 5).

## Phase 4

- Wave 5 / Document syntax lifetime: added a "One syntax collection per shell site" section (example, Mermaid flow, wrapper coordinate mapping, drop-at-return lifetime, retained text entry points) to `claudine/docs/topics/pre-flight-checks.md`; a "Parsed-once syntax for a host" section to `darkmatter/docs/inline/preflight-checks.md`; short sections in `darkmatter/docs/topics/expression-engine.md`, `.claude/skills/darkmatter/expressions.md` and `.claude/skills/claudine/SKILL.md`. READMEs and dependencies unchanged (public behavior unchanged). No doc links to this feature.
- Wave 5 / Audit static reuse: searched `preflight/mod.rs` for text parsers. The only remaining hit is `guard_proves_whole_value` (frontmatter `set` path, scan_plain + placeholder_guard), intentionally out of scope. `PreparedInterpolation` is constructed at one site (`prepare_site`), is passed by reference, and is stored in no template, candidate, approval record, or field. No source defects found; no source files changed in Phase 4.
- Gate-skip check: `affected_scope.py` returned `packages: ["claudine-cli","darkmatter"]`, `change_class: package` (docs are test inputs), so the gates ran.
- Wave 6 / Gates (serial, nothing else running): `just test` + `just lint` in `darkmatter/` (12,994 passed, 10 skipped; lint clean) then in `claudine/` (11,789 passed, 1 slow, 15 skipped; lint clean). Counter tests ran (recipes enable `work-counters`).
- A6 evidence on this host (macOS 27.2, release bench profile) is in `implementation.md`: 100-step allocation row 6,000 parses, 25,588,809 bytes, 195,225 allocs; Criterion median 7.5963 ms.
- Input Robustness Matrix: not applicable (text grammar). Subagents: none. No cross-OS check (no OS-specific code).
