---
area: claudine
status: implemented
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 1
completed: true
implemented: true
implemented_by: claude/opus
clarified: false
created: 2026-10-22
owner: Ken Snyder <ken@ken.net>
packages:
    - claudine-cli
human_review: true
needs_rulings: false
---

# Sequence body steps load a second, larger effective schema

## Problem

A `claudine sequence` run spends most of its time on schema work. On
2026-10-22 a profile of the slow `claudine-cli` L1 tests attributed it to
the sequence-level pass (`drop_invalid_optionals`) and the per-step pass
(`pre_validate_layered_for_mode`) each building an effective schema.

Counting every `load_effective_schema` call on a three-item sequence
`--dry-run` (debug build) showed 19 loads. Darkmatter's process-wide
caches served 17 of them in 1–12 ms each. Two were cold:

| Load | Frontmatter it validates | Cost |
| --- | --- | --- |
| 1st | `{sequence}` | 0.9–2.2 s |
| 11th | `{sequence, initialize, start, success, blocked, failure, finalize, cancelled}` (every stack `[]`) | 0.7–1.3 s |

## Cause

`mask_body_events` (`claudine/cli/src/commands/wrap/sequence/container.rs`)
stops a body step, which composes the sequence document itself, from
entering the document's lifecycle again. It does that by putting `[]`
into the step's data overrides for **every** lifecycle event except
`loop`, whether or not the document declares it. Pre-validation layers
those overrides onto the frontmatter. A present lifecycle key, even an
empty one, activates that event's trigger. So every step validates
against a different, larger effective schema whose lifecycle payloads
resolve cold.

An absent stack and an empty one both run nothing, so masking an
undeclared event changes no behavior. It only adds the second schema.

## Required behavior

- **R1.** A body step masks only the lifecycle events that the composed
  document declares or that its overrides already carry. Declared events
  still never run per step.
- **R2.** A sequence whose document declares no lifecycle events
  validates its steps against the same effective schema as the
  sequence-level pass, so the cache serves them.

## Acceptance

- **A1 (R1).** A unit test: undeclared events add no override; declared
  and handed events are masked; `loop` never is.
- **A2 (R1).** The existing `sequence_containers::body_only_steps_run_root_success_once`
  passes: a declared `success` and `finalize` run once.
- **A3 (R2).** The three-item `--dry-run` makes one cold schema load
  instead of two. The measured before and after are recorded in the
  implementation log.
- **A4.** The claudine area's L1 suites pass.
