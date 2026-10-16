---
$schema: feature-review.yaml
ready: false
findings:
  - "high: Source rereads change the frozen container graph"
  - "high: Referenced sequence events lose their own frontmatter"
  - "high: Container schemas accept forbidden cancellation actions"
observations: []
human_review: true
human_in_the_loop: true
human_review_items:
  - |-
      The implementation deliberately gives container events only shallow schema validation to reduce repeated schema work. For example, an editor accepts `cancelled: [retry]` on a nested sequence even though execution rejects it. The specification requires the schema to reject that action before execution.

      Review whether that reduction in editor validation is an intended product decision:

      - Keep the specified validation, as this review recommends, and fix container schemas while checking their processing cost.
      - Explicitly relax the specification and document which mistakes are caught only when running a sequence.

      This decision does not prevent an agent from implementing the existing specification. The other findings need no new design decision.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-15T14:03:24-07:00"
spec: "2026-10-24-sequence-containers/spec.md"
implemented: true
next: "2026-10-24-sequence-containers/review-2.md"
log: "claudine/features/2026-10-24-sequence-containers/log.md"
implemented_by: claude/opus
description: "A **feature** review of `2026-10-24-sequence-containers/spec.md`"
feature: "2026-10-24-sequence-containers/review-1.md"
---

# Review 1

The feature is **not production ready**. Three high-priority findings violate the specified container behavior. The existing focused tests pass, but do not exercise these cases. No implementation files were changed during review.

## Findings

### High: Source rereads change the frozen container graph

**Authority — spec:** requirement 2 states, “The discovered task graph stays frozen, but prompt sources are reread from disk at their normal preparation boundaries.” Requirement 3 separately requires, “Freeze discovered task graphs for the invocation.” An earlier shell or agent editing a later container file is a realistic input: the specification explicitly supports observing file edits between nested steps.

**Defect class:** preparation replaces the approved container structure with a newly discovered structure, checking command membership without preserving task membership or order.

In the Claudine library, [the nested sequence runner](../../lib/src/composition/sequence/task/sequence.rs:58) executes the freshly loaded sequence, and [the group runner](../../lib/src/composition/sequence/task/group.rs:99) does the same when a captured source changes. The shared [verify_nested_graph](../../lib/src/composition/sequence/task/sequence.rs:302) checks approved commands, templates, and prompt paths, but does not compare the child graph. Consequently, newly added work executes if it reuses an approved command.

Reproduce with a private directory containing these files:

```yaml
# root.md frontmatter
sequence:
  - name: rewrite
    shell: cp replacement.yaml child.yaml
  - name: nested
    sequence: child.yaml
success: [{stdout: "{{ seq.outputs }}"}]
```

```yaml
# child.yaml before execution
kind: sequence
sequence:
  - {name: original, shell: echo child}
```

`replacement.yaml` contains that same document plus a second step, `{name: added, shell: echo child}`. Run `claudine sequence --goose --yolo --silent root.md` with a private home and a fake Goose executable. The command exits 0 and prints `["",["child","child"]]`: the added step executes despite not belonging to the discovered graph. The command approval is not bypassed; the violated guarantee is the frozen structure.

The sweep used the same single edit—adding a second child with the already approved command—and the shipped CLI at each site:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Root sequence control | Earlier task replaces root file | Only original child executes | Original graph retained; clean |
| Inline nested sequence | Earlier task replaces its authoring document | Added child executes | Original graph retained |
| Referenced nested sequence | Earlier task replaces referenced YAML | Added child executes | Original graph retained |
| Inline group | Earlier task replaces its authoring document | Added leg executes | Original graph retained |
| Referenced group | Earlier task replaces group YAML | Added leg executes | Original graph retained |
| Catalog group (`bundle@child.yaml`) | Same edit in selected catalog entry | Added leg executes | Original graph retained |
| External task containing a sequence | Same edit in its referenced sequence | Added child executes | Original graph retained |
| External task containing a group | Same edit in its referenced group | Added leg executes | Original graph retained |

Keep the discovered children and their execution order while refreshing the sources and values that the contract permits to change. Add public execution regressions covering both container runners; command-approval refusal tests alone do not protect this boundary.

### High: Referenced sequence events lose their own frontmatter

**Authority — spec:** section 1 says, “For a referenced sequence, events belong in the target’s frontmatter,” and section 2 requires the same lifecycle events “with document semantics.” A reusable sequence declaring a value next to its own events must be able to read that value when those events run. It currently behaves differently when invoked directly and when referenced.

**Defect class:** crossing a sequence file boundary loads the target's lifecycle actions without loading the target's authored state into their evaluation context.

In the Claudine library, [load_sequence](../../lib/src/composition/sequence/preflight/mod.rs:905) extracts the referenced document's events but retains the caller's effective state. [The nested event evaluator](../../lib/src/composition/sequence/task/sequence.rs:87) builds event frontmatter from the caller's stack and runtime mutations. The referenced document's ordinary properties never enter that context; shell discovery also receives the caller's state.

For example, `child.md` contains:

```yaml
---
local_value: child-value
start: [{stdout: "{{ local_value }}"}]
sequence: []
---
```

Run it directly: it prints `child-value`. Reference it from a root document with `sequence: [{name: child, sequence: child.md}]`: it prints an empty line. If the root declares `local_value: caller-value`, it prints `caller-value`. These runs all exit 0, so a handler can silently take the wrong branch or send the wrong value.

The sweep kept the target declaration and expression unchanged, varying the reference format, placement, and expression consumer:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Direct root sequence control | Target authored value read by `start.stdout` | `child-value` | `child-value`; clean |
| Inline nested sequence control | Value declared in its authoring root | Authoring root's value | Authoring root's value; clean |
| Referenced Markdown sequence | Target property absent from caller | Empty line | `child-value` |
| Referenced Markdown, YAML, JSON, and JSON5 sequences | Target and caller declare different values | `caller-value` in all four | Target authored state must be established; caller values must not erase it merely by being the caller's frontmatter |
| Referenced sequence inside a group leg | Same declarations and `start.stdout` | `caller-value` | Target's authored value |
| External task referencing a sequence | Same declarations and `start.stdout` | `caller-value` | Target's authored value |
| Referenced sequence `initialize` and `finalize` | Same expression moved to either event | `caller-value` | Target's authored value |
| Referenced sequence condition | `local_value == 'child-value'` | False branch | True branch |
| Referenced sequence runtime `set` | Copy the expression, then print it | Copies `caller-value` | Copies target's authored value |
| Referenced sequence lifecycle shell | `echo {{ local_value }}` | Executes with `caller-value` | Discover and execute against target's authored state |

Establish the referenced document's state at the container boundary, preserving explicit parameters and runtime mutations with the intended precedence. Use that same state for approval discovery and event execution. Add direct-versus-referenced tests with a target-only property and a caller collision; current tests use literals or caller-owned values and miss this difference.

### High: Container schemas accept forbidden cancellation actions

**Authority — spec:** requirement 5 requires the `cancelled` event to be “typed with the section 4 action set, on every primitive” and says, “do not defer validation of nested containers to a later feature.” Section 4 excludes `retry`, `resume`, `proxy`, `prep`, `skip`, `error`, `break`, and `defer` from cancellation stacks.

**Defect class:** container schema fields validate only the collection's outer shape, so invalid actions pass schema validation and nested child collections bypass action validation entirely.

In Claudine's [task schema](../../schemas/lifecycle.yaml:354) and [group schema](../../schemas/lifecycle.yaml:370), `cancelled` uses `loose-body` instead of the restricted cancellation type. Nested `sequence` items and group `tasks` also use `unknown[]`. Runtime parsing rejects forbidden actions, but the required schema/editor validation does not. The implementation log records this as an intentional processing-cost tradeoff; it does not satisfy the stated requirement.

Reproduction: copy the shipped `action.yaml` and `lifecycle.yaml` into a private directory and validate this Markdown file with `md schema validate --no-baseline-schema --no-trigger-schemas --format json example.md`:

```yaml
---
$schema:
  nested: task@./lifecycle.yaml
nested:
  name: child
  sequence: []
  cancelled: [retry]
---
```

The shipped CLI returns exit 0 and `valid: true`. Replacing the field type with `cancelled-stack@./lifecycle.yaml` and its value with `[retry]` correctly returns exit 1. A `cancelled: 123` control is rejected under the task schema, proving that the container schema is being applied.

The sweep tested each of the eight forbidden verbs listed above at every site below, using the same public schema validation command:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Document cancellation type control | Each forbidden verb as a stack item | All eight rejected | Rejected; clean |
| Inline sequence task schema | Same items in `task.cancelled` | All eight accepted | Rejected |
| Group schema | Same items in `group.cancelled` | All eight accepted | Rejected |
| Nested inline sequence child | Same items one level inside `task.sequence` | All eight accepted | Rejected |
| Group leg containing a sequence | Same items inside `group.tasks` | All eight accepted | Rejected |
| Positive controls | `[stop]` and an empty cancellation stack | Accepted | Accepted; clean |

Restore the restricted action contract at container fields and nested child boundaries, with negative schema tests at each boundary. Prefer bounded reuse of existing action types over a new validation layer. If the performance cost requires reducing the guarantee, that needs an explicit specification change; documenting the implementation alone cannot close this finding.

## Verification and requirement coverage

Executed during this review:

- `just test sequence_containers` in Claudine: 20 passed.
- `just test-library container` in Claudine: 47 passed.
- `just test-l2 level2_sequence_container_ctrl_c` in Claudine: one real tmux test passed; the generator package had zero matching tests.
- `just test the_runner` in Messenger: five passed.
- `just check-tier-coverage claudine`: no stranded tests.
- Private CLI fixture sweeps described above, with a fake provider, private home/CWD, and audio disabled. No real provider or focused window was used.

The table below records the strongest relevant verification level present in the implementation, checked against test sources and declarations. “L1 present” means the requirement has in-process or hermetic subprocess assertions; it does not claim this review reran every test named by the implementation log.

| Acceptance requirements | Verification present | Assessment |
| --- | --- | --- |
| 1: nesting, cycle detection, concurrent ordered legs | L1 CLI and library | Present; graph-freeze defect above remains |
| 2: removed grammar and options | L1 parser and CLI | Appropriate level |
| 3–4: events, failure propagation, retry, no-session recovery | L1 CLI and library | Appropriate level; referenced authored state is missing |
| 5–6: fail-fast, sibling cancellation, timeouts | L1 controlled runners and CLI process termination | Appropriate level |
| 7: terminal Ctrl-C, descendant cleanup, exit 130 | L2 real tmux foreground-group interruption | Appropriate level; passed locally |
| 8–11: group/sequence values, renames, lexical restoration | L1 expression, parser, CLI, and library | Appropriate level |
| 12–14: body ownership, publication, lift-ready guard | L1 CLI, library, and source guard | Appropriate level |
| 15: ledger removal and Messenger command generation | L1 CLI/completion and Messenger tests | Appropriate level; repository search found no remaining ledger spelling outside historical material |
| 16–19: proxy cleanup, shipped prompts, edge cases, atomic state | L1 execution and corpus tests; shipped prompts have an explicit opt-in target | Appropriate level |
| 20–21: retry retention, file rereads, nested approval positions | L1 deterministic runner and CLI tests | Appropriate level; rereads currently replace the graph |
| 22: Messenger runner protection and legacy records | L1 public API and CLI | Appropriate level; focused runner tests passed |
| 23: container retry parameters | L1 CLI and library | Appropriate level |
| Requirement 5: schema restrictions and nested validation | L1 shipped-schema tests | Negative action cases stop at the document boundary; container cases need coverage |

The implementation's shipped-schema test accepts nested empty stacks and rejects a scalar stack, but does not test forbidden actions inside containers. Its task/group runtime tests do not substitute for schema tests. The declared L1 and L2 targets compile these focused tests, and the tier audit confirms their selection routes exist.

Input robustness review covered the new sequence/group/cancellation field checks, the authored-YAML matrix tests, and Messenger's run-record matrix. The three findings concern execution context, graph replacement, and schema action validation; none depends on silently defaulting a malformed configuration field. Rendered strings and shell templates are outside the requested configuration-shape matrix.

Full package L1 suites and lint were not rerun during this review. Their prior successful runs are recorded in the implementation log, rather than represented here as newly collected evidence. Cross-OS execution proof is left to CI and does not affect this readiness decision. The native Windows L2 test is currently an explicit skip; that platform coverage limitation is recorded in the implementation log and is not counted as a fourth finding.

## Observations

None.
