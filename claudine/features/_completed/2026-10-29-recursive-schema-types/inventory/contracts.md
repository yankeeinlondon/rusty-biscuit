# Reference API Contracts (Phase 1, Wave 1)

These are the shared interfaces every later wave of
`2026-10-29-recursive-schema-types` builds on. They were fixed on 2026-10-23
against the tree at `e5976e3f0`, after `2026-10-28-schema-enhancements` and
`2026-10-14-union-type-constraints` were implemented. Parallel waves (4, 5, 6,
7) code against the names and semantics here; a wave that needs a different
shape changes this file first and records why in the implementation log.

The consumer inventory these contracts were written against is
[consumers.md](consumers.md).

## 1. Starting point in the code

| Today | Where | What changes |
| --- | --- | --- |
| `TypeExpr::{Primitive, InlineObject, Imported, Union, Tuple, Function}` | `darkmatter/lib/src/markdown/schemas/simplified/types.rs:218` | gains `Ref(DefKey)` |
| `NamespaceKey::{Root, File, Embedded, Shipped}` (private) | `schemas/resolve.rs` (above `ImportEngine`) | `Root` is one sentinel shared by an inline `$schema`, every root-union arm, and an embedded inline schema; it splits into distinct identities (section 2) |
| `Namespace.named_types: Arc<IndexMap<String, PropertyDef>>`, shared through `namespace_cache` / `embedded_cache` / `shipped_cache` | `resolve.rs` | unchanged: the **unexpanded** tables stay shared; the new definition table holds **expanded** bodies |
| `ImportEngine.stack` + `expanded_cache` + `MAX_IMPORT_DEPTH` (32) | `resolve.rs::expand_import` | a revisit of a frame on the stack is a back-edge, classified before the depth check (section 5) |
| `SchemaError::ImportCycle { chain: String }` | `schemas/errors.rs:156` | kept for acyclic chains that exceed `MAX_IMPORT_DEPTH`; a rejected named-type cycle becomes the new typed error (section 6) |
| `SchemaReferenceSite { source: SchemaSourceId, path: SchemaSourcePath }` | `simplified/source.rs:148` | reused unchanged as the identity of every reference site; spans are projected exactly as `UnknownNamedType` projects them today |
| `ResolvedSchema`, `EffectiveSchema` | `resolve.rs:68`, `schemas/mod.rs:849` | each gains a definitions table (section 3) |
| `validate_simple_object_schema` root allow-list | `resolve.rs` (`ALLOWED_KEYS`) | admits root `$defs` only (section 8) |

Cutoffs recorded as the "old" values the recursion work replaces or keeps:

| Constant | Value | Where | Fate |
| --- | --- | --- | --- |
| `CYCLE_EXPANSIONS` | 4 | `schemas/json_shape.rs:61` | replaced for supported recursive references by reusable shape handles; raw cycles outside the supported shape keep it |
| `MAX_REFERENCE_DEPTH` (JSON translator) | 32 | `schemas/json_shape.rs:56` | kept as a per-path guard for raw references |
| `REFERENCE_BUDGET` | 4,096 | `schemas/json_shape.rs:65` | kept |
| `MAX_REFERENCE_DEPTH` → `TranslateError::TooDeep` | 32 | `compose/expression/call_binding/translate.rs:46`, raised at `:273` | no longer reached by a supported recursive type; kept for acyclic inline nesting |
| `MAX_IMPORT_DEPTH` | 32 | `resolve.rs` | kept for acyclic import chains only |
| `MAX_REFERENCE_DEPTH` (`$schema` file chain) | 32 | `resolve.rs:508` | unchanged (`ReferenceCycle` / `ReferenceDepthExceeded`) |
| `MAX_REFERENCE_DEPTH` (signatures) | 16 | `simplified/signature.rs:20` | unchanged |
| `BindOptions.max_depth` | 64 | conversion engine | the "existing document resource limit" for deep conversion |

## 2. Definition identity: `DefKey`

```rust
/// The identity of one named definition: the namespace that declares it and
/// its name there. Public, `Clone + Eq + Ord + Hash`, never parsed by callers.
pub struct DefKey {
    pub namespace: DefNamespace,
    pub name: String,
}

pub enum DefNamespace {
    /// The inline `$schema` mapping of the document being prepared.
    InlineRoot,
    /// Inline arm `index` of a root union (`$schema: [ {...}, ./b.yaml ]`).
    RootUnionArm(usize),
    /// A schema file, by the canonical path the resolver loaded it from.
    File(PathBuf),
    /// Inline arm `index` of a schema file whose declaration is a root
    /// union (added in Wave 2, see below).
    FileRootUnionArm(PathBuf, usize),
    /// An embedded document (virtual path), including an embedded inline
    /// schema authored inside one (`importer`).
    Embedded(String),
    /// A shipped partial reached through `@darkmatter:<stem>`.
    Shipped(String),
    /// A raw JSON Schema `$defs` table, by the identity of the document that
    /// carried it (a file's canonical path, or the merge layer that supplied
    /// an in-memory value, e.g. the caller baseline).
    Raw(RawScope),
}
```

- **Wave 2 change:** `FileRootUnionArm(path, index)` was added. The inline
  arms of a root-union schema *file* each resolve `@this` against their own
  properties; they shared `File(path)` before, so two arms of one file that
  each declare `item` would have collided on one `DefKey` once both arms
  allocate into the union's single table. `Raw(RawScope)` is not in the code
  yet: Wave 3 adds it with raw merge ingress, the only producer.
- `NamespaceKey` becomes `DefNamespace` (or converts into it 1:1); the four
  existing variants keep their meaning, and `Root` is replaced by
  `InlineRoot` / `RootUnionArm(i)`. An embedded inline schema is
  `Embedded(importer)`, as its `@this` already resolves there.
- `SchemaSourceId` is **not** widened: it still says where text came from
  for spans (`InlineDocument` covers both inline cases). `DefNamespace` says
  which table a name belongs to.
- A `DefKey` never appears in output. Generated JSON uses the allocated key
  (section 4).

## 3. Definition table and ownership

```rust
pub struct Definition {
    /// The reusable value shape, expanded; nested uses of cyclic definitions
    /// are `TypeExpr::Ref`. Carries no presence or timing (section 7).
    pub body: PropertyDef,
    /// Where the definition was declared (source + `types.<name>` path).
    pub origin: SchemaReferenceSite,
    /// The generated `$defs` key (section 4).
    pub json_key: String,
}

/// Finite, immutable after resolution; shared behind `Arc`.
pub struct DefinitionTable {
    entries: IndexMap<DefKey, Definition>,   // allocation order
    by_json_key: BTreeMap<String, DefKey>,   // explicit identity/key map
}
```

- Only **cyclic** definitions (members of a reference-graph SCC with at least
  one edge, including a self-edge) enter the table. Acyclic definitions
  inline exactly as today, and an acyclic alias that names a cyclic target
  inlines to a `Ref` of that target.
- Ownership: `ResolvedSchema.definitions: Arc<DefinitionTable>` and
  `EffectiveSchema.definitions: Arc<DefinitionTable>` (the merged table); DMLS
  bundles carry the `EffectiveSchema`'s `Arc`. Every other carrier of a typed
  shape pairs it with the table its `Ref`s point into:
  `ShippedBaseline.schema`, `TriggerLayer.shape` (and
  `EffectiveSchema.trigger_shapes`), and DMLS `SchemaBundle.extension_shapes`
  (consumers.md §2). A merged effective table is the union of these, re-keyed
  per section 4, so one lookup serves the merged shape. Bodies refer to each other by
  `DefKey`, never by `Arc`, so no ownership cycle can form and dropping the
  table frees everything.
- An empty table is the acyclic case: lowering emits no `$defs` key, which is
  what keeps the golden corpus byte-identical.
- Imported-file dependencies: every file read while discovering a cyclic
  group, including a file reached only through the cycle-closing edge, is
  added to `ResolvedSchema.imports`, so the existing dependency-hash
  invalidation covers it.
- **As built (Wave 2):** `DefinitionTable` exposes `is_empty`, `len`,
  `get(&DefKey)`, `key_for_json(&str)`, `iter()` (allocation order), and
  `outer(&DefKey) -> Option<DefinitionOuter>` (`Object`, `Array`, `Tuple`,
  `Union`, `Other`, looking through alias bodies); `insert` is crate-private
  (the resolver allocates). `definition_pointer(json_key)` builds the escaped
  `#/$defs/…` reference. `ResolvedSchema.definitions` is a public field;
  `EffectiveSchema` keeps the table private behind `definitions()`, like its
  other fields. `expand_named_types` now returns `NamedTypeExpansion {
  schema, imports, definitions }`. Lowering is
  `to_json_schema_with_definitions(schema, &table)`; `to_json_schema` is the
  empty-table case.
- Typed consumers receive the table explicitly: functions that walk a
  `SimplifiedSchema`/`SchemaShape` and can meet a `Ref` take
  `&DefinitionTable` (or a `TypeContext<'_> { definitions: &DefinitionTable }`
  view when more context travels with it). A `Ref` whose key is missing from
  the table is a bug, reported as `SchemaError::Convert` naming the path, never
  treated as unconstrained.

## 4. Generated keys and references

- Keys are allocated in **stable traversal order**: depth-first over the root
  schema's properties in declaration order, then pattern keys, then each
  definition body as it is first reached. As built, a definition is
  allocated when its first use is reached; bodies are expanded from a FIFO
  worklist after the root, so a definition first reached inside a body comes
  after everything the root reached. An inline root property that is itself
  a named type (`$schema: { group: …, member: … }`) is not a use, so in that
  example `member` (reached inside `group`'s property body) is allocated
  before `group`. The first definition named `item`
  gets `item`; a later, different `DefKey` with the same name gets `item-2`,
  `item-3`, … No path, hash, or process-random value appears in a key.
- Keys are opaque: no consumer derives a name or namespace from one; they use
  `by_json_key`.
- A use lowers to `{"$ref": "#/$defs/<escaped key>", …use-site siblings}`,
  with `~` → `~0` and `/` → `~1` (RFC 6901). The validator probe
  (`recursive_schema_validator_probe::an_escaped_pointer_key_resolves`) pins
  that the pinned `jsonschema` 0.55 resolves escaped keys.
- `$defs` is emitted at the **root** of the lowered document, after the
  existing root keys, in allocation order.
- **Re-keying** when two independently lowered roots combine (baseline +
  document, trigger payloads, root-union arms): the incoming side's tables are
  folded into the receiving side by `DefKey`. The same `DefKey` (same
  namespace and name, hence the same body) shares one key. A different
  `DefKey` whose key is taken gets the next free suffix, and only that side's
  `$ref` values are rewritten. Rewriting visits **schema-bearing locations
  only**; literal data under `const`, `enum`, `default`, `examples`, and
  `x-darkmatter-*` suggestion/example metadata is never touched, even if it
  contains `$ref`/`$defs` keys. Last-writer-wins over two bodies is never
  allowed.

## 5. Reference graph and cycle admission

- Nodes are `DefKey`s. Edges are reference sites (`SchemaReferenceSite` +
  target `DefKey` + `descends: bool`).
- `descends = true` exactly when the reference sits in one of: an object
  property (literal key of an inline object), a dictionary value (a pattern
  key), an array item (`is_array` atom, including `U[]` of a union), or a
  tuple slot (element or spread). Everything else is `descends = false`: an
  alias (a definition whose body *is* the reference), a union arm (property-
  or definition-level), and a parameter or return type of a `function(...)`
  signature.
- Admission: drop every descending edge; the remaining graph must be acyclic.
  Each cyclic SCC of that remainder is reported, naming only its edges. A
  group that also contains descending cycles is otherwise fine; removing the
  non-descending cycle makes the same graph resolve.
- Discovery visits each `(namespace, name)` once and recognizes a revisit of a
  frame on the current path as a back-edge **before** comparing the stack to
  `MAX_IMPORT_DEPTH`. An acyclic chain deeper than the limit keeps its current
  `ImportCycle` "exceeded 32 levels" error. Root-file cycles
  (`ReferenceCycle`), trigger-payload cycles (`TriggerPayloadCycle`), unknown
  names, unreadable imports, and remote references keep their errors.

## 6. Errors

New variants on `SchemaError` (boxed fields keep the enum small, as
`UnionReference` does):

```rust
/// Named types form a cycle that never descends into a child value.
NonDescendingCycle { cycles: Vec<Vec<CycleReference>> },

/// A generated or raw local `$ref` names no definition in its scope.
UnresolvedReference { reference: String, location: String },

/// A raw schema at a merge boundary uses a reference form the merge cannot
/// keep in scope (`$id` change, external target, root `$ref`/applicator).
UnsupportedMergeReference { path: PathBuf, location: String, reason: String },

pub struct CycleReference {
    /// The authored reference.
    pub site: SchemaReferenceSite,
    /// The named type it refers to, with that type's namespace source.
    pub target: String,
    pub target_source: SchemaSourceId,
    /// Byte range of the reference name (Wave 2 addition), filled from the
    /// file's source map when `site.source` is a schema file; `None` for the
    /// inline `$schema`, which the consumer projects as it does for
    /// `UnknownNamedType`.
    pub span: Option<Range<usize>>,
}
```

- **As built (Wave 2):** only `NonDescendingCycle` exists so far;
  `UnresolvedReference` and `UnsupportedMergeReference` belong to Wave 3's
  scope lookup and merge ingress. Edges are reported in discovery order.
- **As built (Wave 3):** `UnresolvedReference { schema, reference, location }`
  and `UnsupportedMergeReference { schema, location, reason }`. The field is
  `schema: String` (a label such as `the baseline`, `the payload \`…\``, or
  `the root-union arm \`./x.json\``), not `path: PathBuf`: an in-memory
  baseline has no path, and a field named `source` is reserved by
  `thiserror`. `location` is the JSON Pointer of the schema holding the
  reference (`/` for the root). Inside a trigger payload both are reported
  through `TriggerPayloadNotMergeable.reason`.

- `NonDescendingCycle` reports every reference in each rejected cycle, with
  spans resolved like `UnknownNamedType`'s (two files ⇒ spans in both). DMLS
  projects it as a schema diagnostic at each span.
- `ImportCycle`'s doc comment and status text (`errors.rs:688-699`) still say
  named types form a DAG; the wave that adds `NonDescendingCycle` rewrites
  them to describe the remaining depth-limit use.
- Conflicting use-site constraints reuse `SchemaError::ConstraintConflict`
  with origins from both the definition and the use site.
- A `Ref` reaching a function type keeps the prerequisite's
  `SchemaError::Convert`, raised while walking definition bodies, with the
  **use-site** property path.

## 7. Use-site semantics

- A definition body is presence-free: `required`, `generated`, `eager`,
  `description`, `default`, and `suggest` on the definition's top-level atom
  are **inherited** into each use site (merged with the use site's own
  constraints under today's `merge_constraints` rules) and removed from the
  stored body. Value constraints (bounds, pattern, scheme, …) stay in the
  body.
- Use-site constraints that differ from the body lower as `$ref` siblings;
  Draft 2020-12 applies them together (pinned by
  `a_constraint_beside_a_reference_applies_with_its_target`).
- Optional-property nullability is the use site's: an optional `item`
  property lowers to today's optional wrapper around the `$ref`; an array item
  of the same type does not admit null. Array-container constraints stay on
  the array; item constraints become siblings of the item `$ref`.
- Two uses of one definition with different constraints produce two
  different sibling sets and one shared body.
- **As built (Wave 2):** the marker set moved from a single-atom body is
  `required`, `generated`, `eager`, `default`, `suggest`, `example`,
  `category`, plus the description. For a union body only `required`,
  `generated`, and `default` are hoisted off its alternatives (what a
  property-level union hoists); an alternative's `eager`, description, and
  examples stay on it. A definition that admits null (a union with a `null`
  or `unknown` arm) drops `required` at every use, so `is_required` stays
  table-free. Use-site value constraints allowed beside a `$ref`: key bounds
  beside an object, item bounds/`unique` beside an array (a use-site
  `min`/`max` on a list definition counts items), none beside a union or
  tuple. A use-site bound is merged against the definition's own top-level
  bounds only to report a contradiction (`ConstraintConflict`, definition
  origin first); the body keeps its bounds.
- The marker summary a use site needs is read off a first expansion pass;
  see `resolve/recursive.rs`. An acyclic schema takes one pass, unchanged.

## 8. Raw references at merge boundaries

- **State after Wave 2:** an inline document root and each inline root-union
  arm already allocate into one table and the union root carries `$defs`. A
  root-union arm that references a schema *file* with definitions is refused
  with `SchemaError::Convert` until this section's re-keying lands; the
  document's own `$defs` survives `merge_baseline` because document root keys
  are kept.
- `validate_simple_object_schema` adds `$defs` to its root allow-list and
  nothing else: root `$ref`, `anyOf`/`oneOf`/`allOf`, and other applicators
  remain rejected.
- Admission checks run only when a root `$defs` is present (no accepted
  schema has one today, since the allow-list rejects it, so nothing already
  accepted becomes stricter): every `$ref` of the form `#/$defs/…` must
  resolve in that table; a `$id` inside the table or the properties, or an
  external target, is `UnsupportedMergeReference`. Raw `$defs` keys are
  imported under `DefNamespace::Raw(…)` and re-keyed like generated ones.
- `merge_baseline` (`resolve.rs`) and the DMLS extension-baseline merge keep
  only `type`, `$schema`, and `additionalProperties` from a baseline root
  today, so a baseline's `$defs` would be dropped silently. Both fold the
  incoming `$defs` into the result through the re-keying of section 4; a
  merge that loses a referenced definition is a bug, never an unconstrained
  property.
- Raw **document** schemas (not at a merge boundary) keep Draft 2020-12
  semantics unchanged and are never rewritten.
- **As built (Wave 3):** `schemas/scope.rs` holds the rules.
  `check_joined_references` runs for every baseline, trigger payload, and
  root-union file arm: a `#/$defs/…` reference must resolve in its own table
  (`UnresolvedReference`) even when the schema has no `$defs` (it could
  otherwise bind to another layer's definition); with a root `$defs`, `$id`,
  `$anchor`, `$dynamicAnchor`, `$dynamicRef`, external targets, and other local
  pointers (`#`, `#/properties/…`, `""`) are `UnsupportedMergeReference`, and
  `validate_simple_object_schema` meta-validates the schema against Draft
  2020-12 (`SchemaError::Baseline`). `fold_definitions` shares a colliding key
  only when both bodies are byte-equal and everything they reference is
  shared; otherwise the joining side's key becomes `key-2`, … . Rewriting
  visits schema keywords only. `DefNamespace::Raw` was **not** added: raw
  definitions have no typed projection, so they exist only as JSON keys.
  A SimplifiedSchema single-shape file arm of a root union joins through
  `DefinitionTable::absorb` (typed table and JSON keys stay equal, which phase
  lowering of the arms relies on); raw arms and file arms that are themselves
  unions fold after every arm. `merge_baseline` refuses to move a baseline's
  `$defs` into a document root union whose arm has its own `$id`.
- **Typed tables (Wave 3):** `ResolvedPayload`/`TriggerLayer` carry
  `definitions`; `EffectiveSchema::definitions()` stays the document's table
  (its keys are the effective JSON's keys, since the document receives every
  merge), and `EffectiveSchema::definition(&DefKey)` searches it and the
  trigger tables. `ShippedBaseline` keeps no table: the built-in baseline does
  not recurse, and `to_json_schema` would refuse one that did. DMLS extension
  shapes are Wave 6's.

### Duplicate members (Phase 1 ruling)

Every reader that feeds these boundaries already rejects a duplicate member:
the raw `.json` reader (`parse_raw_json_schema` → `coercion::content_error`,
schema-enhancements ruling R3, tested by
`schema_enhancements_matrix::raw_schema::raw_json_schema_keyword_matrix`) and
both YAML loaders. An in-memory `serde_json::Value` handed to
`with_baseline_json_schema` cannot carry duplicates. The ruling is therefore
"reject, at the existing readers", and it tightens no accepted schema; no
author decision is needed.

## 9. JSON walkers and synthetic validators

```rust
/// The schema document a reference is resolved against.
pub(crate) struct SchemaScope<'a> { root: &'a Value }

impl SchemaScope<'_> {
    /// The target of `schema`'s `$ref` when it is a local `#/$defs/…`
    /// (or `#`) pointer, decoding `~0`/`~1`. `Ok(None)` when `schema` has no
    /// `$ref`; `Err(UnresolvedReference)` when the target is missing. The
    /// caller still applies `schema`'s sibling keywords.
    pub(crate) fn resolve<'s>(&'s self, schema: &'s Value)
        -> Result<Option<&'s Value>, SchemaError>;
}
```

- Instance walkers (validate, clean ×2, coerce, rewrite, discriminant, the
  conversion translator) guard `(schema location, instance location)` pairs,
  never a global visited-definition set.
- Whole-schema walkers visit each definition once and keep references.
- A sub-schema compiled on its own (a property, a union arm, a phase
  projection) is compiled as `with_root_definitions(sub, root)`: the
  sub-schema plus the root's `$defs`. Generated keys are root-level, so this
  keeps every reference in scope.
- Each walker keeps its own union policy; closest-arm reporting never
  authorizes coercion, cleanup, or eager-file rewriting.
- The sites that compile a sub-schema alone are listed in consumers.md §5
  (root-union arm wrapping, discriminated-arm narrowing, `coerce::accepts`,
  both `rewrite` unions, `select_file_mode`, `root_schema_arm_applies`,
  completion's `alternative_accepts`, lint candidates, alphanumeric and
  example checks). Each one takes its root's `$defs` through
  `with_root_definitions`. `json_shape::Index::build` already embeds the whole
  document under `$defs.document` with its own `$id`, so it is root-scoped
  today and needs only a test. `format_assertion` compiles a `{format}`
  fragment with no references and is exempt.
- **As built (Wave 3):** `SchemaScope::{new, named, resolve, chain}`
  (`resolve` is one local hop, `Ok(None)` for no `$ref` or a non-local one;
  `chain` follows hops and errors on a missing target or a ref-only loop) and
  `with_root_definitions(sub, root)`. `wrap_arm_as_root_schema(arm, root)` now
  takes the root. Adapted sites: `CompiledValidators`/`wrapped_root_arms`,
  `coerce::accepts`, `rewrite_root_union` and `rewrite_property_union` (the
  rewrite walkers carry `root`), `narrow_discriminated_union_problems`,
  `select_file_mode` and `root_schema_arm_applies` (the caller file-mode
  classifier carries `root`), `schema_declares_property` (follows a chain),
  and `example` targets (`resolve_document_examples` passes the root `$defs`;
  inline union arms resolve examples before hoisting). Not adapted, with
  reason: completion's `alternative_accepts` lowers a typed atom (Wave 5
  follows `Ref` there), lint candidates never see a `Ref` (a suggestion on a
  recursive reference is rejected), `check_alphanumeric_values` and
  `format_assertion` compile reference-free fragments, and
  `json_shape::Index::build` is already root-scoped.
- Exhaustive matches gain a deliberate `Ref` arm. The 22 wildcard matches and
  49 partial destructures in consumers.md §1 are each reviewed in the wave that
  owns their file; a wildcard that would silently treat `Ref` as "something
  else" is rewritten to name `Ref` explicitly.

## 10. Phase context

- `SchemaPhase` projection runs over the root shape **and** once per
  definition per phase, memoized by `(DefKey, SchemaPhase)`, producing a
  projected `DefinitionTable` with the same keys.
- Effective presence and timing are derived at each use site from the use-site
  atom (section 7), so a definition shared by an eager and a deferred property
  projects once and the two uses differ only in their siblings.
- **As built (Wave 3):** `phase::project_definitions(&table, phase)` maps every
  body through `project_def` under the same keys (`DefinitionTable::map_bodies`);
  it runs per `validate_for_phase` call rather than being memoized, since it
  is linear in the table and the lowering beside it costs more. The projected
  `EffectiveSchema` carries the projected table. `merge_with_effective`
  replaces the document's own `$defs` bodies with the projection's and keeps
  the baseline/trigger-only ones.

## 11. Type-level consumers

`is_subtype`, `DeclaredTypes::top_level_types`, and `classify_argument` /
`check_call` compare two `Ref`s by `DefKey` plus use-site constraints, and
descend into a body at most once per `(left, right)` pair on a path
(coinductive). Nested-path projection through recursion stays with
`2026-10-31-type-aware-evaluation`.

## 12. Input robustness matrix: confirmation against the tree

| Matrix | State on 2026-10-23 |
| --- | --- |
| YAML tagged/pure envelope, `kind`, `types`, union arms, `$schema` shapes, duplicate key, trailing content | Existing cells pinned by `schema_enhancements_matrix::envelope` and `schema_yaml_spellings`. Native YAML null in a named type is the `null` type (R5), as the matrix assumes. |
| Referenced `types.item` with a **recursive** body | Not possible today (`ImportCycle`); built and tested in Wave 2 |
| Raw `$defs` / `$ref` / `$id` / structural keywords at document level | Pinned by `schema_enhancements_matrix::raw_schema` (JSON and YAML); unchanged by this feature |
| Raw `$defs` / `$ref` / `$id` at the **merge ingress** | New; rejected at the root today by `validate_simple_object_schema`; built and tested in Wave 3 against section 8 |
| Raw JSON duplicate members | Rejected by the readers (section 8) |

No reader this feature changes exposes a load-bearing field beyond the
matrix's columns; tuple slots are typed-model edges, not configuration
fields, and need no column.
