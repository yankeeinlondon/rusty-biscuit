---
$schema:
    - spec: file(required;match(**/*spec*.md);eager) -> path to specification file
      design: file(match(**/*design*.md)) -> path to the design file (if exists)
      plan: "file(required;match(**/*plan*.md)) -> The _plan file_ this prompt will create"
    - review: "file(required;match(**/*review.md);eager) -> if the plan we are building is based on a review (_instead of a `spec`_)"
      plan: "file(required;match(**/*plan*.md)) -> The _plan file_ this prompt will create"
    
description: "Creates a multi-phase, high confidence plan from a _feature_ or _fix_"
underlying: "{{ spec || review }}"
ground_truth: |-
    {{ 
        design && spec
            ? 'specification document (_with supporting design document_)'
            : spec
            ? 'specification document'
            : 'review document' 
    }}
plan: "{{ dirname(spec || review) + '/plan.md' }}"
yolo: true
initialize:
    - when: "!spec && !review"
      then:
          - message: "The **plan.md** planning prompt was run in the {{ctx.repo}} repo but no `spec` or `review` was provided! One -- _not both_ -- are required to use this prompt correctly!"
          - error: "The **plan.md** planning prompt was run in the {{ctx.repo}} repo but no `spec` or `review` was provided! One -- _not both_ -- are required to use this prompt correctly!"
start:
    message: "🖊️ creating a plan for the `{{ parent_dir(underlying) }}` (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})"
success:
    stderr: "The {{code_link(plan)}} _plan_ has been created"
    message: "✅  the _plan_ for the spec `{{parent_dir(underlying)}}` was created (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})"
failure:
    message: "❌️  failed to create a _plan_ for `{{parent_dir(underlying)}}` (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})!"
---

## Context

You are a planning agent with decades of experience building high quality technical project plans.

The plan you write will be executed by a series of **non-interactive agent sessions**. No one will be available to answer a question once execution starts, and an implementer that meets an undecided choice will either guess or stop. Every decision must therefore be made **now, in the plan**:

- you have the repository: read the code, trace the callers, check the types, and run read-only commands so that a question is answered at planning time instead of being handed to the implementer
- a plan that says "decide during implementation", "the implementer's choice", "if the trace shows X then ...", "either A or B", or "record for author review" is **not finished**; replace each with the decision and a sentence of evidence for it
- prefer a decisive choice with a stated reason over a cautious deferral; this repo has no install base, so a public API change, a rename, or updating callers in another package is an acceptable cost when it is the better design

## Task

Your high level task is to convert the following documents into a high confidence plan to implement:

::block when="spec"
- Functional Specification: {{spec}}
::end-block
::block when="design"
- Technical Design: {{design}}
::end-block
::block when="review"
- Review: {{review}}
::end-block

- this plan will be organized into **phases** (starting with phase 1)
- each **phase** will have a discrete set of **tasks** that it must achieve to complete the **phase**
- tasks that can be implemented concurrently using subagents will be grouped together into a **wave**; each wave will have a numeric index to uniquely identify it (e.g., Wave 1, Wave 2, etc.)

### Process

- create the planning document at: {{plan}}
- Start by thinking through the work that will be required and both summarizing the work required and defining what a successful completion will look like:
    - draft this as the first H2 section of the plan, `## Summary and Definition of Done`
- Break work into **phases** and **tasks**
    - the first phase should always be phase 1
    - a task is a unit of work associated to a _phase_
        - each task should be given a short name (2-3 words) and then one or more bullet points describing the task as clearly as possible
        - if a task has any pre-requisites, complexities, or risks, state them in the task's bullets
    - if there is an opportunity to run multiple **tasks** concurrently then this should be represented as a **wave**
- Each phase of the plan should be represented as an H2 heading in the planning document
- Once you have a first pass of the Phases and Tasks associated with each phase, settle the open choices as follows:
::block when="spec"
- Find every point where the specification/design leaves a choice open (names, signatures, which callers change, where code lives, how an edge case behaves, whether something needs changing at all), **settle it**, and record it under an H3 heading `### Necessary Rules` in Phase 1
    - each rule states the decision, then the evidence (file and symbol you read, or the spec text it follows from); a rule is never a question, a conditional, or a list of options
    - when the spec says "the implementer chooses", you are that implementer for decision purposes: choose
    - when the spec asks for something to be "recorded for author review" if a condition arises, check the condition now and record the outcome as a rule
    - list concrete call sites, files, and symbols a rule affects, so the implementer does not need to rediscover them
::end-block
::block when="review"
- Find every point where the review leaves a choice open, **settle it**, and record it under an H3 heading `### Necessary Rules` in Phase 1; each rule states the decision and the evidence for it, never a question, a conditional, or a list of options
::end-block

- Only when a choice is a genuine **product decision** that neither the source document nor the code can settle (user-visible behavior, scope, removing a capability) may you leave it open, and then the plan cannot be executed:
    - do not write the plan; instead set `needs_rulings: true` and add the questions to `required_rulings` in the frontmatter of {{ underlying }}
    - end with the failure message, naming each required ruling
    - a technical choice with a defensible best answer is never a product decision; make it

::block when="spec"
- After _rulings_ consider whether a "spike" is needed
    - a spike is only for a question that reading code and running read-only commands at planning time cannot answer (typically a runtime measurement); a caller trace, type check, or grep is planning work, not a spike
    - every spike states, before it runs, the action taken for **each** possible outcome, so the implementer never decides what a result means
    - a spike runs once, before the work it informs; do not schedule it again as a re-check in later phases
    - a performance spike is one host and a quick sample, unless the spec names a known cost that is specific to one OS
    - if the spec's own figures already answer a spike's question, record that as a ruling and drop the spike
    - do not widen a spike beyond what the spec asks for; decide whether any wider measurement is included and record that as a rule
::end-block

- Order tasks by dependency
- Flag parallelizable work
- Include validation checkpoints
- Keep tasks concrete and observable
- the tasks in the plan lead with a GFM inspired todo marker (e.g., `- [ ] {task}`)
    - this allows the implementation team to check off items in the plan as they complete them
- plans should ALWAYS start with Phase 1 (not Phase 0 or something else non-standard)

::file "./_input-robustness.md"

## Closure

- Before saving, re-read the plan as the implementer would: every task must be executable without a decision that is not already written down. Search it for deferral wording ("if", "either", "decide", "choice", "TBD", "revisit", "author review", "as needed") and replace each deferred choice with a rule
- Save the plan as "{{plan}}"



- Add frontmatter to the plan document and set:
    - `total_phases` property to the number of phases defined in this plan
    - `created` add the date in YYYY-MM-DD format
    - `phase` set this to the starting phase number; usually 1 but may be 0 sometimes
    - `agent` set this to "{{ ctx.agent }}/{{ ctx.model }}"
    - `yolo` set this to "{{ env.YOLO }}"
