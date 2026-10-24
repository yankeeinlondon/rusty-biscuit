---
implementation_1: "2026-10-24T10:12:18-07:00"
implementation_2: "2026-10-24T10:22:19-07:00"
implementation_3: "2026-10-24T11:10:02-07:00"
implementation_4: "2026-10-24T11:22:29-07:00"
---

# Review-to-Implement Log: Mermaid Embedded Font

## Implementation of Review Findings #1

> **started at:** 2026-10-24T10:12:18-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- inventory of `review-1.md` (read in full: frontmatter, body, both tables, deferred-work section)
        - `findings: []`, `recurrence: false`, `has_blocked_findings: false`, `blocked: false`
        - the body states the review "stops at the requested first-review decision checkpoint": no implementation-to-spec comparison, no code review, and no test execution were performed, and "there are no findings yet"
        - the five `human_in_the_loop` entries (mirrored in `human_review_items`) are design rulings addressed to the author, each with options and an "Expected ruling"; they are not repair requests
        - the "Other recorded decisions" table is explicitly "not independently justify[ing] stopping" and "not endorsements"; it requests no change
        - the 42 Darkmatter JSON-order failures are explicitly "not … classified … as findings"
- `implemented` on the review was already `false` before work started; left `false` until the closing step
- checked the one prerequisite a ruling depends on: `darkmatter/cli/src` contains no `--feature` flag and no `embed-mermaid-font` wiring, so the shared `--feature` convention of `2026-10-22-merman-text-fallback` has still not landed

### Finding Ledger

The review raises zero findings, so the ledger has no repair rows. The author's decisions are listed below so they stay visible. Acting on any of them now would make the ruling the review reserves for the author; for example, adding the CLI flag here is itself one of the options in decision 4.

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| — | No findings (`findings: []`; body: "There are no findings yet") | n/a | review frontmatter line 11; body § intro |
| HITL-1 | Typography of `allsorts` subsets (no `GPOS`/`GSUB`) vs. whole fonts vs. other subsetter | author decision, not a finding | awaits author ruling; no code change is valid until it is made |
| HITL-2 | Two-weight (regular + bold) `fsType` check vs. only the weights used | author decision, not a finding | awaits author ruling |
| HITL-3 | Static-TrueType-only embedding (variable/CFF → Inter) | author decision, not a finding | awaits author ruling |
| HITL-4 | Unfinished `md --feature embed-mermaid-font` option and its two tests | author decision, not a finding | awaits author ruling; the prerequisite `--feature` flag is still absent from `darkmatter/cli/src` (grep, 2026-10-24) |
| HITL-5 | AC-E1 measurement: advance widths vs. 1 px bbox tolerance vs. 0.5 px bbox | author decision, not a finding | awaits author ruling |

- no defect class to sweep: with no finding there is nothing to generalize, and the review explicitly withheld a code review
- no source, test, or doc files changed in this cycle, so no `just test`/`just lint` run is required to verify a repair; none was run

### Completion Report

- **outcome:** complete; there are no findings to repair (elapsed about 5 minutes)
- parent findings: `total 0 = fixed 0 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0`
- open finding IDs: none
- still outstanding, for the **author** (not implementer work): the five `human_in_the_loop` rulings HITL-1 to HITL-5 above; the review's `human_review: true` and `human_review_items` are preserved unchanged
        - the spec is not finished: the CLI option (R-E1, AC-E9's CLI test) is still unbuilt pending HITL-4, so "complete" here describes only this repair cycle, not the feature
        - next action: the author records the rulings; the next review then does the full implementation-to-spec comparison and produces the finding inventory
- changed files: this log and `review-1.md` frontmatter (`log`, `implemented`, `implemented_by`) only
- verification commands: `grep -rn 'long = "feature"\|embed-mermaid-font' darkmatter/cli/src` → no matches (supports HITL-4's blocker); no test or lint runs were needed because no code changed

## Implementation of Review Findings #2

> **started at:** 2026-10-24T10:22:19-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- inventory of `review-2.md` (read in full: frontmatter `findings`, the five `human_review_items`, the previous-finding table, every finding's child table, blockers, recurrence, input robustness, commands)
        - five parent findings: R2-F1, R2-F2 (unblocked, high); R2-F3, R2-F4 (blocked, high); R2-F5 (blocked by the review, medium)
        - `implemented` on the review was already `false`; kept `false` until the closing step
        - prerequisite check for R2-F3: `2026-10-22-merman-text-fallback` is still `implemented: false`, and `git grep 'long = "feature"'` finds nothing under `darkmatter/` on this branch or on `main`

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Whole-font embedding retains shaping that layout disables | fixed | all children verified; see R2-F1.1–1.9 |
| R2-F1.1 | regular whole-font output: layout width = emitted file's default-feature shaping | fixed | `the_opt_in_measures_each_weight_as_its_emitted_file_draws` (0x0100/0x0100, 400) and `an_opt_in_label_in_a_whole_face_is_laid_out_kerned`; failed before (176.625 vs 160.21875 px) |
| R2-F1.2 | bold whole-font output: same | fixed | same test, 0x0100/0x0100 at 700 against the emitted bold file |
| R2-F1.3 | only regular forbids subsetting: regular shaped, bold plain | fixed | same test, 0x0100/0x0000: regular has `GPOS` and kerns; bold subset width = hmtx |
| R2-F1.4 | only bold forbids subsetting: bold shaped, regular plain | fixed | same test, 0x0000/0x0100 (inverse) |
| R2-F1.5 | neither forbids subsetting: plain advances kept (clean control) | clean | 0x0000/0x0000 case; `the_measurer_uses_plain_advances_with_the_opt_in` passes |
| R2-F1.6 | collection member forbidding subsetting: same shaping as standalone whole face | fixed | two `.ttc` cases in the same test: emitted rebuilt member has `GPOS`, measured = drawn |
| R2-F1.7 | non-opt-in targets (plain HTML-static SVG, raster-safe SVG, PNG) unchanged (clean control) | clean | `just test golden_tests` 209 passed, no golden changes; non-opt-in measurers keep `layout_tables = true` |
| R2-F1.8 | other consumers of the measurer (gitGraph tag spacing in `prepare`) follow the same per-face rule | clean | sweep: main render and gitGraph tag measurer both use `shape` → `face_for`; no other `ShapeParams` builder |
| R2-F1.9 | comments and topic docs describing "plain advances for every opt-in" | fixed | rustdoc (`measure.rs`, `render.rs`, `subset.rs`, `shaper.rs`), both `docs/` pages, both skill files |
| R2-F2 | Reusing one face for two weights drops bold-only characters | fixed | all children verified; see R2-F2.1–2.8 |
| R2-F2.1 | one installed face for both weights: a 700 rule covering the bold characters is emitted | fixed | `one_installed_face_carries_both_weights_rules_fresh_and_cached` (rules 400+700, 700 maps `Z`,`b`), `one_face_serving_both_weights_gets_a_rule_per_weight_drawn`; both failed before |
| R2-F2.2 | same through a cache hit (sidecar → `page_font`) | fixed | same render test: second render is `cache_hit`, `page_font` CSS identical |
| R2-F2.3 | single-face regular-only flowchart: one rule (clean control) | clean | `one_installed_face_drawn_regular_only_gets_one_rule` |
| R2-F2.4 | separate regular and bold faces: two rules, fresh and cached (clean control) | clean | `separate_regular_and_bold_faces_get_a_rule_each_fresh_and_cached` |
| R2-F2.5 | same family and weight across diagrams / aliases: still one rule with the union (clean control) | fixed | `requested_families_choosing_one_family_share_its_rules_and_union` (failed before on the single-face chain); existing union tests pass |
| R2-F2.6 | bold-only use of a shared ID: one 700 rule (clean control) | clean | `bold_only_use_of_a_face_serving_both_weights_embeds_one_bold_rule` (passed before, by control flow) |
| R2-F2.7 | sibling sweep: `page_font` dedupe, `glyphs.rs` family/weight keys, comment/OFL once per family | clean | `page.rs` dedupe compared by choice (same chain ⇒ same choice), collector keys on leading family and splits at 600; `inter_is_declared_as_inter_with_its_ofl_line_immediately_before` passes |
| R2-F3 | The specified command-line opt-in is still unavailable | blocked | prerequisite shared `--feature` flag (`2026-10-22-merman-text-fallback`, `implemented: false`) has not landed; no scope change recorded |
| R2-F3.1 | `md --feature embed-mermaid-font` parsing and wiring | blocked | no `long = "feature"` under `darkmatter/` on this branch or `main`; a standalone flag would contradict D3 |
| R2-F3.2 | CLI test and frontmatter-negative test | blocked | depends on R2-F3.1 |
| R2-F3.3 | library/page entry points (clean control) | clean | darkmatter `just test mermaid_embedded_font` 6 passed after this cycle |
| R2-F4 | The recorded browser comparison does not meet the specified width criterion | blocked | spec AC-E1 requires bbox within 0.5 px; recorded result fails on 4 of 56 readings and the excess also appears with no embedded font (Linux Chromium SVG text box); changing the metric/tolerance is the author decision the review reserves |
| R2-F4.1 | macOS-generated page: every label's bbox within 0.5 px | blocked | needs the author's metric ruling |
| R2-F4.2 | Linux-generated page: every label's bbox within 0.5 px | blocked | needs the author's metric ruling |
| R2-F4.3 | advance-width diagnostic (clean) | clean | recorded 0.0000 px advance-width difference (implementation log, Phase 7) |
| R2-F4.4 | no-font controls (clean) | clean | recorded no-font control differs up to ~29 px |
| R2-F4.5 | committed browser test (clean) | clean | browser test unchanged; not rerun this cycle (no browser-facing markup change beyond rule count/contents) |
| R2-F5 | An unused restricted weight changes the font of permitted labels | fixed | D13 applied per diagram from the weights it draws; all children verified |
| R2-F5.1 | regular permitted, bold Restricted, regular-only diagram → host face kept | fixed | `only_a_drawn_refused_weight_makes_the_family_inter_fresh_and_cached` case 1: Arial kept, `[Arial 400]`, fresh and cached; failed before |
| R2-F5.2 | regular Restricted, bold permitted, bold-only diagram → host face kept | fixed | same test case 2: `[Arial 700]`; failed before |
| R2-F5.3 | prohibited weight used → whole family Inter (control) | fixed | same test case 3: Inter 400+700, comment names Restricted Arial, one warning |
| R2-F5.4 | both permitted → host face (control) | clean | same test case 4 |
| R2-F5.5 | cache-hit reconstruction applies the same choice | fixed | every case asserts cached faces/glyphs/SVG and `page_font` equal fresh; `FontRecord.forced_inter` is required (old records re-render, tested) |
| R2-F5.6 | page aggregation (`page_font`) when two diagrams make different choices for one requested family | fixed | `a_page_embeds_each_diagrams_own_choice_for_one_requested_family`: `[Arial 400, Inter 400, Inter 700]`; `page_font` now dedupes whole choices |
| R2-F5.7 | existing tests asserting the two-weight rule updated to D13, AC-E13 clause still covered | fixed | `a_drawn_restricted_weight_makes_the_whole_family_inter` (4 cases), `an_undrawn_restricted_weight_keeps_the_resolved_family`, `before_layout_a_refused_face_is_kept_and_recorded` |
| R2-F2.8 | discovered: installed `Inter` and bundled Inter declared under one name with two faces | fixed | a resolved face named `Inter` becomes the bundled Inter (reason `Resolved`); `an_installed_inter_is_replaced_by_the_bundled_inter`, `an_installed_inter_and_the_bundled_inter_never_share_a_page` |
| R2-F5.8 | discovered while testing 5.3: a page's first comment for a family hid a more explanatory reason | fixed | one comment per family with the most explanatory reason (`reason_rank`); covered by R2-F5.3's comment assertion |
- starting the work on 'R2-F1 whole-font shaping' at 10:24:25
        - defect class: measurement selects shaping behavior from the opt-in flag instead of the actual file emitted for the chosen face and weight
        - sweep: every opt-in width goes through `LabelMeasurer::width` → `shape` → `face_for` (main render and the gitGraph tag-spacing measurer in `prepare`); `measured_family` ignores shaping; nothing outside `measure.rs` builds `ShapeParams`
        - fix: `LabelMeasurer::face_for` caches `(ID, layout_tables)` per style key; opt-in → `EmbeddedFace::unsubset` of the face serving that weight, otherwise `true`; `FamilyChoice::face_for_weight` now returns the `EmbeddedFace`
        - tests (L1, `embed_render_tests`): `the_opt_in_measures_each_weight_as_its_emitted_file_draws` (4 `fsType` combinations + 2 `.ttc` members; decodes each emitted file and shapes it with rustybuzz defaults) and `an_opt_in_label_in_a_whole_face_is_laid_out_kerned`; both failed before the fix (176.625 vs 160.21875 px)
        - docs: `biscuit-visualized/docs/mermaid-embedded-font.md`, `darkmatter/docs/rendering/mermaid.md`, both skill files, and rustdoc in `measure.rs`, `render.rs`, `subset.rs`, `shaper.rs`
        - subagent checks: `just test embed_` 78 passed; `just test measure` 108 passed; `just test golden_tests` 209 passed; `just lint` clean
        - noted: opt-in whole-face output changed, so `MERMAID_BACKEND`'s `+bv` needs a bump; done once after R2-F5 (which also changes output)
- work completed for 'R2-F1 whole-font shaping' at 10:28:33
- starting the work on 'R2-F2 shared face for two weights' at 10:28:33
        - defect class: deduplication of emitted rules uses font-file identity where output identity also includes CSS family and weight
        - fix: `embedding.rs::build` dedupes on (family, weight, face ID), family compared ASCII-case-insensitively; requested lists choosing one family still share one rule per weight holding the union
        - sweep: `page.rs::page_font` (dedupes by `requested`; same chain ⇒ same choice), `glyphs.rs` (keys on leading family, splits weight at 600), comments and OFL line (once per family) — no further instance of this class
        - tests: `one_installed_face_carries_both_weights_rules_fresh_and_cached`, `one_face_serving_both_weights_gets_a_rule_per_weight_drawn`, `requested_families_choosing_one_family_share_its_rules_and_union` (all failed before the fix), plus controls `one_installed_face_drawn_regular_only_gets_one_rule`, `separate_regular_and_bold_faces_get_a_rule_each_fresh_and_cached`, `bold_only_use_of_a_face_serving_both_weights_embeds_one_bold_rule`
        - docs: "per face and weight" → "per family and weight" in rustdoc, `biscuit-visualized/docs/mermaid-embedded-font.md`, `darkmatter/docs/rendering/mermaid.md`, skill `mermaid-rendering.md`
        - subagent checks: `just test embed_` 84 passed; `just lint` clean; darkmatter `just test mermaid_embedded_font` 6 passed
        - discovered (sibling of the identity class, added as R2-F2.8): an installed face whose family is `Inter` and the bundled Inter can both be chosen on one page under the family name `Inter` with different face IDs, giving two same-weight `Inter` rules measured with two faces; repaired together with R2-F5 because it lives in the face choice
- work completed for 'R2-F2 shared face for two weights' at 10:32:32
- starting the work on 'R2-F5 unused restricted weight' at 10:32:32
        - decision (no author ruling recorded): implement D13 as written ("if any weight the page uses"), because the spec text is the authority and AC-E13's whole-family clause is satisfied when the bold weight is used; alternatives were amending the spec to the conservative two-weight rule (author's call) or leaving the discrepancy; recorded in the review's `human_review_items`
        - scope of "uses": per diagram (each diagram's labels lead with the family it was measured in, so two diagrams choosing differently for one requested list never share a family name or metrics)
        - defect class: family substitution depends on unused font weights rather than the weights the diagram draws
        - fix: `EmbeddedFace.permission` is `Result<Permission, Refusal>`; `choose_family` keeps refused faces; `forced_inter(faces, glyphs)` lists families drawing a refused weight (bold ≥ 600) and logs the warning; `render_with_font_in` re-runs `prepare` and the render with those families forced to Inter and stores under the first pass's key; `FontRecord.forced_inter` (required) replays the choice on a hit; `page_font` dedupes whole choices; `build` skips a refused face (debug assertion + warning) instead of `unreachable!`
        - also fixed here: R2-F2.8 (installed `Inter` → bundled Inter) and R2-F5.8 (page comment keeps the most explanatory reason per family)
        - `MERMAID_BACKEND` bumped to `merman@6e6b1e6554e8+bv6` (opt-in output changed by R2-F1 and R2-F5); pinned tests and docs updated; no golden contains it
        - docs: `biscuit-visualized/docs/{mermaid-embedded-font,merman,mermaid-gitgraph}.md`, skill `SKILL.md`, `mermaid-rendering.md`, `artifacts-caching.md`; the `darkmatter/docs/rendering/mermaid.md` edit landed in the author's commit `57f051da8` made during this work
        - behavior note for the author: the refusal warning is logged only on a render that substitutes; cache hits repeat the choice silently (the page comment still states the reason)
        - subagent checks: `just test embed_` 91 passed; `just test cache` 47 passed; `just test golden_tests` 209 passed; `just lint` clean; darkmatter `just test mermaid_embedded_font` 6 passed
- work completed for 'R2-F5 unused restricted weight' at 10:54:45
- R2-F3 and R2-F4: no code change; blockers re-verified (see ledger) — R2-F3's prerequisite flag is absent, R2-F4 needs the author's metric/tolerance ruling, which the spec's acceptance text does not leave to the implementer
- phase gates (run by the orchestrator after every subagent finished)
        - `just lint` in `biscuit-visualized`: clean; `just lint` in `darkmatter`: clean
        - `just test` in `biscuit-visualized`: 1070 passed, 4 skipped, 0 failed; `layout_matrix_{browser,markdown}_snapshots` ran just over 5 s under host load (load average 30–68 from other sessions) and pass in 1.1 s alone, so they are load-sensitive, not regressions
        - `just check-tier-coverage biscuit-visualized`: zero stranded tests
        - `just test --no-fail-fast` in `darkmatter`: 10677 run, 10631 passed, 43 failed, 3 timed out
                - 42 failures are JSON key-order snapshot diffs (40 `schemas_convert_snapshots`, `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`, `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`) from Merman's `serde_json` `preserve_order` unification; this is the known set of 42 recorded by the first review, and nothing in this cycle touches schema code
                - `dmls::startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration` and the 3 timeouts (`claudine_bundle_loads_and_its_triggers_are_discovered`, `loadable_shipped_schemas_resolve_and_lower`, `quick_fix_encodes_quoted_replacements_for_their_style`) pass when rerun alone (host load average about 90): load-sensitive
                - every `mermaid_embedded_font` test passes
        - cross-OS check: not run; the changes are font-choice and measurement logic with no path, process, or `cfg` code, so they carry no notable OS risk

### Completion Report

- **outcome:** blocked — the three repairable findings are fixed and verified; the two remaining findings need prerequisites this cycle cannot supply (elapsed about 43 minutes, 10:22 → 11:05)
- parent findings: `total 5 = fixed 3 + already fixed 0 + blocked 2 + unresolved 0 + disputed 0`
        - fixed: R2-F1 (9 child rows), R2-F2 (8 child rows, including discovered R2-F2.8), R2-F5 (8 child rows, including discovered R2-F5.8)
        - blocked: R2-F3 and R2-F4
- open finding IDs
        - **R2-F3** (R2-F3.1, R2-F3.2): `md --feature embed-mermaid-font` and its CLI and frontmatter-negative tests. Reason: the shared `--feature` flag of `2026-10-22-merman-text-fallback` has not landed, and a standalone flag would contradict D3. Next action: land that flag, then wire `embed-mermaid-font` to `HtmlOptions::embed_mermaid_font` and add the two tests; or the author changes this feature's scope
        - **R2-F4** (R2-F4.1, R2-F4.2): AC-E1's bounding-box criterion (0.5 px) is not met on 4 of 56 recorded readings. Reason: choosing a different metric or tolerance is the author's ruling, which the review reserves. Next action: the author picks advance widths, a 1 px bounding-box tolerance, or an investigation of the Linux Chromium SVG text-box excess, and records it in the spec
- decisions made this cycle, recorded in the review's `human_review_items`: D13 implemented per diagram (second layout pass); an installed face named `Inter` is replaced by the bundled Inter; the refusal warning is logged only on the render that substitutes
- changed files
        - code: `biscuit-visualized/src/src/fonts/{measure,shaper}.rs`, `.../mermaid/embed/{faces,embedding,page,comment,subset}.rs`, `.../mermaid/render.rs`, `.../cache/file_cache.rs` (`+bv6`)
        - tests: `biscuit-visualized/src/src/tests/{embed_render_tests,embed_embedding_tests,embed_faces_tests,embed_comment_tests,embed_subset_tests,cache_tests,mermaid_cache_key_tests}.rs`
        - docs: `biscuit-visualized/docs/{mermaid-embedded-font,merman,mermaid-gitgraph}.md`, `.claude/skills/biscuit-visualized/{SKILL,mermaid-rendering,artifacts-caching}.md`; `darkmatter/docs/rendering/mermaid.md` (the edit is included in the author's commit `57f051da8`)
        - records: this log, `review-2.md` frontmatter
- verification tied to findings
        - R2-F1: `just test embed_`, `just test measure` (108), `just test golden_tests` (209), all passing
        - R2-F2: `just test embed_`; darkmatter `just test mermaid_embedded_font` (6), passing
        - R2-F5: `just test embed_` (91), `just test cache` (47), passing
        - all: `biscuit-visualized` `just test` 1070/1070, both `just lint` clean, darkmatter full suite with only the known JSON-order failures and load-sensitive tests (above)

## Implementation of Review Findings #3

> **started at:** 2026-10-24T11:10:02-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/review-3.md'
- this is iteration 3 of the review-to-implement cycle 
- inventory of `review-3.md` (read in full: frontmatter `findings`, the five `human_review_items`, the previous-finding verification table, both blocked findings' child tables, recurrence, requirement levels, input robustness, commands)
        - two parent findings, both carried from `review-2.md` and both under "Blocked Findings": R2-F3 (high) and R2-F4 (high); "Unblocked Findings: None"
        - R2-F1, R2-F2, R2-F5 are verified fixed by the review and carry no open row
        - `recurrence: true`: the recurrence list names only the unchanged R2-F3 and R2-F4 instances, no missed sibling
        - `implemented` on the review was already `false` before work started; kept `false` until the closing step
        - prerequisite re-check at 11:10:24: `2026-10-22-merman-text-fallback` is `implemented: false` / `status: finalized-spec`; `git grep 'long = "feature"'` finds nothing in the worktree, and `main` (`eb2e0a642`) has none either

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F3 | The specified command-line opt-in is still unavailable | blocked | prerequisite `2026-10-22-merman-text-fallback` (D4 there defines the shared `--feature` flag) is `implemented: false`; no author scope change recorded; a standalone flag would contradict D3 |
| R2-F3.1 | `md --feature embed-mermaid-font` parsed by the shared convention and wired to `HtmlOptions::embed_mermaid_font` | blocked | no `long = "feature"`/`long("feature")` in any `.rs` file in the worktree or on `main`; `git log --all -S'long = "feature"' -- darkmatter` empty; no CLI wiring of `embed_mermaid_font` (library only: `html.rs:82`, `page.rs:736`, `code_renderer.rs:630`) |
| R2-F3.2 | CLI activation test, default-mode identity, frontmatter cannot enable embedding | blocked | depends on R2-F3.1; docs (`darkmatter/docs/cli/render.md:71`, `darkmatter/docs/rendering/mermaid.md:387`) already mark the flag **planned** |
| R2-F3.3 | library/page controls: default off, static opt-in, non-static unchanged | clean | darkmatter `just test mermaid_embedded_font`: 6/6 passed (`the_opt_in_is_off_by_default`, `without_static_svg_the_opt_in_changes_nothing`, `the_html_entry_point_embeds_the_font_too`, `a_page_of_diagrams_embeds_one_rule_per_face_and_weight`, `the_ofl_line_accompanies_inter_only`, `mermaid_font_inter_embeds_inter_with_its_ofl_line`) |
| R2-F4 | The recorded browser comparison does not meet the specified width criterion | blocked | needs the author's metric/tolerance ruling; this cycle's investigation shows the excess is Linux Chromium's `getBBox` adding glyph ink rounded outward to whole device pixels, and no page markup removes it (see R2-F4.1) |
| R2-F4.1 | macOS-rendered page: every label's `getBBox` width equal across readers within 0.5 px | blocked | reproduced exactly: max \|getBBox diff\| 0.673 px, `CUSTOMER` 0.661, `ORDER` 0.673, `LINE_ITEM` 0.619; `text-rendering:geometricPrecision`/`optimizeLegibility`, `font-kerning:none`, `-webkit-font-smoothing`, CSS `scale(4)` all leave 0.673–0.680; only the Linux reader's `--force-device-scale-factor=4` (a browser flag) gives 0.208 px, 0/28 over |
| R2-F4.2 | Linux-rendered page: same | blocked | not re-rendered (needs `build-linux`); same mechanism: `Yes` exceeds by DejaVu `Y` ink (+1.000 px left edge on the Linux reader, also on the no-font control) |
| R2-F4.3 | advance-width diagnostic | clean | max \|advance diff\| 0.0000 px under every variant; Linux `getComputedTextLength()` agrees with macOS within 0.031 px; per-character extents within 1/64 px |
| R2-F4.4 | no-font sensitivity control | clean | control page: 29.11 px advance diff, 28/28 labels over 0.5 px, unchanged by every tweak |
| R2-F4.5 | committed headless-browser loading/coverage check | clean | `BISCUIT_BROWSER_REQUIRED=1 just test-browser browser_embedded_mermaid_font_draws_every_label` (darkmatter): 1 passed in 24.0 s (load average ~26) |
- starting the work on 'R2-F3 command-line opt-in' at 11:11:46
        - defect class: a required user entry point and its wiring tests are missing while the library behavior exists
        - sweep: every `.rs` file in the worktree and `main` for a `--feature` argument; `darkmatter/cli/src` for `embed_mermaid_font` wiring; both CLI docs pages that name the flag (already marked **planned**)
        - blocker unchanged: the prerequisite spec is `implemented: false`, `status: finalized-spec`; its D4 defines the shared flag (repeatable, comma-separated, case-insensitive, per-CLI name list, unknown name a usage error); building it here would implement another spec's scope, which D17 condition 3 forbids, and a standalone flag would contradict D3
        - clean control: darkmatter `just test mermaid_embedded_font` 6/6 passed (11:12:03–11:12:36)
- work completed for 'R2-F3 command-line opt-in' at 11:12:36
- starting the work on 'R2-F4 AC-E1 bounding-box criterion' at 11:11:35
        - defect class: verification substitutes a different measurement for an explicit acceptance criterion; the open question was whether the criterion can be met at all, so this cycle investigated the four discrepancies (the review's third option) instead of changing the criterion
        - reproduced section 7.2 of `implementation-log.md` exactly on the macOS-rendered page (spike `spikes/ac-e1-widths`, macOS Chrome and Debian Chromium 156 in Docker)
        - diagnosis: on macOS `getBBox().width` equals `getComputedTextLength()` for every label; on Linux it is the advance box joined with the glyph ink rounded outward to whole pixels (right-edge excess matches canvas ink overhang within 0.01 px; ink left edges exactly 0 or -1); the same excess appears on the no-font control, so it is independent of the embedded font
        - markup tweaks tried on copies of the page (`text-rendering` ×2, `font-kerning:none`, `-webkit-font-smoothing`, CSS `scale(4)`): none moves the maximum below 0.673 px; the Linux reader at device scale factor 4 gives 0.208 px (0/28 over), confirming pixel rounding, but that is a reader launch flag, not something the page can set
        - no source change: nothing within this feature's output can make Linux Chromium's `getBBox` meet 0.5 px at scale factor 1, so the criterion's wording (advance width / `getComputedTextLength`, a 1 px tolerance, or `getBBox` at scale factor 4) remains the author's ruling; the new evidence is added to the review's `human_review_items`
        - cleanup: scratch dir `~/ac-e1-work-3`, image `rb-ac-e1-chromium`, and the copied `Cargo.lock` were removed; `git status` of `spikes/` clean
        - R2-F4.5: committed browser test passed (11:16:38–11:17:04)
- work completed for 'R2-F4 AC-E1 bounding-box criterion' at 11:17:04
- phase gates: no production source, test, golden, or doc file changed in this cycle, so the area-wide `just test`/`just lint` runs verify nothing new and were not repeated (last full run: iteration 2, `biscuit-visualized` 1070/1070, both lints clean); `git status` confirms the tree differs from iteration 2 only in records
- cross-OS check: not run; no code changed

### Completion Report

- **outcome:** blocked — both findings depend on prerequisites this cycle cannot supply (elapsed about 9 minutes, 11:10 → 11:19)
- parent findings: `total 2 = fixed 0 + already fixed 0 + blocked 2 + unresolved 0 + disputed 0`
        - instance rows: R2-F3.1, R2-F3.2, R2-F4.1, R2-F4.2 blocked; R2-F3.3, R2-F4.3, R2-F4.4, R2-F4.5 clean
- open finding IDs
        - **R2-F3** (R2-F3.1, R2-F3.2): `md --feature embed-mermaid-font` and its CLI and frontmatter-negative tests. Reason: the shared `--feature` flag (D4 of `2026-10-22-merman-text-fallback`) has not landed on this branch or `main`. Next action: implement that spec's D4, then wire `embed-mermaid-font` to `HtmlOptions::embed_mermaid_font` and add the two tests; or the author narrows this feature's scope
        - **R2-F4** (R2-F4.1, R2-F4.2): AC-E1's 0.5 px `getBBox` criterion fails on 4 of 56 readings. Reason: the excess is Linux Chromium's whole-pixel ink rounding in `getBBox`, which this cycle showed no page markup can remove; changing the metric or tolerance is the author's ruling. Next action: the author records one of advance width / `getComputedTextLength`, a 1 px `getBBox` tolerance, or `getBBox` read at device scale factor 4 in the spec
- changed files: this log and `review-3.md` frontmatter only; no source, test, or doc changes
- verification
        - R2-F3.3: darkmatter `just test mermaid_embedded_font` 6/6 passed
        - R2-F4.1/4.3/4.4: spike readings on macOS Chrome and Docker Debian Chromium (table in the R2-F4 entry above)
        - R2-F4.5: darkmatter `BISCUIT_BROWSER_REQUIRED=1 just test-browser browser_embedded_mermaid_font_draws_every_label` passed

## Implementation of Review Findings #4

> **started at:** 2026-10-24T11:22:29-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- inventory of `review-4.md` (read in full: frontmatter, previous-finding table, unblocked and blocked findings with instance tables, recurrence, verification levels, input robustness, commands)
        - frontmatter `findings`: R2-F3 (high, blocked), R2-F4 (high, blocked), R4-F1 (medium, unblocked); `has_blocked_findings: true`, `recurrence: true`
        - `implemented` was already `false` before work started; left `false` until the closing step
        - the recurrence section names no missed sibling site: both carried findings are unchanged instances, and R4-F1 is new
        - the six `human_review_items` are author decisions; none is a repair request, and no blocker was removed by an accepted decision since iteration 3
- work is small and serial (one config edit plus checks), so it is done directly by the orchestrator instead of through subagents

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R4-F1 | A sanitizer render test misses the macOS font-scan setup | fixed | all five child rows closed; darkmatter filter selects every Mermaid/Merman-named L1 test; `just test merman_flowchart_survives_with_labels_and_shadow` 0.716 s |
| R4-F1.1 | `svg_sanitizer::tests::merman_flowchart_survives_with_labels_and_shadow` gets the `coretext-small-bundle` setup and stays below 5 s | fixed | `.config/nextest.toml`; alone 0.716 s with `SETUP PASS` (was 10.293 s); 2.312 s with all 77 selected tests together |
| R4-F1.2 | darkmatter `mermaid_embedded_font` L1 page tests keep the setup and coverage | clean | six `mermaid_embedded_font::` tests still selected (now through the name pattern); 1.263–2.510 s together |
| R4-F1.3 | biscuit-visualized's seven embedding test modules keep the package-wide setup | clean | biscuit-visualized entry unchanged; `just test` 1070/1070 passed |
| R4-F1.4 | darkmatter `mermaid_font` unit helpers and browser-tier consumer stay outside the override | clean | `browser_` marker excluded from the filter; `mermaid_font` unit helpers are in-process and launch nothing |
| R4-F1.5 | sweep: every other darkmatter L1 test that renders Mermaid over host fonts is covered by the setup | fixed | two siblings found: `code_renderer::...render_browser_mermaid_returns_svg_or_none_never_silent_code_block` (2.401 → 0.568 s alone) and `entrypoints::...render_tree_terminal_draws_mermaid_through_the_installed_hook` (timed out at 30 s in the suite; 2.815 → 1.217 s alone); both now selected; reverse grep found no unselected Mermaid-rendering test |
| R2-F3 | The specified command-line opt-in is still unavailable | blocked | prerequisite `2026-10-22-merman-text-fallback` still `implemented: false`, `status: finalized-spec`; no `--feature` arg on branch or `origin/main` |
| R2-F3.1 | `md --feature embed-mermaid-font` parsing and `HtmlOptions` wiring | blocked | shared `--feature` flag absent (grep `darkmatter/cli/src`, `git grep origin/main`); a standalone flag contradicts D3 |
| R2-F3.2 | CLI activation and frontmatter-negative tests | blocked | depends on R2-F3.1 |
| R2-F3.3 | library and page controls | clean | darkmatter `mermaid_embedded_font` 6/6 passed in the 77-test run and the full suite |
| R2-F4 | The recorded browser comparison does not meet the specified width criterion | blocked | AC-E1 metric/tolerance is the author's ruling; no amendment recorded since iteration 3 |
| R2-F4.1 | macOS-generated page, bounding boxes within 0.5 px | blocked | 0.673 px max `getBBox` excess is Linux Chromium ink rounding (iteration 3 evidence); awaits author ruling |
| R2-F4.2 | Linux-generated page, bounding boxes within 0.5 px | blocked | 0.969 px max; same ruling |
| R2-F4.3 | advance-width diagnostic | clean | recorded advance widths agree to 0.0000 px (iteration 3); no code changed since |
| R2-F4.4 | no-font sensitivity controls | clean | recorded no-font controls differ by about 29 px (iteration 3); no code changed since |
| R2-F4.5 | committed headless-browser loading test | clean | committed browser test passed in review #4 (24.875 s); no production or test code changed this cycle |

- starting the work on 'R4-F1 sanitizer render test setup' at 11:23:25
        - defect class: an L1 test that renders through the host font chain on macOS is not selected by the nextest setup that keeps CoreText from scanning the test executable's directory
        - repair (R4-F1.1): `.config/nextest.toml` darkmatter `coretext-small-bundle` entry; first added the named test, then widened per the sweep below; `just test merman_flowchart_survives_with_labels_and_shadow` now shows `SETUP PASS` and passes in 0.716 s (review: 10.293 s)
        - sweep (R4-F1.5): a full darkmatter `just test` (load average about 100) flagged two more Mermaid-rendering unit tests outside the old filter
                - `markdown::render_tree::code_renderer::tests::render_browser_mermaid_returns_svg_or_none_never_silent_code_block`: 29.0 s in the suite; alone 2.401 s without the setup, 0.568 s with `CFProcessPath=/usr/bin/true`
                - `markdown::render_tree::entrypoints::tests::render_tree_terminal_draws_mermaid_through_the_installed_hook`: **TIMEOUT** at 30 s in the suite; alone 2.815 s without, 1.217 s with
                - the other three slow tests in that run (`installed_shells::*` ×2, `finding_35_2::relevel_output_matches_the_oracle_across_shipped_fixtures`) render no Mermaid and are outside this class; they passed and are load-sensitive
        - decision: listing test names would miss the next sibling, so the darkmatter filter now selects by name, `package(darkmatter) & test(/mer(maid|man)/) & !test(/(^|::)(level2_|level3_|browser_|real_)/)`, mirroring the tier exclusion of the biscuit-visualized entry; 77 tests match (56 lib unit, 21 `l1`)
                - incorrect extra results: none of the 77 launches an application (checked the list for chrome/open/launch/spawn/terminal; the terminal ones render in-process), so the bundle override cannot break them; `browser_embedded_mermaid_font_draws_every_label` stays excluded by its `browser_` marker (R4-F1.4)
                - missing results: grepped darkmatter lib and CLI test code for `MermaidMode::Image`, `render_to_svg`, `StaticSvg`, `embed_mermaid_font: true`, `with_embedded_font`; every enclosing test is Mermaid-named (the other hits are production functions or doc comments above Mermaid-named tests); the CLI's only Mermaid test, `validate_refs::validate_refs_graph_mermaid`, prints Mermaid source and renders nothing
                - recorded the name-based selection contract in `.claude/skills/darkmatter/rendering.md`
        - all 77 together: `cargo nextest run -p darkmatter --all-features -E '<filter>'` 77/77 passed in 2.722 s, slowest 2.510 s, `SETUP PASS` once
        - R4-F1.2: all six `mermaid_embedded_font::` tests are in the 77 (1.263–2.510 s); R4-F1.3: the biscuit-visualized entry is unchanged; R4-F1.4: `mermaid_font` unit helpers are selected by name, but they run in-process and launch nothing, so the override only skips a scan they never make
- work completed for 'R4-F1 sanitizer render test setup' at 11:59:10
- starting the work on 'R2-F3 command-line opt-in' at 11:59:10
        - defect class: a required user entry point and its wiring tests are missing while the library behavior exists
        - re-checked the blocker: `md get biscuit-visualized/features/2026-10-22-merman-text-fallback/spec.md implemented status` → `false`, `finalized-spec`; `git grep 'long = "feature"'` and `embed-mermaid-font` over `*.rs` on this branch and `origin/main`: no hits
        - no accepted decision has changed the dependency, so building the shared flag here would still implement another spec's scope (D17 condition 3), and a standalone flag would contradict D3
- work completed for 'R2-F3 command-line opt-in' at 11:59:10 (blocked)
- starting the work on 'R2-F4 AC-E1 bounding-box criterion' at 11:59:10
        - defect class: acceptance evidence substitutes a different measurement for the one the criterion names
        - no new ruling is recorded in the spec (AC-E1 still reads "compare every label's bounding box; widths equal within 0.5 px"); iteration 3 showed no page markup can meet it on Linux Chromium at device scale factor 1, so the remaining step is the author's choice of metric or tolerance
- work completed for 'R2-F4 AC-E1 bounding-box criterion' at 11:59:10 (blocked)
- phase gates (no subagents were running; load average 100–370 came from Finder and DaisyDisk processes outside this session)
        - darkmatter `just test --no-fail-fast`: 10677 run, 10630 passed, 43 failed, 4 timed out
                - 42 failures are the known JSON key-order snapshot diffs from Merman's `serde_json` `preserve_order` unification (40 `schemas_convert_snapshots`, `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`, `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`): the same set iteration 2 recorded, tracked by the active darkmatter fix `2026-10-23-frontmatter-key-order`, and unreachable from a macOS nextest setup entry
                - `dmls::l1 startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration` and the four timeouts (`claudine_schema_bundle::` ×3 in darkmatter-cli, `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower`) all passed rerun alone (0.233–10.381 s at load about 110): load-sensitive
                - every Mermaid test passed; at load 370 the page tests ran 6–7 s, which is load, not the scan (2.5 s together at load 17)
        - darkmatter `just lint`: clean
        - biscuit-visualized `just test`: 1070 passed, 4 skipped, 0 failed; biscuit-visualized `just lint`: clean
        - `just check-tier-coverage darkmatter` and `just check-tier-coverage biscuit-visualized`: zero stranded tests
- cross-OS check: not run; the only code change is a nextest entry gated to `cfg(target_os = "macos")` hosts, so other OSes see no change

### Completion Report

- **outcome:** partial; R4-F1 is fixed, and R2-F3 and R2-F4 are still blocked on prerequisites only the author can provide. A cycle with an unblocked finding fixed and the rest blocked counts as partial, not blocked (elapsed about 37 minutes, 11:22 → 11:59)
- parent findings: `total 3 = fixed 1 + already fixed 0 + blocked 2 + unresolved 0 + disputed 0`
        - instance rows: R4-F1.1 and R4-F1.5 fixed; R4-F1.2–1.4 clean; R2-F3.1, R2-F3.2, R2-F4.1, R2-F4.2 blocked; R2-F3.3, R2-F4.3, R2-F4.4, R2-F4.5 clean
- verified repair
        - **R4-F1**: on macOS, every darkmatter L1 test whose name contains `mermaid` or `merman` (77 tests, browser and other tier markers excluded) now gets `CFProcessPath=/usr/bin/true`. That covers the named sanitizer test and two siblings the review did not name, one of which had timed out in the suite
- open finding IDs
        - **R2-F3** (R2-F3.1, R2-F3.2): `md --feature embed-mermaid-font` and its CLI and frontmatter-negative tests. Reason: the shared `--feature` flag from `2026-10-22-merman-text-fallback` (D4 there) has not landed. Next action: implement that spec's D4, then wire the feature and add the two tests; or the author narrows this feature to the library and page APIs
        - **R2-F4** (R2-F4.1, R2-F4.2): AC-E1's 0.5 px `getBBox` criterion fails on 4 of 56 readings because of Linux Chromium's whole-pixel ink rounding. Next action: the author records one of: advance width or `getComputedTextLength`, a 1 px `getBBox` tolerance, or `getBBox` read at device scale factor 4
- changed files
        - `.config/nextest.toml`: darkmatter `coretext-small-bundle` filter and its comment (R4-F1)
        - `.claude/skills/darkmatter/rendering.md`: records that the setup selects tests by name (R4-F1)
        - this log and the `review-4.md` frontmatter
- verification commands
        - R4-F1.1: `just test merman_flowchart_survives_with_labels_and_shadow` (darkmatter): passed in 0.716 s, `SETUP PASS`
        - R4-F1.5: the two siblings timed alone with and without `CFProcessPath=/usr/bin/true` (above); `cargo nextest run -p darkmatter --all-features -E 'test(/mer(maid|man)/) & !test(/(^|::)(level2_|level3_|browser_|real_)/)'`: 77/77 in 2.722 s
        - R4-F1.2–1.3, R2-F3.3: darkmatter full suite and biscuit-visualized `just test` (above)
