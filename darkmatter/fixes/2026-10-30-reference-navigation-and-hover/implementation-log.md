---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/fixes/2026-10-30-reference-navigation-and-hover/spec.md"
plan: "darkmatter/fixes/2026-10-30-reference-navigation-and-hover/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
decisions_for_review:
    - |-
      Error text of a whitespace-padded schema reference. Before this change the three copies of
      the file-selection code disagreed on the reference they put into a "no match" error: the
      `$schema` path used the trimmed text, while named-type imports and `example(...)` used the
      text exactly as written (including surrounding spaces). The plan says the shared selector keeps
      error values identical. Choice: the shared selector always reports the trimmed reference, in
      every error. Alternative: carry the untrimmed text and a per-caller flag to reproduce the old
      inconsistency exactly, which adds state with no user benefit. No existing test depended on
      the old spacing. Redo if rejected: `SchemaFileSelection::into_result` in
      `simplified-schema/lib/src/resolve/select.rs` would need the authored text and a flag.
    - |-
      Where the selector's unit tests live. The plan suggested adding them to
      `simplified-schema/lib/tests/l1/schema_files.rs`, which tests the `SimplifiedSchema` file
      factories. Choice: a new file `schema_file_selection.rs` in the same test binary, declared in
      `main.rs`, because the subject is different. Alternative: append to `schema_files.rs`. Redo
      if rejected: move the file's tests; nothing else depends on the location.
    - |-
      `SchemaFileSelection`, `SelectionOutcome`, and `SelectionFailure` derive only `Debug`. The
      plan did not state derives; `biscuit_file::FileReferenceError` is neither `Clone` nor `Eq`,
      so the types cannot derive them. Phase 3 stores the selection in an `Arc`'d fact, which needs
      neither. Redo if a later phase needs `Clone`: wrap the error in an `Arc`.
source_files_during_phase_1:
    - simplified-schema/lib/src/resolve/select.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/tests/l1/schema_file_selection.rs
    - simplified-schema/lib/tests/l1/main.rs
    - darkmatter/lib/src/markdown/schemas/roots.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs
    - darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schema_reference_parity.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
docs_updated_during_phase_1:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/topics/lifecycle/host-artifact.md
    - claudine/docs/topics/schemas.md
    - simplified-schema/docs/topics/language/schema-documents.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/simplified-schema/SKILL.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - simplified-schema
    - claudine
message_to_agent: |-
  Phase 1 is done. Facts the next phase needs:
  - The selector is `simplified_schema::resolve::select_schema_file` (file `simplified-schema/lib/src/resolve/select.rs`). Its types derive only `Debug` because `biscuit_file::FileReferenceError` is not `Clone`/`Eq`; wrap in `Arc` if a fact must be cloned.
  - The resolver's three call sites (`resolve_reference_in_context`, `named_type_file_path`, `resolve_example_path` in `resolve.rs`) now call it, so the Phase 2 loader restriction (Rules 5 to 8) only touches `load_schema_from_path_in_context` and the parsers.
  - The new root order also changed two catalog test fixtures not named in the plan (`catalog/discovery_tests.rs`, `catalog/host_function_cache_tests.rs`, fields now `earlier`/`later`) and three tests in `dmls/src/overlay/registry.rs`. A fail-fast `just test` hides later failures, so expect to rerun after each fix.
---

# Implementation Log for 2026-10-30-reference-navigation-and-hover (6 phases)

## Phase 1

### 1.1 Root order (Rules 1 and 2)

- `darkmatter/lib/src/markdown/schemas/roots.rs`: the `candidates` array and the
  `SchemaRootKind` variants are now `SchemasDir, Package, PackageArea, Tree, Home`; module docs,
  `SCHEMAS_DIR_VARIABLE` docs ("first schema root"), and the aliasing consequence are documented.
  The duplicate-identity loop, symlink policy, and pattern bases are unchanged.
- `triggers/discovery.rs` module docs state the new order.
- Tests whose expected order changed:
    - `darkmatter/lib/tests/l1/schema_roots.rs`: the order test (renamed
      `schema_roots_are_schemas_dir_package_area_tree_then_home`) and the "root 4"/"fourth root"
      comments.
    - `darkmatter/dmls/tests/l1/schema_roots_parity.rs`: the searched-roots assertion.
    - `claudine/lib/tests/l1/claudine_schemas_embedded.rs`: the `Root` enum order and
      `nearest_claudine_trigger_shadows_farther_ones` order array.
    - `darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs` and
      `catalog/host_function_cache_tests.rs` (not listed in the plan; found by running the
      filtered suite): their fixtures named the roots `tree` (assumed first) and `external`
      (`SCHEMAS_DIR`, assumed second). The fields are renamed `earlier` (`SCHEMAS_DIR`) and `later`
      (tree root) so every assertion keeps its meaning.
    - `darkmatter/cli/tests/l1/schema_triggers.rs` and its snapshots needed no change (no fixture
      sets `SCHEMAS_DIR` alongside another root); the triggers discovery unit tests
      (`discovery.rs` 1203-1273) and `claudine/lib/src/invocation_context/document_tests.rs` do not
      depend on order.
- New tests in `darkmatter/lib/tests/l1/schema_roots.rs`:
    - `schemas_dir_shadows_the_package_root_by_file_name`: `searched()` lists `SchemasDir` first;
      the scan reports the package's `shared.trigger.yaml` and `shared.yaml` as shadowed; the
      bare-name `$schema` and the trigger come from `SCHEMAS_DIR`; unset restores the package.
    - `schemas_dir_aliasing_a_package_schemas_folder_searches_it_as_schemas_dir` (Rule 2): the
      package kind is `Duplicate { of: SchemasDir }`, the pattern base becomes the tree root, so
      a `docs/*.md` trigger applies to `{repo}/docs/x.md` and not to the package's document;
      unaliased, the package document matches.

### 1.2 Shared selector (Rules 3 and 4)

- New `simplified-schema/lib/src/resolve/select.rs`, re-exported as
  `simplified_schema::resolve::{select_schema_file, SchemaFileSelection, SelectionOutcome,
  SelectionFailure}`. `is_bare_name`, `try_bare_name_in_roots` (now `select_in_roots`), and the
  resolver's private `resolve_file_reference_in_context` moved into it.
- `resolve_reference_in_context`, `named_type_file_path`, and `resolve_example_path` call
  `select_schema_file(..)`, extend their `searched` set from `attempted`, then `into_result()`.
- `observed::probed` is now `observed::record_probes(&[PathBuf])` (frame recording only), called
  inside `select_schema_file`; callers keep the `BTreeSet` extend.
- Without the `file` feature the selector returns `Failed(Resolution(ResolutionUnavailable))`
  after the parse and URL checks, as before; `just test-minimal` passes.
- Creation target: bare name with roots → `roots[0]/name` for `Missing` and `SiblingOnly`;
  otherwise the first `candidate_plan_with_order(.., Resolution)` candidate on a clean miss
  (made absolute against `base_dir` if relative); `Failed` never carries a path.
- Every pre-existing resolver test passed unchanged (the equivalence proof): 1252 + 1110
  (minimal) before the new tests.
- New tests `simplified-schema/lib/tests/l1/schema_file_selection.rs` (declared in `main.rs`;
  `#![cfg(feature = "file")]`; contexts from `from_snapshot` with no home and no env):
    - first-of-two / only-second / both (first wins, second never probed), ordered `attempted`
      including the nearer miss, trimmed `reference`;
    - clean miss with one and two roots → `creation == first/name`, no file created;
      zero roots → `creation == docs/name`, `attempted == [docs/name, repo/name]`; zero roots with
      a repository-root file → `Found`;
    - sibling only → `SiblingOnly { suggestion: "./near.yaml", creation: first/near.yaml }`;
    - explicit `./x.yaml` and nested `sub/x.yaml` plan beside the document;
    - `../../../outside.yaml` (boundary), blank (malformed), `~/…` with no home (missing anchor) →
      `Failed(Resolution)`; URL → `Failed(Remote)`;
    - unreadable first root (unix, ancestor mode 000) → `Failed(Resolution(Io))`, the second
      root's file is never probed;
    - `into_result()` gives `NoMatch { bare_name }`, `BareNameSiblingExists`, `Unresolved`,
      `RemoteUnsupported` respectively.

### 2.0 Consumer parity

New `darkmatter/lib/tests/l1/schema_reference_parity.rs` (declared in `l1/main.rs`):

- `every_bare_name_consumer_selects_the_schemas_dir_copy`: `$schema: shared.yaml`,
  `Fields@types.yaml`, `example(example.yaml)`, and a package-root trigger whose payload is
  `payload.yaml` all take the `SCHEMAS_DIR` copy over the package copy (asserted on the effective
  JSON Schema text: each copy carries a distinct tag).
- `a_malformed_schemas_dir_file_is_reported_not_skipped`: the error names
  `SCHEMAS_DIR/shared.yaml`; the valid package copy is not used.
- `the_creation_target_follows_the_searched_roots`: set → link into `SCHEMAS_DIR`; unset,
  relative (invalid), and missing `SCHEMAS_DIR` → no `SCHEMAS_DIR` root, link into the package
  root, nothing created; aliasing the package folder → `Duplicate { of: SchemasDir }`, one root.
- "An unreadable root stays an error" is already covered by
  `schema_roots.rs::an_inaccessible_schemas_dir_root_is_an_error` (and the other four kinds),
  which passes with the new order; it was not duplicated.

### 2.1 Roots documentation

Updated the order (table, list, Mermaid diagram, and `md schema triggers` sample output) and the
aliasing consequence in: `darkmatter/docs/topics/schemas/definition.md`,
`docs/topics/schemas/authoring-schemas.md`, `docs/topics/schemas/dmls-schema-support.md`,
`docs/lsp/indexing.md`, `docs/lsp/configuration.md`, `docs/topics/lifecycle/host-artifact.md`,
`claudine/docs/topics/schemas.md`. `schema-activation.md`, `dmls/docs/editors/README.md`,
`schema-targeting.md`, and `claudine/docs/topics/composition.md` mention the roots without an
order and needed no change. `simplified-schema/docs/topics/language/schema-documents.md` gained the
`select_schema_file` contract with an example.

Skills: `.claude/skills/darkmatter/SKILL.md` and `schema.md` (order, aliasing, the one selector);
`.claude/skills/simplified-schema/SKILL.md` (quick-reference row).

### Checkpoint grep

`grep -rn "try_bare_name_in_roots\|is_bare_name" simplified-schema/lib/src/resolve.rs` shows only
the `bare_name_phase3` test module importing `select::is_bare_name`; no production caller remains
outside `select.rs`.

### Extra fixes found by the full suite

- `darkmatter/cli/tests/l1/schema_triggers.rs::triggers_command_prints_the_five_schema_roots_in_search_order`:
  the expected line order now starts with `SCHEMAS_DIR`.
- `darkmatter/dmls/src/overlay/registry.rs` tests `per_document_registries_equal_direct_discovery`,
  `watched_changes_to_an_envelope_an_external_payload_and_a_new_root_file_update_the_registry`, and
  `deleting_a_trigger_removes_it_and_creating_a_missing_payload_recovers`: the trigger name order
  now starts with the `SCHEMAS_DIR` root's `env.trigger.yaml`.

### Gates

- `simplified-schema`: `just test` 1259 passed + 1110 passed (`test-minimal`, no `file` feature);
  `just lint` clean.
- `darkmatter`: `just test` 12781 passed, 10 skipped (third run; the first two stopped on the
  order-dependent tests above, fixed and rerun alone before the full rerun); `just lint` clean.
- `claudine` (dependent, more than 200 tests): `cargo nextest run -p claudine -p claudine-cli -E
  'test(/schema|trigger|roots|SCHEMAS_DIR|document_tests/)'`: 1345 passed.
- The gate-skip check was not applicable: source files changed, so both gates ran.
- Cross-OS check skipped: the changes reorder an array and move existing resolution code into one
  function without new path handling; the only OS-specific test (`an_unreadable_first_root_...`)
  is `#[cfg(unix)]`, matching the existing inaccessible-root tests. CI runs Windows nightly.
- No pre-existing or load-sensitive failures were seen.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| Rule 1 order | `schema_roots::schema_roots_are_schemas_dir_package_area_tree_then_home`, `schema_roots_parity` (dmls), `claudine_schemas_embedded::nearest_claudine_trigger_shadows_farther_ones`, `schema_triggers::triggers_command_prints_the_five_schema_roots_in_search_order` |
| Trigger shadowing follows the order | `schema_roots::schemas_dir_shadows_the_package_root_by_file_name`, dmls `overlay::registry` tests |
| Rule 2 aliasing | `schema_roots::schemas_dir_aliasing_a_package_schemas_folder_searches_it_as_schemas_dir`, `schema_reference_parity::the_creation_target_follows_the_searched_roots` |
| Rule 3 one selector, same errors | all pre-existing resolver tests unchanged; `schema_file_selection::*` `into_result` assertions |
| Rule 4 creation target | `schema_file_selection::*`, `schema_reference_parity::the_creation_target_follows_the_searched_roots` |
| 2.0 consumer parity | `schema_reference_parity::every_bare_name_consumer_selects_the_schemas_dir_copy`, `a_malformed_schemas_dir_file_is_reported_not_skipped` |

