- set `ready` to whether you think this feature is **ready for production** (boolean)
    - `ready` is `false` exactly when the review has at least one finding of any priority, high, medium, or low (see "What blocks readiness")
    - things that contribute to **readiness** include:
        - incomplete implementation of the contract that the spec file (or design file when you have one) had asked for
        - defect in the implementation that will cause problems due to flakiness or incorrect results for input a real caller produces
        - a poorly optimized solution that has a more performant variant that doesn't materially change the work effort nor increase the implementation risk
        - testing:
            - a poorly constructed test that will not adequately test what it intends to
            - a test that will become unstable over time (e.g., using source code in the repo to test versus creating a static fixture to run the test on, etc.)
            - testing gaps for functionality derived from new or changed code from the specification
                - Note: you do not need to validate tests which were external to the changes imposed by the spec; anything glaring you notice there is still a finding
    - things that should NOT contribute to **readiness** include:
        - Evidence/Proof of cross-os results (this will be done as part of CI/CD)
        - Need for a Human Review
            - this may indeed be a gate on this functionality before it can be released, but
            - we will treat human based reviews as being an external process to this review that will happen _after_ we've completed the review/fix cycle
        - a guarantee stronger than the spec states, including one an earlier repair cycle wrote into a `docs/` page; when such a page overpromises, the finding is to narrow the page
        - inputs only a contrived or adversarial test can produce
- set the `findings` frontmatter property to every finding in the review, high, medium, and low, whether or not `ready` is `true`:
    - write each entry as `<ID> (<priority>): <title>`, for example `R2-F3 (medium): absent cap reads as zero`; no details
    - the repair step reads this list: it fixes high findings first, then medium and low ones, and the cycle continues until the list is empty, so a finding missing from it is a finding nobody repairs
- set `human_review` to a boolean value to indicate whether the author should look at something **after** the review/fix cycle ends
    - this list never stops the cycle and never blocks a finding: it is where decisions the agents made on their own are reported so the author can confirm or reverse them later
    - a decision that must be made **before** a finding can be repaired goes in `human_in_the_loop` instead, and only when it meets the "Stopping for a human" test
    - when you've set `human_review` to `true` you must also set the `human_review_items` as a list of things you want the human to review:
        - set `has_blocked_findings` to `true` when any finding waits on a `human_in_the_loop` item, and `blocked` to `true` when every finding does; a finding is never blocked by a `human_review_items` entry
        - this list should avoid any jargon and should NOT assume that the reviewer has any familiarity with this repo
        - if a review has made "decisions" they do not need to be reaffirmed again during this cycle unless something in the decision has changed enough that the review has changed. In these cases you must describe the change, why it was changed, and rather than referring to an obscure decision reference provide the actual decision text itself.
        - each review item must clearly articulate what the review item is, what is expected from the user, and in cases where a design decision is being asked for, providing an enumeration of possible solutions (as a markdown list). 
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
