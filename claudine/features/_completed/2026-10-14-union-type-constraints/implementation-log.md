---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-14-union-type-constraints/spec.md
plan: claudine/features/2026-10-14-union-type-constraints/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - darkmatter/lib/tests/union_fixtures/mod.rs
    - darkmatter/lib/tests/l1/schemas_union_fixture_controls.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/schemas_union_schema_matrix.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/gen/src/schema_compat.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_union_report.rs
    - darkmatter/lib/tests/l1/schemas_union_coerce.rs
    - darkmatter/lib/tests/l1/schemas_union_rewrite.rs
    - darkmatter/lib/tests/l1/schemas_union_clean.rs
    - darkmatter/lib/tests/l1/schemas_union_completion.rs
    - darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/union_item_editor.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/dmls/docs/hover.md
    - darkmatter/dmls/docs/autocomplete.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_union_pipeline.rs
    - darkmatter/cli/tests/l1/schema_union_matrix.rs
    - darkmatter/cli/tests/l1/schema_union_matrix_cells.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/content-policy/SKILL.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/dmls/tests/l1/union_item_editor.rs
    - darkmatter/dmls/Cargo.toml
docs_updated_during_phase_5:
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
    - darkmatter/lib/tests/union_fixtures/mod.rs
    - darkmatter/lib/tests/l1/schemas_union_fixture_controls.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/schemas_union_schema_matrix.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/gen/src/schema_compat.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/tests/l1/schemas_union_report.rs
    - darkmatter/lib/tests/l1/schemas_union_coerce.rs
    - darkmatter/lib/tests/l1/schemas_union_rewrite.rs
    - darkmatter/lib/tests/l1/schemas_union_clean.rs
    - darkmatter/lib/tests/l1/schemas_union_completion.rs
    - darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/union_item_editor.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_union_pipeline.rs
    - darkmatter/cli/tests/l1/schema_union_matrix.rs
    - darkmatter/cli/tests/l1/schema_union_matrix_cells.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
    - darkmatter/dmls/Cargo.toml
documentation:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/dmls/docs/hover.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
completed_phase: 5
implemented: true
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-gen
---

# Implementation Log for 2026-10-14-union-type-constraints (5 phases)

## Phase 1

### Wave 1 — Verify Prerequisite

- **Baseline revision:** `b2cc72a004845286085b614df3f06ac3f754869e` (tree
  `e6a9af710950365213f2206a366f680849495498`), branch `feat/osc-9-4-protocol`.
  The working tree also had the uncommitted move of
  `2026-10-13-named-type-presence-and-local-names` into `_completed`. That
  move changes no source.
- **The prerequisite is present.** `strip_top_level_presence` no longer exists
  in `darkmatter/lib/src/markdown/schemas/resolve.rs`, and the prerequisite's
  implementation log closes at Phase 6 with "implementation complete". The
  plan's warning about an un-landed prerequisite described an older checkout
  and does not apply here.
- **This feature's surface is not implemented yet.**
  `resolve.rs::apply_import_postfix` still rejects any postfix on an
  `ExpandedDef::Union` with `SchemaError::Convert`: "cannot apply
  `[]`/constraints to the union-typed named type …". The test
  `schemas_named_type_merge::union_postfix_and_already_array_suffix_remain_rejected`
  passes and asserts that rejection for `choice(required)` and
  `choice -> described`. Phase 2 must turn those two rows into acceptance
  rows and keep `list[]` over an already-array type rejected.
- **Baseline tests green on the tree (macOS):**
  `cargo nextest run -p darkmatter --test l1 -E 'test(/schemas_named_type_merge|shipped_schema_corpus|schemas_conjunctive_lowering|schemas_convert_snapshots/)'`
  ran 94 tests, 94 passed.
- **Byte-identity baseline (AC 6).** No CLI prints lowered JSON Schema, so a
  scratch L1 test captured it. It is not a shipped test.
  - Source kept at `baseline/capture_lowering.rs` in this feature directory,
    with rerun instructions in its module doc.
  - It lowers every `.yaml` under the eight shipped schema groups that
    `shipped_schema_corpus.rs` walks, plus the darkmatter lib/cli/dmls and
    content-policy test fixtures, through `$schema: ./file`. It also lowers
    every inline `$schema` in shipped Markdown and Markdown fixtures through
    `resolve_yaml_schema`.
  - Each entry is `serde_json::to_string_pretty(json_schema)`, which keeps
    serializer order, or `ERROR <message>` with the repository root replaced
    by `<root>`.
  - 347 entries: 322 lowered and 25 errors (drafts and rejected envelopes).
    The full capture is 3.6 MB, so only the per-entry BLAKE3 manifest
    (`biscuit_hash::blake3_hash`) is kept: `baseline/lowered-corpus.blake3`.
    Two consecutive captures were byte-identical (`cmp`).
  - Phase 5 reruns the capture on the implemented tree and compares the
    manifest. For any differing entry, it reruns the capture in a temporary
    worktree at the baseline revision to diff the full bytes.
  - The scratch module and its `mod` line were removed from
    `darkmatter/lib/tests/l1/` afterwards. `git status darkmatter` is clean.
  - Inline test-string resolver fixtures are already guarded by exact
    assertions: the `schemas_convert_snapshots` insta snapshots, and
    `schemas_conjunctive_lowering::schemas_without_repeats_keep_their_lowered_output`.

### Wave 1 — Inventory Consumers

Method: the Wave 2 prototype added the variant, and `cargo check --all-targets`
over `darkmatter`, `dmls`, `darkmatter-cli`, `claudine`, `claudine-cli`, and
`claudine-gen` listed every exhaustive match it forces open. An Explore pass
then audited the JSON Schema walkers and the wildcard, `matches!`, and
`let-else` sites that the compiler cannot flag. Line numbers are at the
baseline revision.

**Compile-forced sites (21).** Phase 2 must touch each one so the packages
keep compiling.

| Crate | Sites |
|---|---|
| `darkmatter` (12) | `resolve.rs` `expand_atom` (~1344), `atom_has_imports` (~2245); `simplified/convert.rs` `atom_fragment_without_null_wrap` (~408); `simplified/serialize.rs` `serialize_property_atom` (17); `simplified/source.rs` `atom_has_suggestions` (~2247); `simplified/lint.rs` (~184); `simplified/query.rs` `inline_object_shape` (~234); `completion.rs` `is_completable` (~218), `kind_for_atom` (~249); `detect.rs` `primitive` (~429); `triggers/matcher.rs` `value_matches_type` (~257); `compose/context/catalog.rs` (~293) |
| `dmls` (5) | `providers/frontmatter.rs` 1310, 1370, 2051; `overlay/named_types.rs` 303; `overlay/suggestions.rs` 115 |
| `darkmatter-cli` (1) | `commands/schema/assignment.rs` `atom_prefers_string` 189 |
| `claudine` (2) | `composition/schema/classify.rs` `interactive_shape_for_atom` 278, `type_label_for_atom` 402 |
| `claudine-gen` (1) | `gen/src/schema_compat.rs` `describe_atom` 304 |

**Claudine is a compile-level consumer.** Adding the variant breaks
`claudine` and `claudine-gen`, so Phase 2 must update those three matches.
That is in scope for keeping affected packages compiling, and the plan's
Phase 5 downstream checks must include `claudine` and `claudine-gen`.
`content-policy` and `messenger` do not depend on darkmatter's schema AST.
The only `content-policy` dependency on darkmatter is a spike under its
`features/`.

**JSON Schema walkers** (`darkmatter/lib/src/markdown/schemas/`):

- `validate.rs`:
  - `unwrap_nullable_arm` (~1133) returns a genuine union unchanged.
  - `resolve_problem_description` (~1015) takes `items` and then
    `properties[seg]`, so `/content_policy/1/rule` reaches
    `items = {anyOf}` and finds nothing.
  - `build_problems` (~631) merges every arm's nested errors (closest-arm
    reporting is root-only, in `collect_root_union_problems` ~559).
  - `narrow_property_union_problems` (~1228) narrows only
    `properties[key].anyOf` with an object value, so it never reaches an item
    union.
- `discriminant.rs`: the shared selector is
  `select_literal_discriminant_arm(arms: &[Value], instance: &Value) -> Option<usize>`
  (~40). It ignores the null sentinel, and the instance must be an object.
  Other callers are `frontmatter_shape.rs` (~296) and DMLS
  `discriminated_arm_shape`.
- `coerce.rs`:
  - Its own `unwrap_nullable_arm` (~113) matches only `[null, T]` and the
    optional-file triple.
  - `coercion_target` (~151) sends `anyOf` to `target_from_any_of` (~240,
    boolish/numberlike only), and an array to `coercion_target(items)`, which
    returns `None` for an item union. So union items get no coercion today.
  - Genuine unions are handled only at property level, by
    `coerce_property_union` (~459).
- `rewrite.rs`: `rewrite_array_items` (~386) recurses through
  `rewrite_property` (~234), which already takes the
  `rewrite_property_union` (~288) path for an `anyOf`. Union items probably
  work already, but no test covers it.
- `clean.rs`: `schema_requires_string` (~479) and `nearest_declared_key`
  (~617) stop at an item union, which is safe. Auto-repair is gated on
  exactly one raw problem (`analyze_frontmatter` ~271), and
  `schema_quoting_safety::multiple_problems_block_auto_apply` pins that gate.
  See the AC 4 finding below.

**Typed-AST sites the compiler does not force.** Each needs a deliberate
`TypeExpr::Union` decision in Phases 3–4:

- `phase.rs` (`project_atom` ~111, `atom_has_property_eager` ~138).
- `frontmatter_shape.rs` (`atoms_of` ~207, `arm_discriminant_json`,
  `discriminated_arm_shape`, `merged_inline_object_shape`,
  `shape_discriminant_json`).
- `simplified/query.rs` (`atoms_of` ~258, `suggest_atom_in_def`, string/number
  `matches!` ~105).
- `completion.rs` (`first_completable_atom` ~206).
- `detect.rs` (`atoms_of`, `widen_atoms`, `strip_required`/`mark_required`).
  Detection never produces a union atom, so it needs only a passthrough.
- `triggers/matcher.rs` (`type_conforms` ~228, nested-union branches).
- `compose/context/catalog.rs` (~46, ~673; `ctx` never declares unions).
- `file_match.rs` (~216) and `expression/catalog/parser.rs` (~350).
- DMLS:
  - `providers/frontmatter.rs` `array_item_def` (~2009) strips `is_array`,
    which yields `Single(atom{ty: Union})`. `materialize_shape` (~1846) and
    `inline_object_shape` (~2051) then miss the object arms.
  - `overlay/schema.rs` (~127, ~130, ~848, ~876, ~885).
  - `overlay/suggestions.rs` and `overlay/named_types.rs` are exhaustive
    sites. They must recurse into union arms to find nested references.
- CLI `commands/schema/assignment.rs`: early return on `is_array` (~186),
  and `PropertyDef::Union` only (~177). `about.rs` has no AST matches.
- Claudine:
  - `composition/schema/classify.rs` (~82, ~95, ~187).
  - `composition/schema/mod.rs` (~835).
  - `composition/schema/supplied.rs` (~115, ~394).
  - CLI `completion/schema_completion/keys.rs` (~276) and `candidates.rs`
    (~53).
- `simplified/cursor.rs` is text-based and unaffected.
  `compose/schema_validation.rs` constructs union atoms only in tests.

**Existing helpers to reuse:**

- lib:
  - `schemas_named_type_merge.rs::resolve_inline`.
  - `schemas_conjunctive_lowering.rs::Fixture`.
  - `request_support::context_at`.
  - `schema_quoting_safety.rs`'s `effective_for` + `auto_apply` pattern for
    clean.
- CLI: `tests/common/fixture.rs::CliProcessFixture` (`write_file`,
  `command`, `command_builder`).
- DMLS: `tests/common/mod.rs` `LspWorkspace`/`LspFixture`, with the pattern
  in `tests/l1/undeclared_property.rs`.
- lib, CLI, and DMLS all set `autotests = false` and have one `l1` target
  (`tests/l1/main.rs`). A new module must be declared there.

### Wave 2 — Prove Representation (decision record)

The spike was run once. Its code is saved as
`spike/union-array-representation.patch` (applies cleanly to the baseline)
and reverted from the source tree.

**Decision: one new type-expression variant,
`TypeExpr::Union(Vec<PropertyAtom>)`**, an ordered, flattened list of
alternatives carried by a `PropertyAtom`.

- **Array of a union:** `PropertyAtom { ty: TypeExpr::Union(arms), is_array: true, .. }`.
  This is the only encoding of an array whose items are a union. The atom's
  `array_constraints` are the container constraints, its `description` is the
  container description, and `constraints` holds only property markers
  (`required`/`generated`/`eager`).
- **A union reference with a postfix but no `[]`** (`U(required)`,
  `U(eager)`, `U -> text`): the same variant with `is_array: false`. The
  container atom carries the markers and description. Distributing markers
  onto arms was rejected for two reasons:
  - `eager` on a `file` arm would change that arm's existence semantics,
    which the spec forbids.
  - A union-level description would still have no home.
- **A bare `U` stays today's inlined `PropertyDef::Union`.** No existing
  schema can contain a postfixed union reference (it is rejected at
  baseline), so every existing schema keeps today's tree and lowering. That
  is what makes AC 6 byte identity hold by construction.
- **Rejected alternative:** a `PropertyDef::Union { arms, is_array, … }`
  container. It would break every `PropertyDef::Union(arms)` pattern (more
  sites than the 21 above). It also could not express an array of a union as
  one arm of another union. The follow-on `(A | B)` / `(A | B)[]` grammar
  (`2026-10-28-schema-enhancements`) would naturally parse into an atom with
  this same variant.
- **Consumer rule for Phases 3–4:** wherever a consumer looks for union
  alternatives, it must treat `PropertyDef::Union(arms)` and
  `Single(atom)` with `atom.ty == TypeExpr::Union(arms)` alike. Item
  consumers descend `is_array` and then read `TypeExpr::Union` as the item
  alternatives. The Phase 2 owner should add one shared accessor (for
  example on `PropertyDef`/`PropertyAtom` in `types.rs`) so consumers do not
  each re-derive this.

**Grammar fact that changes the plan's "pre-array" wording.** A named
reference has one constraint list. `U[](c)` is legal, but `U(c)[]` is a
parse error ("`[]` must come before the constraint list in a named type
reference"). After `[]` the grammar parses that list at the array level into
`atom.constraints` (not `array_constraints`). So:

- In `U[](…)`, the resolver must move the reference's list into the
  container's `array_constraints`. The spike does this.
- "Pre-array value constraints" on a union reference cannot be written.
  "`eager` before `[]`" is not expressible for a reference; `eager` reaches
  an array of a union only as `U[](eager)` or inherited from an arm.
- Value constraints after `[]` (`U[](pattern(x))`) are rejected by the
  existing array-level applicability check (`apply_array_constraints`:
  "constraint `pattern` is not valid at the array level"). Phase 2 must turn
  that into the spec's typed error.
- Without `[]`, `U(max(10))` and the other value constraints are rejected by
  name. The spike allows only `required`/`generated`/`eager` there.

**Provenance flow:**

- The container's use-site constraints get `authored_origins(…, &site.source, &site.path)`
  exactly as a single-atom postfix does.
- Arms keep their own definitions verbatim: the resolver expands them in
  their defining namespace through `expand_union_arms`, so multi-hop scope,
  dependency edges, cycle errors, and depth limits are unchanged.
- The authored `U[]` reference survives in the passive tree (`TypeExpr::Imported`
  with `is_array`). That already round-trips through
  `serialize_property_atom`, and only the resolved tree holds
  `TypeExpr::Union`.

**Marker hoisting is computed, not copied.** `PropertyAtom::is_required`
(and, in Phase 2, the generated/eager queries) also consults the arms of a
`TypeExpr::Union` type. The arms keep their constraints unchanged, so an
item arm lowers exactly as the same arm does in a property union (AC 7
parity). A string `min(3)` on an arm stays `minLength: 3` on that item arm
and never becomes `minItems`. The spike proved this.

**Serializer/display contract:**

- `serialize_property_atom` must stay total.
- For `TypeExpr::Union` it may produce only a display form, never a claimed
  round-trippable expression. The spike printed `[a, b][]`, which is not
  parseable as a type expression. Phase 2 should keep that or choose another
  explicitly display-only spelling, and document on the function that only
  passively parsed atoms round-trip.
- `md schema about` and the descriptor catalog describe the alternatives
  themselves (Phase 3).

**Lowering (spike-proven):**

- `atom_fragment_without_null_wrap` lowers `TypeExpr::Union` to
  `{ anyOf: [arm fragments] }` and calls each arm with
  `allow_optional_empty_file = false`. Items get no null wrapper.
- The optional-property null wrapper goes only on the enclosing property,
  through the existing `atom_to_schema`.
- Container `minItems`/`maxItems`/`uniqueItems` come from
  `apply_array_constraints`.
- For a non-array `TypeExpr::Union`, Phase 2 should lower through the
  existing `union_property_to_schema` logic: a flat `anyOf: [null, arms…]`,
  hoisted equal defaults, `x-darkmatter-generated`, plus the container
  description. Then `U(required)` lowers like a property union with a
  required arm.

**Nested unions:**

- A bare union reference used as an arm keeps today's flattening
  (`expand_union_arms`).
- An arm that is itself a postfixed union reference (`[choice(required), number]`)
  should stay one nested `TypeExpr::Union` arm rather than be flattened.
  Flattening it would mean pushing its markers onto arms (the `eager`/`file`
  problem) or dropping its description. Phase 2 confirms this and tests it.
- The "an arm is already an array" check for `U[]` must look through nested
  `TypeExpr::Union` arms.

**Ownership for later waves:**

- Phase 2 owns `types.rs`, `resolve.rs`, `simplified/{convert,serialize,source,lint,query}.rs`,
  `errors.rs` (structured union errors), and the compile-only passthrough arms
  at the remaining 21 sites, including `claudine` and `claudine-gen`.
- Phases 3–4 own semantic consumer changes per the plan's file split.

**Spike evidence (macOS, both passed):**

- `spike_union_array_lowers_to_items_any_of_and_validates`:
  - `content_policy: policy[](min(1))@./policy.yaml` lowers to
    `anyOf:[{type:null}, {type:array, minItems:1, items:{anyOf:[string, string, {object … required:[rule, action]}]}}]`.
    `short_form`'s two arms flatten beside `long_form`, and `rule:
    short_form(required)` lands in `long_form`'s `required` list.
  - Every AC 1 data case gives the expected outcome: the mixed list,
    missing, and `null` pass; `[null]`, a scalar, `[]`, `[Duration(3mo)]`,
    missing or `null` `rule`, and `action: delete` fail.
- `spike_inherited_marker_hoists_without_distributing_array_bounds`:
  - `choice: [string(required; min(3)), number]` with `list: choice[](max(2))`.
  - `list` is required, has no null wrapper, and has `maxItems: 2`.
  - It has no `minItems`, and item arm 0 keeps `minLength: 3`.

### Wave 2 — Prepare Fixtures

- **Shared fixture module:** `darkmatter/lib/tests/union_fixtures/mod.rs`,
  std-only so the CLI and DMLS suites can `#[path]`-include it in later
  phases.
  - It is declared from `darkmatter/lib/tests/l1/main.rs` beside
    `request_support`, and `test_layout` passes.
  - Its doc tells each caller to read the shipped schema with its own
    `include_str!` inside the `#[test]`, so CI's test-input index keeps
    narrowed cells.
- **Contents:**
  - `content_policy_bare_schema(shipped)`: bare same-file references.
  - `content_policy_union_schema(shipped)`: the same, plus `long_form.rule:
    short_form(required)`.
  - Both edits require exactly one match, so a moved shipped line fails
    loudly instead of testing an unmodified schema.
  - `CONTENT_POLICY_LIST` (`policy[](min(1))@./content-policy.yaml`) and
    `CONTENT_POLICY_CASES` (AC 1's ten data cases with expected validity).
  - `TAGGED_TYPES`, `OVERLAPPING_TYPES`, `FILE_TYPES` (eager and lazy `file`
    arms), `MULTI_HOP_FILES` (`outer.yaml` → `middle.yaml` with a bare
    in-file arm), and `REPAIRABLE_SIBLING` (`label: string` / `label: 1.20`).
- **Controls** (`darkmatter/lib/tests/l1/schemas_union_fixture_controls.rs`,
  4 L1 tests, all passing at baseline):
  - `content_policy_fixture_edits_only_the_three_reference_lines`.
  - `bare_content_policy_fixture_validates_each_entry_as_a_policy_property`:
    each entry validated as a single `policy@` property, including that
    `rule` is still optional.
  - `union_fixtures_validate_as_property_unions`: tagged selection,
    overlapping arms, eager-file plus number, and the multi-hop `word`
    pattern.
  - `repairable_sibling_is_quoted_by_clean_analysis`: clean's proven quoting
    repair applies when the sibling is the only problem.
- **Process setup:** every fixture uses a canonicalized tempdir with
  `request_support::context_at`, and loads documents from disk with
  `Markdown::try_from(path)`. No HOME, process directory, provider, shell,
  network, or terminal/browser window is used.

#### Input Robustness Matrix — current loader behavior (probed at baseline)

A scratch probe (not kept) drove every schema-surface cell through the
public `DarkmatterSchemas::validate` on a document loaded from disk. It used
the baseline-legal `policy@./policy.yaml` (no `[]`), since loader behavior
does not depend on the postfix.

| Cell | Imported `kind` | Imported `types` | Imported `types.policy` | Document `$schema` | `$schema.content_policy` |
|---|---|---|---|---|---|
| Control | resolves, valid | resolves | resolves | loads | resolves |
| Absent | fails (raw-schema fallback also rejects: "neither a valid standalone SimplifiedSchema nor a valid JSON Schema") | fails ("tagged schema documents support only `kind` and `types`; found unsupported keys: " with an **empty** key list; message defect, outcome correct) | fails `UnknownNamedType` | no schema; validates | n/a (no declaration) |
| Null | fails | fails "`types` must be a mapping" | fails "property value must be …, got null" | fails unsupported-shape | fails "got null" |
| Wrong whole type | fails | fails | fails "got number" | fails | fails "got number" |
| One wrong element | fails | fails | fails "union arm[1] … got number" | fails "root union arms must be a mapping or a file reference" | fails "arm[1]" |
| Every element wrong | fails | fails | fails "arm[0]" | fails | fails "arm[0]" |
| Empty | `''` fails | `{}` → `UnknownNamedType policy` | `[]` fails "at least one arm"; `{}` → object type, control value fails validation | `{}` loads an explicit empty shape (valid); `[]` fails "at least one arm" | `''` fails "expected a type name"; `[]` fails; `{}` → object, value fails validation |
| Duplicate key | fails loading | fails loading | fails loading | fails loading ("duplicate entry with key `$schema`") | fails loading ("duplicate entry with key `p`") |
| Trailing / invalid | fails loading | same | same | invalid frontmatter fails loading | same |

Document-data duplicates (`p` twice, and `rule` twice inside an item) fail
loading ("duplicate entry with key"). So **no loader gap exists on these
surfaces, and Phase 2 needs no loader fix.**

Two traps for test authors:

1. `Markdown::from(&str)` swallows a frontmatter parse error and treats the
   block as absent. With it, duplicate keys and invalid YAML appear to
   *validate*. Matrix tests must load through `Markdown::try_from(path)` or
   `Markdown::try_from_content`, the paths `md` uses. This repeats the
   prerequisite's open observation.
2. Schema-file duplicate keys, trailing documents, and invalid YAML all
   surface as the generic "neither a valid standalone SimplifiedSchema nor a
   valid JSON Schema", which drops the YAML cause. The outcome is correct.
   Assert on the error variant or file, not on cause text.

**Smells grep for the matrix:** the load-bearing reads on these surfaces are
`parse_standalone_schema_document` (`kind`/`types`), `parse_yaml_schema` /
`parse_property_definition` (definitions and `$schema`), and the frontmatter
loader (`parse_yaml_map_rejecting_duplicates`). The probe found no absent/null
collapse or element filtering on any of them. Phase 2 re-greps
`resolve.rs`/`simplified/mod.rs` changes for `#[serde(default)]`,
`unwrap_or_default`, `filter_map(.. as_str())`, and `.ok()` before matrix
sign-off.

#### Finding for Phase 3 — AC 4's "repairable sibling is still repaired"

`clean::analyze_frontmatter` auto-applies its proven quoting repair only when
the raw query finds **exactly one** problem.
`schema_quoting_safety::multiple_problems_block_auto_apply` pins this on
purpose. A document whose union item fails every arm *and* whose sibling is
repairable has at least two raw problems, so today the sibling is reported
but not repaired. AC 4 as written ("cleanup leaves both items unchanged …
while a repairable non-union sibling property is still repaired") cannot
hold in one document without weakening that safety gate.

This does not affect Phase 2. Recommendation for the Phase 3 clean owner:

- keep the gate;
- assert the unrepaired invalid union items in one document;
- assert the sibling repair in a document whose union list is present and
  **valid**, which proves the walker does not stop at union items;
- record the departure, unless the author rules otherwise.

### Phase 1 — Verification and close

**Requirement-to-test mapping for this phase.** Phase 1 changes no
production behavior. Its deliverables are evidence, fixtures, and a
decision.

| Phase 1 requirement | Evidence |
|---|---|
| Prerequisite present and green | `schemas_named_type_merge`, `shipped_schema_corpus`, `schemas_conjunctive_lowering`, `schemas_convert_snapshots`: 94/94 (macOS). `union_postfix_and_already_array_suffix_remain_rejected` passes, which proves this feature's surface is still absent. |
| Byte-identity baseline captured | `baseline/lowered-corpus.blake3` (347 entries; two runs identical) plus `baseline/capture_lowering.rs` |
| Consumer inventory | Compile-forced list from `cargo check --all-targets` over the six crates. Walker and wildcard audit above. |
| Representation spike: mixed scalar/object item union and an inherited marker, without distributing array constraints | `spike_union_array_lowers_to_items_any_of_and_validates`, `spike_inherited_marker_hoists_without_distributing_array_bounds`. Both passed, and both are preserved in `spike/union-array-representation.patch`. |
| Fixtures seeded from the shipped schema, hermetic, reusable | `schemas_union_fixture_controls::{content_policy_fixture_edits_only_the_three_reference_lines, bare_content_policy_fixture_validates_each_entry_as_a_policy_property, union_fixtures_validate_as_property_unions, repairable_sibling_is_quoted_by_clean_analysis}` |
| Matrix loader behavior known before Phase 2 | Probe table above: no loader gap |

**Gates (macOS, this host):**

| Area | Command | Result |
|---|---|---|
| `darkmatter/` | `just test` | 9303 passed, 11 skipped (30.8 s) |
| `darkmatter/` | `just lint` | exit 0; zed-dmls wasm32-wasip2 check passed |
| `claudine/` | `just test` | 8683 passed, 9 skipped (58.7 s) |
| `claudine/` | `just lint` | exit 0. The only warning is the unrelated macOS linker note `__eh_frame section too large` on the `claudine` binary. |

**OS evidence:**

- `just cross-check darkmatter --os linux schemas_union_fixture_controls`:
  4/4 passed.
- `just cross-check darkmatter --os windows schemas_union_fixture_controls`:
  4/4 passed.
- WSL2 was not run. The new tests are std-only and tempdir-based, with no
  platform paths.

**Other notes:**

- The new tests are L1 and run in a fraction of a second each. They open no
  terminal or browser window. Their only repository read is
  `include_str!("../../../../content-policy/schemas/content-policy.yaml")`
  inside each `#[test]` that needs it. No CI cell, gate, or dependency was
  added.
- **Skipped or deferred:** no Phase 1 task was skipped. No pre-existing
  failure was encountered. The spike's production-quality version (typed
  errors, the shared accessor, consumer semantics) belongs to Phases 2–4 by
  plan.
- **Skills:** the `claudine` skill needed no update. This phase changed no
  Claudine code or workflow, only test fixtures in `darkmatter`. The
  `darkmatter` skill update is scheduled for Phase 4.
- **`cargo fmt` was not run.** Nothing was staged or committed.
- **State:** Phase 1 complete. Phase 2 may proceed. `human_review: false`,
  because nothing here needs a ruling before Phase 2. The AC 4 clean finding
  is addressed to Phase 3 through `message_to_agent`.

## Phase 2

### Wave 3 — what was built

Started from `spike/union-array-representation.patch` (source files only;
the spike test was not applied) and replaced every `SPIKE` marker with
production code.

**Typed model (`simplified/types.rs`).**

- `TypeExpr::Union(Vec<PropertyAtom>)` is documented as resolver-only: the
  one array-of-union encoding with `is_array`, and the carrier of use-site
  markers/description over a union without it.
- Shared accessors for later phases:
  - `PropertyAtom::union_arms()` returns the arms of a `TypeExpr::Union`
    (item alternatives when `is_array`).
  - `PropertyDef::alternatives()` returns property-level alternatives
    whichever way they were written: `PropertyDef::Union(arms)`, or a
    non-array `Single(atom)` whose type is a union. It returns `None` for an
    array of a union.
- `PropertyAtom::is_required`, the new `is_generated`, and `has_default`
  read through union arms. Nothing is copied onto the container.

**Resolver (`resolve.rs`).**

- `apply_import_postfix` gained a `definition: &SchemaReferenceSite`
  (target namespace source plus `root.property(name)`), built in
  `expand_import`. A union-typed base goes to the new `apply_union_postfix`.
- Without `[]`: only `required`/`generated`/`eager` are accepted. Anything
  else, including `default`, `min`, `max`, `pattern`, and `unique`, is
  rejected by keyword.
- With `[]`: the reference's one constraint list belongs to the container.
  Accepted: the three markers plus `min`/`max` (already `MinItems`/`MaxItems`
  at the array level), `unique`, `default`, and `example`, which mirrors
  `convert::apply_array_constraints`. Anything else is rejected with
  `array_level: true`.
- `[]` over a flattened arm whose outer value is an array is rejected, looking
  through nested postfixed union arms. Arrays inside an object arm are fine.
- Arm defaults under `[]` lift to the container only when they are JSON
  arrays, merging under the prerequisite's equal-value rule through
  `merge_constraints`. Any other arm default is rejected.
- Container constraints merge through `merge_constraints`, so contradictory
  bounds produce the existing `ConstraintConflict { kind: Bounds }`.
- Multi-hop scope, dependency edges, cycle detection, and `MAX_IMPORT_DEPTH`
  are untouched. Arms are still expanded by `expand_union_arms` in their
  defining namespace.

**Structured error.** The new variant is
`SchemaError::UnionReference { name, rejection: Box<SchemaUnionRejection>, site, definition: Box<SchemaReferenceSite> }`.

- `SchemaUnionRejection` lives in `simplified/source.rs` beside
  `SchemaConstraintConflictKind` and is re-exported from `schemas`. Its
  variants:
  - `Constraint { keyword, array_level }`.
  - `ArrayArm { arm }`.
  - `ScalarDefault { arm, value }`.
- Display example: `p: constraint `max` cannot apply to a union-typed type (named type `choice`)`.
- It has a `BlockError` status block.
- The two fields are boxed because clippy's `result_large_err` failed every
  function returning `Result<_, SchemaError>` once the variant reached 160
  bytes.
- Downstream code matches on the fields, never on the message.

**Lowering (`simplified/convert.rs`).**

- The per-arm loop and default hoisting moved out of
  `union_property_to_schema` into `union_arm_fragments`. Two callers use it:
  - the property union;
  - the `TypeExpr::Union` branch of `atom_fragment_without_null_wrap`, used
    for items and for a union nested as an arm.
- `union_property_to_schema` takes `container: Option<&PropertyAtom>`. A
  non-array union atom (`U(required)`, `U -> text`) lowers as a property
  union, with the container's markers and its description placed at the
  union level.
- Items get no null wrapper and no optional-empty-file arm. The optional
  wrapper goes only on the enclosing property, through `atom_to_schema`.
- A default that reached an item union would be a converter error. The
  resolver prevents it.
- `atom_is_generated` was replaced by `PropertyAtom::is_generated`, which is
  identical for non-union atoms.

**Other `simplified/` sites.**

- `lint.rs` lints each union arm's `suggest` with its arm index. The resolved
  schema is linted too, so returning early would have hidden arm candidates.
- `query.rs::atoms_of` descends union alternatives, including item arms.
- `serialize.rs` documents that the bracketed `[arm, arm][]` form is display
  only and that only passively parsed atoms round-trip.

**Compile-forced sites outside the lib.** Each is a deliberate branch, not a
wildcard.

- CLI `assignment.rs`: a union prefers string if any arm does, matching
  `PropertyDef::Union`.
- DMLS:
  - `named_types.rs` and `suggestions.rs` recurse into, or explicitly skip,
    union arms. These walk passive schemas only.
  - `frontmatter.rs` hover labels a union `union`/`union[]`.
  - `inline_object_shape` returns `None` for a union.
- `claudine-gen` `describe_atom` labels a union `union`/`union[]`.
- Claudine `classify.rs`:
  - The label is `union`/`union[]`.
  - `interactive_shape_for_atom` returns `None` (no single widget).
  - `is_required`/`is_eager` now go through `any_atom_constraint`, which
    reads union arms. Without that, Claudine would treat `U[]` with an
    inherited `required` as optional.

**DMLS error projection (`diagnostics/named_types.rs`, `overlay/mod.rs`).**

- `UnionReference` is a schema-definition error
  (`dm.schema.invalid_type_definition`), ranged on the reference name.
- Its `relatedInformation` points at the union's definition, through the new
  `definition_location`, which works for the current buffer or a file.
- Both sources are dependency edges.

### Departures and findings

- **`default([...])` is a string in today's grammar.** `arg_to_json` reads
  `[a]` as the string `"[a]"`. A probe confirmed `string[](default([a]))`
  lowers to `default: "[a]"`. `definition.md`'s array table nevertheless
  says "`default([...])` — Default array". This is pre-existing drift and
  was not changed here. Consequences:
  - An arm can never carry an array default, so every inherited arm default
    under `U[]` is rejected as `ScalarDefault`, including
    `string(default([a]))` (value `"[a]"`).
  - The lift-and-merge path the spec requires is implemented but cannot be
    reached through the current grammar, so no public test covers it.
  - A use-site default on `U[]` lowers exactly as on any other array
    (asserted by parity).
  - Phase 4's docs owner should not document array defaults as working.
- **The prerequisite's rejection test changed.**
  `schemas_named_type_merge::union_postfix_and_already_array_suffix_remain_rejected`
  asserted `choice(required)` and `choice -> described` fail. Those are now
  this feature's accepted forms, so it was renamed
  `already_array_suffix_remains_rejected` and keeps only the
  `list: string[]` / `list[]` row. The accepted forms are covered in
  `schemas_union_postfix`.
- **Nested postfixed union arms stay nested** (`[pair(required), boolean]`
  lowers to `items.anyOf: [{anyOf: [...]}, {boolean}]`), as Phase 1 decided.
  The nested arm's `required` hoists through both levels.
- **Left for later phases:**
  - `query.rs::suggestions_for_def` reports `is_array` from the arm, so
    suggestions on an array-of-union item report `is_array: false` (Phase 3
    completion owner).
  - DMLS hover shows `union[]` rather than the alternatives (Phase 3).
  - Problem paths below an item are already reported at depth
    (`/p/0/tags`), but description/node lookup through `items.anyOf` is
    Phase 3's `validate.rs` work.
- **Stale until Phase 4:** `definition.md`, the shipped content-policy
  schema comment ("a union-typed reference cannot take constraints today"),
  and the darkmatter skill still describe the old rejection. The plan
  assigns them to Phase 4, and the shipped schema stays unchanged by rule.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| AC 1: Content Policy list (all 10 data cases), file unchanged after validation | `schemas_union_postfix::content_policy_list_validates_the_acceptance_cases` |
| AC 3: lowering `type: array`, `minItems: 1`, `items.anyOf` with 9 flattened arms, arm descriptions, `rule` required with no null arm, simplified atom shape | `schemas_union_postfix::content_policy_list_lowers_to_an_array_of_the_flattened_arms` |
| AC 2: `U(required)` requires, no null arm; missing/null fail | `required_union_reference_requires_the_property_without_a_null_arm` |
| AC 2: `U(max(10))` and other value constraints, with and without `[]`, rejected by keyword with provenance | `value_constraints_on_a_union_reference_are_rejected_by_name` |
| AC 2/7: array arm rejected, including behind a cross-file import and a nested postfixed union; usable without `[]` | `an_array_arm_rejects_the_array_suffix_even_behind_an_import` |
| Bare `U` unchanged | `union_reference_without_a_postfix_still_inlines_the_union` |
| Descriptions: use-site on union and array; arm descriptions kept | `a_described_union_reference_keeps_the_arm_descriptions` |
| Inherited `required`/`generated` hoist; arm min stays `minLength`; item/property parity | `inherited_markers_hoist_to_the_property_and_arms_lower_as_in_a_property_union` |
| `eager` on union/array never makes a lazy file arm eager; `file(eager)` keeps its check | `eager_on_the_union_does_not_make_lazy_file_arms_eager` |
| Bare, `@this`, and cross-file forms agree | `bare_this_and_cross_file_references_lower_identically` |
| Forward references | `a_forward_reference_resolves_regardless_of_declaration_order` |
| Multi-hop scope, import edges, item validation paths | `a_multi_hop_union_keeps_its_defining_scope` |
| Nested union flattening vs. nested postfixed arm | `a_bare_nested_union_flattens_and_a_postfixed_one_stays_one_arm` |
| Object arm containing an array | `an_object_arm_may_contain_an_array` |
| Independent items choose different arms; `[null]` and scalar fail; null property passes | `independent_items_select_different_arms_and_null_items_fail` |
| `min`/`max`/`unique` on the container, boundary values, arm constraints stay on items | `container_constraints_apply_to_the_array_and_value_constraints_stay_on_arms` |
| Contradictory bounds give `ConstraintConflict` with origins | `contradictory_container_bounds_fail_with_their_origins` |
| Defaults: container parity, `ScalarDefault` with provenance, property-union hoisting without `[]` | `container_defaults_match_plain_arrays_and_arm_defaults_are_rejected` |
| Cross-file error provenance (inline site, file definition) | `a_cross_file_rejection_names_both_sources` |
| Cycles and the depth limit through union postfixes | `cycles_and_depth_limits_still_apply_through_union_postfixes` |
| YAML schema-surface matrix (5 columns × every row, control first) | `schemas_union_schema_matrix::{imported_kind_cells, imported_types_cells, imported_policy_definition_cells, document_schema_cells, document_content_policy_declaration_cells}` |
| Error rendering | `errors::tests::union_reference_names_the_rejection_the_reference_and_the_definition` (lib unit) |
| DMLS projects the same error at the reference, with related definition (file and inline) | `dmls named_type_references::an_unsupported_union_postfix_is_reported_at_the_reference_with_its_definition` |
| DMLS: accepted `U[]` / `U(required)` gives no schema diagnostics | `named_type_references::an_array_of_a_union_reference_prepares_without_schema_diagnostics` |
| Claudine reads presence/timing through union arms | `claudine composition::schema::tests::presence_and_timing_read_through_a_resolved_union_reference` |
| The corpus guard treats `UnionReference` as a named-type failure | `shipped_schema_corpus` (helper `is_named_type_error` extended) |

**Input Robustness Matrix (schema surfaces) — implemented outcomes.** Every
cell also asserts that neither file changed.

| Cell | `kind` | `types` | `types.policy` | `$schema` | `$schema.content_policy` |
|---|---|---|---|---|---|
| Control | valid | valid | valid | valid | valid |
| Absent | prepare fails | prepare fails | `UnknownNamedType policy` | valid (no schema) | valid (no declaration) |
| Null (`null` and empty value) | prepare fails | prepare fails | prepare fails | prepare fails (both) | prepare fails (both) |
| Wrong whole type | prepare fails | prepare fails | prepare fails | prepare fails | prepare fails |
| One wrong element | prepare fails | prepare fails | prepare fails | prepare fails | prepare fails |
| Every element wrong | prepare fails | prepare fails | prepare fails | prepare fails | prepare fails |
| Empty | `''` fails | `{}` gives `UnknownNamedType` | `[]` fails; `{}` gives invalid data under `/content_policy` | `{}` valid; `[]` fails | `''`/`[]` fail; `{}` gives invalid data |
| Duplicate key | prepare fails | prepare fails | prepare fails | load fails | load fails |
| Trailing / invalid | extra document and invalid YAML fail | both fail | both fail | invalid frontmatter fails loading (body is a normal Markdown link) | unterminated quote fails loading |

A deliberately wrong expectation was injected once to confirm the harness
fails, then reverted. The smell grep over this phase's production diff
(`serde(default)`, `unwrap_or_default`, `.ok()`, `filter_map(.. as_str())`)
found nothing.

### Gates and evidence (macOS unless noted)

| Check | Result |
|---|---|
| Byte identity (AC 6 checkpoint): reran `baseline/capture_lowering.rs` on this tree | 347/347 manifest entries identical to `baseline/lowered-corpus.blake3` (`diff` clean). Scratch module removed afterwards. |
| `darkmatter/` `just test` | 9331 passed, 11 skipped |
| `darkmatter/` `just lint` | exit 0 (after boxing the error fields and one clippy pattern fix) |
| `claudine/` `just test` | 8684 passed, 9 skipped (run before and after the boxing change) |
| `claudine/` `just lint` | exit 0. The only warning is the known macOS `__eh_frame` linker note. |
| `just cross-check darkmatter --os windows union` | 165 passed |
| `just cross-check darkmatter --os linux union` | 165 passed |
| `just cross-check dmls --os windows named_type_references` | 13 passed |

- WSL2 was not run. The new tests are tempdir-based and compare paths only
  against `canonicalize_simplified` roots, which native Windows exercised.
- All new tests are L1 with no process spawns, terminals, or browsers. The
  only repository read is the per-test
  `include_str!("../../../../content-policy/schemas/content-policy.yaml")`.
  No CI cells, dependencies, or features were added.
- Nothing was skipped. No pre-existing failure was seen.
- `cargo fmt` was not run. Nothing was staged or committed.
- **Skills:** the `claudine` skill needs no update; Claudine's change is a
  local reading of union arms in `classify.rs`. The `darkmatter` skill is
  scheduled for Phase 4.
- **State:** Phase 2 complete. The core API and file ownership are stable for
  the parallel Phase 3 consumers.

## Phase 3

### How the phase was run

- New L1 modules were registered up front, before any parallel work, so no
  two agents edited a `main.rs`:
  - lib: `schemas_union_{report,coerce,rewrite,clean,completion,typed_consumers}`;
  - DMLS: `union_item_editor`.
  - `test_layout` passed with the placeholders.
- Wave 4 (four walkers) and the two library tasks of Wave 5 ran in parallel,
  each owning disjoint files. DMLS ("Adapt Editor Surfaces") ran after them,
  because it consumes the selection and completion contracts they published.
  - **Departure:** the plan orders Wave 5 after the Wave 4 checkpoint. The
    Wave 5 library tasks overlapped it instead, since their files were
    disjoint and nothing they depend on was being changed.
- Every new branch was proved load-bearing: each agent temporarily broke its
  fix, saw the named tests fail, then restored the source. Evidence is per
  task below.

### Wave 4 — Library walkers

**Report union items (`validate.rs`; `discriminant.rs` unchanged).**

- `resolve_problem_description(root, problem, instance)` now takes the
  instance. The new `schema_nodes_at` walks the schema along the problem
  path:
  - At each step it looks through `anyOf`. A nullable wrapper becomes its
    single arm. A genuine union (property, `items.anyOf`, or a nested
    postfixed arm) becomes the arm the literal tag selects, or every non-null
    arm when no tag selects one.
  - Arms without the next segment drop out.
  - When several arms reach the path with different descriptions, it returns
    `None` rather than guessing.
- `narrow_discriminated_union_problems` replaces
  `narrow_property_union_problems`, which handled only top-level properties.
  - `collect_discriminated_arms` finds every tagged union location: through
    nullable wrappers, object fields, array indices, and selected arms.
  - Narrowing revalidates against a stand-in schema that holds only the
    selected arm at that location, so problem paths keep their normal
    `/k/0/...` form.
  - A location is narrowed **only if it already has a problem**, so anyOf
    acceptance is unchanged: a value another arm accepts stays valid.
- `select_literal_discriminant_arm` already took any arm slice, so its
  callers (`frontmatter_shape.rs`, DMLS) are untouched.
- Also touched: `schemas/mod.rs`, where the call sites changed.

**Coerce union items (`coerce.rs`).**

- New `CoercionTarget::ItemUnion(Vec<Value>)` for `items: {anyOf}`. The array
  branch is now `coercion_target(items).or_else(|| item_union_target(items))`:
  - `boolish[]` and `numberlike[]` keep their specialized targets first.
  - Each other item goes through the existing `coerce_property_union`
    candidate rule: exactly one validating candidate, judged from the
    original item.
- **Behavior note:** `coerce_property_union` now returns `None` when the one
  accepted candidate equals the original value. It used to report
  `changed: true` with an identical value. Values are unchanged; only the
  flag differs, and it now agrees with `coerce_value`.
- **Interaction:** validation coerces a working copy before checking it. So
  a union item such as `body: 1` against a `string` arm now validates, as a
  property union already did.

**Rewrite union items (`rewrite.rs`).**

- Scalar `file(eager)` arms in `U[]` items were already rewritten. Two cases
  were not, and both are fixed:
  1. **Object arm with an eager file field.** Only arms whose top level was
     `format: darkmatter-file` counted, so a nested `path: file(eager)` was
     never rewritten. This applied to items and properties alike.
  2. **Optional union property, a pre-existing bug.** The eager arm lowers as
     `{anyOf: [{const: ""}, {format: darkmatter-file}]}`, which was not
     recognized, so values like `one: asset` were never rewritten. Items have
     no such wrapper, so items and properties disagreed.
- **New rule for both:** an arm qualifies when it contains an eager-file
  marker anywhere (`contains_eager_file_fragment`) and its validator accepts
  the value.
  - With exactly one qualifying arm, it recurses through `rewrite_property`,
    so an object arm rewrites only its own eager fields.
  - With zero or several, the value is left alone.
  - The captured request context is used, as before.
- **Behavior change to note for review:** existing optional
  `[file(eager), …]` property unions are now normalized, where before they
  were silently left as written.

**Preserve cleanup safety (`clean.rs`, comments only).**

- The existing code was already safe. `schema_requires_string` and
  `nearest_declared_key` stop at an `items.anyOf` wrapper. `arm_index` only
  ever refers to a root union. Cleanup has no key-deletion repair; its only
  automatic repair quotes a bool or number scalar.
- The comments now state that this safety is deliberate.
- **AC 4 departure (Phase 1 finding, gate kept):**
  - AC 4 asks for the repairable non-union sibling to be repaired in the same
    document as the invalid union items. Cleanup auto-applies its proven
    quoting repair only when the raw query finds exactly one problem
    (`analyze_frontmatter`, pinned by
    `schema_quoting_safety::multiple_problems_block_auto_apply`).
  - A document with an invalid union item and a repairable sibling has two or
    more raw problems. So the sibling is reported but not auto-applied
    (`repairable_sibling_beside_invalid_union_item_is_reported_not_applied`).
  - The sibling repair is proven instead in a document whose union lists are
    present and valid (`repairable_sibling_beside_valid_union_items_is_quoted`).
  - The invalid items are proven unrepaired both together and each alone,
    with the single-problem gate open.
  - **Limit:** the extra-key tests pin the outcome but cannot fail under a
    walker break, because cleanup has no deletion path to break.
    `mistyped_scalar_inside_union_item_is_not_quoted` is the walker-sensitive
    test.

### Wave 5 — Typed consumers

**Complete item alternatives (`completion.rs`, `simplified/query.rs`).**

- `flattened_alternatives(def)` lists every leaf alternative with its arm
  index and whether it sits in an array. It is now **public** and re-exported
  from `schemas`, with `Alternative`.
  - `suggestions_for_def` takes `is_array` from it, which fixes the Phase 2
    gap.
  - `suggestions_for_path` and `suggest_atom_in_def` look through union
    references.
- New public API in `completion`:
  - `for_list_item`, `for_property_value`, and
    `alternatives_for_def(def, AlternativeSite, value)`, returning
    `AlternativeCompletion { is_array, selected_arm, keys, values }`.
  - Alternatives are filtered by the JSON kind of the value.
  - They narrow only through `select_literal_discriminant_arm`.
  - They are passive.
- Added during orchestration: `completion::alternative_accepts(atom, value)`.
  - It judges a value structurally against one alternative, as a *present*
    value with no optional-null wrapper, so `null` matches only an
    alternative that accepts null.
  - It exists so DMLS does not keep its own acceptance logic (see the editor
    task below).

**Describe typed arrays.** Per-site audit:

| Site | Outcome |
|---|---|
| `about.rs` shape catalog | Added the descriptors "Array of a named union" (`step[](min(1); unique) -> …`) and "Marked union reference" (`choice(required) -> …`). Neither uses anonymous `(A \| B)[]` syntax. `md schema about` renders them through its existing list, with no CLI change. `md schema about` takes no schema argument, so AC 5's "renders the array-of-union type" is met by the catalog plus a test of the resolved model against the fixture. |
| `phase.rs` `project_atom` | **Bug fixed.** Union arms were never projected: `choice[]` with a `string(required)` arm was required at Launch. Arm markers now hoist once to the container, object arms project as shapes, and `eager` on arms (including `file(eager)`) is untouched. |
| `phase.rs` `atom_has_property_eager` | `U(eager)` counts an eager arm, as a property union does. For `U[]`, only `[](eager)` sets property timing, matching the documented `file(eager)[]` rule. |
| `frontmatter_shape.rs` | **Bug fixed.** `U(required)`/`U[]` were opaque. The new `alternatives_of` flattens arms for classification and descent. `arm_discriminant_json`'s `_ =>` is now explicit. `U[]` items merge their object arms; frontmatter-shape key paths carry no index, so there is no per-item tag narrowing, the same as for `{…}[]`. |
| `triggers/matcher.rs` | `type_conforms` already handled `U[]`. **Gap fixed:** `is_required` now reads through arms, so an inherited required arm makes a gate. |
| `detect.rs` | Unchanged. Detection builds only primitive atoms. A doc note on `schema_to_yaml` was added. |
| `compose/context/catalog.rs` | The `Union` branch is split out, with the reason: the base schema is parsed passively. |
| `file_match.rs`, `compose/expression/catalog/parser.rs` | Unchanged, with evidence: a union is not `File`; the parser never resolves. |

**Adapt editor surfaces (DMLS `providers/frontmatter.rs`, `overlay/schema.rs`).**

- `array_item_def`:
  - The item of `U[]` becomes the union of its alternatives.
  - The container's markers, bounds, and description stay off the item.
- `descend`, `inline_object_shape`, `declares_constraint`, and
  `value_completions` read through union arms.
- Property hover:
  - The old `union`/`union[]` label is replaced by "array, each item one of:"
    (or "one of:").
  - One line follows per alternative, with its type, keys, and description.
- Item hover (`union_item`):
  - It names an alternative only when exactly one accepts the item.
  - A missing, invalid, or multiply-matching item lists every alternative.
  - A field inside an object item uses the matching alternative's
    definition, or falls back to tag-select-or-merge.
- Item completion:
  - It calls `completion::alternatives_for_def(…, AlternativeSite::Item, value)`.
  - It combines alternatives unless a literal tag selects one.
  - Keys already present are excluded.
  - It covers block-style `- ` items only.
- `overlay/schema.rs`: `meta_schema_kinds` and `merged_inline_shape` see
  `U(required)` object arms.
- **Orchestrator correction:** the DMLS agent first added a local copy of
  the library's flattening and its own acceptance check. That is the
  editor-specific union interpretation the plan forbids. Both copies were
  deleted:
  - DMLS now imports `schemas::flattened_alternatives` and
    `completion::alternative_accepts`.
  - The library acceptance check differs from the deleted copy in one way: it
    judges a *present* value, so a `null` item is "invalid" rather than
    "matches every alternative". Hover lists the alternatives in both cases.
- DMLS still shows no `problem.description` in diagnostics (pre-existing).
  Item line/column positioning remains out of scope.

### Docs touched by owners (ahead of Phase 4)

These are user-facing statements of the behavior each task changed. Phase 4
still owns the full rewrite.

- `darkmatter/docs/topics/schemas/definition.md`:
  - discriminated-union narrowing at items;
  - coercion of union items;
  - "Completing inside a union value".
- `darkmatter/docs/inline/schema-validation.md`: union items in eager
  rewriting and coercion.
- `darkmatter/dmls/docs/hover.md` and `darkmatter/dmls/docs/autocomplete.md`:
  union items.
- **Still stale, for Phase 4:**
  - `definition.md` ~888 still says union postfixes are rejected;
  - `dmls-schema-support.md`;
  - the darkmatter skill;
  - the shipped content-policy schema comment.

### Findings (not fixed here)

- **Content Policy suggestions.** The shipped schema's `suggest(...)` list
  sits on the `^Evergreen$` arm. The suggestion lint checks each candidate
  against that arm only, so only `Evergreen` is ever offered. This was true
  before and after this phase. Either the lint should check candidates
  against the whole union, or the schema should move each candidate onto its
  own arm. The schema change belongs to `2026-10-17-content-policy-base-schema`.
- **Darkmatter and Claudine disagree on list `eager`.** Claudine's
  `classify.rs::any_atom_constraint` treats item-level `eager`
  (`file(eager)[]`, and now `U[]` arms) as property timing. Darkmatter phase
  treats only `[](eager)` as property timing. This predates this feature for
  plain arrays.
- **Possible trigger gap.** The matcher's gate reads `constraints`, so
  `x[](required)` on a trigger may act as a guard rather than a gate. Not
  verified.
- **Inconsistent reports for an untagged object item.** A missing field
  (`{action: archive}`) is reported at the item with the generic anyOf
  message, while `action: delete` is narrowed. Closest-arm reporting is still
  root-only. A property union behaves the same way.
- **Unclear path wording.** `schema-validation.md` describes stored eager
  values as an "absolute native path", while `rewrite.rs` writes
  document-relative portable paths. This predates this feature.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| AC 4 validate: description/node lookup at `/content_policy/1/rule`, item/property parity | `schemas_union_report::{content_policy_rule_lookup_reads_the_union_inside_the_object_arm, an_item_union_reports_like_the_same_union_used_as_a_property}`; lib unit `resolve_missing_field_looks_through_item_alternatives`, `resolve_item_field_uses_the_discriminated_arm_and_refuses_to_guess` |
| AC 4 discriminant: a literal-tagged item narrows; AC 7 missing, unknown, duplicate, conflicting tags, overlapping arms, independent items | `schemas_union_report::{a_literal_tag_narrows_each_item_to_its_own_arm, missing_unknown_duplicate_and_conflicting_tags_keep_ordinary_union_reporting, overlapping_item_arms_accept_either_shape_and_report_a_field_once, an_item_another_arm_accepts_stays_valid_despite_its_tag}` |
| AC 4 coerce: a boolish/numberlike arm in an item; AC 7 zero/multiple candidates unchanged, nested object, mixed items, parity; composition writes back | `schemas_union_coerce::{boolish_and_numberlike_arms_coerce_inside_a_union_item, composition_writes_back_a_coerced_union_item, an_item_no_arm_accepts_is_left_unchanged_beside_one_that_coerces, an_item_several_arms_accept_is_left_unchanged_beside_one_that_coerces, object_arm_candidates_coerce_their_nested_fields, independent_items_select_different_arms, a_union_item_coerces_exactly_as_the_same_union_property}`; lib unit `boolish_items_stay_boolean_and_other_item_unions_are_item_unions` |
| AC 4 rewrite: an unambiguous eager file arm in an item; lazy preserved under `eager`; ambiguous unchanged; nested object file field; idempotence; validation read-only | `schemas_union_rewrite::{eager_file_arm_in_union_item_is_rewritten_like_the_property, required_union_array_without_null_wrapper_is_rewritten, lazy_file_arm_is_preserved_under_union_and_container_eager, nested_file_field_of_selected_object_arm_is_rewritten, ambiguous_item_and_property_unions_are_left_unchanged, overlapping_object_arms_still_rewrite_when_only_one_validates}` |
| AC 4 clean: untagged and tagged extra-key items unchanged and reported; sibling repaired (departure above) | `schemas_union_clean::{union_items_with_extra_keys_are_reported_and_left_unchanged, union_item_with_extra_key_alone_keeps_its_key, mistyped_scalar_inside_union_item_is_not_quoted, repairable_sibling_beside_valid_union_items_is_quoted, repairable_sibling_beside_invalid_union_item_is_reported_not_applied}` |
| AC 5 completion: object-arm keys and scalar-arm values for a list item; tag narrowing; overlapping; neighboring indices; `is_array`; parity; no effects | `schemas_union_completion::{content_policy_item_offers_object_keys_and_scalar_values, missing_invalid_and_partial_items_offer_their_applicable_alternatives, a_literal_tag_narrows_each_item_independently, unknown_and_duplicate_tags_keep_the_combined_choices, overlapping_arms_combine_without_choosing, scalar_arms_offer_enum_suggest_and_literal_values, is_array_is_reported_for_items_suggestions_and_shell_completion, a_union_property_completes_like_a_union_item, completion_reads_and_writes_no_files, alternative_accepts_judges_each_item_against_every_flattened_alternative}` |
| AC 5 `md schema about` and the descriptor catalog; typed consumers | `schemas_union_typed_consumers::{about_model_exposes_every_content_policy_alternative_and_the_container, about_catalog_describes_named_union_references_without_anonymous_syntax, about_catalog_union_examples_resolve_to_what_they_describe}`; lib unit `descriptor_shape_examples_parse` (extended) |
| frontmatter_shape, phase, trigger matcher, detect, ctx catalog | `schemas_union_typed_consumers::{frontmatter_shape_narrows_a_marked_union_by_its_literal_tag, frontmatter_shape_descends_into_the_item_alternatives_of_an_array_of_a_union, frontmatter_shape_finds_an_expression_alternative_of_a_marked_union, inherited_required_on_a_list_of_a_union_waits_for_completion_unless_the_list_is_eager, an_eager_alternative_times_a_marked_union_but_only_the_items_of_a_list, nested_object_fields_in_union_items_keep_their_own_presence_and_timing, eager_on_a_list_of_file_alternatives_is_timing_and_lazy_arms_stay_lazy, a_list_of_a_union_matches_when_every_item_matches_some_alternative, an_inherited_required_alternative_makes_the_condition_a_gate, detection_never_produces_a_union_type_and_its_yaml_parses_back, the_ctx_catalog_projects_no_union_type}` |
| AC 5/8 DMLS: parser/overlay, hover (unique match vs. alternatives), completion (combined vs. tagged), schema errors | `dmls union_item_editor::{the_overlay_prepares_an_array_of_a_union_and_hover_lists_its_alternatives, hover_on_an_item_names_the_alternative_only_for_a_unique_match, completion_in_an_item_combines_alternatives_unless_a_literal_tag_selects_one, a_problem_inside_an_item_is_published_as_the_library_reports_it}`; DMLS unit `a_required_union_reference_activates_authoring_in_its_object_arm`; the unsupported-postfix error is covered by the existing `named_type_references::an_unsupported_union_postfix_is_reported_at_the_reference_with_its_definition` |

**Load-bearing evidence (each break was temporary and has been restored):**

- **validate:** with `validate.rs` reverted, 6 of 6 report tests failed.
- **coerce:** with the array branch reverted, 7 of 7 failed. A first-match
  variant without the exactly-one check failed the zero-match,
  several-match, and parity tests.
- **rewrite:**
  - item `anyOf` disabled: 4 tests failed;
  - first arm wins: the ambiguous and overlapping tests failed;
  - lazy treated as eager: the lazy test and the AC 4 test failed;
  - old direct-arm rule: 3 tests failed.
- **clean:**
  - `items.anyOf` read as its first arm: the mistyped-scalar test failed;
  - refusing every union: both sibling tests failed.
- **completion:** five separate breaks failed 1 to 8 tests each. Removing the
  present-value rule from `alternative_accepts` failed
  `alternative_accepts_judges_…` ("items get no null wrapper").
- **typed consumers:**
  - original phase, frontmatter_shape, and matcher restored: 6 of 13 tests
    failed;
  - eager `||`→`&&`: 1 test failed;
  - descriptor renamed: 2 tests failed.
- **DMLS:** six separate breaks, each of which failed its targeted test.

### Gates and evidence (macOS unless noted)

| Check | Result |
|---|---|
| `darkmatter/` `just test` | 9387 passed, 11 skipped (33.3 s) |
| `darkmatter/` `just lint` | exit 0; zed-dmls wasm32-wasip2 check passed |
| `claudine/` `just test` | 8684 passed, 9 skipped (71.4 s) |
| `claudine/` `just lint` | exit 0. The only warning is the known macOS `__eh_frame` linker note. |
| `cargo nextest run -p dmls --test l1 --lib` | 828 passed, 6 skipped |
| `just cross-check darkmatter --os windows union` | 214 passed |
| `just cross-check darkmatter --os linux union` | 214 passed |
| `just cross-check dmls --os windows union_item_editor` | 4 passed |

- WSL2 was not run. The new tests are tempdir-based and compare paths
  through `canonicalize_simplified`, which native Windows exercised.
- All new tests are L1. They use no process spawn, terminal, or browser. The
  only repository read is the per-test `include_str!` of the shipped
  content-policy schema. No CI cells, dependencies, or features were added.
- Nothing was skipped. No pre-existing failure was seen.
- `cargo fmt` was not run. One agent ran `rustfmt` on its own new test file
  only. Nothing was staged or committed.
- **Skills:** the `claudine` skill needs no update, because Claudine code was
  not changed this phase. The `darkmatter` skill update is Phase 4's.
- **State:** Phase 3 is complete. Phase 4 may proceed.

## Phase 4

### How the phase was run

- The two Wave 6 tasks ran in parallel with disjoint ownership: CLI
  pipeline tests (`darkmatter/cli/tests/`) and current docs plus the
  darkmatter skill.
- Both ran in parallel. Their files did not overlap, and each was reviewed
  before its task was checked off.

### Wave 6 — Verify CLI Pipeline

**New L1 tests (50).** They run the real `md` binary through
`CliProcessFixture`, with a private HOME, CWD, and PATH and no terminal,
browser, or provider:

- `cli/tests/l1/schema_union_pipeline.rs` (18 tests) covers AC 1, unchanged
  input, `about`, compose, and assignment.
- `cli/tests/l1/schema_union_matrix.rs` holds the cell table, the runner, and
  two guard tests.
- `cli/tests/l1/schema_union_matrix_cells.rs` has one `#[test]` per matrix
  cell (30).
- `cli/tests/l1/main.rs` registers the three modules and `#[path]`-includes
  `lib/tests/union_fixtures/mod.rs`.

**Report contract relied on.** `md schema validate --format json` emits one
NDJSON line per file:

- the line is `{file, problems:[{path, kind, message, description, …}], schema, valid}`;
- exit 0 means valid, 1 means problems, and 3 means a load failure, which also
  carries `"error":"frontmatter_parse"`.

**`assignment.rs` was not changed.** Its existing `TypeExpr::Union` branch
already keeps raw text for `U(required)`, as it does for a property union.

- `schema_validate_assignment_keeps_raw_text_for_every_union_reference_form`
  proves this for an inline union, a bare reference, and `ver(required)` with
  `version=1.10`.
- With that branch disabled, the test fails because the value becomes the
  float `1.1`.
- `U[]` assignments are not string-coerced, the same existing rule as
  `string[]`.

**CI metadata.** `darkmatter/lib/tests/union_fixtures/mod.rs` was added to the
CLI's `[package.metadata.ci.tests] source-inputs`, the same way the
entry-point parity matrix is declared.

- Each test that uses the fixture spells it with
  `include_str!`/`let _ = include_str!(…)` inside the `#[test]`.
- An edit to the fixture therefore schedules exactly those narrowed tests.
- No standing CI cell was added.

**Departures.**

- **Matrix cells are written out, not generated by `macro_rules!`.**
  - An `include_str!` inside a macro body is not inside a `#[test] fn`, so the
    CI test-input scanner would widen the cell to the whole binary.
  - A table plus two guard tests keeps the table and the tests in step
    instead: `matrix_covers_every_row_and_column` and
    `every_matrix_cell_has_its_own_test`.
- **The `about` test is in the new file**, not `schema_about.rs`, which is
  already 702 lines (over the `lint-files` 500-line cap).
- **`just lint-files` already fails on `main`.** Six existing CLI test files
  exceed the cap without an exemption. The recipe is not part of `just lint`,
  and none of the new files are flagged; each is under 500 lines.
- **The compose idempotence test re-composes the output.**
  - `md compose` never writes the source file, so "repeated normalization"
    is proven by composing the composed output again.
  - This follows the existing persist-and-recompose pattern. It expects
    identical bytes and an unchanged key set, so no default is inserted.

**Load-bearing evidence.** Each break was temporary and has been restored:

- disabling the `assignment.rs` union branch failed the assignment test;
- disabling item-union coercion in `coerce.rs` failed the compose test and
  both unchanged-input tests;
- skipping eager-file arms in `rewrite.rs` failed the rewrite test;
- using the unmodified shipped schema (`rule` optional) failed
  `rule_absent_is_reported_at_the_item`;
- a wrong expected path failed its cell;
- a renamed cell test or a duplicated acceptance call tripped the guards.

### Wave 6 — Update Current Docs

**`darkmatter/docs/topics/schemas/definition.md`**

- New "Named Union References" section. It covers:
  - the three reference forms;
  - an allowed-operations table, and a rejected-operations table that names
    the `SchemaUnionRejection` variant for each row;
  - item versus container scope;
  - presence and timing, including that `eager` never makes a lazy `file`
    arm eager;
  - defaults, scalar arm defaults rejected, and nothing inserted;
  - lowering, with the null wrapper only on the optional property;
  - the typed-model representation;
  - the Content Policy example, with the AC 1 pass/fail table.
- New "Choosing an Arm: Reporting versus Changing Data" section, with a
  Mermaid flow and a table per consumer.
- The stale "union postfixes are rejected / planned" sentence under
  cross-file imports was replaced.
- The Error Model now has a `UnionReference` row.
- The `md schema about` shape list now uses the real descriptor names.
- The limitations now say:
  - "no arrays of anonymous unions";
  - a problem inside a list item is located at its top-level key.

**`darkmatter/docs/topics/schemas/dmls-schema-support.md`**

- `UnionReference` diagnostics are ranged on the reference, with related
  information at the union's definition.
- New coverage of completion and hover inside union items, cross-linked to
  DMLS `hover.md` and `autocomplete.md`.
- Matrix row and footnotes updated.

**Skills**

- `.claude/skills/darkmatter/schema.md` has a new "Named union references"
  section. It covers:
  - the forms and rejections;
  - scope and defaults;
  - the representation and lowering;
  - the consumer rule (`flattened_alternatives`, `alternative_accepts`, the
    completion functions);
  - the safety rule per consumer, and known gaps.
- `.claude/skills/content-policy/SKILL.md` said Darkmatter rejects
  `policy[]`, which is no longer true. That claim was corrected. The skill
  keeps the still-true facts that the shipped schema does not type
  `content_policy` and leaves `rule` optional.

**Comment-only source change.** `darkmatter/lib/tests/l1/content_policy_editor_schema.rs`
no longer says the language cannot make `rule` required. It now says the
shipped schema leaves `rule` optional.

**Drift found and resolved**

- **`default([...])`.** `definition.md` said this produces a default array.
  The grammar reads `[a]` as the string `"[a]"`. The code wins, so the doc row
  was corrected.
- **`md schema about` shape names.** The doc showed outdated names. They now
  match `about.rs`.
- **READMEs.** None claimed that union references reject postfixes, so none
  changed.

**Drift left in place deliberately.** The comment in
`content-policy/schemas/content-policy.yaml` ("a union-typed reference cannot
take constraints today") is now stale.

- It was not edited, because the plan forbids changing the shipped schema
  and Phase 5 checks that it is unchanged.
- Whoever adopts `short_form(required)` in that file should update the
  comment in the same change.

**Not documented as implemented:**

- anonymous `(A | B)[]` syntax;
- recursion;
- the Content Policy baseline change;
- per-item line and column, which is listed only as a limitation.

The docs name no feature or fix spec. The tests that read these docs pass:
`schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes`,
`schema_roots::no_doc_or_skill_names_the_singular_schema_variable`, and the
`content_policy_editor_schema` tests.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| AC 1 through the CLI: one test per acceptance case | `schema_union_pipeline::{acceptance_mixed_scalar_and_object_list_validates, acceptance_missing_property_validates, acceptance_null_property_validates, acceptance_null_item_is_rejected_at_its_index, acceptance_scalar_in_place_of_the_list_is_rejected, acceptance_empty_list_fails_min_items, acceptance_item_matching_no_arm_is_rejected, acceptance_object_item_without_rule_is_rejected, acceptance_object_item_with_null_rule_is_rejected, acceptance_object_item_with_unknown_action_is_rejected}`; guard `acceptance_cases_each_have_an_expected_report_and_a_test` |
| Document-data matrix: every cell of the 3 columns × 9 rows, plus the control, with file bytes unchanged in each cell | `schema_union_matrix_cells::*` (30 cell tests); guards `schema_union_matrix::{matrix_covers_every_row_and_column, every_matrix_cell_has_its_own_test}` |
| Validation leaves the input unchanged (coercible items, eager-file items, invalid items) | `schema_union_pipeline::{validate_leaves_coercible_and_eager_file_union_items_unchanged, validate_leaves_a_document_with_invalid_union_items_unchanged}` |
| AC 5/8 `md schema about` renders the array-of-union shapes | `schema_union_pipeline::schema_about_renders_the_named_union_reference_shapes` |
| AC 8 composition: coercion rule (exactly one candidate), stage order (normalization runs before the body expression), eager rewrite and lazy preserved, idempotence, no default insertion | `schema_union_pipeline::{compose_coerces_an_unambiguous_union_item_and_keeps_ambiguous_items, compose_rewrites_an_eager_file_union_item_and_keeps_lazy_items, recomposing_normalized_union_items_is_idempotent_and_inserts_nothing}` |
| CLI assignment rules for union references | `schema_union_pipeline::schema_validate_assignment_keeps_raw_text_for_every_union_reference_form` |
| Docs describe the final behavior | `definition.md`, `dmls-schema-support.md`, darkmatter skill; doc-reading tests listed above |

Matrix outcomes worth noting:

- A missing `rule` or `action` in an object item is reported at the item
  (`/content_policy/1`) with the generic `anyOf` message.
- An explicit null or a wrong-typed field is reported at the field.
- This matches Phase 3's recorded behavior.
- Duplicate-key and invalid-YAML cells fail loading through the real loader
  (exit 3).

### Gates and evidence

| Check | Result |
|---|---|
| `darkmatter/` `just test` (macOS) | 9437 passed, 11 skipped (27.7 s) |
| `darkmatter/` `just lint` (macOS, run by the CLI task after its last change) | exit 0; clippy clean for all four packages; zed-dmls wasm32-wasip2 check passed |
| `claudine/` `just test` (macOS) | 8684 passed, 9 skipped (48.0 s) |
| `claudine/` `just lint` (macOS) | exit 0. The only warning is the known macOS `__eh_frame` linker note. |
| `just cross-check darkmatter-cli --os windows schema_union` | 50 passed |
| `just cross-check darkmatter-cli --os linux schema_union` | 50 passed |

- WSL2 was not run. The tests are tempdir-based, and native Windows exercised
  the path-sensitive rewrite assertions.
- All new tests are L1, and each takes well under 1 s.
- No `cargo fmt` was run, and nothing was staged or committed.
- No pre-existing test failure was seen. Nothing was skipped, apart from
  the `macro_rules!` departure explained above.
- **Skills:** the `claudine` skill needs no update, because no Claudine code
  or workflow changed this phase.
- **State:** Phase 4 is complete. Phase 5 may proceed.

## Phase 5

### How the phase was run

- Run sequentially on macOS (`Darwin 27.2.0`, arm64), from the working tree
  on `feat/osc-9-4-protocol` (HEAD `14cc1277f` plus the uncommitted Phase 4
  and Phase 5 changes).
- One read-only review subagent audited the feature's diff while the first
  `darkmatter/` `just test` ran. Its two confirmed defects were fixed here,
  test first. Its nits are listed below with what was done about each.

### Wave 7 — Compare Compatibility Corpus

- **Byte identity (AC 6).** `baseline/capture_lowering.rs` was rerun on the
  implemented tree as its module doc describes. The scratch module and its
  `mod` line were then removed, and `git status darkmatter/lib/tests/l1/`
  shows only Phase 4's comment edit.
  - The manifest has **347 entries, all identical** to
    `baseline/lowered-corpus.blake3` (`diff` clean).
  - No entry was added or dropped. The new-form fixtures exist only in
    tempdirs (`lib/tests/union_fixtures/mod.rs`), so nothing on disk had to
    be excluded, and there was no difference to investigate.
  - Inline resolver fixtures stay guarded by the `schemas_convert_snapshots`
    insta snapshots and
    `schemas_conjunctive_lowering::schemas_without_repeats_keep_their_lowered_output`.
    Both are in the passing `just test` run.
- **Shipped Content Policy schema is unchanged.**
  `git diff --stat b2cc72a00 -- content-policy/` is empty, and
  `content_policy_editor_schema::*` and `shipped_schema_corpus::*` pass.
- **Downstream checks from the consumer inventory.** `claudine` and
  `claudine-gen` are compile-level consumers.
  - `claudine/` `just test`: 8684 passed, 9 skipped.
  - `claudine/` `just lint`: exit 0. The only warning is the known macOS
    `__eh_frame` linker note.
  - `content-policy` and `messenger` do not depend on the schema AST (Phase 1
    inventory) and were not rerun.
- **No dependency or CI-scheduling change came from this feature.**
  - Since the baseline, the feature changed `Cargo.toml` only in
    `[package.metadata.ci.tests] source-inputs`: the CLI in Phase 4 and DMLS
    in this phase.
  - The other manifest, `.github`, and `docs/dependencies.md` diffs on the
    branch since `b2cc72a00` arrived through the `main` merge (`069f8d2e8`),
    not from this feature.

### Wave 7 — Final review and fixes

The review read commits `44ee1aa08`, `ed5a53a98`, `3fc863692`, `43429bccc`,
`153829689` and the uncommitted Phase 4 diff.

**Defect 1, fixed: a union rejection inside a referenced schema lost its
`ReferencedSchema` wrapper.**

- `resolve.rs::load_guarded_in_context` wrapped `Grammar | Convert |
  UnknownNamedType | ConstraintConflict | Aggregate`, but not the new
  `UnionReference`.
- Before this feature, the same misuse raised `Convert`, which was wrapped,
  so this was a regression. Users saw:
  - the CLI message without the "referenced schema … is invalid" prefix;
  - DMLS without the "schema definition is authored in this file" location.
- Regression test, written first and seen to fail with "expected
  ReferencedSchema, got UnionReference { … }":
  `schemas_union_postfix::a_rejection_inside_a_referenced_schema_keeps_the_referenced_wrapper`.
- Fix: added `UnionReference` to the wrapped set. The `## Errors` docs on
  `expand_named_types` also omitted `UnionReference`, and now list it.
- DMLS follow-on:
  - The DMLS test
    `named_type_references::an_unsupported_union_postfix_inside_a_referenced_schema_points_into_that_file`
    then showed that the use site inside the referenced file was missing
    from the related information.
  - `diagnostics/named_types.rs` now adds "`U` is referenced here" when the
    site is in another file, as it already does for `UnknownNamedType`.
  - The test asserts all three related locations: the file, the use site,
    and the definition.
  - The `dmls-schema-support.md` table row was updated to match.

**Defect 2, fixed: DMLS read the shared union fixture without declaring it to
CI.**

- `dmls/tests/l1/union_item_editor.rs` includes
  `darkmatter/lib/tests/union_fixtures/mod.rs` by `#[path]`. A fixture edit
  would therefore not have run those four tests.
- `dmls/Cargo.toml` `source-inputs` now lists the file, and each of the four
  tests spells it with `let _ = include_str!(…)`. This is the same pattern
  the CLI uses.
- `python3 scripts/ci/affected_scope.py darkmatter/lib/tests/union_fixtures/mod.rs`
  exits 0 (the declaration validates) and selects `dmls`.

**Acceptance gap closed: authored-reference round trips.**

- The grammar proptest never generates named references. So no test
  serialized and reparsed an authored `U[]` reference, although the plan's
  audit names round trips.
- Added `schemas_union_postfix::authored_union_references_round_trip_and_anonymous_union_arrays_stay_rejected`.
  - Each authored form goes through parse, serialize, reparse, and serialize
    again, with equal atoms and stable text. The forms are
    `policy[](min(1))@./content-policy.yaml`, `short_form(required)`,
    `choice[]`, `choice[](min(1); max(3))`, `choice(eager)@this`, and
    `choice[](min(1))@./types.yaml`.
  - The resolved `TypeExpr::Union` display form must not reparse.
  - The anonymous `(string | number)[]` and `[string, number][]` must be
    rejected by both the grammar and resolution. This also covers the
    review's nit that nothing pinned the anonymous form as rejected.

**Review nits left as they are.** None changes behavior, and each is
recorded for the review cycle:

- `dmls/src/overlay/schema.rs::alternatives()` and the private
  `frontmatter_shape::alternatives_of` repeat the public
  `flattened_alternatives`. They are structural walks, not a second
  representation or acceptance logic.
- `PropertyDef::alternatives()` has only a test caller.
- `dmls/src/providers/frontmatter.rs::union_item_alternatives` has an
  `ItemMatch::Unique` arm that its callers never reach.
- The `SchemaUnionRejection::{ArrayArm, ScalarDefault}.arm` docs say "among
  the flattened arms", but the value is the top-level position. They differ
  only for a nested postfixed union arm, which today's grammar cannot
  produce under these rejections.

**Review categories reported clean** (subagent, with the code checked):

- one array-of-union encoding (`TypeExpr::Union` + `is_array` → `items.anyOf`);
- no constraint redistribution: value constraints are rejected rather than
  moved, and arm bounds stay on arms;
- report, hover, and completion paths are read-only;
- no item is dropped by coerce or rewrite;
- no new parser effects, and the grammar is untouched;
- docs match the code and name no spec;
- every new test is declared, and none carries a tier marker.

### Acceptance criteria → evidence

| AC | Evidence (all L1, passing on macOS in the final `just test`) |
|---|---|
| 1 Content Policy shapes | lib `schemas_union_postfix::{content_policy_list_validates_the_acceptance_cases, content_policy_list_lowers_to_an_array_of_the_flattened_arms}`; fixture controls `schemas_union_fixture_controls::*`; CLI `schema_union_pipeline::acceptance_*` (10 cases plus the guard `acceptance_cases_each_have_an_expected_report_and_a_test`) |
| 2 required / rejected operations | `schemas_union_postfix::{required_union_reference_requires_the_property_without_a_null_arm, value_constraints_on_a_union_reference_are_rejected_by_name, an_array_arm_rejects_the_array_suffix_even_behind_an_import, a_cross_file_rejection_names_both_sources, a_rejection_inside_a_referenced_schema_keeps_the_referenced_wrapper}`; `schemas_named_type_merge::already_array_suffix_remains_rejected` (the former union rows became acceptance tests in Phase 2) |
| 3 lowering | `schemas_union_postfix::content_policy_list_lowers_to_an_array_of_the_flattened_arms` (`type: array`, `minItems: 1`, one `items.anyOf` entry per flattened arm) |
| 4 walkers | validate/discriminant: `schemas_union_report::{content_policy_rule_lookup_reads_the_union_inside_the_object_arm, a_literal_tag_narrows_each_item_to_its_own_arm}`; clean: `schemas_union_clean::{union_items_with_extra_keys_are_reported_and_left_unchanged, repairable_sibling_beside_valid_union_items_is_quoted}`; rewrite: `schemas_union_rewrite::eager_file_arm_in_union_item_is_rewritten_like_the_property`; coerce: `schemas_union_coerce::boolish_and_numberlike_arms_coerce_inside_a_union_item`. Phase 3 recorded each as failing when its walker reads the wrapper blindly. |
| 5 typed tree | completion: `schemas_union_completion::content_policy_item_offers_object_keys_and_scalar_values`; DMLS hover: `union_item_editor::hover_on_an_item_names_the_alternative_only_for_a_unique_match`; `about`: `schemas_union_typed_consumers::about_model_exposes_every_content_policy_alternative_and_the_container` and CLI `schema_union_pipeline::schema_about_renders_the_named_union_reference_shapes` |
| 6 byte identity | 347/347 manifest entries identical (above); snapshot and lowering guards pass |
| 7 boundaries | `schemas_union_postfix` (three reference forms, forward and multi-hop, inherited markers, descriptions, container versus value constraints, bounds, contradictions, defaults, hidden array arm, nested array in an object arm, cycles and depth, round trips); `schemas_union_report` (overlap, invalid and conflicting tags, item/property parity); `schemas_union_coerce` / `schemas_union_rewrite` (ambiguous left unchanged, independent items, parity); `schemas_union_typed_consumers` (presence and timing); YAML schema-surface matrix `schemas_union_schema_matrix::*` (5 columns × 9 rows) |
| 8 CLI and DMLS | CLI `schema_union_pipeline::*` (validate leaves input unchanged, compose coercion and rewrite, idempotence, assignment) and the document-data matrix `schema_union_matrix_cells::*` (30 cells plus 2 guards); DMLS `union_item_editor::*` (parser/overlay, hover, completion, published problems), `named_type_references::{an_unsupported_union_postfix_is_reported_at_the_reference_with_its_definition, an_unsupported_union_postfix_inside_a_referenced_schema_points_into_that_file, an_array_of_a_union_reference_prepares_without_schema_diagnostics}` |
| 9 gates | below |

- **Matrix coverage.** The YAML schema-surface matrix (Phase 2) walks all
  5 columns × 9 rows from the shipped-schema fixture, one edit per cell, with
  a control.
- The document-data matrix (Phase 4) walks all 3 columns × 9 rows through
  the real `md` binary, with row and column guards.
- Duplicate-key and trailing-content cells fail through the real loader.
- The smell grep over this phase's production diff (`serde(default)`,
  `unwrap_or_default`, `.ok()`, `filter_map(.. as_str())`) found nothing new.

### Gates and evidence

| Check | Host | Result |
|---|---|---|
| `darkmatter/` `just test` (final, after every Phase 5 edit) | macOS | 9440 passed, 11 skipped |
| `darkmatter/` `just lint` (final) | macOS | exit 0: clippy clean for `darkmatter`, `darkmatter-cli`, `dmls`, and `zed-dmls-cli`; zed-dmls wasm32-wasip2 check passed. Run with `KACHE_DISABLED=1`; see the environment finding. |
| `claudine/` `just test` | macOS | 8684 passed, 9 skipped |
| `claudine/` `just lint` | macOS | exit 0. The only warning is the known `__eh_frame` linker note. |
| `just cross-check dmls --os windows named_type_references` | native Windows | pass |
| `just cross-check darkmatter --os linux schemas_union_postfix` | Linux | pass |

- Observed passes are listed above. WSL2 was not run, and that is a
  portability review rather than evidence.
  - The Phase 5 tests use tempdir roots compared through
    `canonicalize_simplified` or `canonical_uri`, the same as the Windows-run
    Phase 2 to 4 tests.
  - Native Windows exercised the new DMLS URI assertions.
- No CI cells, features, L2/L3/browser tests, or dependencies were added.
  `cargo fmt` was not run, and nothing was staged or committed.

**Environment finding (host, not this feature).** A plain `just lint`
intermittently failed on this Mac.

- The failing run compiled `darkmatter` from another checkout,
  `/Volumes/coding/personal/rusty-biscuit/darkmatter/lib`: "Checking
  darkmatter v0.1.0 (/Volumes/coding/personal/…)".
- That checkout is on `main` with an uncommitted, non-compiling
  `darkmatter/lib/src/markdown/schemas/errors.rs` from other work. It failed
  with E0432/E0599 on `SchemaSourceId` and `UnknownNamedType`.
- The same happened once with a plain
  `cargo test --no-run -p claudine-cli` in this worktree.
- `cargo metadata` resolves no package outside this worktree, the target
  directory is local, and nothing in the worktree links to that path.
- Runs with `KACHE_DISABLED=1` compiled this worktree's sources and passed.
  This points at the `kache` rustc wrapper's shared store serving another
  checkout's build, but the root cause was not proven: one
  `KACHE_DISABLED=1 just lint` run still failed before an uncached rebuild
  refreshed the artifacts.
- Clearing an inherited `CDPATH` was tried and ruled out.
- Recommended follow-up: investigate kache's cross-worktree behavior and,
  once it is understood, record it in the `kache` skill.

**Skills.** The `claudine` skill needs no update, because no Claudine code or
workflow changed in this phase. The `darkmatter` skill's union section still
matches. The new related location is a DMLS diagnostic detail, which
`dmls-schema-support.md` documents.

**Known follow-ups carried forward (not defects of this feature):**

- the stale comment in `content-policy/schemas/content-policy.yaml`, left so
  the shipped schema stays unchanged;
- the Phase 2 and 3 findings (suggestion lint per arm, list-`eager`
  disagreement with Claudine, possible trigger gap, untagged-item reporting,
  path wording);
- `just lint-files` already failing on `main`.

**State: implementation complete, ready for review.** Lifecycle placement and
commits are left to the author.
