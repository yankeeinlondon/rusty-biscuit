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
    - stderr: "The {{code_link(plan)}} _plan_ has been created"
    - message: "✅  the _plan_ for the spec `{{parent_dir(underlying)}}` was created (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})"
    - when: "file_exists(plan) && !is_empty(frontmatter(plan, 'open_questions'))"
      then:
          - warn: |-
                the plan has **{{ length(frontmatter(plan, 'open_questions')) }}** open question(s) that must be resolved in `{{ underlying }}` before it can be implemented:

                {{ as_ordered_list(frontmatter(plan, 'open_questions')) }}
failure:
    message: "❌️  failed to create a _plan_ for `{{parent_dir(underlying)}}` (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})!"
---

## Context

You are a planning agent with decades of experience building high quality technical project plans.

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
    - draft the first H2 section of the 
- Break work into **phases** and **tasks**
    - the first phase should always be phase 1
    - a task is a unit of work associated to a _phase_
        - each task should be given a short name (2-3 words) and then one or more bullet points describing the task as clearly as possible
        - if a task has any pre-requisites, complexities, 
    - if there is an opportunity to run multiple **tasks** concurrently then this should be represented as a **step** 
- Each phase of the plan should be represented as an H2 heading the planning document
- Once you have a first pass of the Phases and Tasks associated with each phase
- Resolve decisions now, not during implementation. The implementation runs unattended, phase after phase, and every question it has to put to the author stops it. Key decisions belong to the {{ground_truth}} and its review; the plan settles what remains:
    - **already decided:** a question the {{ground_truth}} answers -- including a stated fallback such as "when X is not available, do Y" -- is not open. Follow it and cite the section; never ask the author to "confirm" it
    - **yours to decide:** a question the {{ground_truth}} leaves open, but whose goals and constraints make one answer reasonable, is decided by you. Record it under an H2 `## Planning Decisions` section placed before Phase 1: the decision, the alternatives, and why, with a reference to the part of the {{ground_truth}} it rests on. The implementer follows these decisions and the author can overrule any of them at review, so phrase each one as decided, never as "confirm or amend"
    - **escalation triggers:** where the right course depends on evidence the implementation will produce (for example, a comparison that may come out mixed), decide the course for each clear outcome and name the specific outcome that must stop for the author. List these under `## Planning Decisions` as well
    - **open questions:** only a question you cannot reasonably decide -- it would change scope, contradict the {{ground_truth}}, choose between outcomes the {{ground_truth}} does not rank, or needs information only the author has -- is left open. Do not schedule it into Phase 1. Add it to the plan's `open_questions` frontmatter (a list; each item uses YAML's `|-` and states the question, the options, and your recommendation) and describe it under `## Planning Decisions`. A plan with open questions cannot be implemented until the author resolves them in the {{ground_truth}}, so expect few or none
::block when="spec"
- After the decisions we need to consider if there are "spikes" that should be run to lower the risk for this implementation or to better inform the specification and/or design
    - a spike runs once, before the work it informs; do not schedule it again as a re-check in later phases
    - a performance spike is one host and a quick sample, unless the spec names a known cost that is specific to one OS
    - if the spec's own figures already answer a spike's question, record that under `## Planning Decisions` and drop the spike
    - do not widen a spike beyond what the spec asks for; record any wider measurement you think is worthwhile under `## Planning Decisions` as out of scope, so the author can add it at review
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

- Save the plan as "{{plan}}"



- Add frontmatter to the plan document and set:
    - `total_phases` property to the number of phases defined in this plan
    - `created` add the date in YYYY-MM-DD format
    - `phase` set this to the starting phase number; usually 1 but may be 0 sometimes
    - `agent` set this to "{{ ctx.agent }}/{{ ctx.model }}"
    - `yolo` set this to "{{ env.YOLO }}"
    - `open_questions` only when questions remain open (see above); omit it otherwise
