---
total_phases: 6
created: 2026-09-15
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: true
reviewed: true
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine
---

# Execution Plan: One Array Rendering, and It Is JSON

## Work Summary and Definition of Success

This plan implements the reviewed specification in `spec.md`. A bare array
rendered into text becomes compact JSON on every expression string-output
path: `interpolation_output_string` is deleted, its three call sites in
`compose/interpolation/evaluator.rs` call `scalar_string` directly, and two
new list functions — `as_json(…)` and `as_json5(…)` — join the family so the
new default has an explicit spelling. Typed boundaries stay typed, the
newline-joined form remains available through `as_line_separated(…)`, the
in-repo audit moves every bare-array-in-text site to an explicit function,
and six authored documentation locations are corrected.

Planning-time verification (already performed, so the plan starts from
facts rather than assumptions):

- `interpolation_output_string` lives at
  `darkmatter/lib/src/markdown/compose/expression/mod.rs:362` with exactly
  three call sites in
  `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs` (lines
  251, 261, 277) plus its import at line 78, and two unit tests at
  `expression/mod.rs:875-887`.
- The list family's shared body (`join_list`, `list_element_strings`,
  null-propagation and `requires an array argument` conventions) is in
  `compose/expression/functions/mod.rs:1011-1054`; registrations are in
  `compose/expression/functions/collections.rs`.
- Catalog orders 90-96 are occupied by the six shipped List Formatting
  entries in `docs/schemas/expression-functions.yaml`; 97 and 98 are free.
  The generated table in `docs/topics/darkmatter-expressions.md` is
  produced by the area just recipe running
  `cargo run -p darkmatter --example expression_doc_generator -- --write`.
- `biscuit_file::json5::to_json5_compact` exists at
  `biscuit-file/lib/src/json5/format.rs:19`; `darkmatter/lib/Cargo.toml:38`
  does not yet enable the `json5` feature explicitly.
- Ken's `prompts/commit.md` edit (`as_csv(ctx.dirty_package_areas)`,
  lines 23-25) is **already in place** — the audit must preserve it
  untouched.
- The related nested-span fix
  (`claudine/fixes/2026-09-13-better-static-analysis`) is active but **not
  implemented** — no suppression code exists anywhere in the tree. Per the
  spec's Sequencing section, D3 is therefore conditional (see Necessary
  Rules).

Successful completion looks like: Darkmatter and Claudine L1 green on macOS
(`just test` in both areas) with every new test proven red on the commit
before its fix; `interpolation_output_string` gone; eight list functions
registered, catalogued, completable, and hoverable; typed whole-value
frontmatter/loop/sequence results still arrays; every audited in-repo site
rendering through an explicit function with updated expected outputs; the
six doc locations, the active comments, and the skill hash corrected; and
the PR description stating the D4 reversal.

## Phase 1: Rulings, Baseline, and Audit Inventory

### Necessary Rules

- **Ruling 1 — D3 sequencing (decisive for Phase 5).** The nested-span fix
  has not landed; no suppression code exists to remove. Default ruling:
  **this fix lands first and Phase 5 (D3) is dropped** — never implement a
  removal against code that never existed. Re-check at implementation start:
  if the nested-span fix has landed in the meantime (its fix directory moved
  toward `_completed` and its suppression code is present), reinstate Phase
  5 in full. Record the decision and the evidence (repo search for the
  suppression) in the verification record.
- **Ruling 2 — `prompts/commit.md` is closed scope.** Ken already applied
  the intended `as_csv(…)` migration; that file is also a regression fixture
  for the nested-span fix at its pre-fix commit. The audit must not replace,
  re-migrate, or reformat it; any audit tooling that touches it is a defect.
- **Ruling 3 — migration choices are per-site, never blanket.** Each audited
  bare-array-in-text site gets the formatter that matches author intent
  (`as_line_separated`, `as_unordered_list`, `as_csv`, `as_json`, …).
  Exact whole-value typed consumers (frontmatter, loop mutations, typed
  dynamic sequence sources) are classified and retained, never converted.
- **Ruling 4 — no behavioral comment-only drift.** Where this plan re-cuts
  comments (`evaluator.rs` array-case test, `capture/repo.rs`,
  `level2_auto_complete_chooser.rs`), the edits are comment/text-only and
  must not carry code changes in the same hunk beyond what the phase's code
  tasks already cover.

**Work-group 1A (concurrent):**

- [ ] **Task 1.1: Capture baseline and blast-radius evidence.** Record `git
  status --short` and the current `just test` results for `darkmatter/` and
  `claudine/`. Run GitNexus upstream impact analysis on
  `interpolation_output_string`, `scalar_string`, and the list-family
  handlers before any edit; report any HIGH/CRITICAL risk before proceeding
  (the spec's redirect is deliberately narrow — `scalar_string` itself is
  unchanged). Save a pre-change `detect_changes` result for later scope
  review.
- [ ] **Task 1.2: Resolve the D3 gate (Ruling 1).** Search the tree for the
  nested-span suppression implementation; confirm absent (expected) and mark
  Phase 5 dropped, or confirm present and mark it retained. One paragraph in
  the verification record is the deliverable.
- [ ] **Task 1.3: Build the audit inventory.** Enumerate every active
  in-repo prompt, example document, and test fixture that embeds a bare
  array into text. Combine text search with schema/catalog array types — a
  text search alone cannot infer the runtime type of an arbitrary
  expression; cover local frontmatter arrays as well as `ctx.*`. Classify
  each hit as *text-embedding* (migrate in Phase 4) or *typed whole-value*
  (retain), record the intended formatter per text site, and note expected
  snapshots that must change. Mark `prompts/commit.md` as
  already-migrated/do-not-touch (Ruling 2).

**Validation checkpoint 1:** The inventory classifies every hit with a
disposition; the D3 decision is recorded with evidence; baseline test
results and impact analysis are saved where the verification record can
cite them.

### Spikes

None required. The three riskiest unknowns were resolved during planning:
`to_json5_compact` exists and is exported; catalog orders 97/98 are free;
the three evaluator call sites and their import are enumerated above. The
JSON5 formatter's exact output shape (single quotes, eligible unquoted
keys, no trailing newline) is pinned by Phase 3's red-first tests rather
than by a throwaway probe.

## Phase 2: Unify Array Rendering on JSON (D1)

Order matters inside this phase: tests are authored first and shown red on
the pre-fix commit, then the redirect lands and turns them green.

- [ ] **Task 2.1: Author the D1 test obligations (red-first).** In
  `darkmatter/lib` L1: `{{ some_list }}` in a document body renders compact
  JSON; the `+` path and the interpolation path render the same array
  identically, asserted directly against each other (not against a shared
  literal); `{{ some_object }}` is byte-identical to before; an exact
  whole-value frontmatter array stays a typed `Value::Array` while the same
  array in a mixed frontmatter string renders compact JSON; and
  `as_line_separated(list)` still produces newline-joined text. Run them
  against the pre-fix commit and capture the failures for the verification
  record.
- [ ] **Task 2.2: Redirect the three call sites and delete the renderer.**
  In `compose/interpolation/evaluator.rs`, replace
  `interpolation_output_string(&…)` with `scalar_string(&…)` at lines 251,
  261, and 277 and remove the import at line 78. Delete
  `interpolation_output_string` from `compose/expression/mod.rs` along with
  its two unit tests
  (`interpolation_output_string_renders_arrays_line_separated`,
  `interpolation_output_string_matches_scalar_string_for_non_arrays`),
  moving their JSON obligation into the evaluator-boundary tests from Task
  2.1. Do not touch `scalar_string`, the evaluator's object-specific
  `get_string` hook, `eval_json`, or any other typed boundary. **Depends
  on:** Task 2.1 (red evidence captured first).
- [ ] **Task 2.3: Re-cut the stale comments in the touched files.** The
  evaluator's array-arm comment ("Bare array interpolation renders
  line-separated (spec D4)", evaluator.rs ~line 256) and the equivalence
  comment on the array-case test (`{{ items }}` ≡
  `{{ as_line_separated(items) }}`) now state a superseded default; rewrite
  them to state the JSON default and the `as_json(…)` equivalence.
  **Parallelizable:** with Task 2.4 once Task 2.2 compiles.
- [ ] **Task 2.4: Add the CLI integration proof.** A deterministic `md
  compose` integration test launched through `CliProcessFixture`
  (`darkmatter/cli/tests/common/fixture.rs`) proves a body interpolation
  emits compact JSON while `as_line_separated` remains newline-joined. The
  fixture pins its environment (no raw process spawn; declare any rendering
  input claim on the builder if the assertion needs a fixed frame).
  **Parallelizable:** with Task 2.3.

**Validation checkpoint 2:** `just test` and `just lint` in `darkmatter/`
green; every Task 2.1 test was shown red pre-fix; the deleted unit tests
are named in the verification record as intentional removals; `rg
interpolation_output_string darkmatter/` returns only this fix's historical
records.

## Phase 3: Extend the List-Rendering Family (D2)

**Work-group 3A (concurrent):**

- [ ] **Task 3.1: Implement the two serializers.** In
  `compose/expression/functions/mod.rs`, add `as_json(list)` — compact
  strict JSON via the same `scalar_string` path as the bare-array default,
  so `as_json(x)` is byte-identical to bare interpolation of `x` for every
  array — and `as_json5(list)` — compact single-line JSON5 via
  `biscuit_file::json5::to_json5_compact`. Both follow family conventions:
  exactly one argument, `null` propagates as `Value::Null`, a non-array
  returns `requires an array argument`, wrong arity is rejected — but,
  unlike the joiners, an empty array yields `"[]"`, not the empty string.
  No trailing newline in either output.
- [ ] **Task 3.2: Register bindings and catalog entries.** Append
  `FunctionBinding`s in `compose/expression/functions/collections.rs` with
  canonical names `as_json`/`as_json5`, collapsed aliases `asjson`/
  `asjson5`, `EvaluationMode::Pure`, and pure handlers. Add both entries to
  the List Formatting catalog in
  `docs/schemas/expression-functions.yaml` at orders 97 and 98 with
  executable examples, preserving the six shipped entries' relative and
  numeric order. **Depends on:** Task 3.1.
- [ ] **Task 3.3: Make the JSON5 dependency explicit.** Add the `json5`
  feature to the `biscuit-file` dependency in `darkmatter/lib/Cargo.toml`
  (line 38) so the direct call does not depend on that feature remaining in
  `biscuit-file`'s defaults. Record in `darkmatter/docs/dependencies.md`
  that Darkmatter directly uses `biscuit-file`'s JSON5 formatter and
  explicitly enables the feature. No new crate.

**Work-group 3B (after 3A):**

- [ ] **Task 3.4: Author the D2 test obligations (red-first where new).**
  `as_json(list)` byte-identical to bare-array text rendering for empty,
  scalar, mixed, and nested arrays; escaped quotes, backslashes, newlines,
  and Unicode exercise the serializer. `as_json5(list)` compact JSON5 for
  the same shapes, with a nested object proving the `biscuit-file`
  formatter's single quotes and eligible unquoted keys, and output that
  parses back to the original `serde_json::Value` through the repository
  JSON5 parser. Both propagate `null`, reject non-arrays and wrong arity,
  and return `[]` for an empty array. A one-item array renders bare text
  under `as_csv` (the `prompts/commit.md` case, asserted in-library without
  touching that file).
- [ ] **Task 3.5: Expand parity, generated docs, and DMLS discovery.**
  Runtime registration/catalog parity covers `asjson`/`asjson5` aliases;
  the typed list-formatter assertion expands from six catalog entries to
  eight including their executable examples. Regenerate the function table
  via the area's just recipe (`expression_doc_generator -- --write`). In
  DMLS, add both functions to the completion name list
  (`dmls/src/providers/dsl.rs`, beside `as_line_separated` at ~line 1410)
  and expand the all-formatters completion test from six to eight; hover
  reports catalog-backed typed signatures and descriptions.

**Validation checkpoint 3:** `just test` and `just lint` in `darkmatter/`
(including the `dmls` package) green; the generated table shows both new
rows; DMLS completion contains both canonical names and hover carries typed
signatures; `cargo tree -p darkmatter -i biscuit-file` confirms the
explicit feature; new tests were shown red on the pre-fix commit.

## Phase 4: In-Repo Audit Migration

Every text-embedding site from the Task 1.3 inventory moves to an explicit
formatter chosen per-site (Ruling 3), with expected-output snapshots and
surrounding prose updated in the same change. `prompts/commit.md` is not
touched (Ruling 2).

**Work-group 4A — Darkmatter side (concurrent with 4B):**

- [ ] **Task 4.1: Migrate Darkmatter prompts, examples, and fixtures.** For
  each inventory site in `darkmatter/` (example docs, test fixtures,
  expected snapshots): apply the chosen formatter, update the expected
  output, and adjust adjacent prose that describes the rendering. Typed
  whole-value uses are retained, not converted.
- [ ] **Task 4.2: Add the typed-boundary L1 regressions.** Exact
  whole-value frontmatter interpolation stays a typed array; loop
  mutations that preserve a typed single span and typed dynamic sequence
  sources continue to return `Value::Array`; mixed strings render compact
  JSON. These pin the typed/string boundary the spec forbids widening.

**Work-group 4B — Claudine side (concurrent with 4A):**

- [ ] **Task 4.3: Migrate Claudine lifecycle and prompt sites.** Audit
  Claudine prompts/lifecycle documents that embed arrays inside surrounding
  text; assign explicit formatters where the audit says intent is prose
  (e.g. `as_unordered_list` for bullets, `as_csv` for a prose list), or
  accept compact JSON where structure is the intent. Exact whole-value
  lifecycle communication fields keep their existing compact JSON output —
  they already flow through `scalar_string` and must not be migrated as
  though they were prose.
- [ ] **Task 4.4: Add Claudine L1 lifecycle regressions.** Mixed-string
  lifecycle values that embed arrays render compact JSON (or the audit's
  explicit formatter); exact whole-value dynamic sequence sources and
  single-span loop mutations stay typed arrays.

**Validation checkpoint 4:** `just test` in both `darkmatter/` and
`claudine/` green; every inventory row is dispositioned migrated/retained
with the snapshot diff named in the verification record; `git diff` shows
no change to `prompts/commit.md`; no L2 or browser tier is invoked by new
tests.

## Phase 5: Nested-Span Suppression Retirement (D3 — Conditional)

> **Gate:** execute this phase only if Ruling 1 retained it (the nested-span
> fix landed first). If the gate is closed — the expected case at planning
> time — the phase is dropped entire and the verification record says so.

- [ ] **Task 5.1: Remove the aggregate suppression and its plumbing.**
  Delete the declared-array-or-object suggestion suppression and the type
  lookup plumbing that exists only for it, across the Darkmatter, DMLS, and
  Claudine implementations of the nested-span diagnostic. Type information
  used by any other diagnostic remains.
- [ ] **Task 5.2: Pin both aggregate cases and cross-fix agreement.**
  Nested-span diagnostics on declared array- and object-valued spans now
  *offer* suggestions (tests for both types, so the object half is not dead
  policy); the rewrite of an array-valued span is byte-identical to the
  interpolation of the same span; Claudine's lifecycle diagnostic/quick-fix
  path agrees byte-for-byte with Darkmatter and DMLS.

**Validation checkpoint 5:** Darkmatter, DMLS, and Claudine L1 green;
suppression code and its tests are absent without leaving unused type
lookups behind.

## Phase 6: Documentation, Skill Hash, and Closure

**Work-group 6A — authored documentation (concurrent with 6B):**

- [ ] **Task 6.1: Rewrite the six authored locations.** (1)
  `darkmatter/docs/topics/context-variables.md` — the bare `{{ ctx.foo }}`
  array note now states compact JSON; (2)
  `darkmatter/docs/inline/fm-interpolation.md` — drop "or rely on the
  default line-separated"; (3)
  `darkmatter/docs/topics/darkmatter-expressions.md` — the regenerated
  table row for `as_line_separated` no longer calls it the default, and
  `as_json`/`as_json5` rows are present (via the Phase 3 generator run);
  (4) `darkmatter/docs/schemas/expression-functions.yaml` — same
  description fix plus the two new entries (landed in Phase 3; verify
  wording here); (5)
  `darkmatter/features/_completed/2026-07-08-single-sourcing-schema/examples/as_line_separated.yaml`
  — remove the equivalence claim while keeping the historical record
  character of the directory intact per the spec (the completed spec, plan,
  and review records stay unchanged); (6)
  `.claude/skills/darkmatter/compose.md` (~line 318) — state the JSON
  default, the `as_json(…)` explicit spelling, and the migration path. Each
  location's migration note tells authors: newline-joined output moves to
  `as_line_separated(…)`, or to whichever explicit function matches intent.
- [ ] **Task 6.2: Correct remaining active comments and finish the search.
  ** Fix the capture comment in
  `darkmatter/lib/src/markdown/compose/context/capture/repo.rs:193` and the
  L2 chooser comment in
  `claudine/cli/tests/level2_auto_complete_chooser.rs:448`. Then run a final
  repository search for `bare array`, `bare-array`, and `line-separated by
  default`; every remaining hit must be a historical record or unrelated
  use, each reviewed and listed in the verification record rather than
  silently excluded.

**Work-group 6B — closure evidence (concurrent with 6A, after Phases 2-5):**

- [ ] **Task 6.3: Refresh the skill hash.** After the Task 6.1 edit to
  `.claude/skills/darkmatter/compose.md`, run `md hash` on the file to
  refresh its frontmatter hash; never hand-edit it.
- [ ] **Task 6.4: Assemble the verification record and PR obligations.**
  The record names: how each new test was shown red pre-fix; the
  intentionally removed/changed tests (`interpolation_output_string_*`);
  the D3 gate decision with evidence; the audit inventory with per-site
  dispositions; and the final search review. The PR description states in
  plain terms that this reverses D4 of
  `darkmatter/features/_completed/2026-07-08-single-sourcing-schema/spec.md`,
  and the change lands as its own revertible commit carrying the reversal
  in its message (author-owned; not an agent commit).
- [ ] **Task 6.5: Final graph and scope check.** Run GitNexus
  `detect_changes` (scope `all`) and confirm the changed-symbol set matches
  this plan's intended surface (evaluator redirect, deleted renderer, two
  serializers, registrations, catalog, audit sites, docs); investigate any
  unexpected entry before hand-off.

**Validation checkpoint 6 (closure):** Acceptance criteria 1-12 of the spec
each map to landed evidence; `just test` green in `darkmatter/` and
`claudine/` on macOS (Linux, native Windows, and WSL2 run the affected L1
cells through CI on the PR); the skill hash verifies clean; no L2/browser
tier was rerun beyond existing suites; historical D4 records remain
byte-identical.
