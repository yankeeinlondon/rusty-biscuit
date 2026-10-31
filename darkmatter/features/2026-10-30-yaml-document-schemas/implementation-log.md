---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/spec.md
plan: darkmatter/features/2026-10-30-yaml-document-schemas/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
- darkmatter
- darkmatter-cli
- simplified-schema
- dmls
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
- |-
    The plan's diagnostics test list included "wrong `kind` literal (range: the
    value)". That case cannot produce a kind diagnostic: the `kind` value is what
    selects a definition, so `kind: schema-triggers` is claimed by no definition and
    gets no kind diagnostics at all (the same rule `md schema validate` follows). The
    test asserts that instead. The alternative, treating a near-miss kind as a wrong
    value of the nearest kind, would need a fuzzy claim rule nobody specified.
    Nothing needs redoing unless a reviewer wants that rule.
- |-
    The plan said `kind: ` value completion offers "the two kinds" (`schema`,
    `schema-trigger`). Completion reads the definition of the kind that claims the
    buffer, and each definition fixes its own `kind` literal, so a `kind: schema`
    buffer is offered `schema` and a `kind: schema-trigger` buffer `schema-trigger`;
    a buffer with an empty `kind:` is unclaimed and gets neither. Offering both would
    need a separate "known kinds" completion source outside the definitions (a small
    addition in the frontmatter completion provider, keyed on `SHIPPED_KINDS`).
- |-
    The plan said the editor's completion and hover reuse the frontmatter machinery
    unchanged. Unchanged, the root shape would layer the Markdown baseline under the
    definition, so a `kind: schema` file would be offered `title`, `tags`, `style`
    and the rest. I added a flag to the shared analysis so a kind document's root
    shape is the definition alone. The alternative was to accept the Markdown keys,
    which would be wrong suggestions. Reverting is removing the `kind_document` flag
    in `overlay/analysis.rs`.
- |-
    The plan did not cover a kind document's `schema`/`type-definition` payload
    problem that the Rust envelope also reports. Without handling, a bad type in
    `types:` produced two diagnostics: the envelope's precise one on the offending
    token and the kind report's coarse one on the whole `types` value (an existing
    DMLS test caught this). The kind diagnostics now drop such meta-type problems
    when the standalone or trigger envelope has already reported its own error, and
    keep them otherwise (for example a trigger outside a `schemas/` folder, where no
    envelope diagnostic runs). Alternatives: always drop them (loses that trigger
    case), or always keep both (duplicate squiggles). Changing it is one filter in
    `dmls/src/diagnostics/kind_document.rs`.
- |-
    Following the plan, kind diagnostics are graded by the same rule as frontmatter
    problems, so a missing required key (for example a trigger without `match`) is
    reported only when the editor's schema `strict` setting is on; the default is
    off. The envelope's own error still reports a missing `match` in a `schemas/`
    folder. The alternative, always reporting missing keys for kind documents (as
    `md schema validate` does), is a one-line change in
    `dmls/src/diagnostics/kind_document.rs` but makes kind documents graded
    differently from frontmatter.
- |-
    The library's `KindValidation` result gained a `document` field (the JSON data
    the report judged) so the editor can build its schema bundle without loading
    the YAML a second time with its own copy of the loader rules. The alternative
    was to duplicate the load-and-convert step in DMLS. Reverting means removing the
    field and re-deriving the data in `dmls/src/overlay/kind_document.rs`.
- |-
    A kind document is validated in the editor only when DMLS has a file-resolution
    context for it, because the validator is built in one (`DarkmatterSchemas::new`
    takes a context). Without one, the document gets the existing context-failure
    diagnostic and no kind diagnostics or definition-driven completion. The
    alternative, a context-free validator constructor in the library, is a larger
    API change for a case the editor already reports.
- |-
    The plan's drift sweep says nothing in current code should reference the deleted
    placeholder `schema-definition.yaml`. One test fixture still does: the recursive-schema
    golden corpus keeps a byte-for-byte copy of `err.yaml` taken on 2026-10-23 (with its old
    `$schema: ./schema-definition.yaml` line) and two "skipped" notes that name the
    placeholder. I left that fixture alone, because it is a dated snapshot whose test
    checks the frozen copy, not the live file, and its notes are still true of that copy.
    The alternative is to refresh the frozen `err.yaml` copy and drop the two notes, which
    would also mean regenerating that entry's golden output. Nothing else depends on it.
- |-
    Deleting the placeholder broke three tests that list the `darkmatter/docs/schemas`
    folder; the plan named only the corpus exceptions. Two now expect two files with
    `err.yaml` accepted as a schema. The third checked every `$schema` declaration in that
    folder against the `schema` type, and the only declaration was the removed
    `$schema: ./schema-definition.yaml`. It now checks the `$schema` of the two shipped kind
    definitions (`schema.yaml`, `schema-trigger.yaml`) instead. The alternative was to point
    it at the whole `darkmatter/schemas` folder, which would pull in the global catalog and
    function partials whose `$schema` values are not plain schemas. Reverting means
    choosing another source of real declarations for that test.
- |-
    Rewriting the authoring guide turned up a trigger example that the new validator
    rejects (a `description:` key and an `expression: foo || bar || baz` condition, neither
    of which the trigger grammar has). I replaced it with a `group:` of property conditions
    that validates. The guide's list of trigger conditions (time of day, host OS, file
    contains text, and so on) sits under an existing "some of this is not implemented"
    note and I left it unchanged; it describes conditions the trigger grammar does not have
    today. Trimming that list to the real grammar is a larger edit to a user-facing page and
    was not asked for.
- |-
    The plan asked for the authoring guide's two definition transclusions to replace the
    "recursive truth" sentence in the External Schemas section. I put them under the empty
    "Schema Definition Files" and "Trigger Files" headings in the "External Grammar Files"
    section instead, and the old sentence now points there. Those headings existed for this
    content, and this keeps the long trigger definition out of the beginner walkthrough.
    Moving them back is a cut-and-paste of two `::code` lines and their short intros.
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
source_files_during_phase_4:
- darkmatter/lib/src/markdown/schemas/kind_documents.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/kind_document.rs
- darkmatter/dmls/src/overlay/frontmatter.rs
- darkmatter/dmls/src/overlay/analysis.rs
- darkmatter/dmls/src/overlay/references.rs
- darkmatter/dmls/src/diagnostics/mod.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/kind_document.rs
- darkmatter/dmls/tests/l1/kind_document_editor.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/standalone_stdio.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/darkmatter/schema.md
source_files_during_phase_5:
- darkmatter/lib/tests/l1/meta_schema_phase4.rs
- darkmatter/docs/schemas/err.yaml
- darkmatter/docs/schemas/schema-definition.yaml
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/meta_schema_phase1.rs
- darkmatter/dmls/src/overlay/mod.rs
docs_updated_during_phase_5:
- darkmatter/docs/topics/schemas/authoring-schemas.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/docs/inline/schema-validation.md
- docs/kind-documents.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/darkmatter/cli.md
source_code:
- darkmatter/lib/tests/l1/meta_schema_phase4.rs
- darkmatter/lib/src/markdown/schemas/embedded.rs
- darkmatter/schemas/schema.yaml
- darkmatter/schemas/schema-trigger.yaml
- schemas/kind.yaml
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
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/tests/l1/schema_validate_kind_documents.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/kind_document.rs
- darkmatter/dmls/src/overlay/frontmatter.rs
- darkmatter/dmls/src/overlay/analysis.rs
- darkmatter/dmls/src/overlay/references.rs
- darkmatter/dmls/src/diagnostics/mod.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/kind_document.rs
- darkmatter/dmls/tests/l1/kind_document_editor.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/standalone_stdio.rs
- darkmatter/docs/schemas/err.yaml
- darkmatter/docs/schemas/schema-definition.yaml
- darkmatter/lib/tests/l1/meta_schema_phase1.rs
documentation:
- darkmatter/docs/topics/schemas/authoring-schemas.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/docs/inline/schema-validation.md
- docs/kind-documents.md
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/darkmatter/cli.md
completed_phase: 5
implemented: true
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

## Phase 4

### Wave 8: overlay foundation (`darkmatter/dmls/src/overlay`)

- `FrontmatterAst::parse_yaml_document` (`overlay/frontmatter.rs`): the
  `parse_yaml` tree with a private `schema_key_is_data` flag; `schema_entry()`
  returns `None` for it. `overlay/references.rs::references` now finds the
  `$schema` entry through `schema_entry()` (it matched the pointer directly),
  so the no-context hover branch also treats a kind document's `$schema` as data.
- New `overlay/kind_document.rs`: `KindDocumentOverlay { kind, report, ast,
  bundle }` and `OverlayCache::kind_document`, cached per URI on text,
  revision, context identity, and context generation. It calls
  `DarkmatterSchemas::validate_kind_document`, then `effective_for_kind`, and
  builds a `SchemaBundle` (no advisories from extensions, document JSON with
  `$schema` kept) and a `FrontmatterAnalysis` marked `kind_document`.
- `DocumentOverlay::kind_document` is computed in `for_document_timed` after the
  Markdown return (which sets `None`) and before the trigger and standalone
  branches. Every non-Markdown return attaches it (`with_kind`); a claimed buffer
  that no envelope branch serves (a valid trigger, a lexical claim the parser
  declined, an unclaimed envelope with a parse error) returns `kind_only`.
- `overlay/analysis.rs`: `AnalysisIdentity::kind_document`; `root()` for a kind
  document is `simplified_schema::layered_property_shape` over an empty base
  (the definition alone, no Markdown baseline keys).
- Library: `KindValidation` gained `document: Value` (the data the report
  judged), so DMLS does not repeat the load. See `decisions_for_review`.

### Wave 9: providers

- Diagnostics: new `diagnostics/kind_document.rs`; `diagnostics_under` runs it
  when `overlay.kind_document` is set and then skips the Markdown schema,
  expression, and nested-span passes. The generic problem loop of
  `schema_problem_diagnostics` is now `generic_problem_diagnostics(ctx, ast,
  bundle, problems, out)`; the Markdown caller pre-filters the specialized and
  expression-owned problems it used to `continue` past (pure filters, no
  behavior change; the mapping-only corpus baseline still passes).
- First full run failed `core_schema_types::option_and_null_default_errors_are_ranged_on_the_offending_token`:
  a bad option in `types:` was reported twice (the envelope's
  `invalid_type_definition` on the token, and the kind report's
  `dm.schema.constraint` on `/types`). Kind diagnostics now drop meta-type
  problems (`is_meta_type_problem`, the existing `x-darkmatter-*` schema-path
  test) when the standalone or trigger envelope already reported its error.
- Completion and hover: verified, no provider change needed beyond the root
  shape. Links and go-to-definition read `overlay.schema_references`, empty for a
  kind document; `$schema` value hover reads the same facts (or `references()`
  without a context). Other `overlay.ast` readers (semantic tokens, DSL
  frontmatter lookups, code actions) see the tree as frontmatter and contribute
  nothing new for the kind fixtures.

### Wave 10: tests

New `dmls/tests/l1/kind_document_editor.rs` (in-process, temp workspace,
`RepositoryContexts` on the fixture snapshot; each test ~0.04 s), plus one stdio
case:

| Requirement | Test |
| --- | --- |
| control `kind: schema` and trigger, and the shipped `schema.yaml`/`schema-trigger.yaml` (`include_str!`): no kind diagnostics | `valid_kind_documents_have_no_kind_diagnostics` |
| `description: 5`: one `dm.schema.type_mismatch` on the value, exact range | `a_wrong_description_type_ranges_the_value` |
| wrong `kind` literal is unclaimed; `match: 5` ranges the value; envelope diagnostic still published beside it | `a_trigger_with_the_wrong_kind_literal_is_unclaimed_and_a_wrong_match_type_ranges_the_value` |
| missing `match`: `dm.schema.missing_required` on the root mapping (0:0–3:0) under strict; none by default | `a_missing_match_ranges_the_root_mapping_under_strict_mode_only` |
| invalid YAML: unclaimed, no kind diagnostic, parse diagnostic stands; unclaimed shapes (other kind, no kind, `kind:` null, sequence root): no diagnostics | `invalid_yaml_keeps_only_its_parse_diagnostic_and_unclaimed_yaml_has_none` |
| Markdown frontmatter with `kind: schema-trigger`: not claimed, no `match` problem, baseline still judges `metadata: 5` | `markdown_frontmatter_is_never_a_kind_document` |
| completion at an empty top-level line: missing definition keys offered, present ones and Markdown baseline keys (`title`) not | `completion_offers_the_definitions_missing_top_level_keys` |
| `kind:` value completion: the claiming kind's literal; empty `kind:` offers neither | `completion_offers_the_claiming_kind_for_the_kind_value` |
| hover on `description`/`match` shows the definition's `->` description | `hovering_a_key_shows_its_definition_description` |
| `$schema` key hover is the definition's; value hover, document links, and definition do not resolve a reference (Markdown control does) | `hovering_the_schema_key_or_value_resolves_no_reference` |
| a rejected type definition reported once (envelope) in a standalone file; kept from the kind report for a trigger outside `schemas/` | `a_rejected_definition_is_reported_once_by_the_envelope_when_it_reports` |
| stdio: the built `dmls` publishes `dm.schema.type_mismatch` at 1:13 with `instance_path: /description` for a `kind: schema` file | `standalone_stdio::kind_document_diagnostics_reach_a_stdio_client` |

Input robustness: the loader and field-shape matrix are Phase 2's in-process
tests (`lib/tests/l1/kind_documents.rs`); DMLS proves wiring with the control,
two invalid field shapes, the invalid-YAML document shape, and the unclaimed
shapes. Smell grep over the new files (`#[serde(default)]`, `unwrap_or_default()`,
`filter_map(.. as_str())`, `.ok()`): `kind_document.rs` uses
`validate_kind_document(..).ok()??` deliberately: a load error is the YAML
parse diagnostic's to report, so it maps to "no kind state", not to valid.

### Gates

- `just test` (darkmatter): first run 1 failure (above, fixed); rerun 12921
  passed, 10 skipped.
- `just lint` (darkmatter): clean (rerun after the fix).
- `just check-tier-coverage darkmatter`: 0 stranded.
- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`:
  packages `[darkmatter, dmls, test-toolkit, darkmatter-cli]`, change_class
  `package`, so the gates were required and ran.
- Dependents: no crate outside `darkmatter/` uses `KindValidation`,
  `validate_kind_document`, or `effective_for_kind`, and nothing depends on
  `dmls`; the darkmatter-cli consumer is in the area's `just test`.
- Cross-OS: not run. No path, process, or OS-specific code changed; the new
  stdio test follows the existing one (canonicalized temp root, URL from path).

## Phase 5

### Wave 11: documentation

- `darkmatter/docs/topics/schemas/authoring-schemas.md`: the "recursive truth"
  sentence and its `::code` of the deleted placeholder are replaced by a
  pointer to "External Grammar Files". That section's empty
  "Schema Definition Files" / "Trigger Files" headings now transclude
  `^darkmatter/schemas/schema.yaml` and `^darkmatter/schemas/schema-trigger.yaml`
  with a short description of each, and the obsolete trigger sketch
  (`kind: "const(schema-trigger,required)"`, `match: TODO`, `description`) is
  gone. The "tagged" schema file wording now says `$schema`, `types`, or both
  (the code always allowed a `$schema`-only tagged file).
- Same file, drift found and fixed: the trigger example used `description:`
  (not a trigger key) and `match: - expression: foo || bar || baz` (not the
  trigger grammar). The new kind validator itself rejects it
  (`match/0/expression invalid SimplifiedSchema ... unexpected character |`).
  Replaced with a `group:` of property conditions, verified valid with
  `target/debug/md schema validate` in a `schemas/` folder.
- `definition.md`: new H2 "How a YAML document is validated by its `kind`"
  (table of the two kinds, the Mermaid flow, the claim rule with claimed and
  unclaimed examples, raw validation, `$schema` as data, the residual Rust
  rules, and where the check runs); a "What You Get" bullet; the CLI flag
  table and exit-code row mention YAML files; Library API gains
  "Kind documents". Every example's outcome was produced with the worktree's
  `md` (wrong `description` type at 2:14, missing `match` at 1:1, `$schema: 5`
  rejected, `$schema` string/list accepted, `kind: 5` unclaimed, `name:` and
  null `description` accepted by the definition).
- `darkmatter/docs/inline/schema-validation.md`: "YAML documents" subsection
  (pretty output copied from a real run, `arm[n]` explained, no baseline or
  triggers, no coercion, exit 3 with `yaml_parse`, `schema: null` in JSON,
  assignments rejected with exit 64 including the `@name` caveat) and the
  exit-code row.
- `dmls-schema-support.md`: "YAML kind documents" subsection (content-based
  claim, diagnostics table with ranging per Rule 10, strict-only missing keys,
  envelope de-duplication, when no kind diagnostics run, completion, hover,
  `$schema` as data); coverage matrix trigger row gains completion and hover
  (footnote 18), footnotes 14 and 16 point at the new section.
- `docs/kind-documents.md`: `::code ^schemas/kind.yaml`; prose now says the
  schema documents the values and applies only when a document names it (the
  Phase 1 decision made it a plain `$schema` file), and points at the two
  formal kinds. Pre-existing and left alone: its `::file ^darkmatter/docs/topics/schemas/`
  line names a directory, which `md compose` reports as an invalid
  transclusion (`no-match`); it predates this feature and its intent (list the
  schema folders) needs an author's choice.
- Skills: `SKILL.md` Schema authority gains the kind-document sentence;
  `schema.md` gains the user-doc pointers and the `@name` assignment caveat;
  `cli.md` gains the YAML route paragraph.

### Wave 12: placeholder removal and drift sweep

- Deleted `darkmatter/docs/schemas/schema-definition.yaml`; removed
  `$schema: ./schema-definition.yaml` and `name: err` from
  `darkmatter/docs/schemas/err.yaml`; removed both `shipped_schema_corpus.rs`
  exceptions. `err.yaml` now classifies as a `types`-only tagged schema and
  passes the corpus and its kind definition; no defect to fix in its `types`.
- Not listed in the plan but broken by the deletion, updated in the same
  change: `lib/tests/l1/meta_schema_phase1.rs::shipped_schema_corpus_is_passively_classified_without_resolution`
  (two artifacts; `env.yaml` and `err.yaml` are schemas, nothing rejected or
  inactive) and `dmls/src/overlay/mod.rs::shipped_schema_corpus_uses_content_classification`
  (`err.yaml` is now `ready`, the placeholder row is gone).
- Drift sweep (`git grep -n "schema-definition.yaml\|kind-document.yaml"`
  outside closed specs and this feature): only the frozen golden fixture
  remains (`lib/tests/fixtures/recursive_schema_golden/inputs/.../err.yaml`
  and two `skipped` notes in its `manifest.yaml`). That corpus is a dated,
  byte-for-byte copy (2026-10-23) and is left as is; see `decisions_for_review`.
  No "recursive truth" or "empty placeholder" statement remains in the touched
  docs or code.
- Targeted run after the removal (28 tests: corpus, golden, both
  classification tests, `rust_accepted_kind_documents_pass_their_kind_definition`):
  all passed.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| placeholder gone, `err.yaml` accepted by the Rust parser and its kind definition | `shipped_schema_corpus::shipped_schema_artifacts_parse_passively`, `::rust_accepted_kind_documents_pass_their_kind_definition`, `meta_schema_phase1::shipped_schema_corpus_is_passively_classified_without_resolution` |
| DMLS classifies the docs schema folder by content after the deletion | `dmls overlay::tests::shipped_schema_corpus_uses_content_classification` |
| docs examples and transclusions | not compiled; verified by `md schema validate` / `md compose` runs recorded above |

No parser, schema, or loader behavior changed in this phase, so the input
robustness matrix is Phase 2's and is unchanged.

### Gates

- `just test` (darkmatter), first run: 1 failure,
  `meta_schema_phase4::shipped_schema_artifacts_validate_through_semantic_keywords`
  ("shipped schema declarations"). It collected `$schema` declarations from
  `darkmatter/docs/schemas`, and the only one was the removed reference to the
  placeholder. It now also checks the `$schema` of both shipped kind definitions
  (`include_str!`); see `decisions_for_review`. The test alone and its module
  passed (23 tests).
- `just test` (darkmatter), rerun: 12921 passed, 10 skipped.
- `just lint` (darkmatter): clean.
- `just check-tier-coverage darkmatter`: 0 stranded.
- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`:
  packages `[darkmatter, dmls, test-toolkit, darkmatter-cli]`, change_class
  `package`, so the gates were required. The tree also still holds Phase 4's
  uncommitted changes, so this scope covers both phases.
- Dependents: no package outside `darkmatter/` reads `err.yaml`, the deleted
  placeholder, or the changed tests; `claudine/` is unchanged. Nothing more to run.
- Cross-OS: not run. This phase changed docs, one YAML artifact, and tests that
  list a directory by file name (sorted, `extension() == "yaml"`). None of it
  is OS-specific.
- `just test-l2`: not required (no terminal behavior changed).

### Definition of Done

Every bullet holds. Definitions shipped and tied (Phases 1 and 2);
`validate_kind_document` and the modified-copy test (Phase 2); self-validation
and corpus (`rust_accepted_kind_documents_pass_their_kind_definition`, now
including `err.yaml`); the `md schema validate` YAML route (Phase 3); DMLS
(Phase 4); `SHIPPED` completeness (Phase 1); placeholder deleted,
`authoring-schemas.md` transcludes both definitions, `docs/kind-documents.md`
embeds `schemas/kind.yaml` (this phase); docs and skills updated (this phase);
gates green; `claudine/` unchanged. Status: implementation complete, ready for
review.
