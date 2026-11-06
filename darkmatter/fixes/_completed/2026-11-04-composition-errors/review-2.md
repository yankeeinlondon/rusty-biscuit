---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (medium): Raw-text contents split across Markdown nodes still become references"
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-05T22:44:11-08:00"
spec: 2026-11-04-composition-errors/spec.md
implemented: false
description: "A **fix** review of `2026-11-04-composition-errors/spec.md`"
fix: 2026-11-04-composition-errors/review-2.md
previous: 2026-11-04-composition-errors/review-1.md
next: 2026-11-04-composition-errors/review-3.md
human_review_items:
  - "A raw-text opener with no closer in any later HTML node makes nothing past its own node inert, so later references stay checked; a browser would run it to the end of the document (R1-F1.17.36)"
  - "A heading inside a multi-paragraph raw-text body stays in the table of contents, so an opt-in fragment check can accept a link to it (R1-F1.17.37)"
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/fixes/2026-11-04-composition-errors/log.md
implemented_by: claude/opus
---

# Composition errors: review 2

The fix is **not production ready**. Four previous findings are verified fixed. HTML reference discovery is partially fixed: its repair ledger claims that text inside scripts and other raw-text elements is excluded, but this fails when Markdown splits the element into separate nodes. One medium finding remains, repairable without a human decision.

## Previous Finding Verification

The required instances below include the repair's additional child rows in `log.md`, not just the original review's examples. There are 124 child rows across the five findings. A passing test is evidence for its actual input; a test name or a ledger disposition is not evidence for an untested shape. No earlier review contains a `## Human rulings` section.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — HTML reference discovery silently drops multiline tags and media targets | R1-F1.1–R1-F1.31: every HTML/CSS extractor, Markdown controls, validation/compose/graph consumers, source positions, inert text, accessors, rewriting, DMLS, and construct projections | **partially fixed** | Original R1-F1.1–R1-F1.14 reproductions now have dedicated public-API/CLI tests. `just test reference_` passes 578 tests, including `reference_html_discovery`, `reference_provenance`, `reference_report`, and DMLS reference checks. The tokenizer, CSS rebasing, target classification, attribute-value rewriting, and consumer wiring support the additional ledger rows. However, R1-F1.17's raw-text exclusion is only effective inside one parsed HTML node. Fresh CLI reproductions below fail. | R1-F1.17 remains incomplete across node boundaries. The test named `tag_in_an_inline_script_body` starts `<script>` at the beginning of a line and exercises an HTML block, not an inline element after prose. The remaining work is carried under R1-F1 below. |
| R1-F2 — Diagnostic projection discards known source locations | R1-F2.1–R1-F2.35: schema, conditions, directives, cycles, all shell context variants, warnings, interpolation, collections, references, frontmatter, iteration, lifecycle/policy delivery, triggers, render, and other command fallbacks | **verified fixed** | Read `compose/report_diagnostics.rs`, condition-site provenance, warning attribution, `ReportSink`, and `CommandRun::set_document`. The diagnostic/source and output test run passes 561 tests, including `diagnostic_source_provenance` and `compose_report_diagnostics`. Tests check file-only fallbacks, authored positions after rewrites, child ownership, stdin, source-free errors, all shell context variants, and delivery through policy stops. Reference tests check the shared reference projection. | None. File-only records where authored coordinates are not proven follow the spec's fallback; no extra position-threading requirement is introduced. |
| R1-F3 — Unlimited component output still uses a million-column sentinel | R1-F3.1–R1-F3.24: component bridge, rules, generic folds, pages, all CLI overrides, stderr, nested components, wrapping/truncation, tables, images/diagrams, and remaining numeric-width readers | **verified fixed** | Read the width-state bridge, `Terminal::output_width`/`columns`, render-context propagation, component changes, and CLI overrides. Both sentinel copies are removed. `just test output_component_bridge::` passes all 22 controls and unbounded cases. The 561-test Darkmatter run includes `output_width_overrides` and output contracts; reference tests cover graph/report callers. Remaining concrete image/viewport budgets are bounded and documented; they do not recreate the large fill width. | None. The direct-component bounded-rule control and the documented concrete-size image cases are retained; this review does not reopen those earlier recommendations. |
| R1-F4 — Frontmatter flow-list references do not report their value locations | R1-F4.1–R1-F4.24: both properties and scalar/block/flow forms, child trails, report/data consumers, comments/quotes/tags/aliases, schema/interpolation positions, writer safety, DMLS, and prepared-only APIs | **verified fixed** | Read `AuthoredLocator::frontmatter_location`, the read-only value locator, the tolerant flow locator, and the writer's separate multiline-flow refusal. The passing reference run includes all 98 provenance tests and report/CLI flow-list assertions. These pin 2:12/2:20, 3:3/4:3, distinct child trail lines, LF/CRLF/BOM, character columns, repeated values, and null when a value cannot be proven. The source-position and writer changes preserve the separation between locating a diagnostic and locating bytes safe to replace. | None within the required instances. |
| R1-F5 — The new styled reference report lacks real-terminal verification | R1-F5.1–R1-F5.10: styling, aligned/narrow layouts, existing error blocks, byte contracts, keyboard scope, graph/compose routes, severity/hints, OSC 8, and shared listing paths | **verified fixed** | Read the registered `cli/tests/level2/level2_reference_report.rs` target. `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 level2_reference_report` passes all seven report tests: tmux captures check colors, emphasis, columns, 30-column stacking, and stderr routing; WezTerm capture checks the location's OSC 8 parameters and bare file URI. Public Level 1 tests cover plain bytes, data streams, exit status, verbose destinations, and shared report construction. The report adds no keyboard behavior. | None. Missing backends were configured to fail in the required-backend rerun, so these are not silently skipped backend checks. |

## Unblocked Findings

### R1-F1 (medium): Raw-text contents split across Markdown nodes still become references

**Authority: defect.** Darkmatter's reference extractors in [`reference/html.rs`](../../lib/src/markdown/reference/html.rs:106) exclude raw-text bodies only within each Markdown HTML node. With prose before an opening tag, an element's contents arrive in later nodes, and their apparent links are independently collected. This produces an incorrect reference report and can refuse composition for an embedded JavaScript template string or a textarea's sample text. It is also an incomplete repair of the explicit raw-text exclusion claimed by **R1-F1.17 in `log.md`'s Finding Ledger**. The remaining case is medium because it needs the less common inline raw-text layout; ordinary multiline resource tags from the first review are fixed.

**Class:** reference discovery forgets an enclosing inert-text range when extraction crosses Markdown node or extractor boundaries.

Create `doc.md` with this content, leaving `draft.md` absent:

```html
Note: <script>const label = '<a href="./draft.md">Draft</a>';</script>
```

Run:

```sh
md validate refs doc.md --output json
md graph doc.md --validate --output json
md compose doc.md --output json
```

All three exit **1** and report `dm.reference.missing_hyperlink` for `./draft.md`. The validation result counts two references and places the supposed hyperlink at **1:30**. There is one inline script, and the string inside it is not an authored hyperlink that reference validation should resolve. Removing only `Note: ` makes the same script pass: one inline-script record, no missing-target issue. This is a document containing executable script text, not a request to validate arbitrary strings as HTML.

The node walker at `html.rs:106` independently visits each HTML node. `with_inline_raw_body` at `html.rs:617` recovers a script/style body for its opening node, but does not prevent later nodes inside that body from being extracted again. For other raw-text elements it does not extend the opening node. Meanwhile, the Markdown-native readers in [`reference/local.rs`](../../lib/src/markdown/reference/local.rs:17) independently extract links and images from the same inert range. Correcting only the report or only one HTML classifier leaves the other paths wrong.

The sweep below uses the shipped CLI and temporary files. For container controls, the payload is `<img src="./not-a-resource.png">`; test it with and without a `Note: ` prefix. For extractor controls, place the stated payload inside `Note: <textarea>…</textarea>`. These are additional cases for the already required R1-F1.17 exclusion, kept under the original finding ID rather than reopening the repaired multiline-discovery instances.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.17.1 | Raw `script` body | Block / after prose | Block clean; inline produces a missing-image issue | Exclude the image string in both; retain the actual script record |
| R1-F1.17.2 | Raw `style` body | Block / after prose | Block clean; inline produces a missing-image issue | Exclude comment text in both; retain actual CSS resource reads |
| R1-F1.17.3 | Raw `textarea` body | Block / after prose | Block clean; inline produces a missing-image issue | No image reference in either body |
| R1-F1.17.4 | Raw `title` body | Block / after prose | Block clean; inline produces a missing-image issue | No image reference in either body |
| R1-F1.17.5 | Raw `xmp` body | Block / after prose | Both produce a missing-image issue | No image reference in either body |
| R1-F1.17.6 | Raw `iframe` body | Block / after prose; no outer `src` | Block clean; inline produces a missing-image issue | No reference to the text inside the frame element |
| R1-F1.17.7 | Raw `noembed` body | Block / after prose | Both produce a missing-image issue | No image reference in either body |
| R1-F1.17.8 | Raw `noframes` body | Block / after prose | Block clean; inline produces a missing-image issue | No image reference in either body |
| R1-F1.17.9 | HTML anchor extractor | `<a href="./missing.md">Example</a>` inside inline textarea | Missing hyperlink; exit 1 | No reference |
| R1-F1.17.10 | HTML image extractor | `<img src="./missing.png">` in same container | Missing image; exit 1 | No reference |
| R1-F1.17.11 | HTML script extractor | `<script src="./missing.js"></script>` in same container | Missing script import; exit 1 | No reference |
| R1-F1.17.12 | HTML link extractor | `<link rel="stylesheet" href="./missing.css">` in same container | Missing CSS import; exit 1 | No reference |
| R1-F1.17.13 | HTML video extractor | `<video src="./missing.mp4"></video>` in same container | Missing media target; exit 1 | No reference |
| R1-F1.17.14 | HTML audio extractor | `<audio src="./missing.mp3"></audio>` in same container | Missing media target; exit 1 | No reference |
| R1-F1.17.15 | HTML source extractor | `<source src="./missing.ogg">` in same container | Missing media target; exit 1 | No reference |
| R1-F1.17.16 | HTML iframe extractor | `<iframe src="./missing.html"></iframe>` in same container | Missing frame target; exit 1 | No reference |
| R1-F1.17.17 | HTML meta extractor | `<meta name="description" content="example">` in same container | One metadata record, exit 0 | No metadata record; text inside the textarea is not document metadata |
| R1-F1.17.18 | HTML style and both CSS readers | `<style>@import "./missing.css"; @font-face { src: url("./missing.woff"); }</style>` in same container | Three records; two missing-target issues | No style, import, or font records from textarea text |
| R1-F1.17.19 | Markdown-native link extractor | `[example](./missing.md)` in same container | Missing hyperlink; exit 1 | No reference |
| R1-F1.17.20 | Markdown-native image extractor | `![example](./missing.png)` in same container | Missing image; exit 1 | No reference |
| R1-F1.17.21 | Validation decision | Inline script template above | `valid: false`; two scanned; exit 1 | Inline-script record only; no missing-link error |
| R1-F1.17.22 | Graph classification and validated exit | Same script and inline style-comment controls | Bogus hyperlink/image diagnostics; exit 1 | Do not classify inert contents as resources |
| R1-F1.17.23 | Compose reference gate | Same script and inline style-comment controls | Refuses the document; exit 1 | No reference refusal for those strings/comments |
| R1-F1.17.24 | Actual inline CSS resource control | `Note: <style>/* <img src="./gone.png"> */ @import "./gone.css";</style>` | Correctly discovers `./gone.css`, but also invents an image reference | Keep the CSS import and its location; omit only the comment's image |
| R1-F1.17.25 | HTML block / code controls | Same script at line start, inside inline code, or inside a fenced block | Script block has no missing link; both code forms have zero references | Clean; preserve these exclusions |
| R1-F1.17.26 | Active HTML control | `Text <a href="./gone.md"><img src="./gone.png"></a>` | Both actual targets found, at 1:6 and 1:26 | Clean; exclude only inert ranges, not all later inline nodes |
| R1-F1.17.27 | Public document/composed accessors, link rewriting, and DMLS reference projection | Code inspection of consumers of the shared extractors and `extract_document_references` | No independent raw-text exclusion; they receive the same false records | Apply the correction at shared extraction, preserving active reference spans and rewriting |

**Recommended repair:** determine the existing raw-text elements' body ranges across node boundaries and exclude contained records consistently in the shared extraction paths, including the Markdown-native readers. Keep actual script/style records and real CSS imports. A focused shared exclusion mechanism is sufficient; no new reference syntax, general HTML parser, escaping layer, or exhaustive matrix of arbitrary JavaScript strings is needed. Add a public-API regression with prose before the opener, clean block/code controls, and one CLI case proving the composition gate no longer refuses it. The eight container names above already exist in `RAW_TEXT_ELEMENTS`; this is not a proposal to expand that list.

## Blocked Findings

None. The repair preserves the intended extraction behavior and requires no public-format, policy, or scope decision.

## Recurrence

R1-F1 in `review-1.md` required a sweep of reference extraction and its consumers. Its repair expanded that sweep to inert HTML text as **R1-F1.17**, but the implementation and test cover bodies contained in one HTML node. This is a **partial repair of that required instance**, not a regression in the corrected multiline-resource cases. The test's misleading use of “inline” concealed the missing after-prose shape. The complete remaining list above includes the existing raw-text containers, all HTML classifiers, both CSS readers, Markdown-native links/images, and the decisions that consume their records.

The remaining work has a concrete boundary: inert body ranges for the eight elements already named by the implementation. It should not grow into new HTML syntax support or arbitrary string interpretation. Its priority is medium because no acceptance criterion names the inline raw-text case specifically.

## Verification and scope

Reviewed the specification's output/data/width design and acceptance criteria, its authored-position and plain-output spikes, the previous review, the repair ledger, source projections, extraction and consumer paths, component width changes, CLI overrides, and test target registration. This review changes review metadata only; it does not repair production code.

| Check run for this review | Result | Verification level and purpose |
|---|---|---|
| `just test reference_` in `darkmatter` | 578 passed | Level 1: extraction, graph/report consumers, authored positions, frontmatter flow values, and DMLS reference projection |
| `just test diagnostic_source_provenance:: compose_report_diagnostics:: output_width_overrides:: output_` in `darkmatter` | 561 passed | Level 1: source attribution, CLI delivery, output contracts/conformance, and width overrides |
| `just test output_component_bridge::` in `biscuit-terminal` | 22 passed | Level 1: natural unbounded sizes, no wrapping/truncation/filling, bounded controls, and nested width propagation |
| `just test collection_analysis:: link_title_warnings::` in `darkmatter` | 34 passed | Level 1: located collection failures and carried link-title diagnostics remain intact |
| `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 level2_reference_report` in `darkmatter` | Seven report tests passed | Level 2: actual tmux/WezTerm captures, semantic styling, alignment, narrow layout, stderr routing, and hyperlink parameters |
| `just check-tier-coverage darkmatter` | Passed, zero stranded tests | Declared targets and active tier recipes |
| Fresh `md validate refs`, `md graph --validate`, and `md compose` temporary-file probes | Reproduced the remaining R1-F1 instance and clean controls | Level 1: shipped command wiring and public data results |

The Level 1 test durations in the recorded runs stayed below five seconds. Concurrent test recipes raced on their shared result-recording files: one slow-test recorder warned about a missing XML file, and another recorded zero tests after nextest had reported 561 passing tests. The counts above come from nextest's actual completed runs, not those recorder summaries. This recording issue is not evidence of a production regression in this fix.

Output parsing, removed flags, schemas, byte equivalence, streams, exit status, width resolution, reflow, and authored coordinates have appropriate Level 1 checks. Terminal presentation now has the required Level 2 checks. No input-key behavior is added, so Level 3 verification is unnecessary. Tests are compiled by declared `l1`/`level2` targets and registered in their `main.rs` modules. No test is added or renamed by this review.

The configuration input-robustness matrix does not apply: the changed readers here process Markdown/HTML/CSS text grammars and source positions, and the other changes are output renderers/serializers. No new configuration-loading policy is introduced.

This review does not claim a new full-suite or cross-OS run. The implementation's previously recorded unrelated shipped-schema corpus failure remains an evidence limitation, not a passing result or a new readiness finding. Existing informational recommendations in `review-1.md` remain available for later author review; none needs to be reaffirmed to repair the finding above.
