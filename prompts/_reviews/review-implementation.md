---
$schema:
    review: file(required; match(**/*review*.md); eager)
    template: file(match(prompts/*.md,.claudine/prompts/*.md); eager)
    plan: file
description: Reviews how well the _implementation_ of a certain set of review findings addressed the underlying problems which were raised.

plan: "{{ basename(review) + '/' + replace(review,'review','plan') }}"

start:
    message: "🥸 reviewing the _implementation_ of the findings in `{{parent_dir(review)}}`"
success:
    say: "The review of the draft specification file has completed"
    message: "✅ review of the draft specification '{{spec}}' has completed"
---
# Review the Implementation of Review Findings

## Context

::block when="template"
The review findings found in '{{review}}' were the result of the following _review_ prompt being run:

- Prompt: {{template}}
::end-block

The review _findings_ were captured here:

- Review Findings: {{review}}

::block when="plan"
These findings were turned into an implementation plan found here:

- Implementation Plan: {{plan}}
::end-block

And the findings from the review have now been implemented in the source code.

## Skills

::block when="has_skill(ctx.area)"
- use the '{{ctx.area}}' agent skill for deep understanding of the functionality and solution approach used in the package area
    - Be aware that having just completed an implementation in this package area, there is some potential for documentation drift having taken place with this agent skill but this should be the exception rather than the rule
    - In general you can trust the information contained in this skill to be accurate and current but where you notice drift take a note of it because part of this task will be to eliminate any documentation drift that may have landed in this agent skill
::end-block
- use the 'rust', 'rust-testing', and 'rust-devops' skills
- use the 'cli' skill for knowledge about 'clap' and associated crates we use for the CLI as well as best practices and standards we adhere to in this repo

## Task

Your task is to review the _implementation_ of the review findings found in '{{review}}' and ensure that test coverage for the new implementation is strong.


- Test Coverage
    - validate that new and updated test provide good test coverage for the 

The review {{review}} had a number of findings which have now been implemented. Your task is to validate that the implementation
