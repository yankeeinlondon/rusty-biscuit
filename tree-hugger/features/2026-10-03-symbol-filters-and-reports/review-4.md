---
$schema: feature-review.yaml
ready: false
findings:
  - title: Go block-comment cleanup discards meaningful documentation text
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T14:17:36-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-4.md
previous: 2026-10-03-symbol-filters-and-reports/review-3.md
next: 2026-10-03-symbol-filters-and-reports/review-5.md
---

The feature is **not production ready**. Review-3's fence-boundary finding is repaired, but the shared documentation cleanup still changes meaningful Go comment text. One grouped, unblocked medium-priority finding remains. No human design decision is required.

## Previous review follow-through

Review-3 had one Unblocked Finding and no Blocked Findings. There are no newly unblocked findings to reconcile.

| Previous finding | Verification | Assessment |
|---|---|---|
| Deprecation detection confuses code fence boundaries | The shared detector retains the opening character and run length, validates closing lines, rejects backticks in a backtick info string, and keeps unclosed fences open. The registered public-result matrix covers all four readers. Independently copied the Java, JavaScript, TypeScript and PHP primary fixtures and added all 15 matrix shapes; JSON, JSON Lines and filtered counts agreed with every expected result. Extractor revision is now 12, with a passing revision-11 rejection test. | Addressed for every reported fence shape and reader. The Go cleanup issue below is a remaining instance of the wider documentation-boundary class. |

Earlier repairs remain covered by passing tests: patterned and curried parameters, receiver exclusions, Python literal/docstring forms, repeated and absent owners, qualified Rust impls and Swift extensions, stacked Scala packages, changing Rust package boundaries, filtered/unfiltered result reuse, and real-terminal report views. No additional defect was identified in those reviewed paths.

## Scope and verification

Used the tree-hugger and rust-testing skills, plus the biscuit-test-harness skill for terminal verification. Reviewed the specification and all three earlier reviews, implementation/repair logs, extraction, predicates, result reuse, cache projection/validation, class grouping, report modes, current documentation and test registration. Public probes used the freshly built `target/debug/hug`, isolated temporary directories, explicit file selectors and `--no-cache`. Language sweeps copied the primary fixtures; Lua additions were inserted before the final return.

| Check | Result |
|---|---|
| `cargo build -p tree-hugger-cli --color=never` | Passed; public probes used this build. |
| `just test` in `tree-hugger/` | 976 Level-1 tests passed; 0 skipped. |
| `just lint` in `tree-hugger/` | Passed for library and CLI. |
| `just test-l2` in `tree-hugger/` | Six selected entry points passed; declaration/report captures executed in tmux and WezTerm. Kitty entry points took their unavailable-backend path. The 263 excluded tests were outside this tier. |
| Root `just check-tier-coverage tree-hugger` | No stranded tests. |
| Independent fence sweep | All 15 shapes passed for Java, JavaScript, TypeScript and PHP in JSON, JSON Lines and filtered count. |
| Independent Go cleanup sweep | Five false deprecation cases among 16 shapes; an additional two-function documentation-presence control reported 50% coverage instead of 100%. |

The library's consolidated `l1` binary declares the relevant matrix modules. Cargo discovers the CLI `symbols` target, whose module declarations include filters, reports, cache robustness and cache upgrades. Both terminal targets require `terminal-tests`; the area's live `test-l2` recipe and CI test feature metadata enable it. No implementation tests were added or renamed by this review.

### Requirement-to-verification assessment

| User-facing requirement | Appropriate level | Evidence and remaining gap |
|---|---|---|
| Scope, async, qualified names, containment, parameter counts and type selectors | Level 1 | Per-language extraction/selection matrices, coverage guards, query/cache equivalence tests and CLI regressions passed. |
| Test/deprecation flags and documentation coverage | Level 1 | Language matrices, source-boundary regressions and public selector tests passed; the Go shapes below are missing from those tests and fail independent public probes. |
| Changing package boundaries and reusable query results | Level 1 | Manifest-transition, source-marked, filtered/unfiltered narrowing and chained-reuse cases passed. |
| Filter composition, class/container selection, smart case and exact names | Level 1 | Registered library predicate matrices and CLI selection/intersection/conflict cases passed. |
| JSON/JSON Lines envelopes, ordering, partial failures, count and stats | Level 1 | Output goldens, round-trip/envelope tests, error and closed-pipe subprocess tests passed. |
| Styled lists, tables, outlines, class sections, stats, signatures and wrapping | Level 2 | Existing declaration checks executed in actual tmux and WezTerm panes, checking scoped rows, indentation, styling, applicable links and widths. |
| Plain output and escaping source control sequences | Level 1 | Byte/text contracts and goldens passed. |
| Keyboard, mouse, paste or input-method behavior | Not applicable | These commands add no interactive input behavior; Level 3 is unnecessary. |

No terminal verification-level mismatch was found. Cross-OS evidence is left to CI and does not determine this review's readiness decision.

### Input robustness matrix

The relevant structured reader is the persistent JSON cache. Its [documented entry/record matrices](../../docs/cache-design.md) and registered [public-output test](../../cli/tests/symbols/cache_robustness.rs) cover load-bearing fields, including scope, qualified name, namespace rooting, modifiers, test/deprecation flags, metric presence, parameter count and documentation presence. Columns include the control, absent, explicit null, wrong whole-field type, one bad element, every element bad, empty, duplicate-key and trailing-invalid rows. Unsupported nullable metrics remain distinct from omitted required metric fields. The test uses a stored sentinel to distinguish a served entry from recomputation and passed in this review.

No new configuration format or manifest-content parser is introduced here; the Rust package rule checks manifest existence. The remaining defect changes source comment text before classification, so its public-result matrix is below rather than a JSON field matrix.

## Unblocked Findings

### Medium — Go block-comment cleanup discards meaningful documentation text

**Defect class:** shared documentation normalization applies another language's decoration rules to Go comments, deleting literal content and indentation before marker and documentation-presence decisions.

In the tree-hugger library, [clean_doc_comment](../../lib/src/file/extract/doc_comments.rs:223) treats a Go `/**` opening as a three-character documentation delimiter. Go's block delimiter is `/*`, so that extra `*` belongs to its text. [clean_block_comment](../../lib/src/file/extract/doc_comments.rs:249) also removes one leading `*` from every body line and trims what follows before checking the language. Consequently, the Go paragraph detector receives `Deprecated:` from source that actually says `*Deprecated:` or `*    Deprecated:`. A comment consisting only of `*` becomes empty, changing `--undocumented` selection and stats as well.

Go's [CommentGroup.Text implementation](https://go.dev/src/go/ast/ast.go) removes `/*` and `*/` and preserves interior asterisks. Its [documentation parser](https://go.dev/src/go/doc/comment/parse.go) removes common spaces/tabs and retains remaining content. Neither performs Javadoc-style star removal. This also contradicts the area's [current documentation](../../docs/symbol-kinds-and-visibility.md), which promises Go paragraph/code-block interpretation following `go/doc/comment`.

Append this to a scratch copy of the primary Go fixture:

```go
/* *Deprecated: this is literal starred text. */
func ReviewLiteral() {}

/* * */
func ReviewBlockStar() {}

// *
func ReviewLineStar() {}
```

```sh
hug functions --file go.go --name ReviewLiteral --deprecated --count --no-cache
hug functions --file go.go --name '*Star' --undocumented --count --no-cache
hug functions --file go.go --name '*Star' --stats --json --no-cache
```

The first count is `1`, expected `0`. The second count is `1`, expected `0`: both star-only comments contain documentation text. Stats report `documented: 1`, `total: 2`, `ratio: 0.5`, expected `2`, `2`, `1.0`. This can incorrectly exclude declarations from ordinary workflows that avoid deprecated APIs and gives scripts incorrect documentation coverage.

#### Complete Go boundary matrix

Every row used the copied primary fixture with one appended function. JSON and JSON Lines flags agreed; a separate exact-name `--deprecated --count` invocation confirmed each flag's selection consequence. The expected value is whether the declaration is deprecated.

| Shape | Observed | Expected |
|---|---|---|
| Plain `/* Deprecated: … */` | true | true — clean |
| Introductory block paragraph, blank line, real marker | true | true — clean |
| Plain `// Deprecated: …` | true | true — clean |
| Introductory line-comment paragraph, blank comment line, real marker | true | true — clean |
| Block intro, blank line, indented marker example | false | false — clean |
| Equivalent indented line-comment example | false | false — clean |
| Block body line `*Deprecated: …` | true | false |
| Single-line `/* *Deprecated: … */` | true | false |
| Block body line ` * Deprecated: …` | true | false |
| Block intro, blank line, ` *    Deprecated: …` | true | false |
| Single-line `/**Deprecated: … */` | true | false — the third opening character is content |
| `// *Deprecated: …` | false | false — clean |
| `// * Deprecated: …` | false | false — clean |
| `/* **Deprecated: … */` | false | false — clean for the flag |
| Unstarred marker in the middle of a paragraph | false | false — clean |
| Literal starred paragraph, blank line, real unstarred marker | true | true — clean |

The separate presence row compared `/* * */` with `// *`: block/line `has_docs` values were false/true, expected true/true. Neither declaration is deprecated. Both declarations must be excluded from `--undocumented` and included as documented in stats.

#### Sibling-site sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Go block opening cleanup | `/*`, `/**` and starred body variants in the matrix | Extra opening/body stars removed; five false flags | Preserve text after the two-character opening delimiter. |
| Go line cleanup and paragraph detector | Equivalent literal-star, ordinary, indented and genuine-marker shapes | Correct negative and positive decisions | Clean; preserve those controls. |
| Go function, method and type readers | Same `/* *Deprecated: … */` before each declaration | All three publish true | All false. |
| Go variable/constant readers | Same comment before each declaration | Flags remain false | No additional false-positive instance observed. |
| Go documentation metrics | Star-only block versus line comment | Block is undocumented, line is documented | Both documented. |
| Java, JavaScript, TypeScript and PHP doc-tag readers | Copied fixtures, star-decorated blocks containing `Deprecated:` or `@deprecated`; also all 15 fence shapes | `Deprecated:` does not mark them; their actual `@deprecated` tags do; fence matrix passes | Clean: these formats use their own documentation decoration and tag rules. |
| Rust, C++, C#, Swift and Scala readers | Same two marker-text forms in attached block documentation | Neither comment form deprecates added declarations | Clean: these branches require attributes/annotations. |
| Python reader | Equivalent marker text in comments before a function; additional leading-comment/docstring probes | Marker comments do not deprecate it; genuine first-statement docstrings remain documented | Clean: deprecation uses decorators and documentation uses docstrings. |
| C, Perl, Bash, Zsh and Lua readers | Equivalent block/comment marker text in copied fixtures | No deprecation flags | Clean: no marker from this feature's source table. |
| Functions/types/symbols/classes selections | Go function, method and type with literal-star comments | Deprecated counts 2/1/3/1, respectively | All four counts 0. |
| JSON and JSON Lines | All 16 Go boundary rows | Agree on the same five false flags | Both consume the corrected extraction fact. |
| `--undocumented`, stats JSON and cache storage/projection | Star-only public metric probes; shared consumers inspected | Incorrect presence flows into selectors/aggregates and stored metrics | Fix normalization once; retain literal docs and invalidate old stored facts. |

Give Go its own delimiter/content-preserving cleanup before generic star decoration is removed. Keep Go paragraph separators and relative indentation intact; do not change Javadoc/Doxygen cleanup in the other languages. Extend the Go public-result matrix with every row above and the presence/coverage control, plus representative CLI selectors/aggregates. Update the changed helpers' comments and current documentation, and bump `EXTRACTOR_REVISION` with a revision-12 cache rejection regression because both flags and metrics are stored.

## Blocked Findings

None. The finding can be repaired with existing extraction helpers and verified through temporary fixtures and public query/CLI results.

## Recurrence

This finding repeats review-1's **Marker detection loses meaningful source boundaries**, and its documentation-presence consequence also overlaps **Python documentation coverage mistakes string expressions for docstrings**. Review-3's **Deprecation detection confuses code fence boundaries** is repaired for all four reported readers, but that repair's adjacent Go cleanup still discards meaningful text.

The earlier class sweeps should have checked both Go `//` and `/* */` paths, two-character block delimiters, literal leading stars, remaining indentation, genuine markers after literal paragraphs, and the normalization result's effect on `has_docs`, `--undocumented` and stats. This review carries the complete observed list: five false-marker shapes and the star-only presence case, with clean controls and all language-specific marker consumers enumerated above. `recurrence: true` records the incomplete wider-class repair; no recurrence was found in ownership or reusable test flags.

## Closure

Recorded one unblocked medium-priority finding, set the review count to four, linked review-4 from review-3 and marked review-3 implemented to record its repair cycle. The spec is not marked completed. This review changes only the requested review documents/frontmatter, makes no commit and leaves the feature in its current lifecycle directory.
