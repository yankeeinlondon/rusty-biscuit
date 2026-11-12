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
            - any slow test (over 5 s, or 10 s or longer in a run on a loaded host) in the implementer's recorded runs or your own: each is a finding, and an implementation record that reports such a run as passing is wrong (see "A Slow Test Is a Failing Test")
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
    - this list never stops the cycle and never blocks a finding, but every item interrupts the author, so it is **not** a log of the decisions the agents made; those belong in the review body
    - an item belongs in `human_review_items` only when it passes the **worth the author's time** test: it is a decision the agents made on their own that **either** meets one of the conditions under "Stopping for a human" (hard to reverse, contradicts a decision, changes scope, safety or trust, no safe default, human-only verification) but was settled with a recommendation, **or** you genuinely doubt and would not simply recommend keeping
    - these never pass the test:
        - a decision you recommend keeping, or anything whose answer would be "keep it" or "go with your recommendation"
        - something a repository rule already decides, such as leaving cross-OS evidence to CI, not reformatting unrelated code, the choice of test recipe or verification scope, or omitting tests of unchanged packages
        - a choice that is cheap to change later, such as internal structure, a private helper or macro, test layout, or the phase in which work landed
        - a factual error in the implementation log or implementation record; correct it instead (see "Correcting the implementation record")
    - write at most **3** items, ranked by the cost of the author reversing them; when no item passes the test, set `human_review` to `false` and omit `human_review_items`, even when the review body lists many informational decisions
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
