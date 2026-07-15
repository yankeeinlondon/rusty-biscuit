---
agent: claude/
total_phases: 4
created: 2026-07-14
phase: 1
yolo: "true"
---

# Execution Plan: `find_first_index` / `find_last_index`

Two new read-side Filesystem expression functions that resolve a file reference
to the lowest- or highest-indexed **existing** sibling in the same directory.
They are the first index-family functions to read the directory itself.

Source spec: [`spec.md`](./spec.md).

## Orientation (read before starting)

All work lives in one crate area:
`darkmatter/lib/src/markdown/compose/expression/`.

Key anchor points confirmed against the current code:

- **Handlers** — `functions/mod.rs`. Mirror `increment_file_index_fn`
  (mod.rs:1781) and `decrement_file_index_fn` (mod.rs:1810). Reuse the existing
  `resolve_path_arg`/`resolve_path_shape` (mod.rs:1640/1653), `indexed_stem_info`
  (mod.rs:1706), `file_stem` (mod.rs:380), `file_extension` (mod.rs:371),
  `path_display_components` (mod.rs:1734), `make_portable_relative`,
  `require_args_expr` (mod.rs:1456), `require_string_expr` (mod.rs:1464), and
  `any_null` (mod.rs:532). Introduce **no** new grammar.
- **Bindings** — `functions/paths.rs` `BINDINGS` (paths.rs:3). Each entry is
  `EvaluationMode::Context` + `FunctionHandler::Context(super::…)`.
- **Catalog** — `docs/schemas/expression-functions.yaml`, embedded at compile
  time via `include_str!` in `catalog/mod.rs:346`. Editing the YAML is a
  compile-time change.
- **Registry validity is all-or-nothing.** `validate_registry`
  (mod.rs:210) panics on first `joined_bindings()` call if a binding lacks a
  catalog entry (`BindingWithoutCatalog`) or vice-versa
  (`CatalogWithoutBinding`). Therefore the handler, the binding, **and** the
  catalog entry for each function must land together before any test runs.
- **Executable examples run against `make_fixture`** (catalog/mod.rs:785),
  which already writes `review-1.md` and `review-2.md` and **no** `review.md`.
  No fixture additions are needed.
- **Doc table is generated + asserted.** `narrative_doc_function_table_matches_catalog`
  (catalog/mod.rs:684) compares the block between the `BEGIN/END GENERATED
  FUNCTION TABLE` markers in `docs/topics/darkmatter-expressions.md` against
  `generate_expression_function_table()` (catalog/mod.rs:376). Adding catalog
  entries makes this test fail until the doc table is refreshed.
- The docs file has **no frontmatter and no `hash:`** — no `md hash` step
  is required (spec's "re-hash if it carries a hash" condition is not met).

Rulings already locked in the spec (do not relitigate): base-first ranking;
empty-family → identity return; append at `order: 88/89` with no renumber.

---

## Phase 1 — Catalog + Runtime Implementation

Goal: both functions are registered, dispatchable, and evaluate correctly.
The registry is valid only once all three edits below land, so treat this
phase as a single atomic unit — do not run the test suite until every task in
it is complete.

- [ ] In `functions/mod.rs`, add a private endpoint marker
  `enum Endpoint { First, Last }` (module-private, near the other path helpers
  around mod.rs:1700).
- [ ] In `functions/mod.rs`, implement the shared helper
  `fn find_index_endpoint(name: &'static str, args: &[Value], ctx: &ResolutionContext, endpoint: Endpoint) -> Result<Value, ExpressionError>`:
  - [ ] `require_args_expr(name, args, 1)?;` then `if any_null(args) { return Ok(Value::Null); }`.
  - [ ] Resolve the argument with `resolve_path_arg(name, &args[0], ctx)?`
    (this is what rejects HTTP(S) URLs via `resolve_path_shape`, and drives the
    non-string → type error via `require_string_expr`).
  - [ ] Derive the display basename via `path_display_components(&path, &ctx.base_dir)`,
    then `file_stem` + `file_extension`; strip the input's own index with
    `indexed_stem_info` to obtain the **base stem** (`Some((base,_,_)) => base`,
    else the stem verbatim).
  - [ ] Compute the directory to scan as `path.parent()`. If there is no parent,
    or `std::fs::read_dir(parent)` errors (missing/unreadable dir), fall through
    to the empty-candidate fallback.
  - [ ] For each `read_dir` entry: take `file_name()` → lossy string; split into
    entry stem + entry extension via `file_stem`/`file_extension`; **membership
    test** — keep the entry iff (a) entry extension equals the input extension
    (exact, case-sensitive) **and** (b) the entry stem is exactly the base stem
    **or** `indexed_stem_info(entry_stem)` yields a base equal to the base stem.
    Record `(Option<u64> ordinal, entry_file_name_string, entry_path)` where the
    unindexed base is `None` and an indexed member is `Some(index)`.
  - [ ] Order key: `(Option<u64>, String)` — derived `Ord` puts `None` (base)
    before every `Some`, then numeric-ascending on the index, with the raw
    filename as the deterministic tie-break for duplicate ordinals from padding
    (`foo-2` vs `foo-002`). Pick `min` for `Endpoint::First`, `max` for `Endpoint::Last`.
  - [ ] If candidates exist, the chosen path is the member's real on-disk path
    (preserving verbatim casing/padding). If the candidate set is empty, the
    chosen path is the resolved input path unchanged.
  - [ ] Return `Ok(Value::String(make_portable_relative(&chosen_path, &ctx.base_dir)))`.
- [ ] In `functions/mod.rs`, add the two public wrappers delegating to the helper:
  - [ ] `pub fn find_first_index_fn(args, ctx) -> …` → `find_index_endpoint("find_first_index", args, ctx, Endpoint::First)`.
  - [ ] `pub fn find_last_index_fn(args, ctx) -> …` → `find_index_endpoint("find_last_index", args, ctx, Endpoint::Last)`.
- [ ] In `functions/paths.rs`, append two `FunctionBinding` entries to `BINDINGS`
  (immediately after `decrement_file_index`, matching the family grouping):
  - [ ] `find_first_index` / alias `findfirstindex`, `EvaluationMode::Context`,
    `FunctionHandler::Context(super::find_first_index_fn)`.
  - [ ] `find_last_index` / alias `findlastindex`, `EvaluationMode::Context`,
    `FunctionHandler::Context(super::find_last_index_fn)`.
- [ ] In `docs/schemas/expression-functions.yaml`, add the two Filesystem
  catalog entries verbatim from spec §"Catalog Changes" — `find_first_index`
  `order: 88` (example `find_first_index("review-2.md")` ⇒ `review-1.md`) and
  `find_last_index` `order: 89` (example `find_last_index("review-1.md")` ⇒
  `review-2.md`), both `returns.type: file`, `fallible: true`,
  `verification: executable`. Do **not** renumber any existing entry.

**Parallelization:** the YAML edit is independent of the two Rust edits and may
be done concurrently; the two Rust edits are sequential (helper → wrappers →
bindings). All three must land before validation.

**Checkpoint (Phase 1 exit):**
- [ ] `just build` compiles (from `darkmatter/`).
- [ ] `just test expression` (or the crate's `just test`) runs without a
  registry panic, and these auto-parity tests pass:
  `registration_names_aliases_and_signatures_are_unique`,
  `catalog_and_runtime_bindings_have_bidirectional_canonical_parity`,
  `handler_kinds_dispatch_through_their_intended_paths`,
  `descriptor_signature_set_equals_dispatchable_signature_set`,
  `every_descriptor_overload_is_dispatchable_at_its_declared_arity`,
  `every_example_evaluates_to_its_declared_result` (proves the two executable
  examples evaluate against `make_fixture`).
- Expected still-red at this point: `narrative_doc_function_table_matches_catalog`
  (fixed in Phase 2). This is the signal that Phase 2 is needed, not a defect.

---

## Phase 2 — Documentation & Parity Sync

Goal: the generated doc table and the hand-maintained read-side list reflect the
two new functions, and the Claudine anti-drift expectation list includes them.
Depends only on Phase 1. **Parallelizable with Phase 3.**

- [ ] Refresh the generated function table in
  `docs/topics/darkmatter-expressions.md` (the block between
  `<!-- BEGIN GENERATED FUNCTION TABLE -->` and `<!-- END … -->`) so it matches
  `generate_expression_function_table()` output — two new rows in the Filesystem
  category, rendered at the **end** of the Filesystem group (order 88/89):
  `| Filesystem | \`find_first_index(file)\` | <description> | \`find_first_index("review-2.md")\` ⇒ \`review-1.md\` |`
  and the `find_last_index` counterpart. Use the exact row format from the
  generator (category, backtick-wrapped `key()`, `|`-escaped description,
  executable example). Easiest verification: run the crate tests and let
  `narrative_doc_function_table_matches_catalog` dictate the exact expected string.
- [ ] Add two rows to the **hand-maintained** read-side Filesystem list in the
  same doc (the `| Function | Description |` table near
  darkmatter-expressions.md:500, after `decrement_file_index`):
  `find_first_index(file)` and `find_last_index(file)` with one-line descriptions
  consistent with the catalog `description`.
- [ ] Add both signatures to the expected list in
  `feature_functions_are_present_in_exported_expression_catalog`
  (catalog/mod.rs:713), under the Phase 4 / Filesystem block:
  `"find_first_index(file)"`, `"find_last_index(file)"`.

**Checkpoint (Phase 2 exit):**
- [ ] `narrative_doc_function_table_matches_catalog` passes.
- [ ] `feature_functions_are_present_in_exported_expression_catalog` passes with
  the two new signatures present.

---

## Phase 3 — Unit Tests

Goal: cover every behavioral branch from spec §"Testing → Unit". Add to the
existing `#[cfg(test)] mod tests` in `functions/mod.rs` (starts at mod.rs:2590),
alongside the existing `increment_file_index` tests, using `tempfile::TempDir`
fixtures (the established pattern in that module). Depends only on Phase 1.
**Parallelizable with Phase 2.**

- [ ] **first/last across a full family** — dir `{foo.md, foo-2.md, foo-3.md}`:
  `find_first_index("foo-2.md")` → `foo.md`; `find_last_index("foo-2.md")` →
  `foo-3.md`; identical results when the input is `foo.md`.
- [ ] **no siblings → identity** — dir `{foo-2.md}` only: both functions return
  `foo-2.md`.
- [ ] **empty candidate set / missing input → identity** — input `bar-4.md`
  with no `bar*.md` present: both return `bar-4.md` unchanged.
- [ ] **numeric vs lexicographic ordering** — dir `{foo-2.md, foo-10.md}`:
  `find_last_index("foo-2.md")` → `foo-10.md`.
- [ ] **zero-padding preserved verbatim** — dir `{foo.md, foo-002.md}`:
  `find_last_index("foo.md")` → `foo-002.md` (real on-disk name).
- [ ] **extension isolation** — dir `{foo-2.md, foo-3.txt}`:
  `find_last_index("foo-2.md")` → `foo-2.md`.
- [ ] **non-family neighbor isolation** — dir `{foo-2.md, food-9.md, foo-bar.md}`:
  `find_last_index("foo-2.md")` → `foo-2.md`.
- [ ] **directory isolation** — a `sub/foo-9.md` does not affect
  `find_last_index("foo-2.md")` scanning the parent.
- [ ] **null propagation** — `find_first_index(null)` → `Value::Null`.
- [ ] **remote rejection** — `find_last_index("https://example.com/foo.md")`
  returns `Err`.

**Checkpoint (Phase 3 exit):**
- [ ] All new unit tests pass via `just test` (nextest).

---

## Phase 4 — Full Validation

Goal: the whole area is green, lint-clean, and cross-platform-safe. Depends on
Phases 1–3.

- [ ] `just build` (from `darkmatter/`) — clean compile.
- [ ] `just lint` — no clippy warnings on the new code.
- [ ] `just test` — full unit suite green (includes all parity + new tests).
- [ ] `just test-l2` — integration tests unaffected/green.
- [ ] Cross-platform sanity review (no host to test Windows/Linux directly):
  confirm membership tests operate on the forward-slash display basename from
  `path_display_components` (no raw `\` leakage), comparisons are case-sensitive
  by design, ordering is numeric on the parsed index, and output goes through
  `make_portable_relative` — matching spec §"Cross-Platform Considerations".
- [ ] `detect_changes()` (GitNexus) to confirm the blast radius is limited to
  the two new functions plus the expected catalog/doc/test edits before handing
  off for commit.

---

## Success Criteria

- `find_first_index(file)` and `find_last_index(file)` are registered,
  dispatchable, `fallible`, `EvaluationMode::Context` functions in the
  Filesystem category (orders 88/89, no existing entry renumbered).
- Behavior matches every worked example and the spec's ordering, fallback,
  extension-isolation, directory-isolation, null, and remote-rejection rules.
- All catalog/runtime parity tests, the doc-table test, the anti-drift catalog
  list test, and all new unit tests pass.
- `just build`, `just lint`, `just test`, and `just test-l2` are green.
- No new grammar introduced; existing index-family helpers reused throughout.
