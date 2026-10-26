---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - |-
      Review the latest repair's sequencing choice: `md --feature embed-mermaid-font` now exists before the text-fallback feature lands. It implements the already specified shared parsing convention, with this feature as its only accepted name. The later feature can add its names to that same list. I recommend retaining this implementation; waiting would leave an otherwise complete interface unavailable without changing its eventual behavior.
    - |-
      Review the latest repair's width-measurement amendment: acceptance now compares each label's text advance within 0.5 pixels, while bounding boxes remain diagnostic. Linux Chromium rounds glyph ink outward in bounding boxes even without embedded fonts; text advances measure the spacing this feature preserves. I recommend retaining the amendment under the specification's bounded-autonomy rule. The alternative is retaining a bounding-box check with a larger tolerance or a prescribed browser scale setting.
    - |-
      Be aware of the command-line limitation: the new option sets the embedding preference, but `md` currently produces interactive Mermaid HTML, where the specification requires the option to have no effect. Static-SVG embedding works through the library and page APIs. No response is needed to close this review; a future command-line route to static SVG would be a separate product choice.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-24T13:03:37-07:00"
spec: 2026-10-21-mermaid-embedded-font/spec.md
implemented: false
description: "A **feature** review of `2026-10-21-mermaid-embedded-font/spec.md`"
feature: 2026-10-21-mermaid-embedded-font/review-6.md
previous: 2026-10-21-mermaid-embedded-font/review-5.md
---

# Review 6: production ready within the specified scope

No open findings remain. The two findings carried by review #5 are verified fixed: the command-line option exists and sets the page option, and the browser-width criterion now uses the explicitly amended measurement. Earlier font and test-setup repairs remain fixed. No partial repair, regression, or newly discovered sibling was found.

This review checked the specification, plan, all five earlier reviews, both implementation records, and the latest Finding Ledger against current source and focused test results. No earlier review contains a `## Human rulings` section with an author ruling to override. Existing typography, supported-format, per-diagram font-choice, and installed-Inter decisions were not reopened. Pending post-cycle inspection alone does not block readiness.

## Previous Finding Verification

Child IDs below refer to the earlier reviews' instance tables and the repair ledger in `log.md`. Each range includes all its children, including clean controls and downstream consumers.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F3 — The specified command-line opt-in is still unavailable | R2-F3.1: parsing and every HTML construction route | verified fixed | Inspected top-level and explicit-render arguments, their list merge, implicit render, compose, and the common HTML artifact builder. The flag accepts repeated/comma-separated and mixed-case names; unknown/empty names return usage errors. Parsing, completion, option-wiring, and spawned CLI checks pass in `feature_flag`. | Nothing. The updated exposure decision records implementing the shared convention here first; no standalone alternative flag was introduced. |
| R2-F3 — CLI activation and frontmatter boundary | R2-F3.2: activation, default-mode identity, frontmatter-negative tests | verified fixed | `embed_mermaid_font_feature_sets_the_html_page_option`, `default_mermaid_html_is_byte_identical_with_the_feature`, `no_frontmatter_key_turns_on_embedding`, and `unknown_feature_exits_with_usage_error` pass. Source tracing confirms the option reaches HTML construction. | Nothing. Default-mode identity is required behavior, even though it means the CLI flag currently has no visible effect. |
| R2-F3 — library/page controls | R2-F3.3 | verified fixed | All six `mermaid_embedded_font` tests pass: default off, static opt-in, both page entry points, both themes, Inter attribution, and non-static identity. | Nothing. |
| R2-F4 — The recorded browser comparison does not meet the specified width criterion | R2-F4.1–4.2: macOS-generated and Linux-generated pages on both readers | verified fixed | AC-E1 is explicitly amended to advance widths under the spec's bounded-autonomy rule. Inspected the spike's five-family fixture and browser probe: it waits for fonts and measures each leaf label in its computed family, size, and weight. Iteration-5 evidence records 28 labels per page, maximum advance difference 0.0000 px on each page, and both used weights loaded. | Nothing under amended AC-E1. The old bounding-box failures remain disclosed, not claimed repaired: maxima 0.6734/0.9688 px at browser scale 1. This review inspected the experiment and recorded evidence; it did not independently repeat the two-host run. |
| R2-F4 — measurement controls and loading | R2-F4.3–4.5: advance reading, no-font sensitivity, committed browser test | verified fixed | Advance is now the acceptance reading. The spike removes the font-rule style for its negative control; recorded differences are 29.1094 px, with all 28 labels over tolerance on each page. The committed headless-browser loading/coverage test passes again in this review. | Nothing. The loading test proves a different requirement from the two-reader experiment; the two are not substituted for one another. |
| R2-F1 — Whole-font embedding retains shaping that layout disables | R2-F1.1–1.4: regular/bold whole fonts and both mixed subset/whole combinations | verified fixed | `the_opt_in_measures_each_weight_as_its_emitted_file_draws` passes, comparing the decoded emitted file's shaped width to measurement for each weight. The measurer retains layout tables exactly for whole faces. | Nothing. |
| R2-F1 — subset, collection, other targets, spacing consumer, docs | R2-F1.5–1.9 | verified fixed | Subset advances, collection members, whole-font node width, and non-static output controls pass. Inspected both render preparation passes: gitGraph spacing uses the matching measurer, including forced Inter. Relevant Rustdoc and topic pages distinguish subset advances from whole-font shaping. | Nothing. Earlier golden results remain prior evidence; goldens were not regenerated or rerun here. |
| R2-F2 — Reusing one face for two weights drops bold-only characters | R2-F2.1–2.2: fresh render and cache hit | verified fixed | `one_installed_face_carries_both_weights_rules_fresh_and_cached` passes, asserting separate weight rules, decoded bold-only character coverage, and fresh/cache equality. Rule identity includes declared family, weight, and source face. | Nothing. |
| R2-F2 — controls and downstream aggregation | R2-F2.3–2.8: regular-only, distinct faces, aliases/union, bold-only, aggregation/comments, installed Inter | verified fixed | Corresponding regular-only, distinct-face, shared-family union, bold-only, attribution, installed-Inter replacement and collision tests all pass. Inspected page aggregation: it keeps complete choices and unions characters for the declared family and weight. | Nothing. No naming-policy change is requested. |
| R2-F5 — An unused restricted weight changes the font of permitted labels | R2-F5.1–5.5: both unused-weight directions, drawn refused weight, permitted control, cache replay | verified fixed | `only_a_drawn_refused_weight_makes_the_family_inter_fresh_and_cached` passes through font choice, output, comments, warnings, and cache-hit equality. The sidecar requires the forced-family replay list. | Nothing. |
| R2-F5 — aggregation and explanation | R2-F5.6–5.8: per-diagram page choices, previous conservative controls, explanatory comment | verified fixed | Per-diagram aggregation, drawn/undrawn refusal, pre-layout recording and comment tests pass. Inspected explanation ranking and the second layout pass. | Nothing. The previously recorded per-diagram interpretation is unchanged. |
| R4-F1 — A sanitizer render test misses the macOS font-scan setup | R4-F1.1–1.4: sanitizer control, six page tests, seven embedding modules, helpers/browser boundary | verified fixed | Canonical runs report `SETUP PASS`; sanitizer render control takes 0.359 s, page controls at most 0.811 s, embedding tests at most 0.475 s. Helpers pass in process. The setup filter excludes browser/resource tiers, and the browser run succeeds without that setup. | Nothing. |
| R4-F1 — repair-discovered setup siblings | R4-F1.5: code-renderer SVG control and installed terminal-hook control | verified fixed | Both pass in the Mermaid run with setup, in 0.758/0.784 s. Neither launches a terminal application; the hook control renders in process. | Nothing. These were already repaired siblings in review #5, not new findings. |

Review #1 contained no formal finding IDs. Its missing-option and width questions became R2-F3/R2-F4; its unused-weight discrepancy became R2-F5. Its other design questions remain recorded in earlier reviews rather than becoming new requirements here. Both previously blocked findings are now repaired, so their final outcome is **verified fixed**, rather than merely newly unblocked.

## Unblocked Findings

None.

## Blocked Findings

None. No decision requires a human stop.

## Decisions and Scope

The flag repair implements behavior already required by the exposure decision: one shared, repeatable feature argument with case-insensitive, comma-separated names and usage errors. Its ordering changed, but it adds no text-fallback behavior or alternate interface. The earlier dependency was the missing shared parser; that parser now exists. Requiring the rest of the text-fallback feature before accepting this interface would protect no remaining behavior in this feature. The sequencing choice is retained for post-cycle inspection.

The measurement amendment preserves the stated purpose of AC-E1: consistent label widths across readers. The recorded investigation isolates bounding-box excess to browser ink rounding, including on pages without embedded fonts. Text advance is a suitable measure of label spacing; the spec permits changing the measurement method when that purpose is preserved. The repair marks the amendment, records its evidence under “Decided during implementation,” retains bounding boxes as diagnostics, and requests post-cycle inspection. No dependency, output format, font-selection policy, or embedding guarantee was broadened.

The command-line option only sets the preference, as the specification requires. Neither its implementation nor the spec promises that it selects static-SVG mode. Adding a CLI static-SVG selector would be additional scope; its absence is disclosed in both CLI and Mermaid documentation. The library's explicit static-SVG path performs embedding and is exercised by page and browser tests.

## Recurrence

No current finding repeats an earlier class, so `recurrence: false`. R2-F3 and R2-F4 close in this iteration. Earlier font repairs and the expanded setup repair remain fixed; there is no newly found sibling to carry forward.

## Requirements and Verification Levels

This feature writes font files and browser markup. In-process Level 1 is appropriate for permission decisions, font bytes, measurement, caching, option wiring, and page assembly. Actual font loading requires the headless-browser tier. It adds no terminal-display or keyboard behavior requiring terminal Level 2 or OS-input Level 3 evidence.

| Requirements | Verification and result |
| --- | --- |
| AC-E1: equal widths across readers | Recorded two-reader experiment on both generated pages meets amended advance-width criterion. Probe and negative control inspected; no fresh cross-host execution claimed. |
| AC-E2: narrow sanitizer URL admission | 35 passing Level-1 sanitizer tests cover allowed TrueType data URLs, external URLs, data URLs in other contexts, malformed rules, and real generated SVG. Inspected shared style-sheet checker and page-font wiring. |
| AC-E3–E4: character union and one rule per used face/weight, both themes | Passing Level-1 decoded-subset, same-face/two-weight, alias-union, and public page tests. |
| AC-E5, E13: permitted face selection and matching layout | Passing injected-font Level-1 tests cover permission bits, missing tables, empty chain, explicit Inter, used/unused restricted weights, mixed whole/subset weights, and downstream node layout. |
| AC-E6–E8, E12: budget, failure, licensing, retained tables, initial browser check | Passing Level-1 size/logging/failure and decoded-table/license tests. Original first-task Chrome evidence records subset used, required name entries retained, and no Reserved Font Name. The current headless-browser loading/coverage control passes. |
| AC-E9: default off, CLI flag, non-static identity | Eleven passing feature-flag Level-1 tests plus library/page default and byte-identity controls. Inspected implicit render, explicit render and compose wiring. |
| AC-E10: cache separation | Passing Level-1 distinct-key, both-order render, fresh/hit reconstruction and damaged/missing/old-record controls. |
| AC-E11, E14–E15: docs, comments, attribution | Inspected both topic pages, CLI documentation, dependency records and relevant skill changes. Passing Level-1 explanation, escaping, whole-font and Inter-only attribution assertions. |

The new CLI file is declared by the consolidated L1 target's `main.rs`. All seven embedding modules are compiled under the library's enabled `image` feature. Darkmatter's browser target declares the loading test and its required feature is in CI metadata. Both area tier-coverage checks report zero stranded tests. Every selected Level-1 test finishes below five seconds.

## Input Robustness

No new file-format or configuration loader was added by the latest repair. CLI argument parsing has control, repeated/comma-separated, mixed-case, absent, empty and unknown-name coverage. SVG/CSS rendering grammars are outside the requested configuration matrix.

The existing internal JSON sidecar is regenerable cache data: unreadable or structurally invalid records cause a miss and rerender. Source inspection confirms required typed fields without default/optional substitutions for font decisions. Public-render tests exercise missing, malformed, incomplete and old records and restored cache hits. The code-smell search finds intentional `.ok()` conversion to a cache miss, plus text-grammar fallbacks. No exhaustive fabricated-sidecar matrix or new validation layer is claimed or required here. Binary permission/table checks cover malformed fonts and unsupported outlines through the existing injected-font tests.

## Commands and Evidence

- `biscuit-visualized`: `just test embed_` — 91 passed, 0.583 s total.
- `darkmatter`: `just test feature_flag` — 11 passed, 0.207 s total.
- `darkmatter`: `just test mermaid` — 79 passed, 0.986 s total, including all six page embedding tests and both setup siblings.
- `darkmatter`: `just test svg_sanitizer` — 35 passed, 0.423 s total; generated SVG control 0.359 s.
- `darkmatter`: `BISCUIT_BROWSER_REQUIRED=1 just test-browser browser_embedded_mermaid_font_draws_every_label` — one passed, 4.024 s total, headless, without focus changes.
- Root: `just check-tier-coverage darkmatter` and `just check-tier-coverage biscuit-visualized` — both passed, zero stranded tests.

Counts overlap where filters select the same test; they are not added together as a measure of rigor. Nextest run durations above exclude compilation. Cargo briefly waited for shared build/cache locks and emitted the existing macOS unwind-section linker warning; no selected test failed.

Full suites, lint, golden comparisons and the two-host experiment were not rerun by this review. The repair records passing lint and 42 previously identified JSON-order snapshot failures in the full Darkmatter run; this review does not claim a clean full-workspace run or independently reclassify those failures. Readiness here assesses the specified feature and the verified repairs.

Only this review, the previous review's `next` pointer, and the specification's review count/completion metadata were changed. Implementation claims in the previous review remain unchanged. The feature directory remains in place for the author's review workflow.
