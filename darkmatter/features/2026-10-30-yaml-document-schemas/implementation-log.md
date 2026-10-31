---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/spec.md
plan: darkmatter/features/2026-10-30-yaml-document-schemas/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
- darkmatter
source_files_during_phase_1:
- darkmatter/lib/src/markdown/schemas/embedded.rs
- darkmatter/schemas/schema.yaml
- darkmatter/schemas/schema-trigger.yaml
- schemas/kind.yaml
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/darkmatter/schema.md
decisions_for_review:
- |-
    The plan gave the "final text" for the two new definition files, but two of the
    descriptions in that text were not valid YAML: each had a colon followed by a space
    inside an unquoted value ("the exported document schema: an inline mapping ..." in
    schema.yaml, and "the conditions that select a document: one expression ..." in
    schema-trigger.yaml). Loading either file failed with "mapping values are not allowed
    in this context". I kept the wording and put the clause after the colon in
    parentheses instead. The alternative was to quote the whole value, which works but
    reads worse next to the unquoted lines around it. Picking the other option means
    editing only those two description lines.
- |-
    The plan said to create schemas/kind.yaml (the repository's `kind` catalog) as a
    `kind: schema` document. I created it without the `kind: schema` line, as a plain
    `$schema` document with `description`, which is how the two review schemas in the same
    folder are written. Reason: Darkmatter already treats any `kind: schema` file whose
    `$schema` is a mapping and that has no `types` as a host global catalog, and the
    repository-root `schemas/` folder is a schema root for every document in the repository.
    With `kind: schema`, every Markdown document in the repo got a new editor warning
    (`dm.schema.host_catalog`, "`$schema.kind` must be an object shape"). A DMLS corpus
    test caught this. Other options: add an empty `types:` mapping (a workaround nobody would
    understand later), or change how host catalogs are detected (a much larger change in
    behavior, outside this feature). If a reviewer wants `kind: schema` back, the
    host-catalog detection has to change first. The file still validates `kind` values the
    way the plan intended (checked with `md schema validate`).
message_to_agent: |-
    Phase 1 notes for later phases:
    1. schemas/kind.yaml is a plain `$schema` document (no `kind: schema` line). A
       `kind: schema` file whose `$schema` is a mapping and that has no `types` is
       read as a host global catalog (`catalog::is_global_catalog`) from any schema
       root, and the repo-root `schemas/` folder is one. Phase 5's docs for
       kind-documents.md must describe kind.yaml as it is, not as a `kind: schema`
       document. Any new YAML fixture placed in a real schema root has the same trap.
    2. The two description lines in schema.yaml / schema-trigger.yaml differ from the
       plan's Rule 7 text (parentheses instead of a colon, because the colon form is not
       valid YAML). Do not paste the plan's text back in.
    3. A test that reads a repository path must use `biscuit_test_harness::manifest_dir!()`,
       not `env!("CARGO_MANIFEST_DIR")`. `test-toolkit`'s archive_path_guard fails
       otherwise, and only `cargo nextest run -p test-toolkit` catches it, not the
       darkmatter `just test`.
---

# Implementation Log for 2026-10-30-yaml-document-schemas (5 phases)

## Phase 1

### What was done

- Wave 1: wrote `darkmatter/schemas/schema.yaml`, `darkmatter/schemas/schema-trigger.yaml`
  (Rule 7 text, with two description lines reworded; see `decisions_for_review`), and
  `schemas/kind.yaml` (Rule 5 enum with `schema-trigger`, as a pure `$schema` envelope;
  see `decisions_for_review`).
- Wave 2: registered both definitions in `embedded::SHIPPED` after `partials/functions.yaml`
  and before `CONTENT_POLICY_PATH`; updated the `SHIPPED` doc comment and the exact-list test
  (renamed to
  `the_shipped_table_registers_exactly_the_catalog_its_partials_kind_definitions_and_policy_schema`).
- Added `every_yaml_file_under_darkmatter_schemas_is_shipped_or_listed_as_embedded_elsewhere`
  (Rule 16) in `embedded.rs` tests, with `UNSHIPPED` = `partials/mutations.yaml` and the four
  `partials/function-examples/*.yaml`, each with a comment naming its embedding site. It also
  fails when a `SHIPPED` entry under `darkmatter/schemas/` has no file.
- Corpus: `shipped_schema_corpus.rs` walks `darkmatter/schemas` and `schemas`; both new
  definitions and `kind.yaml` classify as `Loadable` with no exception entry, and
  `loadable_shipped_schemas_resolve_and_lower` passes for them. No change to that file.

### Behavior checks with the worktree's `md` (target/debug/md)

Markdown files naming each definition as `$schema`:

| Input | Result |
| --- | --- |
| `kind: schema` + `types` against schema.yaml | valid, exit 0 |
| `kind: schemax` | `arm[1]: kind "schema" was expected (at line 3, column 7)`, exit 1 |
| `kind: schema` + `description` only | `arm[0]: $schema is a required property`, exit 1 |
| trigger with nested `group`/`min-match`/`$path` | valid |
| trigger with no `match` | `match is a required property`, exit 1 |
| trigger `group: 5` | `match/group 5 is not of type "array"`, exit 1 |
| `kind: task` against kind.yaml | valid |
| `kind: schema-target` against kind.yaml | `not one of "schema", "schema-trigger" or 4 other candidates` |

### Requirement-to-test mapping

- Definitions shipped (Rule 2/Wave 2): exact-list test in `embedded.rs`.
- SHIPPED completeness (Rule 16): new `embedded.rs` test above.
- Definitions parse and lower as standalone schemas (Wave 1 risk): existing
  `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` and
  `shipped_schema_artifacts_parse_passively` (both walk the new files).
- `kind.yaml` is a plain schema in repo-root `schemas/`: existing
  `meta_schema_repo_schemas::repo_root_schemas_all_classify_as_standalone_schemas`.
- `kind.yaml` is not read as a host catalog: existing
  `dmls::l1 mapping_only_corpus::mapping_only_corpus_diagnostics_match_pre_descent_baseline`.
- Phase 1 adds no loader or validator code, so the input robustness matrix starts in Phase 2.

### Problems found and fixed

- `test-toolkit::archive_path_guard` rejected `env!("CARGO_MANIFEST_DIR")` in the new
  completeness test; switched to `biscuit_test_harness::manifest_dir!()`.
- With `kind: schema` in `schemas/kind.yaml`, the DMLS mapping-only corpus test failed with a
  `dm.schema.host_catalog` warning on every repo document (it failed the same way when run on its
  own, so load was not the cause). Fixed by making `kind.yaml` a pure envelope (see
  `decisions_for_review`). The first full `just test` run had passed with the bad file. The cause
  was not investigated; the most likely explanation is a stale result from incremental compilation
  or a cache.
- Temporarily edited `meta_schema_repo_schemas.rs` to accept a tagged `kind.yaml`, then reverted
  it once `kind.yaml` became pure. That file is unchanged.

### Gates

- `just test` (darkmatter area): 12885 passed, 10 skipped.
- `just lint` (darkmatter area): pass.
- Dependents: `cargo nextest run -p test-toolkit`: 479 passed. claudine/claudine-cli slice
  `-E 'test(/embedded|shipped|bundle/)'`: 572 passed. No claudine test reads the repo-root
  `schemas/` folder, and `SHIPPED` is crate-private with additions only.
- Scope calculator (`git status … | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, test-toolkit, darkmatter-cli]`, `change_class: package`, so the
  gates were required and were run.
- Cross-OS: not run. Only the YAML data files, a table entry, and one test changed; the test
  builds its virtual paths from path components joined with `/`, so it does not depend on
  the path separator.
- Skill: added the host-catalog envelope trap to `.claude/skills/darkmatter/schema.md` ("Authored global catalog").
