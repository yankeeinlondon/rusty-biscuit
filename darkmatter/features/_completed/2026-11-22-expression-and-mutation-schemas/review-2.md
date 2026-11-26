---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-2.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-1.md"
next: "2026-11-22-expression-and-mutation-schemas/review-3.md"
created: "2026-11-25T00:27:11+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R2-F1 (high): Host registrations accept names that Markdown cannot call"
    - "R2-F2 (high): Type-reference errors can point to unrelated text"
    - "R2-F3 (medium): The no-terminal approval test exceeds the Windows L1 budget"
    - "R2-F4 (medium): The Claudine trigger discovery test exceeds the Windows L1 budget"
    - "R2-F5 (medium): The recursive lifecycle corpus test exceeds the Windows L1 budget"
    - "R2-F6 (medium): The depth-four lifecycle test exceeds the Windows L1 budget"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "codex/gpt-6.2-sol"
---

# Expression and mutation schemas review 2

The feature needs repairs before it is production ready. All four prior findings passed their required checks. Two new behavior findings and four recorded slow tests remain. All repairs can proceed within the existing scope.

## Previous Finding Verification

The child IDs below refer to the [repair ledger](log.md#finding-ledger). I checked the assertions and reran the relevant Level 1 suites. Direct CLI and editor probes supplemented them where the consumer mattered.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 Inline YAML comments make valid catalogs fail | R1-F1.1 helpers in each carrier, .2 callable, .3 mutation, .4 ordinary schema, .5 loader and scalar controls, .6 additional carriers, .7 BOM and CRLF coordinates, .8 loader BOM rule | **verified fixed** | For .1–.2, the shared parser preserves the definitions and callable span, standalone CLI and editor accept the commented catalog, and the installed catalog exposes the callable with a matching companion. For .3–.4, CLI and editor accept the comments, host admission still refuses the mutation kind, and the ordinary schema export accepts text and refuses a number. For .5–.8, the public parser and YAML loader tests check quoted and joined hashes, comments on flow and block values, explicit keys, anchors, document markers, LF and CRLF with and without BOM, and refusals for duplicate keys, trailing content, and a second document. A direct multiline callable control also passed in both kinds. [Shared carrier tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L386), [CLI comments](../../cli/tests/l1/schema_validate_yaml.rs#L772), [Editor comments](../../dmls/tests/l1/yaml_document_catalog_editor.rs#L93), [Installed comments](../../lib/tests/l1/host_registration_companions.rs#L290), [Whole-document loader](../../lib/tests/l1/schema_extraction_characterization.rs) | None |
| R1-F2 Catalog diagnostics lose declaration locations | R1-F2.1 direct duplicate overloads, .2 resolved duplicates, .3 category failures, .4 compose projection, .5 facts conflicts, .6 built-in coordinates, .7 conflict and refusal controls, .8 grouped arms, .9 claim order, .10 checkout navigation, .11 refresh, .12 verified inspection, .13 doctor, .14 single editor report, .15 kind-registry API | **verified fixed** | For .1–.3 and .8, the parser, CLI, editor and installed-host checks preserve both arms for conflicts and locate the rejected category. Changing the later overload to `number` passes. For .4–.7 and .9, compose preserves related positions in text and JSON, including cross-catalog cases. Registered facts point into the correct catalog or companion for each claim order. Standalone built-in conflicts carry embedded coordinates. Duplicate keys, narrowing conflicts, invalid alias elements and mutation-kind refusal keep their expected routes. For .10–.11, the editor links matching source in Git and non-Git workspaces and switches back to embedded coordinates when the source changes or disappears. For .12–.13, the doctor fixture reaches verified inspection with updated digests and reports both category positions. For .14–.15, editor checks assert one diagnostic for category and overload conflicts and the distinct YAML error for a duplicate key. The kind-registry test asserts the exact signature span for an undeclared category. [Shared overload tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L317), [CLI](../../cli/tests/l1/schema_validate_yaml.rs#L800), [Compose](../../cli/tests/l1/compose_host_scope.rs#L189), [Editor](../../dmls/tests/l1/yaml_document_catalog_editor.rs#L110), [Installed host](../../lib/tests/l1/host_registration_companions.rs#L351), [Doctor](../../../claudine/lib/src/doctor/registration_tests.rs), [Kind registry](../../lib/tests/l1/yaml_kind_registry.rs) | None of the ledger instances. R2-F2 covers a newly found sibling in type-reference diagnostics. |
| R1-F3 The shared review and fix schema cannot validate documents | R1-F3.1 nested blast-radius shape, .2 pipeline arm, .3 review-only arm, .4 reserved placeholder, .5 remaining corpus | **verified fixed** | Direct `md schema validate` accepts the repaired shared schema. The public Markdown test resolves both mapping arms, checks the pipeline name requirement, accepts both document shapes, refuses missing `reviewed` on both, and locates the wrong packages shape. The corpus checks the zero-byte placeholder and checks parse, resolution, and registry expectations for the other schemas. [Shared schema](../../../schemas/review-fix-cycle.yaml), [Corpus](../../lib/tests/l1/shipped_schema_corpus.rs#L1069) | None |
| R1-F4 Registration and catalog references still describe the old formats | R1-F4.1 architecture, .2 module map, .3 fresh-publication name, .4 corpus classification, .5 publication and cleanup notes, .6 current and legacy documentation, .7 inventory fixture, .8 setup expectation | **verified fixed** | Both maintainer pages, the fresh-publication test and its module header, the inventory fixture, and the setup expectation identify format 3. The publication test asserts the integer written to disk and loads the installation without a warning. Legacy cleanup notes describe their separate format 2 fixtures, and the docs and caller guidance agree with the encoder. The corpus calls the two kinds private and exercises their schema-target and import refusals. [Architecture](../../../.claude/skills/claudine/architecture.md#L36), [Module map](../../../.claude/skills/claudine/module-map.md#L13), [Publication](../../../claudine/lib/tests/l1/host_publication.rs), [Inventory](../../../claudine/lib/tests/l1/registration_inventory.rs), [Setup](../../../claudine/cli/tests/l1/setup_command.rs#L102), [Registration guide](../../docs/topics/host-registration.md), [Corpus](../../lib/tests/l1/shipped_schema_corpus.rs) | None |

## Unblocked Findings

### R2-F1 (high) Host registrations accept names that Markdown cannot call

**Authority (spec).** “A new host expression declaration makes the function available in expression positions, including expressions in action operands.” The registration section also applies the existing expression name matching rules to canonical names and aliases. Renaming the project fixture's callable to `"2fa_label"` or `"project label"` passes catalog validation. A host can bind a Rust handler to the exact name, generate the companion, and register the bundle. A Markdown call using that name fails before dispatch. An alias such as `"project label"` behaves the same way. [Callable scope](spec.md#what-each-kind-describes), [Host registration](spec.md#publish-and-load-the-declarations)

The defect class is admission of callable spellings the expression parser cannot dispatch. Quoting a YAML key or changing only the ASCII case of a supported name must preserve admission and lookup. Changing a name or alias to a spelling that does not parse as that same callable must refuse it where it was declared. The probes used the project fixture's string signature, facts generated from the public host builder, and a temporary format 3 installation with recalculated digests.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Darkmatter's shared authored expression catalog reader, `md schema validate`, and DMLS | Rename the fixture callable to `"project label"`, `"2fa_label"`, or the Boolean spelling `"true"` | The CLI accepts the space and leading-digit keys. DMLS publishes no objection to the space or Boolean key. The public parser fails to parse the first two as calls. | Refuse an uncallable key at the declaration. |
| R2-F1.2 | Darkmatter's Rust host builder and facts generation | Bind the exact space or leading-digit canonical name, or add the alias `"project label"` or `""` to the valid canonical name | The builder accepts the registrations and writes companions. The space and leading-digit forms fail to parse as calls. The empty alias followed by parentheses parses as a grouped expression. | Refuse an uncallable name or alias before generating facts. |
| R2-F1.3 | Installed catalog and companion admission, then the selected document's expression dispatch | Install the digest-consistent bundles and compose a matching document. Also edit an alias to `"true"` or add surrounding spaces | The host loads and is selected. Space and leading-digit calls fail to parse even though the installation advertises them. The facts reader accepts the empty, padded, and Boolean aliases. DMLS reports malformed calls for the space and Boolean aliases in Markdown. Completion offers the unusable canonical name and matches the unusable alias to its canonical entry. | Refuse a bad canonical name at its catalog key and a bad alias at its JSON element. The publisher's verified inventory uses this same admission path. |
| R2-F1.4 | Expression parsing, selected lookup, built-in and cross-catalog collision rules | `project_label`, `project-label`, and `project.alias`, plus the existing case-folded name and alias tests | Clean. The spellings parse as calls and reach the expected host-runtime boundary. The existing tests still refuse collisions and allow the same name in different hosts. | Keep the names and matching rules already accepted by the expression parser and the selected host. |
| R2-F1.5 | Mutation admission and ordinary type helpers | Install the mutation fixture or keep its ordinary `Label` helper | Clean. The installed reader refuses the action catalog on `kind`. The private helper stays a type and contributes no callable name. | Keep mutation admission and helper semantics. |

The facts array shape controls pass. Omitting `aliases` locates the missing field at the containing object. Null and numeric fields are refused at the field. `["good", 1]` and `[1]` are refused at the bad element. `[]` and a valid string alias pass. A well-typed string with an unusable spelling is the failing semantic case. The catalog's `types` shape tests separately cover missing, null, scalar, list, empty and malformed declarations. YAML quotes around an otherwise valid name preserve its identity. [Catalog matrix](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L236), [Facts reader](../../lib/src/markdown/compose/expression/host/artifact.rs#L447), [Public registration matrix](../../lib/tests/l1/host_registration_companions.rs#L99)

Use the existing expression grammar to decide whether a spelling can appear as one call name. Reuse that decision for authored expression keys and both alias admission paths, retaining their existing source locations. The repair can keep the current manifest and facts formats. [Catalog checks](../../lib/src/markdown/compose/expression/catalog/mod.rs#L599), [Runtime name validation](../../lib/src/markdown/compose/expression/host/mod.rs#L371), [Installed admission](../../lib/src/config/registration.rs#L1616), [Expression parser](../../../simplified-schema/lib/src/expression/parser.rs#L589)

### R2-F2 (high) Type-reference errors can point to unrelated text

**Authority (spec).** “Diagnostics point to the declaration or field that failed and include the other location when two declarations conflict.” If a missing type called `Missing` appears in a later overload, the catalog reader can highlight an earlier overload's description. In a single signature, adding `# Missing` to the YAML block header moves the error out of the signature and onto the comment. The CLI, editor, and installed-host warning reproduce the same relocation. [Catalog format and validation](spec.md#catalog-format-and-validation)

The defect class is choosing a diagnostic position by matching source text unrelated to the parsed declaration. A comment, description, or parameter rename that preserves the failing reference must keep the mark on the failing declaration. Changing the reference back to the declared helper `Label@this` must make the fixture valid. The probes used that passing control and then changed the reference or added an earlier matching word. Positions below use one-based coordinates.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Missing local helper in a callable parameter | `name: Missing@this`, then add `# Missing` to the scalar header or rename the parameter to `Missing` | Both kinds locate the correct reference in the unambiguous control. The header edit moves the mark from 7:30 to 5:25. The parameter rename puts the mark on the parameter name at 7:24. The installed expression registration shows the same positions. | Point to the failing type reference or its actual signature. |
| R2-F2.2 | Return type and overload selection of the location | Put `Missing@this` in the return type with a parameter named `Missing`. In a separate control, put the missing type in overload 2 and mention it in overload 1's description | The return error highlights the parameter. A YAML sequence marks the description on the preceding arm at 6:85 through CLI, editor and installed expression admission. The inline-flow sequence also marks the earlier description. Folded and literal pipe controls with `Missing` as a parameter name in arm 1 likewise misplace the error in that arm. | Point inside the actual return or the second arm. |
| R2-F2.3 | External reference branch and callable-used-as-type branch | Put `Label@./helpers.yaml` or `project_label@this` in the parameter type, with the same filename or callable name in a header comment | Both branches highlight the header comment in both standalone kinds and installed expression registration. The callable-as-type branch correctly preserves the related callable key at 5:5. | Use the offending signature as the primary location and retain the related key where one exists. |
| R2-F2.4 | Private helper traversal through a mapping, inline object, object extension, and tuple | Compare `item: Missing@this # Check Missing` with `Missing: Missing@this`, and add an earlier child description before `$extends: Missing@this`. Check the same absent type in a tuple. | Mapping and inline-object keys can steal the value's location. The extension points to the earlier child's description when it mentions the name. The ordinary mapping with a trailing comment and the unambiguous tuple point to the reference. | Point at the declaration containing the failing reference, including the extension when that is the cause. |
| R2-F2.5 | Independent syntax, overload, category, template-slot and collision diagnostics | Unknown type versus malformed Simplified Schema syntax, identical overloads in scalar and sequence carriers, category repetition, a slot in a helper, and a case-folded duplicate | Clean for this class. The syntax path ranges the invalid type expression, duplicate overload and category checks use their typed arm or entry positions, the slot check marks its owning key, and canonical conflict checks use the two keys. The repaired related-location tests cover compose, verified inspection and doctor. | Preserve the structural marks already used by these checks. |

The shared Darkmatter check knows which parsed reference failed but searches the entire authored entry for the first matching word. Every output surface faithfully displays the resulting offset. The smallest repair can use the failing arm or declaration range from the existing source map when an exact reference offset is unavailable. The spec permits that location. Keep the related callable key and avoid adding a general source-rewriting layer. [Reference checks](../../lib/src/markdown/compose/expression/catalog/checks.rs#L123), [Catalog reader](../../lib/src/markdown/compose/expression/catalog/mod.rs#L526), [Installed diagnostic projection](../../lib/src/config/registration.rs#L1616)

### Recorded Windows test timings

The original Windows CLI package run contains four cases above the 5-second Level 1 limit. The implementer's one-thread run and the final full run with four test threads provide useful comparison points. The original record contains no load measurement that would qualify it for the under-load exception. The standard package result therefore remains evidence of a timing failure for each row. The measurements below come from the original, focused, and final Windows logs named in the [implementation record](log.md#completion-report). In the later focused Linux run all 12 cases in the same scope finished below 5 seconds.

| Instance | Test | Original Windows package | Isolated Windows | Four-thread Windows package | Expected |
| --- | --- | ---: | ---: | ---: | --- |
| R2-F3.1 | No-terminal approval | 5.007 s | 2.503 s | 2.945 s | At most 5 s at normal package concurrency |
| R2-F4.1 | Claudine bundle trigger discovery | 5.657 s | 1.804 s | 4.188 s | At most 5 s at normal package concurrency |
| R2-F5.1 | Recursive lifecycle corpus verdicts and positions | 5.324 s | 1.752 s | 3.131 s | At most 5 s at normal package concurrency |
| R2-F6.1 | Depth-four lifecycle controls and positions | 5.856 s | 1.997 s | 4.169 s | At most 5 s at normal package concurrency |

The four cases were the only tests over the limit in the inspected original Windows log. The class is Level 1 tests whose runtime grows under package concurrency. The isolated controls narrow the search to shared work such as subprocess startup, repository setup, and schema preparation. The measurements alone do not identify which work is responsible.

### R2-F3 (medium) The no-terminal approval test exceeds the Windows L1 budget

**Authority (defect).** A normal Windows CLI package run takes 5.007 seconds for `without_a_terminal_a_command_shape_is_not_prompted_for`, over the Level 1 limit. The test checks that an unapproved shell template returns an error without prompting or modifying configuration. Its isolated and four-thread controls are in R2-F3.1 above. [Approval test](../../cli/tests/l1/approval_prompt.rs#L308), [Level 1 budget](../../../.claude/skills/rust-testing/test-speed.md)

This is the single-spawn member of the timing class. Measure fixture setup, process startup, and composition separately at default package concurrency. Reduce the setup or production work that dominates and keep the single CLI assertion for the no-terminal path. The neighboring approval tests in the original run stayed below the limit.

### R2-F4 (medium) The Claudine trigger discovery test exceeds the Windows L1 budget

**Authority (defect).** A normal Windows CLI package run takes 5.657 seconds for `claudine_bundle_loads_and_its_triggers_are_discovered`, over the Level 1 limit. This test checks that the published schemas validate and their triggers are discovered. Its isolated and four-thread controls are in R2-F4.1 above. [Trigger discovery test](../../cli/tests/l1/claudine_schema_bundle.rs#L153), [Level 1 budget](../../../.claude/skills/rust-testing/test-speed.md)

This member of the timing class builds a full registration fixture and calls into the CLI to check validation and trigger selection. Time those steps with the other bundle fixtures running. Reuse an in-process trigger query for redundant CLI work if the measurement identifies it, keeping a CLI witness for installed-bundle selection.

### R2-F5 (medium) The recursive lifecycle corpus test exceeds the Windows L1 budget

**Authority (defect).** A normal Windows CLI package run takes 5.324 seconds for `recursive_lifecycle_corpus_gets_the_agreed_verdicts_and_positions`, over the Level 1 limit. This test checks recursive schema verdicts and locations against the shared corpus. Its isolated and four-thread controls are in R2-F5.1 above. [Recursive lifecycle test](../../cli/tests/l1/claudine_schema_bundle.rs#L521), [Level 1 budget](../../../.claude/skills/rust-testing/test-speed.md)

This member of the timing class prepares a complete bundle and runs a batch with recursive fixtures. Profile bundle setup and validation separately. If repeated preparation across commands dominates, move the duplicate preparation behind the existing public in-process schema API while retaining a representative CLI location check and every corpus verdict.

### R2-F6 (medium) The depth-four lifecycle test exceeds the Windows L1 budget

**Authority (defect).** A normal Windows CLI package run takes 5.856 seconds for `depth_four_container_controls_validate_and_their_innermost_defects_are_located`, over the Level 1 limit. The test checks that nested actions validate and that three bad actions are located at their innermost field. Its isolated and four-thread controls are in R2-F6.1 above. [Depth-four lifecycle test](../../cli/tests/l1/claudine_schema_bundle.rs#L631), [Level 1 budget](../../../.claude/skills/rust-testing/test-speed.md)

This member of the timing class uses both a trigger trace and a validation batch against the same bundle. Measure those two phases at default package concurrency. Keep all depth and location cases and reuse preparation where the existing schema API makes that possible.

## Recurrence

R2-F2 is a sibling of **R1-F2, Catalog diagnostics lose declaration locations**, in [review 1](review-1.md#r1-f2-high-catalog-diagnostics-lose-declaration-locations). The earlier sweep fixed structural overload, category, name, and facts positions, and their downstream projections. It missed the separate first-word source search in Darkmatter's local-reference check. That search already existed before the repair. The R2-F2 instance table covers its local, external and callable-type branches, the helper and overload shapes they traverse, and the downstream positions. The previous ledger instances still pass.

## Input robustness and verification

Invalid inputs are expected to produce a located refusal before evaluation or publication. An omitted or empty catalog description is valid. Catalog categories and types require contents. A manifest can have `files: []` with its lifecycle member present. A blank nullable facts rule is expressed as explicit JSON null. Empty aliases, facts and source-exclusion lists are valid collections.

| Boundary | Level 1 evidence |
| --- | --- |
| Catalog shape, validation and privacy | The public envelope and kind-registry suites cover required metadata, null and empty values, whole-field and element types, private helpers, category inheritance, overloads and source locations. Direct CLI and headless DMLS probes covered both catalog kinds. The YAML tests cover comments, BOM, LF and CRLF, duplicate keys, second documents and trailing content. Schema-target and named-type import checks refuse both kinds. The remaining semantic and source-location cases are R2-F1 and R2-F2. |
| Manifest and facts readers | Public snapshot and installed-host tests cover required fields, closed format 3, the version remedy, wrong field and element types, empty members, exact digests, same-stem and listed companion collisions, path confinement, missing companions, stale catalog hashes and retries across changed manifests. The shared strict JSON loader checks duplicate keys, trailing content, invalid JSON5 syntax and locations after non-ASCII characters. Facts rebuild tests cover nullable rules, required fields, and facts incompatible with the signature. Edited alias shapes were also checked directly through a verified installation. |
| Selected host, execution boundary and mutation refusal | The compose and DMLS tests check selected and unselected documents, separate hosts, inherited host scope in transclusions, binding before conversion, missing/stale/returned companion bytes, and the host-runtime diagnostic. Public host tests cover runtime registration, eager execution and return validation. Mutation catalog members still receive the host-action refusal at their kind. |
| Built-in behavior and generation | Signature, lifecycle parity, operand, event and boundary tests check the generated action shapes and execution rules. The repaired comment controls also cover each catalog kind and an ordinary exported schema. The implementation record preserves the byte comparison to the pre-feature generated lifecycle and function references. |
| Publication and repair | Claudine's published inventory, installed setup, and doctor tests check format 3, the fixed path, frozen format 2 upgrade, missing and modified companions, related positions from verified inspection, and repeat repair. |

These requirements exercise structured input, effects, file contents, and protocol results. Level 1 covers the observable boundary. Terminal-emulator rendering and OS keyboard encoding are outside the feature's contract.

Fresh Linux Level 1 runs passed 71 focused Simplified Schema tests plus 2,196 minimal-feature tests, Darkmatter slices of 393, 820, 54, and 12 tests, and Claudine slices of 155 and 25 tests. No test in those runs reached five seconds. All three areas passed `just shipped-schemas --check`, and the tier audit found zero stranded tests. The recorded native Windows timing exceptions remain R2-F3 through R2-F6.
