---
status: draft
created: 2026-08-12
area: darkmatter
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-12
packages:
  - darkmatter
  - claudine
---

# A declared optional parameter should be a binding, not a typo

## Summary

A document can declare an optional parameter in its `$schema` — a value the
caller *may* pass, with no default when they don't. Declaring "unset is
meaningful" is a legitimate and useful pattern: claudine's shipped
`implement-plan.md` prompt declares `commit_message: string` precisely so that
*unset* means "let AI write the commit message" and *set* means "use exactly
this one."

Today that declaration is inert. If the caller doesn't supply the value, the
key simply does not exist in the composed frontmatter — indistinguishable from
a typo. Every fail-closed check downstream then treats a reference to it as a
defect and aborts. The shipped prompt cannot be composed at all without
passing a `commit_message`, which defeats the point of it being optional.

This spec proposes that a document-owned SimplifiedSchema declaration create a
real post-schema binding: an optional parameter the caller did not supply
is materialized as an explicit `null` in the effective frontmatter. Later
compose consumers then recognize the root and resolve it as null/falsy, while a
genuinely undeclared root still fails closed exactly as it does today.

> **Reader note — review correction:** The original draft proposed using every
> property in the merged effective schema, including baseline properties. That
> would cause the default Darkmatter baseline to inject a large catalog of
> unrelated metadata and style keys into nearly every composed document. The
> reviewed design instead materializes only top-level properties declared by
> the document's own inline or referenced SimplifiedSchema. Baseline and
> trigger schemas remain validation policy, not parameter declarations.

## Reproduction

From the monorepo root:

```sh
claudine compose prompts/implement.md spec='fixes/2026-08-07-retire-specialized-workflows/plan.md' -y --codex
```

```
CompositionError: composition failed

lifecycle shell command at `success.stack[1].action[2].command` failed
pre-flight resolution: Transform error: subtree-compose strict mode: unknown
root 'commit_message' in '{{ commit_message }}'
(raw: `git commit -m "{{commit_message}}"`)
```

The redirect from `implement.md` is incidental; composing
`prompts/_implement/implement-plan.md` directly without `commit_message=...`
fails identically.

## Background, for a reader new to this area

**A `$schema` declaration** is the frontmatter block where a document names its
parameters, their types, and constraints. In the failing prompt:

```yaml
$schema:
    plan: file(eager; required; match(**/*plan*.md)) -> the plan being implemented
    commit_message: string -> if you pass in a git commit message then it will
      be used as the git message instead of using AI to calculate it
```

`commit_message` is optional (no `required`) and has no `default(...)`. Both
are deliberate: the *absence* of a caller-supplied value is the signal that
selects the AI-authored-commit branch.

**The effective frontmatter** is the flat map of key/value pairs a document
ends up with after compose merges its authored frontmatter with caller
overrides (`key=value` on the CLI, `--set`, chained-document forwarding).
Post-schema compose stages, Claudine lifecycle guards, and lifecycle command
resolution read from this map. Today, `$schema` contributes nothing to it: a
declared parameter the caller didn't pass is simply not a key.

**The fail-closed checks** exist to catch typos. A `{{ spec_fil }}` where the
author meant `{{ spec_file }}` must not silently resolve to an empty string
inside a shell command or a guard that decides whether to commit to git. Two
independent checks enforce this, and both key off the same question — *is this
root a key in the effective frontmatter?* — with no way to distinguish
"declared but unset" from "never declared."

## The defect

The prompt's success lifecycle has three guarded branches:

```yaml
success:
    stack:
        - when: "ctx.dirty_files && !commit_message"     # AI writes the message
          action: [ ..., shell: just commit, ... ]
        - when: "ctx.dirty_files && commit_message"      # caller's message
          action: [ ..., shell: 'git commit -m "{{commit_message}}"', ... ]
        - when: "!ctx.dirty_files"
          action: [ ... ]
```

With `commit_message` unset, this trips two separate fail-closed checks:

### Check one — pre-flight shell resolution (the error above)

Lifecycle `shell:` commands are resolved at pre-flight, before any event
fires, so the command the user approves is byte-identical to the one that
executes
([`resolve_lifecycle_shell_commands`](../../../claudine/lib/src/composition/preflight.rs),
called from `prepare.rs`). This walk necessarily ignores `when:` guards — it
must approve every command that *could* run. Each command's `{{ }}` spans are
resolved through Darkmatter's subtree compose in strict mode, whose pre-pass
rejects any root not present in the effective frontmatter
([`validate_strict_roots`](../../lib/src/markdown/compose/subtree.rs)).
`commit_message` is absent → `unknown root` → the whole compose aborts, even
though the branch containing the reference could never have run.

### Check two — event-time `when:` guards (would fail next)

Even if pre-flight passed, lifecycle guards fail closed on undefined roots at
event time:
[`when_matches`](../../../claudine/lib/src/composition/lifecycle/executor.rs)
runs `first_undefined_stack_variable` against the live frontmatter and raises
on any referenced key that is missing, and an expression-layer raise halts the
stack. With `commit_message` unset, **both** commit branches raise — including
`!commit_message`, the branch that is *supposed* to run when the value is
unset. The pattern is unimplementable, not merely blocked at pre-flight.

Both checks tolerate only `||` fallbacks and unchosen ternary branches
(`collect_variable_roots` and the undefined-variable walk share these
short-circuit semantics). A bare operand of `&&` or `!` is always policed.
Writing `{{ commit_message || '' }}` everywhere would silence the checks, but
it erases the unset-vs-empty distinction the schema author deliberately
encoded, and it makes the `$schema` declaration meaningless — every consumer
of the parameter must independently re-declare "this might not exist."

## Required behavior

**Declaring a document parameter creates a post-schema binding.** During
the schema stage of composition, materialize a top-level key as JSON `null`
when all of the following are true:

- the winning declaration comes from the document's inline SimplifiedSchema or
  a SimplifiedSchema file referenced by that document;
- the property is optional and therefore has Darkmatter's nullable optional
  wrapper;
- it has no `default(...)` annotation; and
- it is absent after authored frontmatter and caller overrides are merged.

Caller-supplied values win unchanged, including explicit `null`, `false`, zero,
an empty collection, and the empty string. Materialization is idempotent across
the pre-shell and post-shell schema passes.

The schema-validation stage
([`schema_validation.rs`](../../lib/src/markdown/compose/schema_validation.rs))
is the required seam. It already mutates stored frontmatter by writing
type-coerced values back through `coerce_frontmatter_with_pending`; it runs
after overrides and first-pass frontmatter interpolation; and it is the one
place that holds the resolved effective schema and its property-origin map.
Materialize eligible bindings after effective-schema resolution and before
coercion and validation. Existing SimplifiedSchema conversion already makes
optional properties nullable, so both materialized and caller-supplied nulls
validate without a validator exception.

This single mutation fixes every consumer downstream of schema validation:

| Surface | Today (unset) | After |
|---|---|---|
| Pre-flight strict shell resolution | aborts compose | root is known; `{{commit_message}}` stamps as empty in a branch its guard prevents from running |
| Event-time `when:` guards | raises, halts the stack — both branches | `null` is falsy: `commit_message` → false, `!commit_message` → true; the AI branch runs as designed |
| Event-time C2 interpolation of communication strings | fails closed on the undefined root | resolves as null/empty |
| Body `::block when="commit_message"` | falsy (lenient) | falsy — unchanged |
| A genuinely undeclared root (typo) | fails closed | fails closed — unchanged |

The last row is the invariant to preserve: this change must not weaken typo
detection. The known-root set grows by exactly the eligible document-owned
parameter names, nothing else.

### Semantics of the materialized null

- `null` is falsy in guards and renders as the empty string in interpolation —
  identical to how a lenient lookup treats a missing key today, so body
  behavior does not shift.
- Unset remains distinguishable from empty: `commit_message` set to `""` is a
  present string; materialized `null` is not. Documents that need the
  distinction can test `is_null(commit_message)` (or plain truthiness, as the
  failing prompt already does). Darkmatter deliberately does not define two
  null values as equal, so `commit_message == null` is not a null predicate.
- For an eligible no-default parameter, a caller *explicitly* passing `null`
  is equivalent to not passing it. No downstream observable difference exists
  between the two.
- Materialized keys are ordinary effective-frontmatter values after the schema
  stage. They appear in composed output and participate in later serialization,
  hashing, forwarding, and subtree composition exactly like an authored null.

### Scope boundaries

Materialization is a composition behavior, not a validation behavior.
Validation-only APIs remain passive and read-only, and a compose operation with
no effective schema does not synthesize bindings. Compose has no public switch
that disables its schema stage while retaining a document `$schema`.

Only document-owned SimplifiedSchema properties participate:

- Include inline document `$schema` properties and properties from a
  whole-file SimplifiedSchema reference. Imported named types used by those
  properties remain eligible because the top-level declaration is still
  document-owned.
- Exclude baseline properties. A baseline describes common frontmatter shape;
  it does not declare caller parameters. In particular, the default Darkmatter
  baseline must not flood output with null metadata and style keys.
- Exclude trigger-payload properties. Trigger matching treats missing and null
  as equivalent already, but a matched validation layer must not create a new
  parameter surface.
- Exclude raw JSON Schema. JSON Schema's absence, nullability, and `default`
  annotations do not imply Darkmatter parameter-binding semantics.
- Exclude root-union document schemas in this fix. Their effective schema has
  no per-arm property provenance, and materializing a key from a non-selected
  arm could change arm selection. Supporting them requires a separate design
  for binding intersection and selected-arm stability.
- Exclude nested object properties. Expression roots and the motivating
  fail-closed checks operate at the top level; recursively creating object
  structure would be a different semantic change.

First-pass frontmatter interpolation occurs before schema validation and is
therefore outside this fix's binding guarantee. This preserves the established
pipeline contract that interpolated values are resolved before schema
selection, coercion, and validation. A document that needs an optional value
while computing another frontmatter key must continue to author the null key
explicitly or use an existing `||`/ternary fallback. Moving schema resolution
ahead of interpolation would be a broader pipeline change and is not necessary
for the lifecycle defect addressed here.

## Design decisions

1. **Use the existing optional-null contract.** SimplifiedSchema already wraps
   optional properties in a nullable JSON Schema arm and treats missing and
   explicit null as equivalent. This fix must reuse that representation rather
   than special-case the validator or materialize after validation.

2. **Materialized nulls appear in composed-output frontmatter.** `md compose`
   output for a document declaring `commit_message` will now show
   `commit_message: null`. This is observable but honest — the binding really
   exists. Serialization order must remain deterministic, and affected
   snapshots over composed frontmatter must be updated.

3. **Preserve `default(...)` as metadata.** The established SimplifiedSchema
   contract explicitly says defaults do not mutate documents; consumers may
   interpret them. A property carrying `default(...)` is excluded from null
   materialization so a synthesized present-null value cannot mask the
   consumer's missing-value default behavior. Applying schema defaults is not
   part of this fix.

4. **Use property provenance, not schema-shape heuristics, for source scope.**
   `EffectiveSchema::origins` already identifies top-level document,
   referenced-file, baseline, and trigger winners. Eligibility must require a
   document or referenced-file origin and a SimplifiedSchema effective shape.
   Do not infer ownership merely because the merged JSON Schema contains a
   nullable property.

## Open questions

None. Root-union binding semantics and pre-schema frontmatter interpolation are
explicitly deferred rather than left to implementation-time judgment.

## Alternatives considered

**Thread the declared-key set into each fail-closed checker.** Teach
`validate_strict_roots`, `first_undefined_stack_variable` /
`when_matches`, and event-time interpolation to accept schema-declared names
as known (resolving null at evaluation). Rejected: at least three seams in two
crates that must stay in sync forever, and each future fail-closed surface
must remember to consult the schema. Materializing the binding once makes
"key exists in effective frontmatter" remain the single source of truth.

**Require authors to write `|| ''` fallbacks and null-materializing
frontmatter lines.** Works today (`commit_message: null` in the frontmatter
body would create the binding by hand), but it means the `$schema` declaration
alone is insufficient and every prompt author must know the trick. The
declaration should be the contract. (This *is* the interim workaround for the
blocked prompt if one is needed before this fix lands.)

**Have claudine's pre-flight skip shell commands in branches whose guards are
statically false.** Unsound — guards react to live state (`ctx.dirty_files`)
that pre-flight cannot know, and it fixes only check one of two.

## Verification

Success criteria, most specific first:

1. **The original failure composes.** `claudine compose
   prompts/_implement/implement-plan.md plan=...` with no `commit_message`
   passes pre-flight. This is the regression test and must use the real
   shipped prompt artifact through the normal invocation path (L2), not a
   synthetic fixture.
2. **The set path still works.** The same compose with
   `commit_message='chore: x'` stamps `git commit -m "chore: x"` into the
   approved command.
3. **Guard semantics at event time.** With `commit_message` unset, a lifecycle
   run selects the `!commit_message` branch and skips the `commit_message`
   branch — no evaluation raise. With it set, the selection inverts.
4. **Typos still fail closed.** An undeclared root in a shell command or
   `when:` guard produces the same errors as today (existing strict-mode tests
   must not loosen).
5. **Positive materialization rules.** Unit coverage in the schema stage:
   optional no-default top-level properties from inline and referenced
   SimplifiedSchema documents materialize as null. A second schema pass is
   idempotent. Caller values — explicit null, empty string, false, zero, and
   empty collections — are untouched.
6. **Source and schema exclusions.** Baseline, trigger, raw JSON Schema,
   root-union, nested, `required`, and `default(...)` properties do not
   materialize. A missing required property remains a validation failure.
   Include a compose test with the normal default Darkmatter baseline proving
   it does not inject unrelated null keys.
7. **Validation API remains passive.** Direct schema validation accepts both a
   missing optional property and explicit null as it does today, returns the
   same diagnostics, and does not mutate the document.
8. **Round trip.** A composed document's frontmatter carrying a materialized
   null re-composes identically (write → read → write is stable).
9. **Passive corpus and normal path.** Parse and resolve every shipped prompt
   schema without executing prompts or lifecycle effects. In addition to the
   focused Claudine L2 regression above, exercise at least one shipped prompt
   through the ordinary compose invocation path.

## Out of scope

- Materializing `ctx.*` / `env.*` namespaces — separate mechanisms, already
  handled (see the 2026-08-02 silent-empty-ctx-values fix for the `ctx` side).
- Loosening strict-mode semantics for undeclared roots in any way.
- Baseline, trigger-schema, raw JSON Schema, root-union, or nested-property
  materialization.
- Making optional bindings available to the first frontmatter-interpolation
  pass.
- Applying `default(...)` values; defaults remain non-mutating schema metadata
  by established contract.
