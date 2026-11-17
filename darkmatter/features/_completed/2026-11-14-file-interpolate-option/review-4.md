---
$schema: feature-review.yaml
ready: false
findings:
    - "R4-F1 (high): Reference projections merge different inclusions of the same file"
    - "R4-F2 (high): Reference preparation resolves collection inputs without their loop values"
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-16T00:33:42-08:00"
spec: 2026-11-14-file-interpolate-option/spec.md
implemented: true
description: A **feature** review of `2026-11-14-file-interpolate-option/spec.md`
feature: 2026-11-14-file-interpolate-option/review-4.md
previous: 2026-11-14-file-interpolate-option/review-3.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-file-interpolate-option/log.md
implemented_by: claude/opus
human_review_items:
    - "R4-F2: passive reference checks now evaluate ::map/::filter sources, callbacks and gates (as composition does, ::shell off) instead of the review-suggested condition-blind preflight expansion, which would report links composition never produces"
next: 2026-11-14-file-interpolate-option/review-5.md
---

# Review 4: File interpolation option

The feature is **not production ready**. The previous review's enumerated
TOC cases are fixed. Two newly verified sibling defects remain: references
from different inclusions are confused after their individual preparation,
and references inside collections are prepared without the item values
composition uses. Both can hide broken links or reject valid documents.
Neither needs an author decision.

## Previous Finding Verification

Read review 3 and all three repair ledgers in [log.md](log.md), then checked
the implementation and public results. No earlier review contains human
rulings, and no finding is blocked. Review 1's four findings were closed in
review 2; their unchanged tooling-speed repair was not rerun here.

The required instances below include the repair ledger's child rows.
“Verified fixed” concerns those enumerated reproductions and controls; the
new siblings below must also be repaired before readiness. In particular,
the earlier repeated-child test inspects the graph's stored nodes. That
does not test which stored node validation subsequently follows.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R3-F1: TOC fragment validation rejects anchors generated from raw headings | R3-F1.1–.12 from review 3, plus ledger .13–.17: generators, target preparation, heading snapshot, body/prologue/epilogue paths, both validation APIs, CLI, ordinary-link controls, fragment opt-out, one-off replacement isolation, literal frontmatter links, file-tree/editor consumers and docs | **verified fixed** | All seven `toc_fragments` tests and the CLI regression pass. Generated records retain the raw target slugs; validation selects them using the recorded reference ID and target. Both APIs accept replaced, templated and conditionally removed raw headings. Authored links still use prepared headings; the literal frontmatter paths are not registered in the TOC snapshot. The CLI/file-tree validation engine shares this selection; DMLS synthesizes no TOC anchors. The four changed guidance pages describe that distinction. | None of the enumerated single-inclusion cases. The new repeated-inclusion lookup collision is R4-F1, including its snapshot instance .8. |
| R2-F1: Shell discovery skips grandchildren with different inherited values | R2-F1.1–.11: recursive file/frontmatter/nested-expression discovery, target and approval decisions, local/remote overlays, modes, deduplication and cycles | **verified fixed** | `visited_key` retains the request fingerprint and inherited discovery state; the source-only ancestor marker remains separate. The inherited-value, pre-approved execution and remote controls pass in the focused and full runs. | None; the remaining reference-graph lookup problem is a separate consumer, R4-F1. |
| R2-F2: Reference preparation loses inherited and resolved frontmatter state | R2-F2.1–.13: original inherited-state cases, resolved conditions/frontmatter targets, shared-child preparation, accumulated heading slugs, other consumers and the previously open TOC raw-heading case | **verified fixed** | Composition's child derivation is used before preparing each inclusion. The original `reference_inheritance` cases and CLI checks pass. The raw node list keeps both supplied values, and the prepared-heading snapshot keeps their accumulated slugs. The previously open .13 reproduction now passes through the TOC tests above. | The original cases are repaired. R4-F1 follows those prepared nodes through previously missed occurrence lookups; R4-F2 exercises the collection-local values that the earlier sweep dismissed. |
| R2-F3: Replacement options are omitted from child preparation and cache identity | R2-F3.1–.16: cached/uncached local composition, precedence, discovery, graph/CLI, remote file/url, code, repeated-site identity and TOC/frontmatter isolation | **verified fixed** | The actual child request carries its replacement option before preparation and identity capture. Existing `replace_options` tests pass for both orders and cache modes, including the TOC one-off-map control and remote descendants. | No original replacement-application case. The later graph projection and collection-local replacement failures are R4-F1/R4-F2. |

This is not a partial repair of the original single-inclusion TOC examples:
they now produce the required public results. It is an incomplete sweep of
the decisions that consume multiple prepared copies, plus a collection
case the earlier ledger incorrectly classified as clean.

## Unblocked Findings

### R4-F1 (high): Reference projections merge different inclusions of the same file

**Authority — spec:** “one interpretation across every path that examines a
child,” and “Two uses of one file must not share prepared content, warnings,
or composed results across different effective settings.” **Defect:** a
realistic reusable fragment included with two supplied values composes both
results, but the public reference report follows the first prepared copy
twice. It can declare a document with a missing link valid. This is a new
sibling of review 2's R2-F2 and review 3's R3-F1.

**Defect class:** Prepared inclusion results are stored separately but later
selected or classified using identities that identify only their source
file and source position.

In the Darkmatter library, the [graph builder](../../lib/src/markdown/reference/graph.rs:484)
gives each prepared copy of one file the same node ID. The
[node lookup](../../lib/src/markdown/reference/types.rs:749) returns its first
copy, so flattening and followed JSON select that copy at every insertion.
The [file-tree model](../../lib/src/markdown/reference/file_tree/model.rs:190)
instead overwrites its map entry and selects the last copy. Its validation
labels also use reference IDs shared by those copies. The new
[TOC snapshot](../../lib/src/markdown/reference/snapshot.rs:132) similarly
overwrites a prior generated reference's heading entry by reference ID;
checking the target afterward cannot recover the lost entry.

Create `present.md` and these two files, leaving `missing.md` absent:

```md
<!-- root.md -->
::file ./leaf.md interpolate=false set.word="a"

::file ./leaf.md interpolate=false set.word="b"
```

```md
<!-- leaf.md -->
::block when="word == 'a'"
[Good](./present.md)
::end-block
::block when="word == 'b'"
[Bad](./missing.md)
::end-block
```

`md compose root.md --output json` exits **0**, emits both links, and reports
no broken references. `md validate refs root.md --output json` also exits
**0** with no issues. Both should report the second inclusion's missing
target. Followed graph JSON shows `present.md` twice; plain graph output
shows `missing.md` twice while saying “all ok.” Reversing the inclusions
reports the missing link twice and labels the existing `present.md` as
missing in the plain graph.

There is also a direct continuation of the TOC problem: include a shared
holder twice with different `set.word` values, where its directive is
`::toc-linking ./{{ word }}.md`. Give both targets their own
`replace: {WORD: World}` and `## Hello WORD`. Composition correctly emits
both raw anchors. Fragment validation rejects the first target's
`#hello-word` twice: its raw-slug entry was overwritten by the second
target, and its prepared heading is `hello-world`. Separate TOC lines and
repeated copies using the same target remain clean.

The sweep followed prepared values through storage, child selection,
flattening, validation, heading selection, serialization and display:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R4-F1.1 | Graph node and child-insertion identities | Same file under `a`/`b`, both orders | Two prepared nodes, identical node IDs; insertions cannot distinguish them | Each insertion selects its own prepared copy |
| R4-F1.2 | Body-file inclusion, direct and through an intermediate parent | Conditional links above, normal and false prose | First link returned twice; missing second link omitted | One link from each inclusion, in order |
| R4-F1.3 | Prologue child | Intermediate parent uses `prologue: ./leaf.md`; same values/orders/modes | Same first-copy reuse | Preserve each child's inherited values through projection |
| R4-F1.4 | Epilogue child | Same with `epilogue:` | Same first-copy reuse | Same |
| R4-F1.5 | Resolved descendant target | Parent uses `::file ./{{ word }}.md`; a/b documents contain good/bad links | Both stored descendants exist, but flattening follows the first parent's target twice | Follow both resolved targets |
| R4-F1.6 | Replacement-dependent child | Repeated leaf `[Link](./WORD.md)`, maps `WORD` to present/missing under false | CLI composition emits both links; validation accepts both as present | Validate each replaced target |
| R4-F1.7 | Public `composed_references` and its typed link/image/CSS/script/font helpers | Body/frontmatter/target fixtures | Public flattened hyperlink result repeats the first copy; the typed helpers all delegate to this same flattening before filtering | Preserve the distinct records before projecting any reference kind |
| R4-F1.8 | Generated TOC reference snapshot and raw-slug selection | Shared holder, a/b targets with own replacement; body/prologue/epilogue, both orders and prose modes | Reference-ID entry overwritten; first target's raw anchor rejected twice in all 12 CLI cases | Keep the raw heading set for each generated reference and target |
| R4-F1.9 | TOC generators and stored prepared nodes | Same TOC fixture | Clean: composition emits both raw anchors and graph preparation builds both targets | Preserve these correct results |
| R4-F1.10 | Public one-step and prebuilt-graph validation | All 16 body/frontmatter/target API cases | Both APIs omit the second error, or duplicate the first error in reverse order | Judge the right reference at each inclusion trail |
| R4-F1.11 | Followed `ReferenceGraphView` / CLI graph JSON | Direct two-inclusion fixture | First prepared copy serialized twice | Serialize the copy belonging to each edge |
| R4-F1.12 | File-tree child map / plain graph | Same fixture | Last prepared copy displayed twice | Show each inclusion's own links |
| R4-F1.13 | File-tree issue map | Missing-first, present-second order | Existing `present.md` displayed as `[missing]` twice | Attach the issue to its actual reference occurrence |
| R4-F1.14 | CLI compose, validate and graph gates | Good-first fixture | All exit 0 despite a missing second link | Report the missing link and exit 1 |
| R4-F1.15 | Same-result and distinct-site controls | Same target twice with normal/false prose; separate TOC lines | Clean: references valid; TOC fragments accepted | Preserve these controls |
| R4-F1.16 | Prepared heading accumulation, raw-load caches, freshness and cycle bookkeeping | Existing repeated-heading and cycle cases; implementation invariants | Clean: prepared slugs accumulate; raw loads/dependencies describe unchanged source bytes; ancestor identity intentionally stays source-based | Keep source identity for those purposes, independently of prepared occurrences |
| R4-F1.17 | Remote/code/editor boundaries | Existing implementations | Clean by contract: remote Markdown is not recursively fetched by this graph; code has no recursively prepared Markdown body; DMLS analyzes an open file independently | No new fetching or editor inclusion mode |

**Recommended repair:** Distinguish prepared occurrences wherever an
insertion selects a graph child, and carry that distinction through the
reference/issue and TOC-heading lookups. Keep source identity separately for
raw caching, dependency freshness and cycle detection. An internal occurrence
handle can preserve existing authored reference IDs and avoid inventing a
new public format. Merely adding the target to the TOC map key will not fix
the wrong child chosen by flattening or the plain graph. Extend the existing
repeated-child tests to assert composed references, both validation results,
and followed projections; inspecting `graph.nodes()` alone misses the defect.

### R4-F2 (high): Reference preparation resolves collection inputs without their loop values

**Authority — spec:** “Conditions/callbacks execute; selected/repeated prose
stays literal,” and “Share the classification of inactive prose and active
inputs between execution, preflight walks … and passive reference-graph
preparation.” **Defect:** an ordinary `::map` or `::filter` inclusion uses its
item value during composition, but reference preparation resolves its target,
supplied values, replacement values and condition without that value. Valid
files can fail the normal CLI before composition runs. This is a newly
verified collection sibling of review 2's R2-F2.

**Defect class:** A preparation pass evaluates executable inputs outside the
lexical item scope that supplies their values.

The Darkmatter library's [reference preparation](../../lib/src/markdown/reference/graph.rs:1209)
excludes collection expansion, then runs ordinary interpolation over the
unexpanded body. It returns neither the item copies nor their scope frames
for the subsequent directive reads. Avoiding a second execution of callbacks
is correct; treating the unbound item as an empty value is not.

Use `root.md` containing `::file ./parent.md interpolate=false`, create
`one.md` and `two.md`, and use these files:

```md
<!-- parent.md -->
---
$schema:
  words: string[](required)
words: [one, two]
---
::map words "item => item"
::file ./leaf.md replace='{"WORD":"{{ item }}"}'
::end-map
```

```md
<!-- leaf.md -->
[Link](./WORD.md)
```

Public composition produces links to `one.md` and `two.md`. The ordinary
`md compose root.md --output json` instead exits **1**, reporting a missing
`./.md`; reference preparation replaced `item` with an empty string before
reading the child. `md validate refs` and `md graph --follow --validate`
report the same nonexistent target. Both actual targets exist.

The API sweep used declared string-array sources, identity map callbacks or
an always-true filter, and one changed directive input per case. It checked
normal and false prose for both collection kinds: 20 cases. Direct composition
produced the expected two item-specific results in every case. Both validation
entry points agreed on the wrong results below.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R4-F2.1 | Shared InlinePre preparation of map bodies | Identity map, declared `[a, b]`, both prose modes | Body read without item frames | Keep each input's item scope for reference discovery |
| R4-F2.2 | Shared InlinePre preparation of filter bodies | Always-true filter over the same source, both modes | Same missing item frames | Same |
| R4-F2.3 | Nested file target reader | `::file ./{{ word }}.md`, map/filter, both modes | Checks `./.md`; records neither child's hyperlink | Discover a.md and b.md and their references |
| R4-F2.4 | Property-form supplied-value reader | `set.word="{{ word }}"`, child's conditional links select good/bad target | No hyperlinks or missing-link issues | Discover the links selected by the supplied item values |
| R4-F2.5 | Object-form supplied-value reader | `set='{"word":"{{ word }}"}'`, same child | Same silent omission | Same |
| R4-F2.6 | Replacement-value reader | `replace='{"WORD":"{{ word }}"}'`, leaf links `./WORD-target.md` | Records/checks `./-target.md` | Record/check a-target.md and b-target.md |
| R4-F2.7 | Transclusion condition reader | `when="word == 'b'"` on a static b.md child | Child incorrectly excluded; no missing-link issue | Include the candidate whose item satisfies the condition |
| R4-F2.8 | Public composed-reference result | All five input shapes, both collections and modes | Empty or incorrect hyperlink list as above | Item-specific reference results |
| R4-F2.9 | Public one-step/prebuilt validation | Same 20 cases | Both validate the empty/wrong values, hiding real missing targets or inventing others | Validate actual candidate references with correct provenance |
| R4-F2.10 | CLI compose/validate/graph | Complete one/two replacement fixture with both targets present | All three commands exit 1 naming `./.md` | Accept the valid document |
| R4-F2.11 | Execution and shell-preflight control | Direct composition for all 20 cases; existing repeated replacement shell-discovery test | Clean: composition uses item values; existing preflight test discovers and executes both commands | Retain those behaviors; do not execute sample prose or commands during reference checks |
| R4-F2.12 | Ordinary conditional blocks and includes outside collections | Existing `reference_inheritance` tests, plus R4-F1's individually prepared copies | Clean preparation: values resolve before selecting links | Preserve this control while fixing collection inputs |

**Recommended repair:** Reuse the existing collection discovery path's
candidate-item frames and the dependency evaluation needed for directive
inputs, rather than interpolating an unexpanded collection body against the
document alone. The [shell preflight collector](../../lib/src/markdown/compose/preflight/collect.rs:571)
already carries candidate copies and site frames without running their shell
commands. Keep reference checking passive and preserve the spec's prohibition
on executing suppressed prose. Test the shared preparation once per input
reader and keep one CLI case with both real targets present. R4-F1 must also
be fixed so distinct discovered copies survive the later projections.

## Blocked Findings

None. Both repairs can follow the existing composition and passive-discovery
contracts. No new syntax, network permission or author decision is needed.

## Recurrence

R4-F1 continues review 2's **R2-F2, reference preparation losing per-child
state**, and review 3's **R3-F1, generated TOC anchors checked against the
wrong heading representation**. Source-only child selection is an unchanged
sibling the earlier sweep missed. The reference-ID-only TOC snapshot is a
new collision in the latest repair. That repair fixes its listed original
fixtures, but its sweep should also have followed two different preparations
of one holder through child lookup, flattening, JSON/plain projections,
issue assignment and the raw-heading snapshot.

R4-F2 is another unchanged sibling of **R2-F2**. Its repair ledger's .12
explicitly dismissed map/filter because the graph does not execute them.
It should instead have followed their item-dependent file targets, both set
forms, replacement values and conditions through passive preparation and
the actual CLI gate. All are now enumerated above for both collection kinds.

The boundary is concrete: each executable input must be judged in its
actual item/inclusion context, and each prepared result must stay attached
to that context through its consumers. The failures use ordinary reusable
fragments and the existing declared-array collection pattern. Their harm
does not depend on adversarial or synthetic escaping cases.

## Verification and requirement coverage

All specified behavior needs **L1** verification: in-process composition,
passive analysis and ordinary CLI results. No requirement depends on a
terminal encoder, browser rendering or OS keyboard events.

| Requirement | Verification present | Assessment |
|---|---|---|
| Boolean/default/null/alias/quoted/repeated option values and authored error locations | L1 parser, library/editor agreement and CLI diagnostics | Passes |
| Literal known/unknown/bad-call/malformed/triple-brace examples, including code-block precedence and deny-all | L1 public composition and diagnostics | Passes |
| Frontmatter interpolation, supplied values and active directive inputs | L1 composition, discovery, graph and CLI | Individual cases pass; reference projections and collection-local readers fail as R4-F1/R4-F2 |
| Conditional and collection execution with literal selected/repeated prose | L1 direct composition and shell preflight | Execution passes; normal CLI/reference agreement fails as R4-F2 |
| Descendant inheritance and independent inclusion branches, orders and cache modes | L1 output, identities, work counters, approval results and cycles | Composition passes; followed reference copies fail as R4-F1 |
| Prologue/epilogue, allowed remote Markdown and nested composition | L1 temporary-file/loopback HTTP and public execution/discovery | Existing cases pass; repeated local reference projections fail as R4-F1 |
| Inactive reads, context samples and command examples cause no effects | L1 read sets, request counts, capabilities/probes and approvals | Passes |
| Replacement, exclusion, links and wrappers retain existing transformations | L1 composition, public reference APIs and CLI | Original cases pass; the two findings affect links after otherwise-correct transformation |
| Editor completion, hover, option validation and independently opened child analysis | L1 DMLS providers and semantic-token tests | Passes |
| Frozen caller-skill publication, colliding name, deny-all guard and passive shipped-source check | L1 ordinary CLI invocation and corpus check | Passes |

Reviewer verification:

- `just test interpolate`: **255 passed**, none at or above five seconds.
- Full `just test` in `darkmatter/`: **16,986 passed, 10 skipped**, exit 0;
  none at or above five seconds. Nextest took 72.713 seconds, recipe 75 seconds.
  These existing passing tests do not exercise the failing projections above.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. The existing
  module/target declarations compile the library, CLI and DMLS feature tests.
- Temporary L1 public-API probes measured 16 repeated-inclusion cases and
  20 collection-input cases, checking direct output and both validation APIs;
  followed graph serialization was also checked in the inclusion probe.
  The probes deliberately failed after collecting the wrong results; each
  ran below 0.4 seconds. An initial probe compile used the wrong API name;
  it was corrected to the public `composed_references` before measurement.
- Direct runs of the built CLI confirmed both findings, the graph's different
  JSON/plain behavior, 12 repeated-TOC cases, and the clean TOC controls.
  All temporary source probes were removed. No production code was changed.

The input robustness matrix does not apply: the changed inputs are Markdown
directive/expression grammar, not a file-format/configuration loader. This
review makes no new lint or cross-OS execution claim.

## Implementation-record corrections

Corrected [log.md](log.md) in place with dated **2026-11-16** notes at
R2-F2.10/.12 and the review-3 repair summary. Preparing both raw nodes is
true; claiming downstream graph consumers are thereby fixed is false.
The public APIs and CLI above show first-copy selection, last-copy display,
and TOC snapshot overwrite. The claim that an unexecuted collection has no
item-dependent reference decision was also false: its executable directive
inputs are still read, without their values. The correction records the
actual results and verification methods. The review-3 repair's claim that
all seven TOC tests failed with its lookup disabled is narrowed to its own
recorded five failing cases and two clean controls.

These corrections are not extra findings or human-review items. The
specification body is unchanged; its review count and review 3's `next`
link are updated. Review 3's implementation claims are preserved.
