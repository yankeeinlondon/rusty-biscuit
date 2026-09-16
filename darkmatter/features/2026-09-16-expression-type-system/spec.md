---
created: 2026-09-16
status: draft
supersedes:
  - ../2026-07-15-type-system/spec.md
  - ../2026-07-22-explicit-null/spec.md
---

# Expression Type System

Chartered by the dasherized-identifiers feature's ratified "ship now +
charter" split
([2026-09-15-dasherized-identifiers](../2026-09-15-dasherized-identifiers/spec.md),
Resolved Decision 16). This is a skeleton — motivation, phased scope, and
open questions — not a design; clarification owns the detail. The
2026-09-16 clarification session settled the three-way disposition of the
earlier drafts (see [Lineage](#lineage)) and re-homed the July draft's
design into the
[declarations annex](declarations-design.md)
(`declarations-design.md`), which now carries the Phase C/E detail.

## Status

Draft skeleton. The phases below are a charter, not an implementation plan;
Phase C/E design detail lives in the
[declarations annex](declarations-design.md). Splitting into per-increment
specs is deferred until the first increment is scheduled.

## Summary & Motivation

Darkmatter's expression language has runtime values but no static type
vocabulary for expressions:

- `SimplifiedType`
  ([`types.rs`](../../lib/src/markdown/schemas/simplified/types.rs)) has `Any`
  but no `unknown` and no `null`.
- The function catalog
  ([`docs/schemas/expression-functions.yaml`](../../docs/schemas/expression-functions.yaml))
  plus `ExpressionFunctionDescriptor`/`ParamType`
  ([`catalog/mod.rs`](../../lib/src/markdown/compose/expression/catalog/mod.rs))
  is a parallel ad-hoc typed system that shares only the type vocabulary.
- The expression parser is untyped.

Consequences:

- Absence intent can only be suppressed through the interim catalog-driven
  rule (dasherized Resolved Decision 9) instead of parameter nullability.
- Parameter type mistakes are invisible until runtime.
- Optional-but-typed properties have no honest type: an optional `A` is
  really `A | null`, and the vocabulary cannot say so.

## Scope (phased, one spec)

- **Phase A — `unknown` and `null` types.** Add both to `SimplifiedType`.
  Union machinery already exists at property and root level; the `null`
  keyword is the missing piece. Its specced core, absorbed from the
  superseded
  [2026-07-22-explicit-null](../2026-07-22-explicit-null/spec.md) draft: an
  explicit `null` property type, so an arm can declare a property as
  deliberately absent —

  ```yaml
  $schema:
    - spec: file(required)
      review: null
    - review: file(required)
      spec: null
  ```

  — the XOR authoring idiom: a caller passes a spec file or a review file,
  not both. That idiom is a Phase A test case. Rename commitment: when this
  lands, the dasherized spec's "type is `any` by default" language becomes
  `unknown` — a documentation rename, no behavior change.
- **Phase B — function schemas in SimplifiedSchema.** Express the
  expression-engine function catalog's signatures in SimplifiedSchema,
  unifying the ad-hoc `ParamType` system; preserve the parity tests.
- **Phase C — type-aware parsing/evaluation.** Untyped variables default to
  `unknown`; optional-but-typed properties carry `A | null`, with null as
  YAML's representation of undefined, under the translation model below.
  The declaration-layer design that feeds this phase — schema-derived
  variable declarations, strict root validation, host retention — is in
  the [declarations annex](declarations-design.md).
- **Phase D — flow-sensitive narrowing.** Inside conditional blocks
  (`::block when="x"` narrows `file | null` to `file`), in ternary
  truthiness branches (`x ? frontmatter(x, 'foo') : null` narrows `x` in
  the true branch), and in `&&`-guarded calls
  (`file_exists(x) && frontmatter(x, 'foo')`; today's catalog declares
  `file_exists(file: file)`).
- **Phase E — diagnostics.** Generalize the dasherized spec's interim
  suppression to "any parameter whose type admits null"; add parameter type
  diagnostics — a known-null passed to a non-null parameter is an error, a
  union-including-null argument is a warning. Landing this phase is the
  explicit retirement trigger for dasherized Decision 9's interim rule.
  The declaration-layer diagnostics distinctions are specified in the
  [declarations annex](declarations-design.md).

Phases A and C share one ratified translation model, recorded in full in
the [declarations annex](declarations-design.md): optionality is a
**default constraint** (`optional` unless `required`), exactly like
`max-length: 5` refines `string`. A schema declaring
`string(max-length: 5; required)` gives the variable a runtime type that is
a string constrained to five characters — calling it merely "string" is
incomplete. Likewise a schema declaring `string` — optional by default —
gives the variable the effective runtime type `string | null`; calling it
merely "string" is incomplete in the same way. In general, an optional
property with declared type A has effective runtime type `A | null`, and
`null` is YAML's representation of undefined. Required-by-default was
considered and rejected: it would make `string(optional)` → `string | null`
the more intuitive reading, but optional-by-default was chosen because far
fewer properties tend to be required.

Also in scope: the two CLI items deferred by the dasherized spec — a
`--deny-warnings`-style promotion flag (mirroring `--strict-style`) and
machine-readable warning output (`--format json` currently emits the document
only).

## Interface Seam

Darkmatter owns parsing, evaluation, and type safety end-to-end. Callers
(e.g. claudine) ask Darkmatter to evaluate and extend the language by passing
in functions plus type definitions (SimplifiedSchema) — never bespoke grammar
or evaluation. Verified today: claudine already calls Darkmatter's
parser/evaluator and supplies variables via `EvaluationLookup`; the residual
coupling is read-only AST walks in claudine
([`composition/looping/config.rs`](../../../claudine/lib/src/composition/looping/config.rs),
[`composition/lifecycle/validate.rs`](../../../claudine/lib/src/composition/lifecycle/validate.rs))
that this work should formalize or replace. The
`EvaluationLookup::is_known_variable_root` hook (default `true`) remains the
sanctioned third-party warning opt-in.

## Lineage

Resolved 2026-09-16, as an **annex-absorb** consolidation (recorded as
Resolved Decision 17 in
[2026-09-15-dasherized-identifiers](../2026-09-15-dasherized-identifiers/spec.md));
this spec is the primary. Both earlier drafts are superseded **in place**
— they remain where they are as the historical record, each carrying
supersession frontmatter and a pointer:

- [2026-07-15-type-system](../2026-07-15-type-system/spec.md) — the large
  July draft (schema-derived variable declarations, strict root
  validation). Its design body is re-homed, with dated corrections, into
  the [declarations annex](declarations-design.md) as Phase C/E detail.
  One stance is consciously overturned in the re-homing: the July draft
  held that schema nullability and optional absence contribute no `null`
  type to the union; the ratified translation model (above) gives an
  optional `A` the effective runtime type `A | null`. Its stale ten-file
  `inputs` list and its "does not authorize changes" gating language were
  dropped rather than carried.
- [2026-07-22-explicit-null](../2026-07-22-explicit-null/spec.md) — the
  small null-type draft. Its specced core (the `null` keyword) and its XOR
  authoring idiom are folded into Phase A above, the idiom as a Phase A
  test case; nothing else was carried because nothing else was specified.

## Open Questions

- The exact `unknown` vs `Any` migration story.
- How narrowing interacts with DMLS's static analysis — the editor must
  mirror runtime narrowing rules.
- Where the function-schema unification leaves `ParamType`.
- Whether `--deny-warnings` promotes all warnings or is code-filterable.
