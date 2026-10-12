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
packages:
    - darkmatter
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
