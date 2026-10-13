---
$schema: feature-review.yaml
ready: false
findings:
  - title: Go directives count as documentation and hide deprecation markers
    priority: medium
  - title: Go declaration wrappers hide attached documentation
    priority: medium
  - title: Block-comment cleanup erases literal stars in Rust, C#, and Swift
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T14:37:13-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-5.md
previous: 2026-10-03-symbol-filters-and-reports/review-4.md
next: 2026-10-03-symbol-filters-and-reports/review-6.md
---

The feature is **not production ready**. Review-4's reported Go cases are repaired, but three unblocked documentation findings remain. They change documentation selectors, coverage totals, or deprecation selections. None requires a human design decision.

## Previous review follow-through

Review-4 had one Unblocked Finding and no Blocked Findings. No previously blocked finding became actionable before the last repair.

| Previous finding | Verification | Assessment |
|---|---|---|
| Go block-comment cleanup discards meaningful documentation text | Go now bypasses the generic star-stripping cleanup. Independently copied the primary Go fixture and exercised all 16 reported boundary rows through complete JSON, selected JSON, JSON Lines, and deprecated counts. All returned the expected flags. The exact-name star-only pair reports documented 2 / total 2 / ratio 1.0. Registered library and CLI regressions passed. | Every reported instance is repaired. |
| Stored stale flags and metrics from that cleanup | Extractor revision is 13; the registered revision-12 cache rejection regression passed. Current docs and skill describe preservation of literal Go stars. | Addressed. |

The implementation log explicitly deferred directive-only Go documentation and mixed block-comment decoration in Rust/C#/Swift. Those are assessed below through public results, rather than accepted as harmless because they were deferred. The log's statement that mixed decoration causes no selector change is incorrect for star-only documentation: the third finding demonstrates the change.

Earlier repairs remain exercised by passing tests: patterned and curried parameters, receiver exclusions, Python string/docstring forms, repeated and absent owners, qualified implementations/extensions, nested Scala packages, Rust package-boundary changes, reusable filtered/unfiltered results, and terminal report layouts. Compared all four earlier reviews; recurrence is detailed below.

## Scope and verification

Used the tree-hugger, rust-testing, and biscuit-test-harness skills. Reviewed the spec, earlier reviews, repair log, documentation extraction, flags, metrics, query filtering/reuse, cache validation/projection, class summaries, report mode resolution, stats, current documentation, and test registration. This review changes review documents only.

Independent probes copied the primary language fixtures into temporary directories and added focused declarations. Lua additions precede the final return. Probes used the freshly built `target/debug/hug`, explicit file selection, and `--no-cache`, except an isolated cold/warm cache check. Expected Go/Rust/C# comment semantics were checked against primary tool documentation/source; Swift's own compiler additionally emitted a symbol graph for the same comment shapes.

| Check | Result |
|---|---|
| `cargo build -p tree-hugger-cli --color=never` | Passed. |
| `just test` in `tree-hugger/` | Initial run stopped on the concurrent kind-coverage change: C# constructor extraction contradicted a still-unsupported matrix row. 532 passed, one failed, 449 not run. |
| `just test --no-fail-fast` in `tree-hugger/` | All 982 selected tests ran: 973 passed, nine golden-output tests failed, zero skipped. Failures involve the added Java `Counter` constructor and changed constructor totals from the concurrent kind-precision work. |
| `just lint` in `tree-hugger/` | Passed for library and CLI. |
| `just test-l2` in `tree-hugger/` | Six selected entry points passed. tmux and WezTerm performed captures; Kitty took its unavailable-backend path. 265 tests were outside the selected tier. |
| Root `just check-tier-coverage tree-hugger` | Zero stranded tests. |
| Prior Go repair and independent class sweeps | Reported repair cases pass; the three matrices below identify remaining failures and clean siblings. |

The worktree contains concurrent changes for `2026-10-05-deferred-kind-precision`, including queries, classifiers, tests, and goldens. The changed first failure between the two runs confirms that this is not a fixed snapshot. The nine golden failures are reported as an external verification limitation, not attributed to the documentation repair or silently treated as passing. They must be reconciled before the combined worktree has a green suite. The readiness decision here independently rests on reproducible feature defects.

The consolidated library `l1` target declares the relevant modules. The CLI `symbols` target declares filters, reports, cache upgrades, and cache robustness. Real-terminal targets require `terminal-tests`, enabled by the live `test-l2` recipe and CI metadata. This review adds no implementation tests.

### Requirement-to-verification assessment

| Requirement | Appropriate level | Strongest evidence and limitation |
|---|---|---|
| Scope, async, names, containment, parameters, signature predicates | Level 1 | Registered per-language matrices, coverage guards, and public selection/query regressions passed. |
| Test flags, filesystem transitions, reusable results | Level 1 | Registered cache/layout/narrowing regressions passed. |
| Deprecation and documentation metrics/selectors | Level 1 | Existing matrices and prior repairs passed; independent public probes expose the missing cases below. |
| Filter intersections, smart case, exact names, command sets and conflicts | Level 1 | Registered library and CLI tests passed. |
| JSON/JSON Lines envelopes, ordering, count, stats, partial errors, closed output pipes | Level 1 | Registered structural/subprocess checks passed. Multi-language goldens are currently red because of concurrent extraction changes. |
| Styled lists, tables, outlines, class sections, stats, signatures, and width wrapping | Level 2 | Captured real tmux and WezTerm reports passed the layout/style/link/width assertions. |
| Plain text and escaping source control sequences | Level 1 | Registered byte/text checks passed; affected goldens have the external limitation above. |
| Keyboard, mouse, paste, input methods | Not applicable | This feature introduces no interactive input behavior; Level 3 is unnecessary. |

No inappropriate verification-level substitution was found. Cross-OS evidence is left to CI and does not determine readiness.

### Input robustness matrix

The load-bearing structured reader is the persistent JSON cache. Its [documented matrices](../../docs/cache-design.md) and [registered public-output test](../../cli/tests/symbols/cache_robustness.rs) cover entry fields and feature record fields: scope, qualified name, namespace rooting, modifiers, test/deprecation flags, metric-facet presence, parameter count, and documentation presence. Each column has control, absent, null, wrong whole-field type, one wrong element, every element wrong, empty, duplicate-key, and trailing-invalid outcomes where applicable. Required metrics cannot disappear under a claimed metrics capability; unsupported nullable metrics stay distinct from missing fields. The public-output matrix passed, including its served-entry sentinel checks.

The Rust package test rule checks manifest existence rather than parsing manifest content. Source readers consume syntax trees, so they add no second JSON/YAML/TOML configuration format. The remaining source-comment classification/attachment matrices are below. Cache validity does not repair a wrong fact generated by extraction: a warm directive probe was a full hit with zero parses and retained the wrong facts.

## Unblocked Findings

### Medium — Go directives count as documentation and hide deprecation markers

**Defect class:** documentation normalization retains non-documentation directives as prose, changing both presence and paragraph-boundary decisions.

In the tree-hugger library, [go_comment_group_text](../../lib/src/file/extract/doc_comments.rs:264) removes comment delimiters but retains every directive line. [has_deprecated_paragraph](../../lib/src/file/extract/flags.rs:265), which decides whether a Go declaration is deprecated, then treats that line as the start of a prose paragraph. This conflicts with the area's promise to follow Go's documentation reader. Go's [CommentGroup.Text implementation](https://go.dev/src/go/ast/ast.go) excludes unspaced directive lines, including `line `, `extern `, `export `, and the lowercase/digit colon directive form. A space after `//` makes the text ordinary prose instead.

Append these declarations to the copied Go fixture:

```go
//go:generate echo generated
func ReviewGenerate() {}

//go:generate echo generated
// Deprecated: use New.
func ReviewBeforeMarker() {}
```

```sh
hug functions --file go.go --name ReviewGenerate --exact --undocumented --count --no-cache
hug functions --file go.go --name ReviewBeforeMarker --exact --deprecated --count --no-cache
```

Both counts are `0`; both must be `1`. Exact-name stats for `ReviewGenerate` report documented 1 / total 1 / ratio 1.0, expected 0 / 1 / 0.0. The second declaration's marker is missed in complete JSON and JSON Lines as well. This can hide undocumented or deprecated APIs from a report used to select cleanup work.

#### Complete directive boundary matrix

Every row was added to the copied fixture. Complete JSON exposed both public facts; focused JSON Lines confirmed marker results. `docs` below means `has_docs`.

| Shape | Observed result | Expected result |
|---|---|---|
| No comment; empty `//` | docs false, deprecated false | Same — clean controls. |
| Normal prose comment | docs true, deprecated false | Same — clean positive control. |
| `//go:generate …`, `//go:build linux`, `//go:noinline` | Each docs true, deprecated false | Each docs false, deprecated false. |
| `//line original.go:1`, `//extern External`, `//export Exported` | Each docs true, deprecated false | Each docs false, deprecated false. |
| `//tool:directive`, `//a1:b2` | Each docs true, deprecated false | Each docs false, deprecated false. |
| Two consecutive directives, no prose | docs true, deprecated false | docs false, deprecated false. |
| Prose plus directive, no marker | docs true, deprecated false | Same — retain the real prose. |
| Directive followed by `// Deprecated: …` | docs true, deprecated false | docs true, deprecated true. |
| Intro, blank comment line, directive, then marker | docs true, deprecated false | docs true, deprecated true. |
| Marker followed by directive | docs true, deprecated true | Same — clean. |
| Intro, directive, marker without a blank line | docs true, deprecated false | Same — removing the directive does not create a new paragraph. |
| `// go:noinline`, `//Go:noinline`, `//go: noinline`, `//go:` | Each docs true, deprecated false | Same — none is a directive under Go's reader rule. |
| `/*go:noinline*/` | docs true, deprecated false | Same — directive removal is a line-comment rule. |

#### Sibling-site sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Go function, receiver method, type, alias, struct field | Directive alone; directive then marker, compared with real-doc and real-marker controls | Directive counted as docs; following marker missed at every site | Directive alone undocumented; following marker deprecated. |
| Go `var`, `const`, and type specs inside groups | Same directive and marker shapes attached directly to the spec | docs true; following marker false | docs false for directive-only; true marker. |
| Go standalone `var`/`const`, or docs before a group | Same comment forms | Even real-doc/marker controls are missed | Attachment defect belongs to the next finding; repair it before asserting these directive cases. |
| Rust, C#, Swift, C, C++ | Ordinary `//go:noinline` versus their supported doc delimiter carrying the same literal text | Ordinary comment undocumented; doc-form literal documented; no deprecation | Clean; do not apply Go's directive rule to these languages. |
| Java, JavaScript, TypeScript, Scala, PHP | Ordinary line comment versus `/**go:noinline*/` | false / true documentation, no deprecation | Clean. |
| Python | Ordinary marker-text comment versus actual first-statement docstring | false / true documentation, no deprecation | Clean. |
| Lua, Perl, Bash, Zsh | Marker-text comment before a function | Documentation unavailable, deprecation false | Clean within unsupported doc detection. |
| Documentation selector, stats, JSON/JSON Lines flags, warm cache | Exact-name probes above; warm complete-index run | Wrong presence/flag persists; warm run reports one hit and zero parses | All consumers must receive corrected extraction facts. |

Remove only the line directives Go's reader recognizes, before common indentation, documentation-presence, and paragraph analysis. Keep spaced lookalikes and block-comment text. Extend one Go public-result matrix with every shape above and the declaration sites, then assert selector/coverage consequences and invalidate stored facts with an extractor revision bump. Correct the current docs/helper comments that imply all retained comment text is documentation.

### Medium — Go declaration wrappers hide attached documentation

**Defect class:** documentation attachment stops at an inner declaration spec and fails to read the comment attached to its enclosing declaration.

In the tree-hugger library, [extract_doc_comment](../../lib/src/file/extract/doc_comments.rs:14) climbs a fixed [wrapper list](../../lib/src/file/extract/doc_comments.rs:42). It includes JavaScript variable wrappers and C++ templates, but not Go's `var_declaration`, `const_declaration`, or `type_declaration`. A captured Go spec therefore cannot see the comment before the enclosing statement/group. Go's [documentation reader](https://go.dev/src/go/doc/reader.go) reads value-declaration docs and uses enclosing type-declaration docs when the type spec has none.

```go
// ReviewSingleVar is documented.
var ReviewSingleVar int

// ReviewSingleConst is documented.
const ReviewSingleConst = 1
```

`hug symbols --file go.go --name ReviewSingle --undocumented --count --no-cache` prints `2`, expected `0`. Stats for the same selection report 0 / 2 documented, expected 2 / 2. Replacing those comments with actual `Deprecated:` paragraphs still gives a deprecated count of `0`, expected `2`. Existing ordinary Go variable/constant controls in review-4 did not include genuine marker/docs controls, so their false results masked the attachment failure.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Go standalone `var` | Genuine docs or marker before statement | docs false; marker false | docs true; marker true for marker control. |
| Go standalone `const` | Same | Same failures | Same expected facts. |
| Go multi-name `var`/`const` spec | Docs before statement | Every extracted name is undocumented | Every extracted name shares the declaration docs; this finding does not request extraction of missing names. |
| Go parenthesized `var` group | Docs before group; two specs | Both undocumented | Both documented. |
| Go parenthesized `const` group | Same | Both undocumented | Both documented. |
| Go parenthesized type group | Same | Both types undocumented | Both documented from the group. |
| Go docs inside `var`/`const`/type groups | Genuine docs immediately before individual spec | docs true | Clean; preserve spec-specific docs. |
| Go standalone type/alias, function, receiver method, struct field | Genuine docs and marker controls | Correct docs and marker flags | Clean; preserve these controls. |
| Go local variable | Comment inside function before local `var` | Undocumented | Clean under existing exclusion of context-less locals from attached docs. |
| Go interface-method probe | Comment before method signature | No standalone method record | Documented extraction limit; no new kind requested. |
| Rust, JavaScript, TypeScript | Documented function plus variable and constant declarations; Rust `static` | Every extracted declaration documented | Clean, including JavaScript/TypeScript variable wrappers. |
| Java, C#, PHP | Documented method/function plus field and constant | Every extracted declaration documented | Clean. |
| C, C++, Swift, Scala | Documented function/method plus variable and constant/value | Every extracted declaration documented | Clean. |
| Python | Genuine function/class docstring controls and leading-comment control | Docstrings recognized; ordinary comments ignored | Clean; Python variables do not use declaration doc comments. |
| Lua, Perl, Bash, Zsh | Primary fixtures and comment controls | Documentation unavailable | Clean within unsupported doc detection. |
| Public documentation selector, stats, deprecated selector | The two standalone declarations above | Count 2 undocumented, coverage 0/2, count 0 deprecated | Count 0 undocumented, coverage 2/2, count 2 deprecated. |

Read the containing Go declaration when a spec lacks attached docs, preserving any spec-specific documentation. Cover standalone, multi-name, before-group, and inside-group forms for all three wrapper kinds; verify deprecated selection, documentation selection, and coverage through public results. Update relevant helper contracts/current docs and bump the extractor revision because these facts are cached.

### Medium — Block-comment cleanup erases literal stars in Rust, C#, and Swift

**Defect class:** shared block-documentation cleanup removes a leading star independently on every line even when the language requires consistent decoration, deleting literal documentation content.

In the tree-hugger library, [clean_block_comment](../../lib/src/file/extract/doc_comments.rs:244) trims each line and unconditionally strips one leading `*`. The same reader serves ten languages. Rust's [rustdoc cleanup](https://raw.githubusercontent.com/rust-lang/rust/master/compiler/rustc_ast/src/util/comments.rs) checks for consistently placed stars before removing decoration. C#'s [documented block-comment rules](https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/xmldoc/) likewise require a common pattern. Independently invoking Swift's compiler with symbol-graph output confirms it retains meaningful stars in the cases below.

```rust
/**
 *
  *
*/
pub fn ReviewMisalignedSpaces() {}
```

The equivalent comment before a C# class or Swift function has the same failure. The public record reports `has_docs: false`; exact-name `--undocumented --count` prints `1`, expected `0`; stats report 0 / 1 documented, expected 1 / 1. This is a metrics/filter defect in this feature even though the underlying raw-documentation cleanup is older. In mixed prose, `*b* is next` also becomes `b* is next`.

#### Complete decoration matrix

Each shape was appended to copied fixtures for all eleven comment-reading languages. Python and the four languages without doc detection were checked separately. The table gives the measured documentation result for each affected reader; results were identical for Rust, C#, and Swift.

| Shape | Observed result | Expected result |
|---|---|---|
| Blank `/** */` | Undocumented | Undocumented — clean. |
| Two aligned decoration-only lines, each ` *` | Undocumented | Undocumented — clean. |
| Star-only lines with one versus two leading spaces | Undocumented; docs absent | Documented; remaining stars are literal text. |
| Star-only lines with tab versus spaces | Undocumented; docs absent | Documented; remaining stars are literal text. |
| Single-line `/** * */` | Undocumented; docs absent | Documented; the first-line star is content. |
| Unstarred prose followed by ` *b* is next` | Documented; raw text loses the first `*` of `*b*` | Documented; preserve the literal emphasis delimiter. |
| Plain nonempty block doc | Documented | Documented — clean. |
| `/// *` | Documented; star retained | Documented — clean line-comment sibling. |

Swift's generated symbol graph retained a star in the space-misaligned case, both stars in the tab/space case, `*` in the single-line case, and `*b* is next` in the mixed-prose case. It returned empty text for the aligned decoration-only control, matching the expected distinction.

#### Sibling-site sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust function/type/method/constant/static/field readers | Space-misaligned star-only block | All extracted declarations undocumented | All documented. |
| C# class/method/field/property readers | Same | All undocumented | All documented. |
| Swift function/type/method/stored property/top-level `let` and `var` readers | Same | All undocumented | All documented. |
| Java, JavaScript, TypeScript, Scala, PHP | All eight shapes using their doc forms | Decoration-only blocks empty; prose blocks nonempty; `///` unsupported | Clean for documentation presence under their per-line star decoration convention; do not impose Rust/C#/Swift's rule on them. |
| C, C++ | All eight shapes with Doxygen forms | Decoration-only blocks empty; prose and `/// *` nonempty | Clean for the supported Doxygen convention. |
| Go | All eight shapes | Stars retained; every `/**…*/` shape nonempty, including the extra opening star | Clean after review-4's repair; Go has no star decoration. |
| Python | String/docstring text containing `*`; comment controls | Genuine docstring documented, ordinary comment ignored | Clean; no block-decoration processing. |
| Lua, Perl, Bash, Zsh | Comment controls | Documentation unavailable | Clean within unsupported detection. |
| Metrics, documentation filter, stats, cached record consumers | Public exact-name probes in all three affected languages; shared consumers inspected | Wrong absence reaches selectors/coverage and stored facts | Correct once during extraction; invalidate stored facts. |

Give Rust, C#, and Swift their own consistent-decoration rule before removing stars. Preserve nonempty literal text and mixed prose; retain the clean blank/aligned and other-language controls. Extend one public-result matrix with all eight shapes per affected language, representative declaration families, `--undocumented`, and stats. Update helper comments/current docs and bump the extractor revision.

## Blocked Findings

None. All three findings can be repaired within the existing architecture and verified through isolated source fixtures and public results.

## Recurrence

The directive finding repeats review-1's **Marker detection loses meaningful source boundaries** and review-3's **Deprecation detection confuses code fence boundaries**: non-prose text changes whether a genuine marker starts a paragraph. The Go sweeps should have included all directive forms removed by `CommentGroup.Text`, spaced lookalikes, block-comment controls, consecutive directives, and directives before/after marker paragraphs. This review lists all those forms and the declaration readers that consume them.

The block-decoration finding directly repeats review-4's **Go block-comment cleanup discards meaningful documentation text**: another language's decoration rule deletes literal content before documentation presence is measured. That repair should have swept the shared cleanup's Rust, C#, and Swift branches for inconsistent star columns, tabs, single-line literal stars, mixed prose, and selector/stats consequences. The implementation log noticed part of this class but incorrectly treated its feature-visible consequence as absent. This review includes the affected branches and clean siblings across all 16 languages.

The Go wrapper finding is a newly identified attachment defect. Earlier review-4 variable/constant rows lacked real-documentation and real-marker positive controls; they therefore could not establish that those reader sites were clean. The current sweep includes standalone declarations, multi-name specs, comments before groups, comments inside groups, and all corresponding language readers.

`recurrence: true` records the repeated boundary/normalization classes. No recurring ownership, signature, reusable-test-flag, or terminal-verification defect was found.

## Closure

Recorded three unblocked medium-priority findings. Linked review-5 from review-4, marked review-4 implemented to record its repair cycle, and advanced the spec's review count to five. The spec is not marked completed. No implementation code was repaired, no commit was made, and the feature remains in its current lifecycle directory.
