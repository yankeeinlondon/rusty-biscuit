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
created: "2026-11-07T19:07:02-08:00"
spec: 2026-11-05-render-surface/spec.md
implemented: false
description: "A **feature** review of `2026-11-05-render-surface/spec.md`"
feature: 2026-11-05-render-surface/review-2.md
previous: 2026-11-05-render-surface/review-1.md
---

# Render surface review, iteration 2

The feature is **production ready** under the specification's review criteria. All four previous findings are verified fixed. No new findings, remaining defects, or decisions requiring a human were identified.

This review checked the specification, the first review, the original implementation log, and the repair log's Finding Ledger against the current implementation and focused test runs. The existing working-tree repairs were reviewed in place; this review changes only review documents and specification metadata.

## Previous Finding Verification

The required instances include the clean controls and the additional siblings recorded during repair. The evidence below is from code inspection and tests rerun for this review, rather than completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Early failures and selective denial discard style diagnostics | R1-F1.1–R1-F1.15: file opening, malformed data, offsets, remote/unsupported references, setter paths, earlier-file warnings, both selective denials, type/duplicate failures, family denial, success, formatted output, schema selection failure, source-output retry | **verified fixed** | In darkmatter-cli, `read_cli_style_inputs` retains the trigger findings and previously read layers. `prepare_read_render_style` obtains their warnings alongside the read failure. The render handler publishes these warnings before returning the failure and preserves unblocked warnings during selective denial. All 15 tests in `render_style_carried_warnings` passed, including original document line/column assertions and formatted stderr/no-page assertions. The source-output retry preserves first-pass document diagnostics on its failure branch; that branch has no reachable reproduction identified, as explained below. | None |
| R1-F2: Whole-style setter type failures use the wrong diagnostic family | R1-F2.1–R1-F2.13: malformed/scalar/null/array roots; absent and empty controls; page, scalar, union, cleanup and metadata fields; file roots; documentation; schema-free root | **verified fixed** | In darkmatter, `assemble_setters` uses the shared `TypeMismatch` diagnostic code for every nonmapping whole-style setter. The public preparation matrix checks each root shape and adjacent container/scalar cases, and keeps file-root errors separate. These tests passed in `just test style_layer`; the shipped CLI's malformed whole-style test passed in `just test render_style`. Current render and data-contract docs describe the corrected codes. | None |
| R1-F3: Superseded duplicate-key setters still fail parsing | R1-F3.1–R1-F3.10, plus R1-F3.11 described in the repair log's progress section: root, border union, metadata, cleanup, scalar, malformed root, surviving duplicates, separate files, CLI wiring and custom fields | **verified fixed** | In darkmatter, `assemble_setters` now chooses the final assignment for each decoded exact path before invoking `SetterValue::parse`. Public preparation tests cover all raw value families, assert effective/retained values, and preserve rejection of surviving duplicates and invalid earlier files. The CLI replacement test passed. Equivalent-path conflict tests also passed, so replacing repeated exact paths has not made different equivalent spellings order-dependent. | None |
| R1-F4: Typo hints ignore the document's effective style schema | R1-F4.1–R1-F4.8: custom and nested keys, rejected built-in suggestions, baseline/schema-free fallback, file/document rename hints, pattern admission and CLI wiring | **verified fixed** | In darkmatter, `AdmittedKeys` queries the effective schema through shared schema completion facilities. It supplies sibling candidates and filters rename hints against admitted paths. All eight `style_layers::suggestions` tests passed, as did the CLI custom-schema hint test. A rejected built-in key is no longer offered as a correction; an admitted pattern key remains valid without an unknown-key error. | None |

### Instance checks

These groups account for every ledger child, including the clean siblings that must stay working.

| Instances | Checked result |
|---|---|
| R1-F1.1–R1-F1.5 | Missing and malformed files, nonmapping offsets, remote references, unsupported extensions and invalid setter paths retain document and trigger warnings. Remote references are rejected during classification before any file opening or transport. |
| R1-F1.6 | An earlier valid file's unknown-key warning survives a later file's malformed-data failure. |
| R1-F1.7–R1-F1.8 | Denying either warning code retains the other warning and trigger warning, with the denied record reported as an error. |
| R1-F1.9–R1-F1.12 | Type errors, duplicate-key parse errors, family denial and successful preparation retain their required diagnostics. |
| R1-F1.13–R1-F1.14 | Formatted failures have empty stdout and applicable warnings on stderr; schema-selection failure retains authored warning positions and trigger findings. |
| R1-F1.15 | Inspected the source-output retry's diagnostic splice. The retry is entered only when every first-pass problem is an ignorable document problem. A CLI-layer problem already prevents entering it. The defensive failure branch retains the document diagnostics; no synthetic test is needed to claim an otherwise unreachable input is covered. |
| R1-F2.1–R1-F2.4, R1-F2.12 | Malformed text, numbers, null, empty arrays and nonempty arrays fail as whole-style schema type mismatches; the schema-free number case uses the same branch. |
| R1-F2.5–R1-F2.6 | No layer produces defaults; an inline empty mapping keeps the earlier document style. |
| R1-F2.7–R1-F2.10, R1-F2.13 | Nested page, scalar, union, cleanup and open-object cases retain their appropriate schema failures; file scalar/array roots keep `dm.style.invalid_root`. Null members remain absence. |
| R1-F2.11 | Render and data-contract docs distinguish setter schema errors from file/offset root failures. |
| R1-F3.1–R1-F3.6, R1-F3.10 | Superseded root, union, metadata, cleanup, scalar, malformed-root and custom-object values are not parsed or converted; surviving values reach public prepared results. |
| R1-F3.7–R1-F3.9, R1-F3.11 | Surviving duplicate objects still fail, a bad earlier file still fails, and CLI replacement succeeds. The progress log names R1-F3.11 even though the ledger has no separate row for it; its `surviving_setter_duplicate_keys_still_fail` test was included. |
| R1-F4.1–R1-F4.2, R1-F4.6, R1-F4.8 | Custom and nested custom typo hints name admitted siblings; the CLI displays the custom correction and never suggests a rejected built-in field. |
| R1-F4.3–R1-F4.5, R1-F4.7 | Baseline/schema-free suggestions and baseline rename hints remain valid; custom schemas suppress invalid rename hints; pattern-admitted keys succeed. |

## Unblocked Findings

None.

## Blocked Findings

None. No author decision is needed before repair, and no repair remains.

## Recurrence

No recurring defect remains. The additional instances discovered during repair belong to the original four findings and are repaired, rather than newly open siblings. No accepted decision or prior optional review item was reopened. The first review contains no pending Human rulings section.

## Specification and verification coverage

The review checked the full render surface, including behavior outside the four repairs. Verification levels below describe the boundary actually exercised; spawning the CLI with a manufactured fixture is Level 1.

| Requirement group | Evidence inspected or rerun | Level and assessment |
|---|---|---|
| Root alias equality, defaults, option placement, input classification, command-name files, terminators, help and removed flags | Shared `RenderOptions`, prepared dispatch, clap-derived inventories, `render_alias_parity`, `render_dispatch`, argument/help tests | Level 1 is appropriate for argument and output contracts. Both spellings share preparation and the render handler. |
| No input, redirected/empty stdin and explicit stdin | Injected environment tests in `render_dispatch`; detached-tmux `level2_render_no_input` | Level 1 checks the decision matrix; the real-terminal test checks stdin terminal detection without reading input or taking focus. |
| Layer order, shorthand specificity, sparse borders, union replacement, width exclusions, null/zero/false, required fields and custom admission | `prepare_style_layers`, normalization/merge code, `style_layers`, `style_layer_matrix`, `render_style_matrix` | Level 1 public preparation tests inspect effective typed values and retained custom data, including rejection paths. |
| Files, offsets, loader syntax, errors, source contexts, passive preparation and stylesheet execution | `render_style` file classifier/reader, shared data loaders, style-origin tracking, `render_style_files` and `render_style_wiring` | Level 1 is appropriate for filesystem, network-refusal and process-output boundaries. Source/data/terminal targets do not read CSS; HTML and MarkdownPlus read it in the winning field's context. |
| Terminal dimensions, alignment, rule padding, code theme/mode, line numbers and plain degradation | Deterministic output tests plus migrated `level2_layout_dimensions`, `level2_horizontal_rules` and `level2_code_block_styling` | Actual terminal rendering has Level 2 coverage. The earlier implementation log records the complete terminal suite passing; repair changes do not alter terminal rendering. |
| Browser layout and presentation | HTML/MarkdownPlus assertions and headless browser tests for page widths, alignment, component colors and code mode/theme | Browser DOM/computed-style/geometry tests exercise the browser boundary. The earlier log records the browser suite's results and its unrelated Mermaid/font timeout; this review does not claim a fresh browser run. |
| Schema-aware completion, supported-shell wiring and no execution during completion | Shared completion queries, `path_completion`, `completion::`, existing real-shell tests | Level 1 checks candidates and passive selection; existing Level 2 shell tests cover shell wiring. No new keyboard encoding or modifier behavior requires Level 3. |
| Theme listing, command-local survivor flags, unchanged authored output and cleanup | Command-output schema tests, save tests, alias/source-output tests and CLI docs | Level 1 checks schemas, bytes, exit status and file preservation. No CLI style is inserted into authored frontmatter or cleanup options. |

The CLI and library use explicit test targets. The affected modules are declared in their L1 entry points, their names select L1, and the test-tier coverage check reports no stranded tests. New fixtures are temporary and do not rediscover repository state from the process working directory.

### Input robustness

The matrix applies to the style-file loaders, not the Markdown grammar or renderers. JSON, JSON5, YAML/YML and TOML use the shared data-loading branches. Inspected public-result tests cover valid controls, invalid/trailing syntax, duplicate-key policy, extra YAML documents, format-specific syntax, offsets and located failures; CLI wiring covers success and failure for each loader.

The library matrix checks validator families through public preparation results: scalar constraints, containers, unions, booleans, colors, themes, metadata and custom arrays. Absent/null/empty cases follow explicit spec exceptions: CLI null members mean absence; whole-root null fails; empty inline objects are a no-op; files without a supplied admitted leaf fail; empty arrays replace earlier arrays but alone do not make a nonempty style file. Invalid collection elements are rejected rather than filtered away. Superseded exact-path setter assignments are discarded before parsing, while separate files remain independently checked.

The requested smell search found no new silent-default problem. `prepare_layer` accumulates failures in its report despite using `filter_map`; diagnostic filtering excludes uncoded informational notices; optional position conversion can leave a location unknown. The repaired root branch explicitly rejects nonmapping values. The matrix and code do not conflate an invalid field with an absent one.

## Checks run for this review

| Command, from the darkmatter area unless noted | Result |
|---|---|
| `just test render_style` | 108 passed |
| `just test style_layer` | 168 passed |
| `just test render_alias_parity` | 42 passed |
| `just test render_dispatch` | 22 passed |
| `just test completion::` | 141 passed |
| Root: `just check-tier-coverage darkmatter` | Zero stranded tests |
| `just test-l2 level2_render_no_input` | One CLI real-terminal test passed; no matching library or DMLS tests; recipe exited 0 |

The 481 selected L1 tests all passed, with no test at or above five seconds. Builds emitted the existing macOS linker unwind-size warning. No production code or tests were changed, no formatting command was run, and no commit was made. Cross-OS proof remains CI's responsibility and is not a readiness blocker for this review.
