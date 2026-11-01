---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/spec.md"
plan: "darkmatter/features/2026-10-30-yaml-document-schemas/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_kind_documents.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/kind_document.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/frontmatter.rs
    - darkmatter/dmls/src/overlay/kind_document.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/references.rs
    - darkmatter/dmls/tests/l1/kind_document_editor.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/standalone_stdio.rs
    - darkmatter/docs/schemas/err.yaml
    - darkmatter/docs/schemas/schema-definition.yaml
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/kind_documents.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/tests/l1/kind_documents.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/trigger_spelling_guard.rs
    - simplified-schema/lib/src/simplified/standalone.rs
docs_updated_during_phase_1:
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - docs/kind-documents.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/cli.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/schemas/schema-trigger.yaml
    - simplified-schema/lib/src/envelope.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/positions.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/schema/mod.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/tests/l1/declaration_grammar.rs
    - simplified-schema/lib/tests/l1/envelope_admission.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/schema_target_classification.rs
    - simplified-schema/lib/tests/l1/schema_target_inspection.rs
    - simplified-schema/lib/tests/l1/shipped_corpus.rs
    - simplified-schema/lib/tests/l1/yaml_schema_envelope.rs
    - simplified-schema/schemas/schema.yaml
    - simplified-schema/schemas/yaml-schema.yaml
docs_updated_during_phase_3:
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/language/local-type-references.md
    - simplified-schema/docs/topics/language/schema-documents.md
    - simplified-schema/docs/topics/schema-object.md
    - simplified-schema/docs/topics/selecting-schema-files.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/simplified-schema/SKILL.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - simplified-schema
decisions_for_review:
    - |-
        The plan expected the pre-feature shipped-schema corpus test to pass
        unchanged after the revert, but it failed: the three kept definition
        files in darkmatter/schemas (schema.yaml, schema-trigger.yaml,
        yaml-schema.yaml) now declare `kind: yaml-schema`, a kind no code
        understands until phases 3-4, so the corpus walk could not classify
        them. I added them as exceptions in shipped_schema_corpus.rs instead
        of moving or deleting them (the plan keeps them in place until 3.3).
        yaml-schema.yaml uses the existing "Draft" class (its `non-null` and
        `property(...)` options do not parse until phase 2). The other two use
        a new test-only class, "UnclaimedKind": no loader claims the file, it
        is refused as a `$schema: ./file` target, and its bodies parse; it is
        not resolved inline because schema-trigger.yaml's `$schema` names a
        type from its own `types:`. Alternatives: reuse "NotASchema" (fails,
        because that class resolves `$schema` inline) or skip the files in the
        walk (loses the check that they stay parseable). If the reviewer
        prefers another shape, only these three exception entries and the
        UnclaimedKind arms in that one test file change; phase 3 moves the
        files and 7.3 re-derives the corpus anyway.
    - |-
        Phase 2: the four declaration options (`mapping`, `property`, `forms`,
        `declared-required`) are listed in the `md schema about` catalog as
        constraints accepted by `schema` / `type-definition`, not as `type_options`
        as the plan said. `type_options` is typed for alphanumeric options and means
        "written before the first `;`", which these are not. Alternative: generalize
        `type_options` to a second option kind. Redo cost: the four descriptor rows
        and two `accepted_constraints` lists in darkmatter's `about.rs`.
    - |-
        Phase 2: `non-null` needed neither a separate "reject null before
        conversion" step in the coercion engine nor an `allOf [not null]` wrapper,
        both named in plan rule 9. The coercion engine already never converts a null
        for any typed shape, so once lowering drops the optional-null arm the null
        reaches the final check unchanged and is reported at the property (a test
        proves it is not turned into "null" or treated as absent). Every fragment a
        `non-null` can mark already restricts its type; the only unrestricted ones
        (`null`, `unknown`) are declaration errors. Alternative: add the explicit
        pre-check anyway as defense in depth. Redo cost: one branch in
        `coercion/shape.rs` plus its test.
    - |-
        Phase 2: `type-definition(forms(literal, enum))` itself rejects a literal
        or enum alternative that is not a non-empty string (`literal(1)`,
        `literal(true)`), and the preparation-time `claimed_values` checks the same
        again. The plan put the string rule only at preparation, but spec ruling 1
        says the constraint must reject numeric and boolean literals and the plan's
        matrix wants that error through ordinary validation. Alternative: keep
        `forms` purely about constructors and leave strings to `claimed_values`
        only. Redo cost: one check in `format.rs` and three matrix rows.
    - |-
        Phase 2: when two schemas are layered, root closure follows the layer
        that wins: `layer_under` keeps the receiver's closure, and `layer_over` with
        a closed incoming layer gives a closed result. A lower layer's
        `additionalProperties: false` is never copied (plan rule 8 as written); a
        closed baseline is now admitted instead of rejected. Alternative: closure
        never crosses a layer in either direction (the result is open unless the
        receiver itself is closed). Redo cost: `resolve.rs::merge_layer` and the
        `closed_roots` layering tests.
    - |-
        Phase 2: beyond the spec's spelling table I added `non-null: true` as a
        `$constraints` flag on a nested block-form object (it moves onto the
        property like `required: true` already does), and canonical export now
        writes a nested block object's presence flags back into `$constraints`
        instead of refusing to export it (it refused `required` plus `min-keys`
        before). Without the flag a block-form nested object could not be marked
        `non-null` at all. Alternative: string-expression spelling only. Redo cost:
        one arm in `simplified/mod.rs`, one in `schema/export.rs`, and the block
        catalog row.
    - |-
        Phase 2: `ClaimProblem.path` for a root-union arm that came from a
        reference (alias, import, file arm) is written as if the arm were inline,
        because the resolved schema does not record which arms were references;
        callers locate it at the nearest existing ancestor (the `$schema` line).
        Alternative: record arm provenance in `ResolvedSchema`. Redo cost: a field on
        `ResolvedSchema` and the path computation in `claims.rs`; phase 4's located
        semantic checks are the consumer that would notice.
    - |-
        Phase 3: a `yaml-schema` named as a `$schema` target or `Name@file` import
        source is refused with a new reason on the existing error,
        `NotASchemaDocument { reason: SelectedByKind }`, instead of the new
        `SchemaError::SelectedByKind { path, span }` variant the plan named. Every
        consumer of "this file is not a schema target" (target inspection, the DMLS
        diagnostics, the terminal rendering, the corpus) already handles that error,
        so the new case needed no new plumbing. It carries no span: the target file's
        classification does not know where the reference was written, and the
        referencing document already locates `NotASchemaDocument` errors at the
        reference. Alternative: the separate variant with a span. Redo cost: one
        error variant, its Display/terminal rendering, and the three resolver sites
        plus tests that match on the reason.
    - |-
        Phase 3: no `kind: StandaloneKind` field was added to
        `StandaloneSchemaDocument`; the envelope enum already says it
        (`StandaloneSchemaEnvelope::YamlSchema`), and `StandaloneSchemaEnvelope::kind()`
        returns the claiming `kind` value. A second field would only repeat the
        envelope. Redo cost: add the field and set it in `standalone.rs`.
    - |-
        Phase 3: the closed `$schema` block of `darkmatter/schemas/schema-trigger.yaml`
        declares only `kind`, `match`, and `$schema` (non-null); it does not add the
        `description` / `categories` the plan's rule 16 said should "follow
        schema.yaml". Darkmatter's trigger parser accepts only those three keys, so a
        definition admitting the other two would contradict the parser that loads
        real trigger files. Alternative: add them to both the definition and the
        trigger parser in phase 4, when triggers are validated by this definition.
        Redo cost: two lines in the YAML plus `KNOWN_KEYS` in `triggers/envelope.rs`.
    - |-
        Phase 3: when an envelope has several defects, the reported one is chosen in
        this order: (1) a problem with the envelope itself (unknown key, wrong
        metadata, missing payload), preferring a non-"missing required" one so a
        misspelled `typed:` is reported rather than the missing `types`; (2) else a
        definition inside `$schema`/`types` the parser rejects, with the parser's exact
        token span; (3) else the carrier's shape (`types: null`). The plan only said
        "the first problem's span". This keeps the order the hand-written checks had
        (envelope before payload), which DMLS's malformed-buffer handling relies on, and
        keeps grammar errors precisely located. `locate_standalone_definition_failure`
        now runs admission and returns `None` when the envelope itself is malformed, so
        it agrees with the classifier. Alternative: always the definition's first
        problem (loses the in-string token span of grammar errors). Redo cost:
        `standalone.rs::rejected_envelope` and its precedence test.
    - |-
        Phase 3: no "carrier guard" was added to the coercion engine. A `schema` or
        `type-definition` value already converts as "any" (the keywords map to
        `Shape::Any`), so mappings, sequences, and reference strings pass through soft
        conversion untouched; a test proves it (sibling scalar converts, carriers
        unchanged), and admission reads `$schema`/`types` from the authored YAML, which
        the test shows is identical to the converted copy. Metadata (`description`,
        `categories`) is read from the converted copy. Alternative: an explicit guard
        branch as defense in depth. Redo cost: one branch in `coercion/shape.rs`.
    - |-
        Phase 3: the library's two compiled definitions are compiled through a
        private validator cache, not the process-wide shared one. They live for the
        process in a `OnceLock`, so in the shared cache they were counted by, and could
        be evicted from, a cache the host resets for benchmarks (an existing darkmatter
        test caught the count change). Added `Prepared::compiled_through` (crate-private)
        for this. Redo cost: none unless the reviewer wants them shared.
    - |-
        Phase 3: added three public helpers the plan did not list, each needed now or
        by phase 4: `resolve::prepare_embedded_yaml_schema` (prepare a `yaml-schema`'s
        exported `$schema` from embedded text; `prepare_embedded_schema` refuses it, so
        phase 4's registry and this phase's claim tests need it),
        `resolve::parse_referenced_schema_document` (the parse with the
        `SelectedByKind` refusal, used by DMLS when it reads imported files itself), and
        `PositionMap::byte_range` (exact byte span of a located problem, used for the
        admission error span). The `SimplifiedSchema` factories DO load a `yaml-schema`
        (a direct load is not a reference), and export writes `kind: yaml-schema` back.
        Redo cost: small, each helper is one function with one caller set.
    - |-
        Phase 3: the darkmatter corpus class `UnclaimedKind` was replaced, not just
        deleted: `schema-trigger.yaml` stays in `darkmatter/schemas/` and is now a real
        `yaml-schema`, so it has a new class `YamlSchemaDefinition` (admitted as a
        `yaml-schema` envelope, refused as a target with `SelectedByKind`). The two
        library definitions left the darkmatter tree and are covered by
        simplified-schema's `shipped_corpus.rs`. DMLS's schema-root summary does not
        count a `yaml-schema` as a named schema (it is never referable by name); phase 6
        may give it its own class. Redo cost: the corpus class and one match arm in
        `dmls/src/overlay/registry.rs`.
message_to_agent: |-
    Phase 3 is complete. The library now admits every standalone envelope
    against its compiled definitions (`simplified_schema::envelope`:
    `ENVELOPE_DEFINITIONS`, `admit_envelope_text`, `EnvelopeAdmission`), and
    `kind: yaml-schema` is a standalone envelope (`StandaloneSchemaEnvelope::YamlSchema`)
    refused as a `$schema` target / import source with
    `NotASchemaDocument { reason: NotSchemaReason::SelectedByKind }` (not a new
    SchemaError variant). The library definitions moved to
    `simplified-schema/schemas/{schema,yaml-schema}.yaml`; `darkmatter/schemas/schema-trigger.yaml`
    is still Darkmatter's (now closed, `$schema: schema(non-null)`, no
    description/categories because the trigger parser rejects them). For phase 4:
    prepare a yaml-schema's export with
    `simplified_schema::resolve::prepare_embedded_yaml_schema(path, documents, shipped)`
    then `claimed_values(&resolved, "kind")` (a test already shows the three shipped
    files claim `schema`, `yaml-schema`, `schema-trigger`). There is no file-based
    yaml-schema preparer yet (filesystem candidates that import a `kind: schema`
    file need one; add it beside `prepare_embedded_yaml_schema` in resolve.rs rather
    than calling `prepare_embedded_schema`, which refuses yaml-schemas). Use
    `envelope::admit_envelope_text` for the located definition report of a whole
    YAML file (problems carry line/column; `positions.byte_range(problem)` gives the
    span). The darkmatter corpus class for schema-trigger.yaml is
    `YamlSchemaDefinition` in shipped_schema_corpus.rs. Phase 2's edits are still
    uncommitted in the working tree alongside phase 3's.
---

# Implementation Log for 2026-10-30-yaml-document-schemas (7 phases)

## Phase 1

### 1.1 Preserved docs-escape work

- Wrote `preserved-docs-escape-fix.patch` (336 lines) in this directory: the
  tracked diffs of `darkmatter/docs/inline/schema-validation.md`,
  `darkmatter/docs/topics/schemas/definition.md`,
  `darkmatter/docs/topics/schemas/dmls-schema-support.md`,
  `docs/kind-documents.md`, plus the untracked
  `darkmatter/lib/tests/l1/schema_documentation_pages_compose.rs` via
  `git diff --no-index`.
- `git apply --check` of the patch succeeds against the discarded tree (verified
  right after 1.2, before the revert).

### 1.2 Discarded superseded uncommitted edits

- `git checkout HEAD --` on all 16 tracked modified paths; deleted the untracked
  `schema_documentation_pages_compose.rs`. No wholesale reset was used; the
  untracked `plan.md` and `implementation-log.md` were untouched.

### 1.3 Revert — departure from a whole-commit revert

The revert is **"reverse of four commits minus five carve-outs"** (rule 1), not a
whole-commit revert:

- `git revert --no-commit 8ebb21dd1 56f9e1abf 19184ffe5 2133a089f`.
- Only conflicts: modify/delete on `darkmatter/schemas/schema.yaml` and
  `darkmatter/schemas/schema-trigger.yaml` (edited by later planning commits);
  resolved by the carve-out.
- Restored to `HEAD`: `simplified-schema/lib/src/resolve.rs`,
  `darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs` (inline
  root-union parity fix), `darkmatter/schemas/schema.yaml`,
  `darkmatter/schemas/schema-trigger.yaml`, `darkmatter/schemas/yaml-schema.yaml`
  (the last was never touched by the four commits), and `schemas/kind.yaml`.
- `git revert --quit` then `git reset` so the result is an unstaged working-tree
  change; the orchestrator makes the single revert commit.
- Deleted by the revert: `darkmatter/lib/src/markdown/schemas/kind_documents.rs`,
  `darkmatter/lib/tests/l1/kind_documents.rs`,
  `darkmatter/cli/tests/l1/schema_validate_kind_documents.rs`,
  `darkmatter/dmls/src/{diagnostics,overlay}/kind_document.rs`,
  `darkmatter/dmls/tests/l1/kind_document_editor.rs`.
- Restored by the revert: `darkmatter/docs/schemas/schema-definition.yaml`
  (empty placeholder) and the `$schema:` line of `darkmatter/docs/schemas/err.yaml`
  (rule 4).
- The `mod` declarations for the deleted test modules were removed by the revert
  itself (they were added by the reverted commits), so 1.4 needed no manual edits.

### 1.4 Compiling baseline

- No `mod` edits were needed (see 1.3).
- First full darkmatter `just test` failed in two corpus tests:
  `shipped_schema_corpus::shipped_schema_artifacts_parse_passively` and
  `loadable_shipped_schemas_resolve_and_lower`, because the kept carve-out
  definition files now declare `kind: yaml-schema` (written by planning
  commits after the reverted Rust). This is the risk 1.3 named.
- Fix: classified the three files in `EXCEPTIONS` (see `decisions_for_review`):
  `yaml-schema.yaml` → `Class::Draft { error: "non-null" }`;
  `schema.yaml`, `schema-trigger.yaml` → new `Class::UnclaimedKind` (asserts
  no schema/trigger claim, refused as `./file` target with
  `SchemaError::SchemaDocument`, bodies parse passively). An attempt with
  `Class::NotASchema` failed on `schema-trigger.yaml` (`unknown named type
  match-payload` when its `$schema` is resolved inline without its `types`).
- `shipped_schema_corpus.rs` does not treat `schema.yaml`/`schema-trigger.yaml`
  as shipped (no embedded-catalog expectation), as the plan requires;
  `embedded.rs` no longer embeds them.

### Requirement → test mapping

Phase 1 changes no behavior beyond restoring the pre-feature baseline, so no
new behavior tests were added. The evidence is the existing suites at the
pre-feature expectations:

| Requirement | Evidence |
|---|---|
| Kind-table route removed, inline root-union parity fix kept | `darkmatter::l1 …embedded_sources` (`an_inline_root_union_prepares_and_a_file_arm_is_refused`) passes; `kind_documents` tests deleted |
| `schema-definition.yaml` placeholder and `err.yaml` `$schema:` restored | `shipped_schema_corpus::shipped_schema_artifacts_parse_passively` (`Placeholder`, `EnvelopeRejected` exceptions) |
| Kept YAML definitions stay parseable while unclaimed | the same corpus test via the new exceptions; `loadable_shipped_schemas_resolve_and_lower` asserts `./file` refusal |
| No consumer of removed API | `grep` for `validate_kind_document`, `kind_documents::`, `KindDocument`, `darkmatter:schema[-trigger]` across the repo: no hits |

The Input Robustness Matrix applies to later phases (new loaders); phase 1
adds no parser or loader.

### Gates

- `simplified-schema`: `just lint` clean; `just test` 1113/1113 passed.
- `darkmatter`: `just lint` clean (rerun after the corpus edit);
  `just test` 12884 passed, 10 skipped, 0 failed (second run; first run
  failed as described in 1.4); `just check-tier-coverage darkmatter`: 0
  stranded.
- Affected scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, darkmatter-cli, dmls, simplified-schema, test-toolkit]`,
  `change_class: package` → gates required and run.
- Dependent packages: the only public-API removals are
  `DarkmatterSchemas::validate_kind_document`, the `kind_documents` module, and
  the `darkmatter:schema`/`darkmatter:schema-trigger` embedded stems; a repo
  grep finds no consumer outside darkmatter, so no dependent-package tests
  target the changed symbols and none were run. `simplified-schema`'s only
  change is a module-doc deletion.
- Cross-OS: not run; the phase restores previously tested code and adds no
  path or OS-specific logic.
- Code-smell grep: not applicable in phase 1 (none of the listed new modules
  exist yet).
- No pre-existing or load-sensitive failures observed.

## Phase 2

### 2.1 Constraint model and grammar

- `simplified/types.rs`: new `Constraint` variants `NonNull`, `Closed`,
  `Mapping`, `Property { name, declaration }`, `Forms(Vec<SimplifiedType>)`,
  `DeclaredRequired` with `keyword()` strings; `PropertyAtom::marks_non_null`,
  `PropertyDef::marks_non_null`, and `PropertyDef::non_null_problem` (the one
  declaration check for `non-null` over a null-admitting value).
- `simplified/grammar.rs`: parse arms for `non-null`, `mapping`,
  `declared-required`, `forms(...)` (nonempty list of type keywords, a set),
  and `property(name, type-definition(...))` (dedicated arm under the
  `MAX_TYPE_NESTING` guard); `check_declaration_option` is the allow-list
  (`mapping`/`property` only in `schema`'s value list, `forms`/
  `declared-required` only in `type-definition`'s; arrays and named
  references rejected); `validate_semantic_constraints` admits the options
  and `non-null`; `parse_definition` rejects `non-null` over a null-admitting
  union, located on the marked option.
- `simplified/mod.rs`: `$constraints` gains `closed` and `non-null` flags
  (both must be `true`); a block object's `non-null` moves onto its atom
  like `required`; the YAML-list union spelling gets the same `non-null`
  declaration check.
- `simplified/serialize.rs`, `schema/export.rs`: canonical spellings. Export
  also now writes a nested block object whose atom carries presence flags
  (`required`/`non-null`/`generated`) back as `$constraints` instead of
  refusing it (it refused `required` + `min-keys` before too).
- No exhaustive `match` outside these needed an arm: `cargo check` of
  `simplified-schema`, `darkmatter`, `dmls`, `darkmatter-cli` was clean
  after the variants were added (every other site uses a wildcard; Wave 2
  revisits the wildcard sites rule 9 lists).
- Tests: `simplified-schema/lib/tests/l1/declaration_grammar.rs` (11 tests,
  including checkpoint 2's `include_str!` parse of
  `darkmatter/schemas/yaml-schema.yaml`'s payload).

### 2.2 Closed roots

- `convert.rs::shape_to_object_schema`: a root or root-union arm carrying
  `Closed` lowers with `additionalProperties: false`; `reject_block_constraints`
  admits `closed` at root and inline scope (nested objects were already
  closed, so it is a no-op there). A `<string>` catch-all still admits every
  key, so `closed` changes nothing beside one.
- `resolve.rs::merge_layer` no longer copies a lower layer's
  `additionalProperties: false`; `validate_simple_object_root` admits a
  boolean `additionalProperties` (a closed baseline used to be rejected; the
  unit test pinning that rejection was replaced by one pinning the new rule).
- `json_shape.rs` already maps root `additionalProperties: false` to
  `Additional::Forbidden`, so soft validation reports the same unknown-key
  problem; no change needed.
- Closure "after merge": the language has no root-level spreading of imports,
  so the only way a root gains another source's keys is layering; tests cover
  that.
- Tests: `simplified-schema/lib/tests/l1/closed_roots.rs` (12).

### 2.3 `non-null`

- `convert.rs::atom_to_schema` / `union_property_to_schema`: a marked atom or
  union gets no optional-null wrapper and stays out of `required`;
  `atom_fragment_without_null_wrap` strips the marker before type builders;
  `apply_array_constraints` accepts it.
- `resolve.rs::check_non_null_use` (called after each import expansion)
  rejects `Alias(non-null)` when the resolved definition admits null (inlined
  or a recursive `Ref`, via `UseSummary::admits_null`); markers added to
  `is_union_marker`, `is_flag`, `recursive.rs::is_use_site_marker` /
  `is_arm_property_marker`, `narrowing.rs` (`has_ordinary_constraints`,
  `type_identity`); `grammar.rs::parse_tuple` admits `non-null`.
- **Departures from rule 9** (see `decisions_for_review`): no separate
  coercion-engine pre-check and no `allOf [not null]` wrapper were needed.
- Tests: `simplified-schema/lib/tests/l1/non_null_properties.rs` (17).

### 2.4 Declaration narrowing

- `convert.rs::semantic_type_fragment` + public
  `simplified::declaration_options_payload` build the options payload;
  `format.rs` factories parse it (malformed payload → schema error) and judge
  parse-only (`TypeDefinitionOptions`/`SchemaOptions`), exposed as
  `format::{judge_schema, judge_type_definition}`; `validate.rs` and
  `json_shape.rs` unchanged (the keywords stay `Shape::Any`).
- `forms` itself rejects `literal`/`enum` alternatives that are not non-empty
  strings (spec ruling 1 needs the error through `validate_*`); `claims.rs`
  checks the same again at preparation.
- New `simplified-schema/lib/src/claims.rs`:
  `claimed_values(&ResolvedSchema, &str) -> Result<Vec<String>, ClaimProblem>`,
  `ClaimProblem { property, arm_index, cause: ClaimCause, path }`. It reads the
  resolved JSON lowering, following `$ref` into `$defs` with a cycle guard; a
  reference arm's `path` is written as if inline (callers fall back to the
  nearest existing ancestor), because `ResolvedSchema` does not record which
  arms were references.
- Tests: `declaration_narrowing.rs` (23 + 2 judge-parity tests added in 2.5),
  `claimed_values.rs` (16; file-backed cases gated on the `file` feature).

### 2.5 Host consumers

- `about.rs`: descriptors for `non-null` (every type but `null`/`unknown`),
  `mapping`/`property` (`schema`), `forms`/`declared-required`
  (`type-definition`) in `accepted_constraints`; parity tests extended; new
  `BLOCK_CONSTRAINT_DESCRIPTORS` catalog of `$constraints` keys by placement
  (root/arm: `min-keys`, `max-keys`, `uncounted-keys`, `closed`; property
  object additionally `required`, `non-null`, `generated`) pinned by
  `block_constraint_keys_match_the_lowering`; re-exported from `schemas/mod.rs`.
- DMLS `providers/frontmatter.rs`: `$constraints:` key completion from that
  catalog (did not exist before); type-expression keyword completion picked up
  the new options from the catalog with no code change.
- `call_binding/translate.rs`: `NonNull` is an annotation; optional `non-null`
  properties get no `Null` arm; meta-type payload from the shared builder;
  closed roots mirrored.
- Triggers: `grammar.rs` load-time allow-list admits `non-null` and the four
  options; `matcher.rs` defeats a `non-null` condition on a present `null` and
  judges meta-type options with `judge_*`.
- `schema_projection.rs`, `compose/context/catalog.rs`: no change needed
  (projection rejects meta-type operands; the catalog only projects
  `required`/`generated` display flags).
- Corpus: `darkmatter/schemas/yaml-schema.yaml` reclassified from
  `Class::Draft { error: "non-null" }` to `Class::UnclaimedKind`.
- Tests: `darkmatter/lib/tests/l1/non_null_frontmatter.rs` (4, through
  `DarkmatterSchemas::validate_summarized` in a temp dir; `null` located at
  line 6), `function_call_binding.rs` (2), `coercion_translator_conformance.rs`
  (5), matcher unit tests (2), `about.rs` (1), DMLS frontmatter provider (2).
- Pre-existing, not fixed: plain `schema`/`type-definition` disagree between
  the grammar-side and JSON-Schema-side translators on native scalars (`4`,
  `true`); meta-type rows kept out of the conformance table for that reason.

### 2.6 Language docs

- Updated `simplified-schema/docs/topics/{conversion,custom-vocabulary,language-core,schema-object,validation}.md`
  and `topics/language/{index,objects-and-named-types,schema-types,subtyping,types,unions}.md`
  (types.md carries the Mermaid "when is `null` rejected?" diagram);
  `darkmatter/docs/topics/schemas/definition.md` (a baseline may now be
  closed; it is never copied); `darkmatter/docs/topics/schemas/dmls-schema-support.md`
  (`$constraints` key completion). `schema-documents.md` and `features.md`
  list nothing that changed.
- Skill: `.claude/skills/simplified-schema/SKILL.md` and new
  `declaration-narrowing.md`.

### Requirement → test mapping

| Requirement | Tests |
|---|---|
| New spellings parse, serialize, re-parse; rejections located | `declaration_grammar::*` |
| Draft `yaml-schema.yaml` payload parses (checkpoint 2) | `declaration_grammar::accepted::the_draft_yaml_schema_definition_parses_as_a_schema_payload`; corpus `shipped_schema_corpus` (UnclaimedKind) |
| Closed roots: control, rejection at key, pattern keys, layering, arms, export, nested no-op | `closed_roots::*` |
| `non-null` field-shape matrix (absent, null soft/strict, wrong type, empty, arrays, unions, references, export, subtyping) | `non_null_properties::*` |
| Meta-type declaration options matrix, root-union arms, `mapping` carrier | `declaration_narrowing::*` |
| Claims over resolved arms (alias, import, file arm, cycle, repeats, every cause) | `claimed_values::*` |
| Host catalog parity, DMLS completion, translator, trigger matching | `about.rs` tests, `meta_schema_phase3`, DMLS `providers::frontmatter`, `function_call_binding`, `coercion_translator_conformance`, matcher unit tests |
| End-to-end through Darkmatter's normal validation entry point | `non_null_frontmatter::*` |

Input robustness: `non-null` field shapes (absent / null / wrong type / empty)
are covered once through the shared lowering + coercion path in
`non_null_properties.rs`, and wired through Darkmatter in
`non_null_frontmatter.rs` (control + null). Closed roots: declared / undeclared
/ pattern / layered / per-arm in `closed_roots.rs`. Meta-type options: every
row of the plan's matrix in `declaration_narrowing.rs`. No new file loader was
added this phase, so document shapes do not apply. Code-smell grep over
`claims.rs` and the added lines of `format.rs`, `convert.rs`, `matcher.rs`:
no hits.

### Gates (run once, after all subagent work merged, none running)

- `simplified-schema`: `just test` 1366/1366 (+ minimal leg 1185/1185);
  `just lint` clean.
- `darkmatter`: `just test` 12900 passed, 10 skipped, 0 failed (single run,
  no reruns); `just lint` clean; `just check-tier-coverage darkmatter` and
  `simplified-schema`: 0 stranded.
- Affected scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, dmls, simplified-schema, test-toolkit, darkmatter-cli]`,
  `change_class: package`, so the gates were required and ran.
- Dependents: `Constraint` gained variants (a public enum), so every
  dependent was compiled: `cargo check --all-targets -p claudine -p
  claudine-gen -p claudine-cli -p messenger -p messenger-cli -p research`
  clean (their `Constraint` uses are non-exhaustive). claudine (>200 tests)
  ran its schema/trigger/composition subset:
  `cargo nextest run -p claudine -E 'test(/schema|trigger|composition/)'`
  3230/3230 passed.
- Cross-OS: not run; the phase adds no path, process, or OS-specific code
  (language lowering, validation, and docs only).
- No load-sensitive or pre-existing failures observed in the gate runs.

## Phase 3

### 3.1 `yaml-schema` envelope

- `simplified/standalone.rs`: `StandaloneSchemaEnvelope::YamlSchema` and
  `StandaloneSchemaEnvelope::kind()`; recognition is now pure (claims by `kind`
  or by the informal key set; never rejects). Admission happens in
  `parse_standalone_schema_document`; `parse_standalone_unadmitted` exists only
  for the bootstrap.
- `errors.rs`: `NotSchemaReason::SelectedByKind` (message "it is a `yaml-schema`;
  it is selected by `kind`, not referenced", hint "move shared types into a
  `kind: schema` file…"). Departure from the plan's new `SchemaError` variant:
  see `decisions_for_review`.
- `resolve.rs`: `parse_referenced_schema_document` (public) applies the refusal,
  even for a malformed yaml-schema, at the three sites: `classify_yaml_schema_text`
  (targets, delegations, root-union arms, `inspect_schema_target`),
  `prepare_embedded_schema_in` (embedded targets), `named_types_from_text` (file
  and embedded `Name@file`). `schema/export.rs` writes `kind: yaml-schema`;
  factories load one. `graph.rs`, `inspect.rs`, `layer.rs`, `view.rs` needed no
  change (they go through those sites or handle any envelope).
- DMLS: `standalone_envelope_claim` / `infer_envelope` recognize
  `kind: yaml-schema`; imported files read through
  `parse_referenced_schema_document`; the schema-root summary does not count a
  yaml-schema as a named schema. Darkmatter's function catalog already required
  `Tagged`.

### 3.2 Definitions

- `git mv` of `darkmatter/schemas/{schema,yaml-schema}.yaml` to
  `simplified-schema/schemas/` (then unstaged; the tree shows a delete plus an
  untracked directory, since this phase must not stage). `schema.yaml`: root
  union of two closed arms with non-null metadata and `types: schema(mapping…)`.
  `yaml-schema.yaml` was already final. `schema-trigger.yaml`: closed,
  `$schema: schema(non-null)` (no description/categories: decision recorded).

### 3.3 Admission

- New `simplified-schema/lib/src/envelope.rs`: `ENVELOPE_DEFINITIONS`
  (`include_str!`), `SCHEMA_DEFINITION_PATH`, `YAML_SCHEMA_DEFINITION_PATH`,
  `admit_envelope_text`, `EnvelopeAdmission { envelope, definition, value,
  positions, problems }`. Definitions are prepared once (`OnceLock`) via
  `resolve::lower_embedded_definition` and a private validator cache
  (`Prepared::compiled_through`).
- Deleted hand-written checks: unknown tagged key, "needs `$schema` or `types`",
  tagged/pure `types` not a mapping (recognition and the local-types check),
  non-string `description`, `categories` not a list / entry not a string.
  Kept in Rust: category entry grammar, duplicate/undeclared categories,
  duplicate overloads, suggestion lints.
- Report order (envelope problem > parser-located definition > carrier shape) in
  `standalone.rs::rejected_envelope`; `locate_standalone_definition_failure`
  returns `None` for a malformed envelope. Span: `PositionMap::byte_range`
  (new), cut to the first line.
- Carrier guard: not needed (decision recorded); proven by
  `envelope_admission::soft_conversion_passes_meta_type_carriers_through_untouched`
  and `…payload_carriers_are_never_converted`.
- New `resolve::prepare_embedded_yaml_schema` for preparing a yaml-schema's export.
- Terminal hint of `SchemaDocument` (`errors.rs`) reworded; it described only the
  old two envelopes (drift found by the docs pass).

### 3.4 Existing-test reconciliation

Fixtures whose messages came from deleted checks now assert the definition's
located message (`does not satisfy its definition \`…/schema.yaml\`: \`/types\``):
simplified-schema `schema_target_classification.rs`, `schema_target_inspection.rs`;
darkmatter `schema_extraction_characterization.rs` (loader matrix),
`schema_enhancements_matrix/envelope.rs`, `schema_grammar_descriptors.rs`,
`schemas_named_import_errors.rs`, `suggest_constraint_phase4.rs`,
`shipped_schema_corpus.rs`; CLI `schema_robustness_matrix.rs`,
`schema_validate.rs`. Behavior changes reflected there: a number `description`
or category entry now converts softly (`5` → `"5"`) instead of failing; wrong-type
element rows now use a mapping element. `schema_files.rs`, `schema_object.rs`,
`located_schema_errors.rs` needed no change. One darkmatter test
(`reset_process_caches_…`) exposed the shared-cache pollution, fixed in code.

### 3.5 / 3.6 Tests

- `simplified-schema/lib/tests/l1/yaml_schema_envelope.rs` (10): parse with
  `types` namespace; key set and `kind()`; missing export located; factory load
  and export round trip; embedded target and embedded import refused; target
  inspection; file target, delegation, root-union arm, malformed yaml-schema,
  `Name@file` source refused; `kind: schema` types-only still imports and is
  still not a target; kinded data and triggers stay non-targets.
- `envelope_admission.rs` (16): library definitions admitted (yaml-schema by
  itself); `schema-trigger.yaml` admitted; each shipped definition claims its
  kind via `prepare_embedded_yaml_schema` + `claimed_values`; controls for
  informal/tagged/yaml-schema; per-control rows: unknown key (at the key),
  `description|categories|types|$schema: null` (at `null`), `types: [a, b]`,
  neither payload, types-only (valid for `kind: schema` only), number
  description converts, carriers never converted, coercion-engine carrier
  pass-through, non-text category entry; content factory, embedded and file
  import sources admit first; report-order precedence.
- `shipped_corpus.rs`: both library definitions pass the compiled definitions
  (own `include_str!`); source guard that the deleted check messages are gone
  from `standalone.rs` and recognition never returns `Err(`.

### 3.7 Docs

Documentation subagent updated `simplified-schema/docs/topics/language/schema-documents.md`
(three envelopes, `yaml-schema` section, admission with Mermaid load path,
reporting order, Rust-only checks), `selecting-schema-files.md`, `diagnostics.md`,
`schema-object.md`, `language-core.md`, `language/index.md`,
`language/local-type-references.md`. Skills: darkmatter `SKILL.md`, `schema.md`;
simplified-schema `SKILL.md`.

### Requirement → test mapping

| Requirement | Tests |
|---|---|
| `yaml-schema` parses as a standalone document with metadata and `types` | `yaml_schema_envelope::a_yaml_schema_parses_as_a_standalone_document_with_a_types_namespace` |
| Reached only through `kind` (file target, delegation, arm, import, embedded target/import, inspection, malformed) | `yaml_schema_envelope::*` (7 tests), darkmatter corpus `YamlSchemaDefinition` |
| `types`-only `kind: schema` import/non-target; kinded data, triggers non-targets | `yaml_schema_envelope::files::a_yaml_schema_is_not_a_named_type_import_source`, `…kinded_data_and_triggers_stay_non_targets` |
| Bootstrap: definitions admitted, yaml-schema by itself, claims | `envelope_admission::the_library_definitions_are_admitted_…`, `…each_shipped_definition_claims_its_kind`, `shipped_corpus::the_library_envelope_definitions_pass_…` |
| Every envelope row of the plan (unknown key, nulls, list types, neither, types-only, soft description, carriers unchanged, informal parity) | `envelope_admission::*` |
| Every direct load admits (factories, file and embedded `Name@file`) | `envelope_admission::the_content_factory_…`, `…embedded_import_source_…`, `…file_import_source_…` |
| No structural check left in Rust | `shipped_corpus::the_standalone_parser_carries_no_hand_written_structural_checks` |
| Reporting order and locations | `envelope_admission::the_envelope_s_own_problem_outranks_…`, `schemas_source_projection::outer_envelope_and_declaration_shape_failures_return_none` |
| End-to-end through `md` | CLI `schema_robustness_matrix::types_catalog_declaration_column`, `schema_validate::schema_validate_rejects_indentless_types_sequence` (real `md` binary) |

Input robustness: one loader (`load_admitted_yaml` behind recognition; document
shapes unchanged and still covered by `yaml_loader_matrix` and
`schema_target_classification::yaml_document_shapes_…`). Field shapes go
through one validator (the definitions): `description`, `categories`, `types`,
`$schema` each have absent/null/wrong-type/empty rows on the informal and tagged
controls, plus control + invalid rows on the yaml-schema control. Exceptions:
a number scalar converts to text (soft); `types`-only is valid for
`kind: schema`. Code-smell grep over `envelope.rs`, `standalone.rs`, and the
added lines of `resolve.rs`, `positions.rs`, `source.rs`: one `.ok()?` in the
locator, whose contract is `Option` (the classifier reports the error).

### Gates (run once each, after all edits, nothing else running)

- `simplified-schema`: `just test` 1394/1394 + minimal leg 1208/1208;
  `just lint` clean.
- `darkmatter`: `just test` 12900 passed, 10 skipped, 0 failed (run 3, after
  the fixture reconciliation; runs 1–2 were the discovery runs that found the
  24 fixtures and the cache-count test). `just lint` clean.
  `just check-tier-coverage darkmatter` / `simplified-schema`: 0 stranded.
- After the final `errors.rs` hint edit: simplified-schema error/terminal tests
  (49) and darkmatter-cli schema tests (425) re-run, all passed.
- Affected scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, darkmatter-cli, dmls, simplified-schema, test-toolkit]`,
  `change_class: package` (includes phase 2's uncommitted edits); gates required and run.
- Dependents: `cargo check --all-targets -p claudine -p claudine-gen -p claudine-cli -p messenger -p messenger-cli -p research`
  clean; claudine subset `cargo nextest run -p claudine -E 'test(/schema|trigger|composition|envelope/)'`
  3244/3244 passed.
- Cross-OS: not run. No path, process, or OS-specific code was added (only
  `/`-joined virtual-path constants and a relative `include_str!`).
- No load-sensitive or pre-existing failures observed.
