---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-05T18:29:23-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: false
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-10.md
previous: 2026-10-03-symbol-filters-and-reports/review-9.md
---

The symbol filters and reports feature is **production ready within its specified scope**. Both review-9 defects are repaired. The sweeps below found no additional implementation defect, missing report requirement, or inappropriate verification level. No human design decision is needed.

This assessment does not certify the concurrent type-relations implementation or repair the previously tracked native Zsh grammar abort. The workspace changed during this review; observed test failures and their dispositions are recorded below rather than presented as an uninterrupted passing run.

## Previous review follow-through

Review-9 had two unblocked findings and no blocked findings. Reviews 1 through 8 also contain no blocked findings, so there is no newly unblocked work to reconcile.

| Previous finding | Verification | Assessment |
|---|---|---|
| Scala block packages disappear from qualified names and selectors | The tree-hugger library now recognizes body-bearing Scala package clauses as enclosing name segments. Registered tests cover standalone, dotted, sibling and nested bodies, preceding statement packages, declarations after a body, qualified selectors and package-qualified access. Nine independent shipped-CLI selectors over an extended copy of the primary Scala fixture each return one match. | Implemented. |
| Shared-context lookups still rescan repeated names and package segments | Trait-level agreement, C/C++ linkage identities, namespace positions, Scala package membership, owner kinds, containment and export targets use shared indexes. The four registered shared-context tests pass, including repeated-name and growing-package shapes, public access/name/owner results and work-count bounds. | Implemented. The concurrent relations implementation replaces the old trait-ID lookup with a per-file declaration index; its updated trait-link expectation also passes. |

The Scala repair bumped the extractor revision and added the previous-revision cache rejection test. Later revision changes belong to concurrent metadata work; old-revision cache tests pass in the CLI suite.

## Scope and verification

Used the tree-hugger, rust-testing and biscuit-test-harness skills. Read the specification, earlier findings and repair records; inspected source-derived metadata, shared predicates, query finishing/reuse, cache validation/projection, owner indexes, class selection, report writers/renderers, current documentation and test registration. This review changes only review documents and specification frontmatter.

### Repair-class sweeps

| Decision site | Shapes checked | Result |
|---|---|---|
| Scala package naming and access | Statement, stacked statement, standalone/dotted block, sibling/nested blocks, block beneath statement packages, declaration after a block, qualified private access | Names preserve package segments and scope boundaries; qualified selectors distinguish siblings. |
| Other block namespace readers | Rust inline modules, TypeScript namespaces, C++ namespaces, C# block namespaces, PHP bracketed namespaces, Perl package blocks | Registered sibling/nesting controls pass. |
| Statement namespace readers | Java and Go packages, C# file-scoped namespaces, Scala stacking, PHP and Perl replacement statements | Language matrices and namespace-position controls pass. |
| Rust trait visibility and owner-kind lookup | Same trait/type name repeated across modules, unique names, differing owner identities | Indexed decisions preserve the tested visibility and ownership results. |
| Lua table owner lookup | Repeated local table declarations and methods, nearest preceding owner, external qualified owner | Owner/containment controls pass. |
| C/C++ linkage and namespace lookup | Unique C functions, repeated prototypes, namespace-distinct C++ functions, out-of-body definitions, overloads, distinct qualified names | Public visibility checks and bounded-work regressions pass. |
| Scala package-segment lookup | Growing sibling block packages and qualified access | Set membership preserves Package access without growing per-declaration scans. |
| Containment and export consumers | Repeated Rust/Lua owners, exported functions with repeated names, full-scan comparison controls | Owner IDs and export targets agree with their controls; containment work remains bounded. |
| Query/cache/report consumers | Qualified names, container selectors, class grouping, count/stats, cached projections and alternate scan roots | Registered consumer tests and independent report comparisons pass. |

An independent shipped-CLI owner sweep also checked the current 13 owner-matrix rows: Rust direct/rooted/qualified owners, Swift body and extension owners, Scala, PHP, C++, C#, Java, Python, TypeScript and Go. All 29 expected parent links matched their owner IDs.

### Requirement-to-verification assessment

| User-facing requirement | Required level | Verification present |
|---|---|---|
| Scope, async, qualified names, containment and accepted kind limits | Level 1 | Per-language extraction matrices, namespace/owner regressions, coverage guards and public CLI probes. |
| Test/deprecation flags, attached documentation and parameter metrics | Level 1 | Source-shape matrices, earlier marker/doc regressions, test-file fixtures and package-layout/cache/reuse transitions. |
| Smart case, exact/glob matching, file/name/container/kind/scope/visibility selectors, signature predicates and imports | Level 1 | Public-query selection matrices and isolated CLI subprocess assertions, including non-ASCII, intersections, aliases and argument conflicts. |
| JSON/JSON Lines envelopes, ordering, count/stats, class members, empty results and failure handling | Level 1 | Structural assertions, per-view goldens, partial-output/broken-pipe checks and independent report comparisons. |
| Cache equivalence, invalidation, narrowing and post-lookup naming | Level 1 | Cold/warm/projection tests, malformed-entry matrix, stale revisions, scan-root and package-boundary transitions. |
| Styled lists/classes, outlines, stats tables, full/short/absent signatures, hyperlinks and terminal-width wrapping | Level 2 | Real-terminal declaration captures executed in tmux and WezTerm. Kitty is not counted as executed terminal evidence. |
| Plain/redirected text, literal control-sequence handling and injected widths | Level 1 | Text/byte assertions and goldens. |
| Shared-context work grows with declarations | Level 1 | Flat-file and repeated-name work-count tests, including the containment consumer; no timing thresholds. |
| Keyboard, mouse, paste or input-method behavior | Not applicable | This feature introduces no interactive input handling; Level 3 is unnecessary. |

No verification-level mismatch was found. Cross-OS proof remains a CI responsibility and does not determine readiness here.

The library's consolidated test target declares the extraction, namespace and shared-context modules. The CLI symbols target declares its selector/report/cache modules. Both real-terminal targets require `terminal-tests`, enabled by the live Level-2 recipe and package CI metadata. Tier coverage reports zero stranded tests. No test was added or renamed by this review.

### Input robustness matrix

The relevant structured reader is the persistent JSON cache. Its [documented field-by-shape contract](../../docs/cache-design.md) and [public-output matrix](../../cli/tests/symbols/cache_robustness.rs) cover entry identity/coverage and record scope, qualified names, namespace rooting, modifiers, flags, metrics-facet presence and nullable metric values. Control, absent, explicit null, wrong whole-field type, one/all wrong elements, empty, duplicate-key and trailing-invalid shapes are represented, with inapplicable cells explicit.

The CLI matrix passes. Its stored sentinel distinguishes serving an edited entry from recomputing it. Required nullable metric keys distinguish missing fields from unsupported values; entry fields have no permissive defaults. Accepted record defaults follow the documented schema. Rust package-test detection checks manifest existence and does not add a manifest-content parser. No additional format reader was introduced by the repair.

### Independent report checks

Copied each primary language fixture into an isolated Git repository with test-owned home, configuration and cache directories. For each of the 16 languages, compared eight selections: all records, async, deprecated, test-only, undocumented, minimum one parameter, member scope and Public visibility. All **128 comparisons** agree on JSON Lines record IDs, count, each of the three stats maps and the documentation denominator against independently filtered complete records. These checks prove report wiring; the registered language matrices supply the expected extraction facts.

The extended Scala fixture independently passes exact function-name, function-container and class-container count selectors for both sibling packages and the nested package: **nine checks**. A further Rust probe confirms repeated same-named traits resolve to their own module's trait IDs under the concurrent relations implementation.

### Test execution and limits

| Check | Result |
|---|---|
| Area `just test -j 2 --max-fail 10` | **1,056 passed, zero failed, zero skipped.** This final run completed the whole selected Level-1 suite; it did not stop at the first failed assertion. |
| Area `just test -j 2 symbols_shared_context` after the assertion update | Four passed; includes all flat-file, repeated-name, namespace-position and containment work checks. |
| CLI Level-1 suite | Final separate run: 293 passed, zero skipped. An earlier run passed 292 and timed out on the older live-repository prelude test; the subsequent complete CLI run and final area run passed it. |
| Area `just test-l2` | Six entry points passed. tmux and WezTerm exercised declaration and god-files captures; Kitty is not counted as executed terminal evidence. 292 tests were outside the Level-2 selection. |
| Root `just check-tier-coverage tree-hugger` | Zero stranded tests. |
| `cargo build -p tree-hugger-cli --color=never --offline` | Passed; shipped CLI used for independent reproductions. |
| Area `just lint` | Failed on two conditional-expression lints in concurrent relations work; details below. |

Runs used nextest's reported `default` profile. The workspace was live throughout, including updates to the relations tests between runs; these results should not be read as evidence for one frozen source snapshot.

During earlier runs, the area suite stopped on a Rust external-owner assertion while concurrent relations changes were landing. After rebuilding, that assertion passed. A later selected run encountered another old external-owner expectation; it was updated by the concurrent implementation to allow a textual unresolved container without assigning a local owner ID. The repeated-trait performance test likewise initially expected an unresolved trait ID; its revised expectation checks the module's actual trait ID and all four shared-context tests now pass. These were observable changes during this review, not repairs made by the reviewer.

The imported-self-type probe initially appeared to expose an ownership regression. Inspection of the separate type-relations specification established an intentional contract change: imported self types remain unresolved even when their final segment matches a same-file declaration. That behavior is outside this feature's original relation-resolution work and is not reported as a new owner-identity defect. Direct, rooted and qualified owner controls continue to pass.

`just lint` failed on two collapsible conditional expressions in the concurrently added tree-hugger relations readers, [source path parsing](../../lib/src/file/extract/relations.rs:121) and [Rust impl traversal](../../lib/src/file/tree_file/relations.rs:376). These are outside this specification's implementation. The recipe stopped before CLI lint; this review does not claim a clean area lint run.

The previously reported native Zsh scanner abort remains assigned to the separate `zsh-scanner-state-abort` fix. Its dependency is still 0.56.0. This review did not repeat the abort reproduction and does not imply that the grammar has been repaired.

## Unblocked Findings

None within this feature's scope.

## Blocked Findings

None. No human decision or foreground input test is required for this feature.

## Recurrence

Compared the reviewed classes with all nine earlier reviews. Earlier source-owner, parameter, marker/doc, test-layout/reuse, terminal-verification and shared-context defects have passing targeted controls. The current Scala body-package and repeated-name cases are repaired; no recurring defect is established. The explicit imported-owner change belongs to the concurrent relations contract, and the pre-existing Zsh abort remains separately tracked. Therefore `recurrence` is false.

## Closure

Saved review 10, linked review-9 forward and set its requested `implemented` value to true. Set the specification's review count to 10 and `completed` to true. No implementation repair, formatting run, commit or lifecycle-directory move was made.
