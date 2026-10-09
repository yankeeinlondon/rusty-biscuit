- set the spec file's `$schema` as:
    ```yaml
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
    ```
