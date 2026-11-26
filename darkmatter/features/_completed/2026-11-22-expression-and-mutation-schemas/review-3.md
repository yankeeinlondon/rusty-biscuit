---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-3.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-2.md"
next: "2026-11-22-expression-and-mutation-schemas/review-4.md"
created: "2026-11-25T01:25:28+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R3-F1 (high): YAML anchors and aliases break overload validation in sequences"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
log: '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md'
implemented-by: codex/gpt-6.2-sol
---

# Expression and mutation schemas review 3

The feature needs one repair before it is production ready. All six findings from review 2 passed their required checks. The new finding affects valid YAML overload sequences and the locations shown when those sequences contain a conflict. The repair can proceed within the existing spec.

## Previous Finding Verification

The child IDs come from the [second repair ledger](log.md#implementation-of-review-findings-2). I checked the implementation and assertions, ran the relevant Level 1 tests, and read the raw Windows runs for the timing findings. Review 2 verified all four review 1 findings for their listed instances. The new sequence finding extends that earlier coverage. Neither earlier review records a human ruling.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1 Host registrations accept names that Markdown cannot call | R2-F1.1–.11 | **verified fixed** | For .1–.3 and .7–.9, the shared catalog reader, Rust host builder, CLI, DMLS and digest-verified installation reject unusable spellings at the authored key or JSON alias. The Rust check runs before the configuration callback. For .4–.6, the passing controls cover dashed, dotted, underscored and case-varied calls, collisions, mutation refusal, private helpers, alias field shapes, catalog field shapes and quoted keys. For .10–.11, compose reports an unknown function from the refused profile, and DMLS completion omits both its canonical name and its otherwise valid alias. The selected-host controls still find valid names. [Host builder tests](../../lib/tests/l1/host_functions.rs#L92), [Installed and verified admission](../../lib/tests/l1/host_registration_companions.rs#L224), [CLI catalog tests](../../cli/tests/l1/schema_validate_yaml.rs), [Compose tests](../../cli/tests/l1/compose_host_scope.rs), [Editor catalog tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs), [Editor completion](../../dmls/tests/l1/host_catalog_reload.rs), [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs) | None |
| R2-F2 Type-reference errors can point to unrelated text | R2-F2.1–.12 | **verified fixed** | For .1–.4 and .9–.10, the shared tests check ranges for parameter and return signatures, block and flow sequences, literal and folded pipe unions, object members and patterns, extension and spread operands, tuples and grouped arrays. Replacing the reference with a declared helper passes. For .5–.8, the independent conflict controls pass, CLI and DMLS highlight the actual arm in both kinds, and registration locates the failing signature with a related key only for a callable used as a type. For .11–.12, ordinary registered schemas still locate the failing declaration, and the DMLS external-reference test confirms no import checks, parses or dependencies. The Claudine doctor control preserves both locations. [Structural range assertions](../../lib/src/markdown/compose/expression/catalog/tests.rs#L1091), [CLI catalog tests](../../cli/tests/l1/schema_validate_yaml.rs), [Editor diagnostics](../../dmls/tests/l1/yaml_document_catalog_editor.rs#L110), [Installed reference tests](../../lib/tests/l1/host_registration_companions.rs#L276), [Ordinary registered schemas](../../lib/tests/l1/host_registration_locations.rs), [Doctor test](../../../claudine/lib/src/doctor/registration_tests.rs#L211) | None of these instances. R3-F1 includes a newly found source-projection sibling reached by a YAML alias in a sequence. |
| R2-F3 The no-terminal approval test exceeds the Windows L1 budget | R2-F3.1–.2 | **verified fixed** | The code removes baseline validation from the single compose invocation and retains the refusal, absence of a prompt, and unchanged configuration assertions. The eight sibling controls still cover their separate approval and persistence paths. The original phase profile located the expensive work in preflight. The final Windows package run records **2.878 s** for the same CLI witness. [Approval tests](../../cli/tests/l1/approval_prompt.rs#L308), [Profile and final records](log.md#implementation-of-review-findings-2) | None |
| R2-F4 The Claudine trigger discovery test exceeds the Windows L1 budget | R2-F4.1–.4 | **verified fixed** | One validation invocation now proves both valid verdicts, one installed inline-composition layer and zero installed composition layers. The header matches the format 3 fixture. The separate sequence-document test still checks the trigger trace, and the seven verdict and location controls still run. The final Windows package run records **2.412 s**. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs#L162), [Profile and final records](log.md#implementation-of-review-findings-2) | None |
| R2-F5 The recursive lifecycle corpus test exceeds the Windows L1 budget | R2-F5.1 | **verified fixed** | The single validation batch still checks all 19 documents, the agreed pointers and positions, deferred operands, and the presence and absence of the two installed trigger layers on the depth-six control. The final Windows package run records **2.030 s**. [Corpus test](../../cli/tests/l1/claudine_schema_bundle.rs#L523), [Profile and final records](log.md#implementation-of-review-findings-2) | None |
| R2-F6 The depth-four lifecycle test exceeds the Windows L1 budget | R2-F6.1 | **verified fixed** | The retained validation batch checks both installed trigger layers, four valid controls and three invalid innermost action keys at 16:11, 24:11 and 22:19. The final Windows package run records **2.467 s**. [Depth-four test](../../cli/tests/l1/claudine_schema_bundle.rs#L629), [Profile and final records](log.md#implementation-of-review-findings-2) | None |

The repair ledger also recorded three verification issues found during implementation. Both the shared review-cycle and the now-populated specification schema passed direct validation and the shipped-schema corpus. The final Windows package run confirms the two replacement style-parity tests at 2.730 s and 2.692 s. The run recorded 2,691 passes without an over-budget case. Its working-tree snapshot is timing evidence for the repaired tests. It did not produce a reusable CI receipt. [Root schemas](../../../schemas/), [Corpus](../../lib/tests/l1/shipped_schema_corpus.rs), [Style-parity tests](../../cli/tests/l1/render_alias_parity.rs#L500), [Verification record](log.md#implementation-of-review-findings-2)

## Unblocked Findings

### R3-F1 (high) YAML anchors and aliases break overload validation in sequences

**Authority (spec).** The catalog format permits a YAML mapping and “a direct `function(...)` declaration or a union of signatures under one name.” It also requires diagnostics to “point to the declaration or field that failed and include the other location when two declarations conflict.” An author can reuse the number signature of one callable in another callable's overloads by replacing the repeated signature with an ordinary YAML alias. The copied spelling is valid. The equivalent alias makes CLI validation and the editor reject the whole catalog. Adding an unused anchor to the first arm of a duplicate-overload fixture also removes the diagnostic's earlier location. [Catalog format and validation](spec.md#catalog-format-and-validation), [Acceptance checks](spec.md#acceptance-checks)

The defect class is losing the authored identity of a sequence item while mapping decoded YAML to declarations. An anchor, a standard YAML string tag or an alias that preserves the decoded signature must preserve the verdict. Switching the number signature to the string signature must still trigger a duplicate-overload refusal with two distinct locations. The probes started from the repository's project catalog. I exercised both kinds through `md schema validate --output json` and the headless DMLS protocol. I also installed expression variants next to matching facts and manifest digests, then ran `md compose` and checked a subset through `claudine doctor --json`. [Project fixture](../../lib/tests/fixtures/host_functions/project.yaml)

For example, `secondary` declares a number overload that `project_label` reuses.

```yaml
kind: expression-schema
categories: [Project]
types:
    secondary: &number 'function(parameters(name: number); returns(string); category(Project))'
    project_label:
        - *number
        - 'function(parameters(name: string); returns(string); category(Project))'
```

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Simplified Schema's block-sequence value reader, used by Darkmatter catalog admission | Keep two distinct signatures. Add an anchor to the earlier quoted or block scalar, or use an alias to an earlier callable's number signature as one arm. An explicitly tagged quoted string exercises the same branch. | CLI rejects the valid edits with a source-span projection error. The DMLS alias probe marks document start. Installed admission refuses the alias variant without a position inside the catalog. | Admit the same contract as the undecorated, fully written control. |
| R3-F1.2 | Simplified Schema's flow-sequence value reader | Write the distinct signatures in `[...]`. Add an anchor to the quoted arm or replace the earlier signature with an alias. | CLI accepts the fully written sequence and rejects both edited forms with the projection error. In a duplicate fixture with `[&primary 'function(...)', *primary]`, CLI, DMLS and installed admission locate the later alias but lose the first mark. | Accept valid arms and preserve both locations for conflicts. |
| R3-F1.3 | Direct overload invariant in the grammar, then its standalone catalog and registration projections | Compare two written string signatures, the same signatures with an unused anchor on arm 1, and a duplicate made by using an alias as either arm. | The written control marks both arms. All three edited block-sequence shapes lose the earlier mark through CLI, DMLS and installed admission. The CLI reports a generic type-definition problem and the editor uses `dm.schema.invalid_type_definition`. The installed host is refused and supplies no callable. | Locate the later arm and the earlier conflicting arm, using the authored alias token when appropriate. |
| R3-F1.4 | Overload checks that run after the authored types are mapped | Use `Label@this` to create an equivalent signature, give the two signatures different declared categories, or put an undeclared helper reference in the aliased signature. | The written overload and category conflicts identify both arms. Replacing a written arm by an alias in the resolved duplicate or category conflict produces a document-start projection error in CLI and DMLS and a file-level refusal in an installed expression catalog. The CLI reference variant also stops at projection before reaching the missing-helper check. | Keep the resolved duplicate and category diagnostics with both locations, and keep the missing-helper error on its declaration. |
| R3-F1.5 | Private helper sequences and the ordinary schema route using the same source reader | Change `Result: [string, number]` to `Result: [*label, number]` after `Label: &label string`. In an ordinary tagged schema, also try `Result: [&arm string, number]`. | Both undecorated controls pass. The edited catalog helper and ordinary schema fail in the CLI with the projection error. | Accept the helper and preserve the ordinary schema's export. |
| R3-F1.6 | Publisher verification and Claudine's doctor using the same installed catalog judgment | With matching digests, compare the working written control, a written duplicate, the duplicate with an unused anchor, and the valid alias. | Doctor reports that Darkmatter loads the written control and shows both positions for the written duplicate. With the anchor it shows the primary only. With the valid alias it reports a file-level refusal. The publisher's inventory uses the same verified catalog judgment before accepting a bundle. | Carry the shared verdict and all its locations through verified inspection. A valid bundle should load. |
| R3-F1.7 | Neighboring carriers and independent checks | Alias a whole scalar, pipe union or sequence as a mapping value. Keep an alias in an unrelated callable while testing a written duplicate. Test a later-arm alias in a missing-narrowing conflict. Keep the case-folded canonical-name control. | Clean for these inputs. Whole-value aliases and an anchored helper mapping value validate. The unrelated alias leaves both duplicate-overload marks intact. An earlier unanchored arm still supplies a related mark in the narrowing case, and the later alias is the primary mark. Case-folded names retain both key spans. | Keep the existing outcomes and positions. |

In Simplified Schema, the block-sequence locator sends inline items straight to the scalar reader. Mapping values take a path that handles YAML node properties. The flow-sequence reader likewise dispatches its items directly to the inline reader. The conflict mapper drops a related mark when it cannot project a decoded span into the authored scalar, even when the item token itself is known. Reuse the existing node-property and alias handling for sequence items. Where an alias has no local signature text, its token is enough to mark that overload. Keep the repair in the shared source reader so the consumers continue to agree. [Block and flow locators](../../../simplified-schema/lib/src/simplified/source.rs#L1412), [Definition failure projection](../../../simplified-schema/lib/src/simplified/source.rs#L779), [Catalog error conversion](../../../simplified-schema/lib/src/simplified/standalone.rs#L650), [Darkmatter catalog reader](../../lib/src/markdown/compose/expression/catalog/mod.rs#L471), [Claudine verified inspection](../../../claudine/lib/src/composition/registration/inspect.rs#L526)

## Recurrence

R3-F1 is a missed sibling of **R1-F1, Inline YAML comments make valid catalogs fail**, and **R1-F2, Catalog diagnostics lose declaration locations**. Review 1 repaired comment handling and verified conflict positions for ordinary written sequence arms and pipe unions. The earlier sweep did not run the valid and conflicting overloads with node properties or aliases on individual sequence items. The new probes cover both sequence forms and follow the result through the checks that need the parsed declarations. The original comment and conflict controls still pass. The issue also blocks the reference locator from reaching an aliased signature, while the repaired reference-location cases in R2-F2 still pass. [Review 1](review-1.md#findings), [Review 2](review-2.md#r2-f2-high-type-reference-errors-can-point-to-unrelated-text)

## Input Robustness Matrix

The default remains a located refusal for invalid input before evaluation or publication. An omitted or empty description is valid. Categories and types must contain declarations. An empty manifest `files` list is valid with its lifecycle artifact present. Facts rules that are nullable require explicit null when unset, and aliases, facts, and source-exclusion lists may be empty.

| Validator or loader | Level 1 evidence |
| --- | --- |
| YAML catalog envelope and shared YAML loader | The public envelope cases cover absent, explicit null, wrong whole-field type, mixed and all-invalid entries, and empty collections. CLI and editor controls cover both kinds, malformed and duplicate keys, comments, LF, CRLF, BOM, trailing content and second documents. Variant-specific syntax worked for mapping aliases and direct sequences. The anchor, tag and alias variants inside sequences are the failing document presentations in R3-F1. The mutation-member refusal still happens at its kind. |
| Manifest admission and verified members | Public snapshot tests cover the seven closed fields, v1/v2/future version refusals, malformed values, empty members, the exact digest inventory, path admission, missing companions, collisions and retries. None of the sequence probes failed at the manifest or digest gate. |
| Strict JSON facts reader and binding rules | Public facts tests exercise absent, null, wrong-field and element shapes, empty collections, incompatible rules, stale inner hashes, duplicate keys and trailing content. JSON5 syntax is refused. The installed alias tests confirm a valid string passes and an unusable spelling or wrong-type element is refused at its own index. |
| Metadata coercions and discarded parse errors | Catalog metadata is checked against the authored YAML value. The manifest deserializer requires every field and denies unknown fields. The facts reader checks required, nullable and collection fields explicitly. I traced the permissive reads used for preliminary discovery and for displaying example text to the strict admission checks that determine the verdict. |

## Verification

| User-facing requirement | Appropriate level and evidence |
| --- | --- |
| Built-in migration and behavior | Level 1. Signature and artifact checks, lifecycle import and operand parity, event binding and execution boundary tests. |
| Standalone validation and catalog privacy | Level 1. Shared parser, CLI and headless DMLS checks cover admitted and refused catalogs, duplicate keys and overloads, and schema-target and named-type import refusals. R3-F1 identifies the missing sequence cases. |
| Selected-host facts and execution | Level 1. Installed-host, compose and DMLS tests cover selection, no exposure in another host, missing and restored companions, binding before conversion and refusal when the host runtime is absent. The runtime fixture checks that an action operand dispatches the host function without registering an action key. |
| Publication, listing, doctor and upgrade | Level 1. Inventory and verified doctor tests cover the same published members, missing and modified companions, version refusal, repair and an in-place format 2 upgrade. |
| Mutation refusal and existing action rules | Level 1. Member admission refuses the catalog at `kind`, and generation, event-binding and lifecycle runtime tests check existing behaviors. |

The observable surfaces are file contents, command results, library calls and editor protocol responses. They require no terminal-emulator or OS keyboard evidence. Fresh Linux runs passed 677 focused Darkmatter catalog and registration tests and 366 diagnostics and lifecycle tests, 25 Simplified Schema catalog tests and 2,196 minimal-feature tests, and 118 Claudine registration and doctor tests. Every test stayed below five seconds. The shipped-artifact checks passed in all three areas and the tier audit found zero stranded tests.
