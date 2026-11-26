---
$schema: feature-review.yaml
ready: false
findings:
  - id: R1-F1
    priority: high
    description: Sequence source expressions can start a shell during a dry run
  - id: R1-F2
    priority: high
    description: A bad row in a root sequence source hides valid siblings
  - id: R1-F3
    priority: high
    description: Required inputs suppress independent checks and previews
  - id: R1-F4
    priority: high
    description: Provider prediction treats runtime-written values as settled
  - id: R1-F5
    priority: high
    description: Remote reads cannot defer on network approval
  - id: R1-F6
    priority: high
    description: Legacy reference JSON baselines were changed
  - id: R1-F7
    priority: high
    description: Schema key findings lose task occurrences and duplicate parser findings
  - id: R1-F8
    priority: high
    description: Live inline sequence steps can corrupt a YAML source
  - id: R1-F9
    priority: high
    description: Native Windows Level 1 timings exceed the review limit
  - id: R1-F10
    priority: high
    description: Sequence load findings drop structured source locations
  - id: R1-F11
    priority: high
    description: The report fixture guard does not cover the scoped diagnostic catalog
  - id: R1-F12
    priority: high
    description: Darkmatter drops the unchecked body verdict after a failed preparation
  - id: R1-F13
    priority: high
    description: Available file choices are reported as hard failures
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: false
created: "2026-11-26T21:04:07+00:00"
spec: 2026-11-22-better-errors-and-dry-run/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-better-errors-and-dry-run/spec.md`"
feature: 2026-11-22-better-errors-and-dry-run/review-1.md
---

# Review 1 — Better errors and a diagnostic dry run

**Production readiness is false.** A dry run can execute a shell through sequence expressions, and several check paths give authors incomplete or incorrect results. The repairs below fit the existing decisions and can proceed without an author ruling.

## Findings

### R1-F1 — Sequence source expressions can start a shell during a dry run (high)

**Authority (spec).** “An expression function the catalog classifies as effectful is not called and is reported as skipped.” The spec requires the shared dispatch boundaries to enforce this across nested documents and callbacks. [What runs and what does not](./spec.md#what-runs-and-what-does-not)

The defect class is a passive execution context lost at an evaluator boundary. A pure function should produce the same value through every source and preview route. Changing only that function to one that probes the login shell should preserve the independent work and leave its dependent value unchecked without starting a process.

Use a disposable `bash` selected through `SHELL` that records each invocation and returns an alias result. Use the same `has_alias('ll')` call in the root expression, reference operator, formal template, nested template, and prompt body. Substitute `file_exists('./ask.md')` for the pure control. The root expression uses a ternary to select the list. The operator uses a ternary that returns the same row on either branch. Both effectful and pure source expressions parse and return a task.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Root `sequence: "{{ … }}"` | Ternary over the effectful call | Exit 0, one shell launch, no deferral | No shell launch; list decision unchecked |
| R1-F1.2 | File-reference `::template(…)` operator | Effectful predicate over one selected row | Exit 0, two shell launches, no deferral | No shell launch; dependent result unchecked |
| R1-F1.3 | Root formal `template:` | Field assigned the effectful call | Exit 0, one shell launch, no deferral | No shell launch; field and dependents unchecked |
| R1-F1.4 | Formal template on a nested inline sequence | Same field and leaf prompt | Exit 0, one shell launch, no deferral | Same as root formal template |
| R1-F1.5 | Prompt body interpolation | Same effectful call | Clean. No shell launch, informational deferral | Same |
| R1-F1.6 | Pure controls on all five paths | Replace the call with a local existence check | Clean for the effect invariant. No shell launch and the source resolves | Same |
| R1-F1.7 | Command-backed source and explicit shell/task dispatch | Existing marker fixtures | Clean. The sequence command and task markers stay absent | Same |

The sequence evaluator constructs a local expression context without a passive session. The runtime and dry check enter it through the root source resolver, the reference operator, or the shared formal template evaluator. The prompt-body path already has the shared guard. Carry the passive context into these source evaluations, including discovery, and preserve deferral through list, row, and template consumers. Test all routes with a harmless marker and a pure control. [Claudine source expression lookup](claudine/lib/src/composition/sequence/expr.rs) · [Reference operator](claudine/lib/src/composition/sequence/source.rs) · [Formal template evaluator](claudine/lib/src/composition/sequence/formal.rs) · [Darkmatter function dispatch](darkmatter/lib/src/markdown/compose/expression/functions/mod.rs)

### R1-F2 — A bad row in a root sequence source hides valid siblings (high)

**Authority (spec).** “A failed subtree becomes findings and the walk continues with siblings whose boundaries are known.” The repository also requires moving inline rows into a referenced file to preserve behavior. [What the check pass walks](./spec.md#what-the-check-pass-walks)

The defect class is whole-list failure after successfully decoding independent rows. Moving three rows between sources of equivalent shape should retain the first and third findings. Changing only the middle row to a malformed task should add a finding at that row.

Use missing prompt paths in the first and third rows. Give the middle row a valid `shell:` task as the control, then add a second executable or change its `with:` to an invalid value. The same split also holds with working outer prompts, where failing routes preview zero and recovering routes preview two.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Root formal YAML reference | Invalid middle row between two prompt tasks | Middle error only, whole task list unchecked | Both siblings checked |
| R1-F2.2 | Root bare YAML data reference | Middle task declares both `shell` and `prompt` | Middle error only, whole task list unchecked | Both siblings checked |
| R1-F2.3 | Root expression returning an authored array | Same two executables in middle row | Middle error only, whole task list unchecked | Both siblings checked |
| R1-F2.4 | Inline Markdown and directly invoked formal YAML | Malformed middle row, including a scalar where rows are strict | Clean. Both siblings checked | Same |
| R1-F2.5 | Nested inline and referenced sequences | Invalid middle row, known boundaries | Clean. Per-row recovery preserves siblings | Same |
| R1-F2.6 | Later graph resolution on the failing source routes | Middle `prompt: 123` reaches the executable loader | Clean for this defect class. All three rows reported | Same |
| R1-F2.7 | Entire file fails to parse | Invalid YAML, JSON5, JSONL or Markdown frontmatter | Clean for this defect class. No sibling boundaries can be recovered inside that file; enclosing tasks continue | Same |

Reuse the recovering source normalization for the root reference and expression routes. Preserve strict rows for a formal document and scalar conversion for selected data. Handle offsets and operators before recovering rows where the selected collection exposes its boundaries. Keep the real run’s original preflight decision. [Claudine recovering root planner](claudine/lib/src/composition/sequence/preflight/check.rs) · [Shared reference planner](claudine/lib/src/composition/sequence/source.rs)

### R1-F3 — Required inputs suppress independent checks and previews (high)

**Authority (spec).** “The placeholder stays in the preview, marked missing, and the input is a finding.” The check also walks prompt references condition-blind, uses the real input policy, and reports one finding per missing input. [What runs and what does not](./spec.md#what-runs-and-what-does-not) · [Severity](./spec.md#severity)

The defect class is using a missing input as a whole-document barrier. Only the code that reads the missing input is unavailable. A prompt with a required `topic`, static text, a `{{ topic }}` span, and an independently broken `::file` should report both problems. Supplying `topic=docs` should remove the input finding, and repairing the file should remove the reference finding. For inline composition, put the static text and placeholder in its frontmatter `prompt:`.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Direct compose | Required string absent; independent body inclusion | Generic schema error, no preview or inclusion finding. Supplying topic reveals the inclusion error | Missing-input finding with remedy, independent inclusion finding, eligible text preserved |
| R1-F3.2 | Inline compose | Missing `string(required; eager)` used in `prompt:` | Generic schema error, no preview. Supplying the input renders the prompt. A completion-only required value can stay absent under the mode’s rule | Report an eager input under the mode’s collection policy and mark its placeholder. Preserve the completion-only rule |
| R1-F3.3 | Sequence composed from its own body or `prompt:` | Root owner lacks topic | Missing input plus whole preview unchecked | Mark missing input and evaluate available prompt content |
| R1-F3.4 | Sequence prompt task | Prompt lacks its own input | One input finding and whole preview unchecked, with no independent reference finding in direct-body mode | Keep the input finding, reference check, and available preview |
| R1-F3.5 | Prompt that depends on a missing sequence-owner input | Existing nested task fixture with a required `spec` and derived `with:` | Existing test requires empty stdout | Keep dependent fields deferred and keep available content and checks |
| R1-F3.6 | Provided input | Same inputs with topic supplied and the reference repaired | Clean in the focused preview controls. The available input renders | Same |

Use the canonical passive preparer to carry missing values as unavailable, report input verdicts through the mode’s collection policy, and run independent checks with placeholders. Include terminal observations for askability and a non-collectable input shape. The gap can be proved with ordinary CLI fixtures and injected terminal state. [Claudine single-document check](claudine/cli/src/commands/document_check/mod.rs) · [Claudine sequence input and preview checks](claudine/cli/src/commands/document_check/tasks.rs) · [Existing nested input fixture](claudine/cli/tests/l1/dry_run_check.rs)

### R1-F4 — Provider prediction treats runtime-written values as settled (high)

**Authority (spec).** “A value only known at launch (`seq.*`, `loop.*`, a runtime `set`)” leaves a dependent check unchecked. Static values remain checkable when prediction proves them. [How each effectful check is handled](./spec.md#how-each-effectful-check-is-handled)

The defect class is a launch-time dependency stripped from prediction before selection. A derived hint and its equivalent direct expression should leave the same dependent provider and model checks unchecked. Changing the earlier writer to set an unrelated key makes the hint statically known and restores the appropriate error.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F4.1 | Prompt frontmatter agent | Earlier `success` sets `pick: goose`; later prompt authors `pick: nosuch` and `agent: "{{ pick }}"` | Dry run reports an unknown provider and says the real run fails before task one | Provider and dependent model decision unchecked |
| R1-F4.2 | Task `with.agent` through document inputs | `pick: claude`, `mid: "{{ pick }}"`, `choice: "{{ mid }}"`; earlier task writes `pick: goose`; reader uses `agent: "{{ choice }}"` | Dry run rejects missing Claude. Real run launches the locally stubbed Goose twice | Model/provider verdict deferred until launch |
| R1-F4.3 | Provider reviews and model checks consume the prediction | The same dynamic provider followed by the required-model lookup | The static provider rejection wins before launch or the model is skipped for “no provider” | Use the launch-time target outcome and its reason for both decisions |
| R1-F4.4 | Independent hint and real graph | Earlier task writes an unrelated key; same later hints | Clean. Dry and real non-interactive runs both refuse the unsatisfied static target | Same |

Use the existing graph treatment of runtime-written keys before projecting selection findings, including dependencies reached through intermediate values. Keep the runtime selector’s established precedence and its successful static cases. Add fixtures for a late model when the provider is settled and for nested/enclosing selection sources that share this graph decision. [Claudine real graph provider analysis](claudine/lib/src/composition/sequence/preflight/providers.rs) · [Claudine dry-run target selection](claudine/cli/src/commands/document_check/selection.rs)

### R1-F5 — Remote reads cannot defer on network approval (high)

**Authority (spec).** The recorded network decision is “`allow` fetches, `ask` becomes an `info` finding instead of a prompt, `deny` is the real run’s refusal.” The acceptance tests require a loopback request count. [Decisions](./spec.md#decisions) · [Exit status and passivity](./spec.md#exit-status-and-passivity)

The defect class is a permission decision missing from the policy consumed by dry-run readers. Relocating a supported remote read between equivalent prompt routes should leave the permission decision intact. Changing only `allow` to `ask` should change an otherwise clean fetch to an informational deferral with zero requests. `deny` should remain a refusal with zero requests.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F5.1 | Darkmatter configuration policy | `permissions.network` set to each of `allow`, `deny`, `ask` | `ask` is rejected by the configuration schema. The policy can only represent allow, deny, or invalid | An explicit approval-required decision |
| R1-F5.2 | Claudine compose, inline-compose, sequence prompt | `load_data` against a loopback responder in the active prompt | Allow fetches. Deny refuses without a request. Ask returns an evaluation error for invalid configuration, with no request | Ask produces an informational deferral without a request |
| R1-F5.3 | Transclusion runtime | Remote `::file` and `::code` use a loopback child | Each allow sends one request. Deny sends zero and refuses. Ask sends zero and returns invalid-policy error | Common allow/ask/deny decision, with unavailable contributions on ask |
| R1-F5.4 | Dynamic schema slots | An enum slot reads the same loopback data | Allow sends one request. Deny sends zero but reports both an approval deferral and an unresolved-schema error. Ask sends zero and reports invalid configuration | An allow fetch, a typed refusal on deny, and only the applicable deferral on ask |
| R1-F5.5 | Sequence source expression | `sequence: "{{ load_data(..., 'string[]') }}"` against the same responder | The local-only lookup disables remote reads for every setting, with no request | Preserve this surface's local-only contract and make the distinction explicit in coverage; apply the captured policy where the real surface permits remote reads |

Represent the opt-in decision at the shared permission boundary, then handle it at the remote read dispatch in a dry run before fetching. Keep existing default permissions and explicit grants. Cover `allow`, `ask`, and `deny` for includes, data, and schema reads, and verify dependent checks and absence of persistent writes. The spec already supplies the permission and its behavior. [Darkmatter network policy](darkmatter/lib/src/config/network.rs) · [Darkmatter slot session](darkmatter/lib/src/markdown/schemas/slots/session.rs)

### R1-F6 — Legacy reference JSON baselines were changed (high)

**Authority (spec).** “`md validate refs` reuses the report renderer and preserves its existing text, machine output, and exit behavior.” The snapshot must be byte-identical. The plan makes the same commitment for the existing graph-reference presentation. [Other commands](./spec.md#other-commands) · [Rendered-output snapshots](./spec.md#rendered-output-snapshots) · [Phase 6](./plan.md#phase-6--shared-report)

The defect class is an enriched record leaking into a compatibility projection. An internal renderer swap must leave the legacy bytes alone. Repairing the missing target should still change the reference report and exit status.

I ran the shipped `md` against the reference baseline fixture's missing hyperlink in the parent and child and missing image, with and without listing. The diff from the baseline committed before the final projection changes confirms the exact fields that changed.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F6.1 | `md validate refs --output json` | Missing hyperlink and image | Adds `base`, `resolved`, and `hint` to records | Legacy normalized JSON bytes |
| R1-F6.2 | `md validate refs --show-all --output json` | Same broken records alongside the listing | Same additions | Legacy normalized JSON bytes including the listing |
| R1-F6.3 | `md graph --validate --output json` | Same document | Same additions | Legacy graph JSON bytes per the plan |
| R1-F6.4 | Plain and Markdown reference presentations, clean and empty reference JSON | Existing unchanged baseline cases | Clean. Baseline tests pass and those fixtures have no diff | Same |
| R1-F6.5 | New compose and Claudine findings | Missing include or document target | Clean for adding evidence on these surfaces. The new records expose the fix and path | Continue exposing the new evidence |

Keep the enriched internal record and use the legacy projection on the compatibility routes. Restore the original three snapshots and verify with the existing fixture. Serialize non-empty inclusion and iteration trails on the new compose and dry-run contracts where they are available. The host-only serialization shortcut currently applies across surfaces, so a new direct-compose record with only an inclusion or iteration trail has only the old projections. [Reference baselines and CLI cases](darkmatter/cli/tests/l1/reference_report_baselines.rs) · [Shared diagnostic serialization](darkmatter/lib/src/markdown/compose/expression/diagnostics.rs)

### R1-F7 — Schema key findings lose task occurrences and duplicate parser findings (high)

**Authority (spec).** The collector must “deduplicate equivalent findings for the same subject, authored location, and typed cause” and keep them separate for distinct task occurrences, even when those tasks share a file. DMLS and runtime key findings must agree. [The collector](./spec.md#the-collector) · [Parity](./spec.md#parity)

The defect class is assigning a key verdict to its source file rather than its authored property in a particular task occurrence. Running an outlined sequence twice should yield the same per-occurrence advice as writing two equivalent inline occurrences. Changing `when` to `whem` on a closed leg with a valid prompt should add one located key error. A legal open step with `promt` alongside a valid `prompt` should add one advisory.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F7.1 | Standalone `kind: task` file | Valid `prompt` plus `whem: "true"` | A located schema row and a parser row for the same typo, with different hints | One located row at the task occurrence |
| R1-F7.2 | Inline group leg in formal YAML | Same typo | Located schema row, whole formal-schema error, and parser row | One typo verdict under the leg |
| R1-F7.3 | Referenced group file leg | Same typo | File-level schema row and task-level parser row | One typo verdict under the leg |
| R1-F7.4 | Referenced formal YAML sequence used twice | Accepted step data `promt` beside its executable | One advice row attached to the file with no task trail | One per occurrence with its trail |
| R1-F7.5 | Referenced formal Markdown sequence | Same accepted step data | No key advice; the CLI only validates Markdown at the root in this check | Same advice as its YAML and inline equivalents |
| R1-F7.6 | Root open step and literal `when` controls | Existing root near-miss fixture and clean spelling on closed tasks and legs | The root advisory is emitted, and valid `when` yields no typo finding | One advisory under the authored step, and no typo on legal keys |
| R1-F7.7 | Selected catalog group | Select `second@./cat.yaml`, with a valid second entry and `whem` in the unselected first entry | Exit 2 with the unselected entry’s key error, attached to the file | Attach verdicts to selected occurrences and keep unrelated entries out of their report |
| R1-F7.8 | Unchecked rows for repeated task references | Two tasks use the same child file and both cannot check provider and inputs | Human headings identify each task. The JSON rows carry no host trail and their related locations can be identical | Preserve the task occurrence in each task-scoped diagnostic |
| R1-F7.9 | Missing-document and empty-agent overlap controls | Two occurrences of one missing prompt and one invalid empty agent list | Clean for these other classes. Missing prompts keep their separate trails; the empty-agent duplicate is suppressed in the focused fixtures | Same |

Validate each loaded document’s authored schema keys once, retain the resolved file and full property with each verdict, then attach it to affected graph occurrences. Use that identity to suppress equivalent parser or aggregate schema rows in the report while retaining the real run’s selected typed error. Check parity through the shipped Claudine CLI as well as the embedded schema validator, which the present DMLS parity fixtures cover. [Claudine schema-key collector and parser projection](claudine/cli/src/commands/document_check/tasks.rs) · [DMLS schema parity](darkmatter/dmls/tests/l1/claudine_schema_key_parity.rs)

### R1-F8 — Live inline sequence steps can corrupt a YAML source (high)

**Authority (defect).** A supported sequence document, `kind: sequence` in YAML with a top-level `prompt:`, has its source rewritten into invalid YAML after a successful task edit. The dry-run path and live task path both recognize the document-level inline prompt. [Claudine sequence task run](claudine/cli/src/commands/wrap/sequence/task_run.rs) · [YAML prompt preview fixture](claudine/cli/tests/l1/dry_run_check.rs)

The defect class is selecting the output format from the transient Markdown model instead of the underlying source. Changing a task's body should update through a format-compatible write path. Running the same sequence with a single step versus two reveals the damage immediately on the second read.

Use a disposable Goose stub that makes a valid YAML edit during task one, like an inline agent editing an authored field. With `prompt: "Step {{ seq.name }}."` and `sequence: [first, second]`, the real `claudine sequence seq.yaml --goose --yolo` prepends a Markdown frontmatter envelope with its hash/date stamp, then fails to load the source for task two.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F8.1 | Direct YAML body tasks using the document-level inline prompt | One edited task | Exit 0, YAML source has a Markdown fence | Source still parses as YAML |
| R1-F8.2 | Same source, two tasks | First edits, second rereads | Task two never launches; YAML multiple-document error | Source parses and task two can run |
| R1-F8.3 | Real inline Markdown document | Same two tasks, first writes in the document’s format | Clean for this defect class. The second launch is reached | Same |
| R1-F8.4 | YAML sequence whose tasks point to separate Markdown prompts | Existing real CLI control | Clean. It does not publish the YAML root as a modified inline Markdown prompt | Same |
| R1-F8.5 | Dry-run YAML inline preview | Existing prompt fixture and two-row reconstruction | Clean for this defect class. It previews and does not publish | Same |

Apply the YAML-safe update or omit Markdown-only stamp publication on this source route, whichever is consistent with the existing live write contract. Verify the control with an actual provider-stub edit. A stub that only prints produces an earlier no-change failure and cannot prove the write-back behavior.

### R1-F9 — Native Windows Level 1 timings exceed the review limit (high)

**Authority (spec).** The spec requires the repository test toolkit and area test recipes for feature acceptance. The review's mandatory Level 1 speed gate fails a test at 10 seconds or more under any load. A recorded result above 5 seconds and below 10 needs the host load and a passing isolated remeasurement to clear it. [Exit status and passivity](./spec.md#exit-status-and-passivity) · [Recorded Windows timings](./implementation-log.md#cross-os-evidence-just-cross-check---os-windows-build-win)

The defect class is too much serial work in a Level 1 test process. The test should retain the same assertions and fleet roster when cases are split. Changing one fleet's binding or lifecycle should still fail the test for that fleet. The Windows gate's result is decided by elapsed time even when all assertions pass.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F9.1 | `research_sequence_artifacts::shipped_fleet_graphs_resolve_per_row_prompt_lifecycles` | One process builds 22 shipped fleet graphs | 11.0 s on Windows. The final test still loops over all fleets | Named per-fleet or bounded cases plus a roster check, each within the limit |
| R1-F9.2 | `dry_run_check::a_dry_run_writes_nothing_anywhere` | Three command routes in one no-write test | 6.1 s on Windows with no isolated Windows timing or recorded load | Split the route cases or clear the original time with matching native measurements under the review rule |
| R1-F9.3 | `a_missing_prompt_file_is_named_by_its_noun_and_cause` | Earlier version ran two dry reports and a live stop in one test | 5.9 s on Windows; the live stop has since moved to its own test and both pass below the limit on Linux | Measure both current tests on native Windows |
| R1-F9.4 | `a_task_file_yaml_error_names_the_file_and_the_parse_stage` | Same pattern for a malformed YAML task | 5.3 s on Windows; the live stop has since moved to its own test and both pass below the limit on Linux | Measure both current tests on native Windows |
| R1-F9.5 | Earlier multi-route and empty-agent loops | Previously slow tests already split into named cases | Clean in the recorded reruns or replaced by current focused cases | Keep the assertions and split structure |

The 11.0 s result establishes the failure without a load exception. The 6.1 s no-write test has no evidence that clears it. The two changed snapshot cases need current Windows timings before carrying forward either older result. Reuse setup that does not depend on the fleet and let nextest schedule independent fleet cases. Measure both in isolation and in the package run, recording host load when using the loaded-run allowance. [Fleet graph test](claudine/cli/tests/l1/research_sequence_artifacts.rs) · [Dry-run timing cases](claudine/cli/tests/l1/dry_run_check.rs)

### R1-F10 — Sequence load findings drop structured source locations (high)

**Authority (spec).** The error standard requires “the place it was written, in the file’s frame,” using the existing authored-position conventions. Its sequence inventory specifically requires locations for failures of source resolution, reading, and parsing. The input robustness rules require the public result to carry the location for invalid input. [The error-message standard](./spec.md#the-error-message-standard) · [Inventory](./spec.md#inventory)

The defect class is flattening a known authored location into a message or discarding it at projection. Leading comments or blank lines should shift the reported position by the same amount while retaining the failure code. Fixing only the malformed token should remove the finding. Unknown positions should remain unknown.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F10.1 | Referenced YAML and Markdown loaders | Duplicate key under a task; invalid frontmatter | The file is retained or quoted, but the check's projection takes only the file from the sequence site. Position stays in lower-layer text when the loader supplies one | Structured authored line and column when supplied by the parser |
| R1-F10.2 | JSON/JSON5 loader and JSONL/NDJSON loader | Trailing token in JSON5; repeated key on JSONL line 3 after a blank line | The message contains a line or column while `source.line` and `source.column` are null | Preserve the available coordinate in the JSON source and human location row |
| R1-F10.3 | Executable reference validation for prompt, task, group and sequence | Existing valid references versus wrong scalar or empty string on a task | The validator records a property internally, but the diagnostic projection emits the task trail or file without the authored value position | Use the validated executable's property to locate the authored value |
| R1-F10.4 | Runtime wrappers around a typed error | A display wrapper around a reference or task-site error | `sequence_site` unwraps display wrappers, but the dry-run resolution-class path reads only the outermost error and `resolution_failure` has no wrapper case | Preserve the known reference class and location through a display wrapper |
| R1-F10.5 | Confirmed no-match with known YAML value position | Fleet Run 2 prompt reference | Clean. The structured source points to the value and `failure` is `no-match` | Same |
| R1-F10.6 | Invalid YAML document with no parser coordinate; valid shared loader controls | A second YAML document, valid JSON5 punctuation, and a legal blank JSONL line | Current YAML error has a file but exposes no coordinate from that parser; valid controls load | Preserve every coordinate the cause can establish. Keep file-only when no position can be proved |

Carry parser positions as data when the cause has them and locate grammar failures from the existing property path. Use the same projection, including traversal of presentation wrappers, in dry and real preflight reports. Keep the original typed cause for lifecycle selection. The same change should preserve the typed class and directive position for a tolerated read or parse failure, which currently surfaces as a generic `dm.compose.warning` even though the including directive and failure cause are known. [Claudine projection](claudine/cli/src/commands/document_check/mod.rs) · [Sequence data loaders](claudine/lib/src/composition/sequence/data.rs) · [Preview warning projection](claudine/cli/src/commands/document_check/preview.rs)

### R1-F11 — The report fixture guard does not cover the scoped diagnostic catalog (high)

**Authority (spec).** “Every new or changed report code in this spec has a fixture and a rendered snapshot (terminal plain and JSON).” The guard must fail when the code, fixture, and two rendered snapshots stop matching. [Enforcement](./spec.md#enforcement) · [Rendered-output snapshots](./spec.md#rendered-output-snapshots)

The defect class is testing agreement between copies of an inventory that production can bypass. Moving an existing case to an equivalent fixture should leave coverage intact. Adding a scoped declaration without its matching report fixture must fail a guard.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F11.1 | Claudine catalog versus `dry_run_check::codes` | Review the registrations and the guard’s inventory | The guard compares fixture directories and snapshots with a local nine-code list. A new registry entry does not change that comparison | Determine scope from declarations or assert that every scoped declaration is in the fixture list |
| R1-F11.2 | Darkmatter changed finding catalog | Missing-reference, removed key, near-miss, empty body, skipped check and warning paths | There are individual fixtures and report tests, but the catalog test checks membership/uniqueness. It cannot fail when a scoped report code has no rendered fixture pair | Connect each scoped code to its fixture and both outputs |
| R1-F11.3 | Claudine sequence error block snapshots | Thirteen changed block/projection shapes in the existing suite | Clean as selected rendering examples. Both projections are checked but their list does not guard an added production code | Preserve these checks and link their applicable cases to the guarded inventory |
| R1-F11.4 | DMLS versus embedded registration parity | Shared fixture across LF/CRLF and UTF-8/UTF-16 | Clean for the embedded and editor verdicts. The Claudine command does not consume that same parity fixture | Route the fixture through the shipping check so an omission such as R1-F7.5 fails |

Make one test inventory per owner, backed by its scoped production codes, and reuse the fixtures that already assert both forms. For an internal fallback with no authored input, exercise it using a deterministic projection seam. The spec requires the output contract and fixture guard at Level 1. Neither a real terminal nor an actual provider is needed to prove them. [Claudine fixture guard](claudine/cli/tests/l1/dry_run_check.rs) · [Claudine diagnostic registry](claudine/lib/src/diagnostics/registry.rs) · [Darkmatter findings tests](darkmatter/lib/tests/l1/findings.rs)

### R1-F12 — Darkmatter drops the unchecked body verdict after a failed preparation (high)

**Authority (spec).** For `md compose`, “If a bad call or failed preparation prevents a check from reaching a verdict, record the later check as unchecked.” The same pre-composition boundary applies after lifecycle initialization. [Other commands](./spec.md#other-commands)

The defect class is computing a diagnostic and discarding it because it is the only additional finding. A document with a missing inclusion and a passive evaluation failure such as `{{ 3 / 0 }}` cannot prove whether its body would be empty. Fixing only the expression should remove the unchecked finding and leave the reference result. A known-false block still proves emptiness.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F12.1 | Ordinary `md compose` reference stop | Broken inclusion and `{{ 3 / 0 }}` | Only `dm.reference.missing_transclusion`. The shared collector inserts an unchecked row without marking that it contributed | Reference finding and unchecked body verdict |
| R1-F12.2 | Lifecycle reference stop after `initialize` | Add an informational initialize action to the same document | CLI again emits only the reference finding without the unchecked row | Same as ordinary route, after initialization |
| R1-F12.3 | Independent call/schema verdicts | Broken reference alongside a call the static check rejects, or a missing required schema property | Clean. The cause appears with the later check marked unchecked | Same |
| R1-F12.4 | Proven and deferred conditions | Broken reference inside an excluded block, or a condition that needs a command result | Clean. Established false conditions can warn about an empty body. The command-dependent fixture keeps the body unchecked | Same |
| R1-F12.5 | Reference-only and stdin routes | Remove the failing expression, or use the established stdin reference eligibility | Clean. A broken inclusion alone gives the original reference report. Stdin still follows its existing policy | Same |

Keep the unchecked result from the failed preparation on both report paths. The implementation already has the required unchecked row and a helper that marks it as contributing. There is no need to widen the first-error rule for expression typing or to execute the document to prove the verdict. [Darkmatter combined check](darkmatter/cli/src/commands/compose/combined.rs) · [Lifecycle boundary](darkmatter/cli/src/commands/compose/lifecycle.rs)

### R1-F13 — Available file choices are reported as hard failures (high)

**Authority (spec).** “An eligible bare-name picker or file-input chooser is described as a pending choice when an existing passive search proves matches.” This applies in both report formats without showing the chooser. [Severity](./spec.md#severity)

The defect class is bypassing a recoverable file miss before the passive candidate check. An eligible match with a terminal should be reported as a pending choice. Using the exact path should resolve it, removing the candidate should report a miss, and an explicitly anchored typo should stay a hard failure. A redirected non-interactive command should use its real refusal policy.

The terminal-backed CLI probes used a mode-appropriate `spec.md` in the fixture’s user prompts directory and passed `spec.md` as the root argument. The eager input probes used a separate `deep/spec.md` with `file(required; eager; match(deep/*.md))` and passed the partial input `spec=spec`.

| ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F13.1 | Root file selection for compose, inline-compose and sequence | Bare name with a mode-eligible candidate | All three report hard `dm.reference.missing_document` before examining picker eligibility | Describe the pending choice without prompting |
| R1-F13.2 | Supplied eager file on all three commands | Partial name with a matching glob candidate | All three report `dm.schema.invalid_file_reference` | Describe the choice when the real route would offer it |
| R1-F13.3 | Sequence task input | Same partial name via prompt-task input preparation | Returns the raw reference refusal. Dry task preparation has no chooser projection | Use the owning input route's admission and interaction rules, and defer the verdict where otherwise eligible |
| R1-F13.4 | Exact root and input references | Pass the full path to the located document or eager input | Clean. The focused controls resolve without asking | Same |
| R1-F13.5 | Explicitly anchored misses and no candidate | Existing missing-file fixtures and eligibility predicate | Clean for the hard-miss case. Anchored misses do not reach autocomplete; the fixture reports an actionable no-match | Same |

Call the existing read-only discovery for eligible root misses and factor the read-only part of eager file completion out of the real chooser if needed. Reuse its admission rules, preserve explicit references, and test the report through a focus-safe Level 1 terminal-state fixture. This review exercised a pseudo-terminal report and no picker interaction. [Claudine root dry-run resolver](claudine/cli/src/commands/document_check/mod.rs) · [Existing operation-file candidate lookup](claudine/cli/src/completion/operation_file.rs) · [Caller file input completion contract](claudine/docs/topics/composition.md#provided-partial-file-references)

## Input robustness matrix

The public loader or resolver should reject invalid shapes with a located cause, without launching or publishing anything. The dry-run graph can continue when it has other known task boundaries. R1-F2 covers rejection that incorrectly consumes valid siblings. R1-F10 covers rejection that loses known coordinates.

- Root `sequence` distinguishes omission (a body task) and an empty list (zero rows). Explicit null, booleans, numbers and objects fail. An authored list rejects null, numbers, booleans and arrays as rows, including mixed and all-invalid lists. Selected data deliberately converts scalar names. Null and array records in a selected list still error.
- Declarations use presence and type checks. The public resolver tests compare bare YAML null and absent `$state` on direct and referenced sources. Template fields can hold null, empty strings, lists and mappings as data. Only top-level strings assigned as template values evaluate. Removed spelling tests cover omission, explicit null and a present value. `group.fail_fast` remains legal.
- Task `with:` takes an absent key or an empty map as no override. Wrong whole-field types are refused by the shared mapping validator. `when:` takes an absent key as unconditional. Null, empty or malformed expressions are refused. The graph fixtures cover invalid and valid group legs together and all-invalid lists. The standalone task, group, catalog, and nested routes reach these grammar and graph validators.
- The direct YAML and referenced YAML loaders reject duplicate keys, trailing invalid syntax and a second document. Bare-key null survives parsing and reaches the declaration validator. Markdown frontmatter separately rejects duplicate keys and preserves bare null. JSON and JSON5 share a loader that rejects duplicate keys and trailing content while accepting comments and trailing commas. JSONL and NDJSON skip blank lines, detect duplicate object keys and refuse a malformed trailing record. The CLI fixture checks that a separate sibling still previews and its shell marker stays absent. The matrix distinguishes parse refusal from the missing structured coordinates listed in R1-F10.
- The passive Claudine configuration loader reuses the canonical YAML validation and accepts a missing configuration using defaults. An invalid Claudine YAML configuration produces a root error while checked documents still contribute findings. The network policy's rejected `ask` value is separately covered in R1-F5.

I checked the collection readers around `serde(default)`, optional fields, string filtering, and parse errors discarded by `.ok()` or defaults. The recovery projections merit repairs at the sites above. For example, the schema-key check silently skips referenced Markdown and the task reference projection skips known positions. The underlying loader controls still distinguish malformed content from a valid empty source. [Loader and shape tests](claudine/lib/src/composition/sequence/tests.rs) · [Graph checks](claudine/lib/src/composition/sequence/preflight/tests/check_tree.rs) · [Removed key tests](simplified-schema/lib/tests/l1/key_declarations.rs)

## Implementation decisions (informational)

Two reviewers independently read the log and spec. The plan authorized ownership of `--json` before `--` unless the author overruled it. No override was recorded. The plan also directed the canonical passive preparer and permitted outstanding cross-platform evidence to be recorded. The finished decision list contains no item that needs a ruling or a separate author review.

- **Early rollout.** The first passive command used a fixture switch after the old preview suite exposed missing traversal and selection behavior. It temporarily kept the old preview and `--dry-run-shell`, duplicated the inline walker and provider/model precedence, and used a separate preview, code list and renderer. Shell dependency stopped the whole preview at that stage. The later phases replaced these prototypes with the shipping dispatch, the canonical preparer and shared selectors. The eight-phase plan governed the completion marker when the shorter spec phase count initially disagreed. Source-relative reference evidence and launch-relative hints were used where their respective context was available.
- **Schema declarations.** The implementation made `removed(...)` a named, block-form property, stored it with boxed key metadata, refused patterns, reserved and inline declarations, and lowered it to a false property with owner metadata. The parser derives removal text by schema role, preserving the value-specific `execution: serial` example. It closed standalone tasks and group legs, kept open step data and the catalog root, and treated a present null removed key as removed. The owner is the problem path and the key token is the location. This preserves the schema's ownership of the grammar.
- **Schema suggestions and editor output.** Open objects need explicit advice, exact per-declaration exclusions, and a unique close candidate. Intentional patterns admit extension keys. The implementation reused the `md clean` ranker, made advice a compose warning, and left advice out of `md schema validate`'s failure records. DMLS appends the hint to the message and supplies it as data. Related declaration coordinates were left empty because the prepared schema lacks their source positions. This is appropriate where no location can be proved. The initially root-only key sweep grew to cover loaded YAML, with the outstanding placement and Markdown gaps in R1-F7. A type alias and boxed call-check value addressed adjacent lint limitations.
- **Graph and input discovery.** Check paths use authored `sequence[i]`, and the real run retains its existing `tasks[i]` identity. Referenced row numbering uses the loader's existing order. Recovering discovery leaves an unreadable owner's input inventory to the walk's error. Nested sources recover by row. Root references and expressions still use fail-fast normalization, and parse causes initially carried their positions only as text. R1-F2 and R1-F10 cover the remaining work. Temporary assumptions about prompting and standalone lifecycle checks were replaced by passive configuration and canonical preparation.
- **Passive evaluation and preview.** Deferral uses authored text with a typed reason, and propagates across fallback, lookup and collection evaluation. Shell stages are skipped with the directive authored text recorded. Provider queries are treated as effects. The slot session uses read-only reference-check mode, recording slot error text when it lacks a single authored span. Interpolation deferrals lack line data where the rewrite pass lacks source positions. Empty-body causes use the observed top-level exclusions, and a no-executable finding takes precedence. The canonical preparer reads guardrails without creating them. It receives the walk's evaluated `with:` values beneath explicit setters, uses a syntactic check on body-task `with:`, and holds checks for late values or a missing owner's input. Human output marks deferred syntax in plain text and JSON carries diagnostic rows only. Required-input handling, source expression passivity and approval deferral remain ordinary repairs.
- **Startup, flags and selection.** Passive configuration follows the launch repository and uses defaults on absence. Invalid Claudine configuration produces a root finding and checking continues with defaults. The plan's default settled the initial author flag about provider `--json` ownership. Explicitly selecting an uninstalled provider is an error. Root literal `initialize` assignments become unchecked inputs. Dynamic assignments and graph-wide runtime writes did not all reach dry-run provider prediction. The unknown-error fallback is `internal.bug`; it has no deliberate authored fixture. Old preview tests that asserted retired shell prompting, metadata, performance and headers were removed or migrated. `--perf` remains accepted in dry-run mode without the old performance display.
- **Shared findings and references.** Host frames are prepended to existing document frames. The implementation emits full JSON trails only when host frames exist and uses the legacy projections otherwise. The reference table retains its own presentation model and shares renderer helpers. The author-facing finding caps suggestions at three while recovery retains five. It hides a case-only alias when it identifies the same file. Document-reference checks admit paths as the existing file-read permission governs schema slots. Usual empty `md compose` output warns and can be denied. After a reference failure the combined report is used when the collector contributes another verdict. Canonical prompt preparation uses the passive shell alias record. R1-F1 covers the other evaluator routes.
- **Live preflight.** A report wraps a settled failure when errors or warnings exist, leaving the original lifecycle facets and persisted cause in charge. Informational rows are omitted from the live failure report. A footer names the primary failure when no existing row carries its text. Advisories on successful preflight are limited to the authored-key and graph walk, before task one, once outside `--silent`. Input descriptions and provider remedies were restored to report rows. The guard classifies the remaining non-sequence fallback variants as outside scope. The implementation split several repeated-route tests and used a scratch clone while the concurrent research move broke compile-time paths.
- **Final error sites and release.** Boxed optional `SequenceSite` records the innermost known site on the way out, retaining pre-existing lifecycle details and code. Prose names the construct and uses one-based steps, while property paths remain zero-based. Empty-agent duplication was removed. The still-overlapping task and schema key rows go to R1-F7. Tolerated preparation warnings now count toward exit 2. Their generic code and missing directive location go to R1-F10 along with the logged `WithFrontmatter` reference-class gap. Windows JSON paths use forward slashes while Unix filenames retain literal backslashes. Legacy plain reference tables were retained and enriched JSON snapshots were re-blessed. R1-F6 specifies the compatibility repair.
- **Deferred work and logistics.** The phase log deferred the live YAML inline write-back bug outside the dry-run switch and routed three tests around it. R1-F8 includes the verified defect. The log also notes an older model-warning rendering issue involving underscores. The review has no independent reproduction of that rendering issue. Network approval and its request-count test waited because the existing policy only has allow/deny. R1-F5 gives a compatible repair. The Darkmatter equivalent is the existing `notes.md` report fixture, with the additional failed-preparation case in R1-F12. The fleet graph split was left unfinished and is R1-F9. The final phase repaired moved research paths and generated baselines, kept a visible temporary rejection for the draft research schema, and published caller skills. Remaining macOS, WSL2 and broader native Windows evidence can follow the repository's CI schedule.

## Verification

I ran focused Linux Level 1 tests through the area recipes. Claudine CLI passed 79 cases covering diagnostics, argument and stream behavior, portable paths and old preflight boundaries. Claudine library passed 11 loader, shape and schema parity cases and 17 graph cases. Darkmatter passed 38 passive composition, reference and editor cases and 13 combined report cases. These runs recorded zero tests at or above five seconds. Simplified Schema passed 16 focused key-declaration cases with features enabled and 2,221 cases in the default-free suite.

| User-visible behavior | Strongest relevant verification | Evidence and gaps |
| --- | --- | --- |
| Sequence walk, preview contents, exit codes, output streams and flags | Level 1, shipped CLI with local fixtures | Healthy and failing routes, help and flag forwarding, following previews, and JSON parsing. R1-F2, R1-F3 and R1-F7 identify paths the tests do not cover or where they assert incomplete behavior. |
| No commands, prompts, lifecycle effects or writes | Level 1, marker stubs and CLI fixtures | Existing prompt, shell and lifecycle controls pass. The source-expression marker and missing approval-policy cases are in R1-F1 and R1-F5. |
| Remote read permission and cache behavior | Level 1, loopback responder through CLI and library | Allow/deny request counts and existing read-cache controls. R1-F5 shows the missing ask behavior and scope of the affected readers. |
| Provider/model grading and real lifecycle cause | Level 1, injected terminal observations, stub providers and lifecycle persistence assertions | Focused real-boundary baselines pass, including a displayed row differing from the selected error. R1-F4 shows the disagreement on runtime-written hints. R1-F13 covers the missing passive file-choice result. |
| Reports, schema validation and authored positions | Level 1, CLI, fixture snapshots, and editor protocol tests across supported encodings | Plain and JSON report behavior is observable through these routes. R1-F6, R1-F7 and R1-F10–R1-F12 document the remaining compatibility, projection and verification gaps. |
| Real terminal rendering and input | Existing evidence only | This feature uses existing terminal components and introduces no key event behavior or specific real-terminal color or width acceptance criterion. The review did not open a terminal window or run keyboard injection. |
| Test latency | Level 1, focused Linux runs and the implementation's recorded native Windows run | Native Windows observations remain actionable under R1-F9. Lack of additional cross-platform evidence does not drive readiness. |

The local probes used disposable homes and directories, a loopback HTTP server, and local provider and shell stubs. No real provider was launched. The review is against the current worktree, including the existing implementation changes.

## Repair

> **started at:** 2026-11-26T21:18:26+00:00

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Sequence source expressions can start a shell during a dry run | pending |  |
| R1-F1.1 | Root `sequence: "{{ … }}"` ternary over an effectful call: no shell launch, list unchecked | pending |  |
| R1-F1.2 | File-reference `::template(…)` operator effectful predicate: no shell launch, dependent unchecked | pending |  |
| R1-F1.3 | Root formal `template:` field assigned effectful call: no shell launch, field and dependents unchecked | pending |  |
| R1-F1.4 | Formal template on nested inline sequence: same as root formal template | pending |  |
| R1-F1.5 | Prompt body interpolation: stays clean (no launch, info deferral) | pending |  |
| R1-F1.6 | Pure controls on all five paths: source resolves, no launch | pending |  |
| R1-F1.7 | Command-backed source and explicit shell/task dispatch: markers stay absent | pending |  |
| R1-F2 | A bad row in a root sequence source hides valid siblings | pending |  |
| R1-F2.1 | Root formal YAML reference, invalid middle row: both siblings checked | pending |  |
| R1-F2.2 | Root bare YAML data reference, middle row with `shell` and `prompt`: both siblings checked | pending |  |
| R1-F2.3 | Root expression returning authored array, invalid middle row: both siblings checked | pending |  |
| R1-F2.4 | Inline Markdown and directly invoked formal YAML: stays clean | pending |  |
| R1-F2.5 | Nested inline and referenced sequences: stays clean | pending |  |
| R1-F2.6 | Later graph resolution on failing routes: all three rows reported | pending |  |
| R1-F2.7 | Whole-file parse failure: no sibling recovery inside file; enclosing tasks continue | pending |  |
| R1-F3 | Required inputs suppress independent checks and previews | pending |  |
| R1-F3.1 | Direct compose: missing-input finding, independent inclusion finding, eligible text preserved | pending |  |
| R1-F3.2 | Inline compose: eager required input reported under mode policy, placeholder marked; completion-only rule preserved | pending |  |
| R1-F3.3 | Sequence composed from own body/`prompt:`: missing input marked, available content evaluated | pending |  |
| R1-F3.4 | Sequence prompt task lacking its own input: input finding, reference check, available preview | pending |  |
| R1-F3.5 | Prompt depending on missing sequence-owner input: dependent fields deferred, available content kept | pending |  |
| R1-F3.6 | Provided input controls: stay clean | pending |  |
| R1-F4 | Provider prediction treats runtime-written values as settled | pending |  |
| R1-F4.1 | Prompt frontmatter agent hint overwritten by earlier `success` set: provider and model unchecked | pending |  |
| R1-F4.2 | Task `with.agent` through chained document inputs: verdict deferred until launch | pending |  |
| R1-F4.3 | Provider reviews and model checks consume launch-time outcome | pending |  |
| R1-F4.4 | Earlier task writes unrelated key: static refusal stays (clean) | pending |  |
| R1-F5 | Remote reads cannot defer on network approval | pending |  |
| R1-F5.1 | Darkmatter configuration accepts `permissions.network: ask` | pending |  |
| R1-F5.2 | Claudine compose/inline-compose/sequence `load_data`: ask → info deferral, zero requests | pending |  |
| R1-F5.3 | Transclusion `::file`/`::code` remote: allow/ask/deny common decision | pending |  |
| R1-F5.4 | Dynamic schema slots: allow fetch, typed deny refusal, ask → only the deferral | pending |  |
| R1-F5.5 | Sequence source expression: local-only contract preserved and covered explicitly | pending |  |
| R1-F6 | Legacy reference JSON baselines were changed | pending |  |
| R1-F6.1 | `md validate refs --output json`: legacy bytes | pending |  |
| R1-F6.2 | `md validate refs --show-all --output json`: legacy bytes | pending |  |
| R1-F6.3 | `md graph --validate --output json`: legacy bytes | pending |  |
| R1-F6.4 | Plain/Markdown/clean/empty reference baselines: stay clean | pending |  |
| R1-F6.5 | New compose and Claudine findings keep evidence; trails serialized on new contracts | pending |  |
| R1-F7 | Schema key findings lose task occurrences and duplicate parser findings | pending |  |
| R1-F7.1 | Standalone `kind: task` file `whem`: one located row | pending |  |
| R1-F7.2 | Inline group leg in formal YAML: one typo verdict under the leg | pending |  |
| R1-F7.3 | Referenced group file leg: one typo verdict under the leg | pending |  |
| R1-F7.4 | Referenced formal YAML sequence used twice, `promt`: one advisory per occurrence with trail | pending |  |
| R1-F7.5 | Referenced formal Markdown sequence: same advice as YAML | pending |  |
| R1-F7.6 | Root open step and legal `when` controls: advisory under authored step, no typo on legal keys | pending |  |
| R1-F7.7 | Selected catalog group: only selected entry's verdicts | pending |  |
| R1-F7.8 | Unchecked rows for repeated task references carry the task occurrence | pending |  |
| R1-F7.9 | Missing-document and empty-agent overlap controls: stay clean | pending |  |
| R1-F8 | Live inline sequence steps can corrupt a YAML source | pending |  |
| R1-F8.1 | Direct YAML body task edited once: source still parses as YAML | pending |  |
| R1-F8.2 | Same source two tasks: task two runs | pending |  |
| R1-F8.3 | Real inline Markdown document: stays clean | pending |  |
| R1-F8.4 | YAML sequence with separate Markdown prompts: stays clean | pending |  |
| R1-F8.5 | Dry-run YAML inline preview: stays clean | pending |  |
| R1-F9 | Native Windows Level 1 timings exceed the review limit | pending |  |
| R1-F9.1 | `shipped_fleet_graphs_resolve_per_row_prompt_lifecycles`: split per fleet plus roster guard | pending |  |
| R1-F9.2 | `a_dry_run_writes_nothing_anywhere`: split routes or cleared | pending |  |
| R1-F9.3 | `a_missing_prompt_file_is_named_by_its_noun_and_cause`: measured on native Windows | pending |  |
| R1-F9.4 | `a_task_file_yaml_error_names_the_file_and_the_parse_stage`: measured on native Windows | pending |  |
| R1-F9.5 | Earlier split tests: keep structure | pending |  |
| R1-F10 | Sequence load findings drop structured source locations | pending |  |
| R1-F10.1 | Referenced YAML and Markdown loaders: structured line/column | pending |  |
| R1-F10.2 | JSON/JSON5 and JSONL/NDJSON loaders: structured coordinate | pending |  |
| R1-F10.3 | Executable reference validation: authored value position | pending |  |
| R1-F10.4 | Display wrappers around typed errors: class and location preserved | pending |  |
| R1-F10.5 | Confirmed no-match with known position: stays clean | pending |  |
| R1-F10.6 | No-coordinate YAML error and valid controls: file-only where unproven | pending |  |
| R1-F10.7 | Tolerated read/parse warning keeps typed class and directive position | pending |  |
| R1-F11 | The report fixture guard does not cover the scoped diagnostic catalog | pending |  |
| R1-F11.1 | Claudine catalog versus `dry_run_check::codes` guard derives from declarations | pending |  |
| R1-F11.2 | Darkmatter changed finding catalog tied to fixtures and both outputs | pending |  |
| R1-F11.3 | Claudine sequence error block snapshots linked to inventory | pending |  |
| R1-F11.4 | DMLS parity fixture routed through the shipping Claudine check | pending |  |
| R1-F12 | Darkmatter drops the unchecked body verdict after a failed preparation | pending |  |
| R1-F12.1 | Ordinary `md compose` reference stop: reference finding plus unchecked body | pending |  |
| R1-F12.2 | Lifecycle reference stop after `initialize`: same | pending |  |
| R1-F12.3 | Independent call/schema verdicts: stay clean | pending |  |
| R1-F12.4 | Proven and deferred conditions: stay clean | pending |  |
| R1-F12.5 | Reference-only and stdin routes: stay clean | pending |  |
| R1-F13 | Available file choices are reported as hard failures | pending |  |
| R1-F13.1 | Root bare-name file selection on compose/inline-compose/sequence: pending choice | pending |  |
| R1-F13.2 | Supplied eager partial file input on all three commands: pending choice | pending |  |
| R1-F13.3 | Sequence task input partial name: owning route's rules, deferred where eligible | pending |  |
| R1-F13.4 | Exact root and input references: stay clean | pending |  |
| R1-F13.5 | Anchored misses and no candidate: stay hard failures | pending |  |
