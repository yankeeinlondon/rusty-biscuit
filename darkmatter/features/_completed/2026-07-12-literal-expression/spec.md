---
status: ready for planning and implementation
reviewed: true
review_iterations: 7
implemented: true
rulings: Q1–Q5 ruled by Ken 2026-07-12; folded into body
inputs:
  - ../../lib/src/markdown/schemas/simplified/types.rs
  - ../../lib/src/markdown/schemas/simplified/grammar.rs
  - ../../lib/src/markdown/schemas/simplified/convert.rs
  - ../../lib/src/markdown/schemas/format.rs
  - ../../lib/src/markdown/schemas/coerce.rs
  - ../../lib/src/markdown/schemas/triggers/matcher.rs
  - ../../dmls/src/providers/frontmatter.rs
  - ../../dmls/src/overlay/expressions.rs
related:
  - ../_completed/2026-05-11-schemas
  - ../_completed/2026-05-28-schema-coercion
---

# SimplifiedSchema: `literal` and `expression` Types

**Status:** Ready for planning and implementation. Two new entries in the
SimplifiedSchema type vocabulary, plus the DMLS capabilities they unlock.
The five open design questions were ruled by Ken on 2026-07-12; rulings are
folded into the body and recorded in the Open Questions section for the
review trail. The Q2 ruling supersedes the original draft's
value-dialect-only expression semantics.

## Goal

1. **`literal(value)`** — a property whose value must equal exactly one
   scalar. Compiles to JSON Schema `const`. Replaces the single-member-enum
   workaround (`kind: enum(foobar)`) and makes discriminated unions
   first-class.
2. **`expression`** — a string that must parse under the Darkmatter
   expression grammar. The third member of the content-format string-type
   family alongside `yaml` and `json`. Never evaluated — parse-only,
   side-effect-free.
3. **DMLS unlocks** — schema-driven expression intelligence inside
   frontmatter values, exact-value completion for literals, and
   discriminated-union arm narrowing.

## Motivation

- Schemas today spell "this key must be exactly X" as a one-member enum.
  That is the classic JSON Schema workaround for a missing `const`, it reads
  as a degenerate list rather than a statement of identity, and it cannot
  express non-string discriminants (`version: 2`).
- Consumers such as Claudine carry frontmatter properties (`when` on
  lifecycle hooks) whose values are Darkmatter expressions. Today the best
  available typing is `string`, which validates nothing and tells DMLS
  nothing. An `expression` type validates syntax at schema time and gives
  DMLS a principled trigger to light up the existing Phase 9 expression
  machinery inside frontmatter.
- The two compose: a union of inline objects with `literal` discriminants
  plus `expression`-typed hook values describes lifecycle/config documents
  precisely.

## Feature A — `literal(value)`

### Grammar

```yaml
$schema:
  kind: literal(spec)                        # string literal
  version: literal(2; required)              # number literal + constraint
  archived: literal(false)                   # boolean literal
  note: literal('a, b'; default('a, b'))     # quoted value (commas/semicolons)
```

- Exactly **one** positional value, lexed with the same rules as enum
  members (bare token or single-/double-quoted string; quoting protects
  `,`, `;`, `)`).
- Constraints follow after `;`, exactly like `enum(a, b; required)`:
  `literal(spec; required)`.
- `literal()` with no value is a `SchemaError` ("literal requires a value").
- Two or more positional values is a `SchemaError` whose message points the
  author at `enum(...)`.

### Value typing

**Ruled (Q1, 2026-07-12): typed values.** The positional value is lexed as a
YAML-style scalar:

| Authored | Typed as |
|----------|----------|
| bare `true` / `false` | boolean |
| bare integer / float (`NUMBERLIKE`-shaped) | number |
| any other bare token | string |
| quoted (`'2'`, `"true"`) | always string |

This mirrors how YAML itself would type the frontmatter value being
validated, so `version: 2` in a document satisfies `literal(2)` without
coercion gymnastics, and quoting opts out. Edge cases to pin down during
planning: bare `null` is rejected with a "quote it or drop the key" error
(optional properties already accept null; a null literal is always an
authoring mistake), and number detection reuses the existing
numberlike-shape test (no scientific notation, no leading-zero octal
surprises — anything that fails the shape test is text).

### AST representation

Follow the `enum` precedent exactly (keeps `SimplifiedType: Copy`):

- `SimplifiedType::Literal` — new bare variant, keyword `"literal"`.
- `Constraint::LiteralValue(serde_json::Value)` — carries the typed value,
  `as_keyword()` → `"<value>"` (mirrors `Members` → `"<members>"`).
- `grammar.rs` gives `literal` a positional-value parse path analogous to
  `parse_enum_members` (single value, then `;`-separated constraints),
  reusing the member lexer for quoting.
- A `SimplifiedType::Literal` atom without a `LiteralValue` constraint is
  rejected at parse/validation time the same way `enum` without members is.

### JSON Schema emission (`convert.rs`)

- `literal(spec)` → `{ "const": "spec" }` (value emitted with its lexed
  type: `literal(2)` → `{ "const": 2 }`).
- The standard optional-nullable wrapper applies: a non-`required` literal
  property accepts missing/`null`, otherwise must equal the value.
- `literal(x)[]` — **Ruled (Q5, 2026-07-12): allowed** — emits an array
  whose `items` carry the `const` (every item must equal the value).
  Grammatically uniform with every other type (`[]` has zero exceptions),
  falls out for free in union arms, documented as niche.

### Constraints

| Constraint | Allowed | Notes |
|------------|---------|-------|
| `required` | yes | |
| `default(v)` | yes | schema-load lint: `v` must equal the literal value, else `SchemaError` — a default that violates its own `const` is always an authoring bug |
| `suggest(...)` | no | completion is implied by the value itself |
| everything else (`min`, `pattern`, ...) | no | nothing to constrain beyond identity |

### Coercion (`coerce.rs`)

The literal implies its scalar type, so the existing write-back pass treats
it like the corresponding primitive: a document value `"2"` against
`literal(2)` coerces to number `2`; `"true"` against `literal(true)` coerces
to boolean. String literals never coerce. Mirrors the boolish/numberlike
rules and the existing "coerce only when the result validates" discipline.

### Unions and discriminated unions (the payoff)

```yaml
$schema:
  width: [literal(auto), number(min(1))]     # keyword-or-value
  event:                                      # discriminated union
    - '{ kind: literal(created), path: file(required) }'
    - '{ kind: literal(deleted), reason: string }'
```

The quotes around each inline-object arm are required. SimplifiedSchema's
inline-object grammar is a string-layer extension; an authored YAML mapping at
a property position remains invalid. This feature does not change that
established contract.

- Property-level unions may mix `literal` arms with any other atom —
  something `enum` cannot express (`enum` members are homogeneous strings).
- Inline-object union arms with `literal` discriminants become genuine
  tagged unions. Validation-side improvement: when an instance's
  discriminant key matches exactly one arm's `LiteralValue`, error
  reporting selects that arm instead of emitting the full `anyOf` noise
  (see DMLS section — the same selection powers arm narrowing). An arm is
  selected only when the same property key is present as a `literal` in at
  least two arms, the instance contains that key, and exactly one arm's typed
  literal equals the instance value. Otherwise validation retains the normal
  union behavior and diagnostics; it never guesses an arm from a partial or
  ambiguous match. Multiple qualifying discriminant keys must all select the
  same arm, or narrowing is abandoned.

### Trigger match expressions

`LiteralValue` is a pure constraint (value equality, no I/O), so `literal`
is permitted in trigger-schema match expressions. `kind: literal(spec)`
becomes the idiomatic trigger discriminant, replacing today's
`enum(spec)` spelling. `triggers/matcher.rs` gains a `Literal` arm in
`primitive_matches` plus the equality check.

### Relationship to `enum`

Documented explicitly in `schema-definition.md`: `literal(x)` validates
identically to `enum(x)` for string values. `enum` remains the tool for
"one of N strings"; `literal` is "exactly this value, of any scalar type".
No deprecation of one-member enums — they keep working.

## Feature B — `expression`

### Grammar

```yaml
$schema:
  when: expression                            # bare
  guard: expression(required)                 # with constraints
  hooks: { on-error: expression, on-done: expression }
```

- A plain keyword type — no positional value. Parses exactly like `string`.
- No parameterized form in v1. `expression(condition)` — "must parse under
  the condition dialect specifically" — is reserved but deferred as a
  future backward-compatible opt-in variant (**Q2, ruled**; see Semantics).
  Adding that form will not change the meaning of bare `expression`.

### Semantics

**Ruled (Q2, 2026-07-12): lenient bare type.** The expression language has
two dialects that differ only on the meaning of `||`: the value dialect (body
`{{ }}` interpolation) reads `||` as fallback and `&&` as logical AND, while
the condition dialect (`when="..."`) reads `||`/`&&` as logical OR/AND, lowered
to `or(...)`/`and(...)`. A bare `expression` property validates when the
string parses under **either** dialect, so `when: expression` accepts
`is_agent() && os == "macos"` on day one. In practice the condition-mode
parser (`parse_condition`, the erased form of `parse_condition_spanned`)
accepts a parse-superset of the value dialect — same token set (`&&` is
accepted in both dialects; only `||` differs, re-lowered from fallback to
logical OR) — so "either" is implemented as a single condition-mode
parse, with a regression test asserting the superset property (every
value-dialect-valid corpus string also condition-parses). Schema validation
checks **parseability only**:

- No evaluation, ever. No I/O, no shell, no function execution. Same
  passivity contract as `yaml` / `json` content-format validation and the
  DMLS overlay (`tests/no_side_effects.rs` pattern).
- Unknown identifiers are **not** schema errors — identifier resolution is
  a compose-time concern (frontmatter keys, `ctx.*`, `env.*` exist only at
  compose). DMLS layers richer, advisory diagnostics on top (below).

### JSON Schema emission and validation

- Emits `{ "type": "string", "format": "darkmatter-expression" }`.
- `format.rs` gains `DARKMATTER_EXPRESSION_FORMAT` registered alongside
  `darkmatter-yaml` / `darkmatter-json`, backed by a pure
  `parse_condition(value).is_ok()` check (the either-dialect rule above).
- Constraint applicability, optional-nullability, and the `$()` / `{{ }}`
  pending-value deferral rules mirror `yaml` / `json` exactly — it is the
  third member of that family, not a new category. Concretely, it permits the
  universal `required`, `default(...)`, and `generated` constraints plus array
  constraints when suffixed with `[]`; string constraints and `suggest(...)`
  are rejected, matching `yaml` / `json`. A `default(...)` value is checked by
  the existing schema-load validation and must itself parse as an expression.

### Coercion

**Ruled (Q3, 2026-07-12): coerce.** `when: true` and `retries: 3` are valid
degenerate expressions that YAML types as boolean/number before the
validator sees them. The coercion pass serializes native boolean and number
scalars to their canonical literal string forms (`true` → `"true"`,
`3` → `"3"`) — the same native-value-accepted-then-serialized behavior
`yaml`/`json` already have. Number spelling canonicalizes through YAML's
reading (`3.10` → `"3.1"`); quoting preserves exact spelling. Mappings and
sequences do not coerce; they are type mismatches.

### Compose pipeline interaction

Nothing new. Validation runs post-interpolation as today; a value still
holding `$(...)` or unresolved `{{ }}` follows the existing pending-value
deferral (`PendingPolicy`). No new pipeline stage.

### Consumer layering

The type ships in Darkmatter; consumers adopt it in their schemas. E.g.
Claudine's extension baseline retypes `when: string` → `when: expression`
with zero Claudine-specific code in Darkmatter — the same pure-config
activation model as DMLS's `[schema.extensions.claudine]`.

## DMLS Improvements

The headline value of both types is that they turn schema knowledge into
editor behavior. All items are gated on the effective schema (base +
extension baselines + triggers + document `$schema`) typing the property —
no heuristics.

### D1 — Expression intelligence inside frontmatter values

Today the Phase 9 expression machinery (`overlay::expressions`) activates
only for body `{{ }}` sites and `$()` shell values. When the effective
schema types a property `expression`, the frontmatter provider activates
the same machinery **inside the YAML value**:

- **Completion** — catalog-backed items via `completion_partial`-equivalent
  logic scoped to the value span: `ctx.*` keys (typed `detail`, Markdown
  `documentation`), expression functions (`typed_signature()` detail), and
  same-document frontmatter keys. The `.`/`(` trigger characters are
  already advertised in `capabilities.rs`; the open-interpolation guard is
  replaced by an is-inside-expression-typed-value guard.
- **Hover** — reuse `format_ctx_hover_block` and `format_function_block`
  verbatim (same bytes as the interpolation hovers) for identifiers and
  the deepest `FunctionCall` under the cursor (`function_call_at`).
- **Diagnostics** — `dm.expression.malformed` and
  `dm.expression.unknown_identifier` (advisory, unknown root identifiers only
  — the same policy as the body provider). Source `darkmatter.frontmatter`.
  The dedicated malformed diagnostic replaces, rather than duplicates, the
  generic `dm.schema.constraint` problem for the same
  `darkmatter-expression` format failure. Other schema problems on the
  property remain visible.

  Parser positions are byte offsets in the decoded YAML scalar, not the raw
  document. DMLS must project decoded boundaries back to authored byte
  boundaries before constructing a range; adding the offset directly to
  `value_span.start` is incorrect for quoted scalars, escapes, and non-ASCII
  text. Extract the existing decoded-to-raw scalar mapping behavior from
  `simplified/source.rs` into a shared Darkmatter helper used by both schema
  source parsing and DMLS. The diagnostic starts at the projected parse-error
  boundary and ends at the projected end of the decoded scalar content,
  excluding YAML quote characters. If projection is unavailable for a valid
  YAML scalar form, range the complete value node as a safe fallback. Add
  plain, single-quoted, double-quoted escaped, and multibyte regression cases.

This is the single biggest unlock: `when:` values in Claudine hook
documents get the full typed-catalog experience with no new provider — one
schema keyword flips it on.

### D2 — Literal-typed completion, hover, and code actions

- **Value completion** — a `literal`-typed property offers exactly its
  value, marked preselected. Unions offer each `literal` arm's value plus
  the usual scaffolds for non-literal arms (subsumes today's enum-member
  completion path; `enum_members`-style accessor gains a literal sibling).
- **Hover** — `schema_hover_details` renders `Type: **literal** — exactly
  ` `spec` `` plus the usual constraint/description lines.
- **Code action** — Phase 10's add-missing-required-key action inserts the
  literal's actual value instead of an empty scaffold (`kind: spec`, not
  `kind: `). Zero-decision insertions become correct-by-construction.

### D3 — Discriminated-union arm narrowing

When a property (or the root `$schema` union) is a union of inline objects
whose arms carry `literal`-typed discriminants:

- **Key completion narrows** — once the document contains `kind: created`,
  sibling-key completion inside that mapping offers only the matching
  arm's keys (today all arms' keys blur together).
- **Diagnostics narrow** — `missing_required` / `unknown_key` report
  against the matched arm only, replacing `anyOf` composite noise. This
  rides the library-side arm-selection rule from Feature A (matched
  discriminant → single-arm error reporting), so `md schema validate`
  improves identically.

The selection rule is the library rule defined under Feature A: a shared
discriminant key, an authored instance value, exactly one typed equality
match, and no conflict between multiple discriminants. Before a discriminant
is present, for an unknown value, or for duplicate literal values across arms,
completion and diagnostics retain today's merged/union behavior. Equality is
type-sensitive (`2` does not select an arm tagged with `'2'`). This makes
incomplete documents stable while they are being edited and prevents an
arbitrary first-arm choice.

Scope note — **Ruled (Q4, 2026-07-12): in scope with a pre-approved escape
hatch.** Both halves (completion narrowing and diagnostics narrowing) are
in scope for this feature. If the library-side arm-selection error-reporting
rework proves a phase-buster during planning or implementation, the
diagnostics half splits into its own follow-up fix without a further
ruling. Guardrail either way: narrowing activates only for schemas using
literal discriminants; existing schemas' validation output stays
byte-identical.

### D4 — Semantic tokens alignment (note only, not scope)

The deferred F5 frontmatter token family from
[2026-07-11-semantic-tokens](../2026-07-11-semantic-tokens/spec.md) gains a
natural, schema-driven target: `expression`-typed values are exactly the
spans F5 would classify. Nothing ships here; recorded so F5 design starts
from the typed-value signal instead of re-deriving it.

## Implementation Surface Map

Every new type keyword pays this tax; listed so planning can phase it:

| Surface | `literal` | `expression` |
|---------|-----------|--------------|
| `simplified/types.rs` — `SimplifiedType` variant + keyword maps | ✓ (+ `Constraint::LiteralValue`) | ✓ |
| `simplified/grammar.rs` — parse path | positional-value path (mirrors enum members) | keyword only |
| `simplified/serialize.rs` — canonical round-trip | `literal(v)` / `literal('q v')` | `expression` |
| `simplified/convert.rs` — JSON Schema fragment | `const` + nullable wrapper | `type: string` + `format` |
| `format.rs` | — | `DARKMATTER_EXPRESSION_FORMAT` |
| `coerce.rs` | typed-scalar coercion | boolean/number → string form |
| `simplified/lint.rs` | `default` ≠ value lint; `suggest` rejection | `suggest` rejection (mirrors yaml/json) |
| `triggers/matcher.rs` | `primitive_matches` + equality (pure) | string-shaped + parse check (mirrors yaml/json) |
| `about.rs` descriptor catalog → `md schema about` | ✓ | ✓ |
| `detect.rs` | never inferred (non-goal) | never inferred (non-goal) |
| Docs: `docs/topics/schema-definition.md` | ✓ | ✓ |
| DMLS `providers/frontmatter.rs` + `overlay/expressions.rs` | D2, D3 | D1 |
| Shared decoded-YAML-scalar source projection (extract from `simplified/source.rs`) | — | precise D1 ranges |
| Skill/docs hash refresh (`md hash`) | ✓ | ✓ |

## Non-Goals

- No `postcode` / `phone` / other locale-bearing convenience types — the
  ruled alternative is pattern-constrained named types in a shared schema
  file via `Name@file` imports.
- No expression **evaluation** anywhere in validation or DMLS. Parse-only.
- No static return-type checking for expressions (`when` evaluating to
  boolean) — the evaluator is not statically typed; `expression(condition)`
  is the reserved future hook.
- No schema **detection** inference of either type (`md schema detect`
  continues to infer plain primitives).
- No deprecation of single-member enums.
- No F5 semantic-token implementation (alignment note only).

## Acceptance Criteria

1. `literal(spec)`, `literal(2)`, `literal(false)`, and `literal('a, b')`
   parse, serialize to a stable canonical spelling, reparse to an equivalent
   AST, and compile to typed `const`
   fragments; `literal()` and `literal(a, b)` fail with the specified
   errors.
2. A non-`required` literal property accepts missing/`null`; `required`
   enforces presence + equality. `default` must equal the value or the
   schema fails to load.
3. Coercion writes back typed scalars for non-string literals
   (`"2"` → `2` against `literal(2)`), pending values excluded, and only
   when the coerced result validates.
4. Property-level unions mixing `literal` with other atoms validate
   correctly (`[literal(auto), number]`).
5. `expression` accepts any string that parses under either expression
   dialect (implemented as a condition-mode parse; superset property
   regression-tested), rejects unparseable strings with a
   `constraint`-class problem carrying the format name, and never executes
   anything (side-effect regression test in the `no_side_effects` style).
6. Native boolean/number values coerce to their string forms against
   `expression`; mappings/sequences are type mismatches.
7. `literal` works as a trigger match-expression constraint; `expression`
   behaves like `yaml`/`json` in triggers.
8. `md schema about` lists both types with descriptors; existing schemas'
   `md schema validate` output is byte-identical (pure addition).
9. DMLS: expression-typed values get completion/hover/`dm.expression.*`
   diagnostics inside frontmatter (D1); literal-typed properties get
   exact-value completion, hover, and required-key insertion with the real
   value (D2); discriminated-union key completion narrows on a matched
   literal discriminant (D3). Malformed expression values emit one dedicated
   diagnostic rather than a duplicate generic format diagnostic; ranges map
   decoded offsets correctly for plain, quoted/escaped, and multibyte scalars.
10. Discriminant narrowing is type-sensitive and occurs only for one
    unambiguous arm; absent, unknown, duplicate, or conflicting discriminants
    preserve existing union behavior in both the library and DMLS.
11. All L1/L2 suites green via the area `just test` / `just test-l2`.

## Open Questions

All five questions were ruled by Ken on 2026-07-12 and folded into the body
sections noted below. Recorded here for the review trail.

- **Q1 — Literal value typing.** **RULED (2026-07-12): typed values** —
  YAML-scalar lexing (bare `true`/`2` typed, quoted always string); bare
  `null` rejected. Folded into the Value typing section.
- **Q2 — expression dialects.** **RULED (2026-07-12): lenient bare type** —
  bare `expression` accepts either dialect (condition-mode parse; superset
  property regression-tested); `expression(condition)` reserved as a future
  backward-compatible opt-in variant. Supersedes the draft's value-dialect-only
  wording, which would have read `||` in `when:` conditions as fallback rather
  than logical OR (both dialects parse `&&`/`||`; only `||`'s meaning differs).
  Folded into the Semantics section.
- **Q3 — Expression native-scalar coercion.** **RULED (2026-07-12):
  coerce** — native boolean/number scalars serialize to their string forms
  (family rule shared with `yaml`/`json`); mappings/sequences remain type
  mismatches. Folded into the Coercion section.
- **Q4 — D3 diagnostics narrowing scope.** **RULED (2026-07-12): in scope
  with pre-approved escape hatch** — both halves planned here; the
  diagnostics half may split into a follow-up fix without a further ruling
  if the reporting-layer rework balloons. Folded into the D3 scope note.
- **Q5 — `literal(x)[]` arrays.** **RULED (2026-07-12): allowed** — grammar
  uniformity (`[]` has no exceptions), zero special-case code, niche but
  well-defined. Folded into the JSON Schema emission section.
