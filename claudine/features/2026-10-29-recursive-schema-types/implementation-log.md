---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-29-recursive-schema-types/spec.md"
plan: "claudine/features/2026-10-29-recursive-schema-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_validator_probe.rs
    - darkmatter/lib/tests/l1/recursive_schema_golden.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/manifest.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/consumers.md
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/README.md
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-gen
human_review: false
message_to_agent: |-
    Phase 3 (Wave 3) landed the shared scope and merge isolation in
    darkmatter. Read inventory/contracts.md "As built (Wave 3)" (sections 6,
    8, 9, 10) first. New module schemas/scope.rs: SchemaScope::{new, named,
    resolve, chain} is the reference lookup Wave 4 walkers should use
    (resolve = one local hop, Ok(None) for no $ref or a non-local one; chain
    errors with UnresolvedReference on a missing target or a ref-only loop);
    with_root_definitions(sub, root) and wrap_arm_as_root_schema(arm, root)
    give a sub-schema its root's $defs. The rewrite walkers (rewrite.rs) and
    the caller file-mode classifier (compose/schema_validation.rs) already
    carry `root`, but none of them FOLLOWS a $ref yet: that is Wave 4's work
    (validate.rs walkers, discriminant.rs, clean.rs, coerce.rs/json_shape.rs,
    translate.rs, rewrite.rs). Conversion today still converts a numeric leaf
    to text only within json_shape's 4-expansion cutoff, so write recursive
    test fixtures with non-convertible bad values ({"not": "text"}); the
    Wave 3 tests already do. EffectiveSchema::definitions() is the document's
    table (its json keys equal the effective JSON's); look a Ref up across
    layers with EffectiveSchema::definition(&DefKey), which also searches
    trigger payload tables. Completion's alternative_accepts still lowers a
    typed atom without a table (Wave 5). Load-sensitive DMLS tests under
    full-suite load (each passes alone): registry_preparation::
    a_held_preparation_leaves_edits_cancellation_and_other_requests_running,
    searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back,
    startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration.
source_files_during_phase_2:
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/schema/classify.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/graph.rs
    - darkmatter/lib/src/markdown/schemas/resolve/recursive.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/scope.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
---

# Implementation Log for 2026-10-29-recursive-schema-types (6 phases)

## Phase 1

Phase 1 (Wave 1, "Starting Evidence") started 2026-10-23 on a clean tree at
`e5976e3f0` (schema-enhancements review iteration 1 committed as
`f6161e3f7`).

### Wave 1: Starting Evidence

**Prerequisites verified.**

- `2026-10-14-union-type-constraints` sits in `_completed/` with its
  implementation log, review rounds, and baseline; its behavior is pinned by
  the passing `schemas_union_*` L1 modules.
- `2026-10-28-schema-enhancements` records all six phases in its
  implementation log (Phase 6 closed in `d466b2d45`) and review iteration 1
  was committed as `f6161e3f7` / `e5976e3f0`; its spec frontmatter has
  `implemented: true` and its log `human_review: false`. Its reader matrix
  (`schema_enhancements_matrix::{envelope, raw_schema}`), conversion engine,
  call binder, catalog, and design-time call check are all in the tree.
- The earlier shared-table, YAML-loader, named-presence/local-name, and
  diagnostic-position contracts are present in code: `Arc`-shared namespace
  tables in `resolve.rs` (`Namespace.named_types`), bare local names
  (`TypeExpr::local`), `SchemaReferenceSite`/`SchemaConstraintOrigin` spans in
  `simplified/source.rs`, and `diagnostic_positions` L1 coverage.
- Old cutoffs confirmed in code: `json_shape.rs` `CYCLE_EXPANSIONS = 4`,
  `MAX_REFERENCE_DEPTH = 32`, `REFERENCE_BUDGET = 4096`;
  `call_binding/translate.rs` `MAX_REFERENCE_DEPTH = 32` → `TooDeep`;
  `resolve.rs` `MAX_IMPORT_DEPTH = 32`; signatures `16`. Recorded in
  `inventory/contracts.md` §1 with each one's fate.

**Validator probe.** `darkmatter/lib/tests/l1/recursive_schema_validator_probe.rs`
(6 tests, all pass, ~0.04 s each) pins pinned `jsonschema` 0.55 / Draft 2020-12:
self-reference and mutual recursion accept a depth-8 instance and reject a bad
value at depth 8 with that node's instance path; a required child missing at
depth 8 is reported at that item; a `$ref` sibling constraint rejects what the
target accepts while the target still applies; an escaped pointer key
(`a/b~c` → `a~1b~0c`) resolves; a missing local target fails to compile. Run
once as the prerequisite probe:
`cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(recursive_schema_validator_probe)'`.

**Golden compatibility corpus.** `darkmatter/lib/tests/l1/recursive_schema_golden.rs`
plus `darkmatter/lib/tests/fixtures/recursive_schema_golden/` (`manifest.yaml`,
`README.md`, `inputs/`, `golden/`), built by a subagent and reviewed here:

- 104 manifest entries: schema-file 23, pure-schema-document 24, inline-root
  39, root-union 5, trigger-payload 6, embedded-baseline 1, effective 6
  (baseline merge: `effective-baseline-{inline,schema-file,root-union}`;
  trigger merge: `effective-trigger-{inline-compose,sequence,with-document-schema}`
  through `DarkmatterSchemas::effective_for` with the frozen Claudine schemas
  as an embedded bundle, discovery in an empty tempdir).
- 112 input files: 92 byte-identical copies of the files at `e5976e3f0`
  (mirroring repository paths, so relative imports still resolve) and 20
  authored under `inputs/authored/` (bare local names / `@this`, union lists,
  effective cases), each with a `note:`; `union_fixtures` are Rust constants,
  and no shipped inline root used bare local names. 9 `imports_only` paths
  each name the entry that must read them.
- 104 goldens: 90 full pretty JSON, 14 compact-text digests.
- Skipped inputs are listed in the manifest with reasons: function catalogs,
  `kind: example` artifacts, empty placeholders, drafts whose definitions do
  not parse, `loop.yaml` (`typed:`), `_fleet_future.md` (unknown type
  `crate`), two messenger probes ("could not project expression spans"),
  `triggers/feature-review-incident.yaml` (invalid YAML at line 17), and
  string `$schema`s whose target is already an entry.
- No lowered output named an absolute path; the test fails if the fixture or
  scratch directory ever appears.
- Self-checks: every manifest input exists, every input file is an entry or a
  declared import, every golden belongs to an entry, ids are unique, each
  entry reads only frozen inputs, and five entries resolve twice to identical
  bytes. The fixture-directory literal sits inside each `#[test]`.
- 17 tests. Seven entries that lower to megabytes have a test each
  (`GROUP_TESTS`, guarded by `every_manifest_kind_and_group_has_a_test`). The
  slowest, `schema_file_claudine_sequence_types_matches_its_golden` (44 MB of
  output), runs in 1.9 s alone at host load ~12–15; about 1.1 s is the
  library's own lowering.
- Regeneration: `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1` (README); legitimate
  only for a change meant to alter acyclic lowering.

**Consumer inventory.** `claudine/features/2026-10-29-recursive-schema-types/inventory/consumers.md`
(subagent, read-only): 53 `TypeExpr` matches (31 exhaustive, 22 with wildcard
or catch-all arms that would silently absorb `Ref`, notably `union_arms` and
`admits_null` in `types.rs`) and 49 partial destructures; definition storage
and namespaces; lowering and resolve callers; merge boundaries; 14 synthetic
validator sites; JSON walkers (only `json_shape.rs` and
`schema_declares_property` follow `$ref` today); typed-tree walkers; cycle and
depth errors; downstream Claudine consumers; test targets per area; and file
ownership by wave. It found that `merge_baseline` and DMLS extension baselines
drop every baseline root key but `type`/`$schema`/`additionalProperties`, and
that `ImportCycle`'s doc and status text still describe a DAG; both are now in
the contract.

**Reference API contracts.** Written to
`claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md`:
`DefKey`/`DefNamespace`, `Definition`/`DefinitionTable` ownership, key
allocation and re-keying, graph edge classification and admission order, new
error variants, use-site semantics, raw merge-ingress rules and the duplicate
ruling, `SchemaScope` and synthetic-validator scope, phase memoization,
coinductive type comparison, and the robustness-matrix confirmation.

**Duplicate-member ruling.** Every raw reader feeding the new merge ingress
already rejects duplicate members (raw `.json` through
`coercion::content_error`, schema-enhancements R3; both YAML loaders), and an
in-memory `Value` cannot carry one. Rejection therefore tightens no accepted
schema; no author decision is needed.

### Design Decisions

1. **Golden inputs are frozen copies; the embedded baseline is live.**
   - Decision: the compatibility corpus copies each file input byte-for-byte
     under `darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/`
     (mirroring repository paths) and resolves the copies; only the compiled-in
     baseline and `@darkmatter:` partials are read live.
   - Alternatives considered: resolve the live shipped files listed in a
     manifest; store only a digest of the live files.
   - Why: the golden must move only when resolver code changes. Live inputs
     would fail on every unrelated schema edit (Claudine's lifecycle schemas
     are scheduled to change in `2026-10-30-claudine-recursive-schemas`), and
     CI's test-input index would schedule this test on each such edit, turning
     the golden into a rubber stamp.
   - Downsides: the corpus does not follow later edits to shipped schemas; the
     separate `shipped_schema_corpus` test keeps covering the live files.
2. **Large goldens are stored as a `biscuit-hash` BLAKE3 digest plus length.**
   - Decision: a schema whose compact serialization exceeds 128 KiB is stored
     as the `bytes:` + `blake3:` of that compact text; smaller ones as full
     pretty JSON. (The first cut digested pretty text over 256 KiB; pretty-
     printing and hashing the 44 MB `sequence-types.yaml` output cost about
     1 s of a debug test, so digests now cover the compact form, which keeps
     key order in the comparison.)
   - Alternatives considered: commit every golden in full; digest everything.
   - Why: byte-identity is what the criterion needs, and a digest proves it
     without committing multi-megabyte JSON; full text keeps small failures
     diffable.
   - Downsides: a mismatch on a digest golden shows no diff in the test
     output; the test writes the actual output to a temp file for inspection.
3. **`DefNamespace` replaces the shared `Root` sentinel; `SchemaSourceId`
   stays narrow.**
   - Decision: inline roots, each root-union arm, embedded documents, shipped
     partials, files, and raw `$defs` tables get distinct namespace identities
     (`contracts.md` §2); span sources are unchanged.
   - Alternatives considered: widen `SchemaSourceId`; key inline namespaces by
     document text.
   - Why: the spec requires distinct identities; widening the public span
     type would ripple through every diagnostic consumer without need.
   - Downsides: two identity types to keep in step.
4. **Generated keys are the type name with a numeric suffix on collision.**
   - Decision: `item`, then `item-2`, … in traversal order; never paths or
     hashes; consumers use the explicit identity/key map.
   - Alternatives considered: purely opaque `d0`, `d1`, …; content hashes.
   - Why: deterministic and path-free as the spec requires, and validator
     schema paths (`/$defs/item/...`) stay readable in diagnostics.
   - Downsides: a key looks like a name; the contract forbids parsing it.
5. **Only cyclic definitions enter the table.**
   - Decision: SCC members (including self-loops) become `Ref`s; everything
     else inlines exactly as today.
   - Alternatives considered: emit `$defs` for every named type.
   - Why: the spec requires byte-identical acyclic output, and an empty table
     emits nothing.
   - Downsides: output size for acyclic but heavily reused types is unchanged
     (the spec accepts this).
6. **Definition bodies are presence-free; markers are inherited per use.**
   - Decision: required/generated/eager/description/default/suggest move from
     a definition's top-level atom to each use site; value constraints stay in
     the body; differing use-site value constraints lower as `$ref` siblings.
   - Alternatives considered: one body per distinct constraint set.
   - Why: the spec's use-site semantics, and the probe shows siblings apply
     together with the target in the pinned validator.
   - Downsides: lowering must merge inherited and use-site constraints at
     every use, not once per definition.
7. **Fixture names in helpers are `concat!` constants.**
   - Decision: `load_manifest`/`check_group` join `MANIFEST_FILE`,
     `INPUTS_DIR`, and `GOLDEN_DIR` (built with `concat!`); the only path
     literal is the fixture directory inside each `#[test]`.
   - Alternatives considered: keep `fixtures.join("manifest.yaml")` in the
     helper; pass every path in from each test.
   - Why: CI's test-input scanner attributed the helper literals to the whole
     `darkmatter::l1` binary (checked with
     `affected_scope.test_input_references`); after the change every golden
     input maps to the single test that reads it.
   - Downsides: a reader must know why the names are spelled with `concat!`;
     the constant's doc comment says so.

### Requirement-to-test mapping (Phase 1)

| Phase 1 requirement | Evidence |
| --- | --- |
| Prerequisites verified, old cutoffs recorded | this log (Wave 1) and `inventory/contracts.md` §1 |
| Pinned validator: self recursion at depth 8, valid and invalid with path | `recursive_schema_validator_probe::{a_self_referencing_definition_validates_depth_eight, a_required_child_missing_at_depth_eight_is_reported_there}` |
| Mutual recursion | `…::mutually_recursive_definitions_validate_depth_eight` |
| `$ref` sibling constraint visibly rejects | `…::a_constraint_beside_a_reference_applies_with_its_target` |
| Escaped JSON Pointer key; missing target is not unconstrained | `…::{an_escaped_pointer_key_resolves, a_reference_to_a_missing_definition_does_not_compile}` |
| Byte-identical golden: shipped schema/trigger artifacts, pure `$schema` documents, embedded baseline, resolver fixtures, inline roots, root-union references, effective merges | `recursive_schema_golden::*_matches_their_goldens` / `*_matches_its_golden` (13 tests by kind/group) |
| Manifest is explicit, complete, and drift-proof | `recursive_schema_golden::{manifest_accounts_for_every_fixture_file, every_manifest_kind_and_group_has_a_test}` |
| Lowering is deterministic | `recursive_schema_golden::repeated_resolution_is_byte_identical` |
| Consumer inventory, file ownership, test targets | `inventory/consumers.md` |
| Shared reference API contracts | `inventory/contracts.md` |
| Duplicate-member ruling | this log; existing `schema_enhancements_matrix::raw_schema::raw_json_schema_keyword_matrix` duplicate rows |

The robustness matrix's new cells (recursive bodies, merge-ingress raw
references) are built and tested in Waves 2 and 3, as the plan assigns; Phase 1
changes no reader, so no matrix cell is implemented here
(`contracts.md` §12 records each table's state).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter", "test-toolkit", "claudine"]`,
  `change_class: "package"`. Claudine is a narrowed `claudine::l1` cell
  because `claudine/lib/tests/l1/boundary_lint.rs:150` (`read_darkmatter_source`)
  joins `../../darkmatter/lib` in a helper, so every non-source file under
  `darkmatter/lib` maps to the whole binary; the helper reads only named
  source files, so the fixtures cannot affect it (pre-existing, left alone).
  `test-toolkit` is its archive-path guard lint cell.
- `darkmatter`: `just test` ran twice. Both runs: every darkmatter lib and CLI
  L1 test passed (10,374, confirmed by diffing `nextest list` against the run
  log); one failure, `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`,
  stopped the run (fail-fast). **Load-sensitive, pre-existing, unrelated**:
  it passes alone 3/3; under load (host load average 30–39 from other
  sessions) request 502 took 765 ms, so DMLS logged it as `slow request`
  rather than `request complete`, and the test's log filter only matches the
  latter. The remaining DMLS and Zed tests were then run with only that test
  excluded: `cargo nextest run -p dmls --features effects-instrumentation -p zed-dmls-cli -E "(L1 filter) & !test(=…)"`
  → 1,351 passed.
- `darkmatter`: `just lint` passed; `just check-tier-coverage darkmatter`
  passed (0 stranded); clippy `-D warnings` on the test target passed.
- `claudine`: the whole `claudine::l1` binary (the CI cell) → 399 passed.
- `test-toolkit`: `just archive-path-guard` → 2 passed.
- Dependent packages: Phase 1 changes no library code or public symbol (only
  darkmatter test files and fixtures), so no downstream package test can be
  affected beyond the cells above.
- Cross-OS: `just cross-check darkmatter --os windows recursive_schema_` →
  23 passed on native Windows (goldens byte-identical there; the canonical
  path comparisons hold).

## Phase 2

Phase 2 (Wave 2, "Reference Graph") started 2026-10-23 on the tree at
`689147156` (Phase 1 committed). Implemented directly, without subagents:
the wave is sequential and every task edits the same resolver.

### Wave 2: Reference Graph

**Definition storage.** New `darkmatter/lib/src/markdown/schemas/simplified/definitions.rs`:
`DefKey { namespace, name }`, `DefNamespace` (`InlineRoot`, `RootUnionArm(i)`,
`File(path)`, `FileRootUnionArm(path, i)`, `Embedded`, `Shipped`, replacing
the private `NamespaceKey` and its shared `Root` sentinel), `Definition {
body, origin, json_key }`, `DefinitionTable` (allocation-ordered entries plus
an explicit `json_key → DefKey` map; `outer()` reports a body's top-level
kind through alias bodies), `DefinitionOuter`, and `definition_pointer`
(RFC 6901 escaping). `TypeExpr::Ref(DefKey)` was added to
`simplified/types.rs`. `ResolvedSchema.definitions: Arc<DefinitionTable>`,
`EffectiveSchema::definitions()`, and `expand_named_types` →
`NamedTypeExpansion { schema, imports, definitions }` carry the table. Bodies
refer to each other by `DefKey`, never `Arc`.

**Graph discovery and admission** (`schemas/resolve/graph.rs`). Before
expansion the engine walks every reachable named type once (worklist over
`(namespace, name)`), records an edge per reference with `descends` set for an
object property, dictionary value, array item (`is_array`), or tuple slot,
and never inside a `function(...)` signature. Iterative Tarjan finds the
groups; members of a group with a cycle (more than one member or a self-edge)
become the cyclic set. The same SCC pass over non-descending edges alone finds
the rejected cycles, reported as `SchemaError::NonDescendingCycle { cycles }`
with every reference of each rejected cycle (in discovery order) and, for a
schema file, the reference name's span from that file's source map. A
reference that does not resolve adds no edge (expansion reports it in its old
order); a reference inside a function signature is resolved with the
dependency and probe sets restored, so expansion's dependency list is
unchanged. Because a cyclic definition is never pushed on the import stack,
a back-edge can no longer reach the `MAX_IMPORT_DEPTH` check; acyclic chains
keep `ImportCycle` "exceeded 32 levels".

**Two-pass expansion** (`schemas/resolve/recursive.rs`). Without a cycle the
engine runs exactly as before (the golden corpus stayed byte-identical without
a bless). With one, every use of a cyclic definition becomes a `Ref` and the
engine expands twice: pass 1 builds use sites from the use-site postfix alone
and expands each reached body from a FIFO worklist; a `UseSummary` (outer
kind, first list arm of a union, inherited markers with origins, description,
the definition's own count bounds, null admission) is read off those bodies,
recursing only through top-level references; pass 2 rebuilds every use site
from the summary under the inlined-reference rules
(`apply_reference_postfix`) and the published body has its use-site markers
stripped. `apply_postfix` routes an acyclic alias of a cyclic definition
(`tree: node`) through the same rules, so `[]` over a list definition and
over a union with a list arm keep their existing errors, resolved from the
outer shape without recursive expansion.

**Lowering** (`simplified/convert.rs`). `to_json_schema_with_definitions`
threads the table through the converter; a `Ref` lowers to `{"$ref":
"#/$defs/<key>", …use-site siblings}` (key bounds beside an object, item
bounds and `unique` beside an array, none beside a union or tuple), and every
reachable body is lowered once under a root `$defs` after the other root
keys, in allocation order. A body is lowered under the property path of its
first use, so a function type reached only through a definition fails as
`Convert { property: "root.f" }`. An empty table emits no `$defs`.
`to_json_schema` is the empty-table case. Root unions allocate every inline
arm into one table and hoist the arms' `$defs` to the union root.

**Consumers kept compiling.** Every exhaustive `TypeExpr` match in
`darkmatter`, `darkmatter-cli`, `dmls`, `claudine`, and `claudine-gen` gained
a deliberate `Ref` arm: back-reference by name where a type is displayed
(`serialize_property_atom`, DMLS hover lines, Claudine `type_label_for_atom`,
`claudine-gen` `describe_atom`), "not completable / no primitive / no
families" where a typed walker cannot follow it without a table (later waves
add following), identity by `DefKey` in `subtype::identity`, pass-through in
`phase.rs`, and a typed error in the grammar translator (Wave 4 replaces it).
DMLS `diagnostics/named_types.rs` projects `NonDescendingCycle` as a
schema-definition diagnostic ranged on the first cycle reference this
document wrote, the others as related information.

**Existing tests whose expectations changed (intended behavior change).**
Alias cycles were `ImportCycle`; they are now `NonDescendingCycle`:
`resolve::schema_plus_phase1::import_cycle_is_detected`,
`schemas::tests::embedded_sources::malformed_imports_cycles_and_examples_fail_preparation`,
`schemas_named_type_merge::a_cycle_through_bare_and_explicit_references_reports_the_chain`,
`suggest_constraint_phase4::import_cycles_remain_bounded_across_envelope_forms`
(wrapped in `ReferencedSchema`, like the other named-type errors), and DMLS
`named_type_references::a_standalone_contradiction_and_a_mixed_cycle_are_schema_definition_errors`
(now also asserts the range and related location).
`schemas_union_postfix::cycles_and_depth_limits_still_apply_through_union_postfixes`:
`a: [b(required), number]` + `b: a[]` descends through list items, so it is
admitted, and `[]` over a union with a list arm is refused as before
(`UnionReference` `ArrayArm { arm: 0 }`); its depth-limit half is unchanged.

**Docs.** Two passages of `darkmatter/docs/topics/schemas/definition.md`
described the removed DAG rule and were corrected to current behavior (with
the not-yet-built cases marked **planned**); the DMLS diagnostic page
describes the cycle diagnostic. The full documentation rewrite remains Wave 7.

### Design Decisions

1. **Use-site summaries come from a first expansion pass.**
   - Decision: a schema with a cycle expands twice; the first pass yields the
     bodies a `UseSummary` is read from, the second builds every use site
     from its definition's summary.
   - Alternatives considered: compute summaries from unexpanded definitions
     (duplicates the union-flattening and postfix rules); patch use sites
     after one pass (loses each use site's provenance once an acyclic
     expansion is cloned into many places); keep markers in the body and
     merge them at lowering (every typed consumer would need the table to
     answer `is_required`).
   - Why: the summary of a definition depends only on its top level, but that
     top level may be a reference, a list of one, or a union with referenced
     arms; reusing the real expansion keeps one implementation of those
     rules, and use sites stay self-contained (`PropertyDef::is_required`
     needs no table), as `contracts.md` §7 requires.
   - Downsides: a recursive schema's expansion costs about twice as much;
     acyclic schemas take one pass and are unaffected.
2. **`DefNamespace::FileRootUnionArm(path, index)` was added.**
   - Decision: inline arms of a root-union schema file are separate
     namespaces (`contracts.md` §2 updated).
   - Alternatives considered: keep `File(path)` for every arm (two arms that
     each declare `item` would share one `DefKey` in the union's table).
   - Why: the spec requires distinct identities for root-union arms.
   - Downsides: none found.
3. **A file-reference root-union arm with definitions is refused for now.**
   - Decision: `resolve_root_union`/`resolve_standalone_root_union` return
     `SchemaError::Convert` when an arm that names a schema file resolved
     with a non-empty table.
   - Alternatives considered: hoist its `$defs` without re-keying (a key
     collision would bind references to the wrong body).
   - Why: combining independently allocated tables is Wave 3's re-keying
     task ("Isolate Merge Definitions", root-union arms); refusing is the
     only safe interim result, and no such schema existed before.
   - Downsides: those schemas fail until Wave 3.
4. **References inside a `function(...)` signature never descend and never
   fail discovery.**
   - Decision: everything inside a signature is a non-descending position;
     an unresolvable reference there is skipped, and resolving one leaves the
     dependency and probe sets as they were.
   - Alternatives considered: treat a signature's object parameters as
     descending (they describe a parameter, not a child of the instance);
     resolve signatures with full side effects (adds dependencies the
     lowering never reads and could raise errors expansion never raised).
   - Why: the spec lists callable signatures as non-descending; expansion
     never reads signatures, so discovery must not change their outcome.
   - Downsides: editing a file reached only through a signature does not
     invalidate a cache; such a file cannot affect the lowered schema except
     by creating a rejected cycle.
5. **A nullable definition is never required at a use site.**
   - Decision: when the definition admits null (a union with `null` or
     `unknown`), `required` is dropped from every use's atom.
   - Alternatives considered: give `Ref` atoms table-aware `admits_null`.
   - Why: keeps the core-type rule (a nullable union is never required)
     without making `PropertyAtom` methods need the table.
   - Downsides: `admits_null()` on a `Ref` atom still answers `false`; the
     optional wrapper `anyOf[null, $ref]` is redundant but correct.
6. **A union definition's arms keep their arm-local annotations.**
   - Decision: only `required`, `generated`, and `default` are hoisted off a
     union body's alternatives; a file arm's `eager`, its description, and
     examples stay in the body. A `Ref` property gets no optional
     empty-string `file` arm inside the shared body.
   - Alternatives considered: hoist everything (an `eager` would turn a file
     arm's format at every use).
   - Why: matches what a property-level union hoists today.
   - Downsides: an optional property typed by a recursive union with a
     `file` arm does not accept the legacy `""` sentinel.
7. **Use-site bounds are checked against the definition's own bounds, not
   merged into the body.**
   - Decision: a use's key or item bounds become `$ref` siblings; they are
     merged with the definition's top-level bounds only to report a
     contradiction with both origins (definition first).
   - Alternatives considered: emit one body per constraint set.
   - Why: the spec forbids mutating a shared definition, and Draft 2020-12
     applies siblings with the target (pinned by the Phase 1 probe).
   - Downsides: a looser use-site bound is emitted although the body's
     stricter one decides.

### Requirement-to-test mapping (Phase 2)

All in `darkmatter/lib/tests/l1/recursive_schema_types.rs` unless noted.

| Requirement | Test |
| --- | --- |
| Direct recursion validates depth 8; bad leaf reports JSON path + line/column through `DarkmatterSchemas::validate` | `a_self_recursive_type_validates_depth_eight_and_positions_a_bad_leaf` |
| Two-file cycle validates depth 6, rejects with the deep path; both files are dependency edges | `a_two_file_cycle_validates_depth_six_and_records_both_files` |
| Same-file mutual recursion, one body each | `mutual_recursion_in_one_file_stores_each_definition_once` |
| All four descending categories accepted | `every_descending_position_closes_a_cycle` |
| Alias cycle / self-union / callable signature rejected naming references | `an_alias_cycle_is_rejected_naming_each_reference`, `a_self_union_is_rejected_even_though_another_arm_accepts_values`, `a_cycle_through_a_function_signature_is_rejected` |
| Mixed group reports only the non-descending cycle; removing it resolves | `a_mixed_group_reports_only_its_non_descending_cycle` |
| Two-file non-descending cycle has spans in both files | `a_two_file_non_descending_cycle_reports_spans_in_both_files` |
| Unknown names and the acyclic chain limit keep their errors | `unknown_names_and_long_acyclic_chains_keep_their_errors`; `schemas_union_postfix::cycles_and_depth_limits_still_apply_through_union_postfixes` |
| Acyclic alias into a cycle inlines to a `Ref` | `an_acyclic_alias_into_a_cycle_inlines_to_a_reference` |
| Repeated imports share one definition; deterministic, path-free keys | `repeated_imports_share_one_definition_and_resolution_is_deterministic` |
| Namespace isolation (inline vs file, root-union arms), `$defs` at the union root | `same_named_types_from_different_namespaces_get_distinct_keys`, `root_union_arms_are_separate_namespaces_with_definitions_at_the_union_root` |
| Embedded self/mutual recursion with no filesystem | `resolve::embedded_recursion::self_and_mutual_recursion_resolve_from_embedded_documents` (unit) |
| Stable keys and JSON Pointer escaping | `simplified::definitions::tests::*` (unit); Phase 1 probe `an_escaped_pointer_key_resolves` |
| Acyclic lowering byte-identical | `recursive_schema_golden::*` (unchanged, not blessed); `an_acyclic_schema_lowers_with_no_definitions` |
| Inherited required/generated/eager, descriptions, defaults moved to uses; body presence-free | `definition_presence_and_annotations_move_to_each_use` |
| Nullable unions override required | `a_nullable_union_definition_is_never_required` |
| Two differently constrained uses, one unchanged body; array bounds | `two_differently_constrained_uses_share_one_unchanged_body`, `a_list_definition_counts_use_site_bounds_as_items` |
| Conflicting constraints name definition and use site | `contradictory_use_site_bounds_name_the_definition_and_the_use` |
| `suggest` on a recursive reference rejected | `a_suggestion_on_a_recursive_reference_is_rejected` |
| Function type reached only through a definition is a load error at the use-site path | `a_function_reached_only_through_a_definition_fails_at_the_use_site` |
| Required children with `[]` leaf; missing/null at depth 4; optional missing/null; null list items | `required_children_accept_an_empty_leaf_and_reject_missing_or_null_at_depth_four` |
| Input matrix, referenced `types.item` column (control recursive; absent, null, wrong type, one/every wrong element, empty map/list/string, duplicate key, trailing document) | `a_referenced_definition_takes_every_declaration_shape_from_a_recursive_control` |
| DMLS projects the cycle at its references | `dmls::l1 named_type_references::a_standalone_contradiction_and_a_mixed_cycle_are_schema_definition_errors` |

Matrix smell check over the touched readers: no `#[serde(default)]`,
absence-collapsing `Option`, `filter_map(.. as_str())`, `unwrap_or_default()`
on a load-bearing parse, or `.ok()` parse was added; the two `.ok()`s in
`graph.rs::locate_reference_spans` sit on the error path's span lookup, where
an unreadable file leaves the span `None` as `locate_origin_spans` does.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "claudine-gen", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit"]`,
  `change_class: "package"`, so no gate was skipped.
- `darkmatter`: `just lint` passed. `just test`: the first run stopped
  (fail-fast) on `embedded_sources::malformed_imports_cycles_and_examples_fail_preparation`,
  an alias-cycle expectation updated above; the second run passed every
  darkmatter library and CLI L1 test and stopped on
  `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  (watch-registration count 3 vs 2). It passes alone 3/3 and touches no
  recursive schema: **load-sensitive, pre-existing**. The remaining DMLS and
  Zed L1 tests were run with only that test excluded → 1,347 passed.
  `just check-tier-coverage darkmatter` → 0 stranded.
- `claudine`: `just test` → 10,692 passed (a first run, overlapping the
  Windows cross-check's sync, stopped on
  `composition::lifecycle::prewarm::worker::tests::a_successful_or_no_op_preparation_counts_as_done`,
  a TTS pre-warm timing test that passes alone 3/3: load-sensitive and
  unrelated; the rerun with nothing else running passed in full).
  `just lint` passed. `just check-tier-coverage claudine` → 0 stranded.
- `claudine-gen`: `cargo nextest run -p claudine-gen` → 250 passed.
- `test-toolkit`: `just archive-path-guard` → 2 passed.
- Dependent packages: `biscuit-speaks-cli`, `messenger`, `messenger-cli`,
  `sniff-cli`, `research`, `biscuit-icon-cli`, and `playa-cli` depend on
  darkmatter but match no `TypeExpr`, build no `ResolvedSchema`, and call no
  `expand_named_types`; `cargo check --tests` on all of them passed. Their
  schema behavior changes only for schemas that recurse, which previously
  failed to resolve.
- Cross-OS: `just cross-check --os windows darkmatter recursive_schema_ embedded_recursion import_cycle cycles_and_depth_limits a_cycle_through_bare definitions::tests`
  → 54 passed on native Windows (canonical namespace paths and file spans
  hold there).

## Phase 3

Phase 3 (Wave 3, "Shared Runtime Context") started 2026-10-23 on the tree at
`689147156` with Phase 2's changes still uncommitted in the working tree
(Phase 2 was not committed before this run; the files Phase 3 touched are
listed in its own frontmatter key). Implemented directly, without subagents:
the wave is sequential and its three tasks share `resolve.rs`, `mod.rs`, and
the new scope module.

### Wave 3: Shared Runtime Context

**Reference lookup** (new `darkmatter/lib/src/markdown/schemas/scope.rs`).
`SchemaScope` resolves a local `$ref` (`#`, `#/…`, the empty reference, with
`%XX` and `~0`/`~1` decoding) against its own root; `resolve` is one hop
(`Ok(None)` for no `$ref` or a non-local one, which only the validator can
follow) and `chain` follows hops, failing with
`SchemaError::UnresolvedReference` on a missing target or a reference-only
loop (`a → b → a`) instead of treating it as unconstrained.
`with_root_definitions(sub, root)` gives a sub-schema compiled alone its
root's `$defs`. `check_joined_references` is the admission rule at every
merge boundary, and `fold_definitions` joins one schema's `$defs` to another
root, renaming only a colliding different body and only the joining side's
references, in schema keywords only.

**Synthetic validators** now compile with their root's definitions:
`wrap_arm_as_root_schema(arm, root)` (used by `CompiledValidators`,
`coerce::accepts`, `rewrite_root_union`, `root_schema_arm_applies`),
`narrow_discriminated_union_problems`, `rewrite_property_union` (the rewrite
walkers carry `root`), `select_file_mode` (the caller file-mode classifier in
`compose/schema_validation.rs` carries `root`), `schema_declares_property`
(follows a `chain`), and `example(...)` targets (`resolve_document_examples`
passes the root `$defs`; inline union arms now resolve examples before their
definitions are hoisted). Sites left alone, each with its reason, are listed
in `inventory/contracts.md` §9 "As built (Wave 3)".

**Merge isolation.** `validate_simple_object_schema(schema, label)` admits a
root `$defs` (root `$ref`, unions, and applicators stay rejected), checks
references, and meta-validates a schema that carries `$defs`.
`merge_baseline` moves a baseline's `$defs` into the document's root (a root
union's root, never an arm) and merges properties through the new
`merge_layer`; it refuses to move them into a root union whose arm declares
its own `$id`. Trigger payloads go through the same contract
(`enforce_merge_compatible`, reported as `TriggerPayloadNotMergeable`) and
the same merge. Root-union file arms: `reject_arm_definitions` is gone;
`join_file_arm_definitions` absorbs a SimplifiedSchema arm's typed table into
the union's (`DefinitionTable::absorb`) and renames its JSON references to
match, and folds raw arms (and file arms that are themselves unions) after
every arm. `ResolvedPayload`/`TriggerLayer` carry their typed tables and
`EffectiveSchema::definition(&DefKey)` looks a reference up across the
document's and the trigger payloads' tables.

**Phases.** `phase::project_definitions` projects every definition body under
the same keys (`DefinitionTable::map_bodies`); `validate_for_phase` lowers the
projected document against it and the projected `EffectiveSchema` carries it.
`merge_with_effective` replaces the document's own bodies in the effective
`$defs` with the projection's and keeps the layer-only ones.

**Docs and skill.** `darkmatter/docs/topics/schemas/definition.md`: the
recursion paragraph no longer calls file-arm, trigger, and baseline recursion
planned; "JSON Schema Baseline Restrictions" and "Merge Semantics" describe
`$defs` admission, the reference rules, and re-keying; the phase section
describes timing below a recursion. `.claude/skills/darkmatter/schema.md`
gained a "`$defs` across merges" entry. `inventory/contracts.md` records the
as-built API (sections 6, 8, 9, 10).

### Design Decisions

1. **The merge error fields name the schema with a label, not a path.**
   - Decision: `UnresolvedReference { schema, reference, location }` and
     `UnsupportedMergeReference { schema, location, reason }`, `schema` being
     a label (`the baseline`, `the payload \`x.yaml\``, `the root-union arm
     \`./a.json\``).
   - Alternatives considered: `path: PathBuf` as the contract drafted (an
     in-memory baseline has no path); `source: String` (`thiserror` treats a
     field named `source` as the error source).
   - Why: every ingress needs a name, only some have a path.
   - Downsides: a consumer cannot open the file from the error alone; inside
     a trigger the payload path is on the wrapping `TriggerPayloadNotMergeable`.
2. **A JSON-level key is shared only for an equal body.**
   - Decision: `fold_definitions` shares a colliding key when both bodies are
     byte-equal and every key they reference is shared too (a fixpoint);
     otherwise the joining side's key is renamed.
   - Alternatives considered: share by `DefKey` (a raw schema has none, and
     `merge_baseline` sees only JSON); always rename (the same baseline merged
     into each arm of a root union would add one copy per arm).
   - Why: two byte-equal bodies with equal references validate identically,
     so sharing can never bind a reference to a different meaning.
   - Downsides: two different `DefKey`s with equal bodies share one key in
     the merged JSON; nothing reads identity from a JSON key (contracts §4).
3. **`#/$defs/…` references are checked at every merge, with or without a
   table; other forms only when the schema carries `$defs`.**
   - Decision: as stated; a baseline without `$defs` keeps `#/properties/…`
     and similar references as before.
   - Alternatives considered: check nothing without `$defs` (contracts §8 as
     drafted: a missing `#/$defs/x` would then bind to the document's `x`);
     check every form always (tightens previously accepted baselines).
   - Why: the matrix requires "`$defs` absent → Unresolved", and the spec
     forbids a reference binding to the assembled root; only a `#/$defs/…`
     reference can newly bind to another layer, since no layer had `$defs`
     before.
   - Downsides: a raw document schema with its own `$defs` merged with a
     baseline that relied on them is now refused; no such pairing could be
     written with an intent other than the bug the rule prevents.
4. **Meta-validation runs only for a merge schema that carries `$defs`.**
   - Decision: `jsonschema::draft202012::meta::validate` on the incoming
     schema when it has a root `$defs`.
   - Alternatives considered: always (an invalid baseline property the
     document overrides was accepted before); never (the structural matrix
     cells would surface only when the merged validator compiles, without
     the baseline's location).
   - Why: matches the plan's "Existing accepted acyclic property schemas must
     not become more restrictive" while giving the new ingress a located
     structural error.
   - Downsides: one meta-validation per merge of a schema with `$defs`
     (merges of trigger layers are memoized).
5. **A typed file arm joins a root union through its typed table.**
   - Decision: `DefinitionTable::absorb` allocates the arm's definitions in
     the union's table, and the arm's JSON is renamed with the returned map.
   - Alternatives considered: fold JSON only (the union's typed table, which
     phase projection lowers the arms against, would then disagree with the
     JSON keys and bind a projected reference to another arm's body).
   - Why: `file_arms_with_same_named_definitions_are_rekeyed_into_the_union_root`
     validates both phases through those keys.
   - Downsides: raw arms fold afterwards, so a raw arm's key can depend on
     the typed keys around it; only key names change.
6. **`EffectiveSchema::definitions()` stays the document's table; a lookup
   across layers is `definition(&DefKey)`.**
   - Decision: trigger tables ride `TriggerLayer` and are searched by
     identity after the document's.
   - Alternatives considered: one merged table (a trigger entry's `json_key`
     would not match the key the JSON merge gave it, inviting a wrong
     `key_for_json` lookup).
   - Why: `DefKey` is unique across layers, so one identity lookup serves the
     merged shape; the document table's keys equal the effective JSON's.
   - Downsides: a consumer wanting every layer's table must use
     `definition`, not iterate `definitions()`.
7. **`DefNamespace::Raw` was not added.**
   - Decision: raw `$defs` exist only as JSON keys.
   - Alternatives considered: the contract's `Raw(RawScope)` variant.
   - Why: nothing produces a typed body for a raw definition; an unused
     variant would add a match arm everywhere for no reader.
   - Downsides: a later typed consumer of raw definitions must add it.
8. **Phase projection of definitions is not memoized.**
   - Decision: `project_definitions` runs per `validate_for_phase` call.
   - Alternatives considered: a per-phase `OnceLock` on `EffectiveSchema`
     (contracts §10 as drafted).
   - Why: it is linear in the table and runs beside a full lowering of the
     document that is not memoized either; the derived JSON is memoized by
     `phase_json_schema`.
   - Downsides: a repeated phase validation repeats the projection.
9. **The caller file-mode classifier and the rewrite walkers carry the root
   now, although Wave 4 owns their reference following.**
   - Decision: thread `root` through `select_file_mode`,
     `resolve_caller_file_value`, `collect_applicable_root_schema_fragments`,
     and the `rewrite_*` walkers, and compile each union arm with its root's
     `$defs`.
   - Alternatives considered: leave arms that hold a `$ref` failing to
     compile (an arm silently dropped from "exactly one applicable arm"
     could turn an ambiguous union into a guessed file mode).
   - Why: the plan's Wave 3 task covers every independently compiled
     synthetic schema, including rewriting.
   - Downsides: a wider signature change in two files Wave 4 also edits.

### Requirement-to-test mapping (Phase 3)

All in `darkmatter/lib/tests/l1/recursive_schema_merge.rs` (L1, F2: public
`DarkmatterSchemas`/`EffectiveSchema`/`resolve_yaml_schema` with real files in
a temp dir) unless noted.

| Requirement | Test |
| --- | --- |
| Lookup: local pointer, escaping, missing target, external, ref-only loop | `schemas::scope::tests::a_reference_resolves_in_its_own_root_and_a_missing_target_is_an_error`, `…a_reference_chain_that_never_reaches_a_schema_is_an_error` (unit) |
| Synthetic root-union arm keeps the union `$defs` (validate + both phases, depth 6) | `a_recursive_root_union_arm_validates_with_the_union_definitions` |
| Same-named definitions in two file arms re-keyed; typed and JSON keys agree through phase projection | `file_arms_with_same_named_definitions_are_rekeyed_into_the_union_root` |
| Mixed raw/simplified arms; raw key moves, generated key kept | `a_raw_arm_keeps_its_definitions_and_an_inline_arm_keeps_its_keys` |
| An arm reference that would bind to another arm is refused (Unresolved / Unsupported) | `an_arm_reference_that_would_bind_to_another_arm_is_refused` |
| Baseline `$defs` join the document root; document keys never move; literal data untouched | `a_baseline_with_definitions_joins_the_document_root_without_rebinding_either` |
| Per-arm baseline merge with definitions at the union root once | `a_baseline_with_definitions_joins_every_arm_of_a_root_union_once` |
| `$id` arm cannot receive moved definitions; without `$defs` it merges as before | `a_baseline_cannot_move_its_definitions_into_an_arm_with_its_own_id` |
| Raw ingress matrix, JSON (every `$defs`, `$defs.item`, `$ref`, `$id`, `type`, `properties`, `required`, `items`, `anyOf`/`oneOf` cell; duplicate keys; trailing content) | `raw_json_baseline_ingress_takes_every_cell_from_a_recursive_control` |
| Raw ingress matrix, YAML (same cells; duplicate keys; second document; trailing content) | `raw_yaml_baseline_ingress_takes_every_cell_from_a_recursive_control` |
| In-memory baseline uses the same validator | `an_in_memory_baseline_shares_the_ingress_validator` |
| Existing acyclic acceptance unchanged without `$defs` | `a_baseline_without_definitions_keeps_its_other_reference_forms` |
| Trigger payload with recursion merges beside a recursive document; phase merge keeps payload defs; identity lookup | `a_recursive_trigger_payload_merges_beside_a_recursive_document` |
| Raw trigger payload ingress wiring (control + unresolved) | `a_raw_trigger_payload_with_an_unresolved_reference_is_not_mergeable` |
| Embedded namespaces (embedded payload + embedded library + document) | `embedded_payload_namespaces_join_a_recursive_document_without_collision` |
| Launch/Completion below recursion | `a_phase_projects_presence_below_the_recursion` |
| One definition shared by eager and deferred properties | `a_definition_shared_by_eager_and_deferred_properties_projects_once` |
| Example target judged with root definitions | `an_example_on_a_recursive_property_is_judged_with_the_root_definitions` |
| Folding: equal bodies shared, different re-keyed, dependent equal bodies move, literal data kept | `schemas::scope::tests::folding_shares_equal_bodies_and_rekeys_different_ones`, `…an_equal_body_that_references_a_moved_key_moves_too` (unit) |

Matrix cells use non-convertible bad values (`{"not": "text"}`) so they stay
meaningful once Wave 4 extends conversion past the old cutoff; a numeric bad
leaf at depth ≤ 4 is converted to text today and would hide the assertion.

Matrix smell check over the touched readers: no `#[serde(default)]`,
absence-collapsing `Option`, or `filter_map(.. as_str())` on a load-bearing
field was added. `schema_declares_property` reads a failed `chain` as "not
declared" (`unwrap_or_default`): it is a passive predicate over an already
compiled schema, documented at the line. `merge_layer`'s existing
`unwrap_or_default` reads of `properties`/`required` are unchanged.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["claudine", "claudine-gen", "darkmatter", "darkmatter-cli", "dmls", "test-toolkit"]`,
  `change_class: "package"` (the working tree still holds Phase 2's
  uncommitted changes as well), so no gate was skipped.
- `darkmatter`: `just lint` passed (including the Zed WASM check).
  `just test`: every darkmatter library and CLI L1 test passed; it stopped
  (fail-fast) on `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`
  (a trace-log equality), which passes alone 3/3. The remaining DMLS tests
  were then run without it: `searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  (already load-sensitive in Phase 2) and
  `startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration`
  failed together and pass alone (2/3 then 4/4); every other DMLS test
  passed, and `zed-dmls-cli` passed 27/27. The host's load average was 30–65
  throughout from processes outside this session (another `rustc`,
  DaisyDisk, Finder). None of the three tests reads a recursive schema:
  **load-sensitive, pre-existing**. `just check-tier-coverage darkmatter` →
  0 stranded.
- `claudine`: `just test` first stopped on
  `claudine-cli::l1 wrap_watchdog_startup_stall::watchdog_startup_stall_terminates_without_a_wall_clock_timeout`
  and `…watchdog_opencode_stall_before_first_step_is_bounded` (whole-run
  wall-clock 5.0–5.4 s against a 4.7 s budget; in-wrapper time 2.1–2.4 s),
  at load average 44–65. Alone they passed, failed, and passed again with
  the load unchanged. Wave 3's new reference walk was measured on Claudine's
  shipped `claudine.yaml` payload at 53 µs per walk, so it cannot account for
  a second; **load-sensitive**. `just test --no-fail-fast` then passed
  10,692/10,692. `just lint` passed. `just check-tier-coverage claudine` → 0
  stranded.
- Dependent packages: `cargo check --tests` on `biscuit-speaks-cli`,
  `messenger`, `messenger-cli`, `sniff-cli`, `research`, `biscuit-icon-cli`,
  and `playa-cli` passed; none matches `SchemaError` exhaustively or calls
  `wrap_arm_as_root_schema`.
- Cross-OS: `just cross-check --os windows darkmatter recursive_schema_merge scope::tests`
  → 31 passed on native Windows (canonical `DefNamespace::File` paths and
  embedded virtual paths hold there).
