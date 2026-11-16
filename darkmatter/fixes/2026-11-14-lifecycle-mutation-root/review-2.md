---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-15T20:09:51-08:00
spec: 2026-11-14-lifecycle-mutation-root/spec.md
implemented: false
description: "A **fix** review of `2026-11-14-lifecycle-mutation-root/spec.md`"
fix: 2026-11-14-lifecycle-mutation-root/review-2.md
previous: 2026-11-14-lifecycle-mutation-root/review-1.md
---

# Review 2

The fix is production ready. All five previous findings are verified fixed,
including their required instances and clean controls. This review found no
new defect requiring another repair cycle. No author decision is needed.

Lifecycle file actions now choose a target using the authoring document's
file-reference rules, then check it against the caller's captured write
boundary. Referenced documents retain their own relative-path base while
sharing that boundary. Failed repository discovery stops lifecycle execution,
and refused actions report the complete target and boundary paths.

## Previous Finding Verification

The required instances below are the child rows of the repair log's
`### Finding Ledger`, checked against source and executable tests. The previous
review contains no human rulings or blocked findings. No earlier review is
present in this directory.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: referenced groups and internal task adapters retain the caller's authoring path | R1-F1.1–R1-F1.11: group events, gate, catalog selection, foreign group, task primary/setup/teardown, referenced-sequence, member, inline-group, and inline-sequence controls | **verified fixed** | Claudine's task and container handoffs now carry the authoring path and matching resolution context together. Six library authoring-document tests and the CLI lifetime tests pass; detailed instance checks follow. | None; this completes the partial handoff identified in review 1. |
| R1-F2: Claudine runs file actions after launch repository discovery fails | R1-F2.1–R1-F2.9 and R1-F2.x: seven CLI engine sites, inline composition, Darkmatter control, duplicate follow-up record | **verified fixed** | All seven engine sites call Claudine's shared discovery-failure check. Public-library healthy/absent/corrupt cases and five CLI refusal cases pass. The follow-up record is reconciled. | None. |
| R1-F3: root sequence events and loop gates omit refusal diagnostics | R1-F3.1–R1-F3.15: five root events, four gate owners, positive/suppressed/evaluation controls, nested and foreign-container reports, cancellation | **verified fixed** | CLI refusal-report tests exercise the root events and every gate owner. Library gate tests verify warning, suppression, successful write, and evaluation-failure behavior. Reporting retains the existing exit policy. | None. |
| R1-F4: L1 tests exceed the timing limit under concurrent execution | R1-F4.1–R1-F4.13: mutation/lifetime tests, staging test, two schema cases, three graph cases, language-server snapshot test, and previously cleared controls | **verified fixed** | Stored subsequent full-area runs record no test at or above five seconds. Independent focused runs also have none; the snapshot case took 1.077 seconds and the three named graph cases took 0.798–1.471 seconds. | None; earlier failing timings remain historical evidence. |
| R1-F5: containment diagnostics truncate the target or effective root | R1-F5.1–R1-F5.8: short-path control, Darkmatter effect/frontmatter/editor terminal and JSON reports, Claudine composition and nested-container reports, shared writer projection | **verified fixed** | Complete messages travel alongside the concise event message. Long-path CLI tests assert both complete paths, including structured Darkmatter output, and pass. All file writers use the repaired shared projection. | None. |

### Authoring-document handoffs: R1-F1

Claudine's [task executor](../../../claudine/lib/src/composition/sequence/task/mod.rs)
now binds a task's guard, validation, setup, primary action, teardown, and
value evaluation to its authoring document. The
[group executor](../../../claudine/lib/src/composition/sequence/task/group.rs)
and [nested-sequence executor](../../../claudine/lib/src/composition/sequence/task/sequence.rs)
pair that document's path with its context for events, gates, loop limits,
and proxy resolution. Context-derivation failures return an error rather
than silently substituting a different document's base.

| Required instances | Verification |
|---|---|
| R1-F1.1, R1-F1.2 | The referenced-group library fixture writes initialize/start/success/finalize and gate lines to `groups/events.log`, with nothing beside the calling sequence. The CLI fixture checks the same distinction. |
| R1-F1.3 | Catalog-selected group events and member write beside the catalog in both the library and CLI fixtures. |
| R1-F1.4 | A group in a separate repository successfully writes absolute targets inside the caller's root, including initialize, member, and finalize. |
| R1-F1.5, R1-F1.10, R1-F1.11 | The internal task-adapter test runs setup, primary, and teardown from `tasks/task.yaml`; all three files appear beside that file and none beside the sequence. |
| R1-F1.6, R1-F1.7 | Referenced-sequence events/gate/task and referenced-group members retain their correct destinations. |
| R1-F1.8, R1-F1.9 | Inline group events/member and inline-sequence tasks use the containing document; the inline sequence retains its containing `origin_path`. |

The six tests in Claudine's
[authoring-document test module](../../../claudine/lib/src/composition/sequence/task/tests/authoring_documents.rs)
also exercise execution with and without an invocation object, and check
relative reads in group variables. The repair's additional loop-limit,
proxy, and group-variable changes complete the same handoff; they are not
remaining siblings of the original defect.

### Repository-discovery authority: R1-F2

Claudine's
[`lifecycle_mutation_root`](../../../claudine/lib/src/invocation_context.rs)
checks the retained launch-discovery result before returning the context's
captured boundary. It distinguishes a healthy repository, an observed
absence, and a failed discovery. It does not repeat repository discovery.

| Required instances | Verification |
|---|---|
| R1-F2.1, R1-F2.3 | Staged compose and inline-compose refuse a corrupt launch repository before initialize or start writes either absolute in-root marker. |
| R1-F2.2 | The wrapper pipeline checks discovery before constructing its lifecycle engine; the start-only CLI case fails before writing. |
| R1-F2.4 | The document-loop preparation path checks discovery before entering its first pass; the looping CLI case fails without markers. |
| R1-F2.5 | The harness uses the shared check whenever its guard contains lifecycle actions. The empty guard for a direct provider wrapper dispatches no lifecycle actions; composition already checks upstream. |
| R1-F2.6 | The root-sequence engine checks before initialize and tasks; the corrupt-repository sequence case writes neither marker. |
| R1-F2.7, R1-F2.8 | Both the task engine and step-guard engine call the same check. Their root-sequence caller checks before reaching either site. |
| R1-F2.9 | Darkmatter's corrupt-repository and broken-gitfile CLI cases still fail before any file action. |
| R1-F2.x | The duplicate launch-discovery follow-up is marked abandoned with a completion note explaining that this repair implements its scope. |

The shared library test also checks that the refusal names the launch
directory and original discovery cause. The public capture API remains
usable for commands that run no lifecycle actions. Sniff's broken-gitfile
handling was inspected using the sniff skill: an invalid gitfile cannot
silently select an enclosing repository as the boundary.

### Refusal reporting: R1-F3 and R1-F5

Claudine's [shared lifecycle reporter](../../../claudine/lib/src/composition/lifecycle/report.rs)
publishes failures that an ordinary error return would otherwise omit.
Root-sequence completion reports its recorded event failures, nested steps
retain secondary failures, and the shared gate runner warns when a gate
action fails. Gate action failures still leave the run outcome unchanged;
evaluation errors still fail the run.

| Required instances | Verification |
|---|---|
| R1-F3.1–R1-F3.4 | Root initialize/start/success/finalize refusals each exit 1 and report the target and root once, without an authored diagnostic-printing stack. |
| R1-F3.5 | A child refusal and a different refusal in the root failure event both appear once. |
| R1-F3.6–R1-F3.9 | Document, root-sequence, referenced-sequence, and referenced-group gates each report outside `ensure_file` and `edit` refusals while preserving exit 0. No editor starts. |
| R1-F3.10–R1-F3.12 | Every gate owner creates an in-root target, suppresses an outside failure under `no_error: true`, and fails on operand evaluation error. |
| R1-F3.13, R1-F3.14 | Nested-sequence primary and secondary event failures are both shown. A foreign group's relative target is now resolved beside that group and its outside-root refusal is reported by the owning step. |
| R1-F3.15 | Cancellation remains checked before editor dispatch; the existing cleanup-event test confirms that healthy cleanup can edit while canceled cleanup cannot. The root's cancellation branch reports cleanup errors separately. |
| R1-F5.1, R1-F5.7 | Short Darkmatter and nested Claudine refusal controls assert the complete root, replacing the earlier prefix-only assertions. |
| R1-F5.2–R1-F5.4 | Darkmatter's long effect and editor refusals retain target/root in terminal and JSON output; the frontmatter-writer JSON case does too. |
| R1-F5.5, R1-F5.6 | Claudine's long effect, frontmatter-writer, and editor composition refusals retain both paths. The library checks that the concise event message remains capped. |
| R1-F5.8 | Every frontmatter and filesystem writer reaches the shared target resolver and failure constructor; the every-writer refusal test passes. The long-path tests exercise effect, frontmatter, and editor projections. |

Darkmatter's [lifecycle error envelope](../../lib/src/lifecycle/context.rs)
keeps complete actionable text alongside the existing concise `err.msg`.
The terminal/JSON report paths use that complete text. Speech, notification,
and event bindings retain their concise text; no new persisted or wire field
is introduced. The new tests are declared in their consolidated L1 targets.

### Timing repair: R1-F4

The repair reduces work rather than removing coverage: it optimizes the
schema and SVG dependencies in the test profile, reuses compiled schema
patterns, indexes repeated operand lookups, and avoids Git subprocesses in
the mutation fixture. The split foreign-sequence tests retain both original
scenarios. No timing threshold, retry, ignore, or tier escape clears the
finding.

| Required instances | Verification |
|---|---|
| R1-F4.1–R1-F4.3, R1-F4.11 | Both split foreign-sequence cases, document-loop progression, referenced/inline containers, and the other mutation cases run in the passing 59-test CLI review run. |
| R1-F4.4–R1-F4.6 | The later full Claudine run includes the staging test and both schema-bundle cases and records no slow tests. The test-profile optimization applies to their shared schema work. |
| R1-F4.7–R1-F4.9 | The later full worktree run has no slow tests. This review also ran all 40 graph tests concurrently; the named sparse-lane cases took 1.471 and 1.310 seconds, and the observed-graph case took 0.798 seconds. |
| R1-F4.10 | The operand-walker indexing and immutable-node memoization were inspected. The language-server snapshot test passed in 1.077 seconds in this review's concurrent 71-test run. |
| R1-F4.12, R1-F4.13 | Executor, lifecycle CLI, passive-compose, base-schema, and expression-scope controls pass in this review's Darkmatter runs, with none slow. The later full Darkmatter run also has none slow. |

The full-run claims were checked against `target/slow-tests/runs.jsonl`,
rather than accepted from the repair summary:

| Area | Stored run/group | Tests recorded | At or above five seconds |
|---|---|---:|---:|
| Darkmatter | `2ff440cb-342e-4e89-895b-866fbbc8bb93` | 16,785 | 0 |
| Claudine | `028c77b8-a3e1-48bd-98f5-9b147daeb1c9` | 12,239 | 0 |
| worktree | `eb856628-0014-468b-94cc-9338f0ee6e06` | 1,506 | 0 |
| simplified-schema | `1794800286-43105` | 2,332 | 0 |

These later runs supersede the earlier timing failures for repair
verification. The old measurements remain in the history; isolated fast
measurements alone were not used to clear a failing concurrent run.

## Spec coverage and review scope

| Requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Caller boundary inside/outside Git, monorepo package launch, linked worktree, failed discovery | L1 isolated CLI filesystem fixtures and public-library observation tests | Appropriate; the working-tree top is selected from retained launch facts. |
| Same boundary across hosts, including external prompts | L1 compose/inline-compose, staged initialization, wrapper, sequence/group, adapter, and loop fixtures | Appropriate; all construction sites and required previous instances were checked. |
| Existing relative lookup, missing first candidate, explicit/nested paths, stdin, inline input, prefixed/environment references, unmatched recursive reference, URL refusal | L1 executor and CLI tests | Appropriate; resolution remains in Darkmatter's shared executor, using biscuit-file. |
| Every writer, active-document mirroring, another-file isolation, subsequent reads | L1 writer tables, identity tests, mutation-visibility and editor tests | Appropriate; the resolved target is reused for writing and identity comparison, and attempted operations invalidate cached reads. |
| Refusal before read/create/editor, parent/sibling/symlink boundaries, exclusive editor creation | L1 effect-engine, executor, and fake-editor tests, with platform-specific link fixtures | Appropriate; checks precede mutation reads as well as writes. Cross-OS execution evidence remains CI's responsibility. |
| Prep/proxy, retries, resumed attempts, loops, parallel member isolation | L1 lifetime fixtures | Appropriate; document changes replace the base while retaining launch authority. |
| Passive composition, validation, and language-server requests | L1 passive-compose and language-server no-side-effects tests | Appropriate; no lifecycle host is driven by these surfaces. |
| Existing interactive editor behavior | Previously recorded two focus-preserving L2 editor tests; L1 fake-editor checks for this fix's path behavior | Appropriate; this fix adds no keyboard-encoding or terminal-rendering requirement needing new L2/L3 evidence. |

The review included the spec, plan, original implementation record, review 1,
repair ledger, implementation commits and working-tree changes, affected
documentation, and repo/caller skills. The docs explain the boundary/base
distinction, shared prompts, subdirectory examples, and migration from
root-relative creation. The additional schema caches and operand indexes
preserve the existing translation and validation decisions.

An input robustness matrix is not applicable: the changed path-selection
and reporting code adds no configuration loader or field validator. The
schema-related repair caches existing analysis; it does not change accepted
document or field shapes. Lifecycle Markdown grammar is excluded by the
review's matrix rules.

## Verification performed in this review

| Command, from the named package area | Result |
|---|---|
| Darkmatter: `just test lifecycle compose_lifecycle authoring_documents lifecycle_refusal_reports invocation_context gate_action_errors` | 672 passed; none at or above five seconds. |
| Claudine: `just test-cli lifecycle_mutation lifecycle_refusal_reports` | 59 passed; none at or above five seconds. |
| Claudine: `just test-library authoring_documents invocation_context gate_action_errors lifecycle::context diagnostics` | 237 passed; none at or above five seconds. |
| Darkmatter: `just test diagnostics_then_point_requests_reuse_one_snapshot lifecycle_passive_compose base_schema_end_to_end expression_type_scope` | 71 passed; none at or above five seconds. |
| worktree: `just test git_graph` | 40 passed; none at or above five seconds. |
| Repository root: `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine` | Both passed; zero stranded tests. |
| Repository root: `git diff --check` | Passed. |

That is 1,079 executed tests in this review. Test resources were isolated
fixtures and fake providers/editors. No real editor or focused terminal or
browser window was launched. The existing full-area evidence was reused
after inspection; cross-OS runs and L2 tests were not repeated.

## Implementation record assessment

The repair log's completion claims agree with the inspected code, executed
regressions, and stored timing records. No new false implementation-record
statement was established that requires an in-place correction. The original
phase log and review 1's dated corrections retain the earlier partial repairs
and failing measurements as history; this review verifies their subsequent
repair rather than rewriting that history as an earlier success.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None. No unchanged instance, repair regression, or newly discovered sibling
of a previous finding remains open.
