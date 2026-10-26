---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: '2026-10-24T07:35:38-07:00'
spec: 2026-10-22-dmls-incremental-graph-updates/spec.md
implemented: false
description: 'A **feature** review of `2026-10-22-dmls-incremental-graph-updates/spec.md`'
feature: 2026-10-22-dmls-incremental-graph-updates/review-3.md
previous: 2026-10-22-dmls-incremental-graph-updates/review-2.md
---

# Review 3: DMLS incremental graph updates

The feature is **production ready** against the specification. R2-F1 is verified fixed, including every required instance in the repair ledger. No new findings, partial repairs, or missed siblings were found.

Opening an unchanged file previously loaded only as a referenced dependency now publishes its ordinary open-document role immediately. Wiki lookup, completion, graph content, and symbols reflect that change. Equal-text opens whose role is unchanged still avoid publication. Content edits continue to share path-derived state while rebuilding content-derived nodes, references, and reverse lookups.

## Previous Finding Verification

Read both earlier reviews, the original [implementation log](implementation-log.md), the repair [finding ledgers](log.md), and the implementation itself. The table checks the parent finding and every child row from the second repair ledger, including its additional test and documentation obligations. Passing tests below were executed in this review; repair-log test summaries alone are not treated as verification.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1 — unchanged dependency opening decision; ledger identifier, not a technical finding | Repair immediate role promotion, or explicitly change the contract to accept delayed eligibility | verified fixed | The technical concern is repaired under the existing spec's equivalence requirement for buffer opens and dependency transitions, as established by review 2. The repair ledger records choosing that implementation. No contract amendment or new architecture was needed. The checks for R2-F1 below verify the result. | No technical work. Review 1's historical human-ruling fields remain untouched; this review does not claim that an author ruling was recorded or require reaffirmation of the implemented contract. |
| R2-F1 — opening an unchanged dependency leaves the live graph in its old role | All fourteen child rows below | verified fixed | In dmls, [open_buffer](../../dmls/src/graph/invalidate.rs:195) captures dependency status before changing ownership; the equal-hash shortcut now also requires an unchanged role. The unit, fresh-build comparison, and LSP session tests all passed. | None |
| R2-F1.1 — role publication | Equal-text dependency opening updates the snapshot once | verified fixed | [Role-transition unit test](../../dmls/src/graph/invalidate.rs:1575) asserts `Reindexed`, generation +1, both mutable and published dependency flags cleared, and exactly one universe and one table build. | None |
| R2-F1.2 — wiki eligibility | Newly opened dependency resolves as a wiki candidate | verified fixed | [Dependency-transition test](../../dmls/tests/l1/incremental_graph_equivalence.rs:689) checks unresolved before opening and the correct root afterward, against an independently constructed fresh universe and graph. The LSP test verifies definition lookup. | None |
| R2-F1.3 — Markdown and wiki content | Assemble both node kinds and their reference edges | verified fixed | The transition fixture contains both link kinds; assertions require their nodes, verify wiki resolution, and compare payloads, spans, forward edges, and reverse edges with the fresh build. | None |
| R2-F1.4 — ordinary frontmatter and expression content | Assemble frontmatter keys, schema uses, file uses, and variable uses | verified fixed | The same fixture contains a title, schema path, stylesheet path, and interpolation. All four node kinds are required; the full graph comparison checks their payloads and resolved edges. | None |
| R2-F1.5 — basename eligibility | Include the opened document in the basename lookup | verified fixed | The transition test requires the `dep` bucket to contain the opened path and compares basename membership and counts with the fresh graph. | None |
| R2-F1.6 — wiki and Markdown path completion | Both provider surfaces offer the opened document | verified fixed | The transition test invokes the public provider registry at both partial references, requires `dep` suggestions, and compares complete answers with the fresh graph. The LSP session separately requires `report` wiki completion. | None |
| R2-F1.7 — frontmatter file completion | The dependency filter must admit the opened document | verified fixed | Code invariant: dmls's [file_path_completions](../../dmls/src/providers/frontmatter.rs:888) filters only on the published record's dependency flag before applying path/prefix checks. R2-F1.1 proves that flag is false immediately after opening. No independent ownership cache or second dependency classification exists in this provider. This row is verified by code trace, not a claimed schema-completion execution. | None |
| R2-F1.8 — graph symbols | Include the opened document in graph-symbol projection | verified fixed | The transition test compares `graph_symbol_documents` against the fresh graph and explicitly requires the opened path. | None |
| R2-F1.9 — portability grouping | Group ordinary documents according to their published role | verified fixed | The transition comparison checks portability twins after promotion. The [grouping implementation](../../dmls/src/graph/arena.rs:1340) reads the rebuilt table's dependency flag. Executed logical case-pair and request-graph controls verify collision grouping and exclusions; no filesystem case-twin assumption is needed. | None |
| R2-F1.10 — dependency content and retained readers | Preserve dependency headings/transclusions and immutable old snapshots | verified fixed | Fresh-build comparison includes dependency headings and transclusions before promotion. Executed retained-snapshot tests hold old snapshots across edits and membership changes and recheck headings, wiki outcomes, basename buckets, and collision twins. | None |
| R2-F1.11 — sibling update entry points | Preserve changed-text opens, listed opens/closes, dependency/listed transitions, stale rereads, and equal-text no-ops | verified fixed | The role-transition unit test covers a stale dependency and equal-text controls for already-open and listed parsed files. Executed invalidation and equivalence tests cover the remaining transitions. Code inspection confirms lazy equal-hash rereads do not change ownership; listing, close, deletion, and context paths retain their rebuild behavior. | None |
| R2-F1.12 — permanent equivalence regression test | Remove the expected stale/fresh disagreement at unchanged opening | verified fixed | The transition test now calls the full equivalence check at the unchanged opening, requires one publication and one universe/table build, then tests unchanged reopening, changed text, closing, rereading, joining, and leaving the listing. | None |
| R2-F1.13 — normal server dispatch | Opening a loaded ignored target updates wiki answers immediately | verified fixed | [LSP session test](../../dmls/tests/l1/workspace_file_list.rs:718) loads ignored `generated/report.md` through a fragment reference, observes missing wiki definition/completion, sends `didOpen` with disk text, then requires one universe/table build, resolved definition, offered completion, and cleared wiki diagnostics. | None |
| R2-F1.14 — publication documentation | Explain the role-changing exception to equal-text no-publication behavior | verified fixed | Reviewed invalidation module and public API comments, [architecture](../../docs/lsp/architecture.md:59), [indexing](../../docs/lsp/indexing.md:600), and the darkmatter skill's DMLS guidance. They describe immediate dependency promotion and preserve the ordinary equal-text shortcut. | None |

The original pending scope question does not constitute an accepted exception to the spec. Its underlying behavior is now repaired without changing the specified design, so no additional human decision is required for readiness.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None. R2-F1's defect class is repaired; no unchanged instance, repair regression, or missed sibling was found. Review 1 contained no technical findings.

## Specification and verification coverage

| Requirement | Strongest checked verification | Assessment |
| --- | --- | --- |
| Reuse path-derived wiki state and document tables on content edits; no repeated path/key work | Level 1: work counters and shared-object identity checks in graph invalidation tests, including edits with a dependency and a retained reader | Passed; content-derived data is rebuilt separately |
| Match a fresh build across edits, opens/closes, listing changes, dependencies, roots, contexts, and awaited creations | Level 1: thirteen transition/corpus tests in `incremental_graph_equivalence`, with independent universe construction and normalized paths, spans, payloads, edges, lookups, and provider answers | Passed, including the previously exempted unchanged dependency opening |
| Preserve complete wiki ambiguity and request-graph collision scope | Level 1: request graphs compared over live and fresh full candidate universes, plus a deliberately incomplete-universe negative control and graph collision tests | Passed; candidates outside a request graph still affect ambiguity |
| Preserve snapshot isolation, dense document/root numbering, and per-document content order | Level 1: retained-snapshot and root-numbering assertions; graph comparison normalizes global IDs while preserving document-local order | Passed; no cross-snapshot identity guarantee is introduced |
| Preserve navigation, hover, diagnostics, completion, references, rename, and symbols | Level 1: corpus provider/oracle comparisons, graph-symbol checks, and normal LSP notification/request sessions | Passed; this graph-only work requires no terminal rendering or physical keyboard evidence |
| Rebuild structural changes once; ignore irrelevant creations; refresh affected diagnostics | Level 1: batched work/publication counters and LSP workspace-file-list sessions | Passed; publication counts are checked in-process where generation is observable |
| Bounded performance measurement and cheaper-rebuild decision | Inspected the original measurement, post-change record, driver, and stated limitations | The record reports 0.78% of one core and 2.61 ms median rebuild wall time. These remain one-host decision aids. The reconstructed replay and unrecorded final parsed/dependency counts are disclosed; no stronger performance guarantee is inferred. No additional timing study was run. |

The cache boundaries match the design: universe reuse compares sorted candidate paths, dependencies, and ordered roots; table reuse additionally checks listing state, document paths, dependency flags, and universe identity. Content hashes and parsed flags remain snapshot-local. Content-derived references and awaited paths are resolved anew through captured document contexts. The documented distinction between the cached wiki basename lookup and content-derived frontmatter-key lookup is consistent with the implementation log's design clarification.

The Input Robustness Matrix does not apply: the changed code assembles graph snapshots and decides cache reuse; it does not add or change a file-format or configuration loader.

## Executed checks and limits

- Root `just test dmls` failed before test execution because the recipe generated an empty `--package` argument. The same runner failure was recorded in review 2. Used nextest directly without changing unrelated runner code.
- `cargo nextest run --color=never -p dmls --features effects-instrumentation -E 'not (test(/(^|::)(level2_|level3_|browser_|real_|slow_)/))'`: **1,301 passed, 6 skipped**, 6.849 seconds total. The repaired transition's unit, equivalence, and LSP tests executed successfully; none relies on a skipped test.
- `just check-tier-coverage darkmatter`: **0 stranded tests**. Both integration modules are declared in `tests/l1/main.rs`, compiled by the explicit `l1` target with `autotests = false`, and selected by L1. The repair adds no feature-gated test requirement.

This review changes review documents and requested metadata only. It did not rerun library/CLI suites or lint; the repair log's broader gate results were read as claims, not presented as this review's execution. No new implementation changes required additional gates. Cross-OS evidence remains CI's responsibility and is not a readiness blocker. Lifecycle movement remains with the author.
