---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-4.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-3.md"
next: "2026-11-22-expression-and-mutation-schemas/review-5.md"
created: "2026-11-25T02:25:26+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R4-F1 (high): The repository kind schema is missing from the worktree"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "codex/gpt-6.2-sol"
---

# Expression and mutation schemas review 4

The feature needs one repair before it is production ready. The YAML overload repairs from review 3 pass all 19 required checks. The shared schema used to validate document kinds is missing, so the kind guide fails to compose and documents using that schema cannot validate. The committed version already contains both callable kinds and supplies a verified repair. No earlier review recorded a human ruling.

## Previous Finding Verification

The required instances come from the [repair ledger](log.md#implementation-of-review-findings-3). For review 3, I checked the test assertions and raw run output against the implementation, reran focused tests, and probed the repository-built CLI. For the closed review 1 and 2 items, I checked for changes to their witnessed paths in this repair and retained the earlier evidence where those paths and assertions still hold.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R3-F1 YAML anchors and aliases break overload validation in sequences | R3-F1.1–.19 | **verified fixed** | All witnesses below preserve their expected results and positions. The shared source reader now recognizes aliases as authored tokens, handles node properties on sequence arms, and uses decoded signature spans for written arms. [Simplified Schema source reader](../../../simplified-schema/lib/src/simplified/source.rs#L1433) | None |
| R2-F1 Host registrations accept names that Markdown cannot call | R2-F1.1–.11 | **verified fixed** | The shared lexer check remains in the catalog reader, Rust builder, and facts reader. The repaired YAML locator also passes the quoted-key controls. Recorded Level 1 witnesses cover CLI, editor, installed admission, and the absence of names from refused profiles. [Review 3 verification](review-3.md#previous-finding-verification), [Installed admission](../../lib/tests/l1/host_registration_companions.rs#L224) | None |
| R2-F2 Type-reference errors can point to unrelated text | R2-F2.1–.12 | **verified fixed** | The written references still locate the owning declaration or the narrower source-map range. The public, CLI, editor, and installed tests retain the cases with decoy text, missing references, external references, and related callable keys. The new alias tests add a control for the range of an alias use. [Review 3 verification](review-3.md#previous-finding-verification), [CLI cases](../../cli/tests/l1/schema_validate_yaml.rs#L800) | None |
| R2-F3 The no-terminal approval test exceeds the Windows L1 budget | R2-F3.1–.2 | **verified fixed** | The same single compose invocation and refusal and persistence assertions remain in place. Final Windows timing was 2.878 s. [Review 3 verification](review-3.md#previous-finding-verification) | None |
| R2-F4 The Claudine trigger discovery test exceeds the Windows L1 budget | R2-F4.1–.4 | **verified fixed** | The consolidated validation batch and separate trigger-trace witness remain in place. Final Windows timing was 2.412 s. [Review 3 verification](review-3.md#previous-finding-verification) | None |
| R2-F5 The recursive lifecycle corpus test exceeds the Windows L1 budget | R2-F5.1 | **verified fixed** | The 19-document batch retains the depth-six trigger layers, pointers, and deferred-operand checks. Final Windows timing was 2.030 s. [Review 3 verification](review-3.md#previous-finding-verification) | None |
| R2-F6 The depth-four lifecycle test exceeds the Windows L1 budget | R2-F6.1 | **verified fixed** | The batch still checks both trigger layers, four admitted documents, and the three innermost invalid actions and their positions. Final Windows timing was 2.467 s. [Review 3 verification](review-3.md#previous-finding-verification) | None |
| R1-F1 Inline YAML comments make valid catalogs fail | R1-F1.1–.8 | **verified fixed** | Commented flow and block declarations, commented markers on their own lines, and literal hashes keep their results. The standalone and whole-document loaders accept the BOM and CRLF controls and preserve the second-document refusal. [Review 2 verification](review-2.md#previous-finding-verification), [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L427) | None |
| R1-F2 Catalog diagnostics lose declaration locations | R1-F2.1–.15 | **verified fixed** | The recorded runs cover direct and resolved overloads, category and name collisions, both claim orders in facts, the built-in source fallback and refresh, and CLI, editor, installed, and doctor ranges. The new alias-location controls also pass. [Review 2 verification](review-2.md#previous-finding-verification) | None |
| R1-F3 The shared review and fix schema cannot validate documents | R1-F3.1–.5 | **verified fixed** | Both review root arms and their nested requirements are still exercised. The former placeholder, now a populated specification schema, validates, as do the review-cycle and suggestion-review schemas. [Corpus](../../lib/tests/l1/shipped_schema_corpus.rs#L903), [Review 3 verification](review-3.md#previous-finding-verification) | None of the original instances. R4-F1 identifies a different root schema that the corpus can silently omit. |
| R1-F4 Registration and catalog references still describe the old formats | R1-F4.1–.8 | **verified fixed** | Maintainer guidance and the publication, inventory, and setup witnesses still describe format 3. The corpus keeps the catalogs private, and legacy format 2 references describe cleanup or upgrade. [Review 2 verification](review-2.md#previous-finding-verification), [Current registration guide](../../docs/topics/host-registration.md) | None |
| R3-V1 The shipped-schema corpus retains its required suggestion-review schema | Explicit file-presence assertion and corpus resolution | **verified fixed** | The candidate contains the file, direct CLI validation accepts it, and the corpus explicitly asserts its presence and resolves it. [Corpus](../../lib/tests/l1/shipped_schema_corpus.rs#L925) | None |

The following witnesses account for the individual review 3 ledger rows.

| Required instances | Verified result |
| --- | --- |
| R3-F1.1, .2, .3, .12 | The public parser compares distinct written arms with anchored, tagged, and aliased versions in both kinds, including block and flow forms. Quoted parameter commas stay inside one flow item. The duplicate controls require two distinct source slices. Direct CLI probes admit the distinct signatures and report the earlier alias at 5:21 and later duplicate at 5:28. The anchored block duplicate also has two locations. [Public parser assertions](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L660) |
| R3-F1.4, .8, .9 | The CLI and DMLS witnesses assert both positions for direct duplicates, resolved duplicates, and category disagreements, and a single alias-use mark for an undeclared helper. The digest-verified loader checks the same semantic outcomes. Compose distinguishes admission that reaches the host-runtime boundary from a refused catalog that contributes no callable. [CLI](../../cli/tests/l1/schema_validate_yaml.rs#L1075), [DMLS](../../dmls/tests/l1/yaml_document_catalog_editor.rs#L446), [Registration](../../lib/tests/l1/host_registration_companions.rs#L729), [Compose](../../cli/tests/l1/compose_host_scope.rs#L310) |
| R3-F1.5, .13, .14, .15 | Private helpers and ordinary schema exports accept equivalent aliased sequences. Public data assertions distinguish the intended root arms from wrong nested values and reject extra properties under aliased closed constraints. The composition test records the alias use and confirms that resolution inherits the constraint. [Public schema assertions](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L717) |
| R3-F1.6 | The five Claudine witnesses exercise the same bytes through inventory verification and doctor. Both written and aliased controls load, and all three duplicate variants retain both positions with valid digests. [Doctor and inventory assertions](../../../claudine/lib/src/doctor/registration_tests.rs#L1288) |
| R3-F1.7, .10, .11 | Whole-value scalar, pipe, and sequence aliases preserve contracts. Narrowing conflicts keep the clause and clause-free arm in both orders. Unrelated aliases, case-folded collisions, comment and reference controls, and the envelope and document-shape refusals keep their outcomes. [Whole-value controls](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L744), [Editor controls](../../dmls/tests/l1/yaml_document_catalog_editor.rs#L159) |
| R3-F1.16, .17, .18, .19 | The standalone reference test asserts the exact reference range through a standard string tag and retains a decorated-key refusal. The spelling and source-map suites preserve tagged values, indentless arms, and exact grammar tokens beside tagged siblings. The editor preserves the diagnostic on the directive fallback. [Reference boundary](../../lib/tests/l1/meta_schema_phase6.rs), [Spelling test](../../lib/tests/l1/schema_yaml_spellings.rs#L936), [Source-map assertions](../../lib/tests/l1/schemas_source_projection.rs), [Editor spellings](../../dmls/tests/l1/schema_yaml_spellings.rs) |

## Unblocked Findings

### R4-F1 (high) The repository kind schema is missing from the worktree

**Authority (design).** The implementation plan's language guidance says “Add the two literals beside the existing schema kinds in the root kind catalog” and “Keep canonical schema links in docs.” The root schema named there, `schemas/kind.yaml`, is deleted in this worktree. The normal composition command for the kind guide fails at its inclusion of that file. A Markdown author who uses the documented schema also gets an unresolved reference for every kind. [Language guidance](plan.md#phase-5--verify-generated-outputs-and-publish-guidance), [Kind guide](../../../docs/kind-documents.md#L5)

The defect class is deletion of a repository schema that existing consumers still require. Both callable kinds should validate against the same shared schema, and changing only the kind to `unsupported-catalog` should produce a located enum error. The guide should expand the shared schema at its existing reference. I compared the candidate with a temporary copy of the committed file and used the same repository-built CLI for both.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R4-F1.1 | Shared kind schema used by Markdown validation | Point three Markdown documents at the checked-out schema. Use `expression-schema`, `mutation-schema`, and `unsupported-catalog`. Repeat with the committed schema copied to a temporary directory. | All three candidate documents fail because the file is missing. The committed control admits both callable kinds and rejects the unsupported kind at line 3, column 7. | Admit the two authored kinds and report the unsupported kind at its value. |
| R4-F1.2 | Kind guide composition and the direct schema link in the repository prompt | Run `md compose docs/kind-documents.md --deny all --output markdown` and `md validate refs system-prompt.md --output plain`. | Composition exits 1 with one broken reference at line 5, column 1. Reference validation also reports that path at line 30, column 100 of the prompt. | The guide embeds the definitions and the prompt opens the shared schema. |
| R4-F1.3 | Darkmatter's shipped-schema corpus | Compare its filesystem scan and explicit required files against the deleted schema. Rerun the corpus test and validate the neighboring root schemas directly. | The scan sees only files that exist and the presence list omits `schemas/kind.yaml`. The fresh corpus test passed in 1.134 s while the path was deleted. The review-cycle, specification, and suggestion-review schemas all validate. | The corpus fails if this required artifact disappears and exercises the restored schema through its ordinary resolution checks. |
| R4-F1.4 | Shipped YAML kind definitions and built-in catalog inputs | Validate the shipped expression and mutation catalogs through `md schema validate`. Check the per-kind Simplified Schema definitions in the shipped artifact evidence. | Both catalogs validate against their own shipped definitions. The definition parity and installed-host tests pass. | Keep both catalog definitions and their current behavior. |

Restore the committed `schemas/kind.yaml` at its existing path. Add that path to the corpus's existing required-artifact assertion so the next deletion is visible. This repairs both references and the coverage gap without changing the format or adding another schema. [Corpus presence check](../../lib/tests/l1/shipped_schema_corpus.rs#L925), [Repository prompt](../../../system-prompt.md#L30)

## Recurrence

R4-F1 is a newly found sibling of **R1-F3, The shared review and fix schema cannot validate documents**. That earlier repair checked structural validity and explicit presence for the review-related schemas. The same corpus also reads the repository's kind schema when it exists, but its required-artifact check missed that path. The current deletion breaks two consumers that the directory scan cannot witness. The neighboring schemas still validate. The review 3 repair ledger recorded the same class as verification issue R3-V1 for a different path, the restored suggestion-review schema. The corpus explicitly checks that path. [Review 1 finding](review-1.md#r1-f3-high-the-shared-review-and-fix-schema-cannot-validate-documents), [Required-artifact assertion](../../lib/tests/l1/shipped_schema_corpus.rs#L925)

## Input Robustness Matrix

Invalid input is refused before evaluation or publication. Catalog descriptions may be omitted or empty. Catalog category and type collections must contain declarations. An empty manifest `files` list is valid with its lifecycle member and digest. Facts require explicit null where a nullable rule is unset. Alias, facts, and source-exclusion collections may be empty.

| Validator or loader | Level 1 evidence |
| --- | --- |
| Catalog envelope and YAML loading | The public envelope suite covers absent, null, wrong whole-field type, invalid elements, and empty collections. Duplicate keys, trailing content and second documents are refused. Comments, line endings, byte-order marks, quoted keys, and the repaired node-property and alias presentations have positive controls. Both shipped catalog inputs also pass the normal CLI. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs) |
| Manifest envelope, member paths, and verified file inventory | The public snapshot cases cover required keys, explicit null and wrong types, invalid members, the empty list control, duplicate and trailing documents, unsupported versions, path boundaries, digest mismatches and missing companions. The legacy cleanup decoder has its own valid and malformed controls. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs) |
| Facts JSON and binding rules | The public host tests exercise absent, null, invalid field and element types, empty collections, duplicate keys and trailing content. The installed companion tests add stale hashes, unusable aliases, JSON5 rejection, and locations in the original bytes. [Facts matrix](../../lib/tests/l1/host_functions.rs#L396), [Installed companion tests](../../lib/tests/l1/host_registration_companions.rs) |
| Readers that could erase invalid shapes | Catalog metadata is checked against the authored YAML. The manifest's strict admission requires every field and checks the digest inventory independently of preliminary watch candidates. The facts reader distinguishes required and nullable properties explicitly. The permissive reads I traced populate watch candidates or the display spelling of already parsed examples. |

## Verification

| User-facing requirement | Level 1 evidence |
| --- | --- |
| Built-in callable and lifecycle behavior | Descriptor and binding parity, data-reading permission checks, generated lifecycle comparisons, and action-form and event admission assertions. The two shipped catalogs passed direct validation. |
| Standalone catalog validation and privacy | Shared parser and schema-target tests, CLI verdicts and locations, and headless DMLS diagnostics. The current CLI probes confirmed admission and both conflict locations in both kinds. |
| Selected-host binding and runtime | Installed registration and editor reload tests cover removal, stale facts, restored files, and selection. Compose and the Rust host fixture cover argument binding, the analysis-only runtime boundary, actual execution, and calls in action operands. |
| Publication, listing and upgrade | Verified inventory, installed publication and doctor tests cover digest-checked contents, unsupported-version guidance, companion repair, and format 2 upgrade in place. |
| Mutation refusal and existing action rules | Installed member admission checks the mutation-kind refusal. Lifecycle generation, runtime, and event tests cover the existing action shapes and restrictions. |
| Kind guide and shared document-kind validation | Direct composition and Markdown validation exposed R4-F1. The extracted committed schema passed both callable-kind controls and rejected the unsupported kind at its field. |

Level 1 reaches the required surfaces because the outcomes are library results, file contents, CLI reports, and editor protocol diagnostics. The repair transcripts record 2,626 default and 2,202 minimal-feature Simplified Schema passes, 18,037 Darkmatter passes, and 13,077 Claudine passes. Those transcripts contain no test at or above five seconds. Fresh Linux runs passed 31 Simplified Schema catalog cases, all 2,202 minimal-feature cases, 45 Darkmatter YAML spelling cases, 13 Darkmatter alias cases, five Claudine doctor and inventory cases, and the isolated corpus check. No selected test reached five seconds. The tier audit found zero stranded tests across all three areas. Cross-OS evidence remains with CI.
