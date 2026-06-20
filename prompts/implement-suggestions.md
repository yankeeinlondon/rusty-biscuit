---
$schema:
    spec: string(required)
    design: string
iteration: "{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}"
review: "{{ dirname(spec) + '/' + 'review-' + iteration + '.md' }}"
# is the review "production ready"?
ready: "{{ review && file_exists(review) ? frontmatter(review, 'ready') : null }}"

design_file: "{{ dirname(review) + '/design.md' }}"
design: "{{ file_exists(design_file) ? design_file : null }}"

feature_or_fix: "{{ contains(spec, 'fixes') ? 'fix' : 'feature' }}"

start:
    message: "🏃 starting the _implementation_ of the `{{ parent_dir(review) }}` review suggestions ({{ctx.area}}, iteration #{{ file_index(review) }}, {{ctx.agent}}/{{ctx.model}})"
success:
    message: "✅ _implementation_ of the suggestions from the #{{ file_index(review) }} review of `{{ parent_dir(review) }}`\n\n({{ctx.area}}, {{ctx.agent}}/{{ctx.model}}) completed"
    say: "the review suggestions for {{ title_case(without_date(parent_dir(review))) }} in {{ctx.area}} completed successfully"
    effect: bong
failure:
    message: "❌ the review suggestions from **{{ title_case(parent_dir(review)) }}** failed to complete!"
    effect: phase-jump-3
---
# Implement Review Suggestions for {{title_case(without_date(parent_dir(spec)))}}

> {{capitalize(feature_or_fix)}}: `{{parent_dir(review)}}`
> Iteration: {{iteration}}

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
