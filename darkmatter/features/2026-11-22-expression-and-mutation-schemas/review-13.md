---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-13.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-12.md"
created: "2026-11-25T21:30:29+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: false
findings:
    - "R13-F1 (high): Reference checks accept targets that the inclusion directive rejects"
    - "R13-F2 (high): TOC target read errors lose the directive location"
    - "R13-F3 (high): Missing-target allowances let invalid links and images pass composition"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
---

# Expression and mutation schemas review 13

The feature is not production ready. Review 12’s reported target-selection and location cases pass. Three additional failures remain. A reference check can pass a document with an unusable inclusion target, a TOC read error can point to the wrong document, and a missing-target allowance can let composition publish a document that contains an invalid reference.

## Previous Finding Verification

The earlier reviews close the findings from iterations 1–11. No review contains a human ruling. I checked the Review 12 instances against the current public readers, the CLI, and the repair ledger. [Review 12](review-12.md) · [Iteration 12 ledger](log.md#implementation-of-review-findings-12)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R12-F1 Reference validation applies hyperlink rules to inclusion targets | R12-F1.1–.6 and ledger checks R12-S1–S3 | **verified fixed** | Whole-filename, required and suppressed TOC, fallback, link and remote controls pass. A loopback probe saw a HEAD request for a valid URL inclusion and for an ordinary remote hyperlink, and zero requests for either a required or a suppressed TOC URL. | R13-F1 covers newly found target-admission cases. |
| R12-F2 Inclusion target errors lose the authored file and line | R12-F2.1–.6 and ledger checks R12-S4–S14 | **verified fixed** | The listed malformed and missing targets retain the authored opener, key or list entry and the child’s identity. Located hyperlink and image issues retain their positions. The specific ledger controls are below. | R13-F2 covers a separate TOC read path. R13-F3 covers a regression introduced when the repair made link and image errors into located issues. |

| Ledger instances | Verification evidence |
| --- | --- |
| R12-F1.1–.2 | The public-reference and CLI suites reject a hash suffix on a local file or code inclusion when the complete filename is absent. Both frontmatter fields have the same behavior. Plain filenames, inline text and its own links pass their controls. |
| R12-F1.3–.4 | The required file, fragment, HTTP, mail and data targets fail as tested. A trailing `false` or `FALSE` suppresses the exhausted chain, and removing it restores failure. The loopback run confirmed zero requests for required and suppressed TOC URLs. |
| R12-F1.5–.6 | Both graph modes and preflight selected the existing local fallback. The raw query listed the chain. The generated TOC links, ordinary fragments, hyperlink, URL inclusion, false condition, and file-list controls passed. The editor has a passive navigation path with its own selection test. |
| R12-F2.1–.3 | The body tests exercised all seven readers with bare and quoted targets at line 8 and after a removed block at line 11. Both frontmatter keys, their list entries, the shape control, and all four missing-target cases passed. Listing-only readers kept their existing behavior. |
| R12-F2.4–.6 | The nested invalid target named the child at line 7, and the required TOC selector failure kept the line before and after a removed block. The diagnostic suites checked grammar, conditions, file lists, shell and page blocks, child YAML, and inline links. |
| R12-S1–S3 | A URL-shaped file-list glob reached neither remote validation nor the request batch. A fragment-shaped file or code inclusion looked for the complete filename. The preflight and query controls passed, and editor navigation resolves through its own local reader. |
| R12-S4–S8 | The tolerated child warning retained its typed code and location. Direct API probes also placed an invalid-UTF-8 file or frontmatter inclusion on its directive or key. The specific code, unreadable child, refused remote, and validation/query fixtures passed. R13-F2 covers the TOC heading read that happens before the graph’s located child load. |
| R12-S9 | Claudine’s missing-include unit test passed for the wrapped and unwrapped causes and distinguishes the nonmissing controls. |
| R12-S10 | Fresh and prebuilt validation, FileTree, and CLI tests confirmed the located `InvalidReference` issue. The default CLI exits with status 1. With recovery enabled, the same issue can publish output and run the composed lifecycle event. R13-F3 records that regression. |
| R12-S11–S12 | The status-block test passed and the CLI tests verified the located human output. Direct and lifecycle CLI checks kept the earlier warning on refusal for an invalid reference and a missing target. |
| R12-S13–S14 | The depth fixture exercised CLI document fallback with null line and column. The depth and cycle errors describe the chain, while the location repair at issue concerns an individual target read or lookup. |

## Unblocked Findings

### R13-F1 (high) Reference checks accept targets that the inclusion directive rejects

**Authority (defect).** With an existing `side.txt`, both validating CLI commands pass `::file ./side.txt`, and both public graphs create a Markdown child from it. Composition and command preflight reject it as an unsupported file type. A copied URL directive switched to `::url ./side.md` also passes the checks if the file exists. Composition requires an absolute URL and refuses it. An author can get a clean check while the same document fails to compose. [Composition target admission](../../lib/src/markdown/compose/transclusion/resolver.rs#L28) · [Graph target classification](../../lib/src/markdown/reference/graph.rs#L734) · [Validation branches](../../lib/src/markdown/reference/validate.rs#L462)

The defect class is treating an inclusion target as usable before applying the admission rule of its directive. Quoting the same target must preserve its meaning. Changing a Markdown inclusion’s target from an existing `.md` to an existing `.txt`, or a URL from absolute HTTP to a relative path, must change the admission result. The probes used a single guide with an existing Markdown file, a plain-text file, a directory, and a loopback HTTP endpoint. I checked default and fragment validation, and enabled remote validation only with loopback fixtures.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R13-F1.1 | Body `::file`, both graph modes, flattened output, fresh and prebuilt validation, FileTree with and without following, and both validating CLI commands | `./side.md` → `./side.txt`; repeat the latter quoted | Both graphs create a child for either extension. Flattening reports a reference and every validator passes. Compose and preflight refuse the text file at line 7. The local query also returns the resolved text filename. | Apply the Markdown-file admission rule before following a child or accepting the inclusion. Keep any navigation-only path lookup distinct from admission. The Markdown control still passes. |
| R13-F1.2 | Frontmatter `prologue` and `epilogue`, the same consumers | Each key with `"./side.md"` → `"./side.txt"` | Both graphs follow the text file, and fresh, prebuilt and FileTree validation pass. Compose and preflight return the unsupported-target error at line 3. The local query finds the literal path. | Apply the same Markdown-file rule to both keys and retain inline Markdown handling. |
| R13-F1.3 | Body `::url`, target kind in the full graph, flattened output, validators, FileTree, and the remote request batch | Absolute loopback HTTP → `./side.md`, quoted `"./side.md"`, `"#here"`, a scheme-relative loopback path, and `mailto:guide@example.invalid` | The relative path and fragment pass fresh and prebuilt validation and both FileTree views. A missing relative file yields a missing-file issue. Relative, fragment and scheme-relative targets fail URL parsing in compose. Remote validation normalizes a scheme-relative URL and attempts a request. The mail target passes validation and fails file-reference admission in compose. Bare `@` keeps its existing located invalid-reference failure. | Validate URL-inclusion syntax before hyperlink or request handling. Accept an admissible absolute URL for passive checking. The raw query and nonvalidating graph can keep listing authored URL targets, and the opt-in remote check remains available for valid targets. |
| R13-F1.4 | Body `::code` and the path-resolution neighbors | `::code ./side.txt`, then `::code ./dir` and `::file ./dir` | Text code composes and validates. Both validators reject the directory as a missing file. The raw query leaves the directory unresolved. Code payload decoding happens during composition, after the path check. | Preserve text source admission and the directory rejection. Keep the presence check independent of code payload decoding. |
| R13-F1.5 | TOC chain selection, file-list globs, ordinary links and images, editor navigation | Selected Markdown fallback, suppressed missing chain, URL-like file-list glob, `[Side](./side.md#side)`, and an image reference to an existing file | TOC selects or suppresses as expected. The file-list glob sends no request. The ordinary fragment and image resolve under their own rules. The editor’s passive navigation selects local paths and uses no reference-validation verdict. | Keep lookup and navigation behavior for these separate reference kinds. |

Reuse the composition resolver’s passive admission checks after local resolution and before opening Markdown children. Apply the URL directive’s target parser before the validating graph’s hyperlink branches or remote batch. Keep permission to fetch with the existing policy. These checks need no content read for code sources or network request for URL syntax.

### R13-F2 (high) TOC target read errors lose the directive location

**Authority (defect).** A guide includes `nested.md`, which has `::toc-linking ./utf16.md` on line 11. The selected file is a Markdown document saved as UTF-16. `md validate refs` then points to the guide with no line, while compose warns in the nested file with no line. Looking at either reported location leaves the author to find the directive unaided. [TOC heading read during composition](../../lib/src/markdown/compose/transclusion/engine.rs#L1439) · [Full graph heading read](../../lib/src/markdown/reference/graph.rs#L1483) · [Located load in the other graph mode](../../lib/src/markdown/reference/graph.rs#L83)

The defect class is losing the authored directive site after a selected file fails to read. Quoting a path or putting a false page block before the opener preserves the source location. Switching from a UTF-8 target to the unreadable target adds a located failure. The fixture used a BOM-bearing UTF-16 file and a UTF-8 control, both as direct and nested inputs.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R13-F2.1 | Composition’s TOC heading read and the warning projection | Root and nested input, with quoted and unquoted unreadable targets before and after an earlier removed block | Compose preserves `dm.input.read_failure` and tolerates the read failure. The diagnostic names the document with the directive but has no line or column. The warning text omits the selected filename. | Keep the warning behavior and code, and report the selected filename and the directive’s authored opener. |
| R13-F2.2 | Full graph’s TOC heading extraction, flattening, fresh validation, FileTree, and the CLI graph with and without validation | Same unreadable target directly, nested, and selected after an absent first alternative | The public failure has no authored source. The CLI uses the input document with no line, including when the directive is in the child. FileTree exposes the same bare file-load error, and a prebuilt validation call cannot obtain a full graph. | Attach the selected file’s read error to the directive before it leaves graph construction, preserving the original hard failure and code. |
| R13-F2.3 | Transclusion-only graph and neighboring file-inclusion reads | Unreadable selected TOC child, ordinary `::file`, and each frontmatter key | The transclusion-only graph reports the TOC directive at the correct file and line. The ordinary file and frontmatter inclusions report their own sites across the public readers. | Keep the correctly located load path and use the same source information for the earlier TOC heading read. |
| R13-F2.4 | Preflight, authored query, target selection and condition controls | Same unreadable TOC target, valid UTF-8 target, required absent target, and absent chain with `false` | Preflight selects the existing path without reading headings. The raw query lists the chain. A required lookup failure retains its site, and the suppressed chain and valid target add no read diagnostic. | Retain preflight and listing boundaries, target selection, and existing lookup diagnostics. |

Construct the existing inclusion site where the TOC branch still has the opener. Use it when returning the read failure from the heading loader in composition and in the full graph, preserving the code and each caller’s warning or error behavior.

### R13-F3 (high) Missing-target allowances let invalid links and images pass composition

**Authority (regression).** Before the repair, a file-reference error escaped the validator before the CLI considered missing-target recovery. Review 12 turned that failure into a located issue. With `md compose --allow missing-hyperlinks`, `[Guide](@)` now produces a document and exits 0 while reporting `dm.reference.failure`. An equivalent image passes with `--allow missing-image-refs`. With a `composed:` event, the command also runs the event and records its output. These flags cover absent targets. [Issue projection](../../lib/src/markdown/reference/validate.rs#L796) · [Shared CLI decision](../../cli/src/commands/compose.rs#L818) · [Recovery contract](../../docs/cli/policy.md#L52)

The defect class is granting recovery by reference kind without checking the failure class. Equivalent spellings of an invalid target should both fail. Replacing the invalid target with a well-formed absent path should allow the matching recovery. I tested direct and lifecycle composition, using `composed: [{stdout: done}]` as an observable event without any external command.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R13-F3.1 | Located hyperlink and image issues, direct compose | `[Guide](@)`, `[Guide](<@>)`, `![Guide](@)` and an inline frontmatter link or image with the matching allowance | Without the allowance the command exits 1. With it, it exits 0 and emits the Markdown alongside a located error with `failure: invalid-reference`. Adding `--deny all` makes the malformed body cases fail again. | Fail before emitting the document even when the matching missing-target recovery is enabled. Keep the issue’s location. |
| R13-F3.2 | File-reference I/O issue in the same hyperlink and image decisions | A link or image to a file under a directory that the process cannot traverse | The issue is located and has `failure: io`. Each matching missing-target allowance makes the command exit 0 and emit output. | Preserve the I/O failure and refuse composition. |
| R13-F3.3 | Lifecycle compose, including its output decision | Both malformed and unreadable reference kinds, inline frontmatter controls, and the composed-event fixture | The same allowance changes failure to success. The returned document is present and the `composed` event records `done`. Without the allowance, no document or composed event output is returned. | Refuse the run at the shared reference check, before emitting the document or dispatching the composed event. |
| R13-F3.4 | Validating CLI and public report consumers outside composition recovery | Malformed link and image, default validation and FileTree controls | The public report records an error severity and the failure class, and `md validate refs` and `md graph --validate` exit 1. The reference validation APIs take no compose recovery flag. | Retain the report and error verdict. |
| R13-F3.5 | Recovery and deny controls | Missing link and image, empty link, malformed file/code/URL target, missing file target, and suppressed TOC | Missing links and images recover with their flags. The empty link still warns. Malformed file, code and URL targets still fail with `--allow missing-transclusions`. A missing `::file` recovers and a suppressed TOC stays clean. | Preserve those results. |

Limit recovery to issues whose failure class is a missing target, then apply the allowance for that reference kind. Both composition routes use the same decision. Keep the existing empty-target warning and deny behavior.

## Blocked Findings

None.

## Recurrence

- **R13-F1** is a sibling of **R12-F1** and **R11-F1**. Review 12 covered path spelling and TOC selection. Its sweep stopped before the Markdown file-type gate and left the URL directive’s target on the generic hyperlink path. The new sweep follows both decisions into graph traversal, validation, FileTree, and the remote batch.
- **R13-F2** is a sibling of **R12-F2**, including ledger row **R12-S6**, and of **R11-F2**, which followed earlier location failures in **R1-F2** and **R2-F2**. The repair covered the general inclusion loader and the TOC selector. The full graph and composition each read headings earlier, through a separate path. Both paths now have reproductions alongside the correctly located transclusion-only graph.
- **R13-F3** is a regression from the located hyperlink and image repair from review 12, recorded as **R12-S10** in the [iteration 12 ledger](log.md#implementation-of-review-findings-12). The repair tested error severity and the default exit status. The decision after issue construction also needed checks with recovery enabled, in both composition routes. The sweep includes malformed and unreadable targets, real misses, empty targets, and inclusion controls.

## Input Robustness Matrix

For the catalog, manifest and companion loaders, the default for an invalid shape is a located refusal before a profile is applied or anything is published. The three findings above occur after an inclusion string or hyperlink has been parsed. The matrix below covers the file-format readers.

| Validator or loader | Controls, exceptions and verification |
| --- | --- |
| Callable catalog envelope and marked YAML loader | Omitted and empty descriptions pass. Categories and types are required and nonempty. Tests cover null and wrong-type values, mixed and wholly invalid entries, invalid union arms, helper-only tables and conflicting categories. Quoted `"null"` remains type syntax. The YAML loader rejects duplicates, malformed content and trailing documents, and accepts comments, aliases and CRLF within the supported shapes. CLI and editor tests check primary and related locations for conflicts. [Public envelope tests](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs) · [Catalog semantic tests](../../lib/tests/l1/callable_catalog_validation.rs) |
| Manifest admission and marked YAML reader | Required fields have no deserialization defaults. An explicit `files: []` passes with its lifecycle digest, while an omitted list is refused. Null and wrong-type fields use shape validation. An unsupported integer version gets the upgrade message before member reads. Verified member tests distinguish bad collection elements, missing or extra digests, and missing or stale companions. The legacy decoder is confined to publication cleanup. [Snapshot cases](../../lib/tests/l1/host_registration_snapshot.rs) · [Located field tests](../../lib/tests/l1/host_registration_locations.rs) |
| Strict facts JSON reader | Required nullable recovery fields accept explicit null. Empty alias, narrowing and exclusion lists are valid, while an empty exclusion-kind list or missing function entry is refused. The shared field checks distinguish wrong types, null and missing values and reject invalid elements. The public registration tests cover duplicate keys, trailing data and JSON5 syntax, as well as a non-ASCII column before a failure. [Facts reader](../../lib/src/markdown/compose/expression/host/artifact.rs) · [Public companion tests](../../lib/tests/l1/host_registration_companions.rs) |
| Frontmatter inclusion fields | Both keys use the shared string-or-list validator. Omission and an empty list add nothing, an empty string is inline text, and scalar and singleton-list forms admit the same file. Null, numeric, mapping, and mixed or entirely invalid list contents fail at the key. The new location finding is a file read after selection. [Frontmatter tests](../../lib/tests/l1/reference_malformed_inclusions.rs) |

I checked the changed readers for defaulting, optional reads that could conflate absence and null, and filters or error suppression on load-bearing values. The optional authored-spelling lookup in the callable reader supplements its admitted YAML table with example and location data. Companion path prediction used for refused-bundle watches is separate from strict admission of the verified inventory.

## Verification

Level 1 is sufficient for the spec’s acceptance checks. The interfaces under test are schema and runtime results, the editor protocol, CLI reports and effects on files. Terminal rendering and keyboard input do not determine those results.

| Acceptance check | Level 1 evidence |
| --- | --- |
| Preserve built-in signatures, aliases, binding, data reads and lifecycle imports, with fresh artifacts | The binding, permission, runtime, lifecycle and shipped-artifact tests establish the existing contracts. The implementation record has the generated lifecycle comparison. [Network permission tests](../../lib/tests/l1/network_permission.rs) · [Shipped artifacts](../../lib/tests/l1/shipped_artifacts.rs) · [Generation record](implementation-log.md#generated-parity--caller-skill-publication) |
| Agree on overload, duplicate-key and folded-name errors between editor and CLI. Refuse callable catalogs as Markdown schemas. | CLI JSON, editor protocol, and Simplified Schema public admission tests passed, including primary and related positions and the refusal to use the catalog as a document schema or import. [CLI catalog tests](../../cli/tests/l1/schema_validate_yaml.rs) · [Editor catalog tests](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Select host expression facts per document, refuse a bad companion, reload a repair and execute only in the host | Public registration, CLI host scope and runtime, and editor reload tests passed. [Companion tests](../../lib/tests/l1/host_registration_companions.rs) · [Editor reload](../../dmls/tests/l1/host_catalog_reload.rs) · [Runtime binding](../../cli/tests/l1/host_runtime_scope.rs) |
| Agree on publisher, listing and doctor inventory, with an in-place upgrade and unsupported-version guidance | Publication and doctor suites passed. The v3 snapshot tests cover the accepted inventory and earlier-format diagnostics. [Publication](../../../claudine/lib/tests/l1/host_publication.rs) · [Doctor](../../../claudine/cli/tests/l1/doctor.rs) · [Version and inventory](../../lib/tests/l1/host_registration_snapshot.rs) |
| Refuse host mutations and preserve built-in action scope and operands | The installed mutation member is refused at its kind. Existing event and operand checks and the host-runtime test distinguish a call in an action operand from a host-added action key. [Mutation refusal](../../lib/tests/l1/host_registration_companions.rs) · [Runtime and operand cases](../../cli/tests/l1/host_runtime_scope.rs) |

The focused Darkmatter runs passed 662 and 112 tests, including the previous repair’s public-reader and CLI suites, the human diagnostic snapshots and lifecycle schema checks. Simplified Schema passed 31 targeted tests and 2,202 minimal-feature tests. Claudine passed 23 publication tests, 26 doctor tests and the missing-include regression test. The Biscuit Terminal status-block test passed. None of those runs reported a test at or above five seconds. The later CLI and public API probes used temporary fixtures and a loopback server. The implementation’s full-suite results are recorded in the iteration 12 log. I corrected that log’s broader claims about target admission, unreadable child locations and exit status with the reproduced results. [Dated record corrections](log.md#review-13-record-corrections-2026-11-25)
