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
message_to_agent: |-
    Phase 1 added, in darkmatter expression/prepared.rs (re-exported from expression/mod.rs): PreparedInterpolation::new(text),
    .text(), .placeholders(), .view(range); PreparedPlaceholder {span(), text(), offset(), parsed() -> Result<&PreparedExpression,&PreparationError>, expr() -> Option<&Expr> (OnceLock-cached erase)};
    PreparedInterpolationView {text(), range(), parent(), placeholders() -> PlaceholderView items with translated span()/offset()/text()/expr(), placeholder_at(&span), has_literal_syntax()};
    and validate_interpolation(&PreparedInterpolation, &BindingView) -> Result<Vec<ValidationDiagnostic>, PreparationError> (diagnostic spans stay in source coordinates).
    Views assert (panic) on non-char-boundary ranges. Match template holes to occurrences with view.placeholder_at(&hole.span), never by text.
    Characterization tests for the old route are in preflight/tests.rs `static_analysis_characterization` (15 tests, all passing on the unchanged route);
    they must keep passing unchanged after Phase 3 wiring. Observed contract worth keeping: malformed + `err` -> resolution error for the first failing occurrence in order;
    malformed + ctx.agent either order -> SequenceShellTargetIdentity; the late-namespace append pass (`current`) reports binding span None while normal diagnostics carry raw-wrapper spans.
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
