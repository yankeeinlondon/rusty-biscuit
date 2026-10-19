---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): Prompt composition retains an earlier launch identity"
    - "R2-F2 (high): Explicit selection flags do not override circular hints"
    - "R2-F3 (high): Static errors in selection inputs are postponed until execution"
    - "R2-F4 (medium): Empty provider hints bypass the canonical fallback rule"
observations:
    - Removed task models remain eligible as planned fallbacks
    - Retry composition also retains its earlier provider environment
    - Looping prompt documents cannot resolve an earlier-output provider hint
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-18T06:49:14-07:00
spec: 2026-10-26-task-inputs-and-providers/spec.md
implemented: true
description: A **feature** review of `2026-10-26-task-inputs-and-providers/spec.md`
feature: 2026-10-26-task-inputs-and-providers/review-2.md
previous: 2026-10-26-task-inputs-and-providers/review-1.md
next: 2026-10-26-task-inputs-and-providers/review-3.md
log: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/log.md
implemented_by: claude/opus
---

# Review 2: task inputs and providers

The feature is **not production ready**. Three high findings affect launch
identity, explicit overrides, and failure before execution. One medium finding
affects optional provider hints. None needs a new design decision.

The contract is the specification plus the author's three decisions under
"Human rulings" in [review 1](review-1.md). The accepted schema limits are
not reopened. This review changes no implementation code.

## Previous Finding Verification

Review 1 contains no code findings. Its repair ledger nevertheless assigns
identifiers to the three resolved decision items. The table verifies every
child prerequisite in that ledger; it does not turn those accepted decisions
into new findings. There are no earlier open findings to carry.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-HITL-1: narrower editor checks | 1.1 event/loop types at each level; 1.2 executable fields and shorthand; 1.3 documented limits; 1.4 recursive-schema follow-up | verified fixed | Read all three task/group levels in [sequence-types.yaml](../../schemas/sequence-types.yaml): cancellation is typed at levels 1–2, loops at level 1, remaining events have outer-shape checks; executable fields and scalar shorthand match the ruling. [Schemas](../../docs/topics/schemas.md#sequence-containers) documents the limits. The active recursive-schema specification explicitly requires every container event at every depth. The seven `sequence_schema` tests passed, including valid/invalid fixtures at each typed level and a deeper valid fixture. | None; the temporary limit remains accepted. |
| R1-HITL-2: one declaration per key | 2.1 split root declarations; 2.2 delayed completion documented; 2.3 follow-up proposal | verified fixed | [sequence.yaml](../../schemas/sequence.yaml) owns `sequence`, `template`, and `fail_fast`; [claudine.yaml](../../schemas/claudine.yaml) owns root events and `loop`. Read the "In the editor" limitation and the existing `cross-trigger-completion` proposal. | None in this cycle; the separate completion proposal remains unscheduled. |
| R1-HITL-3: schema/parser differences | 3.1 numeric sequence; 3.2 null root `fail_fast`; 3.3 group diagnostic location; 3.4 boolean sequence | verified fixed | Read each documented case and executed `sequence_schema::sequence_trigger_activation_follows_the_value_shape` and `removed_and_misspelled_keys_are_reported_by_name`. Numbers/booleans and null `fail_fast` pass the schema and fail the public sequence parser. Group errors locate the group and name the offending key. | None. |

The repair ledger's nonblocking investigation row is not a finding. Its model
concern was investigated below, rather than treated as a claimed repair.

## Unblocked Findings

### R2-F1 (high): Prompt composition retains an earlier launch identity

**Authority: spec, section 5:** "Refresh selection at the existing preparation
boundary so composition-time `env.AGENT`/`env.MODEL` and the child environment
agree." Acceptance criterion 8 makes provider selection from earlier outputs
and concurrent-group state part of the required behavior.

Defect class: the launch identity changes after the prompt has already been
composed under the prediction or enclosing provider's environment.

In the Claudine CLI, [the prompt task runner](../../cli/src/commands/wrap/sequence/task_run.rs:903)
composes first and calls `launch_target` afterward. Updating the child
environment at that point does not update the rendered prompt. A freshly read
document that changes a statically predicted provider has a related result:
the shared launch rebuild chooses the new executable, while the task's older
`AGENT` overlay survives into its child environment. This can send instructions
intended for one provider to another.

Reproduction: use the existing `sequence_task_providers` fixture with Claude
and Goose recording stubs. Give `seq.md` `agent: claude`, run a shell step
`echo goose`, then a prompt task with `with.agent: '{{ seq.outputs[0] }}'`.
The prompt body is `BODY_AGENT={{ env.AGENT }}`. The Goose stub receives
`BODY_AGENT=claude`, although its child environment says `AGENT=goose`.

The sweep follows selection through prompt composition, child environment,
and the completion line. All prompt tasks, including group legs and external
task files, reach this runner; ordinary sequence body steps use the separate
step-composition path. The following instances include the clean controls.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Prompt task, dynamic `with.agent` | Earlier shell output selects Goose; root fallback is Claude | Goose launches; prompt says Claude; child says Goose; completion correctly says Goose | Prompt and child both say Goose |
| R2-F1.2 | Prompt task, document `agent` | Move the same `seq.outputs[0]` expression into prompt frontmatter | Same mismatch as 1.1 | Both say Goose |
| R2-F1.3 | Prompt task, dynamic `with.model` | Earlier shell emits `new-model`; body reads `env.MODEL` | Claude receives `--model new-model` and `MODEL=new-model`, but body renders an empty model | Body renders `new-model` |
| R2-F1.4 | Concurrent group leg | `group.choice: goose`, leg `with.agent: '{{ group.choice }}'`, root fallback Claude | Goose launches with a prompt saying Claude | Both say Goose |
| R2-F1.5 | Fresh task document | Earlier shell copies a replacement prompt changing `agent: claude` to `agent: goose` | Goose launches; prompt and child `AGENT` still say Claude; completion says Goose | Prompt, child, and completion agree on Goose |
| R2-F1.6 | Static own-provider control | Prompt declares `agent: goose`; body reads `env.AGENT` | Prompt and child say Goose | Same; clean |
| R2-F1.7 | External task control | `kind: task` file supplies `with.agent: goose` | Prompt and child say Goose | Same; clean |
| R2-F1.8 | Looping prompt control | Prompt declares `agent: goose` and a one-pass loop | Prompt and child say Goose | Same; clean |
| R2-F1.9 | Proxy target control | Claude prompt proxies to a separate Goose prompt; each body reads `env.AGENT` | Each prompt and child names its actual provider | Same; clean |
| R2-F1.10 | Existing sequence body-step path | Body steps share the enclosing document's selected target; no task-local change of hint source | The task-specific compose-then-select branch is not used | Preserve this path; its selection/environment setup must remain consistent |
| R2-F1.11 | Inline nested sequence | Put the shell-output producer and dynamic prompt task inside the nested sequence | Goose launches with a prompt saying Claude | Prompt and child both say Goose |
| R2-F1.12 | Dynamic task with no fallback | Remove root `agent` from the first reproduction | Goose launches; child says Goose; prompt agent is empty | Prompt and child both say Goose |
| R2-F1.13 | External task, dynamic hint | Move the first reproduction's prompt task and `with` into a `kind: task` file | Goose launches with a prompt saying Claude | Prompt and child both say Goose |
| R2-F1.14 | File-reference nested sequence | Move the producer and dynamic prompt task into a separate sequence document | Goose launches with a prompt saying Claude | Prompt and child both say Goose |

Resolve the effective identity before rendering identity-dependent content at
the existing preparation boundary, and ensure a later rebuild replaces stale
task-owned identity overlays. Extend the current stub tests to inspect both
the delivered body and child environment. A second independent picker or a
new protection layer is unnecessary.

### R2-F2 (high): Explicit selection flags do not override circular hints

**Authority: spec, section 5:** "A statically known higher-precedence override
makes a lower dynamic hint irrelevant." The precedence table puts explicit
provider flags first; model resolution puts explicit `--model` first.

Defect class: prediction rejects a lower-priority hint before applying the
selection source that makes the hint irrelevant.

In the Claudine CLI, [selection](../../cli/src/commands/wrap/sequence/providers.rs:302)
calls `own_hints` before checking `explicit_provider`.
[That reader](../../cli/src/commands/wrap/sequence/providers.rs:374) rejects
either hint's circular state immediately. Consequently, a prompt containing
`agent: '{{ env.AGENT }}'` fails even with `--claude`; the diagnostic recommends
passing a provider flag that the caller already supplied. A document's
`model: '{{ env.MODEL }}'` similarly defeats `--model explicit-model`.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Document `agent`, explicit provider flag | `agent: '{{ env.AGENT }}'`, `--claude` | Located circular-selection error; nothing launches | Explicit flag selects Claude |
| R2-F2.2 | Task `with.agent`, explicit provider flag | Same expression supplied through `with`, `--claude` | Same error at `tasks[0].with.agent` | Explicit flag selects Claude |
| R2-F2.3 | Document `model`, explicit model flag | `model: '{{ env.MODEL }}'`, `--model explicit-model`, static Claude agent | Circular-selection error | Explicit model wins |
| R2-F2.4 | Document `agent`, caller setter control | Same circular agent, caller setter `agent=claude` | Claude launches | Same; clean |
| R2-F2.5 | Document `model`, caller setter control | Same circular model, caller setter `model=explicit-model` | Claude launches | Same; clean |
| R2-F2.6 | Noncircular dynamic agent control | `agent: '{{ seq.outputs[0] }}'` with `--claude` | Claude launches | Same; clean |
| R2-F2.7 | No higher override | Circular hint without a deciding override | Existing `a_hint_reading_its_own_identity_fails_as_circular` checks the source-located rejection | Retain rejection; clean |
| R2-F2.8 | Task `with.model`, explicit model flag | `with.model: '{{ env.MODEL }}'`, `--model explicit-model` | Circular-selection error at the task input | Explicit model wins |
| R2-F2.9 | Captured generic model environment | Document `model: '{{ env.MODEL }}'`, captured `MODEL=explicit-model` | Circular-selection error before the model resolver uses the environment | Captured higher-precedence model wins |

Both provider-flag spellings feed the same `explicit_provider` input. Both
selection keys pass through `ProviderPrediction::failure`; provider-specific
model variables and generic `MODEL` are resolved only after this reader, so
both higher-precedence environment sources are subject to the same refusal.
Nested and external tasks also share it. Apply precedence separately for provider and model before
rejecting an irrelevant lower hint, preserving validation of whichever hint
actually decides the launch.

### R2-F3 (high): Static errors in selection inputs are postponed until execution

**Authority: spec, section 5:** "ordinary expression errors in otherwise
static inputs remain typed failures." Predictable invalid task selection
must fail preflight "before any task launches."

Defect class: a failed static evaluation is converted into a launch-time
dependency, allowing earlier work to run before reporting an already known
selection error.

The Claudine library's [reference reader](../../lib/src/composition/sequence/preflight/mod.rs:1922)
puts every unsuccessful `with` evaluation into `deferred`, losing its cause.
[Provider prediction](../../lib/src/composition/sequence/preflight/providers.rs:277)
then treats that list as evidence of a late value. In contrast, the document
hint branch retains a typed `Failed` state. The CLI accepts the deferred
selection, launches the earlier task, and only later reports the invalid
`with` expression. A misspelled function in a provider expression is a normal
authoring error, not a value that depends on earlier outputs.

Reproduction: two prompt tasks under a root `agent: claude`, with a recording
Claude stub. On task 2 add `with: {agent: '{{ no_such_function() }}'}`.
Task 1 launches before task 2 fails with `Unknown function: no_such_function`.
Moving that expression into task 2's prompt `agent` stops preflight with no
recorded launch.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F3.1 | `with.agent` evaluation and prediction | Unknown function in a static expression | Marked for launch-time resolution; task 1 launches; task 2 fails | Located preflight failure; no task launches |
| R2-F3.2 | `with.model` evaluation and prediction | Same expression in `model` | Same delayed failure and earlier launch | Same early failure |
| R2-F3.3 | Document `agent` control | Same invalid expression in prompt frontmatter | Typed preflight failure; zero launches | Same; clean |
| R2-F3.4 | Document `model` control | Same invalid expression in prompt frontmatter | Typed preflight failure; zero launches | Same; clean |
| R2-F3.5 | Genuine late values | Existing earlier-output, runtime-written-value, and concurrent-snapshot fixtures | Tasks launch successfully at their own turns | Remain deferred; clean controls |
| R2-F3.6 | CLI gate consuming these states | `DecidedAtLaunch` versus `Failed` from the same graph | Deferred state bypasses the gate; failed state aborts selection | Keep the distinction, but supply the correct state |

Every prompt reference uses the same reader, regardless of task nesting or
external source. Preserve the early evaluation result and its cause for
selection-relevant entries, distinguishing actual late dependencies from
static failures. Do not turn unrelated task inputs into new preflight
requirements or eagerly execute dynamic expressions.

### R2-F4 (medium): Empty provider hints bypass the canonical fallback rule

**Authority: spec, section 5:** "Use the canonical provider resolver for scalar
and list-valued hints." Review 1 explicitly accepts the existing resolver's
rule that `agent: null` and `agent: []` mean no own hint. That decision is
retained; the implementation does not consistently apply it.

Defect class: raw field presence is used as the provider category after the
canonical parser has normalized the value to no hint.

The Claudine library's `ProviderPrediction::class` labels every static value
as predicted. The CLI then [keeps that category](../../cli/src/commands/wrap/sequence/providers.rs:393)
even when the parsed hint is empty, and rejects the task without consulting
its enclosing sequence's valid provider. This affects optional values in
templates; it is medium because explicit null and empty lists are less common
than omitting the field.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F4.1 | Prompt frontmatter hint | `agent: null`, enclosing `agent: claude` | Task-selection error, zero launches | No own hint; inherit Claude |
| R2-F4.2 | Prompt frontmatter hint | `agent: []`, same enclosing provider | Same error | Inherit Claude |
| R2-F4.3 | Task `with.agent` | `null`, same enclosing provider | Error at `tasks[0].with.agent` | Inherit Claude |
| R2-F4.4 | Task `with.agent` | `[]`, same enclosing provider | Same error | Inherit Claude |
| R2-F4.5 | Caller-setter projection and gate | Caller supplies `agent: null` or `agent: []` | Static predicted category reaches an own-hint error | No own hint; use the ordinary missing-provider gate when the caller also clears the root hint |
| R2-F4.6 | Absent field control | Omit task agent, enclosing Claude | Claude launches | Same; clean |
| R2-F4.7 | Scalar/list nonempty controls | Static Goose; `[claude, gemini]` with only Claude runnable from that list | Canonical provider launches | Same; clean |
| R2-F4.8 | Body/enclosing hints | `names_agent` checks parsed hint presence before creating a body/enclosing site hint | Empty parsed hints are not classified as own providers | Retain this behavior; clean by code invariant |
| R2-F4.9 | Review projection | Task `with: null/[]` agent reaches selection before drafts are constructed | Hard failure prevents the missing-provider row | Missing hints inherit a provider or produce an editable row |

Normalize the category from the parsed hint before choosing own-provider,
fallback, or review behavior. Keep null/empty-list semantics as already
accepted; do not add a new rejection rule.

## Blocked Findings

None.

## Recurrence

None. Review 1 contained no code findings. R2-F4 contradicts an implementation
claim recorded there, but is a newly verified implementation defect, not a
reopened decision or a recurrence of an earlier finding.

## Input Robustness Matrix

Default outcome for invalid task-input shapes is a source-located error before
the affected target launches. The provider-specific early-failure contract is
stronger: a statically invalid selection must stop the entire graph before
any task launches. R2-F3 identifies where that stronger rule is lost.

The shared `with_mapping_shape` validator is used by task, proxy, retry, and
prep readers. Absent `with` means no overlay; `{}` is an explicit empty overlay;
null, scalar, and array whole fields fail. An expression supplying the entire
mapping gets `LifecycleWithWholeMapping` before the generic shape error.
Dynamic keys fail. Overlay values deliberately retain arbitrary scalar,
collection, object, and null data, so numeric elements inside a value are not
malformed mapping elements. The public sequence and lifecycle parser tests
exercise those outcomes and source locations; the CLI error tests prove
wiring. Provenance tests cover eager/lazy missing files, remote values,
ambiguous schema arms, forwarded anchored values, changing target schemas,
caller precedence, and atomic evaluation failure.

The shared selection-hint validator accepts a string or string list. Its
exceptions are absent/null/empty list, which mean no hint; an empty string is
an invalid provider name, not absence. Whole-field numbers/objects and mixed
or wholly non-string lists are errors. Existing
`invalid_hint_shapes_fail_as_the_task` checks the latter cases through a
preflight graph. The absent, null, empty-list, empty-string, and nonempty
controls were also exercised through the CLI during this review. R2-F4
enumerates the downstream decisions that mishandle the accepted empty cases.
`agent` and `model` share the hint parser; only agent emptiness chooses a
provider category.

No new Markdown/YAML/JSON document decoder is introduced here: existing
source loaders feed the changed field validators. This review does not claim
new duplicate-key or trailing-content guarantees for those unchanged loaders.
Schema inputs were checked separately with the committed three-level fixture
corpus and trigger-shape tests, respecting the author's accepted numeric,
boolean, and null exceptions. Searches for defaulted fields, discarded parse
errors, `filter_map`, `.ok()`, and `unwrap_or_default()` found the discarded
`with` evaluation cause described in R2-F3. The enclosing-sequence `.ok()`
keeps the documented "enclosing hint resolves" rule; it is not a new strict
configuration contract.

## Requirement Verification

All ordinary execution requirements here need Level 1 verification. The
review-screen requirement explicitly needs Level 2. No new requirement
depends on a physical modifier-key encoder or needs Level 3.

| Requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Task `with`, freed `params`, option errors, generated bindings | L1 public parser/task tests and CLI errors | Implemented; maintained task-input uses were checked with a text search |
| File provenance and atomic evaluation | L1 shipped CLI with private fixtures and stubs | Fifteen provenance tests passed |
| Per-task provider selection, precedence, headless execution | L1 public preflight plus shipped CLI/stubs | Existing eleven provider tests passed; R2-F1–F4 expose missing assertions/cases |
| Read-only own-provider rows and editable choices that launch | L2 detached tmux captures and stub launch records | Five tests passed, including the new read-only-row test; no window focus required |
| Late selection and actual attempt reporting | L1 shipped CLI/stubs | Earlier outputs, runtime writes, group snapshot, retries, and no-launch statuses have existing tests; identity agreement needs R2-F1 coverage |
| Sequence trigger, three typed levels, deeper boundary, embedded imports | L1 schema/public parser fixture suite and embedded-bundle tests | Seven sequence-schema tests passed; documented limits agree with the accepted decisions |
| Editor discovery and diagnostics | L1 DMLS document diagnostics using the shipped bundle | Focused DMLS verification recorded below |

New tests are declared by their consolidated `main.rs` modules; the CLI L1
target is declared in `Cargo.toml`. The review-screen module is declared in the
`level2` target, enabled by `terminal-tests`, selected by a live `test-l2`
recipe, and actually executed here. The temporary review probes used the
existing L1 provider-test module and `CliProcessFixture`; they were removed
after execution. They introduced no persistent tests or implementation edits.

Verification performed on this working tree:

- `just test-cli sequence_task_providers::`: 11 passed.
- `just test-cli with_file_provenance::`: 15 passed.
- `just test-library sequence_schema::`: 7 passed, including the previous repair's new cases.
- `just test-l2 sequence_review_screen`: 5 passed; generator scope contained no matching tests.
- `just test-cli sequence_errors_cli::`: 52 passed.
- Darkmatter `just test claudine_schema_discovery::`: 11 DMLS tests passed.
- `just test-library with_shape_errors`: 2 public-parser tests passed.
- `just test-cli invalid_hint_shapes_fail_as_the_task`: 1 passed.
- `just test-library embedded_sequence_types_exports_resolve_without_disk_schemas`: 1 passed.
- `just test-library embedded_payloads_apply_without_disk_schemas`: 1 passed.

Full L1 suites and lint results in the implementation/repair logs are prior
evidence, not independent reruns by this reviewer. Cross-OS proof is left to
the repository's normal CI and does not affect this readiness verdict.

## Observations

**Removed task models remain eligible as planned fallbacks.** The concern
deferred by review 1 is reproducible: a prompt predicts `old-model`; an earlier
shell step replaces that prompt with one having no model, or the failing
provider replaces it before retry; Claude still receives `old-model`. Replacing
it with an explicit `new-model` works. The shared resolver expressly allows a
compatible planned fallback, so the spec does not clearly require removal of
the field to erase that fallback. Document the retained-fallback behavior or
schedule a separate decision about clearing it; this review does not impose
the stronger freshness promise found in some prose.

**Retry composition also retains its earlier provider environment.** In a
Claude task, a success-stack retry with `with.agent: goose` launches Goose and
correctly reports `Claude → Goose`, but the retried body still renders
`env.AGENT` as Claude. The retry materialization/rebuild path already existed
before this feature; the feature changes its overlay provenance and reporting,
not that ordering. This is a separate maintenance observation, not another
blocking repair instance. The new task-selection ordering defects in R2-F1
remain independently reproducible on the first attempt.

**Looping prompt documents cannot resolve an earlier-output provider hint.**
Adding a one-pass `loop` to the prompt whose document-level `agent` reads
`seq.outputs[0]` changes a successful Goose launch into an agent-resolution
failure. The looping-document branch re-enters direct composition instead of
the new task-local launch selector. Both that branch and its direct-selection
ordering predate this feature, so this is an observation under the requested
scope rule. A static own-provider looping prompt is a passing control. The
new per-task prediction should not be read as proof that this older path
supports every late document hint.
