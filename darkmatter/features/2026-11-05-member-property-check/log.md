---
kind: log
feature: 2026-11-05-member-property-check
implementation_1: "2026-11-09T03:38:48-08:00"
---

# Member Property Check: Review Repair Log

## Implementation of Review Findings #1

> **started at:** 2026-11-09T03:38:48-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-member-property-check/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began
- the earlier implementation record is `implementation-log.md` in this directory

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | DMLS omits shell ternary conditions and value branches | fixed | all children fixed or clean; shared `FrontmatterShellTernary::member_pieces` inventory used by library precheck and DMLS `shell_member_diagnostics` |
| R1-F1.1 | Shell condition `$( server.nmae == 'x' ? echo yes : '' )`: composition 1 / editor 1 | fixed | `undeclared_member::library_and_editor_agree::shell_condition` (code, message, severity, line/column at `nmae`; `name` control clean) |
| R1-F1.2 | Shell then value `fm.server.nmae`: composition 1 / editor 1 | fixed | `undeclared_member::library_and_editor_agree::shell_then` |
| R1-F1.3 | Shell else value `fm.server.nmae`: composition 1 / editor 1 | fixed | `undeclared_member::library_and_editor_agree::shell_else` |
| R1-F1.4 | Placeholder `$( echo {{ server.nmae }} )`: 1 / 1, no duplicate | clean | `undeclared_member::library_and_editor_agree::shell_placeholder` still passes; new `undeclared_member::library_and_editor_agree::shell_member_inside_piece_placeholder` proves no duplicate |
| R1-F1.5 | Ordinary frontmatter interpolation and expression-typed values reach shared checker | clean | existing frontmatter and expression-typed agreement/surface tests pass (`undeclared_member::` 27 passed) |
| R1-F1.6 | Body interpolation, block/file conditions, callbacks, region bodies agree | clean | existing body, `::block`/`::file` condition, callback, region agreement rows pass; added `file_condition_beside_placeholder` |
| R1-F1.7 | Lifecycle-stack expressions remain call-only | clean | `undeclared_member::lifecycle_unchanged` passes; stacks excluded in `shell_member_diagnostics` |
| R1-F1.8 | Editor checks unselected shell value branches (passive, no execution) | fixed | `undeclared_member::shell_ternary_pieces_in_the_editor`: both unselected branches warn in the editor, composition 0; parse only, no execution |
| R1-F1.9 | Docs/skill no longer describe the editor shell-piece omission | fixed | omission removed from `docs/topics/darkmatter-expressions.md`, `dmls/docs/diagnostics.md`, skills `dmls.md`, `compose.md` |
| R1-F2 | Evaluation failures discard already-observed member warnings | fixed | all children fixed or clean; reference-check CLI gap decided (see Progress) and pinned by a test |
| R1-F2.1 | Body pass, two interpolations, failing site first: 1 warning retained | fixed | `compose_member_warnings::ac8_same_stage_body_later_site` |
| R1-F2.2 | Body expression `{{ E }}`: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_body_expression` |
| R1-F2.3 | Mixed frontmatter scalar with two interpolations: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_frontmatter_mixed_scalar` |
| R1-F2.4 | Whole-value frontmatter `title: "{{ E }}"`: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_frontmatter_whole_value` |
| R1-F2.5 | Page-block `when="E"`: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_block_condition`, `compose_member_warnings::ac8_same_stage_nested_block_condition` |
| R1-F2.6 | Collection-region `when="E"`: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_collection_gate` |
| R1-F2.7 | `::file` `when="E"`: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_file_condition` |
| R1-F2.8 | `::code` `when="E"`: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_code_condition` |
| R1-F2.9 | Shell condition `$( E ? … )`: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_shell_condition` |
| R1-F2.10 | Shell then value: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_shell_then_value` |
| R1-F2.11 | Shell else value: 1 retained | fixed | `compose_member_warnings::ac8_same_stage_shell_else_value` |
| R1-F2.12 | `::map` callback: clean (1) | clean | `compose_member_warnings::ac8_same_stage_map_callback` (regression test kept) |
| R1-F2.13 | `::filter` callback: clean (1) | clean | `compose_member_warnings::ac8_same_stage_filter_callback` |
| R1-F2.14 | `filter=` callback: clean (1) | clean | `compose_member_warnings::ac8_same_stage_filter_option` |
| R1-F2.15 | `sort=` callback: clean (1) | clean | `compose_member_warnings::ac8_same_stage_sort_option` |
| R1-F2.16 | `while=` callback: clean (1) | clean | `compose_member_warnings::ac8_same_stage_while_option` |
| R1-F2.17 | `until=` callback: clean (1) | clean | `compose_member_warnings::ac8_same_stage_until_option` |
| R1-F2.18 | Later-stage failure (`ac8_later_failure`): clean | clean | `compose_member_warnings::ac8_later_failure` passes |
| R1-F2.19 | Warnings collected before a later shell preparation/execution failure are retained | fixed | `compose_member_warnings::ac8_same_stage_shell_later_preparation`, `compose_member_warnings::shell_later_execution`; siblings fixed: `frontmatter_later_key`, `frontmatter_later_item`, `directive_target`; `denial_still_blocks_first` keeps blocking |
| R1-F2.20 | Structured CLI failure case keeps earlier diagnostic with empty stderr | fixed | CLI `compose_member_warnings::a_failed_composition_keeps_the_earlier_member_warning` (JSON, exit 1, empty stderr); pre-composition reference-check failure pinned by `a_reference_check_failure_reports_no_member_warning` |
| R1-F3 | Interpolation hides other member reads in a shell expression | fixed | all children fixed or clean; `AuthoredExpression` join in `member_sites.rs` |
| R1-F3.1 | Condition with `'{{ expected }}'` beside `server.nmae`: 1 at `nmae` | fixed | `compose_member_warnings::a_placeholder_does_not_hide_a_shell_piece_member` (failed before repair) |
| R1-F3.2 | Then value with placeholder beside `fm.server.nmae`: 1 at `nmae` | fixed | same test |
| R1-F3.3 | Else value with placeholder beside `fm.server.nmae`: 1 at `nmae` | fixed | same test |
| R1-F3.4 | Controls with literal `web`: 1 each | clean | same test loops `web`/`{{ expected }}` fillers and `nmae`/`name` spellings |
| R1-F3.5 | Member inside placeholder: 1, no duplicate | clean | `a_placeholder_in_a_frontmatter_shell_value_is_a_frontmatter_interpolation_site`, `ac7_surfaces_shell_condition_and_branch` pass (exactly 1) |
| R1-F3.6 | Unselected ordinary branch: no composition warning | clean | existing unselected-branch test passes; added unselected else branch holding a placeholder (0) |
| R1-F3.7 | `--deny dm.expression.undeclared_member` blocks the placeholder-adjacent typo | fixed | `compose_member_warnings::a_placeholder_does_not_bypass_member_denial` (library `BlockingPolicy` deny; composed successfully before repair); sibling `a_placeholder_does_not_hide_a_condition_member` (`::file when=`) |
| R1-F4 | Loading-data skill guidance still says member typos are silent | fixed | obsolete sentence replaced |
| R1-F4.1 | Remove/replace the obsolete sentence in the skill's loading-data section | fixed | `.claude/skills/darkmatter/expressions.md` typed-literal-returns bullet |
| R1-F4.2 | Sweep docs/skills for other "member typos are silent" statements | clean | grep sweep of docs, READMEs, darkmatter/use-darkmatter/use-dmls skills: no other contradicting statement |

### Progress

- order chosen: R1-F3 (library shell pieces) first, then R1-F1 (editor shell pieces, agreeing with the repaired library), then R1-F2, then R1-F4; F1 and F3 share the shell-piece inventory, so repairing the library side first lets the editor tests assert agreement on the F3 rows too
- starting the work on 'R1-F3: Interpolation hides other member reads in a shell expression' at 03:39:18
    - defect class: a whole expression is excluded from member analysis because it contains an independently checked nested interpolation
    - reproduced first: new acceptance tests failed before the change (shell pieces 0 warnings; `--deny` composed successfully; `::file when="server.nmae != '{{ expected }}'"` 0 warnings, a sibling instance of the class)
    - repair: `MemberCandidate.expression` is now an `AuthoredExpression` (`member_sites.rs`); it matches the evaluated text exactly, or with each `{{ }}` placeholder that lies inside one string-literal token filled by text holding neither that quote nor a backslash, which keeps the token structure and so the access-node ordinals; `record_shell_members` no longer skips pieces containing `{{`
    - sibling sites: `::file`/`::code` `when=` fixed by the same join (body interpolation fills the attribute before transclusion); `::block` `when=`, collection gates and callbacks clean (evaluated before body interpolation, authored text joins exactly; tests added); frontmatter/body `{{ }}` interpolation clean by grammar (a placeholder cannot nest inside another; each is its own leaf/body site)
    - tests: `compose_member_warnings::{a_placeholder_does_not_hide_a_shell_piece_member, a_placeholder_does_not_bypass_member_denial, a_placeholder_does_not_hide_a_condition_member}`, unit `member_sites::tests::{an_in_string_placeholder_matches_its_filled_text, a_placeholder_outside_a_string_matches_only_unfilled}`
    - docs: `docs/topics/darkmatter-expressions.md` (placeholder paragraph), skill `compose.md` join/routing bullets; the editor sentence in the topic page is left for R1-F1
    - verification: `just test member` 172 passed; `just test shell` 1219 passed; touched-module and doc-reader filters passed; `just lint` clean
    - decisions recorded for human review: authored-text template join (in-string placeholders only; fills with the enclosing quote or a backslash, and placeholders outside strings, are not joined and stay silent); R1-F3.7 evidenced at library level only (CLI shell approval is not cheaply scriptable)
- work completed for 'R1-F3: Interpolation hides other member reads in a shell expression' at 03:52:26
- starting the work on 'R1-F1: DMLS omits shell ternary conditions and value branches' at 03:52:26
    - defect class: an editor inventory of authored expression sites omits a site kind that composition observes
    - reproduced first: the six new shell agreement rows failed with the editor entry point disabled (composition 1 / editor 0)
    - repair: one shared inventory, `FrontmatterShellTernary::member_pieces` (library), used by both the precheck's `record_shell_members` and the new DMLS `diagnostics/frontmatter.rs::shell_member_diagnostics`; it drops a piece whose placeholder lies outside a string (`placeholders_keep_expression_structure`, built on the R1-F3 `AuthoredExpression` rule), so both sides check the same pieces; DMLS checks pieces member-only (`call_check::member_diagnostics`), as composition does not call-check them, projects spans through the frontmatter `ScalarProjection`, and checks both value branches; top-level keys only, `$schema` and lifecycle stacks excluded
    - sibling sites: frontmatter leaf `{{ }}`, body `{{ }}`, `::block`/`::file`/`::code` `when=`, collection gates/callbacks/region bodies all already reach the editor's shared checker (existing agreement rows); expression-typed values are editor-only by approval; lifecycle stacks stay call-only; `::file when=` beside an in-string placeholder (R1-F3 sibling) added as an agreement row
    - tests: `undeclared_member::library_and_editor_agree::{shell_condition, shell_then, shell_else, shell_condition_beside_placeholder, shell_then_beside_placeholder, shell_else_beside_placeholder, shell_member_inside_piece_placeholder, file_condition_beside_placeholder}`, `undeclared_member::shell_ternary_pieces_in_the_editor` (unselected branches, outside-string placeholder skipped, pipeline silent)
    - docs: `docs/topics/darkmatter-expressions.md` editor paragraph, `dmls/docs/diagnostics.md` member row, skills `dmls.md` and `compose.md`
    - verification: DMLS `test(member) | test(shell) | test(agreement)` 75 passed; `just test member` 181, `just test shell` 1227, `just test frontmatter_shell` 160, `just test diagnostics` 292 passed; doc readers and compose doctests passed; `just lint` clean
    - decisions recorded for human review: the placeholder-position filter lives in the shared inventory (so library and editor cannot drift); the editor warns on in-string-placeholder pieces regardless of the runtime fill value (it cannot know the fill), an accepted editor superset like unselected branches
- work completed for 'R1-F1: DMLS omits shell ternary conditions and value branches' at 04:02:42
- starting the work on 'R1-F2: Evaluation failures discard already-observed member warnings' at 04:02:42
    - defect class: an error is propagated with `?` before the warnings the failing step had already collected are handed to the document report
    - reproduced first: 17 new rows failed with 0 retained warnings (body site/expression, frontmatter mixed/whole-value/later key/later list item, `::block` and nested `::block`, collection gate, `::file`/`::code` `when=`, shell condition/then/else, later shell preparation, directive target); callback rows (`::map`/`::filter` positional, `filter=`/`sort=`/`while=`/`until=`) and `ac8_later_failure` already passed
    - repair: `LocatedInterpolationError` and `LocatedFrontmatterError` carry `warnings` (the rewrite's collected warnings in authored order; a policy stop keeps none, as it already carries them); `interpolate_text_selected`, the whole-value path, `rewrite_directive_targets`, frontmatter leaves/lists/objects/keys attach them; the body stage (`anchor_authored_failure`) and `interpolate_frontmatter_pass` add them to the report before converting the error; `execute_frontmatter_shell_expansion_into` reports to a caller-owned `warnings` vector, which the pipeline adds on failure; `::block`, collection-gate, and transclusion conditions record their observation before returning the evaluation error (a transclusion skips its `ReportWatch` on failure so the evaluation error still wins); a failing ternary piece reports its own members (`FailingPiece::keep`)
    - sibling sites: body `::shell`/shell blocks clean (each command's warnings reach the report before the next runs); callbacks clean (pending flush on failure, tests kept); call precheck clean (`PrecheckFailure` carries `earlier`); subtree compose, prepared values, lifecycle `interpolate_text_in`, and the ternary pieces' inner `interpolate_text` keep no report and install no member sites; heading relevel and policy-file warnings precede every `?`; volatile-cache note in `execute_prepared_directive` unreachable on failure within one run (the first failing execution ends the run)
    - root `dm.expression.undeclared_property`: not the same accumulator (report candidates, reconciled only at document end on success), so a failed composition never reports it at any stage; unchanged, conditions now record their candidates before failing (harmless)
    - found: `md compose` runs a passive reference check first; a failing body/frontmatter `{{ }}` or `::block` condition fails there, before composition, and the check reports no member warning, so the CLI loses the warning (also for the `ac8_later_failure` shape); not fixed (choice of alternatives), documented, reported to the orchestrator
    - tests: lib `compose_member_warnings::{ac8_same_stage_*}` (21 macro rows + `shell_later_execution`, `directive_target`, `denial_still_blocks_first`), CLI `compose_member_warnings::a_failed_composition_keeps_the_earlier_member_warning` (`::file when=`, JSON, empty stderr)
    - docs: `docs/topics/darkmatter-expressions.md` "When composition fails"; skill `compose.md` known-limit bullet replaced
    - verification: `just test member` 206, `just test failure` 437, `just test shell` 1232 passed; touched-module filter 1830 passed; CLI compose/failure/warning/member/output 1053 passed; claudine `invalid_trigger|composition` 2325 and claudine-cli `composition_seams` 19 passed; `just lint` clean
    - orchestrator decision on the reference-check gap: keep the pre-composition reference check passive (the spec requires passive discovery and that discovery-only candidates never escape into a composition report; a failure there precedes any composition observation, so no earlier member warning exists to keep); the alternatives (defer the check's expression failures to composition, or let the check record member warnings) change the CLI's error source or violate passivity; pinned by CLI `compose_member_warnings::a_reference_check_failure_reports_no_member_warning` (passes) and recorded for human review
    - other decisions recorded for human review: a failing shell ternary piece reports its own members though the branch is not yet selected; a failing `::file`/`::code` condition's evaluation error wins over a `--deny` stop (matches interpolation's ordering); warnings travel on the two crate-internal located-error types plus a shell-stage out-parameter
- work completed for 'R1-F2: Evaluation failures discard already-observed member warnings' at 04:25:50
- starting the work on 'R1-F4: Loading-data skill guidance still says member typos are silent' at 04:25:50
    - defect class: current guidance for a typed value contradicts the behavior it describes after that behavior changed
    - repair: `.claude/skills/darkmatter/expressions.md` typed-literal-returns bullet now points at the skill's **Undeclared member check** paragraph and states that `h.nmae` warns while named references and computed contracts stay silent
    - sweep: grep of `darkmatter/docs`, `darkmatter/dmls/docs`, READMEs, and the darkmatter/use-darkmatter/use-dmls skills for "silent"/"not checked" member statements; `docs/topics/loading-data.md`, `docs/inline/interpolation.md`, and `docs/inline/fm-interpolation.md` already describe the warning (clean)
- work completed for 'R1-F4: Loading-data skill guidance still says member typos are silent' at 04:26:15
- orchestrator reconciliation at 04:30:25: inspected the combined diff (`member_sites.rs` join, DMLS `shell_member_diagnostics`, CLI tests); every ledger row has a disposition and evidence; ran the area gates once with no subagent running

### Completion Report

- **outcome: complete** (started 03:38:48, finished 04:30:25; elapsed about 51 minutes)
- parent findings: total 4 = fixed 4 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 38 children, 25 fixed, 13 clean, 0 open
- verified repairs:
        - R1-F3: an authored-text join (`AuthoredExpression`) keeps ordinary member reads in a shell piece or `::file`/`::code` condition that also holds an in-string `{{ }}` placeholder; denial can no longer be bypassed by adding a placeholder
        - R1-F1: DMLS checks shell ternary conditions and both value branches through the same shared inventory as composition, with authored spans and the lifecycle exclusion kept
        - R1-F2: every stage that collects member warnings hands them to the failure report before propagating its error (interpolation, frontmatter keys/items, directive targets, `::block`/collection/transclusion conditions, frontmatter shell stage)
        - R1-F4: the skill's loading-data guidance now matches the member check
- open findings: none
- decisions made for the author's later confirmation (also in the review's `human_review_items`): in-string placeholder join rule; shared placeholder-position filter and editor superset for fill values; CLI reference-check failure reports no member warning (kept passive); failing shell piece reports its own members; failing `::file`/`::code` condition error wins over a deny stop; R1-F3.7 denial evidenced at library level
- no performance measurement was required; no cross-OS check run (no path, process, or OS-specific code changed)
- changed files:
        - library: `darkmatter/lib/src/markdown/compose/{member_sites.rs, call_precheck.rs, frontmatter_shell_expansion.rs, mod.rs, interpolation/rewrite.rs, inline/interpolation.rs, frontmatter_interpolation.rs, directive_targets.rs, pipeline/mod.rs, page_blocks/engine.rs, collection_blocks/engine.rs, transclusion/engine.rs}` (R1-F1, R1-F2, R1-F3)
        - editor: `darkmatter/dmls/src/diagnostics/{frontmatter.rs, call_check.rs}` (R1-F1)
        - tests: `darkmatter/lib/tests/l1/compose_member_warnings.rs` (R1-F2, R1-F3), `darkmatter/dmls/tests/l1/undeclared_member.rs` (R1-F1), `darkmatter/cli/tests/l1/compose_member_warnings.rs` (R1-F2.20)
        - docs and skills: `darkmatter/docs/topics/darkmatter-expressions.md`, `darkmatter/dmls/docs/diagnostics.md`, `.claude/skills/darkmatter/{compose.md, dmls.md, expressions.md}` (R1-F1, R1-F2, R1-F3, R1-F4)
- final verification (from `darkmatter/` unless noted):
        - `just test`: 16248 passed, 10 skipped, exit 0 (all findings)
        - `just lint`: clean, exit 0 (all findings)
        - `just check-tier-coverage darkmatter` (repo root): 0 stranded (all new tests)
        - `cargo nextest run -p darkmatter-cli -E 'test(compose_member_warnings::)'`: 7 passed (R1-F2.20)
        - subagent-scoped runs recorded above, including claudine `invalid_trigger|composition` 2325 passed and claudine-cli `composition_seams` 19 passed (consumers of failure warnings)
