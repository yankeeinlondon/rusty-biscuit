### Implement the Plan

You must act as an Orchestrator while implementing the plan found in "{{base_dir}}/plan.md":

- a complex plan may be multiple phases and when it is you will create a subagent for each phase (in parallel if there are no dependencies between phases, but in most cases you'll run them serially):
    - Each subagent will also act as a subagent to all of the tasks in that phase of the plan
- if the plan is not multi-phased then you will act as an orchestrator to handle the various tasks/features which need completion in the plan
- during the orchestration process you will make sure to keep the caller up-to-date on the current status
- In both cases when the Agent believes it has completed the implementation:
    - Testing
        - spawn a sub-agent for each package area which was affected (run `sniff repo dirty-packages`)
        - each sub-agent should be told:
            - which package area they are responsible for 
            - and told to run `just test` in that package area's root directory (this can be determined by running `sniff repo package-area-root`).
            - they must then iterate until all tests pass
                - tests MUST NOT BE CHANGED!
                - they are allowed to exclude any tests which they believe is a result of another package in the monorepos faults but this is UNLIKELY so the subagent should be warned to be leary of tagging a failing test for this reason
                - if after 3 attempts a failing test can't be fixed then we should report the error to STDERR and then immediately return to the orchestrator with their status.
- communicate to the caller that the implementation is implemented and all tests are passing
