---
$schema: feature-review.yaml
ready: false
findings:
  - title: "R3-F1 — Truncated Git indexes escape fallible readers as panics"
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-07T21:25:10-07:00"
spec: 2026-10-09-changed-files-filter/spec.md
implemented: true
description: "A **feature** review of `2026-10-09-changed-files-filter/spec.md`"
feature: 2026-10-09-changed-files-filter/review-3.md
previous: 2026-10-09-changed-files-filter/review-2.md
log: tree-hugger/features/2026-10-09-changed-files-filter/log.md
implemented_by: codex/gpt-6.1-sol
next: 2026-10-09-changed-files-filter/review-4.md
---

# Changed Files Filter — Review 3

The feature is **not ready for production**. All previously required repairs
are verified fixed, including the parent-component ownership cases from the
second review. One new high-priority finding remains: a truncated Git index
makes every declaration command panic with exit `101`, instead of returning
the specified actionable error with exit `1`. The same input also panics in
sniff's other status readers and its optional object-cache sizing helper.
No human decision is needed to repair this.

I reviewed the specification, plan, initial implementation log, both earlier
reviews (including review 1's legacy `Findings` section), and both repair
iterations in `log.md`. The verification below reconciles the historical
parent findings, child instances, consumers, frontmatter, and repair ledger.
The ledger's closure claim is supported for the enumerated ownership and
file-type repairs; the new index-error defect was outside those inventories.

## Previous Finding Verification

Evidence references in the tables:

- **CLI:** `just test --test changed` in tree-hugger: **85 passed, zero
  skipped**, nextest `09ff930c-1a1e-468d-9880-6a14dbaf5d3b`.
- **API:** `just test changed_paths` in sniff: **33 passed** (other tests
  filtered out), nextest `7a30bcb3-fda4-4fa4-8ec9-d0937da33b7a`.
- **Ownership errors:** `just test --bin hug changed::tests`: **6 passed**,
  nextest `daaece52-eddd-48b8-b3b6-ac267f1f85bf`.
- **Extended traversal:** a temporary test in the declared CLI `changed`
  target: **passed**, nextest `e4bd9f94-f9da-4bb5-9ca2-97e22b50c982`.
  It asserted independent declaration names for ordinary deeper paths and
  empty results for the same paths through a nested repository, across
  untracked/staged/committed evidence, both selectors, and all four commands.
  It also preserved an ordinary directory alias pointing back to the checkout
  root. The separate index-error test failed in that run; it does not weaken
  the completed traversal assertions or count as a passing error test.

Every historical child ID is retained below, including the preservation and
documentation instances added by the implementation ledger.

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
| R1-F2 — Symlink identities admit files from another repository | Original boundaries, every union bucket, identity matching, four consumers, and recurring parent-component sibling R2-F1 | verified fixed | All original CLI ownership matrices and all R2-F1 regressions passed; extended traversal found no additional ownership failure | None |
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
| R2-F1 — Repository ownership lost through parent components, sibling of R1-F2 | Six forbidden traversal shapes, original controls, positive parent/alias controls, errors, documentation, four consumers | verified fixed | All 24 permanent parent-case tests passed; extended traversal checked deeper composition and root-alias controls | None |
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
| R2-F1.S11 | Ordinary directory-alias parent semantics | verified fixed | `ordinary_directory_alias_parent_*`: exact outer declarations and warm hits; extended deeper/root-alias controls also passed | None |
| R2-F1.C1 | `functions` selection/report/cache consumers | verified fixed | Six forbidden shapes × three buckets × both selectors: all supported reports empty and zero source/cache work | None |
| R2-F1.C2 | `types` selection/report/cache consumers | verified fixed | Same complete rejection matrix for types | None |
| R2-F1.C3 | `symbols` selection/report/cache consumers | verified fixed | Same complete rejection matrix for symbols, plus independently empty uncached JSON | None |
| R2-F1.C4 | `classes` selection/report/cache consumers | verified fixed | Same matrix for classes, including its distinct empty-outline contract | None |

Neither earlier finding had a blocker, and no prerequisite was removed before
or after the repairs. The accepted choices recorded in the initial
implementation log's **Phase 1 / Rulings** remain intact: reject the prelude
combination before Git inspection, allow valid empty selections, compare local
commit tips directly, distinguish omitted from explicit references, and prove
avoided work with counters rather than a timing benchmark. This review does
not reopen those choices.

## Unblocked Findings

### R3-F1 — High: Truncated Git indexes escape fallible readers as panics

**Defect class:** An upstream binary decoder can panic on malformed external
input, and callers that advertise fallible results fail to contain that panic
at every index-reading boundary.

In the sniff package, the optional object-cache heuristic
[`configure_cache`](../../../sniff/lib/src/filesystem/git/open.rs#L60) calls
`index_or_empty()` inside an `if let Ok(...)`. That handles returned errors,
but does not handle panics. The new
[`GitRepo::changed_paths`](../../../sniff/lib/src/filesystem/git/types.rs#L886)
uses `with_cached_gix`, which runs this heuristic before collecting status.
A real fixture index truncated below 20 bytes causes the pinned decoder to
panic with `attempt to subtract with overflow`. The public path API unwinds;
the tree-hugger-cli commands exit `101` and print an upstream panic location
instead of the required repository-naming exit-1 error.

The independent status paths also panic when invoked without cache sizing:
[`classify_status_paths`](../../../sniff/lib/src/filesystem/git/status.rs#L125),
the fast dirty reader, and the detailed count reader all initialize status
iteration without containing the same index-decoder failure. Merely protecting
cache sizing therefore leaves required sibling readers broken. Conversely,
protecting status alone leaves `changed_paths` crashing before it reaches status.

There is an existing correct boundary in sniff:
[`detect_merge_conflicts_fallible`](../../../sniff/lib/src/filesystem/git/status.rs#L1125)
already catches this decoder panic and maps it to a Git error. Its public
`GitRepo::merge_conflicts` consumer returned errors for every malformed index
in the sweep. Reuse that contract rather than adding a command-specific panic
handler or treating the malformed index as an absent/empty index.

**Minimal reproduction, using the existing real-tool fixture:**

```rust
let repo = Repo::committed(&[("leaf.rs", "fn original() {}\n")]);
repo.write("leaf.rs", "fn edited() {}\n");
let index = repo.dir.path().join(".git/index");
let original = std::fs::read(&index).unwrap();
std::fs::write(index, &original[..16]).unwrap();
let output = repo.hug(&["symbols", "--changed", "--json"]);
```

Before truncation the public report contains `edited`; after truncation the
expected outcome is exit `1`, empty stdout, and stderr naming the repository
and index-read failure. Observed: exit `101`, empty stdout, and a panic from
`gix-index-0.52.0/src/file/init.rs:73`. This is ordinary repository corruption,
which the specification explicitly requires the feature to handle as an error.

**Sweep evidence:**

- **CLI corruption matrix:** nextest
  `fe31eaec-ddc5-4471-a9ff-2dda692ec368`, a deliberately failing assertion
  test in the declared `changed` target. Starting from the same libgit2-written
  104-byte index, independently restore and truncate it to 0, 1, 4, 8, 11,
  12, 13, 16, 20, 31, 84 or 103 bytes. All four commands, both bare
  `--changed` and `--changed=HEAD`, and every supported report mode were run.
  Each short shape produced **78 panics**: **624 incorrect exits** total.
  Each of the four longer malformed shapes correctly produced 78 actionable
  exit-1 failures: **312 passing error controls**. There were no successful
  reports for any malformed input.
- **API consumer matrix:** nextest
  `2e8300f9-dd18-4213-b9a0-6010eb454be9`. A diagnostic test caught unwinds
  around each independently invoked reader, printed the outcomes, and restored
  the index. The eight short shapes panic in the unguarded sites below; the
  four longer shapes return normal errors in status-bearing readers. The
  original 104-byte fixture succeeds across all tested consumers. This
  diagnostic test's successful exit is observation evidence, **not a passing
  corruption assertion**.
- **Linked checkout and absence controls:** nextest
  `78cc39e2-a277-4746-b9ec-8be564e9d859`. Corrupting the active linked
  checkout's own administrative index to 0 or 16 bytes produced exit `101`
  in all four commands. Its 20- and 103-byte controls returned exit `1`.
  A detailed worktree request from the valid main checkout also panicked
  while configuring the linked checkout's cache. The probe independently
  asserted that an **absent** index remains valid and reports current `edited`
  declarations, and that a libgit2-written **valid empty** index in an unborn
  repository reports the untracked `first` function. Those positive assertions
  passed; the corruption observations are not correctness assertions.

The instance table inventories every production live-index loader and all
consumer groups reached by the changed-file and shared-cache paths. Compatibility
methods that already suppress returned errors retain that policy; preventing a
panic is required, not a redesign of their established return shapes. Several
shared-cache consumers predate this feature and are explicitly included because
the same helper now serves the new API.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- |
| R3-F1.S1 — cache sizing → `ensure_cache` / `with_cached_gix` | Eight short index prefixes; four longer malformed prefixes; original index | Short prefixes panic; longer errors skip sizing; original succeeds | Cache optimization must never panic; required status reads still return a Git error | Direct `configure_cache` API matrix; source at `open.rs:61` and `types.rs:634,646` |
| R3-F1.S2 — shared status collector | Same prefixes, direct path/rich readers | Short prefixes panic; longer prefixes return errors | Fallible Git error, no partial status/paths | API path, file-change and status rows |
| R3-F1.S3 — fast dirty reader | Same prefixes, direct call | Same panic/error split | Fallible Git error; preserve first-change stopping on valid input | API `dirty` row and original/type-change controls |
| R3-F1.S4 — detailed and total count readers | Same prefixes, direct calls | Same panic/error split | Fallible Git error; no empty/zero fallback | API count rows |
| R3-F1.S5 — rich `StatusContext::new` index reader | Same malformed fixtures through rich status | Earlier status walk panics/errors before this reader is reached | Preserve ordering: an invalid index cannot reach enrichment as an empty context | API rich rows; `get_repo_status_with_changes` uses `classify_status_paths(...)?` before `StatusContext::new` at line 379; no separate reachable failure in these fixtures |
| R3-F1.S6 — guarded conflict reader | All 12 malformed prefixes | Git error for every shape, no escaping panic | Exactly that failure contract | API direct fallible-conflicts and public-conflicts rows; existing `catch_unwind` boundary |
| R3-F1.S7 — discovery / identity-only projection | All prefixes and original | Discovery succeeds; identity projection succeeds | Preserve repository identity without requiring a readable index; no status report is produced | API identity row; trusted discovery and `is_identity_only` return before cache/status reads |
| R3-F1.S8 — cache sizing in opened linked-checkout details | Main index valid; linked index 0/16/20/103 bytes | Short prefixes panic; longer prefixes return a detail-read error | No escaping panic; preserve existing fallible detail behavior | Linked probe; `remote_refresh.rs:1043` is the other production `configure_cache` call site |
| R3-F1.C1 — `functions` selection and every report mode | All prefixes, omitted/explicit HEAD; linked short controls | Exit 101 on short shapes, stdout empty | Exit 1 and actionable repository/index error before report or source/cache analysis | CLI matrix and linked probe |
| R3-F1.C2 — `types` selection and every report mode | Same | Same | Same | CLI matrix and linked probe |
| R3-F1.C3 — `symbols` selection and every report mode | Same | Same | Same | CLI matrix and linked probe; original fixture independently reports `edited`/`Edited` |
| R3-F1.C4 — `classes` selection and every supported report mode | Same, outline instead of tree | Same | Same | CLI matrix and linked probe |
| R3-F1.C5 — public changed-path API, both base variants | All prefixes | Short shapes unwind; longer shapes return Git errors | Typed error, never unwind or empty success | API `changed omitted` and `changed explicit HEAD` rows |
| R3-F1.C6 — `file_changes` / `repo_status` | All prefixes | Short shapes unwind | Typed error; preserve valid rich status | API rich rows; original and type-change matrices |
| R3-F1.C7 — minimal/summary requests | All prefixes | Short shapes unwind through fast dirty reader | Typed error; no invented clean status | API request rows |
| R3-F1.C8 — count-only and full requests | All prefixes | Short shapes unwind through counts or cache/rich status | Typed error; no partial summaries | API request rows |
| R3-F1.C9 — branch readers and compatibility wrapper | All prefixes; `try_branches`, `branches`, `branch_info(false)` | Short shapes panic in cache sizing; longer shapes complete | No panic; retain local branch facts or established error policy | API branch rows; original fixture succeeds; helper is an optimization, not branch evidence |
| R3-F1.C10 — tracking readers and compatibility wrapper | All prefixes; `try_tracking_status`, `tracking_status` | Short shapes panic in cache sizing; longer shapes complete | No panic; preserve no-remotes result and established error policy | API tracking rows; fixture has no remotes |
| R3-F1.C11 — detailed worktree reader | Active and separately corrupted linked index | Short shapes panic | No panic; preserve valid metadata and existing fallible detail policy | API `worktrees` row and linked detail probe |
| R3-F1.C12 — legacy and structured recent-commit readers | All prefixes; `recent_commits(1)`, `RecentCommits::collect` | Short shapes panic before collection; longer shapes return success/error according to later work | Cache heuristic must not introduce an unwind; retain each reader's own success/error contract | API recent-commit rows; original fixture succeeds; legacy reader also reaches cached ref decorations |
| R3-F1.C13 — worktree list/name on an existing handle | All prefixes; `list_worktrees_with_repo`, `current_worktree_name_with_repo` | Short shapes panic in cache sizing; longer shapes complete | No panic; preserve metadata-only worktree list/name | API worktree rows; main checkout has no linked-worktree name |
| R3-F1.C14 — preferred remote on an existing handle | All prefixes; `preferred_remote_url_with_repo` | Short shapes panic in cache sizing; longer shapes return absent remote | No panic; preserve the fixture's absent remote | API preferred-remote row; no remote configured |
| R3-F1.C15 — aggregate branch/worktree/commit observations | All prefixes; `observe_aggregate_evidence(None)` | Short shapes panic before observation; longer shapes return a downstream error | No panic; preserve fallible aggregate contract and metadata reuse | API aggregate row; original fixture succeeds; cached branches/worktrees and `collect_observed` share the same guarded-helper requirement |

**Decision trace:** valid repository discovery → cache setup reads the live
index and panics before `changed_paths` can return a `Result` → CLI `resolve`
never reaches its error formatter → identity insertion, ordinary selection,
intersection, analyzer/cache lookup, record filters, counts/stats and all
renderers are never reached → Rust terminates the command with exit `101`.
The independent dirty/count/rich readers have another route into the same
decoder through status iteration. There is no later selection or reporting
decision that can recover either failure. Full JSON uses the same early path.

**Required repair:** Contain malformed-index decoder panics at the shared
index-reading boundaries, including optional cache sizing and status iterator
initialization. Follow the already established fallible-conflict-reader
approach or another shared solution that produces typed Git errors. Preserve
metadata-only results when optional sizing cannot be performed; status-bearing
requests must reject corruption, not reinterpret it as no index. Cover both
cache call sites, all status reader paths and every consumer listed above.
Keep one status observation, early stopping, no extra discovery/network work,
and valid absent/empty/unborn/linked-index behavior.

Add permanent public-result tests using a restored tool-written index per
mutation, with positive controls and the entire short/long truncation matrix.
Assert typed errors without escaping unwinds in sniff and exact exit `1`,
empty stdout, and repository/index context through all four CLI consumers.
Keep normal returned-error controls and the existing conflict containment.
Review the touched error-contract comments and current API/topic/skill guidance
in the same repair. Catching only at the binary entry point would leave the
public library contract and sibling readers broken.

## Blocked Findings

None. R3-F1 is unblocked; no approval, changed prerequisite, dependency update,
or cross-OS execution result is needed before implementing the repair.

## Recurrence

This review compared the complete findings in **review-1.md** and
**review-2.md**. R1-F1 concerned dropping a valid status category. R1-F2 and
its R2-F1 sibling concerned erasing repository boundaries during identity
normalization. Both classes are verified fixed, with all historical instances
and consumers accounted for above.

R3-F1 is a **newly discovered defect class**, not an unchanged failing earlier
reproduction, a regression from the ownership repair, or another sibling of
the ownership/status-category findings. Earlier reviews ran the malformed-HEAD
control and described Git corruption coverage, but did not enumerate an index
decoder finding. That HEAD control still passes. `recurrence: false` therefore
reflects the finding history rather than treating every Git error as one class.

## Verification and Requirement Coverage

**Level 1 is the appropriate level for every requirement in this feature.**
These requirements concern local Git state, filesystem selection, argument
syntax and existing report representations. The feature changes no terminal
encoding, styling, scrolling, keyboard, mouse or browser behavior. No terminal
or browser window was opened, and no Level-2/Level-3 evidence is needed here.

| User-facing requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Optional value, equals syntax, repeated/empty values, command availability, public help, prelude conflict, full JSON compatibility | L1 CLI parser/help tests | Passed |
| Status union, current disk content, edits cancelling across stages, copies, renames, deletion/recreation, type changes | L1 API and CLI fixtures with exact paths/declarations | Passed |
| Direct local tip comparison, divergent branches, commit IDs/tags, unborn and detached HEAD | L1 API and CLI repository fixtures | Passed |
| Package scope, active linked checkout, outside-package explicit files, nested repositories/submodules, alias boundaries and parents | L1 CLI fixtures and independent extended traversal | Passed; previous ownership class closed |
| File/name/exclusion/language selectors, ignore/fixture precedence | L1 CLI selector fixtures | Passed |
| Empty representations in every mode and zero per-file analysis/cache work | L1 CLI report/work-counter assertions | Passed |
| Bad references, missing/bare repository, malformed HEAD, conflicts outside package, ownership inspection failures | L1 API/CLI error assertions | Passed |
| Corrupt Git index returns an actionable exit-1 error before reporting | L1 public API/CLI truncation reproductions | Failed: R3-F1; existing permanent corruption test changes HEAD only |
| Same cache entries across ordinary/changed scans and references | L1 actual warm-hit assertions plus cache-key inspection | Passed |
| Native paths, deterministic deduplication, spaces/Unicode and case spellings | L1 path fixtures | Passed where applicable; macOS non-UTF-8 filesystem fixture returns early and is not runtime evidence for that case |
| One status observation, no optional rich diff/history/network/inventory enrichment | L1 API work counters and code inspection | Passed on valid input; preserve during R3-F1 repair |

Both `just check-tier-coverage tree-hugger` and
`just check-tier-coverage sniff` passed with **zero stranded tests**. Cargo
compiles the CLI `tests/changed/main.rs` target, which declares `ownership`
and `type_changes`; sniff declares its Git test module under `cfg(test)`.
Their test names select Level 1 and the reported nextest runs executed them.
Temporary review probes were explicitly registered in those same targets and
removed after collecting evidence, restoring the pre-review source files.

`just lint` in tree-hugger passed for both library and CLI after probe removal.
`git diff --check` passed. No formatter, staging, commit, implementation repair,
or lifecycle move was performed. Existing staged and unstaged work was preserved.
Earlier full-area and Linux results in the implementation ledger remain
implementer claims; this assessment uses the independently run checks above.
Cross-OS proof remains CI's responsibility and is not a readiness finding.

### Input robustness

The manifest/lockfile/configuration field-shape matrix does not apply: the
feature introduces no such schema or deserializer, and Git's binary index
decoding belongs to the existing gix backend. That does not exempt corruption
from the public error contract. The following public-result matrix records
the complete index shape sweep rather than relying on a malformed-HEAD test.

| Input shape | Changed-path/status readers | Four declaration CLI consumers | Independently expected result |
| --- | --- | --- | --- |
| Unedited tool-written index | Success; exact changed path in control | Current `edited`/`Edited` declarations | Positive changed-file result |
| Absent linked index | Normal selection | Current `edited`/`Edited` declarations | Valid no-index state; preserve current source |
| Valid tool-written empty index, unborn repository | Normal selection through CLI/public API | Untracked `first` declaration in control | Valid empty index, not corruption |
| Truncated to 0/1/4/8/11/12/13/16 bytes | Escaping panic, except guarded conflict reader | Exit 101 in every tested mode/base variant | Typed Git error / actionable exit 1 |
| Truncated to 20/31/84/103 bytes | Returned error in status readers | Actionable exit 1, empty stdout | Exactly that error result |
| Linked index truncated to 0/16 bytes | Cache/detail reader also panics | Exit 101 in all four count controls | Typed Git error / actionable exit 1 |
| Linked index truncated to 20/103 bytes | Detail error | Exit 1 | Exactly that error result |
| Malformed HEAD, invalid/non-commit reference | Returned error | Exit 1, no report | Existing error behavior; unchanged clean controls |

No missing declarations were found in the positive ownership controls and no
extra declarations were found in the forbidden-boundary cases. Cache agreement
was supplemented with independently expected names/counts and actual hit
assertions; it was not used as a correctness oracle by itself.

## Nonblocking Recommendations

None. The only remaining required work is the complete R3-F1 error-boundary
repair and its public-result regression coverage.
