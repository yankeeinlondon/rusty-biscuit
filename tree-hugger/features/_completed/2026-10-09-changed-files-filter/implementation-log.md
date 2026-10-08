---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-09-changed-files-filter/spec.md"
plan: "tree-hugger/features/2026-10-09-changed-files-filter/plan.md"
implemented_by: "opencode/zai-coding-plan/glm-5.3"
started_phase: "1"
packages: [sniff, tree-hugger-cli]
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/error.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/discovery.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/changed_paths_tests.rs
docs_updated_during_phase_2:
  - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .opencode/skill/sniff/SKILL.md
  - .opencode/skill/os/macos.md
source_files_during_phase_3:
  - tree-hugger/cli/src/changed.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/Cargo.toml
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .opencode/skill/tree-hugger/SKILL.md
source_files_during_phase_4:
  - tree-hugger/cli/tests/changed/main.rs
  - tree-hugger/cli/tests/changed/parser.rs
  - tree-hugger/cli/tests/changed/selection.rs
  - tree-hugger/cli/tests/changed/failure.rs
  - tree-hugger/cli/tests/changed/empty.rs
  - tree-hugger/cli/tests/changed/cache.rs
  - tree-hugger/cli/tests/changed/paths.rs
docs_updated_during_phase_4:
  - tree-hugger/features/2026-10-09-changed-files-filter/plan.md
  - tree-hugger/features/2026-10-09-changed-files-filter/implementation-log.md
  - tree-hugger/features/2026-10-09-changed-files-filter/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/changed/parser.rs
docs_updated_during_phase_5:
  - docs/dependencies.md
  - sniff/docs/dependencies.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/dependencies.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-09-changed-files-filter/plan.md
  - tree-hugger/features/2026-10-09-changed-files-filter/implementation-log.md
  - tree-hugger/features/2026-10-09-changed-files-filter/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/sniff/SKILL.md
source_code:
  - sniff/lib/src/error.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/discovery.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/changed_paths_tests.rs
  - tree-hugger/cli/src/changed.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/tests/changed/main.rs
  - tree-hugger/cli/tests/changed/parser.rs
  - tree-hugger/cli/tests/changed/selection.rs
  - tree-hugger/cli/tests/changed/failure.rs
  - tree-hugger/cli/tests/changed/empty.rs
  - tree-hugger/cli/tests/changed/cache.rs
  - tree-hugger/cli/tests/changed/paths.rs
  - Cargo.lock
documentation:
  - sniff/docs/sniff-library-architecture.md
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/os/macos.md
  - .claude/skills/tree-hugger/SKILL.md
  - tree-hugger/features/2026-10-09-changed-files-filter/plan.md
  - tree-hugger/features/2026-10-09-changed-files-filter/implementation-log.md
  - tree-hugger/features/2026-10-09-changed-files-filter/spec.md
  - docs/dependencies.md
  - sniff/docs/dependencies.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/dependencies.md
  - tree-hugger/docs/symbol-reports.md
completed_phase: 5
implemented: true
---

# Implementation Log for 2026-10-09-changed-files-filter (5 phases)

## Phase 1

Non-interactive session: no author was available to answer R1–R6, so each
ruling below records the plan's recommendation as confirmed, which the plan
explicitly authorizes ("the plan proceeds on the recommendation unless told
otherwise").

### Rulings

- **R1 — `--prelude` combination: confirmed (recommended).** `--changed` and
  `--prelude` together are an argument error (exit 2), rejected by clap
  `conflicts_with` before any Git inspection, matching `--prelude`'s existing
  conflict pattern. Documented in help and `symbol-reports.md` (Phase 5).
- **R2 — empty-success exception: confirmed (recommended).** When `--changed`
  is set, `NoSourceFiles` from `collect_selected_files` and an empty
  changed-set ∩ selection intersection both short-circuit to the existing
  empty report for the active format. Any other scanner error propagates;
  commands without `--changed` are untouched.
- **R3 — validation ordering: confirmed (recommended).** clap syntax → R1
  conflict (exit 2) → Git discovery / reference resolution / conflict /
  comparison failures (exit 1) → ordinary file selection → intersection →
  report. Git work runs before any file is read or reported.
- **R4 — dependency features: confirmed (recommended).** sniff is added to
  `tree-hugger-cli` with `default-features = false`. sniff's `default = []`
  (sniff/lib/Cargo.toml), so this excludes nothing today but pins the intent:
  no `network`/`remote`/`metrics`. Verified `cargo check -p sniff
  --no-default-features` compiles clean (51.5s). No feature extraction or
  `docs/dependencies.md` note beyond recording the dependency is needed.
- **R5 — reference type: confirmed (recommended).** A small public enum
  (final shape below) makes the unborn-`HEAD` exception type-enforced:
  `Omitted` compares against nothing committed; `Reference(token)` must
  resolve locally to exactly one commit, peeling annotated tags and rejecting
  trees/blobs. An explicit `HEAD` token on an unborn repository is an error.
- **R6 — wider measurement: confirmed.** No wall-clock benchmark or timing
  matrix. Avoided work is proven by work assertions (one status observation,
  zero line-diff/network/inventory work), reusing sniff's existing counters
  and the `status.rs` walk probe.

### Baseline

Run on the macOS host with this worktree's pre-existing uncommitted
symbol-feature changes present (the plan expects them).

| Area | Gate | Result |
| --- | --- | --- |
| tree-hugger | `just test` | green — 1792 run, 1792 passed, 0 failed (45 slow), 154.8s |
| tree-hugger | `just lint` | clean (tree-hugger + tree-hugger-cli) |
| sniff | `just test --no-fail-fast` | 3127 run, 3117 passed, 10 timed out (30s cap), 32 skipped |
| sniff | `just lint` | clean |
| sniff | `cargo check -p sniff --no-default-features` | clean (R4 evidence) |

The 10 sniff timeouts are pre-existing, environment-dependent flakes:
`hardware::audio` (2), `hardware::tests` (2), `tests::test_detect_*` /
`test_os_present_by_default` / `test_skip_os_returns_none` (3), and
`sniff::l1 integration::test_detect_*` / `test_hardware_only_json_*` (3).
Each takes ~15s in isolation (4 re-run individually: all pass) and exceeds
the 30s per-test cap only under full-suite parallel load on this host. None
touch git/filesystem code; the `filesystem::git` subset is 203/203 green.
Not attributed to this feature; recorded here so later phases know the
baseline.

A plain `just test` in sniff (fail-fast) aborts after the first timeout
batch with exit 100 — same tests, same cause. Use `--no-fail-fast` or a
filterset when a full sniff suite run is needed mid-implementation.

### Skills loaded

Read per the plan's task: `tree-hugger`, `sniff` (with `architecture.md` /
`testing.md` / `performance.md` queued for Phase 2's status-walk work),
`rust-testing`, `os`, `biscuit-file`, and `docs/cicd/test-inputs.md`.
Expected consequence, as the plan states: this feature's tests build
temporary repositories through library APIs and read no repository files,
so no `[package.metadata.ci.tests] source-inputs` declaration is expected.

### API shape (contract for Phase 2 and Phase 3)

Grounded in the current seams: `GitRepo` in
`sniff/lib/src/filesystem/git/types.rs` (method home, `discover`, `is_bare`,
`repo_root`), `status.rs::get_repo_status_with_changes` (the one-walk status
classification whose `BString`-keyed staged/unstaged/untracked/conflicted
maps and `walk_probe` the path-only variant shares), and
`discovery.rs::{resolve_single_opt, get_commit_files_with_cache_fallible}`
(the `changes()` + `track_rewrites(None)` +
`for_each_to_obtain_tree_with_cache` tree-diff pattern, already available
under the pinned `gix = "=0.84.0"` features — shipped code calls
`diff_resource_cache_for_tree_diff`, so no new gix feature is expected; a
Phase 2 compile check confirms).

Public surface (final names follow sniff's style; placement: method on
`GitRepo` in `types.rs`, path-only classification in `status.rs`, tree
comparison alongside the existing diff code in `discovery.rs`, enum and
errors exported from `error.rs` / re-exported from `git/mod.rs`):

```rust
/// Which committed base a changed-path request compares the HEAD tip
/// against.
pub enum ChangedBase {
    /// No reference was supplied. The committed comparison contributes
    /// nothing (empty base), so an unborn HEAD is valid in this mode.
    Omitted,
    /// An explicit reference token. Must resolve locally, without network
    /// access, to exactly one commit (annotated tags peel); a tree or blob
    /// is rejected. `Reference("HEAD")` on an unborn repository is an
    /// error, distinguishing an explicit HEAD from [`ChangedBase::Omitted`].
    Reference(String),
}

impl GitRepo {
    /// Deduplicated, deterministically ordered (sorted by native path
    /// bytes) native repository-relative paths for the union of: base-tip
    /// vs HEAD tree differences (only when the resolved base differs from
    /// HEAD), staged, unstaged, and nonignored untracked paths. One status
    /// observation; no line stats, patches, rename similarity, history, or
    /// inventory.
    ///
    /// Rejects bare repositories explicitly. Index conflicts anywhere in
    /// the repository are an error naming the paths.
    pub fn changed_paths(&self, base: ChangedBase) -> Result<Vec<PathBuf>>;
}
```

`Result` is `sniff::Result`. New `SniffError` variants (following the enum's
documented-field style; `operation`-tagged gix failures reuse the existing
`SniffError::Git`, and repository absence reuses the existing
`SniffError::NotARepository`):

| Variant (fields) | Meaning |
| --- | --- |
| `NotARepository(PathBuf)` — existing | invocation path is inside no git repository |
| `BareRepository { path: PathBuf }` | repository has no working tree to select from |
| `UnresolvedReference { reference: String, path: PathBuf }` | token names no local object (no fetch is attempted) |
| `NonCommitReference { reference: String, kind: &'static str, path: PathBuf }` | token resolves but peels to a tree (`"tree"`) or blob (`"blob"`), not a commit |
| `MergeConflicts { path: PathBuf, paths: Vec<PathBuf> }` | unmerged index entries exist; `paths` names every conflicted repository-relative path |
| `Git { operation, source }` — existing | wrapped gix failure (status walk, rev parse, peel, tree diff, index read); the CLI layer adds repository/reference context to the message |

Path contract for Phase 2: keep paths byte-native (`gix::bstr::BString`)
through classification, diff collection, and dedup, converting to native
`PathBuf` (`OsStr::from_bytes` on Unix) only at the public boundary, so
non-UTF-8 paths survive where the platform allows and never collapse
through lossy strings (mirrors `status.rs`'s existing byte-native keying,
whose lossy conversion is public-output-only and stays that way for the
rich APIs).

Unborn/detached semantics carried by the type: `Omitted` + unborn HEAD →
empty committed comparison plus working-tree status; `Reference(_)` +
unborn HEAD → base compared against the empty current tree, unioned with
unborn working-tree status; detached HEAD is valid and uses its commit.

### Test-design mapping (Phase 1)

Phase 1 changes no behavior — rulings, baseline, skills, and the contract
above are its whole deliverable — so there are no new tests and no
requirement-to-test mapping yet. The plan places this feature's tests in
Phases 2 (sniff L1 + work assertion) and 4 (CLI), against the spec's
acceptance table; the Input Robustness Matrix is not applicable per the
plan (no parser, manifest, config loader, or deserializer is added).

### Gates

Gates were run, not skipped. After all Phase 1 edits (feature-directory
files only; no source, docs, or skill files), the scope calculator was run
from the repository root:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
# → packages: ["tree-hugger", "tree-hugger-cli", "test-toolkit", "sniff"]
# → change_class: "package"
```

The non-empty list comes from this worktree's pre-existing uncommitted
symbol-feature changes (an earlier feature's phase outputs, not Phase 1 of
this plan, which touched only `tree-hugger/features/2026-10-09-*`). Per the
rule, a non-empty `packages` means both gates run: the baseline table above
is exactly that evidence — `just test` and `just lint` in `tree-hugger`
(green) and `just test` (no-fail-fast; 10 pre-existing hardware-probe
timeouts documented above) and `just lint` (clean) in `sniff`. `test-toolkit`
is selected by the prior feature's changes, not by this phase's files, and
its gates belong to that feature's phase.

## Phase 2

Implemented the sniff half of the contract recorded under "API shape" above,
exactly as shaped: the method lives on `GitRepo` (`types.rs`), the path-only
status classification in `status.rs`, base resolution and the tip-vs-tip tree
comparison in `discovery.rs`, the enum beside `GitRepo`, and the four new
error variants in `error.rs`.

### What changed (sniff only)

- `SniffError` gains `BareRepository`, `UnresolvedReference`,
  `NonCommitReference { kind: "tree"|"blob" }`, and `MergeConflicts` (whose
  Display names every conflicted path). `NotARepository` and `Git` are reused
  as planned.
- `status.rs`: `status_platform()` is the one configured walk (probe + counter
  recorded once per observation); `classify_status_paths()` is the single
  classification point — `get_repo_status_with_changes` now observes through
  it too, so the rich report and the path-only collector can never disagree
  about what staged/unstaged/untracked/conflicted means;
  `collect_path_status()` returns the byte-native `PathStatus` sets.
  `is_repo_dirty` / `get_repo_status_counts_detailed` share `status_platform`
  only (their early-stop/counting loops are untouched). `bstr_to_path` became
  `pub(crate)` for the public boundary conversion. No behavior of the existing
  rich APIs changed: the pre-existing `filesystem::git` suites (209 tests)
  pass unmodified.
- `discovery.rs`: `resolve_commit_base()` (local-only resolution, annotated
  tags peeled via `peel_tags_to_end`, trees/blobs rejected, empty token
  mapped to `UnresolvedReference` rather than an opaque too-short-hex
  revparse failure) and `tip_diff_paths()` (direct tip-vs-tip tree diff,
  `track_rewrites(None)`, `head: None` = unborn HEAD vs empty tree, tree
  entries skipped, gitlinks never descended).
- `types.rs`: `ChangedBase { Omitted, Reference(String) }` and
  `GitRepo::changed_paths(&self, base)` — bare rejection, one status
  observation, conflicts-as-error naming sorted native paths, union +
  byte-sort + dedup, committed comparison only when the tips differ.
  Re-exported from `git/mod.rs` and `filesystem/mod.rs`.
- gix feature check: `cargo check -p sniff --features remote` compiles the
  tree diff under the pinned `=0.84.0` set — no new gix feature was added,
  confirming the Phase 1 expectation.

### Fixture lessons (recorded for future phases)

- git2/libgit2 caches one index per repository handle. A fixture that stages
  through one handle and commits through a handle opened earlier can commit a
  tree that lacks the staged path. All fixture helpers in
  `changed_paths_tests.rs` open a fresh handle per call, and a branch switch
  resets the index to HEAD's tree (`reset_index_to_head`) so one branch's
  staged files do not leak into the next branch's commit.
- macOS rejects any non-UTF-8 filename at the syscall level (`EILSEQ`), so a
  non-UTF-8 worktree file exists only on Linux. The non-UTF-8 test builds the
  path through the index (`add_frombuffer`) and proves byte-exactness through
  the committed comparison on every Unix host, plus the worktree half only
  where the filesystem accepts the name. Recorded in the `os` skill
  (`os/macos.md`).

### Requirement-to-test mapping (sniff scope; all in
`sniff/lib/src/filesystem/git/changed_paths_tests.rs`, 26 L1 unit tests)

| Behavior (spec/plan) | Test |
| --- | --- |
| staged / unstaged / both / untracked union, dedup, byte order | `staged_unstaged_both_and_untracked_paths_union_once` |
| clean repository → empty selection | `clean_repository_yields_empty_selection` |
| ignored untracked excluded | `ignored_untracked_files_are_excluded` |
| staged edit undone on disk still selects | `staged_edit_undone_on_disk_still_selects_the_path` |
| committed change undone locally still selects | `committed_change_restored_to_base_content_still_selects_with_base` |
| rename = delete+add | `staged_rename_surfaces_as_delete_and_add` |
| deletion selects path (absence is caller's concern) | `staged_deletion_selects_the_deleted_path` |
| staged deletion + recreate selects | `staged_deletion_then_recreate_selects_the_current_file` |
| copy, no similarity pairing | `copy_keeps_both_paths` |
| tip-vs-tip, not merge-base (divergent branches) | `divergent_base_compares_tips_not_merge_base` |
| base == HEAD contributes nothing (HEAD/main/branch forms) | `base_equal_to_head_contributes_no_committed_paths` |
| commit id, abbreviated id, annotated tag bases | `commit_sha_abbreviated_sha_and_annotated_tag_bases_resolve` |
| empty / unknown / absent-sha refs | `empty_and_unknown_reference_tokens_are_unresolved` |
| tree / blob refs rejected with kind | `tree_and_blob_references_are_non_commit` |
| omitted base + unborn HEAD = status only | `omitted_base_on_unborn_head_uses_working_tree_status` |
| explicit HEAD on unborn fails | `explicit_head_on_unborn_head_is_unresolved` |
| explicit base + unborn HEAD vs empty tree (isolated from status) | `explicit_base_with_unborn_head_compares_against_the_empty_tree` |
| detached HEAD uses its commit | `detached_head_uses_its_commit` |
| conflicts anywhere fail naming every path | `index_conflicts_anywhere_fail_naming_every_path` |
| bare rejected explicitly | `bare_repository_is_rejected` |
| no repo → discovery `None` | `discovery_outside_any_repository_returns_none` |
| nested repo not descended | `nested_repository_contents_are_not_selected` |
| non-ASCII + spaces | `non_ascii_and_spaced_paths_survive` |
| non-UTF-8 byte-exact (unix; worktree half Linux-only) | `non_utf8_paths_survive_byte_exact` |
| mode-only committed change (unix) | `executable_mode_change_selects_the_path` |
| one status walk; zero blob loads / file diffs / ref walks / commit visits / worktree opens | `changed_paths_performs_one_status_walk_and_no_diff_work` |

Test-tier notes: all 26 are in-module L1 unit tests (compiled by the lib test
target, selected by `just test`); no level2+/real/browser markers, no repo
files read (temp fixtures only), so no `source-inputs` declaration is needed.
`#[cfg(unix)]` gates the two platform-bound tests; Windows CI compiles and
selects the remaining 24. Network absence is by construction (no remote code
path is reachable from `changed_paths`), backed by the ref-walk/commit-visit/
worktree-open counters at zero in the work-assertion test.

### Spec departures

None. One deliberate narrow addition beyond the letter of the plan: an empty
reference token maps to `UnresolvedReference` in `resolve_commit_base`
(generic rev parsing would report it as a too-short hex prefix, i.e. an
opaque `Git` failure). The CLI layer rejects empty values as argument errors
anyway (Phase 3); this keeps the sniff-level error honest if ever reached.

### Gates

Scope calculator (run from the repository root after all Phase 2 edits):

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
# → packages: ["sniff","tree-hugger","tree-hugger-cli","test-toolkit"]
# → change_class: "package"
```

Non-empty, so both gates ran. tree-hugger/tree-hugger-cli/test-toolkit are
selected by this worktree's pre-existing symbol-feature changes (Phase 2
itself touched no tree-hugger or test-toolkit file); their gates were run
anyway per the rule.

| Area | Gate | Result |
| --- | --- | --- |
| sniff | `just test --no-fail-fast` | 3153 run, 3143 passed, 10 timed out (30s cap) — the exact 10 pre-existing hardware/audio/OS-probe timeouts from the Phase 1 baseline; 5 spot-checked individually: all pass in ~14s under no load |
| sniff | `just lint` | clean |
| sniff | `cargo clippy -p sniff --features remote --all-targets -- -D warnings` (handoff standard) | clean |
| sniff | `cargo clippy -p sniff-cli --all-targets -- -D warnings` | clean |
| tree-hugger | `just test` | 1566/1569 partition passed; 3 pre-existing symbol-cache timeouts under parallel load pass in ~7s in isolation |
| tree-hugger | `just lint` | clean |
| sniff on Linux (build-linux, `just cross-check sniff --os linux`) | `filesystem::git` L1 | 234/234 passed, including all 26 `changed_paths_tests` (full non-UTF-8 worktree path included) |

Windows/WSL2 evidence follows the existing CI schedule per the plan
(implementation contains no OS-conditional code; only the two tests are
platform-gated). macOS (local) and Linux (build-linux) are green as the
plan's success criteria require.

## Phase 3

Implemented the tree-hugger-cli half: the clap surface (Wave 1) and the run
path (Wave 2). Three files changed — `tree-hugger/cli/Cargo.toml` (sniff
dependency), `tree-hugger/cli/src/main.rs` (flag + wiring), and the new
`tree-hugger/cli/src/changed.rs`.

### What changed (tree-hugger-cli only)

- **Dependency (R4):** `sniff = { path = "../../sniff/lib",
  default-features = false }` — sniff's `default = []`, so no
  `network`/`remote`/`metrics` surface is pulled in. `cargo check
  -p tree-hugger-cli` compiles clean.
- **Flag:** a `ChangedArgs` clap struct flattened into `DeclarationArgs`
  (`functions`/`types`/`symbols`) and `ClassArgs` (`classes`) only — not
  `ScanOptions`, so `imports`, `lint`, `god-files` and `completions` reject
  it (verified: exit 2, "unexpected argument"). The field is
  `Option<Option<String>>` with `num_args = 0..=1`, `require_equals =
  true` and a `value_parser` that rejects an empty value:
  - absent → not a changed run; `--changed` → `Some(None)` →
    `ChangedRequest::Omitted`; `--changed=main` → `Some(Some("main"))` →
    `ChangedRequest::Reference("main")`.
  - **The flag alone is never materialized as an explicit `HEAD`.** The plan
    phrase "default_missing_value semantic of omitted → `HEAD`" names the
    *semantic* default (nothing committed is compared), but the spec's
    unborn-`HEAD` row requires the Omitted/`Reference("HEAD")` split to stay
    observable, so `Option<Option<String>>` is used instead of clap's
    `default_missing_value = "HEAD"` (which would collapse the two).
    Verified on an unborn repo: bare `--changed` reports the untracked
    file; `--changed=HEAD` exits 1 (unresolved reference).
  - `--changed main` leaves `main` positional (verified: it behaves as a
    smart-case name filter), repeated `--changed --changed` is exit 2
    ("cannot be used multiple times"), `--changed=` is exit 2 with the
    parser's message, `--changed --prelude` is exit 2, and
    `--json-full symbols --changed` is allowed (1 index, `errors: []`).
  - `conflicts_with = "prelude"` (R1) on both surfaces, `display_order =
    29` between `--file`/`--name`/`--exact` and `--exported`.
  - **`given_flags` interpretation:** `SelectionArgs::given_flags` exists
    only to feed `--json-full`'s record-selector rejection, and the spec
    requires `--changed` to stay compatible with `--json-full` — so the flag
    is deliberately NOT added there. The "consistent with neighbors" part
    is honored through the `Command::changed()` accessor (beside
    `exported()`/`prelude()`) and the help text.
- **`cli/src/changed.rs`:** `ChangedRequest { Omitted, Reference(String) }`
  (mapping 1:1 onto sniff's `ChangedBase`, pinned by a unit test so the
  unborn exception cannot silently collapse), `ChangedFiles::contains`
  (`file_identity` keyed), and `resolve(request, cwd)`:
  `GitRepo::discover` from the invocation's cwd (active checkout; linked
  worktree uses its own root and index per sniff's `repo_root`), one
  `changed_paths` call, native relative paths joined onto the checkout
  root, absent paths dropped *before* canonicalization, survivors keyed by
  `file_identity`. Failures become one-line exit-1 stderr messages: the
  typed sniff variants already name the repository (and every conflicted
  path), so they keep the request's spelling (`--changed=main failed:
  …`); a raw `SniffError::Git` gets repository + `--changed=…` context
  added; `Ok(None)` discovery becomes "`--changed` needs a git
  repository; <cwd> is not inside one".
- **Run path (R3 ordering):** in `main()`, after the `--json-full`
  argument-error rejection and before `resolve_file_selectors`, the changed
  set resolves; any failure prints `error: <message>` and exits 1 with no
  report. Git work therefore precedes every file-selection decision.
- **Intersection:** after `collect_selected_files`, the file list (already
  `file_identity`-keyed, sorted and deduped by the scanner) is filtered by
  `changed.contains`, preserving scanner order. Git paths are only ever
  tested against the ordinary selection — never fed into `explicit_files`
  — so fixtures, ignored files and `.gitignore`d paths are not
  re-included. Files outside the discovered repository (or deleted) drop
  out without a second discovery; an explicit file outside the package
  scan root but inside the repo stays eligible.
- **Empty success (R2):** under `--changed`, `NoSourceFiles` from
  `collect_selected_files` and an empty intersection both yield an empty
  file list; every other scanner error propagates unchanged. An empty list
  flows through the existing writers, verified in every mode on a
  deleted-only change: terminal/plain `(no symbols)`, table/tree
  `(no symbols)`, `--jsonl` empty, `--count` `0`, `--json`
  `"files": [], "errors": []`, `--stats` zero tables with
  `Doc coverage: 0 of 0 documented (n/a)`. Zero files means zero per-file
  cache lookups (only the cache handle is constructed, as for any run).

- **Skill:** the tree-hugger skill's CLI quick-reference table
  (`.opencode/skill/tree-hugger/SKILL.md`, hardlinked to
  `.claude/skills/tree-hugger/SKILL.md`) gains one `--changed[=<ref>]` row
  so the working reference is not stale after this phase; Phase 5's skills
  task still owns the fuller guidance and examples.

### Cache key audit

`--changed` and its reference do not enter the persistent symbol-cache key,
and no code change was needed. `SymbolCache::key` →
`entry_key(source, records_fingerprint)` (tree-hugger/lib/src/cache/
symbol_cache.rs) digests the file's identity path, language, content hash
and the *shipped* fingerprints (`FingerprintInputs::shipped(language,
path)`: grammar and query text, config). The flag lives entirely in the
CLI's file-selection layer, before `Analyzer`/`SymbolQuery` are even
built, so a file reached through a changed-file scan and an ordinary scan
shares one entry. (The warm-cache hit proof is Phase 4's cache test.)

### Checkpoint evidence (scratch repos, library-free CLI runs)

| Scenario | Result |
| --- | --- |
| staged + unstaged + untracked, deleted path | 5 records from a.rs/b.rs/d.rs; c.rs (deleted) dropped |
| `--changed=main` from a diverged `feature` branch | tip-vs-tip: only the tip-difference file selected (merge-base would have added another) |
| clean tree, bare `--changed` | `0`, exit 0 |
| unborn HEAD: bare flag vs `--changed=HEAD` | untracked file reported vs exit-1 unresolved reference |
| `--file` selector excluding every changed file | `0`, exit 0 (empty intersection) |
| every report mode on a deleted-only change | existing empty representation, exit 0 |
| no repository | exit 1, message names the searched directory |
| non-`--changed` runs | golden fixtures untouched — full suite green |

### Requirement-to-test mapping

Phase 3 owns the implementation; the plan places this feature's CLI test
suite in Phase 4, and every Phase 3 behavior maps onto a named Phase 4
task: parser surface → *Parser tests*; the acceptance-table rows
(staged/unstaged/untracked, restored-on-disk, ignored, rename/delete/
recreate, branch tip, package invocation, linked worktree,
outside-package explicit file, submodule/nested) → *Selection tests*;
exit-1 failures and stderr text → *Failure tests*; empty representations
→ *Empty-selection tests*; the cache-key audit → *Cache test*; path
spellings → *Path tests*. What Phase 3 adds now, in
`changed.rs`'s unit tests (L1, compiled by the `hug` bin test target,
selected by `just test`; no repo files read):

| Behavior | Test |
| --- | --- |
| Omitted maps to `ChangedBase::Omitted`, never `Reference("HEAD")` | `an_omitted_request_stays_omitted_down_to_sniff` |
| requests display the form the user wrote (error-message prefix) | `a_request_displays_the_form_the_user_wrote` |
| an empty set contains nothing | `an_empty_changed_set_contains_nothing` |
| failures name the request and (for raw Git errors) the repository | `failure_messages_name_the_request_and_the_repository` |
| discovery outside a repository names the directory | `resolving_outside_a_repository_names_the_directory` |

Test-tier notes: the five unit tests are in-module L1 (bin target; no
level2+/real/browser markers); the CLI behaviors above were verified
against the built binary on scratch repos this phase and become pinned
assert_cmd tests in Phase 4, which will need a way to build fixture
repositories through library APIs (git2 dev-dep, mirroring
sniff's `changed_paths_tests.rs` fixture helpers).

### OS considerations

No `#[cfg]` code was added: `changed.rs` uses `Path::join` + `exists()` +
`file_identity` on both sides of the comparison, and `file_identity`
(`canonicalize_simplified`) normalizes Windows long-name and separator
spellings for analyzed files already — the changed set goes through the
same function, so the identity comparison is the `os` skill's sanctioned
pattern. Git's forward-slash relative paths survive `join` and wash out in
canonicalization. Windows/WSL2 evidence follows the existing CI schedule
(Phase 4's *OS review* re-checks `#[cfg]` and path-comparison code).

### Spec departures

None against the spec. One plan-phrase interpretation recorded above:
`--changed` is not added to `SelectionArgs::given_flags` because that list
exists to reject `--json-full` combinations, which the spec explicitly
allows for `--changed`; the flag surfaces through `Command::changed()` and
help instead.

### Gates

Scope calculator (run from the repository root after all Phase 3 edits):

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
# → packages: ["sniff","tree-hugger","tree-hugger-cli","test-toolkit"]
# → change_class: "package"
```

Non-empty, so both gates ran (sniff and test-toolkit are still selected by
this worktree's pre-existing symbol-feature changes; sniff's own sources
are untouched this phase — its Phase 2 gates stand).

| Area | Gate | Result |
| --- | --- | --- |
| tree-hugger | `just test --no-fail-fast` | final run (after the last comment-only edits): 1797 run, 1797 passed, 0 failed, 0 timed out; earlier same-phase runs showed 2–4 timeouts of the pre-existing `cache_robustness` symbol-cache flakes documented in the Phase 1/2 baselines (they pass in ~25s in isolation and are load-dependent, not related to this change) |
| tree-hugger | `just lint` | clean (tree-hugger + tree-hugger-cli) |
| tree-hugger-cli | `cargo check --features terminal-tests --tests` | clean (L2 targets still compile) |

## Phase 4

### Initial audit and test design

The session started with six Phase 4 checklist items checked and an untracked
`cli/tests/changed/` suite (40 tests). Its targeted baseline passes 40/40.
Earlier phases and unrelated symbol work are already staged; this phase
preserves the existing index and changes. No implementation code is changed
before the following public-result mapping is recorded.

| Requirement | Concrete CLI test (module under `cli/tests/changed/`) |
| --- | --- |
| Bare/equal flags, positional token, empty/repeated flags, prelude, command availability, full JSON | `parser`'s eight existing tests |
| Staged/unstaged/both/untracked, restored contents, ignore/fixture rules, rename/delete/copy/recreate | `selection`'s existing status/eligibility tests |
| Tip comparison, selectors, package root, linked worktree, nested repository, unborn/detached HEAD | `selection`'s existing scope/reference tests |
| Commit ID, lightweight/annotated tag bases | add `selection::commit_ids_and_tags_select_the_same_current_records` |
| Explicit resolvable base on unborn HEAD | add `selection::an_explicit_base_on_unborn_head_compares_with_the_empty_tree` |
| Submodule boundary and foreign explicit file | add `selection::submodule_contents_are_never_selected` and `selection::an_explicit_file_in_another_repository_is_not_selected` |
| Tracked ignored explicit override, alternative file selectors, name filters | add `selection::tracked_ignored_files_and_combined_selectors_keep_existing_precedence` |
| Missing/bare repo, invalid/noncommit ref, outside-package conflict, corrupt Git | `failure`'s six existing tests; strengthen JSON invocation and blob variant |
| Empty reports, zero analysis/cache, ordinary NoSourceFiles unchanged | `empty`'s five existing tests; strengthen counters/totals and add ordinary control |
| Actual shared warm entry in both directions | `cache`'s two existing tests; compare whole records and repeat reads |
| Spaces/non-ASCII, native non-UTF-8, case spelling | `paths`' three existing tests; distinguish supported host skip from unexpected I/O failure |

All fixtures use library-created Git repositories and the normal `hug`
subprocess path. These are hermetic L1 filesystem/CLI tests under the
rust-testing decision tree; no terminal/browser resource is required.
Cargo auto-discovers `tests/changed/main.rs`, which declares every module;
no feature gate or higher-tier name marker applies. No file-format reader
changes, shipped configuration artifacts, or persisted-value format changes
are made, so the input matrix and parser corpus requirements do not apply.

### Targeted verification and OS review

`just test --test changed` passes **47/47** in the package area. The seven
added tests are exactly the five named `selection` tests in the mapping,
`failure::a_blob_reference_is_not_a_commit`, and
`empty::no_source_files_remains_an_error_without_changed`. The existing
40 tests were retained and strengthened. No production behavior changed.
Two intermediate control-test runs failed because the new assertion assumed
stdout/display formatting; the existing public CLI writes the Rust error
name `NoSourceFiles` on stderr. Corrected the test, without changing code.
An accidental root `just test --test changed` invocation failed argument
parsing; all reported test evidence comes from the absolute package-area cwd.

The helper now uses `biscuit_test_harness::bin_exe!("hug")` so archive consumers
resolve the relocated executable at runtime. `just check-tier-coverage
tree-hugger` passes with **0 stranded tests**; nextest lists the `changed`
binary and runs every module as L1. No L2 files changed, so `just test-l2`
is unnecessary and no terminal or browser windows were opened.

Loaded the `os` skill and its macOS, build-host, and Windows-path guidance.
Reviewed `changed.rs` and `file_identity`: native `PathBuf` roots and joins
are preserved until both candidate/scanner paths use the same simplified
canonical identity; no raw slash, case-folding, or display-string identity
comparison exists. The new tests have only `#[cfg(unix)]` for byte names
and a narrowly scoped macOS EILSEQ (92) fixture exclusion. Unexpected I/O
errors now fail rather than silently passing. macOS APFS cannot create
non-UTF-8 worktree names; that case reports its limitation and Linux/WSL
execute it. Case-only spelling ran on this macOS filesystem; case-sensitive
hosts explicitly report that alias scenario as inapplicable.

Local evidence this phase is **macOS**. `BUILD_LINUX=build-linux`,
`BUILD_WIN=build-win-native`, and `BUILD_WSL=build-win` are declared, but
cross-check was not run: its current implementation stages a temporary Git
index and creates an unsigned snapshot commit, contrary to this session's
explicit no-staging/no-commit instruction. No new OS-specific production
behavior was introduced. Linux, native Windows, and WSL2 use the existing
CI schedule; no cells were added. Prior Phase 2 Linux sniff path evidence
remains recorded above. The Phase 4 plan's sniff gates are not repeated,
following the user's narrower instruction to run only tree-hugger gates.
Existing golden fixtures are untouched by this phase (they already carried
staged changes on arrival). No skill architecture/workflow update is needed;
Phase 5 retains ownership of the documentation and fuller skill examples.

Final test audit removed false-positive paths from the divergent-tip and
detached-HEAD fixtures: both now prove a clean default changed selection
before asserting committed comparison results. The explicit-base/unborn
fixture makes the old base path ignored-untracked, proves omitted selection
is empty, and proves only the explicit base plus ordinary file override
selects it. This isolates the promised comparison from working-tree status.

The commit-ID/tag fixture also proves a clean default scan before checking
each committed-base result, then separately checks current edited contents.
Final targeted run after all assertion changes: **47/47 passed**.
Final `just lint`: **passed** for tree-hugger and tree-hugger-cli.
`git diff --check`: **passed**. No `cargo fmt` invocation occurred.

### Scope and final gates

Required scope command from the repository root:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

The initial post-edit result is `packages: [sniff, tree-hugger,
tree-hugger-cli, test-toolkit]`, `change_class: package`. The working tree
contains earlier staged phases and unrelated symbol changes, contrary to
the prompt's assumption of an isolated phase tree; those files account for
the broader scope. Both required tree-hugger gates ran; neither was skipped.
Only the seven CLI test source files and the plan/log/spec documents are
edited in this session. No dependencies or production files changed here.

First full `just test` run: 1627/1844 executed, 1624 passed, three timed out
at the existing 30-second cap; fail-fast left 217 unexecuted. Timeouts:
`cache_robustness::deleted_written_paths_are_recomputed_in_every_language`,
`cache_robustness::every_kind_evidence_identities_name_retained_declarations_in_every_language`,
`cache_robustness::retained_geometry_written_paths_and_candidate_order_are_checked_in_every_language`.
These are the pre-existing parallel-load cache-robustness failures documented
in earlier phases; none exercises changed-file selection. Rerunning the full
area gate as `just test --test-threads 4`, preserving timeouts and retry policy.

### Acceptance rows: exact test names

Names below are in the `tree-hugger-cli::changed` binary unless qualified
as sniff. Parser and empty-format matrices additionally cover each argument
and representation listed in the initial mapping.

| Public requirement | Named test |
| --- | --- |
| Staged, unstaged, both, untracked; current declarations once | `selection::staged_unstaged_both_and_untracked_report_current_declarations_once` |
| Staged and committed edits restored on disk | `selection::a_staged_edit_undone_on_disk_selects_the_restored_content`; `selection::a_committed_change_restored_on_disk_is_selected_with_a_base` |
| Ignored untracked, including explicit files | `selection::ignored_untracked_files_are_excluded_even_when_explicitly_named` |
| Default fixture exclusion and explicit override | `selection::fixture_directories_stay_excluded_and_a_targeted_override_wins` |
| Rename, deleted paths, recreation, copies | `selection::renames_and_deletions_keep_present_destinations_and_drop_absent_paths`; `selection::a_staged_deletion_recreated_on_disk_reports_the_current_content`; `selection::a_copied_file_selects_the_copy_and_an_edited_original_selects_both` |
| Divergent local tip comparison | `selection::a_supplied_branch_tip_compares_tips_not_the_merge_base` |
| Omitted/explicit unborn HEAD, resolvable unborn base, detached HEAD | `selection::on_an_unborn_head_only_the_omitted_request_succeeds`; `selection::an_explicit_base_on_unborn_head_compares_with_the_empty_tree`; `selection::a_detached_head_uses_its_own_commit` |
| Commit IDs and both tag forms | `selection::commit_ids_and_tags_select_the_same_current_records` |
| Invalid/tree/blob reference failures | `failure::an_unresolved_reference_fails_naming_the_reference`; `failure::a_non_commit_reference_fails_naming_the_kind`; `failure::a_blob_reference_is_not_a_commit` |
| Missing/bare repository, conflicts outside package, corrupt Git | `failure::outside_any_repository_the_run_fails_naming_the_directory`; `failure::a_bare_repository_is_rejected`; `failure::conflicts_outside_the_package_still_fail`; `failure::corrupt_git_data_fails_without_a_report` |
| Package scope, linked checkout, outside-package explicit file | `selection::a_package_invocation_keeps_the_scan_scope`; `selection::a_linked_worktree_selects_its_own_checkout`; `selection::an_explicit_file_outside_the_package_root_stays_eligible` |
| Submodule/nested/foreign repository boundary | `selection::submodule_contents_are_never_selected`; `selection::a_nested_repository_is_not_descended_into`; `selection::an_explicit_file_in_another_repository_is_not_selected` |
| File/name/exclusion/language intersection and extensionless language | `selection::selectors_intersect_the_changed_set`; `selection::tracked_ignored_files_and_combined_selectors_keep_existing_precedence` |
| Every empty mode for deletion/exclusion/unsupported/out-of-selection | `empty::deleted_only_changes_render_the_empty_report_in_every_mode`; `empty::excluded_changes_render_the_empty_report_in_every_mode`; `empty::unsupported_changes_render_the_empty_report_in_every_mode`; `empty::out_of_selection_changes_render_the_empty_report_in_every_mode`; `empty::classes_renders_the_empty_selection_in_its_outline` |
| Ordinary empty scan still fails | `empty::no_source_files_remains_an_error_without_changed` |
| Shared warm cache in either direction and repeated reads | `cache::an_ordinary_scan_then_a_changed_scan_hits_the_same_entry`; `cache::a_changed_scan_then_an_ordinary_scan_hits_the_same_entry` |
| Native paths, ordering, spaces, Unicode, byte names, case aliases | `paths::spaced_and_non_ascii_paths_are_reported_once`; `paths::non_utf8_paths_are_selected_where_the_filesystem_allows_them`; `paths::case_insensitive_spelling_differences_resolve_to_one_identity` |
| Syntax and command surface | `parser::the_bare_flag_and_the_equals_form_both_run`; `parser::a_token_after_the_bare_flag_stays_a_positional_filter`; `parser::an_empty_value_is_an_argument_error`; `parser::a_repeated_flag_is_an_argument_error`; `parser::the_flag_conflicts_with_prelude`; `parser::only_the_declaration_commands_accept_the_flag`; `parser::all_four_declaration_commands_accept_the_flag`; `parser::json_full_combines_with_the_flag` |
| One focused status observation, no optional rich work | `sniff::filesystem::git::changed_paths_tests::changed_paths_performs_one_status_walk_and_no_diff_work` (Phase 2 evidence; unchanged, not rerun this phase) |

### Completion record

| Verification | Final result |
| --- | --- |
| `just test --test changed` (latest source) | 47/47 passed |
| `just test --test-threads 4` (full tree-hugger area, latest source) | **1844/1844 passed**, 0 failed, 0 timed out, 0 nextest skips; 426.192 seconds |
| `just lint` (latest source) | Passed, both tree-hugger and tree-hugger-cli |
| `just check-tier-coverage tree-hugger` | Passed, 0 stranded tests |
| `git diff --check` | Passed |

The three initial timeout cases passed in the full rerun at 22.752 seconds,
25.210 seconds, and 10.708 seconds respectively. No remaining pre-existing
failure blocks this phase. The non-UTF-8 worktree scenario remains
inapplicable on macOS, as explicitly documented above; nextest counts that
conditional return as passed rather than skipped. No L2, sniff-area, or
cross-OS gates were run in this phase, for the reasons recorded above.

All eight Phase 4 checklist tasks are complete. Phase 5 is intentionally
left open. Implementation complete for Phase 4, ready for review; no human
ruling is needed before Phase 5. No formatting, staging, or commits were
performed by this session.

Final scope calculator after source, checklist, and frontmatter edits: {"packages": [], "change_class": "documentation"}; both required area gates ran.

## Phase 5

### Scope and requirement-to-test mapping

Phase 5 closes documentation and help wording; selection, parsing, cache,
and persistence behavior remain as implemented in Phases 2–4. Before any
implementation edit, help wording is mapped to the new L1 CLI test
`parser::declaration_help_explains_changed_selection`: normal
`hug <command> --help` and `-h` invocations for all four declaration commands
assert equals-form syntax, current on-disk reporting, direct tip comparison,
and the prelude conflict in long help, plus availability in short help.
The existing `changed` target declares this file via `mod parser`; no
tier marker or additional feature is required.

Documentation examples map to existing public CLI tests: bare/equals forms
and prelude conflict (`parser`), divergent tips and selector intersection
(`selection`), empty representations and zero work (`empty`), warm-cache
sharing (`cache`), and Git failures without success output (`failure`).
No file-format/configuration reader, parser artifact, or persistence behavior
changes: robustness matrices, passive parser corpus tests, and new round
trips are not applicable. Existing cache tests retain repeated reads.

The initial working tree has three staged lifecycle documents from earlier
work; their contents and index entries are preserved. No formatting, staging,
committing, or lifecycle directory move is authorized. The explicit user
scope limits Phase 5 gates to tree-hugger, overriding the plan's older
request to rerun sniff gates. An initial command used the area directory
with root-relative edit paths and failed before any writes; its filtered
run selected zero tests and is not verification evidence.

### Documentation, skills, and drift review

Updated CLI README and Symbol Reports with examples for bare HEAD, local
`=main` tip comparison, file intersection, full JSON and empty count. The
topic page explains all selection/error rules for a new reader and includes
a Mermaid union → intersection → analysis/report flow. No topic page links
to or names the feature snapshot. Dependency notes at root and in both areas
record the production sniff edge with optional features disabled and the
dev-only git2/libgit2-sys fixture constraints. The sniff Git API topic page
already documents the full method contract from Phase 2 and was verified
against its current implementation; no rewrite was needed.

Expanded tree-hugger skill guidance for invocation, selection order, native
identities, errors, empty success, cache invariants and test routing. Expanded
sniff skill guidance for deleted/recreated candidates and native matching.
Both catalogs (.claude and .opencode) share content. Fixed a drifted sniff
skill link: its old relative path did not reach the Git API topic page.

Reviewed ChangedArgs docs, request conversion and changed-set resolution
comments, main's intersection/empty path, sniff ChangedBase/changed_paths
rustdoc and the dependency comments. The implemented contracts match; the
public help omitted the existing prelude conflict. Added the test first:
`just test declaration_help_explains_changed_selection` ran one test and
failed specifically on the missing conflict sentence. Added that sentence
to clap's doc-derived long help without changing flag semantics. No behavior
departures from the spec; the only verification scope departure is honoring
the user's tree-hugger-only gates. Existing OS evidence remains as recorded
in Phase 4; Phase 5 adds only help text and platform-neutral CLI assertions.

The empty-output drift check confirmed text modes show `(no symbols)`
(there are zero declaration records, not zero output bytes), while JSON
Lines emits no bytes. README/topic/skill wording now states the observable
marker; the existing empty-mode tests pin this behavior. Code remains
authoritative and no renderer change was made.

`just test --test changed`: **48/48 passed**, including the formerly failing
help regression; zero skipped tests in that invocation. The non-UTF-8 path
test's macOS conditional return is still the Phase 4 platform limitation,
not new path evidence. `just lint`: **passed** for tree-hugger and
tree-hugger-cli, including all targets with warnings denied.
`just check-tier-coverage tree-hugger`: **passed**, zero stranded tests.
Full area L1 is running via `just test -j 4`, preserving the proven Phase 4
concurrency without changing any timeout or retry policy.

Scope command after documentation and lifecycle metadata edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Returned `packages: [tree-hugger-cli, test-toolkit]`,
`source_packages: [tree-hugger-cli]`, `change_class: package`. The toolkit
cell reads shipped skills as suite inputs; no toolkit code was edited.
No gate skip is claimed; required tree-hugger gates ran. Other package
gates are excluded by the user's explicit tree-hugger-only scope.

Final aggregate ledgers union all per-phase source and documentation lists,
normalizing prior .opencode skill aliases to tracked .claude paths. The
root Cargo.lock dependency update is included: it was committed with the
CLI feature but omitted from earlier per-phase frontmatter. Independent
reference-report changes sharing that earlier commit are outside this
feature ledger. Every aggregate entry exists in the working tree.

### Final verification and completion

| Requirement | Concrete public-result test | Phase 5 result |
| --- | --- | --- |
| Long help explains equals syntax, current source, direct tips and prelude conflict; short help advertises the flag on all four commands | `changed::parser::declaration_help_explains_changed_selection` (new) | Failed before the help edit; passed after, and in full L1 |
| Default HEAD and local reference examples | `parser::the_bare_flag_and_the_equals_form_both_run`, `selection::a_supplied_branch_tip_compares_tips_not_the_merge_base` | Passed in targeted and full L1 |
| File intersection and selection precedence | `selection::selectors_intersect_the_changed_set`, `selection::tracked_ignored_files_and_combined_selectors_keep_existing_precedence` | Passed in targeted and full L1 |
| Empty success and zero analysis/cache work | `empty::deleted_only_changes_render_the_empty_report_in_every_mode` and excluded/unsupported/out-of-selection variants | Passed in targeted and full L1 |
| Prelude and malformed argument errors; Git failures produce no report | `parser::the_flag_conflicts_with_prelude`, empty/repeated parser tests, all `failure` tests | Passed in targeted and full L1 |
| Selection-independent cache and repeated reads | `cache::an_ordinary_scan_then_a_changed_scan_hits_the_same_entry`, `cache::a_changed_scan_then_an_ordinary_scan_hits_the_same_entry` | Passed in targeted and full L1 |

- `just test --test changed`: **48 passed, 0 skipped**.
- `just test -j 4` in tree-hugger: **1845 passed, 0 skipped**, 355.727s;
  all area L1 library/CLI tests, including the new help regression, ran.
- `just lint` in tree-hugger: **passed**, both packages and all targets,
  warnings denied. The shared lint recipe does not run cargo fmt.
- `just check-tier-coverage tree-hugger`: **passed**, zero stranded tests.
- `git diff --check`: **passed**; no formatter was run.

No unresolved or pre-existing failure affected these final gates. The
initial zero-test invocation was a working-directory mistake and is not
counted; the deliberate one-test red run established the help regression.
No L2 files, styled rendering, path comparison or OS-specific implementation
changed, so L2 and cross-OS gates were not rerun. The Unix non-UTF-8 fixture
cannot be created on macOS and retains its existing conditional return;
Linux/WSL path evidence follows the previously recorded runs and CI schedule.
Sniff and test-toolkit gates were not rerun under the user's explicit area
scope; sniff's unchanged API retains its earlier evidence. No CI cells were
added, and no test timeout or retry policy was changed.

All Phase 5 tasks are checked. Plan/log record Phase 5 file lists, aggregate
source/documentation ledgers, `completed_phase: 5` and `implemented: true`.
Spec records `implemented: true`, `implemented_by: codex/gpt-6.1-sol`,
`human_review: false` and a review handoff. No human ruling is needed.
Implementation complete, ready for review. The feature stays in place; no
formatting, staging or commits were performed.

Final scope calculator after all edits and completion flags: `{"packages": ["tree-hugger-cli", "test-toolkit"], "source_packages": ["tree-hugger-cli"], "change_class": "package"}`. Both required area gates passed; none were skipped.
