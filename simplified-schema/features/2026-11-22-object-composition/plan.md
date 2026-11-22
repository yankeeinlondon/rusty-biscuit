---
kind: plan
total-phases: 5
created: 2026-11-22
status: planned
phase: 4
agent: codex/gpt-6.2-sol
yolo: true
source_files_during_phase_1:
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/yaml_documents/semantic.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/object_composition.rs
- simplified-schema/lib/tests/l1/shipped_corpus.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/named_types.rs
- darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
- darkmatter/lib/tests/l1/schemas_source_projection.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- simplified-schema
- darkmatter
- darkmatter-lsp
source_files_during_phase_2:
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/composition.rs
- simplified-schema/lib/src/resolve/graph.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/simplified/convert.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/triggers/assemble.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/tests/l1/object_composition.rs
- darkmatter/lib/tests/l1/schemas_named_type_merge.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- simplified-schema/lib/src/schema/export.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/simplified/serialize.rs
- simplified-schema/lib/src/simplified/cursor.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/object_composition_export.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
---

# Object composition implementation plan

## Summary and Definition of Done

Implement inline `...Name` and YAML `$extends` through the existing named-type resolver. Keep the authored syntax for export and editor navigation. Resolve each operand, choose winning declarations in source order, and then expand those declarations in their original scopes. The receiving object owns its constraints and lowers as one object.

Done means a composed object behaves like its handwritten equivalent in JSON Schema, strict and soft validation, resolved queries, and subtype checks. Authored YAML round-trips in the same resolution context. Recursive schemas work across files, cycles without a descent report their full reference chain, and a replaced declaration causes no template or example work. Disk and embedded trigger fixtures establish dependency behavior. DMLS supports incomplete operands, navigation, located diagnostics, and refresh after an unsaved import edit. The documentation, published caller skill, and scoped checks are complete.

Run phases in order. Tasks within the same numbered wave can run concurrently in the stated file scopes. The coordinator integrates each wave in the session worktree and runs the phase checkpoint. Include in every delegated brief that the entire session tree is non-interactive. Leave the feature active for review after Phase 5.

## Phase 1 — Authored syntax and source model

### Necessary Rules

Paths beginning with `simplified/`, `resolve/`, `schema/`, or `triggers/` in these rules and tasks are relative to `simplified-schema/lib/src/`. DMLS module paths are relative to `darkmatter/dmls/src/`.

1. **Keep the shape's existing property maps and add one authored field.** Add `SchemaShape::composition: Option<ObjectComposition>` in `simplified/types.rs`, with Serde default and omission for `None`. Use `ObjectComposition::Inline(Vec<InlineSpread>)` and `ObjectComposition::Extends(Extends)`, where `Extends` has scalar and sequence variants. An `InlineSpread` stores the numbers of preceding literal and pattern declarations and a `CompositionOperand`. The operand stores `name: String` and `reference: Option<String>` (`None` preserves a bare name). Iterate the literal and pattern streams up to those counts, apply the spread, and flush both streams at the end. Relative order within each kind and across each spread is preserved. Normalize whitespace, trailing commas, and the interleaving of literals and patterns between spread positions. The two member kinds already have separate order in `SchemaShape` and separate output channels in `convert.rs`. Use empty metadata in constructed and resolved shapes. Preserve the serialized form of existing prepared shapes and update exhaustive struct literals in `resolve.rs`, `simplified/mod.rs`, and Darkmatter's `markdown/{schemas,compose}` modules to use the empty field.

2. **Parse operands with the named-reference lexer and without its postfix parser.** In `simplified/grammar.rs`, add one operand reader shared by the inline body and the YAML directive. Use `parse_type_expr`'s identifier and keyword precedence, `read_fileref` and its nesting-specific delimiters, and the normal whitespace policy. A bare built-in name is a grammar error with the `name@this` hint; `$extends: string` also names the reserved directive and explains that a named object is required. Postfixes on the name before `@` or on a bare name are grammar errors, as are descriptions, slots, and empty names or paths. Path text after `@` uses the existing delimiters, so punctuation that belongs to the file reference remains path text. Leave postfix parsing on the receiving object, and preserve the duplicate-literal check in `parse_inline_object_body` even across intervening spreads.

3. **Reserve `$extends` at schema-object boundaries.** Handle it in `simplified/mod.rs::parse_schema_shape`, so nested mappings, mapping-valued named declarations, and root and property union mapping arms share the validator. For standalone `types` carriers, set table mode only at the outer mapping in `simplified/standalone.rs::parse_standalone` and `simplified/source.rs::parse_standalone_schema_payload_with_source`, including the types-only envelope and its source-failure fallback. In that mapping `$extends` is an ordinary table key subject to existing declaration rules. Entry values use object mode. Keep `$constraints` rules, raw JSON Schema, and instance keys under their current parsers. The same standalone parser supplies named-type imports, while `parse_mapping_atom` already handles nested property and union mappings.

4. **Give each operand a stable source address.** Add `SchemaSourcePathSegment::Spread(usize)` and `SchemaSourcePath::spread` for zero-based inline operands. Use `path.property("$extends")` for the YAML scalar and append `.union_arm(index)` for sequence items, matching the YAML node tree. Project an `ImportName`, an `ImportReference` for an explicit file half, and a new `CompositionOperand` span for the complete reference without the ellipsis. Keep raw byte ranges in the source map and errors, and use `DecodedScalar` and existing projection to cross YAML quoting, folding, and aliases. In `source.rs`, extend `project_shape_source` and `project_inline_shape_range`, and use the same paths in `DefinitionFailure` and the resolver. DMLS already finds ordinary named references via `AuthoredDeclaration::reference_at`; the new full span supports exact diagnostic ranges and distinguishes repeated operands.

5. **Assemble before graph traversal or expansion of any member.** Create `resolve/composition.rs` as part of `ImportEngine`. Use a private assembled member record with its complete `PropertyDef`, original `Namespace`, and original `SchemaSourcePath`. Build this view from authored shapes and alias targets, without walking any member's child references. Let `graph.rs::discover_shape`, `ImportEngine::expand_shape`, and `shape_default_files` consume the assembled view. Cache completed target member sets within the engine by `DefKey` and remaining import depth, and check the active target stack before using the cache. Treat assembly at a receiving position as a read of the target's member declarations. Visit every operand in that assembly even when a later entry replaces its entire contribution. Reach a copied member's nested object only when that member survives. Skip the assembly allocation for ordinary shapes. The current graph eagerly walks every declaration it sees, and `expand_shape` immediately expands children, so both passes must receive the merged view to avoid observing discarded members.

6. **Resolve aliases for shape and recurse only through value edges.** Use the same `resolve_namespace`, `DefKey`, `NamedTypes`, and import-depth limit (`MAX_IMPORT_DEPTH = 32`) as `expand_import`. Follow single non-array `TypeExpr::Imported` aliases to a declared `InlineObject`, ignoring the alias and target objects' own qualifiers for the copy. Reject every union, array, primitive, opaque object, callable, and slot-only target. Keep the composition/alias stack as ordered `CycleReference` edges and report the cycle slice as `NonDescendingCycle`; use the ordinary depth failure for an acyclic over-limit chain. Charge composition and alias hops against the currently active expansion depth. Clear the composition stack on return from assembly, before following child definitions. In `graph.rs`, discover ordinary edges through the winning member set, attributing their sites to the declaring member and their descent to the receiving value position. A composition-only target is not a value use or a stored definition. An ordinary use of the same target follows the usual discovery and expansion path. This retains `Graph::admit` and `recursive.rs::{recursive_use,expand_reached_definitions,publish_definitions}` for productive recursion. The base/branch example creates a branch-to-branch child edge and needs no expanded base body to discover it.

7. **Keep namespaces authored.** Snapshot explicit `types` or authored root literals before assembly, following `expand_imports` and `expand_named_types_searched`. Preserve the distinct `DefNamespace` and source path of each implicit root-union arm, and use the existing explicit table for all arms when supplied. The same rule applies to `NamedTypes` extracted from files and to `schema/layer.rs::Layer::select`, which already looks up the authored declaration before consulting its expansion. A copied key is unavailable as a local name, a file export, or a selection. Resolve a copied member's bare and qualified references in its declaring namespace, even when the receiver defines a name with the same spelling.

8. **Replace members atomically and deduplicate patterns by emitted key.** Use `IndexMap::insert` for literal winners, retaining the first insertion position. Separate the pure emitted-pattern-key computation from regex validation in `simplified/convert.rs::emitted_pattern_property`. Assemble literals first, then choose pattern winners in entry order using that pure function with the final literal set and a separate catch-all identity. Retain the winning pattern's authored spelling and provenance. Only the selected definitions reach semantic expansion and regex validation. Preserve the receiving shape's `constraints` and containing atom's postfix. `shape_to_object_schema` already lowers one object's required keys, literal exclusions, catch-all and open/closed policy. It also owns existing constraint-level `allOf` behavior.

9. **Carry winning origins through every consumer.** Expand each member using its authored path and namespace. Apply the existing per-definition slot stamping and relative-example rehoming to the winning declaration in its own file context, before receiver-level work, without rehoming an already attributed example. Keep the receiver path separate for validation and default attribution. Make `shape_default_files` walk the same winners under the receiving property names, applying the current first-arm and unsupported path rules of `ResolvedSchema::default_files`. Use `AtomProvenance` for constraint diagnostics, and carry a transient map from receiver paths (qualified by the body’s `DefKey` for stored definitions) to authored sites into lowering. Wrap a conversion error for a copied member in `SchemaError::ComposedMember { site, source }` and use the deepest matching member address. Local errors retain their existing form. Keep this data out of emitted schemas and serialized prepared shapes. A diagnostic on instance data retains its existing instance location. `expand_imports` runs before slot collection, and `DarkmatterSchemas` uses the default map when it assigns `AppliedDefault.origin`; both must see the winning origin rather than the first declaration for the key.

10. **Keep one failure and dependency path.** Use `UnknownNamedType` for absent names, `NonDescendingCycle` for zero-descent loops, and add `SchemaError::InvalidCompositionTarget { name, site, definition, reason }` with `CompositionTargetKind::{Array, Union, Primitive, OpaqueObject, Callable, Template}` as the reason. Check arrays before their item type. Add `SchemaError::CompositionReference { operand, site, source }` for other operand lookup failures and for depth exhaustion so the original typed cause remains in the error chain. Use the authored target site for an ineligible target's related location, including the concrete declaration reached through an alias. Show the exact operand at the failing site. Reuse `resolve_namespace` for disk, buffers, shipped sources and embedded bundles. Report successful transitive reads and probe attempts through the existing engine sets and `observed` collectors. In `schema/layer.rs`, route content-only schemas containing composition through expansion with an explicit no-file namespace mode. Use the existing unprepared `ResolutionUnavailable` contract for external reads encountered on that path, with the operand context in the source chain. Keep the old content-only path for schemas without composition. The explicit no-file route is needed because `external_reference` scans authored members before winners are known and the existing resolver can otherwise attempt disk selection. Retain existing ordinary whole-schema selection, embedded lexical resolution, and template read policies. A winning template member that returns a type expression with composition follows `expand_template_definition` and the same read gate. Assembly rejects a slot-only operand target without evaluating it.

11. **Keep export lossless where representable.** In `schema/export.rs`, write `$extends` in its original scalar/sequence form before local properties. Write an inline spread object as a scalar via `simplified/serialize.rs::write_inline_object`, including when it would otherwise qualify for mapping export. Emit mapping union arms with `write_atom` so YAML `$extends` survives. For an anchor, alias, or folded operand, write the decoded reference and preserve its syntax and resolution on reparse. For `Layer::select` of an inline-composed named object, record the selected name on the private `AuthoredSchema` used for export. Write its derived root as `$schema: { $extends: "SelectedName@this" }` beside the existing authored `types` table. This table already contains the original inline expression and its lexical scope, and `select` extracts the inline shape without the containing atom’s qualifiers. A selected name outside ordinary named-reference syntax has no equivalent root export and returns `ExportUnsupported`. In the existing limited layered YAML export, return `ExportUnsupported` for root composition in any layer and for a copied lower-layer descendant containing composition; a flattened document cannot faithfully express the winning scopes and ordering of those roots. Preserve existing export of non-composed layers and retained nested compositions in the top layer. `write_layered` already refuses lower-layer properties with named references and other unrepresentable combinations. Leave layer validation and merging in their existing order.

12. **Lower and compare resolved shapes.** Reject surviving composition metadata at the start of shared shape lowering, with the first operand's path, so direct `to_json_schema` never drops a directive. Use `is_subtype` and `is_subtype_with_definitions` on expanded operands without changing subtype policy. `subtype.rs::same_shape` compares the literal and pattern members that the expansion now supplies. Keep existing stored definition identities and ordinary reference lowering.

13. **Use DMLS's current authoring and cache boundaries.** In `simplified/cursor.rs`, expose a composition-specific cursor role that carries name versus import-path state and the raw replacement range. Extend `providers/frontmatter.rs::meta_schema_completion` and its flow and block cursor helpers for scalar and sequence `$extends` values only in schema-object positions. Reuse `AuthoredDeclaration::local_namespace`, `named_references::local_reference_spelling`, and the current overlay catalogs, alongside the existing file-reference completer for the import half. Filter by following known aliases in hydrated catalogs with a visited set. Omit known non-object targets and known cycles; retain candidates whose terminal definition is unavailable to passive completion. Use the file-path suggestions for incomplete imports whose contents are unknown. Offer no primitive, array, or constraint candidates for composition operands. Reuse `reference_at` for hover and definition, pointing at the declaration the user named. Extend the tolerant catalog collection in `overlay/named_types.rs`, and use `standalone_named_types` and frontmatter resolver dependency tracking for unsaved refresh. Those paths already use open source buffers and include probes as well as successful reads. Carry the resolver's error through `diagnostics/named_types.rs` and `overlay/mod.rs::file_failures` for both exact diagnostics and recovery on file appearance.

14. **Keep rollout scoped to the language.** The tracked search for `$extends` in YAML, JSON, Markdown, Rust, and TOML found one schema source use, `claudine/registration/composition.yaml`. It already authors the directive and its `common-attributes` import is absent from `claudine/registration/shared.yaml`. Leave both files to host registration, and use a complete independent fixture. Document migration for legacy literal keys. Use current generator and caller entry points without touching host configuration, layering policy, or schema envelope contracts. No runtime spike is scheduled. The spec calls for behavioral parity and sets no performance threshold, and the graph and consumer traces settle the implementation order.

### Tasks

- [x] Define authored shape
  - Add the types, reexports, source addressing, and empty-field compatibility in Rules 1 and 4. Update constructions so the workspace's direct consumers compile. Name the new types in the public AST docs. Keep the snapshot/record field omitted for ordinary shapes.

- [x] Parse object composition
  - Implement Rules 2 and 3 in the shared grammar and YAML mapping path. Use the existing scanner's nested file-reference delimiters and grammar error projection. Add scalar/list and inline parser cases through the public parse APIs. Prove bare built-in rejection and explicit collision resolution syntax.

- [x] Project operand spans
  - Implement Rule 4 for YAML and inline operands, including each repeated list item, quoted keys and scalars, root-union arms, and malformed values. Assert exact source slices and source-path distinctions through the public source-map and located parse APIs. Share the scanner with the serializer and editor through the existing grammar/source boundaries.

### Input Robustness Matrix

The new field validator is for `$extends`. Every invalid field shape or operand produces a located error before schema evaluation and yields no prepared schema, trigger payload, or slot call. Use the directive value span for a rejected carrier and the individual item or operand span for a rejected element. Use the parser line/column or document span for rejected document shapes. Preserve the loader’s reported line and column in the public error when the existing wrapper only retains its message.

- Absent directive keeps the ordinary authored object. An empty target object is a valid resolved operand, and repeated operands are evaluated in order. A YAML quoted directive key retains its reserved meaning.
- Valid carriers are one nonempty operand string and a nonempty sequence of operand strings. Starting with a valid standalone schema fixture and editing only the tested field, exercise explicit and implicit null, a number, boolean and mapping, an empty string, an empty sequence, a mixed list with a wrong-type item, and an all-invalid list. Include one invalid scalar and one invalid list operand from the shared name parser. Assert successful control preparation and distinguish the null, empty and wrong-type source locations.
- For accepted field syntax prove one flow list and one block list and use a quoted operand in the positive control for source projection. For each additional carrier route (nested mapping, mapping-valued named declaration, inline root-union mapping arm, property-union mapping arm, Markdown frontmatter and trigger payload), run a positive control and one invalid field case. Use `parse_standalone_schema_document`, `SimplifiedSchema::try_from_markdown`, and the public trigger resolver/preparation entry point. Use the phase-specific public result that exposes the originating grammar error; retain original error details through the trigger path.
- Exercise duplicate `$extends` keys, trailing malformed YAML, and a second YAML document once through the standalone admitted YAML loader (`recognize_standalone` → `numeric::load_admitted_yaml`), and once through the claimed trigger envelope loader (`parse_trigger_envelope_from_str` → normalized `serde_yaml_ng`). Check Markdown frontmatter's admitted YAML path against the existing loader coverage and use the new Markdown field-wiring case above. Keep raw JSON Schema and schema-free YAML loaders out of this matrix. The public library results expose the outcomes directly, so run these in process.
- Audit the new parse paths for load-bearing Serde defaults, option fields conflating absence and null, silent string filtering, `unwrap_or_default()`, and discarded parse failures via `.ok()`. The optional AST field's Serde default preserves prepared-record compatibility and is downstream of authored input validation.

Checkpoint: pass the grammar and source-map slices with `cargo nextest run -p simplified-schema -E "test(composition)"` after naming the new focused grammar and span cases with `composition`. Confirm core parser cases compile without default features. The resolver phase runs the public preparation controls and trigger cases that require composition.

## Phase 2 — Resolution and validation

### Tasks

- [x] Assemble winning members
  - Implement Rules 5–8 in `resolve/composition.rs` and wire `expand_imports`, `graph.rs`, and `recursive.rs`. Keep alias resolution for assembly separate from ordinary value expansion. Reuse the no-import fast path for schemas with neither imports nor composition and include composition in `schema_has_imports` and its nested walkers.
  - Keep receiver and source paths distinct. Reuse assembled winners in discovery, both expansion passes, recursive body expansion, and the default walk. Ensure a copied child in a recursive body uses the existing definition queue. Hold the origin for each occurrence even when that occurrence shares its target with another spread.
  - Test local, imported and aliased targets, a built-in collision selected with `@this`, an explicit empty object, and all invalid targets from the spec. Cover aliases with object annotations, authored-only names and exports, explicit and implicit root-union scopes, host-supplied locals through `expand_named_types_with_locals`, and a receiver and file defining the same child type name.
  - Exercise 32 and 33 hops including a mixed alias/operand chain, a pure spread/alias cycle with locations, an ordinary recursive object used outside its cycle, the allowed base/branch cycle locally and across two files, and the same base independently referenced as an ordinary value. Assert lowering and nested validation to prove the recursive definition is usable.

- [x] Preserve winning origins
  - Implement Rules 9–10 with the resolver and lowerer. Carry the typed target and lookup errors through their existing error wrappers, including the terminal `BlockError` implementation in `errors.rs`; follow the biscuit-terminal skill and the existing status block pattern. Carry `CompositionReference` causes through trigger error classification in `triggers/{assemble,finding}.rs`. Retain operand and origin context in the embedded public failure from `resolve_embedded_trigger_payload`.
  - Stamp and rehome winners before `slots::apply_active`, slot collection, and example resolution; keep inherited slots anchored in the declaring file. Preserve ordinary named reference behavior and wrapper precedence. Keep the content-only no-file check at namespace selection, before any probe or read.
  - Test whole-definition replacement including loss of requiredness, defaults, descriptions and examples; test two real files with the same relative example spelling, and assert the winning file and default origin via the library's resolver and Darkmatter's `EffectiveSchema`. Assert `AppliedDefault.origin` in a Darkmatter L1 case. Use an invalid example path and instrumented slot host on replaced declarations to show no read or call. A checked operand with a fully replaced contribution still fails on its missing target. Test a copied member's conversion failure at its declaration and an invalid supplied value at its instance location.
  - Test transitive imports and probe tracking, an invalid imported document retaining its original cause, no file reads from content-only APIs, and runtime plus prepared embedded payloads using `resolve_embedded_trigger_payload` and `prepare_trigger_bundle`. The complete embedded fixture uses a trigger with `$extends: common-attributes@./shared.yaml` and a shared type containing at least one required property and one optional local override. Include a transitive bundle member and assert virtual reads versus empty disk dependencies.

- [x] Prove merged semantics
  - Add the unresolved guard from Rule 12 to shared lowering. Exercise public JSON Schema and `SimplifiedSchema` validation using a handwritten equivalent with equal property order. Assert strict errors, soft repairs/defaults, required keys and a resolved `SimplifiedSchema::property` lookup. Use a common `DefinitionTable` for the case with stored recursive definitions. Preserve public `Layer::select` eligibility from its authored table.
  - Cover spread first, middle and last, spreads-only objects, trailing comma, overwrite in both directions, and duplicate authored literals across a spread. Cover every YAML mapping position from the robustness matrix and prove placement independence.
  - Check pattern collisions using distinct authored spellings with the same emitted regex, including a collision induced by the literal exclusion wrapper, and assert both validators still apply for distinct overlapping patterns. Test catch-all replacement, literal precedence, pattern-only closure, and receiver `closed` and key-count rules. Verify copied target-level constraints and object markers are discarded while an ordinary use of the target retains them. Keep existing constraint-level `allOf` and nested `$ref` behavior.
  - Place the focused public cases in `simplified-schema/lib/tests/l1/object_composition.rs`, registered in `main.rs`. Gate only real disk-resolution cases with `feature = "file"`; local grammar, merge, embedded, lowering, and validation cases run in the minimal build. Add the Darkmatter default-origin case alongside `darkmatter/lib/tests/l1/schemas_named_type_merge.rs`. Use in-process APIs, temporary directories, and existing test harness conventions.

Checkpoint: pass the `object_composition` cases and the existing named-type, recursion, template and embedded cases in the affected test modules. Use nextest package and test filters for this checkpoint. Confirm the raw lowerer refuses an unresolved operand and the authored namespace never acquires imported names.

## Phase 3 — Authored export and editor cursors

### Wave 1

- [x] Round-trip composition
  - Own `schema/export.rs` and `simplified/serialize.rs`. Implement Rule 11. Emit pattern entries in the inline serializer for spread-containing inline shapes using the same key text as block export. Allow pattern keys in the export loss check for spread-containing inline shapes because their serializer writes them. Keep the loss check for `$constraints` on objects serialized inline and for pattern keys on other inline shapes. Route mapping unions through `write_atom` and keep scalar arms on the existing expression path. Refuse a layered export as specified in Rule 11, and preserve legacy output for non-composed shapes.
  - Add export tests in a separate `simplified-schema/lib/tests/l1/object_composition_export.rs`. Round-trip both operand forms, arrays and union arms, explicit `@this`, imported references with path punctuation, spread positions among literal and pattern entries, and repeats. Compare the prepared JSON Schema and verify stable insertion order. Have the coordinator register the test module in `main.rs` after the wave.
  - Test `Layer::select` validation of a composed named object and the derived-root export rule from Phase 1. Test that the original schema still exports and reparses with the spread in its authored named declaration.

- [x] Recognize operand cursors
  - Own `simplified/cursor.rs` and its unit cases. Implement the tolerant operand role and state from Rule 13, with a public scalar-operand cursor entry point for DMLS's YAML `$extends` discovery. Keep one decoder and raw edit range authority for name, `@this`, and file-path states.
  - Test an empty and partial operand immediately after `...`, before and after `@`, inside quoted YAML and multiline scalars, in nested object arrays and union arms, and before a receiving postfix. Cover bare literal keys, a description containing an ellipsis, and document data as negative controls. The YAML sequence framing belongs to the DMLS task in Phase 4; this cursor takes the scalar or partial item located by that framing.

Checkpoint: run the export and cursor slices after integration. Reparse exported documents through `SimplifiedSchema::try_from_yaml` with the original binding context and assert equal resolution.

## Phase 4 — DMLS support and documentation

### Wave 2

- [ ] Complete schema operands
  - Own `providers/frontmatter.rs` and `providers/frontmatter/named_references.rs`. Implement Rule 13 using the Phase 3 cursor role. For incomplete YAML, use the schema-authoring state's semantic paths and the existing flow and indentation scanners to find scalar, flow-sequence and block-sequence operands. Normalize quoted keys through the YAML scanner. Preserve the exact implicit root-union arm when deriving local candidates.
  - Preserve the ordinary eligible local and external name suggestions and import-path conventions. Filter known ineligible targets through the passive catalogs, including alias chains, and replace only the active portion of the operand. Do not run the resolver or read files in a completion request.

- [ ] Track editor dependencies
  - Own `overlay/{mod,named_types}.rs` and `diagnostics/{named_types,frontmatter,codes}.rs`. Collect composition targets in authored AST traversal and in tolerant YAML collection even when a sibling has malformed syntax. Reuse open-buffer reads and existing cache invalidation. Hydrate the catalog with transitive alias targets through the existing overlay read boundary where required for eligibility filtering.
  - Project the full failing operand at its authored site and the terminal target declaration as related information for a known ineligible definition. For an indirect composition failure, keep navigation back to the receiver and a related location at the failing declaration. Retain typed causes for missing-file watches. Reuse the existing named-type diagnostic code family.

- [ ] Update author guidance
  - Own the SimplifiedSchema README, `simplified-schema/docs/topics/language/{objects-and-named-types,index,local-type-references}.md`, `simplified-schema/docs/topics/diagnostics.md`, `darkmatter/docs/topics/schemas/parsing/grammar.md`, `.claude/skills/simplified-schema/SKILL.md`, and `darkmatter/docs/use-skills/use-dmls/schemas.md`. Explain the two composition forms with runnable examples, authored scope, lookup and collisions, order and replacement, origin, constraints and patterns, recursion with one child edge, YAML export, and the migration for literal `$extends`. Use a compact Mermaid flow for assembly, selection, and child resolution.
  - Correct the older rejection of nested mappings in `objects-and-named-types.md` using the existing `parse_mapping_atom` behavior as authority. Document the new diagnostic from Rules 2 and 10 and the exact hover and completion behavior. State that composed-root layers can still validate together and have no single authored YAML export. Keep the README an overview, and use current topic pages as the skill's references. Leave publication for Phase 5.

### Tasks after Wave 2

- [ ] Exercise editor sessions
  - Add `darkmatter/dmls/tests/l1/object_composition_editor.rs` and register it in the existing L1 harness. Follow the `LspWorkspace` and `LspFixture` patterns in `recursive_schema_editor.rs`. Cover valid and partially typed operands in both forms, including quoted and flow/block list items, completion edits and filtered suggestions, implicit union-arm scopes, hover and go-to-definition, and a diagnostic on the second of two operands that name the same file.
  - Open an importer and imported file in the test session with unchanged disk contents. Change the imported definition in its buffer from an eligible object to an ineligible type and back to an object with renamed and newly required members. Wait on the existing diagnostics/session synchronization, then assert the error appears and clears, supplied-value diagnostics refer to the correct document, and reading a consuming Markdown document through the session offers the new keys with the old keys absent. Add a missing-file appearance case using the existing watched-file path. Run in L1 with no real editor or terminal.

Checkpoint: pass the focused DMLS provider and LSP session cases. Verify declaration spans, completion lists, and diagnostics with both on-disk and unsaved sources.

## Phase 5 — Integration and publication

### Tasks

- [ ] Check shipped artifacts
  - Run root `just shipped-schemas` after the parser and resolver changes, before the full suites. The recipes prepare SimplifiedSchema, Darkmatter, and Claudine artifacts in dependency order. Expect byte-identical outputs because the tracked scan found no shipped composition and the new authored AST field is omitted when empty. Correct any incidental serialized drift at the source and confirm the three artifacts remain byte-identical.

- [ ] Verify affected packages
  - Run `just test` and `just lint` from `simplified-schema/` to cover default and minimal feature sets, and `just test` and `just lint` from `darkmatter/` to cover the library and editor together. In the root run `cargo check --locked -p claudine -p claudine-cli -p claudine-gen -p messenger --features messenger/research` for the other direct manifest consumers. The public change adds authored shape metadata, and the repository search found no direct struct-literal migrations in those crates.
  - Resolve failures in the changed paths and rerun failing slices followed by the affected final recipe. Use standard workspace L1 configuration. Run the derived suites sequentially on the same host. The new coverage uses in-process and hermetic LSP cases, and needs no L2, L3, live services, or CI workflow changes. Do not run `cargo fmt`.

- [ ] Publish caller skills
  - From `darkmatter/`, run `just publish-skills` with the repository `md` built from the final tree on `PATH`. Build it with `cargo build --locked -p darkmatter-cli --bin md` from the root and put that build’s debug binary directory first on `PATH` for publication. Check the installed `use-dmls` skill for both composition forms and intact examples. Keep the canonical edit in `docs/use-skills/`.

- [ ] Record verification results
  - Record scope, commands and results, acceptance evidence, and any departures from the spec in `implementation-log.md` beside this plan. Include example diagnostics for an ineligible target and cycle, one composed and handwritten comparison, and the unsaved-buffer session result. Check each phase's completed tasks against the spec before marking it complete. Hand the finished implementation into the review cycle with the active feature directory in place.