### Implement the Plan

Implement the plan found in `{{base_dir}}/plan.md` directly. You are writing the code yourself — do not attempt to delegate to subagents.

::file @prompts/feature-prompts/error.md when="error == true"

#### Step 0: Mark Implementation as In-Progress

```bash
md set "{{base_dir}}/log.md" implement_complete false --save
```

#### Step 1: Read and Understand the Plan

1. Read `{{base_dir}}/plan.md` in full
2. Read `{{base_dir}}/tech-design.md` for architectural context
3. Read `{{base_dir}}/spec.md` for the functional requirements
4. Identify the phases and their ordering — execute them sequentially

#### Step 2: Implement Each Phase

For each phase in the plan, implement every task in order:

1. **Print a status line**: `Starting Phase N: <phase title>`
2. For each task in the phase:
    - Read any existing files you need to modify
    - Write the implementation code
    - After each task, verify the code compiles: `cargo check -p <package>`
    - If compilation fails, fix the errors before moving to the next task
3. **Print a status line**: `Phase N complete. Files changed: <list>`
4. **Append to the log file** (`{{base_dir}}/log.md`) after each phase:
    - Heading: `### Phase N: <phase title>`
    - A brief summary of what was implemented
    - List the files created or modified
    - Note any compilation issues encountered and how they were resolved

If a task cannot be completed, log what was accomplished so far, then STOP and report the failure with details. Do not proceed to the next phase.

#### Step 3: Run Tests

After ALL phases are complete:

1. Run `just test` from the package area root to run all tests
2. If tests fail, read the failing test code and fix the **implementation** (never modify tests)
3. You have 3 attempts to fix failing tests
4. If tests still fail after 3 attempts, report the details to STDERR

**Tests MUST NOT be changed.** If a test appears to be a bug in a different package, note it but do NOT skip it without strong justification.

#### Step 4: Log and Report

1. Append a final summary to `{{base_dir}}/log.md`:
    - Heading: `## Implementation of {{feature}} Complete`
    - Overall status (all phases completed, test results)
    - Note any test failures or unresolved issues
2. Set frontmatter on the log file:

    ```bash
    md set "{{base_dir}}/log.md" implementation_files "${files}" --save
    md set "{{base_dir}}/log.md" implement_complete true --save
    ```

3. Communicate to the caller: implementation complete, test status, and any caveats
