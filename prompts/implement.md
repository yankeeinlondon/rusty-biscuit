---
$schema:
    - spec: file(required;eager;match(**/*spec*.md)) -> the specification file as reference
    - review: file(required;eager;match(**/*review*.md)) -> the review file
    - plan: file(required;eager;match(**/*plan*.md))
name: Implementation Router
description: |-
    This is a _dynamic_ prompt which will route the inputs it receives to the property prompt template:

    1. If you pass in a `spec` then an existing unimplemented review takes precedence over the original plan. Otherwise it will check the spec's `implemented` flag:
        - if the spec has already been implemented this indicates we're now in a review-to-implement cycle and we'll assume that the intention is to implement the _suggestions_/_findings_ found in the review
        - if the spec has NOT yet been implemented and has no unimplemented review then we will route to a prompt that will implement it
    2. If you pass in a `review` then we will assume that we are in a review-to-implement cycle that originated not from a spec but a review
    3. If you pass in a `plan` then:
        - we will investigate if there is a associated spec attached to that plan
        - if there is no associated spec then we will look for an associated review
        - if neither spec or review's are found nearby we'll implement the plan without linked/associated content

pending_review: |-
    {{ 
        spec 
            ? replace(spec, basename(spec), 'review-' + (frontmatter(spec, 'review_iterations') || 1) + '.md') 
            : null 
    }}

initialize: 
    stack:
        - when: "spec && pending_review && file_exists(pending_review) && !frontmatter(pending_review, 'implemented')"
          action:
            - info: |-
                found the specification review that needs implementation: {{link(pending_review)}}. Will route to the **implement-suggestions** prompt for completion
            - ensure_file: "{{ dirname(spec) + '/implementation-log.md' }}"
            - action: proxy
              target: ./_implement/implement-suggestions.md
              with:
                  review: "{{ pending_review }}"
                  iteration: "{{ file_index(pending_review) }}"
                  log: "{{ dirname(spec) + '/implementation-log.md' }}"
        # - when: "spec && frontmatter(spec, 'implemented')"
        #   action: 
        #     - info: |-
        #         an _implemented_ spec file was passed into the **implementation** router and will be routed to **implement-suggestions** with the assumption that we are in a _review-to-implement_ looping cycle currently.
        #     - proxy: ./_implement/implement-suggestions.md
        - when: "spec && !frontmatter(spec, 'implemented')"
          action:
              - info: |-
                    a _specification file_ was passed in that has not been implemented yet; it will be routed to **implement-plan** so that the spec get's implemented
              - ensure_file: "{{ dirname(spec) + '/implementation-log.md' }}"
              - action: proxy
                target: ./_implement/implement-plan.md
                with:
                    log: "{{ dirname(spec) + '/implementation-log.md' }}"
        # `|| false` is the guarded-optional form: a bare `when: review` is a
        # hard error when no review was passed in, which would crash the router
        # instead of falling through to its own routing error below.
        - when: "review || false"
          action:
              - info: "a _review_ was passed into the implementation router and will be routed to **implement-review**"
              - proxy: ./_implement/implement-review.md
        - action:
            - stderr: |-
                Couldn't determine what _type_ of implementation to route; the following files were passed in or identified:
                    
                {{ spec ? '- a specification file was identified: ' + spec : '' }}
                {{ plan ? '- a plan file was identified: ' + plan : '' }}
                {{ review ? '- a review file was identified: ' + review : ''}}
                {{ !spec && !plan && !review ? '- no spec, plan, or review file was identified!' : ''}}
            - error: |- 
                Unable to _route_ the implementation to an appropriate prompt:

                > You called the {{ link(^prompt/implement.md) }} prompt which is a _router_ prompt that looks at what has been passed into it via _parameters_ as well as files or metadata that either exists or doesn't.
                >
                > The parameters this router would expect:
                > 
                > - **spec:** {{ spec ? '✔' : '<red>⤫</red>' }}
                > - **plan:** {{ plan ? '✔' : '<red>⤫</red>' }}
                > - **review:** {{ review ? '✔' : "<red>⤫</red>"}}
                >
                > {{
                    spec 
                        ? 'Because you passed in a _specification file_, that means this prompt must distinguish between the _initial implementation_ of this spec (based on a _plan_ file) versus an implementation of some review findings in the review/fix cycle.\n\nThe spec file\'s `implemented` Frontmatter property determines which of these two phases the router sees the specification file in. Currently this is set as `' + frontmatter(spec, "implemented") || false + '`.\n\nIf the frontmatter indicates we're in the review/fix cycle then this router will try to identify the _last_ review which was completed and implement the findings found in it. If you're getting this error and are in the review/fix cycle that likely means you haven't actually run any reviews yet!'
                        : review
                            ? 'Because you passed in a review file the router assumes that unlike when a "spec" is passed in, that the original "change document" was a review that was run. Obviously the review file you passed us is a valid file path (or you wouldn't have gotten this error) and typically what we'll do at this point is to proxy to the `_implement/implement-review.md` prompt but for some reason that did not happen.'
                            : 'Because you passed in a plan file, the router must decide what this plan is **for**. It does this by seeing if there is a specification file next to the review and if it sees that then it will proxy it to the prompt for handling spec based reviews. Alternatively, if it finds a review file that this plan clearly relates to then it will proxy to a review originated prompt.'
                }}
---

This prompt should never be reached. This is a router and it's goal is to proxy execution to other prompts.
