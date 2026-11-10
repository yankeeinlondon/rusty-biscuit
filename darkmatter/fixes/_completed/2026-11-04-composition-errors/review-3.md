---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-05T23:13:44-08:00"
spec: 2026-11-04-composition-errors/spec.md
implemented: false
description: "A **fix** review of `2026-11-04-composition-errors/spec.md`"
fix: 2026-11-04-composition-errors/review-3.md
previous: 2026-11-04-composition-errors/review-2.md
---

# Composition errors: review 3

The fix is **production ready** under this review's criteria. The remaining finding, R1-F1, is verified fixed, including all 37 child instances in the latest repair ledger. The other four findings remain fixed. No new finding or decision requiring a human was identified.

## Previous Finding Verification

Read both earlier reviews and both Finding Ledgers in `log.md` before assessing the repair. Neither earlier review contains human rulings. The informational decisions already recorded there are retained; this review does not request that the author reaffirm them. The required instances include the implementer's additional ledger rows, not just the original reproductions.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — HTML reference discovery silently drops multiline tags and media targets; remaining raw-text contents split across nodes become references | Original R1-F1.1–R1-F1.31, with R1-F1.17 expanded to R1-F1.17.1–R1-F1.17.37 in the second repair ledger | **verified fixed** | Read the shared HTML walker, Markdown-native filtering, public HTML accessors, metadata replacement, rewriting, and DMLS consumer. All 654 `reference_` tests pass, including every `reference_html_discovery` row and the CLI validation/graph/compose regressions. The separately selected DMLS raw-text projection also passes. The instance table below accounts for all 37 latest ledger rows. Original multiline discovery, media classification, CSS rebasing, spans, frontmatter provenance, and report controls remain green. | None. This closes the partial repair identified in review 2; no newly found sibling remains open. |
| R1-F2 — Diagnostic projection discards known source locations | R1-F2.1–R1-F2.35: schema, conditions, directives, cycles, shell variants, warnings, interpolation, collections, references, frontmatter, iteration, lifecycle/policy delivery, triggers, render, and command fallbacks | **verified fixed** | Checked `compose/report_diagnostics.rs` and the caller's `CommandRun::set_document` fallback. The 595-test contract run includes `diagnostic_source_provenance`, `compose_report_diagnostics`, collection analysis, and link-title warnings. Typed sources and proven positions survive projection; file-only and genuinely source-free cases retain their intended distinction. Reference diagnostic delivery also passes in the reference run. | None; the previous verification stands. |
| R1-F3 — Unlimited component output still uses a million-column sentinel | R1-F3.1–R1-F3.24: adapter, rules, folds, pages, CLI overrides, stderr, nested components, wrapping/truncation, tables, images/diagrams, and remaining numeric-width consumers | **verified fixed** | Checked the width-state adapter and render-context propagation. All 22 `output_component_bridge` tests pass, including bounded controls. Darkmatter's output-width overrides pass in the contract run; page checks pass separately. The adapter retains `Source`/`Unlimited`, and callers can distinguish absent columns from a concrete size. The earlier documented concrete image/viewport budgets and direct-constructor behavior are retained. | None; no additional size guarantee is imposed on the accepted concrete-budget cases. |
| R1-F4 — Frontmatter flow-list references do not report their value locations | R1-F4.1–R1-F4.24: scalar/block/flow forms for both properties, child trails, report/data consumers, comments/quotes/tags/aliases, schema/interpolation positions, writer safety, DMLS, and prepared-only APIs | **verified fixed** | Checked `AuthoredLocator::frontmatter_location`, its read-only fallback, and the distinction from replacement-safe writer ranges. The reference run passes the provenance and report cases for both properties, one-line and multiline flow lists, repeated values, multibyte columns, BOM/CRLF, and distinct child trails. Diagnostic tests also retain frontmatter interpolation positions. No prepared coordinate or key coordinate is substituted when a value's position cannot be proven. | None; the previous verification stands. |
| R1-F5 — The new styled reference report lacks real-terminal verification | R1-F5.1–R1-F5.10: styling, alignment/narrow layout, existing error controls, byte contracts, keyboard scope, graph/compose routes, severity/hints, OSC 8, and shared listing paths | **verified fixed** | All seven `level2_reference_report` tests pass with missing backends configured as failures. Actual tmux captures verify styles, aligned columns, narrow stacking, and stderr delivery; the WezTerm capture verifies location hyperlink parameters and the bare file URI. Level 1 reference/output tests cover streams, exit codes, plain bytes, verbose destinations, and shared report paths. | None; no keyboard interaction was added, so Level 3 is unnecessary. |

### Remaining raw-text class: complete instance check

The correction is shared by the reference readers rather than applied to a particular report. In the darkmatter package, [`visit_html_elements`](../../lib/src/markdown/reference/html.rs) walks HTML nodes in order and carries a spanning raw-text element through its closing tag. The same module supplies inert ranges to Markdown-native readers and masks bodies for public HTML accessors. [`extract_document_references`](../../lib/src/markdown/reference/mod.rs) consequently gives graph, validation, rewriting, and DMLS the corrected records.

Each row below was checked against its test input and assertions, and its test passed in this review. These checks include clean controls as well as repairs.

| Ledger child ID | Required instance | Verification and result |
|---|---|---|
| R1-F1.17.1 | Script at line start and after prose | `script_body_as_a_block`, `script_body_after_prose`: only the inline script remains. |
| R1-F1.17.2 | Style at line start and after prose | Style-body and style-comment rows retain inline CSS and exclude the apparent image. |
| R1-F1.17.3 | Textarea in both layouts | Both textarea-body rows return zero records and issues. |
| R1-F1.17.4 | Title in both layouts | Both title-body rows return zero records and issues. |
| R1-F1.17.5 | Xmp in both layouts | Both xmp-body rows return zero records and issues. |
| R1-F1.17.6 | Iframe body and outer target | Both body rows are empty; `frame_target_after_prose_with_a_body` retains only the outer target at 1:7. |
| R1-F1.17.7 | Noembed in both layouts | Both noembed-body rows return zero records and issues. |
| R1-F1.17.8 | Noframes in both layouts | Both noframes-body rows return zero records and issues. |
| R1-F1.17.9 | Anchor inside textarea | `textarea_holding_an_anchor`: no record. |
| R1-F1.17.10 | Image inside textarea | `textarea_holding_an_image`: no record. |
| R1-F1.17.11 | Script import inside textarea | `textarea_holding_a_script`: no record. |
| R1-F1.17.12 | Stylesheet link inside textarea | `textarea_holding_a_stylesheet`: no record. |
| R1-F1.17.13 | Video inside textarea | `textarea_holding_a_video`: no record. |
| R1-F1.17.14 | Audio inside textarea | `textarea_holding_an_audio`: no record. |
| R1-F1.17.15 | Source inside textarea | `textarea_holding_a_source`: no record. |
| R1-F1.17.16 | Iframe inside textarea | `textarea_holding_an_iframe`: no record. |
| R1-F1.17.17 | Metadata inside textarea | `textarea_holding_a_meta` and public-accessor controls return no metadata. |
| R1-F1.17.18 | Style and both CSS readers inside textarea | `textarea_holding_a_style_with_css_targets`: no style, import, or font record. |
| R1-F1.17.19 | Markdown link inside textarea | `textarea_holding_a_markdown_link`: no record. |
| R1-F1.17.20 | Markdown image inside textarea | `textarea_holding_a_markdown_image`: no record. |
| R1-F1.17.21 | Validation of the inline script template | Whole-body public-API test gives one script and no issue; CLI validation exits 0. |
| R1-F1.17.22 | Graph's validation decision | Both CLI script-template and style-comment cases exit 0. |
| R1-F1.17.23 | Compose's reference gate | Both CLI script-template and style-comment cases exit 0; the active missing-link control still fails. |
| R1-F1.17.24 | Actual CSS import beside a comment | `css_import_beside_a_comment_after_prose` retains `./gone.css` at 1:43 and excludes the apparent image; public CSS accessor agrees. |
| R1-F1.17.25 | Block script and inline/fenced code controls | Script-at-line-start control retains one script; both code controls return zero records. |
| R1-F1.17.26 | Active tags after prose | Position test retains both real targets at 1:6 and 1:26; active missing link after a script still exits 1. |
| R1-F1.17.27 | Public accessors, rewriting, and DMLS | Public accessor controls exclude inert targets and retain active ones; composing a child rewrites only active targets; DMLS separately retains only the active link. |
| R1-F1.17.28 | Metadata mutation | `setting_a_meta_tag_leaves_one_in_raw_text_or_code_alone` preserves the original text and appends the active tag. Shared extracted spans also exclude comments. |
| R1-F1.17.29 | Public HTML link/image accessors | Both public-accessor tests check inert exclusion and active retention, in block and after-prose layouts. |
| R1-F1.17.30 | Public Markdown link/image accessors | The same tests assert the Markdown accessors independently. |
| R1-F1.17.31 | Apparent closer in inline code | Whole-body test retains the script tail and the actual CSS import at 1:25. |
| R1-F1.17.32 | Active tag after a long body line | `a_tag_after_a_closer_on_a_long_body_line_stays_active_in_every_accessor` retains the HTML image after masking. |
| R1-F1.17.33 | CSS-reader controls | Real import beside a comment and block-quote import retain their own positions; style text nested in textarea yields no CSS records. |
| R1-F1.17.34 | Composition directive inside a raw body | `a_file_directive_in_a_raw_body_is_still_a_composition_directive` retains the missing transclusion; HTML exclusion does not disable composition syntax. |
| R1-F1.17.35 | Layout and delimiter siblings | Heading/table openers, mixed case, block quotes, CRLF, escaped/code openers, code/fence closers, consecutive elements, paragraphs, and resuming after the closer all have passing named rows. |
| R1-F1.17.36 | Unclosed opener | Three `unclosed_*` controls preserve subsequent active references. This is the documented bounded discovery choice, not a requirement to emulate a browser through end of file. |
| R1-F1.17.37 | Heading discovery inside a raw body | Read the separate heading/TOC path: it remains unchanged. The previous informational decision preserves Markdown heading discovery and existing fragment-validation behavior; this repair does not claim to change it. |

## Unblocked Findings

None.

## Blocked Findings

None. No prerequisite or author decision is needed to close a finding.

## Recurrence

No recurring defect remains. Review 2 identified a partial repair of R1-F1.17, not a new failure in the corrected multiline-resource cases. The latest repair closes that instance and its expanded sibling list at shared extraction. No new finding ID is introduced.

## Spec coverage and verification levels

Reviewed the specification, both supporting spikes, earlier findings, repair ledgers, shared output trait/dispatch, CLI output registry and delivery, report construction, authored-location projection, width adapters, raw-text extraction and consumers, and relevant docs and test registration.

| User-facing requirement | Appropriate evidence checked |
|---|---|
| Reason/construct phrases, grouping, transclusion trails, summary, severity visibility, and clean reports | Level 1 report-builder and CLI reference tests; Level 2 for the styled presentation. |
| Authored character positions after preparation, inserted references, frontmatter values, sections, and in-memory null positions | Level 1 provenance and diagnostic tests, including LF/CRLF/BOM and multibyte controls. |
| JSON5 link titles, legacy/malformed warnings, location queries, and plain/verbose fallbacks | Level 1 output, reference-report, and link-title tests; Level 2 for received OSC 8 links. |
| Output families/defaults, removed arguments, schema conformance, one-value data output, stderr separation, and exit status | Level 1 output-contract and command-conformance tests. These are byte/data contracts and do not need an emulator. |
| Plain structural rendering, width precedence/reflow, unlimited natural sizes, overrides, and page width | Level 1 output, component, and page tests; Level 2 for report alignment and narrow stacking. |
| Correct reference acceptance and rejection after raw-text repair | Level 1 public library results, shipped CLI tests, rewriting control, and DMLS projection. |

There is no keyboard/input behavior in this fix requiring Level 3. The new tests are compiled by declared targets, included from their `main.rs` modules, and selected by active tier recipes. The tier-coverage check reports zero stranded tests. The configuration input-robustness matrix does not apply: these changes concern text grammars, source-position readers, and output rendering/serialization, rather than configuration admission.

## Checks and limitations

| Check run for this review | Result |
|---|---|
| `just test reference_` in `darkmatter` | 654 passed. |
| `just test diagnostic_source_provenance:: compose_report_diagnostics:: output_width_overrides:: output_ collection_analysis:: link_title_warnings::` in `darkmatter` | 595 passed. |
| `just test output_component_bridge::` in `biscuit-terminal` | 22 passed. |
| `just test raw_text_after_prose_is_not_indexed_while_an_active_link_is page::` in `darkmatter` | 135 passed; checks DMLS's new projection directly and the page controls. |
| `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 level2_reference_report` in `darkmatter` | Seven passed in the CLI's Level 2 target, including tmux and WezTerm. Other selected package targets correctly contain zero matching tests. |
| `just check-tier-coverage darkmatter` | Passed; zero stranded tests. |

All Level 1 runs above recorded no test at or above five seconds. This review adds no test, changes no production code, and makes no cross-OS claim.

The implementer's full-suite run remains **13,978 passed, one failed**, as recorded in `log.md`: the shipped-schema corpus rejects an unrelated research YAML schema using dynamic type syntax that is not implemented. That same failure predates this repair and was disclosed in both earlier cycles. This review does not relabel it as passing, claim a new full-suite run, or add the separate schema feature to this fix's scope. The relevant focused checks above passed.

Only this review, the previous review's `next` link, and the spec's review count/completion metadata are updated. The fix remains in its active directory for the author's review process.
