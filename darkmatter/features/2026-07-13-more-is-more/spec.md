---
status: ready for planning
reviewed: true
review_iterations: 23
rulings: index fallback/order plus initial Git and remote-query surfaces supplied by Ken 2026-07-13/14; branch existence is live and authoritative with zero/branch/branch-plus-remote overloads; ambiguous remote vendors are actively probed; expression catalogs gain lowercase snake_case enum returns including the explicit no-remote empty value; object string values stay quoted and bare identifiers stay variables; exact PR/CI lookup accepts scoped/native IDs and canonical URLs without shorthand; list defaults are 20/max 100, open PRs, all CI job statuses, newest first, and positive counts; PR/CI lists expose one repository-scoped canonical provider-neutral query model with explicit unsupported-filter errors; CI/CD records are jobs; exact/list output uses deterministic compact Markdown links; all expression functions have frontmatter/body parity
inputs:
  - ./git-pr.md
  - ./ci-cd.md
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
  - ../../lib/src/markdown/compose/expression/ast.rs
  - ../../lib/src/markdown/compose/expression/lexer.rs
  - ../../lib/src/markdown/compose/expression/parser.rs
  - ../../lib/src/markdown/compose/expression/resolve_ctx.rs
  - ../../lib/src/markdown/compose/context/options.rs
  - ../../../sniff/lib/src/filesystem/git/api.rs
  - ../../../sniff/lib/src/filesystem/git/status.rs
  - ../../../sniff/lib/src/filesystem/git/worktree.rs
  - ../../../sniff/lib/src/filesystem/git/remote_refresh.rs
  - ../../../sniff/lib/src/remote/mod.rs
  - ../../../sniff/lib/src/remote/provider.rs
  - ../../../sniff/lib/src/remote/types.rs
  - ../../../sniff/lib/src/remote/url_parser.rs
related:
  - ../_completed/2026-06-15-context-vars-additions
  - ../_completed/2026-07-10-function-schemas
  - ../_completed/2026-07-08-single-sourcing-schema
  - ../2026-07-12-literal-expression
---

# More Is More: Filesystem, Git, and Remote Repository Intelligence

**Status:** Reviewed and ready for planning. This feature adds two
filesystem-index expression functions, three Git-aware runtime context
variables, and seven Git/remote expression functions. Local filesystem and Git
state remain portable and side-effect-free. Remote branch, provider, pull-request,
and CI/CD intelligence is supplied through structured `sniff` APIs, while
Darkmatter owns expression validation and human-readable Markdown projection.

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

Add these remote-repository expression functions:

```text
branch_exists_on_remote() -> boolean | error
branch_exists_on_remote(branch: string) -> boolean | error
branch_exists_on_remote(branch: string, remote: string) -> boolean | error
remote_vendor([remote: string]) -> enum("", github, gitlab, gitea, forgejo, bitbucket, azure_devops, aws_code_commit, source_hut) | error
pr(id: number | string) -> string | error
pr_list(query: object | count: number(integer)) -> string[] | error
cicd(id: number | string) -> string | error
cicd_list(query: object | count: number(integer)) -> string[] | error
```

The exact and list functions use the caller repository's preferred configured
remote unless their identifier/query explicitly selects another configured
remote or contains a canonical provider URL. Sniff returns typed records and
typed failures; Darkmatter converts successful records into stable,
Markdown-friendly strings.

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

Remote-aware prompts also need to answer whether a branch has been published,
which forge owns the configured remote, what a particular pull request or
CI/CD job is doing, and which recent items match a focused query. Sniff
already has provider detection, authentication, normalized pull-request data,
and a narrow GitHub workflow-run listing, but the supplemental assessments show
that it lacks exact lookup, canonical structured queries, reliable pagination,
stable result identity, and cross-provider CI/CD job models. This feature
adds those foundations once in Sniff and keeps Darkmatter's layer deliberately
presentation-oriented.

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

#### Remote URLs and expression surfaces

- HTTP(S) URLs are **rejected** with an error, exactly as `resolve_path_shape`
  already does for the path family. A directory scan has no remote analogue.
- These functions touch only the local filesystem and require **no remote
  runtime**, but they follow the same function-availability invariant as every
  other expression function: they are callable in body interpolation,
  frontmatter interpolation, and frontmatter `$()` function/ternary evaluation.

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

### Resolved Index Decisions

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

## Remote Repository Intelligence

### Scope and terminology

This section adds six expression functions over the caller repository's
configured remotes. In this contract:

- **pull request** includes GitLab merge requests and the equivalent
  repository-review object on every supported forge;
- **CI/CD job** means one provider-addressable unit of work within a workflow
  run or pipeline: a GitHub/Gitea/Forgejo Actions job, a GitLab pipeline job, or
  a Bitbucket pipeline step;
- **parent execution** means the workflow run or pipeline which contains a
  CI/CD job;
- **preferred remote** is the single configured remote selected by the shared
  Sniff rule below;
- **provider URL** means a canonical web or API URL that carries enough host,
  repository, and item identity to resolve without the caller repository.

The initial provider-query implementation covers GitHub, GitLab (including
self-managed), Gitea/Forgejo where the server version exposes the required API,
and Bitbucket Cloud. Bitbucket Data Center is a distinct API flavor and must not
be sent to the Bitbucket Cloud adapter. Other hosting vendors may be identified
by `remote_vendor()` even when they do not yet support PR or CI/CD queries.

### Caller repository and preferred-remote authority

Every repository-anchored operation resolves from
`ResolutionContext::caller_dir()`, never from the Markdown document directory.
A prompt stored in repository A but invoked from repository B therefore uses
repository B's remotes, matching the existing Git context and
`predict_conflicts()` contract.

Sniff gains one public remote-resolution model, conceptually:

```text
ResolvedRemote {
    name,
    fetch_url,
    push_url,
    hosting_provider,
    api_flavor,
    host,
    namespace,
    repository,
}
```

and one shared selector:

```text
resolve_remote_at(path, requested_remote) -> Result<Option<ResolvedRemote>>
```

Selection is deterministic:

1. An explicitly requested non-empty remote name is an exact, case-sensitive
   config-name lookup. A missing explicit remote is an error.
2. Without an explicit name, `origin` wins when it has a usable URL.
3. Otherwise the first non-`upstream` remote with a usable URL wins in
   lexicographic name order.
4. `upstream` is the last resort when it is the only usable remote.
5. A repository with no usable configured remote returns `Ok(None)`.

This replaces the current drift between `preferred_remote_url()` (origin then
alphabetically first) and the `GitRepo` projection (which deliberately places
`upstream` last). All existing preferred-remote callers migrate to the shared
selector; no third rule is added in Darkmatter.

### Sniff remote API foundation

Sniff remains the sole authority for provider detection, authentication,
provider capabilities, URL/reference parsing, pagination, and response
normalization. Darkmatter does not call Schematic clients, build provider URLs,
or classify HTTP status codes.

The remote layer gains separate exact and collection operations:

```text
get_pull_request(reference) -> Result<Option<PullRequestInfo>>
query_pull_requests(query) -> Result<PullRequestPage>

get_cicd_job(reference) -> Result<Option<CiCdJob>>
query_cicd_jobs(query) -> Result<CiCdJobPage>
```

Exact references preserve provider, host, repository coordinates, native ID,
and the original URL when present. A genuine provider `404` maps to `Ok(None)`;
malformed identity, missing credentials, invalid credentials, forbidden access,
rate limiting, transport failure, unsupported server capability, and provider
unavailability remain distinguishable `SniffError` variants.

Collection results are page objects, not bare vectors:

```text
PullRequestPage / CiCdJobPage {
    items,
    next,
    total,
    warnings,
}
```

Every item retains its provider, API flavor, host, repository coordinates,
native identifier, display identifier, canonical web URL, API URL when known,
raw provider state, and normalized state. Unsupported structured filters are
errors by default; an adapter must never silently ignore them. Warnings are
reserved for a documented, result-preserving degradation such as unavailable
total-count metadata.

The existing `PullRequestInfo` is widened rather than replaced when that can be
done without ambiguous fields. CI/CD configuration detection, parent execution
identity, and job records are split into distinct types:

```text
CiCdConfiguration
CiCdParentExecution
CiCdJob
CiCdJobReference
CiCdJobQuery
CiCdJobPage
CiCdCapabilities
```

`CiCdInfo` no longer serves as both “configuration file detected” and “real
job.” Compatibility projections may remain for the aggregate remote report,
but focused query APIs never return configuration observations or bare parent
executions as if they were jobs. Each job retains its parent execution
identity and URL when the provider exposes them.

Provider capabilities are explicit and include exact lookup, support for each
canonical query field, direct repository-wide job listing, bounded
parent-execution traversal, pagination, logs, artifacts, and test reports.
Gitea and Forgejo capability decisions use detected server family and version
rather than assuming current public Gitea behavior. GitHub Enterprise, GitLab
self-managed, and Bitbucket Cloud/Data Center are represented as API flavors so
an unknown enterprise host is not silently treated as Gitea.

### Network, credentials, and runtime boundary

`remote_vendor()` resolves unambiguous providers from local configuration, then
uses a focused network probe when a self-hosted URL is ambiguous. The other
five remote functions are network operations. All six are fallible when they
need provider or transport access.

Darkmatter enables Sniff's `remote` feature and introduces one run-local
remote-query runtime, separate from file-content fetching but governed by the
same consent policy. Both network services are installed before frontmatter
interpolation and are available to every expression evaluation surface:

- deny all provider hosts by default;
- `md compose --allow-host <host>` authorizes that exact provider host;
- a provider URL argument must pass the same host policy;
- no redirect may escape the authorized host policy;
- provider credentials remain owned and resolved by Sniff/Schematic;
- focused errors are never replaced with empty strings or arrays;
- repeated identical normalized requests within one compose run are
  single-flight and memoized;
- concurrency uses the existing remote-concurrency cap;
- no persistent authenticated-response cache is introduced in v1.

The expression evaluator is synchronous today while Sniff's provider trait is
asynchronous. The runtime owns its own Tokio executor, following the established
`RemoteFetchRuntime` pattern, so it does not call `block_on` on a caller's active
runtime. Darkmatter passes policy and request data into Sniff; it does not expose
provider clients through `ResolutionContext`.

Function availability does not vary by document region. The same binding and
runtime behavior applies in both frontmatter interpolation passes, frontmatter
`$()` safe-function/ternary evaluation, body interpolation, and any other
compose expression surface which supports function calls. For example, both of
these are valid when the provider host is allowed:

```markdown
---
pr_status: "{{ pr(123) }}"
---

PR status: {{ pr(123) }}
```

Host denial, missing credentials, or provider failure produces the same focused
error regardless of the expression surface. Request memoization is run-wide, so
the two identical calls above share one provider request.

This feature also removes the existing runtime-wiring exception for remote URL
arguments to read-side file functions: when authorized, those functions work
in frontmatter exactly as they do in body interpolation. A function may still
reject an argument that is outside its semantic domain—for example, the index
family rejects HTTP URLs because a remote directory scan is undefined—but it
must not reject an otherwise valid call merely because it appears in
frontmatter. `remote_vendor()` follows the same host-consent rule whenever its
local classification requires a network probe. DMLS remains passive and never
evaluates functions or performs network discovery.

### Query-object expression syntax

The requested `pr_list({ ... })` and `cicd_list({ ... })` forms require a real
expression-language addition: the current AST can consume object values from
frontmatter/context but cannot author object or array literals.

Add JSON-like literals to the one authoritative lexer/parser/AST:

```text
object_literal = "{" (object_entry ("," object_entry)*)? "}"
object_entry   = (IDENT | STRING) ":" expression
array_literal  = "[" (expression ("," expression)*)? "]"
```

Examples:

```text
pr_list({ state: "open", limit: 5 })
cicd_list({ statuses: ["failed", "cancelled"], branch: "main", limit: 10 })
```

Bare object keys are ergonomic aliases for string keys. String values remain
quoted; `open` without quotes is still a variable reference, not an enum token.
This rule is global and does not change based on the called function, object
key, expected query type, or catalog enum declarations.
Duplicate object keys, missing commas/colons, non-string keys, and non-finite
numbers are parse errors. Trailing commas, spread syntax, computed keys, and
object mutation are not supported in v1. Literal values evaluate recursively to
`serde_json::Value`, so a query object may mix literals with context variables.
Passing an existing object-valued variable remains valid and follows the same
validation path.

This grammar addition is general-purpose, but its initial acceptance fixtures
are the two query functions plus the existing display-only
`validate_schema("fixture.md", {})` example, which becomes executable once
empty-object evaluation exists.

### `branch_exists_on_remote([branch[, remote]])`

```text
branch_exists_on_remote() -> boolean | error
branch_exists_on_remote(branch: string) -> boolean | error
branch_exists_on_remote(branch: string, remote: string) -> boolean | error
```

- Zero arguments selects the caller's current attached local branch and the
  preferred remote.
- One argument selects that branch and the preferred remote.
- Two arguments select that branch and the exact configured remote name.
- An empty branch string selects the caller's current attached local branch,
  including `branch_exists_on_remote("", "upstream")`. Detached/unborn HEAD is
  an error when the current-branch default is needed.
- A non-empty argument names a branch exactly. One leading `refs/heads/` is
  accepted and removed; tags, SHAs, patterns, and remote-tracking spellings are
  rejected.
- A supplied remote must be non-empty and is an exact, case-sensitive
  configured-remote lookup. A missing or URL-less supplied remote is an error.
- When the remote argument is omitted, the preferred remote is used. With no
  configured remote, the result is `false`.
- Sniff performs a live, read-only remote-ref observation. It does not infer
  existence from stale `refs/remotes/*`, does not fetch, and does not mutate
  local refs or configuration.
- Sniff may use a provider branch endpoint when that API flavor supports an
  authoritative lookup, or Git transport ref advertisement otherwise. The
  transport path requires Sniff's explicit network/credential feature set but
  must not negotiate or download a pack.
- An authoritative absence is `false`; presence is `true`; authentication,
  authorization, rate-limit, transport, protocol, and provider failures are
  errors rather than `false`.

The Sniff API is conceptually:

```text
branch_exists_on_remote_at(path, branch, requested_remote) -> Result<bool>
```

The zero-, one-, and two-argument forms all call this one Sniff operation;
Darkmatter does not implement a second remote-selection rule.

### `remote_vendor([remote])`

```text
remote_vendor([remote: string])
    -> enum("", github, gitlab, gitea, forgejo, bitbucket, azure_devops,
            aws_code_commit, source_hut) | error
```

- With no argument or an empty string, resolve the preferred remote.
- With a non-empty argument, perform an exact configured-remote lookup.
- No configured remote yields the empty string only in the implicit/default
  form, matching the requested neutral result.
- A named remote that does not exist or has no usable URL is an error.
- Sniff first applies deterministic URL/config classification. Canonical and
  otherwise unambiguous hosts return immediately without network access.
- An ambiguous self-hosted URL triggers a bounded, read-only Sniff discovery
  probe against the remote's exact host. The host must be authorized by the
  shared allowlist before any request or credential lookup occurs.
- The probe distinguishes supported enterprise/server API flavors, including
  GitHub Enterprise, GitLab self-managed, Gitea, Forgejo, Bitbucket Data Center,
  and Azure DevOps Server, without defaulting an unknown host to Gitea.
- Host denial, unreachable service, required-but-unavailable authentication,
  conflicting signatures, and an unidentified forge are focused errors.
  `Self-Hosted` and `Unknown` may remain Sniff-internal detection states but are
  not successful Darkmatter results because this function promises a vendor.
- Darkmatter returns a stable lowercase snake_case token for a concrete vendor:
  `github`, `gitlab`, `bitbucket`, `azure_devops`, `aws_code_commit`, `gitea`,
  `forgejo`, `source_hut`, and future concrete enum variants. Human-facing
  descriptions retain each vendor's official brand capitalization.

Sniff retains the typed `GitHostingProvider`; Darkmatter projects it as one of
the catalog's closed enum strings. A newly supported Sniff provider must add its
Darkmatter return variant, formatter mapping, docs, and tests in the same
change; it may not leak an undeclared string through this function.

### Enum return types in the expression catalog

The authored function catalog gains first-class return-only enums using the
existing schema-style spelling:

```yaml
returns:
    type: enum("", github, gitlab, gitea, forgejo, bitbucket, azure_devops, aws_code_commit, source_hut)
    fallible: true
```

Parameter types remain the current payload-free `DataType` vocabulary. Return
metadata separates ordinary data types from closed enums conceptually:

```text
ReturnValueType = Data(DataType) | Enum(&'static [&'static str])
ReturnType { value: ReturnValueType, array, fallible }
```

The catalog parser reuses SimplifiedSchema's enum-value grammar and rejects an
enum with no members, duplicate variants, and enum syntax in parameter
position. A quoted empty-string member is valid and preserved; `remote_vendor`
uses it for the explicitly specified no-remote result. `array: true` is valid
and means every returned string belongs to the declared enum. `fallible: true`
remains orthogonal and renders `| error`.

Enum values evaluate as ordinary JSON strings, so comparisons, interpolation,
serialization, and existing expression operators need no new runtime value
kind. The public projected descriptor retains the variants with stable static
lifetimes. `typed_signature()`, generated documentation, `md schema about`, and
DMLS completion/hover render the closed variants instead of degrading them to
`string`. Executable catalog examples and focused handler tests verify that
successful results are declared variants.

### `pr(id)`

```text
pr(id: number(integer) | string) -> string | error
```

- A positive integer is a repository-scoped PR number on the preferred remote.
- A digit-only string has the same meaning, preserving values sourced from
  frontmatter or environment state.
- A canonical provider PR/MR URL supplies its own host, repository coordinates,
  and repository-scoped number. It may target a repository other than the
  caller's configured remote, subject to host policy.
- Other strings are malformed-reference errors in v1; provider-global IDs and
  ambiguous shorthand such as `owner/repo#12` are not guessed.
- Sniff returns `Option<PullRequestInfo>`; Darkmatter converts `None` into a
  not-found expression error naming the requested identity.
- Provider unreachability, authentication/authorization failure, rate limiting,
  unsupported API flavor, and malformed references remain actionable errors.

### `pr_list(query | count)`

```text
pr_list(query: object) -> string[] | error
pr_list(count: number(integer)) -> string[] | error
```

The integer overload is shorthand for the latest `count` open pull requests on
the preferred remote. `count` must be positive and is capped by the same global
result limit as an object query.

The structured object accepts this canonical v1 vocabulary:

| Key | Type | Meaning |
|-----|------|---------|
| `remote` | string | Exact configured remote name; preferred remote when absent |
| `state` | string or string[] | Any of `open`, `closed`, `merged`; default `open` |
| `draft` | boolean | Independently select draft/non-draft state |
| `source_branch` | string | Exact source branch |
| `target_branch` | string | Exact destination branch |
| `author` | string | Provider login/username |
| `assignee` | string | Provider login/username |
| `reviewer` | string | Provider login/username |
| `labels` | string[] | Require all listed labels |
| `milestone` | string | Provider milestone title/identifier |
| `search` | string | Portable title/body search term |
| `commit` | string | PRs associated with a commit SHA |
| `created_after` / `created_before` | datetime | Inclusive creation window |
| `updated_after` / `updated_before` | datetime | Inclusive update window |
| `sort` | string | `created`, `updated`, or `provider-default` |
| `direction` | string | `ascending` or `descending` |
| `limit` | number(integer) | Maximum returned items |

Unknown keys, wrong types, invalid enum values, inverted time ranges, a
non-positive limit, or invalid filter combinations are invalid-query errors
before network access. Sniff reports an unsupported-filter error naming the
field and provider/server flavor when the selected provider cannot honor a
valid canonical filter exactly. Exact adapter-side emulation is allowed only
when Sniff can traverse a complete bounded result domain; approximating a
filter or applying it to only the first provider page is forbidden. Sniff
follows provider pagination until `limit` matches are collected or the provider
exhausts the result set.

The default object limit is 20 and the v1 hard maximum is 100. A successful
query with no matches returns `[]`.

### `cicd(id)`

```text
cicd(id: number(integer) | string) -> string | error
```

This is the CI/CD-job analogue of `pr()`:

- a positive integer resolves in the preferred remote's repository when that
  provider has an integer job identity;
- a string may be a provider-native ID (including a Bitbucket UUID) or a
  canonical job/step URL;
- a genuine not-found result becomes an expression error;
- provider/network/authentication/capability failures remain errors.

The result is one job. Its structured Sniff record includes parent workflow
run/pipeline identity, job name, stage when applicable, normalized and native
status/conclusion, branch/ref, commit, timestamps, canonical URL, and available
runner metadata. `cicd()` does not implicitly retrieve logs, artifacts, test
reports, steps, or sibling jobs.

### `cicd_list(query | count)`

```text
cicd_list(query: object) -> string[] | error
cicd_list(count: number(integer)) -> string[] | error
```

The integer overload returns the latest `count` jobs, regardless of status,
from the preferred remote repository.

The structured object accepts:

| Key | Type | Meaning |
|-----|------|---------|
| `remote` | string | Exact configured remote name; preferred remote when absent |
| `statuses` | string or string[] | Normalized lifecycle states/conclusions |
| `name` | string | Exact or provider-supported job-name match |
| `stage` | string | Pipeline stage when the provider exposes stages |
| `workflow` | string | Parent workflow/pipeline name, definition ID, or path |
| `parent` | number(integer) or string | Exact parent workflow-run/pipeline identity |
| `branch` | string | Exact branch/ref |
| `commit` | string | Exact commit SHA |
| `actor` | string | Triggering provider login/username |
| `trigger` | string | Push, PR/MR, schedule, manual, parent, or provider event |
| `created_after` / `created_before` | datetime | Inclusive creation window |
| `updated_after` / `updated_before` | datetime | Inclusive update window |
| `direction` | string | `ascending` or `descending` |
| `limit` | number(integer) | Maximum returned jobs |

Validation, unsupported-filter handling, pagination, default limit 20, hard
maximum 100, and canonical capability rules match `pr_list()`. A successful
query with no matches returns `[]`. When a provider has no direct
repository-wide job endpoint, Sniff walks parent executions newest-first and
enumerates their jobs until the requested job limit is satisfied or the
provider is exhausted. Both parent pages inspected and jobs inspected are
bounded so a broad query cannot create unbounded fan-out.

### Darkmatter string projection

Sniff returns structured records. Darkmatter owns exactly two pure formatters,
one for PRs and one for CI/CD jobs. The exact function and each list
element use the same formatter, preventing output drift.

Canonical shapes are:

```text
[PR #123 — Fix parser](https://provider/pr/123) · open · @alice · feature/parser → main
[CI job #456 — test](https://provider/job/456) · failed · push · main @ abcdef1
```

Rules:

- output is Markdown-friendly text with no ANSI or terminal markup;
- provider titles/names are whitespace-collapsed and Markdown-escaped;
- the canonical web link is included when available;
- missing optional segments are omitted with no placeholder noise;
- normalized state is always included, with provider raw state retained only
  in Sniff's structured record;
- commit SHAs render at seven hexadecimal characters when available;
- exact and list output is deterministic and does not include volatile fields
  such as “updated N minutes ago.”

### Catalog registration and order

The authored expression-function catalog remains the signature and
documentation authority. Append without renumbering existing functions:

| Order | Category | Function |
|-------|----------|----------|
| 88 | Filesystem | `find_first_index(file)` |
| 89 | Filesystem | `find_last_index(file)` |
| 90 | Git | `predict_conflicts(branch)` |
| 91 | Git | `branch_exists_on_remote([branch[, remote]])` |
| 92 | Git | `remote_vendor([remote])` |
| 93 | Pull Requests | `pr(id)` |
| 94 | Pull Requests | `pr_list(query)` / `pr_list(count)` |
| 95 | CI/CD | `cicd(id)` |
| 96 | CI/CD | `cicd_list(query)` / `cicd_list(count)` |

Runtime handlers live in focused `functions/git.rs`, `functions/pull_requests.rs`,
and `functions/cicd.rs` slices. Bindings contain only canonical names, aliases,
evaluation mode, and handler pointers; all descriptive metadata stays in YAML.
Remote examples are `display-only` because they require provider fixtures and
credentials. Wiremock-backed Sniff tests provide executable behavioral proof.

### Context-variable boundary

This draft adds no further `ctx.*` properties beyond `ctx.branch`,
`ctx.worktree`, and `ctx.merge_conflicts`. Remote provider queries are
demand-driven, fallible, credential-sensitive network operations and therefore
must not run during eager context capture. `remote_vendor()` remains a function
rather than a generated context value so callers can select an alternate
configured remote.

### Error projection

Darkmatter maps structured Sniff errors without erasing their category:

- malformed reference/query → identify the invalid argument/key;
- no repository/default branch/remote → identify the missing prerequisite;
- not found → name the PR/job and repository;
- provider identification failed → name the host and distinguish host denial,
  unreachable/authentication failure, conflicting signatures, and an
  unsupported or unidentified forge;
- missing or invalid credentials → name the provider and expected credential
  source without printing secret values;
- forbidden → distinguish authorization from not-found;
- rate limited → include reset/retry metadata when Sniff provides it;
- unsupported capability/server version → name the provider, flavor/version,
  and unsupported operation/filter;
- unreachable/transport/provider failure → name the host and operation.

Empty strings and arrays are successful neutral values only where this spec
explicitly assigns them. They never stand in for focused provider failures.

## DMLS Integration

No Git or remote-provider operation runs in DMLS. The existing schema/catalog
integrations supply editor intelligence passively:

- the schema-derived `ctx.*` catalog adds completion and hover for
  `ctx.branch`, `ctx.worktree`, and `ctx.merge_conflicts`, including their exact
  nullable/array types;
- the expression-function catalog adds completion, signature details, and
  documentation for all nine functions in this feature;
- the shared expression parser accepts and ranges object/array literals, so
  malformed query syntax receives ordinary expression diagnostics;
- v1 does not add query-object key/value completion; the function hover links
  to the authored query vocabulary instead of duplicating it in DMLS;
- DMLS never evaluates a function, discovers a repository, reads the index,
  resolves credentials, or contacts a provider while serving editor requests.

Add parity tests proving the three context descriptors and all function
descriptors reach DMLS through the shared catalogs. Do not add hard-coded DMLS
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
- Remote-provider operations are bounded by result limits, pagination guards,
  request timeouts, the run-wide concurrency cap, and run-local memoization.
- Provider clients may read credentials through Sniff's established environment
  integration but never log, serialize, interpolate, or cache secret values.
- No command execution, hook, provider mutation, implicit credential prompt, or
  network request outside an explicitly allowed host is introduced.

## Documentation and Single-Sourcing

Update alongside implementation:

- `darkmatter/docs/schemas/darkmatter.yaml` — the three context descriptors;
- `darkmatter/docs/topics/context-variables.md` — capture-group table and
  regenerated catalog block;
- `darkmatter/docs/schemas/expression-functions.yaml` — the function descriptor;
- `darkmatter/docs/topics/darkmatter-expressions.md` — Git function semantics,
  query-object syntax, remote function semantics, projections, and examples;
- `sniff` public API docs and its Git/remote documentation — actual versus
  predicted conflict APIs, preferred-remote selection, provider capabilities,
  exact references, query types, pagination, authentication, and errors;
- Schematic provider definitions for exact PR and CI/CD endpoints or missing
  list filters, tested through `--manifest-path schematic/schema/Cargo.toml`;
- `docs/dependencies.md` and affected area dependency docs for enabling Sniff's
  `remote` feature and any added generated-client surface;
- the `sniff`, `rust-devops`, and `darkmatter` skills when their public
  architecture/workflow descriptions need the new surface.

Darkmatter already depends on `sniff`, but it currently uses Sniff's default
feature set; enabling the `remote` feature is a meaningful dependency-surface
change and must be documented. If conflict planning needs a new direct
plumbing-crate dependency because the `gix` facade cannot enforce the hermetic
boundary, document that in the same pass.

## Compatibility

- The context variables and expression functions are additive names.
- Existing context capture remains demand-driven and produces byte-identical
  values for existing keys.
- Existing expression-function names, aliases, order values, and behavior do
  not change. New functions occupy orders 88 through 96 without renumbering.
- Object/array literals are additive grammar: `{` was previously invalid in an
  expression primary and `[` was previously valid only as postfix indexing.
- Existing Sniff convenience list/report methods may remain as compatibility
  projections, but focused APIs use page results and preserve errors.
- The aggregate remote report retains graceful degradation; the new focused
  query APIs do not inherit its error swallowing.
- Correcting Bitbucket PR state/draft handling and Gitea/Forgejo CI detection is
  an intended correctness change.
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
- `predict_conflicts()` performs no remote fetch, refresh, or implicit
  `origin/<branch>` lookup.
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
- No PR merge/close/approve/comment/review mutation.
- No CI/CD dispatch, cancel, retry, approval, deployment, or other mutation.
- No implicit retrieval of CI/CD logs, artifacts, test reports, step detail, or
  sibling jobs outside bounded list-query traversal.
- No organization/group/account-wide query scope in the first Darkmatter
  surface; list functions are repository-scoped through a configured remote.
- No raw provider-native query strings or provider-specific query objects in
  the Darkmatter surface; Sniff adapters translate the canonical model.
- No Bitbucket Data Center PR/CI query adapter in v1; vendor detection remains
  supported.
- No eager remote-derived `ctx.*` variables.

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
- Add parser/evaluator tests for nested object/array literals, mixed computed
  values, duplicate/invalid keys, existing indexing behavior, and the now-
  executable empty-object catalog example.
- Use Wiremock-backed provider fixtures for exact found/not-found, pagination,
  every canonical query field supported by each adapter, unsupported filters,
  authentication, authorization, rate limiting, malformed responses, and
  transport failure. No test calls a live provider.
- Cover local vendor classification and ambiguous-host probing separately,
  including each supported enterprise/server flavor, conflicting signatures,
  unidentified servers, allowlist denial before I/O, and authenticated probes
  without credential disclosure.
- Assert the provider URL parser handles canonical PR and CI/CD URLs without
  mistaking GitLab nested paths for repository coordinates.
- Assert preferred-remote selection once in Sniff and reuse that fixture from
  every Darkmatter function; include origin, fork/upstream, URL-less remotes,
  explicit missing names, and no-remote repositories.
- Assert host denial occurs before provider client construction/network access;
  authorized frontmatter and body calls both succeed; identical requests are
  single-flight across surfaces; and existing remote URL read functions gain
  the same authorized frontmatter behavior.
- Assert exact and list projections use the same pure formatter, escape
  Markdown, collapse hostile/multiline provider text, and remain deterministic.

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
17. `find_first_index()` and `find_last_index()` satisfy the merged index-family
    contract, executable catalog examples, isolation/fallback cases, and orders
    88/89 without introducing a second index grammar.
18. The expression parser/evaluator supports immutable JSON-like object and
    array literals with spans, computed values, deterministic duplicate-key
    rejection, and no regression to postfix indexing or existing expressions.
19. Sniff owns one preferred-remote resolver with the specified origin/fork/
    upstream order; all local and provider-query surfaces reuse it.
20. All three `branch_exists_on_remote()` overloads perform a live read-only
    observation with the specified current/explicit branch and
    preferred/explicit remote selection: true and false mean authoritative
    presence/absence, while remote failures remain errors and local
    refs/configuration remain unchanged.
21. `remote_vendor()` returns the declared lowercase snake_case provider token,
    returns `""` only for an implicit no-remote lookup, errors for a missing
    explicit remote, resolves unambiguous hosts locally, and performs an
    allowlisted bounded probe for ambiguous self-hosted URLs. Indeterminate or
    failed probes remain focused errors rather than `Unknown` strings.
22. Sniff exposes exact PR references/lookups and structured/native paginated
    queries with provider/repository identity, explicit capabilities, complete
    traversal to the requested limit, and no silently ignored filter.
23. `pr()` errors on authoritative not-found and focused provider failures;
    `pr_list()` returns deterministic formatted strings or `[]` for a successful
    no-match query. Integer and object overloads validate as specified.
24. Sniff separates CI/CD configuration, parent executions, and jobs, and
    exposes exact job lookup plus capability-aware paginated job queries for
    every supported provider/API flavor, preserving native/display identity,
    parent identity, and raw/normalized state.
25. `cicd()` and `cicd_list()` mirror the PR error, overload, pagination, and
    projection contracts for jobs without implicitly fetching logs, artifacts,
    test reports, step detail, or unrelated sibling jobs.
26. Every expression function has frontmatter/body availability parity.
    Network-capable calls are deny-by-default, exact-host allowlisted, bounded,
    run-wide single-flight, and credential-safe on every evaluation surface;
    DMLS and context capture still perform no provider I/O.
27. Sniff errors preserve malformed, missing, unauthorized, rate-limited,
    unsupported, and unreachable states; Darkmatter converts them into
    actionable messages without collapsing them into neutral results.
28. Catalog orders 91–96, runtime binding parity, DMLS completion/hover,
    generated docs, aliases, overloads, and display-only examples agree with the
    authored catalog and introduce no hard-coded DMLS name list.
29. Sniff, Darkmatter, and any changed Schematic definitions pass focused and
    full L1 suites with Wiremock-only network tests; macOS, Windows, and Linux
    compile checks pass.
30. The authored expression catalog accepts closed `enum(...)` returns but not
    enum parameters, preserves variants in the public descriptor, renders them
    in typed signatures/docs/DMLS, supports orthogonal array/fallible shapes,
    and rejects malformed or duplicate variants. A quoted empty member is
    preserved, and `remote_vendor()` returns only that declared no-remote value
    or a declared concrete-vendor variant.

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
- **D10 — Focused versus aggregate errors.** Aggregate reports may degrade;
  exact/list expression calls preserve typed provider errors.
- **D11 — Remote ownership.** Sniff owns remote selection, provider identity,
  credentials, exact references, query validation/capabilities, pagination, and
  structured records. Darkmatter owns expression syntax, query projection, and
  human-readable output.
- **D12 — Canonical query values, not query strings.** Darkmatter exposes one
  typed provider-neutral query object. It is a useful superset rather than a
  lowest-common-denominator promise: Sniff translates supported fields and
  returns an explicit unsupported-filter error for fields a provider cannot
  honor exactly.
- **D13 — No eager remote context.** Credential-sensitive network results are
  demand-driven functions, not `ctx.*` capture.
- **D14 — One record, one formatter.** Exact and list functions share the same
  deterministic Markdown formatter for their domain.
- **D15 — CI/CD job boundary.** `cicd()` identifies one provider-addressable
  job/step. The parent workflow run or pipeline is metadata and a list filter;
  logs, artifacts, test reports, and step detail are separate future reads.
- **D16 — Live remote branch authority.** `branch_exists_on_remote()` performs
  a live remote observation. Cached remote-tracking refs are not authoritative;
  connectivity and provider failures remain errors rather than becoming
  `false`.
- **D17 — Expression-surface parity.** All expression functions are available
  in frontmatter and Markdown content. Runtime capabilities and host consent
  are wired consistently before either surface is evaluated; document region
  is never a reason to reject an otherwise valid function call.
- **D18 — Branch/remote positional overloads.** Zero arguments means current
  branch plus preferred remote; one means explicit branch plus preferred
  remote; two means explicit branch plus exact configured remote. An empty
  branch selects the current branch so callers can target an alternate remote
  without repeating the branch name.
- **D19 — Concrete remote-vendor result.** `remote_vendor()` classifies locally
  when possible and probes an allowlisted ambiguous host when necessary. A
  non-empty successful result names a concrete vendor; the empty string is
  reserved for an implicit lookup with no remote. `Self-Hosted`/`Unknown`,
  failed probes, and indeterminate signatures are errors rather than successful
  strings.
- **D20 — Return-only enum catalog type.** Closed enums are first-class return
  metadata backed by ordinary string runtime values. They are not added to the
  parameter `DataType` domain; array and error-union return modifiers remain
  composable with them.
- **D21 — Machine-oriented enum values.** Enum runtime values use lowercase
  snake_case tokens (`github`, `azure_devops`, `aws_code_commit`, …). Official
  brand capitalization is presentation metadata, not part of the value. The
  quoted empty-string member is the sole neutral exception and represents only
  an implicit lookup with no configured remote.
- **D22 — Quoted string literals.** Object values use the ordinary expression
  grammar: strings are quoted and bare identifiers are variable references.
  There are no bare enum atoms or query-key-dependent parsing rules.
- **D23 — Exact reference forms.** `pr()` and `cicd()` accept the selected
  repository/provider's scoped or native ID and canonical provider URLs.
  Shorthand such as `owner/repo#123` and `remote#123` is rejected rather than
  guessed.
- **D24 — List defaults and bounds.** An omitted limit means 20 and the hard
  maximum is 100. PR queries default to open items; CI/CD job queries default
  to all statuses; both order newest-first. Explicit count overloads require a
  positive integer.
- **D25 — No provider-native query escape hatch.** PR and CI/CD list functions
  expose only the canonical query vocabulary. Adapter-native syntax stays
  internal to Sniff; unsupported canonical fields fail explicitly and are
  never ignored, approximated, or silently downgraded.
- **D26 — Repository-only query scope.** PR and CI/CD list queries are scoped
  to the repository identified by the selected configured remote. Organization,
  group, workspace, account, and all-visible scopes are outside v1.
- **D27 — Compact Markdown string projection.** Exact PR/job functions and
  their list elements use the same deterministic domain formatter: a canonical
  Markdown link followed by compact state and identity metadata, with missing
  optional segments omitted.
