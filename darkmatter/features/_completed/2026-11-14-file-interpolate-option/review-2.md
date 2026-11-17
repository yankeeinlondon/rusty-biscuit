---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): Shell discovery skips grandchildren with different inherited values"
    - "R2-F2 (high): Reference preparation loses inherited and resolved frontmatter state"
    - "R2-F3 (high): Replacement options are omitted from child preparation and cache identity"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-15T23:06:58-08:00"
spec: 2026-11-14-file-interpolate-option/spec.md
implemented: true
description: A **feature** review of `2026-11-14-file-interpolate-option/spec.md`
feature: 2026-11-14-file-interpolate-option/review-2.md
next: 2026-11-14-file-interpolate-option/review-3.md
previous: 2026-11-14-file-interpolate-option/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-file-interpolate-option/log.md
implemented_by: claude/opus
---

# Review 2: File interpolation option

The feature is **not production ready**. Every instance listed in the previous
review and repair ledger is verified fixed. Three newly found sibling defects
remain in recursive discovery, reference preparation, and replacement handling.
All can be repaired within the existing decisions; none needs an author ruling.

## Previous Finding Verification

The required instances below include the repair ledger's added child rows.
There are 35 child instances across four parent findings. This assessment uses
the implementation and observed public results, rather than completion flags.
There is no earlier review or pending human ruling to carry forward.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Remote shell discovery ignores the inclusion's interpolate option | R1-F1.1–.6: both remote spellings, bad-call samples, nested commands, capabilities/probes; .7–.11: local discovery, inherited remote suppression, execution, prologue/epilogue, nested composition; added .12: both inclusion settings and orders | **verified fixed** | The remote collector folds the option before recursion. The `file_interpolate::remote` tests check literal results, empty approvals, absent capabilities/probes, enabled controls, and both orders. Local suppression and nested-composition tests pass. The inherited-child fold, remote inherited request, and default frontmatter-reference options preserve false for .8/.10/.11. Source-only ancestor markers preserve cycle termination. | None of these instances. The different inherited-value identity problem is the new sibling R2-F1. |
| R1-F2: The editor rejects valid expressions in interpolate values | R1-F2.1–.2: four expression spellings on file/url; .3–.5: valid/invalid literals and code's unknown option; .6: repeated dynamic options; added .7: non-boolean expression result; .8: completion, hover, parser and other passive consumers | **verified fixed** | The DMLS validator defers values found by the existing expression scanner. Provider tests exercise both keywords, flag values, literal locations, and code's unknown-option diagnostic. Public composition/preflight/graph tests check resolved flags and located rejection of `banana`. Completion prioritizes expressions; hover makes no claim about a dynamic result. All are compiled in existing targets and selected by L1. | None. |
| R1-F3: Child preparation and remote execution discard set parameters | R1-F3.1–.8: local execution/preflight/graph/CLI, remote file/url execution/preflight, property/object/control forms; .9: loaders without a directive set; added .10: multiple direct value sets; .11: changing-value self-cycle; .12: other callers of these loaders | **verified fixed** | All three formerly missing overlay calls now precede child preparation and retain data origins. The 42 generated `set_overlay` cases and its extra data-origin, deduplication, and cycle cases pass; the normal CLI reference-check regression passes. `md graph`, reference validation, and file-tree consume the same graph; shell reports consume the same preflight APIs. Frontmatter references and nested expressions have no direct set overlay to discard; remote references are not fetched by the reference graph. | None of the listed direct-child instances. R2-F1/R2-F2 expose previously unswept inherited state below those children; R2-F3 covers the sibling replacement parameter. These are new siblings, rather than partial repairs of the listed rows. |
| R1-F4: A recorded L1 test exceeds the hard ten-second limit | R1-F4.1: two-run kache idempotence; .2: all nine worktree-base cases; .3: archive path guard | **verified fixed** | Reviewed the reduced process work, retained two-run assertions, nine declared tests, and exact archive prefilter. Fresh package times: idempotence 3.442 s; nine base cases 0.588–0.766 s; archive guard 1.333 s. Targeted measurements: idempotence 2.073 s; nine base cases 0.601–0.714 s; archive guard 0.957 s. The 563-test package run has one other loaded-host slow result, cleared below. | None. |

The source and test evidence is in the Darkmatter library's
[inclusion tests](../../lib/src/markdown/compose/tests/file_interpolate.rs),
the language server's
[provider tests](../../dmls/tests/l1/file_interpolate_option.rs),
the CLI's [publication/reference tests](../../cli/tests/l1/compose_file_interpolate.rs),
and the [repair ledger](log.md#finding-ledger).

## Unblocked Findings

### R2-F1 (high): Shell discovery skips grandchildren with different inherited values

**Authority — spec:** “Discovery must approve exactly the command bytes
execution will use” and “one interpretation across every path that examines a
child.” **Defect:** ordinary repeated inclusions with different supplied values
approve only the first grandchild's command, and execution with those approvals
fails on the second command.

**Defect class:** A recursive discovery identity omits inherited values that
change the child's executable inputs.

In the Darkmatter library,
[collect_document](../../lib/src/markdown/compose/preflight/collect.rs:427)
deduplicates by source, prose setting, and the child's starting frontmatter.
The child's inherited defaults live separately in the request's external
state. Two children can therefore have equal starting frontmatter and different
effective values. The second visit returns before discovering its commands or
descendant targets.

Use these three files:

```md
<!-- root.md -->
::file ./parent.md interpolate=false set.word="one"

::file ./parent.md interpolate=false set.word="two"
```

```md
<!-- parent.md -->
::file ./leaf.md
```

```md
<!-- leaf.md -->
::shell echo {{ word }}
```

Public `compose_preflight` returns only `echo one`. Public composition with a
recording approval handler runs `echo one` and `echo two`. Composing with the
discovered pre-approved set fails with `NotPreApproved` for `echo two` in
`leaf.md`. This is a normal reusable-fragment pattern, and uses the same
property-form set syntax as the repaired direct-child test.

The sweep covers the recursive source kinds and the decisions consuming this
identity. Controls retain the same fixture with one inclusion or with the
override placed directly on the leaf.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Local file grandchild discovery | Above fixture; two inherited values, false ancestor | Approves only `echo one`; execution requests both commands | Discover both |
| R2-F1.2 | Local prologue discovery | Parent frontmatter `prologue: ./leaf.md`; same two values | Approves only `echo one`; execution requests both | Discover both |
| R2-F1.3 | Local epilogue discovery | Parent frontmatter `epilogue: ./leaf.md`; same two values | Same omission | Discover both |
| R2-F1.4 | Nested expression discovery | Parent body `{{ as_markdown("::shell echo {{ word }}") }}` under normal prose; same two supplied values | Approves only `echo one`; execution requests both | Discover both |
| R2-F1.5 | Grandchild target discovery | Leaf includes `./{{ word }}.md`; values `10.1.2.3` and `10.1.2.4`, both files present | Only the first target edge is recorded; second-only control records the second target | Keep both resolved descendant targets |
| R2-F1.6 | Pre-approved execution decision | Feed .1's discovered commands into `with_pre_approved_commands` | Fails `NotPreApproved` on `echo two` | Execution accepts all commands it discovered and approved |
| R2-F1.7 | Direct local set control | Set on the shell child itself: one/two/default | Clean: all three commands discovered and executed | Same |
| R2-F1.8 | Direct remote file/url set controls | Same one/two/default overlays | Clean: repaired `set_overlay` tests discover and execute all three | Same |
| R2-F1.9 | Repeated sources with different prose settings and changing-value cycles | Existing both-order and self-cycle tests | Clean: both settings walked; self-cycle ends | Preserve these behaviors |

Remote children retain their existing absence of parent-frontmatter inheritance;
no additional inheritance is requested there. Static capability classification
reads the same authored source on both visits, so its set union does not need
another copy merely because a scalar inherited value differs. The target and
approval decisions above do depend on that value and were checked separately.

**Recommended repair:** Include the inherited effective inputs in discovery's
reuse identity, alongside the starting child frontmatter and prose setting.
Reuse the existing request/state identity machinery where suitable. Keep the
separate source-only ancestor marker, so values changing along a cycle cannot
cause unbounded recursion. Add public-result regressions for the affected
loaders and pre-approved execution; no new syntax or protection layer is needed.

### R2-F2 (high): Reference preparation loses inherited and resolved frontmatter state

**Authority — spec:** “The implementation must preserve one interpretation
across every path that examines a child,” including passive reference-graph
preparation; child frontmatter retains normal resolution. **Defect:** a normal
`md compose` run can emit a missing hyperlink while reporting zero broken
references because its reference preparation silently removed the selected
grandchild branch.

**Defect class:** Reference preparation selects branches from different state
than composition, losing inherited values and reading unresolved frontmatter.

The Darkmatter library's
[accepted_child_options](../../lib/src/markdown/reference/graph.rs:420)
changes the source and prose setting but does not pass the current parent's
effective frontmatter into the child. Separately,
[build_node's condition state](../../lib/src/markdown/reference/graph.rs:529)
uses the original Markdown object's frontmatter after preparation has resolved
frontmatter in a clone; transclusion `when=` therefore reads the old strings.

Reproduction: root includes `parent.md` with
`interpolate=false set.word="supplied"`; parent includes `leaf.md`; leaf is:

```md
::block when="word == 'supplied'"
[Selected](./missing.md)
::end-block
```

The public graph contains no hyperlink, public reference validation returns no
issues, and normal `md compose --output json` exits 0 with
`[Selected](missing.md)` in its document and `broken: 0`. Including the leaf
directly with the same set override is the control: the graph records the
hyperlink and reference validation reports its missing target.

A separate frontmatter-resolution case uses a parent with `who: default`,
`greeting: "Hi {{ who }}"`, and
`::file ./leaf.md when="greeting == 'Hi supplied'"`. Root supplies
`set.who="supplied"`. Composition includes the leaf; the graph omits it because
the condition sees the unresolved greeting.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F2.1 | Graph body-file child derivation | Inherited supplied value; selected leaf link | No hyperlink; composition emits Selected | Record Selected's hyperlink |
| R2-F2.2 | Graph prologue child derivation | Parent uses `prologue: ./leaf.md` with the same fixture | Same omission | Same selected hyperlink |
| R2-F2.3 | Graph epilogue child derivation | Parent uses `epilogue: ./leaf.md` | Same omission | Same selected hyperlink |
| R2-F2.4 | Graph transclusion `when=` decision | Greeting frontmatter template resolved from supplied `who` | Leaf skipped by graph, included by composition | Both use the resolved greeting |
| R2-F2.5 | Reference-validation decision | .1–.3 with selected target absent | No issue | Missing-target error for Selected |
| R2-F2.6 | CLI reference gate | Normal `md compose --output json` for .1–.3 | Exit 0, selected broken link emitted, `broken: 0` | Reject missing selected hyperlink under normal policy |
| R2-F2.7 | Direct explicit-set control | Root includes leaf with `set.word="supplied"` | Clean: graph records link, validation rejects missing target | Same |
| R2-F2.8 | Normal composition and nested-expression child derivation | Inherited values and active nested composition | Clean: composition selects the inherited value; nested execution derives through `inherited_child_options` | Preserve current composition behavior |

The graph never fetches remote children, so it has no remote preparation
instance to repair. TOC-linking reads raw headings rather than executing a
transcluded body, so it has no execution counterpart for this inherited-branch
comparison. Its existing prose-setting propagation remains covered by the
earlier graph tests.
Replacement-dependent reference preparation is inventoried separately in R2-F3.

**Recommended repair:** Retain the resolved frontmatter/effective data from
preparation, use it for the graph's own condition decisions, and derive actual
inclusion children with the same inherited-data rules as composition. Preserve
disabled operations, supplied-value precedence, and value origins. Prove both
the graph's selected references and the normal CLI's missing-link rejection.

### R2-F3 (high): Replacement options are omitted from child preparation and cache identity

**Authority — defect:** Two ordinary inclusions of one file with different
`replace=` objects produce the same text with the default cache, while turning
the cache off produces the two correct results. Remote includes ignore an
explicit replacement, and local reference validation checks its unreplaced
target. The spec also preserves replacement behavior under false: “Text
replacement … retain[s] [its] existing behavior.”

**Defect class:** Recursive consumers and result identities omit an explicit
directive replacement map or its precedence mode.

In the Darkmatter library,
[render_markdown_transclusion](../../lib/src/markdown/compose/transclusion/engine.rs:1634)
builds the compose cache key from the parent options before applying this
directive's replacement inputs. Those inputs therefore collide despite the
request fingerprint already supporting them. The remote execution and
preflight branches never apply the directive's replacement option; the
reference graph also derives its child without that option. Preflight's new
starting-frontmatter digest does not distinguish two replacement settings.

For a child containing just `WORD`, use:

```md
::file ./leaf.md interpolate=false replace='{"WORD":"one"}'

::file ./leaf.md interpolate=false replace='{"WORD":"two"}'
```

With caching on the observed output is `one` twice; reversing directive order
produces `two` twice. With caching off both orders yield their respective one
and two results. A shell-child version exposes the discovery mismatch, and a
link-child version exposes the incorrect CLI reference gate.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F3.1 | Local Markdown compose cache | Two one-off objects, both directive orders, false inclusions | Both results use one inclusion's map | Each result uses its own map |
| R2-F3.2 | Local Markdown cache, precedence mode | Parent `replace: {WORD: parent}`, child `replace: {WORD: child}`; default then `replace=true` | `child` twice | `child`, then `parent` |
| R2-F3.3 | Local uncached execution control | Same inputs as .1/.2 with cache off | Clean: one/two in each order; child/parent for precedence | Same |
| R2-F3.4 | Local preflight deduplication | Shell child, two one-off maps; separately default/parent-wins | Only `echo one`; separately only `echo child` | Both one/two commands; separately both child/parent commands |
| R2-F3.5 | Local reference preparation, one-off map | Child `[Link](./WORD.md)`, map to one | Graph records `./WORD.md`; composition emits `one.md` | Graph records `./one.md` |
| R2-F3.6 | Local reference preparation, precedence | Same link shape with parent/child replacement maps and default/true | Graph records child target twice; uncached composition emits child/parent | Graph records child/parent targets |
| R2-F3.7 | CLI reference gate | .5 with only `one.md` present | Exit 1: missing `./WORD.md` | Exit 0 with link to existing `one.md` |
| R2-F3.8 | Remote `::file` execution | Allowed loopback child `::shell echo WORD`; explicit map WORD→one | Runs `echo WORD` | Run `echo one` |
| R2-F3.9 | Remote `::url` execution | Same fixture and map | Runs `echo WORD` | Run `echo one` |
| R2-F3.10 | Remote `::file` preflight | Same fixture and map | Approves `echo WORD` | Approve `echo one` |
| R2-F3.11 | Remote `::url` preflight | Same fixture and map | Approves `echo WORD` | Approve `echo one` |
| R2-F3.12 | Code-transclusion control | Same local file included twice through `::code`, one/two maps | Clean: distinct fenced one/two results | Same |
| R2-F3.13 | Loaders with no directive replacement option | Frontmatter references, nested-expression composition, TOC-linking | Clean by contract: no per-directive replacement option exists there; inherited replacements use their request | Preserve their existing rules |

The code-transclusion cache explicitly includes both replacement payload and
precedence in its variant parameters. The remote transport caches store raw
bodies, so they do not need per-inclusion replacement identities. Remote
reference preparation does not fetch children. Wrapping and section exclusion
are applied after the composed-child cache and do not account for this failure.

**Recommended repair:** Apply the directive's replacement settings before
taking identities and preparing local/remote children; include those settings
in discovery reuse as well. Computing the existing fingerprint from the actual
child request avoids a second partial list of cache inputs. Keep remote
parent-state behavior unchanged except for honoring the explicit directive
option. Cover one-off maps and parent-wins locally, remote one-off maps,
reference validation, and approvals. No new public option is required.

## Blocked Findings

None.

## Recurrence

R2-F1 and R2-F2 are newly found siblings of **review-1.md, R1-F3: Child
preparation and remote execution discard set parameters**. Its original cases
are fixed. The earlier sweep checked direct overlays but did not follow their
values through a shared grandchild's inherited defaults, graph child derivation,
or the graph's post-preparation condition state.

R2-F3 belongs to the same recursive-parameter parity class as R1-F3 and the
missing remote option fold in **review-1.md, R1-F1**. The earlier sweep covered
`set` and `interpolate`, but missed the sibling `replace` option in local cache
identity, preflight reuse, remote child options, and reference preparation.

These are previously unswept siblings, not a reopening of an accepted decision
or demonstrated regressions introduced by the repair. Their bounded repair is
to pass and identify the inputs the existing composition already uses. The
instance tables above include clean controls and the downstream acceptance and
reference-check decisions.

## Verification and coverage

All specification requirements concern text composition or passive editor
analysis. **L1 is the appropriate verification level**; no requirement needs a
terminal emulator, browser, focused window, or OS keyboard injection.

| User-facing requirement | Verification inspected/run | Assessment |
|---|---|---|
| Default/null/aliases/quotes, malformed and repeated assignments | L1 shared parser, public composition, DMLS provider agreement and CLI located errors | Appropriate level; passes |
| Literal known/unknown names, invalid/malformed samples, triple braces, deny-all | L1 public composition, preflight and frozen publication CLI fixture | Appropriate level; passes |
| Code-block opt-in cannot override false | L1 public composition, fenced/indented controls | Passes |
| Frontmatter and supplied values retain normal resolution | L1 composition, all direct overlay cases, reviewer graph/descendant probes | Direct cases pass; R2-F1/R2-F2 identify missing recursive coverage |
| Conditions/collections execute while prose stays literal | L1 public output and repeated-inclusion cases | Passes covered cases; resolved graph condition gap is R2-F2 |
| Nested targets/options and shell approval bytes | L1 public APIs and pre-approved execution | Direct cases pass; R2-F1/R2-F3 fail nested/replacement cases |
| Descendant inheritance; parent/sibling independence; both cache orders | L1 composition, identities, reuse counters and both-order tests | Prose setting passes; replacement identity gap is R2-F3 |
| Prologue/epilogue, remote Markdown, nested composition | L1 temporary files and allowed loopback HTTP | Suppression passes; inherited-value and replacement gaps listed above |
| Suppressed samples cause no reads/fetches/context/approvals | L1 read sets, zero-request mock, preflight capability/probe checks | Passes |
| Replacement, exclusion, links, wrappers, fresh-root recomposition | L1 public composition, graph and normal CLI | Exclusion/wrappers/recomposition pass; R2-F2/R2-F3 cover wrong references/replacement results |
| Editor completion/hover/validation and independently opened child | L1 public provider registry and semantic tokens | Passes, including dynamic option values |
| Frozen publication with colliding name and deny-all; shipped sources | L1 spawned normal CLI, passive corpus parser | Passes |

Reviewer verification:

- `just test file_interpolate`: **102 passed**, none at or above five seconds.
- `just test interpolate`, including a final run after removing reviewer
  probes: **198 passed**, none at or above five seconds.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage tools`:
  **zero stranded tests**. The library tests and declared CLI/DMLS `l1`
  modules compile and select the added tests.
- `just test` in `tools/`: **563 passed assertions, four skipped**. One
  test, `the_shipped_planner_emits_plans_the_guards_reader_accepts`, took
  **6.046 s** in the package and **4.197 s** alone. Host load was **33.40**
  on **16 cores** after the package run, so this clears the loaded-host
  allowance. No result reached ten seconds.
- The three repaired test-toolkit measurements, including all nine split
  worktree-base cases, are recorded in the previous-finding table.
- Temporary L1 probes over the public compose/preflight/reference APIs
  reproduced the finding tables. Diagnostic panics printed the observed
  results; these were reviewer probes, not passing regression tests. They
  were removed. Normal CLI runs independently reproduced missing-reference
  acceptance, replacement-reference rejection, and the clean code control.

The configuration robustness matrix does not apply: the changed inputs are
Markdown directives and expressions, not a configuration/document loader. No
cross-OS evidence is required for this review's readiness decision. The
implementation's full-area and lint results were read; this review makes no
new full-area lint claim.

## Implementation-record correction

Corrected the repair log's instance totals in place, with a dated note and
the counting method. It counted four parent findings among the child instances:
the ledger has **35 child rows, 20 marked fixed and 15 clean**, plus four
parent findings. This correction is not a finding or human-review item.
The specification body and prior review's implementation claims are preserved.
