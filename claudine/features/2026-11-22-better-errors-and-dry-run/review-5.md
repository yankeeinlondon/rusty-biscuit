---
$schema: feature-review.yaml
ready: false
findings:
  - id: R4-F1
    priority: high
    description: Dry runs judge initializer-dependent files against stale filesystem state
  - id: R5-F1
    priority: high
    description: Dry runs overlook frontmatter rewrites during initialization
  - id: R5-F2
    priority: high
    description: A repeated destination pattern can discard a content-changing write
  - id: R5-F3
    priority: high
    description: A later task preview ignores files written by an earlier task
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-27T16:49:46+00:00"
spec: 2026-11-22-better-errors-and-dry-run/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-better-errors-and-dry-run/spec.md`"
feature: 2026-11-22-better-errors-and-dry-run/review-5.md
previous: 2026-11-22-better-errors-and-dry-run/review-4.md
---

# Review 5 — Better errors and a diagnostic dry run

**Production readiness is false.** Two previous findings are verified fixed. The repair for initializer-dependent files still misses two directive readers. Three newly found cases also let a dry run report a failure from state that the live command has already changed by the time it reads it.

## Previous Finding Verification

The required instances come from the [review 4 repair ledger](./review-4.md#finding-ledger). Review 4 verified the remaining finding carried from review 2 and all the review 3 repairs. The earlier reviews contain no human rulings and have no other open findings.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R4-F1 — Dry runs judge initializer-dependent files against stale filesystem state | R4-F1.1–.18 | **partially fixed** | Claudine Level 1 verified .1–.4 across five prompt routes and the section, code, graph, and schema readers. For .5–.9 the expression, listing, schema-import, existing-content, and root-list cases respected literal and templated destinations. The .10 boundary controls passed, including the headless tmux sequence case. Darkmatter Level 1 verified .11–.13 for fragment, graph, and preflight reads. Claudine passed the seven .14 resolver cases and the .18 launch-context guard. For .15, repeated-copy resolution uses the same deferral rule, the launch file opens before initialization, location-only reads only attribute findings, and file discovery itself only supplies the listing. The claimed clean directive readers, however, produced false verdicts in paired CLI runs. The .16 volume-sensitive unit test passed on Linux and the previous ledger records the repaired Windows separator result. The .17 literal-anchor decision stands. [Claudine fixtures](claudine/cli/tests/l1/dry_run_initialized_files.rs) · [Darkmatter checks](darkmatter/lib/tests/l1/lifecycle_created_files.rs) · [Resolver cases](claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs) | R4-F1.15 remains open for `::toc-linking` and `::file-links`. R5-F1 through R5-F3 are newly found siblings. |
| R4-F2 — Dry-run prompt checks lose the enclosing group scope | R4-F2.1–.10 | **verified fixed** | Claudine Level 1 checked group placement and all three member spellings (.1), body/frontmatter/schema plus the inline prompt (.2–.3), collection sites and both transclusion boundaries (.4–.6), and input, provider, model and live-report controls (.7–.8). The input and retry use the same preview binding (.9). The early reference walk clears it, and the approval and Phase 1c probes retain their earlier scope (.10). [Group fixtures](claudine/cli/tests/l1/dry_run_group_scope.rs) · [Preview binding](claudine/cli/src/commands/document_check/preview.rs) | None. |
| R4-F3 — `try_frontmatter` turns a deferred read into a failed read | R4-F3.1–.14 | **verified fixed** | Claudine and Darkmatter Level 1 passed the network and initialized-local cases and sibling readers (.1–.3), including zero requests under `ask`. Editor observation and the slot refusal gate passed (.4–.5). The filesystem readers, logical operators and pure predicate preserve the read boundary (.6–.10). The nested fold, expression wrappers and their consumers preserve typed deferrals (.11–.12). The ignored errors in .13 concern parsing or resolving a reference. Provider prediction in .14 emits a refusal with no value for an unsupported read. The new `try_frontmatter` row below concerns an upstream write that was never declared. [Claudine fixtures](claudine/cli/tests/l1/dry_run_recovery_wrappers.rs) · [Darkmatter wrappers](darkmatter/lib/tests/l1/recovery_wrappers.rs) · [Error propagation](darkmatter/lib/src/markdown/compose/expression/error.rs) | None for this finding. |

## Unblocked Findings

### R4-F1 — Dry runs judge initializer-dependent files against stale filesystem state (high, carried)

**Authority (spec).** [Preflight and dry-run parity](./spec.md#parity) requires the dependent check to stay unchecked when initialization writes a file the live command needs. [Checks](./spec.md#checks) reserves an empty-prompt finding for a proven empty result.

**Class and invariant.** The checked directive makes a filesystem verdict at the wrong time. The exact-path and glob forms must defer when the declared write can supply their output. Moving the writer outside the selected path must preserve the real absence or empty-body verdict. An append that changes headings can change a table of contents. It cannot add an already existing file to a listing.

The existing observation is available to ordinary inclusions and `find_files`. The TOC branch selects a target and reads headings without the same deferral check. The file-links branch discovers and renders a listing from the current directory entries. In a prompt made entirely of one of these directives, this can lead to an error and exit 2 even though the live run launches. [TOC graph and references](darkmatter/lib/src/markdown/reference/graph.rs#L885) · [Composition and listing](darkmatter/lib/src/markdown/compose/transclusion/engine.rs#L1640)

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R4-F1.15a | TOC target selection, reference walk, and missing-target report | The prompt contains only `::toc-linking ./gen/facts.md`. The root ensures that file with a heading, then in the control ensures `gen/other.md`. | Dry run reports `dm.reference.missing_transclusion` for both. Matching live run exits 0 and launches once. Unrelated live run exits 1 without launching. | Defer target selection for the matching write. Keep the missing-target error for the unrelated write. |
| R4-F1.15b | TOC heading read, generated links, and empty-body check | Keep a blank `gen/facts.md` and append a heading there or to its sibling. | Both dry runs report `dm.compose.empty_body`. Only the matching live run launches. | Defer the heading read and dependent body verdict on a match. Keep the proven empty result with an unrelated append. |
| R4-F1.15c | File-link glob and directory listing, preview, and empty-body check | `::file-links ./gen/*.md` or `::file-links --dir gen`, with the initializer creating a Markdown file in `gen/` or only in `elsewhere/`. Also append to a file already present in `gen/`. | Both listing forms report an empty prompt before a matching creation even though the matching live run launches. Unrelated creation outside `gen/` leaves the same empty verdict in both runs. The existing-file append control is clean in both. | Defer the listing and dependent body verdict only when the write may change the listing. |
| R4-F1.15d | Other sites in the repair ledger's reader sweep | Repeated `::file` copies, ordinary block and section directives, fragment validation, and `find_files`. Root loading and location reads were inspected. | Level 1 covered missing and appended targets on the ordinary readers. Repeated copies share the target check, fragment validation checks the content observation before reading headings, and `find_files` already asks whether a listing could change. The root loads before the event, and diagnostic location reads supply positions. | Retain those boundaries. |

Have the TOC and file-list readers consult the existing content, creation, and listing observations before treating their results as final. Use the same deferral for the TOC graph's generated records and the preview. This needs no change to what either directive renders during a live run.

### R5-F1 — Dry runs overlook frontmatter rewrites during initialization (high)

**Authority (spec).** [What runs and what does not](./spec.md#what-runs-and-what-does-not) says that values lifecycle events would write stay unchecked where a later check needs them. [A real run uses the same checks](./spec.md#a-real-run-uses-the-same-checks) places root initialization before graph construction and preserves the state visible at each later boundary.

**Class and invariant.** The dry run omits a whole family of file writes from its launch-state observation. A literal destination and a template resolving to that same file must give the same verdict. Moving the write to a different file must preserve a verdict about the unchanged target. A frontmatter rewrite can change a document's values without changing whether the file exists.

Darkmatter's initializer collector recognizes `ensure_file`, `append_line`, and `append_jsonl`. Its lifecycle executor also runs seven frontmatter mutations and mirrors writes on the current document into the values that later initializer actions read. Claudine's initializer destination and value checks only collect keys written by `set`. As a result, a dry run can report an empty prompt, a missing nested prompt, or a missing include that the live run resolves. [File-write collection](darkmatter/lib/src/lifecycle/created_files.rs#L135) · [Lifecycle frontmatter writes](darkmatter/lib/src/lifecycle/executor.rs#L1418) · [Claudine initialization observations](claudine/cli/src/commands/document_check/mod.rs#L435)

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R5-F1.1 | Seven frontmatter verbs, then the prompt's page-block and empty-body checks | Change an existing `gen/facts.md` with `set_frontmatter`, `merge_frontmatter`, `delete_frontmatter`, `increment_frontmatter`, `decrement_frontmatter`, `append_frontmatter`, or `prepend_frontmatter`. A page block tests the changed property. Use literal and unrelated-file operands for all seven, and a static templated operand for the single-key verbs. | Each verb's matching case reports `dm.compose.empty_body` in dry run but launches live. Unrelated writes leave the genuine empty verdict. `set_frontmatter` reproduces on compose, inline-compose and sequence. Inline-compose launches and then the fake provider fails to update the artifact. | Leave the dependent block and prompt verdict unchecked for each matching write. Preserve the unrelated-file verdict. |
| R5-F1.2 | Expression document load, wrappers, and schema verdict | Read the updated property with `frontmatter` and `try_frontmatter(...).value`. Add a required key through the same writer and check `validate_schema`. | All three matching dry runs report an empty prompt. Only the matching live runs launch. The `try_frontmatter` read succeeded against old bytes, so the already repaired deferral propagation never ran. | Defer the changed content in the shared reader before either wrapper or schema judgment uses it. |
| R5-F1.3 | Reference graph, approval discovery, then nested composition | Put a gate over the included document's own `ready` value and include it through `::file`, `prologue:`, or `epilogue:`. Set `ready` in its file or its sibling. | All three report an empty prompt in dry run. Only the matching live runs launch. The file already exists in each case. | Leave dependent content unchecked. Keep independent reference and prompt checks. |
| R5-F1.4 | Claudine task graph and dependent prompt selection | Rewrite the `prompt:` or task list in a referenced Markdown task, group, catalog entry, nested sequence, or root sequence source from a missing prompt to `./p.md`. Separately rewrite a referenced prompt's own guard value. | The five graph forms report a missing document before the live run resolves and launches its prompt. The ordinary prompt file produces a false empty-body verdict. The unrelated writer controls preserve the live refusals. | Defer the changed document and graph decisions that read it. Continue checking unaffected siblings. |
| R5-F1.5 | Current document's values and a later initializer action's operand | Change `ready` in the invocation document itself. Then have a separate fixture change its `dest` before `ensure_file: ["{{ dest }}", "facts"]`, followed by `::file ./gen/facts.md`. Compare with writing those properties to another file. | Compose and inline-compose give a false empty-body result on the first fixture. The root sequence body control refuses in both modes. The second fixture gives a false missing transclusion on compose. The live initializer uses its newly written destination and launches. The unrelated writer keeps the missing-file error. | Mark values that the relevant live boundary rereads as unchecked. Carry a self-write into the existing dependency test for later initializer destinations. Keep the sequence's demonstrated earlier stop. |
| R5-F1.6 | File existence, listings, and readers whose result does not load Markdown frontmatter | Check existence and a glob with both target files already present. Read a raw body/title from the same fixtures. Inspect `load_data`, raw code and document reference validation. | Existence, listing, body-emptiness and heading-title controls agree. `load_data` does not consume Markdown frontmatter as a data format. Raw `::code` does read changed bytes when including a Markdown file, even though it ignores their semantics. Plain link existence is stable. | Preserve stable existence/list results. Defer a raw code preview or any other content read of the rewritten file where it matters. |
| R5-F1.7 | Child reference interpolation and `as_markdown` | The initializer switches a child's `part` from `missing` to `exists`. Read the child with an authored `::file` and separately with `as_markdown("::file ./gen/facts.md")`. Its own include is `::file ./{{ part }}.md`. | Both matching dry runs report a missing transclusion from the old value. Both live runs launch. Unrelated writers preserve the missing-reference result. | Defer the affected child content in discovery, reference analysis, and nested preview. |

Use the lifecycle executor's existing frontmatter-verb inventory to declare an existing-file content mutation alongside creation and append. The graph and expression readers already know how to ask about changed content. For a proven self-target, also mark its changed keys where the live flow rereads them, including the initializer's later operands. A missing destination of a frontmatter mutation is still missing. This repair needs no prediction of the value written inside the event.

### R5-F2 — A repeated destination pattern can discard a content-changing write (high)

**Authority (spec).** [Preflight and dry-run parity](./spec.md#parity) requires an initializer-dependent read to remain unchecked. [Severity](./spec.md#severity) requires a proven failure before an error is reported.

**Class and invariant.** Deduplication loses a later declaration with broader effects on the same match. For an existing target, an `ensure_file` immediately before an append has no effect on the final content and must leave the same deferral as the append alone. Dropping the append must restore the ordinary content verdict.

Darkmatter's `declare_initializer_write` keeps the strongest effect for a resolved file identity. `declare_unresolved_write` retains only the first entry for a matching pattern. An author can use one path variable in an initializer that ensures a file and appends to it. The dry run can then report the existing blank file as the final prompt even though the append fills it. [Declaration merge](darkmatter/lib/src/markdown/compose/passive.rs#L419) · [Existing-file pattern predicate](darkmatter/lib/src/markdown/compose/passive.rs#L624)

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R5-F2.1 | Unresolved destination merge, content read, and empty-body verdict | In `initialize`, use `set: {dest: facts}`, then ensure and append to `gen/{{ dest }}.md`. The prompt only includes the initially blank `gen/facts.md`. Compare append-only, reversed order, ensure-only, and an `elsewhere/` destination. | Ensure then append reports `dm.compose.empty_body`. Append-only and reversed order both defer. All three matching configurations launch live. Ensure-only and the unrelated pattern preserve the empty verdict. | All matching appends give the same unchecked content verdict for the existing file. |
| R5-F2.2 | Resolved destination merge and unchanged decisions | Write both destinations literally. Follow the pattern through existing-file content, absent-file existence and listing, and existing-file existence. | The two literal declarations defer content as expected. The absent-file predicate accepts either write kind and the listing predicate uses the pattern itself, so the merge only loses information at the existing-file content decision. Existing regular-file existence is independent of the merge. Content readers share this predicate. | Keep those decisions and retain the strongest effect for an identical pattern. |

Combine the write kinds when the same pattern is registered twice, as the resolved path already does.

### R5-F3 — A later task preview ignores files written by an earlier task (high)

**Authority (spec).** [Preflight and dry-run parity](./spec.md#parity) explicitly requires a fixture where an earlier task writes a file needed later, the live run succeeds, and the dry run leaves the dependent check unchecked. [A real run uses the same checks](./spec.md#a-real-run-uses-the-same-checks) preserves the earlier graph checks and the later task-turn checks.

**Class and invariant.** The task-turn preview reuses an invocation-wide write observation that only sees root initialization. A declared file writer before its reader can make the later result unavailable to dry run. Reordering the writer after the reader, or moving the write to another file, must preserve the reader's genuine verdict. A reference the live run reads during preflight still has to resolve then.

In a sequence, put `side_effect: {ensure_file: ["gen/facts.md", "Generated"]}` before a prompt whose only block tests `file_exists('gen/facts.md')`. The writer is correctly shown as skipped. The dry run then declares the block false and exits 2 for an empty prompt. The live run exits 0 and launches the reader once. Claudine carries earlier value writes into prompt preparation. Its file observations are populated from the root's `initialize` before the walk and carry no earlier-task placement. [Observation setup](claudine/cli/src/commands/document_check/mod.rs#L435) · [Per-task value preparation](claudine/cli/src/commands/document_check/tasks.rs#L771)

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R5-F3.1 | Earlier side-effect task, later expression and empty-body verdict | Use `ensure_file` before `file_exists`, `append_line` before `markdown_body_empty`, `append_jsonl` before `load_data`, or `set_frontmatter` before `frontmatter`. Move the writer to an unrelated file. | Each matching dry run gives an empty-prompt error. The corresponding live runs exit 0 with one launch. Unrelated writers preserve the empty verdict, with no live launch. | At the reader's turn, defer its file read and dependent block. Preserve the unrelated-file result. |
| R5-F3.2 | Earlier graph reference and prompt's own early reads | Replace the expression with a static `::file` of a still-absent file. Separately put the writer in the reader prompt's own `initialize`. | Dry and live both refuse the static missing include before the write. The prompt's own initializer did not rescue the expression-only empty prompt in the live probes either. | Preserve these early verdicts. Apply a later write only at checks the real run reaches after it. |
| R5-F3.3 | Task `with:` and inline prompt preparation | Put `file_exists` in a task `with.ready`, then in the only block of an inline prompt. Change the earlier writer to an unrelated file. | Both matching dry runs give an empty-body error. The matching live runs launch. The fake provider leaves the inline artifact unchanged and that run subsequently exits 1. Unrelated runs both refuse before launch. | The later input binding and either preview mode use the same task-turn file observations. |
| R5-F3.4 | Reader before writer | Move the matching writer below the reader. | Both runs report an empty prompt and dispatch no provider. | Keep the reader's proven verdict when no earlier task can write its input. |
| R5-F3.5 | Preview of an existing inclusion | The earlier task appends to an existing blank file. The later prompt only contains `::file ./gen/facts.md`. Move the append to an unrelated file for the control. | Existence is established during early discovery. The matching dry run still reports an empty prompt. The matching live run launches and the unrelated one refuses. | Keep the early existence result and defer the later content verdict. |

Use the shared file-write observation with the task placement already carried by the sequence walk. Add supported file writes from earlier tasks to the later task-turn view. Keep the graph and early approval observations at their existing boundaries. There is no need to execute a task, simulate shell commands, or predict the contents a writer will produce.

## Blocked Findings

None. The specification supplies the behavior for each repair.

## Recurrence

- **R4-F1** carries the TOC and file-list part of the reader sweep marked clean in review 4, R4-F1.15. These are unchanged misses. The earlier sweep covered ordinary inclusions, expression listings, and fragment checks, but the adjacent directive target, heading, and directory readers did not consult the same observation.
- **R5-F1** is a new sibling of review 4's R4-F1, and of the launch-time state class in review 2's R2-F2. The file sweep stopped at effects that can create a file. It also needed the existing-file frontmatter verbs and the executor's current-state update that a later initializer operand reads.
- **R5-F2** recurs under review 4's R4-F1.16 pattern work. This is a regression introduced by the unresolved-pattern declaration. The sweep needed to compare coalescing an identical pattern with coalescing a resolved identity, then check both the existing-content and existence decisions.
- **R5-F3** is a new sibling of review 3's R3-F3 and review 4's R4-F1.10. Their earlier-task probes correctly preserved a preflight reference refusal. The missing counterpart was a file read at the later task-turn boundary, where the spec explicitly requires the other result.

The shared rule has a bounded scope. Use the declared write kinds and the live reader's boundary. The previous decision to keep the fixed portion of an unresolved destination as its anchor still applies.

## Input Robustness Matrix

Invalid required shapes and malformed documents produce a located refusal without dispatch or publication. Where the loader can identify an invalid row independently, valid sibling rows stay checkable. The initializer repair observes already-parsed lifecycle actions. It does not add a second lifecycle configuration loader.

| Loader or validator | Control and exceptions | Evidence |
| --- | --- | --- |
| Sequence data and task graph | Explicit empty sequence is an empty plan, missing `sequence:` is diagnosed, selected foreign rows may use scalar or nameless data, and `with: {}` is an empty overlay. | Public graph checks distinguish absent, null, wrong whole and element types, all-invalid lists, and an invalid row beside a valid sibling. YAML/Markdown positions, duplicate and trailing syntax, JSON5 comments and trailing commas, and JSONL blank lines are exercised through the data loader. [Graph cases](claudine/lib/src/composition/sequence/preflight/tests/check_tree.rs) |
| Schema-derived task and group keys | A group may use `fail_fast`. An open step may retain ordinary step data. A removed key is refused when present even with a null value. | The Claudine occurrence suite and the Simplified Schema declaration cases check known, removed, and opted-in keys, including repeated occurrences. [Occurrences](claudine/cli/tests/l1/dry_run_schema_key_occurrences.rs) · [Declarations](simplified-schema/lib/tests/l1/key_declarations.rs) |
| Darkmatter permission configuration | Absent and null fields use the scope defaults. An empty permission mapping expands its defaults, and an explicit empty file-read list replaces its default. Valid `allow`, `ask`, and `deny` settle independently. | Public loader tests cover wrong field and element types, duplicate keys, trailing or malformed input, and YAML's null spelling. The Claudine route fixtures assert one located configuration finding and no effect dispatch. [Fields](darkmatter/lib/tests/l1/config_fields.rs) · [YAML loader](darkmatter/lib/tests/l1/config_load.rs) · [CLI wiring](claudine/cli/tests/l1/dry_run_invocation_prerequisites.rs) |
| Claudine's invocation configuration | The absent user configuration takes the passive default without writing a file. | The CLI prerequisite suite exercises valid and invalid configuration across compose, inline-compose and sequence. The loader's error remains invocation-wide. [CLI cases](claudine/cli/tests/l1/dry_run_invocation_prerequisites.rs) |

The source and permission loaders were checked for unvalidated serde defaults, null collapsed to absence, silent element filtering, and ignored parse failures. The permission defaults above are explicit in the public loader tests. The findings concern the provenance and timing of a value after loading succeeds.

## Verification

| User-facing requirement | Verification |
| --- | --- |
| Consolidated reports, stable codes, task order, authored sites, and human/JSON stream and exit contracts | Level 1 Claudine snapshots and diagnostic guards, CLI dry-run corpus, Darkmatter combined-report cases, and byte comparisons for the legacy reference output. |
| Passivity, effect deferrals, provider and input asking, independent checks and previews | Level 1 process markers, loopback request counts, injected terminal observations, and matched live/dry CLI probes. The false file-dependent verdicts are listed above. |
| Live preflight ordering, persisted failure identity, retry, and the task-turn boundary | Level 1 baseline tests and paired fake-provider runs. Level 2 detached tmux verified the early-stop note followed by the report and the original exit. |
| Reused observations, per-occurrence verdicts, schema keys and editor parity | Level 1 source-open counts, public graph and schema preparation, schema-key occurrence cases, and fresh DMLS/Claudine parity for UTF-8 and UTF-16 with LF and CRLF. |
| Terminal presentation and prompt suppression | Level 2 detached tmux verified red SGR and OSC 8 on the typed proxy block and confirmed that the dry run asks for no shell approval. The feature has no OS keyboard-event requirement. |

The focused Linux Claudine run returned 815 passing assertions and flagged six timings above five seconds. The host's one-minute load reached 38.74 on 16 logical CPUs during the run. Each flagged case cleared in isolation at the times below. The Claudine graph and real-run baseline suite passed 57 tests, and the initializer resolver passed seven. Darkmatter's wrapper, configuration, and editor/placement runs passed 76, 64, and eight tests respectively. Those focused runs recorded zero slow cases. All three selected Level 2 tmux cases executed and passed. Cross-OS evidence is left to CI.

| Claudine Level 1 case | Package run | Isolated |
| --- | ---: | ---: |
| Referenced sequence input appears once and is previewed | 5.280 s | 2.875 s |
| Portable paths, magic-root order, and CRLF positions | 5.539 s | 2.889 s |
| A failed occurrence does not turn the next into a cycle | 5.302 s | 2.713 s |
| Selected catalog keys are reported at their occurrence | 5.723 s | 2.634 s |
| Dry-run and debug traces redact provider tail secrets | 5.492 s | 2.934 s |
| The nested owner's schema failure matches the live refusal | 5.232 s | 2.642 s |

The implementation log now describes where Darkmatter emits an inclusion-only JSON trail. The dated correction records the direct CLI comparison that verified the distinction. [Implementation log](./implementation-log.md#2026-11-27-review-5-correction)
