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
    - |-
      Trigger files named by a `$schema` or a trigger payload. The plan's loader restriction makes the
      loader refuse a `kind: schema-trigger` file (it has `$schema` beside `kind`/`match`) before the
      existing post-resolution checks ran, so users lost the specific "document `$schema` referenced a
      schema-trigger file ... reference the payload `x.yaml` instead" error and the trigger-cycle error;
      the plan did not mention this. Choice: keep the restriction and translate a loader refusal of a
      file that claims the trigger envelope back into the old errors (`refused_trigger_file` in
      `darkmatter/lib/src/markdown/schemas/triggers/assemble.rs`, used by document `$schema` resolution
      and trigger payload resolution); every existing trigger test passes with its original assertion.
      Alternatives: accept the generic "unsupported top-level key(s): kind, match" message (worse for
      users, simpler), or teach the simplified-schema loader about triggers (wrong layer: the library
      has no trigger concept). Redo if rejected: delete the two `map_err` calls and
      `refused_trigger_file`/`payload_cycle`, and update ten trigger tests to the generic error.
    - |-
      The global catalog tests. The plan removes the parser's "global catalog" refusal and says the
      catalogs are unaffected because they load through their own loader. Two tests asserted that a
      document `$schema` naming a catalog fails (`darkmatter/lib/tests/l1/global_catalog.rs`,
      `claudine/lib/tests/l1/host_catalog.rs`). Choice: rewrite them to the new behavior: named by
      `$schema`, a catalog is an ordinary tagged schema whose globals become document properties, and
      only registration makes it the baseline/globals (both tests still prove the catalog is never a
      trigger). Alternative: keep a catalog-specific refusal in Darkmatter's `$schema` path (adds a rule
      the spec removed). Redo if rejected: add a check in `DarkmatterSchemas::effective_for` using
      `catalog::is_global_catalog` and restore the two tests' old assertions.
    - |-
      Named-type import of a non-UTF-8 file. The import loader used to report undecodable bytes as the
      old ambiguous error; with the rename there was no fitting reason (it is not a "not a mapping").
      Choice: report `SchemaDocument` "not UTF-8 text", the wording the `$schema` loader already uses.
      Alternative: `NotASchemaDocument { NotAMapping }`. Redo if rejected: one `map_err` in
      `read_named_types` (`simplified-schema/lib/src/resolve.rs`) and the test
      `schemas_named_import_errors::an_import_target_that_is_not_utf8_is_a_schema_document_error`.
    - |-
      Where the classifier tests live. The plan named `schema_files.rs` and `located_schema_errors.rs`
      (which test the `SimplifiedSchema` factories and located grammar failures). Choice: a new file
      `simplified-schema/lib/tests/l1/schema_target_classification.rs`, because its subject is the
      `$schema` loader; the `md schema validate` tests likewise went in a new
      `darkmatter/cli/tests/l1/schema_target_recognition.rs`. Redo if rejected: move the test functions;
      nothing depends on the location.
    - |-
      Raw-YAML JSON Schema golden and matrix coverage. Three raw-YAML matrices in
      `schema_enhancements_matrix/raw_schema.rs` and two raw-YAML golden inputs (with their goldens)
      under `darkmatter/lib/tests/fixtures/recursive_schema_golden/` tested a format that is no longer a
      `$schema` target. Choice: collapse the matrices into one rejection test and delete the two golden
      inputs and goldens, because their `.json` twins keep every cell and were byte-identical. Redo if
      rejected: restore the deleted files from git and point them at `SimplifiedSchema::from_file`
      (which still accepts raw YAML with a dialect URI) instead of the `$schema` loader.
    - |-
      Shipped `types`-only files in the corpus test. `darkmatter/schemas/partials/*`,
      `darkmatter/docs/schemas/env.yaml`, Claudine's `action`/`linking`/`lifecycle`/`sequence-types`
      schemas, `content-policy.yaml`, and research `_types.yaml` files are `types`-only libraries the
      corpus test used to load through `$schema`. No document names them by `$schema`, so the plan's
      audit was right that no shipped file needs migrating. Choice: the corpus test derives a
      `TypesOnly` class from content (refused as `$schema`, every non-function type imported and
      lowered) instead of listing each file as an exception. Redo if rejected: add `Exception` entries
      per file in `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`.
    - |-
      Phase 3 schema reference facts carry no `target` (preview) field yet. The plan's struct lists
      `target: Option<SchemaTargetPreview>` with "None in Phase 3". Choice: leave the field and its type
      out until Phase 4 builds the preview, instead of adding a placeholder type that nothing fills.
      Alternative: an empty placeholder type now. Redo if rejected: none; Phase 4 adds the field to
      `SchemaReferenceFact` in `darkmatter/dmls/src/overlay/references.rs`.
    - |-
      The facts parse the frontmatter a second time on a schema-cache miss (`SchemaReferenceFacts::select`
      calls `FrontmatterAst::parse` on the same text `schema::assemble` reads), instead of borrowing the
      analysis tree, which is built after `schema_for` returns. The parse happens only when the schema is
      reassembled, never on a cache hit. Alternative: thread the tree into `schema_for` (a larger
      reshuffle of `for_document_timed`). Redo if rejected: pass the parsed AST into `schema_for`.
    - |-
      Brackets in file URIs. The parity tests found that a link or definition whose target sits in a
      folder whose name has `[` or `]` was silently dropped: `url::Url::from_file_path` leaves `[`, `]`,
      `^`, `|`, and `\` raw, and `lsp_types::Uri` refuses them. The plan asks that links be correct for
      folder names with Markdown punctuation, so I fixed `dmls::workspace::file_path_to_uri` to
      percent-encode those characters in the path (it is shared by every DMLS link, definition, and
      location, so all of them gain the fix). Alternative: leave it for a separate fix and keep targets
      out of bracketed folders in tests. Redo if rejected: revert the loop in `file_path_to_uri` and the
      test `a_target_in_a_bracketed_folder_has_a_link_and_definition` (which would then fail).
    - |-
      Two docs rows were corrected in this phase although Phase 6 owns the full docs pass, because they
      became false here: the `dm.schema.invalid_file_reference` row in
      `darkmatter/docs/topics/schemas/dmls-schema-support.md` and the failure-class table in
      `darkmatter/dmls/docs/file-references.md` (a missing file is no longer an editor diagnostic), plus
      the navigation bullet (the `$schema` link falls back to the creation path). Phase 6 should extend,
      not redo, them.
    - |-
      Where the two value-hover modules live. The plan named `dmls/src/providers/schema_value_hover.rs`
      and `dmls/src/providers/file_value_hover.rs`. Choice: they are child modules of the frontmatter
      provider (`dmls/src/providers/frontmatter/schema_value_hover.rs` and `.../file_value_hover.rs`), like
      the existing `frontmatter/named_references.rs`, because they use that provider's private helpers
      (the hover-range builder, the typed-scalar lookup). The pure wording module is where the plan said,
      `dmls/src/providers/reference_hover.rs`. Alternative: the plan's paths, with those helpers made
      crate-visible. Redo if rejected: move two files and widen two helpers; no behavior changes.
    - |-
      First-line indentation of a YAML preview. The plan's rule gives the first line of the `$schema`
      value the column it starts at, always. Choice: that column counts only when nothing but spaces
      precede the value on its line (a block mapping or list under the key, the normal shape); a value
      written after the key on the same line (a flow mapping or a single reference) starts flush and its
      continuation lines keep their indentation. The two rules give the same text for every block value
      and every one-line value; they differ only for a flow mapping that spans several lines, where the
      plan's rule would indent the first line by the width of `$schema: `. Redo if rejected: one line in
      `simplified-schema/lib/src/resolve/inspect.rs::yaml_definition_lines`.
    - |-
      Telling "no schema root exists" apart from an explicit miss. The selector's `Missing { bare_name }`
      is false both for `./x.yaml` and for a bare name looked up with no schema root, but the hover
      words them differently. Choice: `simplified_schema::resolve::is_bare_name` is now public and the
      hover asks it. Alternatives: a new field on `SelectionOutcome::Missing`, or a flag on the overlay
      fact; both add state the selector already implies. Redo if rejected: add the field and drop the
      export.
    - |-
      Unreadable targets. The spec asks that an unreadable file keep its own reason and path. That holds
      for a target outside the schema roots (for example `./locked.yaml`): the hover says the file
      "could not be used as a schema" with the I/O error and links it. A file in a schema root is read
      first by the trigger-registry scan, which fails, so the hover says the schema roots could not be
      read (no file is linked, no root guessed), exactly what diagnostics already report. Alternative:
      make the registry scan skip unreadable files, a behavior change to trigger discovery outside this
      fix. Nothing to redo unless the registry policy changes.
    - |-
      A `file` value in a document with no file-resolution context. Without a context DMLS assembles no
      document schema (not even an inline one), and the built-in baseline has no `file`-typed property,
      so no value is `file`-typed and the "no file-resolution context" sentence cannot appear on a `file`
      value today; it does appear on a `$schema` value. The `file` hover keeps the branch for a future
      file-typed baseline property, and a test pins that such a value gets no file hover. Redo if
      rejected: delete one match arm.
    - |-
      Smaller choices. (1) The `file` hover resolves with `FileReference::resolve_in_context` directly
      (what `ctx.resolve_reference` does internally) so an invalid reference found during resolution keeps
      its error text. (2) The selected file is read as bytes (`work_counters::read`), so a non-UTF-8 file
      gets the loader's own "not UTF-8 text" error. (3) A description is shown trimmed. (4) Range tests
      cover UTF-8 and UTF-16 only: the plan also named UTF-32, which DMLS does not offer. (5) Rule 14
      escapes backticks in a quoted reason, so a file-reference error's inline code shows as literal
      backticks in the hover. Each is a one-line change if a reviewer prefers otherwise.
    - |-
      Wiki-link hovers do not show the target's summary. The spec and plan describe the summary for
      "body links" and name only `providers/hover.rs::resolved_hover`; `[[wiki]]` links have their own
      hover (`providers/wiki.rs`). Choice: only Markdown body links gain the summary, as written.
      Alternative: append the same `DocumentRecord::summary` in the wiki hover too (a few lines plus
      tests); the summary is already on every parsed document record, so nothing captured would change.
    - |-
      "Unreadable target" in the new session tests is a target file holding invalid UTF-8, which the
      server's text read rejects on every OS. A permission-denied file was not used because Windows has
      no portable way for a test to make a file unreadable. Both paths go through the same failed-read
      branch, so a reviewer preferring a permission-based case would only add a Unix-only test.
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
    - claudine-cli
message_to_agent: |-
  Phase 5 is done: body-link hovers append the target's frontmatter summary. `just lint` and `just test` pass in darkmatter (see the Phase 5 log). Facts for Phase 6:
  - The summary is `DocumentIndex::summary`, read by `graph/substrate.rs::summary_text` (first nonempty, non-pending YAML string among `excerpt`, `summary`, `description`, `about`; trimmed; nothing stringified). It is stored on the per-snapshot `DocumentState` and exposed as `DocumentRecord::summary`; it is not in `FileFacts`, so `FACTS_VERSION` and `index_store_golden.txt` are unchanged.
  - `providers/hover.rs::resolved_hover` appends `\n\n{summary}` after the title, path, and `Heading:` line. Wiki-link hover (`providers/wiki.rs::resolved_hover`) was NOT changed: the spec and Rule 16 name only body links; see decisions_for_review.
  - Task 16.x owns the docs: `darkmatter/docs/lsp/features.md` still carries the MIME/image-preview claim and does not yet describe the summary.
source_files_during_phase_2:
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/schema_target_classification.rs
    - simplified-schema/lib/tests/l1/shipped_corpus.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase5.rs
    - darkmatter/lib/tests/l1/numeric_admission.rs
    - darkmatter/lib/tests/l1/raw_root_union_assertions.rs
    - darkmatter/lib/tests/l1/raw_schema_conversion.rs
    - darkmatter/lib/tests/l1/recursive_schema_golden.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
    - darkmatter/lib/tests/l1/schema_cache_validity.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/manifest.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/schema-enhancements-matrix-keywords-schema-json.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/schema-enhancements-matrix-detected-schema.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/schema-enhancements-matrix-keywords-schema.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.yaml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
    - darkmatter/cli/tests/l1/schema_target_recognition.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/tests/fixtures/suggest_constraint/tagged.yaml
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - claudine/lib/src/composition/completion/tests.rs
    - claudine/lib/src/composition/ownership/tests/setters.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/system_prompt/prepare/tests.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/ctx_launch_anchor.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
    - prompts/_implement/implement.yaml
docs_updated_during_phase_2:
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/language/schema-documents.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/design/research/r5-schema-error-range-mapping.md
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/simplified-schema/SKILL.md
source_files_during_phase_3:
    - darkmatter/dmls/src/overlay/references.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/workspace/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/schema_reference_parity.rs
    - darkmatter/dmls/tests/l1/schema_reference_sessions.rs
    - darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/schema_reference_navigation.json
    - darkmatter/cli/tests/l1/schema_validate.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/file-references.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_4:
    - simplified-schema/lib/src/resolve/inspect.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/select.rs
    - simplified-schema/lib/tests/l1/schema_target_inspection.rs
    - simplified-schema/lib/tests/l1/main.rs
    - darkmatter/dmls/src/overlay/references.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/schema_value_hover.rs
    - darkmatter/dmls/src/providers/frontmatter/file_value_hover.rs
    - darkmatter/dmls/src/providers/reference_hover.rs
    - darkmatter/dmls/src/providers/reference_hover/tests.rs
    - darkmatter/dmls/tests/l1/value_hovers.rs
    - darkmatter/dmls/tests/l1/schema_reference_sessions.rs
    - darkmatter/dmls/tests/l1/main.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_5:
    - darkmatter/dmls/src/graph/substrate.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/index_store/facts.rs
    - darkmatter/dmls/src/providers/hover.rs
    - darkmatter/dmls/tests/l1/index_store_facts.rs
    - darkmatter/dmls/tests/l1/referenced_targets.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
---
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


## Phase 2

### 3.1 Standalone parser (Rule 9)

- `simplified-schema/lib/src/simplified/standalone.rs`: `StandaloneSchemaDocument` gains
  `exports_schema` and `description`. The informal envelope accepts `$schema` plus optional
  `types` and `description`; the tagged envelope allows `kind`, `types`, `categories`, `$schema`,
  `description` and needs at least one of `$schema`/`types` ("tagged schema document needs a
  `$schema` or `types` property"). `description` must be a string in both (`""` kept as
  `Some("")`). The "global catalog" refusal is removed. `check_named_types` on the payload runs only
  when the payload key is `types`. A non-string top-level key now counts as unsupported in the
  tagged key check (it was counted only through the length comparison before).

### 3.2 Errors (Rule 7)

- `simplified-schema/lib/src/errors.rs`: `AmbiguousReferenced` renamed `NotASchemaDocument { path,
  reason: NotSchemaReason }`; new `UnsupportedJsonDialect { path, declared }` and
  `InvalidJsonSchema { path, message }`; `NotSchemaReason::hint(path)` gives the migration text
  (the Display appends it on a `hint:` line through `with_hint_line`). `DRAFT_2020_12_DIALECT` is
  public. `StatusBlock` rendering and unit tests added. Re-exported from `lib.rs`.
- Uses moved: `schema/layer.rs` (`NotAMapping`), embedded `Ok(None)` (`NoSchemaProperty`),
  `parse_raw_json_schema` non-object (`NotAMapping`), the named-type import loader's `Ok(None)`
  (`NoSchemaProperty`).
- The named-type import loader's UTF-8 failure now reports `SchemaDocument` "not UTF-8 text"
  (the `$schema` loader's wording) instead of the old ambiguous error; recorded as a decision.
- `darkmatter/lib/src/markdown/schemas/triggers/finding.rs` (`related_path`),
  `darkmatter/lib/src/markdown/schemas/mod.rs` (`with_baseline_from_file` pass-through), and
  `darkmatter/dmls/src/overlay/mod.rs::error_dependencies` name the three variants' `path`.
  `file_failures` is unchanged: it only stores located definition failures, which these variants
  never carry, so adding them would only read the file for nothing.
- Docs: `simplified-schema/docs/topics/diagnostics.md` variant table;
  `darkmatter/dmls/design/research/r5-schema-error-range-mapping.md` variant list.
- `grep -rn AmbiguousReferenced` finds nothing outside `_completed` and this fix's own files.

### 4.1 Referenced-file loader (Rules 5 and 8)

- `simplified-schema/lib/src/resolve.rs`: `load_schema_from_path_in_context` decodes UTF-8 once
  and dispatches to `classify_json_schema_text` (via `parse_referenced_json_schema`) for `.json`
  in any case, else `classify_yaml_schema_text`. Both classifiers are `pub(crate)`, do no I/O, and
  return typed errors, for Phase 4's inspection to reuse. The raw-YAML fallback,
  `missing_simplified_envelope_advisory`, and `is_draft_2020_12_keyword` are deleted.
  `parse_raw_json_schema` (used by the `SimplifiedSchema` file factories) keeps accepting any
  object, as Rule 5 requires.
- **The `missing_simplified_envelope` advisory framework (`SchemaAdvisory`,
  `SchemaAdvisoryKind`, the `dm.schema.missing_simplified_envelope` code, root-union advisory
  merging, DMLS/CLI projection) is kept but now has no producer.** Docs mark the code reserved.
- Docs: `resolve_schema` and loader `## Errors`;
  `simplified-schema/docs/topics/language/schema-documents.md` (new "What a Schema Reference May
  Target" section with table and Mermaid flowchart, raw-YAML-to-JSON migration, envelope keys,
  `description`, `types`-only libraries, catalogs); `darkmatter/docs/inline/schema-validation.md`
  (target rules replaced the raw-YAML/advisory paragraph);
  `darkmatter/docs/topics/schemas/dmls-schema-support.md` (advisory code row marked reserved).
  `darkmatter/docs/topics/schemas/definition.md` is left for task 16.1, which owns it.

### Trigger references after the loader restriction (found during 5.x)

- Before this phase a `$schema` or trigger payload naming a `kind: schema-trigger` file loaded it
  through the raw-YAML fallback, and the post-resolution checks then reported
  `TriggerSchemaReferenced` (with the "reference `x.yaml` instead" suggestion) or
  `TriggerPayloadCycle`. The loader now refuses the trigger file first, so those errors became
  unreachable from disk and users got a generic "unsupported top-level key(s): kind, match".
- Fixed in `darkmatter/lib/src/markdown/schemas/triggers/assemble.rs`:
  `refused_trigger_file(&SchemaError)` finds the refused file in the error's source chain and, when
  it claims the trigger envelope, the document path (`schemas/mod.rs::effective_for` resolution)
  maps it to `TriggerSchemaReferenced` and the payload path (`resolve_file_payload`) to
  `TriggerPayloadCycle` (`payload_cycle`, shared with `check_payload_cycles`).
  `file_claims_trigger_envelope` no longer returns a `Result` (it never failed). Every trigger test
  the migration agents had switched to the generic error was restored to its original assertion
  and passes: `phase4_trigger_assembly` (4), `triggers::assemble::tests` (2),
  `trigger_discovery_recovery` (2), `trigger_payload_load_errors` (1), and the CLI
  `invalid_trigger_warnings::compose_of_a_document_referencing_the_ignored_trigger_still_fails`.

### 4.2 DMLS standalone claim

- `darkmatter/dmls/src/overlay/schema.rs::standalone_envelope_claim` claims `Pure` when `$schema`
  is present and every top-level key is in `{$schema, types, description}`. Claim tests gained
  described informal and tagged cases plus declines for an extra key and another `kind`; two tests
  that used `description` as the stray key now use `notes`.
- New L1 `dmls::l1 lsp_session::meta_schema_standalone_with_description_keeps_last_good_while_mid_edit`.

### 5.x Audit and migration

- Planning-time audit (Rule 6): the only shipped `$schema` target newly rejected was
  `prompts/_implement/implement.yaml`; its `types:` key is renamed `$schema:` (it keeps
  `kind: schema`), so `_fm.md`'s entries are document properties.
- Full run before the sweep: simplified-schema 6 failures; darkmatter/CLI/DMLS 106 failures in 36
  files. Four parallel agents fixed disjoint file groups with scoped runs; I fixed the corpus,
  catalog, claim, and simplified-schema tests. Files and reasons (fixture = add the 2020-12
  dialect / wrap under `$schema:`; flip = the test's subject was the removed behavior):
  - simplified-schema: `lib/src/resolve.rs` in-source tests (5 flipped: bare map, YAML without
    `$schema`, advisory exclusions, root-union advisories now a rejection naming the arm, plus the
    JSON control gained the dialect); `lib/tests/l1/shipped_corpus.rs` (the catalog now parses as a
    tagged envelope with its description instead of being refused).
  - darkmatter lib l1: `shipped_schema_corpus.rs` (new content-derived `Class::TypesOnly`: refused
    as `$schema` with `TypesOnly`, each non-function type imported and lowered; the function
    catalogs now expect `TypesOnly`; `claudine/docs/schemas/partials/communication.yaml` lost its
    stale "description header" exception; counters split into resolved and types-only files);
    `global_catalog.rs` (rediscovery test rewritten: named by `$schema` the catalog is an ordinary
    tagged schema, never the baseline unless registered); `schema_extraction_characterization.rs`
    (JSON loader matrix: every dialect cell, meta-schema violation, trailing `#`, JSON5 trailing
    comma, non-object roots; YAML matrix `types_absent` message and new
    `types_absent_beside_schema`); `schema_yaml_spellings.rs`; `schemas_named_import_errors.rs`;
    `suggest_constraint_phase1.rs`, `suggest_constraint_phase4.rs`; `trigger_discovery_recovery.rs`
    (dialect; two compile tests use an invalid regex because `"type":12` now fails the meta-schema
    first); `raw_schema_conversion.rs`, `raw_root_union_assertions.rs` (dialect via helpers;
    `yaml_raw_schema_converts_like_json` flipped to `yaml_raw_schema_is_rejected_as_a_schema_target`);
    `schema_enhancements_matrix/raw_schema.rs` (JSON matrices declare the dialect, meta-schema
    cells expect `InvalidJsonSchema`; the three raw-YAML matrices collapse into one
    `raw_yaml_schema_is_not_a_schema_target`, their JSON twins keep every cell);
    `schema_enhancements_matrix/envelope.rs` (types-only cells expect `TypesOnly`);
    `numeric_admission.rs`, `schemas_core_type_grammar.rs`, `schemas_source_projection.rs`,
    `schemas_named_type_merge.rs`, `schema_cache_validity.rs`, `variable_type_projection.rs`,
    `meta_schema_phase1.rs`, `meta_schema_phase5.rs`, `recursive_schema_merge.rs`
    (`raw_yaml_baseline_ingress_*` flipped to `raw_yaml_baseline_file_is_refused_before_merge_ingress`),
    `recursive_schema_golden.rs` (schema-file goldens lower the `types` table after asserting the
    `TypesOnly` refusal; all 23 goldens byte-identical).
  - golden corpus: `fixtures/recursive_schema_golden/manifest.yaml` and `README.md`; deleted the
    raw-YAML inputs `detected.schema.yaml`, `keywords.schema.yaml` and their goldens (byte-identical
    to the kept `.json` twins); `keywords.schema.json` input and its golden gained the dialect.
  - darkmatter lib in-source: `compose/schema_validation.rs` (dialect),
    `compose/tests/frontmatter.rs` (4 advisory tests flipped), `schemas/tests/mod.rs` (3 flipped,
    2 fixtures wrapped), `schemas/mod.rs` `claudine_compat_tests` (2 fixtures wrapped).
    Advisory de-duplication across a transclusion merge and through baseline assembly can no
    longer be reached from a file; those tests now pin the rejection.
  - darkmatter CLI l1: `schema_validate.rs` (tagged spellings use `$schema:`; 3 bare-sidecar
    tests flipped), `schema_robustness_matrix.rs` (root-key-removed cell expects the load error;
    raw fixture dialect; unused `Expect::ValidWarning` removed).
  - DMLS l1: `lsp_session.rs` (stray key `notes`; 2 bare-sidecar tests flipped, dependency
    tracking kept: repairing the sidecar clears the error), `named_type_references.rs`,
    `tests/fixtures/suggest_constraint/tagged.yaml` (`types:` to `$schema:`).
- No shipped schema file other than `implement.yaml` needed editing: the shipped `types`-only
  files (`darkmatter/schemas/partials/*`, `darkmatter/docs/schemas/env.yaml`,
  `content-policy/schemas/content-policy.yaml`, `claudine/schemas/{action,linking,lifecycle,sequence-types}.yaml`,
  `claudine/docs/research/_types.yaml`, ...) are named-type libraries no document names by `$schema`.

### 6.1 / 6.2 New tests

- `simplified-schema/lib/tests/l1/schema_target_classification.rs` (new, declared in `main.rs`;
  19 tests through `resolve_schema_with_roots`): YAML controls; loader document shapes (duplicate
  key, trailing content, second document, bare `$schema:`); every Rule 8 YAML row; raw JSON Schema
  in YAML with the migration note; informal unknown key and another `kind`; tagged `$schema`-only,
  neither, unknown key; types-only refused but importable; delegation and root-union arms naming a
  types-only / no-dialect file fail naming that file; malformed first-root file reported, later
  root unused; field matrices for `description` (absent, `""`, null, implicit null, number,
  sequence, mapping, in both envelopes), `$schema` (null, number, boolean, `""`), `types` (`{}`,
  null, scalar, sequence), `categories` (absent, `[]`, null, scalar); JSON control (and `.JSON`),
  duplicate key, trailing content, JSON5 comment and trailing comma, dialect absent/null/number,
  `""`/trailing `#`/draft-07, non-object roots with the dialect hint, meta-schema failure.
- `darkmatter/cli/tests/l1/schema_target_recognition.rs` (new, declared): `md schema validate`
  in a `git init` repository on a described schema file (valid exits 0, missing required exits 1)
  and on a types-only target (exit 2 naming the file and the import hint).
- Shipped corpus: `simplified-schema` `shipped_corpus.rs` and darkmatter `shipped_schema_corpus.rs`
  as above; `GlobalCatalog::darkmatter()` and Claudine's catalog load (`global_catalog.rs`, 88 pass).

### Gates (run once, after all agents finished, with no agent running)

- `darkmatter/` `just test`: 12786 passed, 10 skipped. `just lint`: pass.
- `simplified-schema/` `just test`: 1280 passed (`file` feature) and 1110 passed (minimal). The
  minimal build first warned that `parse_raw_json_schema` was unused without `file`; it is now
  `#[cfg(feature = "file")]` like its only caller. `just lint`: pass (one `err().expect()` in the
  new test fixed to `expect_err`).
- `claudine/` `just test`: 11807 passed, 15 skipped (first run: 15 migration failures, fixed by one
  agent plus `host_catalog.rs`). `just lint`: pass.
- `just check-tier-coverage darkmatter` and `simplified-schema`: nothing stranded.
- Smell grep over added lines of the touched source files (`#[serde(default)]`,
  `unwrap_or_default()`, `filter_map(.. as_str())`, `.ok()`): no hits. The one `let Ok(..) else`
  in `classify_yaml_schema_text` sits behind the standalone parser, which has already rejected
  text that is not YAML; it carries a comment saying so.
- No load-sensitive failures were seen. No cross-OS run: the change is classification logic, and
  every new path assertion uses `canonicalize_simplified` like the existing tests.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| Rule 8 YAML rows (not mapping, accepted, types-only, extra key with/without raw note, no `$schema`) | `simplified-schema::l1 schema_target_classification::{a_non_mapping_yaml_target_is_not_a_mapping, yaml_controls_resolve_as_simplified_schemas, a_types_only_document_is_refused_as_schema_but_still_imports, raw_json_schema_in_yaml_names_its_extra_keys_and_the_json_migration, an_informal_mapping_with_an_unknown_key_is_a_document_error, a_mapping_without_schema_is_not_a_schema_document}` |
| YAML loader document shapes | `schema_target_classification::{yaml_document_shapes_are_located_document_errors, a_bare_schema_key_is_a_null_payload_error}` |
| Rule 8 JSON order of checks, dialect cells, meta-schema | `schema_target_classification::{json_control_resolves_as_raw_json_schema, json_document_shapes_are_located_document_errors, json_dialect_and_root_shapes, a_declared_schema_failing_the_meta_schema_is_invalid_json_schema}`; `darkmatter::l1 schema_extraction_characterization::json_loader_matrix::*` |
| Rule 9 field shapes (`description`, `$schema`, `types`, `categories`) and tagged rules | `schema_target_classification::{description_field_shapes, schema_field_shapes_in_yaml, types_field_shapes, categories_field_shapes, tagged_envelope_rules}`; `darkmatter::l1 suggest_constraint_phase4::{content_classifier_recognizes_only_the_two_envelopes, claimed_malformed_envelopes_are_schema_document_errors}` |
| Delegation / root-union hops classified; first-root malformed file reported | `schema_target_classification::{delegation_and_root_union_arms_classify_each_hop, a_malformed_file_in_the_first_root_is_reported_not_skipped}`; `simplified-schema resolve::tests::root_union_arm_naming_a_raw_map_fails_with_that_file` |
| Rule 7 errors, hints, rendering | `simplified-schema errors::tests::{not_a_schema_document_display_includes_path_reason_and_hint, json_dialect_errors_display_path_and_detail, not_a_schema_document_block_renders_path, json_dialect_blocks_render_path}` |
| Catalog refusal removed; catalogs still load | `simplified-schema::l1 shipped_corpus::darkmatter_catalog_and_partials_resolve_from_supplied_sources`; `darkmatter::l1 global_catalog::*` (88); `claudine::l1 host_catalog::*` (54) |
| Shipped corpus under the new rules | `darkmatter::l1 shipped_schema_corpus::*` (types-only class, function catalogs, research contracts) |
| Normal invocation (`md schema validate`) | `darkmatter-cli::l1 schema_target_recognition::{a_described_schema_file_applies_its_schema, a_types_only_schema_file_is_refused_with_the_import_hint}` |
| DMLS claim with `description`, last-good mid-edit | `dmls overlay::schema::tests::envelope_claim_recognizes_block_and_flow_presentation`; `dmls::l1 lsp_session::meta_schema_standalone_with_description_keeps_last_good_while_mid_edit` |
| Bare sidecar now an error through every surface | `darkmatter-cli::l1 schema_validate::schema_validate_*_bare_sidecar*`; `dmls::l1 lsp_session::bare_sidecar_*`; `darkmatter markdown::compose::tests::frontmatter::bare_sidecar_*`; `claudine-cli::l1 compose_schema_cli::compose_and_inline_reject_a_bare_sidecar_without_a_schema_property::*` |
| Trigger references keep their specific errors | `darkmatter markdown::schemas::phase4_trigger_assembly::*`, `triggers::assemble::tests::rejects_payload_*`, `darkmatter::l1 trigger_discovery_recovery::a_payload_cycle_is_a_finding_and_a_valid_neighbor_installs`, `trigger_payload_load_errors::a_payload_cycle_skips_each_trigger_*`, `darkmatter-cli::l1 invalid_trigger_warnings::compose_of_a_document_referencing_the_ignored_trigger_still_fails` (all unchanged assertions) |

No test was skipped, ignored, or left failing; there are no pre-existing failures.

## Phase 3

### 7.1 Schema reference facts (Rule 10)

- New `darkmatter/dmls/src/overlay/references.rs`: `SchemaReferenceFacts { entries }` and
  `SchemaReferenceFact { pointer, reference, selection }`. `SchemaReferenceFacts::select` parses the
  text the schema was assembled from (the last-good text while the buffer is mid-edit), takes the
  scalar `/$schema` or each `/$schema/{n}` item of a root-union sequence, keeps only values whose
  authored text decodes (`serde_yaml_ng`) to a YAML string, and calls `select_schema_file` with
  `path.parent()`, the registry's `roots.search_paths()`, and the document context: the inputs
  `schema::assemble` gives the validator. The `target` preview field is left for Phase 4 (decision
  recorded).
- `overlay/mod.rs`: `CachedSchema` gains `probes` (each attempted location and whether it existed)
  and `schema_references`; a hit also requires `probes_unchanged`. `collect_dep_hashes` adds each
  selected file. `FrontmatterSchema`/`DocumentOverlay` carry `schema_references`; the trigger-error,
  trigger-warning, and both standalone constructors use `Arc::default()`. No context: no facts. A
  registry error: every fact is `Err(error)`.
- Unit tests (`dmls overlay::tests::`): `schema_reference_facts_cover_the_scalar_and_root_union_string_items`,
  `schema_reference_facts_skip_values_that_are_not_strings` (block and flow mappings, `null`, bare
  `$schema:`, number, boolean, alias, `[null, 3]`, a non-`$schema` key),
  `schema_reference_facts_keep_the_last_good_text_while_mid_edit`, `schema_reference_facts_need_a_context`,
  `unreadable_schema_roots_fail_every_reference_without_guessing_a_root` (a non-UTF-8 root file),
  `a_cached_schema_reselects_when_a_probed_location_changes` (miss cached, create, break, delete).

### 8.1 Links and definition (Rule 11)

- `providers/frontmatter.rs`: `schema_target` became `schema_targets`, reading the overlay facts
  (pointer match and `reference == entry.scalar`): `Found` links the file, `Missing { creation:
  Some }` and `SiblingOnly` link the creation path (never the sibling), everything else nothing.
  `nav_targets`/`definition` use it; their docs no longer say `$schema` resolution is uncached.
  `looks_like_path` had no other caller in the provider and was removed with its unit test
  (`graph/substrate.rs` keeps its own copy, as Rule 18 says).

### 8.2 Diagnostic policy (Rule 13)

- `diagnostics/frontmatter.rs`: a problem with `file_reference: Some(NoMatch { .. })` and no
  `caller_file` is not published (typed match). `dmls/tests/l1/repository_contexts.rs`: the
  repaired-context test now asserts the missing eager file is NOT a diagnostic; a stale comment in
  `repository_sigils_resolve_in_every_feature` was corrected.
- Tests (`dmls diagnostics::frontmatter::tests::`): `a_missing_file_value_is_left_to_the_hover_not_diagnosed`
  (eager and plain missing, existing control; the validator still reports NoMatch at `/missing`),
  `a_missing_eager_file_in_a_sequence_item_is_not_diagnosed`, `a_malformed_file_reference_keeps_its_diagnostic`
  (`InvalidReference`), `a_repository_reference_without_a_repository_keeps_its_context_diagnostic`
  (`MissingContext`), `an_io_failure_keeps_its_diagnostic` (`Io`, `#[cfg(unix)]`: Windows reports a
  path through a regular file as not found), `missing_schema_import_and_example_files_keep_their_prepare_diagnostic`,
  `a_missing_required_value_keeps_its_strict_diagnostic`, `a_yaml_syntax_error_keeps_its_parse_diagnostic`;
  each pins code, range, message, and `data`.
- CLI: `darkmatter-cli::l1 schema_validate::schema_validate_still_rejects_a_missing_eager_file`
  (`md schema validate` exits 1 naming `./nope.md`; the lazy `file` value is not reported).

### 9.1 Response contracts and parity

- `dmls::l1 frontmatter_response_contracts::schema_reference_navigation_is_unchanged` with the new
  golden `tests/fixtures/response_contracts/schema_reference_navigation.json` (11 cases: the
  `review-3.md` regression targeting the repository's `schemas/feature-review.yaml`, clean miss to
  the first root, no-root folder to the document folder, package copy, `SCHEMAS_DIR` winning,
  sibling-only to the first-root creation path, `./missing.yaml`, a root-union item, a malformed
  winning file, a document in a `[x] (y) café` folder, a non-ASCII explicit target). Read after
  generation; every target matches Rules 4 and 11, and no creation path exists after the requests.
  `Session::portable` became the shared `portable_under` (same behavior).
- `dmls::l1 schema_reference_parity::{links_definition_and_validation_select_the_same_schema_file,
  the_fixture_has_the_schema_roots_each_case_relies_on, a_target_in_a_bracketed_folder_has_a_link_and_definition}`:
  per case, link target = definition target = `select_schema_file` over
  `SchemaRoots::for_document(..).search_paths()` = darkmatter's `resolve_schema_with_roots`
  `referenced_files`; a malformed target's diagnostic related information names the same URI;
  UTF-16 ranges are checked independently of the server's source map.
- **Defect found and fixed:** a link or definition whose target is in a folder with `[`/`]` was
  dropped, because `url::Url::from_file_path` leaves `[`, `]`, `^`, `|`, `\` raw and
  `lsp_types::Uri` rejects them. `dmls/src/workspace/mod.rs::file_path_to_uri` now percent-encodes
  them in the path (unit test `workspace::tests::file_uris_encode_brackets_and_other_characters_a_uri_rejects`
  round-trips through `uri_to_file_path`). Every DMLS link/definition/location gains the fix.

### 9.2 Session tests

- `dmls::l1 schema_reference_sessions::` (7 tests, each 0.3-0.6 s, no sleeps; `flush_server_with`
  after each watched change): editing the selected file, deleting it (next candidate, then the
  creation path plus the missing-`$schema` diagnostic), a `SCHEMAS_DIR` candidate outside the
  workspace taking over and handing back, a created `SCHEMAS_DIR` root folder, a created home
  `~/schemas` root resolving a miss, repairing a malformed target, and a 2020-12 JSON target. After
  each step links, definition, and every file-naming diagnostic agree. Ran 3 times: 7/7 each.
  No production defect found.

### Docs and skill

- `darkmatter/docs/topics/schemas/dmls-schema-support.md` (the `invalid_file_reference` row; the
  navigation bullet's creation-path fallback) and `darkmatter/dmls/docs/file-references.md`
  (failure-class table) were false after this phase and were corrected; Phase 6 extends them.
- `.claude/skills/darkmatter/dmls.md`: one bullet on the facts, probes, `schema_targets`, and the
  NoMatch policy.

### Gates (run once, after every agent finished, with no agent running)

- `darkmatter/` `just lint`: pass (after fixing one `cloned_ref_to_slice_refs` in the new overlay test).
- `darkmatter/` `just test`: 12812 passed, 10 skipped, 0 failed.
- `just check-tier-coverage darkmatter`: nothing stranded.
- Dependents: no package outside the darkmatter area depends on `dmls`; the darkmatter library is
  unchanged this phase (only a CLI test file changed), so no Claudine or simplified-schema run was needed.
- Smell grep over the new code: the one `.ok()?` in `references.rs::decoded_string` sits on text the
  frontmatter loader already parsed; a value that does not decode is, by Rule 10, not a reference.
- Load-sensitive failures: none seen.
- Cross-OS (the URI encoding change carries Windows risk): `just cross-check dmls --os windows`: 1602 passed, 9 skipped.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| Facts for scalar and root-union string items only; not for mappings, null, numbers, booleans, aliases | `overlay::tests::schema_reference_facts_cover_the_scalar_and_root_union_string_items`, `schema_reference_facts_skip_values_that_are_not_strings` |
| Stale text keeps last-good facts; no context, no facts; unreadable roots give `Err` | `overlay::tests::schema_reference_facts_keep_the_last_good_text_while_mid_edit`, `schema_reference_facts_need_a_context`, `unreadable_schema_roots_fail_every_reference_without_guessing_a_root` |
| Probes and selected-file hashes invalidate the cache | `overlay::tests::a_cached_schema_reselects_when_a_probed_location_changes`; `schema_reference_sessions::*` |
| Problem 1 (`review-3.md` targets the repository's `schemas/feature-review.yaml`) for link and definition | `frontmatter_response_contracts::schema_reference_navigation_is_unchanged`; `schema_reference_parity::links_definition_and_validation_select_the_same_schema_file` |
| Creation-path fallback (first root; document folder with no root); sibling never linked; nothing created | same two tests |
| Validator, link, definition, and diagnostics select the same file | `schema_reference_parity::*`, `schema_reference_sessions::*` |
| URIs with spaces, Markdown punctuation, non-ASCII | `schema_reference_parity::a_target_in_a_bracketed_folder_has_a_link_and_definition`, the golden's `café`/`[x] (y)` cases, `workspace::tests::file_uris_encode_brackets_and_other_characters_a_uri_rejects` |
| A missing `file`/`file(eager)` value is not a DMLS diagnostic; every other file-reference and schema diagnostic unchanged | `diagnostics::frontmatter::tests::*` (8 new), `repository_contexts::a_failed_context_is_one_diagnostic_and_no_resolution_until_repaired` |
| `md` runtime eager check unchanged | `darkmatter-cli::l1 schema_validate::schema_validate_still_rejects_a_missing_eager_file` |

No test was skipped, ignored, or left failing.

## Phase 4

### 10.1 `inspect_schema_target` (Rule 15)

- New `simplified-schema/lib/src/resolve/inspect.rs`, re-exported from `resolve`:
  `inspect_schema_target(path, bytes) -> SchemaTargetInspection { format, classification, description, lines }`.
  It classifies with the loader's own `classify_yaml_schema_text` / `classify_json_schema_text` (non-UTF-8
  gets the loader's "not UTF-8 text" error), keeps a description only when it is a nonblank string (trimmed),
  and returns lines only for an accepted file: YAML is the root `Declaration` span of the `$schema` value,
  dedented; JSON is the whole document. `\r\n` normalized; a final newline is not a line. Decision on the
  first line's indentation recorded in `decisions_for_review`.
- `resolve.rs` module docs said unrecognized YAML "remains raw JSON Schema" and JSON is "always raw JSON
  Schema", stale since Phase 2; corrected (code is right, comment was wrong).
- `select.rs::is_bare_name` is now public (decision recorded).
- Tests: `simplified-schema::l1 schema_target_inspection::` (8): block mapping with an inner comment, the
  `feature-review.yaml` block-sequence shape with a header comment, flow mapping / scalar / quoted scalar
  delegation, CRLF + final newline, ten and eleven lines, JSON whole document with a `.JSON` extension,
  rejected targets (types-only, JSON without dialect, draft-07, duplicate key, non-UTF-8) with no preview,
  and the description matrix (absent, `""`, blank, nonempty informal and tagged, comment block, `null` and
  number rejected by the envelope; JSON absent, `null`, number, blank, nonempty).

### 10.2 Hover helpers (Rule 14)

- New `darkmatter/dmls/src/providers/reference_hover.rs` (pure: no context, no I/O): `markdown_escape`,
  `code_span`, `fence_for`, `display_path`, `markdown_link` (`(`/`)` encoded on top of `file_path_to_uri`),
  and one function per template (valid target, not a schema, unsupported dialect, unusable target, clean
  miss / zero roots / sibling-only, unresolvable, remote, roots unreadable, no context, `file` invalid /
  found / missing with `Eagerness::{Lazy, Eager, EagerFileArm}` / unchecked).
- Unit tests `dmls providers::reference_hover::tests::` (12): exact template strings, ten vs eleven lines,
  four-backtick fence, escaped `[`, `]`, `*`, `_` label, `%28`/`%29` URIs, repository-relative vs absolute
  labels, non-ASCII.

### 11.1 `$schema` value hover (Rules 10, 14)

- `overlay/references.rs`: `SchemaReferenceFact` gained `target: Option<SchemaTargetPreview { format,
  description, preview (≤ `PREVIEW_LINES` = 10), total_lines, verdict }>`, built inside `select` for every
  `Found` selection: one counted read (`work_counters::read`), one `inspect_schema_target`, one
  `record_schema_target_inspection`. `verdict` is `Valid` when the schema assembled (`SchemaOutcome::Ready`),
  otherwise that reference alone is re-resolved with darkmatter's `resolve_yaml_schema_with_roots`
  (so another union item or a trigger failing does not mislabel it). `select` takes the new `assembled`
  flag; `schema_for` passes it. The selected path was already in `collect_dep_hashes` (Phase 3).
- New counter `work_counters::schema_target_inspections` (struct, `since`, `plus`, `work_counts`, unit test).
- New `providers/frontmatter/schema_value_hover.rs`: reads only the facts (pointer match and
  `reference == entry.scalar`); with no context it finds the `$schema` string with the same reference
  walker (`references`, now `pub(crate)`) and says so.

### 11.2 `file` value hover (Rule 12)

- `ScalarTyping` gained `eager` (definition, or the array property of a sequence item) and `mixed_arms`
  (an arm that is not `file`), computed in `classify_scalar`.
- New `providers/frontmatter/file_value_hover.rs`: `file_value_is_static` (pending `{{`/`$(` syntax that
  is `$(` or not a `FileReference` is not static, so `{{DIR}}/x.md` is), `file_value_hover` (decoded YAML
  string only; `FileReference::new` error -> invalid; resolve -> found / missing (property = nearest
  mapping-property `dotted`) / unchecked class), Markdown from `reference_hover`.

### 12.1 Wiring

- `providers/frontmatter.rs::hover`: `schema_value_hover` first, then `meta_schema_hover`, then the
  expression/stack block `.or_else(file_value_hover)` (only inside the scalar's value span), then the key
  path. Docs of `hover` and `schema_hover` updated; the module header was still accurate.
- No `a3_*hovers.json` or `a3_navigation.json` golden changed: the full suite passed without regeneration,
  so no golden hovers a `file`-typed value position.

### 13.1-13.3 Tests

- New `dmls::l1 value_hovers::` (21 tests, in-process `Hovers` harness over a canonical temp root):
  - the `review-3.md` acceptance case against the repository's real `schemas/feature-review.yaml`
    (`include_str!`): `Schema Definition:`, the first ten dedented lines, `*Showing **10** of **N** lines*`
    with N computed from the shipped file, and the link; key hover unchanged and ranged on the key;
    2 / 10 / 11 lines; description policy (tagged, informal block scalar, blank, absent, comment);
    JSON (`json` fence, CRLF normalized, description); delegation (`./real.yaml` preview, link to the
    direct file); not-a-schema (types-only, no `$schema`, no-dialect JSON, JSON array); draft-07 dialect;
    malformed / missing import / unreadable beside the document / unreadable in a root (roots-unreadable
    text, no link); root-union items hovering independently while another item is missing, inline arm and
    inline mapping keeping their hovers; clean miss with one and two roots (nothing created), zero roots,
    sibling-only (sibling never linked); remote; no context.
  - `file` values: found, missing lazy, missing eager, invalid (`@`), nested mapping (`nested.main`),
    scalar array item (`items`), mixed union (eager-arm wording), discriminant-selected arm, ambiguous
    arms, the other arm (no file hover), `{{ title }}.md` and `$(echo x.md)` keeping the property hover,
    `{{DIR}}/x.md` resolving to the missing-context text, null / number / inline mapping / collection
    marker / comment with no file hover, key hover kept, `&root-only.md` outside a repository
    (missing context), remote URL, I/O (`./afile.md/inner.md`, Unix), and no context (no file hover).
  - counters: preparing the overlay inspects the one target once; two hovers on the same snapshot add
    0 inspections and 0 reads; no engine build or network attempt (`effects-instrumentation`).
  - ranges and URIs: UTF-8 and UTF-16 ranges equal the value span with `é🎉` earlier in the document,
    for a target and a file in `[x] (y) café`; labels escaped, URIs free of raw `( ) [ ]` and spaces.
- `dmls::l1 schema_reference_sessions::` (7, extended, not duplicated): every observation also requests
  the hover on the `$schema` value; `assert_agree_on` now requires the hover to name the same file (its
  link, or its `Missing:` creation path), the missing and invalid assertions check the hover's lead, and
  each edit/fallback/shadow/repair step checks the preview shows the new definition line.
- Found and fixed while writing tests: none in production code. Two test expectations were corrected:
  reasons are escaped per Rule 14 (backticks in a `FileReferenceError` show literally), and an unreadable
  file inside a schema root fails the registry scan first (recorded as a decision).

### Gates (run once, after all edits, nothing else running)

- `simplified-schema/` `just test`: 1288 passed; minimal-features run 1118 passed. `just lint`: pass
  (default, `work-counters,terminal`, and `--no-default-features`).
- `darkmatter/` `just lint`: pass. `just test`: 12845 passed, 10 skipped, 0 failed.
- `just check-tier-coverage darkmatter` and `simplified-schema`: nothing stranded.
- Dependents: Claudine uses no changed symbol (the library changes are additions); `cargo check -p claudine
  -p claudine-cli --tests` passes.
- Cross-OS (display paths and URIs): `just cross-check dmls --os windows`: 1635 passed, 9 skipped.
- `affected_scope.py` over the working tree: packages `dmls, simplified-schema, test-toolkit, darkmatter`,
  change class `package`, so the gates were required and run.
- Smell grep over new code: `inspection.lines.unwrap_or_default()` in `references.rs::preview` follows an
  early return on every rejected classification, so `lines` is always `Some` there; no `.ok()` or
  `#[serde(default)]` on a load-bearing parse was added.
- Load-sensitive failures: none seen.

### Checkpoint reading

The `review-3.md` equivalent (the shipped `feature-review.yaml` in a tree root) hovers as:
`Schema Definition:`, a `yaml` block starting `- description: string -> description of this _feature review_`
with ten lines, `*Showing **10** of **N** lines*`, and `Resolves to [schemas/feature-review.yaml](file://…)`,
which matches the spec's Acceptance section. Pinned by
`value_hovers::a_review_document_previews_the_first_ten_lines_of_the_shipped_feature_review_schema`.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| Preview extraction (Rule 15) incl. comments, CRLF, final newline, boundaries, JSON | `simplified-schema::l1 schema_target_inspection::*` |
| Format verdict and description matrix from the loader's classifiers | `schema_target_inspection::rejected_targets_keep_the_loader_error_and_have_no_preview`, `description_is_kept_only_when_it_is_a_nonblank_string` |
| Every Rule 14 template and helper, exact text | `dmls providers::reference_hover::tests::*` |
| `$schema` hover: success, truncation, description, JSON, delegation, failures, misses, sibling, remote, no context, union items, inline hovers kept, key unchanged | `value_hovers::*` (`$schema` section) |
| Shipped artifact through the normal hover path | `value_hovers::a_review_document_previews_the_first_ten_lines_of_the_shipped_feature_review_schema` |
| `file` hover: every spec row, nested/item/union, expressions, non-strings, unchecked classes | `value_hovers::*` (`file` section) |
| Prepared once, never on hover; no composition | `value_hovers::hovering_reuses_the_prepared_preview_and_never_composes` |
| Hover agrees with links, definition, diagnostics after change, delete, nearer candidate, repair, JSON | `schema_reference_sessions::*` |
| Ranges in negotiated encodings; URIs with spaces, `[`, `(`, non-ASCII | `value_hovers::value_hover_ranges_and_links_survive_unusual_text_and_paths` |

No test was skipped, ignored, or left failing.

## Phase 5

### Wave 14: capture (14.1)

- `graph/substrate.rs`: `DocumentIndex` gains `summary: Option<String>`; `SUMMARY_FIELDS`
  (`excerpt`, `summary`, `description`, `about`) is walked over `ast.top_level()` in
  `index_document_timed`, and `summary_text` decides each field: a `FmValueKind::Scalar` whose value
  span decodes with `serde_yaml_ng` to a YAML string that is nonempty after trimming and has no
  pending syntax (`simplified_schema::pending::holds_pending_syntax`, the same check Phase 4's `file`
  hover uses). Everything else is `None` and the next field is tried; nothing is stringified.
- `graph/arena.rs`: `DocumentState` is now `Clone` (not `Copy`) with `summary: Option<Arc<str>>`,
  set in the one `states[..] = DocumentState { .. }` construction; `record_at` borrows the state and
  `DocumentRecord` gains `summary: Option<&'a str>`. The shared `DocumentTable` is untouched.
- `index_store/facts.rs`: `into_index` yields `summary: None`; the doc lists it with the unstored
  fields. `tests/l1/index_store_facts.rs::stored_part` clears it. `FACTS_VERSION` and
  `index_store_golden.txt` are unchanged (`git diff` shows neither); all 120 `index_store` tests pass.

### Wave 15: hover (15.1) and sessions (15.2)

- `providers/hover.rs::resolved_hover` appends `\n\n{summary}` after the title, path, and the
  `Heading:` line; module docs updated. No disk read was added.
- `tests/l1/referenced_targets.rs`: five new consumers, each run for the listed, excluded, and
  outside-workspace target (15 `#[test]` cells through the existing `cells!` macro).
- Mutation check: with the append disabled, all 15 positive tests (12 session + 3 unit) fail and the
  4 negative ones (byte-identical / unreadable) pass, so the assertions are load-bearing.

### Input Robustness Matrix (one validator: `summary_text`)

| Shape | Outcome | Test |
|---|---|---|
| absent | next field | `an_unusable_summary_value_is_skipped_never_stringified` (`title:` only) |
| null (`summary:`, `null`, `~`) | next field, never `"null"` | same; `"null"` quoted is accepted as text in `a_summary_is_a_nonempty_trimmed_yaml_string` |
| wrong type (int, float, bool, flow/block sequence, flow/block mapping) | next field | same |
| empty / blank | next field | same |
| pending `{{ title }}`, `$(whoami)` | next field | same |
| plain, single/double quoted, literal and folded block, quoted number | accepted, trimmed | `a_summary_is_a_nonempty_trimmed_yaml_string` |
| each field control + null | per field | `every_summary_field_skips_a_null_value` |
| precedence | `excerpt` > `summary` > `description` > `about` | `summary_fields_are_read_in_precedence_order` |
| nested key / no frontmatter / broken YAML | `None` | `only_top_level_summary_fields_count`, `a_document_without_frontmatter_has_no_summary` |

Smell grep over the new code: no `#[serde(default)]`, `unwrap_or_default()` on a parse, or
`filter_map(.. as_str())`; the one `.ok()` (`serde_yaml_ng::from_str(..).ok()?`) is the intended
"not a YAML string → next field" outcome.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| Field capture, matrix, precedence, no frontmatter | `dmls graph::substrate::tests::{a_summary_is_a_nonempty_trimmed_yaml_string, an_unusable_summary_value_is_skipped_never_stringified, summary_fields_are_read_in_precedence_order, every_summary_field_skips_a_null_value, only_top_level_summary_fields_count, a_document_without_frontmatter_has_no_summary}` |
| Store untouched | `dmls::l1 index_store_*` (120 tests), golden unchanged |
| Hover appends each field, precedence, unusable values byte-identical, heading line kept | `dmls providers::hover::tests::*` (4) |
| Summary shown on document and heading links in all three target cases; only source + target parsed (a listed sibling with a summary is not) | `referenced_targets::{listed,excluded,outside}_target_summary_hover` |
| Open buffer beats disk, edits change it, close falls back to disk | `referenced_targets::*_summary_from_buffer` |
| Watched change / delete refresh | `referenced_targets::*_watched_summary_refresh` |
| Save-based refresh / delete | `referenced_targets::*_save_summary_refresh` |
| Unreadable target keeps the hover without a summary | `referenced_targets::*_unreadable_summary` |

Fidelity: the capture and hover text are F1 unit tests; the refresh paths cross the LSP protocol,
the filesystem, and the router's preparation, so they are F2 in-process-server L1 sessions
(`LspFixture`). No window, no L2.

### Gates

- `just lint` (darkmatter area): pass.
- `just test` (darkmatter area): 12870 passed, 10 skipped, exit 0 (70.5 s). No failures, so no
  load-sensitive reruns.
- Dependents: no package outside `darkmatter/` depends on `dmls`, and only `dmls` changed, so there
  were no downstream tests to run.
- Cross-OS: no path, `#[cfg]`, or OS-specific code changed; the unreadable-target fixture uses invalid
  UTF-8 bytes, which fail the text read identically on every OS. No `just cross-check` was run.
- No test was skipped, ignored, or left failing.
