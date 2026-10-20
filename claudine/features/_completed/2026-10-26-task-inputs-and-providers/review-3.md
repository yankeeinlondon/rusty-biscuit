---
$schema: feature-review.yaml
ready: false
findings:
    - "R3-F1 (high): Shell-dependent selection still renders a stale launch identity"
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-18T08:11:13-07:00
spec: 2026-10-26-task-inputs-and-providers/spec.md
implemented: true
description: A **feature** review of `2026-10-26-task-inputs-and-providers/spec.md`
feature: 2026-10-26-task-inputs-and-providers/review-3.md
previous: 2026-10-26-task-inputs-and-providers/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/log.md
implemented_by: claude/opus
next: 2026-10-26-task-inputs-and-providers/review-4.md
---

# Review 3: task inputs and providers

The feature is **not production ready**. The four previous findings' enumerated instances have been addressed, with an expected-result correction for two empty-list cases described below. One newly reproduced sibling of the launch identity finding remains: shell-dependent selection can send a provider a prompt rendered for a different provider or model.

The contract is the specification and the three author decisions under "Human rulings" in [review 1](review-1.md). Those schema decisions remain accepted. No new human decision is needed. This review changes no implementation code; temporary reproduction tests were removed after use.

## Previous Finding Verification

Each range below includes every child in the repair's Finding Ledger in [the repair log](log.md), including its newly discovered children. Review 1 had no code findings and no still-open decision items; its three decisions were verified in review 2 and are not reopened. Previous observations are not carried as findings.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: Prompt composition retains an earlier launch identity | 1.1–1.14: dynamic task/document agent, dynamic model, group leg, rewritten document, static/external/loop/proxy/body controls, inline/file nested tasks, no fallback, external dynamic task | verified fixed | All fourteen corresponding identity tests passed in the 54-test provider CLI run. They assert delivered body and child environment; relevant root cases also assert completion. Read pre-render selection in the CLI's [jit.rs](../../cli/src/commands/wrap/sequence/jit.rs:253), identity replacement, and fresh launch selection in [task_run.rs](../../cli/src/commands/wrap/sequence/task_run.rs:941). | None of the original fourteen instances. R3-F1 is a newly found shell-dependent sibling, not a failure of one of those repaired cases. |
| R2-F2: Explicit selection flags do not override circular hints | 2.1–2.9: document/with agent and model, provider flag/option, caller controls, dynamic control, no override, generic/provider-specific model environment; 2.10: provider-dependent environment with unknown provider | verified fixed | CLI precedence cells passed, including both provider flag spellings, caller setters, both model environments, and rejection without the deciding override. All 17 review-unit tests passed. The CLI's [select_one](../../cli/src/commands/wrap/sequence/providers.rs:335) handles circular provider/model hints separately. For 2.10 only provider-independent higher sources are statically known; the existing contract permits rejection. | None. |
| R2-F3: Static errors in selection inputs are postponed until execution | 3.1–3.6: with/document agent and model failures, genuine late controls, consuming CLI gate; 3.7: runtime-created key | verified fixed | Both invalid-expression CLI tests passed for both keys and assert zero launches. The 239-test preflight run includes failed-entry propagation, indirect failed-entry reads, unrelated-entry control, late values, and runtime-created keys. The library's [task input reader](../../lib/src/composition/sequence/preflight/mod.rs:1933) retains evaluation causes; [prediction](../../lib/src/composition/sequence/preflight/providers.rs:308) distinguishes failure from a late value. The CLI runtime-created-key test also passed. | None. |
| R2-F4: Empty provider hints bypass the canonical fallback rule | 4.1–4.9: document/with null and empty list, caller setters, absent/nonempty controls, body/enclosing consumers, review projection; 4.10: launch-schema test consumer | verified fixed | Null document/with values now inherit; caller null/empty-list values reach the missing-provider gate; absent, scalar, and installed-list controls passed. Authored empty lists now fail before any launch, with the correct source. Read the normalized [provider category](../../lib/src/composition/sequence/preflight/providers.rs:157), review unit cases, and changed launch-schema fixture. See the expected-result correction below for 4.2 and 4.4. | None under the corrected expectation; the spec does not require making a schema-invalid authored list launch successfully. |

**Correction to the previous review's empty-list expectation.** R2-F4.2 and R2-F4.4 asked an authored agent empty list to inherit and launch. The canonical hint parser treats that value as no hint, but the pre-existing launch schema independently requires a nonempty authored agent list. Reusing the canonical resolver does not promise exemption from launch validation. Those two original expected results were too strong: the repair correctly rejects the invalid list early instead of allowing earlier work to run before launch validation rejects it. Null inheritance and caller-setter missing-provider behavior remain verified. This correction does not change any of the author's three schema decisions or establish new empty-list syntax.

## Unblocked Findings

### R3-F1 (high): Shell-dependent selection still renders a stale launch identity

**Authority: spec, section 5:** "Refresh selection at the existing preparation boundary so composition-time env.AGENT/env.MODEL and the child environment agree." The same section explicitly permits shell-dependent hints to wait until launch and forbids executing shell substitutions during prediction. The reproduction uses ordinary supported frontmatter command substitution, not an adversarial payload.

Defect class: selection-dependent prompt rendering proceeds with a predicted identity when the early selection read cannot resolve a hint, and the later launch updates only the child identity.

In the Claudine library, [read_selection_hints](../../lib/src/composition/prepare.rs:778), the frontmatter reader used to select an identity before rendering a task, returns no hints whenever either selection value contains a shell-substitution opener. In the Claudine CLI, [compose_step_selecting](../../cli/src/commands/wrap/sequence/jit.rs:253), which prepares the prompt, then keeps the planned environment. Full preparation executes the frontmatter command and renders the body, but [the task runner](../../cli/src/commands/wrap/sequence/task_run.rs:959) selects the resulting provider/model only afterward. Correcting the child's environment cannot correct instructions already rendered into the prompt.

Minimal reproduction, adapted from the existing sequence_task_providers recording-provider fixture:

~~~yaml
# seq.md frontmatter
agent: claude
sequence:
  - name: run
    prompt: task.md
~~~

~~~markdown
---
agent: '$(echo goose)'
---
BODY_AGENT={{ env.AGENT }}
~~~

Run "claudine sequence seq.md --yolo" with the fixture's Claude and Goose recording stubs. Goose launches with AGENT=goose, receives BODY_AGENT=claude, and completion correctly reports Goose. Replacing the agent command with "model: '$(echo fresh-model)'" launches Claude with "--model fresh-model" and MODEL=fresh-model, while the body contains an empty BODY_MODEL. Removing the root fallback makes BODY_AGENT empty instead of correct.

The sweep checked both selection keys, indirect dependencies, each prompt-task placement, and downstream child/report consumers. All executed probes used the shipped CLI through CliProcessFixture; each was its own nextest test, with recording stubs, isolated CWD/home, and silent audio. The original static identity fixture is the positive control. Explicit flags are additional controls for the same shell-bearing documents.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Direct prompt document agent | Agent command emits Goose; enclosing Claude | Goose receives BODY_AGENT=claude; child agent is Goose | Body and child both identify Goose |
| R3-F1.2 | Direct prompt document model | Model command emits fresh-model | Child/argv select fresh-model; body model is empty | Body also identifies fresh-model |
| R3-F1.3 | Indirect document agent | Choice field runs the command; agent reads choice | Goose receives a body identifying Claude | Both identify Goose |
| R3-F1.4 | Concurrent group leg | Same agent command in the leg's prompt | Goose receives a body identifying Claude | Both identify Goose |
| R3-F1.5 | External task reference | Same prompt through kind: task | Goose receives a body identifying Claude | Both identify Goose |
| R3-F1.6 | Inline nested sequence | Same prompt in nested task list | Goose receives a body identifying Claude | Both identify Goose |
| R3-F1.7 | File-reference nested sequence | Same prompt in referenced sequence | Goose receives a body identifying Claude | Both identify Goose |
| R3-F1.8 | Task without enclosing fallback | Same agent command, no root agent | Goose receives an empty body agent | Body identifies Goose |
| R3-F1.9 | Resolved task-input model | with.model contains the literal command-looking string | Child receives that literal model string; body model is empty because the reader also skips this value | Body matches the literal child model; do not execute or reinterpret resolved task data |
| R3-F1.10 | Resolved task-input agent control | with.agent contains the same literal command-looking string | Located unknown-provider failure; no provider launches | Same; clean. Resolved task data is not a frontmatter shell command |
| R3-F1.11 | Explicit provider control | Document agent command plus --claude | Body and child both identify Claude | Same; clean |
| R3-F1.12 | Explicit model control | Document model command plus --model explicit-model | Body and child both identify explicit-model | Same; clean |
| R3-F1.13 | Shared early-reader decision | Shell-bearing agent or model after frontmatter-only read | No hints returned: identity selector is bypassed; the other key also remains planned | Resolve effective identity before identity-dependent body rendering |
| R3-F1.14 | Child and completion consumers | Successful document-command probes above | Child environment/argv and completion identify the actual launch | Same; clean. Preserve these consumers while fixing the body |
| R3-F1.15 | Non-shell identity controls | All fourteen R2-F1 cases | Delivered body and child agree | Same; clean |

The task-input agent control is intentionally clean: section 2 says resolved strings must not be reinterpreted as templates. Its command-looking text must remain invalid provider data. The model case exposes the same reader's overly broad decision to skip both keys even when that text is deliberately literal.

The repair log calls the document-command mismatch an accepted limitation, but that is an implementation choice, not an author decision or spec exception. Avoid repairing it by rendering the whole document twice: that would duplicate body commands and effects. Resolve the already-approved frontmatter command result at the existing preparation boundary, then select the identity before rendering the body. Keep the boundary limited to selection inputs; no new escaping layer or synthetic payload matrix is needed. Verify that the frontmatter command and any body command each run once. This is a named phase-ordering case with a concrete boundary, not a request for unbounded robustness guarantees.

## Blocked Findings

None.

## Recurrence

R3-F1 repeats the defect class of **review-2.md, R2-F1: Prompt composition retains an earlier launch identity**. It is a sibling the earlier sweep missed, not a regression in one of the fourteen repaired instances. That sweep should also have checked shell-dependent document agent and model values, their indirect frontmatter dependencies, and the early reader's treatment of resolved task inputs. The repair added a frontmatter-only reader but explicitly left this branch using the old ordering. The instance table above now includes those sites, all affected container placements, and clean launch/report controls. The original findings R2-F2 and R2-F3 have no reproduced recurrence.

## Input Robustness Matrix

Default outcome: invalid task-input shapes produce a source-located error and launch no affected target; statically invalid selection inputs stop the graph before any task launches. The inspected code introduces no new serialized file loader: task/lifecycle mappings use existing document loaders, and the new selection reader composes an already parsed Markdown document. Duplicate-key and trailing-content policy is unchanged by this feature; this review does not assert new document-loader coverage.

The shared with_mapping_shape validator is reached by task, proxy, retry, and prep parsing. Exceptions to rejection are: absent with means no overlay; an empty mapping is the explicit empty overlay; arbitrary typed values inside the mapping, including null, arrays, and objects, are data. A single whole-mapping expression gets its dedicated error before the generic shape error. Non-mapping null, scalar, and list values fail, and dynamic mapping keys fail. The public sequence parser's task_with_shape_errors_are_the_with_family_at_the_task exercises these errors and locations; lifecycle parser cases cover the shared validator and directive wiring. The 15 passing public CLI provenance cases cover every overlay route, lazy/eager files, remote values, forwarding, changing target schemas, caller precedence, and atomic evaluation failure.

The canonical hint validator's exceptions are: absence and null mean no hint; an empty model list means no model; an empty caller agent list means no hint; an authored empty agent list is rejected by launch validation and now by preflight. Empty strings are values, not absence. Whole-field numbers/objects and mixed or entirely non-string lists fail. Read the canonical validator's complete scalar/list match and ran invalid_hint_shapes_fail_as_the_task inside the 17-test review-unit run, plus the empty/nonempty CLI controls. Provider and model states use the same prediction path; static expression errors and genuinely late values were verified for both keys.

Searched the changed readers for serde defaults, Option fields, filter_map, unwrap_or_default, and suppressed parse errors. The dependency scan's filter_map does not filter hint-list elements. The three suppressed errors in read_selection_hints retain full preparation as the error reporter, but its no-hints return for shell-bearing values also bypasses the identity update: R3-F1 follows that value through rendering, child selection, and completion. No separate invalid-shape finding resulted.

## Verification level and results

| Spec requirement | Required level | Evidence assessed |
| --- | --- | --- |
| Task with rename, precedence, ordinary seq.params data | L1 | Public sequence parser cases, provider CLI precedence cells, maintained-use text sweep |
| File provenance, forwarding, atomic overlays | L1 | Provenance CLI suite: 15 passed |
| Located task/directive shape errors | L1 | Public parser shape case and shared validator/wiring inspection |
| Independent providers, looping sequence, actual reporting | L1 | Provider CLI suite: 54 passed, including body/child identity assertions |
| Provider/model precedence, circular overrides, unavailable hints | L1 | Same 54 CLI cases; 17 review-unit cases; preflight tests |
| Early failure and provider-free graphs | L1 | Zero-launch invalid-selection cases and shell/side-effect-only CLI case |
| Read-only predicted rows and editable choices that launch | L2 | Review-screen suite: 5 passed in headless tmux, including own-provider read-only row and submitted provider choices |
| Late outputs/runtime/group selection and side-effect-free prediction | L1 | Preflight run: 239 passed; CLI late-value and group-snapshot cases passed. Shell-dependent identity gap is R3-F1 |
| Schema discovery, bounded levels, embedded imports | L1 | Sequence-schema library suite: 7 passed; schema bundle/import and CLI wiring inspection; accepted editor limits retained |
| Maintained params migration | L1/static inspection | Remaining searched occurrences are ordinary data, hook-action parameters, research protocol fields, or historical text |

Commands run in claudine: "just test-cli sequence_task_providers::", "just test-library preflight", "just test-library sequence_schema", "just test-cli with_file_provenance::", "just test-cli review::tests", "just test-library task_with_shape_errors", and "just test-l2 sequence_review_screen". The public parser shape test also passed on its own.

"just check-tier-coverage claudine" reported **0 stranded tests**. The changed library tests compile in the library or its declared L1 binary; the CLI provider and provenance modules are declared in its L1 binary, and review-screen tests are declared in the live L2 target. Recording-shell provider fixtures are Unix-gated; this run is macOS evidence, not native-Windows proof. No OS evidence requirement is used as a readiness blocker.

The temporary shell sweep ran twelve separate L1 cases. Two explicit-flag controls passed. Eight document-command cases reached successful launches and failed the body-identity assertion; the literal task-input model also failed that assertion. The literal task-input agent was correctly rejected and is classified as a clean control above. All temporary cases finished under two seconds each. Their tests were removed and the existing test file restored byte-for-byte. Full L1, lint, and Darkmatter suites were not rerun for this documentation-only review; earlier logged results were not substituted for the targeted executions reported here.

## Observations

None.
