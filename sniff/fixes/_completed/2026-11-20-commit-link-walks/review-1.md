---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Required Windows Git startup optimization is missing"
  - "R1-F2 (high): Root-package status selection hides history and changes"
  - "R1-F3 (high): Dirty package reports read arrays absent from ordinary requests"
human_review: true
human_review_items:
  - "R1-F2: root `-p` now selects the whole repository in git-status (matches dirty-files); recent-commits and package attribution give the root only unclaimed paths. Keep, or make root selection exclusive?"
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-21T20:05:04+00:00"
spec: 2026-11-20-commit-link-walks/spec.md
implemented: true
description: "A **fix** review of `2026-11-20-commit-link-walks/spec.md`"
fix: 2026-11-20-commit-link-walks/review-1.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/sniff/fixes/2026-11-20-commit-link-walks/log.md
implemented_by: claude/opus
next: 2026-11-20-commit-link-walks/review-2.md
---

# Review 1: Commit link walks

The fix is not production ready. Shared ancestry pruning and measured status
preparation pass their focused tests, but required Windows startup work is
missing. Manual CLI checks also expose two incorrect reports for ordinary
workspace inputs. All three findings have a recommended repair and need no
new author decision.

## Implementation decisions (informational)

Two parallel readers independently examined the implementation log and plan,
including `decisions_for_review`, against the spec and author rulings.

- Skip the selected worktree's extra ref snapshot at history count zero.
  Nothing is displayed or linked; keep this inexpensive optimization.
- Retain generic status text and JSON branches behind the focused dispatch.
  Keep them as functioning fallback projections.
- Put status tests in an attached sibling test module rather than inline.
  Keep the existing repository convention; test discovery works.
- Use owned object IDs for completed-set membership, avoiding a possible
  mismatch between the owned and borrowed types' hashes. Keep this choice.
- Box the selected worktree handle after Clippy identified the large enum
  variant. Keep the smaller enum.
- Use a private filesystem mode and companion link map, conservative rules
  for sharing worktree refs, one selected-worktree status scan, and separate
  initial/selection/link timing stages. These are reasonable implementations
  of the specified library preparation and observation reuse.
- The public status option/result types, shared library selector resolver,
  and selection error implement the focused library operation. Their concrete
  design was recorded in the plan; no conflicting caller requirement emerged.
- Compare performance at different source commits while holding remote refs
  and the unpushed-history workload shape fixed. Comparing linked-minus-JSON
  visits accounts for the changed initial history depth. Keep that method.
- Accept the recorded Linux median gap of 112 ms against the 500 ms limit;
  no additional timing breakdown was needed after the target was met.
- Reuse unchanged dependent checks and narrow verification to relevant
  consumers. This is appropriate verification scope.

The first three items were explicitly flagged for review. None merits a
human interruption. Cross-OS samples and checks were left to CI because the
implementation recorded no configured rigs. Missing cross-OS evidence is
excluded from readiness by this review's instructions. In this review session,
`BUILD_WIN`, `BUILD_WSL`, and `BUILD_MACOS` are set; that does not establish
what was available during the earlier implementation runs.

The Windows code change was omitted from the four-phase plan, without a
recorded authorized deferral. That is required implementation work, addressed
by R1-F1, rather than a request to reconsider scope.

## Findings

### R1-F1 (high): Required Windows Git startup optimization is missing

**Authority: spec**, “Windows: name the Git installation,” steps 2–4, and
author ruling 3: “When the saving is 10 ms or more, move `wt`'s `EXEPATH`
helper into sniff, call it from a plain `main` before the Tokio runtime
starts, and switch `wt` to the moved copy.” The same ruling records the
repeated branch-build saving as approximately 50–120 ms, clearing that gate.

**Defect class:** a required, measurement-gated execution path was omitted
from the implementation plan and code.

Sniff still starts through [`#[tokio::main]`](../../cli/src/main.rs), with
no installation hint. The helper and its installation lookup remain owned
by `worktree`. Searching Sniff's program and process code finds neither
helper nor any `EXEPATH` handling. Consequently the specified startup
optimization is absent for an ordinary native Windows PowerShell invocation
without the two Git installation variables. This is a missing implementation,
not a finding about unavailable cross-OS proof.

| Child ID | Site | Shape checked | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Sniff library program lookup | Ownership of the two helpers and installation lookup | Remain solely in `worktree/lib/src/git/program.rs` | One implementation owned by Sniff |
| R1-F1.2 | Sniff main binary | First repository open with no caller hint | Tokio runtime starts before any hint; no hint call exists | Set the hint from plain main before threads |
| R1-F1.3 | Sniff process launch boundary | Environment of a child after an internally set hint | No cleanup hook exists at `run_command_with_timeout` | Remove only the internally supplied hint |
| R1-F1.4 | Worktree startup and command helper | Existing initialization and child cleanup | Existing calls are present and remain local | Keep their behavior while switching to the shared helpers |
| R1-F1.5 | Sniff fixture binaries | Other declared binaries | Render fixtures use synthetic Git facts; foreground fixture does not open a repository | Clean: no repository-open startup path to initialize |

The table is a source/contract sweep, not a claimed Windows runtime run.
Complete the specified helper move, initialization, and child cleanup; retain
caller-provided `EXEPATH`/`GIT_EXEC_PATH` behavior and off-Windows no-ops.
Add the specified process-observation and child-environment tests, and retain
Worktree's existing tests. No new design decision is necessary.

### R1-F2 (high): Root-package status selection hides history and changes

**Authority: defect.** In Sniff's newly moved
[`resolve_package_and_area`](../../lib/src/filesystem/repo/scope.rs), used to
turn a selected package into a Git path prefix, the package path always gains
`/`. A realistic Cargo workspace explicitly listing `"."` as a member has a
root package with an empty relative path. Selecting that package produces
`"/"`, which matches no repository-relative Git path. The CLI reports no
history and a clean working tree despite changes in `src/lib.rs`.

**Defect class:** an empty repository-root identity is converted into an
absolute-looking prefix, then used to exclude valid relative paths.

Reproduction: create a temporary Git repository with one commit and this root
manifest, plus valid root and member `src/lib.rs` files and a member manifest:

```toml
[package]
name = "root-app"
version = "0.1.0"

[workspace]
members = [".", "area/member"]
```

Modify both source files. `sniff --base <fixture> repo packages --json`
recognizes both packages. Unscoped status shows the commit and changes;
`repo git-status --json -p root-app` instead returns `recent: []`, no
`file_changes`, and `status.is_dirty: false`. Compact text likewise hides the
commit and root source changes. This predates the move; it is reported as a
defect, not claimed as a regression.

| Child ID | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | Package-prefix resolver and path-history reader | Root member, one committed history entry | `"/"` prefix; zero history | Root-relative empty prefix; history retained |
| R1-F2.2 | Working-tree narrowing and status recount | Root source modified, staged, and untracked | All changes removed; clean state and zero counts | Retain root changes and report dirty |
| R1-F2.3 | Final link selection and rendering | Root history is remotely contained | Earlier empty history removes all link candidates; compact text hides the commit | Keep the displayed root commit eligible for its captured link |
| R1-F2.4 | Nested package selection | Same fixture, `-p member` | History and member changes retained | Clean control |
| R1-F2.5 | Area selection and package/area intersection | `-a area`; `-p member -a area` | Correct member history and changes | Clean controls; root's empty area is intentionally excluded by the area resolver |
| R1-F2.6 | Later branch override | `-p root-app --branch side` | Branch history restored and changes cleared | Clean: later branch replaces earlier selection |
| R1-F2.7 | Package and area catalog handlers | `packages -p root-app`; `package-areas -p root-app` | `['root-app']`; `['']` | Clean: these callers only validate the resolver result and filter by identity |
| R1-F2.8 | Remaining generic validation consumers | Structure, dependency, and changed-package actions | Returned prefix is discarded, as at R1-F2.7 | Clean by contract: no prefix filtering through this return value |

Return an empty prefix for the root package and retain trailing slashes for
non-root packages. The existing path-history reader already supports an empty
prefix. Add a public status preparation regression using this root-member
fixture, asserting retained history, changes, counts, and full-ID link keys.
This needs a small normalization fix, not a new path abstraction.

### R1-F3 (high): Dirty package reports read arrays absent from ordinary requests

**Authority: defect.** Sniff CLI's existing
[`dirty_package_names`](../../cli/src/output/filesystem/packages.rs) determines
which packages have uncommitted changes using only `status.dirty` and
`status.untracked`. The ordinary command requests file changes without
patches; Sniff's library deliberately leaves those two detail arrays empty
unless patches are requested. Thus `repo dirty-packages --json` and
`repo dirty-package-areas --json` return empty names for ordinary modified,
staged, or untracked files. This is existing code reached while sweeping
consumers of the moved selector, outside the implementation's changed sites.

**Defect class:** a projection treats unrequested detail data as evidence
that the underlying set is empty.

Use the R1-F2 fixture, with both packages' source files staged and then
modified again, plus an untracked source file in each package. The staged and
unstaged reports identify both packages correctly. Both dirty reports return
empty names, with or without `-p member`. The nested-package control proves
this failure is independent of root selection.

| Child ID | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F3.1 | Dirty package collector / JSON projection | Root and nested package; staged + unstaged + untracked | Empty `names` for both package selections | Selected changed package named |
| R1-F3.2 | Dirty area projection | Same files and package selections | Empty `names` | `['']` for root; `['area']` for member |
| R1-F3.3 | Dirty package and area text renderers | Nested package, ordinary text | Empty main content; exit 1 | Member name / area name; successful result |
| R1-F3.4 | Staged package and area projections | Same fixture and selections | Correct root/member names and areas | Clean: use `file_changes` |
| R1-F3.5 | Unstaged package and area projections | Same fixture and selections | Correct root/member names and areas | Clean: use `file_changes` |
| R1-F3.6 | Current-area dirty and source-change decisions | Base inside changed `area/member` | `dirty: true`; `has_source_code_changes: true` | Clean: both use `file_changes` in addition to optional detail arrays |
| R1-F3.7 | Dirty package/area clean controls | Reset tracked files and remove untracked files | Empty `names` | Clean: empty is correct only here |

Read changed paths from the already collected `file_changes`, following the
staged/unstaged sibling projections. Preserve the current package ownership
rules, include untracked and conflicted paths, and keep identity-only status
indeterminate/empty as before. Do not enable patch collection to populate the
old arrays: that would restore unnecessary work. Add a CLI wiring test with
ordinary collection and a library/result-level test of the projection rather
than only manufacturing populated detail arrays.

## Verification and requirement coverage

Reviewed implementation commit `45075c7a4` and the current files after the
merge at `c0385f1d7`. Existing edits to the plan, spec, and implementation log
were preserved. No production code was changed during review.

```sh
cargo nextest run --color=never -p sniff --features remote \
  -p sniff-detection-cli \
  -E 'test(commit_links::) | test(status_report::) | test(test_repo_git_status_) | test(captured_links_match_full_ids)'
```

Result: **51 passed**, 3,814 excluded by the filter; longest test 0.149 s.
No selected test exceeded five seconds. Two attempts to forward the compound
filter through `just test` failed in shell argument expansion before running
tests; the direct nextest command above supplied the targeted evidence.
The implementation also records full Sniff L1 and strict Clippy runs with no
slow tests. Those historical runs were inspected, not repeated wholesale.

| Requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Shared visits, exact budget, ordering, duplicates, single-tip behavior | L1 real repository fixtures and exact counters | Passing |
| Failure fallback and subsequent revisit | L1 injected failure followed by real ancestry walking | Passing |
| Shallow parity and missing-snapshot uncertainty | L1 library results and counter comparison | Passing |
| Final selector precedence, detached HEAD, worktree config and ref reuse | L1 public preparation and spawned CLI fixtures | Passing for existing fixtures; root-member gap is R1-F2 |
| Full-ID association and no Git reads in renderer | L1 renderer markup and zero work counters | Passing; spec explicitly selects this boundary |
| Plain/zero-history/JSON skip linking; final perf snapshot | L1 spawned CLI, stage checks and visit deltas | Passing |
| Linux speed within 500 ms of JSON without commit-graph | Recorded external release timings: 256 ms text, 144 ms JSON | Target met in recorded workload |
| Windows startup and child environment | Required production path and tests absent | R1-F1 |

This fix changes collection and supplies existing renderer input, without a new
key, mouse, or terminal-encoding requirement. The spec explicitly asks for
renderer-boundary markup tests; no new Level-2/Level-3 requirement is inferred.
The input robustness matrix does not apply: no file-format/configuration
loader or field validator was added or changed.

Manual reproductions use disposable repositories and the shipped CLI. The
local repeatable script and captured output are under the ignored
`target/commit-link-walks-review-1/` directory. A fresh
`cargo build --color=never -p sniff-detection-cli --bin sniff` succeeded;
`python3 target/commit-link-walks-review-1/reproduce.py` then reproduced both
reporting defects with that build and verified the controls in the tables.
Cross-OS evidence is not used
as a readiness blocker. No earlier review findings exist for this iteration.
