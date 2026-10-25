---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F3 (high): The specified command-line opt-in is still unavailable"
    - "R2-F4 (high): The recorded browser comparison does not meet the specified width criterion"
human_review: true
human_review_items:
    - |-
      Resolve the missing command-line option's completion path. The shared feature-flag prerequisite is still absent, and the specification still requires the option.

      - Keep the scope and finish the option and tests after the shared flag lands.
      - Authorize implementing the shared flag in this work.
      - Explicitly narrow this feature to library and page APIs.
    - |-
      Choose the browser width measurement and record it in the specification. Four recorded label bounding-box comparisons exceed the required 0.5 pixels, although text advance widths match.

      - Compare text advance widths, retaining bounding boxes as diagnostics.
      - Keep bounding boxes with a 1-pixel tolerance.
      - Keep the original criterion and investigate the four discrepancies.
    - |-
      Review the existing typography choice: small subsets omit kerning, ligatures, and contextual alternates, and measurement now follows the file emitted for each weight.

      - Keep the subsets and matching simpler typography.
      - Embed whole fonts, accepting larger pages.
      - Authorize work on subsets that preserve typography features.
    - |-
      Review the existing supported-font boundary: variable fonts and CFF outlines use bundled Inter instead.

      - Accept static TrueType-only embedding.
      - Authorize additional formats and any required output-format changes.
    - |-
      Review the repair's font-choice decisions. A restricted weight now forces Inter only when the diagram draws it; two diagrams requesting the same family can therefore embed different families on one page. An installed face named Inter is replaced by bundled Inter to prevent conflicting rules. Refusal warnings occur on substitution, while cache hits replay the choice silently and retain the explanation in page comments.

      - Retain these choices, which preserve matching measurement and embedded output in the tested cases.
      - Request a different page-wide selection policy or installed-Inter naming policy, with its scope recorded before implementation.
    - |-
      Evidence for the browser width ruling (iteration 3 investigation, no code changed). The four bounding-box excesses are Linux Chromium's getBBox adding glyph ink rounded outward to whole pixels. They also appear without an embedded font, and no page markup tried removes them: text-rendering, font-kerning, font smoothing, and a CSS scale all leave 0.673 px. SVG text lengths agree within 0.031 px, and advance widths agree to 0.0000 px. A Linux reader at device scale factor 4 is within 0.5 px (0.208 px), so the bounding-box option is satisfiable only through a reader launch flag.
has_blocked_findings: true
blocked: true
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-24T11:04:10-07:00"
spec: 2026-10-21-mermaid-embedded-font/spec.md
implemented: false
description: "A **feature** review of `2026-10-21-mermaid-embedded-font/spec.md`"
feature: 2026-10-21-mermaid-embedded-font/review-3.md
previous: 2026-10-21-mermaid-embedded-font/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/log.md
implemented_by: claude/opus
next: 2026-10-21-mermaid-embedded-font/review-4.md
---

# Review 3: not production ready

The three repaired font defects are verified fixed. Two prior findings remain blocked: the specified command-line entry point is missing, and the recorded browser comparison does not meet the written bounding-box tolerance. No new defect or missed sibling was found. Pending human inspection of the other recorded design choices does not independently block readiness.

In biscuit-visualized, [LabelMeasurer::face_for](../../src/src/fonts/measure.rs:201) selects how labels are shaped, [forced_inter](../../src/src/mermaid/embed/faces.rs:111) identifies families requiring substitution, and [page_font](../../src/src/mermaid/embed/page.rs:93) combines diagram font records into page CSS. These are the main repaired decision points.

The review covered the specification, plan, both earlier reviews, original implementation evidence, repair ledger, font selection and measurement, emitted font files, page aggregation, cache reconstruction, Darkmatter entry points, sanitizer wiring, documentation, and test selection. Existing source changes were left intact; this review changes only review records and specification metadata.

## Previous Finding Verification

Evidence below combines source inspection with this review's test execution. The repair ledger adds instances beyond the earlier review's tables; those are included rather than treating the parent completion claim as proof. Test names refer to `biscuit-visualized` unless explicitly labeled Darkmatter.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1 — Whole-font embedding retains shaping that layout disables | R2-F1.1–1.4: regular, bold, and both mixed no-subsetting combinations | verified fixed | `the_opt_in_measures_each_weight_as_its_emitted_file_draws` decodes each emitted weight and compares its default-feature shaped width with the measurer; all four combinations pass. `face_for` selects layout tables from the selected weight's `unsubset` value. | Nothing. |
| R2-F1 — subset control and collection member | R2-F1.5–1.6 | verified fixed | Same test covers plain subset advances and two collection cases; `the_measurer_uses_plain_advances_with_the_opt_in` passes. The whole-file layout test verifies the measured width reaches the node box. | Nothing. |
| R2-F1 — other targets, tag-spacing consumer, documentation | R2-F1.7–1.9 | verified fixed | `without_html_static_svg_the_flag_changes_nothing` and all 209 goldens pass. Inspected `prepare`: tag spacing and the second render both use the forced-family measurer; all widths share `shape` → `face_for`. Rustdoc, the embedding topic, and Merman docs now distinguish subset and whole-font shaping. | Nothing. |
| R2-F2 — Reusing one face for two weights drops bold-only characters | R2-F2.1–2.2: fresh render and cache hit | verified fixed | `one_installed_face_carries_both_weights_rules_fresh_and_cached` passes, asserting both rules and decoded coverage of bold-only `Z` and `b`; cached CSS equals fresh CSS. Builder identity now includes family, weight, and face ID. | Nothing. |
| R2-F2 — regular-only, separate faces, aliases, bold-only | R2-F2.3–2.6 | verified fixed | `one_installed_face_drawn_regular_only_gets_one_rule`, `separate_regular_and_bold_faces_get_a_rule_each_fresh_and_cached`, `requested_families_choosing_one_family_share_its_rules_and_union`, and `bold_only_use_of_a_face_serving_both_weights_embeds_one_bold_rule` all pass. | Nothing. |
| R2-F2 — aggregation/comment sweep and installed Inter sibling found during repair | R2-F2.7–2.8 | verified fixed | Inspected whole-choice aggregation in `page_font` and family/weight collection. `inter_is_declared_as_inter_with_its_ofl_line_immediately_before`, `an_installed_inter_is_replaced_by_the_bundled_inter`, and `an_installed_inter_and_the_bundled_inter_never_share_a_page` pass. | Installed-Inter policy remains a human review item, not an unrepaired instance. |
| R2-F3 — The specified command-line opt-in is still unavailable | R2-F3.1: parsing/wiring; R2-F3.2: CLI and frontmatter-negative verification; R2-F3.3: clean library controls | still blocked | Searched `darkmatter/cli/src`; no shared `--feature` definition or `embed-mermaid-font` wiring. The prerequisite remains unimplemented. Darkmatter's six library/page tests pass but contain no completed CLI path. | R2-F3.1–3.2; carried below. |
| R2-F4 — The recorded browser comparison does not meet the specified width criterion | R2-F4.1–4.2: both generated pages; R2-F4.3–4.4: advance and no-font controls; R2-F4.5: committed browser check | still blocked | Re-read original implementation log, section “7.2 AC-E1”: maxima 0.673 and 0.969 px; 4/56 bounding-box comparisons fail. Advance controls match and removing the font causes differences up to 29 px. The spec still requires every bounding box within 0.5 px. The committed browser check passed in an isolated run in this review; it tests loading/coverage, not this cross-host criterion. | R2-F4.1–4.2; carried below. No new two-host measurement claimed. |
| R2-F5 — An unused restricted weight changes the font of permitted labels | R2-F5.1–5.5: symmetric unused weights, drawn prohibited weight, permitted control, and cached reconstruction | verified fixed | `only_a_drawn_refused_weight_makes_the_family_inter_fresh_and_cached` passes all four cases and verifies choices, glyphs, SVG, rules, comments, warnings, and fresh/hit equality. `forced_inter` checks collected used weights; required sidecar field replays the decision. | Nothing in the reproduced defect. |
| R2-F5 — page aggregation, old conservative tests, explanatory comment | R2-F5.6–5.8 | verified fixed | `a_page_embeds_each_diagrams_own_choice_for_one_requested_family`, `a_drawn_restricted_weight_makes_the_whole_family_inter`, `an_undrawn_restricted_weight_keeps_the_resolved_family`, and `before_layout_a_refused_face_is_kept_and_recorded` pass. Inspected `reason_rank` and substitution-comment assertions. | Per-diagram interpretation remains explicitly recorded for human review. |

R2-F5 had been blocked on choosing the permission policy. The repair log, “Implementation of Review Findings #2,” explicitly chose to implement the existing specification's used-weight rule rather than amend it to the conservative two-weight rule. That recorded choice enabled the repair; this review verifies its concrete original cases. This is repaired work, not a newly discovered sibling or a claim that the author approved every implementation decision.

The first review contained no formal findings. Its typography and supported-format questions remain unresolved human review items; its unused-weight discrepancy is repaired as R2-F5, and its missing-CLI and measurement questions remain R2-F3 and R2-F4. There is no partially repaired finding and no newly found sibling in this iteration.

## Unblocked Findings

None.

## Blocked Findings

### R2-F3 (high): The specified command-line opt-in is still unavailable

**Authority — spec:** requirement R-E1 and acceptance criterion AC-E9 require `--feature embed-mermaid-font` to set Darkmatter's `HtmlOptions::embed_mermaid_font`; the exposure decision requires the shared repeatable, comma-separated, case-insensitive convention with unknown names rejected. That entry point remains absent.

**Class:** a required user entry point and its wiring verification are missing while the library implementation exists.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F3.1 | Darkmatter CLI arguments and render wiring | `md --feature embed-mermaid-font` | Source still has no shared option or embedding wiring; prior binary rejection remains explained by unchanged code | Recognize the specified feature convention and set the HTML option |
| R2-F3.2 | CLI and frontmatter boundary tests | Feature activation; default-mode identity; frontmatter cannot enable embedding | Completed CLI-path tests remain absent | Verify activation, unchanged non-static output, and no frontmatter activation |
| R2-F3.3 | Library and page controls | Default off; static opt-in; non-static modes | Clean: six existing Darkmatter tests pass | Preserve this behavior |

**Blocker:** the ordered prerequisite `2026-10-22-merman-text-fallback` has not supplied the shared flag; no author scope change or authorization to replace that prerequisite appears in the records. Complete the option and tests when the prerequisite lands, or record an explicit scope decision. A standalone alternative flag would not meet the existing contract. This review does not ask for a broader CLI redesign.

### R2-F4 (high): The recorded browser comparison does not meet the specified width criterion

**Authority — spec:** AC-E1 requires comparing “every label's bounding box” with widths equal “within 0.5 px.” The original experiment explicitly records four failures, then accepts text advance widths instead. No subsequent specification amendment settles that change.

**Class:** verification substitutes a different measurement for an explicit acceptance criterion.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F4.1 | macOS-generated page on both reader hosts | 28 labels, five diagram families | Recorded maximum bounding-box difference 0.673 px; `CUSTOMER`, `ORDER`, `LINE_ITEM` exceed tolerance | Every label within 0.5 px |
| R2-F4.2 | Linux-generated page on both reader hosts | Same corpus | Recorded maximum 0.969 px; `Yes` exceeds tolerance | Every label within 0.5 px |
| R2-F4.3 | Text advance diagnostic | Both generated pages | Clean diagnostic: maximum difference 0.0000 px | Retain as diagnostic unless the specification adopts it |
| R2-F4.4 | No-font sensitivity controls | Both pages without font rules | Clean: differences up to about 29 px | Demonstrate sensitivity to fallback fonts |
| R2-F4.5 | Committed headless-browser check | Actual loading and label glyph coverage | Passed in the isolated headless Chrome run; checks loading/coverage, with no cross-host bounding-box assertion | Keep loading coverage and resolve the separate acceptance criterion |

**Blocker:** the earlier reviews reserve the metric/tolerance choice for the author, and no ruling is recorded. The smallest resolution is a decision adopting advance widths or a different bounding-box tolerance; retaining the current wording requires investigating the affected labels. This finding is about recorded failing measurements and an unapproved replacement criterion, not missing OS evidence. No new browser matrix is requested.

## Recurrence

Both carried findings repeat unchanged instances from `review-2.md`: R2-F3's missing command-line entry point and R2-F4's substituted acceptance measurement. Their predecessors were also recorded as design questions in `review-1.md`. `recurrence: true` makes that continuity explicit. These are neither repair regressions nor siblings missed by the earlier sweep. The instance tables retain every previous site, including clean controls; no new machinery or broader requirement is proposed.

## Requirement and Verification Levels

This feature adds browser font output, not terminal interaction or key handling. L1 is appropriate for face permissions, emitted bytes, CSS structure, cache behavior, size/logging, failure branches, and default/wiring checks. Actual browser loading and use need the browser tier. Real-terminal and OS keyboard injection add no relevant evidence here.

| Requirements | Verification level and result |
| --- | --- |
| AC-E1: consistent widths across reader hosts | Original two-host headless-browser experiment fails the specified metric; R2-F4 remains open |
| AC-E2: sanitizer URL boundary | L1 sanitizer cases and generated-page wiring; existing browser test exercises the admitted data URI |
| AC-E3–E4: character union, rule counts, both themes | L1 decoded emitted fonts, shared-face regressions, public page results, and Darkmatter HTML checks |
| AC-E5, E13: chosen face and matching layout | L1 injected permitted/refused sets, per-weight emitted-file shaping, actual node geometry, and fresh/cache-hit checks |
| AC-E6–E8, E12: budget, failure, license/tables, first browser check | L1 size/log/error and decoded-table assertions; original Chrome spike recorded; committed browser coverage checks actual loading |
| AC-E9: exposure/default/non-static output | L1 API/page controls pass; CLI path remains absent, R2-F3 |
| AC-E10: cache isolation | L1 separate keys, both insertion orders, missing/damaged sidecar, recorded substitution reconstruction |
| AC-E11, E14–E15: documentation, comments, OFL placement | Topic-page inspection and L1 output assertions, including both Darkmatter themes |

All seven embedding test modules are declared under the library's `image` feature. The package's CI metadata enables all features. Darkmatter's L1 and browser tests are declared targets; browser tests require the `browser-tests` feature enabled by its recipe and CI metadata. No test added by the repair is silently outside the declared target or tier.

## Input Robustness

The CSS/SVG collector and sanitizer are text/rendering machinery and do not require the configuration matrix. Font permissions are binary fixed-width values; existing tests cover permitted/refused bits, malformed or missing tables, unsupported outlines, and collection indices through emitted choices and files.

The internal sidecar is regenerable cache data. Its default invalid-document outcome is a cache miss and rerender, not publication of a guessed empty font. Required typed fields are `requested`, `forced_inter`, `families`, `regular`, and `bold`. There are no `serde(default)` or `Option` field coercions. Serde rejects missing/null/wrong-type fields and wrong collection elements; `from_slice` rejects trailing invalid content. Empty generated lists/maps remain valid for no-text output. The public render test covers missing/damaged records and old records without required structure, then verifies restoration of cache hits. The code-smell sweep found the intentional `.ok()` cache-miss conversion and text-grammar fallbacks, not a new permissive configuration default. No extra protection layer is warranted for fabricated sidecar content.

## Commands and Evidence

- `biscuit-visualized`: `just test embed_` — 91 passed; all below five seconds, including every repaired regression.
- `biscuit-visualized`: `just test golden_tests` — 209 comparisons passed, no snapshot updates. Eighteen tests exceeded five seconds during the overlapping verification runs; these unchanged golden tests are rendering controls, not new repair tests.
- `darkmatter`: `just test mermaid_embedded_font` — six tests passed; initial overlapping run had five tests above five seconds. A separate run without the golden suite passed all six in 0.620–3.146 seconds (3.316 seconds total). No test-speed defect was established by the initial overlap.

- `darkmatter`: `BISCUIT_BROWSER_REQUIRED=1 just test-browser browser_embedded_mermaid_font_draws_every_label` — the first run timed out at the 30-second limit (termination reported at 34.821 seconds), without an assertion result. After the overlapping golden and L1 runs ended, a separate invocation passed in 11.378 seconds. Both outcomes are retained: the isolated pass verifies loading/coverage, but does not erase the earlier timeout or establish its precise cause. No timeout setting, retry configuration, or source was changed. This run did not establish a reproducible implementation defect.
- Root: `just check-tier-coverage biscuit-visualized` and `just check-tier-coverage darkmatter` — both passed, zero stranded tests.
- Read-only source searches confirmed the absent CLI flag and prerequisite `implemented: false`; metadata and review-chain checks passed. No CLI spawn was repeated for the unchanged absent parser option.

The two-host acceptance experiment was inspected, not rerun. No cross-OS proof gap or pending human inspection alone was promoted into a finding. No production code, tests, goldens, or implementation logs were edited by this review.
