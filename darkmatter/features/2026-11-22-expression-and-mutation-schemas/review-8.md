---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-8.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-7.md"
next: "2026-11-22-expression-and-mutation-schemas/review-9.md"
created: "2026-11-25T05:41:08+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R8-F1 (medium): Nested Git checkouts can inherit the enclosing checkout's schemas"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "claude/opus"
---

# Expression and mutation schemas review 8

The eight iteration 7 cases are verified fixed. A nested Git checkout can still lose its own schema roots or pick up an enclosing checkout's trigger. The outcome can change when two inclusions switch order. This finding blocks production readiness and needs no author decision.

## Previous Finding Verification

I checked the eight rows of the iteration 7 repair ledger through the worktree CLI with an isolated home. The earlier findings retain their verified outcomes. The table includes the previous checks by name and the evidence for their original scope. No prior review contains a human ruling. [Iteration 7 ledger](log.md#implementation-of-review-findings-7)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R7-F1 Documents opened through another checkout lose their schema roots | R7-F1.1–.8, expanded below | **verified fixed** | All eight rows matched their expected results for sibling checkouts and the original controls. The library and CLI tests exercise the same fixture through preflight, composition, and reference checks. [Library tests](../../lib/tests/l1/reference_bare_schema_roots.rs), [CLI tests](../../cli/tests/l1/reference_bare_schema.rs) | None of the eight instances. R8-F1 covers nested repositories and order-dependent reuse. |
| R6-F1 Included documents and fragment targets inherit schema-root eligibility from the parent | R6-F1.1–.7 | **verified fixed** | Direct and nested checks agree for the tested note outside Git. The repository child and the scoped eligibility test passed. [Library tests](../../lib/tests/l1/reference_bare_schema_roots.rs), [CLI tests](../../cli/tests/l1/reference_bare_schema.rs) | None of the seven instances. |
| R5-F1 The kind guide's expression catalog uses a reserved name | R5-F1.1–.5 | **verified fixed** | The guide uses `project_slug`. The built-in name and alias checks still assert the conflicting declaration. The accepted examples and published caller guidance were verified in review 6. [Kind guide](../../../docs/kind-documents.md), [Prior verification](review-6.md#previous-finding-verification) | None |
| R5-F2 The category guide misstates which catalogs require categories | Guide, maintainer skill, expression and mutation controls | **verified fixed** | The guide scopes optional categories to ordinary schema envelopes. The two strict callable envelope cases passed the category-shape tests. [Guide](../../../simplified-schema/docs/topics/language/schema-documents.md#categories), [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs) | None |
| R5-F3 Reference commands miss schemas named by bare filename | R5-F3.1–.7 | **verified fixed** | The in-repository root, child, fragment, named import, and disabled-trigger cases passed. Review 6 separately checked the actual review and evidence documents. [Library tests](../../lib/tests/l1/reference_bare_schema_roots.rs), [Prior verification](review-6.md#previous-finding-verification) | None of the seven instances. |
| R4-F1 The repository kind schema is missing from the worktree | R4-F1.1–.12 | **verified fixed** | The schema is tracked and both shipped corpus checks ran. The kind admission and reference classification tests passed. Review 5 recorded the deletion and exact test-selection checks. [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs), [Prior verification](review-5.md#previous-finding-verification) | None |
| R3-F1 YAML anchors and aliases break overload validation in sequences | R3-F1.1–.19 | **verified fixed** | The Simplified Schema, CLI, editor, and installed-loader tests passed the decorated and aliased arm cases and source-range checks. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [Prior verification](review-4.md#previous-finding-verification) | None |
| R2-F1 Host registrations accept names that Markdown cannot call | R2-F1.1–.11 | **verified fixed** | Current builder, installed catalog, and editor tests passed the spelling and case controls, including refusal before selection. [Host function tests](../../lib/tests/l1/host_functions.rs), [Installed catalog tests](../../lib/tests/l1/host_registration_companions.rs) | None |
| R2-F2 Type-reference errors can point to unrelated text | R2-F2.1–.12 | **verified fixed** | Current CLI, editor, and registration tests still check locations with earlier lookalike text and repeated arms. [Editor tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs), [Prior verification](review-3.md#previous-finding-verification) | None |
| R2-F3 The no-terminal approval test exceeds the Windows L1 budget | R2-F3.1–.2 | **verified fixed** | The same test passed in 2.878 s in the recorded Windows package run. Its refusal and persistence assertions passed on Linux in this review. [Approval tests](../../cli/tests/l1/approval_prompt.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R2-F4 The Claudine trigger discovery test exceeds the Windows L1 budget | R2-F4.1–.4 | **verified fixed** | The recorded Windows package run measured 2.412 s. Both document verdicts and the trigger-count assertions remain in the Linux suite. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R2-F5 The recursive lifecycle corpus test exceeds the Windows L1 budget | R2-F5.1 | **verified fixed** | The recorded Windows package run measured 2.030 s. The corpus and its deep trigger, location, and deferred-operand assertions passed on Linux. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R2-F6 The depth-four lifecycle test exceeds the Windows L1 budget | R2-F6.1 | **verified fixed** | The recorded Windows package run measured 2.467 s. The combined valid and invalid action fixtures passed on Linux. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R1-F1 Inline YAML comments make valid catalogs fail | R1-F1.1–.8 | **verified fixed** | The loader, CLI, editor, and installed reader tests passed their comment, literal hash, and line-ending controls. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [Prior verification](review-2.md#previous-finding-verification) | None |
| R1-F2 Catalog diagnostics lose declaration locations | R1-F2.1–.15 | **verified fixed** | The current catalog diagnostics tests passed their authored and related location assertions for syntax, overloads, categories, names, and facts. [Installed catalog tests](../../lib/tests/l1/host_registration_companions.rs), [Prior verification](review-2.md#previous-finding-verification) | None |
| R1-F3 The shared review and fix schema cannot validate documents | R1-F3.1–.5 | **verified fixed** | The shipped corpus and both branches of the review-cycle document validation passed. [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs) | None |
| R1-F4 Registration and catalog references still describe the old formats | R1-F4.1–.8 | **verified fixed** | The registration guide describes format 3 and derived companions. The current listing and format 2 upgrade tests passed. Prior verification checked the corresponding published caller skills. [Registration guide](../../docs/topics/host-registration.md), [Prior verification](review-6.md#previous-finding-verification) | None |
| R3-V1 The shipped-schema corpus retains its required suggestion-review schema | File presence and ordinary resolution | **verified fixed** | The file is tracked and both passing corpus checks explicitly require it. [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs) | None |

| Iteration 7 instance | Verification evidence |
| --- | --- |
| R7-F1.1 Reference report and Mermaid graph | Body, prologue, and epilogue accepted bare, relative and absolute schema paths. Each refused the missing schema and named it. |
| R7-F1.2 Standalone graph | Plain graph, validation, and follow-and-validate gave the same lookup verdict for each inclusion form. |
| R7-F1.3 Heading fragments and TOC targets | All three resolvable spellings generated the heading and passed TOC composition and reference preparation. A missing schema lost the generated fragment. A literal heading still resolved. |
| R7-F1.4 Composition and command discovery | Default and disabled-trigger compose accepted the three resolvable spellings on all three parent forms. Report-only `compose --shell` accepted those spellings for the body inclusion. Each refused the missing name. |
| R7-F1.5 Named-type import | Direct validation and cleaning, graph, references, composition, shell discovery, generated fragments and TOC checks accepted the bare and explicit type libraries. A missing library failed preparation while the static fragment check still passed. |
| R7-F1.6 Root trigger selection | Matching frontmatter caused all three inclusion forms to report the child checkout's missing `version`, with no `parent_only` diagnostic. Changing the topic, or disabling triggers, cleared the error. Passive reference and graph checks passed for both topics. |
| R7-F1.7 Direct checks and Bash completion | Direct schema validation, cleaning, compose, graph, and references accepted all three resolvable spellings and refused the missing schema. Bash offered the schema's value for those three only. |
| R7-F1.8 Established source boundaries | A child in the same checkout still uses its root; a note outside Git still refuses bare names, accepts explicit paths, and preserves static fragments. The existing library controls and CLI tests cover both cases. |

## Unblocked Findings

### R8-F1 (medium) Nested Git checkouts can inherit the enclosing checkout's schemas

**Authority (defect).** A developer can keep a component as a separate Git checkout inside a project, as with a submodule. In an isolated fixture, `md compose ./vendor/review.md` from the project root reports that the component's `$schema: review.yaml` cannot be resolved. Running `md compose review.md` from inside the component resolves that same schema and renders the heading. With an explicit schema path, the matching trigger can still come from the enclosing project and request the wrong property. With the component inside a separate notes checkout, merely including an introductory note from that checkout first can cause the same failures. [Darkmatter CLI document context](../../cli/src/request.rs#L335), [Darkmatter source admission](../../lib/src/markdown/compose/context/options.rs#L175), [Repository memo](../../lib/src/markdown/compose/context/request.rs#L512)

The defect class is inferring a source's repository identity from directory containment. A source in a distinct Git checkout should use that checkout's schema roots. Spelling its schema as a bare name or an explicit path should preserve the result. A missing name must still fail. Reordering an independent inclusion with no frontmatter must preserve both schema and trigger selection. With the schema spelled explicitly, changing the note's topic from `review` to `other` must clear the trigger result.

To reproduce, initialize `~/app`, `~/app/vendor`, `~/notes`, and `~/notes/vendor` as separate Git repositories in a temporary home. The direct result also reproduces with a `.git` file made by `git init --separate-git-dir`, the layout used by submodules. Put a standalone `schemas/review.yaml` defining a required `description` in each `vendor` checkout. Place this Markdown in each checkout's `review.md`.

```markdown
---
$schema: review.yaml
description: Review heading
topic: other
---

# {{ fm.description }}

## Static
```

From `~/app`, include `./vendor/review.md` or `~/notes/vendor/review.md` using `::file`, `prologue:`, or `epilogue:`. For the order test include `~/notes/seed.md` before or after the second note. The seed contains only a heading. For the trigger test add a trigger to each `vendor` checkout requiring `version` for `topic: review`, and one with the same match in the enclosing checkout requiring `parent_only`. Use `./schemas/review.yaml` for this test to separate trigger selection from bare-name lookup.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R8-F1.1 | Direct CLI document admission and schema preparation | Run `schema validate`, `clean`, `compose`, `validate refs`, and `graph --validate` on the component note from the enclosing repository. Repeat from the component and on the nested note outside the launch tree. | Inside the launch tree, bare fails from the enclosing directory and succeeds from the component. Relative and absolute succeed from either. Missing fails. The foreign nested note directly opened from `app` behaves correctly. | Each direct entry point accepts all resolvable spellings from either launch directory and refuses the missing name. |
| R8-F1.2 | Reference report and graph inclusion nodes | Use body, prologue and epilogue with bare, relative, absolute and missing. Run both reference reports and plain, validate, and follow-and-validate graph modes. Swap the independent seed and the body inclusion for the foreign nested note. | All modes refuse bare inside the launch tree. For the foreign note, bare passes when visited first or alone and fails after the seed. Explicit paths pass and missing fails. | Bare and explicit paths agree for both trees and both visit orders. Missing still fails and names the unresolved reference. |
| R8-F1.3 | Fragment and TOC preparation | Check the generated fragment, static fragment, and `::toc-linking` with the same four schema spellings. Run references and follow-and-validate graph for TOC, plus compose. Precede the foreign note's fragment or TOC target with the seed inclusion. | Bare loses the generated fragment and fails TOC preparation inside the launch tree. It also fails after the seed for the foreign note. Bare works for the foreign target on its own. Explicit paths pass. Missing fails preparation and the static fragment remains available. | All resolvable spellings generate the heading, including after the seed. Preserve the missing-name and static-fragment outcomes. |
| R8-F1.4 | Composition and report-only shell discovery | Use the same parent forms, four spellings and seed order. Run default compose and disabled-trigger compose on the inclusion forms, and `compose --shell` on both body orders. | Bare fails inside the launch tree and after the enclosing foreign checkout was visited. Opening the foreign note alone or visiting it first passes. Explicit paths work. Missing fails. | The schema resolves the same way in discovery and final composition regardless of visit order. Disabling triggers still keeps bare-name lookup. |
| R8-F1.5 | Named-type import during schema preparation | Replace the schema reference with inline `description: entry@types.yaml`, with the library in each component's `schemas/`. Repeat the direct, reference, fragment, TOC, and composition paths with explicit, bare and missing names. | The same bare-only failures occur for the launch-tree note and the foreign note after the seed. Explicit imports pass. Missing fails where preparation is needed. | Inline imports use the same component root as a standalone schema reference. |
| R8-F1.6 | Trigger discovery and its consumers | Change the topic with an explicit schema path. Check direct schema validation, trigger tracing, and cleaning. Check all three inclusion forms, TOC composition, report-only shell discovery, both foreign visit orders, and the disabled-trigger and passive modes. | When the enclosing repository supplies the context, direct validation, inclusions, TOC composition, and report-only shell discovery report `parent_only`. Cleaning surfaces that property and trigger tracing lists the enclosing trigger. The foreign target alone or first reports `version`. Changing the topic clears the error. Disabled-trigger compose and passive reference and graph checks pass. | Matching frontmatter always uses the component's trigger and reports `version`. Unmatched, disabled-trigger and passive outcomes stay as observed. |
| R8-F1.7 | Bash parameter completion and schema detection | Complete `description=Review` using an enum schema. Infer a schema for an authored `attachment: '&marker.md'` whose file exists only at the component root. Repeat with a missing schema or attachment. | From the enclosing checkout, Bash omits the valid bare schema's value and offers it for the explicit paths. The inferred attachment is `string` for both the real and missing files. From the component, the real attachment is inferred as `file` and the missing one as `string`. | Completion offers the value for all resolvable schema spellings and none for a missing schema. Detection sees the component-root file from either launch directory. |
| R8-F1.8 | Schema-aware CLI editing and render preparation | Run `md set` with an enum value on a temporary copy and render with an explicit, valid style input. Compare bare, explicit and missing schema paths from both directories. | Schema-aware operations from the enclosing directory fail to prepare the bare schema. The same bare input works from inside the component. Explicit paths work and the missing path fails in both locations. `md rm` succeeds for each spelling because it does not prepare a schema. | Both entry points use the same source repository as direct validation. |
| R8-F1.9 | Independent repository discovery and earlier boundaries | Open the launch-tree component note through the DMLS protocol. Compare present and missing schema names. Repeat the iteration 7 sibling-checkout, same-checkout and outside-Git controls through the CLI. | DMLS accepts the resolvable spellings and diagnoses the missing name. The earlier fixtures keep the outcomes in the verification table. | Preserve these outcomes. |

Darkmatter's `source_derivation_for` and the CLI's `document_context` take a successful containment check as proof the document belongs to the request's repository. Darkmatter's `SourceRepositories::containing` also returns the first cached repository whose root contains the source. That second shortcut explains the order dependency. DMLS looks up Git membership from the document folder before reusing a prepared repository context. [Darkmatter source derivation](../../lib/src/markdown/compose/context/options.rs#L175), [Darkmatter repository memo](../../lib/src/markdown/compose/context/request.rs#L512), [DMLS folder lookup](../../dmls/src/context.rs#L262)

The smallest repair is to establish the source's nearest Git root at the existing admission boundary, remember that lookup per source directory, and continue reusing the expensive repository observation by actual root. Apply the same identity rule to direct CLI preparation. Reuse the existing Git-root lookup or its equivalent and keep the current launch scope, permission snapshot, and source-opening rules. One Level 1 fixture with a Git file or nested checkout and a two-order inclusion test can protect both decisions.

## Blocked Findings

None.

## Recurrence

R8-F1 is a newly found sibling of review 7's cross-checkout source-context finding, which followed review 6's outside-Git eligibility finding and review 5's bare schema lookup finding. The launch-tree shortcut predates iteration 7. Iteration 7 added the separate order dependency by caching a repository under every directory it contains. The earlier sweeps tested sibling and outside-Git paths. They would have needed nested roots both inside the launch tree and under an already visited foreign checkout to catch both decisions. Path-based exceptions leave this requirement without a clear stopping point. The recurring failures come from changing which paths the lookup shortcut covers. The repair should anchor the rule to Git membership at source admission and keep the existing downstream consumers. The acceptance checks do not specifically name nested checkouts, so this recurrence is medium. [Review 7 cross-checkout finding](review-7.md#r7-f1-high-documents-opened-through-another-checkout-lose-their-schema-roots), [Review 6 outside-Git finding](review-6.md#r6-f1-high-included-documents-and-fragment-targets-inherit-schema-root-eligibility-from-the-parent), [Review 5 bare lookup finding](review-5.md#r5-f3-high-reference-commands-miss-schemas-named-by-bare-filename), [Record correction](log.md#review-8-record-correction-2026-11-25)

## Input Robustness Matrix

Invalid catalog, manifest and facts inputs produce a located refusal before a callable runs or an invalid registration is published. The shared catalog validator allows an omitted or empty description and quoted `"null"` as type syntax. The manifest allows an empty `files` list with the required lifecycle member and digest. Facts allow explicit null where the binding rule is nullable and empty alias, narrowing-fact and source-exclusion lists. The source-admission change takes already resolved paths and is exercised through the valid inputs in the finding.

| Validator or loader | Level 1 evidence |
| --- | --- |
| Common YAML loader and callable envelope | Both catalog kinds exercise the shared shape validators. The fixtures cover omitted, null, wrong-type and empty fields, invalid collection members, duplicate keys, an additional YAML document, comments, anchors, aliases, BOM and CRLF. CLI and editor tests check invalid overloads, duplicate keys and folded name collisions at the authored positions. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs), [Editor tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Manifest admission and digest inventory | The verified snapshot tests cover required fields, empty and malformed collections, member paths, unsupported versions, and missing, extra or stale digests. The manifest uses the shared YAML loader. Its lenient companion preview adds watch candidates, while scope, digest, and catalog validation decide admission. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs), [Bundle reader](../../lib/src/config/registration/bundle.rs#L653) |
| Facts JSON and binding rules | The public analysis and registration tests exercise omitted, null, wrong-type, mixed and malformed collection values, duplicate keys, trailing content, JSON5 syntax refusal, and positions after non-ASCII text. Claudine's checks cover missing and modified companions and restoration through publication. [Facts shape tests](../../lib/tests/l1/host_functions.rs#L396), [Installed tests](../../lib/tests/l1/host_registration_companions.rs), [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs) |

I checked the admission paths for serde defaults that erase field presence, silently dropped collection members, and discarded parse errors. The manifest's companion preview contributes watch candidates. The strict readers continue to decide whether the host loads, and a required nullable facts field still gets a located error if omitted. [Manifest reader](../../lib/src/config/registration/manifest.rs), [Facts reader](../../lib/src/markdown/compose/expression/host/artifact.rs)

## Verification

| Acceptance requirement | Level and evidence |
| --- | --- |
| Built-in signatures, aliases, read rules and lifecycle contract | Level 1 descriptor, binding, `load_data`, lifecycle operand and artifact parity tests cover the contract. The lifecycle partial still has the baseline Git blob `1f5c316e92f4d4485c4e0ce3cf63c560b265d622`. [Baseline](implementation-log.md#prerequisite-baseline), [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs) |
| Matching CLI and editor errors and private catalogs | Level 1 public parser, CLI and editor protocol tests assert overload and duplicate key positions, folded name collisions, and refusal as a Markdown schema or import library. [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs), [Editor tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Selected host facts, refusal, restoration and runtime boundary | Level 1 installed-loader, CLI and editor tests cover selected and unselected hosts, missing and stale facts, and restoration. Host binding tests execute the registered handler through the host API and CLI tests assert its host-runtime requirement. [Installed tests](../../lib/tests/l1/host_registration_companions.rs), [Editor tests](../../dmls/tests/l1/host_catalog_reload.rs), [Runtime tests](../../lib/tests/l1/host_functions.rs) |
| Shared publication, listing and in-place repair | Level 1 Claudine coverage exercises inventory, listing, publication, doctor, missing or changed companions and the format 2 upgrade. [Inventory tests](../../../claudine/lib/tests/l1/host_publication.rs), [Listing tests](../../../claudine/cli/tests/l1/registrations_command.rs) |
| Mutation registration refusal and existing actions | Level 1 registered mutation refusal at its authored kind, plus event and operand checks for existing lifecycle actions. [Registered catalog tests](../../lib/tests/l1/host_registration_companions.rs), [Lifecycle corpus](../../lib/tests/l1/shipped_schema_corpus.rs) |

Level 1 reaches the required runtime, filesystem, CLI, and headless editor protocol boundaries. The spec has no terminal-rendering or keyboard acceptance criteria.

The Linux rerun of Darkmatter `just test --max-fail all` passed 18,076 tests with 10 skipped. In Simplified Schema, `just test callable_catalog_envelopes::` passed 31 selected catalog tests and all 2,202 minimal-feature tests. Claudine's focused `just test` run passed 69 publication, listing, upgrade, and artifact tests. No test reached five seconds. `just check-tier-coverage darkmatter simplified-schema claudine` reported zero stranded tests.

The first Darkmatter run stopped on a malformed `ready` declaration in the concurrently edited `schemas/specification.yaml`. The worktree version now uses `->` and passed direct validation and the two shipped-corpus tests in the rerun. This review did not change that file.
