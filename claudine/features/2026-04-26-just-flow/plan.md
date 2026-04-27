---
phases: 5
created: 2026-04-26
start_phase: 1
---

# Execution Plan: End-to-End `just` Flow

## Objective

Build a resilient, idempotent `just` orchestration (`just/flow.just`) that automates the full feature lifecycle from interactive configuration through autonomous design, planning, implementation, and review.

---

## Phase 1: Prompt Infrastructure

**Goal:** Create the six compose-based prompts required by the flow.

**Parallelizable:** All steps in this phase can be written in parallel.

| Step | Task | Artifact | Validation |
|------|------|----------|------------|
| 1.1 | Write `prompts/design.md` — instructs agent to produce a `design.md` from a spec | `claudine/prompts/design.md` | `md get design.md` returns non-empty content |
| 1.2 | Write `prompts/plan.md` — instructs agent to produce a `plan.md` from spec + design | `claudine/prompts/plan.md` | `md get plan.md` returns non-empty content |
| 1.3 | Write `prompts/implement-phase.md` — instructs agent to implement one plan phase | `claudine/prompts/implement-phase.md` | File exists and references `plan`, `phase`, `total_phases`, `memory` vars |
| 1.4 | Write `prompts/commit.md` — instructs agent to stage and commit changes | `claudine/prompts/commit.md` | File exists and references git staging / commit behavior |
| 1.5 | Write `prompts/review-feature.md` — instructs agent to review implementation against spec/design and set `ready` frontmatter | `claudine/prompts/review-feature.md` | File exists and instructs setting `ready: true/false` |
| 1.6 | Write `prompts/implement-feature-review-suggestions.md` — instructs agent to apply review suggestions | `claudine/prompts/implement-feature-review-suggestions.md` | File exists and references `review`, `spec`, `design`, `iteration` |

**Checkpoint:** Run `ls claudine/prompts/{design,plan,implement-phase,commit,review-feature,implement-feature-review-suggestions}.md` — all six files must exist with non-zero size.

---

## Phase 2: State Management Utilities in `flow.just`

**Goal:** Build idempotent state helpers that read/write spec.md frontmatter.

**Depends on:** Phase 1 (conceptual; utilities are independent but prompts must exist before flow runs).

| Step | Task | Validation |
|------|------|------------|
| 2.1 | Add `_read_config <key>` helper using `md get` to read frontmatter values from spec.md | `just _read_config clarify_agent` returns value or empty string without error |
| 2.2 | Add `_write_config <key> <value>` helper using `md set` to persist frontmatter values | After `just _write_config test_key test_value`, `md get spec.md test_key` returns `test_value` |
| 2.3 | Add `_has_design <dir>` helper — returns true if `design.md` exists and has non-whitespace content | Returns `true` for a dir with a populated design.md, `false` otherwise |
| 2.4 | Add `_has_plan <dir>` helper — returns true if `plan.md` exists and has non-whitespace content | Returns `true` for a dir with a populated plan.md, `false` otherwise |
| 2.5 | Add `_get_flow_iteration <spec>` helper — reads `flow_iteration` from spec frontmatter (default 0) | Returns numeric value or 0 when absent |

**Checkpoint:** Create a dummy spec.md with frontmatter, run each helper, and assert expected output.

---

## Phase 3: Interactive Setup Flow

**Goal:** Implement Phase 1 of the lifecycle (Interview, Spec, Clarify) with resume support.

**Depends on:** Phase 2 utilities.

| Step | Task | Validation |
|------|------|------------|
| 3.1 | Add `flow` recipe entry point that accepts an optional feature directory filter | `just flow` launches; `just flow just-flow` targets the correct feature dir |
| 3.2 | Implement configuration interview: if spec.md frontmatter lacks agent configs, run `fzf` selections for each stage (clarify, design, plan, implement, review) and persist with `md set` | Re-running `just flow` on same spec skips interview if all keys present |
| 3.3 | If Opencode is selected for a stage, additionally prompt for model and persist `*_model` key | `md get spec.md clarify_model` returns selected model when opencode chosen |
| 3.4 | After config is confirmed, check that spec.md has non-empty requirements body; abort with helpful message if empty | Empty spec triggers error banner and exit before clarify |
| 3.5 | Launch clarify session: `claudine compose @prompts/clarify.md --${clarify_agent} -y -i doc="${spec}" ${clarify_model}` | Session starts interactively; exiting session allows flow to continue |
| 3.6 | On clarify exit, print transition banner indicating move to autonomous execution | Console shows clear "Phase 1 complete → entering autonomous mode" message |

**Checkpoint:** Run `just flow` against a test feature. Verify interview runs once, config is saved, clarify launches, and on re-run the interview is skipped.

---

## Phase 4: Autonomous Execution Flow

**Goal:** Implement Phase 2 of the lifecycle (Design, Plan, Implement, Review Loop) with full idempotency.

**Depends on:** Phase 2 and Phase 3.

**Parallelizable within this phase:** Design and Plan checks are sequential by dependency, but Implement and Review are iterative.

| Step | Task | Validation |
|------|------|------------|
| 4.1 | **Design stage:** Check `_has_design`; if false, run `claudine compose @prompts/design.md --${design_agent} ...` targeting the feature dir | Second run skips design if design.md exists and is non-empty |
| 4.2 | **Plan stage:** Check `_has_plan`; if false, run `claudine compose @prompts/plan.md --${planning_agent} ...` | Second run skips plan if plan.md exists and is non-empty |
| 4.3 | **Implement stage:** Invoke existing `just implement-plan` (or inline equivalent) using `implement-phase.md` prompt and saved implementation agent/model | Plan phases execute sequentially; `start_phase` advances in plan.md frontmatter |
| 4.4 | **Commit after implement:** After implement-plan completes, ensure a commit is made (reuse existing `just commit` or invoke `prompts/commit.md`) | Git log shows a commit after implementation finishes |
| 4.5 | **Review loop entry:** Run `claudine compose @prompts/review-feature.md --${review_agent} ...` saving output to `review-${iteration}.md` | Review file created with `ready` frontmatter |
| 4.6 | **Review decision branch:** Read `ready` from review file frontmatter | |
| 4.6a | If `ready: true` — print success banner, exit flow | Console shows completion message |
| 4.6b | If `ready: false` and `flow_iteration < 5` — increment `flow_iteration` in spec.md, run `claudine compose @prompts/implement-feature-review-suggestions.md`, then commit, then loop back to step 4.5 | Suggestions applied, commit made, next review iteration starts |
| 4.6c | If `ready: false` and `flow_iteration >= 5` — print error banner, exit with non-zero status | Console shows cap-reached error; flow terminates |
| 4.7 | **Commit after review implementation:** Ensure commit runs after each review suggestion implementation | Git log shows commit between review iterations |

**Checkpoint:** Execute full autonomous flow on a small test feature. Verify idempotency (re-run skips design/plan), review loop iterates correctly, and cap enforcement works.

---

## Phase 5: Integration & Hardening

**Goal:** Wire `flow.just` into the build system and add resilience.

**Depends on:** Phase 4.

| Step | Task | Validation |
|------|------|------------|
| 5.1 | Import `flow.just` into `claudine/justfile` (or root justfile if appropriate) | `just --list` shows `flow` recipe |
| 5.2 | Add error handling: if any `claudine compose` stage fails, persist current `flow_iteration` and `start_phase` to spec.md frontmatter before exiting | Interrupted flow resumes at correct stage on re-run |
| 5.3 | Add timeout / crash resilience: each stage saves its progress to frontmatter immediately after success so crashes mid-flow are recoverable | Kill mid-stage; re-run resumes from next stage |
| 5.4 | Add user-facing status messages before each autonomous stage (colorized, with OSC8 links to artifacts) | Output shows "[Design] Skipping — design.md exists" or "[Design] Running..." |
| 5.5 | Update `claudine/just.md` (or README) with `just flow` usage instructions | Documentation mentions `just flow` and resume behavior |
| 5.6 | Run a dry-run end-to-end test on a feature directory with all stages | No errors; all artifacts produced; idempotency verified on second run |

**Checkpoint:** Run `just flow` twice on same feature. Second run should complete in <10 seconds by skipping all existing artifacts.

---

## Dependency Graph

```
Phase 1 (Prompts) ─────┐
                       ├──→ Phase 3 (Interactive) ──→ Phase 4 (Autonomous) ──→ Phase 5 (Integration)
Phase 2 (Utilities) ───┘
```

- Phase 1 and Phase 2 can be worked on in parallel.
- Phase 3 depends on Phase 2.
- Phase 4 depends on Phase 2 and Phase 3.
- Phase 5 depends on Phase 4.

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Missing `md` CLI | Verify `darkmatter` is installed in environment before running flow; abort with install instructions if missing |
| `fzf` not installed | Flow falls back to env vars or aborts with clear message |
| Agent CLI (e.g., `claude`, `opencode`) not installed | Check in interview step; allow user to select only installed agents |
| Git working tree dirty at start | Flow checks `git status` and warns user; optionally auto-stashes |
| Review loop never converges | Hard cap at 5 iterations enforced by step 4.6c |
