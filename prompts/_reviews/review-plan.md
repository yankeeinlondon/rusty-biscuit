---
$schema:
    plan: file(required;eager;match(**/*plan*.md)) -> the plan which is being reviewed
    spec: file -> a spec file that the plan was based on
    design: file -> a design file that the plan was based on
    review: file -> a review that the plan was based on
description: >-
    This prompt will _review_ a plan before it's executed:

    - if there are any related docs such as a `spec`, `design`, or `review` document that the plan was based on then this review will try to identify those files and give the reviewing agent the context which these supplemental docs provide.
    - you may also explicitly pass in file references to these docs if the naming convention is non-standard
# Sibling documents, found by the naming convention when they were not passed in.
spec_doc: "{{ spec ? spec : file_exists(dirname(plan) + '/spec.md') ? dirname(plan) + '/spec.md' : null }}"
design_doc: "{{ design ? design : file_exists(dirname(plan) + '/design.md') ? dirname(plan) + '/design.md' : null }}"
review_doc: |-
    {{
        review
            ? review
            : contains(basename(plan), 'review-plan-')
                ? dirname(plan) + '/' + replace(basename(plan), 'review-plan-', 'review-')
                : file_exists(dirname(plan) + '/review.md') ? dirname(plan) + '/review.md' : null
    }}
start:
    message: "🧐 reviewing the plan `{{parent_dir(plan)}}/{{basename(plan)}}` before it is executed"
success:
    message: "✅ the plan review of `{{parent_dir(plan)}}/{{basename(plan)}}` has completed"
failure:
    message: "💥 the plan review of `{{parent_dir(plan)}}/{{basename(plan)}}` failed to complete ({{err.msg}})"
---

# Review a Plan Before Execution

## Context

- **Plan:** {{plan}}
::block when="spec_doc"
- **Specification:** {{spec_doc}} (the plan must deliver everything it requires)
::end-block
::block when="design_doc"
- **Design:** {{design_doc}}
::end-block
::block when="review_doc && file_exists(review_doc)"
- **Review:** {{review_doc}} (the plan must address every finding in it)
::end-block

The plan will be executed phase by phase by agents running unattended. Any
gap, ambiguity, or wrong assumption in it becomes wasted work or a stalled run,
so fixing it now is far cheaper than fixing it after execution.

::file "../_headless-orchestration.md"

## Task

Review the plan and correct it **in place**. Check that:

1. **It is complete.** Every requirement in the spec, and every finding in the
   review, maps to at least one task. List anything that is missing.
2. **Its facts are true.** Every file path, symbol, crate, command, and
   prerequisite the plan names exists in the repository today, or is created
   by an earlier phase of the plan.
3. **Its phases are ordered.** No phase depends on work from a later phase,
   and each phase leaves the build and tests passing.
4. **Each phase can be verified.** Every phase ends with concrete checks:
   named tests, commands, and their expected results.
5. **Its scope is right.** It does no work the source documents didn't ask
   for, and it pins every decision an unattended agent would otherwise have
   to guess.

Fix what you can directly in the plan. Anything that needs the author's
decision goes in an `## Open Questions` section, with the options and your
recommendation.

## Frontmatter

On the plan ({{plan}}), set:

- `plan_reviewed` to `true`
- `plan_reviewed_by` to "{{ctx.agent}}/{{ctx.model}}"
- `plan_reviewed_on` to "{{ctx.today}}"
- `total_phases` to the number of phases in the plan, if it is not already set
