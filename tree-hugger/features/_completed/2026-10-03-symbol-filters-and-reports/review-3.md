---
$schema: feature-review.yaml
ready: false
findings:
  - title: Deprecation detection confuses code fence boundaries
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T13:57:47-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-3.md
previous: 2026-10-03-symbol-filters-and-reports/review-2.md
next: 2026-10-03-symbol-filters-and-reports/review-4.md
---

The feature is **not production ready**. Both findings from review-2 have been addressed, but the earlier marker-detection repair still mishandles valid documentation fences in four languages. This review records one grouped, unblocked finding. No human design decision is required.

## Previous review follow-through

Review-2 listed two Unblocked Findings and no Blocked Findings. There are no newly unblocked findings to reconcile.

| Previous finding | Repair and verification | Assessment |
|---|---|---|
| A sole same-named type overrides explicit owner qualification | The shared owner agreement check now gates both containment and same-file container-kind lookup. The absent-owner matrix includes Rust, Swift and Lua, alongside matching-owner controls. A public CLI reproduction with local `W` and `impl other::W` now puts `probe` beneath `[in other::W]`; local `W` has no member in class JSON or JSON Lines. Separate external-owner headings and repeated-owner regressions are registered. | Addressed; no remaining instance found in the reviewed ownership paths. |
| Reused query results retain obsolete file-wide test flags | The tree-hugger library's result remembers precisely which records the previous file-wide decision marked, undoes that decision before projection, and recomputes it. A result already filtered by a test selector declines reuse when that decision changes. The tests for all four manifest transitions, unchanged layouts, source-marked controls, all 16 language fixtures and chained narrowing passed in the package run. | Addressed; source-derived flags survive while filesystem-derived flags follow the current layout. |

The ownership sweep included enclosing declarations, qualified-owner matching, sole-candidate fallback, same-file type/table kind lookup, Rust impls, Swift extensions, Go receivers, Lua table members, outline context headings and class member projections. The reuse sweep included fresh source/parsed-file queries, persistent-cache hit/widening paths, coverage projection, filtered/unfiltered narrowing and prelude reuse. The existing C++ out-of-body extraction limit remains outside this feature's accepted declaration set.

Review-1's parameter-pattern, Python docstring and terminal-verification examples are also covered by their repaired tests. Its marker-detection class has the remaining cases below. Review-2's original claim that this class was closed did not include these fence shapes.

## Scope and verification

Read the specification, plan, implementation and repair logs, both earlier reviews, current topic documentation, extraction/filter/query/cache paths, declaration and class report writers, output-mode rules and test registration. Used the tree-hugger, rust-testing and biscuit-test-harness skills. Public probes used the built `target/debug/hug`, temporary directories, copied primary language fixtures, explicit file selection and `--no-cache`. Lua declarations were placed before the fixture's final return statement.

### Verification results and limits

| Check | Result |
|---|---|
| `just test` in `tree-hugger/` | Stopped after 512 of 957 tests: 511 passed, one failed in `symbols_header::headers_that_cannot_be_isolated_are_none`; 445 were not run. The new narrowing regressions passed. |
| Focused repeat of the failed library test | Could not compile: two `extract_type_alias_target` unit-test calls omitted a newly required language argument. |
| `just lint` in `tree-hugger/` | Could not compile those same type-alias unit tests. |
| Root `just check-tier-coverage tree-hugger` | Could not list library tests because compilation failed. Its reported listing failures are not evidence of stranded tests. |
| First root `just test tree-hugger-cli` attempt | Could not compile while definitions referenced the not-yet-present `completes_variant_list` and `align_variant_metadata` helpers. |
| Subsequent root `just test tree-hugger-cli` | All 261 tests executed: 260 passed, one failed in `order::equal_declaration_spans_fall_through_to_the_id_tie_breaker`. Its grouped Go type declarations now have distinct spans, contrary to that test's tie-case setup. Cache robustness, the ownership regressions, filter tests, report tests and format goldens passed. |
| `just test-l2` in `tree-hugger/` | Six selected entry points passed. Declaration/report checks executed in tmux and WezTerm; Kitty entry points took their unavailable-backend path. |
| Public ownership reproduction | Correct external-owner outline and empty local-class member lists in JSON and JSON Lines. |
| Public marker sweep | Four affected readers reproduced the same false positives and false negatives; sibling language controls are listed below. |

**The workspace changed during verification.** Additional edits appeared in classification, type metadata, definitions and shared symbol data while these commands were running. The header and Go-span assertions and intermediate compilation failures prevent claiming a clean full-area test/lint result. They are recorded here as verification limits, rather than assigned to the symbol-filter repairs without a stable source snapshot. No concurrent source edits were reverted or repaired by this review. The last successfully built CLI was used for the public probes; its shared fence detector agrees with the source cited below.

The library's consolidated `l1` target declares the flags, metrics, owner-identity and coverage modules. The CLI's `symbols` target declares its cache, filter, report and class modules. Terminal targets require `terminal-tests`, which the live `test-l2` recipe and CI feature metadata enable. No tests were added or renamed by this review. Registration was inspected directly; the unsuccessful tier-listing command cannot replace an executable coverage audit.

### Requirement-to-verification assessment

| Requirement | Appropriate level | Evidence reviewed |
|---|---|---|
| Scope, async, names, markers, metrics and selector composition | Level 1 | Per-language public-result matrices and CLI selector regressions; fence-boundary cases remain missing and incorrect. |
| Same-file ownership and unresolved-owner reporting | Level 1 for relations/text; Level 2 for styled layout | Owner-identity/absent-owner matrices, class/outline CLI regressions, public ownership reproduction and real-terminal outline captures. |
| Changing test-file classification and reusable results | Level 1 | Fresh/cache transition tables, all-language narrowing comparisons, both test selectors and chained-reuse controls. |
| JSON, JSON Lines, count, stats JSON, ordering, errors and closed pipes | Level 1 | Registered CLI report tests, format goldens and subprocess checks. |
| Styled lists, class sections, tables, outlines, stats, signatures and wrapping | Level 2 | The shared declaration checks exercise these modes in real tmux and WezTerm panes, including scoped rows, indentation, styles and applicable links. |
| Plain/redirected output and escaping source control sequences | Level 1 | CLI text/byte contracts and goldens. |
| Keyboard, mouse, paste and input-method behavior | Not applicable | These report commands add no interactive input behavior; Level 3 is unnecessary. |

No new mismatch between a terminal requirement and its verification level was found. Successful real-terminal checks do not establish source-classification correctness. Cross-OS proof is left to CI and does not determine readiness here.

### Input robustness matrix

The structured reader affected by this feature is the persistent **JSON cache**. The existing [cache matrix](../../cli/tests/symbols/cache_robustness.rs:141) and [reader contract](../../docs/cache-design.md) enumerate entry and record fields, including scope, qualified name, namespace rooting, async modifiers, test/deprecation flags, metrics, parameter count and documentation presence. Each column has the control, absent, explicit null, wrong whole-field type, one wrong element, every wrong element, empty, duplicate-key and trailing-invalid-content rows. Nullable unsupported metrics are explicitly distinguished from absent required metric fields; malformed collection elements are not silently discarded. The test compares shipped CLI output with a no-cache result and uses a stored sentinel to distinguish a served entry from recomputation. This matrix passed in the CLI run.

No new configuration or manifest-field parser is introduced by these repairs. Rust test-file classification observes manifest existence rather than reading its contents. The fence defect concerns source documentation boundaries, so the following matrix varies those boundaries through the public result rather than adding irrelevant JSON field shapes.

## Unblocked Findings

### Medium — Deprecation detection confuses code fence boundaries

**Defect class:** documentation marker detection toggles code-block state on any fence-looking line instead of matching the opening delimiter, its length and a valid closing line, causing examples to count as annotations and real annotations to disappear.

In the tree-hugger library, [has_deprecated_tag](../../lib/src/file/extract/flags.rs:205), used to recognize documentation-based deprecation in Java, JavaScript, TypeScript and PHP, keeps only a boolean. Any trimmed line starting with three backticks or three tildes toggles that boolean. A fence shorter than the opening fence, the other fence character, or a fence followed by non-whitespace content therefore closes the example prematurely. Encountering the actual closing fence toggles the state back into a code block, so a subsequent real tag is missed too. The current [marker-boundary regression](../../lib/tests/l1/symbols_flags.rs:657) checks matching three-backtick fences but none of these shapes.

This contradicts the [documented deprecation rule](../../docs/symbol-kinds-and-visibility.md:611): a tag inside a fenced example must not deprecate the declaration. It matters because `--deprecated` supplies incorrect selections, while JSON and JSON Lines publish the same incorrect flag to downstream consumers.

Copy the TypeScript primary fixture to a scratch directory, append the following declarations, and run the shipped CLI there:

~~~~~typescript
/**
 * ````text
 * ```
 * @deprecated example text
 * ````
 */
export function fence_decoy() {}

/**
 * ````text
 * ```
 * example text
 * ````
 * @deprecated use another function
 */
export function fence_real() {}
~~~~~

```sh
hug functions --file typescript.ts --name fence_decoy --deprecated --count --no-cache
hug functions --file typescript.ts --name fence_real --deprecated --count --no-cache
```

The first command prints `1` instead of `0`; the second prints `0` instead of `1`. The short three-backtick line is content inside the four-backtick fence, not its closing delimiter.

#### Complete boundary matrix for the affected readers

Each cell below was checked through `hug functions --file FILE --json --no-cache` against a copied primary fixture with the same documentation edit. Values are the public `visibility.is_deprecated` flag. Java probes used methods in a separate class; the other three used functions.

| Documentation shape | Java | JavaScript | TypeScript | PHP | Expected |
|---|---|---|---|---|---|
| Real tag without a fence | true | true | true | true | true — clean positive control |
| Ordinary documentation text | false | false | false | false | false — clean negative control |
| Tag inside matching three-backtick fences | false | false | false | false | false — clean |
| Tag inside matching three-tilde fences | false | false | false | false | false — clean |
| Four-backtick opening, three-backtick content line, tag still inside | true | true | true | true | false |
| Four-tilde opening, three-tilde content line, tag still inside | true | true | true | true | false |
| Backtick opening, tilde content line, tag still inside | true | true | true | true | false |
| Backtick opening, line beginning with backticks plus non-whitespace text, tag still inside | true | true | true | true | false |
| Three-backtick opening, valid longer closing fence, then a real tag | true | true | true | true | true — clean |
| Matching closing fence, then a real tag | true | true | true | true | true — clean |
| Shorter fence inside the example, actual closing fence, then a real tag | false | false | false | false | true |
| Other delimiter inside the example, actual closing fence, then a real tag | false | false | false | false | true |
| Fence-looking content with trailing text, actual closing fence, then a real tag | false | false | false | false | true |

#### Sibling-site sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Java doc-tag reader | All 13 boundary shapes above | Four false-positive shapes and three false-negative shapes | Follow the opening fence until its valid matching close. |
| JavaScript doc-tag reader | Same 13 shapes | Same failures | Same. |
| TypeScript doc-tag reader | Same 13 shapes | Same failures | Same. |
| PHP doc-tag reader | Same 13 shapes | Same failures | Same. |
| Shared documentation cleanup | Block comments containing those shapes in all four languages | Fence characters and lengths survive cleanup; detector still toggles blindly | Preserve meaningful boundaries and match them in the detector. |
| Rust, C++, C#, Swift, Scala and Python deprecation branches | Copied fixtures plus the same tag/fence example text, using each language's comment syntax | Added declarations remain non-deprecated | Clean: these branches require attributes, annotations or decorators rather than `@deprecated` documentation tags. |
| Go paragraph reader | Same examples in attached line-comment groups | Added declarations remain non-deprecated | Clean: Go uses `Deprecated:` paragraphs, not `@deprecated` tags. |
| C, Perl, Bash, Zsh and Lua | Same comment/example controls; Lua inserted before final return | Added declarations remain non-deprecated | Clean: no deprecation marker from this feature's table. |
| Attribute/decorator and test-marker readers | Existing syntax-boundary and flag matrices inspected; marker-boundary regression passed in the library run | Matching marker names and arguments remain syntax-derived | No fence decision at these sites; no additional instance found. |
| Query filter and CLI `--deprecated --count` | TypeScript `fence_decoy` and `fence_real` reproductions | Counts are respectively 1 and 0 | Respectively 0 and 1. The shared filter correctly consumes an incorrectly extracted flag. |
| JSON and JSON Lines projections; cache storage | Shared record flag and capability paths inspected; public JSON flag checked above | Serialize/store the extraction decision | Repair the source decision; no separate report-specific matcher is needed. |

Repair the shared detector with the opening fence character and run length. While inside the fence, close it only on the same character with at least the opening length and no trailing non-whitespace content. Keep recognizing real tags after the closing fence. Add one table-driven public-result regression covering the entire boundary matrix for **all four readers**, including both false positives and false negatives; also assert filtered CLI counts for both examples. Preserve the clean controls and update the detector's comments/current documentation if its contract changes. Because the deprecation flag is stored, bump the extractor revision and add the stale-revision cache case so warm entries cannot retain the wrong flag. This does not require a new dependency or a human ruling.

## Blocked Findings

None. The finding can be repaired and tested with temporary source fixtures and the existing public query/CLI paths.

## Recurrence

**Deprecation detection confuses code fence boundaries** repeats review-1's **Marker detection loses meaningful source boundaries**. That repair correctly excluded a tag inside a matching three-backtick example, but should also have swept Java, JavaScript, TypeScript and PHP with longer opening fences, shorter interior runs, mixed delimiters, invalid closing-looking lines and genuine tags after the actual close. Review-2 verified the earlier examples and called that class closed without those controls. This review checks every doc-tag consumer, all seven failing boundary shapes, the passing controls and the other language-specific marker branches. `recurrence: true` records that incomplete class repair.

The two findings unique to review-2 have no remaining instance identified by this review. The recurring issue is marker-boundary handling, not owner identity or result reuse.

## Closure

Recorded one unblocked medium-priority finding, advanced the spec's review count to three and linked review-3 from review-2. Review-2 remains marked implemented to record that its repair cycle ran. The spec is not marked completed. This review changes review documents/frontmatter only, makes no commit and leaves the feature in its current lifecycle directory.
