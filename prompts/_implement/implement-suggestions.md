---
$schema:
    spec: file(required; match(**/*spec*.md)) -> the specification file defining the target functionality
    design: file(match(**/*design*.md)) -> optionally, a design file which compliments the spec file with more details on the technical design
    review: file -> the review file who's findings/suggestions we will implementing
    iteration: number(required) -> what _iteration_ of the review/implement cycle that we are on
    log: file -> the log file we're writing to while working on the implementation
    retry: string -> if you're retrying this prompt after a failure then pass in what you know about prior progress
description: |-
    Implements the findings in a review which was conducted to determine the drift between
    what was actually implemented versus the specification that was being targetted in that
    implementation.
iteration: "{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') || 1 : 1 }}"
review: "{{ dirname(spec) + '/' + 'review-' + iteration + '.md' }}"
log: {{ dirname(spec) + '/log.md' }}
# is the review "production ready"?
ready: "{{ review && file_exists(review) ? frontmatter(review, 'ready') : null }}"

design: "{{ file_exists(dirname(review) + '/design.md') ? dirname(review) + '/design.md' : null }}"

feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"

initialize:
    stack:
        - action:
            - ensure_file: '{{log}}'

start:
    message: "🏃 starting implementation #{{ file_index(review) }} of `{{ parent_dir(review) }}` review suggestions (_using_ {{ctx.agent}}/{{ctx.model}} _in_ {{ctx.area}})"
success:
    message: "✅  implemented suggestions from review **#{{ file_index(review) }}** of `{{ parent_dir(review) }}` in **{{ctx.area}}** package area"
    say: "the review suggestions for {{ title_case(without_date(parent_dir(review))) }} in {{ctx.area}} completed successfully"
    effect: bong
failure:
    message: "❌ implementation of the review #{{iteration}} suggestions from **{{ parent_dir(review) }}** failed to complete ({{err.msg}})!"
    effect: sad-trombone
---

# Implement Review Suggestions for {{title_case(without_date(parent_dir(spec)))}}

> - **{{capitalize(feature_or_fix)}}:** `{{parent_dir(review)}}`
> - **Iteration:** {{iteration}}
> - **Log File:** {{log}}

## Key Documents

- **Review:** @{{review}}
- **Iteration:** {{file_index(review)}}
- **Specification:** {{contains(spec, ctx.area) ? '@' + spec : '@' + ctx.area + '/' + spec }}
::block when="design"
- **Design:** @{{design}}
::end-block

::block when="spec"
> **Note:**
>
> The review who's suggestions you are tasked with implementing was based
> on the detected _delta_ between the specification file above and the
> actual implementation source code.
::block when="design"
> 
> The design document above was created as a complimentary document to the
> specification file.
::end-block
::end-block

## Skill Selection

- use the '{{ctx.area}}' agent skill
- use the 'rust' skill when writing code
- use the 'rust-testing' skill when writing or debugging tests

## Task

::block when="!ready"
The review file above has completed in the {{ctx.area}} package area with a number of findings/suggestions for implementation.
::end-block

### Start with Logging

In order to track progress of this task we are using the '{{log}}' as a log file:

- you should immediately create a section for your part of this log file:
    - add an H2 header section called `## Implementation of Review Findings #{{iteration}`
    - then timestamp it with: `> **started at:** {{ctx.now}}`
- when you write to this section you always write as a set of Markdown unordered list items 
    - if you have structured sub-items for something then you can and should use a nested unordered list to represent this
- now start this unordered list by mentioning some important metadata for this implementation:
    - `- this implementation is attempting to implement _all_ of the review findings found in '{{ review }}'`
    - `- this is iteration {{iteration}} of the review-to-implement cycle `
    ::block when="retry"
    - `- we are _retrying_ iteration #{{iteration}} because in our last attempt the agent failed before achieving it's objectives`
        - `- the only context we have about where we got to in this previous attempt is:\n\t{{retry}}`
        - `- this does mean that some of the findings from the review are already likely implemented (but not all)`
        - `- to accommodate this, as you iterate over the different findings/suggestions, always first check if the work appears to be one before attempting to fix it`
        - `- if you believe that the previous run did fix it then be sure to log an entry to the log file mentioning this`
    ::end-block

> **Important:**
> 
> - always write idiomatic Markdown (CommonMark + GFM)
> - if there is a good reason to create an illustration, represent the diagram with a [Mermaid](https://mermaid.js.org/intro/) code block in your Markdown
> - use indentation of 8 spaces
> - be sure to use the Markdown H2/H3 headings recommended so that the document has the right structure

::block when="ready"
The review was marked as being **production ready** so there is no longer a need to continue the review-to-implement loop.

- add the following entries to the log file ({{log}}):
    - `- the review found in '{{review}}' indicated that the specification is **production ready**!`
    - `- the specification file used to define the functional/non-functional target of all this work can be found at '{{spec}}'`
- there are times in which a review can mark the work as "production ready" but still have some findings that should be considered as follow on work
    - when this is the case we need to document this in the log file:
        - `- while the review found this feature to be production ready, it did have findings worth looking at for follow on work:`
        - the as a nested list add the "name" of each findings
        - `- refer to the review file -- {{review}} -- for more details`

Now explain to the user the current state of this review/implement cycle and exit.
::end-block


::block when="!ready"

From this point on in the task be sure to log progress on the task:

- indicate when you're _starting_ the work on a review finding: `- starting the work on '{task-name}' at {HH:MM:SS-local-time}`
    - add sub-bullets for things you've discovered, completed, or were blocked on while working on the task
- indicate when you've _completed_ the work on a review finding: `- work completed for '{task-name}' at {HH:MM:SS-local-time}`

> **Note:** 
> 
> - when employing subagents to do work always inform them about the log file and ask them to update as they do their work
> - the only exception to asking the subagents to own the logging is if you are running multiple subagents in parallel:
>    - in this case you should ask the subagents to report back to you as the orchestrator when they have a log item to report
>    - this allows you to report back to the caller in a coherent way by grouping the log messages by the various subagents
>    - Note: this does mean you'll need to accumulate the list of log items of any given subagent until it completes so you can log that subagent's log items as one group.

Now your task is to:

1. Act as an orchestrator and iterate over each suggestion (serially)
2. For each suggestion call a subagent to:
    - implement the suggestion,
    - add and/or update tests to provide full test coverage for the suggestion,
    - and make sure that the implementation passes all tests (just test)
    - and has no lints (just lint)
    - tell the subagent to use the 'rust', 'rust-testing', and '{{ctx.area}}' agent skills

    > Note: 
    > 
    > - Run testing and linting _only_ in the package areas which are directly impacted by the spec file ({{spec}})
    > - often, but not always, the spec file will explicitly state the 

3. Write the closing log entries to the log file ({{log}}):

    - add a H3 heading of `### Successful Completion\n`
    - then the following prose: `The implementation of review cycle {{iteration}} has completed successfully in {duration}. During this implementation all {#} review findings were evaluated to see if they could be fixed as a part of this implementation cycle: {fixed} were fixed, {deferred} were deferred (see reasons below):`
        - `{duration}` is the duration that the task took to complete
        - `{#}` is the number of findings/suggestions which the review contained
        - now list each finding/suggestion that had to be deferred and describe WHY it was deferred

            > NOTE: the most common reason for deferring a suggestion is that a performance metric was required but the machine's CPU load did not allow for a legitimate measurement to take place. In this case we should log similarly how we would in any other situation but take two additional measures:
            - set the `deferred_perf_measurement` to `true` on the log file's frontmatter
            - create (or append to if already exists) the '{{perf}}' file full detail of the deferred performance test and be sure to indicate which finding this maps back to as well as the review file ({{review}})

    - `The files `

4. When all suggestions have been implemented and you've reported your final log entries, you need to update metadata for the following files:

    - update the review file's metadata ({{review}}):
        - set the `log` frontmatter to '{{log}}'
        - set the `implemented` frontmatter to `true`
        - set the `implemented_by` frontmatter to `{{ ctx.agent }}/{{ ctx.model }}`
    - update the log file's metadata ({{log}}):
        - set the `implementation_{{iteration}}` frontmatter to "{{ ctx.now }}"

::end-block
