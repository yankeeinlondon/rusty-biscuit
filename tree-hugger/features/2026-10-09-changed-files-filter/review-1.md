---
$schema: feature-review.yaml
ready: false
findings:
  - title: "R1-F1 — Unstaged file-type changes disappear from changed-file selection and status"
    priority: high
  - title: "R1-F2 — Symlink identities admit files from another repository"
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-07T20:41:34-07:00"
spec: 2026-10-09-changed-files-filter/spec.md
implemented: true
description: "A **feature** review of `2026-10-09-changed-files-filter/spec.md`"
feature: 2026-10-09-changed-files-filter/review-1.md
log: 'tree-hugger/features/2026-10-09-changed-files-filter/log.md'
implemented_by: codex/gpt-6.1-sol
next: 2026-10-09-changed-files-filter/review-2.md
---

# Changed Files Filter — Review 1

The feature is **not ready for production**. Two high-priority defects violate
the file-selection contract: unstaged changes between regular files and
symlinks are missed, and a changed symlink can admit source owned by another
repository. Both are reproducible through the shipped CLI. Neither requires
a human design decision before repair.

This is the first implementation review of this feature. There are no previous
findings to carry, partially repaired findings, or earlier child IDs to reconcile.
The specification, plan, implementation log, sniff API, CLI selection, reports,
tests, and required documentation were reviewed. Existing staged work was preserved.

## Findings

### R1-F1 — High: Unstaged file-type changes disappear from changed-file selection and status

**Defect class:** Status classifiers silently discard a valid Git change category,
so selection and summaries lose files whose filesystem type changed.

In the sniff package,
[`classify_status_paths`](../../../sniff/lib/src/filesystem/git/status.rs#L109)
collects staged, unstaged, untracked, and conflict evidence for both the new
path-only API and existing rich status. Its index-to-worktree match handles
`Removed` and `Modification`, but drops `Change::Type` through the wildcard arm
at line 161. The pinned gix status enum explicitly uses that variant for a
symlink becoming a regular file or the reverse. The separate fast dirty check
and detailed count reader repeat the omission at lines 1063 and 1103.

This violates the specification's requirement that Git-reported file-type
changes count. The CLI returns a successful empty report even when the user
explicitly selects a present, eligible changed source file. The problem is
missing results, rather than an error message or display issue.

**Reproduction:** Use the existing temporary-repository helpers in
[`changed/selection.rs`](../../cli/tests/changed/selection.rs) and
[`changed_paths_tests.rs`](../../../sniff/lib/src/filesystem/git/changed_paths_tests.rs).
Commit `leaf.rs` as a regular file and `target.rs` containing `fn target() {}`.
Remove `leaf.rs`, create a symlink named `leaf.rs` pointing to `target.rs`, and
leave that replacement unstaged. Run
`hug symbols --changed --file leaf.rs --count`: expected `1\n`, observed `0\n`.
For the reverse case, **stage and commit the symlink first**, then replace it
with a regular `leaf.rs` containing `fn restored() {}` without staging.
Expected `1\n`, again observed `0\n`. The sniff public API returns no changed
paths for either case, although `leaf.rs` must be present in the result.

The same fixtures were swept through every status reader and the committed
comparison. Child IDs identify the required repair or control sites.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- |
| R1-F1.S1 — path collector → `GitRepo::changed_paths` | Both unstaged type-change directions | `[]` in both cases | `["leaf.rs"]` | Public API diagnostic sweep, nextest run `248cfc1e-cc32-4b0e-b0ff-4d688ae69a18` |
| R1-F1.S2 — rich collector → `file_changes` / `repo_status` | Both unstaged directions | No file changes; dirty false; unstaged count 0 | One modified unstaged file; dirty true; unstaged count 1 | Same fixture and run as S1 |
| R1-F1.S3 — `is_repo_dirty` → minimal/summary requests | Both unstaged directions | `false` | `true` | Same fixture, direct reader observation; `detect_with_request` routes minimal requests to this reader |
| R1-F1.S4 — `get_repo_status_counts_detailed` → count-only requests / `get_repo_status_counts` | Both unstaged directions | `(false, 0, 0, 0)` | `(true, 0, 1, 0)`; total 1 | Same fixture, direct reader observation; total-count helper only sums these counts |
| R1-F1.S5 — staged tree/index classification | Both directions after staging | Changed path present; staged count 1; dirty true | Same | Same API sweep; clean control |
| R1-F1.S6 — committed `tip_diff_paths` | Both directions after committing, compared with previous commit | Changed path present; current status clean | Same | Same API sweep; clean control |
| R1-F1.S7 — conflict detection | Unmerged entries, including outside the selected package | Request fails before output | Same | Existing `index_conflicts_anywhere_fail_naming_every_path` and CLI `conflicts_outside_the_package_still_fail`; the conflict arm precedes type classification |
| R1-F1.C1 — `functions` | Unstaged regular→symlink; target contains one function and one struct | Count 0, empty JSON/JSON Lines/full indexes/stats | One function; one selected file/index; nonzero function stats | CLI report sweep, run `b6e2efc2-f540-4c5a-a7fe-92f7a33154e8` |
| R1-F1.C2 — `types` | Same | All five modes empty | One type; one selected file/index; nonzero type stats | Same report sweep |
| R1-F1.C3 — `symbols` | Same | All five modes empty | Two declarations; one selected file/index; nonzero stats | Same report sweep |
| R1-F1.C4 — `classes` | Same | All five modes empty | One class-report type; one selected file/index; nonzero type stats | Same report sweep |

**Decision trace:** gix emits the type-change evidence → the status classifier
drops it → the union in `changed_paths` has no identity for `leaf.rs` → the CLI
intersection removes an ordinarily eligible file → analysis and per-file cache
lookup never run → declaration filtering, count/stats, and all renderers see an
empty selection. Full-index reporting uses the same narrowed file list and
cannot recover the missing file. There is no reference lookup or candidate
search in this selector that could repair the dropped evidence later.

**Repair:** Classify `Change::Type` as an unstaged modification in all three
status readers, preferably sharing the classification rule while preserving
the dirty reader's early stop. Add public-result regressions for both
directions, with staged and committed controls. Pin the CLI result as well as
the path API and status summaries. Keep regular scanner eligibility: an
ordinary directory walk excludes symlinks, while an explicit file selector
can already resolve one.

The fast dirty/count omissions are pre-existing sibling defects outside the
new path API's direct implementation, but are included here because repairing
only the new collector would leave conflicting answers for the same status
evidence. Existing rich status now shares the classifier changed by this feature.

### R1-F2 — High: Symlink identities admit files from another repository

**Defect class:** Filesystem identity normalization loses repository ownership,
allowing an alias to transfer changed-file eligibility across a repository boundary.

In tree-hugger-cli,
[`resolve`](../../cli/src/changed.rs#L93) joins Git paths to the checkout root,
checks existence, and immediately inserts their canonical `file_identity` into
a set. In the tree-hugger library,
[`file_identity`](../../lib/src/file/source_file.rs#L23) deliberately resolves
symlinks. Neither resolution nor membership verifies that the resulting file
still belongs to the discovered checkout or avoids an intervening nested
repository. The source comment saying the intersection removes files outside
the repository therefore promises behavior the code does not enforce.

**Reproduction:** Create independent committed fixture repositories A and B.
B's unchanged `leaf.rs` contains `fn foreign() {}` and `struct Foreign;`.
In A, create an untracked `alias.rs` symlink to B's `leaf.rs`. Invoke from A:
`hug symbols --changed --file <absolute-B-leaf.rs> --count`.
Expected `0\n`, observed `2\n`; JSON reports B's source using a `../…/leaf.rs`
path, with `errors: []`. Using A's alias as the explicit selector also admits
B. This contradicts the same-repository rule even though no second discovery
occurs. Repeat with B initialized under A as `inner/`: canonical containment
inside A's root alone would still admit a different repository.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- |
| R1-F2.S1 — external identity insertion / membership | A's changed alias targets an independent external repo; select alias or target | Both report foreign source | Both empty | CLI identity sweep, run `b62df972-25ae-474e-9b68-3c2a4331e471` (one-function fixture: count 1 versus expected 0) |
| R1-F2.S2 — nested repository boundary | Changed alias targets `inner/leaf.rs`; select alias or target | Both report nested source | Both empty | Same sweep and independently initialized nested fixture |
| R1-F2.S3 — legitimate same-checkout aliases | Changed alias targets A's own `clean.rs`; select alias or target | One declaration for either spelling | Same; preserve shared identity | Same sweep; clean control |
| R1-F2.S4 — normal foreign file without an alias | Explicit source in another repo | Empty | Same | Existing `an_explicit_file_in_another_repository_is_not_selected` passed |
| R1-F2.S5 — ordinary nested repository and submodule | No changed alias into either boundary | Contents excluded | Same | Existing `a_nested_repository_is_not_descended_into` and `submodule_contents_are_never_selected` passed |
| R1-F2.S6 — package scope / linked checkout | Same-repo explicit source outside package; active linked worktree | Source selected in correct checkout | Same | Existing outside-package and linked-worktree CLI tests passed; canonical root/ownership repair must retain these controls |
| R1-F2.C1 — `functions` | External target with function + struct | Count 1; foreign JSON/JSON Lines record, index, and function stats | Empty in every mode | CLI report sweep, run `efa8ece0-ce2c-4a5b-b6dd-d65767c49960` |
| R1-F2.C2 — `types` | Same | Count 1; foreign type record/index/stats | Empty in every mode | Same report sweep |
| R1-F2.C3 — `symbols` | Same | Count 2; two foreign records, index, nonzero stats | Empty in every mode | Same report sweep |
| R1-F2.C4 — `classes` | Same | Count 1; foreign type and members envelope/index/stats | Empty in every mode | Same report sweep |

**Decision trace:** Git correctly reports A's untracked alias → canonicalization
turns that candidate into B's identity → the ordinary explicit selector also
resolves to B's identity → membership accepts it → B's source is read and
analyzed → declaration filtering, class selection, count/stats, and full-index
output all include foreign evidence. None of those downstream consumers checks
checkout ownership. Staged and committed candidates feed the exact same
unconditional identity conversion at lines 97–102, so they require the same
ownership guard; the defect is independent of which union bucket supplied the alias.
Terminal/plain/table/tree rendering likewise consumes the already accepted
files and has no boundary guard; its renderer should not become responsible
for repairing file selection.

**Repair:** Validate canonical candidate ownership against the active checkout
before adding an identity. Account for nested `.git` boundaries as well as
external targets, without discovering a second repository or scanning
submodules. Preserve native identity matching and legitimate aliases inside
the checkout. Add external and nested alias regressions with both selector
spellings, plus the same-checkout positive control. Review the repository-boundary
comments alongside the fix.

## Verification and requirement coverage

The appropriate verification level is **Level 1**: hermetic local repositories,
public library calls, and CLI subprocesses. This feature adds no keyboard,
mouse, terminal-encoder, scrolling, or new styling behavior requiring Level 2
or Level 3. No terminal or browser window was opened.

| User-facing requirement | Strongest relevant verification | Outcome |
| --- | --- | --- |
| Flag syntax, repetition, equals-only reference, command availability, prelude conflict, full JSON compatibility | L1 CLI parser tests | Passed |
| Status union, current disk content, cancellation across stages, copies, renames, recreated deletions | L1 public API and CLI fixtures | Passed for covered cases; file-type hole is R1-F1 |
| Direct branch-tip comparison, commit IDs/tags, unborn and detached HEAD | L1 public API and CLI fixtures | Passed |
| Package scope, linked worktree, explicit outside-package files, nested repos/submodules | L1 CLI fixtures | Passed for covered cases; alias hole is R1-F2 |
| Ignored files, fixture overrides, file/name/exclusion/language intersection | L1 CLI fixtures | Passed |
| Every empty-report mode and zero per-file cache work | L1 CLI assertions | Passed |
| Invalid/bare/missing repository, invalid/non-commit reference, corrupt Git data, conflicts outside package | L1 CLI/API failure assertions | Passed |
| Cache reuse across ordinary/changed scans | L1 CLI assertions of actual warm hits | Passed |
| Native paths, ordering, spaces, Unicode, case aliases | L1 path fixtures | Passed on this host; non-UTF-8 worktree case has a documented macOS filesystem guard |
| One status observation; no optional line diff, inventory, history, or network enrichment | L1 focused work assertions plus code inspection | Passed |

Commands run before temporary review probes:

- In `tree-hugger`: `just test --test changed` — **48 passed**.
- In `sniff`: `just test changed_paths` — **32 passed**, including 26 new
  changed-path tests and six neighboring blast-radius tests.
- At repository root: `just check-tier-coverage tree-hugger` — **0 stranded tests**.

Cargo discovered the CLI `tests/changed/main.rs` target and its six declared
modules; the sniff tests are declared under `#[cfg(test)]` in the Git module.
Their names select L1, and nextest executed them. No undeclared or incorrectly
tiered feature test was found.

Additional temporary tests reused those fixture builders and were run through
the same `just test` recipes. The assertion sweep failed as expected on the
six incorrect selection results listed above; diagnostic sweeps printed the
public results across both type directions, three status phases, all status
readers, and all four commands in five report modes. Diagnostic tests returning
success are observations, not passing correctness assertions. Temporary probes
were removed after recording their results; no implementation fixes were made.

The input robustness matrix for newly authored file-format/configuration
parsers does not apply: this feature consumes typed gix status/tree observations
and adds no manifest, lockfile, configuration deserializer, or file-format
schema. Reference validation and Git corruption are nevertheless covered by
the failure tests. No permissive JSON/YAML/TOML reader was introduced.

Cross-OS execution evidence remains CI's responsibility and is not a readiness
finding. This review ran on macOS. The two findings are behavioral reproductions,
not requests for additional CI cells, benchmarks, or human approval.

## Nonblocking recommendations

None beyond the focused repairs and regression tests above. The shared status
walk, selection-before-analysis ordering, optional-feature dependency, cache
key policy, and documented local tip comparison are appropriate for the scope.
