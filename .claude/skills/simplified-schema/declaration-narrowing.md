# Declaration narrowing: `non-null`, `closed`, meta-type options, `claims`

Implementation notes for four language additions. The user-facing record is
in `simplified-schema/docs/topics/`; this file says where the code is and
which traps to avoid.

```yaml
$schema:
    kind: literal(yaml-schema; required)
    description: string(non-null)
    categories: string[](non-null)
    $schema: schema(required; property(kind, type-definition(forms(literal, enum); declared-required)))
    types: schema(mapping; non-null)
    $constraints: { closed: true }
```

## `non-null` (`Constraint::NonNull`)

- A presence marker like `required`: read from either constraint list
  (`string[](non-null)` == `string(non-null)[]`), hoisted from union arms,
  moved from an object block's `$constraints: { non-null: true }` onto the atom
  (`simplified/mod.rs::parse_mapping_atom`), treated as `required` by
  subtyping and use-site merges (`resolve.rs::is_union_marker`, `is_flag`,
  `resolve/recursive.rs`).
- Lowering (`convert.rs::atom_to_schema`, `union_property_to_schema`) drops
  the optional `anyOf [{type: null}, X]` wrapper and never adds the property to
  `required`. No custom vocabulary.
- Over a null-admitting value it is a grammar error: in the parser
  (`PropertyDef::non_null_problem`) and again after named-type expansion
  (`resolve.rs::check_non_null_use`), since `Opt(non-null)` cannot be judged
  before `Opt` resolves.
- A `null` fails as `TypeMismatch` when the fragment has a `type`, as
  `ConstraintViolation` for `enum` and property unions. Soft validation never
  converts it. Tests: `tests/l1/non_null_properties.rs`.

## `closed` (`Constraint::Closed`)

- Lives in `SchemaShape::constraints`; `shape_to_object_schema` lowers it to
  `additionalProperties: false`. Nested objects are already closed (no-op). A
  `<string>` catch-all still wins.
- `merge_layer` never copies a lower layer's `additionalProperties: false`;
  `validate_simple_object_root` now admits it. Closure follows the winning
  layer. Tests: `tests/l1/closed_roots.rs`.

## Meta-type options

- `schema(mapping)`, `schema(property(NAME, type-definition(...)))`,
  `type-definition(forms(...))`, `type-definition(declared-required)`; the
  grammar rejects them off their meta-type, on arrays, and on named-type
  references (`grammar.rs::check_declaration_option`).
- Judged parse-only on the authored declaration in `format.rs`
  (`SchemaOptions::judge`, `TypeDefinitionOptions::judge`): reference carriers
  and arms are skipped, named-type alternatives pass `forms` provisionally.
- JSON payload (`convert.rs::semantic_type_fragment`): `true`, or
  `{"mapping": true, "property": [{"name", "declaration"}]}` /
  `{"forms": [...], "declaredRequired": true}`. Tests:
  `tests/l1/declaration_narrowing.rs`, grammar in `declaration_grammar.rs`.
- One payload builder and one judge, reused by hosts (Darkmatter's
  call-binding translator and trigger matcher): `simplified::
  declaration_options_payload(property, constraints)` and
  `format::{judge_schema, judge_type_definition}(constraints, instance)`.
  Never re-implement either in a host.

## `claims::claimed_values`

`claimed_values(&ResolvedSchema, "kind") -> Result<Vec<String>, ClaimProblem>`
reads the resolved lowering, so it closes the gaps the parse-only check
leaves. `ClaimProblem`'s `Display` is the same for every `ClaimCause`; match on
`cause`. Tests: `tests/l1/claimed_values.rs`.
