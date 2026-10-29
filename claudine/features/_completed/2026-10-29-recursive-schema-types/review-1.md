---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Inline trigger definitions share an identity and corrupt editor results'
  - 'R1-F2 (high): Recursive references lose root-union file match rules'
  - 'R1-F3 (high): Caller file overrides lose their launch-directory meaning through references'
  - 'R1-F4 (high): CLI timing reports omit assignment preparation and misreport cache hits'
observations: []
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: '2026-10-23T21:44:27-07:00'
spec: 2026-10-29-recursive-schema-types/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-29-recursive-schema-types/log.md
description: 'A **feature** review of `2026-10-29-recursive-schema-types/spec.md`'
feature: 2026-10-29-recursive-schema-types/review-1.md
next: 2026-10-29-recursive-schema-types/review-2.md
---

# Review 1

The feature is **not production ready**. Four high findings remain. Each was reproduced through a public library API, the shipped CLI, or an in-process editor session. No decision requires human approval.

## Implementation decisions

Two independent agents read the specification and the implementation log before the code review. Both found `decisions_for_author: []` and no decision meeting the human-review bar.

The implementation keeps acyclic types inlined, stores cyclic types in shared definition tables, emits deterministic definition keys, puts presence and timing on reference uses, and uses weak back references in conversion shapes to avoid memory leaks. It preserves eager discovery errors rather than delaying malformed unmatched trigger payloads; the specification explicitly permits that choice. It also adds stage timing, frozen benchmarks, import parsing caches, and context-independent validator reuse for lazy file references.

The documentation-only `md schema about` command has no schema input. Its unchanged termination test is a reasonable interpretation of the requested invocation coverage. The existing editor descent tests exercise levels 1–3 and distinguish cursor-path types from unopened definitions. Neither choice needs an author decision.

Two departures are forbidden scope narrowing: the log's known gap against criterion 15 substitutes an acyclic named type for recursive `match()` coverage, and Phase 4's caller file-classifier decision leaves recursive file unions unhandled because no shipped schema uses them. These are findings R1-F2 and R1-F3, respectively. They require implementation, not a ruling.

Review decisions and their alternatives, reasons, and costs are recorded under **Review iteration 1 → Design Decisions** in the implementation log.

## Findings

### R1-F1 (high): Inline trigger definitions share an identity and corrupt editor results

**Authority — spec:** Design 4 requires that “inline roots, separate root-union arms, and embedded schemas need distinct identities rather than a shared root sentinel.” Acceptance criterion 8 requires that same-named types cannot collide during trigger assembly. **Defect class:** independently resolved inline namespaces receive equal definition identities, so later typed lookups return another schema's body.

In the `darkmatter` package, [DefNamespace](../../../darkmatter/lib/src/markdown/schemas/simplified/definitions.rs:52) gives every inline mapping the same `InlineRoot` identity. [resolve_file_payload](../../../darkmatter/lib/src/markdown/schemas/triggers/assemble.rs:297) resolves a disk trigger's inline payload without giving that payload the trigger file's identity. [EffectiveSchema::definition](../../../darkmatter/lib/src/markdown/schemas/mod.rs:1034) searches the document table first, then trigger tables, and returns the first equal key. JSON merging correctly separates the emitted keys, but typed lookup still conflates them.

**Reproduction.** Create two matching disk trigger files. Both begin with:

```yaml
kind: schema-trigger
match:
  prompt: string(required)
```

The first has this payload:

```yaml
$schema:
  node: "{ left_value: string, children: node[] }"
  left: node
```

The second declares `node: "{ right_value: number, children: node[] }"` and `right: node`. Open a document with `prompt: x`, `left: {}`, and a `right` mapping. Library completion under `right` offers `left_value`. An actual DMLS protocol session offers `left_value` and `children`, returns no hover for authored `right_value`, and publishes no validation error for `right_value: 2`. The editor and validator disagree on a valid document.

**Class sweep.** Repeated the fixture through `DarkmatterSchemas::effective_for`, `property_shape`, and `completion::for_path_in`, changing only payload location or adding the document's own `node`. Also requested DMLS completion and hover using the same two inline trigger payloads.

| Child ID | Site / shape tested | Observed result | Expected result |
|---|---|---|---|
| R1-F1.1 | Two disk triggers with inline payloads; effective lookup and completion under `left` / `right` | Both offer `left_value` | `left_value` / `right_value`, respectively |
| R1-F1.2 | Same triggers plus document-local `node: "{ own_value: boolean, children: node[] }"` | Both offer `own_value` | Each trigger retains its own body |
| R1-F1.3 | DMLS completion under `right` | Offers the first trigger's keys | Offers `right_value` and `children` |
| R1-F1.4 | DMLS hover over `right_value`, with a description on its definition | `null` hover | The right-hand property's number type and description |
| R1-F1.5 | Same payloads in separate referenced schema files | Clean: distinct left/right keys | Distinct bodies |
| R1-F1.6 | Same inline payloads in an embedded bundle | Clean: distinct left/right keys | Distinct bodies |
| R1-F1.7 | Root-union arm identity and typed-table absorption | Existing namespace/merge tests pass; arm index separates arms within one resolution and file identity separates referenced roots | Separate bodies in the assembled root |

The wrong identity reaches both library completion and DMLS's [SchemaBundle lookup](../../../darkmatter/dmls/src/overlay/schema.rs:938); changing only generated JSON keys cannot repair those decisions. File-backed baselines/extensions and embedded payloads already have source identities that rule out this specific collision. Raw JSON definitions have no typed `DefKey` and use the separately tested JSON re-keying path.

**Smallest repair:** give each independently resolved inline payload a stable source namespace, carry it through all its recursive keys, and retain a regression for both trigger/trigger and document/trigger collisions. Preserve deterministic JSON output. Correct the namespace invariant's comment alongside the implementation.

### R1-F2 (high): Recursive references lose root-union file match rules

**Authority — spec:** Part A requires that “File formats and `match()` below `$ref` or union arms must still select the context-sensitive path”; criterion 15 requires correct request context for references to these schemas. The implementation log explicitly identifies this as a known gap and substitutes a named-type test. **Defect class:** reference-blind lowering drops a load-bearing file-selection constraint before validation and cache classification.

In the `darkmatter` package, [attach_root_union_match](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs:342) asks `file_match_groups` about the unresolved reference-shaped property and then visits only inline fragments. Neither step follows a recursive definition. The resulting schema has no enforcing match keyword on that file alternative.

**Reproduction.** Start with the existing cache-validity fixture: create `a/notes/x.md`; validate documents from folders `a` and `b` using the same long-lived `DarkmatterSchemas`. Replace its direct file type with:

```yaml
$schema:
  - node: "file(match('./notes/*.md')) | { children: node[] }"
    spec: node(required)
  - spec: number
```

The document in `a` supplies `spec: ./notes/x.md`; the document in `b` supplies `spec: ../a/notes/x.md`. The first must pass and the second must fail: the existing file is outside the second document's `./notes` directory. Both pass. The second validation also reports a compile-cache hit because the missing keyword makes the schema appear context independent.

**Class sweep.** Ran this positive/negative pair through the public timed validation API at every affected root-union assembly route. The raw control explicitly puts the existing `x-darkmatter-match` keyword on a definition reached through `$ref`.

| Child ID | Site / shape tested | Observed result for inside / outside | Expected result |
|---|---|---|---|
| R1-F2.1 | Inline simplified root-union arm | Pass / pass; second compilation hits cache | Pass / fail; retain context |
| R1-F2.2 | Referenced simplified file used as a root-union arm | Pass / pass; second compilation hits cache | Pass / fail |
| R1-F2.3 | Referenced schema file containing the root union | Pass / pass; second compilation hits cache | Pass / fail |
| R1-F2.4 | Mixed simplified/raw root union | Pass / pass; raw arm makes the whole validator context-sensitive, but cannot constrain the other arm | Pass / fail |
| R1-F2.5 | Direct file property control | Clean: pass / fail | Pass / fail |
| R1-F2.6 | Acyclic named file type control | Clean: pass / fail | Pass / fail |
| R1-F2.7 | Raw local `$ref` with the explicit match keyword | Clean: pass / fail | Pass / fail |

The sibling attachment sites are `resolve.rs`'s inline-arm and file-arm branches in both document unions and referenced-file unions, plus `union_to_root_schema` for re-lowering. They share the same attachment helper. Follow the emitted keyword into root-arm validity and validator cache classification: the former accepts the wrong arm and the latter omits context when no other contextual rule remains. The raw control shows that reference-aware validation and context keying work when lowering preserves the keyword.

**Smallest repair:** carry match semantics through recursive definitions during root-union lowering, without modifying a shared definition in a way that changes other uses. Exercise the actual recursive case in criterion 15's cache tests; retain existing single-schema suggestion-only behavior and union transformation policies. Do not resolve this by documenting away the requested scope.

### R1-F3 (high): Caller file overrides lose their launch-directory meaning through references

**Authority — spec:** Design requires a reference to remain useful for “validation, transformations, and editor assistance”; Design 7 preserves composition's existing authorized transformations in the captured request context, and criterion 9 requires normal composition coverage. Phase 4's decision explicitly leaves this consumer unimplemented for recursive file unions. **Defect class:** reference-blind caller classification and materialization lose the origin of a caller-supplied file value.

In the `darkmatter` package, [select_file_mode](../../../darkmatter/lib/src/markdown/compose/schema_validation.rs:847) follows arrays and union fragments but never `$ref`. [resolve_caller_file_value](../../../darkmatter/lib/src/markdown/compose/schema_validation.rs:1198) also requires a directly visible format before materializing a file. These functions received root-schema parameters in this feature, but their reference following was omitted.

**Reproduction.** Copy the existing caller-file test's layout: `launch/spec.md` and `document/spec.md` contain different files; the request snapshot is captured in `launch`, and the composed prompt lives in `document`. Supply the caller override `spec = "spec.md"` through `ComposeOptions::with_set_overrides`, then call `Markdown::compose_with`. Use:

```yaml
$schema:
  node: "file(eager) | { children: node[] }"
  spec: node
```

Composition succeeds but retains `spec.md`, allowing it to mean the document's file. With `spec: file(eager)` directly, the same request correctly retains the absolute launch-file identity. This is a silent wrong-file selection when both files exist, rather than merely cosmetic output.

**Class sweep.** Ran the same public composition request with scalar/list and eager/lazy variants. Each control uses the identical directories and override value.

| Child ID | Site / shape tested | Observed result | Expected result |
|---|---|---|---|
| R1-F3.1 | Recursive reference with an eager file alternative | Relative `spec.md` survives | Absolute launch file |
| R1-F3.2 | Recursive reference with a lazy file alternative | Relative `spec.md` survives | Absolute launch candidate |
| R1-F3.3 | Array of recursive eager-file alternatives | `["spec.md"]` survives | Array of launch-file identities |
| R1-F3.4 | Array of recursive lazy-file alternatives | `["spec.md"]` survives | Array of launch candidates |
| R1-F3.5 | Direct eager/lazy scalar controls | Clean: absolute launch identity | Same |
| R1-F3.6 | Acyclic named eager-file control | Clean: absolute launch identity | Same |
| R1-F3.7 | Direct eager/lazy array controls | Clean: absolute launch identities | Same |

The classification result is read by `resolve_caller_file_overrides`, `classify_caller_overrides`, `caller_file_fragments_shared_by`, and the later caller-projection/recheck path in this same module. All call the shared reference-blind classifier; the resolver has the additional direct-format restriction. Public composition exercised initial projection and the later validation path and still returned the wrong value. The separate document-authored eager rewrite tests pass through recursive references; the defect is specific to caller provenance, so repairing that rewrite alone is insufficient.

**Smallest repair:** make classification and materialization follow the same root-scoped references together, retaining sibling assertions, existing union ambiguity rules, and the caller's captured origin. Add scalar/list eager/lazy regression coverage. This delivers the omitted scope without adding new syntax or a new file-resolution policy.

### R1-F4 (high): CLI timing reports omit assignment preparation and misreport cache hits

**Authority — spec:** Part A says stages are accumulated work “within one request,” compilation includes “every validator built by those stages,” and “If any validator was built, the request is a compile miss.” Criterion 12 requires the CLI to print that breakdown. **Defect class:** incomplete recording boundaries omit schema work and falsely classify a cold request as a cache hit.

In `darkmatter-cli`, [validate_one](../../../darkmatter/cli/src/commands/schema/validate.rs:243) calls `api.effective_for` to prepare assignments before opening its validation recording. That call resolves the schema and compiles its validator. The recording begins later at `validate_timed`, which sees the newly warmed cache. Another `effective_for` call after validation is also outside the recording, so its resolution work is absent from total time.

**Reproduction.** In a fresh CLI process, validate this file:

```yaml
---
$schema:
  review_counter: number(required)
review_counter: 1
---
```

Run `md schema validate -vv --no-baseline-schema review_counter=2 doc.md`. It succeeds and prints `compile cache: hit` and `compile: 0.00 ms`. The validator was necessarily built in this process for the previously unseen inline schema. The temporary CLI regression asserting `compile cache: miss` failed on that output.

**Class sweep.** Examined all timing boundaries around this command's schema preparation and compared them with the public library and DMLS timing tests.

| Child ID | Site / shape tested | Observed result | Expected result |
|---|---|---|---|
| R1-F4.1 | CLI pre-assignment `effective_for`, fresh inline schema | Reproduced: compilation omitted; reported hit | Include resolution/compilation; report miss |
| R1-F4.2 | CLI post-validation `effective_for` presence lookup | Code-confirmed: runs after the recording; repeated resolution omitted | Count the work, or reuse the already obtained schema-presence result |
| R1-F4.3 | CLI without assignments; discovery and early schema failure | Existing CLI tests pass; discovery and validation are recorded | Keep partial report and original exit/output behavior |
| R1-F4.4 | Library cold/full-hit/mixed-hit/no-schema/error recording | Existing public timing tests pass | Accumulate stages and miss state correctly |
| R1-F4.5 | DMLS request recording, cached bundles and mixed compilation | Existing protocol timing tests pass | Keep its tested compile-state reporting |

The error return from pre-assignment resolution is inside R1-F4.1's unrecorded boundary too: fixing only the successful cache label would still omit partial work on that return. The library recorder already supports accumulating separate recorded parts; it does not need replacement.

**Smallest repair:** include pre-assignment schema preparation in the document's accumulated timing, and count or eliminate the repeated presence-only resolution. Ensure an assignment-induced trigger change still re-resolves after assignment. Test the cold CLI assignment case without asserting elapsed values.

## Observations

None.

## Verification

After removing all temporary review probes, the existing focused suite passed **232/232 tests**:

```sh
cd darkmatter
just test recursive_schema schema_recursive schema_cache_validity schema_validation_timings schema_validate_timings validation_timing trigger_payload_load_errors recursive_call_check coercion_translator_conformance
```

`just check-tier-coverage darkmatter` reported **zero stranded tests**. The earlier recursion-only selection passed 131/131 tests. Passing existing tests do not cover the failing cases described above. Temporary public-API, CLI, and DMLS probes reproduced all four findings; they were removed so this review leaves no test or production-code changes. The final change contains the review, the review counter, and the review's decision record. Lint was not rerun for these Markdown-only edits; the implementation log records the implementation's lint gates.

| Requirement group | Appropriate verification | Evidence reviewed / limit |
|---|---|---|
| Acyclic byte identity, graph admission, namespace safety, finite size | L1 | Golden, recursive type, merge, and frozen-fixture tests pass; R1-F1 adds a missing namespace combination |
| Deep positioned validation, presence, phase behavior | L1 | Library and CLI recursive tests pass |
| Cleanup, conversion, rewriting, discriminant and synthetic validators | L1 | Walker tests and translator conformance pass; R1-F2/R1-F3 expose omitted file-rule consumers |
| Editor completion, hover, positioned schema errors, lazy lookup | L1 protocol session | Existing editor tests pass; additional two-trigger session reproduces R1-F1 |
| Stage logging, configuration precedence, cache behavior | L1 library/CLI/protocol | Existing tests pass; R1-F2/R1-F4 expose missing recursive match and assignment cases |
| Portable generator | L1 | Existing unrelated-CWD and path-literal tests pass |
| Performance measurement and frozen benchmarks | Recorded release measurements and code inspection | Log records resolution/I/O/compile separation and Part A before/after results; no timing threshold inferred |

These requirements do not depend on terminal input encoding, keyboard events, or terminal layout; L2/L3 tests are unnecessary. Cross-OS execution evidence is left to CI as requested.

The only new configuration field is the integer `schema.slow_validation_ms`. Its public `ConfigState` tests distinguish absence/default 200, explicit zero, valid editor override, whole-settings null restoring the file layer, explicit field null producing a configuration error, and invalid numeric/string values retaining the lower layer. No new document loader was introduced. The tests use the existing shared configuration deserializer; no new permissive filtering or absent/null collapse was found on this field.

This is the first implementation review. There are no previous findings to classify as repaired, partially repaired, or recurring.
