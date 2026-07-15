---
status: ready for planning and implementation
reviewed: true
review_iterations: 1
reviewed_by: codex/default
reviewed_on: 2026-07-14
rulings: index fallback/order plus Git surface names, value types, and incoming merge direction supplied by Ken 2026-07-13
inputs:
  - ../../docs/schemas/darkmatter.yaml
  - ../../docs/schemas/expression-functions.yaml
  - ../../docs/topics/context-variables.md
  - ../../lib/src/markdown/compose/context/catalog.rs
  - ../../lib/src/markdown/compose/context/capture/groups.rs
  - ../../lib/src/markdown/compose/context/capture/repo.rs
  - ../../lib/src/markdown/compose/context/capture/snapshot.rs
  - ../../lib/src/markdown/compose/expression/functions/mod.rs
  - ../../lib/src/markdown/compose/expression/functions/paths.rs
  - ../../lib/src/markdown/compose/expression/catalog/mod.rs
  - ../../../sniff/lib/src/filesystem/git/api.rs
  - ../../../sniff/lib/src/filesystem/git/status.rs
  - ../../../sniff/lib/src/filesystem/git/worktree.rs
  - ../../../sniff/lib/src/filesystem/git/remote_refresh.rs
related:
  - ../_completed/2026-06-15-context-vars-additions
  - ../_completed/2026-07-10-function-schemas
  - ../_completed/2026-07-08-single-sourcing-schema
  - ../2026-07-12-literal-expression
---

# More Is More: Filesystem Discovery, Git Context, and Conflict Prediction

**Status:** Ready for planning and implementation. This feature adds two
filesystem-index expression functions, three Git-aware runtime context
variables, and one Git-aware expression function. Filesystem discovery stays
local and portable; branch/worktree identity and both actual and predicted
merge conflicts are provided through `sniff`, keeping Darkmatter free of Git
subprocess parsing and giving every consumer one cross-platform Git authority.

> **Reader's note (review correction):** The draft described
> `gix::Repository::merge_commits` as a purely in-memory operation. In the
> pinned `gix` release, that high-level method can persist synthesized merge
> objects, consult the live index for attributes, and invoke configured merge
> drivers. Those behaviors conflict with this feature's committed-tip and
> side-effect-free contracts. The reviewed design therefore requires one
> hermetic `sniff` merge primitive: it uses object-memory storage, derives its
> temporary index and attributes from the captured `ours` commit, and never
> launches an external driver or filter. This is a safety and correctness
> requirement, not an optional implementation optimization.

## Goal

Add these Filesystem expression functions:

1. **`find_first_index(file) -> file`** — returns the lowest-indexed existing
   member of the file's index family in its directory.
2. **`find_last_index(file) -> file`** — returns the highest-indexed existing
   member of the file's index family in its directory.

Add these context variables:

| Variable | Type | Meaning |
|----------|------|---------|
| `ctx.merge_conflicts` | `string[]` | Files currently in an unmerged index state; empty when there are none or no repository exists |
| `ctx.branch` | `string | null` | Current local branch name; null outside a repository or at detached HEAD |
| `ctx.worktree` | `string | null` | Current linked-worktree name; null in the main/default worktree or outside a repository |

Add this expression function:

```text
predict_conflicts(branch: string) -> string[] | error
```

The function predicts which files would conflict if the named local branch
were merged **into the caller's current branch**. It performs the merge in
memory, returns an empty array for a clean merge, and never changes the index,
working tree, refs, or object database.

## Motivation

Authors composing indexed review, draft, and artifact series need to locate the
first or latest file that actually exists without hard-coding a suffix. The
existing index-family functions inspect or transform one path but cannot inspect
the directory to find the real endpoints of the series.

Prompts that coordinate feature work and reviews need to answer two related
questions:

1. **What state am I in?** — repository branch, linked worktree, and any merge
   conflicts already present in the index.
2. **What would happen next?** — whether merging another branch into the
   caller's branch would produce conflicts, and in which files.

Darkmatter already captures repository, file-change, and monorepo context
through `sniff`, but it does not expose the current branch/worktree or the
existing `sniff::filesystem::git::merge_conflicts_at` result. `sniff` also
already probes a `gix` commit merge when computing the boolean
`WorktreeEntry::has_conflicts`. That helper supplies the integration seam, but
not the final primitive: the pinned high-level `gix` call has observable and
live-index behaviors that this feature must remove. One hermetic path-producing
primitive will replace it, then project the result into both the boolean
worktree summary and Darkmatter's new function.

## Expression Functions: `find_first_index` and `find_last_index`

**Status:** Ready for planning and implementation. Two new read-side
Filesystem expression functions that resolve a file reference to the
lowest- or highest-indexed existing sibling in the same directory.

### Goal

Add two expression functions to the Filesystem category:

1. **`find_first_index(file) -> file`** — returns the lowest-indexed existing
   member of the file's index family in its directory.
2. **`find_last_index(file) -> file`** — returns the highest-indexed existing
   member of the file's index family in its directory.

Both are read-side, local-only (no remote), directory-scanning functions that
extend the existing indexed-file family (`is_indexed_file`, `file_index`,
`increment_file_index`, `decrement_file_index`, `basename_without_index`).
They are the **first** functions in this family to read the directory itself;
all of the existing ones are pure string transforms over one path.

### Motivation

- Authors compose documents that reference "the current review" or "the newest
  draft" of an indexed series (`review-1.md`, `review-2.md`, `review-3.md`).
  Today there is no way to ask "give me the newest one" without hard-coding the
  suffix. `increment_file_index` produces the _next_ name whether or not it
  exists; these two functions instead resolve to a name that **exists on disk**.
- The pair rounds out the index-family algebra: `file_index` reads the ordinal,
  `increment`/`decrement` walk it arithmetically, and `find_first_index` /
  `find_last_index` locate the endpoints of the real, on-disk series.

### Behavior

#### Worked examples

Given a directory containing `foo.md`, `foo-2.md`, `foo-3.md`:

| Call                           | Result     |
| ------------------------------ | ---------- |
| `find_first_index("foo-2.md")` | `foo.md`   |
| `find_last_index("foo-2.md")`  | `foo-3.md` |
| `find_first_index("foo.md")`   | `foo.md`   |
| `find_last_index("foo.md")`    | `foo-3.md` |

Given a directory whose only member of the family is `foo-2.md` (no `foo.md`,
`foo-1.md`, `foo-3.md`, …):

| Call                           | Result     |
| ------------------------------ | ---------- |
| `find_first_index("foo-2.md")` | `foo-2.md` |
| `find_last_index("foo-2.md")`  | `foo-2.md` |

#### The index family of a file

For an input file, its **index family** is the set of directory entries that
share the same base stem **and** the same extension, where the base stem is the
input's stem with any indexed suffix removed (the same operation
`basename_without_index` performs, via `indexed_stem_info`).

An entry `E` in the input's directory is a family member iff **both** hold:

1. `E`'s extension equals the input's extension (case-sensitive, exact
   comparison of the trailing extension string), and
2. `E`'s stem is either exactly the base stem (the **unindexed base**, e.g.
   `foo.md`) or an indexed form of the base stem (`indexed_stem_info(stem)`
   yields a base equal to the input's base stem, e.g. `foo-3` → base `foo`).

Notes:

- The base stem is derived from the **input**, so `find_last_index("foo-2.md")`
  and `find_last_index("foo.md")` scan the same family.
- Extension must match exactly: `foo-2.md` and `foo-2.txt` are different
  families. `foo.md` and `foo.markdown` are different families.
- Non-family neighbors are ignored: `food.md`, `foo-bar.md` (no numeric
  suffix), and `sub/foo-9.md` (different directory) are never members.
- The grammar for "indexed form" is exactly the existing one
  (`(?P<base>.+)-(?P<digits>[0-9]+)` with the pre-hyphen guard): `foo-3`
  matches with base `foo`; `foo--3`, `foo-`, `foo-x` do not.

#### Ordering within a family

Members are ordered by a single key so `first` = minimum and `last` = maximum:

- The **unindexed base** (`foo.md`) ranks **below all** indexed members. It is
  the "first" of its family. This matches the worked example
  (`find_first_index("foo-2.md")` → `foo.md`) and is consistent with
  `increment_file_index("foo.md")` → `foo-2.md` (the base sits before its
  indexed siblings).
- Indexed members order by their **numeric** index ascending
  (`foo-2.md` < `foo-3.md` < `foo-10.md`), i.e. `10` sorts after `3`, not
  lexicographically. Zero-padding does not affect ordering
  (`foo-002.md` and `foo-2.md` carry the same numeric index).

Implementation guide: order by `Option<u64>` where the unindexed base is `None`
and an indexed member is `Some(index)`; Rust's derived `Ord` places `None`
before every `Some`, giving base-first, numeric-ascending order. Duplicate
numeric ordinals from differing padding (`foo-2.md` vs `foo-002.md` both
present) are not expected in practice; when they occur, break ties by the raw
filename so the result is deterministic.

#### Candidate set and the "no siblings" fallback

The candidate set is the family members that **exist on disk** in the input's
directory (obtained via one `std::fs::read_dir` of the resolved parent).

- If the candidate set is **non-empty**, return the minimum (`find_first_index`)
  or maximum (`find_last_index`) by the ordering above. The returned name is the
  **actual on-disk filename**, so its real extension casing and zero-padding are
  preserved verbatim (these functions never re-format an index).
- If the candidate set is **empty** — the directory does not exist, or no entry
  matches the family (the input itself does not exist and has no siblings) —
  return the **input path unchanged** (in the same portable form the other
  path functions emit). This yields the second worked-example row: an input
  whose only presence is itself returns itself, and it degrades gracefully like
  the other path-shape functions that operate on missing files.

Because the input file is normally one of its own family's on-disk members, a
lone `foo-2.md` naturally lands in the candidate set and is returned as both
first and last. The empty-set fallback only fires when nothing in the family
exists.

#### Path resolution, display shape, and directory scanning

- The single argument is resolved with the **same rules** as the rest of the
  index family — `resolve_path_shape` (`FileReference` resolution first, then a
  deterministic path shape against `ctx.base_dir` for missing files, honoring
  `@`/`!`/`./`/`../` and magic paths). This gives the absolute path whose
  `parent()` is the directory to scan and whose `file_stem`/extension seed the
  family.
- The result is rendered through `make_portable_relative(&result_path,
&ctx.base_dir)` — the same display policy used by `increment_file_index` /
  `decrement_file_index` — so composed Markdown stays portable (repo-root
  relative, base-dir relative, `~`-aliased, or absolute, `/`-separated). The
  directory portion of the input is preserved: the result lives in the same
  directory as the input.

#### Remote URLs and local-only contexts

- HTTP(S) URLs are **rejected** with an error, exactly as `resolve_path_shape`
  already does for the path family. A directory scan has no remote analogue.
- These functions touch only the local filesystem and require **no remote
  runtime**, so they are valid in **every** resolution context — body
  interpolation _and_ the local-only frontmatter passes (both interpolation
  passes and the `$()` ternary). They behave like `file_exists`'s local branch,
  not like `frontmatter`/`load_markdown` remote reads.

#### Null and error handling

- Follows the family contract via `any_null`: a `null` argument returns
  `Value::Null` (not an error). The functions are marked `fallible` in the
  catalog because path resolution can fail (e.g. a rejected remote URL, or a
  `vault:` reference to a missing file), matching every other Filesystem
  function's `returns.fallible: true`.
- A non-string, non-null argument is an arity/type error via
  `require_string_expr`, consistent with `increment_file_index` et al.

### Catalog Changes (`darkmatter/docs/schemas/expression-functions.yaml`)

Add two entries in the **Filesystem** category. Because the authored `order`
integers `0..=87` are contiguous around the index family (there is no free slot
between `decrement_file_index` = 68 and `basename` = 69), assign the pair the
next unused global orders so no existing entry is renumbered (Rule 3 — surgical):

- `find_first_index` — `order: 88`
- `find_last_index` — `order: 89`

`md schema about` groups by category then sorts by `order`, so within the
Filesystem group these two render at the end (after `has_command`); the pair
stays adjacent. If a reviewer prefers them to render immediately after
`decrement_file_index`, that is a deliberate data renumber of the trailing
Filesystem block and can be done in the same commit — but it is not required
for correctness.

Both examples are **executable** against the existing example fixture directory
(`catalog/mod.rs::make_fixture`), which already writes `review-1.md` and
`review-2.md` (and no `review.md`). No fixture additions are needed:

```yaml
- name: find_first_index
  category: Filesystem
  order: 88
  description: Returns the lowest-indexed existing sibling of the file in its
      directory (the unindexed base sorts first); returns the file itself
      when it has no indexed siblings.
  overloads:
      - parameters:
            - name: file
              type: file
        returns:
            type: file
            fallible: true
        example:
            expression: find_first_index("review-2.md")
            result: review-1.md
            verification: executable
- name: find_last_index
  category: Filesystem
  order: 89
  description: Returns the highest-indexed existing sibling of the file in its
      directory; returns the file itself when it has no indexed siblings.
  overloads:
      - parameters:
            - name: file
              type: file
        returns:
            type: file
            fallible: true
        example:
            expression: find_last_index("review-1.md")
            result: review-2.md
            verification: executable
```

Family verification against `make_fixture` (`review-1.md`, `review-2.md`
present; no `review.md`):

- `find_first_index("review-2.md")` → candidates `{review-1.md, review-2.md}`,
  min → `review-1.md`. ✓
- `find_last_index("review-1.md")` → candidates `{review-1.md, review-2.md}`,
  max → `review-2.md`. ✓

### Runtime Registration and Handlers

Add executable behavior in the owning Rust domain slice; do not duplicate
descriptor metadata (see the darkmatter skill's "Expression Function
Registrations" contract).

1. **`functions/paths.rs`** — two new `FunctionBinding`s in `BINDINGS`, both
   `EvaluationMode::Context` with a `FunctionHandler::Context(...)` pointer,
   mirroring `increment_file_index`. Aliases: `findfirstindex` /
   `findlastindex` (the family uses lower-no-underscore aliases uniformly).

2. **`functions/mod.rs`** — two handlers,
   `find_first_index_fn` / `find_last_index_fn`, implemented over a shared
   private helper to avoid duplication, e.g.:

    ```text
    fn find_index_endpoint(name, args, ctx, Endpoint::First | Endpoint::Last)
        -> Result<Value, ExpressionError>
    ```

    The helper:
    - `require_args_expr(name, args, 1)?;` and `any_null` → `Value::Null`.
    - `resolve_path_shape` the argument (rejects remote URLs).
    - Compute base stem + extension from the resolved basename
      (`file_stem` / `file_extension`, `indexed_stem_info` to strip the input's
      own index), and the parent directory.
    - `std::fs::read_dir(parent)`: for each entry, take its file name, split
      stem/ext, test family membership (ext equals input ext; stem is the base
      or an indexed form of the base), and record `(Option<u64> ordinal, name)`.
    - If any members exist, pick min/max by `(ordinal, name)`; else fall back to
      the resolved input path.
    - Return `Value::String(make_portable_relative(&chosen_path, &ctx.base_dir))`.

    Reuse the existing `indexed_stem_info`, `file_stem`, `file_extension`, and
    `make_portable_relative` helpers; introduce no parallel grammar.

### Cross-Platform Considerations

- Directory reads via `std::fs::read_dir` and per-entry `file_name()` are
  portable across macOS, Windows, and Linux.
- Extension and stem comparisons operate on the already-normalized display
  basename (forward-slash rendering happens only at output via
  `make_portable_relative`), so Windows `\` separators do not leak into
  membership tests.
- Filename comparison is **case-sensitive** (matching the rest of the index
  family and `is_indexed_file`). On case-insensitive filesystems (default
  macOS/Windows), the physical directory cannot hold two names differing only
  in case, so case-sensitive membership testing is safe and does not change
  observed results.
- Ordering is numeric on the parsed index, never lexicographic, so
  `foo-10.md` correctly sorts after `foo-3.md` on every platform.

### Testing

#### Unit (in `functions/mod.rs` tests, using a `tempfile::TempDir` fixture)

- **first/last across a full family** — dir `{foo.md, foo-2.md, foo-3.md}`:
  `find_first_index("foo-2.md")` → `foo.md`; `find_last_index("foo-2.md")` →
  `foo-3.md`; same results when the input is `foo.md`.
- **no siblings → identity** — dir `{foo-2.md}` only:
  both functions return `foo-2.md`.
- **empty candidate set / missing input → identity** — input `bar-4.md` with no
  `bar*.md` present: both return `bar-4.md` unchanged.
- **numeric vs lexicographic ordering** — dir `{foo-2.md, foo-10.md}`:
  `find_last_index("foo-2.md")` → `foo-10.md` (not `foo-2.md`).
- **zero-padding preserved verbatim** — dir `{foo.md, foo-002.md}`:
  `find_last_index("foo.md")` → `foo-002.md` (real on-disk name, not
  reformatted).
- **extension isolation** — dir `{foo-2.md, foo-3.txt}`:
  `find_last_index("foo-2.md")` → `foo-2.md` (`.txt` excluded).
- **non-family neighbor isolation** — dir `{foo-2.md, food-9.md, foo-bar.md}`:
  `find_last_index("foo-2.md")` → `foo-2.md`.
- **directory isolation** — a sibling `sub/foo-9.md` does not affect a
  `find_last_index("foo-2.md")` scanning the parent directory.
- **null propagation** — `find_first_index(null)` → `Value::Null`.
- **remote rejection** — `find_last_index("https://example.com/foo.md")` errors.

#### Catalog / parity (existing tests exercise these automatically)

- `descriptor_signature_set_equals_dispatchable_signature_set` and
  `every_descriptor_overload_is_dispatchable_at_its_declared_arity` — pass once
  both bindings and catalog entries land.
- `every_example_evaluates_to_its_declared_result` — the two executable
  examples evaluate against `make_fixture`'s `review-1.md`/`review-2.md`.
- Add both signatures to the `feature_functions_are_present_in_exported_catalog`
  expectation list (Phase 4 / filesystem block).

#### Docs

- Regenerate/refresh `darkmatter/docs/topics/darkmatter-expressions.md` if it
  enumerates the Filesystem functions, and re-hash if the file carries a
  frontmatter `hash:` (`md hash <file>`).

### Open Questions

1. **Base-first ranking** — the spec ranks the unindexed base (`foo.md`) as the
   _first_ of its family (below `foo-1.md`), matching the worked example.
   **Ruled** by the prompt's example; recorded here for the review trail.
2. **Empty-family fallback** — **Ruled by Ken 2026-07-13: return the input
   verbatim** (identity-return), not an error or `null`. Graceful, matches the
   family's tolerance of missing files.
3. **Order placement** — **Ruled by Ken 2026-07-13: append at 88/89** with no
   renumbering (Option A). `order` is a display-sort key only; the pair renders
   together at the end of the Filesystem group in `md schema about`. The
   family-adjacent renumber (Option B) was declined.

## Git Conflict Semantic Summary

The two conflict surfaces intentionally answer different questions:

| Surface | Source | Includes dirty working-tree changes? | Meaning of empty array |
|---------|--------|---------------------------------------|------------------------|
| `ctx.merge_conflicts` | Current repository index stages | No; it observes only non-zero index stages already written by Git | No unresolved index entries, or no repository |
| `predict_conflicts(branch)` | In-memory merge of two committed branch tips | No | The committed tips merge without unresolved conflicts |

`ctx.merge_conflicts` is an observation of current state. `predict_conflicts()` is
a prediction. Neither scans files for textual conflict-marker strings; Git's
index/merge model is authoritative.

## Shared Git Authority (`sniff`)

All repository discovery and Git operations in this feature belong in
`sniff`. Darkmatter consumes typed APIs and does not shell out to `git`, parse
porcelain output, or duplicate `gix` merge behavior.

### Existing APIs to reuse

- `GitRepo::discover(path)` / `GitRepo::try_current_branch()` — current branch
  identity.
- `get_current_worktree_name(path)` — basename of a linked worktree directory,
  or `None` for the main worktree/outside a repository.
- `merge_conflicts_at(path)` — repo-relative paths with non-zero index stages,
  covering unresolved merge, rebase, cherry-pick, and revert state.
- the existing `WorktreeEntry::has_conflicts` merge probe — the call site to
  migrate onto the new hermetic path-producing primitive. The high-level
  `Repository::merge_commits` invocation itself is not reusable unchanged.

### New predicted-conflict API

Add a public, read-only `sniff` API with the semantic shape:

```rust
pub fn merge_conflicts_with_branch_at(
    path: &Path,
    incoming_branch: &str,
) -> Result<Vec<PathBuf>>
```

The exact module placement follows the existing Git API facade
(`filesystem/git/api.rs`, re-exported from `filesystem::git` and
`filesystem`). The behavior is fixed by this spec even if planning chooses a
slightly different Rust symbol name.

Internally, replace the existing boolean helper with one commit-pair authority:

```text
merge_conflicts_between(repo, ours, theirs) -> Result<Vec<PathBuf>>
```

`WorktreeEntry::has_conflicts` becomes `!paths.is_empty()` over this helper.
The public branch-oriented function resolves refs and delegates to it. There
must not be separate boolean and path-producing merge algorithms, and no caller
may bypass the hermetic setup by invoking the old high-level merge directly.

### Hermetic merge boundary

The commit-pair helper owns these invariants:

- Clone/open the trusted repository into a probe-local view and enable
  `gix::Repository::with_object_memory()` before any merge. Synthesized merged
  blobs, trees, or virtual merge bases remain visible to that probe but never
  reach the on-disk object database.
- Resolve and peel `ours` and `theirs` to commit IDs once, before the merge.
  Ref movement after that snapshot does not change the running prediction.
- Build attribute and index state from the captured `ours` commit tree. Never
  read the live worktree index or worktree content; a missing, corrupt, staged,
  or already-conflicted index therefore cannot change or block prediction.
- Use Git-compatible rename-aware tree options from the repository's safe merge
  configuration, with `TreatAsUnresolved::git()` as the conflict classifier.
  The previous `Options::default()` boolean probe did not enable the repository's
  rename profile; migrating `WorktreeEntry::has_conflicts` is an intentional
  correctness widening for rename-related conflicts.
- Never execute a configured merge driver, clean/smudge/process filter, hook,
  or other command. If an external driver/filter or renormalization setting
  would be required for a participating path, return an unsupported-merge-
  configuration error naming the setting/path. Silently substituting the text
  driver would produce a result that looks authoritative but may disagree with
  a real merge.
- Leave `fail_on_conflict` disabled so every unresolved path is collected, not
  merely the first one.

If the pinned `gix` facade cannot provide all of these controls through
`Repository::merge_commits`, planning must use the lower-level `gix` merge
plumbing. Weakening an invariant to preserve the convenience call is not an
acceptable implementation trade-off.

### In-memory merge algorithm

For `merge_conflicts_with_branch_at(path, incoming_branch)`:

1. Discover the repository containing `path` using `sniff`'s trusted `gix`
   discovery.
2. Require an attached current local branch. Snapshot and peel its tip commit as
   **ours**.
3. Normalize `incoming_branch` by removing at most one leading `refs/heads/`,
   validate the remainder as a complete local-branch name, rebuild the full
   ref, and perform an exact ref lookup. Do not use rev-parse, DWIM, prefix, tag,
   SHA, or remote-tracking resolution. Snapshot and peel its tip as **theirs**.
   A short name such as `feature/foo` and its fully qualified
   `refs/heads/feature/foo` spelling address the same ref.
4. Create the hermetic merge view above and merge `theirs` into `ours` with the
   shared labels/options. `gix` owns merge-base and virtual-merge-base
   calculation.
5. Materialize the merge result into a temporary **in-memory** index, apply the
   conflicts with `TreatAsUnresolved::git()`, and collect the paths whose index
   stage is not `Unconflicted`. This is the semantic authority for the result;
   do not infer paths by selecting only `ours.location()` or
   `theirs.location()` from a conflict record.
6. Convert Git byte paths through `sniff`'s existing lossy public-path helper,
   then sort and deduplicate by the portable `/`-separated string form before
   returning repository-relative `PathBuf`s.

The temporary-index definition aligns both conflict surfaces: actual conflicts
are non-zero stages in the live index, while predicted conflicts are the
non-zero stages that the in-memory merge would produce. Ordinary content
conflicts normally contribute one path. Rename/rename, directory/file, and
other structural conflicts can contribute multiple paths; all staged paths are
preserved.

The analysis uses locally available objects and refs only:

- no fetch or remote refresh;
- no hooks, merge drivers, filters, or other commands;
- no checkout, index write, ref update, object write, lock file, or temporary
  worktree;
- no ambient `git` executable requirement.

The prediction describes the captured local commit tips, their committed
attributes, and safe repository merge settings at evaluation time. A later
fetch, commit, rebase, ref move, or merge-configuration change can naturally
change the answer. Ordinary working-tree and index edits cannot.

### Errors versus clean results

An empty vector means a merge was successfully analyzed and had no unresolved
conflicts. Conditions that prevent a trustworthy prediction are errors, not
empty results:

- `path` is outside a Git repository;
- HEAD is unborn or detached, so there is no caller branch;
- the incoming local branch does not exist or is not a commit;
- the branches have no merge base/unrelated histories;
- required Git objects are missing or corrupt;
- trust, permission, or object-database access fails;
- an applicable external merge driver/filter or renormalization rule prevents
  a command-free prediction.

Merging the current branch into itself, merging an ancestor, or merging a
branch already contained by ours succeeds with an empty vector.

### Cross-platform path contract

The shared `sniff` APIs return repository-relative `PathBuf`s after applying the
same explicit lossy UTF-8 boundary as existing Git status projections.
Darkmatter projects them to JSON strings with `/` separators on macOS,
Windows, and Linux without changing the order. Sorting and deduplication happen
once in `sniff` over that portable string form, so every consumer sees the same
stable result and this feature does not introduce a second path-encoding
policy.

## Context Variables

### `ctx.branch`

```yaml
branch: "string(generated) -> Current local Git branch name, or null outside a
  repository or at detached HEAD."
```

- Returns the short local branch name (`feature/foo`, not
  `refs/heads/feature/foo`).
- Returns `null` outside a repository, for an unborn HEAD, or at detached HEAD.
- Does not infer a branch from remote containment or worktree directory name.
- Reads known local refs only and never refreshes remotes.

### `ctx.worktree`

```yaml
worktree: "string(generated) -> Current linked Git worktree name, or null in
  the main worktree or outside a repository."
```

- Reuses `sniff::filesystem::git::get_current_worktree_name`.
- The value is the basename of the linked worktree root, canonicalized first
  when the filesystem permits. It is a worktree name, not a branch name; the
  two may differ.
- Returns `null` in the main/default worktree, a bare repository, outside a
  repository, or when no valid UTF-8 basename is available.

### `ctx.merge_conflicts`

```yaml
merge_conflicts: "string[](generated; required) -> Repository-relative paths
  currently in an unresolved Git index state."
```

- Reuses `sniff::filesystem::git::merge_conflicts_at`.
- Reports non-zero index stages created by merge, rebase, cherry-pick, or
  revert operations. It does not grep working files for `<<<<<<<` markers.
- Returns a real JSON string array, sorted and deduplicated.
- Returns `[]` when the index has no unresolved entries and when the caller is
  outside a repository.
- The empty array is intentionally useful in conditions because Darkmatter's
  established truthiness model treats empty arrays as falsy and non-empty
  arrays as truthy:

  ```text
  ctx.merge_conflicts ? "resolve conflicts" : "working tree has no conflicts"
  ```

### Demand-driven capture group

Add a dedicated `ContextGroup::Git` containing exactly:

- `branch`
- `worktree`
- `merge_conflicts`

These values require only repository discovery, ref identity, worktree
identity, and an index-stage walk. They must not trigger monorepo package
discovery, full working-tree status, document inventory, or remote access when
no other context group needs those probes.

`ContextCapture` continues to build one snapshot per compose run. When the Git
group is requested, it derives all three values from the caller/launch-area
repository. Add handle-oriented helpers behind the existing path facades where
needed so this group performs one trusted discovery and shares that repository
handle; repeated path-based discovery for each key is not the intended design.
Every transcluded document sees the same early-bound snapshot.

Capture failures follow the established partial-runtime policy:

- if trusted repository discovery fails, record one
  `PartialRuntimeCapture { area: "git", ... }`, project `branch`/`worktree` as
  `null` and `merge_conflicts` as `[]`;
- after discovery succeeds, probe the three fields independently. A field
  failure records a diagnostic whose detail names that field and substitutes
  only that field's neutral value (`null`, `null`, or `[]` respectively);
  successfully captured sibling values are preserved;
- do not abort composition solely because optional context discovery failed.

Absence of a repository is an ordinary value state, not a diagnostic.

### Schema-derived catalog and presentation

`darkmatter/docs/schemas/darkmatter.yaml` remains the authored authority for
names, types, descriptions, generated/required flags, and catalog order. Add
the three properties to its `ctx` mapping. Place `branch`/`worktree` with the
repository identity fields and `merge_conflicts` with the file-change fields;
the YAML declaration index is also the generated catalog's stable display
order.

Update the Rust-only presentation grouping:

- `branch` and `worktree` — category **Repository**, subsection **Git**;
- `merge_conflicts` — category **File Changes**, subsection **Conflicts**.

Capture grouping and presentation grouping are intentionally independent: all
three share the cheap `ContextGroup::Git` probe even though documentation
presents current conflicts with other file-change information.

Regenerate the marked catalog block in
`docs/topics/context-variables.md` through the existing
`md schema about --verbose` projection. Do not hand-author a parallel variable
catalog.

## Expression Function: `predict_conflicts(branch)`

### Signature and direction

```text
predict_conflicts(branch: string) -> string[] | error
```

The direction is fixed:

```text
ours   = caller's current local branch tip
theirs = local branch named by `branch`
result = unresolved paths if `theirs` were merged into `ours`
```

For a caller on `feature/review`:

```text
predict_conflicts("main")
```

answers which files would conflict when running the conceptual operation
`merge main into feature/review`. It does not answer the reverse merge.

### Caller repository anchor

"Caller" means the directory that launched composition, matching the existing
context-variable contract—not the directory containing a transcluded or
remotely referenced Markdown document.

Add one explicit crate-visible `ResolutionContext` accessor for this concept:

```rust
pub(crate) fn caller_dir(&self) -> &Path
```

It returns `file_ref_fallback_dir` when present and `base_dir` otherwise.

Production compose surfaces already thread the captured launch area through
`file_ref_fallback_dir`; small programmatic callers that construct only
`ResolutionContext::new(base_dir)` naturally use that base directory. The Git
function calls the accessor rather than reading the fallback field directly,
so the ownership meaning is named and testable.

A prompt stored in repository A and invoked while the caller is in repository
B evaluates `predict_conflicts()` against repository B. This matches `ctx.branch`,
`ctx.worktree`, and `ctx.merge_conflicts` and prevents document location from
silently changing Git identity.

### Input, null, and error behavior

- Exactly one argument is required.
- A string names an exact local branch; no fuzzy matching, prefix matching, tag
  resolution, SHA resolution, or implicit remote-tracking lookup occurs. Ref
  syntax is validated before lookup, and lookup must use the full
  `refs/heads/...` name rather than rev-parse. Except for accepting/removing one
  `refs/heads/` prefix, non-empty input is not trimmed or rewritten.
- `null` propagates to `null`, following the established read-side function
  contract. Catalog return typing continues to show `string[] | error`; null
  propagation is a language-wide rule.
- Non-string, non-null input is an expression type error.
- Empty/whitespace-only names and unknown branches are errors.
- Repository absence, detached/unborn HEAD, unrelated histories, missing
  objects, unsafe external merge configuration, and Git access failures are
  expression errors with actionable messages.
- A successfully analyzed clean merge returns `[]`; errors never collapse to
  `[]`.

### Committed-state boundary

The function compares the two committed branch tips. It deliberately ignores:

- unstaged changes;
- staged but uncommitted changes;
- untracked files;
- conflict stages already present in the current index.

That includes staged `.gitattributes`: merge attributes come from the captured
`ours` tree, not from the live index. A missing or corrupt live index must not
prevent prediction. Committed `.gitattributes` and safe repository merge
settings remain part of the merge model; applicable executable drivers/filters
fail as described above.

Those states can cause a real `git merge` command to refuse or require cleanup,
but they are not part of the branch-to-branch three-way merge. Authors inspect
`ctx.dirty_files`, `ctx.staged_files`, `ctx.untracked_files`, and
`ctx.merge_conflicts` separately when they need readiness checks.

### Output

The result is a real JSON string array of portable repository-relative paths,
sorted and deduplicated under the shared path contract. A bare interpolation
renders the existing line-separated array representation; list-formatting
functions provide alternative presentation:

```md
{{ as_unordered_list(predict_conflicts("main")) }}
```

Because empty arrays are falsy, the direct condition is ergonomic:

```text
predict_conflicts("main") ? "merge needs conflict resolution" : "clean merge"
```

### Catalog entry

Add a new **Git** category to the authored expression-function catalog. The
same-day `finding-indexes` feature reserves global orders 88 and 89, so use the
next unused order without renumbering existing entries:

```yaml
    - name: predict_conflicts
      category: Git
      order: 90
      description:
          Returns the repository-relative paths that would conflict if the
          named local branch were merged into the caller's current branch.
      overloads:
          - parameters:
                - name: branch
                  type: string
            returns:
                type: string
                array: true
                fallible: true
            example:
                expression: predict_conflicts("feature/example")
                result: src/config.rs
                verification: display-only
                reason:
                    Requires a Git fixture with divergent local branch tips.
```

The authored catalog remains the single authority for signature,
documentation, category, order, and example. Runtime registration contains no
duplicate descriptor metadata.

### Runtime registration

- Add a focused `functions/git.rs` domain slice with a
  `FunctionBinding { canonical: "predict_conflicts", aliases:
  &["predictconflicts"], evaluation: Context, ... }`.
- Register the slice in `BINDING_GROUPS`.
- The context handler validates the argument, obtains
  `ResolutionContext::caller_dir()`, calls
  `sniff::filesystem::git::merge_conflicts_with_branch_at`, and projects the
  already sorted portable paths to `Value::Array` without a second ordering or
  encoding policy.
- Convert `sniff` failures into the existing expression error model without
  losing the branch name or repository anchor from the message.

The function is valid in every local expression surface, including body
interpolation, frontmatter interpolation, and `$()` ternary expressions. It
needs no remote-fetch runtime and never performs network I/O.

## DMLS Integration

No Git operation runs in DMLS. The existing schema/catalog integrations supply
editor intelligence passively:

- the schema-derived `ctx.*` catalog adds completion and hover for
  `ctx.branch`, `ctx.worktree`, and `ctx.merge_conflicts`, including their exact
  nullable/array types;
- the expression-function catalog adds completion, signature details, and
  documentation for `predict_conflicts`;
- DMLS never evaluates the function, discovers a repository, reads the index,
  or simulates a merge while serving editor requests.

Add parity tests proving the three context descriptors and the function
descriptor reach DMLS through the shared catalogs. Do not add hard-coded DMLS
name lists.

## Performance and Safety

- Branch/worktree capture is ref/config metadata only; conflict-state capture
  is one index walk.
- Context values are captured once per compose run and reused across the
  document graph.
- Conflict prediction cost includes merge-base traversal, changed-tree/blob
  merging, and construction of the probe-local index used for exact stage-path
  projection. Object reads may use `gix`'s existing repository caches; no
  global persistent cache is added by this feature.
- Every production Git operation is pure Rust through `sniff`/`gix`, preserving
  macOS, Windows, Linux, and WSL portability.
- The function is read-only by construction. Regression tests snapshot HEAD,
  refs, index bytes, worktree status, relevant files, and the on-disk object-ID
  set before/after evaluation. At least one clean auto-merge and one
  multiple-merge-base fixture must synthesize objects so the object-memory
  assertion is exercised rather than vacuously passing on conflict-only input.
- No command execution, hooks, credentials, remote access, or network consent
  surface is introduced.

## Documentation and Single-Sourcing

Update alongside implementation:

- `darkmatter/docs/schemas/darkmatter.yaml` — the three context descriptors;
- `darkmatter/docs/topics/context-variables.md` — capture-group table and
  regenerated catalog block;
- `darkmatter/docs/schemas/expression-functions.yaml` — the function descriptor;
- `darkmatter/docs/topics/darkmatter-expressions.md` — Git function semantics,
  committed-state boundary, and examples;
- `sniff` public API docs and its Git documentation — actual versus predicted
  conflict APIs;
- the `sniff`, `rust-devops`, and `darkmatter` skills when their public
  architecture/workflow descriptions need the new surface.

No dependency document changes are expected: Darkmatter already depends on
`sniff`, and `sniff` already uses `gix` merge support. If planning needs a new
direct plumbing-crate dependency because the `gix` facade cannot enforce the
hermetic boundary, update `docs/dependencies.md` and the affected area document
instead of leaving this statement stale.

## Compatibility

- The context variables and expression function are additive names.
- Existing context capture remains demand-driven and produces byte-identical
  values for existing keys.
- Existing expression-function names, aliases, order values, and behavior do
  not change. `predict_conflicts` uses order 90 so the pending 88/89 pair remains
  stable.
- Existing `WorktreeEntry::has_conflicts` retains its public meaning (“would
  this branch-tip merge have unresolved conflicts”) and derives from the new
  shared path result. Rename-aware answers may widen relative to the old
  `Options::default()` probe; this is an intended correctness fix. Unsafe
  external merge configuration now produces an error rather than being
  executed or silently approximated.
- No `git` binary becomes a runtime dependency.

## Non-Goals

- No automatic merge, checkout, rebase, cherry-pick, abort, or conflict
  resolution.
- No inclusion of dirty/staged/untracked content in branch-tip prediction.
- No remote fetch, refresh, or implicit `origin/<branch>` lookup.
- No support for tags, arbitrary revspecs, commit SHAs, or remote-tracking refs
  as the `branch` argument in v1.
- No prediction of whether Git would refuse a merge because local changes would
  be overwritten; existing context variables describe local changes.
- No conflict hunks, marker text, side labels, or suggested resolutions—only
  paths.
- No execution or emulation of repository-defined merge drivers or external
  filters. Repositories that require them receive an explicit unsupported
  configuration error in v1.
- No DMLS-time Git discovery or function evaluation.
- No new persistent cache or background repository watcher.

## Test Strategy and Oracles

All feature tests are L1: they use process-local APIs and disposable
repositories, not a real terminal, browser, device, or external service.
`test-l2` is therefore not required by this feature.

- Build fixtures with `git2`, which remains a `sniff` dev-dependency. Compare
  its in-memory conflict-index paths for clean, content, add/add, and
  modify/delete cases where the two engines have equivalent options.
- Keep explicit expected path sets for rename/rename, directory/file,
  multi-path, direction-sensitive, and multiple-merge-base fixtures. When the
  canonical `git` executable is available in the test environment, a
  non-interactive disposable-repository oracle also verifies those expected
  sets. The production library never invokes that executable.
- Exercise the temporary-index projection directly: collecting only one side's
  change locations must fail at least one rename/multi-path fixture.
- Configure a repository with an applicable external merge driver/filter and
  verify prediction returns the dedicated unsupported-configuration error
  before any command can launch. A safe built-in text merge remains supported.
- Vary or corrupt the live index, including staged `.gitattributes`, while
  holding the captured tips fixed. Prediction stays unchanged; only
  `ctx.merge_conflicts` observes that live index.
- Snapshot the on-disk object-ID set around clean auto-merges and criss-cross
  histories that synthesize virtual merge-base objects, in addition to the
  ordinary HEAD/ref/index/worktree snapshots.

## Acceptance Criteria

1. The base schema defines `ctx.branch` and `ctx.worktree` as optional generated
   strings and `ctx.merge_conflicts` as a required generated `string[]`; the
   schema-derived catalog exposes exactly those types and descriptions.
2. A dedicated demand-driven Git capture group owns all three keys. Referencing
   only one of them performs no monorepo structure scan, full status walk,
   document scan, hardware/OS probe, subprocess, or network request. One trusted
   repository discovery is shared across the three probes, and a failure in one
   field preserves successful sibling values.
3. `ctx.branch` returns the short attached local branch and returns null outside
   a repository, at unborn HEAD, and at detached HEAD.
4. `ctx.worktree` returns the linked-worktree directory basename (canonicalized
   first when possible) and returns null in the main worktree, bare
   repositories, and non-repositories; it does not substitute the branch name.
5. `ctx.merge_conflicts` returns sorted, deduplicated, portable repo-relative
   paths for non-zero index stages from merge/rebase/cherry-pick/revert state;
   clean and absent repositories return `[]`, which is falsy under
   `is_truthy`.
6. `sniff` exposes one read-only branch-merge conflict-path API. Existing
   worktree conflict booleans derive from the same commit-pair path helper
   rather than a parallel merge implementation.
7. The shared probe enables object-memory storage before merging, derives its
   temporary index/attributes from the captured `ours` tree, leaves
   `fail_on_conflict` disabled, and obtains result paths from non-zero temporary
   index stages. It never persists synthesized blobs, trees, or virtual bases.
8. Predicted conflict paths match a `git2` index oracle for clean, content,
   add/add, and modify/delete cases. Fixed and, when available, canonical-Git
   oracle fixtures cover rename/rename, directory/file, multi-path, and
   multiple-merge-base cases under the shared rename-aware options.
9. `predict_conflicts(branch)` merges the named local branch tip into the caller's
   current local branch tip in memory and returns the same sorted portable paths
   as the shared `sniff` API. Reversing ours/theirs is covered by a fixture where
   direction affects the reported paths.
10. Same-branch, ancestor, already-contained, and genuinely clean divergent
   merges return `[]`. Unknown branch, non-repository, detached/unborn HEAD,
   invalid ref syntax, unrelated history, missing/corrupt object, unsafe
   external merge configuration, and Git access failures return expression
   errors rather than `[]`.
11. Prediction ignores staged, unstaged, untracked, and already-conflicted index
    state, including staged `.gitattributes`; a missing or corrupt live index
    does not block it. Tests vary those states while keeping branch tips fixed
    and verify the prediction remains unchanged.
12. The function resolves the repository from the caller/launch-area anchor,
    not the Markdown document directory, and behaves consistently in body,
    frontmatter, and `$()` ternary expression surfaces.
13. Evaluating `predict_conflicts` leaves HEAD, every ref, the index, worktree
    files/status, and on-disk object-ID set unchanged, including fixtures that
    synthesize merged objects. It invokes no subprocess, hook, driver, filter,
    credential helper, fetch, or network operation.
14. The function catalog contains one `Git` entry at order 90 with
    `string -> string[] | error` metadata; runtime/catalog parity, alias
    uniqueness, completion, hover, and display-only example validation pass.
15. The generated context-variable documentation, expression docs, `sniff` API
    docs, and affected skills describe the shipped contracts without a second
    hand-maintained catalog.
16. Relevant L1 suites pass from both package areas (`just test` in Darkmatter
    and `sniff`). No L2 test is added without a real-terminal requirement.
    Cross-platform compile checks pass for macOS, Windows, and Linux.

## Specification Decisions

- **D1 — Existing versus predicted conflicts.** `ctx.merge_conflicts` observes
  unresolved index stages; `predict_conflicts()` predicts a branch-tip merge.
  Conflating them would make dirty state unpredictably affect a supposedly
  branch-based function.
- **D2 — Merge direction.** The named branch is incoming/theirs and the
  caller's current branch is ours, matching “this branch were merged into the
  caller's branch.”
- **D3 — Repository anchor.** Both context and function use the launch-area
  caller repository, not the document repository.
- **D4 — Failure meaning.** Empty arrays mean “successfully checked and no
  conflicts.” Missing prerequisites and Git failures remain errors for the
  function; optional context capture degrades to null/empty with a diagnostic.
- **D5 — Committed-state boundary.** Prediction uses captured local branch tips,
  committed attributes, and safe merge settings without fetching or reading
  the live index/worktree.
- **D6 — Shared ownership.** `sniff` owns branch/worktree/conflict Git logic;
  Darkmatter owns capture, expression projection, and authored catalogs.
- **D7 — Temporary-index path authority.** Predicted paths are the non-zero
  stages of an in-memory merge index, not a heuristic selection from conflict
  change records. This aligns predicted and actual conflict semantics.
- **D8 — Hermetic execution.** Object-memory storage is mandatory and external
  drivers/filters are rejected when applicable. Executing them violates the
  read-side safety contract; silently ignoring them gives a false prediction.
- **D9 — Partial capture isolation.** Git context fields share discovery but
  degrade independently after discovery, preserving every successful value.
