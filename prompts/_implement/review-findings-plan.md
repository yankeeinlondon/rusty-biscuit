---
$schema:
    spec: file(required;eager; match(**/*spec*.md)) -> the specification file that defines the target goal for this feature or fix
    design: file(match(**/*design*.md))
    iteration: number(required) -> the iteration of the review cycle
    skip_performance_gates: boolean -> allows you to skip findings in the review which are due to a performance gate having not been met (or proven)
    review: file -> the latest review iteration which we will use to create the plan
description: |-
    This prompt will take a feature review as input and convert it into an executable plan
iteration: |-
    {{
        frontmatter(spec, 'review_iterations')
            ? frontmatter(spec, 'review_iterations') || 1 
            : 1
    }}
review: {{ dirname(spec) + '/' + 'review-' + iteration + '.md' }}
plan: {{ dirname(spec) + '/' + 'review-plan-' + iteration + '.md' }}
---
# Create Plan from Feature Review

## Key Documents

::block when="spec"
- Functional Specification: {{spec}}

> _the functional specification defines the ground truth for what this feature or fix is intending to do_

::end-block
::block when="design"
- Technical Design: {{design}}

> _this feature also includes a design document which is meant to act as a complimentary document to the specification document but going to into greater technical detail on some or all aspects of the specification's scope_

::end-block
::block when="review"
- Review: {{review}}

> _the last feature review of this specification is the primary concern for you building your plan and should present a set of findings all of which will have a importance metric (critical, high, medium, low) as well as a title on the first line of that feature followed by details around how this finding presents underneath._
::end-block
## Task

You are a senior technical manager with deep expertise in the Rust programming language and in planning complicated and large projects. Your task is to review the findings in the feature review "{{review}}" and create a high confidence plan from that which will be saved to "{{plan}}".

The plan which you create must follow the requirement listed below:

- Break work into **phases** and **tasks**
    - each **phase** will be a 1:1 mapping to a finding in the review
    - tasks map 1:M for each phase and describe the discrete tasks (in order) of the work that phase must complete
- Order tasks by dependency
- Flag parallelizable work
- Include validation checkpoints
- Keep tasks concrete and observable
- the tasks in the plan lead with a GFM inspired todo marker (e.g., `- [ ] {task}`)
    - this allows the implementation team to check off items in the plan as they complete them
- plans should ALWAYS start with Phase 1 (not Phase 0 or something else non-standard)

Once you have built the first draft of the plan, you must:

1. Iterate over each phase of the plan and document explicitly what the "acceptance criteria" will be for that phase being considered complete
2. Ensure each phase in the plan is started with a H2 heading indicating the finding's phase number and title; it should look something like:

    ```md
    ## Phase {3} [_{importance}_]: {title}
    ```

3. Directly under the H2 Heading of each phase you will add the following:

    ```md
    > Performance Gate: {status}
    ```

    Where `{status}` can be one of the following values:

    - `false` - use this when the finding the plan is addressing does not require a performance gate to be passed to be complete
    - `partial` - use this when the finding the plan is addressing _does_ include a performance gate to pass but also includes source code changes too
    - `true` - use this when the finding depends on a performance gate passing and does not require any source code changes

## Closure

- Save the plan as "{{plan}}"
- Add frontmatter to the plan document and set:
    - `total_phases` property to the number of phases defined in this plan
    - `created` add the date in YYYY-MM-DD format
    - `phase` set this to the starting phase number; usually 1 but may be 0 sometimes
    - `agent` set this to "{{ ctx.agent }}/{{ ctx.model }}"
    - `yolo` set this to "{{ env.YOLO }}"
