---
spec: "claudine/fixes/2026-09-16-better-spec-syntax/spec.md"
plan: "claudine/fixes/2026-09-16-better-spec-syntax/plan.md"
implemented_by: "claude/default"
started_phase: "1"
implemented: false
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
human_review: true
human_review_items:
  - "Confirm Rulings 1, 3, 4, and 5 before Phase 2."
message_to_agent: "Resume Phase 1, not Phase 2. Storage is restored, but baseline and graph impact evidence are missing. Ruling 2 is approved: reject duplicate YAML mapping keys in Darkmatter. Read implementation-log.md and obtain confirmation of Rulings 1, 3, 4, and 5."
---

# Implementation Log for 2026-09-16-better-spec-syntax (5 phases)

## Phase 1

### Recovery record — 2026-09-17

These findings were recovered from the prior agent's handoff supplied by the
author. They are reported results, not independently rerun during this
update. The original agent attribution is retained.

The attempt stopped with ENOSPC on `/private/tmp` and `/Volumes/coding`,
including failed log writes and cleanup. The author has restored storage.
No source changes were reported. The plan's startup `implemented: true`
was not completion evidence and is corrected to `false`; Phase 1 remains
incomplete and no `completed_phase` is set. Empty phase file lists describe
implementation deliverables, excluding plan/log bookkeeping. The log's
`spec` field incorrectly pointed to the plan; it now points to the spec.

### Spike 1 — Duplicate keys

Tested with the workspace's exact `serde_yaml_ng` version and `md get`:

- Darkmatter's frontmatter map silently keeps the last duplicate at both
  top-level and nested mappings. `set: {epilog: first, epilog: second}`
  retains `second`. The same occurs inside `{{ … }}` values.
- Parsing into `serde_yaml_ng::Value` instead rejects duplicates with line
  and column.
- `parse_lifecycle_config` receives already-collapsed JSON; Claudine cannot
  detect duplicates at the `set` boundary.

At recovery, Ruling 2 awaited the author's decision: (a) reject duplicates in shared
Darkmatter `parse_yaml_with_fallbacks` (recommended; affects all frontmatter
readers and requires expanded scope/validation); (b) reparse raw frontmatter
in Claudine (requires amending the no-second-parser constraint); or (c) drop
the duplicate-key requirement. Option (a) was subsequently approved; see
the decision record below.

### Spike 2 — Execution and result plumbing

- Reported locations: `dispatch_side_effect` at `executor.rs:1293`, its
  `set` arm at 1342, `apply_runtime_set` at 1439, and outdated comment at
  1436. Line numbers are navigation hints to recheck before editing.
- Only sequence side-effect tasks expose results through `run_side_effect`
  and `Ok(other) => other.to_string()`. Event stacks and setup/teardown
  discard values (`executor.rs:1172-1182`).
- `dispatch_task_side_effect` writes its working map back to live state even
  when the action fails. Validate and resolve everything before mutation;
  test this outer write-back path for partial updates.
- `sequence/task/group.rs:406` also calls `RuntimeState::set` to merge
  parallel-group results; the original caller inventory missed it.
- Lifecycle `set('k', v)` already fails with `LifecycleShortFormRemoved`,
  but suggests the positional form being removed. Phase 2 needs a
  mapping-specific suggestion for `set`.

### Spike 3 — Migration inventory corrections

- `claudine/lib/src/composition/sequence/task/tests.rs`: about 17 sites.
- `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md`:
  executable fixture migration touches Darkmatter.
- `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`:
  already drifted from the real prompt.
- `action_shape_control.rs`: no `set` usage; not a migration target.
- Real prompt: lines 40, 56, and 57. `sequences.md`: 389, 392, and 479.
- `composition/error/tests.rs:2174,2181`: `set` is only a label; review
  these assertions when diagnostics change.
- No hits in `claudine/schemas/` or Claudine skill files.

Preserve the plan's non-targets: loop-control `set(...)`, Darkmatter
capability descriptors, unrelated jq expressions, and historical completed
specs that are not executable inputs.

### Rulings and remaining Phase 1 work

Rulings 1, 3, 4, and 5 retain the plan's recommendations, without recorded
author confirmation. Ruling 3 additionally requires explicit matching in
`is_side_effect_action` and task dispatch. Ruling 5 includes the call-spelling
fix above. Ruling 2 is now approved as recorded below.

1. Review scratch cleanup: `/tmp/bss-spike` holds the small test crate and
   build output; `/tmp/bss-baseline` holds partial logs. Previous deletion
   failed. If further build-space reclamation is necessary, follow the
   storage-strategy skill and run `just sweep` before manual deletion.
2. Rerun `just test`, `just test-l2`, and `just lint` in `claudine/`, saving
   outputs and exit statuses. The prior L1 attempt started a from-scratch
   build and exited nonzero, likely but not conclusively due to disk
   exhaustion. Neither L2 nor lint left a log. No baseline is established.
3. Run `just gitnexus`, then upstream impact for `parse_lifecycle_config`,
   `parse_positional_action`, `dispatch_side_effect`, `RuntimeState::set`,
   `dispatch_task_side_effect`, and `is_known_side_effect`, plus
   Darkmatter's `parse_yaml_with_fallbacks` for approved Ruling 2. Record callers,
   processes, and risk. The prior refresh failed once; later attempts hit
   ENOSPC. No impact results were recorded.
4. Obtain confirmation of Rulings 1, 3, 4, and 5. Ruling 2 is approved
   and incorporated into the spec, scope, and validation plan. Capture
   Darkmatter `just test` and `just lint` baselines for its shared parser.
5. Record actual baseline and graph results before closing Phase 1. Keep
   `human_review: true` while author decisions remain pending.

This recovery update changes only the plan and implementation log. Tests,
graph refresh, cleanup, and Phase 2 implementation were not run.

### Ruling 2 closed — author approval, 2026-09-17

The author approved the recommendation to reject duplicate keys in
Darkmatter's shared frontmatter parser (option a). This is a confirmed
design decision, not a completed implementation task.

The approved contract rejects duplicates within any YAML frontmatter
mapping, including nested mappings, consistently across direct,
indentation-normalized, and expression-protected parsing. Preserve typed
errors and source-location information. Fallbacks must not hide duplicates;
Claudine must not reparse frontmatter. Duplicate-looking text in expression
strings remains expression content. Explicit merging between separate
documents is unchanged.

The behavior change applies to all Darkmatter frontmatter consumers;
last-wins documents must be corrected. The spec and plan now include
Darkmatter scope, parser impact analysis and baselines, fallback and nested
mapping regressions, valid expression coverage, passive shipped-artifact
validation, normal CLI invocation, and documentation updates.

Ruling 2 is checked off and removed from `human_review_items` in both plan
and log. Rulings 1, 3, 4, and 5 still await confirmation, so
`human_review: true` and Phase 1 remain in place. No source changes, baseline
runs, or implementation completion are claimed by this decision update.
