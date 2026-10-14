---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Postfixed aliases bypass union operation checks"
    - "high: Nested postfixed unions lose coercion and tagged error reporting"
    - "high: Eager before the array suffix remains unsupported"
    - "high: Inherited eager timing does not hoist to union arrays"
observations:
    - "Existing cleanup safety gate limits the combined sibling-repair example"
    - "Array defaults remain unavailable through the authored grammar"
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-12T08:24:14-07:00
spec: 2026-10-14-union-type-constraints/spec.md
implemented: true
next: 2026-10-14-union-type-constraints/review-2.md
implemented_by: claude/opus
log: claudine/features/2026-10-14-union-type-constraints/log.md
description: "A **feature** review of `2026-10-14-union-type-constraints/spec.md`"
feature: 2026-10-14-union-type-constraints/review-1.md
---

# Review of Union Type Constraints

The feature is **not production ready**. Its main Content Policy example works,
but the four findings below leave supported reference forms and consumer
behavior inconsistent with the specification. No human-only verification is
needed to reproduce these findings.

## Findings

### High: Postfixed aliases bypass union operation checks

**Authority: spec, “What a use site may add to a union-typed reference,” and
acceptance criteria 2 and 7.** Value constraints without `[]` must be rejected
by name; `[]` must reject an array arm “after resolving and flattening imported
arms, including multi-hop references.” Rejections must retain the named type,
definition, and use-site information.

**Defect class:** operation checks depend on the resolver's intermediate
representation rather than whether the resolved target is a union.

In the darkmatter package,
[apply_import_postfix](../../../darkmatter/lib/src/markdown/schemas/resolve.rs#L1775)
checks unions only when expansion returns `ExpandedDef::Union`. A previous
`required`, `eager`, or description postfix instead produces a single atom
holding `TypeExpr::Union`. A later reference takes the ordinary single-type
path. The lowering code then ignores forbidden value constraints on that atom.

This realistic reusable definition reproduces the bypass:

```yaml
kind: schema
types:
  choice: [string, number]
  required_choice: choice(required)
  list_choice: ["string[]", number]
  required_list_choice: list_choice(required)
```

With that file named `types.yaml`, `md schema validate --format json doc.md`
returns exit 0 for both `p: required_choice(max(2))@./types.yaml` with `p: abcd`
and `p: required_list_choice[]@./types.yaml` with `p: [[a], 3]`. Direct references
to `choice(max(2))` and `list_choice[]` correctly return schema errors, exit 2.

For example, the first accepted-but-forbidden declaration is:

```markdown
---
$schema:
  p: required_choice(max(2))@./types.yaml
p: abcd
---
Body
```

The sweep covered the two expansion paths, scalar and container operation
checks, inherited defaults, and bare, same-file, cross-file, and multi-hop
references through the public resolver and shipped CLI:

| Site / route | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Direct union, resolver | `min`, `max`, `pattern`, `default`, `unique`, `integer` without `[]` | Named `UnionReference` rejection for each | Same; clean control |
| Alias after `required`, resolver | Same six constraints | All accepted; constraints disappear from the lowered property | Reject each with `UnionReference` |
| Alias after description, resolver | `pattern(^x)` | Accepted; pattern discarded | Reject with `UnionReference` |
| Direct union, resolver | `[](pattern(^x))`, `[](integer)` | `UnionReference` with container scope | Same; clean control |
| Postfixed alias, resolver | Same two container constraints | Generic `Convert` error, without union definition/use-site information | Structured union rejection |
| Direct union, resolver and CLI | Array alternative plus `[]` | Rejected | Same; clean control |
| Postfixed alias, bare / `@this` / cross-file / multi-hop | Same array alternative plus `[]` | Accepted as an array of arrays or numbers | Reject every route |
| Direct union, resolver | Inherited scalar default plus `[]` | `UnionReference::ScalarDefault` | Same; clean control |
| Postfixed alias, resolver | Same inherited scalar default | Generic `Convert` error | Structured rejection retaining provenance |
| Both expansion paths, resolver | `[](min(1); max(3); unique)` and `required; eager; generated` | Bounds, uniqueness, and markers retained | Same; clean controls |

Apply the union operation rules to resolved non-array union atoms as well as
expanded arm lists, preserving the alias's existing markers, description, and
origins. Add alias cases to the existing public-result tests; checking only
direct union definitions leaves this entire class undetected.

### High: Nested postfixed unions lose coercion and tagged error reporting

**Authority: spec, “Lowering,” “One representation,” and acceptance criteria
4 and 7.** “A union nested inside a union is flattened into one `anyOf`,” and
literal-discriminant narrowing and consumer behavior must also work for union
items. Coercion must evaluate each candidate independently.

**Defect class:** nested union wrappers survive lowering, but consumers that
choose candidates or tagged arms do not consistently descend those wrappers.

The darkmatter package's
[union_arm_fragments](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs#L264)
retains a postfixed union as one nested `anyOf` arm. Its
[coerce_property_union](../../../darkmatter/lib/src/markdown/schemas/coerce.rs#L482)
does not obtain a coercion target for that generic nested union, and
[collect_discriminated_arms](../../../darkmatter/lib/src/markdown/schemas/validate.rs#L1375)
cannot select a literal tag hidden inside it. This also affects the same new
form used as a property.

```yaml
kind: schema
types:
  toggle: [boolish, number]
  direct: [toggle, object]
  nested: [toggle(required), object]
  tagged:
    - "{kind: literal(a; required), a: string(required)}"
    - "{kind: literal(b; required), b: string(required)}"
  direct_tag: [tagged, number]
  nested_tag: [tagged(required), number]
```

The public resolver, coercion, validation, cleanup, normalization, and
completion APIs were exercised with copies of the existing union fixtures
modified to add a postfix to an inner reference. DMLS was also exercised
through its JSON-RPC interface:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| JSON Schema lowering | `[toggle(required), object]` used with `[]` | Outer `items.anyOf` contains another `anyOf` | One flattened list of alternatives, retaining metadata |
| Coercion, property and item | `direct` / `direct[]`, value `"TRUE"` | `true` / `[true]` | Same; clean controls |
| Coercion, property and item | `nested` / `nested[]`, same value | `"TRUE"` / `["TRUE"]` remains | Coerce the unique boolish candidate |
| Shipped `md compose --frontmatter` | Direct and nested union arrays, `["TRUE"]` | Both exit 0; direct emits `[true]`, nested retains `['TRUE']` | Both emit boolean items |
| Tagged reporting, property and item | `direct_tag` / `direct_tag[]`, `{kind: a}` | Reports missing `a` | Same; clean controls |
| Tagged reporting, property and item | `nested_tag` / `nested_tag[]`, same object | Generic union failure | Report the selected arm's missing `a` |
| Library completion | Direct and nested tagged items, `{kind: a}` | Selects the `a` arm and offers its keys | Same; clean |
| DMLS completion and hover | Same nesting, incomplete and valid tagged items | Offers `a`; valid hover identifies alternative 1 of 3 | Same; clean |
| DMLS diagnostics | Same incomplete tagged item | Nested form publishes a generic union failure; direct form suppresses the unfinished missing-field diagnostic | Preserve the direct form's editor behavior |
| Eager-file normalization | Direct and nested `file(eager)` union items, `./a.md` | Both normalize to `a.md` | Same; clean |
| Cleanup | Direct and nested tagged items with an extra key | Neither carries a repair deleting that key | Same; clean |

The new
[nested-union test](../../../darkmatter/lib/tests/l1/schemas_union_postfix.rs#L331)
explicitly expects nested lowering, so its passing result does not prove the
specified flattening behavior. Fix the representation/lowering and candidate
walks together, preserving property timing separately from file-arm eagerness.
Add paired property/item tests for a postfixed inner union, including both
coercion and tagged failures.

### High: Eager before the array suffix remains unsupported

**Authority: spec, “What a use site may add to a union-typed reference.”**
“Universal `eager` is accepted before or after `[]` as property timing.”
Acceptance criterion 7 also requires coverage of pre-array versus container
constraints.

**Defect class:** an explicitly supported postfix order is still rejected by
the passive grammar before resolution can interpret it.

The darkmatter package's
[named-reference parser](../../../darkmatter/lib/src/markdown/schemas/simplified/grammar.rs#L1048)
still rejects `U(eager)[]`. The implementation log records this as an existing
grammar restriction, but neither that log nor the changed documentation
fulfills the specified new operation.

| Site / reference form | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Public resolver, bare | `U(eager)[]` | Grammar error: `[]` must precede constraints | Array of union with property timing |
| Public resolver, same file | `U(eager)[]@this` | Same grammar error | Same accepted operation |
| Public resolver and CLI, cross-file | `U(eager)[]@./types.yaml` | Grammar error; CLI exit 2 | Same accepted operation |
| Public resolver and CLI, after `[]` | `U[](eager)` | Accepted; CLI exit 0 | Same; clean control |
| Public resolver, unsupported pre-array value constraint | `U(max(2))[]` | Grammar error | Named structured union constraint rejection |

Accept the authored ordering passively, then let resolution distinguish the
universal marker from unsupported value constraints. Verify serialization and
resolution for all three reference forms. This requires Level-1 coverage, not
a terminal window.

### High: Inherited eager timing does not hoist to union arrays

**Authority: spec, “What a use site may add to a union-typed reference,” and
acceptance criterion 7.** Inherited markers “survive and hoist to the containing
property, including when that property is an array,” and the required boundary
coverage includes “inherited required/generated/eager.”

**Defect class:** phase projection treats an inherited union timing marker as
item-only metadata and loses the containing property's launch requirement.

For `U: [string(required; eager), number]`, the darkmatter package's public
`EffectiveSchema::validate_for_phase` accepts a missing `p: U[]` at launch,
although the same union used as `p: U` is required then. In
[atom_has_property_eager](../../../darkmatter/lib/src/markdown/schemas/phase.rs#L166),
the recursive arm check is explicitly skipped for arrays. The subsequent phase
projection removes the inherited required marker. Claudine's
[constraint classifier](../../../claudine/lib/src/composition/schema/classify.rs#L100)
does consult the arms for array timing, so the packages also disagree on this
new shape.

Each public-API case supplied `U` itself and omitted only `p`, avoiding an
unrelated missing-definition property:

| Site / form | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Darkmatter launch projection | `p: U` and `p: U(required)` | Missing `p` rejected | Same; clean controls |
| Darkmatter launch projection, bare | `p: U[]` | Missing `p` accepted | Reject due to inherited required/eager |
| Darkmatter launch projection, same file | `p: U[]@this` | Missing `p` accepted | Reject |
| Darkmatter launch projection, cross-file | `p: U[]@./types.yaml` | Missing `p` accepted | Reject |
| Darkmatter launch projection, explicit container timing | `p: U[](eager)` | Missing `p` rejected | Same; clean control |
| Darkmatter completion projection | All six forms | Missing `p` rejected | Same; clean |
| Claudine classifier, inspected implementation | Array union with inherited eager | Recursive constraint query retains eager | Consistent with the spec; clean |

The new
[inherited-timing test](../../../darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs#L292)
asserts the opposite of the specified hoisting rule. Correct the property
timing query and that expectation while retaining the existing rule that a
lazy file arm does not gain a file-existence check from property timing.

## Verification

| Requirement | Strongest relevant evidence checked | Assessment |
|---|---|---|
| Content Policy acceptance and array lowering | L1 public resolver/validator and hermetic CLI tests | Main example passes |
| Presence, reference forms, bounds, defaults, cycles, provenance | L1 resolver tests and additional public-API/CLI probes | Alias and timing findings above |
| Validation, cleanup, rewrite, coercion, tagged reporting | L1 public consumer tests and paired nested-union probes | Nested coercion/reporting finding; existing cleanup limitation below |
| Completion, hover, schema errors, `md schema about` | L1 library, CLI, and DMLS JSON-RPC tests | Direct forms covered; nested diagnostics differ |
| Existing-schema byte identity | Implementation log records 347/347 identical entries; inspected baseline capture and current snapshot/corpus tests | Historical comparison was not independently recaptured in this review |
| Passive validation, composition, repeated normalization | L1 hermetic CLI pipeline tests | Covered; no windows required |
| Required local gates | `KACHE_DISABLED=1 just test` and `KACHE_DISABLED=1 just lint`, in `darkmatter/` | 9,440 passed, 11 skipped; lint passed |
| Test compilation and tier selection | Declared L1 modules/targets inspected; `just check-tier-coverage darkmatter` | No stranded tier tests |

The schema-input robustness tests cover the YAML `kind`, `types`, union
definition, document `$schema`, and property declaration surfaces with positive
controls and edited-field cases. The CLI document-data matrix covers list and
object data. No new file-format deserializer was introduced. All new tests are
declared in their consolidated targets and use L1 names. These requirements
need neither terminal-emulator rendering nor OS keyboard injection.

Additional review probes used temporary directories and the public APIs or
shipped binaries. Their temporary test modules were removed afterward. No
production code, test code, dependencies, or CI scheduling was changed by this
review.

## Observations

### Existing cleanup safety gate limits the combined sibling-repair example

The existing darkmatter cleanup gate allows a quoting repair only when there
is one raw problem and the complete document can validate afterward. It is
unchanged from the prerequisite baseline. The public cleanup API still repairs
`label: 1.20` beside valid union items, but offers zero repairs beside either a
tagged or untagged invalid item, or both. The new tests explicitly preserve
that limitation, while acceptance criterion 4 describes repairing the sibling
alongside invalid items. Treat this as a recorded contract discrepancy for the
author: the existing limitation is conservative, and a broader repair-proof
mechanism would be large relative to the benefit. Retaining and documenting
the limitation is preferable to casually weakening that safety gate; the
acceptance summary should not present the combined example as verified.

### Array defaults remain unavailable through the authored grammar

The pre-existing parser reads `default([a])` as the string `"[a]"`, so authored
schemas cannot reach the feature's compatible-array-default lifting branch.
The new tests establish parity with ordinary arrays and scalar-default
rejection, but not authored compatible inherited defaults. The documentation
now describes this limitation accurately. Keep it documented rather than
expanding the default-value grammar in this review cycle.
