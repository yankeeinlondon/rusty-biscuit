# Consumer Inventory: Recursive Schema Types

This is the Wave 1 inventory for `2026-10-29-recursive-schema-types`. It lists every place in the tree that has to handle the new `TypeExpr::Ref(DefKey)` variant or follow a lowered `$ref`, who owns each place in the later waves, and which existing tests cover it. Everything here was read from the worktree at `feat/pipeline` on 2026-10-23. Paths are relative to the repository root. `L/` means `darkmatter/lib/src/markdown/`. Anything not confirmed by reading the code is marked **unverified**.

`kind: inventory` is not one of the kinds in the `kind` schema (`claudine/features/2026-10-28-schema-enhancements/drafts/kind-document.yaml`), so this file has no frontmatter.

## 1. `TypeExpr` match sites

`TypeExpr` is defined in `L/schemas/simplified/types.rs:218`. Its variants are `Primitive`, `InlineObject`, `Imported{name,reference}`, `Union(Vec<PropertyAtom>)`, `Tuple`, and `Function`.

Totals for production code, excluding `#[cfg(test)]` modules, `tests/`, and spikes:

- **53 `match` expressions** have `TypeExpr::` arms. 31 are exhaustive and will fail to compile until `Ref` is handled. 22 have a wildcard or binding catch-all, so they would **silently absorb `Ref`**.
- **49 partial destructures** (`if let`, `let … else`, `matches!`) see one variant and treat everything else as "not this one". They carry the same risk as a wildcard.

In the "Ref action" column, **lower** means emit `$ref`, **follow** means look up the definition (lazily where an editor path is involved), **back-ref** means display the type by name, **coind.** means compare by `DefKey` and expand each pair at most once, and **unreach.** means the site only ever sees authored or unresolved input (only the resolver produces `Ref`).

### 1a. `match` expressions (W = wildcard or catch-all arm)

| Site | Function | W | Ref action |
|---|---|---|---|
| `L/schemas/simplified/convert.rs:462` | `atom_fragment_without_null_wrap` | – | **lower** to `#/$defs/<key>`; callable ban must also walk the `$defs` bodies |
| `L/schemas/simplified/serialize.rs:32` | `serialize_property_atom` | – | back-ref (authored name) |
| `L/schemas/resolve.rs:1711` | `ImportEngine::expand_atom` | – | **produce** `Ref` for cyclic targets (Wave 2) |
| `L/schemas/resolve.rs:3103` | `atom_has_imports` | – | `Ref` is not an unresolved import → false |
| `L/schemas/resolve.rs:1416` | `AtomProvenance::authored` | W `_` | deliberate: a `Ref` has no shape constraints |
| `L/schemas/resolve.rs:2482` | `apply_import_postfix` | W `_` | inspect the target's outer shape before rejecting `[]` on an array (Design 3) |
| `L/schemas/simplified/types.rs:528` | `PropertyAtom::admits_null` | W `_` | **follow**; today a `Ref` would return `false` silently |
| `L/schemas/simplified/types.rs:558` | `PropertyAtom::union_arms` | W `_` | **follow**; `is_required`/`is_generated`/`has_default` read arms through this |
| `L/schemas/simplified/subtype.rs:66` | `is_subtype` | W `_` | **coind.**; today the wildcard gives `false` |
| `L/schemas/simplified/subtype.rs:176` | `is_always_text` | – | follow / coind. |
| `L/schemas/simplified/subtype.rs:224` | `identity` | – | identity by `DefKey` |
| `L/schemas/simplified/signature.rs:141` | `canonical_atoms` | W `other` | unreach. (function contracts); unverified |
| `L/schemas/simplified/query.rs:264` | `inline_object_shape` | – | follow |
| `L/schemas/simplified/lint.rs:174` | `lint_property` | – | unreach. (authored) |
| `L/schemas/simplified/standalone.rs:341` | `visit_atom` | – | unreach. (authored); unverified |
| `L/schemas/simplified/source.rs:535` | `project_located_atom_suggestions` | W `_` | unreach. (authored) |
| `L/schemas/simplified/source.rs:1692` | `project_atom_source` | W `_` | unreach. (authored) |
| `L/schemas/simplified/source.rs:2397` | `atom_has_suggestions` | – | unreach. (authored) |
| `L/schemas/phase.rs:121` | `project_atom` | – | project the definition once per phase |
| `L/schemas/phase.rs:147` | `strip_union_arm` | – | same |
| `L/schemas/completion.rs:244` | `is_completable` | – | follow lazily |
| `L/schemas/completion.rs:280` | `kind_for_atom` | – | follow lazily |
| `L/schemas/completion.rs:490` | `alternatives_for_def` | – | follow lazily |
| `L/schemas/frontmatter_shape.rs:295` | `arm_discriminant_json` | – | follow |
| `L/schemas/frontmatter_shape.rs:338` | `discriminated_arm_shape` | W `_` | follow (today the wildcard gives `None`) |
| `L/schemas/detect.rs:429` | `primitive` | – | unreach. (detection builds from instances) → `None` |
| `L/schemas/catalog.rs:155` | `GlobalDeclaration::shape` | W `_` | follow |
| `L/schemas/catalog.rs:504` | `check_kind` | W `_` | follow; today a `Ref` would be misreported as "a union" |
| `L/schemas/triggers/matcher.rs:262` | `value_matches_type` | – | unreach. in loaded triggers (`Imported` arm at :351 returns false); decide follow vs false |
| `L/compose/context/catalog.rs:275` | `project_one` | – | follow / back-ref for descriptors |
| `L/compose/expression/call_binding/translate.rs:348` | `Translator::atom` | – | **follow** through a reusable handle; replaces `TooDeep` |
| `L/compose/expression/call_binding/registration.rs:292` | `function_types` | W `_` | unreach. (parse only) |
| `L/compose/expression/call_binding/registration.rs:300` | `function_types` | W `ty` | unreach. |
| `L/compose/expression/catalog/mod.rs:431` | `overload_atoms` | W `_` | unreach.; unverified |
| `L/compose/expression/scope/shape.rs:228` | `Shape::from_definition` (tuple match) | W `_` | follow for null/unknown arms |
| `L/compose/expression/scope/shape.rs:375` | `finite_values` | W `_` | follow (a `Ref` to an enum or literal union) |
| `L/compose/expression/scope/shape.rs:415` | `families` | – | follow (a `Ref` to an object is `Family::Object`) |
| `L/compose/expression/scope/shape.rs:555` | `arm_equal` | W `_` | deliberate pass-through |
| `L/compose/expression/scope/shape.rs:569` | `arm_differs` | W `_` | deliberate pass-through |
| `darkmatter/cli/src/commands/schema/assignment.rs:198` | `atom_prefers_string` | – | follow (a union definition with string) |
| `darkmatter/dmls/src/overlay/schema.rs:850` | `meta_schema_kinds` | W `_` | follow or skip; deliberate |
| `darkmatter/dmls/src/overlay/suggestions.rs:115` | `walk_shape` | – | follow once per definition |
| `darkmatter/dmls/src/overlay/named_types.rs:315` | `collect_atom_targets` | – | navigation target = defining file/span |
| `darkmatter/dmls/src/overlay/expressions.rs:903` | `member_summary` | W `_` | back-ref |
| `darkmatter/dmls/src/providers/frontmatter.rs:1609` | `meta_schema_definition_hover_body` | – | back-ref |
| `darkmatter/dmls/src/providers/frontmatter.rs:1708` | `schema_hover` | W `_` | follow lazily |
| `darkmatter/dmls/src/providers/frontmatter.rs:1837` | `alternative_line` | – | back-ref |
| `darkmatter/dmls/src/providers/frontmatter.rs:1890` | `schema_hover_details` | – | back-ref |
| `darkmatter/dmls/src/providers/frontmatter.rs:2323` | `descend` | W `_` | **follow one level per cursor step** |
| `darkmatter/dmls/src/providers/frontmatter.rs:2529` | `inline_object_shape` | – | follow |
| `claudine/lib/src/composition/schema/classify.rs:288` | `interactive_shape_for_atom` | – | follow or report non-interactive |
| `claudine/lib/src/composition/schema/classify.rs:420` | `type_label_for_atom` | – | back-ref |
| `claudine/gen/src/schema_compat.rs:304` | `describe_atom` | – | back-ref |

### 1b. Partial destructures (production code)

These silently ignore `Ref`. Review each one in the owning wave.

- **Resolver and model**
  - `L/schemas/resolve.rs:2128`, `:2135` (`rehome_atom_examples`)
  - `L/schemas/resolve.rs:2382` (`apply_import_postfix`, `suggest` target)
  - `types.rs:567`, `:593`
  - `convert.rs:360`, `:535`
  - `grammar.rs:868`
  - `triggers/grammar.rs:616`
  - `signature.rs:33`, `:109`, `:116`, `:225`
  - `lint.rs:218`, `:271`
  - `query.rs:196-197`
  - `source.rs:1943`, `:2385`, `:2425`
- **Typed consumers**
  - `completion.rs:183`, `:547-548` (`applies_to`), `:557` (`discriminant_json`)
  - `frontmatter_shape.rs:132` (`expression_atom`), `:356` (`merged_inline_object_shape`)
  - `file_match.rs:216`
  - `compose/context/catalog.rs:431`
  - `compose/expression/scope/shape.rs:542`
  - `catalog/mod.rs:349`, `:440`
  - `translate.rs:249` (`inline_reference`), `:542`
  - `shell_expansion/template/canonical.rs:349`, `typing.rs:116`, `:313`
- **DMLS**
  - `overlay/schema.rs:127`, `:825`
  - `providers/frontmatter.rs:1085`, `:1619`, `:1637`, `:1849`, `:1853`, `:2641`
- **Claudine**
  - `composition/schema/supplied.rs:394`, `composition/schema/mod.rs:994`
  - `claudine/gen/src/schema_compat.rs:81`, `:176`, `:216`, `:259`

Helpers that hide the `TypeExpr` shape from callers: `PropertyDef::alternatives` (`types.rs:200`), `flattened_alternatives` (`simplified/query.rs:308`), and DMLS `alternatives` (`overlay/schema.rs:872`).

## 2. Definition storage and namespaces

**`NamespaceKey`** is private (`L/schemas/resolve.rs:1227`). Its variants:

- `Root` is the inline document `$schema` **and every root-union arm**. All arms share it.
- `File(PathBuf)` is a canonicalized named-type file.
- `Embedded(String)` is an embedded virtual path.
- `Shipped(String)` is a `darkmatter:<stem>` partial.

`source_id()` (`:1244`) maps these keys to the public `SchemaSourceId` (`simplified/source.rs:94`) variants `InlineDocument`, `File(PathBuf)`, and `Embedded(String)`. **Both `Embedded` and `Shipped` map to `SchemaSourceId::Embedded`.**

What separates the roots today:

- **Root-union arms** are told apart only by `SchemaSourcePath::root().union_arm(index)` (`resolve.rs:360`, `:1091`). The namespace key is the same for all of them.
- **Inline roots** use `NamespaceKey::Root` (`:198`, `:292`, `:360`).
- **Standalone files** use `File(canonical)` (`:898`).
- **Embedded schemas** use `Embedded(path)` (`:1032`).

The spec's Design 4 requires distinct identities for these.

How namespace tables are shared through `Arc` (`resolve.rs:1212-1216` comment):

- `Namespace.named_types: Arc<IndexMap<String, PropertyDef>>` (`:1265`)
- `CachedNamespace.named_types` (`:1321`)
- `ImportEngine.embedded_cache` and `shipped_cache: HashMap<String, Arc<IndexMap<…>>>` (`:1287`, `:1291`)
- `ImportEngine.expanded_cache: HashMap<(NamespaceKey, String), ExpandedDef>` (`:1297`)
- `stack: Vec<(NamespaceKey, String)>` holds the frames
- `dependencies: BTreeSet<PathBuf>`

**The sharing ends when `expand_document_imports` returns. Nothing persists across calls.**

| Type | Location | Fields today | Where a definitions table would ride |
|---|---|---|---|
| `ResolvedSchema` | `L/schemas/resolve.rs:68` | `simplified: Option<SimplifiedSchema>`, `json_schema: Value`, `origin`, `imports`, `examples`, `referenced_files`, `searched`, `advisories` | new field next to `simplified`; `imports` must include files reached only on closing edges |
| `EffectiveSchema` | `L/schemas/mod.rs:849` | `simplified`, `trigger_shapes: Vec<Arc<SchemaShape>>`, `json_schema: Arc<Value>`, `origins`, `validators: CompiledValidators`, `base_dir`, `file_ref_fallback_dir`, `file_resolution_context`, `caller_origins`, `dependencies`, `advisories`, `conversion: OnceLock<Arc<ConversionPlan>>` | table for `simplified` and for each trigger shape |
| `ShippedBaseline` | `L/schemas/mod.rs:157` | `schema: SimplifiedSchema`, `json_schema: Arc<Value>` | baseline table |
| `BaselineSchema` | `L/schemas/mod.rs:285` | `json_schema: Arc<Value>`, `advisories` | JSON only |
| `TriggerLayer` | `L/schemas/mod.rs:297` | `json_schema: Arc<Value>`, `shape: Option<Arc<SchemaShape>>`, … | table beside `shape` |
| `CompiledValidators` | `L/schemas/mod.rs:1837` | `Object(Arc<Validator>)` or `RootUnion(Vec<Arc<Validator>>)` | per-arm validators need the root `$defs` (§5) |
| DMLS `SchemaBundle` | `darkmatter/dmls/src/overlay/schema.rs:888` | `effective: EffectiveSchema`, `advisories`, `frontmatter_json`, `extension_shapes: Vec<SchemaShape>`, `extension_dependencies` | extension shapes need their own tables |
| DMLS `SchemaOutcome::Ready(Option<Arc<SchemaBundle>>)` | `darkmatter/dmls/src/overlay/mod.rs:64` | — | — |
| DMLS `DocumentOverlay.named_types: Arc<NamedTypeCatalog>` | `overlay/mod.rs:122`; struct at `overlay/named_types.rs:143` (`externals: Vec<ExternalNamespace>` with `path`, `text: Arc<str>`, `document: Option<StandaloneSchemaDocument>`) | navigation source | defining file and span |

Source provenance (`L/schemas/simplified/source.rs`):

- `SchemaSourcePath(Vec<SchemaSourcePathSegment>)` (`:42`), with segments `Property(String)` and `UnionArm(usize)`.
- `SchemaReferenceSite { source, path }` (`:148`). **It carries no span.**
- `SchemaConstraintOrigin { source, path, keyword, occurrence, span: Option<Range> }` (`:129`).
- `SchemaSourceMap { spans: BTreeMap<(SchemaSourcePath, SchemaSpanKind), Vec<Range>> }` (`:261`). Reference spans are stored under `SchemaSpanKind::ImportName` and `ImportReference`.
- `StandaloneSchemaDocument.source_map` (`standalone.rs:79`).

Reference spans are therefore recovered from the source map by path. The site itself never holds one.

## 3. Lowering and resolve-entry calls

Production callers of `to_json_schema` (`L/schemas/simplified/convert.rs:85`):

| Caller | Function | Notes |
|---|---|---|
| `L/schemas/resolve.rs:204` | `resolve_yaml_schema_with_roots` | inline mapping |
| `L/schemas/resolve.rs:367` | `resolve_root_union` | **per arm**; arms joined at `:433` (`{$schema, anyOf}`) |
| `L/schemas/resolve.rs:911` | `resolve_standalone_schema` | referenced file |
| `L/schemas/resolve.rs:1038` | `lower_embedded_schema` | embedded |
| `L/schemas/resolve.rs:1104` | `resolve_standalone_root_union` | per arm; joined at `:1147` |
| `L/schemas/resolve.rs:3595` | `shape_to_schema` (pub) | no in-tree caller found |
| `L/schemas/mod.rs:170` | `shipped_baseline` | |
| `L/schemas/mod.rs:379`, `:409` | `with_baseline`, `with_baseline_from_file` | |
| `L/schemas/mod.rs:1061` | `validate_for_phase_with_positions` | **re-lowers the phase projection** of `simplified` |
| `L/schemas/completion.rs:432` | `alternative_accepts` | synthetic single-property schema (§5) |
| `L/schemas/example.rs:91` | `example_envelope_json` | |
| `L/compose/context/options.rs:2595` | `baseline_canonical_json` | |
| `darkmatter/cli/src/commands/schema/detect.rs:130` | `emit_schema` | detected schemas cannot contain `Ref` |

`suggestion_target_schema` (`convert.rs:617`) lowers one atom on its own. Its callers are `lint.rs:216` (`lint_candidates`) and `lint.rs:274` (`sibling_validators`).

Resolve entry points (`resolve.rs`):

- `resolve_schema` :133 → `_with_roots` :149 → `resolve_yaml_schema_with_roots` :171. The chain also includes `resolve_yaml_schema` :161.
- `expand_named_types` :283
- `resolve_root_union` :332
- `resolve_standalone_schema` :890
- `prepare_embedded_schema` :959, `_in` :969, `prepare_embedded_inline_schema` :1005
- `lower_embedded_schema` :1023
- `resolve_standalone_root_union` :1070
- `expand_document_imports` :1504 → `expand_imports` :1521

External callers:

| Caller | Calls |
|---|---|
| `L/schemas/mod.rs:415` | `resolve_yaml_schema` |
| `mod.rs:594` (`effective_for_with_override`) | `resolve_schema_with_roots` |
| `L/schemas/catalog.rs:485-486` (`resolve_declaration`) | `prepare_embedded_inline_schema`, `resolve_yaml_schema` |
| `L/schemas/triggers/assemble.rs:292` (`resolve_file_payload`) | `resolve_yaml_schema_with_roots` |
| `assemble.rs:731`, `:734` (`resolve_embedded_payload`) | `prepare_embedded_schema_in`, `prepare_embedded_inline_schema` |
| `darkmatter/dmls/src/overlay/schema.rs:1051` (`load_extension_schema`) | `resolve_schema` |
| `overlay/mod.rs:887` (`standalone_named_types`) | `expand_named_types` |
| `overlay/suggestions.rs:90` (`reference_lints`) | `expand_named_types` |
| `claudine/lib/src/composition/schema/tests.rs:2504`, `:2535` | `resolve_schema` (tests) |

## 4. Merge boundaries

| Boundary | Location | Root keys allowed today | Property level |
|---|---|---|---|
| `validate_simple_object_schema` | `L/schemas/resolve.rs:3477` | `ALLOWED_KEYS` = `$schema`, `type` (must be `"object"`), `properties`, `required`, `additionalProperties` (only `true`), `description`, `title`. A root `$defs`, `$ref`, or `anyOf` is rejected as `SchemaError::Baseline` | not inspected; any property schema passes |
| `validate_baseline_schema` | `resolve.rs:3462` | object, then the check above | — |
| `merge_baseline` | `resolve.rs:3359` | validates the **baseline** only. A document root `anyOf` merges into each arm (`:3366-3373`). Document root keys other than `properties`/`required` are kept as they are. `type`, `$schema`, and `additionalProperties` are copied from the baseline when the document lacks them. **Baseline root keys other than these are dropped** (so would a baseline `$defs`) | property-keyed; the document wins on a shared key; baseline `required` is added only for baseline-only keys |
| `enforce_merge_compatible` | `L/schemas/triggers/assemble.rs:764`; called at `:298`, `:751` | the same contract, through `validate_simple_object_schema`, mapped to `TriggerPayloadNotMergeable` | not inspected |
| `merged_trigger_schema` | `L/schemas/mod.rs:1780` (fold at `:1813-1819`) | baseline → triggers → document through repeated `merge_baseline`; memo keyed by `Arc` identity | — |
| `effective_for_with_override` | `L/schemas/mod.rs:620-633` | baseline + document goes through `merge_baseline`; triggers go to `merged_trigger_schema` | — |
| Phase merge `merge_with_effective` | `L/schemas/phase.rs:32` | pairs effective and projected root `anyOf` arms one-to-one and merges each through `merge_baseline`; both non-union → `merge_baseline`; mismatched → projected document alone | — |
| `phase_json_schema` | `L/schemas/mod.rs:1714` | memoized derivation over `merge_with_effective` | — |
| Root-union assembly | `resolve.rs:429-433`, `:1143-1147` | root = `{$schema, anyOf:[arm…]}`; each arm is lowered separately, so a `#/$defs/...` inside an arm would resolve against the **union root** when the union is compiled whole | — |
| `convert.rs:130` | `union_to_root_schema` | `anyOf` of arms | — |
| DMLS `combined_baseline` | `darkmatter/dmls/src/overlay/schema.rs:1001` (`merge_baseline` at `:1031`) | extension baselines merged in order | — |

## 5. Synthetic validators (sub-schemas compiled away from their root)

Each of these sites must later carry the root `$defs` or a scope context.

| Site | Function | What is compiled |
|---|---|---|
| `L/schemas/validate.rs:1252` | `wrap_arm_as_root_schema` | an `anyOf` arm copied as its own root (adds `$schema` only) |
| `L/schemas/mod.rs:1870`, `:1852-1857` | `wrapped_root_arms`, `CompiledValidators::build` | each root-union arm, through the wrap above |
| `L/schemas/validate.rs:1300-1332` | `narrow_discriminated_union_problems` | the selected arm nested in synthetic `properties`/`prefixItems` |
| `L/schemas/coerce.rs:224-233` | `accepts` | each root arm (wrapped) |
| `L/schemas/rewrite.rs:119-124` | `rewrite_root_union` | each root arm (wrapped), through `build_validator_in_context` |
| `L/schemas/rewrite.rs:330` | `rewrite_property_union` | **a property-level union arm alone** |
| `L/compose/schema_validation.rs:875` | `select_file_mode` | **a property union arm alone** (`without_match_keyword(arm)`) |
| `L/compose/schema_validation.rs:1074-1075` | `root_schema_arm_applies` | root arm (wrapped), through `build_validator_with_callers` |
| `L/schemas/completion.rs:425-437` | `alternative_accepts` | one atom lowered as `{value: atom}` |
| `L/schemas/simplified/lint.rs:216-217`, `:274-275` | `lint_candidates`, `sibling_validators` | one atom through `suggestion_target_schema` |
| `L/schemas/simplified/convert.rs:1592` | `check_alphanumeric_values` | an alphanumeric target fragment |
| `L/schemas/example.rs:182`, `:199`, `:252` | `validate_example_object`, `validate_returns_against_target` | envelope or target fragment |
| `L/compose/expression/call_binding/translate.rs:1035-1053` | `format_assertion` | `{format}` only (no references) |
| `L/schemas/json_shape.rs:789-817` | `Index::build` (node assertion) | **already root-scoped**: embeds the document under `$defs.document` and refers to it with `$ref` and `$id: urn:darkmatter:assertion` |

Validator builders (`L/schemas/validate.rs`):

- `ValidatorCache::validator_for` :235, `validator_for_shared` :252, `validator_for_hashed` :283, `structural_validator_for` :323, `structural_validator_hashed` :329
- `build_structural_validator` :438, `build_validator_in_context` :453, `build_validator_with_callers` :470, `build_validator_with` :505 (`jsonschema::options().with_draft(Draft202012)`)

Process-wide caches:

- `SHARED_VALIDATOR_CACHE` (`mod.rs:282`)
- `CONVERSION_VALIDATOR_CACHE` (`coerce.rs:47`)
- `ALTERNATIVE_VALIDATOR_CACHE` (`completion.rs:55`)
- example cache (`example.rs:70`)

## 6. JSON walkers

| Walker | Functions | Follows `$ref` today? | Union policy |
|---|---|---|---|
| `L/schemas/validate.rs` | `schema_nodes_at` :1097, `push_union_arms` :1129, `resolve_problem_description` :1041, `collect_discriminated_arms` :1367, `narrow_discriminated_union_problems` :1300, `unwrap_nullable_arm` :1229 | **no** (no `$ref` handling in the file) | `push_union_arms` uses the literal-discriminant arm when one is selected, else **all non-null arms**. `collect_discriminated_arms` descends only a discriminant-selected arm. Root unions report per arm (`collect_root_union_problems` :579) |
| `L/schemas/clean.rs` walk 1 | `schema_requires_string` :484 | **no** | unwraps the nullable arm and **returns false at any `anyOf`** |
| `L/schemas/clean.rs` walk 2 | `nearest_declared_key` :626 | **no** | unwraps the nullable arm only; a genuine union has no `properties`, so it returns `None` |
| `L/schemas/coerce.rs` | `convert` :149, `property_wise` :189, `accepts` :224, `partial_copy` :319, `relax_presence` :389 | indirectly: walks the `coercion::Shape` from `json_shape`. The JSON is not walked except at the root `anyOf` | root union judged arm by arm; `property_wise` refuses root `anyOf`/`oneOf` |
| `L/schemas/rewrite.rs` | `rewrite_eager_file_values` :74, `rewrite_root_union` :109, `rewrite_object` :194, `rewrite_property` :259, `rewrite_property_union` :316, `rewrite_array_items` :409, `rewrite_inline_object` :436, `contains_eager_file_fragment` :470 | **no** | root: every accepting arm must agree. Property: exactly one validating arm carrying the eager-file marker, else unchanged |
| `L/schemas/discriminant.rs` | `select_literal_discriminant_arm` :40, `literal_const_props` :107, `const_of` :120 | **no** (a `$ref` arm has no `properties`, so it is never a candidate) | shared `const` keys in ≥2 arms; ambiguous → `None` |
| `L/schemas/json_shape.rs` | `shape_from_json_schema` :152, `Index::resolve` :253, `Translation::reference` :717, `node_assertion` :734 | **yes**: local `#/...`, `$id`, `$anchor`. `MAX_REFERENCE_DEPTH = 32` (:56), `CYCLE_EXPANSIONS = 4` (:61), `REFERENCE_BUDGET = 4096` (:65), checked at :723. A cutoff becomes `Shape::Any` plus a passive assertion | `anyOf`/`oneOf` options at :359-362 become `union(...)` |
| `L/compose/expression/call_binding/translate.rs` | `shape_from_property_def` :122, `property_shape_from_def` :141, `shape_from_simplified_schema` :155, `inline_reference` :245, `resolve` :271, `reference` :388 | follows `Imported` through `TypeResolver`, not `$ref`. `MAX_REFERENCE_DEPTH = 32` (:46) → `TranslateError::TooDeep { path, limit }` (:96) | `union_property` :288 mirrors the lowering |
| `L/schemas/mod.rs` | `schema_declares_property` :806 | **yes** (local `#…` pointer through `root.pointer`), depth cap `MAX_DEPTH = 32` (:808) → false | all of `allOf`/`anyOf`/`oneOf`/`if`/`then`/`else` |

## 7. Typed-tree walkers

Each line gives the file, then the entry functions, then the internal helpers (line numbers in parentheses).

- **`L/schemas/completion.rs`**
  - Entry: `for_property` 111, `completable_properties` 128, `for_list_item` 394, `for_property_value` 407, `alternative_accepts` 425, `alternatives_for_def` 456.
  - Helpers: `completable_in_shape` 150, `suggestion_from_union` 170, `property_def` 512, `applies_to` 544, `discriminant_json` 556, `add_keys` 571.
- **`L/schemas/frontmatter_shape.rs`**
  - Entry: `effective_property_shape` 46, `property_def_at_path` 93, `nested_property_shape` 109, `expression_atom` 129, `frontmatter_expression_values` 173.
  - Helpers: `alternatives_of` 239, `overlay_root_union` 251, `discriminated_arm_shape` 324, `merged_inline_object_shape` 353, `merge_defs` 378.
- **`L/schemas/phase.rs`**: `project` 16, `merge_with_effective` 32, `project_shape` 90, `project_def` 98, `project_atom` 119, `strip_union_arm` 146, `strip_presence` 158, `atom_has_property_eager` 174.
- **`L/compose/context/catalog.rs`**: `context_variable_descriptors` 244, `project_descriptors` 254, `project_one` 267.
- **`L/schemas/simplified/subtype.rs`**: `is_subtype` 53, `is_definition_subtype` 101, `is_string_like` 133, `is_always_text` 175, `identity` 223.
- **`L/compose/expression/scope/projection.rs`**: `DeclaredTypes` 19, `top_level_types` 285. It reads `SchemaArm::Inline` shapes only.
- **`L/compose/expression/call_check.rs`**: `classify_argument` 167, `arm_fit` 261 (`shape_from_property_def(…, &NoNamedTypes)` at 267, so named types are **not** resolved there), `subtype_of_parameter` 296, `check_call` 674, `classify_call` 694.
- **`L/compose/expression/scope/shape.rs`**: `Shape::from_definition` 222, `Shape::finite_values` 366, `families` 411, `arm_equal` 534, `arm_differs` 562.
- **`darkmatter/cli/src/commands/schema/assignment.rs`**: `coerce_value` 139, `force_string_for_path` 163, `lookup_property` 190, `atom_prefers_string` 194. Called from `commands/schema/validate.rs:235`.
- **`darkmatter/dmls/src/overlay/schema.rs`**: `semantic_type_regions` 74, `shape_definitions` 147, `frontmatter_authoring` 212, `effective_shape` 793, `def_at_path` 812, `merged_inline_shape` 822, `meta_schema_kinds` 847, `alternatives` 872, `assemble` 929, `combined_baseline` 1001.
- **`darkmatter/dmls/src/providers/frontmatter.rs`**: `schema_hover` 1692, `alternative_line` 1836, `descend` 2310 (counts `SHAPE_MATERIALIZATIONS`, useful for the lazy-descent instrumentation), `def_at_path_ctx` 2458. Hover sites are listed in §1a.
- **DMLS navigation and diagnostics**: `overlay/named_types.rs` (`collect_atom_targets` 315); `diagnostics/named_types.rs:37-40`, `:119-143` (projects `UnknownNamedType`/`ConstraintConflict`/`UnionReference`/`ImportCycle` onto ranges).
- **`L/schemas/about.rs`**: no production `TypeExpr`/`PropertyDef` walk. Its descriptor tables are static (`schema_shape_descriptors` 471), and its `TypeExpr` uses are in tests (1439, 1467). It needs no `Ref` handling beyond grammar docs.
- **`L/schemas/detect.rs`**: `detect_schema` 45, `unify_types` 394, `primitive` 428, `schema_to_yaml` 487. It builds from instances, so it never sees `Ref`.
- **`L/schemas/triggers/matcher.rs`**: `property_matches` 203, `type_conforms` 224, `scalar_conforms` 243, `value_matches_type` 261. Loaded trigger conditions cannot import (comment at :351).

## 8. Existing cycle and depth errors

Variants in `L/schemas/errors.rs`:

```rust
#[error("named-type import cycle detected: {chain}")]
ImportCycle { chain: String },                                   // :155-156
#[error("$schema file-reference cycle detected: {chain}")]
ReferenceCycle { chain: String },                                // :216-217
#[error("$schema file-reference chain exceeded the {limit}-file depth limit: {chain}")]
ReferenceDepthExceeded { limit: usize, chain: String },          // :226-227
TriggerPayloadCycle { trigger: PathBuf, payload_path: PathBuf, chain: String }, // :384
```

The doc comment on `ImportCycle` says "Named types must form a DAG in v1". The status rendering at `errors.rs:688-699` says "recursive types are not supported in v1". Both must change. Related variants that carry sites: `UnknownNamedType{name, namespace, site}`, `ConstraintConflict{…, site, constraints}`, and `UnionReference{…, site, definition: Box<SchemaReferenceSite>}`.

| Guard | Constant | Raised at |
|---|---|---|
| Named-type expansion depth | `MAX_IMPORT_DEPTH = 32` (`resolve.rs:1220`) | `ImportEngine::expand_import` `resolve.rs:1774-1781` → `ImportCycle` ("named-type import expansion exceeded 32 levels near `N@r`") |
| Named-type cycle | `stack.contains(&frame)` | `resolve.rs:1803-1806` → `ImportCycle { chain: describe_cycle(name) }`; `describe_cycle` at `:2023` renders `a -> b -> a` from frame names only (no file, no span) |
| `$schema` file-reference chain | `MAX_REFERENCE_DEPTH = 32` (`resolve.rs:508`), `ReferenceStack` (`:516`) | `ReferenceStack::enter` `:521-533` → `ReferenceCycle` / `ReferenceDepthExceeded` |
| Trigger payload → envelope | — | `triggers/assemble.rs:725` (embedded), `:801`, `:812` (`check_payload_cycles` :794) → `TriggerPayloadCycle` |
| Function-contract reference depth | `MAX_REFERENCE_DEPTH = 16` (`simplified/signature.rs:20`) | `canonical_atoms` :115. Deeper or recursive references compare by name (no error) |
| Inline-object nesting | `MAX_INLINE_OBJECT_DEPTH = 32` (`simplified/grammar.rs:116`) | grammar `:1453`, `simplified/mod.rs:422` |
| JSON translator | `CYCLE_EXPANSIONS 4`, `MAX_REFERENCE_DEPTH 32`, `REFERENCE_BUDGET 4096` (`json_shape.rs:56-65`) | `:723`, validation-only fallback (no error) |
| Grammar translator | `MAX_REFERENCE_DEPTH 32` (`translate.rs:46`) | `:272-276` → `TranslateError::TooDeep`; renamed at `call_binding/signature.rs:225` |
| Coercion engine | `MAX_DEPTH = 64` (`L/coercion/mod.rs:57`) | engine `ResourceLimit` |
| `$ref` property lookup | `MAX_DEPTH = 32` (`L/schemas/mod.rs:808`) | returns false |

## 9. Downstream public consumers (outside darkmatter lib, cli, and dmls)

| Crate | File | Uses |
|---|---|---|
| claudine lib | `src/composition/schema/classify.rs` | `TypeExpr` (matches :288, :420), `PropertyAtom`, `PropertyDef`, `SchemaShape`, `EffectiveSchema` |
| claudine lib | `src/composition/schema/mod.rs` | `TypeExpr` (:994), `EffectiveSchema`, `SchemaShape`, `PropertyDef` |
| claudine lib | `src/composition/schema/supplied.rs` | `TypeExpr` (:394), `SchemaShape`, `PropertyDef`, `EffectiveSchema` |
| claudine lib | `src/composition/schema/translate.rs`, `bundle.rs`, `src/composition/types.rs` | `EffectiveSchema`, `SimplifiedSchema` |
| claudine lib | `src/composition/inline_prompt.rs` | `PropertyDef`, `SchemaShape`, `serialize_property_atom` |
| claudine lib tests | `tests/l1/host_catalog.rs`, `tests/l1/claudine_schemas_embedded.rs`, `src/composition/schema/tests.rs` (`resolve_schema` :2504, :2535) | |
| claudine gen | `src/schema_compat.rs` | own local-type inliner `inline_local_object_types` :70 (`TypeExpr::Imported` let-else :81), `describe_atom` :304 |
| claudine gen | `src/inputs.rs` | `EffectiveSchema`, `PropertyDef` |
| claudine cli | `src/completion/schema_completion/{mod,keys,candidates}.rs` | `EffectiveSchema`, `PropertyDef`, `SchemaShape`, `completion` |
| messenger lib | `src/research/load.rs` | `EffectiveSchema` only |

No other crate calls `to_json_schema` or the resolve functions. The `resolve_schema` hits in `schematic/define` are an unrelated method. The prompts under `claudine/features/**/spike*` and `baseline/` are historical.

## 10. Test targets per area

Paths with `lib:` are `darkmatter/lib/tests/l1/`, `cli:` are `darkmatter/cli/tests/l1/`, and `dmls:` are `darkmatter/dmls/tests/l1/`. In-crate test modules are given by their `#[cfg(test)]` line.

| Area | Existing tests |
|---|---|
| Resolver, graph, and errors (§1 resolver rows, §2, §8) | `resolve.rs` mod tests (:3598), `L/schemas/resolve/searched_tests.rs`, `L/schemas/tests/embedded_sources.rs` (`ImportCycle` :179), lib: `schemas_named_type_merge.rs`, `schemas_union_postfix.rs`, `schemas_union_schema_matrix.rs`, `meta_schema_reference_graph.rs` (`ReferenceCycle`/`DepthExceeded`), `schema_roots.rs`, `shipped_schema_corpus.rs`, `schemas_source_projection.rs`, `suggest_constraint_phase4.rs`; dmls: `named_type_references.rs` |
| New in this feature (untracked) | lib: `recursive_schema_golden.rs` (fixture `fixtures/recursive_schema_golden/manifest.yaml`), `recursive_schema_validator_probe.rs` |
| Lowering (§3) | `convert.rs` mod tests (:1698, :2841), `types.rs` (:935), lib: `schemas_convert_snapshots.rs` (+ `snapshots/`), `schemas_conjunctive_lowering.rs`, `schemas_core_type_grammar.rs`, `schemas_core_type_values.rs`, `schema_grammar_descriptors.rs`, `base_schema_end_to_end.rs`, `global_catalog.rs` |
| Merge (§4) | `assemble.rs` mod tests (:527, e.g. `resolves_mergeable_payload` :998), lib: `global_catalog.rs`, `schema_embedded_trigger_bundles.rs`, `trigger_input_matrix.rs`, `base_schema_end_to_end.rs`; cli: `schema_validate_baseline.rs`, `schema_validate_baseline_selection.rs`, `schema_triggers.rs`, `standalone_baseline.rs` |
| Phases | lib: `schema_phase_validation.rs`, `schemas_effective_presence.rs`; cli: `schema_validate_phase.rs` |
| Synthetic validators (§5) | `validate.rs` mod tests (:1527), `coerce.rs` (:530), `rewrite.rs` (:480), `completion.rs` (:629), `compose/schema_validation.rs` (:112); lib: `schemas_union_report.rs`, `schemas_union_coerce.rs`, `schemas_union_rewrite.rs`, `match_keyword_input_matrix.rs` (unverified) |
| validate and discriminant | `discriminant.rs` (:131); lib: `schemas_validate_table.rs` (fixtures `fixtures/validate/*`, including `json_schema_ref`, `root_union*`, `property_union_*`), `schemas_union_report.rs`, `diagnostic_positions.rs` (uses `position_fixtures/mod.rs`), `schemas_union_fixture_controls.rs` (uses `union_fixtures/mod.rs`); cli: `diagnostic_positions.rs`, `schema_validate.rs` |
| clean (both walks) | `L/schemas/tests/clean_quoting.rs`, `clean_suggestions.rs`; lib: `schemas_union_clean.rs`, `clean_rules.rs`, `clean_parity.rs`, `clean_frontmatter_repair.rs`; cli: `clean_schema.rs` |
| coerce, json_shape, translate | `json_shape.rs` (:1004), `coerce.rs` (:530); lib: `raw_schema_conversion.rs`, `schema_enhancements_matrix/raw_schema.rs`, `coercion_translator_conformance.rs`, `coercion_core.rs`, `coercion_engine_parity.rs`, `frontmatter_coercion_engine.rs`, `function_call_binding.rs`, `function_binding_conversions.rs`, `schemas_union_coerce.rs` |
| rewrite | `rewrite.rs` (:480); lib: `schemas_union_rewrite.rs`; cli: `compose_schema_file_rewrite.rs` |
| Typed consumers (§7) | `completion.rs` (:629), `frontmatter_shape.rs` (:391), `subtype.rs` (:252), `signature.rs` (:206), `matcher.rs` (:531), `detect.rs` (:589), `about.rs` (:878), `catalog.rs` (:535), `compose/context/catalog.rs` (:346, :639), `L/schemas/mod.rs` tests (:2165); lib: `schemas_union_completion.rs`, `schemas_union_typed_consumers.rs`, `schemas_core_type_values.rs` (`is_subtype`), `expression_type_scope.rs`, `design_time_call_check.rs`, `compose_call_precheck.rs`, `global_catalog.rs`, `ambient_ctx_capture.rs` |
| DMLS | `overlay/schema.rs` (:1113), `overlay/mod.rs` (:1170), `overlay/named_types.rs` (:338), `providers/frontmatter/named_references.rs`, `providers/frontmatter/sequence_tests.rs`; dmls: `named_type_references.rs`, `union_item_editor.rs`, `core_schema_types.rs`, `undeclared_property.rs`, `call_check_agreement.rs`, `expression_call_diagnostics.rs`, `expression_scopes.rs`, `lsp_session.rs`; fixtures `dmls/tests/fixtures/sequence_descent` |
| CLI | `assignment.rs` (:261); cli: `schema_validate.rs`, `schema_validate_core_types.rs`, `schema_about.rs`, `schema_union_matrix.rs`, `schema_union_pipeline.rs`, `schema_robustness_matrix.rs`, `compose_schema.rs`, `compose_schema_enhancements.rs`, `schema_validate_shipped_artifacts.rs` |
| Claudine | `claudine/lib/src/composition/schema/tests.rs`, `claudine/lib/tests/l1/host_catalog.rs`, `claudine_schemas_embedded.rs` |

Fixture directories relevant to the resolver:

- `darkmatter/lib/tests/fixtures/{validate, recursive_schema_golden, schema_enhancements_matrix, schema_yaml_spellings, triggers, clean, detect, frozen_prompts}`
- `darkmatter/lib/tests/union_fixtures/mod.rs`, `darkmatter/lib/tests/position_fixtures/mod.rs`
- `darkmatter/cli/tests/fixtures/schema_validate_baseline`

Shared test registration is `darkmatter/lib/tests/l1/main.rs`, which is already modified in the worktree.

## 11. Proposed file ownership by wave

Ownership is disjoint within each parallel wave. Wave 2 is sequential and may touch every exhaustive match to add compile-only `Ref` arms ("Update exhaustive matches to keep packages compiling", plan Wave 2). Semantic ownership then moves to the waves below.

| Wave | Owner set | Files |
|---|---|---|
| 2 (seq.) | Reference graph and storage | `L/schemas/resolve.rs` (graph, `NamespaceKey`, `ImportEngine`, `apply_import_postfix`, `AtomProvenance`), `L/schemas/simplified/types.rs` (`Ref`, `DefKey`, `admits_null`, `union_arms`), `simplified/source.rs` (provenance), `L/schemas/errors.rs`, `simplified/serialize.rs`, `simplified/signature.rs`, `simplified/lint.rs`, `simplified/standalone.rs`, `simplified/query.rs` |
| 2 (seq.) | Lowering | `L/schemas/simplified/convert.rs` |
| 3 (seq.) | Lookup context and synthetic-validator plumbing | `L/schemas/validate.rs` (builders, `wrap_arm_as_root_schema` only), `L/schemas/mod.rs` (`EffectiveSchema`, `CompiledValidators`, `wrapped_root_arms`, `phase_json_schema`, `merged_trigger_schema`, `schema_declares_property`), `L/compose/schema_validation.rs` (`select_file_mode`, `root_schema_arm_applies`), `L/schemas/example.rs` |
| 3 (seq.) | Merge | `resolve.rs` merge section (`merge_baseline`, `validate_simple_object_schema`), `L/schemas/triggers/assemble.rs` |
| 3 (seq.) | Phases | `L/schemas/phase.rs` |
| 4a | Report recursive failures | `L/schemas/validate.rs` (walkers §6), `L/schemas/discriminant.rs` |
| 4b | Cleanup | `L/schemas/clean.rs` |
| 4c | Conversion | `L/schemas/coerce.rs`, `L/schemas/json_shape.rs`, `L/compose/expression/call_binding/translate.rs` (+ `call_binding/signature.rs` `TooDeep` mapping) |
| 4d | Rewrite | `L/schemas/rewrite.rs` |
| 5a | Completion | `L/schemas/completion.rs`, `simplified/query.rs` helpers (if Wave 2 releases it) |
| 5b | Describe and compare | `L/schemas/frontmatter_shape.rs`, `L/compose/context/catalog.rs`, `simplified/subtype.rs`, `L/compose/expression/scope/projection.rs`, `L/compose/expression/scope/shape.rs`, `L/compose/expression/call_check.rs`, `L/schemas/catalog.rs`; audits of `about.rs`, `detect.rs`, `triggers/matcher.rs`, `call_binding/registration.rs`, `expression/catalog/mod.rs`, `shell_expansion/template/{canonical,typing}.rs`, `file_match.rs` |
| 6a | Editor | `darkmatter/dmls/src/overlay/{schema,mod,named_types,suggestions,expressions}.rs`, `providers/frontmatter.rs`, `diagnostics/named_types.rs` |
| 6b | CLI | `darkmatter/cli/src/commands/schema/assignment.rs`, `commands/schema/detect.rs` (audit), new cli tests |
| 7 | Fixtures and docs | fixture dirs (§10), `darkmatter/docs/topics/schemas/*`, `.claude/skills/darkmatter/schema.md` |
| 9 | Downstream audit | `claudine/lib/src/composition/schema/{classify,mod,supplied}.rs`, `claudine/gen/src/schema_compat.rs`, `claudine/cli/src/completion/schema_completion/*` |

Not yet assigned by the plan, and proposed here:

- `L/schemas/catalog.rs` → 5b
- `L/compose/expression/scope/shape.rs` → 5b
- the DMLS `expressions.rs`, `suggestions.rs`, and `named_types.rs` → 6a
- the Claudine files → Wave 9 audit, because no plan wave owns code outside darkmatter
