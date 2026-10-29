---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Shell assignments still copy the enclosing sequence"
observations: []
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-26T18:27:09-07:00"
spec: 2026-10-26-sequence-quadratic-frontmatter/spec.md
implemented: true
description: "A **fix** review of `2026-10-26-sequence-quadratic-frontmatter/spec.md`"
fix: 2026-10-26-sequence-quadratic-frontmatter/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/fixes/2026-10-26-sequence-quadratic-frontmatter/log.md
implemented_by: claude/opus
next: 2026-10-26-sequence-quadratic-frontmatter/review-2.md
---

# Review 1

The fix is **not production ready**. The principal scaling fixtures pass, but
shell assignments retain whole-sequence copies in preflight and execution.
This is one high finding, with the affected and clean sibling paths below.
No human decision is needed.

## Decisions reviewed

Two independent reviewers read the spec and implementation log before the code
review. Both found `decisions_for_author: []`, no exceptional human-review
decision, and the same explicit narrowing: conditional shell assignments were
left copying full state because adapting the interpolation state was considered
a wider change. That omission is included in R1-F1, rather than referred to the
author.

The other implementation choices are settled:

| Log phase | Decisions and their effect |
| --- | --- |
| Attribution | Count copied/scanned JSON nodes; place shared state in Darkmatter; seed and scan common state once per preparation; bucket prompt references before comparing them; remove the temporary profiling repeat loop. These target measured repeated work without a permanent profiling interface. |
| Shared foundation | Use one immutable base with private top-level writes/removals; preserve deterministic iteration; keep the existing external-state merge path; stage preflight migration until the following phase. The migration is now implemented. |
| Preflight and execution | Scan step overlays with a correctness fallback; classify provider hints using their transitive reads; have lifecycle stacks borrow shared views; share a document-read interface; materialize shell state lazily; validate each group owner once per walk; distinguish view creation from copied nodes; enable counted regressions in CI. Only the lazy shell-state materialization leaves requested scope unfinished. |
| Verification | Probe the actual live-cell construction; use one CLI test for the next-step disk reread; retain the allocation bench because it already uses production preflight. |

The review's decision to retain all shell-assignment shapes in scope, with
alternatives, reasoning, and downsides, is recorded under Phase 4's
`### Design Decisions` in the implementation log. It preserves the existing
syntax and behavior while requiring shared state through the shell-value
boundary. It does not need confirmation.

## Findings

### R1-F1 (high): Shell assignments still copy the enclosing sequence

**Authority: spec.** Required behavior says preparing, preflighting, or running
step *k* “never copies or re-analyzes the other steps' definitions.” The
attribution section also states that a site is in scope when its cost per step
grows with sequence size. The implementation log's “Sites deliberately left”
and Phase 3's plain-`set` decision explicitly retain the conditional preflight
copy. Removing it is requested scope, not an optional constant-factor saving.

**Defect class:** a shared frontmatter view is converted back into a fully
owned map at shell-assignment boundaries, making a fixed assignment's work
grow with unrelated sequence definitions.

In Claudine, [the preflight shell-value recorder](../../lib/src/composition/sequence/preflight/mod.rs#L1941)
passes a closure calling `self.step_frontmatter.to_hash_map()` to Darkmatter's
[conditional shell-value resolver](../../../darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs#L125).
Plain commands skip this closure; conditional commands invoke it and copy
the entire `sequence:` list. A sequence of conditional assignments therefore
still incurs quadratic copying.

In Darkmatter, [the lifecycle shell-value execution adapter](../../../darkmatter/lib/src/lifecycle/executor/shell.rs#L193)
unconditionally clones every entry of the shared state before calling the
shell-value engine. This affects plain commands as well as conditional ones.
The returned command output is correct, but each invocation copies all the
other definitions. Claudine's production lifecycle runner calls this adapter.

For example, repeat this ordinary container step with distinct names in a
document containing `flag: true`:

```yaml
sequence:
  - name: check
    sequence: []
    success:
      - set:
          result: "$( flag ? echo yes : echo no )"
```

The reproduction starts from the existing plain-shell-`set` scaling fixture
and changes just its assignment to the conditional form. Both branches stay
in the approval inventory. The graph contains every requested step; the
runtime public API returns `yes`. These checks rule out a skipped walk or
failed command as the cause of the measured work.

#### Class sweep

Preflight probes use `resolve_sequence_plan` and `build_preflight_graph` on
temporary Markdown documents at 10 and 100 steps. Runtime probes pass the same
document state and resolved assignment to the public
`execute_set_shell_values` API. Temporary instrumentation at its collection
boundary confirms that the copied state includes all 10 or 100 sequence
entries. Node counts below are the existing `ShellValueStateNodes` counter,
not elapsed time.

| Child ID | Site and shape tested | Observed result at 10 / 100 steps | Expected result |
| --- | --- | --- | --- |
| R1-F1.1 | Sequence-container `success` event, conditional assignment | 938 / 72,488 copied nodes | No copy of unrelated definitions; fixed assignment work independent of step count |
| R1-F1.2 | Same event, plain assignment control | 0 / 0 copied nodes; all commands recorded — clean | No whole-state materialization |
| R1-F1.3 | Group-container `success` event, conditional assignment; one shell leg per group | 1,438 / 122,488 copied nodes | Same bounded assignment work as sequence events |
| R1-F1.4 | Same group event, plain assignment control | 0 / 0 copied nodes; leg and assignment commands recorded — clean | No whole-state materialization |
| R1-F1.5 | Runtime adapter, conditional assignment | Copies the complete 10 / 100-entry definition list; returns `yes` | Read condition and branches through shared state |
| R1-F1.6 | Runtime adapter, plain assignment | Copies the complete 10 / 100-entry definition list; returns `yes` | Execute fixed bytes without copying unrelated state |
| R1-F1.7 | Authored task `setup` and `teardown`, each assignment form | Retired grammar is rejected before preflight — clean public boundary | Rejection; these internal legacy stack fields do not admit new authored inputs |
| R1-F1.8 | Ordinary `shell:` commands | Existing ten-hole regression passes at both sizes — clean | Constant expression parses per step; recorder returns before the assignment-state conversion |

The event callers share `prepare_container_events`; nested sequences and
groups feed their events into the same recorder. The root sequence also uses
it, but its root events are prepared once per walk, so they do not introduce
a per-step multiplier. The internal legacy setup/teardown collector also
feeds this recorder, with the authored rejection shown above. At execution,
both resolved-byte and template assignments converge on `run_set_shell_values`
and the same adapter; template mode supplies a hole evaluator but does not
bypass the full-state collection. Approval enumeration then consumes the
resolver's pipelines through `commands()`, so the repair must retain both
conditional branches and their approved bytes.

**Smallest repair:** carry the existing shared frontmatter through the
shell-value seed lookup used for branch interpolation and execution. Keep
authored structural values readable; do not delete `sequence`, weaken the
acceptance criterion, or restrict the repair to the plain-command fixture.
Add counted coverage for conditional preflight and for both runtime forms,
with outcome/approval assertions. This needs an adapter change in Darkmatter,
but no new syntax, protection layer, or large synthetic input matrix.

## Verification and requirement coverage

All changed behavior here concerns ownership, expression analysis, and file
visibility. **Level 1 is appropriate**: no new terminal rendering, keyboard,
mouse, or input-encoding contract is introduced.

| Requirement | Verification and result |
| --- | --- |
| Equal shell expression parses per step at 10 and 100 | Existing counted L1 regression passed during review |
| Constant per-leg view construction and every leg executes | Existing counted L1 regression passed during review; log reports 288-byte cell probes at every measured size |
| Prompt preflight scaling | Counted L1 regressions passed during review; implementation log reports full-preflight bytes/task within 3.1% at 100 and 1,000 tasks; allocation benches were not rerun in this review |
| Private sibling writes, ordered publication, retries, approvals, nested execution | `just test-library --features work-counters composition::sequence composition::lifecycle composition::looping runtime_state`: 1,486 passed, 7.506 seconds total; no selected test exceeded five seconds |
| Disk write becomes visible to the next root step, while sibling sees group-start state | `just test-cli a_leg_write_reaches_disk_for_the_next_step_but_not_its_sibling`: passed, 1.269 seconds |
| Every per-step shell assignment avoids unrelated definition copies | Review probes reproduce R1-F1; existing plain-preflight test alone does not meet this requirement |
| Full package gates and tier placement | Implementation log records Claudine L1, L2, lint, and tier-coverage success, plus Darkmatter L1/lint; these full gates were not repeated during review |

The new CLI test is declared by the consolidated L1 target and was selected
by the normal recipe. Counted tests are enabled in Claudine's test recipe and
CI metadata. Review probes and instrumentation were temporary and removed;
production code was not repaired during this review.

The Input Robustness Matrix does not apply: the change adapts state ownership
and lookup APIs without changing a file loader, deserializer, or configuration
grammar. Cross-OS proof is left to CI and does not block this review.

## Observations

None.
