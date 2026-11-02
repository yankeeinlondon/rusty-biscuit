---
$schema: feature-review.yaml
ready: false
findings:
    - 'R2-F1 (high): Secondary envelope checks override the selected YAML contract'
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: '2026-10-31T23:57:28-07:00'
spec: 2026-10-30-yaml-document-schemas/spec.md
implemented: true
description: 'A **feature** review of `2026-10-30-yaml-document-schemas/spec.md`'
feature: 2026-10-30-yaml-document-schemas/review-2.md
previous: 2026-10-30-yaml-document-schemas/review-1.md
human_review_items:
    - 'R2-F1.19: registry candidate preparation still admits discovered yaml-schema files against the compiled yaml-schema.yaml, even when a checkout copy is selected (ruling 2 step 4)'
    - 'R2-F1.12–14: kind checks still report metadata the library cannot read (non-mapping types, non-string categories) as KindCheck; an absent trigger match gives no finding; trigger match values are read raw'
    - 'R2-F1.1–7: in untagged root-$schema YAML, a sibling types is data, not a type library; completion/hover/goto inside $schema no longer reach into it (matches CLI)'
    - 'R2-F1.16: an opened checkout trigger with an extra key shows no discovery-skip warning while open; untagged YAML that fails to load reports dm.frontmatter.yaml_parse with no last-good model'
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/log.md
implemented_by: claude/opus
next: 2026-10-30-yaml-document-schemas/review-3.md
---

# Review 2

The feature is **not production ready**. The original failing instances have been repaired, and the disputed lookup finding is withdrawn after checking its authority. One new finding remains: secondary checks apply schema-envelope rules even when the selected contract permits the document. It includes siblings the first review missed; it is not a regression introduced by the repairs.

The review read the specification, plan, original implementation log, review 1, repair log and its Finding Ledger, production changes, and the relevant public-API, CLI, and editor tests. Review 1 has no Human rulings section. Its accepted decisions remain accepted; none needs a new author decision.

## Previous Finding Verification

“Verified fixed” below means the required behavior is verified for every listed instance. For R1-F3, the requirement was already satisfied: the finding is withdrawn, rather than claiming a repair changed lookup behavior.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: DMLS suppresses required fallback validation | R1-F1.1–R1-F1.7, individually checked below | verified fixed | Normal protocol test deletes the final data property and verifies the missing-property error, completion, and hover; declaration and named-type errors appear once. Overlay assembly retains the YAML report. | No original instance. R2-F1 records newly found siblings of the same class. |
| R1-F2: Unsaved schema sources do not consistently refresh YAML validation | R1-F2.1–R1-F2.18, including the repair's added siblings | verified fixed | Protocol tests and source tracing verify shared buffer reads, new-file discovery, dependency invalidation, saved controls, and restoration on close. | None within the decided YAML scope. The previously accepted Markdown/trigger boundaries remain accepted. |
| R1-F3: Standalone YAML loses bare-name schema lookup roots | R1-F3.1–R1-F3.8 | verified fixed | Library, CLI, and protocol parity tests establish the existing gated frontmatter lookup policy and successful explicit paths outside it. | None. The earlier requested ungated bare-name search exceeded the spec's “existing policy”; that recommendation is withdrawn. |
| R1-F4: YAML schema preparation still runs on the editor request thread | R1-F4.1–R1-F4.8 | verified fixed | Worker prepares selected contracts, conversion plans, reference facts, and standalone imports. Thread evidence covers actual preparation/kind/import work; data-only edits update diagnostics with zero preparation, compilation, schema-target inspections, or counted file reads. | None of the original thread-placement or reuse instances. |

The following is the instance-by-instance evidence behind that assessment. Test names refer to the `yaml_document_session`, `yaml_document_editor`, `yaml_kind_registry`, `yaml_document_selection`, and `schema_validate_yaml` L1 modules unless qualified otherwise.

| Required instance | Verification evidence and result |
|---|---|
| R1-F1.1 | `a_root_schema_without_data_fails_its_required_property`: CLI exits 1 and names `owner`. |
| R1-F1.2 | `deleting_the_last_data_property_of_a_root_schema_document_reports_the_missing_property`: missing `owner` remains an editor error after deletion. |
| R1-F1.3 | Same protocol test starts with the expected type mismatch for sequence-valued `owner`. |
| R1-F1.4 | Same test checks root completion, data-key hover, and declaration completion/hover after deletion. |
| R1-F1.5 | Claimed-kind editor and selection tests retain claiming-definition precedence, including `$schema` as data. |
| R1-F1.6 | `an_invalid_declaration_of_a_root_schema_document_is_published_once`: one precisely ranged declaration error, then one parse error for malformed YAML. |
| R1-F1.7 | `an_informal_standalone_schema_is_judged_as_data_and_each_problem_is_published_once`: named-type diagnostic ownership and missing-key action remain correct. |
| R1-F2.1 | `an_unsaved_definition_edit_refreshes_an_open_document_until_the_definition_closes`: unsaved definition changes affect dependents. |
| R1-F2.2 | `an_unsaved_edit_of_a_type_library_in_a_schema_root_refreshes_the_definitions_dependents`: saved/string control, unsaved/number diagnostic and hover, close restoration, saved change. |
| R1-F2.3 | Corresponding `outside_the_schema_roots` test verifies the same sequence for a relative external import. |
| R1-F2.4 | `an_unsaved_edit_of_an_explicit_root_schema_target_refreshes_the_document_that_names_it`: target buffer replaces disk text and close restores it. |
| R1-F2.5 | `an_unsaved_new_definition_in_a_schema_root_claims_its_kind_until_it_closes`: an editor-only candidate is discovered. |
| R1-F2.6 | Saved/watched steps in the import and explicit-target tests, plus create/delete tests, reach the same results. |
| R1-F2.7 | Close steps restore saved imports/targets/definitions, or remove editor-only claims. |
| R1-F2.8 | `an_unsaved_file_at_a_missing_import_of_a_definition_is_read_until_it_closes`: a previously searched missing import becomes usable without saving. |
| R1-F2.9 | `an_unsaved_file_at_a_missing_root_schema_target_is_selected_until_it_closes`: the same behavior for an explicit target. |
| R1-F2.10 | `an_unsaved_edit_of_an_imported_type_library_refreshes_a_schema_documents_kind_checks` and its in-process editor counterpart verify kind-check imports. |
| R1-F2.11 | `an_unsaved_copy_at_the_shipped_path_replaces_the_built_in_definition`: source-path identity supports editor-only files; the Unix alias case is conditional as documented. |
| R1-F2.12 | Source trace: `overlay/references.rs::preview` checks the active buffer before reading disk; target dependencies invalidate the prepared reference facts. |
| R1-F2.13 | Source trace: `resolve_one_example`, origin-span rereads, and cycle-span rereads use `buffers::read`; the shared buffer tests prove replacement, isolation, and restoration through public resolution. |
| R1-F2.14 | Markdown-only behavior is the already accepted scope boundary; no new requirement imposed. |
| R1-F2.15 | `an_unsaved_edit_of_an_imported_type_library_refreshes_an_open_schema_documents_authoring_state`: both tagged and informal standalone documents refresh authoring state. |
| R1-F2.16 | `an_unsaved_edit_of_a_json_schema_target_refreshes_the_document_that_names_it`: JSON targets also use buffer text. |
| R1-F2.17 | `an_unsaved_definition_in_a_schema_root_folder_that_does_not_exist_claims_its_kind_until_it_closes`: absent-root candidate enumeration is wired. |
| R1-F2.18 | Saved-file trigger behavior is the already accepted scope boundary; no new requirement imposed. |
| R1-F3.1 | `a_bare_root_schema_searches_the_schema_roots_only_inside_a_repository`: outside-repository bare name follows existing frontmatter policy. |
| R1-F3.2 | `outside_a_repository_a_bare_root_schema_resolves_as_frontmatter_does`: Markdown and YAML both exit 2 for the root-only bare target. |
| R1-F3.3 | `a_bare_root_schema_searches_the_schema_roots_only_inside_a_workspace_folder`: Markdown and YAML outside the folder both report preparation failure. |
| R1-F3.4 | The library and editor tests resolve the same bare name inside the repository/workspace gate. |
| R1-F3.5 | Explicit-path controls succeed outside the gate in all three surfaces. |
| R1-F3.6 | `a_registry_built_outside_a_repository_holds_the_shipped_kinds_only`: automatic filesystem discovery stays disabled. |
| R1-F3.7 | `semantic::kind_checks` receives the selected registry's roots; no competing bare-name policy. |
| R1-F3.8 | Contract preparation passes those same roots to `SchemaReferenceFacts::select_in`. |
| R1-F4.1 | `a_yaml_documents_kinds_are_prepared_off_the_dispatch_thread`: registry preparation remains on the worker. |
| R1-F4.2 | `a_yaml_documents_contract_is_prepared_and_compiled_off_the_dispatch_thread`: claimed-kind preparation excludes the dispatch thread. |
| R1-F4.3 | Same test covers inline fallback preparation. |
| R1-F4.4 | Same test covers referenced fallback preparation. |
| R1-F4.5 | `a_data_only_edit_reuses_the_prepared_contract_and_reads_no_schema_file` covers claimed, inline, and referenced contracts; the library's `a_prepared_contract_judges_later_data_as_validation_does_without_compiling` verifies reusable public results. |
| R1-F4.6 | Thread test observes `YamlKindCheck` while a schema imports a type library and after an edit. |
| R1-F4.7 | Same test observes `StandaloneImportCheck` outside dispatch; YAML authoring looks up worker-prepared state. |
| R1-F4.8 | Data-edit test observes zero reference-target inspections; `PreparedYaml` retains reference facts by declaration/context/source dependencies. |

The repair ledger's additional verification entries are not review findings. The declaration key now uses `biscuit_hash`; the cache dependency-count assertion follows the moved cache. The Windows golden-path adjustment is a test-only repair; cross-OS proof is left to CI and is not claimed by this review.

### Correction of the earlier lookup finding

The spec says both “an explicit root `$schema` still works outside those gates” and “Schema-file lookup continues to use the shared selector and its existing policy.” Existing frontmatter lookup outside a repository/workspace resolves explicit file paths but does not search the five schema roots for a bare name. The new parity tests establish that YAML behaves the same way. The explicit-path controls satisfy the first sentence. Requiring a broader bare-name search would change that existing policy, so R1-F3 should not have required it. This is a correction of the review, not an unresolved implementation defect or a pending author choice.

## Unblocked Findings

### R2-F1 (high): Secondary envelope checks override the selected YAML contract

**Authority: spec.** Design 3 says “Otherwise a root `$schema` is the contract” and that properties have only their selected schema's meaning. Ruling 4 says the shipped definitions state the whole shape, with Rust retaining semantic checks; ruling 8 says whole-file reports follow a checkout's selected definition. The acceptance criterion explicitly requires an edited shipped definition to change validation without a Rust change. Both paths below violate those decisions for ordinary data or an ordinary definition edit.

**Class:** a secondary envelope validator or classifier applies a different structural or semantic contract after the document has selected its authoritative schema.

**Fallback data is mistaken for an informal schema definition.** This document is valid through the library and CLI, but DMLS publishes `dm.schema.document_malformed` saying the informal schema envelope forbids null:

```yaml
$schema:
    description: string
description: null
```

Optional `string` admits null under the existing language rules. DMLS's [overlay assembly](../../dmls/src/overlay/mod.rs:225) retains findings and authoring state from the informal-envelope classifier beside the selected YAML report. [Diagnostic publication](../../dmls/src/diagnostics/frontmatter.rs:149) then emits that envelope error even though this document's contract accepted it. Adding an ordinary `owner` property, declared in the inline contract, makes that classifier stop claiming the document and the false error disappears.

The same mismatch affects data named `categories` and `types`. For example, `categories: [ops, ops]` under `categories: string[]` receives an unrelated duplicate-schema-category error. `types: [ops]` under `types: string[]` is rejected as a non-mapping named-type declaration. A data mapping `types: { mode: production }` under `types: unknown` produces an unknown-named-type error for `production`. These are normal configuration shapes, not adversarial strings.

**Selected checkout definitions are checked again under different shape rules.** Copy each real shipped definition to its repository-relative location in a temporary repository, make its directory a schema root, and remove its root `$constraints: { closed: true }` line(s). Keep every other declaration unchanged. The registry is complete and selects that filesystem definition. The normal example document passes; adding `note: hi` passes the selected open shape, but the library returns a `KindCheck` failure:

- For `schema`, [the semantic check](../../lib/src/markdown/schemas/yaml_documents/semantic.rs:107) calls the admitted standalone parser again, which enforces the compiled closed envelope.
- For `yaml-schema`, [the preparation check](../../lib/src/markdown/schemas/yaml_documents/semantic.rs:123) likewise re-admits against the compiled envelope.
- For `schema-trigger`, [the envelope parser](../../lib/src/markdown/schemas/triggers/envelope.rs:92) still has a hard-coded allowed-key list, independently of the selected definition.

The code runs kind checks only after structural validation succeeds, so a report containing only `KindCheck` establishes that the selected definition accepted the added property. This does not ask direct schema factories/imports to follow mutable checkout definitions: ruling 8 explicitly keeps their compiled admission rules until rebuild.

#### Complete instance sweep

The fallback cases start from the existing protocol fixture and change one data field. Definition-override cases start from the real shipped artifacts, change only root closure, and compare the same document before/after adding one property. Each failing row is independently reproducible; clean controls are included.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | DMLS informal-envelope admission: `description` | Inline fallback; `description: null` permitted by selected optional string | False envelope error | No error |
| R2-F1.2 | DMLS informal-envelope admission: `categories` | Selected `unknown`; null, scalar, empty string/object | False envelope errors | Only selected contract judges data; all these values pass |
| R2-F1.3 | DMLS standalone category semantics | Selected `string[]`; `[ops, ops]` | False duplicate-category error | Valid data array |
| R2-F1.4 | DMLS informal-envelope admission: `types` | Selected `string[]`; `[ops]`; shared-shape matrix below | False mapping/reference envelope errors | Valid data array; selected contract controls other shapes |
| R2-F1.5 | DMLS standalone named-type resolution | Selected `unknown`; `{ mode: production }` | False unknown-named-type error | Valid data mapping |
| R2-F1.6 | DMLS referenced fallback | Same optional description, `$schema: ./contract.yaml` | False null-envelope error | No error |
| R2-F1.7 | DMLS root-union fallback | Two inline arms accepting optional description | False null-envelope error | No error |
| R2-F1.8 | Library fallback validation | All fallback matrix inputs above | Valid; clean | Valid |
| R2-F1.9 | CLI fallback publication | `description: ops` control, then null | Exit 0 for both; clean | Exit 0 |
| R2-F1.10 | DMLS ordinary-data classifier control | Declared `owner: ops` beside the null description | No error; clean | No error |
| R2-F1.11 | DMLS/Library unmodified shipped-kind control | `kind: schema` with null description | Selected definition rejects null; clean | Reject null |
| R2-F1.12 | Library `schema` kind semantic check | Valid source-copy definition with root closure removed; add `note` to schema document | Compiled envelope rejects `note` as `KindCheck` | Follow selected open definition; semantic checks only |
| R2-F1.13 | Library `yaml-schema` kind semantic check | Same edit to its real definition; finite kind export plus `note` | Compiled envelope rejects `note` as `KindCheck` | Follow selected open definition; verify export claims |
| R2-F1.14 | Library `schema-trigger` kind semantic check | Same edit to its real definition; valid match plus `note` | Hard-coded key list rejects `note` as `KindCheck` | Follow selected open definition; verify match semantics |
| R2-F1.15 | CLI selected-definition report | All three source-copy controls and added-property cases | Controls pass; added property fails | Pass under the selected open shape |
| R2-F1.16 | DMLS selected-definition report | All three source-copy controls and added-property cases | Controls pass; added property produces schema errors | Pass under the selected open shape |
| R2-F1.17 | Actual declaration diagnostics and assistance | Existing invalid `$schema`/named-type tests and missing-data protocol test | Proper declaration errors appear once; completion/hover remain available; clean | Preserve these results during repair |
| R2-F1.18 | Direct library envelope admission | Factories/imports use compiled `envelope::definitions`, independently of the kind registry | Compiled admission remains authoritative by design; clean invariant | Preserve direct-load admission, as ruling 8 requires |

**Recommended repair:** use the selected whole-YAML contract as the single authority for data shape. For fallback documents, retain declaration assistance and located errors inside `$schema`, but do not treat their other fields as schema-envelope metadata or local type libraries. For the built-in kinds, split already-admitted semantic/payload parsing from direct-load envelope admission so whole-file checks cannot reapply a different shape. Remove the trigger parser's competing structural authority while retaining its match semantics and strict public loading behavior. This is bounded to the existing selection and semantic-check paths; it needs no new format, escaping layer, configuration, or author decision. Add permanent public-result tests for these child rows, retaining the currently correct declaration-error and assistance controls.

## Blocked Findings

None. R2-F1 can be repaired within the spec and its existing decisions. No human-only verification is required.

## Recurrence

R2-F1 repeats the class of **review-1.md, R1-F1: DMLS suppresses required fallback validation**. It is a **sibling the earlier sweep missed**, not an unchanged required instance and not a repair regression. The earlier sweep followed report suppression, completion, and hover, but did not check whether the retained envelope findings and standalone import/category checks could reject data the selected contract accepts. The same competing-authority class also appears in the built-in kind checks, which repeat structural admission after selecting an editable definition. The complete scope is enumerated in R2-F1.1–R2-F1.18.

This requirement has a clear boundary: the selected contract owns whole-document shape; declaration parsing and the specified semantic checks retain their own jobs. The repair should remove the competing validation decisions rather than grow a list of field-specific exceptions.

## Input Robustness Matrix

The shared admitted-YAML loader is unchanged by the repair. Public loading tests cover duplicate keys, malformed/trailing content, a second YAML document, non-string keys, inadmissible numbers, BOM/CRLF, document markers, and scalar/sequence/empty roots. Invalid documents produce located load errors; valid non-mapping roots select no schema. Existing CLI tests prove loading versus preparation exits, assignment/baseline preflight, no-schema records, and mixed-batch precedence.

Default outcome: invalid declarations or values rejected by the selected contract produce located errors without executing or rewriting anything. Exceptions are intentional: absent/null/non-string `kind` makes no claim; absent `$schema` permits no selection; present invalid `$schema`, including null, is a preparation error; optional unmarked properties retain their nullable behavior; non-null optional properties permit absence and reject null. Empty data is judged by its selected type. This feature promises no execution of template or shell-looking strings.

For R2-F1's fallback classifier, the selected declaration is `{field}: unknown`, so **all shapes in this table must pass**. Every cell was exercised through public library validation and ordinary editor protocol messages. `E` means a false editor error; `OK` means both surfaces accepted it. This tests the classifier once across all three fields that can cause the informal-envelope path to claim ordinary data.

| Data shape | `description` | `categories` | `types` |
|---|---|---|---|
| absent | OK | OK | OK |
| explicit null | E | E | E |
| scalar `123` | OK | E | E |
| mixed array `[ops, 123]` | E | OK | E |
| numeric array `[123]` | E | OK | E |
| empty string | OK | E | E |
| empty array | E | OK | E |
| empty object | E | E | OK |
| string `ops` | OK | E | E |

Array elements are ordinary data here, not invalid elements of a selected `unknown` type. Numeric category elements happen to pass the unrelated envelope's soft conversion; that agreement is a clean observation for those cells, not evidence that the classifier is correct. Additional probes cover category duplicate semantics, named-type-looking data maps, a valid declaration-looking type map, the three fallback forms, and a non-metadata-key control. These are all listed in the finding inventory above. The selected-definition sweep covers all three built-in semantic-check sites, with passing controls before adding `note`.

The required smell search covered YAML loading/selection/registry/semantic checks, standalone envelope parsing/admission, and the editor contract cache: `serde(default)`, optional fields, filtered elements, defaulted parses, and discarded errors. None adds another permissive-loading finding. The declaration-key serialization default follows successful admitted loading; non-string mapping keys are rejected before key traversal defaults; optional numeric position conversions affect locations, not schema acceptance. R2-F1 concerns a second, overly restrictive interpretation after successful loading, not a missing field silently becoming empty.

## Verification Level and Execution

All user-facing requirements here concern passive validation, CLI records, and editor protocol results. **Level 1 is the appropriate level**: no terminal rendering, keyboard encoding, mouse, browser, or focus behavior is required. Normal protocol tests exercise diagnostics, completion, and hover; no Level-2/Level-3 gap was found. The YAML tests are declared by each package's consolidated L1 target, and `just check-tier-coverage darkmatter` reports zero stranded tests.

Executed during this review:

- Darkmatter `just test yaml_`: **337 passed**, including the repair's protocol, CLI, registry, selection, loading, and reuse tests.
- Darkmatter `just test kind_semantic_checks`: **9 passed**.
- Darkmatter `just test shipped_schema_corpus`: **12 passed**.
- Darkmatter `just test guard`, after removing the temporary probes: **266 passed**.
- SimplifiedSchema `just test`: **1,398 default-build tests and 1,210 minimal-feature tests passed**, including public validation, envelope constraints, buffer resolution, serialization, imports, layering, and ownership/layout guards.
- **Five temporary review probes passed**, exercising the metadata-shape matrix, CLI fallback controls, all three selected-definition overrides through the public library, and their CLI/editor projections. Their assertions confirm the current wrong outcomes and the passing controls; they establish R2-F1 rather than demonstrate a fix. They are diagnostic probes, not permanent regression coverage.

Temporary probes were removed after verification, restoring the exact pre-review test contents. Production files were not changed. No formatter, commit, terminal window, or browser was used. Cross-OS execution evidence remains CI's responsibility and does not affect this readiness decision.
