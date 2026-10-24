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
      Evidence for the browser width ruling (iteration 3 investigation, no code changed). The four bounding-box excesses are Linux Chromium's getBBox adding glyph ink rounded outward to whole pixels. They also appear without an embedded font, and no page markup tried removes them: text-rendering, font-kerning, font smoothing, and a CSS scale all leave 0.673 px. SVG text lengths agree within 0.031 px, and advance widths agree to 0.0000 px. A Linux reader at device scale factor 4 is within 0.5 px (0.208 px). Among the tested approaches, only changing the reader device scale factor met the bounding-box tolerance.
has_blocked_findings: true
blocked: true
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-24T12:00:06-07:00"
spec: 2026-10-21-mermaid-embedded-font/spec.md
implemented: false
description: "A **feature** review of `2026-10-21-mermaid-embedded-font/spec.md`"
feature: 2026-10-21-mermaid-embedded-font/review-5.md
previous: 2026-10-21-mermaid-embedded-font/review-4.md
---

# Review 5: not production ready

The macOS test-setup defect is fixed, including the two additional tests found during repair. No new finding, regression, or missed sibling was found. Two previously reported requirements remain blocked: the command-line opt-in is unavailable, and the recorded browser bounding-box comparison exceeds the specification's tolerance. Neither prerequisite has changed. Pending human review of the other recorded design choices does not independently block readiness.

This review read the specification, relevant plan decisions, all four earlier reviews, the original implementation evidence, and the repair log's Finding Ledger before verifying source and tests. Implementation completion flags were not treated as proof. The review uses the biscuit-visualized, rust-testing, just, and os skills.

## Previous Finding Verification

Child IDs identify the required instances in review #2 and the expanded repair ledger in `log.md`. Rows below include every finding from review #4, earlier repaired findings, and the additional setup instances discovered by the repair.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R4-F1 — A sanitizer render test misses the macOS font-scan setup | R4-F1.1: generated-flowchart sanitizer control | verified fixed | The macOS-only setup filter in `.config/nextest.toml` now selects Mermaid/Merman-named Darkmatter L1 tests. An isolated canonical run reports `SETUP PASS`, then passes in 1.081 s; the sanitizer group passes this control in 0.479 s. | Nothing. |
| R4-F1 — preserve existing setup and tier boundaries | R4-F1.2–1.4: six page tests; seven embedding modules; pure helpers and browser exclusion | verified fixed | All six page controls pass in `just test mer`; all 91 embedding tests pass with setup. Helper tests run in-process. The name filter explicitly excludes browser and other resource tiers; the browser run has no CoreText setup and passes headlessly. | Nothing. |
| R4-F1 — additional host-font-rendering tests found during repair | R4-F1.5: code-renderer SVG control and installed terminal-hook control | verified fixed | Both names match the new filter and both tests pass in `just test mer`. The code-renderer control renders SVG in-process; the terminal-hook control renders through the installed hook without launching a terminal application. | Nothing. These are repaired siblings found by the implementer, not new findings in this review. |
| R2-F1 — Whole-font embedding retains shaping that layout disables | R2-F1.1–1.4: regular, bold, both mixed subset/whole combinations | verified fixed | `the_opt_in_measures_each_weight_as_its_emitted_file_draws` passes all combinations against decoded emitted files. In biscuit-visualized, `LabelMeasurer::face_for` uses each selected weight's `unsubset` flag to retain shaping for whole fonts. | Nothing. |
| R2-F1 — subset, collection, other targets, spacing consumer, docs | R2-F1.5–1.9 | verified fixed | Subset/whole measurement and node-layout tests, collection extraction, and non-static output controls pass. Inspected both `prepare` passes: gitGraph spacing uses the selected measurer, including forced Inter. Rustdoc and both embedding topic pages distinguish plain subset advances from whole-font shaping. | Nothing. Golden comparisons were not repeated; review #3 records their prior passing result. |
| R2-F2 — Reusing one face for two weights drops bold-only characters | R2-F2.1–2.2: fresh and cached | verified fixed | `one_installed_face_carries_both_weights_rules_fresh_and_cached` passes, checking both rules, decoded bold-only coverage, and fresh/hit equality. Rule identity includes family, weight, and face. | Nothing. |
| R2-F2 — controls, aggregation, comments, installed Inter | R2-F2.3–2.8 | verified fixed | Regular-only, distinct-face, aliases/union, bold-only, attribution, installed-Inter substitution and page-collision tests pass. Inspected `page_font` whole-choice aggregation and the collector's family/weight grouping. | Nothing. Existing naming-policy review item is preserved. |
| R2-F3 — The specified command-line opt-in is still unavailable | R2-F3.1–3.3: parsing/wiring; CLI and frontmatter-negative tests; library controls | still blocked | Search of `darkmatter/cli/src` finds no feature argument or embedding activation. The shared-feature prerequisite still says `implemented: false`; the six library/page controls pass. | R2-F3.1–3.2, carried below. No decision changed this dependency. |
| R2-F4 — The recorded browser comparison does not meet the specified width criterion | R2-F4.1–4.5: both generated pages; advance diagnostic; no-font controls; browser loading | still blocked | Re-read original two-host measurements: maxima 0.673/0.969 px, four of 56 comparisons over 0.5 px; advances agree, no-font controls differ by about 29 px. The spec still requires bounding boxes within 0.5 px. The committed loading/coverage test passes again, without asserting that cross-host criterion. | R2-F4.1–4.2, carried below. No metric/tolerance amendment was recorded. |
| R2-F5 — An unused restricted weight changes the font of permitted labels | R2-F5.1–5.5: both unused-weight directions, used refused weight, permitted control, cache reconstruction | verified fixed | `only_a_drawn_refused_weight_makes_the_family_inter_fresh_and_cached` passes through choices, output, comments, logging and cache equality. `forced_inter` checks actual used weights and the sidecar requires its replay list. | Nothing. |
| R2-F5 — page aggregation, former conservative tests, explanatory comment | R2-F5.6–5.8 | verified fixed | Per-diagram page choices, drawn/undrawn refusal controls, pre-layout recording, and substitution comment assertions pass; inspected comment reason ranking. | Nothing. The per-diagram interpretation remains a recorded design review item. |

Review #1 had no formal findings. Its missing-option and measurement questions are the carried R2-F3 and R2-F4; its unused-weight discrepancy was repaired as R2-F5. The repair of R2-F5 followed the specification's used-weight rule, as recorded in the iteration-2 ledger; this review does not reopen that choice. No previously blocked finding became newly unblocked here, and no individual repair is partial. The overall implementation remains incomplete because two findings remain blocked.

## Unblocked Findings

None.

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

R2-F3 and R2-F4 are unchanged instances from reviews #2, #3, and #4, preceded by the same questions in review #1. They are neither repair regressions nor newly discovered siblings. Their complete existing instance tables, including clean controls, are retained above. `recurrence: true` records those still-open classes. R4-F1 and its repair-discovered siblings are verified fixed; no new setup instance is reported.

## Requirement and Verification Levels

This feature emits browser font files and page markup. In-process Level-1 tests can verify font permissions, decoded subsets, measured widths, cache behavior and page assembly. Font loading and drawing need an actual browser, provided by the headless browser tier. Terminal display and keyboard-input requirements are outside this spec, so no Level-2 terminal capture or Level-3 keyboard injection is required.

| Acceptance requirements | Appropriate verification and result |
| --- | --- |
| AC-E1: equal label widths across readers | Two-reader browser experiment remains outside the written bounding-box tolerance; R2-F4. The same-host loading test cannot close it. |
| AC-E2: narrow sanitizer URL admission | 35 L1 sanitizer tests pass, including font-data positives, external/data URL negatives, malformed rules, and the generated SVG control; browser font loading passes. |
| AC-E3–E4: character union and per-weight rule count, both themes | L1 decoded subset and public page-output tests pass, including one face serving both weights and multiple requested families sharing rules. |
| AC-E5, E13: selected face, permissions and matching layout | L1 injected-font tests, emitted-file shaping comparisons, node widths, used/unused refused weights, and cache-hit controls pass. |
| AC-E6–E8, E12: size, failure, licensing, required tables and initial browser check | L1 size/logging/failure, retained-table and license tests pass. Original first-task browser evidence is recorded; the committed headless loading/coverage test passes again. |
| AC-E9: default off, CLI activation and non-static identity | L1 API defaults and unchanged non-static output pass. Required CLI path and boundary tests remain absent; R2-F3. |
| AC-E10: cache separation and reconstruction | L1 plain/opt-in separation, fresh/hit equality and missing/damaged/incomplete sidecar tests pass. |
| AC-E11, E14–E15: documentation, explanations and attribution | Inspected both topic pages and dependency records. L1 comment escaping, reasons, whole-font explanation and Inter-only attribution tests pass. CLI documentation correctly remains marked planned. |

All seven embedding modules are compiled under the library's `image` feature, enabled by its all-features CI configuration. Darkmatter's L1 main declares the page tests; its declared browser target includes the loading test and requires `browser-tests`, present in CI test features. Both area's tier-coverage checks pass with zero stranded tests. The relevant browser recipe is live.

## Input Robustness

SVG/CSS collection and sanitization process rendering/text grammar, so the configuration-field matrix does not apply. Binary font permission checks cover unreadable/short tables, unsupported outlines, collection indices and permission-bit combinations.

The JSON sidecar is internal regenerable cache data. Invalid data must cause a cache miss and rerender. `FontRecord` requires typed `requested`, `forced_inter`, `families`, `regular`, and `bold` fields, with no default or optional field substituting font decisions. Public-render tests for missing, corrupt, structurally incomplete and old records pass. `read_font_record` converts read/parse errors to misses. The code-smell search found intentional cache-miss `.ok()` handling and text-grammar fallbacks, rather than a new load-bearing configuration default. An exhaustive fabricated-sidecar shape matrix was not executed; this review makes no broader validation claim.

## Commands and Evidence

- `biscuit-visualized`: `just test embed_` — 91 passed, 3.566 s total; every selected L1 test below five seconds.
- `darkmatter`: `just test merman_flowchart_survives_with_labels_and_shadow` — passed in 1.081 s with `SETUP PASS`.
- `darkmatter`: `just test mer` — 331 passed, 7.075 s total. The substring also selects unrelated names containing `mer`; this count is not a count of embedding requirements. It includes all six embedding page controls and both repair-discovered setup siblings.
- `darkmatter`: `just test svg_sanitizer` — 35 passed, 0.550 s total; generated-flowchart control 0.479 s.
- `darkmatter`: `BISCUIT_BROWSER_REQUIRED=1 just test-browser browser_embedded_mermaid_font_draws_every_label` — passed in 13.616 s, headless, without focus changes. Its browser-tier duration is outside the five-second L1 limit.
- Root: `just check-tier-coverage biscuit-visualized` and `just check-tier-coverage darkmatter` — passed, zero stranded tests.

An initial attempt to combine names with `|` through `just test` failed because the recipe interpreted the pipes as shell syntax; it supplied no verification evidence. The corrected commands above completed successfully. Darkmatter compilation emitted the existing macOS linker warning about a large unwind section; no test assertion failed.

No new two-host experiment, full package suite, lint run, or golden regeneration is claimed. Only this review, the previous review's `next` pointer, and the specification's review count were edited. Existing implementation claims and logs were preserved; the spec remains incomplete.
