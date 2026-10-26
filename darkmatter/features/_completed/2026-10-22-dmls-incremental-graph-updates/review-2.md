---
$schema: feature-review.yaml
ready: false
findings:
    - 'R2-F1 (high): Opening an unchanged dependency leaves the live graph in its old role'
human_review: true
human_review_items:
    - |-
        The previous review's scope choice remains unanswered: should opening an unchanged, reference-only Markdown file make it an ordinary open document immediately?

        This review independently confirms incorrect wiki matching, missing completion suggestions, and omitted graph content until another rebuild. Repair is recommended under the current specification; no additional design decision is needed to implement that repair.

        - Repair the transition: rebuild once when opening changes the file's role, retaining the shortcut for unchanged text whose role stays the same.
        - Retain the delay: explicitly amend the equivalence requirement to accept this behavior and document the limitation. No such acceptance is currently recorded.

        This carries the existing unanswered choice forward; it does not request reaffirmation of an accepted decision. The finding is actionable against the current specification and is not blocked on this optional scope amendment.
    - 'Implementation cycle 2 chose the repair option: an equal-text open of a dependency now rebuilds once and publishes it as an ordinary document (confirm, or amend the spec to accept the delay instead).'
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: '2026-10-24T07:19:54-07:00'
spec: 2026-10-22-dmls-incremental-graph-updates/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/features/2026-10-22-dmls-incremental-graph-updates/log.md
description: 'A **feature** review of `2026-10-22-dmls-incremental-graph-updates/spec.md`'
feature: 2026-10-22-dmls-incremental-graph-updates/review-2.md
previous: 2026-10-22-dmls-incremental-graph-updates/review-1.md
next: 2026-10-22-dmls-incremental-graph-updates/review-3.md
---

# Review 2: DMLS incremental graph updates

The feature is **not production ready**. The cache implementation and the tested content-edit path satisfy the intended reuse boundaries, but opening an unchanged dependency violates the specified equivalence with a fresh graph. One high-priority finding covers that transition, its affected consumers, and the test that currently expects the discrepancy.

## Previous Finding Verification

Read [review-1.md](review-1.md), the repair log's `### Finding Ledger` in [log.md](log.md), and the original [implementation log](implementation-log.md). The previous review contains **zero technical findings**; its `implemented: true` records completion of that empty inventory, not a repaired graph.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1 — unchanged dependency opening decision (ledger ID; explicitly not a technical finding) | One pending choice: repair immediate role promotion or explicitly accept delayed wiki eligibility | unresolved | `review-1.md` still has `human_resolved: false` and “Human rulings: Pending.” The repair ledger says no code changed. In dmls, `WorkspaceIndex::open_buffer` still inserts into the open set before calling the hash-short-circuiting `set_document`. The existing dependency-transition test and this review's public-API reproductions confirm the mismatch. | No ruling has been recorded. The independently verified defect is now R2-F1; it can be repaired under the current specification without choosing an unrequested architecture. |

There is no partial repair or newly discovered sibling of a previous technical finding. The previous review explicitly did not execute a technical review. This review does not treat its unanswered question as an accepted exception to the specification.

## Unblocked Findings

### R2-F1 (high): Opening an unchanged dependency leaves the live graph in its old role

**Authority — spec and defect.** Phase 2 Acceptance requires that, for “buffer opens and closes” and “dependency transitions,” the cached snapshot “equals a fresh full build over the same inputs.” In dmls, [WorkspaceIndex::open_buffer](../../dmls/src/graph/invalidate.rs:200) changes ownership before [set_document](../../dmls/src/graph/invalidate.rs:184) returns `Unchanged` on equal bytes. The live graph therefore still excludes the now-open document from ordinary analysis. This is an existing defect, not a regression attributed to caching; the stated review scope permits reporting it.

**Defect class:** a content-only equality check suppresses publication after non-content document ownership changes.

A realistic input is an ignored `generated/report.md` referenced by an open `guide.md`. Navigation loads the report's headings as a dependency. Opening the report with its on-disk text changes its ownership, but its published record remains a dependency until an unrelated edit or rebuild. Wiki links cannot find it, completion omits it, and its own ordinary references are missing from the graph.

#### Reproduction and bounded class sweep

Used the existing temporary-repository fixture and transition harness in dmls's [incremental_graph_equivalence.rs](../../dmls/tests/l1/incremental_graph_equivalence.rs:312), exercising public `WorkspaceIndex` and `WorkspaceGraph` APIs. Listed only `source.md`; created an unlisted `dep.md`; loaded it with `ensure_parsed`; opened it with identical text; compared the live snapshot with a fresh independently constructed universe and graph. The dependency contained a heading, Markdown link, wiki link, frontmatter key, `$schema` path, stylesheet path, interpolation, and transclusion. A second reproduction queried the public provider registry for wiki and Markdown path completion and called the graph-symbol projection.

The temporary probes asserted the observed discrepancy, then verified that `reresolve()` restores full equivalence and that a subsequent unchanged open remains equivalent. Both probes passed; their temporary test additions were removed. The permanent [dependency transition test](../../dmls/tests/l1/incremental_graph_equivalence.rs:695) already asserts the stale/fresh disagreement explicitly at lines 709–717, then edits the text before resuming equivalence checks. Thus the suite passes while this required transition is unequal.

The following are instances of one finding, not separate requested mechanisms. “Code trace” identifies consumers checked by following the reproduced stale dependency flag; it does not claim an additional provider execution.

| Child ID | Site in dmls | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | `open_buffer` → `set_document`, graph record | Parsed unlisted dependency opened with identical bytes | Mutable index says non-dependency; published graph says dependency; generation unchanged | Publish one graph reflecting the new ownership |
| R2-F1.2 | `wiki_universe`, `resolve_wiki_link` | Source has `[[dep]]` when dependency opens | Live resolution remains `Unresolved`; fresh graph resolves to the dependency's root | Same resolved target in both graphs |
| R2-F1.3 | `WorkspaceGraph::assemble`: Markdown and wiki node loops | Opened text contains `[x](source.md)` and `[[source]]` | Both node kinds remain absent; fresh graph contains them | Assemble both kinds and their reference edges |
| R2-F1.4 | `WorkspaceGraph::assemble`: dependency `continue` before frontmatter keys, schema uses, file uses, variable uses | Opened text contains `title`, `$schema: ./schema.yaml`, `style.page.stylesheet: ./style.css`, and `{{ title }}` | All four corresponding node kinds absent from live graph and present in fresh graph | Assemble all four and resolve their edges |
| R2-F1.5 | `DocumentTable::build` basename index | Same unchanged open, basename `dep` | Live basename bucket excludes the file; fresh bucket includes it | Rebuild path-derived eligibility on ownership transition |
| R2-F1.6 | `providers::wiki::file_completions` and `providers::completion::path_completions` | Source has `[[de]]` and `[path](de)` | Public registry omits `dep` in live answers and includes it with fresh graph | Offer the newly opened file on both surfaces |
| R2-F1.7 | `providers::frontmatter::file_path_completions` | Same reproduced dependency flag; code trace at [frontmatter.rs](../../dmls/src/providers/frontmatter.rs:894) | The explicit dependency filter also excludes this path from schema-driven file suggestions | Newly opened document eligible for file suggestions |
| R2-F1.8 | `providers::symbols::graph_symbol_documents` | Same open, containing `# Dep` | Public graph projection omits the file; fresh projection includes it | Equal graph-symbol projection; this does not claim that the separate persistent-store symbol request loses its row |
| R2-F1.9 | `portability_collisions` | Same open without a case twin; inspected dependency filter at [arena.rs](../../dmls/src/graph/arena.rs:1340) | Clean for this fixture: both report no collision. The stale flag also governs exclusion from grouping | Keep collision grouping consistent with the published role; existing logical-case-pair controls must stay green |
| R2-F1.10 | Heading/transclusion assembly and retained snapshot | Same dependency text and explicit subsequent rebuild | Clean: headings and transclusions are intentionally assembled for dependencies; the retained snapshot remains immutable | Preserve this dependency behavior and old-reader isolation |
| R2-F1.11 | Other ownership/update entry points | Listed opens/closes; changed-text dependency open; dependency-to-listed; unlisted unsaved open/close; stale disk reread | Clean in existing executed transition and work-count tests: these reach rebuild when required. Already-open equal-text updates remain a no-op | Preserve these controls while repairing only role-changing equal-text opens |

All graph construction decisions consuming dependency status were checked: universe candidate inclusion, path record eligibility, basename and collision grouping, ordinary-content suppression, and the provider filters above. The cache reuse check itself compares dependency sets correctly; the faulty open never reaches it. Reference, rename, and diagnostic consumers of the omitted nodes inherit that missing content and need no separate invalidation machinery.

**Smallest repair:** capture the old dependency role before adding the path to the open set. When equal text would otherwise return `Unchanged`, rebuild once if that role changed. Preserve the equal-text shortcut for an already-open document and for a listed closed parse becoming open, where graph eligibility is unchanged. Replace the permanent test's expected disagreement with a full equivalence check at the unchanged opening. Extend the normal LSP notification/request test to cover opening an already-loaded ignored target and verify its wiki eligibility immediately. Keep the graph work at one universe and one table rebuild for that transition. Review the invalidation module's hash-only publication comments alongside the change.

## Blocked Findings

None. R1-H1 remains an unanswered scope question, not an accepted decision or a blocker to repairing R2-F1 against the current specification. If the author explicitly accepts the limitation instead, that must change the contract; this review does not invent that acceptance.

## Recurrence

`recurrence: false`: review 1 has no technical findings or previous defect-class sweep. R2-F1 independently verifies the same behavior raised there as a pending scope question. It is neither a repair regression nor a newly discovered sibling, and no prior technical finding is claimed fixed.

## Verification and coverage

| Requirement | Strongest checked level and evidence | Assessment |
| --- | --- | --- |
| Content-only reuse; zero unchanged-document path/key work; structural changes rebuild once | Level 1: graph invalidation work-count tests and table/universe identity assertions | Appropriate for in-memory graph operations; passed |
| Equivalence across edits, listed membership, roots, contexts, lazy parsing, dependency transitions | Level 1: 13 `incremental_graph_equivalence` tests, including independent universes and provider oracle | Appropriate level; unchanged dependency opening is explicitly exempted by the test and fails the contract (R2-F1) |
| Immutable old readers; dense roots and per-document order; request graph ambiguity and collision scope | Level 1: arena/invalidation tests, retained snapshots, request-graph controls | Passed; no cross-snapshot node identity is introduced |
| Notifications, dependent diagnostics, navigation, hover, completion, watched batches, awaited creations, ignored build output | Level 1: in-process framed LSP sessions in `workspace_file_list`, through normal notification/request dispatch | Appropriate: graph/LSP behavior makes no terminal display or keyboard-encoding promise; passed |
| Performance decision and release re-measurement | Reviewed recorded measurements and driver; no timing threshold in tests | Record reports 0.78% of one core and 2.61 ms median wall time. Replay is reconstructed, and parsed/dependency counts are explicitly unrecorded; treat it as the bounded decision aid specified, not a statistical guarantee |

Executed:

- Root `just test dmls` failed before running tests: the recipe generated an empty `--package` argument. Used nextest directly as a non-interactive fallback; did not change unrelated runner code.
- `cargo nextest run --color=never -p dmls --features effects-instrumentation -E 'not (test(/(^|::)(level2_|level3_|browser_|real_|slow_)/))'`: **1,299 passed, 6 skipped**. One unchanged schema-discovery test was reported slow; the changed equivalence tests took at most 0.205 s and the three added LSP session tests at most 0.131 s in this run. A passing assertion run does not erase R2-F1.
- Two temporary public-API reproduction tests: **2 passed**, asserting the stale answers and fresh-build controls described above. An initial probe assumption about the basename result shape was corrected: an excluded file has an empty bucket, not a missing basename. No production change was made.
- `just check-tier-coverage darkmatter`: **0 stranded tests**. The new modules are declared in `tests/l1/main.rs`, compiled by the `l1` target with `autotests = false`, and selected by L1. New tests require no additional feature beyond the features exercised above.

The Input Robustness Matrix does not apply: this work changes graph assembly and cache reuse, not a file-format/configuration loader. No terminal/browser tests are required. Cross-OS proof remains CI's responsibility and is not a readiness finding. This review did not rerun the area's full library/CLI suite or lint because it makes no implementation changes; implementation-log gate summaries were read as claims, not independent results of this review.
