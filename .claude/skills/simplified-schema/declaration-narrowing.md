# Declaration narrowing: `not-empty`, `closed`, meta-type options, `claims`

Implementation notes for four language additions. The user-facing record is
in `simplified-schema/docs/topics/`; this file says where the code is and
which traps to avoid.

```yaml
$schema:
    kind: literal(yaml-schema; required)
    description: string(not-empty)
    categories: string[](not-empty)
    $schema: schema(required; property(kind, type-definition(forms(literal, enum); declared-required)))
    types: schema(mapping; not-empty)
    $constraints: { closed: true }
```

## Presence is `required` alone

- `PropertyDef::is_required` / `PropertyAtom::is_required` are the written,
  inherited, or hoisted `required` marker. Null admission (`admits_null`)
  decides only whether a written `null` satisfies the type; it never cancels
  `required` (`null(required)`, `[number(required), null]`, `Maybe(required)`
  for `Maybe: string | null` all demand the key). Tests:
  `tests/l1/required_presence.rs`.
- Phase projection is a lowering option, not an AST rewrite:
  `phase::lower(schema, definitions, phase)` →
  `convert.rs::Lowering::lists_required`. Value fragments keep the authored
  marker (a launch-unlisted `number(required)` still rejects `null`); only
  parent `required` arrays change. Bump `phase::PROJECTION_REVISION` with any
  change to what it emits.
- `declared-required` (format option) still also refuses a null-admitting
  alternative (`format.rs`); that is the option's own contract.

## `non-null` is removed; `not-empty` covers containers

- `non-null` in a type string or a `$constraints` block is a located
  `SchemaError::Grammar` with `grammar::NON_NULL_REMOVED` (kept in
  `is_constraint_keyword` so every spelling reaches that message). There is
  no `Constraint::NonNull`.
- `Constraint::NotEmpty` lowers per family (`convert.rs`): text (`string`,
  `alphanumeric`, `yaml`, `json`) → `pattern: "\S"`
  (`NOT_EMPTY_TEXT_PATTERN`); list (array-level list) → `minItems` through
  the same `Bounds` as `min(n)`; `object`, inline objects, dictionaries, and
  `$constraints: { not-empty: true }` → `minProperties` through
  `apply_object_arity`; `schema` / `type-definition` → `pattern`,
  `minProperties`, and `minItems` together (each applies only to its JSON
  type). `number`, `boolean`, `null`, `unknown` refuse it
  (`invalid_constraint`'s dedicated message). Trigger conditions judge it by
  the value's shape (`triggers/matcher.rs`). Tests:
  `tests/l1/not_empty_values.rs`, `declaration_grammar.rs`.

## `closed` (`Constraint::Closed`)

- Lives in `SchemaShape::constraints`; `shape_to_object_schema` lowers it to
  `additionalProperties: false`. Nested objects are already closed (no-op). A
  `<string>` catch-all still wins.
- `merge_layer` never copies a lower layer's `additionalProperties: false`;
  `validate_simple_object_root` now admits it. Closure follows the winning
  layer. Tests: `tests/l1/closed_roots.rs`.

## Meta-type options

- `schema(mapping)`, `schema(property(NAME, type-definition(...)))`,
  `type-definition(forms(...))`, `type-definition(declared-required)`,
  `type-definition(container)`; the
  grammar rejects them off their meta-type, on arrays, and on named-type
  references (`grammar.rs::check_declaration_option`).
- Judged parse-only on the authored declaration in `format.rs`
  (`SchemaOptions::judge`, `TypeDefinitionOptions::judge`): reference carriers
  and arms are skipped, named-type alternatives pass `forms` and `container`
  provisionally.
- `container` is `Constraint::Container` in the grammar and serializer; it
  judges with `simplified::narrowing::container_admission`
  (`Container` / `Provisional` / `NotContainer`); `denotes_container` is true
  only for `Container`. Both walk `query::flattened_alternatives`, drop `null`
  arms, and need one container arm. The YAML list `[string, number]` is a
  union, never a tuple. A type-definition *value* is parsed by
  `simplified::parse_type_definition_value` (the meta-type keyword, a
  `type-definition` default, Darkmatter's `load_data` contract, trigger
  matcher, and DMLS): a string holding a bracket list that the grammar
  rejects is re-read as that YAML list, so `"[string, number]"` is the same
  union; malformed text keeps the grammar error. A schema declaration's own
  string keeps `parse_property_definition`. Tests:
  `tests/l1/container_contracts.rs`.
- JSON payload (`convert.rs::semantic_type_fragment`): `true`, or
  `{"mapping": true, "property": [{"name", "declaration"}]}` /
  `{"forms": [...], "declaredRequired": true, "container": true}`. Tests:
  `tests/l1/declaration_narrowing.rs`, grammar in `declaration_grammar.rs`.
- Adding an option touches Darkmatter's lists too: the call-binding
  translator (`call_binding/translate.rs`), trigger match-safety and matcher
  (`triggers/{grammar,matcher}.rs`), and the `about.rs` descriptors that drive
  `md schema about` and DMLS completion.
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
