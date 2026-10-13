---
$schema: feature-review.yaml
ready: false
findings:
  - title: Scala block packages disappear from qualified names and selectors
    priority: high
  - title: Shared-context lookups still rescan repeated names and package segments
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T17:42:23-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-9.md
previous: 2026-10-03-symbol-filters-and-reports/review-8.md
next: 2026-10-03-symbol-filters-and-reports/review-10.md
---

The feature is **not production ready**. Scala block-form packages are omitted from names, breaking qualified selectors and making declarations in different packages indistinguishable by name. The previous performance repair removes the original flat-file scans, but several shared-context lookups still grow quadratically when declarations repeat names or introduce additional package scopes. These are two unblocked findings; neither needs a human design decision.

## Previous review follow-through

Review-8 had two unblocked findings and no blocked findings. Earlier reviews likewise contain no blocked findings, so there is no newly unblocked requirement to reconcile.

| Previous finding | Verification | Assessment |
|---|---|---|
| Declaration metadata repeatedly searches shared file and module context | File-level namespaces now use a lazy index; Rust inner test attributes use a per-file memo. Additional indexes cover sibling containers, visibility, owners and field grouping. The registered 13-shape work-count test passes, and the implementation log records bounded before/after measurements with identical output. | The original flat-file cases are repaired. The repeated-name/package cases below show that the broader class is only partially repaired. |
| Large valid Zsh files abort the process in the existing grammar | Dependency remains `tree-sitter-zsh` 0.56.0. The repair log explicitly defers a patched/replaced grammar to the separate `zsh-scanner-state-abort` fix. Independently repeated the valid fixture plus 500 functions: functions, types, symbols, classes, imports, lint and god-files all exit with SIGABRT and the scanner's `size == length` assertion. `/bin/zsh -n` accepts the input. | **Not implemented.** Investigation and a separate fix record were added. This remains the pre-existing external defect identified in review-8, not a new symbol-report finding or an assertion that Zsh is safe. |

Marking review-8's frontmatter implemented records completion of its repair cycle; the table above states the actual outcome, including its deferral.

## Scope and verification

Used the tree-hugger, rust-testing and biscuit-test-harness skills. Reviewed the specification, every earlier review's findings, repair records, source metadata/name construction, shared predicates, query finishing/reuse, cache validation/projection, class selection, report modes/writers, current topic documentation and test registration. This review edits review documents and frontmatter only.

The workspace includes concurrent symbol-type-relations work. Current CLI probes report extractor revision 20; the shared-context reproductions agree between the built workspace library and a separate temporary executable using the public library API. Concurrent metadata additions are not treated as requirements of this specification.

| Check | Result |
|---|---|
| `cargo build -p tree-hugger-cli --color=never` | Passed; this CLI was used for independent output checks. |
| Area `just test --profile ci -j 2 symbols_` | 255 passed; 785 tests outside the selection. Includes extraction/selection matrices, earlier repair regressions and the new shared-context tests. |
| Root `just test tree-hugger-cli --profile ci -j 2` | Recipe selected 287 tests, all passed with zero skipped; nextest's output identifies its effective profile as `default`. |
| Area `just test-l2` | Six entry points passed. tmux and WezTerm executed declaration and god-files captures; Kitty followed its unavailable-backend paths. 287 tests were outside this tier. |
| Root `just check-tier-coverage tree-hugger` | Zero stranded tests. |
| Area `just lint` | Failed at `lib/src/file/extract/modifiers.rs:193`, `clippy::collapsible_match`, in the concurrent modifier implementation. This is outside the changes required by this spec. The recipe stopped before CLI lint. |
| Independent all-language report comparisons | 144 comparisons passed: nine selector combinations per primary fixture, copied into an isolated Git repository with test-owned home/config/temp directories. Counts, all three stats maps and JSON Lines record IDs agree with filtered complete public records. |
| Independent namespace and work sweeps | Public results expose the findings below. Namespace reproductions parse without syntax errors. Work measurements use counters, not elapsed-time thresholds. |

The independent comparisons verify wiring, while the registered language matrices pin expected extraction facts. A complete library suite was not rerun, and the selected run is not represented as one. No implementation test was added or renamed. The library's `l1/main.rs` declares `symbols_shared_context`; the CLI `symbols` target declares its report/cache tests. Both terminal targets require `terminal-tests`, enabled by the live L2 recipe and package CI metadata.

### Requirement-to-verification assessment

| Requirement | Appropriate level | Present evidence and gap |
|---|---|---|
| Scope, qualified names, async, owner identity and containment | Level 1 | Language matrices and owner/name regressions pass; Scala block-package cases are absent and fail the independent sweep. |
| Test/deprecation flags, documentation/parameter metrics and unsupported values | Level 1 | Source-shape matrices, package-layout/cache transitions and query-reuse regressions pass. |
| Smart case, exact/type/container/file selectors, imports and argument conflicts | Level 1 | Library predicates and CLI subprocess tests pass; qualified Scala package selectors fail below. |
| JSON/JSON Lines, ordering, count/stats, empty selections, failures, broken pipes and cache equivalence | Level 1 | Structural assertions, goldens, revision/robustness tests and independent comparisons pass on their fixtures. Wrong source-derived names propagate consistently into these reports. |
| Styled lists/classes, outlines, stats tables, signatures, wrapping and links | Level 2 | Declaration scenarios execute in tmux and WezTerm and cover each new view. |
| Plain/redirected text and control-sequence escaping | Level 1 | Byte/text assertions and goldens pass. |
| Shared-context work proportional to declarations | Level 1 | The new test covers growing files with distinct names or a fixed number of packages. Repeated-name and growing-package cases below violate its stated bound. |
| Keyboard, mouse, paste or input-method behavior | Not applicable | No interactive input behavior is introduced; Level 3 is unnecessary. |

No terminal-verification-level mismatch was found. Cross-OS evidence is left to CI and does not determine readiness.

### Input robustness matrix

The persistent JSON cache is the relevant structured reader. Its [field-by-shape contract](../../docs/cache-design.md) and [public-output matrix](../../cli/tests/symbols/cache_robustness.rs) cover entry identity/coverage, scope, qualified name, namespace rooting, modifiers, test/deprecation flags, metrics presence and nullable parameter/documentation values. The control, absent, explicit null, wrong whole-field type, one/all wrong elements, empty, duplicate-key and trailing-invalid outcomes are represented, with inapplicable cells stated. A stored sentinel distinguishes serving an edited entry from silently recomputing it. These tests passed in the complete CLI run.

Required nullable metrics distinguish missing keys from unsupported values. Entry fields have no permissive defaults; accepted record defaults follow the documented schema. No new manifest-content parser is involved: the Rust test-file rule checks manifest existence. The findings below concern syntax-tree context, not configuration-field coercion.

## Unblocked Findings

### High — Scala block packages disappear from qualified names and selectors

**Defect class:** an enclosing namespace represented by a body-bearing syntax node is omitted from declaration identity, so sibling namespaces collapse onto the same qualified name.

In the tree-hugger library, [file_level_namespace](../../lib/src/file/extract/qualified_name.rs:402) deliberately excludes Scala package clauses with a body, because those clauses scope their children rather than subsequent file statements. However, [Builder::prefix](../../lib/src/file/extract/qualified_name.rs:195), which finds a declaration's enclosing naming context, has no corresponding handling for those package bodies: [ancestor_container](../../lib/src/file/extract/classify.rs:611) does not recognize `package_clause`, and the unextracted-name fallback does not recognize it either. The existing Scala tests cover plain and stacked file-level package clauses only.

Copy the primary Scala fixture, which starts with `package example.symbols`, and append:

```scala
package R9Left { class R9W { def probe(): Unit = () } }
package R9Right { class R9W { def probe(): Unit = () } }
package R9Outer {
  package R9Inner { class R9Nested { def probe(): Unit = () } }
}
class R9After
```

Both classes are reported as `example.symbols.R9W`, and both methods as `example.symbols.R9W.probe`. The expected names are `example.symbols.R9Left.R9W` and `example.symbols.R9Right.R9W`, with their respective `.probe` suffixes. The nested class should be `example.symbols.R9Outer.R9Inner.R9Nested`; it instead becomes `example.symbols.R9Nested`. The trailing `R9After` correctly remains `example.symbols.R9After`.

The shipped CLI returns **0 instead of 1** for each of these selectors against that copy:

```sh
hug functions --file scala.scala --name example.symbols.R9Left.R9W.probe --exact --count
hug functions --file scala.scala --in example.symbols.R9Left.R9W --exact --count
hug classes --file scala.scala --in example.symbols.R9Left.R9W --exact --count
```

The corresponding exact symbol selection with `--stats --json` reports empty count maps. Separate declaration IDs still exist, but IDs do not make qualified-name selection work.

#### Complete namespace-form and consumer sweep

Each applicable block form was tested by adding same-named declarations in two named scopes to its primary fixture. PHP's original statement namespace was converted into an equivalent bracketed namespace before adding sibling bracketed namespaces, avoiding an illegal mix of PHP namespace styles. All seven resulting files have zero syntax-error diagnostics. Statement-only controls come from the passing all-language matrices and copied-fixture comparisons.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Scala `package_clause`, body present | Sibling packages, nested packages, with an existing file-level package | Every body package segment disappears; sibling names collide | Retain every enclosing package segment; distinguish siblings. |
| Scala `package_clause`, body present | Same body shapes with the fixture's file-level package removed | Names become `file#R9W` / `file#R9Nested`, `is_namespace_rooted: false` | Explicit package names root the names even without a preceding statement package. |
| Scala `package_clause`, body absent; declaration after a package body | Plain/stacked clauses; `R9After` | Correct statement prefixes and restoration after the body | Clean; preserve these controls. |
| Rust `mod_item` | Same-named types and impl methods in sibling modules | `file#R9Left::R9W::probe` and corresponding right name | Clean; preserve Rust's file prefix. |
| TypeScript `internal_module` | Same-named classes in sibling namespaces | Distinct `file#R9Left.R9W.probe` / right names | Clean; preserve TypeScript's file prefix. |
| C++ `namespace_definition` | Same-named classes in sibling namespaces | Distinct `R9Left::R9W::probe` / right names | Clean. |
| C# `namespace_declaration`; file-scoped statement | Sibling block namespaces; primary statement-namespace control | Distinct rooted names | Clean. |
| PHP `namespace_definition`, body present/absent | Sibling bracketed namespaces; primary statement control | Distinct `R9Left\R9W::probe` / right names | Clean; preserve native separators. |
| Perl `package_statement`, block present/absent | Sibling package blocks; primary replacement-statement control | Distinct `R9Left::probe` / right names | Clean; package names remain absolute. |
| Java `package_declaration`; Go `package_clause` | Primary package controls | Correct rooted declaration names | Clean; these languages have no equivalent block-package form. |
| JS, Python, C, Swift, Lua, Bash, Zsh | Primary naming/owner matrices | Existing file/owner names pass | No block-package reader to repair; retain existing naming contracts. |
| Library records and selected JSON/JSON Lines projections | Complete Scala records; qualified selectors | Already wrong before projection; selectors cannot recover package segments | Fix source-derived identity once, before filtering/cache storage. |
| Class selection, count, stats and containment | Exact package-qualified class/member selectors; complete record relations | Zero selected matches; containment can still attach in-body members by spans | Correct links do not excuse wrong names; selectors must distinguish the packages. |
| Cache and post-lookup naming | Namespace-rooted versus standalone body-package copies | Wrong in-file name/rooting is extracted before cache lookup | Preserve explicit package context in stored names and invalidate old extraction results. |

Repair naming for body-bearing Scala package clauses without adding a new symbol kind or requiring package declarations to be extracted as records. Add public-result tests for standalone, sibling and nested bodies, bodies under stacked statement clauses, and the declaration after a body. Assert qualified selectors as well as identities and containment. Bump `EXTRACTOR_REVISION`, add a stale-cache regression, and update the current naming documentation alongside the repair.

### Medium — Shared-context lookups still rescan repeated names and package segments

**Defect class:** a per-file index retains growing candidate lists, then repeatedly scans those lists for individual declarations instead of indexing or memoizing the decision those declarations share.

The new [shared-context work test](../../lib/tests/l1/symbols_shared_context.rs:152) passes because its Rust types, C functions and Go owners have unique names, its Rust trait is singular, and its Scala packages are fixed. Those inputs miss the growing candidate lists. Repeated names in separate modules/namespaces and same-named overloads are ordinary valid source, not a malformed-input stress case.

Independent probes copied primary fixtures and appended N simple declarations/groups. `TreeFile::symbol_records()` and public `work_counters()` measured the declaration work; a second public `SourceFile::query` probe requested containment. All emitted records remain available. The table gives `shared_context_steps` at N=100 / 200 / 400; these are operation counts, not timing estimates.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust [same_file_trait_level](../../lib/src/file/extract/visibility/rust.rs:100) | N modules, each with public trait `Probe`, a uniquely named implementing struct and one impl method | **10,703 / 41,303 / 162,503**; every lookup scans all same-named trait levels | Aggregate the existing agreement/Unknown decision once per trait name; retain present visibility semantics. |
| Rust `SameFileType` in [resolve_container_kind](../../lib/src/file/tree_file/definitions.rs:669) | N modules, each `struct Probe; impl Probe { fn m(&self) {} }` | **20,303 / 80,503 / 320,903**; all same-named owners revisited for methods and receiver records | Index qualified owners or reuse the owner decision; retain qualified/absent/ambiguous-owner rules. |
| Lua `SameFileTable` in the same function | N successive `local Probe = {}` / `function Probe:m() end` pairs, inserted before the fixture's final return | **10,005 / 40,005 / 160,005** | Reuse the kind decision; preserve nearest-preceding table ownership. |
| C++ [has_earlier_static_declaration](../../lib/src/file/extract/visibility/c.rs:155) | N namespaces, each `static void probe(); void probe() {}` | **5,472 / 20,822 / 81,522**; namespace-distinct functions share one bare-name candidate list | Index complete function identity and earlier position. |
| Same C++ reader, out-of-body definitions | `namespace r9_i { static void probe(); } void r9_i::probe() {}` | Same **5,472 / 20,822 / 81,522** growth | Same identity rule, without walking every unrelated candidate. |
| Same C++ reader, overloads | N distinct parameter types `R9_i`; `static void probe(R9_i); void probe(R9_i) {}` | **5,372 / 20,622 / 81,122** | Include parameter/template identity in the index; preserve existing spelling/equivalence rules. |
| C++ [FunctionIdentity::of](../../lib/src/file/extract/visibility/c.rs:271), namespace lookup | N distinct namespace-qualified function names, so no repeated-name candidate scan | At N=200 the counter reports only **922**, but each qualified lookup also runs `namespaces.iter().any(...)` over the growing namespace list; those visits are uncounted. Public definitions retain ModulePrivate visibility. | Index namespace path/earliest declaration position; count these visits so tests can detect the scan. |
| Scala [extract_scala_visibility](../../lib/src/file/extract/visibility/scala.rs:9) | N sibling `package r9_i { object P { private[r9_i] def f() = 1 } }` blocks | **5,836 / 21,586 / 83,086**; membership lookup scans the growing segment vector | Use set membership for the existing package-segment decision. Qualified-name correctness is the separate finding above. |
| C linkage reader | N uniquely named static prototypes/definitions; repeated declarations of one function | **322 / 622 / 1,222** in both controls | Clean: C's first same-name match terminates the search. |
| Go `SameFileType`; local repeated types | Unique receiver owners; same-named local types in separate functions | **579 / 1,079 / 2,079**; **279 / 479 / 879** | Clean controls; no valid repeated package-level receiver-type case is claimed. |
| Namespace index and sibling container index | Java repeated nested classes; PHP repeated classes in successive namespaces; Perl successive packages; C# classes | **240 / 440 / 840**; **455 / 855 / 1,655**; **767 / 1,367 / 2,567**; **1,729 / 3,029 / 5,629** | Clean shared-context controls; retain the repaired indexes. |
| [find_container_symbol](../../lib/src/analysis/mod.rs:531), containment consumer | Same Rust/Lua repeated-owner copies through public containment queries | Rust methods link correctly, but the function enumerates the same bare-name candidate list again per child. These visits are also outside `shared_context_steps`. | Sweep this consumer with owner-kind resolution; preserve span-first and nearest-qualified-owner semantics. No separate timing figure is claimed. |
| Original namespace/root/module tests; C++ access sections; field-owner grouping; god-files callouts | Registered growing-file shapes and existing result regressions | Passing controls; indexed child state and per-owner groups are retained | Keep the repairs. Per-type field pairing and growing output-name lengths are not claimed to be the same shared lookup defect. |
| Filters, cache hits and report writers | 144 independent selector/report comparisons and registered warm-hit tests | Correct on their controls; hits avoid parsing, while cold analysis pays the remaining scans | Fix extraction/containment work, not report-specific paths or cache identity. |

For reproducibility, the failing additions are simple repetitions of these shapes, with `i` replaced by a fresh integer:

```text
Rust trait: mod r9_i { pub trait Probe { fn m(&self); } struct Si;
                      impl Probe for Si { fn m(&self) {} } }
Rust owner: mod r9_i { struct Probe; impl Probe { fn m(&self) {} } }
Lua table:  local Probe = {}; function Probe:m() end
C++ scope:  namespace r9_i { static void probe(); void probe() {} }
C++ types:  struct R9_i {}; static void probe(R9_i); void probe(R9_i) {}
Scala:      package r9_i { object P { private[r9_i] def f() = 1 } }
```

The temporary probe inputs/executables are under `/var/folders/l9/xdcp3xnn6s78_5l9w2_mnvtw0000gn/T/hug-r9-work-f_ly7uff`; the shapes above make the reproduction independent of those temporary artifacts.

Finish the shared-context repair across the listed decision sites. Extend the existing L1 work-count matrix with repeated names, namespace-qualified declarations, overloads and growing package segments, and include currently uncounted namespace/containment visits. Assert public identities, access levels and owner links alongside bounded work. The current schema documentation and skill API reference promise linear shared-context work; retain that promise only when these cases satisfy it. No timing assertions, new dependencies or performance CI cells are needed.

## Blocked Findings

None within this feature review. Both listed findings can be repaired with the existing syntax trees and dependencies. The previously reported native Zsh abort remains assigned to its separate grammar fix; this review neither authorizes a dependency replacement nor treats its investigation as a completed repair.

## Recurrence

Compared both classes with every earlier review in this directory.

- **Qualified identity:** review-1's “Owner identity is lost in qualified names and containment” established that enclosing source scopes must distinguish same-named owners; review-2 narrowed the explicit-owner matching rule. That sweep included Scala stacked statement packages but missed body-bearing `package_clause` nodes. This review includes standalone, sibling, nested and trailing-body controls, every corresponding supported namespace reader, and the selector/cache consumers.
- **Shared-context performance:** review-8's “Declaration metadata repeatedly searches shared file and module context” is only partially repaired. Its repair swept the newly indexed Rust trait, C/C++ linkage, Scala package and owner-kind readers, but tested distinct names or fixed package counts. It should also have swept repeated-name candidate decisions, namespace membership and the containment consumer. The complete list and clean controls are in the second finding.

The prior parameter, test-layout/reuse, documentation/marker and terminal-verification regressions passed. No recurrence in those classes was established. `recurrence: true` reflects the two classes above; the unchanged external Zsh issue is not presented as a newly discovered finding.

## Closure

Saved review #9, linked review-8 forward and left its requested `implemented: true` value in place, and set the spec's review count to nine. The feature remains incomplete. No implementation repair, test edit, formatting run, commit or lifecycle-directory move was made.
