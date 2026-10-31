---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-30T19:53:20-07:00
spec: 2026-10-30-reference-navigation-and-hover/spec.md
implemented: false
description: "A **fix** review of `2026-10-30-reference-navigation-and-hover/spec.md`"
fix: 2026-10-30-reference-navigation-and-hover/review-2.md
previous: 2026-10-30-reference-navigation-and-hover/review-1.md
---

# Review 2: Reference Navigation and Hover

The fix is production ready against the specification. The previous finding is verified fixed across every required instance, including the two related sites discovered during repair. No new findings or decisions requiring a human were identified.

The review read the specification, review-1.md, the repair's Finding Ledger in log.md, and the implementation and tests. The earlier review has no Human rulings section. Its accepted implementation decisions remain accepted; this review does not ask the author to reaffirm them.

## Previous Finding Verification

The repair makes simplified-schema's [envelope key predicate](../../../simplified-schema/lib/src/simplified/standalone.rs) the shared definition of permitted properties. The standalone parser, referenced-schema classifier, and DMLS lexical recognition now use it. The existing category validator receives informal documents as well as tagged documents. Evidence below comes from reading those paths and executing the named tests, rather than from completion flags or the repair summary.

Test names in the table are in simplified-schema's [classification tests](../../../simplified-schema/lib/tests/l1/schema_target_classification.rs), except where the owning package or file is specified.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Informal schema documents reject the permitted categories property | All 17 child rows below | verified fixed | Shared predicate inspected; every required repair or clean boundary checked below | Nothing |
| R1-F1.1: Standalone recognition | Informal and tagged controls with categories | verified fixed | `an_informal_document_with_categories_is_recognized_and_exposes_them` passes; asserts envelope, export, and category contents | Nothing |
| R1-F1.2: Document-schema resolution | Direct reference to each control | verified fixed | `an_informal_document_with_categories_is_a_schema_target` passes and checks the lowered title property | Nothing |
| R1-F1.3: Target inspection | Same selected file and bytes | verified fixed | Same test checks successful inspection and the exact authored preview | Nothing |
| R1-F1.4: Named imports | `Id@./subject.yaml` for both controls | verified fixed | `an_informal_document_with_categories_serves_named_imports` passes; checks imported path and lowered property | Nothing |
| R1-F1.5: Delegation | Another schema references the control | verified fixed | `an_informal_document_with_categories_is_a_delegation_and_root_union_arm` passes; delegated title retains its type | Nothing |
| R1-F1.6: Root-union file arm | File reference plus inline arm | verified fixed | Same test passes with both title and count in the union result | Nothing |
| R1-F1.7: Normal CLI validation | Markdown document names an informal schema | verified fixed | darkmatter-cli `schema_target_recognition::an_informal_schema_file_with_categories_applies_its_schema` passes through its hermetic CLI fixture; exit 0 and `valid: true` | Nothing |
| R1-F1.8: Schema value hover | Informal and tagged controls | verified fixed | DMLS `value_hovers::informal_and_tagged_targets_with_categories_preview_their_definition` passes through the provider registry and checks the definition preview | Nothing |
| R1-F1.9: Trigger payload | Trigger names each control | verified fixed | Darkmatter `trigger_discovery_recovery::a_payload_file_with_categories_applies_informal_or_tagged` passes; invalid level data proves the payload is enforced | Nothing |
| R1-F1.10: Schema-root classification | Both controls and a schema-less negative control | verified fixed | DMLS `overlay::registry::tests::informal_and_tagged_schemas_with_categories_classify_as_named_schemas` passes | Nothing |
| R1-F1.11: Selection and navigation | Existing target regardless of content | verified fixed | Shared selector reads no schema contents; navigation reads its selected path. The 34 schema-reference tests pass, including malformed-target navigation and validation/link/definition agreement | Nothing |
| R1-F1.12: JSON and example contracts | Separate recognition paths | verified fixed | JSON classifier remains separate and its accepted/rejected document tests pass. Example artifacts retain their own envelope; neither uses the informal key predicate | Nothing |
| R1-F1.13: Category field shapes | Absent, valid list, empty list, null, scalar, mapping, mixed and all-invalid lists; unknown-key and schema-less controls | verified fixed | `informal_categories_field_shapes` and `categories_without_schema_is_not_an_informal_document` pass; invalid values reach category validation and errors name the file and offending element where applicable | Nothing |
| R1-F1.14: Informal-envelope documentation | Both areas' topic pages, skills, and affected parser/editor comments listed in the ledger | verified fixed | Reviewed the listed changes: informal categories are documented, including the meaning of an empty list and category restrictions | Nothing |
| R1-F1.15: Editor lexical recognition | Informal envelope with categories | verified fixed | DMLS recognition uses the shared predicate; `envelope_claim_recognizes_block_and_flow_presentation` passes with the added control | Nothing |
| R1-F1.16: Informal export | Categories survive parse/export/parse and repeated export | verified fixed | simplified-schema `schema_object::export::informal_categories` passes; export retains categories without adding `kind`, reparses, preserves the lowered schema, and is stable on repeat | Nothing |
| R1-F1.17: Explicit global catalog | Separate strict catalog contract | verified fixed | Darkmatter catalog loader still requires its explicit envelope; the repair changes its documentation only. The minimal-feature shipped-corpus tests pass for Darkmatter and Claudine | Nothing |

The editor recognition and export rows are siblings discovered and repaired by the implementer, not unresolved instances or new findings in this review. No partial repair remains.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None. R1-F1's defect class is repaired at all listed sites, including the added sibling rows. No new instance of that class was found.

## Input Robustness Matrix

The affected YAML envelope reader rejects invalid field shapes with a schema-document error naming the source file. The relevant valid exceptions are absence of categories, which leaves category names unrestricted; an empty category list, which permits no category names; and a list of valid category strings. Empty-list controls use no categorized named type, so they remain valid documents.

The category validator is shared by both envelopes. The informal matrix exercises absent, empty and valid lists, explicit null, a scalar, a mapping, a mixed list, and an all-invalid list through the public resolver. Null/scalar/mapping errors identify the required list shape; mixed and all-invalid lists identify the offending element. Tagged controls and category validation were checked in the same classification suite. Unknown extra properties remain rejected, and categories alone do not make a document a schema.

The same suite exercises description and types field validation, plus the distinct JSON dialect validator. Loader tests cover YAML duplicate keys, invalid trailing content and multiple documents; JSON duplicate keys, trailing content, comments and trailing commas. Valid YAML and JSON controls pass. Target inspection reuses these classifiers rather than implementing another envelope parser.

A search for field defaults, string-filtering collections, ignored parse errors, and defaulted parse results found no new permissive fallback in the repaired recognition or category paths. The reviewed optional source-span recovery may omit a span after a failed reread; it does not change schema acceptance. Preview line defaults follow successful classification and do not turn an invalid document into a valid schema.

## Requirement Verification

The specification explicitly requires Level 1 protocol and temporary-file tests. These features return navigation destinations, Markdown hover text, and diagnostics; actual editor rendering and modifier handling are outside scope. No Level 2 or Level 3 verification gap remains.

| User-facing requirement | Verification level and evidence checked |
| --- | --- |
| Validation, document links, and definition select the same schema; override roots win | L1 shared-selector tests and schema-reference parity tests, including the shipped response contract |
| Missing targets produce the specified creation destination; malformed winning files remain selected | L1 selector/parity tests and live schema-reference sessions |
| Accepted YAML and JSON forms, description, categories, types-only rejection, imports, delegation and unions | L1 classification, schema-object and inspection tests; normal CLI validation; shipped-corpus coverage |
| Schema value previews, ten-line limit, exact total, fenced authored text and paths | L1 value-hover provider tests and passive inspection tests |
| File value outcomes, nested/list values, selected unions, pending values and unchanged key hovers | L1 value-hover tests, including both negotiated position encodings and punctuation/non-ASCII cases |
| Suppress only editor file-existence errors; retain malformed/context/I/O/schema/import/example/required-value failures | L1 frontmatter diagnostic tests; shared runtime validation remains unchanged |
| Markdown summary precedence and fallback; preserve title, path and fragment | L1 provider/substrate assertions inspected and referenced-target protocol tests executed |
| Buffer precedence, close/delete/save/watch refresh, external Markdown dependencies | L1 referenced-target sessions for listed, excluded and outside-workspace targets |
| Schema edit/delete/repair and higher-priority root creation; JSON refresh | L1 live schema-reference sessions asserting hover, links, definition and diagnostics together |
| Reuse prepared schema previews and parse referenced Markdown on demand | L1 work-counter assertions in value-hover and referenced-target tests |

## Executed Checks

All commands below completed successfully through nextest and the package recipes:

| Area | Command | Result |
| --- | --- | --- |
| simplified-schema | `just test schema_` | 211 selected tests passed; recipe's unfiltered minimal-feature leg also passed all 1,119 tests |
| darkmatter | `just test value_hovers` | 24 passed |
| darkmatter | `just test schema_reference` | 34 passed |
| darkmatter | `just test referenced_targets` | 48 passed |
| darkmatter | `just test categor` | 42 passed, including CLI recognition, trigger payload enforcement and schema-root classification repairs |
| darkmatter | `just test envelope_claim` | 21 passed |
| darkmatter | `just test diagnostics::frontmatter` | 47 passed |
| Repository root | `just check-tier-coverage darkmatter` | No stranded tests |
| Repository root | `just check-tier-coverage simplified-schema` | No stranded tests |

The changed integration modules are declared by their consolidated L1 targets; the file-dependent tests are enabled by the package's CI feature configuration. Fixtures anchor filesystem work in temporary directories. No test in these runs exceeded five seconds.

The builds emitted the existing macOS linker unwind-table warning. There were no test failures. This review did not repeat the implementer's full-area lint or all-feature suites and does not claim cross-platform execution evidence; CI supplies that evidence as requested.

Only this review and the requested review/spec metadata were changed. No implementation, test, accepted decision, or lifecycle directory was changed.
