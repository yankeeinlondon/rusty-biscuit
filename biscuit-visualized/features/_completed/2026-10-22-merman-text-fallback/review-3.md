---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-24T17:47:38-07:00"
spec: 2026-10-22-merman-text-fallback/spec.md
implemented: false
description: "A **feature** review of `2026-10-22-merman-text-fallback/spec.md`"
feature: 2026-10-22-merman-text-fallback/review-3.md
previous: 2026-10-22-merman-text-fallback/review-2.md
---

# Review 3: Merman text fallback

The feature is **production ready** under the specification's review criteria. Both findings from review 2 are verified fixed across every required instance, including the repair ledger's additional documentation instance. Earlier repairs remain intact. There are no new findings, partial repairs, blocked findings, or outstanding author decisions.

Read the specification, plan, both earlier reviews, the original implementation log, and both repair Finding Ledgers in `log.md`. Reviewed the final text backend, component selection, generic diagram hook and finished-width check, Darkmatter capability adapter and public page path, CLI adapters, highlighting, test registration, and affected documentation. Neither earlier review contains a `Human rulings` section. Their accepted implementation decisions remain accepted; informational review items are not author rulings and do not require reaffirmation.

## Previous Finding Verification

Required instances include child rows added by the repair ledgers. The five review-1 findings were already closed in review 2; they are included here to make their regression checks explicit. R2-F1 was the newly found sibling of R1-F1, rather than a partial repair of its original broken-flowchart cases. That sibling is now closed too.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: source-only CLI routes still hide the disabled math capability | R2-F1.1–R2-F1.10: image reference, parse-only validator, both source branches, restrictive CLI flags/environment, narrow width, successful text, parse/valid controls, shared consumers, infallible/hook contracts, documentation | **verified fixed** | Checked every instance below. Both source branches call the same capability-plan validator. Public component tests and shipped CLI probes preserve missing-math errors; successful partial text stays successful. | None. |
| R2-F2: page geometry overrides disabled graphics and emits Kitty images | R2-F2.1–R2-F2.10: default page, five geometry conditions, direct adapter, source mode, hook/numbered renderer, explicit image prohibition | **verified fixed** | Checked every instance below. Public page and direct-render tests pass; the adapter retains the detected image route, encoder app, and cached cell size. Positive image-route and explicit-prohibition controls pass. | None. |
| R1-F1: bt turns Mermaid rendering failures into successful source output | R1-F1.1–R1-F1.7: broken CLI source under default/restrictive policy, successful text, export failure, fallible backend, infallible component, nine shared commands | **verified fixed** | Fresh visualized and bt suites pass the parse-error, request-count, export-error and valid controls. Inspected all nine calls through the common display handler. The later capability-failure sibling is separately verified as R2-F1. | None. |
| R1-F2: failed preprocessing is mistaken for an ineligible diagram | R1-F2.1–R1-F2.13: classification/readers, public text API, component/hook strictness, valid/unsupported/invalid controls, unknown headers, title escaping, UTF-8 separator, image preprocessing and consumers | **verified fixed** | Fallible preprocessing and detection errors reach text readers and the real hook; strict/warn/permissive tests pass. Valid frontmatter, pie and ZenUML controls pass. Title scalar round trips, multibyte separator test, image goldens and gitGraph tests pass. | None. |
| R1-F3: text fallback can discard an explicitly supplied title | R1-F3.1–R1-F3.6: frontmatter/untitled text, source, image-error source, bt propagation, drawn-title and same-named-label siblings | **verified fixed** | Presentation-aware title checks remain in place. Fresh component and nine-command CLI tests verify visible title counts on text/source routes, sequence/XY titles, labels sharing the title, and image failure. | None. |
| R1-F4: line-number gutters make accepted text drawings overflow | R1-F4.1–R1-F4.9: backend fit, numbered/unnumbered and nested output, sufficient width, two-digit gutters, standalone/bt, generic fragment boundary | **verified fixed** | Fresh Darkmatter numbered-code tests cover 60/62/64 columns, nested lists, two-digit state gutters and note wrapping. Generic hook tests verify silent decline after decoration, exact-fit success and unchanged already-rendered image output. | None. |
| R1-F5: required styling tests depend on ambient terminal capabilities | R1-F5.1–R1-F5.9: component note, md page, bt color/no-color, syntax, plain/styled controls, hook Unicode, force-color leakage | **verified fixed** | Explicit capability fixtures and child-process environment cleanup remain. Full area suites pass. All 33 component fallback tests also pass under `NO_COLOR=1 TERM=dumb LC_ALL=C`, retaining their styling assertions. | None. |

### R2-F1 instance evidence

In biscuit-visualized, [try_present](../../src/src/components/mermaid.rs) preserves failures for callers such as bt. Its two source-only outcomes now share [validate_renderable](../../src/src/mermaid/render.rs), which asks Merman for the parsed model's required rendering capabilities. This does not render an image or change the public parse-only validator. The repair stays within review 2's bounded recommendation.

| Child ID | Site and reproduction checked | Verified result |
| --- | --- | --- |
| R2-F1.1 | Artifact image API; existing `A["$$x^2$$"] --> B` fixture | Still reports the missing `math` capability. The new plan test asserts equality with this image error. |
| R2-F1.2 | Public `validate()`; same formula | Still succeeds under its parse-only contract. |
| R2-F1.3 | Public component `try_present`; `Never` and `NoPartial` policies | Both return the missing-math error. Counter assertions show no font-chain initialization or artifact writes. |
| R2-F1.4 | bt; both restrictive features and `MERMAID_FALLBACK=code-block` | Fresh shipped-binary probes exit 1 with the capability error in all three cases. CLI and in-process regression tests pass. |
| R2-F1.5 | Width-refused component and bt `--width 8`; same formula | Capability error after exactly one text request in the component test; shipped CLI exits 1. |
| R2-F1.6 | Same formula with text allowed at 80 columns | Public component returns text containing the literal formula and partial note. Shipped CLI exits 0 and prints that note. |
| R2-F1.7 | Broken node-label controls, default/restrictive policies | Existing CLI parse-error tests pass and continue to exit unsuccessfully. |
| R2-F1.8 | Valid flowchart/pie and narrow valid source; all nine command consumers | Valid controls succeed. Flowchart, state, ER, pie, quadrant, timeline, bar, line and git graph share bt's [display_mermaid](../../../biscuit-terminal/cli/src/commands/mermaid.rs); there is no independent source-only bypass. |
| R2-F1.9 | Infallible `present`, render-tree hook, and post-image `NoImageSupport` handler | Infallible failures remain source; hook policy/width refusals remain declined. The post-image handler is reached only after image generation succeeds. Real-hook and component tests pass. |
| R2-F1.10 | Touched component/backend/CLI documentation, rustdoc and skill explanations | Math failures are qualified by presentation: a successful partial text drawing is allowed; fallible source-only selection checks unavailable capabilities. Public `validate()` remains explicitly parse-only. |

The capability-plan check can report another unavailable capability as well as math. This is consistent with preserving known rendering failures, uses Merman's own model, and needs no new family list or source scanner. The extra parse after a width refusal is not another text request. No attempt is made to predict every possible later image-stage failure.

### R2-F2 instance evidence

In Darkmatter, [capability_terminal](../../../darkmatter/lib/src/markdown/render_tree/entrypoints.rs) now separates a caller's pinned width and existing optimistic styling profile from the detected image route. Copying the route together with its app and cell size preserves both image permission and the data needed to encode it correctly.

| Child ID | Site and reproduction checked | Verified result |
| --- | --- | --- |
| R2-F2.1 | Public automatic-mode page, default geometry | Text drawing; no Kitty image sequence. |
| R2-F2.2 | Same page with line numbers | Text drawing; no Kitty sequence. Separate finished-width tests still select source if a gutter makes the drawing too wide. |
| R2-F2.3 | Same page with maximum width 60 | Text drawing; no Kitty sequence. |
| R2-F2.4 | Same page with margin 1 | Text drawing; no Kitty sequence. |
| R2-F2.5 | Same page with padding 1 | Text drawing; no Kitty sequence. |
| R2-F2.6 | Same page with subtle background | Text drawing with page decoration; no Kitty sequence. |
| R2-F2.7 | Public `Markdown::as_terminal`, width absent and width 60 | Both draw text without Kitty. Adapter controls retain a Sixel route and its encoder app, keep no-pixel routes disabled, and preserve detection without width or on a TTY. |
| R2-F2.8 | Numbered page in explicit source mode | Source code remains visible; no Kitty sequence. |
| R2-F2.9 | Real generic hook and Darkmatter numbered code renderer | Fresh full suites pass image, text, strictness and numbered-width cases with the proper capability snapshot. |
| R2-F2.10 | Explicit image prohibition and forced-image controls | `Never` still maps to graphics off. `Force` plus width cannot override `BISCUIT_GRAPHICS=none`; the existing forced-image test on an unsupported injectable snapshot still passes. |

The public page test exercises all five geometry settings through the shared adapter, with graphics explicitly disabled. The helper's positive route test prevents fixing the defect by unconditionally removing images. Keeping nonterminal output without images, including under `Force`, follows biscuit-terminal's existing documented override contract and requires no author decision.

## Unblocked Findings

None.

## Blocked Findings

None. `human_in_the_loop` is null. No new decision needs later author confirmation either; previously accepted choices were not reopened.

## Recurrence

No recurring defect remains. Review 2's R2-F1 was a sibling of review 1's failure-preservation finding; its complete enumerated source-only class is now repaired. No new sibling or repair-induced regression was found. Accordingly, this review's `recurrence` is false.

## Verification and acceptance coverage

The specification explicitly assigns these behaviors to **Level 1 with explicit terminal capabilities** and says real windows and focus changes are unnecessary. The feature adds no keyboard interaction. Level 2 or Level 3 evidence is therefore not missing for these acceptance criteria.

| Acceptance criteria and user-facing behavior | Verification level and evidence | Assessment |
| --- | --- | --- |
| AC-F1–AC-F3: family/result eligibility and pinned metadata | L1 public component/backend tests, metadata snapshot, dense overflow and real structured-result guard; eligibility implementation inspected | Pass; retains the previously accepted projection-test approach and seven metadata-derived families. |
| AC-F4: code-node languages, exact note, emphasis and suppression | L1 node assertions, exact plain output and explicit styled terminal assertions | Pass, including hostile-environment rerun. |
| AC-F5–AC-F6: explicit setting/environment precedence and no-partial policy | L1 injected environment tests, per-eligible-family controls and bt/md consumer tests | Pass. |
| AC-F7: repeated/comma-separated/case-insensitive feature names, usage errors and completion | L1 parser, CLI process and completion tests | Pass. |
| AC-F8: automatic image/text/code, normal decline, strictness, no hook, unchanged explicit modes | L1 real installed hook through generic renderer, including malformed frontmatter and unknown-header cases | Pass. |
| AC-F9–AC-F10: md automatic selection and no-rendering across terminal/HTML/Markdown/JSON | L1 public page and CLI tests, including highlighting and absence of Mermaid.js | Pass; repaired geometry path also covered through the public API. |
| AC-F11: automatic/compact/canonical layout and fit | L1 reported-layout/width assertions, generic finished-width tests and real numbered-code renderer tests | Pass, including nested positions and growing gutters. |
| AC-F12: flattened syntax, header/comment/init/node highlighting and license | L1 syntax-loading and terminal/HTML highlighting tests; vendored license, pinned source comment and both dependency pages inspected | Pass. |
| AC-F13: image compatibility, no text-route fonts/artifacts, one text request | L1 existing image/golden tests and deterministic counters, including new source-only capability checks | Pass; text success fabricates no image metadata. |
| AC-F14: both character sets, Unicode labels, spaces, multiline labels, controls, failures and limits | L1 exact drawings, public result assertions, nested-width and injected-failure tests | Pass. |
| AC-F15: mode handling throughout the workspace | Existing implementation audit plus current source search and compiled callers in all four area suites | Pass; no unhandled mode mapping found. A fresh workspace-wide check was not repeated. |
| AC-F16: current documentation with examples and no feature-name references | Component, rendering-policy, CLI, render-tree, dependency and skill pages inspected; targeted search over affected topic pages | Pass. |
| AC-F17: bt uses Darkmatter highlighting without reversing library dependencies | L1 explicit color-on/images-off bt tests; library and CLI dependency manifests inspected | Pass; Darkmatter/syntect remain outside visualized and terminal library dependencies. |

Fresh local runs through repository recipes:

| Area / check | Result |
| --- | --- |
| biscuit-visualized `just test` | **1,150 passed, 4 skipped** |
| biscuit-terminal `just test`, including bt | **3,921 passed, 55 skipped** |
| Darkmatter `just test`, including md and DMLS | **10,719 passed, 10 skipped** |
| renderable `just test` | **684 passed** |
| visualized `NO_COLOR=1 TERM=dumb LC_ALL=C just test component_fallback_tests` | **33 passed**; remaining tests filtered out |
| `just check-tier-coverage` for each of the four areas | **Zero stranded tests** in each area |

Checked consolidated module declarations and required feature settings. The newly added/renamed tests are compiled and selected by L1; their observed durations are below the five-second budget. The shipped CLI formula probes independently reproduced review 2's four failing cases and successful-text control. No terminal or browser window was opened.

The Input Robustness Matrix does not apply: this feature renders Mermaid text grammar and reads typed renderer metadata, rather than adding a file-format/configuration loader. The bounded smell search found no permissive field filtering in the new selection code. The environment reader's `.ok()` deliberately maps non-Unicode values to the same default as every unrecognized value, as already accepted.

Cross-OS execution evidence remains the CI/CD process's responsibility and does not block this review. No production code was changed during review, no formatting command was run, and no feature directory was moved.
