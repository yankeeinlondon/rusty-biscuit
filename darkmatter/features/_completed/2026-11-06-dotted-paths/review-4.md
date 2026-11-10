---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-07T02:41:55-08:00
spec: 2026-11-06-dotted-paths/spec.md
implemented: false
description: 'A **feature** review of `2026-11-06-dotted-paths/spec.md`'
feature: 2026-11-06-dotted-paths/review-4.md
previous: 2026-11-06-dotted-paths/review-3.md
---

# Review 4: Dotted Paths

The feature is production ready under this review's criteria. Every previous finding is verified fixed, including the schema-directory discovery reader missed by the earlier read-once repairs. No new finding or decision requiring human input remains.

Reviewed the specification, all three previous reviews, the repair Finding Ledgers in log.md, implementation-log.md, and the implementation and tests. No earlier review contains author rulings under a Human rulings heading. Informational decisions already recorded in earlier reviews are not reopened. This review changes only review documents and the requested spec frontmatter; the feature stays in its active directory.

## Previous Finding Verification

Required instances include clean controls and additional sweep rows in the repair ledgers. Outcomes below come from current source inspection and fresh execution, not the implementation's completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R3-F1: Schema-root discovery bypasses completion read retention | R3-F1.1–.14 and R3-F1.S1–.S4 | **verified fixed** | Every instance is checked in the table below. Eight new completion tests pass; discovery now uses the retained reader inside the existing completion scope. | Nothing. This closes the newly found sibling from review 3, rather than reclassifying it as a partial catalog repair. |
| R1-F1: Setter binding and completion use the wrong baseline selection; subsequently carried for repeated catalog reads | R1-F1.1–.24 and expanded R1-F1.22.1–.14 | **verified fixed** | Selected-baseline public and CLI cases pass for built-in/custom/disabled selection, flag/environment policy, document and trigger layers, raw JSON declarations, recursive definitions, and fallback. Catalog and cross-layer read-count tests pass. Current selection and declaration projection retain the request's baseline. | Nothing. The additional discovery sibling is separately closed as R3-F1. |
| R2-F1: Optional shell script tests pass without loading a script | R2-F1.1–.8 | **verified fixed** | Fish and Elvish loaders pass through the explicit optional-shell recipe. Both use mandatory L1 `expect_level!` prerequisites. Bash, Bash 3, and Zsh loaders pass in the focused L1 run; the Windows loader retains the same mandatory gate. All six local L2 cases pass; each uses `require_level!` for its shell and terminal backend. Windows ConPTY cases retain their mandatory prerequisites. | Nothing. The earlier review's unavailable-shell negative runs are historical evidence; this review verifies the gates in source and reruns the positive controls. |
| R1-F2: Declared values bypass the shared conversion rules | R1-F2.1–.22 | **verified fixed** | Public setter-value and set/compose cases pass for declared unknown text, decoded quoted conversion, containers, native null, scalar unions, pattern keys, raw schema constraints/references, missing required siblings, and persistence. Inspection confirms `set_checked` saves the value validation judged and `partial_copy` retains unrelated root data. | Nothing. |
| R1-F3: Union selection loses the origin of nested file setters | R1-F3.1–.25 | **verified fixed** | Public/CLI compose cases pass for objects, arrays, dictionaries, recursive children, whole overrides, authored siblings, launch-only/document-only files, refused matches, defaults, templates, and transclusion. Inspection follows supplied origins through arm selection, both validation passes, scoped child checks, diagnostics, and presentation. Claudine's caller-file deferral remains in place. | Nothing. Claudine and native-Windows tests were not rerun in this review; their implementation was inspected. |
| R1-F4: Elvish and PowerShell lack the required shell verification paths | R1-F4.1–.6 | **verified fixed** | Local word-shape and script tests pass. Real-terminal tests pass for Bash, Bash 3, Zsh, Fish, and Elvish, including Elvish's measured acceptance fallback. Native-Windows PowerShell script and actual Tab/PSReadLine test paths remain declared and selected by the terminal recipe. | Nothing. Fresh cross-OS proof is outside readiness criteria. Optional-shell prerequisite handling is separately verified as R2-F1. |

### Schema discovery repair: every required instance

In the darkmatter package, [FilesystemRoots::root_file_text](../../lib/src/markdown/schemas/triggers/discovery.rs:525) now obtains schema-directory file text through simplified-schema's scoped reader. In darkmatter-cli, [DocumentSite::load](../../cli/src/args/path_completion.rs:309) already surrounds baseline selection, discovery, and effective-schema assembly with one retention scope. Nested resolver scopes join it; retained text is discarded when that operation ends. No persistent cache or additional filesystem abstraction was introduced.

The new [discovered-root tests](../../cli/src/args/path_completion.rs:588) use a temporary repository, fixture-owned home and environment, a document schema, a repeated named-type dependency, and a matching trigger inside `schemas/`. They assert candidates and the exact observed path/count map. Including the trigger file in that map verifies that discovery itself is observed, addressing the previous counter's blind spot. These tests do not count the input document's read; its separate single-open path is verified by inspection of `open_document` and the existing CLI wiring tests.

| Required instance | Verification evidence | Result |
|---|---|---|
| R3-F1.1: filesystem discovery | `root_file_text` uses `buffers::read_to_string`; the observed set includes the discovery-only trigger. | Fixed |
| R3-F1.2: get → schema/import | `get_reads_each_root_file_once`: offers `status`; schema, type file, and trigger each opened once. | Fixed |
| R3-F1.3: set → schema/import | `set_reads_each_root_file_once`: offers `custom`; same exact one-read map. | Fixed |
| R3-F1.4: remove → schema/import | `rm_reads_each_root_file_once`: offers `status`; same exact map. | Fixed |
| R3-F1.5: compose → schema/import | `compose_reads_each_root_file_once`: offers `status=custom`; same exact map. | Fixed |
| R3-F1.6: custom catalog overlapping discovery | `an_edit_baseline_inside_the_root_is_read_once`: get/set/rm each offer the expected candidate; catalog and shared schema files each read once. | Fixed |
| R3-F1.7: compose flag baseline overlapping discovery | `a_compose_baseline_flag_inside_the_root_is_read_once`: correct value candidate; baseline and shared files once. | Fixed |
| R3-F1.8: trigger payload import | The fixture trigger imports `shared@./types.yaml`; every new enabled-discovery test asserts one read despite discovery and document resolution sharing that dependency. Both discovery orderings use the same scoped reader. | Fixed |
| R3-F1.9: example file overlapping discovery | `an_example_file_inside_the_root_is_read_once`: outside-root schema offers `greeting`; example, schema, and root files each once. | Fixed |
| R3-F1.10: trigger-envelope control | The trigger is present exactly once in each enabled-discovery observed set; its parsed envelope is reused. | Clean |
| R3-F1.11: disabled-discovery control | `compose_without_trigger_discovery_reads_no_trigger_file`: only document schema and type dependency are observed, once each. | Clean |
| R3-F1.12: outside-root control | Existing CLI cross-layer and public baseline/document repeated-import tests pass. | Clean |
| R3-F1.13: raw JSON control | Discovery enumerates YAML/YML only; raw JSON schema resolution uses the retained resolver reader. Public raw JSON declaration and one-read baseline controls pass. | Clean |
| R3-F1.14: host-supplied source control | `SchemaRootSource` remains unchanged; DMLS `RootContents` and `BufferedRoots` still supply their own text. Fresh unsaved YAML target/import/root tests pass. | Clean |
| R3-F1.S1: YAML kind discovery | `KindRegistry::from_filesystem` uses the repaired `FilesystemRoots`; YAML-document and editor registry cases pass. | Fixed |
| R3-F1.S2: remaining direct catalog reads | Baseline loading reads retained text before `from_text`. Direct catalog file factories, DMLS host discovery, facts artifacts, and lifecycle discovery are outside the CLI completion call path. None bypasses this completion scope. | Clean |
| R3-F1.S3: resolver, trigger checks, payload memo, baseline, YAML document schema | Resolution/graph readers, trigger envelope checks, payload fingerprints, and baseline selection use the shared reader. Existing repeated-import, buffer-restoration, payload-memo, catalog, and YAML-document tests pass. | Clean |
| R3-F1.S4: other CLI reads and probes | Frontmatter editor/validator reads are execution paths, not completion. File/directory selection probes remain metadata enumeration permitted by the spec. | Clean |

The shared-reader change also makes filesystem discovery respect an already-open source buffer. That matches the resolver's existing contract. DMLS supplies its own source implementations and retains authority over unsaved text.

## Unblocked Findings

None.

## Blocked Findings

None. No prerequisite or human decision remains outstanding.

## Recurrence

No finding in this review repeats an earlier defect class. The repeated-read instances from reviews 1–3 are all closed; no newly missed sibling or repair regression was found in the bounded sweep.

## Requirement-to-Verification Mapping

| Requirement | Appropriate level | Current evidence |
|---|---|---|
| Shared path grammar, escaping/round trips, empty and Unicode keys, numeric mapping keys versus canonical array indexes, huge indexes, scalar/null parents | L1 | Public `property_paths` suite and CLI path cases pass; typed traversal supplies schema steps and diagnostic pointers. |
| Missing parents, last-child containers, removal conflicts and original indexes; failed edits write nothing | L1 | Public edit plans, source editor, and get/set/rm tests pass. |
| Nested sibling comments/bytes, flow fallback, LF/CRLF, alias refusal, body/fences, persisted read/write/read | L1 | `path_edit`, property-path, and command cases pass. Inspection confirms full-result reload/comparison before writing; fallback cannot bypass an unnamed-value refusal. |
| Flat multi-get output, paths as written, top-level remaining keys, stdout/save/stdin contracts and output envelopes | L1 | `get_set_rm`, `property_paths`, and output conformance tests pass. |
| Shared JSON5/YAML-flow values; declared precision, quotes, null, unknown, numberlike/boolish/unions, admission errors | L1 | Public `setter_values` and command/pipeline parity cases pass. |
| Setter precedence/specificity, whole-container replacement, selected baseline/schema/triggers, changed-value validation, passive pending values and caller origins | L1 | Public/CLI compose and edit cases pass; validation and origin paths inspected. |
| Candidate source/order/spelling, descriptions, union narrowing, earlier words, no-input/failure fallback, file values, no command/network/evaluation | L1 | Candidate and shipped completion-entry tests pass, including observable denied-effect checks. |
| Each distinct schema/document content file read once | L1 observable operations | New discovery-overlap maps and existing catalog/import maps pass; input-document opener and remaining reader call paths inspected. |
| Generated script loading and shell word shapes, including Bash's split equals | L1 | Platform-selected local loaders/word tests and both explicit optional-shell loaders pass. Windows script loader inspected. |
| Actual Tab replacement/continuation and documented shell limits | L2 | Six local tests pass in detached tmux. Native-Windows PowerShell uses windowless ConPTY and reads PSReadLine's actual input buffer; its test path was inspected, not rerun. |
| Representative release responsiveness | One recorded release sample | Original implementation measurement remains in implementation-log.md: about 40 ms outside a repository and 80 ms inside this monorepo. No new timing threshold or benchmark requirement is introduced. |

No required behavior is supported only by a test at an insufficient level. OS keyboard injection is unnecessary here: the requirement concerns the shell's handling of Tab and completion candidates, not terminal keyboard encoding. No terminal or browser focus was taken.

## Input Robustness Matrix

This applies to the shared setter container loaders, not the dotted-path text grammar or source-preserving writer. The latest repair introduces no field validator or format loader.

Default outcome: duplicate keys or inadmissible numbers in an admitted container fail without a write. Syntax rejected by both parsers may remain text, subject to declared-type validation. Exceptions preserve absent members as absent, explicit null as data, empty input/collections as empty, and quoted strings decoded once before declared conversion.

Fresh public `SetterValue` tests exercise both JSON5 and YAML-flow loaders: absent/null, wrong whole-field type, mixed/all-invalid elements, empty fields, duplicate keys, malformed/trailing content, JSON5 comments/trailing commas, YAML extra documents/tags/anchors, and numeric overflow/underflow. Invalid elements remain present for validation; they are never filtered into success. CLI tests verify successful controls, declared binding, persistence, and refusal without a write.

The source-smell search found no load-bearing default or element-filtering parse in this reader. Its numeric `.ok()` follows admission and fails the whole YAML container conversion rather than dropping a member. Advisory completion deliberately omits invalid unfinished words or unavailable schema sources; that is its specified fallback policy.

## Fresh Verification

| Command | Result |
|---|---|
| Darkmatter `just test path_candidates:: path_completion:: compose_setters:: get_set_rm:: property_paths:: path_edit setters::tests output_commands:: payload_memo yaml_document catalog` | 1,011 passed; none reached the five-second L1 limit. Includes all eight new discovery cases. |
| Simplified-schema `just test setter_values property_paths buffers` | 86 selected tests passed; the recipe's subsequent minimal-feature suite also passed all 1,298 tests. |
| Darkmatter `just test-optional-shells` | Fish and Elvish loaders: 2 passed. |
| Darkmatter `just test-l2 level2_path_completion` | 6 CLI cases passed in real shells; other package targets select no matching tests. |
| `just check-tier-coverage darkmatter` and `just check-tier-coverage simplified-schema` | Zero stranded tests in either area. |

The new tests compile in the CLI's live unit-test target and run at L1; optional shell loaders and real-terminal cases retain their deliberate feature/tier selection. Builds emitted the existing macOS large-unwind-table linker warning. Full-area L1/lint, Claudine, and cross-OS runs were not repeated for this review; prior implementation results are not represented as fresh evidence.
