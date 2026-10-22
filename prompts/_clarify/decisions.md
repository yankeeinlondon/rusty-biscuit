---
description: |-
    Runs an interactive session with the author to settle the decisions a first
    review raised in `human_in_the_loop`, then turns every decision that changes
    the implementation into a finding the review/fix cycle repairs.
$schema:
    review: file(required; eager; match(**/review-*.md)) -> the review whose `human_in_the_loop` decisions are settled in this session
spec: "{{ replace(review, basename(review), 'spec.md') }}"
log: "{{ replace(review, basename(review), 'implementation-log.md') }}"
review_id: "R{{ file_index(review) }}"
interactive: true
initialize:
    - when: "frontmatter(review, 'human_resolved') == true || length(frontmatter(review, 'human_in_the_loop') || []) == 0"
      then:
          - error: "the review {{link(review)}} has no unresolved `human_in_the_loop` decisions to settle"
start:
    say: "The review of {{ title_case(without_date(parent_dir(review))) }} has decisions that need you"
success:
    message: "✅  the author settled the decisions raised by review #{{ file_index(review) }} of `{{parent_dir(review)}}` ({{ctx.agent}}/{{ctx.model}})"
---

## Context

The first review of an implementation found decisions that only the author can
make. They are listed in the `human_in_the_loop` frontmatter of the review
{{review}}, and explained in its body.

- the specification: {{spec}}
- the implementation log: {{log}}

The implementing agents already chose an option for each decision and built
the implementation on it. This session confirms or changes those choices.

## Task

1. Read each `human_in_the_loop` item, the review body that explains it, and the
   log's `### Design Decisions` entries and `decisions_for_author` that relate to it.
2. Show the author a short numbered summary of every item: the decision, the
   option the implementation used, and the recommendation. Then ask one
   question first: **accept every recommendation**, or **go through the items
   one at a time**. When the author accepts them all, skip to step 4.
3. For each item, one at a time:
    - explain it without jargon, for a technical reader who does not know this repository
    - offer the options (2–4) with pros and cons, mark the recommended one and say why, and always allow "other"
    - say which option the implementation currently uses
    - if the author picks "other" and the answer is unclear, ask a follow-up until it is clear
4. Record the outcome:
    - in the review, under a `## Human Rulings` heading: one entry per item with the decision and the date ({{ctx.today}})
    - in the spec, under `## Design Decisions`: the same decisions, marked as decided by the author on {{ctx.today}}
    - when a decision changes what a later spec in the same pipeline assumes (see the log's `## Pipeline Refresh`), update that spec and its plan too, and say so in the review's `## Human Rulings`
5. Turn every decision that requires the implementation to change into a
   finding the repair step will implement:
    - add a `### {{review_id}}-H1: <title>`, `### {{review_id}}-H2: …` section to the review's body stating exactly what must change
    - add each to the review's `findings` frontmatter as `{{review_id}}-H1 (high): <title>`
    - a decision that confirms the option the implementation already uses needs no finding

## Closure

Set on the review ({{review}}):

- `human_resolved`: `true`
- `findings`: the list from step 5 (an empty list when nothing must change)
- when `findings` is empty: `implemented` to `true` and `implemented_by` to "{{ctx.agent}}/{{ctx.model}}", so the cycle goes straight to the next full review

Leave `ready` as `false`: the next review compares the implementation against
the spec in full.

Finish with a short summary for the author: each decision, whether it changed
the implementation, and what `just fix` does next.
