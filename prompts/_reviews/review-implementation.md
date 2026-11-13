---
$schema:
    review: file(match(**/*review*.md); eager) -> the review whose findings were implemented; derived from `plan` when omitted
    plan: file(match(**/*plan*.md); eager) -> the plan that turned the review's findings into implementation work
    template: file(match(prompts/*.md,.claudine/prompts/*.md)) -> the review prompt that produced the review's findings
description: |-
    Reviews how well the _implementation_ of a set of review findings addressed
    the problems the review raised, and files the verdict as the next indexed
    review (`review-1.md` -> `review-2.md`) with a `ready` flag.

    Pass either the `review` or the `plan` (the router passes `plan`); the other
    is derived from the naming convention when it is not passed:

    - `review-plan-N.md` <-> `review-N.md`
    - `plan.md` <-> `review.md` (or `review-1.md`)
# The baseline review: the one passed in, else the review that sits beside the plan.
baseline: |-
    {{
        review
            ? review
            : !plan
                ? null
                : contains(basename(plan), 'review-plan-')
                    ? dirname(plan) + '/' + replace(basename(plan), 'review-plan-', 'review-')
                    : file_exists(dirname(plan) + '/review.md')
                        ? dirname(plan) + '/review.md'
                        : dirname(plan) + '/review-1.md'
    }}
# The plan: the one passed in, else the plan that sits beside the baseline review.
baseline_plan: |-
    {{
        plan
            ? plan
            : !baseline
                ? null
                : is_indexed_file(baseline)
                    ? dirname(baseline) + '/review-plan-' + file_index(baseline) + '.md'
                    : dirname(baseline) + '/plan.md'
    }}
iteration: "{{ baseline && is_indexed_file(baseline) ? file_index(baseline) + 1 : 1 }}"
output: "{{ baseline ? dirname(baseline) + '/review-' + iteration + '.md' : null }}"
initialize:
    - when: "!baseline || !file_exists(baseline)"
      then:
          - error: "no baseline review found: pass `review=<file>`, or a `plan` that sits beside its review (`review-plan-N.md` or `plan.md`)"
start:
    message: "🥸 reviewing the _implementation_ of the findings in `{{parent_dir(baseline)}}` (review #{{iteration}})"
success:
    - when: "file_exists(output) && frontmatter(output, 'ready') == true"
      then:
          - message: "✅ review #{{iteration}} of `{{parent_dir(baseline)}}`: every finding from {{basename(baseline)}} is addressed (_production ready_)"
      else:
          - message: "⚠️ review #{{iteration}} of `{{parent_dir(baseline)}}` is NOT production ready: {{ code_link(output) }}"
failure:
    message: "💥 the implementation review of `{{parent_dir(baseline)}}` failed to complete ({{err.msg}})"
---

# Review the Implementation of Review Findings

## Context

::block when="template"
The findings in `{{baseline}}` came from running this review prompt:

- Prompt: {{template}}
::end-block

The review _findings_ are here:

- Review Findings: {{baseline}}

::block when="file_exists(baseline_plan)"
These findings were turned into an implementation plan:

- Implementation Plan: {{baseline_plan}}
::end-block

The findings have now been implemented in the source code. Treat completion
flags, plan checkboxes, and log summaries as claims to verify against the code,
not as evidence.

## Skills

::block when="has_skill(ctx.area)"
- use the '{{ctx.area}}' agent skill for a deep understanding of the package area
    - the implementation that just landed may have caused some documentation drift in this skill; note any drift you find, because removing it is part of this task
::end-block
- use the 'rust', 'rust-testing', and 'rust-devops' skills
- use the 'cli' skill for `clap` and the CLI standards this repo follows

::file "../_test-tiers.md"

::file "../_headless-orchestration.md"

## Task

Write the review to `{{output}}`.

1. **Verify every finding.** Act as an orchestrator: give each finding in
   `{{baseline}}` to a sub-agent, which checks:
    - the implementation addresses the problem the finding raised, at every
      site the finding names, not just the first one
    - tests exist that would fail if the fix were reverted; flag missing,
      faulty, or tautological tests
    - the fix is not a narrow patch where a DRYer, more ergonomic, or
      faster solution was the better choice
2. **Write `## Previous Finding Verification`.** A table with one row per
   finding: the finding, its outcome (**verified fixed**, **partially fixed**,
   **regressed**, or **unresolved**), the evidence you checked, and what
   remains.
3. **Write `## Findings`.** Every finding that is not verified fixed, plus any
   new defect the implementation introduced, with its location, the input
   that reaches it, and the result you expect.
4. **Write `## Summary`.** Credit what the implementation achieved, then
   state plainly whether the code is production ready and why. Every open
   finding blocks readiness, whatever its priority.
5. **Fix documentation drift** you noted in the package area's skill.

## Frontmatter

On the review file ({{output}}), set:

- `created` to "{{ctx.now}}"
- `agent` to "{{ctx.agent}}/{{ctx.model}}"
- `baseline` to "{{basename(baseline)}}"
- `ready` to `true` only when the `## Findings` section is empty, otherwise `false`
- `findings` to a list with one short line per open finding (an empty list when there are none)

On the baseline review ({{baseline}}), set `follow_up_review` to "{{basename(output)}}".
