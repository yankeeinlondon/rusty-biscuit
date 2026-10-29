# Prerequisite Verification

This page records which behavior this feature needs from each prerequisite,
and whether that behavior exists in the code at `e116c6acc` (branch
`feat/pipeline`). The status comes from source and tests, not from spec
frontmatter or directory names (plan, Phase 1, rule 1). All paths are
relative to the repository root. `dm/` stands for `darkmatter/lib/src/`.

| Prerequisite | Needed for | Status |
|---|---|---|
| `2026-10-16-core-schema-types` | Presence (effective `required`) and the core types `unknown`/`any`, `null`, `alphanumeric`, `shell` | **SUPPLIED** |
| `2026-10-14-union-type-constraints` | The list-of-union representation `items: { anyOf }` for `(A \| B)[]` | **SUPPLIED** (representation); the inline spelling is this feature's work |
| `2026-10-18-fm-namespace` | Static `fm.x` / `fm["x"]` access as an alias of the document property; bare reserved roots keep their meaning | **SUPPLIED** |
| `2026-10-16-type-narrowing` | `TypeScope` (with its declared-type layer), the single `is_subtype` relation, the predicate fact registry, and branch scopes inside expressions | **SUPPLIED**; the declared layer is completed by this feature (P5W7 Variable projection), as the spec intends |

No prerequisite is MISSING, and none blocks a later plan task. The notes
below list the seams later tasks must reuse, and the comments those tasks
will make stale.

## 2026-10-16-core-schema-types: SUPPLIED

Evidence:

- **Effective presence**, the one shared calculation:
  - `PropertyDef::is_required` at `dm/markdown/schemas/simplified/types.rs:160`.
    It returns `marked && !self.admits_null()`, and its doc comment names
    lowering, triggers, materialization, completion, and phase projection
    as consumers.
  - `PropertyDef::admits_null` at `types.rs:173`.
  - `PropertyAtom::is_required` at `types.rs:330`.
  - `PropertyAtom::marks_required` at `types.rs:336`. It hoists `required`
    from union arms, which covers inherited and postfixed markers.
  - `PropertyAtom::admits_null` at `types.rs:346`.
  - Lowering reads the shared calculation: `let required =
    atom.is_required();` at `dm/markdown/schemas/simplified/convert.rs:415`.
- **Core types:**
  - `SimplifiedType::{Unknown, Null, Alphanumeric, Shell}` at
    `types.rs:478,480,484,487`.
  - `from_keyword` maps `"unknown" | "any"` to `Unknown` at `types.rs:558`.
  - Lowering: `convert.rs:1071-1074` (`unknown_fragment`, `null_fragment`,
    `alphanumeric_fragment`, `shell_fragment`).
  - `NumberLike`, `Boolish`, `Json`, `Yaml`, and `File` already exist as
    `SimplifiedType` variants (`types.rs:437-457`).
- **`EffectiveSchema.simplified`**, the authored SimplifiedSchema projection
  that the variable projection reads: `dm/markdown/schemas/mod.rs:845`.
- **Existing tests:**
  - `darkmatter/lib/tests/l1/schemas_effective_presence.rs`, including
    `inherited_and_postfixed_required_is_dropped_by_a_null_arm` (`:62`)
  - `schemas_core_type_grammar.rs`
  - `schemas_core_type_values.rs`
  - `darkmatter/dmls/tests/l1/core_schema_types.rs`

For later tasks:

- **Criterion 50** ("`[string(required), number]` has no implicit null arm;
  `[string(required), null]` stays optional; `unknown(required)` agrees
  with lowering") follows from `PropertyDef::is_required`. P5W7 Variable
  projection must call it and must not recompute presence.
- **The function-parameter null rule is not a prerequisite.** Under that
  rule, `unknown` at a parameter does not admit null. It belongs to this
  feature (P3W4 Call binder). Document-property `admits_null` must not be
  reused for parameters as-is: it returns `true` for `unknown`
  (`types.rs:346-353`).

## 2026-10-14-union-type-constraints: SUPPLIED (representation)

Evidence:

- **One representation.** `TypeExpr::Union(Vec<PropertyAtom>)` on an atom
  with `is_array` is "the one representation of an array whose items are a
  union": `dm/markdown/schemas/simplified/types.rs:217-250`, doc comment at
  `:232-249`.
- **Lowering to `items: { anyOf }`:**
  - The union arm builds `json!({ "anyOf": any_of })` at `convert.rs:451-482`.
  - The array wrapper puts that fragment under `items` at `convert.rs:513-517`.
  - An arm `default` on an array is rejected at `convert.rs:460-467`.
- **Union-aware consumers:**
  - `CoercionTarget::ItemUnion` at `dm/markdown/schemas/coerce.rs:92-100`.
  - `item_union_target` at `coerce.rs:229-235`.
  - Existing tests in `darkmatter/lib/tests/l1/`: `schemas_union_postfix.rs`,
    `schemas_union_coerce.rs`, `schemas_union_schema_matrix.rs`,
    `schemas_union_typed_consumers.rs`, `schemas_union_report.rs`,
    `schemas_union_rewrite.rs`. Also `darkmatter/dmls/tests/l1/union_item_editor.rs`.

What is not supplied, and is this feature's work rather than a gap:

- **The passive parser never produces `TypeExpr::Union`.** Only the resolver
  does: `dm/markdown/schemas/resolve.rs:1349,1628,1707,2525,2619,2685`. The
  doc comment states it at `types.rs:234-235` ("Only the resolver produces
  this variant"). **P2W2 Grammar descriptors** must build the same variant
  from the inline `(A | B)[]` spelling to meet criterion 47's "identical
  JSON Schema". That task must also update the `types.rs:234-235` comment,
  which will otherwise drift.
- **Item-union coercion today commits only when exactly one arm validates**
  (`coerce.rs:93-96`). Root unions try arms "in index order"
  (`coerce.rs:331-334`). Both differ from this spec's union rule, which uses
  exact match, then number-over-boolean and JSON-over-YAML, and is
  order-independent. P3W3 Conversion core and P3W4 Schema adapter replace
  them, as planned. This is a behavior change, not a prerequisite gap.

## 2026-10-18-fm-namespace: SUPPLIED

Evidence:

- **Namespace API** in `dm/markdown/compose/expression/fm_namespace.rs`:
  - `FRONTMATTER_ROOT = "fm"` at `:23`
  - `FmPath` at `:27`
  - `fm_namespace_path` at `:36`
  - `strip_fm_prefix` at `:52`
  - `literal_index_key` at `:62-69`. It returns the key for `fm['a']` and
    `None` for a computed index such as `fm[name]`.
- **Static resolution, the design-time side**, in
  `dm/markdown/compose/expression/scope/projection.rs`:
  - `DeclaredTypes::subject` at `:184-205`. It maps `fm["x"]` through
    `literal_index_key` to `BindingId::Document("x")` (`:191-196`).
  - `classify_variable` at `:231-249`. It maps `fm.x` to `Document { key }`
    (`:234-236`) and bare `fm` to `WholeFrontmatter`. A bare reserved root
    stays `Namespace` (`:241-242`; `reserved_root_descriptors` at
    `dm/markdown/compose/expression/catalog/roots.rs:95`).
  - The module documentation states that bare `x` and `fm.x` are the same
    `BindingId::Document` (`projection.rs:33`).
- **Runtime resolution:**
  - `dm/markdown/compose/frontmatter_interpolation.rs:153-154`
  - `dm/markdown/compose/conditions.rs:369-370`
  - `dm/markdown/compose/context/effective_state.rs:233`
- **Existing tests:**
  - `darkmatter/lib/tests/l1/fm_namespace_contract.rs`, including
    `qualified_and_bracket_reads_select_members` (`:99`) and
    `an_absent_qualified_read_is_null_and_never_falls_back` (`:109`)

For later tasks: **P5W7 Variable projection** gets the `fm.flag` /
`fm["flag"]` parity for criterion 50 by reading types through
`TypeScope::type_of_expr`. That path goes through `DeclaredTypes::subject`,
so no new alias logic is needed. A nested path (`fm.meta.count`) yields
`DeclaredType::NestedProperty` (`projection.rs:113-115`, `:278`), which is
the `unknown` the spec asks for.

## 2026-10-16-type-narrowing: SUPPLIED (declared layer completed by this feature)

The fix lives in the active `claudine/fixes/2026-10-16-type-narrowing/`. Its
frontmatter says `status: draft-spec` alongside `implemented: true` and
`completed: true`. The code is present.

Evidence:

- **`TypeScope`** at `dm/markdown/compose/expression/scope/mod.rs:133`:
  - `unguarded` at `:155`
  - `assume` at `:164`
  - `type_of` at `:174`
  - `type_of_expr` at `:197`
  - `join` at `:238`
  - Re-exported from `dm/markdown/compose/expression/mod.rs:111-113`.
- **Declared-type layer:**
  - `DeclaredTypes` in `scope/projection.rs`, with `new` at `:152` and
    `declared` at `:276`.
  - `DeclaredType` at `:105-125`.
  - A root union whose arms disagree, or a missing SimplifiedSchema, gives
    `NoProjection` (`:252-273`). This matches the "raw and mixed root union
    project `unknown`" part of criterion 48.
- **The single subtype relation:** `is_subtype` at
  `dm/markdown/schemas/simplified/subtype.rs:53`. It is exported at
  `simplified/mod.rs:26` and re-exported from `dm/markdown/schemas/mod.rs:128`.
- **Predicate fact registry:**
  - `PredicateFact` at `scope/facts.rs:20-31`. True-iff variants are
    `Family` and `Null`. True-implies variants are `Empty` and `FileExists`.
  - `PREDICATE_FACTS` at `facts.rs:34-42` registers `is_string`,
    `is_number`, `is_array`, `is_object`, `is_null`, `is_empty`, and
    `file_exists`, keyed by canonical name through
    `canonical_function_name` (`facts.rs:13`).
- **Equality and literal narrowing:** `facts.rs:61` (`Expr::Comparison`)
  and `literal_operand` at `:162`.
- **Branch scope inside one expression** (rows g and h, and dead-branch
  walking):
  - `branch_scope` at `dm/markdown/compose/expression/absence.rs:162`,
    re-exported at `expression/mod.rs:86`.
  - Block guards: `BodyGuards::scope_at` at
    `dm/markdown/compose/page_blocks/guards.rs:124`.
  - DMLS already consumes the scope: `with_site_scope` at
    `darkmatter/dmls/src/providers/dsl.rs:405-422`.

What this feature still completes, by design (spec "Expression-variable
Type Projection" and decision 81):

- `DeclaredType::Property(PropertyDef)` (`projection.rs:107`) carries the raw
  definition. Nothing yet turns it into the spec's projected type, for
  example `maybe: number | null` for an optional property. **P5W7 Variable
  projection** adds that projection on top of this layer. It must not add a
  second store.
- Predicate facts live in a static table in `scope/facts.rs`. They are not
  on `FunctionBinding` registration (`dm/markdown/compose/expression/functions/paths.rs:6`
  shows the registration shape). Plan rule 4 says registration facts are
  shared between binding and classification. Spec line 160 says the
  migration "keeps every predicate's fact in that registry". **P3W4 Call
  binder** and **P4W6 Registry integration** should reference
  `PREDICATE_FACTS` rather than duplicate it.
- The fix's own log lists these as planned, not claimed
  (`claudine/fixes/2026-10-16-type-narrowing/implementation-log.md:509-511,704-712`):
  - nested-union and discriminant projection
  - function-result narrowing
  - host types for `seq`/`loop`/`group`
  - argument binding
  These are outside this feature's scope too, or are this feature's own
  work (argument binding). Rows f and i stay conservative.

Minor comment drift observed, not fixed here because this is an
inventory-only phase: `PredicateFact::FileExists`'s comment
(`facts.rs:28-29`) says "today's handler rejects every other original type".
The handler, `file_exists_fn` at
`dm/markdown/compose/expression/functions/mod.rs:1640-1643`, returns
`Ok(false)` for any non-string, with no rejection. The fact itself, "true
implies a present string", is still sound. **P4W5 Context handlers** should
reword the comment when it moves `file_exists` onto bound arguments.

## Working-tree state

`git status --short` shows these uncommitted entries:

```
 M claudine/features/2026-10-27-darkmatter-lifecycle-ownership/spec.md
?? claudine/features/2026-10-27-darkmatter-lifecycle-ownership/review-1.md
?? claudine/features/2026-10-28-schema-enhancements/implementation-log.md
```

`git log --oneline -15` runs from `e116c6acc` (Phase 6 close for
`darkmatter-lifecycle-ownership`) back to `cfdfe7a13`. The most recent code
commits are `f0cea85cf`, `75b0cf446`, `8a5559d6d`, and `b675360b6`, all
lifecycle and schema-catalog ownership moves into Darkmatter.

**No uncommitted edit touches a prerequisite code area.** The modified and
untracked files are planning documents only. Plan rule 1 warns of "ongoing
prerequisite edits". In this worktree, none are pending. The last commits to
touch `scope/`, `subtype.rs`, `fm_namespace.rs`, and `convert.rs` are
`f0cea85cf`, `b59ba28e0` (typed equality), and `e3eb8e7ef` (guard scopes
through composition consumers).

## Other facts noted during verification

- A draft function catalog already exists at
  `claudine/docs/schemas/partials/functions.yaml`. Its header reads "Draft
  function contracts for review during 2026-10-28-schema-enhancements".
  The live catalog is still `darkmatter/docs/schemas/expression-functions.yaml`.
- These read `expression-functions.yaml` today:
  - `dm/markdown/compose/expression/catalog/{mod,parser}.rs`
  - `darkmatter/dmls/src/overlay/mod.rs`
  - the tests `darkmatter/lib/tests/l1/{shipped_schema_corpus,meta_schema_phase1,predict_conflicts}.rs`

  P4W5 Catalog replacement and P4W6 Consumer migration must switch all of
  them. The draft's fixed transcluded path is preserved per plan.
- `darkmatter/schemas/partials/` exists. It has no `functions.yaml` yet.
