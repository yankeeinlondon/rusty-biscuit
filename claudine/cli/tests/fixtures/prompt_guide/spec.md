---
created: 2026-09-20
status: draft-spec
clarified: false
reviewed: true
reviewed_by: codex/gpt-6-astra
reviewed_on: 2026-09-20
review_iterations: 1
needs_rulings: false
implemented: false
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
# Finding ids (F1–F8, D1–D2) whose fix has landed, wherever it landed. `prompts/_prompt.md`
# reads this list and stops warning agents about a defect once its id is here (R10).
# Keep the key present even when empty: the guide's gates do not tolerate a missing list.
fixed:
    - F1
    - F2
    - F3
    - F4
    - F5
    - F6
    - F7
    - F8
    - D1
    - D2
area: claudine
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-cli
    - dmls
    - biscuit-terminal
related:
    - 2026-07-13-proxy-with
    - 2026-09-15-initialize-after-proxy
    - 2026-09-17-remove-strict-mode
---

# Lifecycle Handoff Gaps Found by the PR Flow

A snapshot of the real specification's frontmatter, taken when the prompt guide started reading it by directory identity. Tests edit temporary copies of this file, one change per case, and never the specification itself.
