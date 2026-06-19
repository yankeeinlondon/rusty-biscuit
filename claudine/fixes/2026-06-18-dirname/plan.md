---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-18
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
docs_updated_during_phase_1:
  - claudine/fixes/2026-06-18-dirname/plan.md
  - prompts/implement-suggestions.md
  - prompts/implement-plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - darkmatter/docs/topics/darkmatter-expressions.md
  - claudine/fixes/2026-06-18-dirname/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - claudine/fixes/2026-06-18-dirname/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
documentation:
  - claudine/fixes/2026-06-18-dirname/plan.md
  - prompts/implement-suggestions.md
  - prompts/implement-plan.md
  - darkmatter/docs/topics/darkmatter-expressions.md
packages:
  - darkmatter
  - claudine
---

# Plan: Rename `dir(file)` to `dirname(file)`

Executable conversion of [`spec.md`](./spec.md). A clean, no-alias rename of a
newly added Darkmatter expression function across three layers: dispatch table,
descriptor catalog, and docs. Claudine is verification-only (its
`context --expressions` report is data-driven from the catalog).

## Dependency graph

```
Phase 1 (library source rename) ──┬── Phase 2 (no-alias regression test)
                                  └── Phase 3 (docs regen + narrative edit)
                                            └── Phase 4 (cross-cutting verification)
```

- **Phase 2 and Phase 3 are parallelizable** — both depend solely on Phase 1 and
  touch disjoint files (`functions.rs` test module vs. `*.md`).
- Phase 1 is sequential within itself but its two files (`functions.rs`,
  `catalog.rs`) can be edited concurrently.
- Phase 4 is the acceptance gate and must run last.

## Conventions for the implementer

- Never run `cargo fmt` in write mode (repo rule). Match surrounding style by hand.
- Do not add `"dir"` to `FS_FUNCTIONS` `aliases` — the spec rejects a deprecated alias.
- Use US English in any prose touched.
- Line numbers below are anchors from the branch at planning time; verify with
  `rg` before editing since offsets drift.

---

## Phase 1 — Rename in library source

Goal: the darkmatter crate compiles and every pre-existing expression test that
referenced the old name is updated to the new name. Two independent files; edit
concurrently.

### 1A. `darkmatter/lib/src/markdown/compose/expression/functions.rs`

- [x] Rename the handler `pub fn dir_fn` → `pub fn dirname_fn` (functions.rs:1255).
- [x] Update the handler rustdoc leading line from `` /// `dir(file) -> string` ``
      to `` /// `dirname(file) -> string` `` (functions.rs:1254); keep the
      trailing "directory portion of the display path" prose unchanged.
- [x] In the handler body, change the two name strings: `require_args("dir", …)`
      → `require_args("dirname", …)` (functions.rs:1256) and
      `resolve_path_arg("dir", …)` → `resolve_path_arg("dirname", …)`
      (functions.rs:1260) so arity/resolution errors name `dirname`.
- [x] Update the `FS_FUNCTIONS` entry (functions.rs:1811) to
      `FsFunction { canonical: "dirname", aliases: &[], signatures: &["dirname(file)"], handler: dirname_fn }`.
      Do **not** add `"dir"` to `aliases`.
- [x] Update the three `dir_fn(…)` test call sites to `dirname_fn(…)`:
      functions.rs:2900 (`path_components_resolve_against_base_dir`),
      functions.rs:2923 (`path_components_handle_bare_basename`), and
      functions.rs:2974 (`path_functions_do_not_require_existence`).

### 1B. `darkmatter/lib/src/markdown/compose/expression/catalog.rs`

- [x] Change the descriptor `signature` from `"dir(file)"` to `"dirname(file)"`
      (catalog.rs:707). Leave `description`, `category: "Filesystem"`, `order`,
      and `verification` unchanged.
- [x] Change the descriptor example `invocation` from `"dir(\"sub/note.md\")"`
      to `"dirname(\"sub/note.md\")"` (catalog.rs:712). `result: "sub"` is
      unchanged.
- [x] In `feature_functions_are_present_in_exported_expression_catalog`, change
      the expected `"dir(file)"` string to `"dirname(file)"` (catalog.rs:1122).

### Checkpoint 1 — compiles, renamed-handler tests green

```sh
cargo build -p darkmatter
cargo test -p darkmatter --lib markdown::compose::expression::functions
cargo test -p darkmatter --lib markdown::compose::expression::catalog::tests::feature_functions_are_present_in_exported_expression_catalog
```

Expected: build succeeds; the functions-module tests (including the three
renamed call sites) pass; the catalog membership test passes with the updated
expectation. The docs-parity test `narrative_doc_function_table_matches_catalog`
is **expected to fail here** — it is resolved in Phase 3.

---

## Phase 2 — No-alias regression test (parallelizable with Phase 3)

Goal: make the "no backward-compatible alias" decision executable rather than
relying on a source search. Depends on Phase 1's renamed `FS_FUNCTIONS`.

### 2A. Focused dispatch regression in `functions.rs` test module

- [x] Add a test in `mod fn_filesystem` (alongside
      `dispatch_fs_returns_none_for_non_fs_names` at functions.rs:2292) that
      asserts **both** directions:
      - `dispatch_fs("dirname", &[json!("sub/note.md")], &ctx)` resolves and
        returns `Ok(Value::String("sub".into()))`.
      - `dispatch_fs("dir", &[json!("sub/note.md")], &ctx)` returns `None`
        (old canonical name no longer dispatches; not present in `aliases`).
- [x] Optionally also extend `path_functions_dispatch_by_name`
      (functions.rs:2982) with a `dispatch_fs("dirname", …)` assertion for
      symmetry with the other path helpers exercised there.

### Checkpoint 2 — no-alias assertion is red without the rename

```sh
cargo test -p darkmatter --lib markdown::compose::expression::functions dispatch
```

Expected: the new regression test passes. (Sanity: on the pre-rename tree it
would fail on the `dispatch_fs("dir", …)` → `None` leg, proving it guards the
decision.)

---

## Phase 3 — Documentation (parallelizable with Phase 2)

Goal: every docs surface reads `dirname(file)`. The generated verbose table is
regenerated from the (now renamed) catalog; the narrative compact table is
hand-edited. Depends on Phase 1B (the generator reads `EXPRESSION_FUNCTION_DESCRIPTORS`).

### 3A. Regenerate the catalog-backed verbose function table

- [x] Run the generator via the darkmatter just recipe:
      ```sh
      just --justfile darkmatter/justfile --working-directory darkmatter regen-expr-doc
      ```
      (This wraps `cargo run -p darkmatter --example expression_doc_generator -- --write`.)
- [x] Verify the generated row in
      `darkmatter/docs/topics/darkmatter-expressions.md` (≈ line 375, between the
      `BEGIN GENERATED FUNCTION TABLE` / `END GENERATED FUNCTION TABLE` markers)
      now reads `| Filesystem | \`dirname(file)\` | … | \`dirname("sub/note.md")\` ⇒ \`sub\` |`.
      Do **not** hand-edit this row — it is generated.

### 3B. Hand-edit the narrative compact helper table

- [x] In `darkmatter/docs/topics/darkmatter-expressions.md` under
      "Indexed and Path Helpers" (darkmatter-expressions.md:462), change the
      signature cell from `` `dir(file)` `` to `` `dirname(file)` ``. Description
      column unchanged.

### Checkpoint 3 — docs parity test now green

```sh
cargo test -p darkmatter --lib markdown::compose::expression::catalog narrative_doc_function_table_matches_catalog
```

Expected: the docs/catalog parity test passes, confirming the generated table is
in sync with the renamed descriptor.

---

## Phase 4 — Cross-cutting verification & acceptance

Goal: prove the rename is exhaustive, no stale references remain, and Claudine's
data-driven report picks up the new name without any claudine source edit. Must
run after Phases 1–3.

### 4A. Stale-reference sweep

- [x] Run the spec's ripgrep and confirm zero matches in the expression engine,
      expression docs, and claudine tree:
      ```sh
      rg -n 'dir_fn|dir\(file\)|canonical: "dir"|signature: "dir\(file\)"' \
        darkmatter/lib/src/markdown/compose/expression \
        darkmatter/docs/topics/darkmatter-expressions.md \
        claudine
      ```
      Note: matches for the POSIX `dirname(1)` shell executable elsewhere (e.g.
      `docs/topics/schema-definition.md`) are unrelated and out of scope.

### 4B. Full expression test suite

- [x] Run the canonical expression test command from the spec:
      ```sh
      cargo test -p darkmatter --lib markdown::compose::expression
      ```
      Expected: all tests pass, including `path_functions_dispatch_by_name`,
      `feature_functions_are_present_in_exported_expression_catalog`, the new
      Phase 2 regression, and `narrative_doc_function_table_matches_catalog`.

### 4C. Claudine report verification (no source edits expected)

- [x] Confirm the claudine report renders a `dirname(file)` row:
      ```sh
      cargo run -p claudine-cli -- context --expressions | rg 'dirname\(file\)'
      ```
- [x] Confirm the data-driven claudine regression still passes (it derives its
      expected set from the renamed catalog):
      ```sh
      cargo test -p claudine-cli --test context_command context_expressions_includes_every_function
      ```

### Checkpoint 4 — acceptance criteria met

Walk the spec's acceptance list and confirm each:

- [x] `dir(file)` is renamed to `dirname(file)` in `FS_FUNCTIONS` (`canonical`,
      `signatures`) and the handler is `dirname_fn`.
- [x] `EXPRESSION_FUNCTION_DESCRIPTORS` reports `signature: "dirname(file)"`
      with example `dirname("sub/note.md")`.
- [x] `dir` absent from `FS_FUNCTIONS` `aliases`; `dispatch_fs("dir", …)`
      returns `None`.
- [x] Focused regression asserts `dispatch_fs("dirname", …)` resolves and
      `dispatch_fs("dir", …)` returns `None`.
- [x] Arity/resolution error messages name `dirname`, not `dir`.
- [x] Generated function table regenerated; narrative helper table reads
      `dirname(file)`.
- [x] `cargo test -p darkmatter --lib markdown::compose::expression` passes.
- [x] `claudine context --expressions` renders a `dirname(file)` row and
      `context_expressions_includes_every_function` passes.

---

## Risk notes

- **Breaking expression-language change** — intentional, per spec, for a newly
  added function. Mitigated by landing before `dir(file)` appears in public
  examples; the Phase 2 regression locks the breakage in.
- **Generated-vs-narrative drift** — the two docs tables are maintained by
  different mechanisms. Phase 3A (regen) and Phase 3B (hand-edit) must both
  land; the `narrative_doc_function_table_matches_catalog` parity test guards
  only the generated side.
- **No claudine edit required** — `claudine context --expressions` iterates the
  descriptor catalog at runtime. If Phase 4C shows a `dir(file)` row, the cause
  is a stale catalog (Phase 1B not applied), not a claudine hardcoding.
