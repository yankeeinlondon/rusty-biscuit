---
$schema:
    - spec: file(required;eager;match(**/*spec*.md))
    - review: file(required;eager;match(**/*review*.md))
    - plan: file(required;eager;match(**/*plan*.md))
name: Implementation Router
description: |-
    This is a _dynamic_ prompt which will route the inputs it receives to the property prompt template:

    1. If you pass in a `spec` then it will check the spec's `implemented` flag:
        - if the spec has already been implemented this indicates we're now in a review-to-implement cycle and we'll assume that the intention is to implement the _suggestions_/_findings_ found in the review
        - if the spec has NOT yet been implemented then we will route to a prompt that will implement it
    2. If you pass in a `review` then we will assume that we are in a review-to-implement cycle that originated not from a spec but a review
    3. If you pass in a `plan` then:
        - we will investigate if there is a associated spec attached to that plan
        - if there is no associated spec then we will look for an associated review
        - if neither spec or review's are found nearby we'll implement the plan without linked/associated content

initialize: 
    stack:
        - when: "spec && frontmatter(spec, 'implemented')"
          action: 
            - info: an _implemented_ spec file was passed into the **implementation** router and will be routed to **implement-suggestions** with the assumption that we are in a _review-to-implement_ looping cycle currently.
            - proxy: ./_implement/implement-suggestions.md
        - when: "spec && !frontmatter(spec, 'implemented')"
          action:
              - info: a _specification file_ was pass in that has **not** been implemented yet; it will be routed to **implement-plan** so that the spec get's implemented
              - proxy: ./_implement/implement-plan.md
        - when: review
          action:
              - info: "a _review_ was passed into the implementation router and will be routed to **implement-review**"
              - proxy: ./_implement/implement-review.md
        - action:
            - error: "Unable to route the implementation to an appropriate prompt"
---

This prompt should never be reached. This is a router and it's goal is to proxy execution to other prompts.
