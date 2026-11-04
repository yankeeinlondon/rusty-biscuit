---
$schema: feature-review.yaml
ready: false
findings:
    - 'R3-F1 (high): Unclaimed YAML in schema folders receives false trigger errors'
human_review: true
human_review_items:
    - 'R3-F1: the editor treats a null/non-string `kind` in a `schemas/` YAML file as unclaimed, while saved-file Markdown trigger discovery still reads it as a broken trigger (kept unchanged per the review); confirm or align discovery'
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: '2026-11-01T00:59:16-07:00'
spec: 2026-10-30-yaml-document-schemas/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/log.md
description: 'A **feature** review of `2026-10-30-yaml-document-schemas/spec.md`'
feature: 2026-10-30-yaml-document-schemas/review-3.md
previous: 2026-10-30-yaml-document-schemas/review-2.md
next: 2026-10-30-yaml-document-schemas/review-4.md
---

# Review 3

The feature is **not production ready**. All required instances of R2-F1 are verified fixed. One newly found sibling remains: DMLS applies trigger rules to an unclaimed YAML document inside a `schemas/` folder when whole-document selection produces no schema or a preparation failure. The ordinary unfinished input `kind:` gets an unrelated error. This is a missed sibling, not a partial repair of a previously enumerated instance.

The review checked the spec and its closed design rulings, plan, both earlier reviews, original implementation log, repair log and both Finding Ledgers, production paths, and relevant tests. Neither earlier review has a Human rulings section. Accepted lookup, direct-load admission, trigger discovery, and deferred embedded-resolution decisions remain accepted. No new author decision is required.

## Previous Finding Verification

The required list includes the repair ledger's four added children, not only the previous review's eighteen rows.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1: Secondary envelope checks override the selected YAML contract | R2-F1.1–R2-F1.22, checked individually below | verified fixed | Selected-contract parsing, overlay construction, and diagnostics now preserve the selected definition's authority for every listed instance. Public library, CLI, editor-provider, and protocol regressions pass. | No required instance remains. R3-F1 is a newly found sibling involving no-schema and failed-selection outcomes. |

| Required instance | Verification evidence |
|---|---|
| R2-F1.1 | `yaml_document_editor::metadata_named_data_the_selected_contract_accepts_is_clean` and `yaml_document_session::metadata_named_fields_of_a_root_schema_document_are_judged_by_its_contract_alone`: optional string description accepts null without an envelope error. |
| R2-F1.2 | `any_categories_shape_is_data_of_the_selected_contract`: absent, null, scalar, mixed/all-numeric sequence, empty string/list/object, and ordinary string are judged by `unknown`. |
| R2-F1.3 | Editor-provider and protocol acceptance tests accept duplicate strings as ordinary array data. |
| R2-F1.4 | `any_types_shape_is_data_of_the_selected_contract` plus the acceptance tests: array-valued `types` no longer becomes an envelope carrier. |
| R2-F1.5 | Acceptance tests retain `{ mode: production }` as data without resolving `production` as a type. |
| R2-F1.6 | Acceptance tests cover a referenced fallback contract and null description. |
| R2-F1.7 | Acceptance tests cover the two-arm inline root union and null description. |
| R2-F1.8 | `yaml_document_selection::metadata_named_fields_are_data_of_the_selected_contract` passes through Darkmatter's public validation API. |
| R2-F1.9 | `schema_validate_yaml::a_metadata_named_field_is_data_of_the_root_schema` passes both the ordinary-string control and null case through `md`. |
| R2-F1.10 | The acceptance tests cover an ordinary sibling `owner`; `metadata_named_data_the_selected_contract_rejects_is_its_problem_alone` confirms that a rejected array gets exactly its selected contract's error. |
| R2-F1.11 | `kind_semantic_checks::the_shipped_schema_definition_rejects_a_null_description_and_an_unknown_key`: default closed shape and non-null metadata still fail structurally, without a duplicate kind check. |
| R2-F1.12 | `an_opened_schema_definition_admits_an_added_property_and_keeps_its_checks`: checkout definition admits the added key; undeclared categories still fail. |
| R2-F1.13 | `an_opened_yaml_schema_definition_admits_an_added_property_and_keeps_its_checks`: checkout definition admits the added key; unresolved imports still fail. |
| R2-F1.14 | `an_opened_trigger_definition_admits_an_added_property_and_keeps_its_checks`: checkout definition admits the added key; invalid path globs still fail. |
| R2-F1.15 | Three `schema_validate_yaml::an_opened_checkout_*_definition_admits_an_added_property` tests pass for schema, yaml-schema, and trigger definitions. |
| R2-F1.16 | `yaml_document_editor::an_opened_checkout_definition_alone_judges_its_documents_shape` covers all three kinds, including a trigger inside a schema folder. The protocol checkout-schema test also passes. |
| R2-F1.17 | Missing-data and invalid-declaration protocol tests retain errors exactly once, completion, and hover. `a_fallback_schema_names_no_sibling_types` confirms that ordinary sibling data is excluded from the declaration namespace. |
| R2-F1.18 | `kind_semantic_checks::an_imported_file_is_still_admitted_against_the_compiled_definition` and SimplifiedSchema's `envelope_admission` tests preserve direct-load admission. |
| R2-F1.19 | Source inspection: registry candidate preparation still uses the compiled definition. This matches spec ruling 2's explicit candidate-admission step and ruling 8's direct-load boundary; no change required. |
| R2-F1.20 | `providers::SubstrateProvider::hover` defers inside `DocumentOverlay::yaml_contract_covers`; protocol declaration hover passes for fallback YAML. |
| R2-F1.21 | `an_invalid_declaration_of_a_root_schema_document_is_published_once` checks the offending declaration token; `located_inline_failure` supplies that range rather than the entire carrier. |
| R2-F1.22 | `standalone_payload_key` now chooses an exported `$schema` before `types`; the 111 `lsp_session` tests pass, including standalone authoring and error-location tests. |

Earlier findings R1-F1 (fallback publication), R1-F2 (unsaved sources), and R1-F4 (worker preparation and reuse) were closed in review 2. Their protocol regressions passed again in `just test yaml_`, including missing-data assistance, imports inside/outside roots, new unsaved definitions and dependencies, JSON targets, close restoration, worker-thread observations, and data-only edits with no schema reads or compilation. R1-F3 remains withdrawn for the authority correction recorded in review 2: YAML follows the existing gated bare-name lookup policy, with explicit paths available outside the gate. No earlier finding was blocked or becomes newly unblocked here.

## Unblocked Findings

### R3-F1 (high): Unclaimed YAML in schema folders receives false trigger errors

**Authority: spec**, Design 3: “Missing, null, or non-string `kind` values claim nothing,” and “A YAML file with neither ... is not validated.” The acceptance criteria likewise require no validation when neither route selects a schema. The editor must not introduce a second validator with different acceptance rules (Editor lifetime). An unfinished `kind:` is explicitly covered by these rules and is a realistic editor input.

**Class:** a secondary envelope classifier mistakes the absence of a successful validation result for permission to apply trigger rules to ordinary YAML.

Open `schemas/data.yaml` in a workspace, starting with `owner: ops`, then insert the first line:

```yaml
kind:
owner: ops
```

DMLS's YAML result is correctly `NoSchema`, but the published diagnostics contain an error over the entire document:

```text
dm.schema.prepare: invalid schema-trigger: `kind`: must be a string such as `schema-trigger`, got null
```

The shipped CLI returns exit 0, `schema: null`, and no problems for the same file. Adding `$schema: ./missing.yaml` makes the CLI return its one correct reference error (exit 2); DMLS publishes that error **and** the false trigger error. Moving the unchanged document outside `schemas/` removes the false error. Adding a valid root contract also removes it. Users therefore receive contradictory feedback while filling in a kind or repairing a schema reference.

In DMLS, [envelope_overlay](../../dmls/src/overlay/mod.rs:661) sets `judged` only when a completed validation report exists. Its trigger branch runs for every YAML file directly inside a folder named `schemas`. `NoSchema` and `Failed` therefore enter `Err(error) if !judged`, even when the authored kind never claimed a trigger. Darkmatter's [strict trigger parser](../../lib/src/markdown/schemas/triggers/envelope.rs:69) deliberately rejects null and non-string kinds for trigger discovery. DMLS then stores that result as `TriggerError`, which [diagnostic publication](../../dmls/src/diagnostics/frontmatter.rs:167) unconditionally emits. The strict parser's existing contract can remain intact; its application to this whole-YAML document is the defect.

#### Complete instance sweep

The reproduction used the existing editor-provider fixture, changing only `kind` and the presence/form of the fallback contract. It checked 78 combinations: two locations × thirteen kind shapes × three fallback outcomes. A separate normal-protocol test opened the passing control, inserted `kind:`, and then added the missing reference; it confirmed the actual publications. These temporary probes observed current behavior and were removed after execution.

The kind shapes were absent; explicit null; YAML's empty-value null; number; boolean; one string element; empty list; mixed string/number elements; all-number elements; nonempty mapping; empty mapping; empty string; and an unclaimed ordinary string. Collection elements are not independently validated here: the selector rejects the whole non-string carrier as a kind claim.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R3-F1.1 | Darkmatter whole-YAML selector | Null/non-string/empty/absent kind, no fallback; same kinds with valid/invalid fallback | Clean: shared selector treats non-string kind as unclaimed; API selection matrix passes | NoSchema, selected-contract validation, or the fallback's own error respectively |
| R3-F1.2 | Shipped `md schema validate` | `kind:` inside `schemas/`, without fallback and with missing referenced fallback | Clean: exit 0/no schema/no problems; exit 2/one reference error | Same outcomes |
| R3-F1.3 | DMLS trigger classification and overlay, `NoSchema` | Both null spellings; number; boolean; all four list shapes; both mapping shapes, inside `schemas/` | Each gets false `TriggerError`, despite `NoSchema` | No trigger classification or error |
| R3-F1.4 | DMLS trigger classification and overlay, failed fallback | Same shapes plus `$schema: ./missing.yaml` | Each gets false trigger error beside the correct preparation error | Only the fallback preparation error |
| R3-F1.5 | DMLS classification with a selected fallback | All thirteen kind shapes, `$schema: {owner: string}`, inside `schemas/` | Clean: validated without trigger errors | Selected contract alone judges data |
| R3-F1.6 | DMLS location/control branches | All thirteen shapes outside `schemas/`; absent, empty-string, and ordinary unclaimed-string kinds inside it; all three fallback outcomes | Clean: no diagnostics without a contract; no errors with the valid contract; one error for the missing reference | Same outcomes |
| R3-F1.7 | DMLS final diagnostic publication | `TriggerError` produced by rows .3/.4; normal open/change protocol | One false error for NoSchema; two errors for failed fallback. False error spans the whole document | No error for NoSchema; exactly the located fallback error for failed fallback |
| R3-F1.8 | Genuine claimed-trigger checks and direct discovery parser | Existing valid-trigger, invalid-glob, vacuous-match, and count-bound cases; checkout definition with an added key | Clean: appropriate kind errors retained; modified definition's accepted key stays accepted | Preserve claimed-trigger semantics and strict Markdown discovery behavior |

**Recommended repair:** gate the editor's legacy trigger diagnostics on an actual trigger-envelope claim, rather than only on the absence of a successful validation report. For loaded ordinary YAML, preserve the whole-document selection result, including `NoSchema` and preparation failure. Keep the strict public trigger parser and saved-file Markdown discovery behavior unchanged. Preserve malformed genuinely claimed-trigger diagnostics. Add a compact provider matrix and one normal-protocol regression for no-schema → failed fallback → valid fallback, with controls outside the schema folder. Update the nearby comment that currently promises ordinary schema-root YAML remains silent. This is a small classification correction, not a new parser or protection layer.

## Blocked Findings

None. R3-F1 has a recommended repair within the existing spec. It needs no author decision or human-only verification.

## Recurrence

R3-F1 is a **sibling the earlier sweep missed** of review-2.md's R2-F1, “Secondary envelope checks override the selected YAML contract,” itself related to review-1.md's R1-F1. It is not a regression introduced by the repair: the earlier `!judged` path already allowed the strict trigger parser's errors through when no validation report existed.

The previous sweep covered valid fallback contracts and selected checkout definitions, but did not cross the trigger classifier with `NoSchema` and failed-fallback outcomes for null/non-string kinds in schema folders. The required earlier rows now pass; this review carries the remaining classifier and publication sites together. The requirement has a clear boundary: an authored null/non-string kind claims nothing, and a failed fallback owns its own error. No additional syntax or schema machinery is warranted.

## Verification and input robustness

Default outcome for invalid documents is a located load error; invalid schema declarations/references produce a preparation error, with no expression execution or writes. The decided exceptions remain: missing/null/non-string kind claims nothing; an absent fallback yields NoSchema; an explicit null fallback is an error; scalar/sequence/empty document roots are unvalidated; optional ordinary properties retain their existing null semantics; soft validation converts a copy only.

The one whole-YAML loader is exercised by `yaml_document_loading`: passing control, malformed/trailing structure, duplicate/non-string keys, inadmissible numbers, second document, document markers, BOM, CRLF, and unvalidated roots. `yaml_document_selection` covers the load-bearing selection fields and root-union elements. Compiled envelope admission, named-type carriers, non-null properties, closure, narrowing, serialization, imports, export, and layering are covered by the SimplifiedSchema suite. The repaired metadata fields additionally have absent/null/wrong-whole-type/mixed-element/all-wrong-element/empty-shape provider tests. The new sweep above records every shape and outcome for the remaining kind-classification defect.

The code-smell search covered whole-YAML loading/selection/registry/semantic checks, standalone envelope parsing, and editor YAML preparation. Optional caches and source ranges, `.ok()` for an already diagnosed load failure or unreadable dependency hash, and defaults for diagnostic/cache state do not turn invalid configuration fields into passing defaults. The remaining wrong result comes from invoking the wrong classifier, not from permissive deserialization.

All requirements in this feature need **Level 1** evidence: public in-process results, hermetic CLI subprocesses, and normal editor protocol requests. No requirement concerns terminal rendering or physical input. No Level 2/3 or browser evidence is missing. The relevant modules are declared by their L1 targets; `just check-tier-coverage darkmatter` reports zero stranded tests.

Executed during this review:

- Darkmatter `just test yaml_`: **349 passed**, including CLI wiring, both selection routes, registry failure outcomes, loader shapes, editor ranges, unsaved-source refresh, thread placement, and contract reuse.
- Darkmatter `just test kind_semantic`: **16 passed**.
- Darkmatter `just test lsp_session`: **111 passed**.
- SimplifiedSchema `just test envelope`: **41 passed** plus **1,212 minimal-feature tests passed** (the recipe runs the minimal leg unfiltered).
- SimplifiedSchema `just test`: **1,400 default-feature tests and 1,212 minimal-feature tests passed**.
- Two temporary L1 probes passed while asserting/recording the current defect: **78 provider combinations** and a normal-protocol open/change sequence. These establish the finding; they are not passing regression evidence for a repair.
- Direct shipped CLI reproductions confirmed the no-schema and failed-reference controls described above.

Temporary test additions were removed, preserving the pre-review implementation and tests. Only this review and the requested review/spec frontmatter links are changed by the review. No formatter, commit, terminal window, or browser was used. Cross-OS execution remains CI's responsibility and does not affect this readiness judgment.
