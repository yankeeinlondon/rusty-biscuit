---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-07T21:58:30-07:00"
spec: 2026-10-09-changed-files-filter/spec.md
implemented: false
description: "A **feature** review of `2026-10-09-changed-files-filter/spec.md`"
feature: 2026-10-09-changed-files-filter/review-4.md
previous: 2026-10-09-changed-files-filter/review-3.md
---

# Changed Files Filter — Review 4

The feature is **ready for production**. Every required instance and consumer
from the earlier reviews is verified fixed. The truncated-index repair now
returns the required errors through the public library and all four declaration
commands, while preserving valid absent/empty indexes and metadata queries.
No new blocking finding, unresolved repair or human decision remains.

I read the specification, plan, initial implementation log, all three earlier
reviews and the complete repair ledger in `log.md`. Review 1's single legacy
`Findings` section, review 2's carried R1-F2 parent and R2-F1 sibling, and
review 3's R3-F1 inventory are all reconciled below. The repair ledger adds
committed-diff sites and consumers to R3-F1; those additions are checked too.
Completion flags were treated as claims, not verification evidence.

## Previous Finding Verification

The following evidence was independently executed on this macOS host:

- **CLI:** in tree-hugger, `just test --test changed --test-threads 4`:
  **110 passed, zero skipped**, nextest `6b63ff0c-b49f-4b9f-8c67-3624742a8273`.
  This includes every permanent ownership, type-change, corruption, selection,
  report, parser, help and cache regression in the declared changed target.
- **API:** in sniff, `just test changed_paths --test-threads 4`:
  **35 passed**, nextest `eae6fbcc-88b1-4c90-93ee-0687680db1c6`.
  The 3,154 other tests were filtered out, not executed. This includes the
  registered public-reader corruption matrices and status phase/cost controls.
- **Context:** in sniff, `just test status_context_rejects --test-threads 4`:
  **one passed**, nextest `c5056b21-1c5c-41fa-ab1a-ff7d91360718`.
  This checks the rich reader independently of the preceding status walk.
- **Ownership errors:** in tree-hugger,
  `just test --bin hug changed::tests --test-threads 4`: **six passed**,
  nextest `ffc742f7-47f8-441a-b1cd-6e38b307fa0d`.
- **Additional prefix sweep:** temporary assertions appended to the existing
  declared CLI module, `just test --test changed review4_all_prefixes --test-threads 1`:
  **one passed**, nextest `1447b41f-7f3b-4d61-9a8c-615fb496f225`.
  For libgit2-written indexes requested as versions 2, 3 and 4, every shorter
  byte prefix was rejected by each of the four commands with exit 1, empty
  stdout and repository/index context. Each unedited index first produced
  independent counts 1/1/2/1 for current on-disk declarations. The temporary
  test was removed afterward, restoring the test file's original content.

Each child row keeps its historical ID, including the extra rows introduced
by the repair ledger. “Verified fixed” on a preservation row means its required
positive or error contract was checked; it does not claim that a previously
clean site needed a behavior change. CLI and API references in the historical
table below refer to this review's runs, not the earlier review's results.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
| --- | --- | --- | --- | --- |
| R1-F1 — Unstaged file-type changes disappear from changed-file selection and status | Both replacement directions, all status readers/projections, staged/committed controls, four commands and report/cache consumers | verified fixed | API public phase matrix and CLI `type_changes` matrix passed; all worktree readers use the shared classifier | None |
| R1-F1.S1 | Path collector → `GitRepo::changed_paths` | verified fixed | API `file_type_replacements_agree_across_status_readers_and_commit_phases`: exactly `leaf.rs` in both unstaged directions | None |
| R1-F1.S2 | Rich collector → `file_changes` / `repo_status` | verified fixed | Same API matrix: one Modified/Modified file, dirty true, unstaged count 1 | None |
| R1-F1.S3 | Fast dirty reader → minimal/summary requests | verified fixed | Same matrix checks direct dirty result and both public projections; classifier use retains early stopping | None |
| R1-F1.S4 | Detailed counts → total counts and count-only requests | verified fixed | Same matrix checks `(true,0,1,0)`, total `(true,1)`, and request category counts | None |
| R1-F1.S5 | Staged classification, both directions | verified fixed | API exact path and staged count 1; CLI exact staged counts for every command | None |
| R1-F1.S6 | Committed tip comparison, both directions | verified fixed | API base comparison selects the path and current status is clean; CLI base counts positive, omitted-base counts zero | None |
| R1-F1.S7 | Conflict detection before selection | verified fixed | API `index_conflicts_anywhere_fail_naming_every_path` and CLI `conflicts_outside_the_package_still_fail` passed | None |
| R1-F1.C1 | `functions` report and cache consumers | verified fixed | CLI type matrix asserts current function, count 1, one file/index, structured/text modes, uncached facts and actual warm hits | None |
| R1-F1.C2 | `types` report and cache consumers | verified fixed | Same matrix asserts current type, count 1, report modes and warm hits | None |
| R1-F1.C3 | `symbols` report and cache consumers | verified fixed | Same matrix asserts both current declarations, count 2 and every supported mode | None |
| R1-F1.C4 | `classes` report and cache consumers | verified fixed | Same matrix asserts current type, count 1, outline and structured reports; full indexes retain both declarations | None |
| R1-F2 — Symlink identities admit files from another repository | Original boundaries, every union bucket, identity matching, four consumers, and recurring parent-component sibling R2-F1 | verified fixed | All original CLI ownership matrices and all R2-F1 regressions passed; current ownership code still checks every visited component before parent removal | None |
| R1-F2.S1 | External aliases; alias and target selectors | verified fixed | `external_{untracked,staged,committed}_aliases_are_rejected_in_every_selector_and_report`: empty reports and zero source/cache work | None |
| R1-F2.S2 | Nested `.git` directory and linked-checkout `.git` file | verified fixed | Six nested boundary/bucket matrices passed with both selectors and every report mode | None |
| R1-F2.S3 | Legitimate same-checkout aliases | verified fixed | `same_checkout_aliases_preserve_expected_declarations`: exact `clean`/`Clean`, counts 1/1/2/1, uncached facts and actual warm hits | None |
| R1-F2.S4 | Explicit foreign file without an alias | verified fixed | `an_explicit_file_in_another_repository_is_not_selected`: empty files/errors | None |
| R1-F2.S5 | Ordinary nested repository and submodule | verified fixed | Nested-repository and submodule CLI controls passed with independent outer-only/empty expectations | None |
| R1-F2.S6 | Package scope, outside-package explicit file, active linked checkout | verified fixed | Package invocation, outside-package same-repo file and linked-worktree CLI tests passed | None |
| R1-F2.S7 | Untracked, staged, committed union buckets | verified fixed | Nine original boundary/bucket matrices and the parent-case bucket matrices passed; one guarded insertion serves the union | None |
| R1-F2.S8 | Nested candidate alias returning to outer checkout | verified fixed | `nested_candidate_alias_cannot_return_changed_identity_to_outer_checkout` proves changed evidence and rejects the unchanged outer target | None |
| R1-F2.S9 | Nested/external directory aliases and back-links | verified fixed | Both original back-link tests and all `directory_alias_parent_*` tests passed | None |
| R1-F2.S10 | Ownership inspection failures | verified fixed | Ownership error test passed; `try_exists`, metadata, canonicalization and `read_link` errors remain propagated | None |
| R1-F2.C1 | `functions` selection, reports, cache | verified fixed | Original and parent-case rejection matrices assert empty reports and zero per-file work | None |
| R1-F2.C2 | `types` selection, reports, cache | verified fixed | Same independent empty-result assertions for types | None |
| R1-F2.C3 | `symbols` selection, reports, cache | verified fixed | Same independent empty-result assertions for symbols | None |
| R1-F2.C4 | `classes` selection, reports, cache | verified fixed | Same assertions using classes' supported outline/structured modes | None |
| R2-F1 — Repository ownership lost through parent components, sibling of R1-F2 | Six forbidden traversal shapes, original controls, positive parent/alias controls, errors, documentation, four consumers | verified fixed | All 24 permanent parent-case tests passed; current traversal retains root aliases and checks each component before parent removal | None |
| R2-F1.S1 | Nested `.git` directory before parent cancellation | verified fixed | `nested_directory_parent_{untracked,staged,committed}`: both selectors, every command/mode, zero per-file work | None |
| R2-F1.S2 | Nested `.git` file before parent cancellation | verified fixed | `nested_file_parent_*`: same assertions through a linked checkout | None |
| R2-F1.S3 | Absolute external-root traversal returning to active checkout | verified fixed | `external_parent_*`: canonical target is local but selection remains empty in every mode | None |
| R2-F1.S4 | Directory alias followed by parents | verified fixed | `directory_alias_parent_*`: alias destination checked before subsequent parents; every report empty | None |
| R2-F1.S5 | Recursive file-alias target | verified fixed | `recursive_file_alias_parent_*`: intermediate alias committed, changed alias alone supplies evidence, all reports empty | None |
| R2-F1.S6 | Dot-plus-parent spelling | verified fixed | `dot_parent_*`: both selectors and all consumers reject the nested boundary | None |
| R2-F1.S7 | Ordinary parent traversal | verified fixed | `ordinary_parent_*`: exact names/counts 1/1/2/1, all buckets/selectors, cached/uncached facts and warm hits | None |
| R2-F1.S8 | Existing clean boundary controls | verified fixed | All nine original boundary matrices and both back-link tests passed | None |
| R2-F1.S9 | Inspection errors, one Git observation, package/linked scope and cache controls | verified fixed | Ownership error and CLI scope/cache tests passed; `resolve` still calls `changed_paths` once, and API work counters passed | None |
| R2-F1.S10 | Comments, README, topic and skill guidance | verified fixed | Traversal contract in `changed.rs`, CLI README, Symbol Reports and tree-hugger skill matches negative and positive fixtures | None |
| R2-F1.S11 | Ordinary directory-alias parent semantics | verified fixed | `ordinary_directory_alias_parent_*`: exact outer declarations and warm hits; root aliases return to the same checked component loop | None |
| R2-F1.C1 | `functions` selection/report/cache consumers | verified fixed | Six forbidden shapes × three buckets × both selectors: all supported reports empty and zero source/cache work | None |
| R2-F1.C2 | `types` selection/report/cache consumers | verified fixed | Same complete rejection matrix for types | None |
| R2-F1.C3 | `symbols` selection/report/cache consumers | verified fixed | Same complete rejection matrix for symbols, plus independently empty uncached JSON | None |
| R2-F1.C4 | `classes` selection/report/cache consumers | verified fixed | Same matrix for classes, including its distinct empty-outline contract | None |
| R3-F1 — Truncated Git indexes escape fallible readers as panics | All original sites and consumers; repair-discovered committed-diff readers; valid-state, metadata and cost controls | verified fixed | API, Context and CLI all passed; production index reads and committed-diff initialization now use shared containment | None |
| R3-F1.S1 | Optional cache sizing, both shared-handle callers | verified fixed | API invokes direct cache sizing under every prefix; metadata remains correct. Both callers use the same guarded helper | None |
| R3-F1.S2 | Shared path and rich status collector | verified fixed | API asserts typed errors for direct path/rich readers and public paths/status; valid exact paths and category counts retained | None |
| R3-F1.S3 | Fast dirty reader and its early stop | verified fixed | API checks direct dirty and request errors; valid dirty control passed. The guard surrounds initialization; the first-change return remains | None |
| R3-F1.S4 | Detailed category counts and total counts | verified fixed | API asserts typed errors for both readers; valid results remain (true,0,1,0) and (true,1) | None |
| R3-F1.S5 | Rich status context, including direct reload without a preceding walk | verified fixed | Context asserts typed errors and no escaping unwind for all 12 prefixes; valid context succeeds. Rich caller propagates its error | None |
| R3-F1.S6 | Direct/public conflicts and compatibility suppression | verified fixed | API checks direct/public Git errors for every prefix, empty compatibility result and valid no-conflict control; existing containment preserved | None |
| R3-F1.S7 | Discovery and identity-only projection | verified fixed | API preserves main branch identity and absent status at every prefix; identity returns before index-dependent work | None |
| R3-F1.S8 | Cache sizing and status of a separately corrupted linked checkout | verified fixed | API linked matrix rejects all 12 prefixes in the linked path API and main-checkout detail reader; valid main path result remains empty | None |
| R3-F1.C1 | functions: both base variants, main/linked index, every supported report mode | verified fixed | CLI main/linked prefix matrix: exit 1, empty stdout, root/index cause, no cache statistics; independent edited function control | None |
| R3-F1.C2 | types: same complete matrix | verified fixed | CLI main/linked prefix matrix: exact error contract; independent Edited type control | None |
| R3-F1.C3 | symbols: same complete matrix | verified fixed | CLI main/linked prefix matrix: exact error contract; independent edited and Edited controls | None |
| R3-F1.C4 | classes: same matrix, including its outline rather than tree mode | verified fixed | CLI main/linked prefix matrix: exact error contract; independent Edited class-report control | None |
| R3-F1.C5 | Public changed-path API, omitted and explicit HEAD | verified fixed | API asserts Git errors for both requests at every prefix; valid exact leaf.rs controls passed | None |
| R3-F1.C6 | Public file-change and repository-status consumers | verified fixed | API asserts Git errors; valid one Modified leaf.rs and exact dirty/category counts passed | None |
| R3-F1.C7 | Minimal and summary requests | verified fixed | API asserts Git errors; valid dirty status retains zero category counts as these projections require | None |
| R3-F1.C8 | Count-only and full requests | verified fixed | API proves count request reaches its nonminimal reader, asserts typed failures and exact valid category counts | None |
| R3-F1.C9 | Fallible branches, compatibility branches and branch details | verified fixed | API preserves exactly one main branch, current flag and zero ahead/behind despite corruption | None |
| R3-F1.C10 | Fallible and compatibility tracking readers | verified fixed | API preserves independently expected empty result in the no-remotes fixture under every mutation | None |
| R3-F1.C11 | Detailed worktrees, both absent registrations and corrupted linked index | verified fixed | API preserves empty result without linked registrations; a registered corrupt linked checkout returns a typed detail error | None |
| R3-F1.C12 | Legacy and structured recent-commit readers | verified fixed | API preserves one expected legacy commit; structured collection returns typed attribute-index errors, with one-commit valid control | None |
| R3-F1.C13 | Existing-handle worktree list and current name | verified fixed | API preserves one main worktree/branch, not current because process CWD is elsewhere, and no registered current name | None |
| R3-F1.C14 | Existing-handle preferred remote | verified fixed | API preserves independently expected None in the no-remotes fixture; optional sizing cannot unwind | None |
| R3-F1.C15 | Aggregate observations | verified fixed | API proves one expected commit on valid input and typed index-dependent failure for all prefixes | None |
| R3-F1.S9 | Absent, valid empty and unborn indexes; main and linked checkout | verified fixed | API and CLI independently assert current declarations and exact paths; no empty fallback for malformed indexes | None |
| R3-F1.S10 | Current error documentation and skill guidance | verified fixed | Read CLI README, Symbol Reports, sniff architecture, both area skills and changed API comments; they describe contained errors and distinguish absent/empty state | None |
| R3-F1.S11 | One observation, early stopping and no extra discovery/network/enrichment | verified fixed | API work-counter test passed. Guard does not preflight or create another walk; dirty loop still stops at first changed item | None |
| R3-F1.S12 | Historical type-change, ownership, selector and cache regressions | verified fixed | CLI 110-test run, API phase/cost tests and Ownership errors passed; historical table above accounts for each required child | None |
| R3-F1.S13 | Committed-diff attribute cache initialization: commit files, path history and structured commits | verified fixed | API errors now contained in each reader; source sweep confirms all three initializers call the guarded shared cache builder | None |
| R3-F1.C16 | Commit-file direct/public readers and compatibility wrapper | verified fixed | API asserts typed direct/public errors, empty compatibility result and exact valid leaf.rs Added result | None |
| R3-F1.C17 | Path-history direct/public readers and compatibility wrapper | verified fixed | API asserts typed direct/public errors, empty compatibility history and one independently expected valid commit | None |
| R3-F1.S14 | Committed-only merge predictor | verified fixed | API preserves expected empty same-tip merge under every mutation; it builds its index and attribute stack from committed trees, with no live-index reader | None |
| R3-F1.S15 | Tip comparison and previously implicit attribute-index loading | verified fixed | API direct tip-diff errors are contained, valid comparison yields exact leaf.rs; source uses explicit guarded cache with rewrite tracking disabled | None |
| R3-F1.C18 | Captured-cache commit-file, wrapper and enriched committed changes | verified fixed | API preserves exact leaf.rs Added facts under every live-index mutation; supplied cache already holds attribute evidence and is not reinitialized | None |

No earlier finding was blocked. No approval, prerequisite change or unblocking
was needed before or after any repair. The accepted choices in the initial
[implementation log](implementation-log.md#rulings) remain intact: reject the
prelude combination before Git work, permit valid empty selections, compare
local commit tips directly, distinguish omitted and explicit references,
disable optional sniff features, and verify avoided work with counters.
These choices are not reopened by this review.

## Unblocked Findings

None. R1-F1, R1-F2, its R2-F1 sibling, and R3-F1 have no remaining required repair.

## Blocked Findings

None. Cross-OS CI results and a discretionary human review are not readiness
blockers for this assessment.

## Recurrence

All findings in review-1.md, review-2.md and review-3.md were compared. R1-F1
is the status-category omission class. R1-F2 and R2-F1 are the repository
ownership class; review 2 discovered parent-traversal siblings beyond review
1's direct aliases. R3-F1 is the malformed-index panic class; its repair sweep
also found committed-diff attribute readers. All these cases now pass.

This iteration found no unchanged failing reproduction, repair regression or
newly discovered defective sibling. `recurrence: false` records the absence
of a recurring finding in this review; it does not erase the ownership
recurrence documented in review 2.

## Class Sweep and Decision Trace

In the sniff package, [the shared index guard and readers](../../../sniff/lib/src/filesystem/git/open.rs)
contain index-dependent initialization. Optional cache sizing skips failure;
[the status readers](../../../sniff/lib/src/filesystem/git/status.rs) return
Git errors through path classification, rich status, dirty checks, counts and
conflicts. Index decoding happens before the gix status workers are started.
The guard stays around initialization rather than swallowing arbitrary errors
from the entire analysis pipeline. The workspace profiles retain unwinding.

The production reader search found no remaining direct live-index read outside
that shared boundary. [Committed-diff readers](../../../sniff/lib/src/filesystem/git/discovery.rs)
and [structured commit collection](../../../sniff/lib/src/filesystem/git/recent_commits/collect.rs)
use the guarded attribute-cache initializer. The tip comparison formerly hid
this index dependency inside a convenience iterator; its explicit cache is
now guarded too. Supplied captured caches keep their committed evidence.
[Committed-only merge prediction](../../../sniff/lib/src/filesystem/git/merge_conflicts.rs)
builds its index and attribute stack from captured trees and remains independent
of live-index corruption. Metadata readers preserve independently expected
branch, tracking, worktree, remote and legacy commit facts rather than silently
losing provable results.

In tree-hugger-cli, [changed-file resolution](../../cli/src/changed.rs) obtains
one path union, drops absent paths and checks each candidate's checkout
ownership before inserting its native identity. Each alias destination and
visited directory is checked before a parent component can remove it; the
same loop handles all Git buckets. Legitimate same-checkout parent and alias
paths retain their independently expected declarations. Foreign paths cannot
supply changed eligibility. There is no second repository discovery.

[The declaration command entry point](../../cli/src/main.rs) rejects Git errors
before ordinary selection, intersects before source/cache analysis, and only
converts the scanner's specific no-source-files outcome to empty success.
Every declaration consumer receives the same narrowed list: record filters,
class selection, count/statistics, structured/full-index reports and existing
text renderers. None can recover or hide a lost path after intersection.
There is no reference lookup or lookup stopping rule in this feature. The
cache key has no changed-selection or base-reference input; actual warm hits
and independently expected names verify reuse without using cache agreement
as the correctness oracle.

## Verification and Requirement Coverage

**Level 1 is the appropriate verification level for every changed behavior.**
The feature adds filesystem/Git selection and argument handling, and uses
existing renderers. It adds no keyboard, mouse, terminal encoder, styling,
width or scrolling behavior. No Level-2 or Level-3 verification gap was found,
and no terminal or browser window was opened.

| User-facing requirement | Strongest relevant verification | Outcome |
| --- | --- | --- |
| Optional value, equals form, repeat/empty rejection, command availability, help, prelude conflict and full JSON compatibility | L1 CLI parser/help assertions | Passed |
| Staged/unstaged/untracked union, whole current files, cancellation across stages, copy/rename/deletion/recreation, executable/type changes | L1 public API and CLI fixtures with exact paths/declarations | Passed |
| Local tip comparison, divergent branches, commit IDs/tags, unborn explicit/omitted base and detached HEAD | L1 API/CLI fixtures | Passed |
| Package scope, active linked checkout, explicit outside-package file, nested repos/submodules, external/nested aliases and parent traversal | L1 CLI ownership/selection matrices, including positive controls | Passed |
| File/name/exclusion/language intersection and ignore/fixture overrides | L1 CLI selector fixtures | Passed |
| Every empty report representation, zero per-file work and unchanged non-changed no-source error | L1 CLI reports and work assertions | Passed |
| Invalid/non-commit reference, missing/bare repo, malformed HEAD, conflicts anywhere and ownership I/O errors | L1 API/CLI error assertions | Passed |
| Malformed main/linked indexes fail before reporting while absent/valid empty indexes remain usable | L1 API, direct context and complete CLI prefix/report matrices; additional exhaustive prefix sweep | Passed |
| Metadata and compatibility consumers retain their error/success policies across index corruption | L1 API matrix with independently stated positive facts | Passed |
| Cache reuse with ordinary and changed scans, including different references | L1 actual warm-hit assertions and key inspection | Passed |
| Native paths, ordering/deduplication, spaces, Unicode and case aliases | L1 path fixtures and native identity inspection | Passed for exercised cases; macOS filesystem-guarded non-UTF-8 cases are not claimed as runtime evidence |
| One status observation, no line diff, patch, similarity, network, history or inventory work | L1 API work counters and production-call inspection | Passed |

Cargo compiles `tests/changed/main.rs` as an automatically discovered CLI
integration target; it declares every feature module, including index
corruption, ownership and type changes. Sniff's Git unit module declares
changed-path tests and their index-reader module. None of these tests needs
an optional feature or a tier marker. Both `just check-tier-coverage tree-hugger`
and `just check-tier-coverage sniff` passed with **zero stranded tests**.

`just lint` passed in both package areas. `git diff --check` passed. The initial
attempt to run the extra probe from the repository root used package-area
arguments and was rejected before testing; the corrected tree-hugger-area
command above executed and passed. No rejected invocation is counted as evidence.
Earlier full-area and Linux runs in the repair log were read but are not
claimed as independently rerun here. This review scoped its checks to the
changed selection and the shared Git-reader consumers. Cross-OS execution
proof remains with the existing CI schedule and is not a readiness finding.

### Input Robustness

The manifest/lockfile/configuration field-shape matrix is not applicable.
This feature adds no such deserializer or schema. Git's existing binary
index decoder still has a public error contract, verified as follows:

| Input shape | Independently expected public result | Verification |
| --- | --- | --- |
| Unedited real-tool main/linked index | Exact current declarations and changed paths; valid status and metadata facts | CLI/API positive controls passed |
| Absent main/linked index | Valid selection of current files | CLI/API controls passed |
| Unborn repository with absent or tool-written empty index | Current untracked declarations selected under omitted base | CLI/API exact-name/path controls passed |
| Tool-written index truncated to 0/1/4/8/11/12/13/16/20/31/84/103 bytes | Typed Git errors in status/index-dependent APIs; CLI exit 1, no stdout, root/index cause | All original main/linked, command, base and report combinations passed |
| Every shorter prefix between those named lengths, versions requested as 2/3/4 | Same CLI error contract for every declaration command | Additional assertion sweep passed |
| Corrupt live index with metadata-only request | Preserve independently expected available metadata; optional sizing cannot unwind | API matrix passed |
| Corrupt live index with previously captured committed-diff cache | Preserve exact committed file/change evidence | API captured-cache controls passed |
| Corrupt live index with committed-only merge prediction | Preserve committed-tree result without consulting the live index | API control and code invariant passed |

Current CLI README, Symbol Reports, sniff architecture, dependency pages and
area skills describe the implemented behavior. Historical specification and
repair logs remain snapshots. Existing source, tests and staged work were
preserved; only this review and the requested review/spec metadata were changed.
No formatter, commit, staging operation or lifecycle move was performed.

## Nonblocking Recommendations

None. No additional abstraction, performance benchmark or approval step is
needed to close this feature's contract.
