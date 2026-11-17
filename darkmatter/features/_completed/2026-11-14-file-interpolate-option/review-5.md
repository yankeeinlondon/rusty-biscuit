---
$schema: feature-review.yaml
ready: false
findings:
    - "R4-F1 (high): Reference projections merge different inclusions of the same file"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-16T01:33:20-08:00"
spec: 2026-11-14-file-interpolate-option/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-file-interpolate-option/log.md
implemented_by: claude/opus
description: A **feature** review of `2026-11-14-file-interpolate-option/spec.md`
feature: 2026-11-14-file-interpolate-option/review-5.md
previous: 2026-11-14-file-interpolate-option/review-4.md
next: 2026-11-14-file-interpolate-option/review-6.md
---

# Review 5: File interpolation option

The feature is **not production ready**. Reference preparation now retains
each map/filter item's values, and the graph follows the correct prepared
copies. One part of R4-F1 remains: the file-tree display attaches a missing
target's validation label to a valid target when repeated inclusions share
the same authored line. The underlying validation report is correct. This
is a partial repair of the existing issue-label instance, not a new
independent finding or a reason to undo the collection repair.

## Previous Finding Verification

Read review 4 and its complete Finding Ledger in [log.md](log.md), including
the added child rows R4-F1.18–.19 and R4-F2.13–.20. Also compared reviews
1–3 and their ledgers for open findings and recurrence. No earlier review
contains human rulings, and no finding is blocked. Review 2 closed review
1's findings; review 4 closed the earlier inherited-state, replacement and
TOC cases. Those earlier Darkmatter controls still pass in this review's
full L1 run; the unchanged test-toolkit speed repair is not reopened.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R4-F1: Reference projections merge different inclusions of the same file | .1–.7: prepared IDs, body/intermediate/prologue/epilogue/target/replacement paths and public reference helpers; .8–.9: repeated TOC holders and generators; .10–.14: both validation APIs, JSON, file-tree child and issue maps, CLI gates; .15–.17: identical-copy, heading/cache/cycle and remote/code/editor controls; added .18–.19: Mermaid/DOT and documentation | **partially fixed** | `repeated_copies` exercises both orders and prose modes through composed references, typed links, both validation APIs, followed JSON and file-tree rows. `toc_fragments` checks both targets through body/prologue/epilogue holders. Graph tests verify distinct IDs and diagram edges; identical copies reuse their ID. CLI gate regressions pass. The node map and TOC snapshot now select correctly. However, .13's issue map still conflates two copies with the same authored inclusion trail: the CLI reproduction and an in-process public-model probe below show a valid reference labeled missing. The four changed guidance pages agree with the repaired graph identity behavior. | **R4-F1.13**, carried below, plus the complete inventory of its label consumers. The original separate-line reproduction is fixed; the collection repair exposes the remaining same-line case in that same lookup. |
| R4-F2: Reference preparation resolves collection inputs without their loop values | .1–.7: map/filter, nested targets, both set forms, replacement and conditions; .8–.10: composed references, both validation APIs and CLI gates; .11–.12: execution/preflight and ordinary-condition controls; added .13–.20: block gates, rejected items, authored locations, TOC targets, generated headings, no shell execution, test naming and remote-target discovery | **verified fixed** | Read `prepare_content`, the shared collection executor and the item-scoped `when=` lookup. The 20 `collection_inputs` reader cases check both prose modes, direct composition and both validation APIs under ordinary and reference-check requests. Additional cases verify selected/rejected items, authored line 7, item-specific TOC targets, repeated headings and absence of shell marker files. Both CLI collection gates and the loopback remote-target test pass. `collection_discovery` is declared in the L1 target, with its renamed test selected by the full run. | None of these instances. Correct item preparation does not fix the separate display-label lookup retained under R4-F1. |

The earlier TOC finding R3-F1 and review 2's R2-F1/R2-F2/R2-F3 remain
verified fixed for their enumerated cases. There is no still-blocked or
newly unblocked work. Running the existing collection traversal during
reference preparation is within the spec's active-input rules; retaining
its real selection behavior is preferable to reporting links from rejected
items. The repair's earlier request for human review of that choice does
not require another decision.

## Unblocked Findings

### R4-F1 (high): Reference projections merge different inclusions of the same file

**Authority — defect:** In the Darkmatter library's
[file-tree issue lookup](../../lib/src/markdown/reference/file_tree/model.rs:213),
an existing file is labeled `[missing]` when a map or filter includes the
same fragment with two replacement values. The public validation report
correctly names only the missing file. This is the remaining issue-label
instance R4-F1.13 from review 4, whose required result was “Attach the issue
to its actual reference occurrence.” It also violates the spec's requirement
to preserve “one interpretation across every path that examines a child.”

**Defect class:** A validation label is selected using source coordinates
that several distinct prepared inclusions can share.

The repair keys labels by reference ID and `included_via`, the list of
authored files and line numbers that led to a reference. Map/filter copies
come from one authored directive line, so their trails are identical. A
link at the same position in the shared child also retains the same
reference ID. Neither the issue-map writer nor its readers distinguish
the two targets. The graph's new prepared-copy IDs do distinguish them,
but those IDs do not participate in this lookup.

Create `present.md`, leave `missing.md` absent, and use these files:

```md
<!-- root.md -->
::file ./parent.md interpolate=false
```

```md
<!-- parent.md -->
---
$schema:
  words: string[](required)
words: [present, missing]
---
::map words "word => word"
::file ./leaf.md replace='{"WORD":"{{ word }}"}'
::end-map
```

```md
<!-- leaf.md -->
[Link](./WORD.md)
```

`md validate refs root.md --output json` correctly returns one issue for
`./missing.md`. `md graph root.md --follow --validate --output plain`
incorrectly shows both `./present.md [missing]` and
`./missing.md [missing]`. The issue's inclusion trail is root line 1 →
parent line 7 for both copies. Reversing the source array, omitting
`interpolate=false`, or using `::filter words "word => true"` gives the same
incorrect label on the valid target.

The missing hyperlink is a warning because its replacement value came from
interpolation. Exit 0 in that fixture preserves existing policy and is
not the defect. Replacing the leaf's link with `::file ./WORD.md` or
`::code ./WORD.rs` makes the missing target an error; the existing target
still receives the false label. The command correctly exits 1 for the
actual error.

The sweep followed the shared key through its writer, all three readers,
the resulting model and command presentation. The following child rows
extend the existing ledger; earlier repaired rows keep their IDs and are
accounted for in the verification table above.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R4-F1.13 | Existing issue-map instance: label selection by reference ID and trail | Shared leaf repeated by map/filter, both orders and prose modes | The valid and missing references receive the same missing label | Attach the issue only to the reference it describes |
| R4-F1.20 | `build_issue_map` insertion/deduplication | One missing target and one existing target with the same reference ID/trail | Stores a label whose key also matches the existing target | Preserve enough identity to distinguish the targets when storing and reading a label |
| R4-F1.21 | Grouped-reference reader in `build_node_model` | Markdown/HTML hyperlinks, Markdown/HTML images, video, audio, source, iframe, stylesheet, script and font imports; one target changed by replacement | Each existing target is falsely labeled missing; validation JSON has only the missing target's issue | Keep the existing row valid in every group; all these kinds share this reader |
| R4-F1.22 | Transclusion-edge reader in `build_node_model` | Leaf contains `::file ./WORD.md`, `::code ./WORD.rs`, or frontmatter `prologue:`/`epilogue: ./{{ word }}.md` supplied with `set.word="{{ word }}"` | Existing and missing edges both show `[missing]`; validation JSON correctly names only the missing edge | Label only the failing edge, for body and frontmatter references |
| R4-F1.23 | Node validation summary in `build_node_model` | Public file-tree model for both collections, orders and prose modes | Each leaf has `issues_count == 1` despite only one reported issue; error-severity labels also feed `has_errors` through this same lookup | The valid leaf has zero issues and no error flag |
| R4-F1.24 | CLI graph presentation | The leaf variants above, `--follow --validate --output plain` | Publishes the false labels from the model; terminal and document-formatted output use that same model | Publish only the actual target's issue; no renderer-specific repair |
| R4-F1.25 | Prepared-node selection, flattened/typed references and followed JSON | Repeated values and replacement targets in the existing public-API regressions and the collection fixture | Clean: both actual targets survive; graph preparation and selection no longer merge them | Preserve this repaired behavior |
| R4-F1.26 | One-step/prebuilt validation and CLI validation JSON | Collection replacement cases and file/code/frontmatter variants | Clean: validation names only the missing target; the full suite's two-API cases agree | Preserve the accurate report and its existing severity rules |
| R4-F1.27 | Generated TOC snapshot and fragment selection | Repeated holder targets under body/prologue/epilogue, both orders/modes; collection-local TOC targets | Clean: existing TOC tests and CLI regressions accept each generated raw anchor | Preserve the repaired reference-ID-plus-target lookup |
| R4-F1.28 | Different-line and identical-value controls | Existing `repeated_copies` projections and `the_same_values_twice_stay_one_copy` | Clean: separate authored lines distinguish trails; identical values have identical outcomes | Retain these controls alongside the same-line regression |
| R4-F1.29 | Other boundaries and display groups without targets | Source-based load/freshness/cycle bookkeeping; inline CSS/script/meta counts; file-links glob records; remote/code/editor preparation boundaries | Clean by their contracts: these do not select local missing-target labels; inline groups only count records, globs skip existence validation, and DMLS does not build inclusion graphs | No new fetching, editor mode, cache identity or inline-content machinery |

**Recommended repair:** Extend the private label association to distinguish
the actual target described by an issue, using the report's existing
path/fragment conventions, and update the writer and all three readers
together. Keep authored locations for diagnostics. A new public identifier,
wire-format change or general escaping layer is unnecessary for this harm.
Add a public-model regression asserting that the valid copy has no label
and zero issues; testing only the accurate validation report cannot catch
this remaining defect.

## Blocked Findings

None. The remaining repair fits the current contract and needs no author
decision. No human-review item is requested.

## Recurrence

This is **R4-F1.13 from review-4.md, partially repaired**, in the same
issue-map writer and readers. Adding an authored inclusion trail repaired
the original two-line fixture, but did not establish a unique identity for
copies produced at one line. The collection preparation repair now makes
those distinct copies available to the display. The prior sweep should
have tested their labels together with the new collection-input tests;
those tests currently assert references and validation reports, not the
file-tree labels or node counts.

The older class history is review 2's R2-F2 (prepared child state lost by
reference consumers) and review 3's R3-F1 (TOC validation using the wrong
target representation). Those listed repairs remain effective. This
review carries one bounded remaining association defect and inventories
its writer and three readers; it requests no expansion of literal-prose
semantics or new protection machinery.

## Verification and coverage

All feature requirements concern text processing, composition effects or
editor-provider results. **L1 is the appropriate level**; the spec requires
no terminal-emulator rendering, keyboard input, focused window or browser
behavior.

| Requirement | Strongest checked evidence | Level |
|---|---|---|
| Defaults, aliases, quoted/null/invalid/repeated values and located errors | Shared parser/value tests, public composition cases, CLI errors and DMLS literal-value parity | L1 |
| Literal known/unknown/malformed expressions, triple braces and code-block precedence | `file_interpolate` literal/code tests and deny-all controls | L1 |
| Active frontmatter, supplied values, conditions, collection callbacks, nested targets and shell bytes | Frontmatter/collection tests, `set_overlay`, `inherited_values`, `collection_inputs` and approval-versus-execution assertions | L1 |
| Branch inheritance, nested composition, frontmatter includes, permitted remotes and independent cache results | Local/remote inclusion tests, both orders/cache modes, cycle controls and nested-composition tests | L1 |
| Suppressed samples cause no reads, fetches, context demand or approvals | Read-set, call-check, probe-planning and loopback remote tests | L1 |
| Replacement, exclusions, wrappers, link handling and fresh composition | Transformation/recomposition tests and both reference-validation APIs; remaining display defect inventoried above | L1 |
| Publication retains examples even when a name matches the caller skill | Frozen publication fixture through ordinary `md compose --frontmatter --deny all`, plus shipped-source check | L1 |
| Parent option completion/hover/diagnostics and independent open-child analysis | `file_interpolate_option` provider tests, including expression-valued options and the author-ruled hover sentence | L1 |

Verification performed from `darkmatter/` unless noted:

- `just test file_interpolate`: **200 passed**, none at or above five seconds.
- `just test reference`: **1,234 passed**, none at or above five seconds.
- `just test`: **17,029 passed, 10 skipped**, nextest 74.132 seconds; none at
  or above five seconds. This run used the original source, before adding
  the temporary review probe.
- `just check-tier-coverage darkmatter` from the repository root: **zero
  stranded tests**. The existing library, CLI and DMLS targets compile and
  select the relevant tests.
- Direct runs of the built CLI reproduced the remaining labels and checked
  validation JSON for every listed local-reference/edge shape. A temporary
  in-process probe through the public graph, validation and file-tree-model
  APIs confirmed the wrong row labels and `[1, 1]` node counts in all eight
  collection/order/prose combinations; it took **0.082 seconds**. Initial
  imports in that review-only probe were corrected before measurement.
  The probe was removed and the source restored exactly.

The input robustness matrix does not apply: these changes read Markdown
directive/expression grammar, not a configuration/file-format loader. No
new production code or permanent tests were added by this review. No new
lint or cross-OS execution claim is made.

## Implementation-record corrections

Corrected the review-4 repair's parent row, R4-F1.13 row and completion
summary in [log.md](log.md) with dated **2026-11-16** notes. The previous
claim that every issue-map instance was fixed was false: distinct prepared
nodes and correct validation reports still become incorrect display labels
when authored trails coincide. The corrections record the CLI results and
public-model measurement above. They are record maintenance, not extra
findings or human-review items.

The specification body is unchanged. Its review count is now 5, and review
4 links to this review; its implementer claims remain unchanged.
