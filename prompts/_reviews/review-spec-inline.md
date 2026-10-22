---
$schema:
    spec: file(required;eager; match(**/*spec*.md)) -> the specification file being reviewed
description: Reviews a draft specification inline and updates the spec file

basename: "{{ basename(spec) }}"
dir: "{{ dirname(spec) }}"

start:
    message: "👀 reviewing the specification file: `{{spec}}`"
success:
    - say: |-
        {{
        ctx.area
            ? "The review of the draft specification file in " + ctx.area + " has completed"
            : "The review of the draft specification file in the " + ctx.repo + " repo has completed"
        }}
    - message: "✅  review of the draft specification {{ code_link(spec) }} has completed"
    - info: the {{link(spec)}} _specification_ has been reviewed and updated inline
    - when: "frontmatter(spec, 'human_review') == true"
      then:
          - warn: |-
                the {{link(spec)}} _specification_ has decisions that need the author before it can be planned:

                {{ as_ordered_list(frontmatter(spec, 'human_review_items') || []) }}

                Settle them (for example with the **clarify** prompt), then set the spec's `human_review` to `false`.
          - message: "🤷  `{{parent_dir(spec)}}` has **{{ length(frontmatter(spec, 'human_review_items') || []) }}** decision(s) that need the author before it can be planned; see `human_review_items` in its spec"
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

A reviewed spec is ready to plan: it has no open questions, pending rulings, or
"to be decided" items. Your review closes them.

::file ../_decisions.md

Look for how this spec file could be improved:

- what feels like a gap in the scope of this specification
    - fill in the gaps: make a clear decision and record it in the spec's `## Design Decisions` section
    - never shrink the scope to avoid a decision; the spec's requested scope is fixed
- resolve every open item the spec already carries
    - an "Open Questions", "Rulings needed", "TBD", or similar section with a recommendation: adopt the recommendation, move it into `## Design Decisions`, and delete the open item
    - an open item without a recommendation: decide it yourself, following "Design Decisions" above
    - when you finish, the spec has no "Open Questions" section and no item that waits on the author
- fix mistakes the spec is making relative to other contracts or standards that are already established in the repo
    - make sure to distinguish "intended" changes to standards versus accidental
    - **intended** changes obviously should not be "fixed" but if they are incomplete in describing their impact then you must choose between:
        - document the side-effects and then add to the spec how these side-effects should be mitigated
        - if you feel there is no easy way to mitigate these side-effects or that the mitigation will cause performance issues or force non-ergonomic solutions going forward then you should:
            - find a better solution that addresses the same design goals but which has an acceptable set of side effects
            - be sure to still includes addressing these side-effects as part of the spec
            - and explain the change as a readers note to indicates the design solution to a reader so they understand why the changes was made
- update with better wording if you think ideas are expressed unclearly
- keep performance measurement proportional to the risk
    - a performance spike is one host and a quick sample, unless the spec names a known cost that is specific to one OS
    - if the spec's own figures already answer a spike's question, remove the spike rather than tightening it
    - never add hosts, workloads, repetitions, or statistical thresholds to a spike on your own initiative, and do not leave a wider measurement as a question for the author

### Decisions That Need the Author

Only a decision that meets the bar in "When a human is needed" above stays
open. For each one:

- describe it in the spec under `## Decisions Needing the Author`, with the options, their pros and cons, and your recommendation
- add it to the spec's `human_review_items` frontmatter list (each item a YAML `|-` block, written without repo jargon)

Planning and implementation refuse to start while such an item is open, so the
author settles it before any code is written. Keep the list empty unless an
item truly meets the bar.

### Update Spec Frontmatter

Update the spec file's ({{spec}}) Frontmatter (keep any other properties that were set unchanged):


::file ../_set_spec_schema.md
- set the spec file's `reviewed` Frontmatter property to `true`
- set the spec file's `reviewed_by` Frontmatter property to "{{ctx.agent}}/{{ctx.model}}"
- set the spec file's `reviewed_on` Frontmatter property to "{{ctx.today}}"
- set the spec file's `review_iterations` to `0`
- set the spec file's `human_review` to `true` when "Decisions That Need the Author" left any item open, otherwise to `false` and remove any `human_review_items`
- set the spec file's `needs_rulings` to `false` (or remove it) when no item is open
