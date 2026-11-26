---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-7.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-6.md"
next: "2026-11-22-expression-and-mutation-schemas/review-8.md"
created: "2026-11-25T04:42:32+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R7-F1 (high): Documents opened through another checkout lose their schema roots"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "claude/opus"
---

# Expression and mutation schemas review 7

The iteration 6 repair is verified for all seven required instances. A separate case blocks readiness. A note in a second checkout resolves its own schema when opened directly, but an inclusion or heading link to that same note can lose its repository context. No author decision is required.

## Previous Finding Verification

The iteration 6 ledger defines the seven required instances. I ran each through the worktree CLI with an isolated home and checked the corresponding Level 1 library and CLI tests. Earlier findings remain closed. No previous review contains an author ruling. [Iteration 6 ledger](log.md#implementation-of-review-findings-6)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R6-F1 Included documents and fragment targets inherit schema-root eligibility from the parent | R6-F1.1–.7, expanded below | **verified fixed** | Direct and nested checks agree for the tested note outside Git. The repository child and the scoped eligibility test passed. [Library tests](../../lib/tests/l1/reference_bare_schema_roots.rs), [CLI tests](../../cli/tests/l1/reference_bare_schema.rs) | None of the seven instances. R7-F1 covers a destination inside a separate checkout. |
| R5-F1 The kind guide's expression catalog uses a reserved name | R5-F1.1–.5 | **verified fixed** | The guide uses `project_slug`. The built-in name and alias checks still assert the conflicting declaration. The accepted examples and published caller guidance were verified in review 6. [Kind guide](../../../docs/kind-documents.md), [Prior verification](review-6.md#previous-finding-verification) | None |
| R5-F2 The category guide misstates which catalogs require categories | Guide, maintainer skill, expression and mutation controls | **verified fixed** | The guide scopes optional categories to ordinary schema envelopes. The two strict callable envelope cases passed the category-shape tests. [Guide](../../../simplified-schema/docs/topics/language/schema-documents.md#categories), [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs) | None |
| R5-F3 Reference commands miss schemas named by bare filename | R5-F3.1–.7 | **verified fixed** | The in-repository root, child, fragment, named import, and disabled-trigger cases passed. Review 6 separately checked the actual review and evidence documents. [Library tests](../../lib/tests/l1/reference_bare_schema_roots.rs), [Prior verification](review-6.md#previous-finding-verification) | None of the original instances. |
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

| Iteration 6 instance | Verified result |
| --- | --- |
| R6-F1.1 | `md validate refs` and its Mermaid graph refused bare and missing schema names in the external note under both a Git parent and a parent outside Git. Absolute and explicit relative names passed. |
| R6-F1.2 | `md graph`, `md graph --validate`, and `md graph --follow --validate` gave the same verdicts and named the unresolved schema on failure. |
| R6-F1.3 | The explicit spellings generated a heading that passed fragment validation. Bare and missing schema names lost the generated heading. The static heading passed in all four cases. |
| R6-F1.4 | The reference report, follow-and-validate graph, fragment check, composition with triggers disabled, and direct validation accepted explicit named-type imports. All refused the bare and missing variants. |
| R6-F1.5 | With triggers enabled and disabled, composition rendered the included note's heading for the explicit spellings and failed with the bare and missing names. |
| R6-F1.6 | Direct references, schema validation, clean, and compose accepted the explicit spellings and refused bare and missing. Bash completion offered the enum value only for explicit spellings in the external note. The in-repository bare-name tests passed. |
| R6-F1.7 | The matched and unmatched external note composed through the parent and directly. Reference and graph checks also passed. For an in-repository child the same trigger required `version` on the matching topic, and disabling triggers or changing the topic restored success. |

## Unblocked Findings

### R7-F1 (high) Documents opened through another checkout lose their schema roots

**Authority (defect).** A developer can open a note in a separate local Git checkout using `~/notes/review.md`. A direct check finds the note's `$schema: review.yaml` in that checkout's `schemas/` directory. Include the same note with `::file ~/notes/review.md` and reference validation and composition refuse that schema. Heading links can fail for the same reason. With an explicit schema spelling, inclusion can also skip a matching trigger from the note's checkout and accept frontmatter that direct composition refuses. [Document context in the Darkmatter CLI](../../cli/src/request.rs#L335), [Schema preparation in Darkmatter](../../lib/src/markdown/compose/schema_validation.rs#L441)

The defect class is missing source repository context after a file crosses an admitted document boundary. The invariant is that a source's repository determines its schema roots. Switching between a bare name and explicit paths to the same schema should preserve the verdict for a source inside Git. Switching to a missing schema must fail preparation. A trigger from the child checkout must affect matching frontmatter in normal composition, and changing the matched field must remove that effect.

To reproduce, create separate Git checkouts at `~/app` and `~/notes` in an isolated home. Put a schema with a required string `description` in `~/notes/schemas/review.yaml`. From `~/app`, check `~/notes/review.md` directly and through a parent containing `::file ~/notes/review.md`. For the fragment probe, link to `~/notes/review.md#review-heading`. Use this note.

```markdown
---
$schema: review.yaml
description: Review heading
topic: other
---

# {{ fm.description }}

## Static
```

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R7-F1.1 | Reference report and Mermaid graph | Body inclusion, `prologue:` and `epilogue:` pointing to the same note. Run both reports for bare, relative, absolute and missing schema names. | All three parent forms accept relative and absolute. Bare and missing exit 1 with the unresolved schema. | Bare, relative and absolute all succeed. Missing continues to fail at the child. |
| R7-F1.2 | Standalone graph and its validation and follow modes | Same three parents, each with plain `md graph`, `--validate` and `--follow --validate`. | All nine combinations accept the explicit spellings and exit 1 for bare and missing. | All nine accept bare and explicit spellings and refuse missing. |
| R7-F1.3 | Fragment preparation and `::toc-linking` | Check the generated and static headings. Check a `::toc-linking ~/notes/review.md` parent with the reference report, follow-and-validate graph, and compose. | Explicit spellings pass. Bare and missing lose the generated heading and fail preparation for the TOC target. The static fragment passes. | Bare and explicit spellings generate the heading and allow the TOC target to prepare. Keep the missing-name failure and static fallback. |
| R7-F1.4 | Composition and command discovery before composition | Compose each inclusion form in default and disabled-trigger modes. Run report-only `md compose --shell root.md` on the body inclusion. | Normal compose and the report accept explicit spellings. Bare and missing exit 1 naming the child's schema. | Compose and the report accept bare and explicit spellings in each tested mode. Missing continues to fail. |
| R7-F1.5 | Named-type import read during preparation | Give the same note an inline schema with `description: entry@types.yaml`, with `entry: string` in `~/notes/schemas/types.yaml`. Repeat the direct, reference report, graph, composition, command discovery, fragment, and TOC checks with bare, relative, absolute, and missing library names. | Direct checks accept bare and explicit imports. Every nested preparation requiring the schema fails on bare as well as missing, including heading generation. Both explicit spellings work. The static fragment still passes. | Bare and explicit imports resolve in both positions. Missing still fails wherever preparation is needed. |
| R7-F1.6 | Root trigger selection during composition | Use the explicit schema. Add a trigger in the note's checkout that matches `topic: review` and requires `version`. Add a matching trigger in the parent checkout that would require `parent_only`. Flip the note's topic to `other`. | Direct schema validation and compose refuse the matching note for missing `version`. Clean surfaces the same problem. All parent inclusion forms compose successfully for either topic, with and without triggers. Reference and graph checks pass for both topics. | Default compose of a parent refuses the matching child for missing `version` and accepts `other`. No child error should name `parent_only`. Disabled-trigger composition and passive reference and graph checks continue to pass. |
| R7-F1.7 | Direct document context and shell completion | Open the note directly from the first checkout. Check schema validation, clean, compose, references, graph, and Bash completion with an enum in the schema. | Clean control. The direct commands accept all three resolvable spellings and refuse missing. Completion offers the value for the resolvable spellings only. | Keep the direct behavior. |
| R7-F1.8 | Existing source boundaries | Repeat with a resident child in the same checkout and with the iteration 6 note outside Git. | Clean controls. The first gets its bare lookup and root triggers. The second only gets the explicit schema resolution and retains the static fragment fallback. | Keep both boundary behaviors. |

The Darkmatter CLI's [`document_context`](../../cli/src/request.rs#L335) prepares a context at the source directory when the directly opened file is outside the launch repository. Darkmatter's library [`with_accepted_source_file`](../../lib/src/markdown/compose/context/request.rs#L670) only derives from the request's captured context. When that context contains just the first checkout, the derived source has no repository. The shared [`prepare_schemas`](../../lib/src/markdown/compose/schema_validation.rs#L412) then disables both root selectors. Inclusion and command discovery use the accepted-source preparation, and the reference graph uses it for inclusion nodes, TOC targets, and fragment targets. The editor has its own workspace context and its YAML validation suite passed.

Reuse the existing per-repository context capture for an already admitted file in another checkout, and make the result available to each consumer of that source. Capture it at the request's source-opening boundary and reuse it for preparation, with the launch `@` scope and the original permission snapshot preserved. A new probe in each validator would add repeated discovery and let the same file receive different results as it moves through the request. A two-checkout Level 1 fixture can drive both schema selectors and the direct, inclusion, and fragment paths. The existing one-checkout and outside-Git fixtures already protect those two cases. [Accepted source options](../../lib/src/markdown/compose/context/options.rs#L1264), [Reference graph source preparation](../../lib/src/markdown/reference/graph.rs#L468), [Fragment preparation](../../lib/src/markdown/reference/validate.rs#L828)

## Recurrence

R7-F1 is a newly found sibling of review 6's R6-F1 and review 5's R5-F3. Those sweeps covered a target in the captured checkout and a target outside Git. They did not distinguish an outside-Git file from an admitted file in a second checkout whose repository has yet to be captured. The seventh review checks that distinction through bare schema selection, named imports, trigger selection, command discovery, graph and fragment preparation, and final composition. The iteration 6 completion report now records the narrower verified result. [Review 5](review-5.md#r5-f3-high-reference-commands-miss-schemas-named-by-bare-filename), [Review 6](review-6.md#r6-f1-high-included-documents-and-fragment-targets-inherit-schema-root-eligibility-from-the-parent), [Record correction](log.md#review-7-record-correction-2026-11-25)

## Input Robustness Matrix

Invalid catalog, manifest, and companion shapes produce a located refusal before a callable runs or a registration is published. An omitted or empty catalog description is allowed. An explicitly empty manifest `files` list still requires the lifecycle member and digest. Explicit null is accepted for the nullable facts rules. Empty alias, narrowing-fact, and source-exclusion lists are valid. Catalog categories and types must be nonempty. R7-F1 concerns context supplied to valid inputs.

| Validator or loader | Public Level 1 evidence |
| --- | --- |
| Catalog YAML and common field rules | The valid fixtures, category and type shape tests, and duplicate, trailing document, comment, tag, alias, BOM, and CRLF cases exercise the common reader. The CLI and editor cover the normal entry points and compare the authored positions for overload, duplicate key, and name collisions. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs), [Editor tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Manifest YAML, paths, and digest inventory | The verified snapshot tests exercise explicit empty files, absent fields, null and wrong collection shapes, unsupported versions, path checks, and missing, extra, or stale digests. The manifest uses the same YAML document loader. Its early classification for companion discovery cannot admit an invalid member as a usable catalog. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs), [Bundle reader](../../lib/src/config/registration/bundle.rs#L653) |
| Facts JSON and binding fields | The public registration result distinguishes absent fields, null, wrong types, invalid elements, and explicit empties. Loader cases include duplicate keys, trailing content, JSON5 comments and commas, and positions after non-ASCII text. Claudine exercises the corresponding publication and repair path. [Facts tests](../../lib/tests/l1/host_registration_companions.rs), [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs) |

I checked the admission paths for serde defaults, dropped elements, and discarded parse failures. The manifest's lenient companion preview only contributes watch candidates. The strict digest and catalog preparation checks still control admission. The facts reader records locations while validating each field, including a located error when a required nullable field is omitted. [Manifest reader](../../lib/src/config/registration/manifest.rs), [Facts reader](../../lib/src/markdown/compose/expression/host/artifact.rs)

## Verification

| Acceptance requirement | Level and evidence |
| --- | --- |
| Built-in signatures, aliases, read rules, and lifecycle contract | Level 1 descriptor and runtime binding tests, `load_data` checks, generated action operand checks, and shipped artifact parity. The lifecycle partial retains the pre-feature Git blob `1f5c316e92f4d4485c4e0ce3cf63c560b265d622`. [Baseline](implementation-log.md#prerequisite-baseline), [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs) |
| Matching editor and CLI errors, and private passive catalogs | Level 1 tests exercise the public parser and resolver, CLI locations, and the editor protocol. They cover invalid overloads, duplicate YAML keys, folded name collisions, and catalog refusal as a frontmatter schema or import library. [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs), [Editor tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Selected host facts, refusal, restoration, and runtime boundary | Level 1 installed-loader, CLI, and editor tests exercise selected and unselected hosts, missing and stale companions, and restoration. Host binding tests exercise invocation and the CLI exercises the host-runtime-required refusal. [Installed tests](../../lib/tests/l1/host_registration_companions.rs), [Editor reload tests](../../dmls/tests/l1/host_catalog_reload.rs), [Runtime tests](../../lib/tests/l1/host_functions.rs) |
| Publication, listing, doctor, and in-place upgrade | Level 1 Claudine tests passed the shared inventory, listing and publication, missing and modified companions, and format 2 refusal and in-place repair. [Listing tests](../../../claudine/cli/tests/l1/registrations_command.rs), [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs) |
| Mutation restriction and existing action checks | Level 1 installed registration refuses `mutation-schema` at its kind. The lifecycle parity, operand, and event tests continue to exercise built-in actions. [Installed catalog tests](../../lib/tests/l1/host_registration_companions.rs) |
| Schema selection when a Markdown file is reached from another file | Level 1 library and CLI tests prove the repaired outside-Git case. Direct Level 1 CLI probes show the second-checkout defect in R7-F1. |

Linux verification ran Darkmatter `just test` (18,053 passed, 10 skipped), Simplified Schema `just test` (2,626 default-feature and 2,202 minimal-feature tests passed, zero skipped), and Claudine `just test` narrowed to publication, listing, repair, and shipped-artifact coverage (71 passed). No test reached five seconds. `just check-tier-coverage darkmatter` reported zero stranded tests. Level 1 reaches the required CLI, filesystem, and editor protocol boundaries. The spec has no terminal-rendering or keyboard behavior to verify.
