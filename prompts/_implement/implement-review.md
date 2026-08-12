---
$schema:
    review: file(required;eager;match(**/*review*.md)) -> the underlying review file who's findings we will implement
    target: file
    iteration: number
    initial_review: file -> the OG review that kicked off this review/implement cycle
description: |-
    This prompt expects that the originating content was a review and that we are either _implementing_ the findings/suggestions 
    of that original review, or, if the review has been marked as "implemented" then we will move to the highest indexed review
    in the same directory and implement that (unless that too is marked as "implemented").

target: "{{ review }}"
iteration: {{ file_index(review) }}
report: {{ dirname(review) + '/' + 'implementation-report-' + iteration + '.md' }}
initial_review: {{ review }}

initialize:
    stack:
        - when: "!frontmatter(review, 'implemented') && !is_indexed_file(review)"
          action:
              - message: "🏃  starting the _implementation_ of the findings/suggestions in the review {{review}}"
        - when: "frontmatter(review, 'implemented') && !is_indexed_file(review) && find_latest_index(review) && frontmatter(find_latest_index(review), 'implemented')"
          action:
              - message: "😵  the initial review `{{review}}` was _implemented_ and so was the most recent iteration of the review cycle: `find_latest_index(review)`! Nothing to implement."
              - error: "the review `{{review}}` and the follow-on review/implement cycle is ready for another _review_ not an _implemenation_!"
---
# Implementation of Review Findings

## Agent Skills

- you are working in the **{{ctx.area}}** _package area_ and should use the '{{ctx.area}}' agent skill for this implementation.
- because this is an implementation task primarily you should also leverage the 'rust' agent skill
- and when working with testing you will use the 'rust-testing' agent skill
- while acting as orchestrator, you should instruct subagents to also use these agent skills

## Task

> **Review:** {{target}}

Your task is to review the _findings_ in the review and then iterate over each as an orchestrator:

- on each finding you will instruct the subagent to:
    - review the finding,
    - make testing adjustments
        - evaluate the current test suite to understand why this finding was not exposed by testing
        - **add** and/or **adjust** tests so that this finding is correctly identified as a failing test
    - implement changes
        - now implement the _finding_ and use your tests to validate the code works
        - iterate through this process until you are confident that you have strong test coverage for this feature and that all tests pass
        - validate that no lint errors/warning exist and fix if they do
    - summarize back to you the test coverage adjustments and the fix strategy they employed
    - they should also be encouraged to mention anything novel or unexpected that they found
        - as orchestrator you will gather all of these novel findings both so that you can provide these insights to the remaining subagents but also so they can be reported as a part of you final report
- evaluate the documentation that might need updating as a function of the changes that were made and update all documentation drift you find
- save a summary of what you did along with any novel/unexpected findings that were discovered during the implementation to: '{{report}}'

Once the review is complete you must set Frontmatter metadata on both the review and implementation report:

- On the underlying review ({{review}}):
    - set `implemented` to `true`
    - set `implemented_by` to `{{ctx.agent}}/{{ctx.model}}`
    ::block when="initial_review != review"
    - set `initial_review` to `{{ parent_dir(initial_review) + '/' + basename(initial_review) }}`
    ::end-block
- On your implementation report ({{report}}):
    - set `agent` to `{{ctx.agent}}/{{ctx.model}}`
    - set `created` to "{{ ctx.now }}"
    - set `underlying` to "{{ parent_dir(review) + '/' + basename(review) }}"

> Note: make sure both documents, along with their updated Frontmatter properties, are saved to disk before considering the task done
