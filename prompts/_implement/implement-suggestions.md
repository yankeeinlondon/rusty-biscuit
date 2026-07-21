---
$schema:
    spec: file(required; match(**/*spec*.md))
    design: file(match(**/*design*.md))
    iteration: number(required;default(1))
description: |-
    Implements the findings in a review which was conducted to determine the drift between
    what was actually implemented versus the specification that was being targetted in that
    implementation.
iteration: "{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') || 1 : 1 }}"
review: "{{ dirname(spec) + '/' + 'review-' + iteration + '.md' }}"
# is the review "production ready"?
ready: "{{ review && file_exists(review) ? frontmatter(review, 'ready') : null }}"

design: "{{ file_exists(dirname(review) + '/design.md') ? dirname(review) + '/design.md' : null }}"

feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"
initialize: 
    stack:
        - when: ready
          action:
              - message: "review implementation for `{{parent_dir(review)}}` in _{{ctx.area}}_ not necessary; already production ready"
              - stop

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

::block when="ready"
The review was marked as being **production ready** so there is no longer a need to continue the review-to-implement loop.

Explain this to the caller and then exit.
::end-block
::block when="!ready"
The review file above has completed in the {{ctx.area}} package area with a number of suggestions for implementation.

Your task is to:

1. Act as an orchestrator and iterate over each suggestion (serially)
2. For each suggestion call a subagent to:
    - implement the suggestion,
    - add tests to provide full test coverage for the suggestion,
    - and make sure that the implementation passes all tests (just test)
    - and has no lints (just lint)
    - tell the subagent to use the 'rust', 'rust-testing', and '{{ctx.area}}' agent skills
3. When all suggestions have been implemented, set the `implemented` frontmatter property to `true` on the review file: {{review}}

::end-block
