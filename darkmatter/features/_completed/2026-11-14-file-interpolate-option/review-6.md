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
created: "2026-11-16T04:06:53-08:00"
spec: 2026-11-14-file-interpolate-option/spec.md
implemented: false
description: A **feature** review of `2026-11-14-file-interpolate-option/spec.md`
feature: 2026-11-14-file-interpolate-option/review-6.md
previous: 2026-11-14-file-interpolate-option/review-5.md
---

# Review 6: File interpolation option

The feature is **production ready**. The remaining defect from review 5 is
verified fixed: repeated inclusions at one authored line now receive only
their own validation labels and issue counts. No new finding or author
decision is required.

## Previous Finding Verification

Read review 5 and the repair's complete Finding Ledger under
“Implementation of Review Findings #5” in [log.md](log.md), then checked the
implementation and public results. Also checked reviews 1–4 and the earlier
ledgers for outstanding work and author rulings. None contains a
`## Human rulings` section; no blocked finding remains.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R4-F1: Reference projections merge different inclusions of the same file | All original .1–.19, plus review 5's .20–.29. The outstanding repair was .13 and .20–.24: issue association, grouped rows, transclusion edges, node summaries and CLI presentation. | **verified fixed** | The writer retains each issue; the shared lookup matches reference ID, authored inclusion trail and target. All three readers use it. Ten public-model tests pass across map/filter, both value orders and both prose modes, asserting accurate reports, labels, counts and error flags. The CLI regression passes. An independent 32-case CLI sweep checks all 16 shapes listed below. Original prepared-copy, TOC, projection and gate regressions pass in the full L1 run. | None. Review 5's partial repair is now complete; no newly found sibling is carried. |
| R4-F2: Reference preparation resolves collection inputs without their loop values | .1–.20 from the earlier ledger: map/filter inputs, nested targets, both set forms, replacement, conditions, both validation APIs and CLI gates; block selection, authored locations, TOC, headings, no shell execution, live test placement and remote discovery. | **verified fixed** | The shared collection traversal still prepares selected items in their item scope. `collection_inputs`, the collection CLI gates, `collection_discovery` and the loopback remote-target regression all pass in the full L1 run. The issue-label repair changes no collection selection or evaluation. | None; retains the verified repair from review 5. |

Earlier findings remain closed: review 2 verified R1-F1 (remote discovery),
R1-F2 (editor option expressions), R1-F3 (supplied child values) and R1-F4
(test-toolkit speed); reviews 4–5 verified R2-F1 (inherited shell discovery),
R2-F2 (inherited reference state), R2-F3 (replacement inputs) and R3-F1
(generated TOC anchors). Their Darkmatter regression tests pass again here.
The unchanged test-toolkit repair retains its earlier verification; this
review did not rerun that separate package. No accepted decision is reopened.

### Verification of the remaining issue-label class

The Darkmatter library's
[`ReferenceIssue::describes`](../../lib/src/markdown/reference/validate.rs:191)
associates an issue with the reference it describes. It accepts the raw
target spelling or its path part, because missing-file diagnostics omit a
fragment. The private
[`IssueMap`](../../lib/src/markdown/reference/file_tree/model.rs:209)
uses that association for grouped rows, transclusion edges and node
summaries. Report serialization continues to render the issues themselves.
No public identifier or output format was added.

The independent CLI sweep copied review 5's fixture: one existing
`present.*` file and one absent `missing.*` file, selected through item-specific
replacement or supplied frontmatter. Each shape ran under both `::map` and
`::filter` with an `interpolate=false` ancestor, using the built `md graph
root.md --follow --validate --output plain` command from an isolated temporary
directory. All 32 cases kept the existing target unlabeled and labeled the
absent target `[missing]`.

| Required instances | Sites and shapes checked | Result |
|---|---|---|
| R4-F1.13, .20 | Issue writer and shared lookup; same ID/trail, different targets | Distinct labels; no coordinate-only deduplication remains |
| R4-F1.21 | Markdown link, link with fragment, Markdown image; HTML link/image, video, audio, source, iframe, stylesheet, script and CSS font import | All 12 shapes correct through the grouped reader |
| R4-F1.22 | Body `::file`, `::code`, frontmatter prologue and epilogue | All four edge shapes correct |
| R4-F1.23 | Public model, all ten permanent cases × both collections/orders/prose modes | Existing copy has zero issues and no error flag; missing copy has one issue and the report's severity |
| R4-F1.24 | CLI graph presentation | Permanent wiring regression and independent sweep pass |
| R4-F1.25–.26 | Prepared-node selection, flattened/typed references, followed JSON, both validation APIs and CLI gates | Existing `repeated_copies` and `collection_inputs` regressions pass |
| R4-F1.27 | TOC snapshot and fragment decisions | `toc_fragments` and CLI fragment regressions pass |
| R4-F1.28 | Separate authored lines and identical-value copies | Existing repeated-copy and identical-copy controls pass |
| R4-F1.29 | Source caches/freshness/cycles, remote/code/editor boundaries, inline count-only groups and file-links globs | No new label consumer: inline groups count records, globs skip existence validation, DMLS does not build inclusion graphs; existing controls pass |

The earlier .1–.19 inventory also remains satisfied: node IDs and insertions
preserve each prepared copy; body/intermediate/frontmatter/target/replacement
cases survive every projection; generated TOC records select their own
target; Mermaid/DOT retain distinct copies. The corresponding graph tests,
public projection tests and CLI gates pass. The graph, validation,
transclusion and link-validation guidance agrees with these behaviors.

## Unblocked Findings

None.

## Blocked Findings

None. No prerequisite changed and no work needs an author ruling.

## Recurrence

No finding in this review. The recurring association defect R4-F1 is closed,
including the same-line instance that remained partially fixed in review 5.

## Spec coverage and verification level

The spec explicitly requires L1 verification. Its requirements concern text
processing, composition effects and editor-provider results; none requires
terminal rendering, OS keyboard input or browser behavior.

| User-facing requirement | Checked evidence | Level |
|---|---|---|
| Defaults, null, aliases, quoting, invalid/missing/repeated assignments and located errors | Shared coercion/parser tests; public composition, CLI error and DMLS parity tests | L1 |
| Literal known/unknown names, invalid calls, malformed examples and triple braces; code-block precedence | `file_interpolate` literal/code tests and deny-all controls | L1 |
| Active frontmatter, supplied values, conditions, collection callbacks, nested directive targets/options and shell inputs | `set_overlay`, `inherited_values`, `collection_inputs`, and approval-versus-execution assertions | L1 |
| Branch inheritance, nested composition, frontmatter references, permitted remotes, independent cached copies and cycle handling | Local/remote tests, both inclusion orders and cache modes, inherited-child and cycle controls | L1 |
| Suppressed samples cause no read, fetch, context demand or approval | Read-set, call-check, probe-planning and loopback remote tests | L1 |
| Replacement, exclusion, wrappers, link handling and fresh-root recomposition | Transformation/recomposition tests, both reference APIs and projection regressions | L1 |
| Published examples survive a colliding caller property | Frozen publication fixture through ordinary `md compose --frontmatter --deny all`; passive shipped caller-skill source check | L1 |
| Parent completion/hover/diagnostics and independently opened child analysis | DMLS `file_interpolate_option` provider tests, including dynamic option values and the author's exact hover sentence | L1 |

Verification performed:

- `just test` from `darkmatter/`: **17,040 passed, 10 skipped**, nextest
  **80.245 seconds**, recipe about **1 minute 23 seconds**. The canonical
  slow-test record reports **none at or above 5.0 seconds**.
- `just check-tier-coverage darkmatter` from the repository root:
  **zero stranded tests**. Checked the explicit L1 module declarations for
  the CLI, DMLS and collection-discovery tests; the library regressions are
  compiled unit tests and appear in the run.
- Independent CLI sweep: **32 cases passed**, covering the 16 reference
  shapes above. Temporary fixture files were removed automatically.

The input robustness matrix does not apply: this feature changes Markdown
directive/expression grammar rather than a configuration or file-format
loader. No additional production code or permanent tests were written by
this review. No new lint or cross-OS execution claim is made; the repair's
recorded lint result is not represented as a fresh run.

## Implementation-record verification and metadata

No newly verified false behavioral statement in either implementation record
requires correction. The earlier dated corrections in [log.md](log.md)
remain historical records of the defects at those repair stages; the latest
ledger's completion claim is supported by this review's results.

Review 5 now links to this review; its `implemented` and `implemented_by`
claims are unchanged. The specification body is unchanged. Its review count
is 6 and `completed` is true. The feature directory remains active for the
author's review closure.
