---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-render-surface/spec.md
plan: darkmatter/features/2026-11-05-render-surface/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
decisions_for_review:
    - |-
        Phase 1 wrote no code. The plan's Phase 1 has one task: finish the two
        prerequisite features (dotted paths, then loading data) before render
        work starts. The plan was written when both were unfinished. By the
        time this phase ran, both were done: every task in both plans is
        checked, both specs say `implemented: true`, and the last review of
        each says `ready: true` with no findings. So this phase only confirmed
        that the promised APIs exist and their test suites pass, and recorded
        that here. It did not reopen or re-run either feature's plan.
        The alternative was to re-run their plan phases anyway. That would
        repeat finished, reviewed work and could disturb their review results.
        Nothing needs redoing if the reviewer agrees. If the reviewer thinks a
        prerequisite is incomplete, the gap belongs in that feature's own
        review cycle, not in this feature.
message_to_agent: |-
    Phase 1 changed no files except this log and the plan's checkbox and
    frontmatter. Both prerequisite features (2026-11-06-dotted-paths,
    2026-11-04-loading-data) were already implemented and approved by their
    final reviews. Their APIs are listed in this log under "Phase 1". Use them
    directly in Phase 2 and later.

    The working tree holds about 80 uncommitted changes that this phase did
    not make. They are the loading-data review-1 fixes (lib/compose/expression/
    data/*, schemas/yaml_documents/*, biscuit-file data_locations, docs, and
    skills). Treat them as the baseline and do not revert them. Because of
    them, `affected_scope.py` reports non-empty packages even when a phase
    changes nothing, so plan to run the gates.

    The plan's Rule 1 says `compose.rs::parse_shorthand_value` still does
    private primitive parsing. That is out of date: compose now classifies
    tokens through `darkmatter::markdown::compose::setters::{RawSetter,
    SetterPlan, TokenClass, classify_token}`.
---

# Implementation Log for 2026-11-05-render-surface (7 phases)

## Phase 1

### Scope found

Phase 1 (Wave 1, "Complete dependencies") requires `2026-11-06-dotted-paths` and
then `2026-11-04-loading-data` to be finished before Phase 2. When this phase
started, both were already finished:

| Dependency | Plan tasks | Spec `implemented` | Last review |
|---|---|---|---|
| `2026-11-06-dotted-paths` | all `[x]` (plan `phase: 11` of 10) | `true` | review-4: `ready: true`, no findings |
| `2026-11-04-loading-data` | all `[x]` (plan `phase: 10` of 9) | `true` | review-2: `ready: true`, no findings |

So no dependency phase was re-run. No product source, docs, or skills were
edited in this phase.

### Checkpoint: the shared APIs exist

The Wave 1 checkpoint names five surfaces. Each one was confirmed in the tree:

- `PropertyPath`: `simplified-schema/lib/src/property_path.rs`, re-exported from
  `simplified_schema` (`lib.rs` `pub use property_path::{…}`)
- `SetterValue` / `SetterValueError`: `simplified-schema/lib/src/setter_value.rs`,
  re-exported from `simplified_schema`
- schema path candidates: `darkmatter::markdown::schemas::completion::path_candidates(&PathQuery)`
  (`darkmatter/lib/src/markdown/schemas/completion.rs`)
- shell-word recovery and shared completer: `darkmatter/cli/src/args/path_completion.rs`
  (`complete_compose_word`, `complete_{get,rm,set}_path`, `complete_set_value`,
  plus the private `select_schema`/`document_effective` helpers that Phase 3 will
  reuse for render completion)
- public format detection: `biscuit_file::detect_from_extension(&Path) -> FileType`
  (`biscuit-file/lib/src/detect.rs`)
- reader location helpers: `biscuit_file::data_locations::{locate_json,
  locate_jsonl, locate_toml, locate_json5, …}`
- compose setters now use the shared types
  (`darkmatter::markdown::compose::setters::{RawSetter, SetterPlan, TokenClass,
  classify_token}`). The private `parse_shorthand_value` that Rule 1 mentions no
  longer exists.

### Prerequisite suites (macOS, this worktree, run serially)

| Command (from area dir) | Result |
|---|---|
| `simplified-schema: just test` | 1362 run, 1362 passed, 0 skipped |
| `biscuit-file: just test` | 1273 passed; second variant 6 passed, 104 skipped (feature-gated variant) |
| `darkmatter: just test` | 14868 run, 14868 passed, 10 skipped, 110.5 s, exit 0 |
| `darkmatter: just lint` | exit 0 (includes zed-dmls wasm32-wasip2 check) |

No test failed, so none were rerun and none are load-sensitive.

### Gate-skip check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
returned `packages: [biscuit-file, darkmatter, darkmatter-lsp, simplified-schema,
test-toolkit, claudine, claudine-cli, darkmatter-cli]`, `change_class: package`.
Those packages come from the uncommitted loading-data review edits already in
the worktree, not from this phase. The gates were therefore run, and passed (see
above).

### Test design mapping

This phase changed no behavior, so it added no tests. Its requirement is that
the dependency surfaces compile and their suites pass. Evidence is the four
suite runs above. Dependent-package testing (claudine, claudine-cli,
darkmatter-lsp) was not run because this phase changed none of their inputs.

### Cross-OS

No code changed, so this phase carries no cross-OS risk and no cross-check was
run. The plan (Phase 7) also rules out `just cross-check` for this feature.
