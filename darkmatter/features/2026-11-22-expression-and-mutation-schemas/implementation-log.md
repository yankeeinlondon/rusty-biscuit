---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/spec.md"
plan: "darkmatter/features/2026-11-22-expression-and-mutation-schemas/plan.md"
implemented-by: "claude/opus"
started-phase: "1"
source_files_during_phase_1:
    - simplified-schema/schemas/expression-schema.yaml
    - simplified-schema/schemas/mutation-schema.yaml
    - simplified-schema/lib/src/envelope.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/shipped/registry.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/src/triggers/finding.rs
    - simplified-schema/lib/src/yaml_documents/load.rs
    - simplified-schema/lib/src/yaml_documents/marks.rs
    - simplified-schema/lib/src/yaml_documents/mod.rs
    - simplified-schema/lib/src/yaml_documents/semantic.rs
    - simplified-schema/lib/generated/shipped.json
    - simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/shipped_artifacts.rs
    - darkmatter/lib/src/markdown/schemas/schema_files.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/lib/generated/shipped.json
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/yaml_kind_registry.rs
    - darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/manifest.yaml
    - darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/globals.lifecycle.json
    - darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/
    - claudine/lib/generated/shipped.json
docs_updated_during_phase_1:
    - simplified-schema/docs/topics/language/schema-documents.md
    - simplified-schema/docs/topics/language/unions.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/diagnostics.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/simplified-schema/schema-documents.md
    - .claude/skills/darkmatter/SKILL.md
packages:
    - simplified-schema
    - darkmatter
    - darkmatter-cli
    - claudine
    - darkmatter-lsp
    - claudine-cli
decisions-for-review:
    - |-
      Duplicate YAML keys: the plan asks the loader to record both the first and the repeated key. I kept the shared loader's error type unchanged (`SchemaError::SchemaDocument`, located at the repeated key) and added the first key's line and column to its message, plus a public `yaml_documents::duplicate_key(text)` that returns both byte offsets for any consumer that wants a "related location". The alternative is a `related` field on `SchemaDocument` itself, which would touch every producer and consumer of that widely used variant. If the reviewer prefers the field, the change is confined to `load.rs` plus the CLI and editor projections Phase 2 adds; `duplicate_key` would then become internal.
    - |-
      Strict shape for the two catalog kinds holds on every path, not only on direct admission. Darkmatter's whole-YAML route validates a document softly against its selected kind definition, which would turn `description: 5` into `"5"`. So `check_written_labels` (which runs on every parse, including the syntax-only kind check) also refuses a non-string `description` and a non-string category entry, as it already refuses `null`. The alternative was to make Darkmatter's YAML validation strict for these kinds, which changes a host adapter shared by all kinds. Redo cost if reversed: drop the two checks and add a strict branch in Darkmatter's YAML contract.
    - |-
      `StandaloneSchemaEnvelope::allowed_keys()` is a hand-written list beside the YAML definitions (the plan asks to "expose their labels and allowed keys"). A test (`allowed_keys_match_each_shipped_definition`) compares it with every shipped definition's keys so they cannot drift. Deriving the list from the lowered definitions at runtime would avoid the duplication but needs the artifact loaded for a constant-time question the editor asks often.
    - |-
      `md schema validate` already has a CLI step that resolves a schema file after its kind check. Phase 1's privacy rule would make that step fail on a valid catalog, so I added the minimal skip for callable catalogs now (as namespaces are skipped). The plan schedules the full CLI catalog route for Phase 2; that work replaces this skip and adds the catalog reader's judgment.
    - |-
      `recent-commits` is now written twice. The context-variables partial used to import this list type from the built-in function catalog (`recent-commits@darkmatter:functions`), but a callable catalog can no longer be imported, so the partial declares an identical local copy and a test (`recent_commits_shared_type.rs`) fails if the two ever differ. Alternatives: move the type into a third, importable partial that both use (the catalog cannot import it either, since a catalog may not reference other files), or allow catalogs to be import sources again (contradicts the spec). Redo cost: small; one YAML helper, one test, and a shipped-artifact regeneration.
    - |-
      `functions.yaml` is no longer in the embedded bundle of importable partials (`SHIPPED`); it only feeds the prepared built-in catalog (`shipped.rs::sources()`), and the preparation revision was bumped to 5. Keeping it in `SHIPPED` would have let `Name@darkmatter:functions` keep working, against the spec's "catalogs are not import libraries". Redo cost: small (one list entry and a regeneration).
    - |-
      The one catalog reader has two modes rather than a growing list of flags: `CatalogOrigin::BuiltIn` (Darkmatter's own two files: compiled-in file examples allowed, registered aliases projected) and `Authored` (every other catalog: no file examples, no reuse of a built-in name, every callable compiled eagerly). The CLI and editor pick the built-in mode only for the exact paths `darkmatter/schemas/partials/{functions,mutations}.yaml`, relative or under the captured repository root, compared by path identity. Alternative: separate boolean parameters per check. Redo cost: small, internal to `catalog/mod.rs` and `yaml_documents/mod.rs`.
    - |-
      Only callable failures become the new `dm.schema.callable_catalog` error. `prepare_yaml_contract` returns an error only for `SchemaError::CallableCatalog`; shape, grammar, unknown-field, duplicate-key, and duplicate-overload failures stay with the kind's existing contract reporting, so each problem is reported once, with the code users already see for that class. Alternative: report every catalog failure under the new code, which would duplicate the kind-contract findings. Redo cost: moderate (CLI, editor, and their tests).
    - |-
      Reusing a built-in function's name is checked against canonical names when a host catalog loads, and against aliases when the host function set is built (aliases are declared in Rust, not in the catalog). The lookup reads only the static binding table (`functions::reserved_builtin_name`), never the joined function registry, because building that registry loads the catalog and would loop. Redo cost: small.
    - |-
      Where a reference inside a callable's declaration is reported (an unknown helper, a callable used as a type, a reference to another file), the location is found by a whole-word search for the name inside that entry's declared text span, because the parser's source map does not record spans for names inside a type string. If the name is not found the error falls back to the entry's key. Alternative: extend Simplified Schema's source map with per-reference spans. Redo cost: moderate (parser work in another crate).
    - |-
      `HostCatalogError::related()` now returns the shared `CatalogRelated` records (message, file, optional span) instead of the earlier Darkmatter-only shape, and the error keeps the underlying `SchemaError` (`schema_error()`), compared by message, span, and related only. This is a public-type change with no outside users yet. Redo cost: small.
    - |-
      The fixed-work test for Claudine's registration load (`claudine_registration_load_does_fixed_work`) now parses a tiny ordinary schema before it starts counting. The fixture's catalog is now an `expression-schema`, whose one-time envelope preparation moved inside the measured window and raised the count from 4 to 7; the warm-up puts that one-time process work back outside, so the test still measures per-load work. Alternative: raise the expected count (it would then measure one-time work too). Redo cost: trivial.
    - |-
      A duplicate overload (two overloads that bind the same calls) is located at the later overload with a message naming both by position ("overloads 1 and 2 …"), but without a separate related location for the first. Simplified Schema's parser refuses it before Darkmatter's reader runs, and the CLI and the editor report it identically (a kind-contract/type-definition finding at the later arm). The plan's "attach the first matching arm" mapping exists in the reader but cannot be reached by a catalog. Alternative: have the parser return a located error with a related location for catalogs, which needs a new error shape in Simplified Schema and de-duplication against the kind check. Redo cost: moderate.
    - |-
      The CLI shows a related location in another file with a line and column only when it can read that file: an absolute path as is, a relative one under the captured repository root (or the working directory outside a repository). The built-in name case outside a checkout therefore prints `darkmatter/schemas/partials/functions.yaml` with no line. The compiled-in catalog text is private, and the shared error carries byte spans, not lines. Alternative: put line and column on `CatalogRelated`, or expose the built-in text. Redo cost: small.
    - |-
      In the CLI's JSON output a catalog failure keeps `error: "schema"` and adds `code`, `line`, `column`, a byte `span`, and a `related` list of location objects. The output contract (`darkmatter/cli/schemas/output/schema-validate.yaml`) was widened so `related` may hold either the existing strings (trigger conflicts) or the new objects. Alternative: a new `error` value for catalogs, or converting trigger conflicts to objects too. Redo cost: small (contract, renderer, tests).
    - |-
      In the editor, a catalog that currently fails loses completion and hover for its top-level fields (`kind`, `description`, `categories`, `types`) until it is valid again, because the library returns the catalog error instead of a contract; type-grammar help inside `types` still works. Alternative: have `prepare_yaml_contract` return the contract together with the catalog failures. Redo cost: moderate (library API plus the editor overlay).
    - |-
      For the editor requirement "repeated reads … neither rebuild the catalog nor enter a handler", the test asserts that repeated hover and completion on the same buffer revision run no YAML preparation, file read, or library compile, and queue no registry job. The editor never builds host handlers, so "enter a handler" was read as "queue no worker job". Redo cost: trivial if a stricter reading was meant.
    - |-
      Two small fixes outside the plan's file list. The YAML loader's duplicate-key message (Simplified Schema) no longer repeats the parser's location, which is the enclosing mapping's start and was shown as if it were the key's, and now names only the first spelling's line. The editor now attaches that first spelling as related information to every YAML document's repeated-key error, not only catalogs'. Redo cost: trivial.
    - |-
      Phase 3: the format 2 manifest decoder is kept only behind `bundle::admit_manifest`, the publisher's question "which files did my previous manifest list, so I may retire them". `admit_installed_manifest` (what `claudine doctor` uses to judge an installation), `load_snapshot`, `verify_read`, and embedded loading accept format 3 only. The v2 decoder is a strict typed read (closed at every level, digests checked as 64 lowercase hex), not the old shipped schema, because the shipped schema is now the v3 contract. Alternatives: a separate `admit_previous_manifest` entry point (Claudine's cleanup would have to switch to it in Phase 4), or no v2 decoding at all (an upgrade would leave the old revision's files behind). Redo cost: small (one function and its callers).
    - |-
      Phase 3: a catalog's companion is derived only for a `files` item whose verified bytes declare `kind: expression-schema` (read leniently: the top-level `kind` only). An item that is not readable YAML is "unclassified": a `digests` key that is its companion spelling is tolerated rather than reported as stray, because that item is refused anyway when it is prepared and its own parse error is the more useful report. A same-stem `a.yaml`/`a.yml` pair is refused only when the shared companion path is derived by one and named by the other (both catalogs, or a catalog plus a listed file of that name); a plain YAML file beside a catalog of the same stem is allowed. Alternative: refuse every same-stem YAML pair regardless of kind. Redo cost: small (one check in `manifest.rs::companion`).
    - |-
      Phase 3: a refusal about a companion (missing digest, missing file, digest mismatch, incomplete functions) carries the catalog's manifest field (`files[i]`) as its member, because the companion is not itself listed; the message names the companion spelling. Alternative: a synthetic field such as `files[i].facts` or `digests.<spelling>`, which is not a location in the manifest. Redo cost: small (the field string and tests that assert it).
    - |-
      Phase 3: the process-wide host function cache (one result per catalog path, with its test-only probes and tests) was deleted together with `is_host_function_catalog`, not just the schema-root probe. After this phase the only caller was registration, which already loads each catalog once per verified revision and whose profiles are cached by their owners (`MdRequest::registered_hosts`, DMLS `HostRegistrations`). Alternative: keep the cache for registration. Redo cost: moderate (restore the cache module and its tests from Git).
    - |-
      Phase 3: `SnapshotRefusal::legacy_format: bool` became `unsupported_format: Option<i128>` (the refused version), because "legacy" no longer fits a future version 4 and doctor (Phase 4) needs the number for its remedy. `i128` holds any integer YAML can spell (negative or above `i64::MAX`). Alternative: keep the bool. Redo cost: trivial.
    - |-
      Phase 3: Claudine was touched before Phase 4, minimally, so the monorepo keeps compiling and Claudine's published registration stays loadable by the new reader: its manifest encoder writes `format_version: 3` without `functions`, and `FileKind` gained an `ExpressionSchema` arm (needed by an exhaustive match on the shared `MemberKind`). The full inventory work (companions, `expression-facts`, doctor upgrade) stays in Phase 4. Alternative: leave Claudine broken until Phase 4. Redo cost: none.
message-to-agent: |-
  Phase 4 landed: Claudine's format 3 inventory encoder, companion input table, hidden `claudine registrations` kinds (`expression-schema`, `mutation-schema`, `expression-facts`), and `claudine doctor` v2→v3 upgrade and companion inspection/repair are complete and verified. What Phase 5 can build on:
  - `RegistrationInventory::claudine()` builds and verifies through `RegistrationInventory::from_embedded_sources` (empty companion table; `claudine/registration/` remains YAML-only, no host expression catalog or `.facts.json` companion shipped by Claudine).
  - `FileKind` exposes `manifest`, `schema-trigger`, `schema`, `yaml-schema`, `namespace-schema`, `expression-schema`, `mutation-schema`, `expression-facts`, and `lifecycle`; `claudine registrations` formats all of them with 16-digit XXH64 fingerprints.
  - `claudine doctor` routes inspection through the v3 inventory including `expression-facts` (missing companion = `error`, modified companion = `warn` with format 3 digest note, unsupported manifest version such as format 2 = `error` with update-and-republish remedy) and `claudine doctor --fix` / `claudine setup` upgrade the frozen v2 install in place to v3, writing `manifest.yaml` last and preserving equivalent/unrelated `register` entries and comments.
  - The inventory constructor rejects a supplied YAML member or facts companion using the reserved generated name `manifest.yaml` in any ASCII case. The published Claudine inventory still uses the same compiled sources and lifecycle artifact.
  - Phase 5 owns the remaining language, host, and publisher guidance, caller skill publication, generator parity against the Phase 1 baselines, and final full-area tests and lint across Simplified Schema, Darkmatter, and Claudine.
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/checks.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/effects/types.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/shipped.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/schemas/partials/mutations.yaml
    - darkmatter/schemas/partials/context-variables.yaml
    - darkmatter/lib/generated/shipped.json
    - claudine/lib/generated/shipped.json
    - darkmatter/lib/tests/fixtures/host_functions/project.yaml
    - darkmatter/lib/tests/fixtures/host_functions/project.facts.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/darkmatter/schemas/partials/context-variables.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-context-variables.json
    - darkmatter/lib/tests/host_fixtures/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/callable_catalog_validation.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/collection_analysis.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/member_check.rs
    - darkmatter/lib/tests/l1/recent_commits_shared_type.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/schemas/output/schema-validate.yaml
    - darkmatter/cli/tests/l1/schema_validate_yaml.rs
    - darkmatter/dmls/src/overlay/yaml_contract.rs
    - darkmatter/dmls/src/overlay/yaml_document.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/diagnostics/yaml_document.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/yaml_document_catalog_editor.rs
    - darkmatter/dmls/tests/l1/yaml_document_editor.rs
    - darkmatter/dmls/tests/l1/yaml_document_session.rs
    - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
    - simplified-schema/lib/src/yaml_documents/load.rs
    - simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs
    - simplified-schema/lib/tests/l1/shipped_corpus.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/host-functions.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/diagnostics.md
    - simplified-schema/docs/topics/language/objects-and-named-types.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/library-surfaces.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/schemas/darkmatter-host.yaml
    - darkmatter/lib/src/config/registration.rs
    - darkmatter/lib/src/config/registration/bundle.rs
    - darkmatter/lib/src/config/registration/locate.rs
    - darkmatter/lib/src/config/registration/manifest.rs
    - darkmatter/lib/src/config/registration/selection.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
    - darkmatter/lib/generated/shipped.json
    - darkmatter/lib/tests/host_fixtures/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/host_namespaces.rs
    - darkmatter/lib/tests/l1/host_registration.rs
    - darkmatter/lib/tests/l1/host_registration_companions.rs
    - darkmatter/lib/tests/l1/host_registration_locations.rs
    - darkmatter/lib/tests/l1/host_registration_snapshot.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/compose_foreign_documents.rs
    - darkmatter/cli/tests/l1/compose_host_scope.rs
    - darkmatter/cli/tests/l1/host_registration.rs
    - darkmatter/cli/tests/l1/host_runtime_scope.rs
    - darkmatter/dmls/benches/pipeline_work_a3.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/lifecycle_host_tests.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/claudine_host.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
    - darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
    - darkmatter/dmls/tests/l1/host_catalog_reload.rs
    - darkmatter/dmls/tests/l1/host_function_diagnostics.rs
    - darkmatter/dmls/tests/l1/lifecycle_host_profile.rs
    - darkmatter/dmls/tests/l1/registered_host_editor_agreement.rs
    - simplified-schema/lib/src/member_envelope.rs
    - simplified-schema/lib/tests/l1/member_envelope.rs
    - claudine/lib/src/composition/registration/published.rs
    - claudine/lib/generated/shipped.json
    - claudine/lib/tests/l1/host_catalog.rs
    - claudine/lib/tests/l1/host_publication.rs
    - claudine/lib/tests/l1/installed_registration.rs
    - claudine/lib/tests/l1/registration_inventory.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/host-registration.md
    - darkmatter/docs/topics/host-functions.md
    - claudine/docs/topics/schemas.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/library-surfaces.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - claudine/lib/src/composition/registration.rs
    - claudine/lib/src/composition/registration/inspect.rs
    - claudine/lib/src/composition/registration/published.rs
    - claudine/lib/src/doctor/registration.rs
    - claudine/lib/src/doctor/registration_tests.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - claudine/lib/tests/l1/host_publication.rs
    - claudine/lib/tests/l1/installed_registration.rs
    - claudine/cli/src/commands/registrations.rs
    - claudine/cli/tests/l1/doctor.rs
docs_updated_during_phase_4:
    - claudine/docs/cli/doctor.md
    - claudine/docs/cli/registrations.md
    - claudine/docs/topics/configuration.md
    - claudine/docs/topics/schemas.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
---

# Implementation Log for 2026-11-22-expression-and-mutation-schemas (5 phases)

## Phase 1

### Prerequisite baseline

- Starting point: `HEAD` `bc0b9f7bc` (tree `fc76b5c2821502ff51dba13c83108273f019e480`) on
  `fix/biscuit-terminal`. The working tree was **not clean** when Phase 1 started: the
  Claudine doctor review cycle (`2026-11-22-claudine-doctor`) had uncommitted edits in
  `claudine/`, `darkmatter/lib/src/config/`, `darkmatter/lib/tests/l1/`,
  `simplified-schema/lib/src/defaults.rs`, and several skills. Those files are not this
  phase's work; the frontmatter lists below name only what Phase 1 changed.
- The host registration v2 prerequisite is present (`darkmatter/lib/src/config/registration/{manifest,bundle}.rs`,
  fixed path `~/.claudine/registrations/darkmatter/manifest.yaml`, `format_version: 2`).
- Focused baseline, before any edit: `cargo nextest run -p darkmatter -p darkmatter-cli -p darkmatter-lsp
  -E 'test(host_registration) | test(registered_host) | test(host_selection)'` → 235 passed.
  The matching Claudine baseline (`-E 'test(registration) | test(doctor)'`) started after the
  first Simplified Schema edit and could not compile, so Claudine's registration and doctor
  tests are instead run as the dependent-package check at the end of this phase (below).
- Regression oracles (byte hashes are Git blob ids: compare with `git hash-object <path>`;
  copies of the bytes are in `target/feature-baseline/expr-mutation/`, outside Git):

  | Path | Git blob id |
  |---|---|
  | `darkmatter/schemas/partials/lifecycle.yaml` | `1f5c316e92f4d4485c4e0ce3cf63c560b265d622` |
  | `darkmatter/docs/topics/darkmatter-expressions.md` (generated expression table) | `4c7e8e02e0357bf59fe3dd2dd575ff6ceea04aad` |
  | `darkmatter/docs/functions/load-data.md` | `347c89bdd4e7b2b1a0e97c373d630e008ff94c23` |
  | `darkmatter/docs/functions/try.md` | `0a9948cda176f8fdf9aa233507e74b65d48f8591` |
  | `darkmatter/schemas/partials/functions.yaml` (pre-retag) | `18fccf348426989ab86f6b9221cb2e2e2532d77c` |
  | `darkmatter/schemas/partials/mutations.yaml` (pre-retag) | `782fb9acf3aa9a772fc51a98d06f30ecec521fb5` |

  All six were identical to `HEAD` at the start. The existing expression descriptor,
  binding, and registry parity suites are the descriptor/dispatch baseline.
- v2 migration fixture: `darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/` holds
  the exact bytes the prerequisite publisher installs: `claudine registrations publish` run
  with `HOME` set to an empty temporary directory (manifest with `format_version: 2` and
  `functions: []`, the 15 schema members, and `globals.lifecycle.json`; the writer's
  `.darkmatter.lock` is omitted). No test reads it yet; Phases 3 and 4 use it only for
  version refusal, fixed-path migration, and preservation of unrelated config.

### Envelope definitions (Rules 2, 3, and the shared diagnostic representation of Rules 5–6)

- Added `simplified-schema/schemas/{expression-schema,mutation-schema}.yaml`: closed roots,
  `kind` literal, optional `description: string` (empty allowed), required
  `categories: string[](not-empty)`, required `types: schema(mapping; not-empty)`.
- `StandaloneSchemaEnvelope::{ExpressionSchema, MutationSchema}`, recognized by `kind`;
  `is_callable_catalog()`, `label()`, and `allowed_keys()` (guarded against the definitions by
  `allowed_keys_match_each_shipped_definition`). `admits_top_level_key` refuses `$schema` for
  them.
- `envelope.rs`: both definitions in `ENVELOPE_DEFINITIONS` (so every YAML kind registry and
  Darkmatter's recorded `kind_definitions` see them), `EnvelopeDefinitions` holds five lowered
  definitions (`ENVELOPE_DEFINITION_PATHS`, `LoweredEnvelopes`), and callable catalogs are
  admitted with `validate_strict` (no metadata conversion); every other envelope keeps soft
  admission. `shipped/registry.rs` records five definitions; `PREPARATION_REVISION` 4 → 5.
- `check_written_labels` refuses `null` `description`/`categories`/`types` and a bare-null
  `types` entry (located at the entry); a quoted `"null"` stays the null type.
- Privacy: `NotSchemaReason::CallableCatalog` (role reason, hint, wording "describes callable
  signatures and cannot be used as a document schema or a named-type import"), raised by
  `parse_referenced_schema_document_with_mode` (the single reader behind disk, embedded, and
  bundle references; malformed catalogs are refused for their kind through
  `claimed_envelope`), `check_standalone_named_types`, both `SimplifiedSchema` factories
  (`schema/layer.rs`, `schema/export.rs`), and Darkmatter's `schema_files.rs::parse`.
- Kind checks: `semantic.rs` adds `expression-schema`/`mutation-schema` to `CHECKED_KINDS`
  (not `FILE_READING_KINDS`) with a syntax-only check (the selected parse; nothing resolved).
- Diagnostics: `SchemaError::CallableCatalog { path, message, span, related: Vec<CatalogRelated> }`
  (with a terminal status block) for Phase 2's catalog reader; the YAML loader's duplicate-key
  path now finds both spellings (`marks::duplicate_key_marks`), keeps the error at the repeat,
  appends "the key is first written at line L column C", and exposes
  `yaml_documents::duplicate_key(text) -> Option<DuplicateKey { first, repeated }>` (byte offsets
  in the caller's text, byte-order mark included) for a related location.
- `md schema validate`: `resolve_schema_file` (CLI) now skips callable catalogs as it skips
  namespaces, so a valid catalog stops after its kind check instead of reaching the new
  refusal. Phase 2 replaces this with the full catalog route.
- Regenerated all three shipped artifacts with `just shipped-schemas` (root).
- Docs: `simplified-schema/docs/topics/language/{schema-documents,unions,index}.md` (new
  "Callable catalogs" section, private callable type scope, index rows),
  `simplified-schema/docs/topics/diagnostics.md` (`CallableCatalog` reason and error). Repo
  skill: `.claude/skills/simplified-schema/schema-documents.md`.

#### Requirement-to-test mapping (all in `simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs` unless noted)

| Requirement | Test |
|---|---|
| both kinds recognized, private (`exports_schema = false`), strictly admitted against their definition | `both_catalog_kinds_are_recognized_private_and_admitted_strictly` |
| definitions ship as kind definitions | `the_catalog_definitions_ship_as_kind_definitions`; existing `envelope_admission::the_library_definitions_are_admitted_the_yaml_schema_by_itself` iterates the new entries |
| allowed keys = definition keys | `allowed_keys_match_each_shipped_definition` |
| `description`: absent ok, `""` ok, `null`/bare/number/list refused, located | `description_may_be_omitted_or_empty_but_not_null_or_another_type` |
| soft admission kept for other envelopes | `a_tagged_schema_still_converts_its_metadata_softly` |
| `categories`: absent, null, scalar, `[]`, one bad element, all bad | `categories_are_required_nonempty_strings` |
| category entry grammar once (trim, blank label, missing description, duplicate, case-sensitive) | `category_entries_follow_the_shared_entry_grammar` |
| `types`: absent, null, list, `{}`, scalar, bare-null entry, quoted `"null"`, located grammar failure | `types_are_a_required_nonempty_mapping_without_null_entries` |
| carrier spellings; mixed and wholly invalid union arms | `type_carrier_spellings_and_union_arms_are_parsed_by_the_shared_grammar` |
| unknown keys (`$schema`, `when`, `name`) | `unknown_keys_are_refused_including_schema_and_when` |
| inline examples passive | `inline_examples_are_passive_data` |
| mutation kind wired to the same validators | `the_mutation_catalog_is_wired_to_the_same_validators` |
| duplicate overloads | `duplicate_overloads_are_refused_by_the_catalog_parse` |
| syntax-only kind check, located after a non-ASCII character, reads no file | `the_kind_checks_are_syntax_only_and_located` |
| duplicate key: repeat located, first named, both marks, BOM | `a_duplicate_key_is_refused_at_the_repeat_and_names_the_first`; unit `marks::tests::a_duplicate_key_is_marked_at_both_spellings` |
| loader: second document, trailing garbage, comments + CRLF | `the_loader_refuses_trailing_documents_and_accepts_comments_and_crlf` |
| factories refuse catalogs | `the_schema_factories_refuse_a_catalog` |
| `CallableCatalog` reason is a role refusal with a hint | `the_callable_catalog_reason_is_a_role_refusal_with_a_hint` |
| `$schema` target and `Name@file` import refused, well formed or malformed; a types library still imports (control) | `file::a_catalog_is_refused_as_a_schema_target_and_an_import_source_well_formed_or_not` |
| reference reader refuses malformed catalogs for their kind | `file::the_reference_reader_refuses_a_malformed_catalog_for_its_kind` |
| never expanded as a types library | `file::a_catalog_is_never_expanded_as_a_types_library` |
| bundle (registered) member catalog not importable | `file::a_bundle_member_catalog_cannot_be_imported` |

Existing tests updated for the new definition count: `shipped_artifacts::a_renamed_source`
(the renamed identity must still sort first, now that `expression-schema.yaml` leads the
sorted sources) and `shipped_artifacts::one_invalid_source` (the appended invalid source is
index 6, not 4).

Later additions in this phase:

- Darkmatter's whole-YAML route judges a document softly against its selected definition, so
  strictness is also enforced in `check_written_labels` (non-string `description`, non-string
  category entry); covered by `the_kind_checks_are_syntax_only_and_located` and Darkmatter's
  `yaml_kind_registry::a_callable_catalog_validates_against_its_shipped_kind_definition`
  (shipped definitions through `DarkmatterSchemas::validate_yaml_document`: route, valid
  controls for both kinds, strict `description`, located undeclared category).
- Darkmatter tests updated for the two new shipped kind definitions:
  `yaml_kind_registry::the_shipped_registry_claims_every_shipped_envelope_kind` (renamed from
  `…_the_four_envelope_kinds`), `each_shipped_definition_validates_against_the_shipped_yaml_schema_definition`,
  and `shipped_schema_corpus.rs` (both definitions are `Class::YamlSchemaDefinition` exceptions).
- Wiring spot-check with the built `md`: `md schema validate text.yaml` on a valid expression
  catalog → valid against `built-in simplified-schema/schemas/expression-schema.yaml`, exit 0;
  with `description: 5` → `description … must be a string, got number (at line 2, column 14)`,
  exit 1.

### Gates

- `just shipped-schemas` (root) regenerated all three artifacts; a second run reported
  "every shipped artifact is current" with identical bytes.
- `simplified-schema`: `just test` → 2614 + 2190 passed, none ≥ 5 s; `just test-minimal` →
  2190 passed; `just lint` → clean; `just check-tier-coverage simplified-schema` → 0 stranded.
- `darkmatter`: `just test` → 17914 passed, 10 skipped, none ≥ 5 s (load ≈ 6–7 on this host);
  `just lint` → clean (including the zed-dmls wasm check); `just check-tier-coverage darkmatter`
  → 0 stranded. Two earlier runs each failed one test that encoded the old count of shipped
  kind definitions (fixed above), never a timing failure.
- Dependents (serially): `cargo check --tests` for claudine, claudine-cli, claudine-gen,
  messenger, messenger-cli, research, biscuit-icon-cli, biscuit-speaks-cli,
  biscuit-terminal-cli, playa-cli, sniff-detection-cli → clean. Claudine/claudine-cli tests
  filtered to `registration | doctor | shipped | host_ | schema | kind | envelope | installed`
  → 2041 passed (this also covers the Claudine baseline the start of the phase could not
  run). Messenger and claudine-gen `schema | shipped | frontmatter | kind` → 25 passed.
- Scope calculator after all edits: `packages` = darkmatter, simplified-schema, test-toolkit,
  claudine, darkmatter-cli; `change_class` = package, so the gates were required and ran.
- Cross-OS: no path, process, or OS-specific code changed (envelope recognition, admission,
  errors, and a YAML marks pass over in-memory text), so no `just cross-check` was run; CI
  covers the other environments.
- Code-smell grep over the changed readers: no new `#[serde(default)]`, `Option<T>`
  presence ambiguity, `filter_map(.. as_str())`, `unwrap_or_default()`, or `.ok()` on a
  load-bearing field; null versus absent is decided from the raw YAML mapping.

## Phase 2

### Catalog reader (Rules 4–6)

One reader, `markdown/compose/expression/catalog/mod.rs::read_catalog`, now
judges every callable catalog: Darkmatter's built-in `functions.yaml` and
`mutations.yaml` (`load_catalog`/`load_catalog_with_mode`,
`CatalogOrigin::BuiltIn`) and every other catalog (`load_authored_catalog`,
`Authored`, used by `HostFunctionCatalog::load`, `md schema validate`, and
DMLS). After the shared Simplified Schema parse it checks, in order and with
document byte ranges from the named-type source map: the expected kind; that
`types` declares at least one callable and no entry mixes `function(...)`
arms with an ordinary type (located at the first ordinary arm); every
entry's references stay inside the catalog, name a declared helper, never
use a callable as a type, and hold no `{{ … }}` slot; names that fold
together without regard to ASCII case (later key, earlier related); category
presence and agreement across overloads (missing at the key, disagreement at
the later arm with the earlier related); no `narrows(...)` in a mutation,
nested signatures included; the declared narrowing facts; and, for
`Authored` only, no reuse of a built-in canonical name (related: the
built-in's key in `functions.yaml`) and an eager compile of each callable
(`FunctionRegistration::builder_from_types(..).build_with`), with overload
indices from `RegistrationError` mapped back to arms (`checks::Locator`).
File-form examples are judged against the supplied table only; an authored
catalog has none. Registered aliases are projected only for the built-in
expression catalog. The located checks live in the new
`catalog/checks.rs`.

`functions/mod.rs::reserved_builtin_name` answers the name-only lookup from
the static `bindings()` rows (the joined registry loads the catalog).
`HostCatalogError` now carries `CatalogRelated` and the underlying
`SchemaError`. `DarkmatterSchemas::validate_callable_catalog` (in
`schemas/yaml_documents/mod.rs`) runs the reader for a loaded YAML document,
recognizing the built-ins by `PathIdentity` (relative, or under the captured
repository root); `prepare_yaml_contract` returns its
`SchemaError::CallableCatalog` and leaves every other failure to the kind
contract. `CALLABLE_CATALOG_CODE` is exported from `markdown::schemas`.

The two built-in catalogs are retagged (`kind: expression-schema`,
`kind: mutation-schema`), as are the host fixture and every inline host
catalog in tests; ordinary function-language fixtures stay `kind: schema`.
`functions.yaml` left `SHIPPED` (it was there only for the
`recent-commits@darkmatter:functions` import) and joined
`shipped.rs::sources()`; `PREPARATION_REVISION` is 5. Because a catalog can
no longer be imported, `context-variables.yaml` declares its own
`recent-commits` helper, pinned equal to the catalog's by
`recent_commits_shared_type.rs`. Simplified Schema, Darkmatter, and Claudine
shipped artifacts were regenerated after the last source edit; the host
fixture's facts companion and the context-variables recursive golden were
re-blessed (the golden's only diff is the added helper).

Tests updated rather than weakened: built-in collisions that used to fail at
`HostFunctions::build` now fail earlier at catalog load
(`host_functions.rs`, `min` and `try`/`Try`/`TRY`; the builder still refuses
the alias `TRY`); `shipped_schema_corpus.rs` expects the catalogs refused
as `CallableCatalog`; `global_catalog.rs`'s context-block extraction follows
the moved helper; the Claudine fixed-work test warms the envelope
preparation it now triggers (see decisions). Simplified Schema's
`shipped_corpus.rs` stopped supplying the two catalogs as partials (nothing
imports them, and they are refused as content).

### Wave 1: CLI catalog route (subagent)

`darkmatter/cli/src/commands/schema/validate.rs`: `validate_yaml`
recognizes a callable catalog from its selected route (`Route::Kind` with
either kind) and returns before `resolve_schema_file`; the Phase 1 guard
inside `resolve_schema_file` is gone and both docblocks are updated. A
catalog failure is a new `FileOutcome::CallableCatalog`: human output shows
the code, the message, `path:line:column` for the failure and each related
location (another file is read under the captured repository root, or the
working directory outside one; unreadable keeps the path without a line);
JSON keeps `error: "schema"` and adds `code`, `line`, `column`, byte `span`,
and `related` location objects. The output contract
`darkmatter/cli/schemas/output/schema-validate.yaml` was widened for that
and every JSON test validates strictly against it; the definition topic's
"JSON Output" paragraph describes the record.

### Wave 1: Editor catalog route (subagent)

DMLS keys catalog preparation on the full text (a same-`kind` edit could
previously reuse an earlier valid result), runs it on the registry worker,
resolves and captures related files there (open buffer first) so the
dispatch thread reads nothing, keeps the kind claim of a failed catalog (no
root `$schema` fallback, no import resolution), publishes one
`dm.schema.callable_catalog` per failure with `relatedInformation`, and
suppresses the envelope's coarser `document_malformed` beside it. Both kinds
use the standalone schema tooling for completion/hover; a catalog in a
schema root is never classified as a named schema. A repeated YAML key now
carries its first spelling as related information in every YAML document.

### Fixes found along the way

- Simplified Schema's duplicate-key message repeated the parser's location,
  which is the enclosing mapping's start, as if it were the key's
  (`… at line 5 column 5; the key is first written at line 12 column 5` for
  a repeat on line 18). The parser's location is now dropped, and the
  message names only the first spelling; the existing test gains a case
  where the mapping start and the first key differ.
- A clippy `redundant_locals` in the reader and a `type_complexity` in the
  new library test were fixed.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| authored control loads; inherited category; no aliases | `callable_catalog_validation::the_host_fixture_is_a_valid_expression_catalog`; `catalog::tests::the_authored_control_loads_with_an_inherited_category_and_no_aliases` |
| helpers-only `types` is not a catalog | `catalog::tests::a_types_table_of_helpers_only_is_not_a_catalog` |
| mixed callable/ordinary entry located at the ordinary arm | `catalog::tests::a_callable_mixed_with_an_ordinary_type_is_located_at_the_ordinary_arm` |
| missing category across all overloads, at the key | `catalog::tests::a_callable_without_any_category_is_located_at_its_key` |
| disagreeing categories, both located | `catalog::tests::overloads_naming_different_categories_point_at_both` |
| ASCII-folded name collision | `catalog::tests::names_matching_without_ascii_case_are_one_callable`; `callable_catalog_validation::a_folded_name_collision_is_a_located_catalog_error_with_the_earlier_key` |
| helper references local, no slot | `catalog::tests::helper_references_stay_inside_the_catalog_and_resolve_no_slot` |
| mutation refuses `narrows`, nested too | `catalog::tests::a_mutation_declares_no_narrowing_even_in_a_nested_signature`; `callable_catalog_validation::a_mutation_catalog_with_narrowing_is_refused`; DMLS `a_mutation_catalog_refuses_narrowing_at_the_declaration` |
| built-in canonical name / alias reuse | `callable_catalog_validation::a_built_in_name_is_refused_with_the_built_in_declaration_related`; `catalog::tests::an_authored_expression_catalog_reuses_no_built_in_name_or_alias`; `host_functions.rs` `min`, `try_reservation` |
| eager compile of authored callables | `catalog::tests::an_authored_callable_compiles_for_eager_evaluation` |
| no file-form examples in an authored catalog; unlisted reference refused from the table | `catalog::tests::an_authored_catalog_has_no_file_form_examples` and the updated example test |
| duplicate overload refused, naming both | `catalog::tests::a_duplicate_overload_pair_is_refused_by_the_parser_naming_both_overloads`; DMLS `a_duplicate_overload_is_one_error_ranged_at_the_later_arm` |
| only the built-in expression catalog projects aliases | `catalog::tests::only_the_built_in_expression_catalog_projects_registered_aliases` |
| byte spans survive non-ASCII text | `catalog::tests::spans_are_document_byte_ranges_after_non_ascii_text`; CLI folded-collision test |
| built-ins recognized by path identity only | `callable_catalog_validation::the_built_in_catalogs_are_recognized_by_path_identity_only` |
| whole-text judgment with the same declaration key | `callable_catalog_validation::the_judgment_reads_the_whole_text_and_evaluates_nothing`; DMLS `catalog_findings_follow_the_buffer_version_and_repeated_reads_prepare_nothing` |
| grammar failure stays a kind-contract problem | `callable_catalog_validation::a_grammar_failure_stays_the_kind_contracts_located_problem`; CLI `catalog_a_malformed_catalog_exits_1_located_as_a_contract_or_load_failure` |
| built-in descriptors, bindings, lifecycle parity | existing `catalog::tests`, `effects::types` tests, `shipped` tests, lifecycle generator oracle (unchanged `lifecycle.yaml`) |
| `recent-commits` stays one shape | `recent_commits_shared_type::ctx_recent_commits_references_the_partials_named_type` |
| catalogs refused as targets/imports in the shipped corpus | `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` |
| CLI: shipped catalogs valid from the checkout | `schema_validate_yaml::catalog_shipped_functions_and_mutations_validate_from_the_checkout_root` |
| CLI: shared fixture valid, sentinels passive | `schema_validate_yaml::catalog_the_shared_host_fixture_is_valid_and_its_sentinels_stay_passive` |
| CLI: semantic failures located with related, exit 1, human + JSON, zero side effects | `schema_validate_yaml::catalog_a_folded_name_collision_exits_1_at_the_key_with_the_earlier_declaration_related`, `catalog_a_built_in_name_reuse_exits_1_with_the_built_in_declaration_related` |
| CLI: malformed catalog located | `schema_validate_yaml::catalog_a_malformed_catalog_exits_1_located_as_a_contract_or_load_failure` |
| CLI: ordinary template schema still resolves | `schema_validate_yaml::catalog_route_leaves_an_ordinary_template_schema_resolved` |
| DMLS: valid control, folded collision, built-in reuse related under repo root | `yaml_document_catalog_editor::the_shared_catalog_control_is_validated_by_its_kind_with_no_findings`, `a_folded_name_collision_is_one_catalog_error_related_to_the_earlier_declaration`, `a_reused_built_in_name_relates_the_built_in_declaration_under_the_repository_root` |
| DMLS: duplicate YAML key | `yaml_document_catalog_editor::a_repeated_yaml_key_is_the_loaders_error_at_the_repetition_with_the_first_spelling_related` |
| DMLS: imports not resolved; kind claim kept, `$schema` is data | `a_catalog_import_is_refused_without_resolving_the_imported_file`, `a_catalog_keeps_its_kind_claim_and_its_root_schema_is_data` |
| DMLS: completion/hover for kind fields and local types | `completion_and_hover_describe_the_catalog_kinds_fields`, `a_catalogs_local_types_are_authored_as_in_a_schema_document` |
| DMLS: buffer-version refresh; repeated reads prepare nothing, queue no job | `catalog_findings_follow_the_buffer_version_and_repeated_reads_prepare_nothing` |
| DMLS: catalog in a schema root is not a named schema | unit `a_callable_catalog_in_a_schema_root_is_not_a_named_schema` |
| duplicate-key message names only the first spelling | `callable_catalog_envelopes::a_duplicate_key_is_refused_at_the_repeat_and_names_the_first` |

### Gates

- Checkpoint (Catalog parity): `cargo nextest run -E 'test(catalog) | test(schema_validate_yaml) | test(yaml_document) | test(schema_reference)'` with `-p darkmatter --features effects-instrumentation,work-counters` (313 passed), `-p darkmatter-cli --features test-fixtures` (45 passed), `-p darkmatter-lsp` (115 passed).
- `darkmatter/`: `just test` 17953 passed, 10 skipped, no test at or over 5 s; `just lint` clean (after the `type_complexity` fix; the changed test file re-run, 7 passed); root `just check-tier-coverage darkmatter` clean.
- `simplified-schema/`: `just test` 2190 + 2614 passed (after the `shipped_corpus` fix); `just lint` clean.
- Claudine (over 200 tests, filtered to the changed symbols): `cargo nextest run -p claudine -E 'test(registration) | test(doctor) | test(shipped) | test(host_catalog) | test(host_function)'` 481 passed, none over 5 s.
- Windows (path-identity risk in the built-in catalog check): `./scripts/cross-check.sh --os windows darkmatter`: 13017 passed, 66 skipped; pass. The first attempt, filtered, failed before running anything because the filter's parentheses break the `just` recipes on both sides; the second ran unfiltered. No observation was published, because the log and plan were edited during the run, so the tested tree differs from the head; no source file changed after the run started.
- Scope (`scripts/ci/affected_scope.py`): claudine, claudine-cli, darkmatter, darkmatter-cli, darkmatter-lsp, simplified-schema, test-toolkit. Also run: `claudine-cli` (4700 tests, filtered the same way as `claudine` plus `test(catalog)`): 99 passed; `test-toolkit`: 592 passed, 3 skipped, 1 failed.

Failure outside this change: `test-toolkit::real_kache_config_selector::real_kache_config_selector_bypasses_the_managed_pin` fails every time on this host (`kache-host: error=doctor-failed`); it exercises the host's real kache install, and nothing under `tools/` or in kache changed. No other tests were skipped or failed.

## Phase 3

### Verified companions (Rule 7) — core loader, done by the orchestrator

- Starting point: `HEAD` `ff0725070`; Phases 1–2 uncommitted in the working tree (they are
  committed separately).
- `darkmatter/schemas/darkmatter-host.yaml` is the closed format 3 contract: `kind`, `format_version`
  (`literal(3)`), `host`, `version`, `files`, `lifecycle`, `digests`; `functions` and `function-pair`
  are gone. Root `just shipped-schemas` regenerated Darkmatter's and Claudine's artifacts (the manifest
  schema is the shipped `darkmatter:host` partial).
- `config/registration/manifest.rs`: `read` checks an integer `format_version` **before** the
  contract; any integer other than 3 (and a version-less manifest with a format 1 field, as 1) is
  `ManifestRefusal::Unsupported` at `format_version` with
  `unsupported host registration format version <n>; update Darkmatter and the host tool, then republish the registration`.
  `read_for_cleanup` additionally accepts a well-formed format 2 manifest through a strict serde decoder
  (closed at every level, digests checked as 64 lowercase hex); it backs only `bundle::admit_manifest`,
  the publisher's "what may I retire" question. A malformed v2 manifest is refused, so it confers no
  cleanup authority. `companion_spelling` derives a catalog's companion through
  `host::facts_artifact_path` (the only naming rule): last `.yaml`/`.yml` → `.facts.json`, earlier dots
  and the directory kept; any other extension is refused at the catalog's `files[i]`.
  `MemberRoots::{Installed, Lexical}` replaces the two closures, so installed, embedded, and lexical
  admission share one member normalizer. The case-fold/duplicate identity check is one function
  (`check_identity`) applied to explicit members and companions alike.
- `config/registration/bundle.rs`: one `attempt` for installed (`load_snapshot_with`), embedded
  (`VerifiedSnapshot::from_embedded`, which now goes through `attempt` with `MemberRoots::Lexical`), and
  pre-read (`verify_read`) bundles. Order: verify explicit members' digests → classify each verified
  `files` item leniently by its top-level `kind` → derive companions (refusing a missing companion
  digest at `files[i]` naming the expected sibling, a companion another member already names, or a
  catalog whose name has no YAML extension) → probe companions → check the digest key set is exactly
  members ∪ companions (a companion-shaped key of an unclassifiable file is tolerated; that file is
  refused when prepared) → verify companion bytes. Every lexically-possible companion is probed before
  any read, so a watcher sees it whichever member is missing. `VerifiedSnapshot::functions()` →
  `companions()` (`BundleCompanion { catalog, facts }`) and `members()`; the revision hashes
  `companions` (sorted catalog→facts paths) and every digest, still order- and root-independent.
  `SnapshotRefusal::legacy_format: bool` → `unsupported_format: Option<i128>`. `AdmittedManifest` gains
  `format_version()`; its members are files, lifecycle, then companions (v3) or pair members (v2).
- `simplified-schema/lib/src/member_envelope.rs`: `MemberKind::ExpressionSchema` (keys `kind`,
  `description`, `categories`, `types`; non-empty `categories` and `types`; `description` optional,
  empty allowed, `null` refused). `kind: mutation-schema` is refused on its `kind` with
  `MUTATION_REFUSAL` = `host-defined actions are not supported` (shared with Claudine's build).
- `config/registration.rs`: an `expression-schema` member loads with `HostFunctionCatalog::load` (the
  authored reader) and `HostFunctions::analysis_only_at` over its verified companion; incomplete
  functions refuse the host at the companion. `name_collisions` walks the host's catalogs in bundle-path
  order and refuses every later claim of a canonical name or alias (folded to ASCII lowercase) at its
  declaration (catalog `types.<name>`, or `/functions/<name>/aliases/<i>` in the companion via
  `biscuit_file::data_locations::locate_json`) with the earliest claim as a related location.
  `RegistrationDiagnostic.related: Vec<RegistrationRelated>` carries those and a catalog failure's
  `HostCatalogError::related()`. `HostFunctionSet` gains `bundle_path`.
- Deleted the process-wide host function cache (`host_function_set_at`, `HostFunctionCatalogError`,
  the discovery probe, `catalog/host_function_cache_tests.rs`, `CatalogOwnerProbe`) and
  `is_host_function_catalog`: registration loads each catalog once per installed revision, and the
  existing owners (`MdRequest::registered_hosts`, DMLS `HostRegistrations`) cache profiles.
- Claudine, minimal to keep it compiling and loadable (Phase 4 owns the inventory work):
  `FileKind::ExpressionSchema` arm, and its manifest encoder now writes `format_version: 3` without
  `functions`.


### Binding facts, Wave 2, and verification

- **Binding facts (`expression/host/{mod,artifact}.rs`, `config/registration/locate.rs`)**:
  `HostFunctions::analysis_only_sited` records typed `ReasonSite` locations (`Field(Vec<String>)`
  resolved via `biscuit_file::data_locations::locate_json`, or `Position(line, column)` from the
  strict JSON parser) without parsing rendered error text. `analysis_only_at`'s direct-call
  behavior is unchanged. Cross-catalog name/alias collisions are walked in normalized bundle-path
  order with ASCII case-folding (`name_collisions`), locating later claims at their catalog key or
  companion alias pointer and attaching the earliest claim as a related location.
- **Compose host scope (`cli/src/commands/compose.rs`, `expression/functions/mod.rs`,
  `compose/pipeline/phases.rs`)**: `run_compose` enters the selected host's function sets for the
  request (passive reference check, static call check, lifecycle, and composition). Worker threads
  composing transcluded children inherit the thread-local sets via `ScopedSets::capture`/`install`.
  Calling an analysis-only host function at runtime fails with
  `host function <name>() requires the host runtime`.
- **Editor host reload (`dmls/src/{providers/dsl.rs,diagnostics/frontmatter.rs,lib.rs}`)**:
  DMLS enters only the selected profile's function sets, projects `RegistrationDiagnostic::related`
  into LSP `relatedInformation`, watches companion paths on both published and refused loads,
  coalesces batched publishes into one reload per settled revision across open documents, and
  reloads nothing on document body edits.
- **Docs and skills**: updated `darkmatter/docs/topics/host-registration.md`,
  `darkmatter/docs/topics/host-functions.md`, `claudine/docs/topics/schemas.md`, and
  `.claude/skills/darkmatter/{SKILL,compose,dmls,expressions,library-surfaces,schema}.md`.
- **Fixes found during final Phase 3 verification**:
  - `schema_language_ownership_guard` flagged `darkmatter/lib/src/config/registration/locate.rs::json_pointer`
    as defining a symbol owned by `simplified-schema`; renamed the internal helper to
    `artifact_pointer`.
  - Updated `claudine/lib/tests/l1/host_catalog.rs` (stopped calling deleted
    `is_host_function_catalog`) and `claudine/lib/tests/l1/{host_publication,installed_registration,registration_inventory}.rs`
    (format 3 manifest / unsupported-version wording) so dependent Claudine tests compile and pass.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| v3 closed manifest, `files: []`, unsupported version (v1, v2, future), frozen v2 refused & watched | `host_registration_snapshot::a_frozen_format_2_manifest_is_never_loaded`, `host_registration::a_format_1_manifest_is_refused_with_the_upgrade_warning_and_watched`, `the_frozen_format_2_install_is_refused_with_its_version_and_watched`, `a_host_with_no_files_loads` |
| v2 cleanup admission kept for well-formed v2 only, malformed v2 confers no authority | `host_registration_snapshot::a_frozen_format_2_manifest_is_admitted_for_cleanup`, `a_format_2_manifest_with_function_pairs_admits_its_pair_members`, `a_malformed_format_2_manifest_confers_no_cleanup_authority` |
| derived companions (`.yml`, multiple dots, non-YAML refused, same-stem pair refused, companion in `files` refused, missing digest vs missing file, mismatch, symlink, case collision, retry & watch candidates) | `host_registration_snapshot::a_yml_catalog_derives_the_same_companion_name`, `a_catalog_name_keeps_every_earlier_dot`, `a_catalog_without_a_yaml_extension_is_refused_at_its_entry`, `a_same_stem_yaml_and_yml_catalog_pair_is_refused`, `a_companion_listed_in_files_is_refused`, `a_missing_companion_digest_is_refused_at_the_catalog_without_a_retry`, `a_missing_catalog_digest_is_refused_at_the_catalog`, `a_missing_companion_file_is_refused_watched_and_retried_only_over_a_changed_manifest`, `a_mismatched_companion_is_refused_at_its_catalog`, `a_companion_reached_through_a_symbolic_link_is_refused`, `a_companion_differing_from_a_member_only_in_case_is_refused`, `a_missing_member_is_refused_with_every_named_companion_watched` |
| `expression-schema` admitted, `mutation-schema` refused at `kind` with `host-defined actions are not supported` | `simplified-schema` `member_envelope::an_expression_catalog_is_a_member_and_a_mutation_catalog_is_refused_on_its_kind`; `host_registration_companions::a_mutation_catalog_member_is_refused_at_its_kind` |
| facts companion matrix (control, absent/null/empty/stray/contradictory/unsupported version/duplicate key/trailing JSON/JSON5/non-ASCII, stale inner hash, missing entry) | `host_registration_companions::the_written_companion_records_explicit_nulls_and_empties_and_loads`, `every_facts_cell_is_refused_where_the_reader_judged_it`, `a_stale_catalog_hash_behind_valid_digests_is_refused_at_the_hash`, `a_missing_entry_is_refused_at_the_functions_map` |
| cross-catalog name/alias collisions with related location, built-in collision, per-host isolation | `host_registration_companions::a_later_catalog_claiming_an_earlier_name_is_refused_at_its_declaration`, `another_host_may_publish_the_same_name_and_alias`, `a_name_spelled_as_a_built_in_is_refused_where_it_is_written` |
| root-discovery isolation (unlisted catalog in schema root or bundle folder supplies nothing) | `host_registration_companions::a_catalog_no_manifest_lists_supplies_no_function`; DMLS `host_catalog_reload::a_catalog_the_manifest_does_not_list_exposes_nothing` |
| CLI compose scope: binding refusal, source exclusion, host-runtime error, transcluded child inheritance, unselected/other-host isolation, missing companion refusal | `compose_host_scope::a_call_that_cannot_bind_is_rejected_and_one_that_binds_needs_the_host_runtime`, `every_transcluded_child_of_a_host_document_inherits_its_functions`, `the_call_is_unknown_in_an_ordinary_document_and_in_another_hosts_document`, `a_catalog_without_its_companion_refuses_the_host_at_its_files_entry` |
| host runtime & lifecycle boundary: operand call vs unknown action key, eager binding + return check, dual built-in, effect-only mutation | `host_runtime_scope::a_host_call_in_an_action_operand_runs_its_handler_and_its_name_is_no_action_key`, `a_host_handler_runs_only_after_binding_and_its_result_is_checked`, `a_built_in_function_that_is_also_an_action_works_in_both_positions`, `an_effect_only_mutation_is_no_ordinary_expression_function` |
| DMLS reload: missing/removed/stale companion, recovery, batched publish one load across docs, body edit reloads nothing, unselected isolation, case-insensitive lookup, cross-catalog collision related info, watch inputs | `host_catalog_reload::a_missing_companion_at_startup_refuses_the_host_until_it_is_published`, `a_removed_or_stale_companion_withdraws_the_host_and_its_restoration_recovers_it`, `a_batched_publish_reloads_once_for_every_open_document_and_an_edit_reloads_nothing`, `an_unselected_registration_exposes_no_host_function`, `a_selected_hosts_function_is_found_case_insensitively_by_name_or_alias`, `a_cross_catalog_name_collision_points_at_the_earlier_declaration`, `the_companion_is_a_watched_input_of_a_published_and_a_refused_load` |

### Gates

- `just shipped-schemas` (root): `every shipped artifact is current`.
- `darkmatter/`: `just test` → 18001 tests passed, 0 failed, none at or above 5.0 s (1m 39s); `just lint` → clean (including `zed-dmls` wasm32-wasip2); root `just check-tier-coverage darkmatter` → 0 stranded.
- `simplified-schema/`: `just lint` → clean (default, `work-counters,terminal`, `--no-default-features`); focused `member_envelope | callable_catalog` tests → passed.
- Dependents (serially, filtered to registration/doctor/shipped/host_catalog/host_function/host_publication): `claudine` → passed; `claudine-cli --features test-fixtures` → passed.
- Scope (`scripts/ci/affected_scope.py`): `claudine`, `claudine-cli`, `darkmatter`, `darkmatter-cli`, `darkmatter-lsp`, `simplified-schema`, `test-toolkit`.
- Code-smell grep across changed readers (`registration.rs`, `manifest.rs`, `bundle.rs`, `host/artifact.rs`, `host/mod.rs`, `member_envelope.rs`): no `#[serde(default)]`, no ambiguous `Option<T>`, no `filter_map(.. as_str())` on load-bearing fields.
- Windows cross-check (`./scripts/cross-check.sh --os windows darkmatter` on `build-win`): 13048 passed, 66 skipped, 0 failed.

## Phase 4

### Inventory encoder (Rule 9)

- `claudine/lib/src/composition/registration/published.rs`:
  - Extended `FileKind` with `ExpressionSchema`, `MutationSchema`, and `ExpressionFacts` alongside `Manifest`, `Trigger`, `Library`, `YamlSchema`, `Namespace`, and `Lifecycle` (`FileKind::as_str()` exposes `expression-schema`, `mutation-schema`, `expression-facts`).
  - Added `RegistrationInventory::from_embedded_sources` (backed by `from_embedded_sources_with`) taking YAML sources, the generated lifecycle artifact, and an explicit companion table keyed by derived published path (`schemas/project.facts.json`). Classifies each YAML source (recognizing both `expression-schema` and `mutation-schema`), emits the format 3 manifest (`files` lists YAML members only, excluding `lifecycle` and `expression-facts`; `lifecycle` names `globals.lifecycle.json`; `digests` hashes every member including derived companions), and validates the entire embedded bundle through Darkmatter's shared verified bundle preparation (`composition::schema::bundle::profile_of_with`) before returning.
  - `RegistrationInventory::claudine()` passes the empty companion table and verifies through `from_embedded_sources_with`, keeping `claudine/registration/` YAML-only and shipping no host expression catalog or `.facts.json` companion.
  - `claudine/lib/build.rs` and `claudine/lib/src/composition/inventory/walk.rs` remain YAML-only and admit members via `simplified_schema::member_envelope::admit_member_text` (accepting `expression-schema`, refusing `mutation-schema` with `MUTATION_REFUSAL`).
- `claudine/lib/src/composition/registration.rs`: kept the v2 writer lock, no-follow preflight, manifest-last atomicity, and format 3 / well-formed format 2 cleanup admission (`bundle::admit_manifest`); updated module and helper docblocks to format 3.
- `claudine/cli/src/commands/registrations.rs`: `FileView::of` exposes all new `FileKind` strings with 16-digit XXH64 fingerprints; added unit test `callable_and_companion_kinds_format_with_xxh64_fingerprints`.
- `claudine/lib/tests/l1/host_catalog.rs`: updated `claudine_ships_no_host_function_catalog_and_no_facts_artifact` to assert both `RegistrationInventory::claudine()` (v3 inventory) and `claudine/registration/` contain no `expression-schema`, `mutation-schema`, or `expression-facts` entries.
- `claudine/lib/tests/l1/host_publication.rs`: added `companion_inventory` fixture (`schemas/project.yaml` declaring `project_double`, alias `double_project`, string source exclusion on `n`, companion generated via `HostFunctions::facts_artifact`, valid trigger, and lifecycle artifact) exercising typed kinds, manifest fields, published bytes, manifest-last order, Darkmatter loading and binding checks, repeat-publish no-op with stable timestamps, and pre-write refusal of missing, stray, and stale companions and `mutation-schema` sources.

### Doctor upgrade

- `claudine/lib/src/composition/registration/inspect.rs`: added `ManifestLoad::UnsupportedFormat { version: i128 }` mapped from `SnapshotRefusal::unsupported_format` in `manifest_load` (format 3 admission via `bundle::admit_installed_manifest`).
- `claudine/lib/src/doctor/registration.rs`:
  - Added `check_registration_with` and `repair_registration_with` taking `&RegistrationInventory` (production `check_registration` / `repair_registration` delegate with `RegistrationInventory::claudine()`), enabling library tests to inject the companion-bearing inventory through the production check and repair callbacks.
  - Updated digest-mismatch note to reference Darkmatter's format 3 loader.
  - Reported `ManifestLoad::UnsupportedFormat { version }` as `Severity::Error` with message naming the version and an update-and-republish remedy (`update Darkmatter and Claudine, then run \`claudine doctor --fix\` to republish the registration` when repair-eligible, or `update Darkmatter and Claudine, then republish the registration` when manual).
- `claudine/lib/src/doctor/registration_tests.rs`: updated existing refused-manifest fixture to `format_version: 3`, added `an_unsupported_v2_manifest_is_a_fixable_error_with_update_and_republish_remedy` (loads the frozen v2 fixture at `darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter`, verifies equivalent register entry + unrelated entry + comments, manifest written last, Darkmatter v3 load via `bundle::admit_installed_manifest` + `bundle::verify_read` + `load_verified`, and repeat-repair no-op), and added `companion_fixture_missing_companion_errors_modified_warns_and_repair_restores_agreement` (missing companion = `error`, modified companion with unchanged manifest = `warn` with digest-mismatch note, repair restores exact bytes and Darkmatter profile agreement).
- `claudine/lib/tests/l1/installed_registration.rs`: added `installed_registration_setup_upgrades_frozen_v2_in_place_and_repeats_as_noop` verifying `setup_claudine` upgrades the frozen v2 fixture in place to v3, writes `manifest.yaml` last, preserves unrelated `register` entries and comments, loads cleanly in Darkmatter, and repeats as a no-op.
- `claudine/cli/tests/l1/doctor.rs`: added `doctor_fix_upgrades_frozen_v2_registration_in_place_and_repeats_as_noop` exercising `claudine doctor --json` and `claudine doctor --fix --json` end-to-end against the frozen v2 fixture.
- **Fixes found during Phase 4 verification**:
  - `installed_registration_setup_upgrades_frozen_v2_in_place_and_repeats_as_noop` initially used `assert_one_claudine_profile`, which asserted `diagnostics().is_empty()` even though the test intentionally included an unrelated nonexistent `~/hosts/mine.yaml` entry to verify preservation; updated the assertion to verify Claudine's upgraded v3 profile loads with zero diagnostics for Claudine while only `~/hosts/mine.yaml` is reported absent.
  - `registrations_command::only_the_registration_module_locates_or_loads_an_installed_bundle` flagged `load_entries` inside `claudine/lib/src/doctor/registration_tests.rs` (since the source guard scans all `.rs` files under `lib/src/`). Replaced those test calls with a `load_installed_profile` helper using `bundle::admit_installed_manifest` + `bundle::verify_read` + `load_verified` (matching `inspect.rs`), while `claudine/cli/tests/l1/doctor.rs` continues to exercise `load_entries` end-to-end.
- **Docs and skills**: updated `claudine/docs/topics/schemas.md`, `claudine/docs/cli/doctor.md`, `claudine/docs/cli/registrations.md`, `claudine/docs/topics/configuration.md`, and `.claude/skills/claudine/SKILL.md`.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| Claudine ships no host expression catalog or `.facts.json` companion in v3 inventory or `claudine/registration/` | `claudine::l1 host_catalog::claudine_ships_no_host_function_catalog_and_no_facts_artifact` |
| `FileKind` exposes `expression-schema`, `mutation-schema`, `expression-facts` with 16-digit XXH64 fingerprints in `claudine registrations` | `claudine-cli` `commands::registrations::tests::callable_and_companion_kinds_format_with_xxh64_fingerprints` |
| Companion-bearing inventory fixture (`project.yaml` + generated facts + trigger + lifecycle): manifest fields, typed kinds, published bytes, manifest-last, Darkmatter alias + source exclusion checks, repeat-publish stable mtimes | `claudine::l1 host_publication::companion_inventory::companion_inventory_publishes_lists_and_republishes_without_changing_mtimes` |
| Missing, stray, stale companions and `mutation-schema` source refused before any write | `claudine::l1 host_publication::companion_inventory::companion_inventory_refuses_missing_stray_stale_companions_and_mutation_before_any_write` |
| Doctor inspection & repair on companion-bearing inventory (missing = `error`, modified with unchanged manifest = `warn` + digest note, repair restores agreement) | `claudine` `doctor::registration::tests::companion_fixture_missing_companion_errors_modified_warns_and_repair_restores_agreement` |
| Doctor v2→v3 upgrade on frozen v2 fixture (library): unsupported v2 error + update/republish remedy, manifest last, equivalent/unrelated entries & comments kept, Darkmatter loads v3, repeat no-op | `claudine` `doctor::registration::tests::an_unsupported_v2_manifest_is_a_fixable_error_with_update_and_republish_remedy` |
| Doctor v2→v3 upgrade on frozen v2 fixture (CLI): `claudine doctor --json` and `claudine doctor --fix --json`, Darkmatter `load_entries`, repeat no-op | `claudine-cli::l1 doctor::doctor_fix_upgrades_frozen_v2_registration_in_place_and_repeats_as_noop` |
| Setup v2→v3 upgrade on frozen v2 fixture: in-place conversion, manifest last, unrelated entries & comments preserved, Darkmatter loads v3, repeat no-op | `claudine::l1 installed_registration::installed_registration_setup_upgrades_frozen_v2_in_place_and_repeats_as_noop` |
| Source guard: no installed-bundle loader outside `composition/registration/` | `claudine-cli::l1 registrations_command::only_the_registration_module_locates_or_loads_an_installed_bundle` |

### Gates

- `cargo nextest run -p claudine --features work-counters -E 'test(registration) | test(host_publication) | test(host_catalog) | test(setup) | test(doctor)'` → 449 passed, 0 failed, 0 slow.
- `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(registration) | test(host_publication) | test(host_catalog) | test(setup) | test(doctor)'` → 95 passed, 0 failed, 0 slow.
- `cargo nextest run -p claudine-contract` (reverse dependency of `claudine`) → passed, 0 failed.
- `cd claudine && just lint` → clean (`lint-transport` 9 passed / 0 slow, `lint-lifecycle-doc-facets` passed, clippy clean across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, `claudine-gen`).
- `just check-tier-coverage claudine` → 0 stranded.
- Scope (`scripts/ci/affected_scope.py` on Phase 4 files): `source_packages` = `["claudine", "claudine-cli"]`, `reverse_dependencies` = `["claudine-contract"]`, `change_class` = `"package"`.
- Code-smell grep across changed Phase 4 files (`registration.rs`, `inspect.rs`, `published.rs`, `doctor/registration.rs`, `commands/registrations.rs`): no `#[serde(default)]`, no ambiguous `Option<T>`, no `filter_map(.. as_str())`, and no `.ok()` on load-bearing validation (`classify_yaml_source`'s fallback `.ok()` only assigns a display kind before Darkmatter's strict `profile_of_with` rejects the bundle).
- Cross-OS: all path handling uses `Layout::destination` / `native_path` / `biscuit_file::canonicalize_simplified`, and no OS-specific or `#[cfg(windows)]` logic changed; CI covers Windows/macOS/WSL2.

### Follow-up verification

- Confirmed both Phase 4 tasks were already checked off in commit `ede1dbe8d`. That commit also included a Cargo sweep documentation edit unrelated to either task.
- The L1 regression `host_publication::companion_inventory::companion_inventory_refuses_sources_using_the_reserved_manifest_name` first failed because a facts companion supplied as `manifest.yaml` disappeared before verification. The same fixture now checks case variants and YAML members, asserts a diagnostic naming the supplied file, and checks that the isolated home was untouched. This is an F1 input-validation test at the public inventory API. The existing publication round trip witnesses the cross-crate filesystem boundary at F2 through Darkmatter's installed reader. The setup and doctor tests use the frozen shipped v2 bundle and the normal entry points to exercise two successive upgrade reads.
- In `claudine/`, `BISCUIT_TEST_NARROW='package(claudine-contract) | test(registration) | test(host_publication) | test(host_catalog) | test(setup) | test(doctor)' just test` passed 594 tests with none at or above 5 seconds, including the new regression at 0.026 seconds. The filter excluded 12,492 tests outside this slice. Passing a second `-E` instead combined its matches with the entire L1 selection, so that invocation was canceled and replaced with the intersected filter.
- In `darkmatter/`, `just test` passed 18,001 tests with none at or above 5 seconds. The ten existing skips were the six DMLS alias and scalar projection cases, three opt-in timing measurements, and `resolve_alias_ll`, which requires a login-shell alias.
- In `darkmatter/`, `just lint` passed, including the Zed WASM extension check.
- In `claudine/`, `just lint` passed, including all nine transport checks with none at or above 5 seconds and the lifecycle documentation guard. `just check-tier-coverage darkmatter claudine` found zero stranded tests. The new test is in Claudine's declared `l1` binary and ran under the default L1 filter.
- `just test claudine-contract` ran separately after the area checks. All 48 tests passed with no skips and none at or above 5 seconds. The Claudine CLI tests that exercise registration listing, setup, and doctor ran in the scoped area invocation.
- The prescribed working-tree scope calculation reported `packages` = `["claudine", "test-toolkit", "darkmatter", "darkmatter-cli"]` and `change_class` = `"package"`. It identified `claudine` as the only source package and `claudine-cli` and `claudine-contract` as its reverse dependencies. The worktree also contains Phase 5 documentation and other pre-existing edits. The inventory fix compares a portable name in memory before publication and adds no OS-specific I/O. No new parser, deserializer, or configuration field was introduced by the follow-up.

## Phase 5

### Wave 3 — Language, host, and publisher guidance

- **Language guidance**:
  - Updated `simplified-schema/README.md`, `simplified-schema/docs/topics/language/schema-documents.md`, `docs/kind-documents.md`, `schemas/kind.yaml`, and `.claude/skills/simplified-schema/SKILL.md`.
  - Documented the separation between Simplified Schema's strict envelope admission / syntax-only kind checks and Darkmatter's callable semantic checks, added a compact example with a private helper type (`Label`), category (`Text`), and two-arm overload (`kebab_case`), updated the envelope admission diagram and definition ownership table to all six definitions, and added `expression-schema` and `mutation-schema` to `schemas/kind.yaml` and `docs/kind-documents.md`.
- **Host guidance**:
  - Updated `darkmatter/README.md`, `darkmatter/docs/topics/host-functions.md`, `darkmatter/docs/topics/host-registration.md`, `darkmatter/docs/topics/lifecycle/host-artifact.md`, `darkmatter/docs/topics/expression-engine.md`, `.claude/skills/darkmatter/SKILL.md`, `darkmatter/docs/use-skills/use-darkmatter/{authoring,cli}.md`, and `darkmatter/docs/use-skills/use-dmls/{SKILL,schemas}.md`.
  - Documented both `expression-schema` and `mutation-schema`, the format 3 `files`/`digests` manifest with derived `.facts.json` companions, the Rust host binding generating `facts_artifact()`, the scope boundary (`declaration → selected-document analysis → host-only evaluation`), the `when` disallowance (keeping lifecycle scope in `globals.lifecycle.json`), and the in-place v2→v3 upgrade workflow (`claudine doctor`, `claudine doctor --fix`, `claudine setup`).
  - Left `darkmatter/docs/topics/darkmatter-expressions.md`'s generated table and `darkmatter/docs/functions/` to the regeneration step.
- **Publisher guidance**:
  - Updated `claudine/README.md` and `claudine/docs/use-skills/use-claudine/{SKILL,cli,config,library}.md` covering format 3 publication, in-place v1/v2→v3 upgrades via `claudine setup` and `claudine doctor --fix`, `claudine registrations` typed kinds and 16-digit XXH64 hashes, and missing/modified companion severity and repair behavior.

### Generated parity & caller skill publication

- Ran `(cd darkmatter && just regen-expr-doc && just regen-lifecycle-schema)` followed by `just shipped-schemas` twice from the repository root, and `just shipped-schemas --check` in `simplified-schema/`, `darkmatter/`, and `claudine/`.
- Both `just shipped-schemas` runs reported `every shipped artifact is current`, and all three `--check` commands exited 0.
- Baseline hash comparisons (Git blob ids via `git hash-object`):

  | Path | Phase 1 baseline | Phase 5 regenerated | Status |
  |---|---|---|---|
  | `darkmatter/schemas/partials/lifecycle.yaml` | `1f5c316e92f4d4485c4e0ce3cf63c560b265d622` | `1f5c316e92f4d4485c4e0ce3cf63c560b265d622` | byte-identical (`cmp` exit 0) |
  | `darkmatter/docs/functions/load-data.md` | `347c89bdd4e7b2b1a0e97c373d630e008ff94c23` | `347c89bdd4e7b2b1a0e97c373d630e008ff94c23` | byte-identical |
  | `darkmatter/docs/functions/try.md` | `0a9948cda176f8fdf9aa233507e74b65d48f8591` | `0a9948cda176f8fdf9aa233507e74b65d48f8591` | byte-identical |
  | `darkmatter/docs/topics/darkmatter-expressions.md` | `4c7e8e02e0357bf59fe3dd2dd575ff6ceea04aad` | `61c493b2325bb844ab04544a9413b9c185b4c911` | generated table byte-identical; only Phase 2 prose above `<!-- BEGIN GENERATED FUNCTION TABLE -->` changed |
  | `simplified-schema/lib/generated/shipped.json` | — | `a0e0ff6bfd36d634f038a48c61be71822ecb8cee` | stable across repeat `just shipped-schemas` |
  | `darkmatter/lib/generated/shipped.json` | — | `5075787cca0394bf607aa8b0b79624922eeab1d3` | stable across repeat `just shipped-schemas` |
  | `claudine/lib/generated/shipped.json` | — | `788931c8cc3e18004c331dc5fa4b6f1262f3da1f` | stable across repeat `just shipped-schemas` |
  | `claudine/lib/generated/globals.lifecycle.json` | — | `dd33686acdc3584ec59903bd361b9f96f348c360` | stable across repeat `just shipped-schemas` |

- Published caller skills with `(cd darkmatter && just publish-skills)` (`use-darkmatter`, `use-dmls`) and `(cd claudine && just publish-skills)` (`use-claudine`), all succeeding under `--deny all`. Inspected `~/.claude/skills/{use-darkmatter,use-dmls,use-claudine}/` to verify all examples and `{{ }}` template literals composed cleanly with no stray triple braces.

### Representative CLI findings

- Valid built-in catalogs (`md schema validate darkmatter/schemas/partials/functions.yaml darkmatter/schemas/partials/mutations.yaml --output plain`, exit 0):
  - `✔ the document darkmatter/schemas/partials/functions.yaml has a schema definition and is valid based on its definition built-in simplified-schema/schemas/expression-schema.yaml.`
  - `✔ the document darkmatter/schemas/partials/mutations.yaml has a schema definition and is valid based on its definition built-in simplified-schema/schemas/mutation-schema.yaml.`
- Failing authored catalog reusing built-in `min` (`md schema validate <tmp>.yaml --output json`, exit 1):
  - `code: "dm.schema.callable_catalog"`, `line: 5`, `column: 5`, `span: { start: 81, end: 84 }`, `message: "invalid callable catalog …: \`types.min\` reuses the name of the built-in function \`min\`"`, with `related[0]` pointing at `darkmatter/schemas/partials/functions.yaml` (`span: { start: 7377, end: 7380 }`, `"the built-in function \`min\` is declared here"`).
- DMLS and `claudine doctor --fix` / `claudine setup` v2→v3 upgrade and companion repair outputs are verified end-to-end by `darkmatter-lsp::yaml_document_catalog_editor`, `darkmatter-lsp::host_catalog_reload`, `claudine::doctor::registration::tests`, `claudine::installed_registration`, and `claudine-cli::doctor` in the final gates below.
