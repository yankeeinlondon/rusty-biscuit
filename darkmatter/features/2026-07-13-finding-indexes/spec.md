---
status: ready for planning and implementation
reviewed: true
review_iterations: 0
rulings: Q2 (identity fallback) + Q3 (append at 88/89) ruled by Ken 2026-07-13
inputs:
    - ../../docs/schemas/expression-functions.yaml
    - ../../lib/src/markdown/compose/expression/functions/paths.rs
    - ../../lib/src/markdown/compose/expression/functions/mod.rs
    - ../../lib/src/markdown/compose/expression/catalog/mod.rs
related:
    - ../_completed/2026-07-10-function-schemas
    - ../2026-07-12-literal-expression
---

# Expression Functions: `find_first_index` and `find_last_index`

**Status:** Ready for planning and implementation. Two new read-side
Filesystem expression functions that resolve a file reference to the
lowest- or highest-indexed existing sibling in the same directory.

## Goal

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

## Motivation

- Authors compose documents that reference "the current review" or "the newest
  draft" of an indexed series (`review-1.md`, `review-2.md`, `review-3.md`).
  Today there is no way to ask "give me the newest one" without hard-coding the
  suffix. `increment_file_index` produces the _next_ name whether or not it
  exists; these two functions instead resolve to a name that **exists on disk**.
- The pair rounds out the index-family algebra: `file_index` reads the ordinal,
  `increment`/`decrement` walk it arithmetically, and `find_first_index` /
  `find_last_index` locate the endpoints of the real, on-disk series.

## Behavior

### Worked examples

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

### The index family of a file

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

### Ordering within a family

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

### Candidate set and the "no siblings" fallback

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

### Path resolution, display shape, and directory scanning

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

### Remote URLs and local-only contexts

- HTTP(S) URLs are **rejected** with an error, exactly as `resolve_path_shape`
  already does for the path family. A directory scan has no remote analogue.
- These functions touch only the local filesystem and require **no remote
  runtime**, so they are valid in **every** resolution context — body
  interpolation _and_ the local-only frontmatter passes (both interpolation
  passes and the `$()` ternary). They behave like `file_exists`'s local branch,
  not like `frontmatter`/`load_markdown` remote reads.

### Null and error handling

- Follows the family contract via `any_null`: a `null` argument returns
  `Value::Null` (not an error). The functions are marked `fallible` in the
  catalog because path resolution can fail (e.g. a rejected remote URL, or a
  `vault:` reference to a missing file), matching every other Filesystem
  function's `returns.fallible: true`.
- A non-string, non-null argument is an arity/type error via
  `require_string_expr`, consistent with `increment_file_index` et al.

## Catalog Changes (`darkmatter/docs/schemas/expression-functions.yaml`)

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

## Runtime Registration and Handlers

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

## Cross-Platform Considerations

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

## Testing

### Unit (in `functions/mod.rs` tests, using a `tempfile::TempDir` fixture)

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

### Catalog / parity (existing tests exercise these automatically)

- `descriptor_signature_set_equals_dispatchable_signature_set` and
  `every_descriptor_overload_is_dispatchable_at_its_declared_arity` — pass once
  both bindings and catalog entries land.
- `every_example_evaluates_to_its_declared_result` — the two executable
  examples evaluate against `make_fixture`'s `review-1.md`/`review-2.md`.
- Add both signatures to the `feature_functions_are_present_in_exported_catalog`
  expectation list (Phase 4 / filesystem block).

### Docs

- Regenerate/refresh `darkmatter/docs/topics/darkmatter-expressions.md` if it
  enumerates the Filesystem functions, and re-hash if the file carries a
  frontmatter `hash:` (`md hash <file>`).

## Open Questions

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
