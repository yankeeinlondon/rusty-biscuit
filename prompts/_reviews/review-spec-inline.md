---
$schema:
    spec: file(required;eager; match(**/*spec*.md)) -> the specification file being reviewed
description: Reviews a draft specification inline and updates the spec file

basename: "{{ basename(spec) }}"
dir: "{{ dirname(spec) }}"

start:
    message: "👀 reviewing the specification file: `{{spec}}`"
success:
    say: |-
        {{
        ctx.area
            ? "The review of the draft specification file in " + ctx.area + " has completed"
            : "The review of the draft specification file in the " + ctx.repo + " repo has completed"
        }}
    message: "✅  review of the draft specification `{{ link(spec) }}` has completed"
    info: the {{link(spec)}} _specification_ has been reviewed and updated inline
failure:
    say: |-
        {{
        ctx.area
            ? "The inline review of the draft specification " + title_case(without_date(parent_dir(spec))) + " in the " + ctx.area + " package area failed to complete!"
            : "The inline review " + title_case(without_date(parent_dir(spec))) + " in the " + ctx.repo + " repo failed to complete!"
        }}
    message: "💥  failed to complete the inline review of `{{parent_dir(spec)}}` spec in **{{ctx.area}}**!"
---
## Context

### Spec Review

You are expected to review a draft specification document located at: 

- {{spec}}

This will be an "inline review" meaning your task is to directly update the specification document with your changes (versus creating a sidecar review document).

::file ../_writing-clearly.md 

::block when='frontmatter(spec, "parent") || frontmatter(spec, "depends-on") || frontmatter(spec, "peers")'
### Spec Frontmatter

> **Important:** 
> 
::block when='frontmatter(spec, "parent")'
> - the spec you're reviewing includes a `parent` frontmatter property, this indicates that the spec you are reviewing is a sibling to the parent specification
::end-block
::block when='frontmatter(spec, "peers")'
> - the spec you're reviewing includes a `peers` frontmatter property, this indicates that the spec you are reviewing is part of a group of specs are related and together are designed to address a larger design goal
::end-block
::block when='frontmatter(spec, "depends-on")'
> - the spec you're reviewing includes a `depends_on` frontmatter property, this indicates that the spec you are reviewing 
::end-block

It's important to note that Frontmatter references to other specs are likely not a full filepath reference as that is _unstable_ in this repo:

- when a specification is completed it's location is _moved_ to the `_completed` folder
- that means you might have a full path reference, an old full path reference (e.g., before it was moved to `_completed`), or ideally what you have is the specifications identifier only (e.g., something like `{date}-{description}`)
- specification files are always located under a package area's "fixes" or "features" directory

::end-block

## Task

Look for how this spec file could be improved:

- what feels like a gap in the scope of this specification
    - fill in the gaps as much as you can
    - ideally make a clear decision and document the design decision 
    - however, if you feel this gap represents a major design decision and should be reviewed before being implemented
        - add the gap in the "Open Questions" sections
        - add 2-3 suggested solutions under the gaps description, each suggested solution should list out that design's pros and cons
        - choose the solution you think is MOST appropriate and recommend; include WHY you recommend this
- fix mistakes the spec is making relative to other contracts or standards that are already established in the repo
    - make sure to distinguish "intended" changes to standards versus accidental
    - **intended** changes obviously should not be "fixed" but if they are incomplete in describing their impact then you must choose between:
        - document the side-effects and then add to the spec how these side-effects should be mitigated
        - if you feel there is no easy way to mitigate these side-effects or that the mitigation will cause performance issues or force non-ergonomic solutions going forward then you should:
            - find a better solution that addresses the same design goals but which has an acceptable set of side effects
            - be sure to still includes addressing these side-effects as part of the spec
            - and explain the change as a readers note to indicates the design solution to a reader so they understand why the changes was made
- update with better wording if you think ideas are expressed unclearly

### Update Spec Frontmatter

Update the spec file's ({{spec}}) Frontmatter (keep any other properties that were set unchanged):


::file ../_set_spec_schema.md
- set the spec file's `reviewed` Frontmatter property to `true`
- set the spec file's `reviewed_by` Frontmatter property to "{{ctx.agent}}/{{ctx.model}}"
- set the spec file's `reviewed_on` Frontmatter property to "{{ctx.today}}"
- set the spec file's `review_iterations` to `0`
