# Catalog Drift Control & Runtime-Accessible Descriptions

**Date:** 2026-06-09
**Status:** Design approved, pending spec review
**Area:** `darkmatter` (descriptor catalogs, expression engine, effects) + `claudine-cli` (context reports)

## Problem

Darkmatter exposes three typed descriptor catalogs that the `claudine context`
reports render directly:

| Subsystem | Catalog | Accessor |
|-----------|---------|----------|
| Context variables | `CONTEXT_VARIABLE_DESCRIPTORS` | `context_variable_descriptors()` |
| Expression functions | `EXPRESSION_FUNCTION_DESCRIPTORS` | `expression_function_descriptors()` |
| Side-effect capabilities | `EFFECT_DESCRIPTORS` | `effect_descriptors()` |

All three are **moving targets** — they will keep changing, and the goal is *not*
to freeze them. The goal is to make the documentation, CLI, and error surfaces
move *with* the code automatically, so drift means "docs lagging behavior" can no
longer happen silently.

Today's drift control (per `claudine/docs/topics/context/drift.md`) closes two of
three layers:

- **Layer 1 — CLI ↔ catalog:** closed structurally (the report *is* the catalog).
- **Layer 2 — catalog ↔ runtime structure:** closed by parity tests (names /
  signatures match the runtime surface, in both directions).
- **Layer 3 — catalog *content* ↔ actual *behavior*:** **partly open.** The
  human-readable `description`, the `display_type`, and the hand-written
  expression-semantics prose have no automated tie to behavior.

Three concrete open gaps:

1. **Context type labels** — `display_type` is hand-maintained; a descriptor
   could claim `Integer` while capture produces a `String` and nothing fails.
   The recent `Nullable(String)` / `Nullable(Integer)` correction was made by
   hand, not surfaced by a test.
2. **Expression semantics prose** — operator precedence, truthiness, the
   interpolation-vs-condition mode table, and null propagation are literal
   arrays in `claudine/cli/src/commands/context.rs`, derived from no Darkmatter
   type. A change to the evaluator would not update or break them.
3. **Effect orphan methods** — a public `EffectEngine` method with no descriptor
   is silently outside the capability surface; the choice is unasserted.

Separately, none of the descriptor data is reachable at **runtime**, so error
messages cannot draw on the same descriptions/examples the docs use.

## Goals

1. Take Layer-3 drift control to the next level across all three subsystems,
   with content **anchored to observable behavior** rather than trusted prose.
2. Make descriptions/examples **runtime-accessible** so error messages improve —
   specifically "did-you-mean" suggestions with a verified example.

## Non-Goals

- Freezing the catalogs or preventing change. They are expected to move.
- Verifying description *wording* (examples anchor behavior, not phrasing).
- Enumerating a Rust type's public method surface at compile time (impossible;
  the effects orphan gap is handled by a reviewed allow-list, not enforcement).

## Decisions (locked during brainstorming)

| Axis | Decision |
|------|----------|
| Ambition | **Unifying framework** — a shared descriptor abstraction all three catalogs adopt. |
| Anchor | **Uniform executable examples** — uniform example *data*; each subsystem executes against its own runtime. |
| Error depth | **Did-you-mean + verified example**, across all applicable surfaces. |
| Semantics prose | **Fully promoted** into typed catalogs (parser anchored). |
| Framework shape | **Approach A** — pure-data trait + per-catalog execution harness (no runtime coupling in the shared module). |

---

## Section 1 — Shared framework (`darkmatter/lib/src/catalog/`)

A new leaf module, deliberately dependency-light: **no** coupling to the
evaluator or `EffectEngine`, so it cannot reintroduce an import cycle.

### `Described` trait — the shared descriptor surface

```rust
pub trait Described {
    fn key(&self) -> &'static str;        // ctx var name, or fn/effect/operator signature
    fn description(&self) -> &'static str;
    fn category(&self) -> &'static str;
    fn order(&self) -> usize;
    fn example(&self) -> Option<Example>;
}
```

### `Example` — uniform example data

```rust
pub struct Example {
    pub invocation: &'static str,  // "upper(\"hi\")"  ·  "ctx.cpu_cores"  ·  "ensure_file(\"a.md\")"
    pub result:     &'static str,  // "\"HI\""          ·  "8"              ·  "absolute path"
}
```

Both fields are `&'static str` so descriptors remain `const`. `Example` is
**display/error data only**. Whether and how it executes belongs to the
subsystem (Section 2); that separation is what keeps this module free of the
three runtimes.

### Accessor — one API over any catalog

```rust
pub fn describe<'a, T: Described>(catalog: &'a [T], key: &str) -> Option<&'a T>;
pub fn suggest<'a, T: Described>(catalog: &'a [T], key: &str, max: usize) -> Vec<&'a T>;
```

- `describe` — exact lookup by `key`.
- `suggest` — fuzzy nearest-match ranked by edit distance, matching on the
  **name portion** of a key (strips `(…)` so `uper` finds `upper(x)`).

### Fuzzy matching

A ~30-line **in-crate Levenshtein** implementation rather than adding `strsim`,
respecting the monorepo's dependency discipline; the need is trivial.

### Error-snippet formatter

```rust
pub fn describe_for_error(d: &dyn Described) -> String; // plain text, no ANSI
```

Renders `signature → description → e.g. example` as plain text. The library
error layer stays style-free; the CLI re-styles. Identical wording across every
surface (errors, `context` reports, narrative doc) flows from this one place.

---

## Section 2 — Per-subsystem adoption & executable anchors

Each existing descriptor struct gains `example: Option<Example>` and an
`impl Described`. The example *data* is uniform; the executable *anchor* is local
to each runtime.

### Context variables — anchor: type-shape parity

The highest-value gap to close.

- New `cfg(test)` `capture_value_shape_matches_display_type`: capture a
  `ComposeContext` once, then for every descriptor assert the captured
  `serde_json::Value` matches `display_type`:
  - `Nullable(T)` → `null` **or** a `T`-shaped value.
  - `Csv` / `MarkdownList` / `NestedMarkdownList` → `String`.
  - `Object` → object; `Integer`/`Number`/`Boolean`/`String`/date-family → their
    JSON shapes.
  This would have caught the `Nullable(String)` / `Nullable(Integer)` bug
  automatically.
- `Example.result` for a context variable is an **illustrative** value
  (e.g. `cpu_cores → "8"`), not the live value — live values are
  environment-dependent and cannot be asserted equal. A second cheap check
  asserts each illustrative `result` is **type-consistent** with `display_type`,
  so an example cannot drift into misrepresenting the type.

### Expression functions — anchor: real evaluation

The cleanest case.

- New `cfg(test)` `every_example_evaluates_to_its_declared_result`: parse
  `example.invocation`, run it through the real `evaluate` pipeline, assert the
  rendered output equals `example.result`. A renamed handler or a wrong result
  fails the build.
- **Pure functions** execute directly.
- **Filesystem functions** (`frontmatter`, `file_exists`, …) need a
  `ResolutionContext`; the harness prepares a small tempdir fixture (a known
  `.md` plus a known file) and their examples reference those fixtures. Any fs
  function not cheaply pinnable to a fixture is marked **display-only** (example
  shown, not executed) — an explicit, visible opt-out, never a silent gap.

### Side effects — anchor: sandbox engine (reuse existing)

- The `cfg(test)` `EFFECT_VERBS` closures remain (they need a live sandbox
  `EffectEngine` and cannot be const strings) — they are the executable side.
  Each descriptor's new `example` is the display/error side. The existing
  `verb_signature_set_equals_descriptor_signature_set` keeps verb↔descriptor
  paired; add a check that every descriptor carries an example and that its
  `result` text is consistent with the verb's return kind.
- **Orphan-method allow-list** (`drift.md` step 3): an explicit
  `const INTENTIONALLY_UNCATALOGUED: &[&str]` naming `EffectEngine` methods
  deliberately outside the capability surface. **Honest caveat:** Rust cannot
  enumerate a type's methods at compile time, so this *documents and reviews*
  the decision — it converts a silent omission into a named, reviewed list,
  which is the most achievable here.

### Deliberate non-change

Unknown `ctx.*` access stays **silent null** — null propagation is intentional
per the expression docs. Context "did-you-mean" therefore lands on a diagnostic
surface (Section 4), never the silent runtime path; a deliberately-absent
variable used with `|| fallback` keeps working untouched.

---

## Section 3 — Promoted expression-semantics catalogs

The literal arrays in `context.rs` (precedence, truthiness, mode table, null
propagation) become typed Darkmatter catalogs, rendered by the CLI (Layer 1) and
anchored to the real lexer/evaluator (Layer 2).

### New catalogs in `expression/semantics.rs`

```rust
pub struct OperatorDescriptor {
    pub symbol: &'static str,        // "*", "&&", "?:"
    pub precedence: u8,              // 1 = tightest
    pub arity: OperatorArity,        // Unary | Binary | Ternary
    pub associativity: Associativity,
    pub modes: ModeAvailability,     // Interpolation | Condition | Both
    pub description: &'static str,
    pub example: Option<Example>,
}
```

Plus `TRUTHINESS_DESCRIPTORS` (value-kind → falsy/truthy + example),
`MODE_DESCRIPTORS` (`||`/`&&` meaning per parse mode), and
`NULL_PROPAGATION_DESCRIPTORS` (rule + example such as `ctx.missing.x → null`).
All four `impl Described`, flowing through the same accessor and renderer.

### Layer-2 anchor — the parser

"Fully promoted" means precedence has a single source of truth the way
`PURE_FUNCTIONS` does for functions. Two realizations:

1. **Assert-equal (start here).** Expose the parser's precedence table as a
   `pub(crate) const` and add a test asserting it equals `OPERATOR_DESCRIPTORS`.
   Same single-source guarantee in practice; minimal surgery.
2. **Full rewire.** The precedence-climbing parser consumes `OPERATOR_DESCRIPTORS`
   directly, so no second copy exists.

**Plan: implement realization 1 first**, escalate to 2 only if you want the
parser to literally hold no copy. Both close the drift gap; the difference is one
place vs. two-kept-equal.

Either way, a `cfg(test)` harness proves the catalog against observable behavior
via curated evaluation examples: `1 + 2 * 3 == 7` (multiplicative tighter than
additive), `!a && b` (unary tighter than AND), mode examples (`||` as fallback in
`{{ }}` vs boolean in `when=""`), truthiness examples (`""`, `[]`, `0` falsy).
Each `OperatorDescriptor.example` and `TRUTHINESS_DESCRIPTOR` is executed, so a
precedence value or truthiness row that contradicts the evaluator fails the
build.

**Risk note:** the parser change is the highest-risk part of the effort — the
only place that modifies evaluator internals rather than adding tests beside
them. Starting with assert-equal contains that risk.

### CLI

`render_expressions_precedence`, `_truthiness`, `_mode`, `_null_propagation`
switch from literal arrays to catalog iteration, mirroring the function table.

---

## Section 4 — Error-message enrichment wiring

The accessor is built once; how much each surface honestly gains differs.

### Expression functions — full payoff (headline)

- `evaluate`'s `Unknown function: <name>` path calls `suggest(...)` and appends
  the nearest match's signature, description, and verified example:

  ```
  Unknown function: uper
    did you mean upper(x)?
    Converts a string to uppercase.
    e.g. upper("hi") => "HI"
  ```

- Arity errors call `describe(...)` on the matched name and append the correct
  signature + example — a self-correcting message.

### Context variables — diagnostic, not runtime error

- Unknown `ctx.*` stays silent-null at evaluation time (null propagation
  preserved). The did-you-mean surfaces as a **compose-time lint/diagnostic**
  that scans `{{ }}` and `when=""` for `ctx.<name>` references absent from the
  catalog and warns with the nearest match. Opt-in (warning stream), never a
  hard failure.

### Side effects — least runtime reach (stated honestly)

- Effects are typed methods with **no string dispatcher**, so there is no
  "unknown effect" runtime error to enrich. The accessor pays off here for
  **documentation**: the `--side-effects` report gains an `Example` column, and
  any future string-dispatched effect surface gets enrichment for free. No
  runtime error-gain is promised for effects.

All three use the shared `describe_for_error` formatter (Section 1).

---

## Section 5 — Narrative doc, recast drift model, layout & phasing

### Narrative-doc parity (cuttable)

`darkmatter/docs/topics/darkmatter-expressions.md`'s function-table section is
generated from `EXPRESSION_FUNCTION_DESCRIPTORS` and pinned by a `cfg(test)`
`narrative_doc_function_table_matches_catalog` (fails CI on drift). A `just`
recipe (`just darkmatter regen-expr-doc`) or hidden CLI flag regenerates it.
**This is the most cuttable component** — drop it without affecting the rest if
coupling a prose doc to a test is unwanted.

### Recast drift-layer model

| Surface | Before | After |
|---------|--------|-------|
| Context `display_type` accuracy | hand-corrected | `capture_value_shape_matches_display_type` |
| Expression function content | trusted prose | executable examples vs `evaluate()` |
| Expression semantics (precedence/truthiness/mode) | literal arrays in `context.rs` | typed catalogs, parser assert-equal, executed anchors |
| Descriptions ↔ behavior | free text | example presence + type/result consistency |
| Narrative doc | fully manual | catalog-parity test |

**Remaining honest gaps:** description *wording* is still trusted; effect
orphan-methods remain a reviewed allow-list, not compile-time-enforced;
fs-function display-only examples are unexecuted by design.

### Module layout

- `darkmatter/lib/src/catalog/mod.rs` *(new)* — `Described`, `Example`,
  `describe`, `suggest`, `describe_for_error`, in-crate Levenshtein.
- `context/catalog.rs`, `expression/catalog.rs`, `effects/catalog.rs` — add
  `example`, `impl Described`, new anchor tests.
- `expression/semantics.rs` *(new)* — operator/truthiness/mode/null-propagation
  catalogs.
- `expression/parser.rs` — expose precedence as `pub(crate) const` for
  assert-equal.
- `claudine/cli/src/commands/context.rs` + `context_render.rs` — render
  semantics from catalogs; add Example column to reports.
- Error paths: `expression/mod.rs` (evaluate); a compose-time lint for ctx typos.

### Implementation phasing

One cohesive feature, six ordered, independently shippable phases — each leaves
the build green:

1. **Framework core** — `catalog/` module, trait, `Example`, accessor,
   Levenshtein, formatter. Pure, fully unit-tested, no consumers yet.
2. **Adopt + anchor the three catalogs** — `impl Described`, add examples, add
   the three executable-anchor harnesses (context type-parity, expression
   example-eval, effects example consistency + orphan allow-list).
3. **Promote semantics** — new `semantics.rs` catalogs + assert-equal anchor +
   CLI rendering.
4. **Error enrichment** — wire `evaluate` unknown/arity, the ctx-typo lint, the
   `--side-effects` Example column.
5. **Narrative-doc parity** (cuttable).
6. **Recast `drift.md`** and the three `context/` topic docs to describe the
   closed gaps.

## Open questions for spec review

- **Narrative-doc parity (Phase 5):** keep or cut?
- **Parser anchor:** confirm starting with assert-equal (realization 1) rather
  than full rewire.

## Success criteria

- All three catalogs implement `Described`; the shared accessor (`describe`,
  `suggest`) is unit-tested and reachable at runtime.
- Context: `capture_value_shape_matches_display_type` passes and would fail on a
  reintroduced `Nullable` type mismatch.
- Expression: every executable example evaluates to its declared result; the
  semantics catalogs are parser-anchored (assert-equal) and CLI-rendered.
- Effects: every descriptor carries an example; orphan allow-list documented.
- `evaluate` unknown-function and arity errors show did-you-mean + verified
  example; the ctx-typo compose-time lint warns with nearest match.
- `drift.md` and the three topic docs updated to reflect the closed gaps.
- Build green at every phase boundary.
