---
$schema: feature-review.yaml
ready: false
findings:
    - 'R1-F1 (medium): Custom catalog completion still reads schema files more than once'
    - 'R2-F1 (high): Optional shell script tests pass without loading a script'
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-07T01:35:24-08:00
spec: 2026-11-06-dotted-paths/spec.md
implemented: true
description: 'A **feature** review of `2026-11-06-dotted-paths/spec.md`'
feature: 2026-11-06-dotted-paths/review-2.md
previous: 2026-11-06-dotted-paths/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-06-dotted-paths/log.md
implemented_by: claude/opus
human_review_items:
    - 'R2-F1: fish and Elvish L1 script loaders now compile only under the opt-in darkmatter-cli feature `optional-shell-tests` (run by `just test-optional-shells`, in neither CI nor local default features) and fail via `expect_level!` when selected without the shell; alternatives were provisioning the shells in CI or enabling them in local `just test`'
next: 2026-11-06-dotted-paths/review-3.md
---

# Review 2: Dotted Paths

The feature is not production ready. The original schema-selection, conversion, caller-file, and shell-behavior reproductions now work. Two verification obligations remain: the repair's read-once claim misses the custom-catalog branch, and two script-loading tests report success when their shell cannot start. Both repairs fit the existing spec and need no human decision.

This review reads the spec, review-1.md, and log.md's Finding Ledger against the current implementation, including the uncommitted repair. There is no earlier Human rulings section. The informational decisions in the previous review are not treated as pending approvals or reopened here. The darkmatter, rust-testing, just, biscuit-file, and OS skills informed the relevant checks.

## Previous Finding Verification

The required instances include every child row in the repair ledger, including the additional instances discovered during repair. The evidence inventory below accounts for those rows; a repair-log statement alone is not verification.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Setter binding and completion use the wrong baseline selection | R1-F1.1–R1-F1.24 | **partially fixed** | Selected-baseline public/CLI tests pass for custom, disabled, document, trigger, raw JSON, fallback, and named-type cases. Code uses `declared_property_shape` and shares command policy with completion. The direct file-open probe below disproves the complete read-once claim in R1-F1.22. | R1-F1.22: the catalog branch still rereads its root and repeated imports. The original incorrect selection and lost digits are repaired. |
| R1-F2: Declared values bypass the shared conversion rules | R1-F2.1–R1-F2.22 | **verified fixed** | Public `setter_values` tests, set/compose parity, raw-reference/constrained-string cases, and persistence tests pass. `SetterValue::bind` converts decoded quoted strings and preserves declared unknown text; `set_checked` persists the judged value; `partial_copy` no longer sets the root aside. | None. |
| R1-F3: Union selection loses the origin of nested file setters | R1-F3.1–R1-F3.25 | **verified fixed** | Public compose matrix and CLI origin tests pass. Inspection follows supplied leaves through arm selection, validation, rewriting, presentation, and diagnostics. Claudine's caller-file and status-report tests pass. | None. Fresh native-Windows execution is not a readiness requirement for this review. |
| R1-F4: Elvish and PowerShell lack the required shell verification paths | R1-F4.1–R1-F4.6 | **verified fixed** | Executable script-load and real-Tab paths now exist. Six local L2 tests pass, including both Elvish cases. Windows-only PowerShell tests use ConPTY and read PSReadLine's actual command buffer. Elvish's measured Enter-to-accept fallback is documented. | The missing test paths and inferred limitation are repaired. R2-F1 is a newly identified sibling about prerequisite enforcement, not a claim that Tab completion still fails. |

### Instance evidence inventory

All test names below refer to tests actually selected by the runs in the verification section, except where explicitly labeled code inspection or existing Windows evidence.

| Required instance | Evidence checked |
|---|---|
| R1-F1.1, R1-F1.2 | Compose `selected_baseline::a_custom_baseline_flag_declares_the_type` and set `set_selected_baseline::a_custom_baseline_environment_declares_the_type`: `1.10` keeps its digits. |
| R1-F1.3, R1-F1.11 | Disabled-baseline tests in both command suites: undeclared `title` gets the numeric guess. |
| R1-F1.4, R1-F1.5 | Completion `selected_baseline` flag/environment cases offer custom values and omit compiled leftovers. |
| R1-F1.6, R1-F1.7 | Completion disable-baseline and disable-trigger cases omit the disabled values. |
| R1-F1.8, R1-F1.9 | Separate document-schema binding and completion controls retain `1.10` and offer `custom`. |
| R1-F1.10 | No-document and broken-document-schema fallback tests retain the compiled fallback for the built-in selection. |
| R1-F1.12, R1-F1.13 | Get/set custom-baseline tests and remove's present-only control pass. |
| R1-F1.14 | `compose_ignores_a_custom_baseline_in_the_environment` passes; command and completer both call `compose_baseline`. No request to change the command's existing environment policy. |
| R1-F1.15, R1-F1.16, R1-F1.17 | Raw JSON baseline/document/trigger tests pass. Inspection confirms the existing raw declaration projection is used for those layers. |
| R1-F1.18, R1-F1.19 | `PathSchema::Nothing` and custom/disabled/broken-selection fallback tests pass. No selected layer is confused with failed selection. |
| R1-F1.20 | `a_custom_baselines_recursive_types_resolve_at_any_depth` passes; `definition` and `layered_definitions` include the custom table. |
| R1-F1.21 | `compose_baseline` reads the captured snapshot; the live environment helper remains outside this completion route. Context-construction guard run checks the entry points. |
| R1-F1.22 | **Partial:** source-buffer reuse covers ordinary YAML/JSON baselines, but the catalog branch returns before that reuse. File-open observations: ordinary YAML/JSON once, catalog root twice, shared catalog import twice. See the carried finding. |
| R1-F1.23 | Compose `a_disabled_trigger_declares_nothing` passes. |
| R1-F1.24 | Code inspection: DMLS's `property_shape` remains separate from request-selected `declared_property_shape`; the built-in DMLS contract is retained. |
| R1-F2.1, R1-F2.2 | Unknown-text adapter tests and set/compose parity keep `1.10` as text, including nested positions. |
| R1-F2.3, R1-F2.4 | Quoted-number adapter and command tests produce numeric `5`; persistence tests check the saved value and unaffected bytes. |
| R1-F2.5, R1-F2.6 | Declared string/number/union/numberlike/boolish controls and recursive container binding pass. |
| R1-F2.7, R1-F2.8 | Null remains null; undeclared values retain the established guess/quoted-string distinction. |
| R1-F2.9 | `value_spelling::every_candidate_decodes_to_its_member_under_each_declared_shape` and the other candidate-spelling tests pass. |
| R1-F2.10 | `declared_shape_paths` and command pattern-key cases pass at depth; `child_shape` handles dictionary descent. |
| R1-F2.11, R1-F2.12 | Raw number/reference tests pass; `value_as_judged` reads the converted copy before persistence. |
| R1-F2.13, R1-F2.14 | `any` parity and `$schema` command tests pass. The schema-edit branch intentionally uses its own schema/reference check rather than a value declaration. |
| R1-F2.15 | Code inspection: guesses are used for the provisional overlay, while `apply_setter_plan` and `set_checked` bind from the retained parsed argument; trigger-order command tests pass. |
| R1-F2.16, R1-F2.20, R1-F2.22 | Missing undeclared-required-key cases pass for a scalar reference, array element, container, and authored validation. The repaired `partial_copy` filters root failures before setting values aside. |
| R1-F2.17 | Contrary adapter tests were replaced; the pattern-key test expects declared unknown text. Those selected suites pass. |
| R1-F2.18 | Quoted `allOf` number is refused in set and compose, like its authored counterpart; unquoted numeric control succeeds. This retains the conversion engine's policy. |
| R1-F2.19 | Pattern, format, typed/untyped string enum, mixed enum, and `allOf` string cases pass. `keeps_text_at` probes the actual written text, not a fixed placeholder. |
| R1-F2.21 | `a_problem_of_the_whole_document_does_not_change_how_text_reads` passes. Root-only problems no longer change the scalar's reading. |
| R1-F3.1, R1-F3.2, R1-F3.3, R1-F3.4 | `caller_file_origins` named-object, array, dictionary, and recursive-child cases pass through the public compose API. |
| R1-F3.5, R1-F3.6 | Whole object, whole `--set`, array/dictionary overrides, and a nested setter into an override pass; representative CLI cases also pass. |
| R1-F3.7, R1-F3.8, R1-F3.9 | Top-level and single-schema controls pass; an outside-glob launch file is still refused. Supplied origins reach the final validator. |
| R1-F3.10 | Code inspection: both validation passes enter the shared preparation/projection route; `adopt_supplied_values` installs the origins in the prepared schema. The earlier post-shell probe is recorded evidence, not a fresh run here. |
| R1-F3.11, R1-F3.12 | Authored-sibling origin controls and transcluded-child setter tests pass. |
| R1-F3.13, R1-F3.14 | State defaults retain document origin; pending nested templates retain the top-level post-render origin policy. Both tests pass. |
| R1-F3.15, R1-F3.16 | Optional/union document-only files are refused at top level and below a parent. |
| R1-F3.17 | Raw `patternProperties` dictionary test passes. |
| R1-F3.18 | Code inspection: classification and mode records include `nested_occurrence` for every supplied leaf; stability compares those records rather than only whole keys. |
| R1-F3.19 | Code inspection: property validation uses `CallerOrigins::scoped_to`, which retains supplied values. Nested-origin matrix passes. |
| R1-F3.20 | Authored copies of the same absolute path remain refused beside the supplied copy; the permitted authored-path control passes. `authored_copies` triggers a second, document-origin judgment for those paths. |
| R1-F3.21 | Native-storage and portable-presentation tests pass locally. Inspection confirms `to_portable_string` is applied to the nested presentation copy, not stored frontmatter. Existing native-Windows results are recorded in the repair log; not rerun here. |
| R1-F3.22 | Claudine caller-file tests include all 11 nested supplied-file cases and status-report positive/missing/authored controls; selected run passes. Inspection confirms pre-validation defers these verdicts to composition instead of dropping the object. |
| R1-F3.23 | CLI origin cases pass through normal preflight. The shared preflight setter/projection route is retained; no separate resolver was added. |
| R1-F3.24 | Code inspection: `md set` validates in `document_context`, not the launch-file projection; its saved text becomes document-authored. |
| R1-F3.25 | Found/missing and top-level/nested refusal-message tests pass; `reported_from_caller_origin` names the launch context without inventing a no-match failure for an existing file. |
| R1-F4.1, R1-F4.2 | Modern Bash and system Bash 3 script/word tests and separate L2 Tab branches pass. |
| R1-F4.3, R1-F4.4 | Zsh and Fish word/script tests and L2 Tab runs pass with the shells present. Missing Fish's prerequisite is separately reported in R2-F1. |
| R1-F4.5 | Elvish script test and both L2 tests pass. The fallback is measured in the shell and documented, replacing the earlier guess. Missing Elvish's prerequisite is separately reported in R2-F1. |
| R1-F4.6 | Code inspection: native-Windows tests start PowerShell 5/7 under ConPTY, load the printed setup, check alias removal, inspect the actual input buffer after each Tab, and run the result. The ledger records successful native runs. No fresh cross-OS proof is inferred. |

## Unblocked Findings

### R1-F1 (medium): Custom catalog completion still reads schema files more than once

**Authority: spec.** Completion's permitted reads are limited to schema/document content files, “each read once”; the spec clarifies that this includes “repeated imports.” The repair ledger's R1-F1.22 claims that `with_baseline_from_file` no longer reads a custom baseline twice per Tab. That repair is incomplete for an existing supported baseline form.

**Class:** completion reloads content it already has instead of sharing that content across the selected schema's preparation.

The remaining defect is in darkmatter's [with_baseline_from_file](../../lib/src/markdown/schemas/mod.rs:480). It reads the file to detect a catalog, then its catalog branch calls [GlobalCatalog::from_file](../../lib/src/markdown/schemas/catalog.rs:256), which opens it again. Only the ordinary-schema branch below it installs the bytes in `SourceBuffers`. Catalog loading also resolves each global separately, so `ctx` and `current` importing the same named type cause two reads of that import. The same route supplies all three edit-command completers through darkmatter-cli's [request_schemas](../../cli/src/commands/schema/validate.rs:398).

Use a separate `types.yaml`:

```yaml
kind: schema
types:
  shared: {name: string}
```

Use this valid custom `base.yaml`, following the shipped catalog's `ctx`/`current` shared-type arrangement:

```yaml
kind: schema
$schema:
  fm:
    status: enum(custom)
  ctx: shared@./types.yaml
  current: shared@./types.yaml
  env: object
  err: {message: string}
  timing: {elapsed: number}
```

For a fixture document containing `status: custom`, run completion with `BASELINE_SCHEMA` pointing to that catalog:

```sh
BASELINE_SCHEMA=./base.yaml COMPLETE=fish md -- md get doc.md sta
BASELINE_SCHEMA=./base.yaml COMPLETE=fish md -- md set doc.md status cu
BASELINE_SCHEMA=./base.yaml COMPLETE=fish md -- md rm doc.md sta
```

All three return the correct candidates. A temporary macOS `open` observer loaded into the built CLI recorded two opens of `base.yaml` and two of `types.yaml` in each invocation. The observer only counted fixture paths; it changed no repository source. Source inspection confirms these are full content reads, not metadata probes.

The instance table subdivides the carried ledger instance R1-F1.22 so the next repair can account for the entire remaining branch.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.22.1 | Get completion → catalog loader and global resolution | Catalog above, key prefix `sta` | Correct `status`; root twice, shared import twice | Correct candidate; each distinct content file once |
| R1-F1.22.2 | Set completion → same loader | Same catalog, value prefix `cu` | Correct `custom`; root twice, shared import twice | Same read-once guarantee |
| R1-F1.22.3 | Remove completion → same loader | Same catalog, present key prefix `sta` | Correct `status`; root twice, shared import twice | Same read-once guarantee |
| R1-F1.22.4 | Ordinary YAML baseline branch | `$schema: {status: enum(custom)}` | Correct candidate; baseline once | Clean control |
| R1-F1.22.5 | Raw JSON Schema baseline branch | `status` enum containing `custom` | Correct candidate; baseline once | Clean control |
| R1-F1.22.6 | Compose baseline selection | Explicit baseline flag; catalog environment variable | Flag reader reads its baseline once; compose ignores the environment variable by existing contract | Clean by its separate command-selection contract |
| R1-F1.22.7 | Ordinary document schema and repeated type import | Existing `s.yaml` importing `types.yaml` twice | Candidate/read-observation test passes; one namespace parse; one preparation | Clean within the shared resolver's namespace cache; the catalog's separate per-global resolutions bypass that sharing |
| R1-F1.22.8 | Built-in and disabled baseline selection | No custom file | Neither branch opens a custom baseline | Clean by construction |

**Smallest repair:** consume the catalog bytes already read and share imported source/namespace reads across its global declarations for this completion invocation. Reuse existing resolution facilities; no persistent cache or general filesystem-observation layer is required. Add a focused public-result/read-count control for the catalog, including its repeated import. The existing `Observed.read` set proves which paths were read, not how many times, and cannot detect this failure by itself.

The priority is medium because it affects an uncommon custom-catalog configuration and returns correct values; its authority is the explicit read-once rule. This is not a request for a stricter timing budget.

### R2-F1 (high): Optional shell script tests pass without loading a script

**Authority: spec.** The L1 script requirement says: “An L1 prerequisite uses `expect_level!`; it cannot silently skip.” It also says that an unrun test cannot count as passing. These are executable verification requirements, not a demand for fresh results on every OS.

**Class:** a failed test prerequisite returns normally, producing a passing test without executing the behavior the test claims to verify.

In darkmatter-cli's [Fish script test](../../cli/tests/l1/path_completion.rs:615) and [Elvish script test](../../cli/tests/l1/path_completion.rs:632), `shell_starts` returning false prints `unproven` and returns. Nextest captures the message and reports `PASS`. Elvish is explicitly absent on the CI hosts according to the repair log, so this is a normal environment condition, not a hostile input.

Reproduction used temporary executable shims named `fish` and `elvish`, each exiting 127, prepended to PATH. This simulates the existing `shell_starts == false` branch without removing host programs or editing source. Each test was selected through `just test`:

```text
Unavailable fish, recipe exit: 0
PASS path_completion::the_fish_script_loads_where_fish_is_installed
Summary: 1 test run: 1 passed

Unavailable elvish, recipe exit: 0
PASS path_completion::the_elvish_script_loads_where_elvish_is_installed
Summary: 1 test run: 1 passed
```

The bounded sweep covers every L1 actual-shell script loader, rather than treating manufactured shell-word tests as evidence of script loading.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Fish script loader | Available shell control; shell cannot start | Real control passes; unavailable-shell probe also passes without loading | Selected L1 test fails its prerequisite via `expect_level!` |
| R2-F1.2 | Elvish script loader | Same two shapes | Same false success; no stand-in-editor/script exercise in the unavailable case | Same explicit prerequisite failure |
| R2-F1.3 | Modern Bash loader | Shell starts / fails to start | Actual control passes; code calls `expect_level!` before script loading | Clean; unavailable prerequisite cannot return success |
| R2-F1.4 | macOS Bash 3 loader | System shell starts / fails to start | Actual control passes; same mandatory gate | Clean |
| R2-F1.5 | macOS Zsh loader | Shell starts / fails to start | Actual control passes; same mandatory gate | Clean |
| R2-F1.6 | Native-Windows PowerShell loader | Shell starts / fails to start | Code uses a noninteractive startup probe and `expect_level!` | Clean by the mandatory gate; platform-selected, not executed on this Mac |

PowerShell 7's script is loaded by its real-shell L2 path. The L2 tests have a different prerequisite contract and are not substituted for these L1 loader obligations. The actual local Tab runs passed; they do not make an unavailable-shell L1 test truthful.

**Repair:** replace both successful early returns with the required L1 prerequisite assertion. Select shell-specific tests deliberately where their prerequisites are provided, as the spec directs; a selected test must not pass without loading its script. Keep the measured Elvish fallback and the real-shell tests. No broad OS matrix, new shell adapter, or human ruling is needed.

## Blocked Findings

None. Neither finding depends on an author decision or another spec.

## Recurrence

- **R1-F1:** the original wrong-schema selection is repaired. The expanded repair ledger's R1-F1.22 remains incomplete: the ordinary YAML/JSON branch was fixed, while the existing catalog branch and its separate global-import resolutions were missed. This is a remaining instance of that ledger obligation, not a new baseline-selection regression. The next repair must cover every row under R1-F1.22 above.
- **R2-F1:** this belongs to review-1.md's R1-F4 shell-verification class. It is a sibling the earlier sweep missed, not a regression in the measured Tab behavior. Fish already had the permissive prerequisite branch, and the new Elvish loader copied it. The previous repair should have swept prerequisite outcomes across the six L1 loaders; all are accounted for above. The original missing executable paths are verified fixed separately.

## Verification and input robustness

| Check | Result |
|---|---|
| Darkmatter `just test compose_setters::` | 106 passed |
| Darkmatter `just test path` | 605 passed, including candidate, completion, property-path, and source-edit cases |
| Darkmatter `just test get_set_rm::` | 128 passed |
| simplified-schema `just test setter_values::` | 43 focused tests passed; recipe also ran all 1,297 minimal-feature tests, passing |
| Claudine `just test caller_file` | 59 passed, including nested supplied-file and status-report cases |
| Darkmatter `just test context_construction_guard::` | Five passed |
| Darkmatter `just test-l2 level2_path_completion` | Six CLI tests passed in detached tmux; other package targets selected zero tests |
| `just check-tier-coverage darkmatter` | Zero stranded tests |
| Missing-shell probes through `just test` | Both falsely passed; evidence for R2-F1 |
| CLI file-open probes | YAML/JSON baseline once; custom catalog root and shared import twice, across get/set/rm |

No selected L1 test reached five seconds. Builds emitted the existing large-unwind-table linker warning. An initial attempt to combine filters with a shell pipe was rejected by the recipe's argument expansion; the separate successful runs above are the evidence. Full-area lint and full-area L1 were not rerun. Cross-OS execution recorded by the implementer is identified as such, not presented as a new observation.

The tests are declared through their consolidated `main.rs` targets. L1 names select into L1, and `level2_path_completion` selects into the live L2 recipe with its terminal feature. The CLI manifest enables that feature for CI. None of these checks took terminal or browser focus.

The input robustness matrix applies to the two setter-value container loaders, not the dotted-path grammar, completion word grammar, or YAML source writer. The public `SetterValue` suite establishes the default: an admitted container with duplicate keys or invalid numbers fails; rejected syntax may remain text as the spec permits. Exceptions preserve missing members, explicit null, empty input/collections, native values under unknown, and quoted scalar text until declared conversion. Both loaders retain mixed/all-wrong-type elements for validation rather than dropping them. The suite covers duplicate keys, trailing/malformed syntax, JSON5 comments/trailing commas, YAML tags/anchors/extra documents, and numeric overflow/underflow. The repaired declared-value and persistence cases also pass through command results.

The source-smell search found no new defaulted field or element-filtering parser in that value reader. Its YAML numeric `.ok()` propagates failure of the entire container conversion after admission. Completion's `.ok()`/filtering deliberately suppresses invalid advisory words and failed schema sources; it does not validate a configuration by silently deleting members. The custom catalog uses the existing strict field validators; its finding here concerns duplicate reads, not a new format-admission defect.

### Requirement-to-verification mapping

| User-facing requirement | Appropriate level | Evidence and limits |
|---|---|---|
| Escaped/empty/Unicode paths, canonical indexes, mapping numbers, missing or scalar parents, atomic removal | L1 | Public path tests and CLI property/get/set/rm cases pass. |
| Sibling bytes/comments, body/fences, flow fallback, aliases, saved round trips | L1 | Source-edit and persisted-command cases pass. |
| Flat get output, top-level remaining keys, stdout/save/stdin contracts | L1 | Command tests pass; output structs retain their existing contracts. |
| Type binding, whole-container replacement, duplicate/specific setters, required-child combination | L1 | Adapter tests, public compose matrix, and CLI parity cases pass. |
| Trigger-selected schema, custom/disabled baseline, raw schemas, unknown-key and supplied-value diagnostics | L1 | Selected-baseline, declared-value, trigger-order, and refusal cases pass. Remaining catalog I/O is R1-F1. |
| Caller-file origins, authored siblings, union selection, child composition, pending values | L1 | Public/CLI compose and Claudine cases pass; Windows presentation implementation inspected. |
| Passive completion, key ordering/spelling, unions, values, fallback, cursor position, file values | L1 | Candidate/completion tests pass, including execution/network denial seams. |
| Each distinct content file read once; bounded release responsiveness | Observable operations / one release sample | The original release sample remains recorded; no new timing threshold. Catalog read-count violation is reproduced in R1-F1. |
| Shell word shapes and generated scripts | L1, using the shell for actual script loading | Word and available-shell controls pass. Selected tests can still pass without the shell: R2-F1. |
| Accepting Tab candidates and continuing without stray spaces | L2 real shell, per this spec | Bash 5, Bash 3, Zsh, Fish, and measured Elvish fallback pass locally. Native PowerShell executable paths and prior evidence inspected. No L3 requirement is introduced for shell completion. |

## Human review

No new design decision needs confirmation. Existing informational choices remain recorded in review-1.md. The two remaining repairs implement explicit spec requirements and can proceed without a pipeline stop.
