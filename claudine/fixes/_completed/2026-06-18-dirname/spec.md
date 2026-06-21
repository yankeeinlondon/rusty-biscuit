---
status: ready for planning and implementation
reviewed: true
---

# Rename `dir(file)` to `dirname(file)`

## Purpose

Rename the Darkmatter expression function `dir(file)` to `dirname(file)` across
its implementation, descriptor catalog, and documentation. The current name is
ambiguous — `dir` reads as "directory" / "is it a dir?" — whereas the function
actually returns the _dirname_ (the directory portion of a display path). The
canonical POSIX utility with identical semantics is `dirname(1)`, which is the
name authors expect.

The function was recently added and is not yet widely referenced in the
repository, so a clean rename is low-risk and keeps the expression surface
self-documenting.

## Background

`dir(file)` is a context-aware filesystem expression in Darkmatter's compose
expression engine. It returns the directory portion of a display path
(`dir("sub/note.md")` ⇒ `"sub"`), complementing `basename`, `ext`,
`parent_dir`, `file_trailing`, and `dir_leading`.

The dispatch + descriptor model has three layers, all of which carry the name:

1. **Handler + dispatch table** — `FS_FUNCTIONS` in
   `darkmatter/lib/src/markdown/compose/expression/functions.rs` maps the
   canonical name to the handler `dir_fn` via `dispatch_fs`.
2. **Descriptor catalog** — `EXPRESSION_FUNCTION_DESCRIPTORS` in
   `darkmatter/lib/src/markdown/compose/expression/catalog.rs` carries the
   user-facing `signature`, `description`, `category`, and executable
   `example`.
3. **Docs** — `darkmatter/docs/topics/darkmatter-expressions.md` renders the
   catalog-backed generated function table and also contains a narrative helper
   table that must be kept in sync.

### `claudine context --expressions` is data-driven

`claudine context --expressions` does **not** hardcode any function names. It
iterates `darkmatter::markdown::compose::expression::expression_function_descriptors()`
at runtime and renders one row per descriptor signature. The claudine test
`context_expressions_includes_every_function`
(`claudine/cli/tests/context_command.rs:486`) likewise derives its expected set
from that catalog.

Consequence: renaming the descriptor signature in Darkmatter is _both_ the
library fix _and_ the claudine report fix. There is no separate claudine source
edit required. The claudine acceptance gate is therefore a verification (the
report shows `dirname(file)`) rather than an edit.

## Decision: Clean Rename, No Alias

The `FsFunction` table supports an `aliases: &[&str]` field, so keeping `dir`
as a backward-compatible alias is mechanically possible. This spec intentionally
rejects that option:

- Existing aliases in `FS_FUNCTIONS` are all snake_case ⇄ flat-case ergonomic
  pairs (e.g. `parent_dir` / `parentdir`, `file_exists` / `fileexists`). None
  are semantic deprecations, and there is no deprecation-warning machinery
  wired into `FsFunction` dispatch today.
- The function is newly added and not referenced anywhere in the repository
  other than its own definition, descriptor, docs, and tests.
- Rule of surgical changes: a single clean rename is simpler than introducing a
  new deprecation category for one function.

`dir(file)` must resolve as an unknown function after this change, surfacing the
existing did-you-mean warning that the compose prepare layer already emits for
unknown expression functions.

Reader's note: this is an intentional expression-language breaking change for a
newly introduced function, not an accidental standards drift. The mitigation is
to land the rename before `dir(file)` appears in public examples or downstream
specs, update every generated and narrative documentation surface in the same
change, and add a regression assertion that the old name does not dispatch.

## Scope

### In scope

Darkmatter library + docs:

- `darkmatter/lib/src/markdown/compose/expression/functions.rs`
  - the `dir_fn` handler (rename symbol to `dirname_fn`)
  - its `FS_FUNCTIONS` entry (`canonical`, `signatures`, `handler`)
  - the handler's rustdoc `/// dir(file) -> string` line
  - the `require_args("dir", …)` / `resolve_path_arg("dir", …)` name strings
  - the three in-module unit tests that call `dir_fn`
- `darkmatter/lib/src/markdown/compose/expression/catalog.rs`
  - the `dir(file)` descriptor entry (signature + example invocation)
  - the `feature_functions_are_present_in_exported_expression_catalog` test
    expectation
- `darkmatter/docs/topics/darkmatter-expressions.md`
  - the generated verbose function table between the
    `BEGIN GENERATED FUNCTION TABLE` / `END GENERATED FUNCTION TABLE` markers
  - the narrative compact path-helper table under "Indexed and Path Helpers"

Claudine (verification only — no source edits):

- confirm `claudine context --expressions` renders `dirname(file)` and that
  `context_expressions_includes_every_function` continues to pass.

### Out of scope

- Keeping `dir` as a deprecated alias (see Decision above).
- Renaming any other path function (`parent_dir`, `dir_leading`,
  `file_trailing`, etc.). Their names are accurate.
- Changing handler _behavior_ — semantics, argument count, null propagation,
  and return shape are identical, only the name changes.
- The shell `$()` expansion's `dirname` executable — that is a different
  namespace (POSIX `dirname(1)` invoked from frontmatter shell directives) and
  is unrelated to the expression function.
- Any CHANGELOG, release note, or migration guide beyond this spec and the docs
  table edits.

## Normative Behavior

After the rename:

- `dirname("sub/note.md")` ⇒ `"sub"` (previously `dir("sub/note.md")`).
- `dirname("note.md")` ⇒ `""` (bare basename, no directory portion).
- `dirname("foo/bar/baz/test.md")` ⇒ `"foo/bar/baz"`.
- Argument-count validation error message names the function `dirname`:
  `require_args("dirname", …)` / `resolve_path_arg("dirname", …)`.
- Null propagation is unchanged: a `null` argument yields `Value::Null`.
- Dispatch is unchanged in every other respect: `FS_FUNCTIONS` remains the
  single source of truth for the filesystem surface, resolution is
  case-sensitive against `canonical` and `aliases`, and `dir` (the old name) is
  no longer resolvable.

The descriptor must keep `category: "Filesystem"`, its current `order`, and the
executable example verification level — only the name tokens change.

Docs source-of-truth: the verbose function table in
`darkmatter/docs/topics/darkmatter-expressions.md` is generated from
`EXPRESSION_FUNCTION_DESCRIPTORS` and enforced by
`narrative_doc_function_table_matches_catalog`. Implementers must update the
descriptor first, regenerate the generated table, and then update the separate
narrative helper table by hand.

## Implementation Requirements

### `functions.rs`

1. Rename the handler `pub fn dir_fn` → `pub fn dirname_fn`. Update the rustdoc
   leading line to `/// dirname(file) -> string` — keep the existing trailing
   description prose ("the directory portion of the display path").
2. In the handler body, change the two name strings passed to `require_args`
   and `resolve_path_arg` from `"dir"` to `"dirname"` so validation/error
   messages report the correct name.
3. In `FS_FUNCTIONS`, update the entry to:
   `FsFunction { canonical: "dirname", aliases: &[], signatures: &["dirname(file)"], handler: dirname_fn }`.
   Do not add `"dir"` to `aliases`.
4. Update the three `dir_fn(…)` call sites in the `#[cfg(test)]` modules
   (`path_components_resolve_against_base_dir` at ~L2900,
   `path_components_handle_bare_basename` at ~L2923, and
   `path_functions_do_not_require_existence` at ~L2974) to call `dirname_fn`.

### `catalog.rs`

1. Change the descriptor `signature` from `"dir(file)"` to `"dirname(file)"`.
   Leave `description`, `category`, `order`, and `verification` unchanged.
2. Change the descriptor example `invocation` from `"dir(\"sub/note.md\")"` to
   `"dirname(\"sub/note.md\")"`. The `result` (`"sub"`) is unchanged.
3. In `feature_functions_are_present_in_exported_expression_catalog`, change
   the expected `"dir(file)"` string to `"dirname(file)"`.

### `darkmatter-expressions.md`

1. Regenerate the generated function table after the descriptor change:

   ```sh
   just --justfile darkmatter/justfile --working-directory darkmatter regen-expr-doc
   ```

   This should change the verbose table row to `` `dirname(file)` `` and the
   example cell to `` `dirname("sub/note.md")` ⇒ `sub` ``. The description is
   unchanged.
2. In the narrative compact table under "Indexed and Path Helpers": change the
   signature cell to
   `` `dirname(file)` ``.

## Testing Requirements

- All existing Darkmatter expression tests pass after the rename, including
  `path_functions_dispatch_by_name` (which does not reference `dir` directly but
  exercises the same `dispatch_fs` path that the renamed entry plugs into).
- Add or update a focused dispatch regression so `dispatch_fs("dirname", …)`
  resolves and `dispatch_fs("dir", …)` returns `None`. This makes the "no alias"
  decision executable instead of relying only on a source search.
- The catalog test `feature_functions_are_present_in_exported_expression_catalog`
  passes with the updated `dirname(file)` expectation.
- The catalog/doc parity test `narrative_doc_function_table_matches_catalog`
  passes after regenerating the verbose function table.
- No test in the repository continues to reference `dir_fn`, `dir(file)` as a
  descriptor signature, or `dispatch_fs("dir", …)`.

Claudine regression coverage is provided by the existing
`context_expressions_includes_every_function` test, which dynamically derives
its expected set from the (now renamed) catalog. No new claudine test is
required, but the spec accepts an optional targeted assertion that the report
contains the literal `dirname(file)` row if the implementer wants belt-and-
braces coverage.

## Documentation Requirements

- The two `darkmatter-expressions.md` table rows are updated as specified
  above: the generated verbose table via `regen-expr-doc`, and the narrative
  compact helper table by hand.
- No skill (`SKILL.md`) edit is required: the claudine and darkmatter skills
  describe the `context --expressions` surface generically and do not enumerate
  `dir(file)`.
- No `AGENTS.md` or README change is required.

## Verification

Confirm the rename is exhaustive and the claudine report picks it up:

```sh
# 1. Regenerate the catalog-backed docs table after the descriptor rename.
just --justfile darkmatter/justfile --working-directory darkmatter regen-expr-doc

# 2. No stale references to the old expression function name remain in the
#    expression engine, expression docs, or Claudine docs/tests. Shell `$()`
#    dirname executable matches outside these paths are unrelated.
rg -n 'dir_fn|dir\(file\)|canonical: "dir"|signature: "dir\(file\)"' \
  darkmatter/lib/src/markdown/compose/expression \
  darkmatter/docs/topics/darkmatter-expressions.md \
  claudine

# 3. Darkmatter builds and the expression/catalog tests pass, including doc
#    table parity.
cargo test -p darkmatter --lib markdown::compose::expression

# 4. Claudine context report shows the new name.
cargo run -p claudine-cli -- context --expressions | rg 'dirname\(file\)'
```

## Acceptance Criteria

- [ ] `dir(file)` is renamed to `dirname(file)` in the `FS_FUNCTIONS` table
      (`canonical`, `signatures`) and the handler is renamed `dirname_fn`.
- [ ] The `EXPRESSION_FUNCTION_DESCRIPTORS` entry reports `signature: "dirname(file)"`
      with an executable example invoking `dirname("sub/note.md")`.
- [ ] `dir` is not present in `FS_FUNCTIONS` `aliases`; `dispatch_fs("dir", …)`
      no longer resolves.
- [ ] A focused regression test asserts `dispatch_fs("dirname", …)` resolves
      and `dispatch_fs("dir", …)` returns `None`.
- [ ] Argument-count / resolution error messages name `dirname`, not `dir`.
- [ ] The generated function table in `darkmatter/docs/topics/darkmatter-expressions.md`
      is regenerated from the descriptor catalog, and the narrative helper table
      also reads `dirname(file)`.
- [ ] `cargo test -p darkmatter --lib markdown::compose::expression` passes.
- [ ] `claudine context --expressions` renders a `dirname(file)` row and the
      claudine `context_expressions_includes_every_function` test passes.
