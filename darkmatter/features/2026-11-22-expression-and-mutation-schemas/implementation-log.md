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
decisions-for-review:
    - |-
      Duplicate YAML keys: the plan asks the loader to record both the first and the repeated key. I kept the shared loader's error type unchanged (`SchemaError::SchemaDocument`, located at the repeated key) and added the first key's line and column to its message, plus a public `yaml_documents::duplicate_key(text)` that returns both byte offsets for any consumer that wants a "related location". The alternative is a `related` field on `SchemaDocument` itself, which would touch every producer and consumer of that widely used variant. If the reviewer prefers the field, the change is confined to `load.rs` plus the CLI and editor projections Phase 2 adds; `duplicate_key` would then become internal.
    - |-
      Strict shape for the two catalog kinds holds on every path, not only on direct admission. Darkmatter's whole-YAML route validates a document softly against its selected kind definition, which would turn `description: 5` into `"5"`. So `check_written_labels` (which runs on every parse, including the syntax-only kind check) also refuses a non-string `description` and a non-string category entry, as it already refuses `null`. The alternative was to make Darkmatter's YAML validation strict for these kinds, which changes a host adapter shared by all kinds. Redo cost if reversed: drop the two checks and add a strict branch in Darkmatter's YAML contract.
    - |-
      `StandaloneSchemaEnvelope::allowed_keys()` is a hand-written list beside the YAML definitions (the plan asks to "expose their labels and allowed keys"). A test (`allowed_keys_match_each_shipped_definition`) compares it with every shipped definition's keys so they cannot drift. Deriving the list from the lowered definitions at runtime would avoid the duplication but needs the artifact loaded for a constant-time question the editor asks often.
    - |-
      `md schema validate` already has a CLI step that resolves a schema file after its kind check. Phase 1's privacy rule would make that step fail on a valid catalog, so I added the minimal skip for callable catalogs now (as namespaces are skipped). The plan schedules the full CLI catalog route for Phase 2; that work replaces this skip and adds the catalog reader's judgment.
message-to-agent: |-
  Phase 1 landed in Simplified Schema (plus small Darkmatter guards). What Phase 2 can build on:
  - `StandaloneSchemaEnvelope::{ExpressionSchema, MutationSchema}`, `is_callable_catalog()`, `label()`, `allowed_keys()`; definitions at `simplified_schema::envelope::{EXPRESSION,MUTATION}_SCHEMA_DEFINITION_PATH`. Both are YAML kinds in every shipped kind registry, and `md schema validate` already routes `kind: expression-schema` to its definition (shipped artifacts regenerated; Simplified Schema `PREPARATION_REVISION` is 5).
  - `parse_standalone_schema_document` admits catalogs strictly, refuses null/wrong-typed metadata and bare-null `types` entries, and parses categories/types (`exports_schema == false`; the callables and helpers are `named_types()`). The kind check for these kinds is syntax-only and resolves nothing.
  - `SchemaError::CallableCatalog { path, message, span: Option<Range<usize>>, related: Vec<CatalogRelated { message, file, span }> }` exists with a status block, but nothing produces it yet: Phase 2's catalog reader should. Darkmatter does not yet map it to a diagnostic code (`dm.schema.callable_catalog` is Phase 2).
  - `yaml_documents::duplicate_key(text) -> Option<DuplicateKey { first, repeated }>` gives both spellings after the loader refuses a duplicate entry (the loader message also names the first key's line and column).
  - Privacy guards: `NotSchemaReason::CallableCatalog` from the reference reader, `check_standalone_named_types`, the factories, and Darkmatter `schema_files.rs::parse`. `darkmatter/cli/src/commands/schema/validate.rs::resolve_schema_file` has a temporary skip for catalogs that Phase 2's CLI route should replace.
  - The built-in `functions.yaml`/`mutations.yaml` are still `kind: schema`; retagging them is Phase 2. When retagged, `shipped_schema_corpus.rs` classifies them as `Class::FunctionCatalog` and its expectations will need checking.
  - v2 fixture for Phases 3–4: `darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/` (exact bytes `claudine registrations publish` installs today).
  - Recipes note: `just test -E '...'` breaks on parentheses in this justfile; run focused tests with `cargo nextest run -p darkmatter --features effects-instrumentation,work-counters -E '...'` (packages `darkmatter`, `darkmatter-cli` with `test-fixtures`, `darkmatter-lsp`).
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
