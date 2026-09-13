---
description: |-
    This prompt is used to learn from a long review/fix cycle and see if there are ways in which the prompts, agent skills, or other repo or environmental factors could be improved to make
    this cycle more efficient going forward.
$schema:
    spec: file(required;eager;match(**/*spec*.md)) -> the specification file that represented the core requirements for this elongated review/fix cycle
    iterations: number -> The number of iterations that it took to complete this cycle (_or at least to get to the point where a lineage review felt necessary_).
    finished: boolean -> a boolean flag indicating whether this review was run after a long `review/fix` cycle that _finished_ or one that is still ongoing!
    count: number -> the number of interations taken in the `review/fix` cycle
spec_name: |-
    {{ 
        ctx.current_package_area
            ? ctx.current_package_area + '/' + parent_dir(spec) 
            : ctx.current_package
            ? ctx.current_package + "/" + parent_dir(spec)
            : parent_dir(spec)
    }}
working_dir: "{{ parent_dir(spec) }}"
last_review: "{{ find_last_index( working_dir + 'review.md') }}"
count: "{{ length(file_index(last_review)) }}"
iterations: |-
    {{
    
    }}
finished: |-
    {{
        frontmatter(last_review, 'implemented')
    }}
spec_context: |-
    {{
        finished
            ? 'The `' + spec_name + '` specification went through **' + count + '** review/fix cycles before it was finished.'
            : 'The `' + spec_name + '` specification has gone through **' + count + '** review/fix cycles before and is still **not** finished!'
    }}
---
# Lineage Review

You are being asked to run a "lineage review" of the `{{ spec_name }}` based around the '{{spec}}' specification file. A lineage review looks at the _elongated_ time that this specification took to complete and tries to find ways in which future cycles can be made more efficient.

## Repo Context

::file ./_repo-context.md

## Review/Fix Cycle Context

{{spec_context}} We typically would like the review/fix cycle to take no more than 4 cycles (_but less is always better_). Things to know about this review/fix cycle include:

- once a specification has been finalized we create a plan with the `@prompts/plan.md` prompt
- that plan is then implemented -- _phase by phase_ -- using the `@prompts/_implement/implement-plan.md` prompt
- once in the review/fix cycle:
    - we use the `@prompts/_review/feature-review.md` file as the _prompt_ for the review
    - we use the `@prompts/_implement/implement-suggestions.md` file as the _prompt_ for implementing the findings in the review
- an ideal review/fix cycle should try to _fix_ as many code related problems as possible before engaging the caller/human for review of any "human-in-loop" decisions which may be required.
    - the fixer agent _should_ allow themselves some latitude to make design calls where it feels like there is a clear decision that can be made (note: always align on the side of a better solution than a quicker solution) and the impact of the design call is not too large
    - any decisions an agent makes must always be documented (first in the prose/body of the implementation log, but also as structured data in the Frontmatter)
    - documentation of a decision in the body/prose includes what alternative solutions were considered and why the chosen solution was deemed best
    - the structured data captured in Frontmatter should be written clearly enough that a person with no context of the repo could understand the decision made clearly; they don't need to know the decision in detail but the summary description should be clear and jargon free
- by getting non-blocking coding fixes out of the way first, we hopefully can engage in a single human-in-the-loop cycle (or none if there is not a need) to get to a "production ready" status. This zero or one human-in-the-loop goal makes the overall process much more efficient
- decisions which are significant or deemed to need human input must be first documented in prose but then also cataloged as structured data in the Frontmatter; the prompts involved in this process need to enforce this in a clear and strongly worded way (ideally leveraging )


## Agent Skills

- use the 'darkmatter' agent skill so that you understand everything you need to know about _composition_ (note: all prompts are "composed" before being handed to the agent)
- use the 'claudine' agent skill so you understand the lifecycle hooks which it supports and so you can offer useful advice on how to better communicate progress as well as enforce compliance
::file ./_agent-skills.md --spec=spec
