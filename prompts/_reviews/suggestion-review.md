---
$schema:
    review: file(required;eager) -> the review file which will be used as the baseline to compare against what has been implemented
    partial: string -> an optional expression used to express what _portion_ of the review's suggestions were implemented.
description: |-
    ## Review Implementation against Suggestions

    This review type will review the current implementation against the suggestions
    made in prior review.
usage: |-
    - pass in the `review` property as a file reference to the review baseline
    - (_optionally_) pass in a `partial` property to express that only a _portion_ of the review has been implemented and to ground this review on the just that set of suggestions
        - **Note:** use this _only_ when the implementation implemented only a subset of the baseline review's 
iteration: {{ file_index(review) + 1 }}
output: {{ dirname(review) + "/" + basename_without_index(review) + "-" + iteration }}
---
# Review of the Implementation of Suggestions

> a review of the efficacy of the current implementation's ability to address
> the concerns raise by the baseline review: {{ review }}.
::block when="partial"

**Note:** the implementation specifically aimed at addressing only a sub-section
of the baseline review's suggestions and the scope of this review will be to 
evaluate against that subset.

The subset of the suggestions which are relevant to this review are:

> {{ partial }}
::end-block

## Baseline Review

::file @{{review}}

## Task

> **Review File:** {{ output }}

- setup the structure of the review you are creating by adding the following sections to the file '{{output}}':
    - '## Baseline Review's Successes'
    - '## Suggestions from this Review'
    - '## Summary'
::block when="partial"
- review and understand the review's suggestions which are part of the subset that were implemented:

    > {{ partial }}
- act as an orchestrator and iterate over each suggestion (in scope)
::end-block
::block when="!partial"
- review and understand the baseline review's suggestions
- act as an orchestrator and iterate over each suggestion
::end-block
- for each suggestion, have the subagent:
    - evaluate the tests meant to evaluate that the implementation is now addressing the suggestion
    - look for _faulty_ tests as well as _missing_ tests and if found document these missing tests as problems that must be addressed
        - suggest the test change/addition, explain why it's important
    - then look at the implementation that has been put in place to address the baseline review's suggestion and make sure that it:
        - functionally addresses the concern that the baseline review brought up
            - _or addresses the non-functional concern if this is a non-functional concern_
    - ask the question: does this implementation focus on a narrow solution where a better solution (DRYer, more ergonomic, higher performance) solution was a better choice. If this is the case then you should add this to your recommendations as well.

    > **Note:** all content added by the subagent should be in the `## Suggestions from this Review` section

- now that all baseline review suggestions have been _reviewed_; review the suggestions you have generated in aggregate and make sure:
    - everything is expressed clearly and with adequate detail
    - if there are any duplicates they are merged together
        - you may consider merging together missing or broken tests with broken or missing functionality so that they are semantically grouped 
        - when you do that be sure that you're not loosing any detail when merging
- now take the time to describe what the implementation _was_ able to successfully achieve in terms of addressing the baseline review's suggestions; write this to the `## Baseline Review's Successes` section of your review document
- now you will fill in the `## Summary` section:
    - start by giving credit to the implementation's successes and how they've improved the quality of the code
    - then assess whether you believe the code is now "production ready"
        - explicitly state whether you believe the code to be production ready
        - explain why you came to your conclusion on readiness
        - Note: 
            - if there are NO recommendations coming from this review the production readiness should be `true`
            - if there _are_ recommendations that typically suggests the code is NOT production ready but that is not always the case:
                - if you have found some ergonomic or performance optimizations that 
- the final step of your review is to set Frontmatter metadata:
    - on your review file -- {{output}} -- you will set the following:
        - `$schema` as "suggestion-review.yaml"
        - `created` as "{{ctx.now}}"
        - `agent` as "{{ctx.agent/ctx.model}}"
        - `description` as "reviewed the _implementation success_ of addressing the concerns raised in the '{{parent_dir(review)}}/{{basename(review)}}' review"
        - `parent_scope` as "{{ parent_dir(dirname(review)) }}"
        - `scope` as "{{ parent_dir(review) }}"
        - `baseline` as "{{parent_dir(review)}}/{{basename(review)}}"
    - on the baseline review file -- {{review}} -- you will set the following:
        - `implemented` as `true`
        - `follow_up_review` as "{{ basename(output) }}"
