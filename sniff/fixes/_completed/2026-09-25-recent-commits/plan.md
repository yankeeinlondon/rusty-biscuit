---
total_phases: 3
created: 2026-09-26
phase: 3
agent: codex/default
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - sniff/fixes/2026-09-25-recent-commits/plan.md
docs_created_during_phase_1:
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/filesystem/git/recent_commits/collect.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/tests/l1/recent_commits.rs
  - sniff/cli/tests/l1/cli.rs
docs_updated_during_phase_2:
  - sniff/docs/topics/repo/recent-commits.md
  - sniff/docs/topics/repo/recent-commits-schema.md
  - sniff/docs/cli/repo_recent-commits.md
  - sniff/cli/README.md
  - sniff/lib/README.md
  - sniff/fixes/2026-09-25-recent-commits/plan.md
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/architecture.md
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - sniff/fixes/2026-09-25-recent-commits/plan.md
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
  - sniff/fixes/2026-09-25-recent-commits/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
  - sniff/lib/src/filesystem/git/recent_commits/collect.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/tests/l1/recent_commits.rs
  - sniff/cli/tests/l1/cli.rs
documentation:
  - sniff/docs/topics/repo/recent-commits.md
  - sniff/docs/topics/repo/recent-commits-schema.md
  - sniff/docs/cli/repo_recent-commits.md
  - sniff/cli/README.md
  - sniff/lib/README.md
  - .claude/skills/sniff/architecture.md
  - sniff/fixes/2026-09-25-recent-commits/plan.md
  - sniff/fixes/2026-09-25-recent-commits/implementation-log.md
completed_phase: 3
implemented: true
packages:
  - sniff
  - sniff-cli
---

# Recent Commits: Package-Area Filtering Plan

## Phase 1: Establish Boundaries and Regression Evidence

### Necessary Rules

- The specification for `2026-09-25-recent-commits` governs this change. Area membership comes from file location within a recognized area, independently of package ownership. Commit scopes and incidental path-name matches confer no membership.
- Preserve deepest-package ownership for `--package`. Shared area files receive an area but no invented package. Keep existing case-insensitive selector validation, typed errors, AND semantics between filter kinds, and non-monorepo behavior.
- Preserve parent-area selection of nested areas. Proposed ruling for overlapping area attribution: retain the owning package's area when a package exists; otherwise choose the deepest containing recognized area deterministically. Confirm this against the boundary spike before implementation; do not inherit unordered `HashSet` iteration as a contract.
- Keep the empty root area special: it must not become a fallback that attributes every repository file. Preserve existing root-package ownership semantics.
- Use the current structure catalog, as today; reconstructing historical package layouts is outside scope. Resolve historical changed paths lexically, including deleted and moved paths, without requiring those files to exist. Preserve attribution from both endpoints of a move.
- Filtering selects commits, not individual files. Count means matching commits, and surviving recent-commit records retain their complete changed-file lists. Existing category projections remain separate.
- Business logic remains in the library. No new dependencies, network access, CLI-side path inference, or rendering redesign is expected.

This work separates area resolution from package ownership in the existing collection pipeline and updates library regressions, CLI regressions, and documentation. Success means the equivalent of planning-only commit `1634e6e` matches `--package-area worktree`, fails `--package worktree`, reports `packages: []` and `package_areas: ["worktree"]`, and does so consistently across macOS, Linux, native Windows, and WSL2. Results must retain exact identities, newest-first ordering, full file lists, and count-filling behavior without widening discovery costs.

Current implementation evidence: `PackageCatalog` in `sniff/lib/src/filesystem/git/recent_commits/collect.rs` stores area member package indices; both `matches` and `attribute` depend on `owner`. `RepoInfo::package_area_for_dir_with_index` already has an area-directory fallback, but its overlapping-area iteration is unordered. Existing library tests and both recent-commit topic documents explicitly encode the old unattributed-shared-file behavior and need deliberate updates.

Waves are numbered globally. Tasks within a wave may run concurrently in separate subagents with the stated file ownership; a wave completes before dependent waves begin. All implementation agents must operate non-interactively and preserve unrelated working-tree edits. This plan does not authorize commits, pushes, or lifecycle moves.

### Wave 1: Boundary Investigation

- [x] **Boundary spike**
  - Inspect `PackageCatalog`, `RepoInfo::package_area_for_dir_with_index`, and `PackageOwnershipIndex` in `sniff/lib/src/filesystem/repo/{types,ownership}.rs`. Trace normalization, package-first precedence, nested area selection, and root-area behavior.
  - Compare existing directory resolution with lexical historical-file resolution using shared files, nested areas, deleted files, and similarly named siblings. Check a filename equal to an area directory name: a file outside that directory must not count as inside it.
  - Record the smallest reuse strategy: use the existing indexed resolver if it meets the contract, or extract a crate-private shared boundary helper if necessary. Avoid rebuilding an index or canonicalizing each changed path. Do not introduce a new public API solely for this fix.
  - Deliverable: a short decision record in this plan resolving the proposed nested-area ruling and naming the exact helper/files to change. This is a bounded code/fixture spike, not a performance benchmark or CI experiment.

- [x] **Regression blueprint**
  - Independently inspect `sniff/lib/tests/l1/recent_commits.rs` and recent-commit tests in `sniff/cli/tests/l1/cli.rs`; specify fixture commits and expected hash sequences for Wave 2. Do not edit production files during this task.
  - Use a disposable Cargo workspace with `worktree/lib`, `worktree/cli`, a second area, and commits isolated to fixes/specs/plans, reviews, an area README, library code, and CLI code. Interleave nonmatching commits newer than and between matches; include one mixed-area commit.
  - Include negative paths such as `worktree-other/README.md` and `.claude/skills/worktree/SKILL.md`, with a misleading `planning(worktree)` subject. Include a matching area commit without a matching conventional scope.
  - Capture hashes at creation and use deterministic distinct commit timestamps. Reproduce the paths and planning-only shape of `1634e6e`; no automated test may require that object or the developer checkout's history.

**Checkpoint:** The spike resolves boundary behavior and identifies a minimal implementation seam; the blueprint maps every acceptance criterion to a library or CLI assertion. Record any remaining specification ambiguity explicitly before dependent implementation.

### Wave 1 Decision Record (2026-09-26)

**Reproduction.** Against this repository with the installed `sniff`, `1634e6e`
is among the newest 60 unfiltered commits with `packages: []` and
`package_areas: []`, and is absent from 200 matches of
`recent-commits 200 --package-area worktree` (and, correctly, from
`--package worktree`).

**Findings from the existing resolvers.**

- `Package::package_area` is always `Path::parent` of `relative`
  (`make_package_area` in `repo/detection.rs`), so a package's area directory
  always contains that package. `relative` is `/`-separated.
- `PackageOwnershipIndex::lookup_relative` joins the Git path onto the
  canonical root and walks parent components through a `HashMap`: lexical,
  component-bounded (`crates/pkg-a20` is not `crates/pkg-a`), no filesystem
  access, so deleted and moved paths resolve. It also resolves a path *equal*
  to a package directory; that is pre-existing and out of scope.
- `RepoInfo::package_area_for_dir_with_index` does **not** meet the contract
  for per-file use and is not reused as-is:
  1. its fallback iterates a `HashSet` of areas and returns the first
     `starts_with` hit, so a file under a nested area (for example
     `claudine/rendezvous/README.md`, contained by both `claudine` and
     `claudine/rendezvous`) resolves nondeterministically across processes;
  2. it allocates a `HashSet` and joins every area on every call, which is
     per-changed-path work in the history walk;
  3. `Path::starts_with` accepts equality, so a historical *file* named exactly
     like an area directory (e.g. a root file `worktree` deleted before the
     `worktree/` area existed) would count as inside the area.
  It correctly skips the empty root area and is component-bounded
  (`worktree-other/README.md` is not under `worktree`).
- Real overlaps exist in this repository: `darkmatter/dmls` is both the `dmls`
  package directory (area `darkmatter`) and the area of `zed-dmls-cli`;
  `tabby` is a top-level package (area `""`) and an area directory. The
  existing library fixture reproduces the same shape (`crates/alpha` is both a
  package and an area). Nested areas: `claudine/rendezvous`,
  `darkmatter/dmls`, `homelab/server`.
- A root `[package]` alongside `[workspace]` is not a catalog entry (verified
  in a disposable repository: its `src/main.rs` stays unattributed), so no
  package can own every shared area file.
- Windows: Git paths stay `/`-separated; `PathBuf::push` onto a `\\?\` verbatim
  root re-parses the pushed path non-verbatim, which is why the existing
  ownership join already works there. Area keys built from the same root with
  the same join behave identically. Rust `Path` comparison is case-sensitive
  on every OS; area boundaries follow the catalog's spelling, while selector
  validation stays ASCII case-insensitive.

**Ruling: one per-path area resolver shared by filtering and attribution.**

For each changed path (both endpoints of a move):

1. `owner` = deepest owning package (existing `PackageOwnershipIndex`).
2. `area` = the owner's `package_area` when an owner exists (package-first,
   consistent with `Package::package_area` and `sniff repo packages`);
   otherwise the **deepest** recognized non-empty area directory that
   **strictly** contains the path (walk from the path's parent upward);
   otherwise none. The empty root area is reachable only through a top-level
   package owner, never as a directory fallback.
3. Attribution inserts `owner`'s name (if any) and `area` (if any).
4. `--package X` matches when any path's owner is X (unchanged).
   `--package-area A` matches when any path's `area` equals A or starts with
   `A/` (ASCII case-insensitive, preserving parent-area selection and the
   existing `UnknownPackageArea` validation from catalog areas). Selecting
   `""` still matches only top-level-package files.

Consequences, stated so review can check them: `crates/alpha/README.md`
reports `packages: ["alpha"]`, `package_areas: ["crates"]` and is not selected
by `--package-area crates/alpha`; the proposed plan ruling is confirmed
unchanged. Because both uses read one resolver, a commit selected by
`--package-area A` always reports A or a descendant of A.

**Implementation seam (Phase 2).**

- Add a crate-private `PackageAreaIndex` (name at implementer's discretion) in
  `sniff/lib/src/filesystem/repo/ownership.rs`: built once from the
  `PackageOwnershipIndex::root()` (already canonical — no extra
  canonicalization) and `&[Package]`, mapping `normalize_path(root.join(area))`
  for each distinct non-empty area to the index of a package carrying it, with
  a lookup that walks ancestors of a given absolute directory (inclusive) and
  returns the first hit, which is the deepest. File callers pass the path's
  parent to get strict containment.
- `PackageCatalog` in `recent_commits/collect.rs` holds it next to the
  ownership index, replaces `area_packages: Vec<usize>` with the lower-cased
  selected area string, and routes `matches` and `attribute` through one
  private `resolve(path) -> (Option<usize>, Option<&str>)` that normalizes
  each path once and calls `lookup_normalized` (add a small accessor if
  needed rather than re-normalizing).
- Replace the `HashSet` fallback in `RepoInfo::package_area_for_dir_with_index`
  (`repo/types.rs`) with the same index (inclusive lookup, since callers pass
  directories) so CWD context and recent commits share one deterministic
  deepest-area rule. Its public signature is unchanged; add a unit test for a
  nested-area directory resolving to the deeper area.
- No new public API, dependency, subprocess, or filesystem access per path.

**Remaining ambiguity (non-blocking).** The specification says membership is
"by file location", but for a directory that is both a package and an area
(`darkmatter/dmls`, `tabby`) the package-first ruling keeps that package's own
files in its declared parent area. The specification's examples do not reach
this case, and the ruling matches existing package metadata. Unwired area
directories (no catalog package names them) remain unselectable, as today:
`--package-area` validates against catalog areas.

### Wave 1 Regression Blueprint

Fixtures commit in-process with `git2` and explicit signatures at
`BASE + 60 s × n`; capture each `Oid` at creation and assert full hash
sequences. No test reads this checkout's history.

**Library fixture `area_workspace()`** in `sniff/lib/tests/l1/recent_commits.rs`
(new `mod area_membership`, reusing `Fixture`). Workspace members
`worktree/lib` (`worktree`), `worktree/cli` (`worktree-cli`), `other/lib`
(`other`). Oldest to newest:

| id | subject | changed paths | area filter `worktree` |
|---|---|---|---|
| C0 | `chore: initial workspace` | manifests, all `src`, `worktree/README.md`, `worktree/docs/a.md` | match |
| C1 | `feat(worktree): lib change` | `worktree/lib/src/lib.rs` | match |
| C2 | `chore: other change` | `other/lib/src/lib.rs` | no |
| C3 | `planning(worktree): record execution plan for 2026-09-25-worktree-file` | `worktree/fixes/2026-09-25-worktree-file/{plan,spec}.md` | match (the `1634e6e` shape) |
| C4 | `planning(worktree): skill note` | `.claude/skills/worktree/SKILL.md` | no (misleading scope) |
| C5 | `docs: sibling readme` | `worktree-other/README.md` | no (sibling prefix) |
| C6 | `docs: refresh readme` | `worktree/README.md` | match, no worktree scope |
| C7 | `review: first review` | `worktree/fixes/2026-09-25-worktree-file/review-1.md` | match |
| C8 | `fix(cli): cli change` | `worktree/cli/src/main.rs` | match |
| C9 | `chore: other again` | `other/lib/src/lib.rs` | no |
| C10 | `chore: mixed` | `worktree/features/x/spec.md`, `other/lib/src/lib.rs` | match |
| C11 | `chore: move shared doc` | rename `worktree/docs/a.md` → `docs/a.md` | match via source path |
| C12 | `chore: drop spec` | delete `worktree/fixes/2026-09-25-worktree-file/spec.md` | match (deleted path) |
| C13 | `chore: newest other` | `other/lib/src/lib.rs` | no |

Expected hash sequences (newest first):

- `count(5).package_area("worktree")` → C12, C11, C10, C8, C7 (walks past C13
  and C9; count filled with matches).
- `count(50).package_area("WORKTREE")` → C12, C11, C10, C8, C7, C6, C3, C1, C0
  (exhaustion; selector case-insensitive).
- `package("worktree")` → C1, C0; C3 absent. `package("worktree-cli")` → C8, C0.
- `package_area("other")` → C13, C10, C9, C2, C0.
- `package("other").package_area("worktree")` → C10, C0 (different files
  satisfy different filters). `scope("worktree").package_area("worktree")` →
  C3, C1 (C4 excluded by area, not scope).
- Unfiltered JSON attribution: C3, C6, C7, C11, C12 →
  `packages: []`, `package_areas: ["worktree"]`; C4, C5 → `[]`/`[]`;
  C1 → `["worktree"]`/`["worktree"]`; C8 → `["worktree-cli"]`/`["worktree"]`;
  C10 → `["other"]`/`["other", "worktree"]` with both files in `files`.
  Filtered JSON for the same commits carries identical attribution and file
  lists (compare the filtered records to the unfiltered records by hash).

**Library boundary cases** (extend the existing `attribution::monorepo()`
fixture or a sibling fixture):

- Rename `monorepo_commits_carry_deepest_package_arrays_and_area_files_stay_unattributed`
  and its expectation: `area root file` (`crates/README.md`) now reports
  `packages: []`, `package_areas: ["crates"]`; `package_area("crates")` gains
  it; `package_area("crates/alpha")` does not.
- Package-and-area overlap: `crates/alpha/README.md` → `["alpha"]`/`["crates"]`,
  selected by `crates` but not `crates/alpha`.
- Nested unowned area: packages `apps/tool` (area `apps`) and `apps/web/site`
  (area `apps/web`); `apps/web/NOTES.md` → `package_areas: ["apps/web"]` only,
  selected by both `apps` and `apps/web`. Repeat collection several times in
  one test to catch hash-order dependence.
- Historical file at an area's own path: commit a root file `worktree`,
  delete it and create `worktree/lib` in the next commit; the first commit's
  `worktree` path neither matches `--package-area worktree` nor attributes
  the area.
- Empty root area: a top-level package still reports area `""`; a root
  `README.md` stays unattributed and `package_area("")`, if accepted, selects
  only top-level-package commits.
- Counter test `attribution_uses_the_structure_tier_without_inventory_language_or_doc_walks`
  keeps passing; add `FS_CANONICALIZATIONS` equality between an area-filtered
  and unfiltered collection of the same fixture to prove no per-path
  canonicalization.

**CLI fixture** in `sniff/cli/tests/l1/cli.rs`: a smaller copy of C0–C8 plus
C10 built with `git2` and explicit times (add a local explicit-time commit
helper; `test_commit_file_with_message` uses wall-clock signatures). Assert
through `run_commit_json`:

- `recent-commits 5 --package-area worktree --json` → exact hashes
  C10, C8, C7, C6, C3; stdout parses as JSON.
- `--package worktree` excludes C3; `--package worktree-cli` → C8, C0.
- Unfiltered `--json`: C3 has `packages: []`, `package_areas: ["worktree"]`;
  C10 lists both files and `["other", "worktree"]`.
- `recent-commits --package-area worktree --plain` stdout contains the C3
  heading; stderr carries no result data.

**Acceptance-criterion map.** Planning-only inclusion/package exclusion → C3
rows (lib + CLI). README/reviews/shared files → C6, C7, C10, C11, C12.
Library vs CLI package distinction → C1/C8 package filters. Sibling and skill
negatives → C4, C5, historical-file case. Attribution without invented
package and filter agreement → unfiltered-vs-filtered attribution
comparison. Identity, order, count filling → count-5 and count-50 sequences.
Docs → Phase 2 Wave 3. Cross-OS → Phase 3 Wave 4.

## Phase 2: Implement Independent Area Membership

### Wave 2: Library and CLI Regressions

Prerequisite: Wave 1 decisions and fixture expectations are recorded. These tasks can run concurrently because they own separate files; the CLI tests may remain red until the library task lands.

- [x] **Library correction**
  - Own `collect.rs`, any necessary shared resolver changes, and `sniff/lib/tests/l1/recent_commits.rs`. Add the regression first and demonstrate the old implementation's failure on shared area files.
  - Replace package-index-only area filtering with recognized directory boundaries. Resolve area membership independently in attribution and filtering through the agreed shared semantics, retaining package ownership for package names and package filters.
  - Preserve selector validation, parent-area filtering, sorted/deduplicated attribution, full surviving file lists, and source-path attribution for moves. Update behavior-linked rustdoc and inline comments, especially `PackageCatalog::attribute`.
  - Assert planning-only and README-only attribution in filtered and unfiltered JSON; library and CLI packages remain distinct. Check nested areas, component boundaries, deleted/moved shared files, and the empty root area's existing behavior.
  - Assert exact hash sequences for count 5 across intervening nonmatches and for a request larger than available matches. Assert a mixed-area survivor retains files and attribution outside the selected area. Preserve combined-filter behavior, including when different files satisfy different filter kinds.
  - Update existing tests that intentionally expected area-root files to remain unattributed; explain the contract change in their names and assertions.

- [x] **CLI regression**
  - Own only the relevant tests in `sniff/cli/tests/l1/cli.rs` and narrowly necessary fixture code. Use `SniffCliFixture` and fixture-owned Git setup, with isolated environment/configuration and the archive-safe binary lookup already provided by the harness.
  - Run the shipped CLI against the blueprint repository with `--package-area worktree`, `--package worktree`, and the CLI package selector. Assert successful exit, exact commit hashes/order, exclusions, and count filling/exhaustion.
  - Parse all stdout as JSON and assert shared-only `packages: []`, the area name, and complete files for mixed commits. Include an unfiltered JSON assertion so attribution cannot accidentally depend on the area filter being present.
  - Add a focused plain-output assertion that the same planning commit appears through the normal command route. Preserve stdout for result data and stderr for diagnostics; no terminal layout changes are required.

**Checkpoint:** Library and CLI regressions pass together, including previously existing recent-commit cases. Review the diff to ensure no ownership logic moved into CLI code and no unrelated working-tree changes were overwritten.

### Wave 3: Documentation and Integration

Prerequisite: Wave 2 behavior is stable. The following tasks can run concurrently with separate ownership of documentation and tests.

- [x] **Document boundaries**
  - Update `sniff/docs/topics/repo/recent-commits.md`, `sniff/docs/topics/repo/recent-commits-schema.md`, and `sniff/docs/cli/repo_recent-commits.md`: distinguish package directories from area directories, replace the obsolete unattributed-area rule, and show the shared-file JSON example.
  - Add concise public behavior notes to the relevant recent-commit sections of `sniff/cli/README.md` and `sniff/lib/README.md`. Explain count-of-matches and full changed-file retention without suggesting scope-based membership.
  - Update `.claude/skills/sniff/architecture.md` where its ownership description needs the independent area contract. Preserve existing edits in these files. No dependency-document changes are needed unless implementation actually changes dependencies.

- [x] **Integration safeguards**
  - Own narrowly scoped integration/counter assertions. Check the aggregate path through `RecentCommits::collect_observed` and `sniff/lib/src/filesystem/repo/aggregate_view.rs`; add a shared-file attribution assertion proving aggregate and focused collection agree.
  - Reuse existing recent-commit performance collector assertions to prove only survivors receive expensive file diffs, structure discovery remains inventory/document/enrichment-free, and the aggregate does not gain redundant manifest discovery. Read the Sniff performance reference before changing or interpreting counters.
  - Inspect category projections for attribution preservation; reuse their existing tests where sufficient. Avoid unrelated renderer changes or new visual snapshots.

**Checkpoint:** Documentation agrees with executable assertions, aggregate attribution reflects the fix, and stable work counters demonstrate no additional discovery tier or survivor enrichment.

## Phase 3: Validate and Prepare Review

### Wave 4: Platform Validation

Prerequisite: Wave 3 is integrated. Local and remote validation may run concurrently against the same frozen source tree, with separate build/output locations and no source edits during runs.

- [x] **Local verification**
  - Follow the `rust-testing` skill and run focused nextest regressions first, then `just test` and `just lint` from the Sniff area. The canonical test recipe enables `remote`; ordinary local L1 must leave CLI `test-fixtures` off.
  - Run `cargo clippy -p sniff --all-targets -- -D warnings` and `cargo clippy -p sniff-cli --all-targets -- -D warnings`, plus formatting checks scoped to changed packages. Run doctests if public examples or rustdoc changed.
  - Preserve existing terminal rendering tests; run `just test-l2` only if actual rendering changes require that evidence. This path-selection fix needs no new L2 test or CI matrix cell.

- [x] **Platform evidence**
  - Load the OS skill's build-host instructions and inspect declared `BUILD_*` hosts. Obtain runtime evidence for the affected library/CLI regressions on macOS, Linux, native Windows, and WSL2 using supported non-interactive area/package recipes; reuse qualifying evidence when available.
  - Exercise native path components on Windows and archive execution on WSL2, including deleted historical paths and sibling-prefix exclusions. A cross-compile alone does not satisfy runtime acceptance.
  - Record source revision/tree, commands, feature selections, environment, test identities, and results. If a host is unavailable, record the missing evidence and its required follow-up; do not claim all-platform completion or change CI scheduling to conceal the gap.

**Checkpoint:** Required tests and lint pass with evidence tied to the final tree. Any platform gap remains explicit and blocks claiming that platform's acceptance criterion is verified.

### Wave 5: Review Handoff

- [x] **Acceptance audit**
  - Prerequisite: Wave 4 results are collected. Map the specification's acceptance criteria to concrete test names and results; verify the planning-only regression, all negative boundaries, attribution, order, count, and full-file preservation.
  - Review implementation comments and docs for remaining old-contract claims. Verify no new dependencies, network work, or broad unrelated refactoring slipped in; inspect the final diff without altering unrelated edits.
  - Record the final implementation decisions, validation outcomes, and any outstanding evidence gaps alongside this plan. Mark completed task checkboxes only when their observable outputs exist.
  - Handoff state is **implementation complete, ready for review** when required work is satisfied. Leave `2026-09-25-recent-commits` in its current lifecycle location; only the author closes the review cycle and moves it.

### Wave 5 Acceptance Record (2026-09-26)

Evidence tree: HEAD `d6ed07b48` plus only this plan's checkbox edits
(cross-check tree `2ae698bd0`; source identical to HEAD). Each OS ran the
package's full L1 suite in CI archive mode through `just cross-check <pkg>
--os all` with no extra features or filters. macOS ran locally with `just
test` (`remote` enabled).

| OS | `sniff` | `sniff-cli` |
|---|---|---|
| macOS (local) | `just test`: 2908 passed across both packages, 32 skipped | (included) |
| Linux (build-linux) | 2023 passed, 30 skipped | 862 passed, 7 skipped |
| Windows native (build-win-native) | 2012 passed, 23 skipped | 858 passed, 7 skipped |
| WSL2 (build-win) | 2023 passed, 30 skipped | 862 passed, 7 skipped |

All 19 targeted library regressions and both targeted CLI regressions passed on every
OS; none were skipped.

| Acceptance criterion | Tests |
|---|---|
| `1634e6e`-shaped planning commit in `--package-area worktree`, not `--package worktree` | `recent_commits::area_membership::planning_only_commit_matches_its_area_but_not_its_package`; CLI `test_recent_commits_package_area_selects_shared_area_files_by_location` |
| Area README, planning, reviews, shared files match | `area_membership::count_is_filled_with_area_matches_across_nonmatching_commits` (C6, C7, C10, C11 moved, C12 deleted) |
| Library/CLI files match the area; package filters distinguish | `planning_only_commit_matches_its_area_but_not_its_package`; CLI test |
| Sibling directories and skill files excluded | `area_membership::commit_scopes_and_path_name_matches_confer_no_area_membership`; `attribution::a_file_named_like_an_area_directory_or_a_prefix_sibling_is_outside_the_area` |
| Attribution includes area without inventing package; agrees with filtering | `area_membership::shared_area_files_are_attributed_to_the_area_and_filtering_keeps_whole_records`; CLI tests including `test_repo_aggregate_attributes_shared_area_files_like_the_focused_command` |
| Identity, order, count filling | count-5 and count-50 sequences (lib and CLI) |
| Tests and docs reflect rules | Phase 2 docs; stale-wording scan clean |
| macOS, Linux, native Windows, WSL2 | table above |

Audit: no dependency, `Cargo.toml`, or CLI source change; no network work.
Rustdoc/comments on `PackageCatalog`, `PackageAreaIndex`, and
`package_area_for_dir_with_index` match the implementation. Open items for
the reviewer: `cargo fmt --check` reports drift in the five changed files
(some pre-existing, some added in Phase 2; not reformatted per the
no-formatting rule), and no `wsl2-ubuntu` receipt was published because the
tested tree carried uncommitted plan edits.

**Checkpoint:** The reviewer can trace each acceptance criterion to the implementation and its evidence without reconstructing the execution history.
