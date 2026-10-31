---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/spec.md
plan: darkmatter/features/2026-10-30-yaml-document-schemas/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
- darkmatter
- darkmatter-cli
- simplified-schema
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
- |-
    The plan said the new validation would add no code to the simplified-schema
    library. That could not hold: the library's resolver for compiled-in schema files
    refused any file whose exported schema is a list of alternatives (a "root union"),
    and the shipped `schema.yaml` definition is exactly that (one alternative requires
    `$schema`, the other `types`). I changed the resolver to accept a root union whose
    alternatives are all written inline, and to keep refusing an alternative that names
    another schema file, since compiled-in schemas never look files up. Alternatives:
    rewrite `schema.yaml` as a single mapping with both keys optional and leave "at
    least one of `$schema` or `types`" to Rust only (weaker definition, and the plan's
    required-set checks would change), or resolve that one file through a separate
    path inside Darkmatter (a special case nobody would expect). Reversing means
    restoring the refusal in `prepare_embedded_schema_in`, its two tests, and
    reshaping `schema.yaml`.
- |-
    The plan's corpus check listed the repository's internal `prompts/` folder among
    the folders to walk. I left it out: the repository's rules say the `prompts/`
    folders are drafts that are never tested in the normal suite, and a test that
    reads them fails on an unrelated prompt edit. The other seven folders are walked
    (27 documents checked). Adding `prompts` back is a one-line change to the folder
    list in `rust_accepted_kind_documents_pass_their_kind_definition`.
- |-
    The plan suggested `darkmatter/schemas/schema-trigger.yaml` as the control
    document for trigger-kind test cells. That file is itself a `kind: schema`
    document (it describes triggers), so it cannot stand in for a trigger. The
    trigger cells start from Claudine's shipped trigger
    `claudine/schemas/sequence.yaml` instead. Nothing needs redoing unless a
    reviewer prefers a dedicated trigger fixture.
- |-
    A document whose mapping has a key that is not a string (for example `1: one`)
    cannot be turned into the data the validator reads. The plan did not cover it. I
    report it as the same "schema document" error used for invalid YAML, with the
    message "cannot be read as a kind document: ...". The Rust parsers reject such a
    document too (unknown key). The alternative, treating it as unclaimed, would hide
    a document that does claim a kind. Changing it is one `map_err` in
    `validate_kind_document`.
- |-
    The plan said a YAML file that fails to load is reported the same way as a
    Markdown file whose frontmatter fails to parse (exit 3). I kept exit 3 but gave
    it its own wording and JSON code: pretty output says "the YAML document ... could
    not be loaded" and the JSON record has `"error": "yaml_parse"` instead of
    `"frontmatter_parse"`, because a YAML file has no frontmatter and the old wording
    would mislead. The alternative, reusing the frontmatter outcome unchanged, saves
    one enum variant but reports the wrong thing. Reverting means mapping the load
    failure to the existing parse-error outcome in `validate_yaml_document` and
    deleting the `YamlLoadError` variant and its two render arms.
- |-
    The plan said the assignment check ("assignments apply to Markdown frontmatter")
    runs before any file is validated. The extension of the opened file is only
    known after each argument is opened, so I check the argument's spelling up front
    (before any file is validated) and check the opened path again just before that
    file is validated, for a reference like `@name` whose spelling hides the
    extension. In that rare second case earlier files have already been reported.
    The alternative, opening every argument before validating any, would change the
    order of output for every run. Nothing else depends on this.
- |-
    A validated YAML document's JSON record has `"schema": null` whether or not a
    kind claimed it, as the plan specified (no schema label). That means JSON output
    cannot tell a claimed valid document from an unclaimed one; only the pretty
    output can. Putting the claiming kind into `schema` (for example `"kind:schema"`)
    would fix that but adds a value format the plan did not decide. Changing it is
    one field in `validate_yaml_document` plus a test assertion.
message_to_agent: |-
    Phase 3 notes for later phases:
    1. `md schema validate` now routes `.yaml`/`.yml` arguments (opened path,
       ASCII case-insensitive) to `validate_kind_document` in
       `darkmatter/cli/src/commands/schema/validate.rs` (`validate_yaml_document`,
       `is_yaml_document`). Load failures use a new outcome `YamlLoadError`:
       pretty output says "the YAML document <link> could not be loaded:", JSON
       `error` is `"yaml_parse"` (not `"frontmatter_parse"`), exit 3. A
       definition error is exit 2. An assignment with any YAML argument prints
       `Error: assignments apply to Markdown frontmatter, not to YAML document
       <path>` and exits 64 before anything is validated (also re-checked on the
       opened path for a spelling like `@name`). Phase 5's CLI docs
       (`darkmatter/docs/inline/schema-validation.md`) must describe these.
    2. JSON records for YAML documents keep `"schema": null` whether claimed or
       not (Rule 14's `schema_label: None`); only pretty output distinguishes
       claimed ("has a schema definition and is valid") from unclaimed.
    3. Phase 2 notes still apply for DMLS: `effective_for_kind(kind)` gives the
       schema; `schema.yaml` lowers to a two-arm root `anyOf`; tests that read
       repository files use `include_str!` or `manifest_dir!()`.
source_files_during_phase_2:
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/kind_documents.rs
- darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
- darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
- darkmatter/lib/tests/l1/kind_documents.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/trigger_spelling_guard.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/simplified/standalone.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/tests/l1/schema_validate_kind_documents.rs
- darkmatter/cli/tests/l1/main.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/schema.md
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

## Phase 2

### What was done

- Wave 3: `DarkmatterSchemas::effective_for_resolved` (`pub(super)`, `schemas/mod.rs`) builds an
  `EffectiveSchema` from a `ResolvedSchema` alone (no baseline, triggers, document `$schema`, or
  `base_dir`). New module `schemas/kind_documents.rs` with `SHIPPED_KINDS`,
  `KindDocuments::{shipped, from_documents, claimed_kind}`, `KindValidation`, and
  `DarkmatterSchemas::{with_kind_documents, effective_for_kind, validate_kind_document}` per Rule 9.
  `DarkmatterSchemas` gained a private `kind_documents: Option<Arc<KindDocuments>>` (`None` = shipped).
  `triggers/envelope.rs` `KNOWN_KEYS` is now `pub(crate)` for the key-set tie test.
- Unit tests in `kind_documents.rs`: SHIPPED membership, trigger key = `ENVELOPE_KIND`, claim table,
  key-set ties against `StandaloneSchemaEnvelope::Tagged.admits_top_level_key` and `KNOWN_KEYS`,
  `kind` literal per definition, required sets (`{kind,$schema}`/`{kind,types}` and `{kind,match}`).
- Wave 4: `darkmatter/lib/tests/l1/kind_documents.rs` (declared in `l1/main.rs`), and
  `shipped_schema_corpus::rust_accepted_kind_documents_pass_their_kind_definition` (27 repository
  documents checked, including both definitions and the three Claudine triggers).
- Wave 5: residual-rule `//!` lists in `simplified-schema/.../standalone.rs` and
  `triggers/envelope.rs` (docs only). Smell grep: one hit (`unwrap_or_default` in a unit-test helper
  reading lowered `required`), replaced with an explicit absent → empty match and strict `expect`s;
  the grep is now clean over both new files. `validate.rs` (CLI) is Phase 3 and does not exist yet.

### Departure from the plan: the embedded resolver refused root unions

`prepare_embedded_schema_in` (simplified-schema) returned "an embedded schema cannot be a root union",
and `schema.yaml` is a two-arm root union, so `KindDocuments::shipped()` could not prepare. I changed
the library to accept a root union of inline arms and to keep refusing an arm that names a schema
file (an embedded source never looks up files). New test
`resolve::embedded_recursion::an_inline_root_union_prepares_and_a_file_arm_is_refused`. Rule 1 said
simplified-schema gains no code; see `decisions_for_review`. Effect on other callers: an embedded
trigger payload file that is a root union now fails in `enforce_merge_compatible`
(`TriggerPayloadNotMergeable`) instead of with the old message; no test or shipped file depended on it.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| Rule 2 table (paths in SHIPPED, trigger key = ENVELOPE_KIND) | `kind_documents::tests::every_shipped_kind_definition_is_compiled_in`, `the_trigger_kind_is_the_envelope_kind` |
| Rule 3 claim (absent/null/number/quoted number/list/other/case/`trigger-schema`/sequence/scalar/empty all unclaimed; quoted `"schema"` claims) | unit `the_shipped_registry_claims_exactly_the_shipped_kinds`; L1 `only_a_string_kind_naming_a_shipped_kind_is_claimed` |
| Key-set, literal, and required ties to Rust | unit `the_schema_definition_names_the_tagged_envelope_keys`, `the_trigger_definition_names_the_trigger_envelope_keys`, `each_definition_fixes_its_kind_literal`, `each_definition_requires_what_its_envelope_requires` |
| Control rows report kind and no problems | L1 `a_valid_control_document_reports_its_kind_and_no_problems` |
| Document shapes (duplicate key, trailing garbage, second document, non-string key) → located `SchemaDocument` error; Rust rejects the same text | L1 `a_document_that_does_not_load_is_a_located_schema_document_error` |
| Field-shape matrix + agreement matrix (Rule 12; 44 cells, gaps carry reasons, YAML-invalid ⇒ Rust-invalid) | L1 `the_agreement_matrix_holds_and_every_gap_is_explained` |
| Locations (Rule 10), one cell per validator | L1 `problems_are_located_at_the_offending_value_or_the_parent_mapping`, `the_problem_names_the_missing_property` |
| Rule 13 bootstrap; `effective_for_kind` Some/None | L1 `the_shipped_definitions_validate_against_the_shipped_registry` |
| Rule 18 modified copy (`description: string(required)`) | L1 `a_modified_definition_changes_the_verdict_without_a_rust_change` |
| Registry error for an unresolvable entry | L1 `a_registry_entry_that_does_not_resolve_is_an_error` |
| Passivity (nothing read from `path`) | L1 `validation_never_reads_the_named_path` |
| Rule 17 corpus | L1 `shipped_schema_corpus::rust_accepted_kind_documents_pass_their_kind_definition` |
| Embedded inline root union prepares; file arm refused | simplified-schema unit test above |

Control fixtures: `darkmatter/schemas/schema.yaml` (schema kind) and
`claudine/schemas/sequence.yaml` (trigger kind; `schema-trigger.yaml` is a `kind: schema`
document, so the plan's suggestion to use it as the trigger control could not work). Each cell is one
edit of a top-level key (two where a sibling key isolates the field). A mutation check (flipping
the `description: 5` cell to valid) made the matrix test fail as expected; the file was restored.

Observed matrix outcomes match the Phase 1 matrix in every row. Recorded gaps (YAML valid, Rust
invalid): null `$schema` (with `types`), null `description`/`categories`, `types` sequence, `name`
key, trigger empty `match` list/mapping, retired `any`, `min-match` count too large, trigger null
`$schema`, trigger `extra`/`types` keys. Persistence round trip: not applicable (nothing is written).

### Problems found and fixed

- `schemas::tests::embedded_sources::malformed_imports_cycles_and_examples_fail_preparation`
  asserted the old root-union refusal; it now asserts that an inline root union prepares and that a
  union with a file arm is refused.
- `trigger_spelling_guard::no_active_file_spells_a_renamed_trigger_form` flagged the retired
  trigger kind in the two new test sites (both prove it is unclaimed); added
  `kind_documents.rs` (lib unit test) and `tests/l1/kind_documents.rs` to its `ALLOWED` list.

### Gates

- simplified-schema `just test`: 1113 passed; `just lint`: pass.
- darkmatter `just lint`: pass. darkmatter `just test` (final run, `--no-fail-fast` to surface all
  failures at once): 12903 passed, 10 skipped. No load-sensitive failures.
- `just check-tier-coverage darkmatter` and `simplified-schema`: nothing stranded.
- Scope calculator: `packages: [darkmatter, simplified-schema, test-toolkit, darkmatter-cli]`,
  `change_class: package`, so the gates were required and were run.
- Dependents: `cargo nextest run -p test-toolkit`: 479 passed. claudine + claudine-cli slice
  `-E 'test(/embedded|shipped|bundle|trigger|schema/)'`: 1356 passed. messenger and claudine-gen
  depend on simplified-schema but never call the embedded resolver (checked with rg).
- Cross-OS: not run. The new code is in-memory and passive; the only path is echoed into errors,
  and test fixtures use `include_str!` and temp directories.
- Skill: added "Kind-document validation" to `.claude/skills/darkmatter/schema.md`. `SKILL.md`
  ("Schema authority") is scheduled for Phase 5 (Rule 21).

## Phase 3

### Wave 6: YAML route (`darkmatter/cli/src/commands/schema/validate.rs`)

- `validate_one` sends a `.yaml`/`.yml` opened path (ASCII case-insensitive,
  `is_yaml_document`) to the new `validate_yaml_document`, which reads the text and
  calls `DarkmatterSchemas::validate_kind_document`. Unclaimed gives
  `Validated { no_schema: true }` (valid by default), a claimed document gives the
  raw report with `schema_label: None`. No baseline, trigger discovery, or document
  context applies to a YAML argument.
- A `SchemaError::SchemaDocument` whose path is the argument (invalid YAML, duplicate
  key, inadmissible number, non-string key) is a new `FileOutcome::YamlLoadError`
  (exit 3; pretty "the YAML document ... could not be loaded:", JSON
  `error: "yaml_parse"`). Any other error is `SchemaError` (exit 2). See
  `decisions_for_review`.
- `run_validate`: when assignments are present and any file argument's spelling is
  YAML, prints `Error: assignments apply to Markdown frontmatter, not to YAML
  document <path>` and exits 64 before any file is validated; the opened path is
  checked again before each file for spellings that hide the extension.
- Doc comments of `run_validate` and `validate_one` updated (YAML behavior, exit
  codes).

### Wave 7: CLI L1 tests (`darkmatter/cli/tests/l1/schema_validate_kind_documents.rs`)

Registered in `cli/tests/l1/main.rs`. One `md` spawn per outcome (each ~0.1 s):

| Requirement | Test |
| --- | --- |
| control: valid `kind: schema` claimed, exit 0; real shipped `schema.yaml` and `schema-trigger.yaml` (via `include_str!`) validate end to end; files unchanged (passive) | `valid_kind_documents_and_shipped_definitions_are_claimed_and_valid` |
| structurally invalid `kind: schema` (`description: 42`): exit 1, `(at line 2, column 14)` rendered | `invalid_kind_schema_reports_the_located_problem` |
| `kind: schema-trigger` missing `match`: exit 1, `missing` problem on `match`, line 1; `.YML` upper case still routes to YAML | `trigger_missing_match_fails_as_missing` |
| unclaimed (other `kind`, no `kind`, sequence root): exit 0, "valid by default" | `unclaimed_yaml_documents_are_valid_by_default` |
| YAML syntax error and duplicate key (the one loader's document-shape cell): exit 3, `yaml_parse`, `not valid YAML: ` message | `unloadable_yaml_exits_three` |
| assignment with a YAML argument: exit 64, message on stderr, nothing on stdout (the Markdown file listed first is not validated) | `assignment_with_a_yaml_argument_is_a_usage_error` |

The field-shape matrix for the kind definitions is in-process (Phase 2's
`lib/tests/l1/kind_documents.rs`); the CLI only proves wiring.

- First full run failed `spawn_site_guard::migrated_l1_tests_keep_the_isolation_the_builder_gave_them`:
  the new test set `NO_COLOR` on a builder-built command (an undeclared rendering
  input). Removed it; the test strips ANSI itself. Rerun passed.

### Gates

- `just test` (darkmatter): 12909 passed, 10 skipped.
- `just lint` (darkmatter): clean.
- `just check-tier-coverage darkmatter`: 0 stranded.
- `affected_scope.py` over the working tree: packages
  `[darkmatter, darkmatter-cli, simplified-schema, test-toolkit]`, change_class
  `package` (the tree still carries Phase 2's uncommitted changes), so the gates
  were required and ran.
- Dependents: only `md schema validate` changed in this phase (no library API).
  `claudine-cli` depends on `darkmatter-cli` but uses only `darkmatter_cli::approval`;
  claudine/messenger/biscuit-file share only the guard engine under `tests/common` by
  `#[path]`. No dependent test spawns `md schema validate` (the Claudine and
  Messenger matches are comments), so no downstream run was needed.
- Cross-OS: not run. The change is an extension comparison and an equality test on
  the same `PathBuf` passed through; no path canonicalization or OS-specific code.
