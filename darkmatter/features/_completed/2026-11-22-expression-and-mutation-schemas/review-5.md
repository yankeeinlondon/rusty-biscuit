---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-5.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-4.md"
next: "2026-11-22-expression-and-mutation-schemas/review-6.md"
created: "2026-11-25T02:43:15+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R5-F1 (high): The kind guide's expression catalog uses a reserved name"
    - "R5-F3 (high): Reference commands miss schemas named by bare filename"
    - "R5-F2 (low): The category guide misstates which catalogs require categories"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "claude/opus"
---

# Expression and mutation schemas review 5

The feature needs three repairs. Reference checks and graph commands can reject a valid review because they miss its schema by bare filename. The repository kind guide also contains an expression catalog that fails when copied, and the Simplified Schema guide describes required catalog categories as optional in its general category section. All previous repairs remain verified, including the restored kind schema and the checks that detect its deletion.

## Previous Finding Verification

The iteration 4 rows below follow the repair ledger in order. I compared the restored bytes to the committed files, ran the CLI against the worktree, reran the public tests, and checked the raw missing-file controls. For earlier findings I checked the relevant assertions, compared them with the prior witness runs, and reran the affected parser, CLI, editor, registration, and inventory tests on Linux. No earlier review has a human ruling. [Iteration 4 repair ledger](log.md#implementation-of-review-findings-4)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R4-F1 The repository kind schema is missing from the worktree | R4-F1.1–.12, detailed below | **verified fixed** | The restored schema, both required-file checks, all consumers, and the test selection are verified below. | None. The guide example and the standalone reference commands exposed separate defects while the schema was present. |
| R3-F1 YAML anchors and aliases break overload validation in sequences | R3-F1.1–.19 | **verified fixed** | The Simplified Schema tests cover block, flow, whole-value and repeated aliases, tags, quoted commas and later-arm positions (.1–.3, .5, .7, .12–.15). Fresh CLI, DMLS, installed and compose tests cover admission, duplicates, categories and helper positions (.4, .8–.11). The five inventory and doctor cases pass (.6). The schema source and editor suites pass tagged, indentless and fallback controls (.16–.19). [Parser tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [Compose witnesses](../../cli/tests/l1/compose_host_scope.rs), [Editor witnesses](../../dmls/tests/l1/schema_yaml_spellings.rs), [Doctor witnesses](../../../claudine/lib/src/doctor/registration_tests.rs) | None |
| R2-F1 Host registrations accept names that Markdown cannot call | R2-F1.1–.11 | **verified fixed** | Fresh tests verify lexer-compatible names in the catalog, Rust builder and facts admission. Dash, underscore, dot and case controls remain valid. Refused names and aliases retain their positions and contribute no callable in compose or completion. [Library witnesses](../../lib/tests/l1/host_functions.rs), [Registration witnesses](../../lib/tests/l1/host_registration_companions.rs), [Review 3 evidence](review-3.md#previous-finding-verification) | None |
| R2-F2 Type-reference errors can point to unrelated text | R2-F2.1–.12 | **verified fixed** | Fresh catalog, editor, and installed tests exercised missing helpers, external references, callable-as-type, and misleading text in comments, prior arms, or keys. The range follows the actual reference or declaration and keeps the competing callable key when applicable. [CLI witnesses](../../cli/tests/l1/schema_validate_yaml.rs#L800), [Installed witnesses](../../lib/tests/l1/host_registration_companions.rs#L276), [Review 3 evidence](review-3.md#previous-finding-verification) | None |
| R2-F3 The no-terminal approval test exceeds the Windows L1 budget | R2-F3.1–.2 | **verified fixed** | The final Windows package transcript records 2.878 s for the no-terminal case. The test still uses one invocation and checks refusal and persistence. The in-process siblings retain their assertions. [Approval tests](../../cli/tests/l1/approval_prompt.rs), [Timing evidence](review-3.md#previous-finding-verification) | None |
| R2-F4 The Claudine trigger discovery test exceeds the Windows L1 budget | R2-F4.1–.4 | **verified fixed** | The final Windows package transcript records 2.412 s. The batch checks both document verdicts and applied trigger counts. The separate trace witness and seven verdict and diagnostic cases remain. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Timing evidence](review-3.md#previous-finding-verification) | None |
| R2-F5 The recursive lifecycle corpus test exceeds the Windows L1 budget | R2-F5.1 | **verified fixed** | The final Windows package transcript records 2.030 s. The batch retains 19 verdicts and the depth-six trigger, location and deferred-operand checks. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Timing evidence](review-3.md#previous-finding-verification) | None |
| R2-F6 The depth-four lifecycle test exceeds the Windows L1 budget | R2-F6.1 | **verified fixed** | The final Windows package transcript records 2.467 s. The batch retains both trigger checks, four valid documents and three invalid action positions. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Timing evidence](review-3.md#previous-finding-verification) | None |
| R1-F1 Inline YAML comments make valid catalogs fail | R1-F1.1–.8 | **verified fixed** | The parser, CLI, editor and installed control runs preserve plain, flow, and block declarations with comments. Literal hashes, BOM and CRLF remain valid, and a second document still fails. The language guide's commented helper now also validates. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L417), [Review 2 evidence](review-2.md#previous-finding-verification) | None |
| R1-F2 Catalog diagnostics lose declaration locations | R1-F2.1–.15 | **verified fixed** | Fresh suites cover overload, category and folded-name conflicts, both facts claim orders, built-in embedded and checkout coordinates, and the CLI, editor, installed and doctor projections. The standalone kind registry retains the located undeclared-category refusal. [CLI witnesses](../../cli/tests/l1/schema_validate_yaml.rs#L878), [Installed witnesses](../../lib/tests/l1/host_registration_companions.rs#L529), [Review 2 evidence](review-2.md#previous-finding-verification) | None |
| R1-F3 The shared review and fix schema cannot validate documents | R1-F3.1–.5 | **verified fixed** | Both review arms and required-field controls pass the public tests. The four review-related schemas pass the CLI and both corpus checks. The reserved specification placeholder now has a schema and validates. [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs), [Review 3 evidence](review-3.md#previous-finding-verification) | None |
| R1-F4 Registration and catalog references still describe the old formats | R1-F4.1–.8 | **verified fixed** | The named maintainer pages and publication, inventory, and setup witnesses agree on format 3. The corpus treats the catalogs as private. Remaining format 2 references describe cleanup or upgrade. [Review 2 evidence](review-2.md#previous-finding-verification), [Registration guide](../../docs/topics/host-registration.md) | None of the original instances. R5-F1 and R5-F2 concern other guidance. |
| R3-V1 The shipped-schema corpus retains its required suggestion-review schema | Required presence and ordinary resolution | **verified fixed** | Both corpus tests require the path and passed. The direct CLI also validated the schema. [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs) | None |

| Iteration 4 instance | Verified result |
| --- | --- |
| R4-F1.1 | The public Markdown regression and CLI admit `expression-schema` and `mutation-schema`. Changing just the value to `unsupported-catalog` gives the enum error at line 3, column 7. |
| R4-F1.2 | `md compose docs/kind-documents.md --deny all --output markdown` returns the expanded enum with both values. The prompt's direct reference passes independently. |
| R4-F1.3 | The resolution and lowering test passes in 1.215 s. Its recorded deletion control fails at the explicit assertion naming `schemas/kind.yaml`. |
| R4-F1.4 | Both built-in catalogs pass CLI validation through their shipped definitions. Fresh registry tests passed. |
| R4-F1.5 | The feature-review, review-fix-cycle, specification and suggestion-review schemas validate directly. Both corpus tests require each file, and the public Markdown test exercises the two review-cycle branches and invalid controls. |
| R4-F1.6 | Both corpus entry points require the same six physical files. The root classification test still checks the envelope of each discovered file. [Corpus](../../lib/tests/l1/shipped_schema_corpus.rs), [Classification tests](../../lib/tests/l1/meta_schema_repo_schemas.rs) |
| R4-F1.7 | A fresh scan using Cargo's declared L1 targets returned both exact corpus filters for all six files and the exact Markdown regression for the kind schema while the scanner's reader modeled that file as deleted. |
| R4-F1.8 | The passive corpus test passed in 0.208 s, including its effect and write assertions. Its recorded deletion control fails at the explicit assertion naming the missing kind schema. |
| R4-F1.9 | The documented named-type import accepts evidence format 3 and rejects format 4 at `/evidence/schema_version`, line 5, column 19. Both independent recorded controls fail at the assertion naming a missing `schemas/ci-evidence.yaml`. The current file matches the committed bytes. |
| R4-F1.10 | All three root classification and feature-review tests passed. The actual previous review also validates through the bare `feature-review.yaml` reference. |
| R4-F1.11 | `md validate refs system-prompt.md --show-all --output plain` reports five working references, including `^schemas/kind.yaml` at line 30, column 100. |
| R4-F1.12 | Direct CLI validation of the two shipped kind definition files accepted each against the shipped `yaml-schema.yaml` definition. |

## Unblocked Findings

### R5-F1 (high) The kind guide's expression catalog uses a reserved name

**Authority (defect).** The repository kind guide presents a complete standalone expression catalog to illustrate the new format. Copying that block to a YAML file and running `md schema validate` fails because it declares `kebab_case`, a name Darkmatter owns. The example appears immediately before the guide's instructions about catalog validation and host registration. The specification's built-in migration also requires updated language guidance and an example of a host's declaration. [Repository kind guide](../../../docs/kind-documents.md#L64), [Built-in migration](spec.md#built-in-migration)

The defect class is a copyable authoring example that violates the same admission rules it describes. Adding a comment preserves the verdict, and changing only the authored callable name to the language guide's `project_slug` makes the same two-overload contract pass. Changing only the capitalization leaves the name reserved. I extracted each complete callable catalog in the feature's current guides and passed it to the repository-built CLI. The public validation behavior is correct and already tested through standalone CLI, editor and installed registration. [CLI collision test](../../cli/tests/l1/schema_validate_yaml.rs#L974), [Editor catalog tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs), [Installed collision test](../../lib/tests/l1/host_registration_companions.rs#L529)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R5-F1.1 | Repository kind guide | Extract the fenced YAML. Repeat with a helper comment, `KEBAB_CASE`, and `project_slug`. | The original, comment and case variants exit 1 at line 7, column 5, with a related built-in declaration at line 312, column 5. Renaming just the key to `project_slug` exits 0. | The published block and its comment-only variant validate. Replacing the corrected key with a reserved name triggers the existing two-location error. |
| R5-F1.2 | Simplified Schema callable catalog guide | Extract its complete expression catalog with `Label`, `project_slug`, the inline comment, and both overloads. | Exits 0. | Retain the valid authored overload and helper example. |
| R5-F1.3 | Darkmatter host function guide | Extract its `is_project_file` expression catalog and its `set_frontmatter` mutation catalog. | Both exit 0. The installed mutation-kind refusal also passed its test. | Both examples validate in isolation, with host registration still refusing mutation catalogs. |
| R5-F1.4 | Darkmatter host registration guide | Extract the catalog for `./functions/project.yaml`. | Exits 0 with its narrowing clause and category intact. | Retain the valid expression declaration that the manifest example refers to. |
| R5-F1.5 | Caller skill guidance and actual built-in catalogs | Inspect the canonical and published caller skill fragments for complete callable YAML examples. Validate both shipped catalogs from their repository paths. | The caller skills link to the guidance and contain no separate complete callable YAML catalog. Both built-ins validate at their repository paths. | Keep the actual built-in name and the caller guides' references. |

Use `project_slug` in the repository kind guide block, as in the language guide. Keep the helper and both overloads. Validate the extracted block and compose the guide again. This repair needs only the example key. [Passing language-guide example](../../../simplified-schema/docs/topics/language/schema-documents.md#L452)

### R5-F2 (low) The category guide misstates which catalogs require categories

**Authority (spec).** Catalog format and validation says “Both `categories` and `types` are required.” The general Categories section of the Simplified Schema guide says “Every envelope may also declare a `categories:` list” and goes on to say that omission and null are equivalent and numeric entries become text. The callable section links readers to this discussion. [Catalog format and validation](spec.md#catalog-format-and-validation), [Categories guide](../../../simplified-schema/docs/topics/language/schema-documents.md#categories)

The CLI admitted both unchanged host-guide examples and refused each callable kind when the categories were omitted, set to null, or supplied as a number in the list. The callable-specific table already gives the correct rule. Qualify the optional and soft-conversion rules in the general section by envelope and point readers to the callable requirement there. Keep the shared label and description syntax in that section. [Callable envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs#L190)


### R5-F3 (high) Reference commands miss schemas named by bare filename

**Authority (defect).** Running `md validate refs` on the actual iteration 4 review exits with an unresolved `$schema: feature-review.yaml` before reporting links. `md schema validate` accepts the same review and `md compose` can prepare it. Review documents in this repository use this spelling to select the shared schema. A reference check or graph for one of these documents should resolve the same schema before checking its content. [Previous review](review-4.md), [Reference command](../../cli/src/commands/validate.rs#L32), [Schema lookup](../../docs/topics/schemas/definition.md#schema-roots)

The defect class is loss of schema-root lookup in standalone reference preparation. For the same valid document, a bare name and an absolute path naming the same schema should give the same result. Changing the name to one that does not exist should still fail wherever preparation is required. For fragments, the successful control should find a heading generated from declared frontmatter. I used a small Markdown review beside the real one with one local link, then included it from a second document and linked to a generated heading from a third. I also tried the same spelling changes on the named-type import in the CI evidence example.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R5-F3.1 | `md validate refs` report | Bare, absolute and missing review schema on the root document | Bare and missing exit 1 with unresolved schema. The absolute spelling exits 0 and checks one link. | Bare and absolute each check the link. The missing schema still fails preparation. |
| R5-F3.2 | `md validate refs --graph mermaid` | Same root fixture | Bare and missing exit 1. Absolute produces the graph. | Both valid names produce the graph. Missing still fails. |
| R5-F3.3 | `md graph` with and without `--validate` | Same root fixture | Bare and missing fail both modes. Absolute produces the tree or reports one working link. | Both valid names produce the same tree and validation verdict. Missing still fails. |
| R5-F3.4 | Inclusion preparation under `md validate refs`, its Mermaid mode, and `md graph --follow --validate` | Root includes the same review via `::file` | All three fail in the included review for the bare and missing variants. Absolute succeeds and the validators check two references across two documents. | Both valid names prepare the inclusion. Missing still identifies the child. |
| R5-F3.5 | Fragment validation of an external Markdown link target | Link to `#review-heading`, produced by a heading that reads `fm.description` | The bare and missing variants report no matching heading. Absolute succeeds. A static heading resolves even in the missing variant through the existing fallback. | Both valid names prepare the target and find the generated heading. Preserve the missing-schema fallback for a static heading. |
| R5-F3.6 | A named-type import in standalone reference preparation | The evidence sample imports `observation_collection@ci-evidence.yaml`, then names the same file absolutely, then names a missing file | `md validate refs` and `md graph --validate` accept only the absolute import. | Accept the bare and absolute imports. Missing still fails. |
| R5-F3.7 | Schema validation and compose controls | The same review, inclusion, and evidence shapes | `md schema validate` accepts both valid root and import spellings and refuses the missing ones. `md compose --deny all` accepts both valid review spellings at the root and in the inclusion and refuses the missing ones. | Preserve these results. |

Darkmatter's [`prepare_schemas`](../../lib/src/markdown/compose/schema_validation.rs#L409) assembles the schema used while preparing references. Its bare-name search uses the roots provided by the trigger registry. The standalone reference and graph commands supply a document context but omit that registry, while compose supplies it in its default mode. Use the existing schema-root selection for authored schema references in standalone reference preparation. Keep its effect and trigger policy, and exercise the shared preparation with one root case, an inclusion, and a generated-heading link. [Schema setup](../../lib/src/markdown/schemas/mod.rs#L1015), [Graph command](../../cli/src/commands/graph.rs#L30), [Shared preparation](../../lib/src/markdown/reference/graph.rs#L1256)

## Recurrence

The two documentation findings belong to the documentation drift class in **review 1, R1-F4, Registration and catalog references still describe the old formats**. They are newly found siblings. Review 1 also recorded the reserved-name problem in the Simplified Schema example as an informational implementation decision. That example now passes. The earlier sweep checked the registration descriptions, corpus classification, and the language guide's local example. It missed the repository kind guide's parallel YAML block and the general Categories section linked by the callable guide. The present sweep checked the four guides' standalone catalog blocks and the corresponding catalog metadata guidance. These documentation fixes keep catalog admission as designed. R5-F3 has a separate cause from the file deletion in review 4. The file now exists, and only the standalone reference preparation omits the search location. [Review 1 findings and decisions](review-1.md#r1-f4-low-registration-and-catalog-references-still-describe-the-old-formats)

## Input Robustness Matrix

Invalid inputs receive a located refusal before evaluation or publication. Catalog descriptions may be omitted or empty. Catalog category and type collections need declarations. A manifest may have an empty `files` list with its lifecycle member and digest intact. Facts use explicit null for the nullable rules when unset, and alias, narrowing fact and source exclusion lists may be empty.

| Validator or loader | Level 1 evidence |
| --- | --- |
| Catalog YAML and metadata | The public envelope tests cover absent, null, wrong whole-field type, invalid elements and empty collections. Both kinds share validation. Direct CLI controls accepted the original project fixture and a comment-only edit, and located bad metadata, a blank category, a duplicate key and a second document. The envelope and CLI/editor suites exercise CRLF, BOM, tags and aliases, trailing content, conflicts and passive examples. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs) |
| Manifest YAML, member paths and digests | Public snapshot cases exercise required fields, version and shape refusals, invalid entries, the empty-list control, duplicate and trailing documents, digest mismatch, missing bytes, path boundaries and derived companions. The cleanup decoder has its own valid and malformed format 2 controls. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs) |
| Facts JSON and binding rules | Public host and installed-companion tests exercise absent, explicit null, wrong field and element types, empty collections, duplicate keys and trailing content. Installed tests also assert exact locations for stale hashes, invalid names and JSON5 syntax rejected by the JSON reader. [Host function tests](../../lib/tests/l1/host_functions.rs), [Installed companion tests](../../lib/tests/l1/host_registration_companions.rs) |
| Reads that could erase invalid shapes | The catalog's strict authored metadata checks run alongside kind validation. The strict manifest checks reconcile digests before exposing a profile. Its permissive companion preview supplies watch candidates. The facts decoder requires fields and projects typed failures. The separate raw catalog read supplies example display spelling after admission. |

## Verification

| User-facing requirement | Appropriate level and evidence |
| --- | --- |
| Built-in signatures, aliases, binding, and data access | Level 1. Darkmatter descriptor/registration parity and permission tests exercise binding and `load_data` read rules. Both shipped catalog files passed direct CLI validation. |
| Lifecycle action forms, event limits, unchanged generated contract | Level 1. Lifecycle generation, schema parity and event-binding tests passed. The current lifecycle schema matches the recorded pre-feature Git blob. Recent scoped generator checks passed in Simplified Schema, Darkmatter and Claudine. |
| Catalog validation, diagnostics, privacy and passivity | Level 1. Public parser and resolver tests cover both private kinds, schema and import refusals, and passive examples. CLI and headless editor cases assert locations, related declarations, and buffer refresh. |
| Selected host binding, companion changes and runtime behavior | Level 1. Installed admission, compose and editor tests distinguish wrong arguments, facts-based source exclusions, valid calls requiring the host, unselected documents, and missing, stale and restored companions. The declared host runtime fixture exercises successful handler execution and return validation, including calls in action operands. |
| Publication, listing, repair and upgrade | Level 1. Claudine's inventory, publication, setup and doctor witnesses cover exact bytes, companion refusal and repair, unsupported format, in-place upgrade, and repeat no-op. |
| Mutation admission and existing actions | Level 1. Installed registration refuses the mutation kind at its field. Runtime and event tests still exercise built-in operand validation and the distinction between ordinary expressions and action keys. |
| Kind guidance and authoring examples | Level 1. The kind guide expands, the prompt link works, and Markdown validates using the shared schema. Extraction and validation of the guide examples revealed R5-F1. The categories probes confirm R5-F2 concerns the written guidance. Standalone reference preparation fails the bare-name cases in R5-F3. |

The fresh Linux runs passed 13,124 Darkmatter library tests, 157 Darkmatter tests across the library, CLI and editor, 131 Simplified Schema default-feature tests and 2,202 minimal-feature tests, and 112 Claudine tests. No selected test reached five seconds. The preceding full Darkmatter transcript has 18,038 passes with zero slow tests, and the preceding full Simplified Schema and Claudine transcripts also show zero slow tests. The tier audit found zero stranded tests across all three areas. These outcomes are API results, files, CLI reports, or editor protocol diagnostics. Cross-OS validation remains with CI.

The implementation record's statement that the worktree still lacked the kind schema was corrected in place, with a dated note preserving the original observation and recording the current CLI and corpus evidence. [Implementation record](implementation-log.md#final-checkpoint)
