---
$schema: feature-review.yaml
ready: false
findings:
  - title: "R1-F2 — Repository ownership is still lost through parent components (new sibling R2-F1)"
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-07T21:11:15-07:00"
spec: 2026-10-09-changed-files-filter/spec.md
implemented: true
implemented_by: codex/gpt-6.1-sol
log: tree-hugger/features/2026-10-09-changed-files-filter/log.md
description: "A **feature** review of `2026-10-09-changed-files-filter/spec.md`"
feature: 2026-10-09-changed-files-filter/review-2.md
previous: 2026-10-09-changed-files-filter/review-1.md
next: 2026-10-09-changed-files-filter/review-3.md
---

# Changed Files Filter — Review 2

The feature is **not ready for production**. The file-type repair is verified
fixed. The ownership repair passes every previously enumerated reproduction,
but a newly discovered sibling still transfers changed-file eligibility across
repository boundaries: a symlink target containing `..` can enter a foreign
repository and return to an unchanged source in the active checkout. All four
declaration commands then report that source instead of an empty selection.

This review checked the specification, plan, initial implementation log,
review 1's frontmatter and complete body, and the repair log's 27 unique ledger
IDs. Review 1 is the only earlier review in this directory; it uses one legacy
`Findings` section containing R1-F1 and R1-F2. There were no blocked findings.
The repair log's claim that all instances are closed is correct for its
enumerated fixtures, but does not establish closure of the ownership class.

## Previous Finding Verification

Evidence labels below refer to independently executed tests in this review:

- **CLI:** `just test --test changed` in tree-hugger; **61 passed**, nextest
  `06bf58b6-7936-42bc-807c-d2077fe7b03d`.
- **API:** `just test changed_paths` in sniff; **33 passed**, nextest
  `3fbb4f62-7a1a-4d95-8439-c26f89951275`.
- **Ownership error:** `just test --bin hug changed::tests::ownership_inspection_failure`
  in tree-hugger; **1 passed**, nextest `458039da-de9b-44b8-922b-ccfc3c8f6245`.
- **Parent-component sweep:** temporary assertions using the existing CLI
  fixtures; nextest `bd74d957-d573-4433-b9a0-3e25f398a4ed` exposed the new
  sibling across all three Git buckets, both selectors, and four commands.

The verification table preserves every child ID from review 1 and the repair
ledger. A fixed child below means its enumerated reproduction and consumers
were checked; the newly discovered shapes have separate R2-F1 child IDs.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
| --- | --- | --- | --- | --- |
| R1-F1 — Unstaged file-type changes disappear from changed-file selection and status | Both type-change directions; all status readers, request projections, four declaration commands and report modes | verified fixed | API phase matrix and CLI `type_changes` matrix passed; all three worktree readers now call the same classifier | None |
| R1-F1.S1 | Path collector → `GitRepo::changed_paths` | verified fixed | API `file_type_replacements_agree_across_status_readers_and_commit_phases`: exact `[leaf.rs]` in both unstaged directions | None |
| R1-F1.S2 | Rich collector → `file_changes` / `repo_status` | verified fixed | Same matrix: exactly one Modified/Modified file; dirty true, unstaged 1 | None |
| R1-F1.S3 | Fast dirty reader → minimal/summary requests | verified fixed | Same matrix asserts direct dirty result and both public request projections; early stop remains in the reader | None |
| R1-F1.S4 | Detailed counts → count-only requests and total counts | verified fixed | Same matrix: `(true,0,1,0)`, total `(true,1)`, and count-only public request categories | None |
| R1-F1.S5 | Staged classification in both directions | verified fixed | API exact path, staged count 1 and one Staged/Modified file; CLI exact staged counts for every command | None |
| R1-F1.S6 | Committed tip comparison in both directions | verified fixed | API exact base comparison path and clean status after commit; CLI base counts positive and omitted-base counts zero | None |
| R1-F1.S7 | Conflicts before selection, including outside package | verified fixed | API `index_conflicts_anywhere_fail_naming_every_path` and CLI `conflicts_outside_the_package_still_fail` passed; classifier's conflict arm remains first | None |
| R1-F1.C1 | `functions` reports and cache consumers | verified fixed | CLI type matrix: exact current function, one file/index, count 1, JSON Lines/stats, text/plain/table/tree, uncached facts and actual warm hits | None |
| R1-F1.C2 | `types` reports and cache consumers | verified fixed | Same matrix: exact current type, count 1 and every supported report mode; full indexes contain both declarations | None |
| R1-F1.C3 | `symbols` reports and cache consumers | verified fixed | Same matrix: exact two current declarations, count 2, every report mode and warm hits | None |
| R1-F1.C4 | `classes` reports and cache consumers | verified fixed | Same matrix: current type, count 1, outline/plain/table and structured reports, stats and warm hits | None |
| R1-F2 — Symlink identities admit files from another repository | Candidate ownership, identity membership, every Git bucket and downstream report/cache consumer | partially fixed | All enumerated ownership regressions pass in CLI; parent-component sweep fails on newly discovered paths | Repair new sibling R2-F1 below; no blocker |
| R1-F2.S1 | External alias and target selectors | verified fixed | CLI `external_{untracked,staged,committed}_aliases_are_rejected_in_every_selector_and_report`: empty reports and zero source/cache work | None for original shapes |
| R1-F2.S2 | Nested repository boundary | verified fixed | Six CLI matrices cover `.git` directories and linked-worktree `.git` files, both selectors and all buckets | None for original shapes |
| R1-F2.S3 | Legitimate same-checkout aliases | verified fixed | CLI `same_checkout_aliases_preserve_expected_declarations`: exact names, counts 1/1/2/1, cached and uncached results, actual hits | None |
| R1-F2.S4 | Foreign explicit file without an alias | verified fixed | CLI `an_explicit_file_in_another_repository_is_not_selected` passed with empty files/errors | None |
| R1-F2.S5 | Ordinary nested repository and submodule | verified fixed | CLI `a_nested_repository_is_not_descended_into` and `submodule_contents_are_never_selected` passed with independently expected outer-only/empty results | None |
| R1-F2.S6 | Package scope and linked checkout | verified fixed | CLI package invocation, outside-package same-repo file, and linked-worktree tests passed with the expected files | None |
| R1-F2.S7 | Untracked, staged and committed union buckets | verified fixed | Nine boundary-by-bucket matrices passed; one guarded identity insertion serves every bucket | None for original shapes |
| R1-F2.S8 | Nested candidate alias returning to outer checkout | verified fixed | CLI `nested_candidate_alias_cannot_return_changed_identity_to_outer_checkout`: proves sniff supplies changed evidence, then rejects the unchanged outer target | None for original shape |
| R1-F2.S9 | Directory aliases through nested/external ancestors | verified fixed | CLI nested candidate and `external_directory_alias_cannot_return_changed_identity_to_active_checkout` controls passed for the original back-link paths | Parent-component variants are new R2-F1 instances |
| R1-F2.S10 | Ownership inspection failure | verified fixed | Ownership error test passed; `try_exists`, canonicalization, marker metadata and `read_link` errors propagate before reports | None |
| R1-F2.C1 | `functions` foreign-source report consumers | verified fixed | Nine CLI matrices: empty listing/tree, JSON/JSON Lines/full indexes, count/stats and zero source/cache work | New path shapes still reach this consumer |
| R1-F2.C2 | `types` foreign-source report consumers | verified fixed | Same independently asserted empty report matrix | New path shapes still reach this consumer |
| R1-F2.C3 | `symbols` foreign-source report consumers | verified fixed | Same independently asserted empty report matrix | New path shapes still reach this consumer |
| R1-F2.C4 | `classes` foreign-source report consumers | verified fixed | Same matrix, using classes' empty outline contract instead of tree | New path shapes still reach this consumer |

No prerequisite or blocker changed before or after either repair: both findings
were actionable in review 1. The plan's accepted choices—reject `--prelude`,
report successful empty selections, compare local tips directly, and perform
no timing spike—remain unchanged and are not reopened here.

## Unblocked Findings

### R1-F2 — High: Repository ownership is still lost through parent components (new sibling R2-F1)

**Defect class:** Normalizing a filesystem path before validating its traversal
erases repository boundaries and lets an alias transfer changed-file eligibility
to an otherwise unchanged source.

In tree-hugger-cli, the ownership check
[`checkout_owns_path`](../../cli/src/changed.rs#L115) canonicalizes each ancestor
before inspecting boundaries. For a target `inner/../clean.rs`, the ancestor
`inner/..` canonicalizes to the active checkout root. The function returns
`true` at lines 147–148 before it ever examines `inner` and its `.git` marker.
Its resolved-ancestor marker loop also sees only the normalized root, so there
is no nested directory left to inspect. The recursive symlink-target check
uses the same function and therefore loses the same evidence.

This violates the current documented rule that directory and file aliases
cannot cross another checkout or nested repository and return eligibility to
an outer source file ([Symbol Reports](../../docs/symbol-reports.md#changed-files)).
It is the same rule already implemented for the repair log's nested/external
back-link controls. The consequence is an **extra result**: an unchanged outer
file becomes eligible solely because a changed alias crossed a forbidden
boundary and returned. It does not require reading a foreign declaration.

**Reproduction using the existing fixture builder:**

```rust
let repo = Repo::committed(&[("clean.rs", "fn clean() {}\nstruct Clean;\n")]);
let inner = repo.dir.path().join("inner");
std::fs::create_dir(&inner).unwrap();
crate::git_init(&inner);
link(Path::new("inner/../clean.rs"), &repo.dir.path().join("alias.rs"));
```

Run `hug symbols --changed --file clean.rs --count` from the outer repository.
Expected `0\n`; observed `2\n`. Selecting `alias.rs` gives the same incorrect
result. Replacing `git_init` with `add_worktree` reproduces it with a `.git`
file. An absolute target `B/subdir/../../A/clean.rs`, with independent sibling
repositories A and B, reproduces the external-boundary case.

The temporary probes copied the real fixture helpers, proved sniff returned
the alias as changed, and checked the canonical identity against `clean.rs`.
The ordinary-directory control uses the same `inner/../clean.rs` spelling
without a repository marker and independently expects counts 1/1/2/1. It
passes, so a repair must preserve legitimate parent components rather than
rejecting all `..` paths.

Evidence for the complete sweep:

- **Boundary/bucket sweep:** nextest `bd74d957-d573-4433-b9a0-3e25f398a4ed`;
  72 incorrect count results across three forbidden boundary shapes, three
  buckets, two selectors and four commands. All 24 ordinary-directory
  control counts passed. The assertion accumulated results before failing.
- **Report sweep:** nextest `9dd2baf9-143f-4456-9853-029787a2af2b`; three
  independently completed boundary tests, each observing 78 incorrect
  reports across both selectors, all four commands and every supported mode.
  Each report incurred source/cache work; uncached JSON also contained one
  selected file. These tests failed their final empty-result assertion;
  they are not passing correctness tests. The runner did not execute the
  already-run bucket test again after these failures.
- **Component/alias sweep:** nextest `64896432-0b46-4c5a-84e3-277aa6fc26ab`;
  24 incorrect counts across dot-plus-parent spelling, directory alias plus
  parents, and an additional file-alias hop, both selectors and four commands.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- |
| R2-F1.S1 — direct target / normalized ancestor / root stopping condition | `inner/../clean.rs`, nested `.git` directory; all buckets and selectors | Counts 1/1/2/1; selected records/indexes and nonzero stats | Empty selection, no per-file cache/source work | Boundary/bucket and nested-directory report sweeps |
| R2-F1.S2 — nested checkout marker reader | Same traversal through a linked checkout with `.git` file | Same extra results | Empty selection | Boundary/bucket and nested-file report sweeps |
| R2-F1.S3 — external-root containment decision | Absolute `B/subdir/../../A/clean.rs`; all buckets and selectors | Same extra results despite traversal through B | Empty selection | Boundary/bucket and external report sweeps |
| R2-F1.S4 — directory-alias ancestor | `bridge/../../clean.rs`, where `bridge → inner/subdir` and inner is a repository | Counts 1/1/2/1 for either selector | Empty selection | Component/alias sweep |
| R2-F1.S5 — recursive file-alias target | `alias.rs → middle.rs → inner/../clean.rs`; middle alias committed, outer alias newly changed | Counts 1/1/2/1 | Empty selection | Component/alias sweep; ensures the extra hop does not supply independent uncommitted evidence |
| R2-F1.S6 — dot/parent spelling | `inner/./../clean.rs` through nested repository | Counts 1/1/2/1 | Empty selection | Component/alias sweep |
| R2-F1.S7 — clean ordinary-directory control | `inner/../clean.rs`, inner has no repository marker; all buckets and selectors | Counts 1/1/2/1 | Exactly those counts; retain legitimate aliases | Boundary/bucket sweep; 24 independently expected controls passed |
| R2-F1.S8 — clean existing boundary controls | Direct foreign targets; nested/external directory back-links without parent cancellation | Empty in all modes | Empty | The nine existing boundary matrices and two back-link tests passed in CLI |
| R2-F1.C1 — `functions` selection/report/cache consumers | Forbidden parent traversal, both selectors | One function in JSON/JSON Lines/full index; count 1, nonzero stats and listing/tree output | Every report empty; zero source/cache work | Report sweeps for all three boundary kinds; bucket sweep for all phases |
| R2-F1.C2 — `types` selection/report/cache consumers | Same | One type, count 1 and nonempty reports | Every report empty; zero source/cache work | Same sweeps |
| R2-F1.C3 — `symbols` selection/report/cache consumers | Same | Two declarations, count 2 and nonempty reports | Every report empty; zero source/cache work | Same sweeps |
| R2-F1.C4 — `classes` selection/report/cache consumers | Same | One type, count 1 and nonempty outline/structured reports | Every report empty; zero source/cache work | Same sweeps; classes has no tree mode |

**Decision trace:** sniff correctly collects alias changes → existence succeeds
→ canonicalization removes the boundary → the normalized-root stopping condition
accepts ownership → `resolve` inserts `clean.rs`'s identity →
[`ChangedFiles::contains`](../../cli/src/changed.rs#L58) matches either selector
→ the intersection in [`main`](../../cli/src/main.rs#L1035) retains the file
→ analysis/cache work runs → declaration selectors and class selection operate
on its current records → summaries and every renderer report the extra result.
Full-index reporting consumes the same selected files. None of these later
decisions rechecks the erased traversal. There is no reference lookup in this
feature that can correct the selection; the faulty stopping condition is in
the ownership check itself.

**Required repair:** Inspect filesystem traversal components and alias targets
before parent components cancel them. Do not stop merely because one normalized
ancestor equals the root while unchecked lexical components remain. Preserve
the paths' actual symlink resolution semantics; lexically stripping `..` first
would erase the same evidence. Enforce external and nested `.git` file/directory
boundaries at that inspection point, then use the canonical identity for matching.
Keep error propagation, one Git observation, legitimate same-checkout aliases,
ordinary `..` paths, package scope and linked-worktree controls.

Add permanent public-result regressions for every R2-F1 instance above, with
both selector spellings and all Git buckets. Verify empty output and zero
source/cache work through all four consumers, and retain independently expected
positive controls. Review the helper's ownership comments and current topic/skill
guidance alongside the repair. No parser, renderer, cache-key redesign, or human
decision is needed.

## Blocked Findings

None. The remaining repair is unblocked.

## Recurrence

R2-F1 belongs to the ownership-normalization class of **review-1.md, R1-F2**.
It is a **newly discovered sibling**, not an unchanged failing original
reproduction or a regression that breaks a previously passing case. The repair
successfully checks direct foreign identities and back-link aliases, but misses
parent cancellation in canonicalization, the normalized-root stopping condition,
and the recursive target check. Those missed decisions feed identity membership,
analysis/cache work, summaries and all declaration consumers. R1-F2 remains
partially fixed at class level; its historical child IDs and verified original
cases remain intact. The instance table above is the next repair inventory.

## Verification and requirement coverage

**Level 1 is appropriate for every requirement in this feature.** The user-visible
change is file selection and existing reports; no new keyboard, mouse, scrolling,
terminal encoding, or styling behavior requires Level 2 or Level 3. No terminal
or browser window was opened.

| Requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Flag syntax, repeated/empty values, equals-only references, command availability, prelude conflict and full JSON compatibility | L1 CLI parser/help tests | Passed |
| Status union, current disk content, cancellation, rename/copy/deletion/recreation and type changes | L1 public API and CLI phase matrices | Passed, including R1-F1 repair |
| Local direct-tip comparison, commit IDs/tags, unborn and detached HEAD | L1 API and CLI repository fixtures | Passed |
| Scope, linked checkout, explicit outside-package source, normal nested repos/submodules | L1 CLI fixtures with exact expected files | Passed for enumerated cases; R2-F1 prevents ownership-class closure |
| Ignore/fixture/language/file/name/exclusion intersection | L1 CLI selector fixtures | Passed |
| Every empty mode and zero source/cache work | L1 CLI assertions | Passed for valid empty selections and original ownership controls; fails for R2-F1 |
| Missing/bare/corrupt repository, bad references, conflicts and ownership inspection errors | L1 API/CLI failure assertions | Passed |
| Cache reuse across ordinary/changed scans and legitimate aliases | L1 CLI assertions of actual warm hits and independent declarations | Passed |
| Native identities, deterministic deduplication, spaces, Unicode and case spellings | L1 path fixtures | Passed on this host; non-UTF-8 filesystem case remains platform guarded |
| One status observation, no rich diff/history/network/inventory work | L1 API work counters plus code inspection | Passed |
| Documented repository-boundary traversal | L1 shipped CLI reproductions | Missing parent-component regressions; R2-F1 fails |

Cargo declares the changed CLI target through `tests/changed/main.rs`, including
`ownership` and `type_changes`; sniff declares its Git unit-test module. Their
ordinary names select L1, and nextest executed them. Both
`just check-tier-coverage tree-hugger` and `just check-tier-coverage sniff`
reported **zero stranded tests**. The temporary review probes used that same
declared L1 target and were removed after collecting evidence; no implementation
or permanent test changes were made by this review.

The input robustness matrix does **not apply**: the feature and repairs consume
typed gix observations and filesystem paths, adding no manifest/configuration
parser or file-format schema. Existing Git corruption/reference failure checks
were still run. The review checked the shared status classifier's three readers,
all request/count projections, the one ownership insertion point, identity
membership, selection-before-analysis ordering, report consumers and cache-key
policy. The CLI README, symbol-report topic, sniff API topic and dependency/skill
guidance were inspected against those contracts; the remaining ownership defect
also contradicts the documented boundary rule.

This review ran on macOS. Cross-OS execution evidence is left to CI and is not
a readiness finding. The repair log's earlier full-area and Linux results are
recorded claims, not substituted for this review's independent focused runs.

## Nonblocking Recommendations

None. The required work is the ownership repair and its complete regression
inventory above.
