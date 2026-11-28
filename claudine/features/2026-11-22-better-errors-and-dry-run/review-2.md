---
$schema: feature-review.yaml
ready: false
findings:
  - id: R2-F1
    priority: high
    result: fixed
    description: A failed inclusion hides independent warnings from the recovered preview
  - id: R2-F2
    priority: high
    result: fixed
    description: Prompt checks treat launch-time values as settled
  - id: R2-F3
    priority: high
    result: fixed
    description: Inline section and schema-slot failures are counted twice
  - id: R2-F4
    priority: high
    result: fixed
    description: Dry runs omit invalid permission settings and repeat configuration parse errors
  - id: R2-F5
    priority: high
    result: fixed
    description: Dry-run checks reread the same permitted sources
  - id: R2-F6
    priority: high
    result: fixed
    description: The real-terminal sequence preflight test still expects the retired error block
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-27T04:57:00+00:00"
spec: 2026-11-22-better-errors-and-dry-run/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-better-errors-and-dry-run/spec.md`"
feature: 2026-11-22-better-errors-and-dry-run/review-2.md
previous: 2026-11-22-better-errors-and-dry-run/review-1.md
next: 2026-11-22-better-errors-and-dry-run/review-3.md
implemented-by: claude/opus
repaired-on: "2026-11-27T05:08:02+00:00"
---

# Review 2 — Better errors and a diagnostic dry run

**Production readiness is false.** The remaining repairs fit the existing spec and can proceed without a ruling.

## Previous Finding Verification

The required instances come from the first review's repair ledger. Outcomes apply to those instances. Newly discovered siblings are listed separately below. [First review and repair ledger](./review-1.md#finding-ledger)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Sequence source expressions can start a shell during a dry run | R1-F1.1–.8 | verified fixed | Source-effect tests cover root list evaluation, the reference operator, root/nested formal templates, body, owner/prompt frontmatter and task `with:`. The pure controls resolve and the marker checks cover commands and task dispatch. The walk and canonical preparation both attach the passive context. [CLI tests](claudine/cli/tests/l1/dry_run_source_effects.rs) · [Walk](claudine/lib/src/composition/sequence/preflight/check.rs) | None for these instances. |
| R1-F2 — A bad row in a root sequence source hides valid siblings | R1-F2.1–.8 | verified fixed | Public graph checks cover invalid middle rows, later prompt resolution, complete parse failure, and the selected data-list input discovery path. CLI tests assert sibling previews across the root source routes. [Graph tests](claudine/lib/src/composition/sequence/preflight/tests/check_tree.rs) · [CLI tests](claudine/cli/tests/l1/dry_run_check.rs) | None for these instances. |
| R1-F3 — Required inputs suppress independent checks and previews | R1-F3.1–.11 | verified fixed | The four preview routes and dependent owner case preserve available text with missing-value marks. Tests exercise supplied values, input policy and `--silent`, referenced owner/body deduplication, exact mixed schema rows, real-run counts, and the repository placement guard. [Preview fixtures](claudine/cli/tests/l1/dry_run_missing_inputs.rs) · [Input policy](claudine/cli/src/commands/document_check/tests.rs) · [Real-run counts](claudine/cli/tests/l1/sequence_referenced_state.rs) | New duplicate schema-slot projection in R2-F3. |
| R1-F4 — Provider prediction treats runtime-written values as settled | R1-F4.1–.7 | verified fixed | Public graph and CLI checks cover frontmatter and chained `with.agent`, static and absent controls, late model, enclosing selection, and a single model deferral with the launch-time reason. The real chained test launches the stub twice. [CLI cases](claudine/cli/tests/l1/dry_run_launch_hints.rs) · [Prediction tests](claudine/lib/src/composition/sequence/preflight/tests/provider_dependencies.rs) | New schema, condition and preview consumers in R2-F2. |
| R1-F5 — Remote reads cannot defer on network approval | R1-F5.1–.8 | verified fixed | The permission matrix covers `allow`, `ask`, `deny`, URL expressions/includes/slots, the deliberately local sequence-source reader, zero requests for denied or pending reads, the real slot gate, host-specific hints, and output from inline mode. Direct loopback probes also observed successful allowed reads. [Darkmatter matrix](darkmatter/lib/tests/l1/network_approval.rs) · [Claudine matrix](claudine/cli/tests/l1/dry_run_network_approval.rs) · [Real slot tests](claudine/cli/tests/l1/launch_network_slots.rs) | The settled permission behavior stands. New findings concern configuration reporting and repeated authorized reads. |
| R1-F6 — Legacy reference JSON baselines were changed | R1-F6.1–.5 | verified fixed | The Darkmatter CLI tests compare bytes for broken, listing, clean, and empty reference results and graph output. The checked-in baselines also have an empty diff against their pre-regression snapshot at `c0345c1c0`. New compose and Claudine report snapshots still contain structured evidence and trails. [Reference baselines](darkmatter/cli/tests/l1/reference_report_baselines.rs) · [New findings](darkmatter/lib/tests/l1/findings.rs) | None for these instances. |
| R1-F7 — Schema key findings lose task occurrences and duplicate parser findings | R1-F7.1–.14 | verified fixed | The CLI matrix covers task, leg, referenced YAML/Markdown, selected catalog entries, repeated failure/unchecked occurrences, checked controls, empty-agent overlap, a failed occurrence next to a successful one, the fallback hint, and DMLS parity. Registry and diagnostic detail checks cover the changed count and records. [Occurrence tests](claudine/cli/tests/l1/dry_run_schema_key_occurrences.rs) · [Registry tests](claudine/lib/src/diagnostics/registry.rs) | None for these instances. |
| R1-F8 — Live inline sequence steps can corrupt a YAML source | R1-F8.1–.8 | verified fixed | Tests inspect YAML after a provider edit, confirm the next step reads it, exercise Markdown and separate prompt controls, and keep dry-run preview and completion checks. The resolver admits YAML and Markdown on the root route, and the diagnostic corpus includes the slot error. [Source format tests](claudine/cli/tests/l1/sequence_inline_source_format.rs) · [Corpus guard](claudine/cli/tests/l1/error_guards.rs) | None for these instances. |
| R1-F9 — Native Windows Level 1 timings exceed the review limit | R1-F9.1–.7 | verified fixed | Checked the saved nextest outputs in `/tmp/f9-win-run1.log` and `/tmp/repairtool/win-*.log`. The 161 targeted tests maxed at 4.796 s. The package-run outliers peaked at 8.606 s, 18 CLI and 7 library tests. Isolated CLI max was 4.831 s and six library tests cleared five seconds. The remaining 5.472 s payload test was replaced by six cases plus a roster guard, all seven at or below 2.742 s on Windows. Reviewed the split source and declared L1 placement. [Fleet split](claudine/cli/tests/l1/research_sequence_artifacts.rs) · [Payload split](claudine/lib/tests/l1/claudine_schemas.rs) | None for these instances. |
| R1-F10 — Sequence load findings drop structured source locations | R1-F10.1–.8 | verified fixed | Public graph load tests cover YAML/Markdown/JSON/JSON5/line-delimited positions and line shifts, good syntax and file-only fallbacks. CLI assertions cover wrong-type references, wrappers, confirmed no-match, tolerated reads, and authored versus computed rows. [Loader positions](claudine/lib/src/composition/sequence/preflight/tests/check_tree.rs) · [Projection fixtures](claudine/cli/tests/l1/dry_run_check.rs) · [Darkmatter inclusion sites](darkmatter/lib/tests/l1/inclusion_target_sites.rs) | New inline section projection in R2-F3. |
| R1-F11 — The report fixture guard does not cover the scoped diagnostic catalog | R1-F11.1–.5 | verified fixed | The production scopes drive the fixture and snapshot guards, the sequence block fixtures have their own link table, the parity doc runs through a referenced sequence as well as through DMLS, and the compose policy recognizes the near-miss warning. [Claudine guard](claudine/cli/tests/l1/dry_run_check.rs) · [Darkmatter guard](darkmatter/lib/tests/l1/findings.rs) · [Block linkage](claudine/cli/tests/l1/sequence_error_blocks.rs) · [Policy](darkmatter/lib/src/markdown/compose/policy.rs) | None for these instances. |
| R1-F12 — Darkmatter drops the unchecked body verdict after a failed preparation | R1-F12.1–.9 | verified fixed | The ordinary/lifecycle tests retain the body verdict and independent call/schema verdicts, and preserve the reference-only and stdin controls. Claudine emits both reference and expression failures, deduplicates the inline input copy, defers unknown targets, and respects the four combinations of parent/child tolerance. A direct probe with only the tolerant child's missing reference reproduced the warning on all checked routes. [Darkmatter regression tests](darkmatter/cli/tests/l1/compose_combined_report.rs) · [Claudine reference tests](claudine/cli/tests/l1/dry_run_independent_references.rs) | New retry warning loss in R2-F1 and inline section duplicate in R2-F3. |
| R1-F13 — Available file choices are reported as hard failures | R1-F13.1–.8 | verified fixed | The picker and eager-input suite covers every eligible mode, exact paths, explicit relative misses, redirected and silent gradings, and sequence task refusals. Real/dry parity tests cover undefined and empty sequences and failed owner preparation at both root and nested boundaries. [File choice suite](claudine/cli/tests/l1/dry_run_file_choices.rs) · [Sequence parity](claudine/cli/tests/l1/sequence_dry_run_parity.rs) | None for these instances. |

## Unblocked Findings

### R2-F1 — A failed inclusion hides independent warnings from the recovered preview (high)

**Authority (spec).** The dry run “gives as many insights as possible” and “one problem never hides the next.” A warning remains an observation even when another finding has already set exit status 2. [The principle](./spec.md#the-principle) · [The preview](./spec.md#the-preview)

This is a dropped diagnostic when a failed phase takes its recovery path. Start with a prompt containing `::file ./first.md` followed by `::file ./child.md`. The child uses `ignore_invalid: true` and includes absent `./other.md`. Adding or removing `first.md` changes whether the first include fails. The child's warning stays the same. The equivalent rewrite moves the same text into `prompt:` or behind a sequence task.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Direct compose | Both missing, then create `first.md`, then create `other.md` | One error, then one warning, then clean | Error and warning, warning, clean |
| R2-F1.2 | Inline compose | Move the same body to `prompt:` | Same loss and controls | Same |
| R2-F1.3 | Sequence prompt file | Reference the same document as a task | Same loss and controls | Same under the task |
| R2-F1.4 | Sequence inline prompt file | Reference the inline version as a task | Same loss and controls | Same under the task |
| R2-F1.5 | Sequence body step | A named step composes the sequence document body | Same loss and controls, plus the body-task info row | Same, with the body-task info row |

The Claudine preview's initial success branch projects `prepared.warnings`. Its successful retry branch consumes the new deferrals and the prompt only. The condition-blind walk intentionally leaves warnings in documents that tolerate invalid references to the preview. Use the same warning projection in both success branches, including the check for references already reported by the walk. Preserve the original report order and the one-row rule for the first inclusion. [Claudine preview](claudine/cli/src/commands/document_check/preview.rs) · [Claudine reference walk](claudine/lib/src/composition/prepare.rs)

### R2-F2 — Prompt checks treat launch-time values as settled (high)

**Authority (spec).** For a value “only known at launch (`seq.*`, `loop.*`, a runtime `set`),” record the dependent check as unchecked. The spec also explicitly tests a real run that succeeds after a prior write against a dry run that defers the check. [How effectful checks are handled](./spec.md#how-each-effectful-check-is-handled) · [Preflight parity](./spec.md#parity)

This is the use of a preflight value as a settled launch value. A two-step sequence first runs a fake Goose prompt with a `success: [{ set: { topic: a } }]` action. The next prompt initially has `topic: missing`. Moving the same read from a page block to a schema or through `with:` should preserve the launch dependency. Renaming the written key to `other` leaves a static `topic` and makes its failed checks conclusive.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Page block and empty-body verdict | Entire second prompt guarded by `topic == 'a'` | Dry run emits `dm.compose.empty_body` for either write. Real run exits 0 for `topic` and 1 for `other` | Conditional preview and unchecked body when the write affects `topic`. Proven empty error for `other` |
| R2-F2.2 | Schema validation | Second prompt declares `topic: enum(a)` | Dry run emits `dm.schema.constraint` for both writes. Live exits 0 for `topic` and 1 for `other` | Schema verdict for `topic` deferred only when the earlier write can affect it |
| R2-F2.3 | Task input projection | `with.topic: "{{ choice }}"`, with `choice: "{{ topic }}"` in the sequence | Same false schema error, and same live exit split | Keep the dependency when forwarding the evaluated task input |
| R2-F2.4 | Literal override control | Change task input to `with: { topic: a }` | Both modes exit 0 under both writes | Keep the literal proof |
| R2-F2.5 | Provider and model consumers | Same prior-write and unrelated-write pair through `agent` and `model` | Existing repaired cases defer only where affected | Preserve the existing prediction behavior |
| R2-F2.6 | Sequence body step with root `initialize` | The root writes `topic` or `other` before its one body task, with the same page-block guard | Dry run emits an empty-body error either way. Live exits 0 when the root writes `topic`, and 1 for `other` | Preserve the root initializer dependency through list discovery and preview |
| R2-F2.7 | Later prompt's sequence outputs | Replace the guard with `length(seq.outputs) > 0` in the second prompt file | Real run exits 0 after the first task produces output. Dry run judges the empty preflight array and exits 2 | Leave the condition and the dependent body verdict unchecked |
| R2-F2.8 | Predictable sequence position | Replace that guard with `seq.index == 2`, then `seq.index == 1` | Both runs accept the true guard and refuse the false guard | Keep static sequence metadata usable |
| R2-F2.9 | Standalone compose and inline-compose | An initializer writes either the key a schema or block reads, or an unrelated key | Dry run reports the same static verdict for both writes. The schema control reaches the stub in both live modes. An all-conditional body can fail an earlier live discovery boundary | Defer values where the real route reaches the initializer first. Keep proven refusals at earlier boundaries |
| R2-F2.10 | Reference preflight control | `::file ./{{ topic }}.md` in a later prompt, with missing initial file | Both live runs stop before task one, and dry runs report the miss | Keep the real preflight check at its existing boundary |

The Claudine graph records lifecycle writes for provider prediction. The CLI's task input and preview paths take their own snapshot without that dependency. Pass the existing dependency information to the checks that consume the later prompt's data. Keep the earlier preflight reference verdict when the real run requires the file before the writer can execute. Update the dry-run topic to explain which reads are deferred when the preview depends on later state. [Current sequence dry-run behavior](claudine/docs/topics/flow-control/sequences.md#dry-run) [Sequence check](claudine/lib/src/composition/sequence/preflight/check.rs) · [Prompt inputs](claudine/cli/src/commands/document_check/inputs.rs) · [Prompt preview](claudine/cli/src/commands/document_check/preview.rs)

### R2-F3 — Inline section and schema-slot failures are counted twice (high)

**Authority (spec).** “Deduplicate equivalent findings for the same subject, authored location, and typed cause.” When the author wrote a frontmatter section, its construct determines the noun, and its authored position belongs in the report. [The collector](./spec.md#the-collector) · [Error-message standard](./spec.md#the-error-message-standard)

This is a double projection of one typed cause from two passes. Place one missing target in `prologue:` or `epilogue:`. Moving the delivered text from the Markdown body to `prompt:` leaves the section in frontmatter. Creating the one target removes its one finding. For slots, the control uses the fixture's legal `load_data` schema slot, and changing the slot to read a bare document key causes one typed forbidden-binding refusal.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F3.1 | Standalone inline compose | Each missing section, independently, then create its file | Two errors for either section, one named as the section and one as `::file`, both without a coordinate. Fixed file is clean | One correctly named finding at the authored section value |
| R2-F3.2 | Sequence with an inline prompt file | Same section changes | Same extra `::file` error and missing coordinates. Fixed file is clean | One located section finding for each affected occurrence |
| R2-F3.3 | Direct compose, chained prompt file and sequence body | Same prologue/epilogue changes | One row at the authored value and a clean fixed control | Preserve |
| R2-F3.4 | Body inclusion and inline expression controls | Missing `::file` or divided-by-zero interpolation | One correctly counted row in the direct probes and the existing independent-reference suite | Preserve one row per authored cause |
| R2-F3.5 | Direct, inline and sequence schema-slot preparation | `enum({{ choices }})` in a schema with an illegal bare binding | A generic `composition.schema_slot_failed` and a `dm.schema.slot_forbidden_binding` per affected prompt. A legal slot composes cleanly | One typed slot row for each occurrence with its proven location |

For inline sections, preserve the authored section identity through the generated prompt. Match the preparation's copy to the reference walk's located row using that identity. For slots, follow the existing input/preview ownership rule for aggregate schema validation when the preview provides the typed detail. Keep distinct rows for two authored sections and for repeated task occurrences. [Claudine projection](claudine/cli/src/commands/document_check/preview.rs) · [Input preparation](claudine/cli/src/commands/document_check/inputs.rs) · [Generated prompt and reference walk](claudine/lib/src/composition/prepare.rs)

### R2-F4 — Dry runs omit invalid permission settings and repeat configuration parse errors (high)

**Authority (spec).** The diagnostic report includes “an invalid configuration” in both formats. Configuration files are one authored subject, and equivalent findings about the same location and cause are deduplicated. [Never the standard error report](./spec.md#never-the-standard-error-report) · [The collector](./spec.md#the-collector)

This is lost or repeated reporting of one invocation-wide configuration verdict. Compare a plain local prompt under `network: ask` with the same prompt under `network: aswk`. Then express equivalent invalid YAML shapes and exercise all entry points. These probes use no remote content and no approval prompt.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F4.1 | Darkmatter public loaded policy | Valid network setting versus bad token | The Level 1 policy test proves the loader returns an invalid policy for the bad token | Preserve this typed verdict |
| R2-F4.2 | Claudine compose and inline-compose | Valid setting versus `aswk`, empty string, or list | Every local dry run exits 0 with no configuration finding | Valid exits 0. Each invalid value yields one located config finding and exit 2 |
| R2-F4.3 | Claudine sequence root and task | Same settings in a named body-task sequence | Invalid typed values yield exit 0. Malformed YAML, duplicate keys, and a second YAML document give the same located error under both root and task | One located configuration finding for the invocation |
| R2-F4.4 | Single-document YAML load failure controls | Malformed YAML, duplicate keys, second document | Compose and inline each report one located `dm.config.invalid` and exit 2 | Preserve |
| R2-F4.5 | Network demand consumes invalid permission | Invalid network setting with a remote read | Existing Darkmatter permission tests prove the invalid policy refuses access and sends zero requests | Preserve the refusal. Keep its evidence tied to the configuration cause |
| R2-F4.6 | Presence controls | Absent or null permission value in the common YAML loader | No error, as the public loader's default contract allows | Preserve |

The Claudine check explicitly reports the Claudine configuration load. Darkmatter can also return a captured configuration whose permissions verdict is invalid. The passive prompt should surface that verdict once even when the prompt needs no such permission. Reuse the captured verdict and attach dependent checks to it. A real local run may still succeed under the current Darkmatter rules. Keep that policy and the existing refusal for an operation that asks for the invalid permission. [Claudine check](claudine/cli/src/commands/document_check/mod.rs) · [Captured permission verdict](darkmatter/lib/src/config/network.rs) · [Permission loader tests](darkmatter/lib/tests/l1/config_fields.rs)

### R2-F5 — Dry-run checks reread the same permitted sources (high)

**Authority (spec).** “Share loaded sources and prepared values within the invocation so each new check does not reread or reparse the same document.” For `md compose`, the spec likewise asks to share the existing data memo and read set. [What the check pass walks](./spec.md#what-the-check-pass-walks) · [Other commands](./spec.md#other-commands)

This is duplicate observation across check passes. I used a loopback server that counted requests to each URL. All the cases used a fixed payload, `permissions.network: allow`, and local provider stubs. In the sequence cases two adjacent tasks named the same prompt and supplied identical inputs. Adding a second consumer of the same source should keep one observation. Changing the URL should add a second observation.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F5.1 | Direct prompt data evaluation | One body `load_data` URL | Two requests, exit 0 | One permitted observation |
| R2-F5.2 | Inline prompt data evaluation | Same text moved into `prompt:` | Three requests, exit 0 | One permitted observation |
| R2-F5.3 | Sequence prompt data evaluation | Two task occurrences of one file and URL | Four requests, exit 0 | One observation with two task occurrences |
| R2-F5.4 | Remote file/code and frontmatter transclusions | One directive or section at a time | One request for each single-document mode. Two requests for each in the repeated-prompt sequence | Keep single-document result and share source bytes across identical task reads |
| R2-F5.5 | External schema slot | Legal enum slot reading one URL | Two requests in direct and inline modes. Four for two sequence occurrences | Share the allowed slot input and keep each occurrence's own verdict |
| R2-F5.6 | Local prompt source | Two occurrences of one local Markdown prompt | The recovering loader already holds the prompt. CLI `load_document` resolves and loads the same path for each occurrence again | Hand the loaded source to the CLI checks, with a separate trail for each occurrence |
| R2-F5.7 | Multiple references inside one evaluation | Two `load_data` calls to one URL, then change only the second URL | Two total wire requests for one URL, then two each for two URLs | One request for each distinct source |
| R2-F5.8 | Darkmatter CLI preparation and combined report | Frontmatter reads one URL and the body displays its value. Add a missing local inclusion to enter the combined-report path | Two requests in the clean run and two in the failure run. The clean envelope lists the URL once | Share the observation between the reference and composition checks, including the failure path |
| R2-F5.9 | Network permission controls | Change `allow` to `ask` or `deny` | Existing tests assert zero requests for every applicable reader | Preserve |

This matters because a single report can otherwise validate and preview different versions of the same input. The network suite currently counts distinct permitted routes, which lets duplicate requests pass. Share existing invocation-local source and data observations across input preparation, the condition-blind walk, preview, and retry. Keep authorization scoped to each read and keep result evaluation tied to the task's own inputs. A loopback request-count assertion and the existing graph fixtures provide a small regression surface. [Claudine input preparation](claudine/cli/src/commands/document_check/inputs.rs) · [Claudine preview](claudine/cli/src/commands/document_check/preview.rs) · [Prompt loading](claudine/cli/src/commands/document_check/tasks.rs) · [Existing request assertions](claudine/cli/tests/l1/dry_run_network_approval.rs)

### R2-F6 — The real-terminal sequence preflight test still expects the retired error block (high)

**Authority (spec).** A stopped real preflight renders the combined report once and retains the selected typed failure. This requirement includes the terminal ordering of the preflight note and the report. [A real run uses the same checks](./spec.md#a-real-run-uses-the-same-checks)

This is a test that asserts the identity of a renderer the command no longer invokes at this boundary. A missing file produced only by the task's own `initialize` still stops before the task begins. Its note precedes the combined report. The direct and proxy routes still show their typed blocks at their own boundaries.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F6.1 | Sequence preflight capture | Prompt task's initializer would make `generated/notes.md` | Headless tmux test failed twice, at 1.098 s and 1.169 s. The pane shows the note, the combined report with the reference at `target.md:7:1`, and exit 1. The assertion searches for `TransclusionError` as a block header | Assert the captured note precedes the consolidated report, the report contains the located finding, and the command launches nothing |
| R2-F6.2 | Direct composition's transclusion error | Missing transclusion in regular composition | Changed tmux capture passed | Keep the existing typed block |
| R2-F6.3 | Lifecycle proxy preparation failure | Missing transclusion through the proxy and its route-equivalence matrix | Both tests passed. The companion SGR and hyperlink capture also passed | Keep the typed block and styling |
| R2-F6.4 | Dry-run terminal | An approval-capable command under dry run | tmux reports no shell approval and the test passes | Preserve |

Update the one sequence terminal assertion to the report contract. The existing Level 1 success/failure fixtures already check the selected error and lifecycle facets independently. [Sequence capture test](claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs) · [Typed captures](claudine/cli/tests/level2/level2_typed_error_render_capture.rs) · [Level 1 preflight fixtures](claudine/cli/tests/l1/sequence_preflight_baselines.rs)

## Blocked Findings

None.

## Recurrence

- **R2-F1 follows review-1.md, R1-F12, dropped preparation results.** The previous repair kept the unchecked-body row and preserved reference and evaluation errors independently. Its sweep reached the retry's error and deferral branches. The new sibling is the retry's success branch, which never projects its warnings. R2-F1.1–.5 exercise the shared branch and the initial-success controls.
- **R2-F2 follows review-1.md, R1-F4, stale launch-time values.** The provider/model instances remain fixed. The earlier sweep followed the value into provider selection and model requirements. The same dependency also reaches task-input preparation and the preview's schema and page-block decisions. R2-F2 identifies these missed consumers and preserves the real preflight reference decision.
- **R2-F3 follows review-1.md, R1-F3.9 and R1-F12.7, double projection.** The earlier repair covered aggregate schema validation and a duplicate inline expression verdict. Schema-slot preparation and the generated representation of inline prologue/epilogue are siblings. R2-F3 sweeps both projections and the unchanged chained, body-directive, and expression controls.
- **R2-F4.3 also follows review-1.md, R1-F3.8, duplicate owner/task findings.** The missing-input case remains fixed. A configuration parse error is another shared prerequisite that the sequence owner and its task see. The configuration-policy and loader rows in R2-F4 show where one located report is already produced, repeated, or omitted.

## Input Robustness Matrix

Invalid syntax and invalid required shapes should yield a located refusal with no task dispatch or publish. An invalid task row leaves siblings checkable when the loader knows their boundaries. R2-F4 lists the exception where the loader knows the policy is invalid but the CLI omits or repeats its finding.

| Shared loader or validator | Valid controls and other defined outcomes | Malformed shapes checked through the public result |
| --- | --- | --- |
| Sequence data, YAML and Markdown frontmatter | Valid sequence mappings and rows, with positions shifted consistently by blank lines. A second YAML document stays file-only in the sequence data path where the parser supplies no coordinate | Duplicate keys, trailing syntax, and invalid frontmatter stop that source and keep any proven position |
| Sequence data, JSON/JSON5 | Valid JSON plus JSON5 comments and trailing commas | Duplicate keys and trailing garbage refuse the file, with coordinates where proven |
| Sequence data, JSONL/NDJSON | Blank lines are allowed between records | Duplicate keys and malformed records refuse the source at the record's file line |
| Sequence and task shape validators | An explicit empty sequence is a no-op. Missing `sequence:` is a separate error. `with: {}` is an explicit empty overlay. Selected foreign data can carry scalar and nameless rows that strict authored tasks refuse | Null, wrong whole type and wrong element types stay separate from empty input. The public graph tests cover bad middle rows, all-invalid lists, wrong-type `with:` and `when:`, nested/group/catalog consumers and executable reference type and empty-value checks |
| Keys with schema declarations | Legal group `fail_fast`, opted-in near-miss advice, pattern-declared keys, and absent retired keys | Explicit null and concrete values on removed keys yield the replacement diagnostic. Closed objects reject unknown keys according to their authored occurrence |
| Darkmatter configuration YAML and the `network` validator | Absent and null use the loader's defaults. Explicit `allow`, `ask`, and `deny` produce their documented policy outcomes | R2-F4 records absent reports for invalid tokens, empty string and a list in Claudine, and the results for duplicate keys, broken YAML and a second document. The public policy rejects malformed non-null values. The field is scalar, so element-only shapes do not enter the validator |

I checked the changed readers for load-bearing defaults, optional fields that collapse null into absence, string filtering, and ignored parse results. The loaded network setting's fallback is downstream of schema validation. The omission is in the CLI's use of the captured policy verdict. The YAML/Markdown, JSON/JSON5 and line-delimited loader tests exercise the public check result before CLI wiring tests assert the rendered locations. [Sequence graph checks](claudine/lib/src/composition/sequence/preflight/tests/check_tree.rs) · [Darkmatter policy tests](darkmatter/lib/tests/l1/config_fields.rs) · [Simplified Schema declarations](simplified-schema/lib/tests/l1/key_declarations.rs)

## Verification

| User-facing requirements | Verification level and evidence |
| --- | --- |
| Report contents, exit codes, stream separation, previews, source locations and reference JSON compatibility | Level 1 public CLI and library tests, plain and JSON snapshots, and direct Linux CLI probes. New coverage is needed for R2-F1–F4. |
| Passivity, provider and input eligibility, permissions, and per-invocation source observations | Level 1 marker fixtures, terminal-state injection for eligibility, public policy tests, and loopback request counts. Level 2 tmux capture confirms a dry run does not ask for shell approval. R2-F5 adds the missing total-request assertion for authorized reads. |
| Real preflight order, retry, persisted failure identity, and later task stop points | Level 1 real runs with provider stubs and the preflight baseline suite. Live probes in R2-F2 confirm the behavior after a prior write and a committed output. |
| Real-terminal note/report ordering, color and hyperlinks | Level 2 tmux captures. Direct and proxy typed blocks, styling and dry-run approval capture passed. The sequence-note capture is red and needs the assertion in R2-F6. The spec has no OS keyboard-event requirement. |
| Schema parity, collection recovery and load-bearing input shapes | Level 1 public graph, schema and DMLS/CLI parity tests and the matrix above. |

Linux `just test` completed for Claudine (13,344 passed), Darkmatter (18,516 passed), and Simplified Schema (2,646 default plus 2,222 minimal passed). No Level 1 test reached five seconds in these runs. The saved tier-coverage checks found no stranded tests in those areas. The targeted tmux sweep ran five tests with four passes and the R2-F6 failure. A separate color and hyperlink capture passed. The Windows timing evidence above comes from the saved repair runs.

The first review contains no human rulings. Its two post-cycle network items concern settled behavior that I recommend keeping. The new findings can be repaired within those decisions.

## Repair

> **started at:** 2026-11-27T05:08:02+00:00

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | A failed inclusion hides independent warnings from the recovered preview. **Class:** a recovery path that prepares or checks again after a failure projects only part of what it gets back. | fixed | One projection, `Check::warning_rows` in `claudine/cli/src/commands/document_check/preview.rs`, serves both success branches (deferrals, warnings, prompt, in report order). `refused_by_walk` now matches on code and file and compares lines only when both rows have one, which keeps the one-row rule for the first inclusion of an inline prompt. With either fix reverted, all five `*_both_missing` cases and both new Darkmatter inclusion-stop cases fail. |
| R2-F1.1 | Direct compose: both missing → error and warning; create `first.md` → warning; create `other.md` → clean | fixed | `dry_run_recovered_warnings::compose_{both_missing,first_created,all_created}`: exit 2 / 2 / 0 with the exact rows |
| R2-F1.2 | Inline compose, same body in `prompt:` → same three outcomes | fixed | `dry_run_recovered_warnings::inline_compose_*` (this route showed three rows before the `refused_by_walk` change) |
| R2-F1.3 | Sequence prompt file task → same outcomes under the task | fixed | `dry_run_recovered_warnings::task_prompt_*` |
| R2-F1.4 | Sequence inline prompt file task → same outcomes under the task | fixed | `dry_run_recovered_warnings::task_inline_prompt_*` |
| R2-F1.5 | Sequence body step → same outcomes plus the body-task info row | fixed | `dry_run_recovered_warnings::body_step_*`: `composition.no_executable` info row first in every state; the clean state exits 0 |
| R2-F1.6 | Sibling: `md compose` combined report after an inclusion stop dropped the reference check's warnings | fixed | `darkmatter/cli/src/commands/compose/combined.rs::after_inclusion_stop` keeps reported warnings in report order. A `warnings_delivered` flag, passed by both callers, keeps JSON from listing them twice. Tests: `compose_combined_report::{an_inclusion_stop_keeps_the_reference_warnings, the_lifecycle_routes_inclusion_stop_keeps_the_reference_warnings, a_refused_check_keeps_the_reference_warnings}` |
| R2-F1.7 | Sibling: `md` `after_refusal` | clean | Already reports every error and reported warning. Probe with both files missing showed the `first.md` error and the child's `other.md` row |
| R2-F1.8 | Sibling: `md` `after_check_failure` | clean | Its retry uses the same full projection as `after_refusal` |
| R2-F1.9 | Sibling: `inputs.rs::prompt_inputs` | clean | It prepares once and never retries. It reads only values; the preview owns warnings |
| R2-F1.10 | Sibling: `md compose` human output when composition fails after the reference check (for example `[x]()` with `{{ 3 / 0 }}`) prints only the error | clean | Contract: the spec keeps `md compose`'s "reference recoveries, warning policy" ("Blocking", "Other commands"). The non-dry failure is the existing standard report, and the warnings stay in `--output json`. This is md's settled policy on every failure, not a recovery branch dropping results. |
| R2-F1.11 | Sibling: `md` combined report's passive composition drops its own non-reference warnings (for example `unknown context variable`) | clean | Contract: the spec scopes `md compose`'s combined report to reference, call-check, schema-verdict and empty-body findings ("Phase 1", "Darkmatter reference failure in `md compose`" inventory row). Other composition warnings are outside that report. |
| R2-F2 | Prompt checks treat launch-time values as settled. **Class:** a check used the pre-run copy of a value that the run writes before that check executes. | fixed | One rule: the graph's `Loader::runtime_written_keys` (already used for provider prediction) now also reaches input preparation and the preview through `SequenceCheck::runtime_written`. The preview session declares those values with Darkmatter's new `PassiveSession::defer_written` (`DeferralReason::WrittenAtLaunch`, `DeferredConstruct::SchemaCheck`). Static `seq.*` members stay usable; `LAUNCH_TIME_SEQ_MEMBERS` lists the launch-time ones. Every sequence row of `claudine/cli/tests/l1/dry_run_later_state.rs` runs the dry run and the real run on the same files with the fake Goose provider. **Decision:** deferral is graph-wide, not ordered by task position. Ordering would need a second "written before this task" list beside the graph's rule. The cost is an occasional extra `not checked`, never a wrong verdict, and it is documented in `claudine/docs/topics/flow-control/sequences.md#dry-run`. |
| R2-F2.1 | Page block / empty body after a prior `set: topic` → conditional preview, unchecked body; `other` write → proven empty error | fixed | `dry_run_later_state::page_block_after_a_write_of_its_key_is_conditional` (dry 0, real 0); `…_an_unrelated_write_is_the_proven_empty_body` (`dm.compose.empty_body`, dry 2 / real 1). The block condition's `not checked` row is the deferred body verdict; no second empty-body row. |
| R2-F2.2 | Schema `topic: enum(a)` → verdict deferred only when an earlier write can affect `topic` | fixed | `schema_after_a_write_of_its_key_is_not_judged` (`not checked: schema check`, real 0); `schema_after_an_unrelated_write_is_the_proven_constraint` (`dm.schema.constraint`, real 1) |
| R2-F2.3 | Task input projection `with.topic: "{{ choice }}"` → dependency kept | fixed | `with_chain_*` rows, including `with_chain_under_another_name_*`; lib `check_tree.rs::a_with_value_reading_a_runtime_write_is_known_only_at_the_tasks_turn` |
| R2-F2.4 | Literal `with: { topic: a }` control → both modes exit 0 | fixed | `with_literal_beside_an_unrelated_write_is_proven` (no `not checked` rows, exit 0); `with_literal_under_a_write_of_its_key_passes_both_runs`. **Decision:** a literal `with:` under a runtime write of the *same* key is left unchecked. Verified: a writer setting `topic: b` overrides the literal at the task's turn (`task_caller_records`), and the real run fails, so the literal does not prove the launch value. |
| R2-F2.5 | Provider/model consumers → existing prediction preserved | already fixed | `dry_run_launch_hints` and lib `provider_dependencies` pass unchanged (inside the scoped 433 CLI / 518 lib runs) |
| R2-F2.6 | Body step with root `initialize` write → dependency preserved through discovery and preview | fixed | `root_initialize_writing_the_key_leaves_the_body_conditional` / `root_initialize_writing_another_key_is_the_proven_empty_body` |
| R2-F2.7 | `length(seq.outputs) > 0` guard → condition and body verdict unchecked | fixed | `a_later_steps_outputs_are_known_only_at_launch`. `sequence_jit::dry_run_composes_every_step_against_the_initial_state` asserted the rejected behavior. It is now `dry_run_composes_only_the_first_steps_outputs`: the first step outside any loop still reads `[]`. |
| R2-F2.8 | `seq.index == 2` / `== 1` → static metadata stays usable | clean | `a_true_step_position_composes` (real 0); `a_false_step_position_is_the_proven_empty_body` (dry 2 / real 1) |
| R2-F2.9 | Standalone compose/inline initializer writes → defer only where the real route reaches the initializer first | clean | Probed: in a real `compose`/`inline-compose`, the document's own `initialize` writes never reach its own composition (an `initialize` setting `topic: a` still renders "Read missing." and judges the schema on `missing`). So the static verdict is the real one. Tests: `a_standalone_initialize_writing_{the_key,another_key}_keeps_the_proven_empty_body` (dry 2 / real 1 both), `an_inline_documents_own_initialize_write_does_not_reach_its_prompt` |
| R2-F2.10 | `::file ./{{ topic }}.md` control → keep the real preflight reference check at its boundary | fixed | The reference walk uses a separate `preflight_session` that leaves written values out, because the real run checks inclusions before any task. `a_reference_after_{a_write_of_its_key,an_unrelated_write}_…`: `dm.reference.missing_transclusion`, real exit 1, zero launches |
| R2-F2.11 | Sibling: a `loop.*` read inside a looping container failed the dry run (`claudine.outside-loop`) while the real run exits 0 | fixed | The preview declares `loop` as launch-time inside a looping container. `a_loop_pass_value_is_known_only_at_launch` |
| R2-F2.12 | Sibling (severity class): an invalid optional `$schema` value was an error in the dry run while the real route drops it with a warning | fixed | Reuses the real classifier: new `composition::prepare_passive_with_schema` passes a schema refusal to the existing `translate::translate_schema_failure`. `PromptTask::drops_invalid_optionals` marks the routes that drop. New warning `composition.optional_dropped` is registered, with a fixture pair and human/JSON snapshots. Its message comes from the shared `DroppedOptional::message()`. `dry_run_dropped_optionals::compose_drops_an_invalid_optional_{literal,template_at_composition}`, `…_of_the_wrong_type`, `…_setter` |
| R2-F2.12a | inline-compose drop | fixed | `dry_run_dropped_optionals::inline_compose_drops_an_invalid_optional` |
| R2-F2.12b | Sequence root body step drop | fixed | `a_root_body_step_drops_the_roots_invalid_optional` (one row) |
| R2-F2.12c | Sequence root's own invalid optional (was silent) | fixed | `a_sequence_root_drops_its_own_invalid_optional` |
| R2-F2.12d | Sequence `prompt:` task (direct and inline) refuses | clean | The real pre-launch check (`task_run.rs::validate` → `validate_document_shell_late_values`) refuses before the drop could run; the dry run matches. `a_prompt_task_refuses…`, `an_inline_prompt_task_refuses…`. **Decision:** keep the existing real-run difference between `compose` and a `prompt:` task. Making the task drop would change real-run behavior, which this repair does not authorize. |
| R2-F2.12e | Nested / referenced body step refuses | clean | `a_nested_body_step_refuses…`, `a_referenced_documents_body_step_refuses…` |
| R2-F2.12f | Required-value controls | clean | `compose_refuses_an_invalid_required_value`, `compose_refuses_a_missing_required_value` |
| R2-F2.12g | `md compose` combined report schema verdict | clean | `compose_combined_report::an_invalid_{optional,required}_value_is_an_error_as_md_compose_refuses_it` (md refuses both, exit 1) |
| R2-F2.12h | An invalid optional beside an invalid required value: the dry run's error also listed the optional problem | fixed | The dry run reports the composition's located error cut down to the required problems, plus the `composition.optional_dropped` warning. Four rows in `dry_run_dropped_optionals.rs`. **Decision:** the warning is always shown. Today's real run prints it in only one of four literal/template combinations. The drop is a real authored concern that the run acts on as soon as the required value is fixed, and a warning never changes the exit status (2 either way). |
| R2-F3 | Inline section and schema-slot failures are counted twice. **Class:** one typed cause reaches the report through two passes that do not share its identity. | fixed | Inline sections: new typed Darkmatter API `Markdown::with_frontmatter_authored_in(path, text)`, used by Claudine's `document_for_mode`. The generated prompt carries the authored section's construct and position, so the existing `reported_by_walk` dedupe applies; no string matching. Schemas: the preview owns validation verdicts (located problems) and the input check owns an unpreparable `$schema` (real-run code). Failed slots travel typed as `SchemaSlotFailed.slots`. Tests: `claudine/cli/tests/l1/dry_run_one_row_per_cause.rs`, `darkmatter/lib/tests/l1/inclusion_target_sites.rs` |
| R2-F3.1 | Standalone inline compose missing prologue/epilogue → one named, located row | fixed | `dry_run_one_row_per_cause::inline_*`: "file not found in prologue:" at `notes.md:4:11`; epilogue list item at `6:5`; both missing → two rows; files created → exit 0, no rows |
| R2-F3.2 | Sequence inline prompt file → one located row per occurrence | fixed | Two tasks → four rows (prologue and epilogue × `sequence[0]`, `sequence[1]`), all located; fixed control clean. The real-run preflight report now also shows `notes.md:4:11`. |
| R2-F3.3 | Direct compose, chained prompt file, sequence body controls → preserved | clean | One located row each (`doc.md:4:11`, prompt task, `seq.md:3:11`); clean when fixed |
| R2-F3.4 | Body `::file` / divide-by-zero controls → one row per cause | clean | `dry_run_independent_references.rs` and `dry_run_recovered_warnings.rs` pass unchanged |
| R2-F3.5 | Schema slot illegal bare binding → one typed slot row per occurrence | fixed | Direct, inline, and two prompt-task occurrences each give one `dm.schema.slot_forbidden_binding` at `s.yaml:3:20`. A sequence root's own slot also gives the typed row. Legal `load_data` slot clean on all three routes. |
| R2-F3.6 | Sibling: a missing or unparseable `$schema` gave `composition.schema_load`/`schema_parse` plus `dm.schema.prepare` | fixed | One row with the real-run code at the `$schema` value (`doc.md:2:10`) on compose, inline and prompt tasks (`dry_run_one_row_per_cause`) |
| R2-F3.7 | Sibling: a sequence whose own verdict fails was reported again by its body step's preview | fixed | One owner row; the body step shows `not checked: preview (seq.md failed its own checks)` (`dry_run_one_row_per_cause`) |
| R2-F3.8 | Siblings checked: frontmatter `{{ 1 / 0 }}`, removed `setup:` key, md combined report (prologue, `::file`, slot, missing schema), tolerated-transclusion warning after retry, real-run `preflight_failure_report` | clean | Each probe gave one row per cause; covered by passing `dry_run_recovered_warnings`, `sequence_preflight_baselines`, `compose_combined_report` |
| R2-F4 | Dry runs omit invalid permission settings and repeat configuration parse errors. **Class:** an invocation-wide prerequisite verdict is either dropped by the passive check or re-reported by every subject that depends on it. | fixed | The configuration loads, the captured permission verdict, skipped triggers, retired guardrails and launch discovery are each one root finding. Dependent checks point at it (`related` / `not checked … the configuration does not load`). Dedupe is by `dm.config.*` code plus location, or the file the Claudine load reported. A broken Claudine reference is matched by data through the new Darkmatter field `ConfigProblem::reference_target`. Tests: `claudine/cli/tests/l1/dry_run_invocation_prerequisites.rs` (46 tests, each under 2.6 s) |
| R2-F4.1 | Darkmatter loaded policy invalid verdict → preserved | already fixed | `darkmatter/lib/tests/l1/network_approval.rs` (`sometimes` → `config_error()`); field shapes in `config_fields.rs::network_defaults_to_deny_and_refuses_every_other_shape`; duplicate key and second document in `config_load.rs` |
| R2-F4.2 | Compose/inline `network: aswk`, `""`, list → one located config finding, exit 2 | fixed | Before: all three exited 0 silently. `compose_network::*`, `inline_compose_network::*`: `allow`/`ask`/`deny` exit 0; each invalid shape gives one `dm.config.invalid` at `.darkmatter.yaml:2:12`, exit 2, one JSON envelope; `an_invalid_network_setting_is_one_row_in_the_human_report_too`. **Decision:** reported in dry runs only, not added to a real run's preflight failure report. A real local run tolerates the invalid value, so listing it there would misstate why the run stopped. Real-run behavior is unchanged. |
| R2-F4.3 | Sequence root and task: one finding per invocation for invalid values and for malformed YAML / duplicate keys / second document | fixed | `sequence_network::*` (named body task and prompt task): one root row, no task trail. `sequence_load_failure::{malformed,duplicate_key,second_document}`: one row; both tasks get `not checked (the configuration does not load)` with `related`. `*_claudine_reference::*`: a `~/.darkmatter.yaml` reference to a broken Claudine file gives one row under the document. |
| R2-F4.4 | Single-document YAML load failure controls → one located `dm.config.invalid` | already fixed | `compose_load_failure::*`, `inline_compose_load_failure::*` (exit 2, one row) |
| R2-F4.5 | Invalid network setting with remote read → refusal kept, zero requests, tied to config cause | fixed | `a_remote_read_under_an_invalid_setting_is_refused_and_relates_to_the_configuration`: zero loopback requests; refusal at `r.md:6:1` with `related` → `.darkmatter.yaml:2:12`. **Smallest repair:** Darkmatter flattens `RemoteReadError::InvalidConfiguration` into the refusal's message, so Claudine links the refusal by finding the configuration error's text inside that message. A miss costs only the `related` link, never a row. Threading a typed cause through `TransclusionError::RemoteFetchFailed` would be a larger change for no extra verdict. |
| R2-F4.6 | Absent/null permission value → no error | clean | `*_network::{absent_network,null_network,null_permissions}`: exit 0, no row |
| R2-F4.7 | Sibling: skipped schema triggers were dropped | fixed | `a_skipped_schema_trigger_is_one_warning_for_the_invocation` (one `dm.schema.invalid_trigger`) |
| R2-F4.8 | Sibling: retired inline guardrails warned once per inline task | fixed | `retired_guardrails_are_one_warning_at_the_guardrails_file` (one row at `.claudine/inline-compose.md`; routed by the now-public `RETIRED_GUARDRAILS_STAGE`) |
| R2-F4.9 | Sibling: failed launch-repository discovery was dropped (real run exits 1) | fixed | `a_failed_launch_repository_discovery_is_one_root_error` (compose and sequence) |
| R2-F4.10 | Sibling: unreadable launch directory printed the standard `Error:` and exited 1 | fixed | `an_unreadable_launch_directory_is_the_dry_runs_one_finding` (Unix only): one `io.read_failed` envelope, exit 2 |
| R2-F4.11 | Sibling: invalid `file-read` / `shell-commands` permission verdicts | fixed | The same verdict row covers them; manual probes of `file-read: 5` and `shell-commands: [""]` give one located row each |
| R2-F4.12 | Sibling: `register:` findings | clean | With `register: [./nope.yaml]` neither the real Claudine run nor the dry run reports anything; Claudine reads no registered hosts |
| R2-F4.13 | Sibling: request snapshot capture failure | clean | `InvocationContext::capture` errors are pushed once at the root and the check returns (`document_check/mod.rs`) |
| R2-F5 | Dry-run checks reread the same permitted sources. **Class:** independent check passes of one invocation each observe the same external source instead of sharing one observation. | fixed | One in-memory `PassiveObservations` on Darkmatter's `PassiveSession`: a `load_data` memo plus one remote runtime per authorization. `sharing_observations()` gives each pass fresh deferrals over the same reads. A runtime is shared only when policy, network, concurrency, freshness, cache-only mode and store all match (`authorizes_like`; `FetchPolicy` gained `PartialEq, Eq`). The policy check and slot gate still run before any memo hit. Nothing is persisted, and the combined report's passive composition uses `RemoteFetchRuntime::without_store`. Only source bytes are shared; each task evaluates its own inputs. Tests count total requests per URL: `claudine/cli/tests/l1/dry_run_shared_observations.rs`, `darkmatter/cli/tests/l1/compose_shared_observations.rs`, `darkmatter/lib/tests/l1/passive_shared_observations.rs` |
| R2-F5.1 | Direct `load_data` URL → one request | fixed | 2 → 1: `dry_run_shared_observations::one_request_per_url::compose_load_data` |
| R2-F5.2 | Inline `load_data` URL → one request | fixed | 3 → 1: `…::inline_compose_load_data` |
| R2-F5.3 | Sequence two occurrences of one URL → one request | fixed | 4 → 1: `…::sequence_load_data`; the undeclared-read warning still appears under both `first` and `second` |
| R2-F5.4 | Remote `::file`/`::code`/section transclusions → one per single-document mode; shared across identical task reads | fixed | Single-document stays 1 each; repeated-prompt sequence 2 → 1 each: `…::compose_transclusions`, `…::sequence_transclusions` |
| R2-F5.5 | External schema slot URL → shared input, per-occurrence verdict | fixed | Direct/inline 2 → 1, sequence 4 → 1: `…::{compose,inline_compose,sequence}_schema_slot`; only task `second` gets its `"Plan9"` error |
| R2-F5.6 | Local prompt source loaded once and handed to CLI checks | fixed | The walk hands its read to the CLI as `SequenceCheck::prompts`; `load_document` uses it first. New `Work::CompositionSourceRead`; lib `check_tree.rs::a_prompt_two_tasks_compose_is_read_once_and_handed_to_the_check` asserts one read for two occurrences, each with its own trail |
| R2-F5.7 | Two `load_data` calls to one URL → one request per distinct source | fixed | `…::one_url_read_twice` (1), `…::two_urls_read_once_each` (1 each) |
| R2-F5.8 | `md compose` preparation and combined report → one request in both | fixed | 2 → 1 in both: `compose_shared_observations::{a_clean_run_requests_a_frontmatter_url_once, a_combined_failure_report_requests_a_frontmatter_url_once}`; the clean `read_set` lists the URL once. A new `with_shared_remote_fetch_under(&remote_config)` keeps the reference check on its default configuration. Moving the whole remote configuration ahead of the reference check broke `compose_icmp::preflight_reports_planned_icmp_effects_and_their_cidr_grant` (the reference check sent a real ping). |
| R2-F5.9 | `ask`/`deny` controls → zero requests | already fixed | `dry_run_network_approval.rs` (all `ask`/`deny` cells) and darkmatter `network_approval.rs` pass |
| R2-F5.10 | Sibling: a failed verdict's classification fetched the slot again | fixed | `schema/translate.rs` reads through `composition::slot_session_observing`; it was the last extra request in `sequence_schema_slot` |
| R2-F5.11 | Sibling: the walk read each prompt three times (discovery, its missing-input rerun, the walk) | fixed | A check-only `PromptMemo` serves all three, never set in a real run; the same lib test's read count went 2 → 1 |
| R2-F5.12 | Sibling: the combined report's passive composition could write `md`'s cache store | fixed | `RemoteFetchRuntime::without_store`; `semantic_results_never_persist` passes |
| R2-F5.13 | Sibling: a cached `allow` result could answer a refused read | fixed | `passive_shared_observations::a_shared_{,slot_}fetch_never_serves_a_pass_whose_policy_refuses_the_host`: a `deny` pass sharing the store gets its own refusal and sends nothing |
| R2-F5.14 | Siblings: `$schema` URL, sequence source data, read-side Markdown functions, URL `file_exists`, group/task/catalog files | clean | A `$schema` URL is refused before any fetch (`SchemaError::RemoteUnsupported`). Sequence sources are local-only (`sequence_source_expression` cells send nothing). Read-side functions and URL `file_exists` go through the same shared remote runtime. Group/task/catalog files were already parsed once per invocation (`NestedSources`). |
| R2-F6 | The real-terminal sequence preflight test still expects the retired error block. **Class:** a terminal capture asserts a renderer the command no longer uses at that boundary. | fixed | One file, `claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs`; its doc comment is corrected too |
| R2-F6.1 | Sequence preflight tmux capture → note precedes report, located finding, nothing launched | fixed | Before the fix: failed alone at 0.954 s. After: passes in 1.006 s. It asserts, in pane order: the note; one report header after it; no `TransclusionError`; "file not found in `::file`" → `::file generated/notes.md` → `at target.md:7:1`; "The run stopped at:" naming the cause; exit 1; no provider prompt file or event, and no `generated/` |
| R2-F6.2 | Direct composition typed block → preserved | clean | `level2_typed_error_render_capture::level2_transclusion_failure_renders_darkmatter_block_in_tmux` (0.870 s) |
| R2-F6.3 | Lifecycle proxy typed block and styling → preserved | clean | `level2_lifecycle_initialize_proxy_compose_error_renders_styled_block`, the three `level2_lifecycle_diagnostic_matrix_*_is_route_equivalent`, `level2_proxy_routes_share_identity_across_routes_in_tmux`, `level2_initialize_proxy_block_carries_red_sgr_and_osc8_link_in_tmux` pass |
| R2-F6.4 | Dry-run terminal asks no shell approval → preserved | clean | `level2_dry_run_approval_capture::level2_dry_run_asks_no_shell_approval_in_tmux` (1.687 s) |
| R2-F6.5 | Sweep: other sequence-boundary L2 captures | clean | 77/77 targeted Claudine L2 tests pass under headless tmux. Six were WezTerm-only and skipped in about 0.004 s. |
| R2-F6.6 | Sweep: dry-run output captures | clean | `level2_lifecycle_dry_run_fires_no_lifecycle_events_and_no_proxy_traversal` asserts the current header; no L2/L3 test asserts `--dry-run-shell`, `Providers:`, "composed prompt is empty" or old preview headings |
| R2-F6.7 | Sweep: claudine level3, claudine/gen level2, darkmatter level2 | clean | No retired headers; darkmatter `level2_reference_report` + `level2_errors` 15/15 pass. The 8 `level2_errors` tests are WezTerm-only and skipped. |

### Outcome

**Complete.** Elapsed: 4 h 21 min (05:08 to 09:29 UTC).

Parent findings: 6 total = 6 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed. No finding or child row is open.

Final verification, run after the last change with no subagent running:

| Command | Result |
| --- | --- |
| `just lint` in `claudine`, `darkmatter`, `biscuit-file` | all exit 0 |
| `just test` in `claudine` | 13,483 passed, 16 skipped, none at or above 5 s |
| `just test` in `darkmatter` | 18,531 passed, 10 skipped, none at or above 5 s |
| `just test` in `biscuit-file` | 1,344 + 6 passed, none at or above 5 s |
| `just check-tier-coverage claudine` and `darkmatter` | 0 stranded |
| Targeted L2 (`just test-l2 …`, headless tmux) | claudine 77/77, darkmatter 15/15 |
| `just cross-check claudine-cli --os windows` | 4,615 passed. 24 untouched tests ran at 5.0–8.4 s in the package run, all at or under 4.555 s run serially alone on build-win, which matches the state R1-F9 accepted. None is a test this cycle added or changed. |

`cargo clippy -p claudine --features work-counters --all-targets -D warnings` fails with `result_large_err` in `claudine/lib/tests/l1/invalid_trigger_findings.rs`. The failure is already at HEAD: `CompositionError` is 136 bytes, and its only new field (`slots` on a variant of about 96 bytes) does not change its size. `just lint` does not build that feature.
