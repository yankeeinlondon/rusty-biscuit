---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F3 (high): The specified command-line opt-in is still unavailable"
    - "R2-F4 (high): The recorded browser comparison does not meet the specified width criterion"
    - "R4-F1 (medium): A sanitizer render test misses the macOS font-scan setup"
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
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-24T11:18:20-07:00"
spec: 2026-10-21-mermaid-embedded-font/spec.md
implemented: false
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/log.md
description: "A **feature** review of `2026-10-21-mermaid-embedded-font/spec.md`"
feature: 2026-10-21-mermaid-embedded-font/review-4.md
previous: 2026-10-21-mermaid-embedded-font/review-3.md
next: 2026-10-21-mermaid-embedded-font/review-5.md
---

# Review 4: not production ready

Two existing findings remain blocked. The specified command-line option is still missing, and the recorded browser comparison still exceeds the specified bounding-box tolerance. One new test-speed defect was found in an existing sanitizer control. No repair regression or sibling of an earlier font finding was found. The earlier three font defects remain fixed. Pending human inspection of recorded design choices does not independently block readiness.

This review read the specification, plan, all earlier reviews, both implementation logs, and the latest repair's Finding Ledger before checking the current implementation. The latest repair changed records and investigated browser measurements; it did not implement either outstanding requirement. Its completion claims are assessed below against source and test results.

## Previous Finding Verification

Child IDs refer to the instance tables in review #2 and the expanded Finding Ledger in `log.md`. Every child of a previously repaired finding is included below, alongside both findings carried by review #3.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1 — Whole-font embedding retains shaping that layout disables | R2-F1.1–1.4: regular, bold, both mixed subset/whole combinations | verified fixed | `the_opt_in_measures_each_weight_as_its_emitted_file_draws` passes all four permission combinations, comparing decoded output shaping with measurement. `LabelMeasurer::face_for` selects layout tables per emitted weight's `unsubset` value. | Nothing. |
| R2-F1 — subset and collection controls | R2-F1.5–1.6 | verified fixed | Same test checks plain subset advances and two collection members; plain-advance and whole-face node-layout tests pass. | Nothing. |
| R2-F1 — other outputs, tag spacing, explanatory docs | R2-F1.7–1.9 | verified fixed | `without_html_static_svg_the_flag_changes_nothing` passes. Inspected both `prepare` passes: gitGraph spacing and rendering use the same selected measurer. Rustdoc and topic pages distinguish plain subset advances from whole-font shaping. Review #3's 209 passing golden comparisons remain prior evidence; goldens were not rerun here. | Nothing. |
| R2-F2 — Reusing one face for two weights drops bold-only characters | R2-F2.1–2.2: fresh and cached | verified fixed | `one_installed_face_carries_both_weights_rules_fresh_and_cached` passes, checking 400/700 rules, bold-only coverage, and equal fresh/cached CSS. Builder deduplication includes family, weight, and face identity. | Nothing. |
| R2-F2 — regular-only, distinct faces, aliases, bold-only | R2-F2.3–2.6 | verified fixed | All four regression/control tests pass: `one_installed_face_drawn_regular_only_gets_one_rule`, `separate_regular_and_bold_faces_get_a_rule_each_fresh_and_cached`, `requested_families_choosing_one_family_share_its_rules_and_union`, and `bold_only_use_of_a_face_serving_both_weights_embeds_one_bold_rule`. | Nothing. |
| R2-F2 — aggregation/comments and installed Inter | R2-F2.7–2.8 | verified fixed | Inspected whole-choice aggregation and family/weight collection. Inter attribution, installed-Inter substitution, and installed/bundled Inter page-collision tests pass. | Naming policy remains a recorded human review item, not a remaining defect. |
| R2-F3 — The specified command-line opt-in is still unavailable | R2-F3.1 parsing/wiring; R2-F3.2 CLI and frontmatter-negative tests; R2-F3.3 library controls | still blocked | Current searches find `embed_mermaid_font` in Darkmatter library/page code, no activation in CLI source, and no shared feature option. Prerequisite spec remains `implemented: false`. All six Darkmatter API/page controls pass. | R2-F3.1–3.2; carried below. |
| R2-F4 — The recorded browser comparison does not meet the specified width criterion | R2-F4.1–4.2 both generated pages; R2-F4.3 advance diagnostic; R2-F4.4 no-font controls; R2-F4.5 browser loading test | still blocked | Original readings remain 0.673/0.969 px maximum bounding-box differences. Latest repair ledger reproduces 0.673 px and reports unsuccessful markup experiments; no metric amendment is recorded. Source inspection confirms the committed browser test checks font loading/coverage rather than cross-host bounding boxes. | R2-F4.1–4.2; carried below. See commands for this review's browser result. |
| R2-F5 — An unused restricted weight changes the font of permitted labels | R2-F5.1–5.5: symmetric unused-weight cases, used prohibited weight, allowed control, cache reconstruction | verified fixed | `only_a_drawn_refused_weight_makes_the_family_inter_fresh_and_cached` passes all four cases through selection, emitted rules, comments, warnings, and cache hits. Inspected `forced_inter` and required sidecar replay field. | Nothing. |
| R2-F5 — page aggregation, prior tests, explanatory comment | R2-F5.6–5.8 | verified fixed | Per-diagram page aggregation, drawn/undrawn refused-weight tests, and pre-layout recording tests pass; comment priority preserves the substitution explanation. | Per-diagram policy remains recorded for human review; no new ruling requested. |

Review #1 had no formal findings. Its unused-weight discrepancy became R2-F5 and was repaired in iteration 2; its missing-option and measurement questions became R2-F3 and R2-F4. Its typography and supported-format questions remain human review items. No previously blocked finding became newly unblocked in this iteration, and no repair is classified as partial.

## Unblocked Findings

### R4-F1 (medium): A sanitizer render test misses the macOS font-scan setup

**Authority — defect:** Darkmatter's existing [generated-flowchart sanitizer control](../../../darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs:1154) renders through the host font chain but is excluded from the existing macOS `coretext-small-bundle` setup in [.config/nextest.toml](../../../.config/nextest.toml:58). On this ordinary development checkout it runs for 6.725 seconds in the sanitizer group and 10.293 seconds alone, exceeding the review's five-second L1 limit. Applying the existing setup's `CFProcessPath=/usr/bin/true` environment produces the same passing assertions in 1.146 seconds. This is an existing adjacent test defect, not a font-output regression introduced by this feature.

**Class:** a host-font-rendering L1 test is omitted from the existing setup that prevents macOS from scanning the test executable's large build directory.

The sweep covers the changed embedding tests, their setup selection, and the sanitizer control reached while verifying the changed sanitizer. It does not expand into unrelated package tests. The embedding modules and page tests were run normally; the failing control was additionally run with the setup environment, preserving its input and assertions.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R4-F1.1 | Darkmatter `svg_sanitizer::tests::merman_flowchart_survives_with_labels_and_shadow` and its nextest setup selection | Existing generated flowchart, ordinary `just test` invocation on macOS | 6.725 s grouped; 10.293 s isolated; 1.146 s with the existing setup environment | Apply the established macOS font-scan setup so the L1 control stays below five seconds |
| R4-F1.2 | Darkmatter `mermaid_embedded_font` L1 page tests | Existing page fixtures, including both themes and opt-in states | Clean: setup runs; all six tests take 0.666–4.327 s | Preserve setup and coverage |
| R4-F1.3 | biscuit-visualized's seven embedding test modules | Existing permission, collection, subsetting, rendering, comments, cache, and page fixtures | Clean: package-wide L1 setup runs; all 91 tests below five seconds | Preserve setup and coverage |
| R4-F1.4 | Darkmatter `mermaid_font` unit helpers and browser-tier consumer | Failure/markup helpers; browser font loading | Clean by contract: unit helpers do not enumerate host fonts; the browser test launches Chrome and is deliberately excluded from the executable-bundle override | Keep pure helpers unmodified and browser processes outside the override |

The smallest repair is adding the exact generated-flowchart test to the existing macOS-only nextest setup filter. No new setup mechanism, timeout increase, test rename, or production font change is needed. The cause and remedy already appear in the `os` skill's macOS CoreText guidance; no new OS fact needs to be added there.

## Blocked Findings

### R2-F3 (high): The specified command-line opt-in is still unavailable

**Authority — spec:** requirement R-E1 and acceptance criterion AC-E9 require `--feature embed-mermaid-font` to set `HtmlOptions::embed_mermaid_font`. The exposure decision requires a repeatable, comma-separated, case-insensitive feature option that rejects unknown names. Current CLI source still provides neither the option nor its embedding wiring.

**Class:** a required user entry point and its verification are missing while the library behavior exists.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F3.1 | Darkmatter CLI argument parsing and HTML option construction | `md --feature embed-mermaid-font` | No shared feature argument or embedding activation in current CLI source; unchanged from the binary rejection recorded in review #2 | Parse the agreed feature convention and set the HTML option |
| R2-F3.2 | CLI activation and frontmatter boundary tests | Activation; unchanged default-mode output; frontmatter cannot activate embedding | Completed CLI tests remain absent; the repair explicitly leaves them blocked | Verify the specified entry point and no frontmatter activation |
| R2-F3.3 | Library and page controls | Default off, static opt-in, non-static unchanged | Clean: all six Darkmatter embedding tests pass | Preserve existing API behavior |

**Blocker:** the specification explicitly orders the shared option from `2026-10-22-merman-text-fallback` first. That prerequisite remains unimplemented. The latest repair ledger records no scope authorization or author decision changing this dependency. The smallest completion path remains implementing the prerequisite option, wiring this feature, and adding the missing tests. A standalone option would contradict the existing exposure decision.

### R2-F4 (high): The recorded browser comparison does not meet the specified width criterion

**Authority — spec:** AC-E1 requires comparing “every label's bounding box” with widths equal “within 0.5 px.” The original experiment records four comparisons outside that limit. The latest repair investigated the discrepancies but neither achieved the written criterion nor recorded an author amendment.

**Class:** acceptance evidence substitutes a different measurement for the explicitly required one.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F4.1 | macOS-generated page on both browser readers | 28 labels across five diagram families | Original maximum 0.673 px; latest ledger reproduces it, including `CUSTOMER`, `ORDER`, and `LINE_ITEM` | Every bounding-box width within 0.5 px |
| R2-F4.2 | Linux-generated page on both browser readers | Same corpus | Original maximum 0.969 px on `Yes`; latest repair did not regenerate this page | Every bounding-box width within 0.5 px |
| R2-F4.3 | Advance-width diagnostic | Both pages | Clean diagnostic: recorded maximum 0.0000 px | Retain diagnostic; using it for acceptance requires the recorded decision |
| R2-F4.4 | No-font sensitivity controls | Both pages with font rules removed | Clean: recorded width differences reach about 29 px | Demonstrate that font fallback changes widths |
| R2-F4.5 | Committed headless-browser check | Actual font loading and label coverage | Source verifies loaded faces, no font network requests, and character coverage; it has no cross-host bounding-box assertion | Preserve loading coverage and resolve the separate measurement requirement |

**Blocker:** the earlier reviews reserve the metric/tolerance choice for the author. The latest investigation attributes the excess to Linux Chromium's glyph-ink rounding, reports that tested page markup changes do not remove it, and reports a passing macOS-page comparison only when the Linux browser is launched with device scale factor 4. These are evidence for a decision, not approval to impose a reader launch setting or replace the criterion. The smallest resolution is to record an accepted metric/tolerance; no new escaping layer, font machinery, or browser matrix is requested. This finding concerns recorded failing measurements, not absent cross-OS proof.

## Recurrence

Both carried findings are unchanged instances from review #2 and review #3. Review #1 recorded the same missing-option and browser-measurement questions before assigning findings. Neither is a regression or a newly discovered sibling. R4-F1 is a newly established test setup defect; none of the earlier reviews reported this control as a finding. It is not a recurrence of the font shaping, weight identity, permission, CLI, or measurement findings. The instance tables preserve the full prior sweep, including clean controls. The new browser investigation explains the existing failure without closing it; `recurrence: true` reflects that continuity.

## Requirement and Verification Levels

This feature produces browser font output. In-process tests appropriately verify font permission decisions, decoded font bytes, page markup, logging, cache behavior, and API wiring. Actual browser loading requires the browser tier. Terminal rendering and keyboard injection are outside this feature's scope.

| Requirements | Appropriate verification and present evidence |
| --- | --- |
| AC-E1: consistent reader widths | Two-reader headless-browser experiment; fails the specified metric, R2-F4 |
| AC-E2: sanitizer URL boundary | L1 sanitizer tests, generated-page CSS check, and browser loading of admitted font data |
| AC-E3–E4: character union and rule counts in both themes | L1 decoded subsets and public page output; shared-face and alias regressions pass |
| AC-E5, E13: selected and measured face, permissions | L1 injected fonts, actual label layout, emitted-file shaping, refused used/unused weights, cache reconstruction |
| AC-E6–E8, E12: size, failure, licensing, subset tables, browser check | L1 decoded tables/logging/failure assertions pass; first Chrome experiment recorded; committed browser tier checks loading and glyph coverage |
| AC-E9: default, CLI activation, non-static behavior | L1 defaults and page controls pass; required CLI verification absent, R2-F3 |
| AC-E10: cache separation | L1 distinct identities, insertion orders, missing/damaged sidecar, old record, and fresh/hit equality pass |
| AC-E11, E14–E15: docs, page comments, attribution | Inspected topic docs and dependency entries; L1 comment escaping, reasons, and attribution assertions pass |

All seven embedding test modules are declared under the library's `image` feature, enabled by the package's all-features CI configuration. Darkmatter declares its L1 and feature-gated browser targets, and has a live browser recipe. Both tier-coverage checks pass with zero stranded tests.

## Input Robustness

The SVG/CSS collector and sanitizer are text/rendering machinery, excluded from the configuration matrix. Binary font permission validation is covered by the permission and face tests, including malformed tables and unsupported outlines.

The JSON sidecar is regenerable internal cache data. Its defined invalid-document result is a cache miss and rerender. Required typed fields are `requested`, `forced_inter`, `families`, `regular`, and `bold`; no `serde(default)` or optional field silently supplies font decisions. Existing public-render tests remove the sidecar, corrupt JSON, omit required structure, and remove the replay field, then verify rerender and restored cache hits. Inspection of `read_font_record` confirms parsing errors take that miss path. The code-smell search found intentional cache-miss `.ok()` conversion and text-grammar fallbacks. This review does not claim an exhaustive public-API field-shape matrix was executed; fabricated internal sidecar documents do not justify a new validation layer or finding.

## Commands and Evidence

- `biscuit-visualized`: `just test embed_` — 91 passed; every selected test below five seconds. This includes all earlier repaired cases and controls.
- `darkmatter`: `just test mermaid_embedded_font` — six passed, 0.666–4.327 seconds per test.
- `darkmatter`: `just test svg_sanitizer` — 35 passed. The existing generated-flowchart control took 6.725 seconds; its isolated follow-up is recorded below.
- Root: `just check-tier-coverage biscuit-visualized` and `just check-tier-coverage darkmatter` — both passed; zero stranded tests.

- `darkmatter`: `just test merman_flowchart_survives_with_labels_and_shadow` — assertions passed in 10.293 s, above the L1 budget. `CFProcessPath=/usr/bin/true just test merman_flowchart_survives_with_labels_and_shadow` — passed in 1.146 s, confirming R4-F1's minimal remedy without editing configuration.
- `darkmatter`: `BISCUIT_BROWSER_REQUIRED=1 just test-browser browser_embedded_mermaid_font_draws_every_label` — passed in 24.875 s. This is the browser tier, where the five-second L1 limit does not apply; no window focus was requested.

All executed assertions passed; R4-F1 records the separate L1 time-budget failure. No full package suite, golden regeneration, or new two-host experiment is claimed. No source, tests, documentation, implementation logs, or snapshots were changed by this review. Review #3's implementation claims are preserved; this review updates only the review chain and specification review count.
