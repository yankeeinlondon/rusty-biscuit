---
created: 2026-06-09
reviewed: true
status: ready for planning and implementation
area:
  - darkmatter
  - claudine-cli
---

# Catalog Drift Control & Runtime-Accessible Descriptions

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
3. Preserve the current `claudine context` reporting contract: reports render
   from Darkmatter's typed catalogs, stay documentation-only for side effects,
   and remain within the existing 140-column layout contract.
4. Keep parsing, catalog ownership, and plain diagnostic text in Darkmatter;
   keep terminal styling and command-specific presentation in Claudine.

## Non-Goals

- Freezing the catalogs or preventing change. They are expected to move.
- Verifying description *wording* (examples anchor behavior, not phrasing).
- Enumerating a Rust type's public method surface at compile time (impossible;
  the effects orphan gap is handled by a reviewed allow-list, not enforcement).
- Adding a string-dispatched side-effect execution surface.
- Changing expression evaluation semantics. Any promoted semantics catalog must
  describe the current parser/evaluator behavior unless a separate spec
  intentionally changes that behavior.
- Duplicating Darkmatter expression parsing or context-variable lookup logic in
  Claudine. Claudine may render diagnostics, but it must not become a second
  parser or second semantic authority.
- Introducing runtime host probes, filesystem mutation, or network access while
  reading descriptor catalogs or rendering `claudine context` reports.

## Decisions (locked during brainstorming)

| Axis | Decision |
|------|----------|
| Ambition | **Unifying framework** — a shared descriptor abstraction all three catalogs adopt. |
| Anchor | **Uniform example model** — executable where deterministic, type-shape checked for context, and explicitly display-only where execution would be misleading or unstable. |
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

**Design note:** `Described` is a read-only display/search contract, not a
replacement for subsystem-specific descriptor types. It intentionally exposes
only the fields shared by every catalog. Existing fields such as
`ContextVariableDescriptor::subsection`, `ContextVariableDescriptor::display_type`,
and `EffectDescriptor::safety` remain on their concrete types and keep driving
their specialized reports.

### `Example` — uniform example data plus verification intent

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Example {
    pub invocation:   &'static str,          // "upper(\"hi\")"  ·  "ctx.cpu_cores"  ·  "ensure_file(\"a.md\")"
    pub result:       &'static str,          // "\"HI\""          ·  "8"              ·  "absolute path"
    pub verification: ExampleVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExampleVerification {
    Executable,
    TypeShapeOnly,
    DisplayOnly(&'static str),
}
```

Fields are `&'static str`/`Copy` so descriptors remain `const`. `Example` is
**display/error data plus verification intent**. Whether and how executable
examples run still belongs to the subsystem (Section 2); that separation keeps
this module free of the three runtimes while making display-only opt-outs
explicit data instead of comments hidden beside tests.

Examples use the same textual rendering that the relevant user-facing surface
uses:

- Expression examples compare against Darkmatter expression output rendering,
  not `Debug` formatting.
- Context examples compare only for shape/type consistency unless the value is a
  fixed literal.
- Effect examples are descriptive display examples unless explicitly paired with
  a sandbox execution assertion.

`DisplayOnly(reason)` is allowed, but never silent. Tests assert that every
display-only example carries a non-empty reason, and reports may choose to hide
the reason from end users while keeping it available to maintainers.

### Accessor — one API over any catalog

```rust
pub fn describe<'a, T: Described>(catalog: &'a [T], key: &str) -> Option<&'a T>;
pub fn suggest<'a, T: Described>(catalog: &'a [T], key: &str, max: usize) -> Vec<&'a T>;
```

- `describe` — exact lookup by `key`.
- `suggest` — fuzzy nearest-match ranked by edit distance, matching on the
  **name portion** of a key (strips `(…)` so `uper` finds `upper(x)`).

Tie-breaking is deterministic: lower edit distance first, then lower catalog
`order`, then lexical `key`. `max == 0` returns an empty list.

Suggestion quality gate: omit suggestions whose normalized name distance is
greater than `max(2, normalized_query.len() / 3)`. This avoids confident-looking
"did you mean" output for unrelated short typos while still catching common
one- and two-character mistakes.

### Fuzzy matching

A ~30-line **in-crate Levenshtein** implementation rather than adding `strsim`,
respecting the monorepo's dependency discipline; the need is trivial.

Suggestion matching normalizes only the lookup token, not the public display
key: trim whitespace, strip a leading `ctx.` for context-variable suggestions,
and strip the parenthesized argument list from callable signatures. It does not
case-fold or rewrite separators; current symbols are already lower snake_case
and preserving that keeps suggestions predictable.

The implementation must be Unicode-safe even though catalog keys are ASCII:
iterate over `char`s rather than bytes so a user typo containing non-ASCII text
does not panic or split a UTF-8 sequence.

### Error-snippet formatter

```rust
pub fn describe_for_error(d: &dyn Described) -> String; // plain text, no ANSI
```

Renders `key → description → e.g. example` as plain text. The library error
layer stays style-free; the CLI re-styles. Identical wording across every
diagnostic surface flows from this one formatter. The `context` reports still
own their table layout and may render concrete descriptor fields that are not
part of `Described`.

Public API boundary: Darkmatter exports the catalog module and concrete
descriptor accessors; Claudine imports those APIs. Do not add a Claudine-local
copy of the fuzzy matcher or formatter unless Darkmatter deliberately keeps an
API private for a documented reason.

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
- Context examples use `ExampleVerification::TypeShapeOnly`; fixed literals are
  allowed only when the captured value is genuinely invariant across macOS,
  Linux, Windows, CI, and developer machines.

### Expression functions — anchor: real evaluation

The cleanest case.

- **Head start (post-`86fe86e2a`):** dispatch is now centralized through the
  `PURE_FUNCTIONS` / `FS_FUNCTIONS` registries with `dispatchable_canonical_names()`,
  and Layer 2 already has bidirectional parity tests
  (`descriptor_name_set_equals_dispatchable_runtime_name_set`,
  `lazy_operators_are_dispatchable`) keeping the descriptor set exactly equal to
  the dispatchable runtime set. Reuse the registries to drive the example-eval
  harness rather than re-deriving the executable function list.
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
- Date-relative functions such as `is_today` remain executable by using fixed
  non-relative examples where possible (for example, invalid-date or strict
  parser cases). If a function's only useful example depends on wall-clock time,
  mark it `DisplayOnly("wall-clock dependent")` and document the reason beside
  the descriptor.
- Every expression descriptor must declare whether its example is `Executable`
  or `DisplayOnly`. Display-only expression examples are acceptable only for
  dependency-heavy cases that cannot be made deterministic with a tempdir,
  fixed clock-free input, or local fixture.

### Side effects — anchor: sandbox engine (reuse existing)

- The `cfg(test)` `EFFECT_VERBS` closures remain (they need a live sandbox
  `EffectEngine` and cannot be const strings) — they are the executable side.
  Each descriptor's new `example` is the display/error side. The existing
  `verb_signature_set_equals_descriptor_signature_set` keeps verb↔descriptor
  paired; add a check that every descriptor carries an example.
- Effect descriptor examples default to display-only with the reason
  "side-effect descriptions are proven by the sandbox verb harness" unless a
  future return-shape metadata field makes executable example comparison
  meaningful.
- **Reader note:** the draft originally asked tests to prove that an effect
  example's `result` text is "consistent with the verb's return kind." The
  current `EffectDescriptor` surface has no typed return-kind field, and
  inferring one from English descriptions would create the same prose drift this
  feature is trying to remove. This spec therefore chooses the simpler contract:
  every effect descriptor must have a display example, and sandbox execution
  continues to prove method reachability. If typed effect return metadata is
  added later, a return-shape assertion can be added then.
- **Orphan-method allow-list** (`drift.md` step 3): an explicit
  `const INTENTIONALLY_UNCATALOGUED: &[&str]` naming `EffectEngine` methods
  deliberately outside the capability surface. **Honest caveat:** Rust cannot
  enumerate a type's methods at compile time, so this *documents and reviews*
  the decision — it converts a silent omission into a named, reviewed list,
  which is the most achievable here.
  - **Existing contract (post-`8d5c628c9`/`01cc40b45`):** the `effects/mod.rs`
    and `effects/catalog.rs` module docs already establish that *"the descriptor
    catalog is the authoritative capability surface — a method without a
    descriptor is intentionally outside the surface until a descriptor adds
    it."* That already encodes the intent of this allow-list in code. The
    `EffectVerb`/`EFFECT_VERBS` harness is now `cfg(test)`, and the no-probe
    counters (`engine_build_count`/`network_attempt_count`) live behind the
    off-by-default `effects-instrumentation` cargo feature — a harness asserting
    "no `EffectEngine` constructed" must enable that feature. Reconcile this
    section with that contract: either reference it instead of adding a parallel
    named const, or justify why a discoverable `INTENTIONALLY_UNCATALOGUED`
    const still earns its keep over the prose contract.

### Deliberate non-change

Unknown `ctx.*` access stays **silent null** — null propagation is intentional
per the expression docs. Context "did-you-mean" therefore lands on a diagnostic
surface (Section 4), never the silent runtime path; a deliberately-absent
variable used with `|| fallback` keeps working untouched.

---

## Section 3 — Promoted expression-semantics catalogs

The literal arrays in `context.rs` (precedence, truthiness, mode table, null
propagation, unary/comparison/arithmetic summaries, and variable-access
summaries) become typed Darkmatter catalogs, rendered by the CLI (Layer 1) and
anchored to the real lexer/evaluator (Layer 2) wherever the behavior is
executable.

### New catalogs in `markdown/compose/expression/semantics.rs`

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
Add descriptor catalogs for unary operator rules, comparison rules, arithmetic
rules, and variable-access rules so `context.rs` no longer owns any parallel
semantics prose arrays. All semantics descriptor types `impl Described`, flowing
through the same accessor and renderer where the shape fits.

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
and the remaining semantics sections switch from literal arrays to catalog
iteration, mirroring the function table. The CLI may keep presentation helpers
and section order locally; it must not keep independent semantic rows.

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
  that scans parsed interpolation and condition expressions for `ctx.<name>`
  references absent from the catalog and warns with the nearest match. It must
  not use a regex-only scan as the authoritative implementation because quoted
  strings and code fences can contain `ctx.` text that is not an expression.
- The diagnostic is warning-only and never changes evaluation. It should be
  enabled on normal composition preparation paths and suppressible anywhere
  Claudine already suppresses non-fatal warnings (`--silent` or equivalent).
  Tests must prove `{{ "ctx.toady" }}` does not warn while `{{ ctx.toady }}`
  does.
- Reader note: this diagnostic should be emitted from Darkmatter as structured,
  non-fatal compose diagnostics, not by formatting warning strings during parse.
  A minimal shape is enough: `kind`, `span`/source location when available,
  `unknown_key`, and `suggestion: Option<String>`. Claudine can decide whether
  to render the warning in stderr, dry-run metadata, both, or neither under
  `--silent`.
- Scope: scan expressions that Darkmatter actually parses during composition
  preparation: body interpolation, composed frontmatter expressions, condition
  attributes, and loop/sequence condition expressions where those paths already
  parse Darkmatter expressions. Do not scan raw Markdown text, fenced code
  blocks, or provider output.

### Side effects — least runtime reach (stated honestly)

- Effects are typed methods with **no string dispatcher**, so there is no
  "unknown effect" runtime error to enrich. The accessor pays off here for
  **documentation**: the `--side-effects` report gains an `Example` column, and
  any future string-dispatched effect surface gets enrichment for free. No
  runtime error-gain is promised for effects.

All three use the shared `describe_for_error` formatter (Section 1).

### Error formatting contract

Darkmatter library errors remain plain text and carry no terminal styling.
Claudine may restyle the appended suggestion text with `biscuit-terminal`
components when rendering to stderr, but pipeable data and JSON-like outputs
must not receive ANSI escapes.

---

## Section 5 — Narrative doc, recast drift model, layout & phasing

### Narrative-doc parity (cuttable)

`darkmatter/docs/topics/darkmatter-expressions.md`'s function-table section is
generated from `EXPRESSION_FUNCTION_DESCRIPTORS` and pinned by a `cfg(test)`
`narrative_doc_function_table_matches_catalog` (fails CI on drift). A `just`
recipe (`just darkmatter regen-expr-doc`) or hidden CLI flag regenerates it.
**This is the most cuttable component** — drop it without affecting the rest if
coupling a prose doc to a test is unwanted.

If kept, generation must be limited to clearly marked generated regions. Manual
explanatory prose outside those regions remains hand-authored, and the test
should compare only the generated region so normal documentation edits do not
turn into brittle test maintenance.

### Recast drift-layer model

| Surface | Before | After |
|---------|--------|-------|
| Context `display_type` accuracy | hand-corrected | `capture_value_shape_matches_display_type` |
| Expression function content | trusted prose | executable examples vs `evaluate()` |
| Expression semantics (precedence/truthiness/mode) | literal arrays in `context.rs` | typed catalogs, parser assert-equal, executed anchors |
| Descriptions ↔ behavior | free text | example presence + type/result consistency |
| Narrative doc | fully manual | catalog-parity test |

**Remaining honest gaps:** description *wording* is still trusted; effect
orphan-methods remain a reviewed allow-list or prose contract, not
compile-time-enforced; `DisplayOnly` examples are unexecuted by design but now
visible as explicit metadata with reasons.

### Module layout

- `darkmatter/lib/src/catalog/mod.rs` *(new)* — `Described`, `Example`,
  `ExampleVerification`, `describe`, `suggest`, `describe_for_error`, in-crate
  Levenshtein.
- `darkmatter/lib/src/markdown/compose/context/catalog.rs`,
  `darkmatter/lib/src/markdown/compose/expression/catalog.rs`,
  `darkmatter/lib/src/effects/catalog.rs` — add `example`, `impl Described`,
  new anchor tests. (The context/expression catalogs live under
  `markdown/compose/`; only the effects catalog sits at the `src/` root.)
- `darkmatter/lib/src/markdown/compose/expression/semantics.rs` *(new)* —
  operator/truthiness/mode/null-propagation catalogs plus
  unary/comparison/arithmetic/variable-access semantics catalogs.
- `darkmatter/lib/src/markdown/compose/expression/parser.rs` — expose precedence
  as `pub(crate) const` for assert-equal.
- `claudine/cli/src/commands/context.rs` + `context_render.rs` — render
  semantics from catalogs; add Example column to reports.
- Error paths and diagnostics:
  `darkmatter/lib/src/markdown/compose/expression/mod.rs` (evaluate);
  Darkmatter composition preparation returns parsed, non-fatal ctx-typo
  diagnostics; Claudine renders those diagnostics through its existing
  stderr/status reporting path.

### Implementation phasing

One cohesive feature, six ordered, independently shippable phases — each leaves
the build green:

1. **Framework core** — `catalog/` module, trait, `Example`,
   `ExampleVerification`, accessor, Levenshtein, formatter. Pure, fully
   unit-tested, no consumers yet.
2. **Adopt + anchor the three catalogs** — `impl Described`, add examples, add
   the three executable-anchor harnesses (context type-parity, expression
   example-eval, effects example presence + orphan allow-list).
3. **Promote semantics** — new `semantics.rs` catalogs + assert-equal anchor +
   CLI rendering.
4. **Error enrichment** — wire `evaluate` unknown/arity, the ctx-typo lint, the
   `--side-effects` Example column.
5. **Narrative-doc parity** (cuttable).
6. **Recast `drift.md`** and the three `context/` topic docs to describe the
   closed gaps.

## Open questions for spec review

### Narrative-doc parity (Phase 5): keep or cut?

The spec can ship without Phase 5 because runtime-accessible descriptor data and
CLI diagnostics do not depend on generated prose docs.

| Option | Pros | Cons |
|--------|------|------|
| Keep generated regions in `darkmatter-expressions.md` | Closes the fourth manual surface; gives maintainers one command to refresh tables. | Adds a doc-generation workflow and a brittle failure mode if generated-region markers drift. |
| Cut Phase 5 entirely | Keeps the feature focused on runtime/catalog behavior; no new doc tooling. | Narrative docs can still drift from catalogs. |
| Add snapshot-only docs tests | Detects drift without introducing a generator. | Produces failures that still require manual table repair. |

**Recommendation:** keep Phase 5 only as generated regions with a `just`
regeneration recipe. That preserves the drift-control goal while keeping manual
docs editable.

### Parser anchor: assert-equal first or full rewire?

| Option | Pros | Cons |
|--------|------|------|
| Assert-equal first | Low-risk, shippable, proves the catalog matches parser precedence. | Two copies still exist, even though tests keep them equal. |
| Full rewire | One literal source for precedence. | Higher parser risk; descriptor display concerns can leak into evaluator internals if not kept clean. |
| Hybrid: private operator table feeds parser and descriptors | One internal semantic table while keeping display descriptors separate. | More refactor work than assert-equal; still needs careful API boundaries. |

**Recommendation:** start with assert-equal. Escalate to the hybrid approach if
future operator changes make the duplication painful. Avoid feeding the parser
directly from display descriptors unless the descriptor type is deliberately
split into semantic data plus display data.

### Where should the ctx-typo diagnostic live?

The draft called it a compose-time lint but did not define ownership. This is a
real design choice because Darkmatter owns parsing/evaluation, while Claudine
owns warning presentation.

| Option | Pros | Cons |
|--------|------|------|
| Darkmatter prepare diagnostics, rendered by Claudine | Parser-aware, reusable by any caller, avoids regex false positives. | Requires a small public diagnostic type/API. |
| Claudine-only scan before execution | Fast to implement in the CLI. | Risks duplicating parser behavior and warning on non-expression text. |
| Evaluator warning side channel | Tied to actual runtime lookup. | Harder to keep silent-null semantics clean and may miss short-circuited branches. |

**Recommendation:** implement this in Darkmatter's composition preparation as a
non-fatal diagnostic list and let Claudine render it. That keeps parsing
authority in Darkmatter and styling policy in Claudine.

## Success criteria

- All three catalogs implement `Described`; the shared accessor (`describe`,
  `suggest`) and suggestion quality threshold are unit-tested and reachable at
  runtime.
- Context: `capture_value_shape_matches_display_type` passes and would fail on a
  reintroduced `Nullable` type mismatch; every context example is
  `TypeShapeOnly` or an explicitly justified stronger verification mode.
- Expression: every executable example evaluates to its declared result; the
  semantics catalogs are parser-anchored (assert-equal) and CLI-rendered;
  display-only expression examples carry reasons and are rare by review.
- Effects: every descriptor carries an example with explicit verification
  intent; orphan allow-list/prose contract documented and reconciled with the
  existing catalog-authority module docs.
- `evaluate` unknown-function and arity errors show did-you-mean + verified
  example; the parsed ctx-typo compose-time diagnostic warns with nearest match
  from Darkmatter-owned structured diagnostics without changing silent-null
  evaluation semantics.
- `drift.md` and the three topic docs updated to reflect the closed gaps.
- Build green at every phase boundary.
