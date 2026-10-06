---
$schema: feature-review.yaml
ready: false
findings:
  - title: Empty Go spec comments inherit group documentation and deprecation
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T16:11:27-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-6.md
previous: 2026-10-03-symbol-filters-and-reports/review-5.md
next: 2026-10-03-symbol-filters-and-reports/review-7.md
---

The feature is **not production ready**. Review-5's reported cases are repaired, but one unblocked documentation-precedence finding remains. An attached empty or directive-only Go comment incorrectly inherits its declaration group's documentation and deprecation marker. No human design decision is required.

## Previous review follow-through

Review-5 contained three Unblocked Findings and no Blocked Findings. All earlier reviews also declared no blocked findings; there are no newly unblocked items to reconcile.

| Previous finding | Verification | Assessment |
|---|---|---|
| Go directives count as documentation and hide deprecation markers | The directive/site matrix and CLI regression passed. Independent copies of the Go fixture return count 1 for an undocumented directive-only function and count 1 for a directive followed by a genuine deprecation paragraph. Same-line trailing-comment and Doxygen member-after controls also pass their registered regressions. | Reported cases repaired. A directive-only comment inside a documented group exposes the precedence finding below. |
| Go declaration wrappers hide attached documentation | The wrapper matrix and CLI regression passed. Independent standalone var/const probes report zero undocumented declarations and coverage 2/2, ratio 1.0. Nonempty spec-specific comments override group text. C/C++ single linkage declarations and Scala single-method extensions have registered controls. | Reported cases repaired. Empty spec-specific comments are not covered by the precedence controls. |
| Block-comment cleanup erases literal stars in Rust, C#, and Swift | The decoration matrix and structured-summary regression passed. Independent public probes in all three languages cover misaligned stars, tab/space disagreement, a single literal star, an empty block, line-comment stars, and mixed prose. Meaningful stars remain in raw docs and structured summaries; empty controls remain undocumented. | Repaired; no additional documentation-presence failure established in this sweep. |
| Cached facts from the repaired extraction | Extractor revision is 15. The revision-14 rejection test passed, alongside all earlier revision controls. | Addressed for the reported repairs. Correct caching still preserves the newly discovered wrong fact. |

The earlier parameter, owner-identity, file-wide test-classification, result-reuse, Python docstring, fence-boundary, and Go literal-star regressions passed in the complete Level-1 run. Compared findings in reviews 1–5; recurrence is described below.

## Scope and verification

Used the tree-hugger, rust-testing, and biscuit-test-harness skills. Read the specification, previous reviews, implementation logs, documentation extraction/normalization, flags, metrics, declaration wrappers and metadata selection, record filtering, query finishing/narrowing, cache validation/projection, report-mode resolution, stats, documentation, and test registration. This review changes review documents and metadata only.

Independent CLI probes used a newly built `target/debug/hug`, temporary copies of the primary language fixtures, explicit file selection, and `--no-cache`. Lua additions precede the fixture's final return. The Go precedence matrix contains 21 comment shapes at each of six declaration sites, with every primary name present and no reported errors. The second name of each multi-name var spec was checked too; multi-name const extraction still reports only its first name, an existing limit outside this feature. A separate Go program used `go/parser` and `go/doc` as an independent oracle for type/alias precedence. An isolated cold/warm cache probe used the complete index and checked verbose hit/parse counters.

| Check | Result |
|---|---|
| `cargo build -p tree-hugger-cli --color=never` | Passed. |
| Initial `just test` in `tree-hugger/` | 486 passed, one repository-scanning test timed out at 30 seconds, 527 not run. The timeout was `file_identity::god_files_paths_are_relative_to_any_spelling_of_the_root`. |
| `just test -j 4` in `tree-hugger/` | All 1,014 tests passed, zero skipped, including the previously timed-out test, all format goldens, cache robustness, and the new repair matrices. |
| `just lint` in `tree-hugger/` | Library and CLI passed. |
| `just test-l2` in `tree-hugger/` | Six selected entry points passed. tmux and WezTerm executed real-terminal captures; Kitty entry points took their unavailable-backend path. 276 tests were outside the selected tier. |
| Root `just check-tier-coverage tree-hugger` | Zero stranded tests. |
| Independent Go reference reader and shipped CLI matrix | Confirm the absent-versus-empty precedence defect described below. |

The worktree already contains concurrent kind-precision changes. They were preserved. Unlike review-5's run, the final full-area run has no golden failures or compilation failures. The initial timeout is reported as a verification limitation resolved by the lower-concurrency run, not as a demonstrated feature defect.

### Requirement-to-verification assessment

| User-facing requirement | Appropriate level | Evidence and remaining gap |
|---|---|---|
| Scope, async, qualified names, containment, signature parameters/type predicates | Level 1 | Per-language matrices, public query checks, coverage guards, and CLI regressions passed. |
| Test/deprecation flags, documentation metrics, selectors and coverage | Level 1 | Registered matrices and CLI regressions passed; the independent precedence matrix finds missing cases below. |
| File/name selection, smart case, exact matching, intersections, command sets and conflicts | Level 1 | Library and CLI filter tests passed, including non-ASCII and class/member selection. |
| JSON/JSON Lines, count, stats, envelopes, ordering, partial errors and closed pipes | Level 1 | Structural/subprocess checks and output goldens passed. |
| Cache capability widening/projection, stable IDs, scan-root prefixes, filesystem transitions and reusable results | Level 1 | Cache, package-boundary, narrowing, and stale-revision regressions passed. The new precedence mistake persists on a genuine cache hit. |
| Styled lists, class sections, tables, outlines, stats, signature modes, wrapping and applicable links | Level 2 | Shared real-terminal declaration checks executed successfully in tmux and WezTerm. |
| Plain/redirected text and escaping literal source controls | Level 1 | Byte/text assertions and goldens passed. |
| Keyboard, mouse, paste and input methods | Not applicable | These report commands add no interactive input behavior; Level 3 is unnecessary. |

The library's consolidated `l1` target declares the relevant modules. The CLI `symbols` target declares filters, reports, cache robustness and cache upgrades. Terminal targets require `terminal-tests`, enabled by the live Level-2 recipe and CI metadata. No implementation test was added or renamed by this review. No inappropriate verification-level substitution was found. Cross-OS evidence remains a CI responsibility and does not determine readiness here.

### Input robustness matrix

The feature's load-bearing structured reader is the persistent JSON cache. Its [documented matrices](../../docs/cache-design.md) and [public-output matrix](../../cli/tests/symbols/cache_robustness.rs) cover entry fields and feature record fields: scope, qualified name, namespace rooting, modifiers, test/deprecation flags, metrics-facet presence, parameter count and documentation presence. Each column enumerates control, absent, explicit null, wrong whole-field type, one wrong element, every element wrong, empty, duplicate key and trailing-invalid outcomes, including inapplicable shapes. Required metric fields remain distinct from unsupported nullable metrics. The matrix passed and uses a served-entry sentinel, so equal output alone cannot disguise recomputation.

The Rust package test rule observes manifest existence rather than parsing manifest fields. The source readers consume syntax trees; they introduce no additional configuration format. The finding below concerns source attachment state lost before normalization, not cache deserialization. Its complete source-shape matrix therefore complements the cache-format matrix.

## Unblocked Findings

### Medium — Empty Go spec comments inherit group documentation and deprecation

**Defect class:** a fallback documentation reader conflates an attached comment group whose normalized text is empty with no attached comment group, allowing parent documentation to replace an explicit child attachment.

In the tree-hugger library, [collect_doc_comments](../../lib/src/file/extract/doc_comments.rs:99), which finds and cleans a declaration's attached comments, returns `None` both when there is no attached group and when its text becomes empty. [extract_doc_comment](../../lib/src/file/extract/doc_comments.rs:18), which chooses a declaration's documentation or its wrapper's fallback, consequently climbs to the parent in both cases. Blank comments and correctly removed directives therefore resurrect unrelated group text.

For Go types, the distinction is explicit in [Go's documentation reader](https://go.dev/src/go/doc/reader.go): `readType` falls back only when the spec's comment-group pointer is absent, before calling `Text()`. Running `go/parser` plus `go/doc` on the reproduction independently returned empty text for the blank, whitespace-only, empty-block, directive-only and alias controls, while the absent control inherited the group text. For per-symbol var/const reports, the expected precedence below follows this area's [documented contract](../../docs/symbol-kinds-and-visibility.md:667), which says a spec's own doc wins. Go's standard reader aggregates var/const groups, so its aggregate output is not used as an oracle for those individual symbol rows.

Append this to a copy of the primary Go fixture:

```go
// Deprecated: use New.
type (
    //
    ReviewEmpty int
    //go:noinline
    ReviewDirective = int
    ReviewAbsent int
)
```

```sh
hug types --file go.go --name ReviewEmpty --exact --undocumented --count --no-cache
hug types --file go.go --name ReviewEmpty --exact --deprecated --count --no-cache
hug types --file go.go --name ReviewEmpty --exact --stats --json --no-cache
```

The counts are `0` and `1`, expected `1` and `0`. Stats report documented 1 / total 1 / ratio 1.0, expected 0 / 1 / 0.0. Complete JSON and JSON Lines publish the group's raw text and `is_deprecated: true`. `ReviewDirective` has the same error; `ReviewAbsent` correctly inherits the group's documentation and deprecation. This hides undocumented declarations and falsely labels APIs deprecated.

#### Complete comment-shape matrix

Every row below was applied to each site in the following instance table, with a parent group containing `// Deprecated: group doc.`. `docs/old` means public `has_docs` / `is_deprecated`; each value was read from `hug --json-full symbols`. Plain and deprecated child prose controls also checked the selected raw text.

| Child comment shape | Observed docs/old | Expected docs/old | Assessment |
|---|---|---|---|
| Absent | true / true; group text | true / true; group text | Clean fallback control. |
| Empty `//` | true / true; group text | false / false; no text | Fails. |
| Whitespace-only `//   ` | true / true; group text | false / false; no text | Fails. |
| Empty `/* */` | true / true; group text | false / false; no text | Fails. |
| Multiline blank `/*` body | true / true; group text | false / false; no text | Fails. |
| `//go:generate …` | true / true; group text | false / false; no text | Fails. |
| `//go:build linux` | true / true; group text | false / false; no text | Fails. |
| `//go:noinline` | true / true; group text | false / false; no text | Fails. |
| `//line original.go:1` | true / true; group text | false / false; no text | Fails. |
| `//extern External` | true / true; group text | false / false; no text | Fails. |
| `//export Exported` | true / true; group text | false / false; no text | Fails. |
| `//tool:directive` | true / true; group text | false / false; no text | Fails. |
| `//a1:b2` | true / true; group text | false / false; no text | Fails. |
| Own ordinary prose | true / false; child text | Same | Clean own-doc control. |
| Own `Deprecated:` paragraph | true / true; child text | Same | Clean own-marker control. |
| Own prose separated from spec by a blank source line | true / true; group text | Same | Clean detached-comment control: no attachment. |
| `// go:noinline` | true / false; child text | Same | Clean spaced lookalike. |
| `//Go:noinline` | true / false; child text | Same | Clean uppercase lookalike. |
| `//go: noinline` | true / false; child text | Same | Clean colon-space lookalike. |
| `//go:` | true / false; child text | Same | Clean empty-suffix lookalike. |
| `/*go:noinline*/` | true / false; child text | Same | Clean block-comment text. |

There are 12 failing shapes and nine clean shapes at every primary site, not just one directive spelling. All 126 primary site/shape cells were exercised; each extracted second var name agrees with its first name.

#### Instance and sibling-site sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Go grouped type spec | All 21 shapes | All 12 empty-normalized attachments inherit docs and deprecation; nine controls pass | An attached empty group suppresses parent fallback; controls unchanged. |
| Go grouped type alias | Same matrix | Same 12 failures | Same precedence; independently confirmed for the empty alias by Go's reader. |
| Go grouped var spec | Same matrix | Same 12 failures | Preserve the documented per-symbol own-comment precedence for empty attachments too. |
| Go grouped const spec | Same matrix | Same 12 failures | Same. |
| Go multi-name var spec | Same matrix, both names | Both names have the same 12 failures | Both use their own spec attachment; absent attachment still shares group docs. |
| Go multi-name const spec | Same matrix, extracted first name | Same 12 failures; second name remains unextracted | Correct the extracted name's precedence; no new extraction requested. |
| Go standalone var/const, function; literal-star function docs | Independent directive-only, real-marker, genuine-doc and literal-star controls without a documented parent group | Correct undocumented/deprecated counts and coverage | Clean; preserve review-5's repairs. |
| Go trailing-comment exclusion | Prior-spec trailing comment before a spec in a documented group | Correctly ignores trailing text and inherits the group's doc | Clean: an excluded trailing comment is not an own attachment. |
| C/C++ single linkage, C++ template, Scala single extension | Absent, empty/blank doc blocks, own prose, own marker text, ordinary comments | Empty text inherits wrapper docs; nonempty text wins | The same information loss is reachable. Go's comment-group suppression rule is not established for these constructs; preserve their current language-specific behavior when fixing Go. |
| C++ braced linkage block | Same forms under a documented wrapper | Empty/absent comments remain undocumented; own prose documents the member | Clean; no wrapper climb. |
| JS/TS lexical variable and function-valued binding wrappers | Same six forms | Variable records retain the statement doc; bindings use nonempty own text and inherit statement text when empty | Enumerated shared callers; no Go spec-group rule applies. Preserve their existing behavior rather than imposing Go precedence globally. |
| JS/TS export wrappers; TS ambient wrapper | Same forms | Export own prose recognized; empty doc does not acquire parent text in the split-keyword probe. Ambient function has no record | Grammar parses split `export` as a separate identifier; ambient function is outside the extracted set. Existing inline export/ambient-variable controls pass. These probes do not establish an additional wrapper defect. |
| Rust module child, Java/C#/Swift/PHP type member | Absent, empty/blank docs, genuine own docs/marker text, ordinary comment | Parent docs do not leak into the child; own docs remain present | Clean controls; ordinary lexical owners are not documentation wrappers. |
| Python method in documented class | Empty versus nonempty docstring; additional leading-comment function/class/method, async and decorated controls | Empty method remains undocumented; nonempty and commented-first-statement docstrings remain documented | Clean; class documentation never substitutes for a method's docstring. |
| Lua, Perl, Bash, Zsh | Empty and prose comments on members/nested functions in a documented-looking owner | `has_docs` unavailable; no inherited raw docs | Clean unsupported behavior. |
| Documentation metrics and flags; JSON/JSON Lines; undocumented/deprecated selection and stats | Exact-name public commands above | Wrong parent fact reaches every consumer | Consume the corrected attachment decision; no separate report-specific matcher needed. |
| Persistent cache lookup/projection | Isolated cold complete-index run, then identical warm run | Both report docs true / deprecated true; warm run has one hit and zero parses/captures/diagnostics runs | Both must expose the corrected fact; invalidate revision-15 entries when extraction changes. |

The non-Go rows deliberately distinguish reachable shared behavior from confirmed Go precedence defects. An empty comment need not have identical inheritance semantics in every language. The repair must retain that distinction.

Preserve attached-group presence until the Go fallback decision is complete. Only an absent attachment may fall back to the declaration group; a present attachment can then normalize to no documentation. Keep genuinely absent, detached and trailing-comment controls, and keep source-based test flags unchanged. Extend one public-result matrix across all six Go sites with all 21 shapes above, including both multi-var names, then verify undocumented selection, deprecated selection, stats, complete JSON and JSON Lines. Bump the extractor revision and add the stale-revision cache regression. Update the helper contract, current documentation and skill to distinguish absent attachment from present empty attachment. No dependency or human ruling is needed.

## Blocked Findings

None.

## Recurrence

**Empty Go spec comments inherit group documentation and deprecation** repeats review-5's **Go declaration wrappers hide attached documentation** at the precedence boundary, and intersects its **Go directives count as documentation and hide deprecation markers** repair. Those fixes tested nonempty own comments and directive-only comments separately, but should have crossed empty/directive-only own groups with documented parent groups for type specs, aliases, vars, consts and both multi-name forms. Normalization now correctly removes directives; losing attachment presence afterwards still selects the wrong documentation source.

This review carries all six Go sites and all 21 shapes, including clean controls, downstream selectors/reports and warm caching. It also enumerates the shared non-Go wrapper callers, direct owners, Python docstrings and unsupported languages without assuming their rules are identical. Earlier reviews 1–4 identify broader marker/documentation classes, but no additional recurrence was demonstrated in their repaired parameter, owner, reusable-result, fence or literal-star cases. `recurrence: true` records the incomplete attachment-class sweep.

## Closure

Recorded one unblocked medium-priority finding. Linked review-6 from review-5, marked review-5 implemented to record its repair cycle, and advanced the specification's review count to six. The specification is not marked completed. No implementation code was changed, no commit was made, and the feature remains in its current lifecycle directory.
