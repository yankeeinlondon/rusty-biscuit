---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-6.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-5.md"
next: "2026-11-22-expression-and-mutation-schemas/review-7.md"
created: "2026-11-25T04:11:52+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
implemented-by: "claude/opus"
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
findings:
    - "R6-F1 (high): Included documents and fragment targets inherit schema-root eligibility from the parent"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
---

# Expression and mutation schemas review 6

The three iteration 5 repairs pass their original checks. One related defect still blocks readiness. A Markdown document outside Git can resolve a bare schema name when a repository document includes it or checks one of its headings, even though checking the external document directly refuses that name. The repair can use the existing document context, so it needs no author decision.

## Previous Finding Verification

I checked every iteration 5 ledger row against the current worktree. All earlier findings were closed before iteration 5. Their verification below uses the earlier instance evidence, current assertions, and fresh Linux runs where applicable. No earlier review contains an author ruling. [Iteration 5 finding ledger](log.md#implementation-of-review-findings-5)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R5-F1 The kind guide's expression catalog uses a reserved name | R5-F1.1–.5 | **verified fixed** | The corrected guide and a comment-only copy validate. Changing the name to `KEBAB_CASE` is refused at 7:5 with the built-in declaration at 312:5. I extracted and validated the Simplified Schema guide, both host function guide blocks, and the registration guide block. The canonical and published caller skills point to guidance without duplicating a complete catalog. Both built-in catalogs validate from their checkout paths. The kind guide composes with `--deny all`. [Kind guide](../../../docs/kind-documents.md), [Schema language guide](../../../simplified-schema/docs/topics/language/schema-documents.md#callable-catalogs-kind-expression-schema-and-kind-mutation-schema), [Host function guide](../../docs/topics/host-functions.md), [Registration guide](../../docs/topics/host-registration.md#expression-catalogs) | None |
| R5-F2 The category guide misstates which catalogs require categories | General Categories section, maintainer skill note, both callable-kind controls | **verified fixed** | The section and skill qualify the lenient behavior by envelope. Both unchanged callable examples pass. Both refuse an omitted field, null, an empty list, a numeric field, an all-numeric list, and a mixed list with a numeric entry. [Categories guide](../../../simplified-schema/docs/topics/language/schema-documents.md#categories), [Maintainer skill](../../../.claude/skills/simplified-schema/schema-documents.md) | None |
| R5-F3 Reference commands miss schemas named by bare filename | R5-F3.1–.7, detailed below | **verified fixed** | The original in-repository root, child, fragment, named-type import, and validation controls pass. The actual iteration 4 review validates its references even when launched from a temporary directory outside the checkout. The five library tests and two CLI tests ran from their declared Level 1 targets. [Library tests](../../lib/tests/l1/reference_bare_schema_roots.rs), [CLI tests](../../cli/tests/l1/reference_bare_schema.rs) | None of the original instances. R6-F1 covers a different repository scope. |
| R4-F1 The repository kind schema is missing from the worktree | R4-F1.1–.12 | **verified fixed** | The tracked kind definition and the other required files are present. The corpus, the kind admission and classification tests, and the guide's CLI composition passed. The CI evidence sample still accepts format 3 and rejects format 4 at line 5, column 19. The previous review records the required-file deletion and exact test-selection controls. [Corpus](../../lib/tests/l1/shipped_schema_corpus.rs), [Instance evidence](review-5.md#previous-finding-verification) | None |
| R3-F1 YAML anchors and aliases break overload validation in sequences | R3-F1.1–.19 | **verified fixed** | The unchanged parser witnesses and the current CLI, editor, and installed tests cover decorated and aliased arms, duplicate positions, quoted comma controls, and standalone schema carriers. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [Instance evidence](review-4.md#previous-finding-verification) | None |
| R2-F1 Host registrations accept names that Markdown cannot call | R2-F1.1–.11 | **verified fixed** | Current tests exercise accepted punctuation and case variants, refusal of uncallable declarations and aliases, and the absence of refused names from host selection and editor completion. [Host builder tests](../../lib/tests/l1/host_functions.rs), [Installed tests](../../lib/tests/l1/host_registration_companions.rs), [Instance evidence](review-3.md#previous-finding-verification) | None |
| R2-F2 Type-reference errors can point to unrelated text | R2-F2.1–.12 | **verified fixed** | CLI, editor and registration tests still locate the authored type reference with lookalike text in comments, earlier arms or keys. [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs), [Editor tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs), [Instance evidence](review-3.md#previous-finding-verification) | None |
| R2-F3 The no-terminal approval test exceeds the Windows L1 budget | R2-F3.1–.2 | **verified fixed** | The recorded final Windows package run measured 2.878 s. The subprocess still checks refusal and persistence, with the other approval cases checked in-process. [Approval tests](../../cli/tests/l1/approval_prompt.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R2-F4 The Claudine trigger discovery test exceeds the Windows L1 budget | R2-F4.1–.4 | **verified fixed** | The recorded final Windows package run measured 2.412 s. The combined test continues to check both document verdicts and the trigger counts, with trace and diagnostic cases alongside it. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R2-F5 The recursive lifecycle corpus test exceeds the Windows L1 budget | R2-F5.1 | **verified fixed** | The recorded final Windows package run measured 2.030 s. The corpus retains its 19 verdicts and depth-six trigger, location, and deferred-operand checks. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R2-F6 The depth-four lifecycle test exceeds the Windows L1 budget | R2-F6.1 | **verified fixed** | The recorded final Windows package run measured 2.467 s. The test retains two trigger checks, four passing documents, and three failing action positions. [Bundle tests](../../cli/tests/l1/claudine_schema_bundle.rs), [Windows evidence](review-3.md#previous-finding-verification) | None |
| R1-F1 Inline YAML comments make valid catalogs fail | R1-F1.1–.8 | **verified fixed** | Current parser, CLI, editor, and registration checks preserve declarations under comment edits, with literal hashes and line-ending controls. The guide example also passed with a helper comment. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [Instance evidence](review-2.md#previous-finding-verification) | None |
| R1-F2 Catalog diagnostics lose declaration locations | R1-F2.1–.15 | **verified fixed** | The tested syntax, overload, category, name, and facts failures retain authored coordinates. The reserved alias probe also verified the competing built-in source. [Installed tests](../../lib/tests/l1/host_registration_companions.rs), [Instance evidence](review-2.md#previous-finding-verification) | None |
| R1-F3 The shared review and fix schema cannot validate documents | R1-F3.1–.5 | **verified fixed** | The current corpus and review-cycle validation tests still pass, including required-field controls for the two branches and admission of the specification schema. [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs), [Instance evidence](review-2.md#previous-finding-verification) | None |
| R1-F4 Registration and catalog references still describe the old formats | R1-F4.1–.8 | **verified fixed** | The current guides, installed caller skills, and published inventory describe version 3 and the derived companion. Version 2 references explain refusal, migration or cleanup. The schema corpus keeps callable catalogs private. [Registration guide](../../docs/topics/host-registration.md), [Instance evidence](review-2.md#previous-finding-verification) | None |
| R3-V1 The shipped-schema corpus retains its required suggestion-review schema | Required presence and ordinary resolution | **verified fixed** | The file is tracked and both passing corpus entry points explicitly require and resolve it. [Corpus tests](../../lib/tests/l1/shipped_schema_corpus.rs) | None |

| Iteration 5 instance | Verified result |
| --- | --- |
| R5-F1.1 | The repaired kind guide block and its comment variant pass. The reserved-name variant reports both positions. The full guide composes with `--deny all`. |
| R5-F1.2 | The unmodified Simplified Schema guide's two-overload catalog validates. |
| R5-F1.3 | The host function guide's expression and mutation catalogs both validate. The installed mutation refusal still passes its regression test. |
| R5-F1.4 | The host registration guide's expression catalog validates with its narrowing rule. |
| R5-F1.5 | The installed caller guides contain no complete catalog example. Both built-in catalogs validate through the CLI. |
| R5-F3.1 | `md validate refs` checks one link with either a bare or absolute schema. The missing name exits 1. |
| R5-F3.2 | `md validate refs --graph mermaid` emits a graph for both valid spellings and refuses the missing name. |
| R5-F3.3 | `md graph` with and without `--validate` succeeds for both valid spellings and diagnoses the missing name. |
| R5-F3.4 | Within one repository, the reference report checks two references across two documents. The Mermaid and follow-and-validate graphs also succeed. A missing schema fails preparation in the child. |
| R5-F3.5 | With `--fragments`, both valid spellings find the heading generated from frontmatter. A missing schema loses that heading, and a static heading still passes. |
| R5-F3.6 | The CI evidence guide's actual sample with `observation_collection(required)` passes `md validate refs` and `md graph --validate` with bare, relative and absolute library spellings. A missing library is refused. [Evidence sample](../../../docs/cicd/evidence.md) |
| R5-F3.7 | `md schema validate` and `md compose --deny all` preserve the valid and missing-name controls, including compose's inclusion and the named-type import. With `--disable schema-triggers`, file-backed compose, clean, schema validation, and shell completion still use the document's in-repository roots. |

## Unblocked Findings

### R6-F1 (high) Included documents and fragment targets inherit schema-root eligibility from the parent

**Authority (defect).** A Markdown note outside Git with `$schema: review.yaml` and the contract stored under `~/schemas/` fails a direct `md schema validate` or `md validate refs` check. Include that exact note from a repository using `::file ~/review.md` and `md validate refs` succeeds. A link from the same repository also passes fragment validation for a heading the external note can generate only after loading the schema. The schema eligibility decision changes even though the document being checked, its location, and its authored schema stay the same. [Schema root policy](../../docs/topics/schemas/definition.md#schema-roots), [Shared schema preparation](../../lib/src/markdown/compose/schema_validation.rs#L403)

The defect class is inherited repository scope in a decision that must be made for each source document. A document outside a repository can use an explicit absolute path to the schema. A document within a repository can also use its bare name. Opening an external note through an already supported home reference must preserve the note's eligibility. Darkmatter's [`prepare_schemas`](../../lib/src/markdown/compose/schema_validation.rs#L410) builds the schema reader with the source context. The preceding [`searches_schema_roots`](../../lib/src/markdown/compose/schema_validation.rs#L406) decision still reads the outer request context for both root discovery paths.

To reproduce, isolate the home directory, place `review.yaml` under its `schemas/` directory, and give the schema a required string called `description`. Create `~/review.md` with this content.

```markdown
---
$schema: review.yaml
description: Review heading
---

# {{ fm.description }}
```

Create a separate Git repository with one document containing `::file ~/review.md` and a second containing `[go](~/review.md#review-heading)`. Run the commands below from that repository. The relative spelling `./schemas/review.yaml` and the absolute spelling refer to the same file and give matching results. Switching either spelling to `review.yaml` is the load-bearing edit. Switching it to `absent.yaml` confirms that the result requires a schema file.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R6-F1.1 | Reference report and Mermaid graph for an inclusion | Repository parent opens the external note; repeat with a parent outside Git | Both repository-parent modes accept bare and explicit schema spellings and reject missing. The report for the outside-Git parent accepts explicit and rejects bare and missing. | Both parents accept the explicit spellings. Bare and missing fail at the included note. |
| R6-F1.2 | `md graph` with no flags, `--validate`, and `--follow --validate` | Same inclusion | All three accept bare and explicit spellings and reject missing. | All three accept explicit spellings and reject bare and missing. |
| R6-F1.3 | `md validate refs --fragments` | Same external note, linked by its generated heading. Repeat a missing-schema case with a static heading. | Bare and explicit spellings find the generated heading. The missing control loses that heading and still finds a static heading. | Only the explicit spellings generate the heading. Preserve the static-heading fallback. |
| R6-F1.4 | Named-type import used for external Markdown frontmatter | Put `entry: string` in `~/schemas/types.yaml` as a types-only schema and use `description: entry@types.yaml` in the note's inline schema. Repeat with relative, absolute and missing library spellings. | The reference report for the inclusion, follow-and-validate graph, fragment check, and compose with triggers disabled accept bare and explicit imports. Checking the note directly refuses bare and accepts explicit. Missing fails. | Each surface accepts the explicit import and refuses to resolve bare and missing imports through a schema root for this external source. |
| R6-F1.5 | Composition of the repository parent | Same note and inclusion, with and without `--disable schema-triggers` | Both modes render the generated heading for bare and explicit spellings. Missing fails. | Both modes render it for the explicit spellings. Bare and missing fail in the child. |
| R6-F1.6 | Top-level reference preparation, schema validation, compose, clean, and schema-aware shell completion | Open the external note directly from the same repository. For completion use an enum with one value. Keep in-repository controls from R5-F3. | Clean. All direct checks accept explicit spellings and refuse bare and missing. Completion offers the explicit schema's value and no value for bare or missing. The in-repository controls still resolve bare names. | Preserve these results. |
| R6-F1.7 | Repository trigger discovery during default composition | Keep the explicit schema. Add a trigger in the isolated home schema root matching `topic: review` and requiring an absent `version`. Change the topic to `other` for the nonmatch. | Direct compose, clean and schema validation accept the external note. From the repository parent, default compose applies the trigger and refuses the matching note for missing `version`. Changing the topic or disabling triggers restores success. Reference and graph checks accept both topics. | Default composition skips root trigger discovery for this external note, so both topics compose with the explicit schema. Preserve the reference results and ordinary in-repository trigger behavior. |

Use repository eligibility from the source context already computed for schema preparation and keep trigger application and bare lookup consistent with that context. Reference graph nodes, heading preparation, schema template inspection, and compose preflight share that preparation. The same source context check can cover both root selectors, including the trigger control in R6-F1.7. Keep the existing explicit file handling and avoid discovering another repository in the middle of the request. Add a Level 1 regression with a repository parent and a home-opened child, using the child both as an inclusion and as a generated-heading target. [Reference graph implementation](../../lib/src/markdown/reference/graph.rs), [Heading preparation](../../lib/src/markdown/reference/validate.rs#L828), [Source context derivation](../../lib/src/markdown/compose/context/request.rs#L697)

## Recurrence

R6-F1 is a sibling of the reference lookup defect in review 5, R5-F3. The original repair fixed lookup for sources sharing one repository context. The earlier sweep used children and fragment targets in that same repository, so it missed what happens when an admitted target has no repository. The repair newly exposes the inherited eligibility in standalone reference checks and composition with triggers disabled. Default composition already used the outer repository check through trigger discovery. The instance table follows both root selectors into the document and named-type lookups and the heading and command results. [Review 5 finding](review-5.md#r5-f3-high-reference-commands-miss-schemas-named-by-bare-filename)

## Input Robustness Matrix

The feature's existing invalid-input rule is a located refusal before a callable runs or a registration is published. The exceptions are an omitted or empty catalog description, a manifest with an explicit empty `files` list and its required lifecycle member and digest, explicit null for the nullable facts rules, and explicit empty alias, narrowing-fact, and source-exclusion lists. Required catalog category and type collections must be nonempty.

| Validator or loader | Level 1 verification |
| --- | --- |
| Catalog YAML and metadata | The public envelope suite covers absent, null, wrong field type, one or all invalid elements, and empty field shapes. Loader tests cover repeated keys, additional YAML documents, trailing content, comments, tags, aliases, BOM and CRLF. Extracted guide fixtures passed the CLI, and the category edits and reserved spelling produced located refusals. [Envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs), [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs) |
| Manifest YAML, member paths and digests | Public snapshot tests cover required fields, the empty files control, unsupported and wrongly typed versions, malformed YAML, duplicate and trailing content, container and path errors, exact digest membership, and the separate cleanup decoder. Missing bytes and missing digest cases have separate assertions. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs) |
| Facts JSON and binding rules | The public host builder and installed-companion tests cover missing fields, explicit null, wrong field and element types, empty collections, syntax and duplicate-key refusals, and stale binding facts with a valid outer digest. [Host tests](../../lib/tests/l1/host_functions.rs), [Installed tests](../../lib/tests/l1/host_registration_companions.rs) |

I checked the readers for permissive defaults and dropped parse errors. The manifest preview supplies watch candidates and the strict digest and member checks decide admission. The catalog's later raw-carrier read supplies example spelling after admission. The facts reader reports syntax and shape failures through the public registration result. R6-F1 concerns the document context passed to schema resolution. It introduces no new loader or field-shape failure.

## Verification

| User-facing requirement | Appropriate level and evidence |
| --- | --- |
| Built-in signatures, aliases, and read rules | Level 1 descriptor, binding, and `load_data` permission tests, plus direct CLI checks of the built-in catalogs. |
| Existing lifecycle imports, action forms, and event constraints | Level 1 generated-schema, shipped corpus, action binding, and event tests. The lifecycle YAML still has the exact recorded pre-feature Git blob `1f5c316e92f4d4485c4e0ce3cf63c560b265d622`. [Baseline](implementation-log.md#prerequisite-baseline) |
| Catalog validation, privacy, and passivity | Level 1 public parser, resolver, CLI and headless editor protocol tests exercise invalid overloads, duplicate keys, folded collisions, related locations, passive examples, and schema/import refusal. |
| Selected host facts, companion refusal and host-only execution | Level 1 installed loader, compose and editor tests exercise per-document selection, stale and missing companions, and analysis-only dispatch. The host builder and runtime tests cover handler parity, source exclusions, and return checks. |
| Publication, listing, doctor and upgrade | Level 1 Claudine inventory, publication, and doctor tests cover the verified filename set, missing or modified companions, and the version 2 refusal and in-place upgrade. |
| Mutation restriction | Level 1 installed loading refuses `mutation-schema` at the kind declaration. The built-in lifecycle tests still exercise existing operands and event rules. |
| Markdown schema lookup and references | Level 1 CLI and public library checks verify every original review fixture. The cross-scope CLI reproduction exposes R6-F1. The in-repository tests exercise a shared repository context. |

Fresh Linux `just test` runs passed 18,045 Darkmatter tests (10 skipped), 2,626 Simplified Schema default-feature tests and 2,202 minimal-feature tests (zero skipped), and 13,077 Claudine tests (16 skipped). No executed test reached five seconds. The Darkmatter tier audit found zero stranded tests. All three suites passed the shipped-artifact checks.

The acceptance checks concern CLI results, file contents, and editor protocol diagnostics. Level 1 reaches these boundaries. Terminal-emulator rendering and keyboard interaction fall outside the specification. Cross-OS evidence is handled by CI.

The iteration 5 implementation log now qualifies its schema-root claim with the verified document boundary. Its original repair evidence remains valid. [Record correction](log.md#review-6-record-correction-2026-11-25)
