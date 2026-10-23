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
human_review: false
message_to_agent: |-
    Phase 1 (Wave 1) produced the shared contracts every later wave codes
    against: read claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
    (DefKey/DefNamespace, DefinitionTable ownership, key allocation and
    re-keying, edge classification, new SchemaError variants, use-site
    semantics, merge-ingress rules, SchemaScope, phase memoization) and
    inventory/consumers.md (every TypeExpr match with path:line, the 22
    wildcard matches that would silently absorb Ref, the 14 synthetic-validator
    sites, walkers, tests per area, file ownership by wave). If your wave needs
    a different shape, change contracts.md first and log why.
    The acyclic golden corpus is darkmatter/lib/tests/l1/recursive_schema_golden.rs
    with frozen inputs under darkmatter/lib/tests/fixtures/recursive_schema_golden/.
    It must stay green through every resolver change; only bless
    (DARKMATTER_BLESS_RECURSIVE_GOLDEN=1) for a change meant to alter acyclic
    lowering, and say so in the log. Keep every fixture path literal inside a
    #[test] (helpers use concat! constants) so CI narrows to single tests.
    Claudine's lib/tests/l1/boundary_lint.rs helper joins ../../darkmatter/lib,
    so any non-source file under darkmatter/lib (including these fixtures)
    schedules the whole claudine::l1 binary on CI; it is pre-existing and was
    left alone.
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
