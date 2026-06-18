---
$schema:
    spec: file(required)
    iteration: number(required; default(1))
    design: file
name: Implement Review Suggestions
description: |-
    Implements all the recommendations/suggestions produced in a review.

    - if implementing a spec review, provide the `spec` file and an `iteration` number for the review
area: "{{ ctx.area }}"
dir: "$(dirname '{{spec}}')"
review: "{{ dir + '/review-' + iteration + '.md'  }}"
log: "{{ 'review-implementation-log-' + iteration + '.md' }}"
review_path: "{{review}}"
spec_path: "{{spec}}"
log_path: "{{dir}}/{{log}}"

success:
    effect: select-4
    say: "Implementation of review suggestions complete in {{ctx.area}}"
---

::block when="spec && iteration"
## Context

Your task revolves around **implementing** all the suggestions found in the recent review:

- {{review_path}}

The review was done to evaluate the fidelity of the implementation to the specification it was derived from:

- {{spec_path}}

## Task


1. Create a log file for this task at '{{log_path}}'

    - Add two H2 headings:
        - `## Implementation Notes`
        - `## Lessons Learned`

2. Review the suggestions in the review
3. Act as an orchestrator and iterate over all the suggestions in the review (serially):

    - ask a subagent to implement the suggestions and then report back a summary of what they did
    - write to the log file for this task -- {{log_path}} -- adding a H3 heading for the suggestion which completed and appending the summary that the subagent provided
    - if anything novel or unexpected came up during this implementation, add unordered list items to the `## Lessons Learned` section of the log document
    - move to the next suggestion

4. Once all review suggestions have been implemented:

    - communicate the summary of what was achieved
    - communicate any lessons learned during the process

## **IMPORTANT:**

- do NOT change the `ready` property in the review file after implementing
    - you may feel that everything in that review was fixed but the review's assessment at that time should not change
    - furthermore, we will be running another review _after_ you've completed here to validate that everything is fixed
- do not run `cargo fmt` ... we want functional changes during this work not formatting changes
- do not commit your work to git (this will be done as an independent process which you are not responsible for)
::file ./you-are-non-interactive.md

::end-block

::block when="!spec && review"

The following review has just completed:

- {{review_path}}

Your task is to implement all the suggestions in that review.

::end-block
