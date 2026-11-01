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
    - messenger
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
    - |-
        Phase 4: the three shipped `yaml-schema` definitions live in their own
        table (`embedded::shipped_kind_definitions()`: the library's two plus
        `schema-trigger.yaml`), not in `embedded::SHIPPED` as the plan said.
        `SHIPPED` is the catalog-and-partials set behind `Name@darkmatter:` and
        an exact-table test; mixing the kind definitions in would make them look
        like importable partials. Alternative: append to `SHIPPED`. Redo cost:
        one table entry and the table test.
    - |-
        Phase 4: an unreadable schema root makes every document with a string
        `kind` a selection error (`RootUnreadable`), even one a healthy shipped
        definition claims, because the unread root could hold a duplicate
        claim. Documents without a `kind` are unaffected. The plan listed the
        cause but not which documents it reaches. Alternative: only unclaimed
        kinds are affected (like an incomplete registry). Redo cost: one branch
        in `yaml_documents/select.rs` and one test.
    - |-
        Phase 4: a file in a schema root that is not valid YAML counts as a
        failed definition (registry incomplete) only when a lexical scan shows
        it declaring `kind: yaml-schema` at the top level (the same scanner
        trigger discovery uses for `kind: schema-trigger`). An unrelated broken
        YAML file, such as a mid-edit trigger, does not make every unclaimed
        kind "unavailable". The plan's matrix said "malformed YAML in a root
        reveals nothing -> unavailable" without this qualification. Alternative:
        any malformed YAML file marks the registry incomplete. Redo cost: one
        line in `registry.rs::Candidate::classify` and one test.
    - |-
        Phase 4: the `schema` kind check does not move the category checks out
        of the library parser (plan rule 22 said "moved from
        check_named_types"); it runs the library's own parse
        (`parse_standalone_schema_document`) and then resolves the document's
        named-type imports (`check_standalone_named_types`), reporting the
        first failure as a located `kind_check` problem. Phase 3 kept those
        checks in the library so direct library loads keep enforcing them;
        moving them would have dropped that. Alternative: move them and have
        library loads skip them. Redo cost: `semantic.rs::schema_checks` plus the
        library parser.
    - |-
        Phase 4: trigger grammar errors are located after the fact:
        `TriggerMatch` gained `span`, but the grammar's constructors still pass
        `None`; `triggers::envelope::locate_trigger_error` reads the field the
        message already starts with (`` `match.all[0].$path`: ``, written only by
        the grammar's own `located` helper) and looks it up in the document's
        position map. Threading positions through every grammar function would
        have touched ~20 constructors for the same result. Errors without a
        field (unknown envelope keys, mixed mappings) are located at `match` or
        the root. Alternative: pass node spans into each constructor. Redo cost:
        the grammar module's constructors and `trigger_error_segments`.
    - |-
        Phase 4: the YAML route validates with no template deferral at all: a
        `{{ ... }}` string nested at a place the schema marks `deferred` is
        judged as text too (Markdown frontmatter defers those). The spec says
        these strings are ordinary YAML data. Alternative: keep deferral at
        `deferred` places. Redo cost: one argument in
        `EffectiveSchema::validate_data`.
    - |-
        Phase 4: the failed-definition, duplicate, and unavailable selection
        errors carry the definition's defect as text (`defect: String` in
        `KindSelectionCause`), not the typed `SchemaError`, so the cause stays
        `Clone + PartialEq` for tests and rendering. The typed defect is still
        available from `KindRegistry::claim` / `incomplete`. Redo cost: the
        enum's field types and their Display.
    - |-
        Phase 4: the duplicate-key load error is located where the YAML parser
        reports it (the start of the mapping holding the key), not on the
        second occurrence. The parser gives no other position, and the
        position map cannot represent a mapping with a repeated key. Redo cost:
        a lexical scan for the repeated key in `yaml_documents/load.rs`.
    - |-
        Phase 5: to tell a YAML file's own load error (exit 3) from a load error
        of the schema file its `$schema` names (exit 2), the library entry point
        gained a two-step form, `LoadedYamlDocument::load` then
        `validate_loaded_yaml_document`; `validate_yaml_document` still does
        both. The previous phase suggested comparing the error's file path with
        the document's path, which misclassifies a document whose `$schema`
        names a file at that same path, or a dedicated error variant, which
        would add a loader-only variant to the shared `SchemaError` enum. Redo
        cost: the CLI's `validate_yaml` and one library test.
    - |-
        Phase 5: an argument that does not open has no resolved path, so the
        preflight judges whether it is YAML by its spelling (`missing.yaml`).
        That decides whether a batch "has YAML" (the exit-64 checks) and
        whether the baseline is loaded. The plan only says "classify by the
        resolved path". Alternative: treat an unopened argument as Markdown,
        which would load (and possibly fail on) the baseline for a batch of
        only YAML names. Redo cost: one expression in `run_validate`.
    - |-
        Phase 5: the JSON record's `schema` field for a YAML file is what
        validated it (`YamlValidation::definition`): the definition label such
        as `built-in simplified-schema/schemas/schema.yaml` on the kind route,
        the `$schema` reference, or the text "the inline `$schema`" for an
        inline one. For Markdown it is the document's `$schema` string or null.
        Alternative: null on the kind route and for inline schemas, matching
        Markdown exactly. Redo cost: one match in `validate_yaml`.
    - |-
        Phase 5: a YAML load error is reported as "is not a valid YAML
        document:" with the parser's message and `(at line L, column C)` from
        the span (JSON `error: "yaml_parse"` plus `line`/`column`), and a file
        that cannot be read is the same outcome (exit 3) without a location.
        Redo cost: wording in `emit_pretty`/`emit_json`.
    - |-
        Phase 6: each YAML document's kind registry is stored inside that
        document's existing trigger-registry entry (built in the same
        background preparation, from the same schema-root reads), not in a
        separate `OverlayCache.kind_registries` map as the plan named it. It
        therefore shares the trigger registry's invalidation (watched
        events, saves, rescans, configuration and manifest changes) and its
        per-context identity for free. Alternative: a parallel cache with
        its own invalidation, which would duplicate that logic. Redo cost:
        move `Scope.kinds` and `set_buffer` into a new map in
        `overlay/registry.rs`.
    - |-
        Phase 6: a kind registry reads an open, unsaved definition buffer in
        place of its file (the spec requires unsaved definition edits to
        refresh open YAML documents), while trigger registries still read
        only saved files, as before. Only a registry that actually read that
        file with different text is prepared again. Alternative: also make
        triggers honor unsaved buffers, a wider behavior change outside this
        feature. Redo cost: none for this feature.
    - |-
        Phase 6: an informal standalone schema file (a YAML file with a root
        `$schema` and no `kind`, the editor's existing schema-authoring
        documents) keeps only its envelope diagnostics in DMLS; the YAML
        document judgment is shown only when its `kind` selects a
        definition. The library (and so `md schema validate`, since Phase 5)
        takes the root-`$schema` route for such a file and judges the file as
        data against its own definition, e.g. "title is required" for
        `$schema: {title: string(required)}`. So DMLS and `md` disagree for
        these files. Alternatives: (a) publish the library judgment in DMLS
        too, which flags every informal schema file in a workspace with
        nonsense errors and broke about ten existing editor tests; (b) change
        the library selection so a recognized informal envelope is "no
        schema", fixing `md` too, but contradicting the spec's literal rule
        ("a root `$schema` selects"). Recommendation: (b), decided by the
        reviewer. Redo cost: (b) is one rule in `yaml_documents/select.rs`
        plus its tests; the DMLS guard in `overlay/mod.rs` can then go.
    - |-
        Phase 6: in a claimed `kind: schema` / `kind: schema-trigger` file
        each problem is published once, by these rules: a definition problem
        (anything but a kind check or a meta-type problem) belongs to the
        YAML report, so the envelope's whole-document error
        (`dm.schema.document_malformed` / `dm.schema.prepare`) is not shown
        beside it; a trigger grammar failure is the report's
        `dm.schema.kind_check` on the offending node, not the envelope's
        whole-file `dm.schema.prepare`; a standalone schema's own named-type
        failure keeps its envelope diagnostic (with related locations) and
        the duplicate kind check is dropped; a meta-type problem keeps the
        envelope's token-located error. Three existing tests changed
        expectations accordingly (`schema_roots_parity` forbidden prefix,
        `lsp_session` block-plain-flow-indicator, `suggest_constraint_phase1`
        malformed tagged envelope). Alternative: keep the envelope codes
        everywhere and drop the report's problems, losing precise ranges.
        Redo cost: the filters in `diagnostics/yaml_document.rs` and the two
        guards in `diagnostics/frontmatter.rs`.
    - |-
        Phase 6: a YAML file that does not load gets one parse diagnostic:
        the loader's `dm.frontmatter.yaml_parse` (source
        `darkmatter.frontmatter`) for ordinary YAML, but the envelope's own
        established code (`dm.schema.prepare` for a trigger,
        `dm.schema.document_malformed` for a standalone schema) when a schema
        envelope claims the text, since those codes are documented as stable
        for clients. Alternative: always the loader's code, which changes two
        established codes. Redo cost: one guard in
        `diagnostics/yaml_document.rs` and two existing tests.
    - |-
        Phase 6: in a whole YAML document an undeclared key
        (`dm.schema.unknown_key`) is an error even without `[schema] strict`,
        like a missing required key (the plan named only the latter). Both
        make `md schema validate` exit 1 and nothing later supplies or
        strips a key. Alternative: keep the frontmatter rule (warning unless
        strict). Redo cost: one arm in `classify`.
    - |-
        Phase 6: the editor test helper pattern `latest = wait(); while let
        Some(p) = take_buffered() { latest = p }` returned the OLDEST
        buffered publication, because `take_buffered_diagnostics` takes the
        newest first. It was copied in seven places in six DMLS test files;
        all seven now keep the first (newest) and drop the rest. All 147
        affected tests still pass, so none depended on the old result.
        Redo cost: none.
    - |-
        Phase 6: `docs/lsp/features.md` and `docs/lsp/frontmatter.md`, which
        the plan listed for 6.6, were not changed: the first is a proposed
        feature inventory and the second describes Markdown frontmatter keys;
        the YAML behavior is documented in `dmls-schema-support.md` (new
        "YAML Documents" section), `lsp/indexing.md` ("YAML document kinds"),
        `lsp/configuration.md`, and the two `dmls/docs` pages.
    - |-
        Phase 7: Messenger's research loader now reads the roster, overrides,
        and mappings files as whole YAML documents (the same library call
        `md schema validate` makes), not as Markdown frontmatter, and all 26
        YAML test fixtures lost their `---` fences. The plan asked only that
        consumers of platforms.yaml be updated. Because one loader serves all
        three YAML contract files, switching it moved all three. Alternative:
        accept both the fenced and the unfenced form. That keeps two formats
        alive, and the fenced form already fails `md schema validate`. Redo
        cost: one branch in `load.rs` and restoring the fixture fences.
    - |-
        Phase 7: the repository YAML corpus does not pass for 11 files, and
        I left them unedited. They are Claudine design sketches in
        `claudine/docs/schemas/` (action, flow-control, lifecycle, loop,
        stack), which the existing corpus and golden manifest already pin as
        drafts that do not parse; four drafts inside a closed Claudine
        feature directory; the frozen golden-test copy of `err.yaml`; and
        `docs/drafts/memory.yaml`. The plan said to fix documents until all
        pass. Fixing the sketches means designing Claudine's lifecycle
        schemas, and editing snapshots rewrites history. Alternative: delete
        them or rewrite them into valid grammar. Redo cost: per file, plus
        updating the corpus exception table and golden manifest.
    - |-
        Phase 7: `darkmatter/docs/schemas/err.yaml` lost `name: err` (the
        closed schema envelope has no `name`) and its `$schema:
        ./schema-definition.yaml` line (under `kind: schema` that line
        exported an empty schema). I deleted the empty placeholder
        `schema-definition.yaml`, and `authoring-schemas.md` now transcludes
        the real definition, `simplified-schema/schemas/schema.yaml`. Four
        tests that pinned the old state were updated. One of them
        (`meta_schema_phase4`) also now reads `$schema` declarations from
        `darkmatter/schemas/`, because `docs/schemas` no longer has one. The
        plan's rule 4 had restored the placeholder as the pre-feature state
        and left 7.3 to reconcile it. Alternative: keep the placeholder and
        the `$schema` line, and only drop `name`. Redo cost: restore one
        file and four test expectations.
source_files_during_phase_4:
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/registry.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/select.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/semantic.rs
    - darkmatter/lib/tests/l1/kind_semantic_checks.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/trigger_envelope_claim.rs
    - darkmatter/lib/tests/l1/yaml_document_loading.rs
    - darkmatter/lib/tests/l1/yaml_document_selection.rs
    - darkmatter/lib/tests/l1/yaml_kind_registry.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/validation.rs
    - simplified-schema/lib/tests/l1/yaml_schema_envelope.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/index.md
    - darkmatter/docs/topics/schemas/schema-discovery.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
    - simplified-schema/docs/topics/diagnostics.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_5:
    - darkmatter/cli/src/args/target.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_yaml.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/lib/tests/l1/yaml_document_loading.rs
docs_updated_during_phase_5:
    - darkmatter/cli/README.md
    - darkmatter/docs/cli/index.md
    - darkmatter/docs/cli/policy.md
    - darkmatter/docs/cli/report-json.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/schema-discovery.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/cli.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_6:
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/diagnostics/yaml_document.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/frontmatter.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/references.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/yaml_document.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/workspace/discover.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/l1/call_check_agreement.rs
    - darkmatter/dmls/tests/l1/collection_editor.rs
    - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
    - darkmatter/dmls/tests/l1/invalid_trigger_warnings.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/recursive_call_check.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - darkmatter/dmls/tests/l1/standalone_stdio.rs
    - darkmatter/dmls/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
    - darkmatter/dmls/tests/l1/yaml_document_editor.rs
    - darkmatter/dmls/tests/l1/yaml_document_session.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
docs_updated_during_phase_6:
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_7:
    - claudine/docs/research/provider-schema.yaml
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/docs/schemas/err.yaml
    - darkmatter/docs/schemas/schema-definition.yaml
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - messenger/docs/platforms.yaml
    - messenger/lib/src/research/error.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/tests/fixtures/research/contract/mappings-valid.yaml
    - messenger/lib/tests/fixtures/research/contract/overrides-valid.yaml
    - messenger/lib/tests/fixtures/research/contract/roster-cap-10.yaml
    - messenger/lib/tests/fixtures/research/contract/roster-refresh-interval-max.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/mapping-missing-fingerprints.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/mapping-sha256.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/override-missing-review-by.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-cap-11.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-cap-above-ceiling.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-platform-refresh-interval-above-max.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-refresh-interval-above-max.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-retained-without-contribution.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-approved-without-approver.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-duplicate-source.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-over-configured-cap.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-source-foreign-interface.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--adapter-platform-mismatch.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--proposed-as-implemented.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--unassessed-without-reason.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--expired-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--orphaned-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--stale-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-adapter-mapped-twice.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-companion-as-adapter.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-email-active.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-missing-adapter.yaml
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_publication.rs
    - messenger/lib/tests/research_validation.rs
    - schemas/kind.yaml
docs_updated_during_phase_7:
    - darkmatter/README.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - docs/kind-documents.md
    - simplified-schema/README.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/cli.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/testing.md
    - .claude/skills/simplified-schema/SKILL.md
source_code:
    - claudine/docs/research/provider-schema.yaml
    - darkmatter/cli/src/args/target.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/schema_validate_kind_documents.rs
    - darkmatter/cli/tests/l1/schema_validate_yaml.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/kind_document.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/diagnostics/yaml_document.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/frontmatter.rs
    - darkmatter/dmls/src/overlay/kind_document.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/references.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/overlay/yaml_document.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/workspace/discover.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/l1/call_check_agreement.rs
    - darkmatter/dmls/tests/l1/collection_editor.rs
    - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
    - darkmatter/dmls/tests/l1/invalid_trigger_warnings.rs
    - darkmatter/dmls/tests/l1/kind_document_editor.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/recursive_call_check.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - darkmatter/dmls/tests/l1/standalone_stdio.rs
    - darkmatter/dmls/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
    - darkmatter/dmls/tests/l1/yaml_document_editor.rs
    - darkmatter/dmls/tests/l1/yaml_document_session.rs
    - darkmatter/docs/schemas/err.yaml
    - darkmatter/docs/schemas/schema-definition.yaml
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/kind_documents.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/registry.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/select.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/semantic.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/function_call_binding.rs
    - darkmatter/lib/tests/l1/kind_documents.rs
    - darkmatter/lib/tests/l1/kind_semantic_checks.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/non_null_frontmatter.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/trigger_envelope_claim.rs
    - darkmatter/lib/tests/l1/trigger_spelling_guard.rs
    - darkmatter/lib/tests/l1/yaml_document_loading.rs
    - darkmatter/lib/tests/l1/yaml_document_selection.rs
    - darkmatter/lib/tests/l1/yaml_kind_registry.rs
    - darkmatter/schemas/schema-trigger.yaml
    - messenger/docs/platforms.yaml
    - messenger/lib/src/research/error.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/tests/fixtures/research/contract/mappings-valid.yaml
    - messenger/lib/tests/fixtures/research/contract/overrides-valid.yaml
    - messenger/lib/tests/fixtures/research/contract/roster-cap-10.yaml
    - messenger/lib/tests/fixtures/research/contract/roster-refresh-interval-max.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/mapping-missing-fingerprints.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/mapping-sha256.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/override-missing-review-by.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-cap-11.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-cap-above-ceiling.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-platform-refresh-interval-above-max.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-refresh-interval-above-max.yaml
    - messenger/lib/tests/fixtures/research/negative/schema/roster-retained-without-contribution.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-approved-without-approver.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-duplicate-source.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-over-configured-cap.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-curated--roster-source-foreign-interface.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--adapter-platform-mismatch.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--proposed-as-implemented.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--unassessed-without-reason.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--expired-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--orphaned-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--stale-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-adapter-mapped-twice.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-companion-as-adapter.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-email-active.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-missing-adapter.yaml
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_publication.rs
    - messenger/lib/tests/research_validation.rs
    - schemas/kind.yaml
    - simplified-schema/lib/src/claims.rs
    - simplified-schema/lib/src/envelope.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/positions.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/recursive.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/schema/mod.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/narrowing.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/validation.rs
    - simplified-schema/lib/tests/l1/claimed_values.rs
    - simplified-schema/lib/tests/l1/closed_roots.rs
    - simplified-schema/lib/tests/l1/declaration_grammar.rs
    - simplified-schema/lib/tests/l1/declaration_narrowing.rs
    - simplified-schema/lib/tests/l1/envelope_admission.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/non_null_properties.rs
    - simplified-schema/lib/tests/l1/schema_target_classification.rs
    - simplified-schema/lib/tests/l1/schema_target_inspection.rs
    - simplified-schema/lib/tests/l1/shipped_corpus.rs
    - simplified-schema/lib/tests/l1/yaml_schema_envelope.rs
    - simplified-schema/schemas/schema.yaml
    - simplified-schema/schemas/yaml-schema.yaml
documentation:
    - darkmatter/README.md
    - darkmatter/cli/README.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/docs/cli/index.md
    - darkmatter/docs/cli/policy.md
    - darkmatter/docs/cli/report-json.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/lsp/configuration.md
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/schemas/index.md
    - darkmatter/docs/topics/schemas/schema-discovery.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
    - docs/kind-documents.md
    - simplified-schema/README.md
    - simplified-schema/docs/topics/conversion.md
    - simplified-schema/docs/topics/custom-vocabulary.md
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/language/local-type-references.md
    - simplified-schema/docs/topics/language/objects-and-named-types.md
    - simplified-schema/docs/topics/language/schema-documents.md
    - simplified-schema/docs/topics/language/schema-types.md
    - simplified-schema/docs/topics/language/subtyping.md
    - simplified-schema/docs/topics/language/types.md
    - simplified-schema/docs/topics/language/unions.md
    - simplified-schema/docs/topics/schema-object.md
    - simplified-schema/docs/topics/selecting-schema-files.md
    - simplified-schema/docs/topics/validation.md
completed_phase: 7
implemented: true
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

## Phase 4

### 4.1 Shipped table

- `embedded.rs`: `SCHEMA_TRIGGER_DEFINITION_PATH` and `shipped_kind_definitions()`, the
  library's `ENVELOPE_DEFINITIONS` plus `include_str!` of
  `darkmatter/schemas/schema-trigger.yaml`. Kept separate from `SHIPPED` (the
  catalog/partials table behind `Name@darkmatter:`); decision recorded.
  `schema-trigger.yaml` already had its final text from phase 3.

### 4.2 Load, selection, entry point

- New module `darkmatter/lib/src/markdown/schemas/yaml_documents/`
  (`mod.rs`, `load.rs`, `registry.rs`, `select.rs`, `semantic.rs`; the
  plan's `validate.rs` content lives in `mod.rs`).
- `DarkmatterSchemas::validate_yaml_document(text, path) -> Result<YamlOutcome, SchemaError>`,
  `with_kind_registry(Arc<KindRegistry>)`; `YamlOutcome::{NoSchema, Validated(YamlValidation)}`,
  `YamlValidation { route, report, definition }` with `byte_range(problem)`;
  `Route::{Kind(String), RootSchema}`. Re-exported from `markdown::schemas`.
- `EffectiveSchema::for_yaml_document` (no baseline, no triggers) and
  `EffectiveSchema::validate_data` (soft copy, no template deferral: nested
  templates at `deferred` places are judged too).
- Loader: strips one BOM (offset kept for spans; BOM+CRLF parsed fine),
  `load_admitted_yaml` (duplicate keys, second document, numbers), plus a
  non-string-key walk; `---`/`...` accepted.
- Library (`simplified-schema`): `SchemaError::KindSelection { kind, cause: KindSelectionCause }`
  (Display + terminal rendering), `ValidationProblemCode::KindCheck` with
  `SCHEMA_KIND_CHECK_CODE = "dm.schema.kind_check"`, and
  `resolve::prepare_yaml_schema_text` (the file/text-based yaml-schema
  preparer the phase-3 message asked for). DMLS `codes::SCHEMA_KIND_CHECK`
  and its `classify` arm were added so DMLS compiles (error severity).

### 4.3 Registry

- `KindRegistry::{build, from_filesystem, shipped, claim, claims, incomplete,
  root_failures, roots, dependencies, searched_locations}`; `RegistryScope::{ShippedOnly, WithRoots, for_context}`;
  `Claim::{Healthy, Duplicate, Reserved}`; `DefinitionSource::{Shipped, File}`.
  Build never fails. Shipped definitions prepared once per process.
- Candidate = parsed top-level `kind: yaml-schema`, or unparseable text the
  trigger envelope's lexical scanner (generalized to
  `triggers::envelope::lexically_declared_kind`) shows declaring it.
- Source-path identity: file path relative to `repository_root()` after
  `canonicalize_simplified` on both sides, compared as `/`-joined normal
  components.
- The context-construction guard rejected an `Option<&FileResolutionContext>`
  in my first draft; restructured into a private `Builder` whose `discover`
  takes a required context.

### 4.4 Semantic checks

- `semantic.rs`: `schema` (library parse + `check_standalone_named_types`),
  `schema-trigger` (`parse_trigger_envelope` on the loaded value),
  `yaml-schema` (`prepare_yaml_schema_text` + `claimed_values`). Problems are
  `KindCheck`, located by instance path through the document's `PositionMap`
  or by the library's span.
- `SchemaError::TriggerMatch` gained `span`; constructors pass `None`;
  `triggers::envelope::locate_trigger_error` fills it from the message's
  leading `` `field` `` (`trigger_error_segments`), and Markdown discovery
  (`scan_with_source`) now locates the cause and positions the finding
  (`InvalidTriggerFinding::located_in`). Warning severity there is unchanged.

### 4.5 Guard + corpus

- `shipped_schema_corpus.rs`: `every_shipped_yaml_schema_definition_is_in_the_shipped_table`
  and `every_kinded_schema_artifact_passes_its_definition_through_the_registry`
  (one context per corpus folder, real checkout, no home; asserts no duplicate
  claims and that `darkmatter/schemas` uses the checkout copy of
  `schema-trigger.yaml`). 1.5 s.
- `meta_schema_phase*.rs`, `trigger_spelling_guard.rs`,
  `schema_language_ownership_guard.rs` needed no change (all guard tests pass).

### Checkpoint 4 ad-hoc check

A temporary test validated `messenger/docs/platforms.yaml` with its `---`
lines removed through `validate_yaml_document`: route `RootSchema`, valid;
`roster_version: two` fails at line 20, column 17 ("1 was expected"). The
test was removed afterwards (phase 7 owns the data file and its CLI check).

### Requirement → test mapping

| Requirement | Tests (darkmatter `lib/tests/l1/` unless noted) |
|---|---|
| Shipped registry claims `schema`, `yaml-schema`, `schema-trigger`; each shipped file passes `yaml-schema.yaml` (itself included) | `yaml_kind_registry::the_shipped_registry_claims_…`, `…each_shipped_definition_validates_against_the_shipped_yaml_schema_definition` |
| Claim in each of the five roots (home via the fixture `HOME`) | `yaml_kind_registry::a_definition_in_each_of_the_five_roots_claims_its_kind` |
| Duplicate naming both files, order-independent; repo file claiming a shipped kind; shipped filename elsewhere | `…two_definitions_claiming_one_kind_are_a_duplicate…`, `…a_repository_file_claiming_a_shipped_kind_is_a_duplicate`, `…a_copy_with_the_shipped_file_name_elsewhere_is_a_duplicate` |
| Same filename, different definition still a candidate; exact-case kinds | `…a_same_named_file_in_a_later_root_is_still_a_candidate`, `…kinds_compare_exactly_including_case` |
| Modified checkout copy replaces the built-in and changes validation with no Rust change (acceptance) | `…a_modified_copy_at_the_shipped_path_replaces_the_built_in_definition`; copy claiming another kind: `…is_an_ordinary_candidate` |
| Ruling 9: reserved claim (exit-2 error naming file; others unaffected; defect reported once in the file) | `…a_definition_that_fails_to_import_reserves_its_inline_claim` |
| Ruling 9: malformed definition → unavailable for unclaimed kinds only; unrelated broken file ignored; unreadable root (unix) | `…malformed_yaml_that_claims_yaml_schema_makes_unclaimed_kinds_unavailable`, `…an_unrelated_malformed_file_in_a_root_leaves_the_registry_complete`, `…an_unreadable_root_is_an_error_never_an_empty_registry` |
| `ShippedOnly` outside a repository | `…a_registry_built_outside_a_repository_holds_the_shipped_kinds_only` |
| `kind: string` fails on the `kind` line; root-union claims and an optional arm | `…a_yaml_schema_whose_kind_is_a_plain_string_fails_on_the_kind_line`, `…root_union_definitions_claim_each_arm_s_kinds_and_reject_an_optional_arm` |
| Registry reuse, dependencies, searched locations | `…a_registry_can_be_shared_by_every_document_of_its_context` |
| Document shapes (loader matrix) | `yaml_document_loading::*` (9: control, duplicate key, non-string key, inadmissible number, malformed, second document, markers, BOM/CRLF/BOM+CRLF/Unicode spans, scalar/sequence/empty roots) |
| Route precedence; `darkmatter.yaml` and triggers judged by their definition only | `yaml_document_selection::a_claimed_kind_wins…`, `…the_global_catalog_is_validated_as_a_schema_document…`, `…a_trigger_is_validated_by_its_definition_never_against_its_payload`, `…an_unclaimed_kind_falls_back…` |
| Fallback `$schema` stripped from the copy only; positions unchanged | `…the_fallback_schema_is_stripped_from_the_validation_copy_only` |
| No baseline/trigger layering | `…no_baseline_or_trigger_layer_applies_to_a_yaml_document` |
| Soft conversion, nothing written back; `{{ }}`/`$()` are data | `…validation_is_soft_and_writes_nothing_back`, `…template_and_shell_strings_are_data` |
| `kind` field shapes; fallback `$schema` field shapes (null, "", number, boolean, `[123]`, `["a", 123]`; every valid form; trigger file refused) | `…a_kind_that_is_not_a_string_claims_nothing`, `…a_present_but_invalid_fallback_schema_is_a_schema_error…`, `…every_valid_fallback_schema_form_is_a_contract`, `…a_fallback_schema_naming_a_trigger_file_is_refused` |
| `non-null` wiring of `description`/`categories`/`types`/`$schema` (yaml-schema) and `$schema` (trigger) | `…null_metadata_and_payloads_fail_the_shipped_definitions_on_their_line` |
| Semantic checks: glob on the glob, empty/vacuous match, `min-match` bounds, undeclared category, unresolvable import, definition failure stops checks, user kind has none | `kind_semantic_checks::*` (9) |
| Markdown discovery keeps the warning, now positioned | `trigger_discovery_recovery::an_invalid_path_glob_is_a_finding_positioned_on_the_glob` |
| Guard: every on-disk `yaml-schema` is in the shipped table | `shipped_schema_corpus::every_shipped_yaml_schema_definition_is_in_the_shipped_table` |
| Corpus through the registry with source-path identity, no manufactured duplicates | `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` |
| File-based yaml-schema preparer; `KindSelection` message | simplified-schema `yaml_schema_envelope::files::a_yaml_schema_on_disk_prepares_its_export_with_file_imports`, `yaml_schema_envelope::a_kind_selection_error_names_the_kind_and_its_cause` |

Fidelity: F1 in-process at the crate's public API (`validate_yaml_document`,
`KindRegistry`) over real temp-dir files and the real shipped definitions;
the corpus test reads the real checkout. The CLI and DMLS boundaries are
phases 5 and 6 (no surface calls the entry point yet), so no spawned-binary
test belongs to this phase.

Input robustness: one loader (`load.rs`), document shapes tested once in
`yaml_document_loading.rs`. Field validators: selection `kind` (`select.rs`,
every shape), fallback `$schema` (the frontmatter resolver, every shape),
`non-null` (the coercion engine, phase 2) wired per field, closed roots
(phase 2) wired through the roster and trigger definitions. Exceptions to
the default outcome: absent/null/non-string/empty `kind` claims nothing;
scalar/sequence/empty roots are `NoSchema`; markers, BOM, CRLF accepted.
Code-smell grep over `yaml_documents/*` and `triggers/envelope.rs`: no
`serde(default)`; the `unwrap_or_default`/`.ok()` hits are on non-load-bearing
paths (an already-string key, a non-repository path, an `enum` with no
members, a `u32` overflow).

### 4.6 Docs

Documentation subagent: `darkmatter/docs/topics/schemas/definition.md` (new
"YAML Documents" section: selection flowchart, loading, validation, kind
checks, selection errors, library API), `authoring-schemas.md` ("Defining a
YAML Document Kind"), `schema-discovery.md` (was an empty stub: discovery of
kind definitions, claims vs shadowing, source-path identity, failure policy),
`schema-triggers.md` (two severities; where a failure is positioned). I then
corrected `index.md`'s stale entry for Schema Discovery and added
`KindCheck`/`KindSelection`/`TriggerMatch.span` to
`simplified-schema/docs/topics/diagnostics.md`. The `md schema validate` and
DMLS YAML surfaces are marked **planned** (phases 5 and 6).
`darkmatter/docs/errors/` was left alone (selection errors do not fit its one
page). The agent noticed that a vacuous trigger's discovery warning had no
position although the in-file kind check locates it; fixed in code
(`triggers::envelope::trigger_error_span`, used by
`InvalidTriggerFinding::located_in`) with
`trigger_discovery_recovery::a_vacuous_match_is_a_finding_positioned_on_the_match_value`.
An existing contract (`invalid_triggers::a_trigger_failure_without_a_position_reports_no_source`)
keeps errors that name no node (unknown envelope keys) unpositioned.

### Gates (run once each, after all edits, no subagent running)

- `simplified-schema`: `just test` 1396/1396 + minimal leg 1209/1209;
  `just lint` clean (re-run after the last test edit).
- `darkmatter`: `just test` 12954 passed, 10 skipped, 0 failed (one run);
  `just lint` clean; `just check-tier-coverage darkmatter` and
  `simplified-schema`: 0 stranded.
- After the gates, one test-only edit (Windows path comparison below); its
  18 tests re-run on macOS and passed.
- Affected scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, dmls, simplified-schema, test-toolkit, darkmatter-cli]`,
  `change_class: package`; gates required and run.
- Dependents: `cargo check --all-targets -p claudine -p claudine-gen -p claudine-cli -p messenger -p messenger-cli -p research`
  clean; `cargo nextest run -p claudine -p claudine-cli -E 'test(/schema|trigger|finding|composition|envelope/)'`
  3704/3704 passed.
- Cross-OS (path comparison in source-path identity): `just cross-check darkmatter --os windows`
  over `yaml_ kind_semantic shipped_schema_corpus trigger_discovery_recovery trigger_envelope schema_roots`:
  297/298 on the first attempt. The failure was the test comparing the
  definition label (`repo\darkmatter\schemas\…`) with a display string built
  by `join("darkmatter/schemas/…")` (mixed separators); production identity
  worked. Fixed by comparing as paths; attempt 2 (`yaml_kind_registry`):
  17/17 passed. (A first invocation with a `-E 'test(/…/)'` filterset failed
  in the remote shell's quoting before running anything; pass substrings.)
- No load-sensitive or pre-existing failures observed.

## Phase 5

### 5.1 Preflight and routing (`cli/src/commands/schema/validate.rs`, `args/target.rs`)

- A preflight opens every argument (`open_argument`) before any validation or
  output and classifies it as YAML by the resolved path's extension
  (`.yaml`/`.yml`, ASCII case-insensitive; an unopened argument by its
  spelling, see decisions). With any YAML input, an assignment or `--schema`
  exits `64` with nothing on stdout.
- The baseline (`load_api`) is loaded only when some input is not YAML; a
  YAML-only batch builds a bare `DarkmatterSchemas` at the launch context.
  `--disable baseline-schema` / `schema-triggers` are accepted and inert for
  YAML (the registry ignores trigger disablement, rule 18).
- `validate_yaml` reads the file, loads it with the new
  `LoadedYamlDocument::load` (failure → new `FileOutcome::YamlLoadError`,
  exit 3, pretty "is not a valid YAML document" with `(at line L, column C)`
  from `simplified_schema::line_col_of_offset`, JSON `error: "yaml_parse"` +
  `line`/`column`), re-anchors the API on the document context, and calls
  `validate_loaded_yaml_document`: `NoSchema` → the Markdown no-schema record,
  problems → exit 1, any error → `SchemaError` (exit 2). Precedence 3 > 2 > 1
  unchanged. JSON problem `kind` is `kind_check` for `ValidationProblemCode::KindCheck`.
- **Departure from the plan (library API):** the plan/previous phase left load
  vs. schema error classification to the CLI (compare paths). A referenced
  schema's load error is also a `SchemaDocument`, so the entry point was split:
  `LoadedYamlDocument::load` + `DarkmatterSchemas::validate_loaded_yaml_document`
  (`validate_yaml_document` = both; signature unchanged). Recorded in
  `decisions_for_review`.
- Help text for `schema validate` and its `inputs` names Markdown or YAML.
- Manual Definition-of-Done check: `md schema validate
  darkmatter/schemas/schema-trigger.yaml simplified-schema/schemas/schema.yaml
  simplified-schema/schemas/yaml-schema.yaml` exits 0 (all three "has a schema
  definition and is valid").

### 5.2 Tests

Requirement → test (`darkmatter/cli/tests/l1/schema_validate_yaml.rs`, F3 spawn of
the real `md`, L1; 18 tests, declared in `cli/tests/l1/main.rs`):

| Requirement | Test |
|---|---|
| kind route control (shipped kind) → 0 | `a_claimed_shipped_kind_validates_against_its_definition` |
| kind definition failure → 1, located | `a_kind_definition_failure_exits_1_located_on_the_value` (line 2, column 8) |
| kind semantic check → 1, JSON `kind_check` with path/line/column | `a_kind_semantic_check_failure_exits_1_as_a_kind_check_record` |
| repository claim selects definition (control + roster break → 1 located) and duplicate claim → 2 naming both files | `a_repository_kind_claim_selects_its_definition_and_a_duplicate_claim_exits_2` |
| reserved kind → 2 naming the definition | `a_kind_reserved_by_a_failed_definition_exits_2_naming_the_definition` |
| root `$schema` control (soft `"8080"`, file not rewritten) + failure → 1 located | `a_root_schema_validates_softly_and_a_failure_exits_1_located_on_the_value` |
| no schema → 0, JSON record `schema: null`, no `error` | `a_document_without_kind_or_schema_is_valid_and_its_json_record_says_no_schema` |
| bad `$schema` reference → 2 | `an_unresolvable_root_schema_reference_exits_2` |
| referenced malformed schema → 2, not 3 | `a_root_schema_naming_malformed_yaml_exits_2_not_3` |
| malformed YAML → 3, located, JSON `yaml_parse` with line/column | `malformed_yaml_exits_3_located_and_reported_as_yaml_parse` |
| `---`/`...` markers are YAML, not fences | `leading_and_trailing_document_markers_are_yaml_not_frontmatter` |
| `.YAML` / `.Yml` route as YAML | `upper_and_mixed_case_extensions_route_as_yaml` |
| `@` reference resolving to YAML | `a_magic_reference_resolving_to_a_yaml_file_routes_as_yaml` |
| assignment + YAML → 64, stdout empty, earlier Markdown file unchanged | `an_assignment_with_a_yaml_input_exits_64_before_any_file_is_validated` |
| `--schema` + YAML → 64 | `a_custom_baseline_with_a_yaml_input_exits_64` |
| broken `BASELINE_SCHEMA` ignored for YAML-only; loaded (exit 2) once Markdown joins | `a_yaml_only_batch_never_loads_the_baseline` |
| `--disable baseline-schema` accepted | `disabling_the_baseline_is_accepted_with_a_yaml_input` |
| mixed batch precedence 1 / 2 / 3 | `a_mixed_batch_exits_with_the_most_specific_failure` |

Library (in-process, `darkmatter/lib/tests/l1/yaml_document_loading.rs`):
`loading_and_validating_are_separate_steps_so_a_referenced_files_load_error_is_not_the_documents`
proves the two-step API: the one-call form fails with `SchemaDocument` for a
referenced malformed schema, the document itself loads, the validate step
fails naming `contract.yaml`, and a malformed document fails the load step
naming the document. The document-shape and field-shape matrices stay in the
Phase 4 library suites; the CLI spawns only for wiring (control + one invalid
per route/loader), per the matrix rules.

Code-smell grep (`serde(default)`, `unwrap_or_default()`, `.ok()`,
`filter_map(.. as_str())`) over `validate.rs` and `yaml_documents/mod.rs`: none.

### 5.3 Docs and skills

- `docs/topics/schemas/definition.md`: new "YAML Documents on the Command
  Line" section (routing, no baseline/triggers, preflight 64s, exit 3/2/1,
  `kind_check`, `schema` field), the CLI table row, exit-code table, JSON
  `error`/`kind` values, and `LoadedYamlDocument` in "Validating from the
  Library". The "planned" markers for the CLI are removed from
  definition.md, authoring-schemas.md, schema-discovery.md, schema-triggers.md;
  DMLS markers remain (Phase 6).
- `docs/cli/{index,policy,report-json}.md`: resolved-path routing, inert
  `--disable` ids for YAML, link to the JSON record docs.
- `cli/README.md`: a short "Schema Validation" overview with a link.
- Skills: `.claude/skills/darkmatter/cli.md` (the YAML route),
  `.claude/skills/darkmatter/schema.md` (two-step API; md now calls it).

### Gates (run once each, after all edits)

- Targeted: `cargo nextest run -p darkmatter-cli --test l1 -E 'test(schema_validate_yaml)'`
  18/18; `cargo nextest run -p darkmatter --test l1 -E 'test(yaml_document_loading)'` 10/10.
- `darkmatter`: `just lint` clean (exit 0); `just test` 12973 passed, 10
  skipped, 0 failed, in one run; `just check-tier-coverage darkmatter`: 0 stranded.
- Affected scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, darkmatter-cli, dmls, simplified-schema, test-toolkit]`,
  `change_class: package` (the tree still holds Phase 4's uncommitted work);
  gates required and run.
- Dependents: the library change is additive (`LoadedYamlDocument`,
  `validate_loaded_yaml_document`; `validate_yaml_document` behaves as
  before), so `cargo check --all-targets -p claudine -p claudine-cli -p messenger -p research`
  (clean) was the downstream check; no dependent test touches the new API.
- Cross-OS: not run. The new code's only path logic is an extension
  comparison and the `canonicalize_simplified` call the Markdown route
  already uses; no new path comparison.
- No load-sensitive or pre-existing failures observed.

## Phase 6

### 6.1 Routing and registry worker integration

- One routing test, `dmls::overlay::is_yaml_path` (ASCII case-insensitive
  `yaml`/`yml`), replaces the copies in `overlay/mod.rs`, `router.rs`
  (`didSave`), `overlay/registry.rs` (invalidation), and
  `workspace/discover.rs` (listing). The workspace watch globs are now
  `**/*.[yY][aA][mM][lL]` and `**/*.[yY][mM][lL]`
  (`workspace::watch::{YAML_GLOB, YML_GLOB}`).
- A YAML path is routed before the Markdown frontmatter path, so a leading
  `---` is a YAML document marker, never a fence.
- `registry_demand` now asks for a registry for every YAML document inside a
  workspace folder. Its preparation (`Preparation::run`, on the registry
  worker) also builds the document's `KindRegistry` from the same root
  contents, through `BufferedRoots` (an open buffer's text in place of its
  file). The registry is stored on the document's `Scope` (`InstalledKinds`,
  versioned), records what it read (`Dependencies.kind_sources`, imports as
  file dependencies, searched locations), and shares the scope's
  invalidation. **Departure from the plan:** the plan named a separate
  `OverlayCache.kind_registries`; see `decisions_for_review`.
- Unsaved definition edits: `didOpen`/`didChange`/`didClose` of a YAML file
  call `OverlayState::set_yaml_buffer` → `TriggerRegistries::set_buffer`,
  which marks stale only scopes whose kind registry read that file with
  other text. Outside a workspace folder the shipped registry is used.

### 6.2 YAML document overlay

- `overlay/yaml_document.rs`: dispatch-thread only. `LoadedYamlDocument::load`
  (a load failure keeps the last loaded tree for completion, stale), then
  `validate_loaded_yaml_document` with the installed registry; cached per URI
  by text, revision, context identity, and registry version, plus
  content-hash dependencies and probes of the root-`$schema` route (the same
  `deps_unchanged`/`probes_unchanged` the Markdown cache uses).
- `DocumentOverlay.yaml_document` carries the outcome (`LoadFailed`,
  `Unvalidated`, `NoSchema`, `Failed`, `Validated`); the bundle is the
  selected schema alone; `AnalysisIdentity.yaml_document` makes the root
  shape the selected schema's (never the Markdown baseline).
  `FrontmatterAst::parse_yaml_document` restores the reverted recovery tree
  (`$schema` is data on the kind route) and treats the buffer end as inside
  the document.
- Library: `YamlValidation` gained `effective()` and `document_json()`
  (additive; its `Debug` is now hand-written because `EffectiveSchema` has
  none).
- The envelope paths (trigger, standalone) still run for YAML; the YAML state
  is attached to their overlay (`with_yaml`) or a YAML-only overlay is built
  with definition-driven `frontmatter_authoring`. A standalone schema
  envelope off the kind route keeps its own overlay (decision recorded).

### 6.3 Diagnostics

- `diagnostics/yaml_document.rs` replaces every Markdown frontmatter pass for
  a YAML document. `ProblemGrading::{Frontmatter, YamlDocument}` threads into
  `classify`: in a YAML document `missing_required` and `unknown_key` are
  errors in either mode; `KindCheck` is ranged by `YamlValidation::byte_range`.
- Selection failures are one `dm.schema.prepare` on the `kind` value with the
  definition files as related information; a root `$schema` failure goes
  through the existing `schema_prepare_diagnostic` (failure site = the
  reference, related information into the referenced file).
- One publication per problem: see the dedup rules in `decisions_for_review`.
- `DslProvider` (Markdown body directives, interpolation, shell) returns
  nothing for a YAML path.

### 6.4 Completion and hover

- Definition-driven on both routes via the shared analysis; schema authoring
  inside `$schema`/`types` of claimed `schema`/`yaml-schema` documents (the
  standalone authoring state, or `frontmatter_authoring` from the
  definition); `$schema` on the root route keeps reference hover/links
  (`SchemaReferenceFacts::select_in` over the YAML tree); on the kind route
  `$schema` is data (`schema_key_is_reference`). The add-missing-required-keys
  code action works unchanged on the YAML tree.

### 6.5 Tests

Requirement → test (all L1, declared in `dmls/tests/l1/main.rs`):

| Requirement | Test |
|---|---|
| kind route control + value / unknown key ranges | `yaml_document_editor::a_claimed_repository_kind_is_validated_by_its_definition_alone` |
| shipped kind outside any root, one publication | `…::a_shipped_kind_is_claimed_without_any_schema_root` |
| missing required = error non-strict, add-keys action | `…::a_missing_required_key_is_an_error_without_strict_mode_and_offers_the_add_keys_action` |
| unknown key = error non-strict | `…::an_undeclared_key_of_a_closed_definition_is_an_error_without_strict_mode` |
| trigger definition problem once; kind check once on the glob line | `…::a_kind_check_is_published_once_on_the_node_it_judges` |
| duplicate claim on `kind`, related to both files | `…::a_kind_claimed_twice_is_one_error_on_the_kind_naming_both_definitions` |
| completion/hover from the definition only (no `title`) | `…::completion_and_hover_describe_the_selected_definition_alone` |
| schema authoring inside exports | `…::a_claimed_schema_document_keeps_schema_authoring_inside_its_exports` |
| user kind's `$schema` is data | `…::a_user_kinds_schema_key_is_data_not_a_reference` |
| root route soft (`"8080"`), nothing written, `$schema` not unknown, `"2.5"` fails | `…::a_root_schema_validates_softly_and_is_not_an_unknown_key` |
| `{{ 1 }}` fails a number, `{{ name }}` passes a string, no body providers | `…::an_interpolation_string_is_ordinary_data_in_a_yaml_document` |
| unresolvable root `$schema` on its value | `…::an_unresolvable_root_schema_is_a_prepare_error_on_its_value` |
| referenced definition failure related into its file | `…::a_root_schema_naming_a_bad_definition_relates_the_failure_into_that_file` |
| root route completion/hover + `$schema` reference hover | `…::the_root_schema_route_completes_its_contract_and_assists_the_reference` |
| NoSchema shapes (mapping without kind/`$schema`, unclaimed kind, sequence, scalar, empty) | `…::a_document_with_neither_kind_nor_schema_is_not_validated` |
| load failures (malformed, duplicate key, second document) → one parse error | `…::a_document_that_does_not_load_gets_one_parse_error_and_no_schema_problems` |
| `---`/`...` markers, `.YAML`/`.Yml` | `…::document_markers_are_yaml_not_frontmatter_fences` |
| Unicode (UTF-16), CRLF, BOM ranges | `…::ranges_count_utf16_after_unicode_and_survive_crlf_line_endings` |
| route change clears stale diagnostics/completion | `…::changing_or_removing_the_kind_or_schema_clears_the_earlier_results` |
| unsaved definition edit refreshes; close restores | `yaml_document_session::an_unsaved_definition_edit_refreshes_an_open_document_until_the_definition_closes` |
| created definition claims (type 1); deleted clears (type 3), no stale hover | `…::a_definition_created_on_disk_starts_claiming_and_deleting_it_clears_the_results` |
| root `$schema` file edit (type 2) / deletion (type 3) refreshes | `…::an_edited_root_schema_file_refreshes_the_document_that_names_it` |
| scope isolation between two workspace folders | `…::each_workspace_folder_selects_from_its_own_definitions` |
| preparation off the dispatch thread | `…::a_yaml_documents_kinds_are_prepared_off_the_dispatch_thread` |
| stdio smoke (built binary) | `standalone_stdio::yaml_document_diagnostics_reach_a_stdio_client` |
| `is_yaml_path` cases; watch globs agree with it | `overlay::tests::a_yaml_path_is_recognized_in_any_ascii_letter_case_and_nothing_else_is`, `workspace::watch::tests::yaml_watch_globs_match_every_letter_case_the_router_accepts` |

Existing tests whose expectations changed with the new contract:
`overlay::tests::{escaped_quotes_stay_inert_without_panicking,
standalone_activation_is_content_based_not_path_based,
shipped_schema_corpus_uses_content_classification}` (an ordinary YAML file now
has an overlay that selects no schema, still inert),
`schema_roots_parity::a_forbidden_path_prefix_is_the_same_definition_error_in_md_and_dmls`
(now `dm.schema.kind_check`, once, on the pattern's line),
`lsp_session::meta_schema_standalone_block_plain_flow_indicator_retains_last_good`
(the `notes` key is `unknown_key` on the key, no `document_malformed`),
`suggest_constraint_phase1::suggest_phase5_malformed_tagged_envelope_error`
(one located `dm.schema.*` error). The drain-loop helper was corrected in
seven places (decision recorded); `workspace_file_list.rs` reads the new glob
constants.

Input robustness matrix: the loader and selection matrices live in the
library suites (Phases 4–5); DMLS spawns nothing per cell and wires one
loader (control + three invalid shapes), the `kind` field (claimed, unclaimed
string, absent), and the fallback `$schema` (valid, unresolvable, bad
definition). Code-smell grep over `overlay/yaml_document.rs`,
`diagnostics/yaml_document.rs`, and the `registry.rs` additions: only an LSP
range default (`unwrap_or_default` on a `Range`) and the existing
"unreadable dependency hashes to `None`" `.ok()`, neither a load-bearing parse.

### 6.6 Docs and skills

- `docs/topics/schemas/dmls-schema-support.md`: new "YAML Documents" section
  (routes, what is reported, completion/hover, refresh with a Mermaid flow),
  the coverage-matrix row, the unsaved-edit bullet.
- `docs/lsp/indexing.md` ("YAML document kinds", refresh table row, listing
  case), `docs/lsp/configuration.md` (globs, strict note, extensions note,
  add-missing-key), `dmls/docs/diagnostics.md` (codes, "Whole YAML
  documents", open-envelope behavior, severity), `dmls/docs/file-references.md`.
- The last two **planned** markers are removed (`definition.md`,
  `schema-triggers.md`).
- Skills: `.claude/skills/darkmatter/dmls.md` ("Whole YAML documents"),
  `.claude/skills/darkmatter/schema.md` (DMLS now calls the entry point).

### Gates (run once each, after all edits, with no subagent running)

- Targeted while building: `cargo nextest run -p dmls -E 'test(yaml_document_editor::)'` 19/19,
  `-E 'test(yaml_document_session::)'` 5/5, the stdio smoke 1/1.
- `darkmatter`: `just lint` clean (exit 0, including the zed-dmls wasm check);
  `just test` 13000 passed, 10 skipped, 0 failed, in one run;
  `just check-tier-coverage darkmatter`: 0 stranded.
- Affected scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [darkmatter, dmls, test-toolkit, darkmatter-cli]`, `change_class: package`;
  gates required and run.
- Dependents: the library change is additive (`YamlValidation::effective`,
  `document_json`); `cargo check --all-targets -p claudine -p claudine-cli -p messenger -p research`
  is clean. No dependent test uses the new API.
- Cross-OS: `just cross-check dmls --os windows` (the buffer map and kind
  sources are keyed by normalized paths): 1691 passed, 9 skipped.
- No load-sensitive or pre-existing failures observed.

## Phase 7

Note on the starting tree: Phase 6's changes were still uncommitted when this
phase began (`git status` listed them), so the working tree holds Phases 6
and 7 together. The `source_files_during_phase_7` list names only this
phase's files.

### 7.1 `messenger/docs/platforms.yaml`

- Before the change, the fenced roster already failed `md schema validate`
  (exit 3, "more than one document"): the trailing `---` is a second YAML
  document under the whole-file route.
- Removed both fences and the obsolete fence-workaround comment, keeping
  `$schema: ./platforms.schema.yaml`. `md schema validate
  messenger/docs/platforms.yaml` exits 0. A deliberate break
  (`roster_version: 2`) exits 1 with "roster_version 1 was expected (at line
  19, column 17)". The file was then restored.
- Consumer: Messenger's research `Loader` (`messenger/lib/src/research/load.rs`)
  read the roster, overrides, and mappings as Markdown frontmatter. YAML
  contract kinds (`ContractKind::is_yaml_document`) now load with
  `LoadedYamlDocument::load` and validate with
  `DarkmatterSchemas::validate_loaded_yaml_document` in the file's own
  context (the `md schema validate` route: root `$schema`, no Markdown
  baseline). Research documents (Markdown) are unchanged. A YAML load failure
  or non-mapping root is the new `ResearchError::Document`. `$schema` binding
  is still checked first, on the loaded JSON.
- Library (additive): `LoadedYamlDocument::json()`, so a host reads the
  values from the same load and never needs a second YAML parser.
- All 26 YAML fixtures under `messenger/lib/tests/fixtures/research/` lost
  their fences; `research_publication.rs`'s written `_overrides.yaml` too.
  `research_corpus.rs` reads `.yaml` files through `LoadedYamlDocument` and
  validates the first document of each schema through
  `validate_loaded_yaml_document`, reusing that call's `effective()` for later
  documents of the same schema (the file already cached Markdown schemas
  the same way for the same reason).
- New test `research_validation::yaml_contract_files_load_as_one_whole_yaml_document`:
  control (shipped roster, clean, `$schema` not data, `roster_version` 1),
  leading `---` marker accepted, a schema break as a `/roster_version`
  `Schema` finding, then three load errors (old fenced form → "more than one
  document", duplicate key, sequence root → "not a mapping"), each a
  `Document` error naming the repository path without the host root. The
  overrides loader gets one fenced case, which proves it is wired to the
  same branch.

### 7.2 Skills

- `.claude/skills/darkmatter/SKILL.md`: ownership, registry lifecycle and
  failure policy in brief, and the "a fenced YAML file is two documents"
  rule.
- `schema.md`: an "Ownership and lifecycle" list (who owns which definition,
  when a registry is built and reused, the failure policy, informal schema
  files judged as data) and the `LoadedYamlDocument::json()` rule.
- `cli.md`: fenced YAML exits 3; informal schema files can fail through `md`
  while DMLS shows nothing.
- `testing.md`: the YAML document corpus test, how its exceptions retire,
  the rule that snapshots are never edited to make a corpus pass, and
  unfenced YAML fixtures.
- `.claude/skills/simplified-schema/SKILL.md`: the ownership boundary.
- `dmls.md` was already updated in Phase 6.

### 7.3 Corpus over repository YAML

One-off run: `md schema validate` on every tracked `.yaml`/`.yml` file with a
top-level `kind` (142 files: 100 `schema`, 16 `schema-trigger`, 3
`yaml-schema`, and 23 unclaimed kinds such as `example`, `sequence`, and
`trigger-schema`, which correctly select nothing). The first run had 13
failures. Fixed:

- `darkmatter/docs/schemas/err.yaml`: removed `name: err` (the closed
  `kind: schema` envelope has no `name`) and the `$schema:
  ./schema-definition.yaml` line. Under `kind: schema`, `$schema` is the
  exported document schema, so that line exported an empty placeholder.
  The file is now a types-only schema document. The empty placeholder
  `darkmatter/docs/schemas/schema-definition.yaml` was deleted, and
  `authoring-schemas.md`'s "recursive truth" transclusion now shows the
  real definition, `simplified-schema/schemas/schema.yaml`. Tests that
  pinned the old state: `shipped_schema_corpus` (two `EXCEPTIONS` removed,
  so the registry corpus now checks `err.yaml`), and
  `meta_schema_phase1::shipped_schema_corpus_is_passively_classified_without_resolution`
  (`err.yaml` is now a schema). The DMLS unit test
  `overlay::tests::shipped_schema_corpus_uses_content_classification`
  (`err.yaml` is now "ready") and
  `meta_schema_phase4::shipped_schema_artifacts_validate_through_semantic_keywords`
  were also updated. The last one lost its only `$schema` declaration, so
  it now also reads the `$schema` declarations of `darkmatter/schemas/`
  (the `schema-trigger` definition and the global catalog), which keeps the
  assertion meaningful rather than weakening it.
- `claudine/docs/research/provider-schema.yaml`: removed a stray empty
  `type:` key. `claudine/docs/research` is now one of the registry corpus
  areas. Checked as load-bearing: putting the key back fails the test,
  naming the file.

Left failing, by decision (see `decisions_for_review`): the Claudine design
sketches `claudine/docs/schemas/{action,partials/flow-control,partials/lifecycle,partials/loop,partials/stack}.yaml`
(already pinned as drafts by `shipped_schema_corpus` and the golden
manifest), four drafts in a closed Claudine feature directory, the frozen
golden input copy of `err.yaml`, and `docs/drafts/memory.yaml`. Final
tally: 131 pass, 11 fail, all 11 in those groups.

Phase 6's open item (informal root-`$schema` files judged as data by `md`)
did not arise: the corpus selects files by `kind`. It is recorded in the
skills.

### 7.4 Kind catalog docs

- `docs/kind-documents.md`: its `::code` pointed at a draft path that no
  longer exists and now shows `^schemas/kind.yaml`. The schema-roots note
  is taken from the preserved patch's hunk. A new "YAML Document Kinds
  (`yaml-schema`)" section has an example and
  `::doc-list kind=yaml-schema group-by=package-area` (the planned
  directive's documented syntax). `md compose docs/kind-documents.md` exits 0.
- `schemas/kind.yaml`: `yaml-schema` added to the `kind` enum.
- READMEs: one overview line in `darkmatter/README.md` (whole YAML documents)
  and one in `simplified-schema/README.md` (owns the `schema`/`yaml-schema`
  definitions), each linking to its topic page.

### 7.5 Gates and the final re-read

Run once each, after every edit, with nothing else running:

- `darkmatter`: `just test`. The first run had one failure,
  `meta_schema_phase4::shipped_schema_artifacts_validate_through_semantic_keywords`
  (fixed as described in 7.3; it passed alone, then in the full rerun).
  Rerun: 13000 passed, 10 skipped, 0 failed. `just lint` exit 0 (including
  the zed-dmls wasm check). `just check-tier-coverage darkmatter`: 0
  stranded.
- `simplified-schema`: `just test` 1396 + 1209 passed; `just lint` exit 0.
- `messenger`: `just test` 783 passed, 2 skipped (lib with
  `desktop,research`, plus messenger-cli); `just lint` clean.
- Dependents: `cargo check --all-targets -p claudine -p claudine-cli -p
  research` is clean (the library change is one additive accessor). The
  Claudine tests that read `claudine/docs/research/` also pass:
  `claudine_schema_shipped_corpus::*` and `agent_errors_fleet::*` (69) and
  claudine-cli `research_sequence_artifacts::*` (7).
- Affected scope over the uncommitted tree (Phases 6 and 7):
  `packages: [darkmatter, dmls, messenger, test-toolkit, claudine,
  claudine-cli, darkmatter-cli, messenger-cli]`, `change_class: package`.
  `claudine`/`claudine-cli` come only from the narrowed test-input cells for
  `provider-schema.yaml`; `schemas/kind.yaml` schedules nothing. Gates
  required and run.
- `just ci-local --plan` was reviewed. This phase adds the messenger
  L1/lint cells, the messenger-cli narrowed cell for `platforms.yaml`, and the
  Claudine narrowed cells. The other cells (sniff, test-toolkit, repo-deps)
  come from the branch's earlier commits.
- Cross-OS check skipped: no path, process, or OS-specific code changed. The
  Messenger loader passes the same absolute path to the existing
  library loader, which already handles CRLF and a BOM.

Definition of Done:

- `md schema validate messenger/docs/platforms.yaml` → 0. A roster break → 1
  with line and column (above).
- `md schema validate darkmatter/schemas/schema-trigger.yaml
  simplified-schema/schemas/schema.yaml simplified-schema/schemas/yaml-schema.yaml`
  → 0 in this checkout.
- Spec Acceptance → tests (detailed tables in the Phase 2–6 sections):

| Acceptance bullet | Tests |
|---|---|
| three definitions are `kind: yaml-schema`, shipped, pass `yaml-schema.yaml` | `envelope_admission::the_library_definitions_are_admitted_…`, `…each_shipped_definition_claims_its_kind`, `shipped_schema_corpus::every_shipped_yaml_schema_definition_is_in_the_shipped_table`, the DoD command above |
| root claims; duplicate names both files | `yaml_kind_registry::…two_definitions_claiming_one_kind_are_a_duplicate…`, CLI `a_repository_kind_claim_selects_its_definition_and_a_duplicate_claim_exits_2`, DMLS `a_kind_claimed_twice_is_one_error_on_the_kind_naming_both_definitions` |
| route selection; roster without fences | `yaml_document_selection::*`, messenger `yaml_contract_files_load_as_one_whole_yaml_document`, `research_corpus::shipped_roster_validates_against_its_schema` |
| ranged diagnostics, completion, hover on both routes | CLI `schema_validate_yaml::*`, DMLS `yaml_document_editor::*` |
| edited shipped definition changes validation, no Rust change | `yaml_kind_registry::…a_modified_copy_at_the_shipped_path_replaces_the_built_in_definition` |
| corpus passes with source-path identity | `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` (now including `err.yaml` and `claudine/docs/research`) |
| constructor narrowing | `declaration_narrowing::*` |
| direct loads admit envelopes | `envelope_admission::*` |
| closure and `non-null` | `closed_roots::*`, `non_null_properties::*` |
| shared library and `md` matrices | `yaml_document_loading::*`, `yaml_document_selection::*`, `schema_validate_yaml::*` |
| DMLS refresh, isolation, route changes, Unicode/CRLF | `yaml_document_session::*`, `yaml_document_editor::*` |

- Code-smell grep over this phase's added lines in `messenger/lib/src`:
  no `unwrap_or_default`, `.ok()`, `filter_map`, or `serde(default)`. The
  one added library accessor has none.
- No doc under `docs/` names this feature (`grep -rn yaml-document-schemas`
  over `darkmatter/docs`, `simplified-schema/docs`, `messenger/docs`,
  `docs/` finds nothing).

Items for the human review, as the plan asked: the registry's
`Unavailable` wording, the two-arm union error messages of `schema.yaml`,
the naming of the `dm.schema.kind_check` code, and the decision on the
`required; non-null` redundancy (Phase 2 decisions), plus this phase's
three decisions.

Implementation complete, ready for review.
