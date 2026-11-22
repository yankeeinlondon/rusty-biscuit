---
kind: feature
created: 2026-11-22
status: human-in-the-loop
area: simplified-schema
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
next-spec: 2026-11-22-host-registration-v2
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed-by: string -> the agent and model used in the spec review
    reviewed-on: date -> the date the spec was reviewed
    review-iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented-by: string -> the agent who implemented the plan
reviewed: true
reviewed-by: "codex/gpt-6.2-sol"
reviewed-on: "2026-11-22"
review-iterations: 0
needs-rulings: true
required-rulings:
    - Decide whether to allow a recursive cycle that includes both a composition reference and a child value.
---

# Object composition with `...spread` and `$extends`

## Why this exists

A schema that needs the same properties in several object types, such as a
Claudine sequence task and a group leg, currently has to repeat the
declarations. The copies can drift. Object composition lets authors reuse
those properties and override individual definitions. Use `...name` inside
an inline object and `$extends` in a YAML schema object.

```yaml
kind: schema
$schema:
    $extends: events
    source: string
    snapshot: "{ ...events, price: number(min(0)) }"
types:
    events: "{ id: string(required), price: number }"
```

The root receives `id` and `price` from `events`. The nested `snapshot`
receives the same required `id` and sets its own minimum price. Each receiving
object validates as if its final properties had been declared there.

## Decisions

### Spread inside an inline object

A spread can appear at the start, middle, or end of an inline object body,
including bodies used in arrays and unions. An object can contain only
spreads, and the existing trailing comma syntax applies.

```yaml
types:
    priced: "{ ...events, price: number(min(0)) }"
    imported: "{ url: url, ...events@./events.yaml, }"
```

- Apply entries left to right. A later spread can replace an earlier
  property, and an explicit property can replace one from an earlier spread.
  In `{ id: number, ...events }`, `id` comes from `events`. In
  `{ ...events, id: number }`, it is the locally declared `number`.
- Keep the existing error for two explicitly written literal properties
  with the same name, even across a spread. For example,
  `{ id: string, ...events, id: number }` is a grammar error.
- A spread takes a named reference without a constraint list, array modifier,
  description, or template expression. For example, `...events(required)`
  and `...events[]` are grammar errors. A postfix after the object, as in
  `"{ ...events }(required)"`, still applies to the containing property.
  Read the import path using the existing named-reference delimiters, so
  punctuation permitted inside a path keeps its usual meaning.

### Extending a YAML schema object

`$extends` is schema metadata, parallel to `$constraints`. It is accepted
wherever the SimplifiedSchema grammar parses an object mapping. This includes
a root schema, a mapping arm of a root or property union, a mapping-valued
entry in `types`, and a nested property mapping. The `types` table itself
remains a table of named declarations.

```yaml
types:
    order:
        $extends: [events, audit@./audit.yaml]
        total: number
```

- The value is one named reference or a nonempty sequence of named
  references. Each item follows the same operand rules as inline spread.
  Repeated references are allowed. Reject null, mappings, empty sequences,
  and non-string items at the offending value or item.
- Apply the referenced objects in list order, then apply the mapping's own
  properties. The placement of `$extends` among the mapping keys has no
  effect. For properties that can be written in either syntax, the merge
  order is the same as `{ ...events, ...audit, total: number }`.
- This intentionally reserves `$extends` inside authored SimplifiedSchema
  object mappings. Quoting the YAML key still invokes the directive, and a
  legacy declaration such as `$extends: string` should produce a diagnostic
  that explains the reservation and the invalid target. A legacy value that
  happens to be a valid composition operand will now compose. Check existing
  schema sources before rollout and use raw JSON Schema when an author
  needs to declare the literal property. Document this migration. A document
  can still contain a data key called `$extends` wherever its schema admits it.

### Which names can be used

Reuse the [existing named-type lookup rules](../../docs/topics/language/local-type-references.md).
They cover bare names, `Name@this`, file and embedded imports, names supplied
by a host, and forward references. To select a local type whose name is also
a built-in keyword, write `string@this`, for example. Resolve aliases until
they reach a concrete type. The target must be a single object with a
declared shape. An explicit empty shape `{}` qualifies.

Local names come from the same authored declarations as before composition.
For example, when a root without a `types` table extends another object, a
property brought in by that extension becomes a property of the root. It
does not also become a new local or exported type name. Inline root-union
arms keep their separate local scopes when there is no explicit `types`
table. A copied definition still resolves any references it contains in
the scope of the file or root-union arm that declared it.

### What is copied

- Replace a property as a unit, including its type, constraints, required
  marker, default, description, and examples. If a base defines
  `id: string(required)` and the receiver writes `id: number`, the result
  has an optional `id`. Replacing an existing literal key keeps its first
  insertion position so output order stays deterministic.
- Copy pattern keys too. A later entry for the same pattern replaces the
  whole definition. Use the same pattern identity as JSON Schema lowering
  for the receiving object, including the single catch-all `<string>`.
  Distinct patterns can still overlap and both apply. Literal keys retain
  precedence over all patterns. Lower patterns against the final literal
  key set, so newly added literal names receive the same treatment as in a
  handwritten object.
- Copy the members of the target object. Its own `$constraints`, postfix
  constraints, description, and required or default markers on the object
  itself stay with that object. The receiver uses its own object constraints,
  including `closed` and key counts. Its usual rules for extra keys apply to
  the **merged** shape. In particular, an inherited catch-all types extra
  keys, and an inherited pattern without a catch-all can close an otherwise
  open root under the existing pattern rules.
- Keep the winning property's origin. Relative `example(...)` paths,
  template origins, and imported default attribution continue to use the
  declaring file. If a local property replaces an imported one, its
  definition and origin replace the imported record together.

### Resolution and errors

Use the existing named-type resolver across inline, file-backed, and
embedded schemas. Count composition hops toward the existing named-reference
depth limit. Resolve each composition operand even when a later entry
replaces every property it contributes, so a missing or invalid target
still fails. A successful cross-file composition records the target and
its transitive file dependencies like ordinary imports. Embedded sources
keep their existing dependency behavior. Content-only loading keeps its
existing restriction on file reads.

Parse every authored member. After property replacement, only surviving
members contribute defaults, examples, or template work to the receiving
object. A target used independently as an ordinary named type continues to
validate under the existing rules for that use.

Report invalid syntax as a grammar error at the operand or the
`$extends` value. Report resolution errors at the individual reference,
with its target and the original cause. Use the existing named-reference
diagnostics where they fit and add a typed error for a resolved target
whose shape cannot be composed. In an editor, include the target
declaration as a related location when its source is available.

| Case | Example |
|---|---|
| Primitive or opaque object | `...label` where `label: string`, or `...meta` where `meta: object` |
| Union, including a union of objects | `...shape` where `shape: ["{ a: string }", "{ b: number }"]` |
| Array of objects | `...rows` where `rows: "{ a: string }[]"` |
| Shape supplied entirely by a template | `...dynamic` where `dynamic: "{{ $shape }}"`. A declared object with a template in a member still uses normal template resolution. |
| Unresolved name or file | `...missing`, or an import from a file that cannot be loaded |
| Cycle with no child value | `a: "{ ...b }"` and `b: "{ ...a }"`. Report the chain of references, including any alias hops. |

A composition reference describes the object at the receiving position.
That hop alone does not descend into a child value. An already valid
recursive named object can be a composition target if the composition
reference stays outside the recursive cycle. For example, extending
`node: "{ children: node[] }"` with `label: string` adds a label to the
receiving object while its `children` still contain the original `node`
type. See [Open Questions](#open-questions) for cycles that contain a
composition reference as well as a child-value reference.

### Output and callers

- JSON Schema lowering receives the merged properties, required keys, and
  patterns. Emit them as one object shape, because an `allOf` of independently
  closed objects would reject properties added by the other branches.
  Existing uses of `allOf` for individual constraints still apply, and
  ordinary nested named references can still use `$defs` and `$ref`.
- Keep the authored form for source locations and SimplifiedSchema YAML
  export, including the position of each inline spread and the span of each
  target. Reparse the export with the same file-resolution context and it
  must describe the same composition. Resolved queries and validation see
  the merged object. Lowering an unresolved composition operand must fail.
- In the SimplifiedSchema library,
  [`is_subtype`](../../lib/src/simplified/subtype.rs) checks whether one type
  is assignable to another. The comparison should give the same answer
  for a resolved composed object and a handwritten equivalent, in both
  directions. Use the definition-aware comparison when nested named
  references require a definition table.
- Schema diagnostics about a copied declaration retain that declaration's
  location where available. A validation error about a supplied value
  continues to point at that value. An error attributed to an inserted
  default retains the default's declaring origin.

### Editor support in DMLS

The Darkmatter language server should offer the existing named-type name
and import completions after `...`, in a scalar `$extends` value, and in
each flow or block sequence item. Completion should work while the operand
is incomplete. Apply the usual local scope and built-in collision rules,
and omit targets known to be ineligible for composition. Hover and
go-to-definition work for operands of either syntax. Editing an imported
target, including in an unsaved buffer, refreshes the receiving schema's
diagnostics and property completions through the existing dependency tracking.

## Out of scope

- Removing inherited properties. An override replaces a property's
  definition. The property remains declared.
- Composing arrays, unions, or document values. The expression language's
  object spread is a separate feature.
- Changing how trigger rules or document schema layers merge. A trigger's
  SimplifiedSchema payload can use composition before the existing merge.
  Host registration and its import restrictions are defined in
  `2026-11-22-host-registration-v2`.

## Open Questions

### Should a recursive cycle be allowed to pass through composition?

```yaml
types:
    branch: "{ ...base, label: string }"
    base: "{ children: branch[] }"
```

Computing the keys of `branch` only requires the keys of `base`. Checking a
child then returns to `branch` on a smaller value. The current recursion
rule relies on that descent. Supporting the example requires the resolver
to assemble object shapes without expanding child references indefinitely.

- **Allow the cycle if every loop descends into a child value.** This keeps
  the existing recursion rule and lets a reusable base describe children
  of the finished type. It requires resolving the object's member set
  independently of recursive child definitions and preserving the winning
  member's origin across files.
- **Reject cycles containing any composition hop.** This keeps the first
  implementation simpler and still lets an object copy members from an
  independently recursive base. It adds a restriction specific to
  composition and forces authors to repeat fields or change the model
  when children need the finished type.

**Recommendation:** Allow productive cycles under the existing child-value
rule. The two parts can be resolved separately because copying an object's
member set need only retain a reference to the child's definition. Cover
both the productive example above and a direct composition loop that must
fail. Apply the same rule to aliases and to composition across files.

## Documentation to update

- Explain both forms, duplicate and override behavior, pattern keys, object
  constraints, migration, and recursion in the
  [object and named-type guide](../../docs/topics/language/objects-and-named-types.md).
  Reconcile its older paragraph that still rejects nested YAML maps with
  the parser's existing support for them.
- Update the [language index](../../docs/topics/language/index.md),
  [local type lookup guide](../../docs/topics/language/local-type-references.md),
  [grammar reference](../../../darkmatter/docs/topics/schemas/parsing/grammar.md),
  and [diagnostics guide](../../docs/topics/diagnostics.md).
- Update the SimplifiedSchema repo skill and the DMLS schema-authoring
  caller skill, then run `just publish-skills` in the caller skill's area.

## Acceptance tests

- Spread at each position, replacement in both directions, a trailing
  comma, and the existing error for duplicate explicit literal properties.
- Scalar and list `$extends`, imports, invalid list items and empty lists,
  and independence from directive placement. Cover each supported YAML
  object position, including a property-union mapping arm.
- Local, imported, and aliased objects, built-in name collisions, empty
  targets, authored-only names, and every error case in the table.
- Whole-property replacement, including removal of inherited requiredness,
  defaults, and annotations. Verify winning example and default origins,
  and imported local references where caller and source reuse a type name.
  Check that a replaced member's example or template is skipped and that
  an invalid composition operand still fails even when all of its possible
  members would be replaced.
- Pattern replacement, overlap, catch-alls, and literal precedence. Check
  that receiver constraints apply to the final shape.
- Compare a composed schema with a handwritten equivalent through JSON
  Schema output, strict and soft validation, and subtype checks. Round-trip
  the authored YAML. Verify that composition leaves normal recursion and
  existing constraint lowering intact.
- Resolve composition from disk and from an embedded trigger payload, and
  verify dependency tracking for transitive imports. The Claudine
  [registration draft](../../../claudine/registration/composition.yaml)
  already uses this form. Its `common-attributes` target is missing
  from [the shared type file](../../../claudine/registration/shared.yaml).
  Use a self-contained equivalent as the acceptance fixture here. The
  host-registration work should supply the real declaration before
  enabling that trigger.
- In DMLS, cover completion for incomplete operands in both forms, hover,
  go-to-definition, a diagnostic on the failing operand, and refresh after
  an imported definition changes in an unsaved buffer.
