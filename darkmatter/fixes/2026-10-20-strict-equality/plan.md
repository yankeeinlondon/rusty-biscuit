---
total_phases: 6
created: 2026-10-20
phase: 1
agent: "opencode/zai-coding-plan/glm-5.3"
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation Plan — Strict equality for `==` and `!=`

Derived from the reviewed specification in `2026-10-20-strict-equality` (status:
`draft-spec`, reviewed, no open questions). File references below are grounded in
the current tree.

## Work Summary and Success Criteria

Darkmatter's expression evaluator decides `==` and `!=` by rendered text
(`scalar_string`) with a both-null exception
(`lib/src/markdown/compose/expression/mod.rs:599-623`), and the guard analysis
(`scope/facts.rs`, `scope/shape.rs`) narrows types by the same text rules. This
plan replaces both with one strict, typed equality — same runtime type and same
value, recursively through containers, with exact numeric value equality
(integer/float storage unified without loss beyond 2^53) — and migrates every
authored expression, frozen prompt copy, doc, and skill that relied on the old
contract. Ordering operators, arithmetic, truthiness, rendering, schema
validation, and the grammar are untouched; no compatibility mode is added.

The work splits into six phases: (1) groundwork, rulings, and the audit
inventory; (2) the evaluator change plus its independent acceptance tests and
the semantics catalog; (3) the guard-analysis rework that must land with it;
(4) consumer-surface verification (shell preflight, body blocks, transclusion
targets, absence warnings, DMLS, Claudine lifecycle/sequence, composition); (5)
migration of authored expressions and frozen copies; (6) documentation, skill
updates, and the final gate.

Successful completion looks like:

- Every row of the spec's decision table holds through the public expression
  API (`parse` + `evaluate`), for both operand orders, including `null == null`
  → `true`, `5 == "5"` → `false`, recursive container equality, object key
  order independence, missing-key-vs-`null` distinction, integer/float pairs,
  signed/unsigned boundaries, negative zero, and the distinct large integers
  `9007199254740992` / `9007199254740993` supplied as typed values (not
  literals). `!=` is the exact complement for successful comparisons;
  reflexivity and symmetry hold over the sample-pair corpus.
- The differential scope tests
  (`lib/tests/l1/expression_type_scope.rs`) stay green with updated
  expectations: every runtime branch admits the values the evaluator sends down
  it, and the analyzed presence facts match the spec's guard table (`x != null`
  proves presence; `x == ''` proves the empty string; `x == null` proves null).
- `x != ''` no longer authorizes a missing shell placeholder or nullable
  transclusion target; `x != null` does. DMLS reports the same facts statically
  (no execution). Claudine lifecycle and sequence conditions behave identically
  through their existing normal invocation tests.
- The motivating commit-message prompt (`ctx.dirty_files && commit_message`,
  unchanged) and both frozen copies pass preflight with the spec's
  generated-vs-supplied route behavior.
- The migration audit — inspected locations, rewritten sites, and intentionally
  retained comparisons with reasons — is recorded in this fix's
  `implementation-log.md`.
- `just test` and `just lint` pass in both `darkmatter/` and `claudine/`, and
  the affected opt-in prompt checks (`just test-prompts`) pass locally. No new
  CI matrix, terminal/browser tier, benchmark suite, or OS-specific behavior is
  introduced.

## Phase 1 — Groundwork, Rulings, and Audit Inventory

Establish the baseline, settle the few implementation defaults the spec leaves
open, and produce the migration worksheet before any behavior changes.

### Necessary Rules

The spec is reviewed with no open questions, so no new author rulings are
required. The following defaults are adopted and are the only places an
implementer may not decide alone — flag to the author if any proves wrong
rather than silently deviating:

- **Comparator placement.** One shared typed comparison lives in a new
  `equality` submodule of `lib/src/markdown/compose/expression/` (e.g.
  `equality.rs` with `pub(crate) fn values_equal(&Value, &Value) -> bool`),
  called by the evaluator's comparison branch and by the scope analysis. No
  separate Claudine, editor, or test-side comparator is added (spec: "keep one
  typed comparison implementation in darkmatter").
- **Numeric literal narrowing breadth.** When `x == <number literal>` cannot
  pin a literal shape that schema validation accepts for equivalent numeric
  storage (e.g. `literal(4)` rejecting an f64-backed `4.0`), the broader number
  arm is retained rather than schema validation changed. The differential suite
  is the arbiter of the exact outcome; the analyzer must never reject a live
  value the evaluator sends down a branch.
- **No performance spike.** The spec states none is required; structural
  comparison stops at the first mismatch and never allocates serialized copies.
  Any wider measurement is out of scope.
- **No input-robustness matrix.** This changes an expression evaluator — a text
  grammar — not a file-format or configuration reader, so the matrix section
  does not apply and no cells are written.
- **Corpus checks stay opt-in.** The prompt-expression corpus gate
  (`lib/tests/prompts/dasherized_identifier_corpus.rs`, `just test-prompts`)
  is parse-only and is never newly added to CI; migration correctness is proven
  by compose and normal-invocation tests plus local opt-in runs (spec
  acceptance 6/7).
- **Out of scope, enforced.** Ordering operators keep `to_number_coerce`;
  arithmetic, truthiness, fallback, rendering, schema literal validation, the
  grammar, JSON Schema `const`/`enum`, and Rust-native comparisons are not
  touched. Historical specs are never rewritten in the audit.

### Wave 1 — Baseline and inventory (concurrent)

- [x] **Baseline verification**
    - Run `just test` and `just lint` in `darkmatter/` and in `claudine/`;
      record the pre-change state so later failures are attributable.
    - Confirm the workspace resolves both package areas (`sniff repo
      package-areas` if there is any doubt about the claudine pairing).
- [x] **Audit inventory worksheet**
    - Enumerate every authored `==` / `!=` site in: root `prompts/` (including
      `feature.md`, `_prompt.md`, `_implement/*`, `_pr/*`, `fix.md`,
      `_interactive-prompting.md`, `_reviews/*`, `code-comment-quality.md`),
      `.claude/commands/`, `darkmatter/prompts/`, `claudine/prompts/`,
      `.claude/skills/` expression-teaching pages, schema examples, test
      fixtures, and documentation examples.
    - Classify each site as *keep* (exact typed comparison is the intent, e.g.
      `stage == 'tech-design'`, `ctx.branch != base`), *rewrite* (choose from
      the spec's migration table: `is_null(x)`, `x != null`,
      `is_null(x) || x == ''`, `is_empty(x)`, `number(...)`,
      `('' + x) == ('' + y)`), or *verify* (behavior-sensitive, e.g.
      `frontmatter(review, 'recurrence') == true`, `phase == total_phases`).
    - Include variable-to-variable comparisons, booleans against strings, and
      containers against JSON text — not only `''`/`null` checks.
    - Cover both frozen copies:
      `claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md`
      and
      `.../frozen_prompts/shipped_implement_route/_implement/implement-plan.md`.
    - Mark the commit-message guard `ctx.dirty_files && commit_message` as
      intentionally unchanged (truthiness; regression case per spec).
    - Deliverable: the worksheet that Phase 5 executes and that the
      implementation log later records.

### Validation checkpoint

- Both package areas are green before any edit; the worksheet accounts for
  every match found by the sweep, and every row carries a keep/rewrite/verify
  decision.

## Phase 2 — Strict Equality in the Evaluator

Implement the typed comparator, rewire the comparison branch, and prove the new
contract through tests whose expected results are written independently of the
comparator.

### Wave 2 — Shared comparator (blocking)

- [ ] **Comparator module**
    - Create `equality.rs` under
      `lib/src/markdown/compose/expression/` exposing one shared
      `values_equal(&Value, &Value) -> bool` (early-exit, allocation-free).
    - Scalars: `null` equals `null` only; booleans and strings compare exactly
      (no trimming, case folding, or Unicode normalization).
    - Numbers: compare represented numeric values exactly — i64/i64 and
      u64/u64 losslessly; i64 vs u64 without loss (a negative i64 never equals
      any u64); integer/float pairs equal only when the float represents that
      exact integer within its range; `5` == `5.0`; `0` == `-0.0`; no
      tolerance; never convert all numbers through f64 (must keep
      `9007199254740992` distinct from `9007199254740993` when supplied as
      typed values).
    - Arrays: equal lengths, then corresponding elements recursively. Objects:
      same key sets, then equal values per key recursively, independent of key
      order; a missing key differs from a key holding `null` (falls out of key
      set equality).
    - No `to_string()`, `serde_json` serialization, or intermediate
      collections.

### Wave 3 — Evaluator, catalog, tests (concurrent)

- [ ] **Comparison branch rewire**
    - In `mod.rs` (`Expr::Comparison`, lines ~599-623): `Equal` →
      `values_equal(&left, &right)`; `NotEqual` → `!values_equal(...)`; delete
      the `both_null` exception. Ordering arms keep `to_number_coerce`
      untouched.
    - Preserve operand evaluation order and error propagation exactly (an
      unavailable host binding, malformed expression, or failed function call
      remains an error, never a value).
    - Update the inline evaluator tests (~lines 1275-1340) and any
      `expression_regression.rs` rows that assert text equality.
- [ ] **Semantics catalog update**
    - `semantics.rs`: rewrite the `COMPARISON_OPERATOR_DESCRIPTORS` rows
      ("compare scalar string representations", the `"5" == 5` example, the
      both-null and one-side-missing rows) and the comparison row's description
      in `OPERATOR_DESCRIPTORS` to state strict typed equality with executable
      examples (`5 == "5"` → `false`, `null == null` → `true`,
      `5 == number("5")` → `true`). These descriptors are user-facing via
      `claudine context --expressions`; their example-verification tests must
      pass.
- [ ] **Acceptance-pair test suite**
    - New/extended tests at L1 asserting the spec's decision tables verbatim
      through the public `parse` + `evaluate` API, with expected results
      hard-coded (never computed via `values_equal`): both operand orders;
      missing document value vs explicit `null`; booleans/numbers/strings;
      empty and nested containers; object key order; missing keys vs null
      values; integer/float pairs; signed/unsigned boundaries (i64::MIN/MAX,
      u64 above i64::MAX via a data lookup); negative zero; and
      `9007199254740992` vs `9007199254740993` supplied as typed values
      through the test lookup, not literals, so parser rounding cannot hide a
      defect.
    - Property loop over the sample-pair corpus: reflexivity, symmetry, and
      `!=` as the exact complement for successfully evaluated pairs.

### Validation checkpoint

- `just test` in `darkmatter/` shows only Phase 3 analysis failures
  (differential scope rows); every evaluator-level equality test is green and
  no ordering/arithmetic/truthiness behavior changed.

## Phase 3 — Guard Analysis Rework

Replace the text-based narrowing in `scope/facts.rs` and `scope/shape.rs` with
strict-value rules, so analysis and evaluator agree and the differential suite
is green again. This must land with Phase 2 semantics (the spec directs the
evaluator and analyzer rules be replaced together); landing either alone leaves
shared tests red.

### Wave 4 — Facts and shapes (single chain)

- [ ] **Equality narrowing rewrite**
    - `scope/facts.rs` `comparison`/`compare_with`/`against_operand`/
      `against_values`: implement the spec's fact table — `x == null` true →
      null only, false → present (non-null); `x != null` mirrored; `x == ''`
      true → empty string, false → no general presence proof; `x == 'a'`
      likewise; reversed operands and negated guards produce the corresponding
      facts. Remove the both-null special routes entirely.
    - `x == y` between subjects: on the equal side retain every declared
      alternative that could strictly equal a possible other-side value; a
      singleton `null` is now a usable finite set. On the unequal side,
      narrow only against a single-value finite set, with the complement now
      exact.
    - Equality against an array/object literal or finite container value
      refines to present + that container family (full container-value
      narrowing not required).
    - Unknown or non-finite other-side types stay conservative; never evaluate
      a function, read a file, or fetch data for a static fact.
    - `scope/shape.rs`: replace `text_equal`/`text_differs` (and
      `number_with_text`, `arm_text_equal`, `arm_text_differs`) with
      strict-value primitives — literals pinned by typed equality, enums and
      booleans narrowed by typed membership, numbers pinned only when the
      pinned shape admits equivalent storage (else the broader number arm is
      retained per the Phase 1 ruling), containers pinning family + presence.
- [ ] **Literal value modeling**
    - `literal_operand`/`number_operand` in `facts.rs` must mirror the
      evaluator's actual literal production, including the integral-to-i64
      conversion (`mod.rs` ~line 516), so the analyzer models the value the
      evaluator really compares.

### Wave 5 — Analysis tests (concurrent)

- [ ] **Scope test rows**
    - Update `scope/tests.rs` (in-module) and
      `lib/tests/l1/expression_type_scope.rs` rows to the new facts — e.g.
      `optional == null` true side becomes `null` (not `literal('')`),
      `optional != null` true side `string`, `either == '4'` no longer keeps a
      numeric arm, and `two_nulls_are_neither_equal_nor_unequal` inverts
      (`null == null` true; `x != y` false side admits both nulls). Keep the
      contradictory-guard conservatism (no dead-code elimination).
- [ ] **Shared guard cases**
    - Update `lib/tests/common/type_scope_cases.rs` and
      `lib/tests/l1/scope_site_conformance.rs` (and the DMLS ownership-guard
      twins if allowlists shift) for the new equality facts.
- [ ] **Differential extension**
    - Extend `samples()` and `GUARDS` in `expression_type_scope.rs` with
      container deep-equality pairs, int/float equivalent pairs, `-0`, large
      typed integers, and null-vs-'' pairs, and add container-family
      refinement cases; every guard × declaration row must keep admitting
      everything the evaluator sends (acceptance 2), including both sides of
      each presence guard and finite other-side values including null.

### Validation checkpoint

- `just test` in `darkmatter/` is fully green again (evaluator + analysis
  aligned); grep confirms no `scalar_string`-based narrowing remains in
  `scope/` and no `both_null` routes remain anywhere.

## Phase 4 — Consumer Surfaces and Claudine

Prove the shared analysis reaches every consumer with the new facts, through
the consumers' own public entry points.

### Wave 6 — Consumer verification (concurrent)

- [ ] **Preflight and placeholders**
    - Extend preflight collection tests (`lib/src/markdown/compose/preflight/`
      collect paths) and `AbsenceScope` branch-fact tests: `x != ''` must not
      excuse a missing shell placeholder; `x != null` must supply the presence
      evidence that authorizes it. Preserve undeclared-property warning
      handling and scope invalidation after writes.
- [ ] **Blocks and transclusion**
    - `page_blocks/guards.rs` (`BodyGuards`) and `directive_targets.rs`
      (nullable-target classification, `narrowed_paths`): extend tests for both
      sides of each presence guard; nullable transclusion targets are excused
      by `x != null`, never by `x != ''`.
- [ ] **DMLS static twin**
    - DMLS diagnostics and hover (`dmls/src/overlay/expressions.rs`, hover via
      `expression::branch_scope`) must display the same facts without
      executing conditions; add/extend tests asserting the narrowed hover text
      and diagnostics for the new guard table, including container-family
      refinement.
- [ ] **Claudine conditions**
    - Claudine lifecycle and sequence conditions (they consume darkmatter's
      evaluator; no new evaluator): verify through existing normal invocation
      tests. Add the acceptance-5 regression: with dirty files, null/empty
      `commit_message` selects the generated-message route and a nonempty
      message the supplied-message route (a harmless command substitute
      receives the message as one argument); with no dirty files neither
      commit route runs; a false branch launches no command from that branch.
- [ ] **Composition-level checks**
    - Representative changed comparisons through ordinary composition:
      frontmatter and body interpolation, ternaries, `::block when`,
      conditional transclusion (acceptance 4). Use frozen fixtures and the
      test toolkit; tests must not launch an agent, commit, stage files, or
      activate a window. Preserve operand errors and skipped-branch behavior.

### Validation checkpoint

- `just test` and `just lint` in both `darkmatter/` and `claudine/` pass with
  the new consumer expectations; no consumer grew its own equality logic
  (grep for stray `== scalar_string` narrowing outside `equality.rs` uses).

## Phase 5 — Migration of Authored Expressions

Execute the Phase 1 worksheet on shipped artifacts. Historical specs are
snapshots and are never rewritten; the audit result lands in this fix's
implementation log.

### Wave 7 — Shipped artifacts (concurrent)

- [ ] **Root prompts rewrite**
    - Apply the worksheet to root `prompts/` (all `_implement/*`, `_pr/*`,
      `_reviews/*`, `_prompt.md`, `feature.md`, `fix.md`,
      `_interactive-prompting.md`, `code-comment-quality.md`, and any other
      audited site) using the spec's migration table and each document's
      declared intent. Keep `ctx.dirty_files && commit_message` and every
      *keep*-classified exact comparison unchanged.
- [ ] **Area prompts, skills, schemas, fixtures**
    - Apply the worksheet to `darkmatter/prompts/`, `claudine/prompts/`,
      `.claude/commands/`, `.claude/skills/` teaching examples, schema
      examples, and test fixtures. For intentional rendered-text comparison
      use `('' + x) == ('' + y)`; for numeric comparison against text use
      `number(...)`; keep `== ''` / `!= ''` where exact string comparison is
      the intent.

### Wave 8 — Frozen copies and log (after Wave 7 decisions)

- [ ] **Frozen copy sync**
    - Migrate both frozen copies of `_implement/implement-plan.md` (under
      `claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/` and
      `.../shipped_implement_route/_implement/`) together with their original;
      where the original needed no change, verify the copy still mirrors it.
      Re-run the frozen-prompt preflight tests.
- [ ] **Implementation log**
    - Record in `implementation-log.md` (this fix's directory): every
      inspected location, every rewritten site, and every intentionally
      retained comparison with its reason, plus the Phase 1 worksheet outcome
      and any departure from the spec (none expected).

### Validation checkpoint

- The shipped-artifact corpus tests pass passively; representative migrated
  prompts compose correctly through normal invocation; `just test-prompts`
  (root and `claudine/`) passes locally and was not added to CI.

## Phase 6 — Documentation, Skills, and Final Gate

Docs describe current behavior, never link back to the spec, and are updated
with the implementation (not before). Detailed rules stay in topic pages;
READMEs stay overviews.

### Wave 9 — Documentation (concurrent)

- [ ] **Expression reference update**
    - `darkmatter/docs/topics/darkmatter-expressions.md`: rewrite the
      "Equality and Inequality" section and the "What a Guard Proves" table
      for strict equality; update nullable-target examples; add strict type
      examples, recursive equality, explicit conversions (`number("5")`,
      `('' + x) == ('' + y)`), the changed presence checks, and the explicit
      distinction that ordering still coerces (`5 <= "5"` and `5 >= "5"` both
      hold while `5 == "5"` is false).
- [ ] **Engine guide update**
    - `darkmatter/docs/topics/expression-engine.md`: align its type-aware
      narrowing narrative and any equality mentions with the new guard facts.
- [ ] **Skill page update**
    - `.claude/skills/darkmatter/expressions.md` (the "Equality follows the
      evaluator" paragraph and guard description) and any affected preflight,
      Claudine lifecycle/sequence, and DMLS hover pages that teach the former
      rules.
- [ ] **README sweep**
    - Check the darkmatter (and claudine, where relevant) READMEs for examples
      presenting changed behavior; update them, keeping detailed rules in the
      topic pages.

### Wave 10 — Final verification gate

- [ ] **Full gate**
    - Run `just test` and `just lint` in `darkmatter/` and `claudine/`, and
      the affected opt-in prompt checks locally (`just test-prompts`).
    - Confirm scope discipline: no new CI matrix cell, terminal/browser tier,
      benchmark suite, or OS-specific comparison behavior was added; no change
      to JSON Schema `const`/`enum` validation, Rust comparisons, or unrelated
      template languages; the grammar, truthiness, arithmetic, ordering, and
      rendering behavior are unchanged.

### Validation checkpoint

- All six acceptance criteria of the spec are demonstrably met; the plan's
  checkboxes are complete; the fix is ready for the author's review
  (the author, not the agent, moves it to `_completed`).
