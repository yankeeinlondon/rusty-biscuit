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
created: "2026-10-30T16:50:54-07:00"
spec: 2026-10-28-extract-simplified-schema/spec.md
implemented: false
description: "A **feature** review of `2026-10-28-extract-simplified-schema/spec.md`"
feature: 2026-10-28-extract-simplified-schema/review-3.md
previous: 2026-10-28-extract-simplified-schema/review-2.md
---

# Extract SimplifiedSchema — Review 3

The feature is production ready under the specification and the author's ruling. The previous finding is verified fixed across all 19 required instances. No new findings or remaining defects were identified in this review.

## Previous Finding Verification

I read both earlier reviews, the original implementation log, and both repair ledgers in `log.md`. The outcomes below come from source inspection and verification runs, rather than the implementation flags or ledger dispositions. Earlier findings already closed by review 2 are included to show that their repairs remain intact.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: Root-union projection loses evaluation results from other accepting arms | R2-F1.1–.19, individually checked below: public validation and factories, host validation, eager files, caller inputs, arrays, schema-valued restrictions, controls, coercion, and descriptions/cleanup | **verified fixed** | Re-ran the complete library suite in both configurations and all ten Darkmatter root-union regressions. Inspected every wrapper consumer and checked the restriction and conversion controls. | None. This completes the previously reported sibling of R1-F1; there is no partial repair or newly open sibling. |
| R1-F1: Root-union validation drops raw JSON Schema assertions | R1-F1.1–.8: content/file/async/Prepared and host validation; .9–.14: false, malformed, empty, ordinary, and generated controls; .15–.20: wrapper consumers, eager files, caller conversion, cleanup, root/nested descriptions | **verified fixed** | Library `root_union_assertions` tests retain required/type assertions, boolean semantics, malformed-schema errors, and descriptions. Darkmatter's ten root-union tests retain the eager-file, caller-origin, and cleanup repairs. Generated unions retain the existing wrapper branch. | None. |
| R1-F2: Minimal dependency graph retains explicitly excluded globset | R1-F2.1–.4: minimal graph, passive syntax boundary, absence of discovery, and default resolver support | **verified fixed** | `review-1.md`, Human rulings, item 1 permits passive `globset`. Recomputed normal graphs: minimal retains `globset` but excludes `gix`, `walkdir`, and `sniff`; default retains the resolver dependencies. Spec and feature documentation match the ruling. | None; the accepted dependency decision is unchanged. |
| R1-F3: Default dependency graph includes the HTTP effect stack | R1-F3.1–.4: default graph, repository-only Sniff use, minimal graph, and host/rendering exclusions | **verified fixed** | Both recomputed normal graphs exclude `reqwest`, `hyper`, `tokio`, `schematic-schema`, and the excluded host/rendering packages. Sniff keeps the remote dependency optional; its remote-enabled library check passes. Worktree CLI explicitly enables its remote use. | None. |
| R1-F4: Shipped model-config documents fail their schema contract | R1-F4.1–.7: original seven documents; .8: three record validators; .9: four generator cases; .10: list control; ledger additionally includes Antigravity, Kilo, and Pi | **verified fixed** | All 146 claudine-gen L1 tests pass, including the ten-document validation test, scalar rejection for each record type, local named-type expansion, and the four generator regressions. The test guards its document list against additions on disk. | None. The additional documents were repaired earlier and remain closed. |
| R1-F5: Schema factories lose typed grammar errors and document locations | R1-F5.1–.7: content/file YAML payloads and types, content/file Markdown, imported files and async resolution; .8–.9: direct-parser and raw-JSON controls | **verified fixed** | All ten `located_schema_errors` tests pass in the applicable builds. They inspect the typed grammar cause, supplied file identity, and exact LF/CRLF byte ranges. File-loader tests retain JSON line/column errors; direct parsing retains its local span. | None. |
| R1-H1: Carry out the author's passive-glob dependency ruling | Amend spec requirements and graph expectations, update feature/dependency documentation, preserve passive syntax | **verified fixed** | Compared the author's item 1 with spec requirement R2, acceptance criterion 14, biscuit-file's syntax feature, both dependency records, and simplified-schema's feature page; checked the actual normal graphs. | None; no further decision is requested. |

### R2-F1 required-instance verification

The central repair is in simplified-schema's [root-arm wrapper](../../../simplified-schema/lib/src/validate.rs), which builds the schema used to judge one union alternative. When a root restriction reads evaluated properties or items, the wrapper retains the original alternatives and appends a `true` alternative, while requiring the selected arm through `allOf`. The retained alternatives supply their evaluation results; the selected arm still has to accept the value. The `true` alternative prevents an extra union-wide error from obscuring errors that the host needs to associate with a pending property. It evaluates no additional properties or items.

I followed this wrapper through simplified-schema's [compiled validation](../../../simplified-schema/lib/src/prepared.rs) and [conversion acceptance](../../../simplified-schema/lib/src/coerce.rs), plus Darkmatter's [eager-file selection](../../lib/src/markdown/schemas/rewrite.rs) and [caller-input selection](../../lib/src/markdown/compose/schema_validation.rs). All reach the corrected construction. The unmodified whole schema remains the source for descriptions and traversal.

| Required child | Checked instance | Verification evidence and result |
| --- | --- | --- |
| R2-F1.1 | Content factory, strict object validation | `fields_of_two_accepting_arms_pass_unevaluated_properties_in_the_content_factory_strictly`: both fields accepted; compares with the complete validator. |
| R2-F1.2 | Content factory, soft object validation | Corresponding `content_factory_softly` test: accepts and borrows the original input. |
| R2-F1.3 | Public `Prepared`, both modes | Corresponding `in_prepared` test: both modes accept unchanged. |
| R2-F1.4 | JSON file factory, both modes | File-factory test loads the schema from a bound temporary JSON file; both verdicts accept. |
| R2-F1.5 | YAML file factory, both modes | Same test loads the schema through the YAML factory; both verdicts accept. |
| R2-F1.6 | Async resolution of JSON and YAML, both modes | Same test exercises both extension-selected loaders and all four verdicts accept. |
| R2-F1.7 | Darkmatter normal effective-schema validation | Public effective-schema test accepts `name` plus `age` and still rejects an undeclared `extra` field. |
| R2-F1.8 | Darkmatter raw effective-schema validation | Public raw-validation test accepts both fields. |
| R2-F1.9 | Darkmatter document validation | Markdown references the temporary schema; frontmatter containing both fields validates. |
| R2-F1.10 | Eager-file normalization and arm selection | Public host test validates both fields and changes `./a.md` to `a.md`. |
| R2-F1.11 | Caller-input composition and applicability | Public composition test converts the caller's count and resolves its file from the caller's separate directory; both record fields remain present. |
| R2-F1.12 | Array evaluation across accepting alternatives | `items_of_two_accepting_arms_pass_unevaluated_items`: strict and soft accept the combined string/number array. |
| R2-F1.13 | Schema-valued unevaluated-property restriction | Test accepts the combined object and an additional boolean; rejects an additional string. |
| R2-F1.14 | Single-alternative object/array controls | Single accepting alternative passes each restriction. |
| R2-F1.15 | Undeclared-field control | Strict and soft still reject the undeclared field and report it; the repair does not discard the restriction. |
| R2-F1.16 | Nested property union and root `allOf` holder | Both public modes accept the two-field object through each holder. |
| R2-F1.17 | Generated SimplifiedSchema unions | Existing generated-arm diagnostic test and full library language/conversion suites pass; the generated wrapper branch is unchanged. |
| R2-F1.18 | Coercion acceptance and pending fallback | Soft public cases accept without copying; the pending-key unit test exercises the shared acceptance decision and rejects an undeclared extra field. An additional public test converts a root numeric property beside the restricted union. |
| R2-F1.19 | Cleanup and problem descriptions | Inspected their use of original root/arm schemas; they do not calculate validity or evaluated sets. Root/nested description tests and Darkmatter's schema-guided quoting test pass. |

The pending fallback test deliberately exercises that internal decision directly. It is not evidence that every raw-schema conversion with pending values reaches the fallback: the existing conversion engine may reject a candidate earlier. The public validation and caller-composition cases separately prove the reported defect is repaired through the exposed entry points. No stronger conversion guarantee is introduced here.

## Unblocked Findings

None.

## Blocked Findings

None. No finding requires a human decision.

## Recurrence

No open recurring finding. Review 2 identified R2-F1 as a sibling of review 1's R1-F1. This review verifies the complete reported class and its controls as repaired; it identifies no additional sibling or repair regression. Accordingly, `recurrence` is `false` for this review.

## Specification and verification coverage

I used the darkmatter, simplified-schema, rust-testing, and just skills. The review covered the specification, ownership inventory and implementation mapping, both earlier reviews and the author's ruling, original and repair logs, facade and adapter code, test declarations, dependency graphs, and current topic documentation.

The extraction adds passive schema APIs and filesystem adapters, without a new terminal interaction or rendering requirement. Level 1 tests are appropriate for its public behavior. No acceptance requirement calls for real-terminal capture or keyboard injection at Levels 2 or 3.

| Requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Parse, inspect, resolve supplied sources, lower, and validate independently of Markdown | L1 language, facade, shipped-corpus, and dependency-direction tests in both configurations | Pass |
| Strict/soft semantics, immutable input, borrowed unchanged success, owned conversion, typed failures | L1 `validation_contract`, `schema_object`, unavailable-check cases, and repaired root-union public tests | Pass |
| Atomic layering, incoming precedence, receiver identity, preparation reuse | L1 public layering cases and private cache-identity tests; implementation inspection | Pass |
| One-argument content/file factories, single capture/read, bound contexts, unsaved sources and origins | L1 `schema_files`, context-boundary instrumentation, and located-error tests | Pass |
| Type queries, function metadata, finite recursive exports, typed unsupported exports | L1 facade/query/function/export tests and exporter inspection, retaining the previously accepted query boundary | Pass |
| Darkmatter baseline, layers, triggers, composition, CLI and editor behavior | L1 effective-schema and extraction-characterization cases, public caller/eager-file regressions, CLI schema invocation and DMLS schema tests | Pass |
| Claudine generator and shipped consumer schemas | L1 claudine-gen suite including document controls and malformed record values | Pass |
| Default/minimal dependency boundaries and single ownership | Recomputed normal Cargo graphs; both complete library configurations; ownership guards; Sniff remote-enabled build | Pass under the author's amended requirement |
| Source diagnostics, file identity, exact offsets and CRLF | L1 public content/file/import/async error tests | Pass |
| Performance and shared baseline/validator preparation | Retained same-host before/after benchmark record in the original implementation log; re-run preparation and host reuse assertions | No new performance defect identified; benchmark record was inspected, not remeasured in this review |
| Documentation and package integration | Current language, host, feature, dependency, and diagnostic pages compared with code; declared targets and tier audit checked | Pass |

The raw-union repair is limited to roots declaring the two evaluation-reading restrictions. Generated unions and ordinary raw roots retain their prior construction. Updated library and Darkmatter documentation describe collective evaluation without adding a new conversion promise. Previously accepted export limitations, content-reference behavior, and query traversal limits remain outside new requirements.

## Input robustness review

Default outcome: malformed schema structure returns a typed construction or preparation error and does not execute expressions, run commands, fetch remote schemas, or mutate the supplied data. Intentional exceptions remain: absent schema-bearing structure returns `MissingSchema`; an empty schema mapping is valid; content parsing can retain unresolved external references; optional metadata can be absent. This review does not impose a new format policy on the extracted language.

The re-run tests exercise field validation through the public facade and file factories, and the shared standalone/declaration validators through their owning library suites. They distinguish absent `$schema` from explicit YAML null, reject wrong whole-field types and malformed union elements, accept the empty-mapping control, and reject empty unions. Tagged envelopes validate their `types` mapping and category entries without silently filtering malformed entries. Shared validators need no format-by-field duplication.

Loader coverage includes duplicate YAML/JSON keys, malformed and trailing content, multiple YAML documents, JSON root-shape errors, and LF/CRLF input. YAML-compatible JSON bytes reach the YAML factory in the repaired union tests; strict JSON loading has its separate public factory controls. Markdown body and type-expression grammars, and export writers, are outside the configuration-field matrix.

The latest repair changes a decision after loading, rather than admitting another field shape. Its required object, array, boolean/schema-valued restriction, single-alternative, unknown-field, nested-holder, and generated-language controls are all accounted for in the child table above. The complete-validator comparison confirms the original failing object is valid independently of the two library paths agreeing.

I searched the facade, standalone reader, resolver, and changed consumer path for `serde(default)`, `Option`, `filter_map`, `unwrap_or_default`, and `.ok()` patterns. The standalone key filter is guarded by the envelope key count; optional typed views and bindings represent deliberately unavailable state; descriptive/advisory fallbacks do not replace validation. No additional permissive load-bearing fallback was identified.

## Verification runs

- `just test` in `simplified-schema/`: **1,252 passed** with `work-counters,terminal`; **1,110 passed** without default features.
- `just test raw_root_union_assertions` in `darkmatter/`: **10 passed**.
- `just test schema` in `darkmatter/`: **2,294 passed** across the library, CLI, and DMLS; 10,492 tests excluded by the selection.
- `cargo nextest run -p claudine-gen --test l1`: **146 passed**.
- `just lint` in `simplified-schema/`: passed in default, test-feature, and minimal configurations.
- `cargo check -p sniff --features remote --lib`: passed.
- `just check-tier-coverage simplified-schema` and `just check-tier-coverage darkmatter`: **zero stranded tests**. New modules are declared in their L1 targets, and required optional test features are enabled in package CI metadata.
- Normal dependency graphs recomputed with `cargo tree -p simplified-schema --edges normal --prefix none`, with and without `--no-default-features`.

Cross-platform execution evidence remains a CI/CD responsibility, not a readiness finding in this review. No terminal windows were opened. This review changes only review/specification metadata and this report; it performs no implementation repair, formatting run, commit, or lifecycle-directory move.
