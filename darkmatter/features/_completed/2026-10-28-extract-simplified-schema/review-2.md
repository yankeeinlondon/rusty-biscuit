---
$schema: feature-review.yaml
next: 2026-10-28-extract-simplified-schema/review-3.md
ready: false
findings:
    - 'R2-F1 (high): Root-union projection loses evaluation results from other accepting arms'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-30T16:25:59-07:00"
spec: 2026-10-28-extract-simplified-schema/spec.md
implemented: true
description: "A **feature** review of `2026-10-28-extract-simplified-schema/spec.md`"
feature: 2026-10-28-extract-simplified-schema/review-2.md
previous: 2026-10-28-extract-simplified-schema/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-28-extract-simplified-schema/log.md
implemented_by: claude/opus
---

# Review 2: Extract SimplifiedSchema

The feature is **not production ready**. All previously enumerated instances are repaired, including the author's dependency ruling. One newly found sibling of the earlier root-union defect still rejects valid raw schemas. It is actionable without an author decision.

## Previous Finding Verification

The verification below uses the implementation and tests, rather than treating the repair log's completion flags as evidence. The required instances include the additional children the repair's Finding Ledger added during its sweep.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Root-union validation drops raw JSON Schema assertions | R1-F1.1–.8: strict/soft content, JSON/YAML files, asynchronous resolution, public prepared validation, and both host validation paths. R1-F1.9–.14: false/true arms, malformed arms, empty unions, and ordinary/generated controls. R1-F1.15–.20: projection callers, eager-file rewriting, caller-origin conversion, cleanup, and root/nested descriptions. | **verified fixed** | Re-ran `root_union_assertions` in both library configurations and all five Darkmatter `raw_root_union_assertions` tests. Inspected shared wrapping, complete-schema conversion checks, root sibling conversion, cleanup traversal, and both description walks. Malformed `anyOf` shapes now produce `Unprepared(BuildValidator)`; the required-property and description cases pass. | None of these listed instances remains. R2-F1 is a newly found sibling involving combined evaluation results, not an unchanged instance or a failed repair of the listed reproductions. |
| R1-F2: Minimal dependency graph retains explicitly excluded globset | R1-F2.1–.4: minimal graph, syntax-feature boundary, absence of filesystem discovery, and default resolver support. | **verified fixed** | `review-1.md`, Human rulings, item 1 explicitly permits `globset` for passive syntax validation. The spec's requirement R2 and acceptance criterion 14 now carry that amendment. Recomputed normal graphs: minimal contains `globset`, with no `gix`, `walkdir`, or `sniff`; default retains file-resolution dependencies. Biscuit-file's syntax feature and both feature/dependency pages match the ruling. | None. The ruling unblocked the finding, and its required follow-through is complete. |
| R1-F3: Default dependency graph includes the HTTP effect stack | R1-F3.1–.4: default graph, repository-only Sniff use, minimal graph, and rendering/host exclusions. | **verified fixed** | Recomputed both normal Cargo graphs: neither includes `reqwest`, `hyper`, `tokio`, `schematic-schema`, or the excluded host/rendering crates. Sniff's `schematic-schema` dependency is optional under `remote`; every use is in its gated `remote/` module. `cargo check -p sniff --features remote --lib` passes. Worktree CLI now explicitly enables the remote feature it uses. | None. |
| R1-F4: Shipped model-config documents fail their schema contract | R1-F4.1–.7: Claude, Codex, Gemini, Goose, Kimi, OpenCode, and Qwen documents; .8: three record validators; .9: four generator cases; .10: list-form control. Repair also swept Antigravity, Kilo, and Pi. | **verified fixed** | Re-ran all 146 claudine-gen L1 tests. The ten-document public validation test, scalar rejection for each of the three record types, and all four previously failing `cli_switches` tests pass. Reviewed the unchanged sidecar contract, list-only data edits, generated catalog changes, and expansion of the sidecar's own named record types. The declared L1 target includes the new test module. | None. The three extra documents and local-type expansion were repaired during implementation; they are not newly open findings. |
| R1-F5: Schema factories lose typed grammar errors and document locations | R1-F5.1–.7: content/file YAML payloads and named types, content/file Markdown, imported files, and delegating asynchronous resolution. .8–.9: direct type-parser and raw-JSON controls. | **verified fixed** | Re-ran all ten `located_schema_errors` tests, including exact LF/CRLF byte ranges, supplied document/file identity, and the original typed `Grammar` cause. Standalone and Markdown adapters retain the cause in `SchemaDocument.source` and project offsets through the existing source mapper. Library file-loader tests preserve raw JSON line/column failures; the direct type parser still reports its local span. | None. Successful schemas need not retain another source map merely to report an already returned parsing failure. |
| R1-H1: Carry out the author's passive-glob dependency ruling | Amend spec, graph expectations, and feature documentation; retain passive parsing. | **verified fixed** | Checked the spec amendment, minimal/default graphs, `biscuit-file/docs/dependencies.md`, and `simplified-schema/docs/topics/features.md` against `review-1.md`, item 1. No guard asserts the superseded `globset` exclusion. | None; no repeat decision is requested. |

## Unblocked Findings

### R2-F1 (high): Root-union projection loses evaluation results from other accepting arms

**Authority — defect:** In simplified-schema, [the root-arm wrapper](../../../simplified-schema/lib/src/validate.rs:1479) removes the original `anyOf` before validating one arm under `allOf`. A realistic raw JSON Schema that combines alternative object shapes with `unevaluatedProperties: false` therefore rejects an object containing fields covered by two accepting alternatives. The complete schema accepts that object through the existing `jsonschema` dependency. This is an incorrect public validation result in reviewed code, independent of whether extraction introduced it. The spec also requires retaining raw JSON Schema and root unions rather than flattening their meaning (Public Schema Object).

**Class:** projecting alternatives independently loses evaluation results that a sibling assertion reads across all successful alternatives. This is a newly found sibling of R1-F1.

For example, a caller allows a record identified by a name or an age, permits both, and rejects undeclared fields:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "anyOf": [
    {"properties": {"name": {"type": "string"}}, "required": ["name"]},
    {"properties": {"age": {"type": "number"}}, "required": ["age"]}
  ],
  "unevaluatedProperties": false
}
```

`{"name":"Ada"}` and `{"age":36}` pass. `{"name":"Ada","age":36}` should also pass: the successful alternatives collectively cover both fields. Instead, the first projected validator sees `age` as undeclared, and the second sees `name` as undeclared. Both strict and soft validation return `InvalidData`. Darkmatter reports `UnknownKey`, naming `age`, for the same valid frontmatter.

This also affects `unevaluatedItems: false`. With `anyOf: [{"contains":{"type":"string"}}, {"contains":{"type":"number"}}]`, the valid array `["Ada",36]` is rejected because each isolated alternative loses the other one's evaluated item. A schema-valued restriction also fails; replacing `unevaluatedProperties: false` with `unevaluatedProperties: {type: boolean}` wrongly rejects the object above even though both properties were already evaluated.

The public reproduction is simply to parse the JSON above with `SimplifiedSchema::try_from(text)` and call `validate_strict` and `validate_soft` with the two-field object. Direct `jsonschema::validator_for(&raw).unwrap().is_valid(&data)` returns `true`, while both facade calls fail. The same distinction was checked with real temporary schema files and a Markdown document referencing them.

The sweep followed the projected validators into acceptance, problem reporting, conversion, normalization, and caller-input selection. Each row below is a repair-ledger instance; controls are included to keep the repair bounded.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Library content factory, strict verdict | Object schema and two-field value above | `InvalidData`, unexpected `age` | Accept unchanged |
| R2-F1.2 | Library content factory, soft acceptance and final verdict | Same schema/value | Rejects, despite no conversion being needed | Accept and borrow input |
| R2-F1.3 | Public `Prepared`, strict/soft | Same raw `Arc<Value>` and value | Both reject | Both accept |
| R2-F1.4 | JSON file factory, strict/soft | Same schema in `schema.json`, bound context | Both reject | Both accept |
| R2-F1.5 | YAML file factory, strict/soft | Same JSON-compatible bytes in `schema.yaml` | Both reject | Both accept |
| R2-F1.6 | Asynchronous `resolve`, JSON and YAML, strict/soft | Same two files | All four verdicts reject | All accept |
| R2-F1.7 | Darkmatter effective-schema normal validation | Same referenced schema and data | `valid: false`, unknown key | Valid |
| R2-F1.8 | Darkmatter effective-schema raw validation | Same schema/data | Same false rejection | Valid |
| R2-F1.9 | Darkmatter document validation | Markdown references schema and declares `name` and `age` | Invalid frontmatter | Valid frontmatter |
| R2-F1.10 | Darkmatter eager-file normalization and accepting-arm selection | Add root `properties.doc` with `darkmatter-file`; create `a.md`; data includes `doc: ./a.md` | No arm accepts, so `doc` remains `./a.md` | Accepted schema permits normalization to `a.md` |
| R2-F1.11 | Darkmatter caller-input composition and arm applicability | Same eager-file schema; provide existing `a.md` through caller-input records | `SchemaValidationFailed`, unexpected `age` | Compose successfully; retain caller file origin |
| R2-F1.12 | Library content strict/soft, array sibling | `contains` alternatives with `unevaluatedItems: false`; `["Ada",36]` | Both reject; complete validator accepts | Both accept |
| R2-F1.13 | Library content strict validation, schema-valued sibling | Object reproduction with `unevaluatedProperties: {type: boolean}` | Rejects; complete validator accepts | Accept |
| R2-F1.14 | Object and array single-alternative controls | Only `name`, only `age`, or only the string item | Accept | Accept |
| R2-F1.15 | Unknown-field control | `{"name":"Ada","extra":true}` under the false restriction | Reject | Reject; do not remove the restriction |
| R2-F1.16 | Nested property union and root `allOf` holder | Move the original whole schema under `properties.person`, or under root `allOf` | Both strict/soft paths accept | Accept; these retain the complete `anyOf` |
| R2-F1.17 | Generated SimplifiedSchema root unions | Existing discriminant, conversion, and arm-diagnostic cases | Library suites pass | Preserve their existing behavior |
| R2-F1.18 | Coercion commit gate | Same already valid object, no pending inputs | Complete structural validator accepts before the arm fallback | Clean for this reproduction; keep complete-schema acceptance |
| R2-F1.19 | Cleanup and problem-description projections | Read the new problem produced by the invalid per-arm verdict | These consume paths/arms; they do not decide validity or compute evaluated-property/item sets | No independent evaluation-loss defect here; preserve the repaired root/nested traversal tests |

The shared construction point is [CompiledValidators::build](../../../simplified-schema/lib/src/prepared.rs:46) in simplified-schema. Its `accepts` and `collect_problems` consumers must agree with the complete root. Darkmatter builds both effective and phase-projected validators through this point. Its [eager-file arm selection](../../../darkmatter/lib/src/markdown/schemas/rewrite.rs:212) and [caller-input arm check](../../../darkmatter/lib/src/markdown/compose/schema_validation.rs:1233) also call the wrapper, so changing only the facade's final verdict would leave two consumers broken. The coercion engine's complete-schema acceptance check precedes its pending-value arm fallback; that fallback must use any corrected wrapper too.

**Recommended repair:** preserve the original alternatives wherever a raw root's sibling assertion needs their combined evaluation results. A small option is to keep the original `anyOf` in the contextual raw-root wrapper and additionally constrain the selected arm, instead of deleting `anyOf`. Alternatively, use the complete raw root for validity and retain arm projections only for reporting and selection where sound. Keep generated-union behavior intact. Use the existing validator; this needs neither a new validation engine nor a general protection layer. Add focused public object/array regressions and the host cases above, and update the wrapper's comments and raw-schema documentation with the repaired behavior. No public format or spec decision needs to change.

## Blocked Findings

None. The author's passive-glob ruling remains accepted. No finding waits on human input.

## Recurrence

R2-F1 repeats the class of `review-1.md`, R1-F1: independent arm projection changes the complete root's validation contract. It is a **sibling the earlier sweep missed**, not an unchanged missing-required/boolean/malformed-arm instance. The first repair restored the sibling keywords but removed the original alternatives those keywords can depend on. The earlier sweep should also have tested evaluated-property and evaluated-item aggregation at the shared wrapper, both compiled verdicts, and its Darkmatter rewrite and caller-input consumers. This review carries those sites together.

This class has a concrete boundary: preserving the result of the existing complete raw-schema validator when splitting a root union. The repair should preserve that context rather than add a new keyword evaluator. The reproduction uses ordinary raw-schema declarations and valid data, so the finding is high priority.

## Verification and acceptance coverage

I used the darkmatter, simplified-schema, rust-testing, and just skills. The review covered the specification, implementation mapping, original and repair logs, prior findings and ruling, current facade and adapter code, repair diffs, test declarations, and normal dependency graphs.

All behavior added by this extraction is passive library work or a file adapter. Level 1 in-process and hermetic filesystem tests are appropriate. The spec adds no keyboard encoding, mouse, scrolling, or new terminal rendering requirement requiring Level 2 or Level 3 evidence.

| User-facing requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Standalone parsing, supplied-source resolution, lowering, schema selection | L1 library language, facade, and shipped-corpus suites | Pass |
| Strict/soft result contract, immutable input, borrowed/owned accepted value | L1 `validation_contract`, facade tests, and public reproductions | Existing cases pass; raw-union acceptance fails R2-F1 |
| Atomic layering, incoming precedence, fluent identity, unchanged caches | L1 facade and private cache-identity tests; source inspection | Pass |
| Content versus file factories, one capture/read, bound and unsaved sources | L1 `schema_files`, capture instrumentation, located-error tests | Pass |
| Nested/quoted type queries and function metadata | L1 facade/query/function tests | Pass within the previously accepted query boundary |
| YAML export and finite recursive/function graphs | L1 parse/export/parse tests; exporter inspection | Pass; unsupported lossless conversions remain typed errors |
| Host baseline/trigger ordering and sharing | L1 effective-schema layering, ownership, and shipped invocation tests | 14 targeted host tests pass; additional public host reproductions expose R2-F1 |
| Consumer generator behavior | L1 claudine-gen suite | 146 pass, including all previous failures |
| Default and minimal dependency boundary | Recomputed normal Cargo graphs; full L1 in both configurations; Sniff remote build | Pass under the author's amended requirement |
| Error identity, typed cause, and exact source offsets | L1 public factory tests for LF/CRLF and imported files | Previous required cases pass |
| Performance and preparation reuse | Existing same-host benchmark record plus re-run L1 cache-identity tests | Prior measurements remain applicable; no new performance finding |
| Documentation and ownership records | Source/document comparison, ownership and dependency guards | Previous dependency and diagnostic drift repaired; raw-union explanation needs the R2-F1 repair |

Commands and results from this review:

- `just test` in `simplified-schema/`: **1,240 passed** with `work-counters,terminal`; **1,099 passed** without default features.
- Darkmatter L1 filter selecting `raw_root_union_assertions`, `effective_schema_layering`, `schema_language_ownership_guard`, and `shipped_corpus`: **14 passed**.
- `cargo nextest run -p claudine-gen --test l1`: **146 passed**.
- `cargo check -p sniff --features remote --lib`: passed.
- `just check-tier-coverage simplified-schema` and `just check-tier-coverage darkmatter`: **zero stranded tests**. The new modules are declared in the L1 targets; library CI enables their optional test features.
- Public-API examples in both packages reproduced R2-F1, including positive/negative controls, file factories, asynchronous resolution, and host consumers. Temporary example sources were removed after use.

No cross-platform proof was made a readiness gate, and no remote host or terminal window was needed. No formatting, implementation repair, or commit was performed by this review.

## Input robustness review

Default outcome: malformed schema structure produces a typed error without evaluating expressions, running shells, fetching remote content, or changing input data. Existing exceptions remain: absent schema-bearing structure yields `MissingSchema`; an empty schema mapping is valid; content parsing can retain unresolved references; optional metadata remains optional. Type-expression text, Markdown grammar, and export writers are outside this configuration matrix.

The library's current public tests cover the shared YAML envelope/declaration validators and raw JSON loader, including missing/null/wrong-type structure, malformed elements, empty mappings/unions, duplicate keys, trailing content, and multiple YAML documents. The repaired adapters change diagnostic preservation, not field admission. Tests for source offsets exercise both LF and CRLF through public content/file results. The Claudine repair keeps the list validator unchanged and proves each of its three record fields reaches it with a scalar rejection and a valid list control.

For R2-F1, the defect occurs **after loading**, in a projection read by the validity decision. I checked the affected field column through the public facade rather than adding a format-by-field Cartesian product:

| Shape | Public result | Assessment |
| --- | --- | --- |
| `anyOf` absent | Ordinary required-property control rejects missing data | Correct, existing regression test |
| `anyOf: null` or `7` | `Unprepared(BuildValidator)` | Correct, reproduced |
| `anyOf` with one or every element malformed | `Unprepared(BuildValidator)` | Correct, reproduced |
| `anyOf: []` | `Unprepared(BuildValidator)` | Correct, reproduced |
| Valid false-schema alternatives | `InvalidData` | Correct, re-run regression test |
| Valid alternatives with root `required` | Missing-required error | Correct, re-run regression tests |
| Valid alternatives with cross-arm evaluation restriction | Valid combined object/array rejected | R2-F1 |
| `unevaluatedProperties` absent | Existing unrestricted root-union controls accept | Correct |
| `unevaluatedProperties: null`, `7`, or `[]` | `Unprepared(BuildValidator)` | Correct, reproduced |
| `unevaluatedProperties: true` or `{}` | Combined object accepted | Correct, reproduced |
| `unevaluatedProperties: false` or `{type: boolean}` | Combined object rejected, unlike complete validator | R2-F1; both boolean and schema-valued restrictions swept |

The matching input row was checked with either alternative alone, both alternatives' fields/items together, a genuinely undeclared field, and nested/full-root-holder controls; their results are in the instance table. Collection-element shapes do not apply to the scalar/object restriction field. Loader-level duplicate/trailing-content cases remain covered by the re-run library suite; the same loaded bytes reproduce the defect in both JSON and YAML factories, so another loader is not needed.

The smell search covered the facade, standalone reader, and changed consumer reader: no newly defaulted load-bearing field was found. The standalone non-string-key filter still rejects unsupported envelopes through its key-count check; inspection fallbacks only represent unavailable typed views; capture fallbacks preserve the existing repository projection contract. None explains away the downstream projection failure.
