---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-12.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
previous: "2026-11-22-expression-and-mutation-schemas/review-11.md"
next: "2026-11-22-expression-and-mutation-schemas/review-13.md"
created: "2026-11-25T19:49:28+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "claude/opus"
findings:
    - "R12-F1 (high): Reference validation applies hyperlink rules to inclusion targets"
    - "R12-F2 (high): Inclusion target errors lose the authored file and line"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: true
---

# Expression and mutation schemas review 12

The feature is not production ready. The previous review’s listed cases pass. Two newly found siblings remain. A reference check can accept a file inclusion that composition cannot open, and it can check an optional TOC target that composition suppressed. An invalid or missing target can also lose the location of the directive that named it. Both repairs fit the existing rules.

## Previous Finding Verification

I checked the four findings in review 11 against the implementation, the Level 1 results, and the public API and CLI probes described below. The earlier review tables close the remaining findings from iterations 1–10. There are no pending rulings. [Review 11](review-11.md) · [Iteration 11 ledger](log.md#implementation-of-review-findings-11)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R10-F2 Reference validation drops malformed transclusions | R10-F2.1–.18, with .11 and .16–.18 open in review 11. Additional code-projection checks R11-S1–S4. | **verified fixed** | Malformed directives fail in the public readers. The specific code survives tolerated warnings, condition failures and the FileTree boundary. The fixtures for missing transclusions, heading overflow, malformed remote frontmatter and preflight failures passed. | None for the code-loss instances. R12-F1 and R12-F2 cover newly found failures in target choice and location. |
| R11-F1 Reference checks misclassify inline and suppressed inclusions | R11-F1.1–.4 | **verified fixed** | Inline content and its links or images are distinguished, and the ordinary local-path TOC cases retain the three selection outcomes. Both graph modes, the local query, public validators and CLI tests passed for the listed fixtures. | R12-F1 is a new sibling exposed by targets that look like hyperlinks. |
| R11-F2 Inclusion diagnostics point to body-relative lines | R11-F2.1–.5, plus R11-S5–S6 | **verified fixed** | Directive grammar faults reach the public results at the authored opener, including after a removed page block. Frontmatter-shape, condition, shell and page-block controls passed. Unknown positions are omitted in the dedicated test. | R12-F2 is a new sibling where the directive parses and its file target fails. |
| R11-F3 FileTree loses the admitted source when checking root headings | R11-F3.1–.3 | **verified fixed** | FileTree, fresh and prebuilt validation accepted the generated heading and refused its missing control in all four source forms, with both bare and explicit schemas. The loaded-file CLI and descendant controls passed. | None. |

| Ledger instance | Verification evidence |
| --- | --- |
| R10-F2.1–.5 | The malformed-inclusion suite exercised missing targets, invalid TOC level and file-list depth, both frontmatter keys and output from both graphs, local queries, flattening and validation. Direct CLI probes refused bare `::file`, `::code` and `::url`, plus `level=h9` and `--depth two`. |
| R10-F2.6–.10 | The inclusion-defer, boolean detector, bad-child YAML, fragment and condition tests passed. The CLI distinguished a false condition, a parse failure and an evaluation failure. Present and absent child and anchor controls reached their expected results. |
| R10-F2.11 and .16 | The typed projection and CLI tests passed, including denial of the matching tolerated warning. Direct compose produced warning codes `dm.toc_linking.invalid_filter`, `dm.file_links.invalid_glob`, `dm.reference.failure`, `dm.file_links.not_a_directory` and `dm.input.read_failure` for a broken filter, malformed glob, missing directory, file used as a directory and unreadable directory. The stdin test kept `dm.file_links.missing_source`. [CLI coverage](../../cli/tests/l1/validate_refs.rs) · [Reader example](../../docs/errors/composition-problems.md) |
| R10-F2.12–.15 | Frontmatter shape failures kept the key line, and a matched inclusion cycle and the depth-boundary controls retained their prior results. Precheck and prefetch leave authoritative parse errors to the readers. The existing local-path TOC failure and suppression fixtures passed. |
| R10-F2.17 | `when="1 +"` returned `dm.expression.parse_failure` through compose, both graph modes, flattening and validation. `when="1 / 0"` kept its evaluation code. The raw query kept the authored directive. |
| R10-F2.18 | Built `md graph`, `md graph --validate` and `md validate refs` agreed on `dm.reference.failure` and `invalid-reference` for `::file @` and `prologue: "@"`. R12-F2 records the missing position separately. |
| R11-F1.1 | `reference_inline_and_suppressed_inclusions` checked prose, inline links, multiline text and empty strings for both keys and both scalar and list spellings. The graph records inline content and the raw query leaves its resolved target empty. File strings still produce inclusion edges. |
| R11-F1.2 | The same module checked flattened output, both validators, FileTree and the compose reference check. Missing inline links and images were reported as their actual reference kinds. Independent CLI probes validated an inline link to `side.md` and rejected one to `gone.md`. |
| R11-F1.3 | The test covered the first existing target, the existing fallback after a miss, all missing local paths followed by `false`, and required missing local paths. Direct API and CLI probes confirmed the last two verdicts. The query lists the target without following it. |
| R11-F1.4 | Existing and missing file and code cases, false conditions, URL discovery and file-list queries passed. Preflight and the editor’s local TOC scanner keep their selection and navigation rules. The editor test rejected a literal prologue as a document link. |
| R11-F2.1–.3 | `directive_authored_lines` and CLI JSON tests exercised the ordinary body directive, TOC and file-list families. Direct runs with a YAML header agreed on the authored line and column. TOC filter warnings kept the opener and file-list discovery still does not collect commands. |
| R11-F2.4–.5 | The removed-block and unproven-position controls passed. The raw query retains the original body map. The frontmatter shape and parse/evaluation conditions stayed located, with valid controls clean. |
| R11-F3.1–.3 | Both `reference_root_heading_admission` tests passed, across the four admitted source forms and three validators. The CLI fragment and graph controls, request-only local-query tests and the separately prepared descendant heading tests also passed. |
| R11-S1–S4 | `typed_diagnostic_codes`, `network_permission` and `directive_authored_lines` passed for missing targets, normalization and transclusion heading overflow, invalid remote frontmatter, block command tokenization and remote preflight fetching. |
| R11-S5–S8 | The shell, shell-block and page-block location checks passed. The two plain-message transforms in S7 do not discard a typed cause. The required-TOC control in S8 was too broad. The built validator rejects bare `@` but accepts a required URL or a path with a fragment. The implementation log now records the observed results. |

## Unblocked Findings

### R12-F1 (high) Reference validation applies hyperlink rules to inclusion targets

**Authority (defect).** Given an existing `side.md` with heading `# Side`, `md validate refs --fragments` and `md graph --validate` accept `::file ./side.md#side`. Composition attempts to open the complete filename `side.md#side` and fails. A document author copying a heading link into an inclusion can get a passing check for a document they cannot compose. TOC fallback chains lose the same distinction. A required URL can pass validation even though the TOC selector rejected it, and a suppressed URL still sends a request under `md validate refs --remote`. [Graph record construction](../../lib/src/markdown/reference/graph.rs#L734) · [TOC selection](../../lib/src/markdown/reference/graph.rs#L846) · [Reference decisions](../../lib/src/markdown/reference/validate.rs#L447)

The defect class is discarding the inclusion selector’s interpretation or result and reclassifying its input as a hyperlink. Quoting the same target should preserve the result. Appending `#side` to a file inclusion should change its lookup to a different filename. Appending `false` to an exhausted TOC chain should suppress all validation of that chain. The probes used one guide, a local `side.md` containing `# Side`, and a loopback HTTP server for the remote-validation control. Fresh and prebuilt validation and both FileTree views reproduced the same verdicts wherever they used the full graph.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R12-F1.1 | Body `::file` and `::code`, both graph modes, local query, flattened output and public validation | `./side.md` → `./side.md#side`, including the quoted spelling | Compose reads the control and fails the changed filename. The local query also finds only the control. The full graph records the changed target as a local path, and every validator strips the suffix and passes. The transclusion-only graph follows no changed `::file` child. | Validate the complete filename the inclusion uses. Report an absent target even if its prefix and matching heading exist. |
| R12-F1.2 | Frontmatter `prologue` and `epilogue`, both graph modes, local query and public validation | Each key with `"./side.md"`, then `"./side.md#side"` | Both control cases add a child. Neither changed case adds a child or resolves in the local query. Every validator passes the changed case and composition refuses it. | Apply the same filename rule to both fields. Keep inline Markdown and its regular links working. |
| R12-F1.3 | Required TOC outcome through the full graph, flattening, public validators and CLI | `./side.md`, `./gone.md`, `./side.md#side`, `#here` and an HTTP URL. Keep an existing `# Here` in the root. Quote and unquote the changed file target. | The normal file succeeds and the missing plain file is reported. The three hyperlink-shaped targets yield no child and preflight refuses them. Every validator passes them. `md validate refs --remote` sends a HEAD request for the URL and returns success after a 200. The generic-scheme and data-URI branches also accept a required TOC target when it is `mailto:docs@example.invalid` or `data:text/plain,guide`. | Preserve the required failure from the selector in every validation mode. The HTTP result and heading table have no bearing on this decision. |
| R12-F1.4 | Suppressed TOC outcome through local-path, fragment and remote validation, including the request batch | `"./gone.md \| false"`, `"#absent \| false"` and `"<loopback URL> \| false"`. Use `FALSE` as an equivalent spelling and then remove it. Also test `false` alone. | Composition suppresses each optional chain. Plain paths and `false` alone pass. Fragment validation reports a missing heading for the optional fragment. The optional URL produces a remote warning by default and sends a HEAD request with `--remote`. Removing `false` does not restore required-failure behavior for the URL. | Suppression applies before target-kind checks and remote request collection. A required failed chain remains an inclusion error. |
| R12-F1.5 | Selected fallback, passive command discovery, authored local query and editor navigation | URL first, then existing `./side.md`, then `false` | Both graph modes follow the local file. The public validators accept it. Preflight selects the local path. The query lists the raw alternatives without following them, and the editor’s passive resolver selects local files for navigation. | Keep these boundaries and fallback order. |
| R12-F1.6 | Neighboring reference readers | Plain links to `#here`, `./side.md#side` and the loopback URL, plus `::url`, valid file-list queries, inline frontmatter links and false `when=` | Ordinary fragment links resolve, ordinary URL validation can send a request, and the explicit URL directive remains a remote reference. Listing and false conditions keep their separate outcomes. The suite also confirms that links generated by TOC validate against the target’s raw headings. | Preserve hyperlink, remote inclusion, listing and generated-link behavior. Ordinary remote-capable inclusions keep their existing network policy. |

Keep the target semantics that the graph already knows. Use the selected TOC outcome before validation branches on target kind, including before the remote batch. For local inclusions, resolve the whole filename through the same file-reference rule as composition. Preserve the authored spelling for display. One extension to the current public API matrix and one CLI control for each affected validation boundary can pin this without a new parser or a network-policy change.

### R12-F2 (high) Inclusion target errors lose the authored file and line

**Authority (defect).** A document containing `::file @` or `prologue: "@"` produces a specific invalid-reference error but no authored line. If a nested file contains the incomplete target, raw diagnostics from the graph or command discovery name neither the nested file nor the line. A reader who follows the CLI diagnostic can land in the parent even though the child contains the typo. The same loss affects a routine missing target in direct library composition. [Reference resolver](../../lib/src/markdown/reference/mod.rs#L34) · [Reference diagnostic projection](../../lib/src/markdown/compose/report_diagnostics.rs#L981) · [Preflight inclusion walk](../../lib/src/markdown/compose/preflight/collect.rs#L1138)

The defect class is propagating a typed target error after dropping the located directive or frontmatter site. Quoting the same invalid target keeps the error. Putting a removable page block above it must leave the location on the authored opener. Replacing it with an existing target clears the error. I ran the built commands with a YAML header and repeated the checks through public graph, query, validation, preflight and compose results.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R12-F2.1 | Body `::file`, `::code` and `::url` target failures in compose, preflight, both graph modes and validators, and local query | `@` at line 8, then quoted `'@'` under an earlier false page block. Existing path or well-formed URL as the control. | CLI compose and both validating commands emit `dm.reference.failure` with a null line. The direct diagnostics for the failing `::file` graph builds, and `::file` and `::code` local queries, have no source. Direct preflight and compose also produce unlocated errors. A graph built without validation can list code and URL targets without judging them, and the raw query lists a URL. The valid file control clears the error. | When a reader refuses a target, preserve its typed cause and publish the known opener. Preserve the listing boundary. |
| R12-F2.2 | Frontmatter target parsing and resolving in graph construction, raw query, preflight, composition and validation | Both keys with `"@"` on line 3, then `"./side.md"`. Control with an invalid YAML value shape. | All paths that refuse the incomplete target lose line 3. The public resolver failures themselves have no source. The shape error still uses the key line and the valid target succeeds. | Preserve the authored key or entry location for the target error as well. |
| R12-F2.3 | Missing targets in direct compose and preflight, and their reference-check neighbors | `./gone.md` → `./side.md` in `::file`, `::code`, prologue and epilogue | The direct library failures carry `dm.reference.missing_transclusion` with no source. The current CLI check can supply a location when it first reports a missing-target issue from the graph. | The direct errors should also carry the known site and keep the existing code. |
| R12-F2.4 | Nested graph build, preflight and validating CLI commands | Root includes `nested.md`, which writes `::file @` at line 7 | Both public graph builds and command discovery return a diagnostic without any authored source. The CLI falls back to the input document with no line. | Name `nested.md`, line 7. Retain file-only output if a rewrite genuinely prevents line mapping. |
| R12-F2.5 | Required TOC error through graph validation versus preflight and composition | Bare `@` as the required alternative, then an existing file | Direct preflight retains the TOC opener at line 8. The graph’s validation path presents the raw target again and emits an unlocated invalid-reference error. | Keep the located selector failure or issue site through the graph path. Use the target-outcome repair in R12-F1 here. |
| R12-F2.6 | Neighboring parsers and projections | Invalid directive grammar, invalid TOC level, file-list depth and glob, malformed `when=`, malformed shell or page block, bad child YAML and ordinary missing inline link | The existing parser- and condition-location tests pass. Direct CLI probes retained the file-list warning and directive positions. Child YAML errors name the child whose bytes failed. Missing inline links use the frontmatter site. | Keep these locations, recovery choices and codes. |

Carry a site on the typed target failure at the call where the inclusion is known, using the existing authored-opener mapping for body directives and the key or entry location for frontmatter. The reference diagnostic can then reuse that site across fresh and prebuilt graphs, command discovery and the CLI. Avoid replacing a known child site with an input-file fallback.

## Blocked Findings

None.

## Recurrence

- **R12-F1** is a sibling of **R11-F1**, which itself extended **R10-F2** beyond parser failures. Review 11’s repair swept inline content and TOC chains whose alternatives were ordinary local paths. Its suppression check sits inside the local-path branch. The other branches and the remote batch still re-decide the selection. The same sweep now follows filename-shaped inclusion results into fragment validation, including both frontmatter keys and the file and code directives.
- **R12-F2** is a sibling of **R11-F2** and the earlier location-loss findings **R1-F2** and **R2-F2**. The previous repair kept the authored site through the directive grammars and the shell and page-block paths. The reconciliation for R10-F2.18 verified the code and failure kind for a malformed target; it did not verify the source on that same path. The probes here follow the typed error from resolution through graph construction, validation, local query, preflight and compose, including a nested file.

## Input Robustness Matrix

The original work adds strict YAML and JSON readers. In the existing public-result matrices, the default for an invalid shape is a located failure before a catalog or registration takes effect. The important exceptions and decision boundaries are below. The inclusion directive text grammar is covered by the finding probes. The frontmatter inclusion value is the one relevant field validator changed in the repair.

| Validator or loader | Controls and exceptions verified |
| --- | --- |
| Callable catalog envelope and its marked YAML loader | An omitted description and an explicit empty description both pass. Categories and types are required and nonempty. The shared tests exercise null, wrong types, mixed and all-invalid category entries, empty and null type entries, and the distinction between a YAML null and the quoted type spelling `"null"`. Unknown and duplicate keys and trailing documents are refused. Comments, CRLF and alias spellings retain their meaning. CLI and DMLS tests also assert authored and related positions for duplicate keys and overloads. [Envelope suite](../../../simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs) · [DMLS protocol suite](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Manifest admission and its marked YAML loader | All seven fields are required. An empty `files` list is valid and still requires a lifecycle digest. An unsupported integer version stops before member reads. Null or non-integer values use the shape check. Member lists and digest maps are checked before snapshot assembly. Installed and embedded controls exercise missing, stale, extra and forbidden companion paths. The shared marked loader covers duplicate keys and trailing documents. [Snapshot suite](../../lib/tests/l1/host_registration_snapshot.rs) |
| Strict expression facts JSON reader | The valid writer output preserves its explicit null and empty values. The field validator checks required fields and rejects wrong whole-field and element shapes. The public registration tests check missing or stale data and refusal locations, and reject duplicate keys, trailing bytes and JSON5 syntax. [Companion suite](../../lib/tests/l1/host_registration_companions.rs) |
| Frontmatter inclusion value validator | An omitted value and an empty list are no-ops. An empty string is inline text. A file scalar and a singleton list take the same route. Null, number, mapping, and mixed or all-invalid lists are rejected at the key. Both keys run through the shared validator. The R12 findings concern what happens after valid string values pass this check. [Shape tests](../../lib/tests/l1/reference_malformed_inclusions.rs) |

I checked the load-bearing reads for defaults, optional values and silently discarded entries. The catalog reader’s optional raw-spelling lookup supplies example and location data after strict admission. It does not replace the admitted type table.

## Verification

Each acceptance check needs Level 1 evidence. The observed contracts are schema results, runtime values, file publication and diagnostics over CLI or editor protocol. None depends on an emulator’s rendering or keyboard encoder.

| Spec acceptance check | Evidence |
| --- | --- |
| Preserve built-in signatures, aliases, binding, data reads and lifecycle imports, with fresh shipped artifacts | Level 1 descriptor, binding, data-permission, lifecycle end-to-end and artifact freshness tests passed. The original implementation record documents the exact generated lifecycle comparison and caller-skill publication. [Artifact freshness](../../lib/tests/l1/shipped_artifacts.rs) · [Generation record](implementation-log.md#generated-parity--caller-skill-publication) |
| CLI and editor agree on invalid overload, duplicate key and folded-name collision. Callable catalogs cannot validate Markdown frontmatter. | Level 1 CLI JSON, DMLS protocol and Simplified Schema corpus tests passed, including both positions for conflicts and catalog role refusal. [CLI catalog validation](../../cli/tests/l1/schema_validate_yaml.rs) · [Editor catalog validation](../../dmls/tests/l1/yaml_document_catalog_editor.rs) |
| Selected-host analysis, facts refusal and recovery, and host-only execution | Level 1 CLI host-scope and public runtime tests passed. DMLS revision and reload tests showed removal, staleness and restoration across open documents. [CLI host scope](../../cli/tests/l1/compose_host_scope.rs) · [Editor reload](../../dmls/tests/l1/host_catalog_reload.rs) · [Runtime binding](../../cli/tests/l1/host_runtime_scope.rs) |
| Publisher, listing and doctor agree, with an in-place format upgrade | Level 1 publication and doctor tests cover exact inventory, missing or modified companion repair, unsupported version and upgrade at the existing path. [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs) · [Doctor CLI tests](../../../claudine/cli/tests/l1/doctor.rs) |
| Hosted mutations are refused and existing event and operand checks remain | Level 1 registration refusal at `kind` and lifecycle schema, event and operand tests passed. The runtime check distinguishes a host function used as an operand from a lifecycle action. [Registration cases](../../lib/tests/l1/host_registration_companions.rs) · [Runtime cases](../../cli/tests/l1/host_runtime_scope.rs) |

I ran the built CLI against temporary fixtures and linked the public API probes to the checkout’s compiled Darkmatter library. The URL validation probe used a loopback server.

- Darkmatter `just test --max-fail all` passed 18,246 tests, with 10 skips and zero tests at or above five seconds.
- Simplified Schema `just test --max-fail all` passed 2,626 default-feature tests and 2,202 minimal-feature tests, with zero skips and zero tests at or above five seconds.
- Claudine `just test --max-fail all` passed 13,077 tests, with 16 skips and zero tests at or above five seconds.
- The tier audit found zero stranded tests across the three package areas. This review validates against `feature-review.yaml` and composes with `--deny all`.
