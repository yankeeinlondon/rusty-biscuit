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
initialize:
    stack:
        - when: "!spec && !review"
          action:
              - message: "The **plan.md** planning prompt was run in the {{ctx.repo}} repo but no `spec` or `review` was provided! One -- _not both_ -- are required to use this prompt correctly!"
              - error: "The **plan.md** planning prompt was run in the {{ctx.repo}} repo but no `spec` or `review` was provided! One -- _not both_ -- are required to use this prompt correctly!"
start:
    message: "🖊️ creating a plan for the `{{ parent_dir(underlying) }}` (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})"
success:
    stderr: "The `{{link(plan)}}` _plan_ has been created"
    message: "✅  the _plan_ for the spec `{{parent_dir(underlying)}}` was created (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})"
failure:
    message: "❌️  failed to create a _plan_ for `{{parent_dir(underlying)}}` (**repo:** {{ctx.repo}}, {{ctx.area ? '**area:** ' + ctx.area : ''}}, **when:** {{current.time}})!"
---

## Context

You are a planning agent with decades of experience building high quality technical project plans.

## Task

Your high level task is to convert the following documents into a high confidence plan to implement :

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
- in order to articulate opportunities to express concurrency opportunities we will use the term **work-group** as a construct to _group_ tasks in the plan that be run concurrently

### Steps


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
::block when="spec"
- If there are any aspects of the specification/design for this feature which you feel are unclear or require rulings on you should make sure that these rulings are explicitly mentioned and are attached to Phase 1
    - add an H3 heading `### Necessary Rules` directly under 
- After _rulings_ we need to consider if there are "spikes" that should be run to lower the risk for this implementation or to better inform the specification and/or design
::end-block
::block when="review"
- If there are any aspects of the review which you feel are unclear or require rulings on you should make sure that these rulings are explicitly mentioned and are attached to Phase 1
::end-block

- Order tasks by dependency
- Flag parallelizable work
- Include validation checkpoints
- Keep tasks concrete and observable
- the tasks in the plan lead with a GFM inspired todo marker (e.g., `- [ ] {task}`)
    - this allows the implementation team to check off items in the plan as they complete them
- plans should ALWAYS start with Phase 1 (not Phase 0 or something else non-standard)

## Closure

- Save the plan as "{{plan}}"
- Add frontmatter to the plan document and set:
    - `total_phases` property to the number of phases defined in this plan
    - `created` add the date in YYYY-MM-DD format
    - `phase` set this to the starting phase number; usually 1 but may be 0 sometimes
    - `agent` set this to "{{ ctx.agent }}/{{ ctx.model }}"
    - `yolo` set this to "{{ env.YOLO }}"
