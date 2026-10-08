---
kind: plan
feature: changed-files-filter
spec: spec.md
total_phases: 5
created: 2026-10-05
phase: 5
agent: claude/sonnet
yolo: true
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

# Plan: `--changed` File Selection for Symbol Reports

## Summary and definition of success

### The work

`hug functions|types|symbols|classes --changed[=<ref>]` narrows the ordinary
file selection to files Git reports as changed, then reports each file's
current on-disk declarations. Two packages change:

- **sniff** gains one focused, fallible, path-only API on `GitRepo`. It
  returns the deduplicated native repository-relative paths for the union of
  (1) base-tip vs `HEAD` tree diff, (2) staged, (3) unstaged, (4) nonignored
  untracked. It rejects bare repositories, reports index conflicts and
  comparison failures explicitly, and distinguishes an omitted reference from
  an explicit `HEAD`. It computes no line stats, patches, rename similarity,
  history, or inventory, and observes status once.
- **tree-hugger-cli** gains the flag on declaration arguments only, the
  validation and ordering rules around it, the intersection with the scanner's
  file list by `file_identity`, and the empty-selection success path.

Existing seams (verified in the tree):

| Concern | Where |
| --- | --- |
| Declaration args and `--prelude` | `tree-hugger/cli/src/main.rs` (`conflicts_with = "prelude"` pattern) |
| File selection pipeline | `main.rs` → `resolve_file_selectors` (`cli/src/selection.rs`) → `collect_selected_files` |
| Declaration run | `cli/src/declarations.rs` (`DeclarationScan`, `run`) |
| `NoSourceFiles` | `tree-hugger/lib/src/scanner.rs:155`; error code in `cli/src/report.rs` |
| File identity | `tree-hugger/lib/src/file/source_file.rs` (`file_identity`) |
| sniff git | `sniff/lib/src/filesystem/git/{types,status,discovery}.rs`; `GitRepo::discover`, `merge_conflicts()`, `file_changes()` (rich; not reusable) |
| gix | pinned `=0.84.0` with `revision`, `status`, `blob-diff`, `dirwalk`, `excludes` |

### Successful completion

- Every row of the spec's acceptance table has a passing test, on macOS and
  Linux locally; Windows/WSL2 paths follow the `os` skill and run in the
  existing CI schedule (no new CI cells).
- `--changed` appears on the four declaration commands only, with the equals
  form, default `HEAD`, rejects empty value and repeats (exit 2).
- Git/ref/conflict/bare/missing-repo failures exit 1 with an actionable
  message and print no report.
- An empty valid selection exits 0 in every report mode with the existing
  empty representations; other scanner errors are unchanged.
- A work assertion proves the sniff request does one status observation and
  no line-diff, network, or inventory work; a warm-cache test proves a real
  cache hit with and without `--changed`.
- README, `docs/symbol-reports.md`, tree-hugger skill, sniff Git docs and
  skill, and dependency docs are updated; topic pages never name this feature.
- `just test`, `just test-l2` where touched, and `just lint` pass in
  `tree-hugger` and `sniff`.

### Input robustness matrix

Not applicable: the feature adds no parser, manifest/lockfile reader, config
loader, or deserializer. The reference token and Git state are covered by the
acceptance-table cases (empty, invalid, non-commit, repeated).

### Spikes

None. The spec states no performance spike is required (the avoidable work is
line-diff and inventory enrichment, proven by work assertions, not timing).
The gix tree-to-tree diff capability is checked as a task in Phase 2 against
the already-enabled `blob-diff`/`revision` features rather than as a spike.

## Phase 1: Rulings and groundwork

### Necessary Rules

These need an author ruling (a recommendation is given for each; the plan
proceeds on the recommendation unless told otherwise).

- [x] **R1 — `--prelude` combination.** The spec's Open Question. Recommended:
  reject `--changed` with `--prelude` as an argument error (exit 2) before Git
  inspection, using clap `conflicts_with` like `--prelude`'s existing
  conflicts. Document in help and `symbol-reports.md`.
- [x] **R2 — Where the empty-success exception lives.** Recommended: when
  `--changed` is set and `collect_selected_files` returns `NoSourceFiles`,
  treat it as an empty selection; also treat an empty intersection the same
  way. Any other scanner error propagates. Commands without `--changed` are
  untouched.
- [x] **R3 — Ordering of validation.** Recommended: clap syntax → R1 conflict →
  Git discovery/reference/conflict/diff failure (exit 1) → file selection →
  intersection → report. Git work runs before any file is read or reported.
- [x] **R4 — Dependency features.** Recommended: sniff added to
  `tree-hugger-cli` with `default-features = false` (no `network`/`remote`/
  `metrics`); confirm sniff compiles that way, otherwise extract the minimal
  feature set and note it in `docs/dependencies.md`.
- [x] **R5 — Reference type in the sniff API.** Recommended: a small enum such
  as `ChangedBase { Omitted, Reference(String) }` so the unborn exception is
  type-enforced; `Reference` resolves to a commit (peeling tags) or errors.
  Final names follow sniff's style.
- [x] **R6 — Wider measurement.** The spec asks for no timing. Any wall-clock
  benchmark on large repositories is out of scope; the author may request one
  separately.

### Tasks

- [x] **Confirm rulings** — record the author's answers to R1–R6 (or the
  recommended defaults) at the top of an `implementation-log.md` in this
  feature directory.
- [x] **Baseline green** — run `just test` and `just lint` in `tree-hugger` and
  `sniff`; note pre-existing failures so they are not attributed to this work
  (the worktree already has uncommitted symbol-feature changes).
- [x] **Load skills** — read the `os`, `rust-testing`, `sniff`, `tree-hugger`
  and `biscuit-file` skills and `docs/cicd/test-inputs.md`; the new tests
  read no repository files, so no source-inputs declaration is expected.
- [x] **Shape the API** — write the signature, error enum variants (not a
  repository, bare, unresolved ref, non-commit ref, conflicts, git failure)
  and result type in the log. This is the contract Wave 1 of Phase 2 and
  Phase 3 both build on.

**Checkpoint:** rulings recorded, baseline known, API shape written.

## Phase 2: sniff focused path API

### Wave 1 (single owner: `sniff/lib/src/filesystem/git/`)

- [x] **Status path collection** — in `status.rs`, add a path-only
  classification over one `gix` status walk (rename tracking off, so renames
  are delete+add) yielding staged, unstaged and untracked native `PathBuf`s,
  plus index conflict paths from the same observation. Share classification
  with the existing code where practical without changing `file_changes()`,
  `repo_status()`, rename conventions or bare behavior. Ignored untracked
  files are excluded. Submodule entries are not descended into.
  - Pre-req: no line stats or blob loads on this path.
- [x] **Tree comparison** — resolve the base locally (no fetch) to a commit,
  peeling annotated tags; reject trees/blobs. Diff base tip vs `HEAD` tips
  directly (not merge-base), once, only when the base differs from `HEAD`.
  Include content, mode and type changes; no rename similarity.
  - Complexity: unborn `HEAD` with an explicit base compares against the
    empty tree; an omitted base on unborn `HEAD` uses the empty base; an
    explicit `HEAD` on unborn must fail; detached `HEAD` is valid.
  - Check that the already enabled `revision`/`blob-diff` features expose tree
    diff; add the minimal gix feature only if not.
- [x] **Public method** — on `GitRepo`, add the fallible method (R5 types)
  returning a deduplicated, deterministically ordered set of native
  repository-relative paths. Reject bare repos explicitly (unlike older
  APIs), return conflicts as an error naming the paths, and wrap gix failures
  with the repository and reference in the message. Rustdoc per the repo
  conventions (H2 sections, no HOW narration).

### Wave 2 (parallel; disjoint files)

- [x] **sniff L1 tests** — temp repos with explicit branch names and fixture
  identity, commits via library APIs (gix writes; git2 dev-dep only where gix
  lacks a write API), no dependence on signing. Cover: staged, unstaged, both,
  untracked, ignored-untracked excluded; staged edit undone on disk; committed
  edit undone locally; rename, deletion, copy, staged deletion then recreate;
  divergent branch tip comparison (not merge-base); commit id and tag base;
  empty/invalid/non-commit refs; omitted vs explicit `HEAD` on unborn;
  explicit base with unborn `HEAD`; detached `HEAD`; conflicts anywhere; bare
  and non-repo rejection; submodule/nested repo not descended; non-ASCII,
  spaces and (unix-only, `#[cfg]`) non-UTF-8 names.
- [x] **Work assertion** — extend the existing status-walk probe/counters to
  prove one status observation per request and zero line-stat, patch, network,
  or inventory work.
- [x] **sniff docs** — Git API docs and the sniff skill: the new method, local
  only cost, error behavior, comparison semantics (tip vs tip, union, unborn).

**Checkpoint:** `just test` and `just lint` pass in `sniff`; method usable
without the CLI.

## Phase 3: tree-hugger-cli wiring

### Wave 1 (single owner: clap surface in `cli/src/main.rs`)

- [x] **Flag definition** — add `--changed[=<ref>]` to the declaration args
  shared by `functions`, `types`, `symbols` and `classes` only (not
  `ScanOptions`, so `imports`/`lint`/`god-files`/`completions` reject it).
  Use optional value with `require_equals`, `default_missing_value`
  semantic of omitted → `HEAD`, a non-empty value parser (`--changed=` is an
  argument error), and clap's single-occurrence behavior for repeat
  rejection. Confirm `--changed main` leaves `main` positional. Allow with
  `--json-full`; `conflicts_with = "prelude"` per R1. Surface it in
  `given_flags`/help consistent with neighbors.
- [x] **Dependency** — add sniff to `tree-hugger/cli/Cargo.toml` per R4.

### Wave 2 (sequential; shared run path)

- [x] **Changed-set resolution** — new focused module (e.g.
  `cli/src/changed.rs`): discover the repo from the invocation's cwd (active
  checkout, linked-worktree safe), call the sniff method, map native
  relative paths onto the checkout root, drop absent paths before
  canonicalization, and key the set by `file_identity`. Any failure maps to
  an exit-1 message naming repository/reference and cause; no report printed.
  Runs before file selection (R3).
- [x] **Intersection** — after `collect_selected_files`, keep only files whose
  `file_identity` is in the changed set; dedupe; preserve scanner order. Git
  paths are candidates only: never fed into `explicit_files`, so fixtures and
  ignored files are not re-included. Files outside the discovered repository
  are dropped without a second discovery. Explicit files outside the package
  scan root but inside the repo remain eligible.
- [x] **Empty success** — per R2, short-circuit to the existing empty report
  for the active format: nothing analyzed, no cache lookups, existing
  envelope/`--count`/stats/trailing-newline conventions, `errors: []`.
- [x] **Cache key audit** — confirm `--changed` and the reference do not enter
  the symbol-cache key (no code change expected; fail the plan if one is
  needed).

**Checkpoint:** `hug symbols --changed` works on a scratch repo; non-`--changed`
runs byte-identical (golden fixtures unchanged).

## Phase 4: CLI tests

### Wave 1 (parallel; disjoint test modules under `cli/tests/`)

- [x] **Parser tests** — optional value, equals form, `--changed main`
  positional, empty value, repeated flag (exit 2), conflict with `--prelude`,
  rejection on `imports`/`lint`/`god-files`/`completions`, `--json-full` allowed.
- [x] **Selection tests** — acceptance-table rows for staged/unstaged/both/
  untracked, restored-on-disk, ignored untracked (incl. explicit selector),
  default fixtures vs explicitly targeted, rename/delete/recreate, supplied
  branch tip, positional/`--file`/`--exclude-files`/`--language` intersection
  including forced-language extensionless file, package invocation, linked
  worktree, outside-package explicit file, submodule/nested repo.
- [x] **Failure tests** — missing repo, bare repo, bad/non-commit ref,
  conflicts outside the package, corrupt Git data: exit 1, stderr names the
  cause, stdout empty (no success JSON).
- [x] **Empty-selection tests** — every report mode (terminal, plain, table,
  outline, JSON, JSON Lines, full JSON, `--count`, stats) with deleted-only,
  excluded, unsupported and out-of-selection changes exits 0 with the
  existing empty representation.
- [x] **Cache test** — cold scan then `--changed` scan reads the same file from
  cache (use existing cache-use stats); also the reverse order.
- [x] **Path tests** — spaces, non-ASCII, and unix non-UTF-8 names;
  case-insensitive filesystem spelling difference (host-gated).
- Tests use `assert_cmd` with the L1 fixtures; no real terminal windows. If an
  L2 harness test is touched, windows must not gain focus.

### Wave 2 (sequential)

- [x] **Full runs** — `just test` and `just lint` in `tree-hugger` and `sniff`;
  `just test-l2` in `tree-hugger` only if L2 files changed; confirm existing
  golden fixtures untouched.
- [x] **OS review** — apply the `os` skill checklist to `#[cfg]` and path
  comparison code; list which OS evidence was produced locally and which
  relies on the existing CI schedule.

**Checkpoint:** every acceptance-table row has a named passing test.

## Phase 5: Docs and close

### Wave 1 (parallel; disjoint files)

- [x] **tree-hugger docs** — CLI README and `tree-hugger/docs/symbol-reports.md`:
  examples for default `HEAD`, `=main` local-tip comparison, selector
  intersection, empty success, and the prelude policy. Written for a reader
  new to the repo, with a Mermaid diagram of the changed-set → intersection
  flow. Do not link to or name this feature.
- [x] **Skills** — `.claude/skills/tree-hugger/SKILL.md` and
  `.claude/skills/sniff/` guidance for the flag and the path-only method.
- [x] **Dependencies** — `docs/dependencies.md` and applicable per-area
  `dependencies.md` pages for the new sniff dependency.
- [x] **Help text** — final wording of `--changed` help, including the equals
  form and prelude conflict.

### Wave 2 (sequential)

- [x] **Comment and drift pass** — review `///` and `//` comments on every
  touched symbol; fix drift (code is authoritative) and report any found.
- [x] **Spec departures** — if the implementation departs from the spec, record
  it in `implementation-log.md` and correct the docs; leave `spec.md` as is.
- [x] **Final verification** — rerun `just test` and `just lint` in both areas;
  set the spec's `implemented`/`implemented_by` per the lifecycle; stop at
  "implementation complete, ready for review" (never move to `_completed`,
  never run `just complete`, never commit unless asked).

**Checkpoint:** docs match behavior, all tests green, ready for review.
