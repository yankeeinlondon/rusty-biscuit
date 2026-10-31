---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (medium): Informal schema documents reject the permitted categories property"
human_review: true
human_review_items:
  - Confirm or reverse later the use of trimmed reference text in lookup errors.
  - Confirm or reverse later the separate selector test file.
  - Confirm or reverse later selection result types supporting Debug without Clone or equality.
  - Confirm or reverse later preserving specific trigger errors in Darkmatter after loader rejection.
  - Note that catalog tests now follow the already approved ordinary-schema behavior; no new ruling is requested.
  - Confirm or reverse later reporting non-UTF-8 imports as decoding errors.
  - Confirm or reverse later the separate classifier and CLI test files.
  - Confirm or reverse later replacing obsolete raw-YAML acceptance cases with rejection coverage while retaining JSON equivalents.
  - Confirm or reverse later identifying types-only corpus files by their contents.
  - Note that the temporarily deferred preview field was added in the subsequent phase.
  - Confirm or reverse later reparsing frontmatter on schema-cache misses instead of reorganizing preparation.
  - Confirm or reverse later the shared URI encoding repair for punctuation in paths.
  - Note that stale documentation was corrected as behavior changed rather than waiting for the final phase.
  - Confirm or reverse later locating value-hover modules under the frontmatter provider.
  - Confirm or reverse later leaving the first line of a multiline flow-style YAML preview flush.
  - Confirm or reverse later exposing the bare-name predicate rather than storing another result flag.
  - Confirm or reverse later retaining root-level failure reporting when schema-root scanning cannot read a file.
  - Confirm or reverse later retaining the future file-hover context-error branch even though no effective file type is currently available without context.
  - Confirm or reverse later resolving file hovers directly through the existing FileReference method.
  - Confirm or reverse later reading selected schemas as bytes so the shared loader owns decoding errors.
  - Confirm or reverse later trimming descriptions before display.
  - Note that UTF-32 tests were omitted because the server offers only UTF-8 and UTF-16.
  - Confirm or reverse later escaping authored backticks in error reasons.
  - Confirm or reverse later limiting new summaries to Markdown links and leaving wiki-link hovers unchanged.
  - Confirm or reverse later using invalid UTF-8 as the portable failed-read fixture.
  - Confirm or reverse later leaving the existing oversized skill entry file structurally unchanged.
  - Note that final Claudine verification compiled tests rather than repeating its suite after documentation-only changes.
  - Note that implementer metadata follows the run instructions rather than the plan's different model name.
  - Confirm or reverse later retaining the unused schema-advisory framework with its diagnostic code reserved.
  - Note that local cross-platform execution was left to CI; this does not affect review readiness.
  - "Confirm or reverse later (R1-F1 repair): the shared envelope key set is a public `StandaloneSchemaEnvelope::admits_top_level_key` method so DMLS reads the same definition."
  - "Confirm or reverse later (R1-F1 sibling): `to_simplified_yaml` now keeps `categories` for informal documents instead of dropping them."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-30T19:34:25-07:00
spec: 2026-10-30-reference-navigation-and-hover/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/fixes/2026-10-30-reference-navigation-and-hover/log.md
description: "A **fix** review of `2026-10-30-reference-navigation-and-hover/spec.md`"
fix: 2026-10-30-reference-navigation-and-hover/review-1.md
next: 2026-10-30-reference-navigation-and-hover/review-2.md
---

# Review 1: Reference Navigation and Hover

The fix is not ready: one permitted YAML schema shape is rejected. The navigation, value-hover, summary, and refresh tests exercised during this review otherwise passed. This is the first implementation review, so there are no previous findings to reconcile.

## Findings

### R1-F1 (medium): Informal schema documents reject the permitted categories property

**Authority: spec.** Under “YAML schema documents,” the formal schema properties include `categories`, and the informal form is defined as “as `schema`, with `$schema` required.” An informal document with a valid `categories` list therefore must remain a usable schema. This is a less common authoring shape, hence medium priority.

**Defect class:** inconsistent envelope allowlists reject a supported metadata property before the shared category validator can run.

In simplified-schema, [the standalone envelope recognizer](/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/simplified-schema/lib/src/simplified/standalone.rs:312) permits `categories` only with `kind: schema`. Its informal allowlist omits the property. [The referenced-schema classifier](/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/simplified-schema/lib/src/resolve.rs:1041) repeats that omission. Consequently, valid files fail validation and hover; named imports report misleadingly that the file has no `$schema` property.

Reproduction: copy the tagged control from `schema_target_classification.rs`, retaining a small valid payload, then remove only `kind: schema`:

```yaml
categories: [Core]
$schema:
  title: string
types:
  Id: string
```

Reference this file from a Markdown document containing `title: Ship it`. `md schema validate --format json` exits **2**, reporting `unsupported top-level key(s): categories`. Restoring `kind: schema` produces exit **0** and `valid: true`. DMLS similarly shows “could not be used as a schema” instead of the definition preview. These observations used the existing hermetic CLI fixture and the provider registry, not a private parser alone.

The following sites were swept with the same informal/tagged control pair. Rows identify the repair inventory; they are manifestations of one finding.

| Child ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | simplified-schema standalone parser | Valid informal categories, then tagged control | Informal returns `Ok(None)`; tagged returns a document | Both recognized |
| R1-F1.2 | simplified-schema document-schema resolver | Direct file reference | Informal reports unsupported property; tagged resolves | Both resolve |
| R1-F1.3 | simplified-schema target inspection | Same bytes and selected path | Informal classification error; tagged accepted | Both accepted with preview |
| R1-F1.4 | simplified-schema named-type loading | `Id@./subject.yaml` | Informal reports no `$schema`; tagged imports | Both import `Id` |
| R1-F1.5 | simplified-schema delegation | `$schema: ./subject.yaml` in another schema | Informal rejected; tagged resolves | Both resolve |
| R1-F1.6 | simplified-schema root-union file arms | Reference arm plus an inline arm | Informal rejected; tagged resolves | Both resolve |
| R1-F1.7 | darkmatter-cli normal validation | Document naming the file | Informal exit 2; tagged exit 0 | Both validate successfully |
| R1-F1.8 | DMLS schema value hover | Hover on the document's reference | Informal failure explanation; tagged preview | Both show definition previews |
| R1-F1.9 | Darkmatter trigger payload assembly | Trigger naming the file | Informal payload is skipped; tagged payload applies | Both apply |
| R1-F1.10 | DMLS schema-root classification | Same parser result used by registry classification | Informal becomes `Unclassified`; tagged becomes `NamedSchema` | Both classified as named schemas |
| R1-F1.11 | Shared file selection and navigation projection | Same existing target | Clean: selection is independent of contents; links and definitions retain the selected path | Keep navigation to the repairable file |
| R1-F1.12 | JSON schema recognition and example artifacts | Their separate envelope contracts | Clean: JSON uses its dialect validator; examples use their own example envelope, not the informal schema allowlist | Preserve those contracts |

The root-classification result follows directly from DMLS [registry classification](/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/dmls/src/overlay/registry.rs:95): it maps the reproduced `Ok(None)` to `Unclassified`. Navigation was checked by reading the projection's content-independent `Found` branch and running the existing parity tests. The JSON and example contracts rule out this YAML-envelope defect at those sites.

**Smallest repair:** admit `categories` in both informal allowlists and reuse [the existing category validator in simplified-schema](/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/simplified-schema/lib/src/simplified/standalone.rs:370). Update the informal-envelope documentation and add regression coverage at the public resolver plus normal CLI and hover boundaries. No new parser or public-format decision is needed.

## Input robustness matrix

The YAML/JSON referenced-schema readers changed, so this review checked their existing document- and field-shape tests and swept the failing category field through the public resolver, target inspection, standalone parser, and named imports.

The intended default for invalid category shapes is rejection naming the schema file. The valid exceptions are: absence declares no category restriction, `[]` explicitly declares none, and a list of valid category strings is accepted. The reproduction used no category-constrained type, so an empty list remains a valid control.

| Category field shape | Tagged document | Informal document |
| --- | --- | --- |
| Absent | Accepted | Accepted |
| `[Core]` | Accepted | Rejected as unsupported property: R1-F1 |
| `[]` | Accepted | Rejected as unsupported property: R1-F1 |
| Explicit null | Category validation error | Rejected at envelope recognition |
| Scalar `Core` | Category validation error | Rejected at envelope recognition |
| Mapping `{}` | Category validation error | Rejected at envelope recognition |
| Mixed list `[Core, 3]` | Error identifies element 1 | Rejected at envelope recognition |
| All-wrong list `[3]` | Error identifies element 0 | Rejected at envelope recognition |

Every row was exercised through each of the four public entry points above. The tagged cases prove the existing validator already supplies the required distinction; the repair should wire informal schemas to it. Invalid informal shapes are rejected today, not silently defaulted, but they never reach that validator.

The remaining load-bearing envelope fields (`$schema`, `types`, `description`, and JSON dialect declaration) have public-result tests covering accepted controls and rejected shapes. Existing YAML and JSON document-shape tests cover duplicate keys, invalid/trailing content, multiple YAML documents, and strict JSON rejection of JSON5 syntax. The targeted classifier run passed. A search of the changed envelope, inspection, and selection readers found no field-defaulting or silent element-filtering issue; the selector's `.ok()` is limited to an optional creation candidate after a clean miss.

## Verification

All requirements here need **Level 1**: the spec explicitly defines protocol requests and temporary files as sufficient. It makes no requirement about the terminal's keyboard encoding or actual editor rendering. No Level 2/3 gap is reported.

| Requirement | Strongest relevant evidence checked |
| --- | --- |
| Shared lookup, root precedence, missing creation targets, typed failures | L1 resolver tests and validation/navigation parity |
| Schema/file value hovers, descriptions, truncation, ranges, static values, selected union arms | L1 provider registry and response-contract tests |
| Key hovers and expression priority remain intact | L1 unchanged-response and computed-value tests |
| Missing-file diagnostic suppression preserves other errors and runtime checking | L1 typed diagnostic tests and existing CLI runtime tests inspected |
| Markdown summaries, precedence, external targets, buffers, deletion, watched/save refresh | L1 live server sessions and summary extraction tests |
| Schema changes, deletion, repair, higher-priority roots, JSON refresh | L1 live server sessions |
| Accepted YAML/JSON formats and passive shipped artifacts | L1 classifier/inspection tests and shipped-corpus tests inspected |

Executed during review:

- `just test value_hovers` in darkmatter: **23 passed**.
- `just test schema_reference` in darkmatter: **34 passed**.
- `just test referenced_targets` in darkmatter: **48 passed**.
- Scoped nextest run for simplified-schema selection, classification, and inspection: **34 passed**.
- `just check-tier-coverage darkmatter` and `simplified-schema`: **no stranded tests**. Changed L1 modules are declared by their consolidated test targets.
- Temporary review probes reproduced R1-F1 across public APIs, the normal CLI fixture, and DMLS hover. Additional live-session probes confirmed that creating missing delegation, import, example, and root-union dependencies refreshes the prior failure; the example advanced to its envelope-validation error as expected for that probe's payload.
- The simplified-schema `just test` recipe also ran its unfiltered minimal-features leg: **1,118 passed**.

Temporary probe edits were removed after recording results. No implementation changes or formatting were made. Existing linker unwind-table warnings appeared during builds. Cross-platform execution evidence is left to CI as requested and does not affect readiness.

## Implementation decisions (informational)

Two agents independently read the implementation log and spec in parallel. The following inventory reconciles their lists, including the phase-log decision to retain the advisory framework. None requires stopping for a human: each is already decided, reversible, or has a recommended option within the spec. The author may confirm or reverse these later; none blocks repair.

1. **Trimmed error references:** accept consistent trimmed text rather than carrying historical whitespace differences between callers.
2. **Selector test placement:** accept a separate declared L1 module for selection tests.
3. **Result derives:** accept `Debug` alone; the underlying error does not support cloning or equality, and current consumers do not need them.
4. **Trigger errors:** accept translating loader refusals in Darkmatter to preserve specific trigger-reference/cycle errors and the dependency boundary.
5. **Catalog tests:** accept the new assertions; ordinary-schema application was explicitly approved in the spec.
6. **Non-UTF-8 imports:** accept a decoding error rather than calling undecodable bytes a non-mapping.
7. **Classifier test placement:** accept dedicated classifier and CLI-recognition modules.
8. **Obsolete raw-YAML fixtures:** accept replacing unsupported-format acceptance matrices with rejection coverage and keeping the supported JSON twins.
9. **Types-only corpus classification:** accept content-derived classification while checking that named imports remain usable.
10. **Preview placeholder:** the field was deferred only between implementation phases and is present now; no work remains deferred.
11. **Second frontmatter parse:** accept the extra parse on cache misses; avoiding it would require a broader preparation reorganization. Cache-hit reuse is tested.
12. **URI encoding:** accept repairing the shared URI conversion so bracketed paths do not silently lose navigation.
13. **Documentation timing:** accept correcting drift as soon as behavior changed.
14. **Hover module location:** accept child modules that preserve private-helper access.
15. **Flow-style preview indentation:** accept keeping the first line flush; this preserves the authored value without introducing indentation from its key.
16. **Public bare-name predicate:** accept exposing the existing classifier rather than duplicating its answer in stored state.
17. **Unreadable schema-root files:** accept the existing roots-level failure. Its underlying error retains the affected path and reason, and it neither claims absence nor invents a link. Changing trigger-scan policy is unnecessary.
18. **No resolution context:** accept the unavailable file-value hover when no effective file type can be established, with one context diagnostic. Keeping the future formatting branch is harmless.
19. **Direct file resolution:** accept the existing `FileReference` method so typed resolution errors retain their details.
20. **Byte reads:** accept letting the shared loader describe non-UTF-8 schema files.
21. **Trimmed descriptions:** accept trimming surrounding whitespace before display.
22. **Position encodings:** accept testing UTF-8 and UTF-16; UTF-32 is not offered, so its planned test was omitted.
23. **Escaped error text:** accept preserving authored backticks literally in Markdown error reasons.
24. **Markdown-only summaries:** accept leaving wiki-link hovers unchanged; the requested summary behavior names the Markdown-link provider and is implemented there. This is cheap to extend later.
25. **Unreadable fixture:** accept invalid UTF-8 for portable failed-read coverage rather than requiring platform-specific permission manipulation.
26. **Skill size:** accept leaving the preexisting oversized entry file structurally unchanged while correcting drift; the missed plan size target does not affect this fix's behavior.
27. **Final Claudine verification:** accept compiling its tests after the final documentation-only phase, given earlier recorded verification of the changed catalog behavior.
28. **Implementer metadata:** accept the model named by the run instructions rather than the plan's conflicting value.
29. **Reserved advisory framework:** accept retaining the existing infrastructure with its former diagnostic code reserved; removal is unnecessary for this contract.
30. **Cross-platform evidence:** accept deferring local cross-platform runs to CI; the review instructions explicitly exclude that evidence from readiness.

The separately scheduled YAML document-by-kind feature remains outside scope as the author decided. No newly discovered work was moved to another spec. The category rejection in R1-F1 is a repairable implementation gap, not a request to reconsider the approved schema format.
