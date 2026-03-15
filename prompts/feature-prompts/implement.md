### Implement the Plan

You must act as an **Orchestrator** while implementing the plan found in `{{base_dir}}/plan.md`.

Your job is to coordinate subagents — you should NOT write implementation code yourself. Read the plan, identify phases, and delegate.

#### Step 1: Read and Understand the Plan

1. Read `{{base_dir}}/plan.md` in full
2. Identify whether the plan has multiple phases or is single-phase
3. Note inter-phase dependencies (most phases are sequential)

#### Step 2: Execute Phases

For each phase in the plan:

1. **Print a status line** to the caller: `Starting Phase N: <phase title>`
2. **Spawn a single subagent** for that phase with a prompt that includes:
    - The phase title and number
    - **Only the tasks for that phase** — copy/paste the relevant section from the plan, do NOT send the entire plan
    - The feature directory: `{{base_dir}}`
    - Instruction: "Implement all tasks in this phase. After each task, verify the code compiles with `cargo check -p <package>`. Report back which files you created or modified."
3. **Wait for the subagent to complete** before starting the next phase (unless the plan explicitly says phases can run in parallel)
4. **Print a status line**: `Phase N complete. Files changed: <list>`

If a phase subagent fails or reports it cannot complete a task, STOP and report the failure to the caller with the subagent's error details. Do not proceed to the next phase.

#### Step 3: Run Tests

After ALL phases are complete:

1. Run `sniff repo dirty-packages` to identify affected package areas
2. For each affected package area, **spawn a test subagent** (these CAN run in parallel) with:
    - The package area name
    - The package area root directory (run `sniff repo package-area-root <area>`)
    - Instruction: "Run `just test` in `<root_dir>`. If tests fail, read the failing test code and fix the IMPLEMENTATION (never modify tests). You have 3 attempts. Report: pass/fail, which tests failed, what you changed."
    - **Tests MUST NOT be changed.** If a test appears to be a bug in a different package, note it but do NOT skip it without strong justification.
3. Collect all test subagent results
4. If any test subagent reports failure after 3 attempts, report the details to STDERR

#### Step 4: Log and Report

1. Append a summary to `{{base_dir}}/log.md`:
    - Heading: `## Implementation of {{feature}}`
    - A few bullets summarizing what was done
    - Note any test failures or issues
2. Set the `implementation_files` frontmatter:

    ```bash
    md set "{{base_dir}}/log.md" implementation_files "${files}" --save
    ```

3. Communicate to the caller: implementation complete, test status, and any caveats
