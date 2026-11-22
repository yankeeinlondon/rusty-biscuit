---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Completion disappears in continued flow-sequence operands"
  - "R1-F2 (high): Multiline quoted object definitions fail source projection"
  - "R1-F3 (high): Deleted sequence type source still prevents consumers from compiling"
  - "R1-F4 (high): Shipped Claudine triggers no longer satisfy their consumers"
  - "R1-F5 (medium): Envelope admission reserves $extends inside type tables"
human-review: true
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-22T18:09:03+00:00"
spec: 2026-11-22-object-composition/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-object-composition/spec.md`"
feature: 2026-11-22-object-composition/review-1.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/simplified-schema/features/2026-11-22-object-composition/log.md
implemented-by: claude/opus
human-review-items:
  - "R1-F3/R1-F4 repair restored claudine/schemas/sequence-types.yaml and removed the published claudine/schemas/lifecycle-events.yaml draft, reversing parts of commits 8f33de0d7 and 89ffd8678; confirm until host registration lands"
next: 2026-11-22-object-composition/review-2.md
---

# Object composition review

The feature is **not production ready**. Core composition, replacement, recursion, export, and the tested editor paths work, but five findings remain. None requires a human decision. This is the first review; there are no previous findings to reconcile.

Two independent agents read the implementation log and specification before this review compared code with the contract. The spec does not contain an additional authorization to depart from its semantics. Its implementation choices and omissions are assessed below.

## Findings

### R1-F1 (high): Completion disappears in continued flow-sequence operands

**Authority — spec:** “Completion should work while the operand is incomplete,” including “each flow or block sequence item.” A normal multiline flow list loses completion on its continuation line in both standalone YAML and Markdown frontmatter.

**Class:** operand discovery depends on the cursor line containing the directive, so it misses operands inside a flow sequence opened on an earlier line.

```yaml
kind: schema
$schema:
  $extends: [base,
    b] # request completion just after b
types:
  base: '{ id: string }'
```

It returns no completion; moving `b` beside `base` returns `base`. The same failure occurs with an empty item, a quoted item, and the reference half `base@`.

The changed Darkmatter LSP [composition operand discovery](../../../darkmatter/dmls/src/providers/frontmatter.rs) checks flow context, the current line's mapping value, or a block-list marker. A continuation line satisfies none of those paths. Its flow-item scanner can scan across lines once given the opening bracket, but discovery does not supply that bracket.

Sweep through the same provider, using its existing `with_document`/`offered` fixture and a temporary L1 probe:

| Instance | Site / shape | Observed | Expected |
| --- | --- | --- | --- |
| R1-F1.1 | Standalone `$extends: [base,` followed by `b` | No items | `base` |
| R1-F1.2 | Same continuation, empty item | No items | Eligible object names |
| R1-F1.3 | Same continuation, `base@` | No items | `this` |
| R1-F1.4 | Same continuation, single-quoted `b` | No items | `base` |
| R1-F1.5 | Markdown frontmatter, continued flow item | No items | `base` |
| R1-F1.6 | Scalar `$extends: b` | `base`; clean | `base` |
| R1-F1.7 | Block-list item `- b` | `base`; clean | `base` |
| R1-F1.8 | Same-line flow item `[base, b]` | `base`; clean | `base` |
| R1-F1.9 | Root-union mapping arm `$extends: b` | `base`; clean | Arm-local `base` |
| R1-F1.10 | Property-union mapping arm `$extends: b` | `base`; clean | `base` from `types` |
| R1-F1.11 | Inline spread `{ ...b }` | `base`; clean | `base` |

**Recommended repair:** recover the enclosing `$extends` flow sequence across continuation lines, then use the existing item and operand scanners. Add public provider/session assertions for the failed rows. This needs L1 editor-protocol verification, not a real editor or terminal. The temporary probe ran in 0.035 seconds and was removed after review.

### R1-F2 (high): Multiline quoted object definitions fail source projection

**Authority — defect:** the simplified-schema loader rejects a plausible authored object definition solely because its quoted YAML scalar wraps onto another line. This affects ordinary definitions and compositions. It is a pre-existing defect explicitly observed in the implementation log, rather than a new composition regression.

**Class:** the source locator fails to consume a complete multiline quoted scalar before projecting expression spans.

```yaml
kind: schema
$schema:
  p: '{ a: string,
    n: number }'
```

The simplified-schema public `SimplifiedSchema::try_from` returns `invalid SimplifiedSchema payload … could not project SimplifiedSchema expression spans through YAML source`. The one-line equivalent and the folded block-scalar equivalent both load and lower successfully. The failure occurs before validation, so the author cannot use the schema at all.

The simplified-schema [source locator and projection](../../lib/src/simplified/source.rs), including `locate_yaml_value_with` and `BlockLocator`, is the common boundary. A temporary example called the public factory and `to_json_schema` for each row:

| Instance | Site / shape | Observed | Expected |
| --- | --- | --- | --- |
| R1-F2.1 | Single-quoted multiline ordinary inline object | Source-projection error | Load and lower |
| R1-F2.2 | Double-quoted multiline ordinary inline object | Same error | Load and lower |
| R1-F2.3 | Single-quoted multiline `{ ...base, n: number }` | Same error | Load and lower |
| R1-F2.4 | Double-quoted multiline `{ ...base, n: number }` | Same error | Load and lower |
| R1-F2.5 | Ordinary object on one quoted line | Loads and lowers; clean | Load and lower |
| R1-F2.6 | Ordinary object in folded `>-` scalar | Loads and lowers; clean | Load and lower |
| R1-F2.7 | Single-quoted multiline `$extends` operand, `base@` followed by `this` | Source-projection error | Load and lower the local reference |
| R1-F2.8 | Spread object in folded `>-` scalar | Loads and lowers; clean | Load and lower |

Both ordinary and composition expressions use this source-aware loader; the downstream resolver and lowerer are not reached on the failing rows. The existing export tests also establish clean anchored, aliased, and folded composition operands. Do not treat substituting those presentations in tests as repairing quoted multiline input.

**Recommended repair:** let the existing YAML source locator consume the full quoted scalar and preserve its byte mapping. Add focused public-factory controls for both quote styles, with and without a spread. Avoid a second expression parser or changes to composition syntax.

### R1-F3 (high): Deleted sequence type source still prevents consumers from compiling

**Authority — defect:** Claudine's production schema bundle contains an `include_str!` naming the absent `claudine/schemas/sequence-types.yaml`. A normal `cargo check --locked -p claudine` fails at that declaration. This predates composition, but it is a concrete build defect encountered during this review, not missing cross-OS evidence.

**Class:** compile-time schema consumers still depend on a deleted source file.

The Claudine [embedded bundle](../../../claudine/lib/src/composition/schema/bundle.rs) includes it at line 29. This also prevents the root shipped-schema generator from building. The implementer's temporary restoration established that the missing source is sufficient to block otherwise compatible consumers; removing it again leaves the submitted tree broken.

The sweep includes every Rust `include_str!` of that deleted source. Each failing site has the same compile-time invariant: its literal resolves to the absent file, so compilation cannot succeed. The rows outside the composition changes are included as existing defects, as required by this review's scope.

| Instance | Site / shape | Observed | Expected |
| --- | --- | --- | --- |
| R1-F3.1 | `claudine/lib/src/composition/schema/bundle.rs:29` | `cargo check -p claudine` fails: file not found | Build from checked-in sources |
| R1-F3.2 | `darkmatter/cli/tests/l1/claudine_schema_bundle.rs`: lines 141, 187, 222, 260, 308, 348, 379, 410, 471, 575, 636 | `cargo check --locked -p darkmatter-cli --tests` reports all eleven missing includes | Compile declared tests |
| R1-F3.3 | `claudine/cli/src/completion/schema_completion/tests.rs:1132` | Same absent-file invariant | Compile declared tests |
| R1-F3.4 | `claudine/lib/tests/l1/host_publication.rs:308` | Same absent-file invariant | Compile declared tests |
| R1-F3.5 | `claudine/lib/tests/l1/claudine_schemas_embedded.rs:171` | Same absent-file invariant | Compile declared tests |
| R1-F3.6 | `claudine/lib/tests/l1/claudine_schemas.rs:511` | Same absent-file invariant | Compile declared tests |
| R1-F3.7 | Darkmatter benchmark fixtures and DMLS `frontmatter_response_contracts` | They include the existing frozen benchmark copy; clean with respect to this missing path | Continue using their declared fixture |
| R1-F3.8 | Simplified-schema corpus and DMLS discovery fixture lists changed by this feature | Deleted include removed; these compile | Compile; semantic failures are R1-F4 |

**Recommended repair:** restore a checked-in compatibility type source with the declarations these current consumers need. Do not leave temporary file restoration as the verification strategy or silently drop the affected tests. This is a bounded repair that does not require implementing the next host-registration feature. Re-run the blocked consumer and generator checks against the final tree.

### R1-F4 (high): Shipped Claudine triggers no longer satisfy their consumers

**Authority — defect:** current shipped files and existing schema consumers disagree about the trigger envelope and the location of local types. Real repository fixtures therefore fail validation or receive no sequence diagnostics. These are recorded pre-existing failures, reproduced here; their age does not make the results correct.

**Class:** shipped schema documents have moved ahead of the contract their loaders and fixtures implement.

In simplified-schema's [shipped bundle test](../../lib/tests/l1/shipped_corpus.rs), `sequence.yaml` now declares `steps` in a sibling `types` table, but the test prepares only its `$schema` mapping. It fails with `sequence: unknown named type steps`. In Darkmatter, shipped trigger admission still allows only `kind`, `match`, and `$schema`; the current `sequence.yaml` has `types`, and `lifecycle-events.yaml` has `description` and `types` but lacks the required `match` and exported payload. DMLS then misses the expected diagnostics for removed sequence keys.

Sweep of the affected consumers and their decisions:

| Instance | Site / shape | Observed | Expected |
| --- | --- | --- | --- |
| R1-F4.1 | Simplified-schema `shipped_corpus::claudine_bundle_and_its_triggers_resolve_from_supplied_sources`, default and minimal builds | Both fail on detached `steps` namespace | Resolve the shipped trigger with its declarations |
| R1-F4.2 | Darkmatter `shipped_schema_artifacts_parse_passively`, current lifecycle trigger | Rejects unsupported `description, types` | Every shipped trigger is admitted by its current loader |
| R1-F4.3 | Darkmatter `loadable_shipped_schemas_resolve_and_lower`, same file | No `$schema` property | A shipped schema classified as loadable resolves |
| R1-F4.4 | Darkmatter `every_kinded_schema_artifact_passes_its_definition_through_the_registry` | Lifecycle file: missing `match` and extra keys; sequence file: extra `types` | Shipped documents satisfy the registered definition |
| R1-F4.5 | DMLS `claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths` | No diagnostic at `/sequence/0/group` for `execution` | Report the removed key at its supplied value |
| R1-F4.6 | Other nine Darkmatter shipped-corpus tests | Pass; clean in the focused 12-test run | Preserve their existing contracts |
| R1-F4.7 | Self-contained composition trigger fixture, runtime and prepared paths | Pass in the simplified-schema run; clean | Resolve composition without depending on draft registration syntax |

**Recommended repair:** keep shipped schemas and their fixtures compatible with the existing trigger contract until host registration supports the new format. Restore the compatible shipped sequence/type split and keep future registration documents in their existing draft area; correct the incomplete lifecycle document's kind or placement to match its purpose. If the corpus test continues to exercise a separate local table, pass that namespace through the existing appropriate API rather than extracting an unbound payload. Do not weaken the envelope validator or implement host-registration behavior as a workaround. Re-run the five failed tests and the current trigger/sequence controls. No human decision is needed to preserve the current supported contract.

### R1-F5 (medium): Envelope admission reserves `$extends` inside type tables

**Authority — spec:** “The `types` table itself remains a table of named declarations.” The implementation instead reserves `$extends` there during envelope admission, although its later table parser correctly treats the key as an ordinary name.

**Class:** one logical table is validated as an object before it is parsed as a declaration table.

```yaml
kind: schema
types:
  $extends: string
```

The public simplified-schema factory rejects this with the reserved-composition-directive diagnostic. A declaration of the same name whose definition happens to be a bare alias succeeds. Thus acceptance depends on the spelling of the declaration's value, rather than the table contract. This is an unusual name, so its priority is medium.

The simplified-schema [envelope admission](../../lib/src/envelope.rs) runs the shipped envelope definitions, whose `types: schema(mapping; …)` field invokes object parsing. The later [table parser](../../lib/src/simplified/mod.rs) and [standalone source parser](../../lib/src/simplified/source.rs) already distinguish the two modes.

Public-factory sweep:

| Instance | Site / shape | Observed | Expected |
| --- | --- | --- | --- |
| R1-F5.1 | Tagged schema, `types.$extends: string` | Reserved-directive error | Ordinary primitive declaration |
| R1-F5.2 | Tagged schema, mapping definition under `types.$extends` | Invalid directive carrier | Ordinary object declaration |
| R1-F5.3 | Tagged schema, `[string, number]` under `types.$extends` | Invalid composition operand | Ordinary union declaration |
| R1-F5.4 | Tagged schema with both `$schema` and `types`, primitive definition | Reserved-directive error | Ordinary declaration |
| R1-F5.5 | Informal `$schema`/`types` envelope, primitive definition | Same error | Ordinary declaration |
| R1-F5.6 | `kind: yaml-schema` with valid required literal `kind` declaration, primitive type definition | Same error | Ordinary declaration |
| R1-F5.7 | Tagged table, `$extends: other` with `other: string` | Loads and lowers; clean | Ordinary alias declaration |
| R1-F5.8 | Actual object `$extends` in root, nested object, or union arm | Composition parsed/validated; clean in existing public tests | Remain a directive |

**Recommended repair:** make the envelope's `types` admission validate a declaration table, consistently with the standalone parser. Preserve object-mode reservation in actual schema objects. Add public-factory cases across the shared envelope validators and regenerate affected shipped artifacts. The plan's instruction to avoid envelope changes does not override this explicit spec contract; a focused correction is an ordinary repair, not an author decision.

## Input robustness and verification level

Invalid `$extends` carriers/items must produce an error at the offending value or item. Absence means no composition. Empty object targets are valid; an empty operand list is invalid. The field validator is shared by root objects, nested objects, named-object declarations, and mapping union arms.

The public parsing tests cover the positive control, absent directive, explicit/implicit null, wrong whole-field types, mixed/all-invalid item types, empty string, empty list, invalid postfixes, and directive placement. Each supported mapping route has a valid case and an invalid number case. Loader tests cover duplicate keys, trailing invalid input, a second YAML document, and trigger-envelope loading. The source-scalar defect and table-admission discrepancy are inventoried above rather than hidden behind that matrix. A scan of the changed parser/resolver sites found no new silent `filter_map(as_str)`, defaulted invalid collection, or swallowed load-bearing parse error; optional source-location fallbacks do not turn invalid composition into success.

All feature requirements concern in-process language behavior or editor protocol responses. L1 is appropriate: none promises terminal rendering, OS key events, or a real browser. Both new simplified-schema test modules and the DMLS session test module are registered in their declared `l1` targets. Their names carry no higher-tier markers.

Requirements and strongest inspected evidence:

| Requirement | Evidence / level | Result |
| --- | --- | --- |
| Spread order, duplicates, operand syntax, `$extends` object positions | Public parse/resolve tests, L1 | Pass, subject to R1-F2 and R1-F5 |
| Whole-property replacement, patterns, receiver constraints, origins, skipped members | Public lowering/validation and file fixtures, L1 | Pass |
| Local/imported/embedded lookup, aliases, depth, dependency tracking, recursive composition | Public resolver and validation tests, L1 | Pass |
| Handwritten equivalence, strict/soft validation, subtype, authored export | Public API round trips and comparisons, L1 | Pass |
| Completion, hover, definition, operand diagnostics, unsaved import refresh | Provider and LSP session tests, L1 | Existing tests pass; continuation gap is R1-F1 |
| Copied member lowering error origin | Public library test, L1 | Pass; direct DMLS routing assertion omitted |

Review runs:

- `just test --no-fail-fast` in simplified-schema: **2,471 passed, one failed**, 2,472 total; canonical recorder reports none at or above five seconds. Run ID `98f2842b-3473-4165-94e3-0356100389cb`.
- `just test-minimal`: **2,091 passed, one failed**, 2,092 total; the same shipped-bundle failure.
- `cargo nextest run -p darkmatter-lsp --lib --test l1 -E 'test(composition)'`: **48 passed**, including all seven composition session tests. No slow tests.
- Focused DMLS discovery/composition run: the removed-sequence-key test fails; a separate composition-only run completed all composition tests after the first run stopped early.
- `cargo nextest run -p darkmatter --test l1 -E 'test(shipped_schema_corpus)' --no-fail-fast`: **nine passed, three failed**; all individual times below one second.
- `cargo check --locked -p claudine` and `cargo check --locked -p darkmatter-cli --tests`: fail on the deleted source in R1-F3.
- `just check-tier-coverage simplified-schema`: passes, with no stranded tests.
- Temporary public-factory and provider probes reproduced the finding tables; their source changes were removed. Existing implementation lint, generator comparison, and caller-skill publication evidence was inspected; it does not substitute for the failed gates.

Cross-OS proof is left to CI and is not a finding. No terminal or browser was opened. No commit or formatter was run.

## Implementation decisions (informational)

None of these needs the author's time. Keep the recommended internal choices; repair the identified behavior gaps within the existing contract.

1. **Type-table admission:** leaving admission unchanged preserved an inconsistency with the spec. Repair it under R1-F5 rather than ask the author to reserve another name.
2. **Boxed composition field:** keep `Option<Box<ObjectComposition>>`; it avoids increasing several enum variants and preserves serialization.
3. **Origin storage:** keep the hidden, nonserialized `composed_sites` field instead of threading a separate map through every lowering API. Its equality behavior deliberately leaves semantic shape comparison unchanged.
4. **Content-only restriction:** keep the early external-reference check for composition. Even an overwritten operand must resolve, so rejecting file-backed operands before I/O follows the spec.
5. **Assembly caching:** keep cache keys containing both definition identity and depth, preserving the existing depth limit.
6. **Error details:** keep the authored operand spelling and final declaration location; classify tuples as lists, opaque object references as opaque, and other primitives as primitive. These choices explain the failure without changing allowed targets.
7. **Property-union export:** keep mapping output for `$extends` arms and the previous expression form for other arms. The model does not retain every YAML presentation choice, and the export tests verify equivalent composition.
8. **Selecting an implicit local type:** drop the receiving root's directive from the copied type table; it is not a named declaration. The selected root can extend the selected inline composition through `Name@this`, with unsupported names rejected rather than exported incorrectly.
9. **Layered export:** retain explicit refusal when composition cannot safely be represented in one authored document. This preserves the existing refusal contract for exports that would lose meaning.
10. **Cursor representation:** keep separate name/reference state, complete operand replacement ranges, and spread addresses. These are useful internal/editor API choices with direct tests.
11. **Darkmatter lookup enum:** keep the targeted large-variant lint allowance for the temporary lookup result. Boxing it would introduce allocations without evidence of a useful benefit.
12. **Unsaved Markdown imports:** keep buffer-aware resolution, read/probe tracking, and dependency hashes. This also fixes ordinary named-import refresh and is necessary for the spec's editor behavior.
13. **Copied-member diagnostics:** keep the named-type diagnostic code and declaration location. The absence of a direct DMLS routing test is recorded; the library's simple function-member fixture already proves the underlying error. The false explanation in the implementation log was corrected in place with a dated verification note.
14. **Editor catalog limits:** keep the bounded 32-file traversal of imported aliases and top-level compositions, resolve paths from the importing directory with available schema roots, and retain unknown candidates. This is a reasonable passive completion boundary; nested targets may remain unknown rather than cause extra per-request reads.
15. **Operand parser visibility:** keep the public passive `parse_extends_operand` entry point used by tolerant editor parsing. A new wrapper would provide little additional value.
16. **Completion routing and malformed buffers:** keep routing before meta-schema classification and the entry-by-entry recovery strategy. A malformed one-line flow document may retain names whose definitions cannot yet be recovered; that is a bounded best-effort fallback. Continued valid flow lists must still work and are R1-F1.
17. **Deleted fixture references:** removing stale includes in the two feature-touched test lists was sensible but did not finish the source migration. Remaining compile and semantic defects are R1-F3 and R1-F4.
18. **Verification substitutions:** direct lower-generator comparisons and package-specific checks were useful evidence when area gates could not compile. Temporary source restoration was diagnostic evidence only. R1-F3/R1-F4 require a working final tree; no one needs to reaffirm this choice.
19. **Loader locations:** retaining the existing loader's error text when it supplies no structured position, including rejection of a second YAML document, is acceptable here. No stronger location guarantee is invented.
20. **Deferred work:** the logged multiline quoted-scalar defect is R1-F2; type-table admission is R1-F5; the remaining broken shipped-schema work is R1-F3/R1-F4. No repair is deferred merely because the implementation called it pre-existing. Real host-registration restrictions and its missing draft `common-attributes` declaration remain outside this feature, as the spec explicitly decides; composition uses a self-contained acceptance fixture. Documentation and skill publication deferred between phases were completed. Cross-OS evidence remains CI's job.
