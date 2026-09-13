- set `ready` to whether you think this feature is **ready for production** (boolean)
    - things that contribute to **readiness** include:
        - incomplete implementation of the contract that the spec file (or design file when you have one) had asked for
        - defect in the implementation that will cause problems due to flakiness or incorrect results
        - a poorly optimized solution that has a more performant variant that doesn't materially change the work effort nor increase the implementation risk
        - testing:
            - a poorly constructed test that will not adequately test what it intends to
            - a test that will become unstable over time (e.g., using source code in the repo to test versus creating a static fixture to run the test on, etc.)
            - testing gaps for functionality derived from new or changed code from the specification
                - Note: you do not need to validate tests which were external to the changes imposed by the spec, however, if you noticing anything glaring then you should list it in the finding section (be sure to be clear that this is not related directly to the spec)
    - things that should NOT contribute to **readiness** include:
        - Evidence/Proof of cross-os results (this will be done as part of CI/CD)
        - Need for a Human Review
            - this may indeed be a gate on this functionality before it can be released, but
            - we will treat human based reviews as being an external process to this review that will happen _after_ we've completed the review/fix cycle
- if `ready` was set to `false` then you will need to set the `findings` frontmatter property:
    - the prose/body of the review should already have a set of _findings_ which are given a title along with priority before providing details about the finding
    - the `findings` frontmatter property should be a list of the titles and priority (not the details) of each finding
- set `human_review` to a boolean value to indicate whether human review is required prior to this specification being fully complete
    - the goal of this flag is to allow agents to do as much work as possible (aka, until the `ready` flag has been set to true) before they need to involve the human in review
    - you should not assume that _every_ specification requires human review; rather only those which require important design decisions, or involve activities and tests that only the human can do (versus an agent) 
    - when you've set `human_review` to `true` you must also set the `human_review_items` as a list of things you want the human to review:
        - this list should avoid any jargon and should NOT assume that the reviewer has any familiarity with this repo
        - each review item must clearly articulate what the review item is, what is expected from the user, and in cases where a design decision is being asked for providing an enumeration of possible solutions. 
        - to aid in each item having good clarity you may treat the content of each item as Markdown content and include diagrams using Mermaid if that helps in providing clarity to your audience.
        - Note: when the review item is longer than a sentence you should leverage YAML's `|-` operator to create Frontmatter properties which are more human readable. For instance:

            Do not do this:
        
            ```yaml
            human_review_items:
                - "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\n- foo\n- bar\n- baz"
            ```

            Instead write as:

            ```yaml
            human_review_items: 
                - |-
                    Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

                    - foo
                    - bar
                    - baz
            ```

            Both are precisely the same string but the second example that leverages the `|-` is far more legible.
