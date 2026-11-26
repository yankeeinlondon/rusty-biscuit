---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-11.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-10.md"
next: "2026-11-22-expression-and-mutation-schemas/review-12.md"
created: "2026-11-25T17:22:46+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R10-F2 (high): Reference validation drops malformed transclusions"
    - "R11-F1 (high): Reference checks misclassify inline and suppressed inclusions"
    - "R11-F2 (high): Inclusion diagnostics point to body-relative lines"
    - "R11-F3 (medium): FileTree loses the admitted source when checking root headings"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "claude/opus"
---

# Expression and mutation schemas review 11

The feature is not production ready. The original malformed inclusions now fail reference validation, but some directive errors still lose their diagnostic codes. New probes also show that valid inline content and suppressed TOC links can block composition, directive errors can name the wrong line, and FileTree can incorrectly reject a heading generated using a bare schema. All four findings can be repaired under the existing contracts.

## Previous Finding Verification

The following checks cover all 24 child rows in the iteration 10 ledger. Earlier reviews have no open findings or human rulings. Review 10 records the earlier verified outcomes, including the Windows measurements. The Linux checks also reran the earlier catalog and schema tests. [Iteration 10 ledger](log.md#implementation-of-review-findings-10) · [Prior verification](review-10.md#previous-finding-verification)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R10-F2 Reference validation drops malformed transclusions | R10-F2.1–.15 | **partially fixed** | The table below confirms refusal of the original malformed inputs, plus the load, condition, cycle, and depth controls. Public API and CLI probes found gaps in .11 and .15. | Preserve specific diagnostic codes under R10-F2.11. R11-F1 names a newly found sibling in the .15 target-selection sweep. R11-F2 covers the new location cases. |
| R10-F1 Library consumers can ignore the admitted document source | R10-F1.1–.9 | **verified fixed** | All nine ledger instances checked below. Existing request-only and relative-source tests passed, including both reference validation entry points. | R11-F3 is a newly found sibling in FileTree's direct use of the shared validation engine. |

| Ledger instance | Verification evidence |
| --- | --- |
| R10-F2.1 | The declared Level 1 test checked both graph modes, local query, flattening and validation. Quoted and unquoted forms retained equivalent records and children. Omitting the target caused refusal for all three body directives. Direct CLI probes returned `dm.directive.parse_failure`; the frontmatter variant exposed R11-F2. |
| R10-F2.2 | The invalid level and invalid depth tests passed in both graph modes and the local query. Built CLI output used `dm.toc_linking.invalid_level` and `dm.file_links.invalid_depth`. Quoting the valid control preserved its shape. |
| R10-F2.3 | The public API shape test passed for both keys. Absent and `[]` returned empty; a valid scalar and list gave child edges; blank, null, number, object, mixed and all-invalid lists returned parser errors. |
| R10-F2.4 | The public local query returned the expected TOC, file-list and frontmatter failures for those cases. The exposed families and the diagnostic code for a bad frontmatter value matched the implementation. |
| R10-F2.5 | The flattened reference and typed output projections propagate graph build failures. A malformed graph cannot reach prebuilt validation. The existing prebuilt validation tests also reject a changed document or changed graph options. The CLI failed with the expected diagnostic family and filename. The line-3 fixtures passed; R11-F2 adds frontmatter and prepared-body cases. |
| R10-F2.6 | The parser-defer and nested-inclusion tests passed. The CLI preserves root rejection and the library composition path can defer child load/parse faults. The command collector propagates its body, TOC and frontmatter parser errors and deliberately does not read code or file-list targets for commands. The editor token scanner still serves navigation independently. |
| R10-F2.7 | The boolean detector states its successful-parse rule and its dedicated test passed. The fallible queries report the original parser faults. |
| R10-F2.8 | The public test passed for a body file, prologue, epilogue and TOC target with invalid YAML. Both graphs and validation report the target file. The file-backed CLI also refuses a bad `::file` child during command discovery. TOC composition can still emit raw headings from the same bytes. |
| R10-F2.9 | The fragment test passed. A plain link checks existence, a fragment into the bad YAML emits `UnreadableTarget`, and a fragment into a valid sibling passes. The built CLI reproduced `UnreadableTarget` with `--fragments`. |
| R10-F2.10 | The public test and built CLI distinguished false, successful, and failing evaluations. Division by zero produced a located `dm.expression.evaluation_failure`. The check before composition defers it. A malformed expression also fails, with the code gap shown under R10-F2.11. |
| R10-F2.11 | The direct typed-error test and CLI level/depth probes passed. Runtime warning projection and the graph's reference-error boundary still lose codes. See the remaining instances under R10-F2. |
| R10-F2.12 | The key-location API test and the CLI test passed. The public probe returned line 2 for `prologue: null`, including structured output. An in-memory key with no authored line displays no line 0. |
| R10-F2.13 | The depth/cycle test passed. A four-node chain fails at limit 3 and succeeds at 4; the passive check defers the refusal. The tracker checks depth before pushing, so a refusal does not change the active stack. |
| R10-F2.14 | The same fixture with a cycle builds a finite graph; the graph and flattening cycle tests passed. The accepted cycle behavior stands. |
| R10-F2.15 | Precheck/prefetch results lead to authoritative parser calls for those inclusion forms. CSS and HTML tokenizers extract link text. Required missing TOC paths are reported, but a suppressed TOC path is also reported as required. This sibling is R11-F1. |
| R10-F1.1 | Request-only relative and absolute source tests passed for all three composition entry points, with bare, explicit and missing schema/import references and the component trigger's required field. |
| R10-F1.2 | Request-only preflight agreement tests passed for both spellings, testing planning, approval and command collection on the same content. The frontmatter-only scan still lists raw shell sites. |
| R10-F1.3 | Both request-only `effective_schema_for` tests passed for a loadable schema, an unresolved name and the matching component trigger. |
| R10-F1.4 | All five graph-root controls passed. Both graph builders and flattened origins used the admitted file, with the component inclusion attached. |
| R10-F1.5 | Fresh and prebuilt validation passed the component link tests, including the expected missing fragment. A direct public API probe additionally accepted a generated root heading for both request-only forms. FileTree reaches the shared engine through a separate call, described in R11-F3. |
| R10-F1.6 | Both component and same-checkout local-query tests passed across their five documented source forms for body `::file`, `::code`, prologue and epilogue, including a missing target. |
| R10-F1.7 | File-backed CLI tests and both public local expression-context controls passed. Editor catalog, schema-scope and registration reload tests passed in the DMLS Level 1 target. |
| R10-F1.8 | The approval entry calls source admission before schema preparation. The request-only preflight agreement tests passed, and the existing schema-slot grant and request-snapshot checks remained green. |
| R10-F1.9 | The cited internal paths use the rooted request, operate on deliberately distinct generated content, or do not accept a request. The generated-identity directive tests and render/compose parity controls passed. |

## Unblocked Findings

### R10-F2 (high) Reference validation drops malformed transclusions

**Authority (defect).** The original parser refusals work. The remaining defect is in the review 10 ledger's typed-error requirement. The repository's [error design rule](../../../AGENTS.md) requires a specific registered code. A user who types an incomplete `when=` expression, an unclosed heading filter, or a file-list glob with a missing bracket receives a generic code even though the implementation knows the precise cause. The graph command can also discard an already typed reference error at its output boundary. [Diagnostic projection](../../lib/src/markdown/compose/report_diagnostics.rs#L863) · [Warning projection](../../lib/src/markdown/compose/pipeline/phases.rs#L744) · [Graph command](../../cli/src/commands/graph.rs#L59)

The defect class is losing an error's type while projecting it to a CLI diagnostic. Equivalent quoting should preserve the code, and changing the cause should select the corresponding registered code. I used the same local guide, its `side.md` target, and a directory containing a Markdown file. Runtime warning versus fatal error remains the owning stage's decision.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R10-F2.11 | Direct typed error projection and all reference command graph-build paths | Valid and invalid `level=`, `cleanup=` and `--depth`. Compare quoted/unquoted malformed values. | The typed variant test passed for these cases and the remaining IO and resolution variants. Real level/depth refusals use the specific codes in compose, both validators and graph output. | Retain the working projection and add a real command assertion where the variant can otherwise be bypassed. |
| R10-F2.16 | TOC and file-list runtime warning projection | Valid heading filter and file glob, then `filter="["` and `./files/[.md`. For directory mode use an existing directory, an absent directory, a Markdown file and an unreadable directory. Run a valid glob from stdin to exercise the missing-source path. | Compose can continue but emits `dm.compose.warning` for the invalid TOC filter, invalid glob, three directory failures and missing source. The direct projection has existing specific codes. Graph and reference checks do not execute file-list discovery or TOC filtering. | Preserve severity and recovery while carrying the available specific code. The reference commands can keep their existing execution boundary. Update the reader example to explain the outcome of tolerated failures. |
| R10-F2.17 | Body condition and shared diagnostic projection | `when="false"`, `when="1 / 0"`, `when="1 +"`. | False yields no inclusion. Division by zero gives `dm.expression.evaluation_failure`. The malformed condition gives `dm.transclusion.failure` in composition, both graph modes and flattened projections without losing the authored location. The local query returns a raw directive by design. | Use the existing expression parse code for the typed condition-parse failure. Keep the evaluation code and the local query contract. |
| R10-F2.18 | FileTree error boundary in `md graph`, with or without `--validate` | Existing file and a genuinely invalid file reference. A bare `@` is an invalid path without a payload. The defect reproduces with `::file @` and `prologue: "@"`. | The graph reports `dm.compose.failure`; `md validate refs` and the public diagnostic projection return `dm.reference.failure`. Graph build failures carried as Markdown errors keep their supplied codes. | Forward the typed reference cause through the graph output route. Keep empty inline content valid under R11-F1 and use an independently invalid reference to pin this route. |

Reuse the typed projection for tolerated errors and adapt the graph boundary to pass the typed reference cause. The shared diagnostic names are already available. A command test should check the code where a parser or runtime outcome reaches the user, including one tolerated warning.

### R11-F1 (high) Reference checks misclassify inline and suppressed inclusions

**Authority (defect).** A document can use `prologue: "[Setup](./side.md)"` with an existing `side.md`, or `::toc-linking "./optional.md | false"` to omit an optional section. The public composition API accepts both. The built CLI's preliminary reference check blocks both and reports missing files. A multiline version of the same prologue still fails because validation treats its internal inline placeholder as an empty authored target. [Composition classifier](../../lib/src/markdown/compose/transclusion/resolver.rs#L259) · [Graph TOC and frontmatter handling](../../lib/src/markdown/reference/graph.rs#L825) · [Empty-target decision](../../lib/src/markdown/reference/validate.rs#L440)

The defect class is collapsing a valid non-file inclusion outcome into a required file reference. Equivalent quotation or list spelling must preserve the outcome. Changing the link within inline text to a missing file, or removing the final `false` from an exhausted fallback chain, must produce a genuine missing-reference issue.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R11-F1.1 | Prologue and epilogue classification in both graph modes and the local query | Single-line prose, a Markdown link, a block scalar with the same link, an empty scalar and a one-element list containing each; controls for a real path and an empty list. | The public composer emits prose and links or treats empty text as a no-op. The graph treats single-line prose and links as path references. It marks multiline content inline but emits an empty placeholder. Both graph modes and the local query try to resolve empty text and fail. The local query retains nonempty inline text with no resolved target. The real path resolves and `[]` emits nothing. | Use the existing frontmatter classifier for both keys. Preserve inline graph/display information and a coherent local-query result without attempting file resolution for inline values. |
| R11-F1.2 | Flattened projections, fresh and prebuilt reference checks, FileTree, validating CLI, and the command's reference check before composition | Run the single-line, multiline, empty, existing-file and missing-link controls above. | Single-line content gives `MissingLocalTarget` for the entire scalar. Multiline content gives `EmptyTarget` for its placeholder even when the actual link is valid. The empty scalar prevents graph construction. `md compose`, `md validate refs` and `md graph --validate` exit 1. A real file inclusion passes, and the graph exposes the nested actual link in multiline content. | Check the actual link inside the inline content and preserve its hyperlink kind. An inline display placeholder contributes no missing-file verdict. |
| R11-F1.3 | TOC selector, graph/flattening, prebuilt and fresh validation, and graph-based CLI routes | Existing first target, existing fallback after a missing path, all missing with trailing `false`, and all missing without it. | Existing alternatives select the right path in the graph. In the all-missing cases both return the same graph record for the first path. Validation flags it as required and blocks compose, even when the public compose API correctly suppresses the directive. The local query lists the authored TOC target and does not follow it. | Keep the suppression outcome distinct from failure in the graph. Preserve required-missing reports, the selected fallback, and the local query's listing role. |
| R11-F1.4 | Related inclusion selectors and downstream boundary controls | Keep or omit a real local `::file` or `::code` target; put a valid file in `when="false"`; quote a real URL; use a valid directory/file-list glob and a required missing TOC path. | The selectors preserve required missing files and conditional omission. The URL validator classifies the target without a remote read. File-list records describe a discovery query and remain outside path-existence checks. The preflight TOC selector already distinguishes an existing fallback, suppression and required failure, and the editor target scanner has an existing suppression control. | Keep the distinct file, URL, conditional and listing decisions. |

Share the existing frontmatter classifier and keep the TOC resolver's three outcomes through graph construction. Use the graph's inline marker to keep placeholders out of path validation while preserving links inside inline content.

### R11-F2 (high) Inclusion diagnostics point to body-relative lines

**Authority (defect).** Add a four-line YAML header to a document with a bare `::file` on body line 3. The saved directive moves to line 7. The built graph and reference commands report line 3, which is inside the YAML header, and the JSON record has no line. The TOC and file-list parsers make the same mistake even in `md compose`. A reader following the diagnostic opens the wrong part of the source. [Graph parser calls](../../lib/src/markdown/reference/graph.rs#L737) · [Local query](../../lib/src/markdown/reference/mod.rs#L209) · [Source projection](../../lib/src/markdown/compose/report_diagnostics.rs#L1299)

The defect class is presenting a prepared-body coordinate as an authored file coordinate. Adding a header or removing earlier body content through an existing page block must leave the diagnostic attached to the directive. Replacing the malformed directive with the valid spelling must remove the diagnostic.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R11-F2.1 | Body `::file`, `::code` and `::url` parsers through graph modes, public local query, typed flattened views, and validation | Add two YAML keys before a directive on body line 3. Run with and without a target. | All invalid graph and local results report line 3, as do built `md graph` and `md validate refs`; structured source line is null. Without YAML, line 3 happens to be correct. The valid graph controls pass. CLI compose's command scan displays line 7 for this fixture but leaves the structured line unset. | Name authored line 7, with a structured location where the directive's origin is known. |
| R11-F2.2 | TOC parser through composition, command discovery, both graphs, local query and validator projections | Use `level=h9` at authored line 7, then change it to `h2`. | The invalid CLI results and public error messages describe line 3 with no structured line. The valid control parses. | Use the authored opener or omit the authored line when it cannot be established. |
| R11-F2.3 | File-list parser through composition, both graphs, local query and validator projections | Use `--depth two` at line 7, then change it to `--depth 1`. | Error reports line 3, and the JSON position is null. The numeric control parses. Command discovery does not parse file-list query options. | Same authored-line rule. Keep the existing discovery boundary. |
| R11-F2.4 | Prepared-body and raw-body paths | Put a false page block before the malformed body directive, then add or remove the block. Compare plain CLI text and public results. | The graph message counts from the shortened body. The local query reports raw body line 7 for the block fixture. Both omit the YAML offset. Without the block the saved directive is on line 7 and the graph reports line 3. With the block the saved directive is on line 11 and the graph reports line 4. | Use the known authored opener, even after a previous block disappears. When the bytes have no authored position, report the file without claiming a file line. |
| R11-F2.5 | Frontmatter key errors, condition errors and editor navigation | Use `prologue: null`, `when="1 / 0"` and `when="1 +"` in the same file, one at a time. | The frontmatter error retains its key line. Condition diagnostics use authored source information and report line 7. The editor's raw authored token scanner is a navigation surface and does not perform these composition parses. | Keep the working locations and use the same distinction for parser failures. |

Use the existing authored-body provenance for the error site. For the raw local query, use its original body and header offset. Where a precise origin is unavailable, keep the file and failure reason without publishing the transformed line as a source position. A root fixture with frontmatter and one earlier removed block covers the difference between a fixed offset and an authored position.

### R11-F3 (medium) FileTree loses the admitted source when checking root headings

**Authority (defect).** An embedder can pass a buffer with a request file and use a bare schema to give a Markdown heading its label. When fragment checking is enabled, Darkmatter's public FileTree reports a link to that heading as broken, while the direct validator and the public prebuilt-graph validator accept it. FileTree builds the graph with the admitted source and then passes its original buffer to the fresh validation entry point. That entry point prepares root headings again using the buffer's missing source. [FileTree validation](../../lib/src/markdown/reference/file_tree/mod.rs#L246) · [Root heading collection](../../lib/src/markdown/reference/validate.rs#L867)

The defect class is losing the admitted document identity during a second preparation. The same file passed as a loaded document, a relative Markdown source, or a relative or absolute request source must yield the same headings. Changing just the fragment must produce one missing-heading issue in all four forms.

```md
---
$schema: labels.yaml
---
# {{ label }}

[Jump to heading](#ready)
```

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R11-F3.1 | FileTree graph build and root fragment overlay | A repository note uses `$schema: labels.yaml` and an interpolated heading (shown above). The schema defaults `label` to `Ready`. Check `#ready` using the four source forms, then change to `#gone`. | The graph reaches the right file. FileTree reports one missing fragment for both request-only forms of `#ready`, zero for both Markdown-sourced forms. Every form reports one error for `#gone`. With `./schemas/labels.yaml`, every form accepts `#ready`. | Accept `#ready` and report the missing `#gone` in every source form, including the bare schema. |
| R11-F3.2 | Direct and prebuilt reference validation, graph origins and heading preparation | Same fixture and all four forms. | Direct and prebuilt validation produce zero issues for `#ready` and one for `#gone`, including the bare schema. Graph origins and direct checkout-specific schema resolution pass the declared request-only tests. | Keep the existing verdict and origin from the admitted document. |
| R11-F3.3 | File-backed CLI and neighboring library consumers | Same generated-heading contract with a loaded file, plus the request-only schema, trigger, approvals and local-query controls. | File-backed fragment validation accepts the existing anchor and rejects the missing one. The request-only composition, schema, planning and local-query tests all pass. Descendant headings already use the graph's prepared records. | Keep loaded and generated-buffer clients consistent at the root, with the existing descendant behavior. |

Apply source admission where the fresh validator receives its root or pass the admitted document from FileTree. The repair can share the existing admission path without changing the public API.

## Blocked Findings

None.

## Recurrence

- **R10-F2** is partially repaired. The previous review's parsing paths now refuse the malformed inputs. The repair ledger's typed error sweep checked the direct error projection but missed tolerated-runtime diagnostics and the CLI graph projection. Condition parsing exposes a sibling of the condition error row at the same projection.
- **R11-F1** is a sibling of review 10's inclusion-reader defect. Its sweep covered malformed input and an ordinary missing TOC target. It missed the non-file outcomes that enter graph classification and validation, including single-line inline content, the multiline display placeholder, and suppression after a TOC fallback. The instance table follows each outcome through the two graph modes and both validation paths.
- **R11-F2** is a sibling of the location loss in review 1's R1-F2 and review 2's R2-F2. Those findings repaired catalog-declaration positions. The iteration 10 location sweep covered frontmatter keys and condition evaluation but left ordinary directive parser positions relative to prepared content. The new probes cover all the inclusion parser families and their public projections.
- **R11-F3** is a missed consumer in the source-identity class from reviews 6–10, most directly review 10's R10-F1. The repair covered the two public validator methods, which both admit the source. FileTree invokes the fresh engine directly. Following the same source through graph construction and the independent root-heading lookup exposed the missed use. The recurring boundary is where an admitted document passes into another preparation step, so the repair should centralize that handoff.

## Input Robustness Matrix

The iteration 10 code changed how existing inclusion parsers report errors and how callers pass document identity. The Markdown text grammar is outside the format matrix. The shared YAML value validator for `prologue` and `epilogue` is relevant because those fields decide which files are read. Its default is a located error for a non-string scalar, null, mapping, or list containing a non-string. Both keys are covered. Absence and an empty list are no-ops. A file string and an equivalent one-element list resolve the same file. Valid inline strings also pass shape checking, then expose the classification defect in R11-F1. No new YAML loader was added by the repair. [Frontmatter shape tests](../../lib/tests/l1/reference_malformed_inclusions.rs)

For the feature's original configuration readers, the Level 1 tests reran the shared callable envelope and strict catalog-facts checks. Required catalog categories and types reject absence, null, wrong types and empty collections. The optional description accepts absence and an empty string. Local helper definitions are accepted alongside at least one callable. Unknown fields, duplicates, invalid trailing content, and wrong catalog shapes are refused. Format 3 keeps an empty `files` list valid and refuses a missing or stale required companion for a listed expression catalog. The strict JSON reader also rejects JSON5 extensions and duplicate or trailing content, while explicit nulls and empty values are retained where the facts contract permits them. YAML comments, quoted hashes and aliases are covered on the shared reader routes. [Catalog envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs) · [Companion tests](../../lib/tests/l1/host_registration_companions.rs) · [Manifest snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs)

## Verification

Each spec acceptance check is a parser, runtime, file, editor-protocol or CLI contract. Level 1 can observe these results directly. The spec has no emulator-rendering or keyboard-encoder requirement.

| Acceptance check | Verification level and evidence |
| --- | --- |
| Preserve built-in signatures, aliases, binding, data reads, lifecycle imports and generated artifacts | Level 1 catalog, read-permission, lifecycle end-to-end, shipped artifact and corpus tests. The implementation record supplies the byte-for-byte lifecycle generation comparison. [Implementation record](implementation-log.md#generated-parity--caller-skill-publication) |
| CLI and editor agree on invalid overload, duplicate key and folded name collision, and never apply callable catalogs to Markdown frontmatter | Level 1 CLI JSON and DMLS protocol tests exercise the same declarations and report primary and related locations. The shipped corpus tests reject callable catalogs as schema targets and imports. [CLI tests](../../cli/tests/l1/schema_validate_yaml.rs) · [DMLS tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Selected-host analysis only, companion refusal and recovery, and host-only execution | Level 1 CLI tests cover selected, unselected and other-host calls and the runtime-required error. DMLS protocol tests cover companion removal, staleness, restoration and reload. Host runtime tests exercise the handler and its return checks. [Host CLI](../../cli/tests/l1/compose_host_scope.rs) · [DMLS reload](../../dmls/tests/l1/host_catalog_reload.rs) · [Host runtime](../../cli/tests/l1/host_runtime_scope.rs) |
| Publisher, listing and doctor agree, with an in-place format upgrade | Level 1 publisher, inventory and doctor tests exercise generated companions, missing/modified companion repair and unchanged-path format 2 upgrade. [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs) · [Doctor CLI tests](../../../claudine/cli/tests/l1/doctor.rs) |
| Refuse hosted mutations and retain built-in event and operand rules | Level 1 registration refusal at `kind`, lifecycle action/operand tests, and an expression-versus-action control. [Hosted mutation refusal](../../lib/tests/l1/host_registration_companions.rs) · [Runtime boundary](../../cli/tests/l1/host_runtime_scope.rs) |

The new public API probes linked the checkout's built Darkmatter library. The CLI probes used the checkout's `md` binary and temporary local fixtures. The review changes Markdown records only.

- Darkmatter `just test` passed all 18,207 selected Level 1 tests, with 10 skips and none at or above five seconds.
- Simplified Schema `just test` passed 2,626 default-feature tests and 2,202 minimal-feature tests, with no failures or skips. Neither run reported a test at or above five seconds.
- Claudine `just test --max-fail all` passed all 13,077 selected Level 1 tests, with 16 skips and none at or above five seconds. The first attempt overlapped the tier audit and six completion-fixture tests launched a Claudine binary without fixture support. The isolated rerun used the fixture-enabled binary and all six passed.
- Darkmatter's shipped-schema freshness check passed. The tier audit found zero stranded tests across Darkmatter, Simplified Schema, and Claudine. The review validates against its schema and composes with `--deny all`.
