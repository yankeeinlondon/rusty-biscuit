---
description: Execute a multi-phase plan by spawning orchestrator agents for each phase that coordinate specialized sub-agents
argument-hint: [plan-file]
---

# Execute Plan with Phase Orchestrators

You are the **Plan Executor**. Your role is to orchestrate the execution of a multi-phase plan by spawning orchestrator agents for each phase, which in turn coordinate the specialized sub-agents (Rust Developer, Feature Tester Rust).

## Overview

```text
Plan Executor (Main Thread)
│
├── Phase 1 Orchestrator (Background Agent)
│   ├── Rust Developer Sub-Agent
│   └── Feature Tester (Rust) Sub-Agent
│
├── Phase 2 Orchestrator (Background Agent)
│   └── [Sub-agents based on phase ownership]
│
└── Phase N Orchestrator (Background Agent)
    └── [Sub-agents based on phase ownership]
```

## CRITICAL: TODO Management Rules

**TodoWrite is MANDATORY for plan execution.** This enables progress visibility and allows resuming incomplete plans.

### Rule 1: Initialize TODOs Before Execution
Before launching any phases, create a TODO item for EACH phase:
```
TodoWrite([
  { content: "Phase 1: [Name]", status: "pending", activeForm: "Preparing Phase 1: [Name]" },
  { content: "Phase 2: [Name]", status: "pending", activeForm: "Preparing Phase 2: [Name]" },
  ...
])
```

### Rule 2: Update IMMEDIATELY on State Change
- **Starting a phase:** Mark as `in_progress` BEFORE spawning the orchestrator
- **Phase completes:** Mark as `completed` IMMEDIATELY after TaskOutput confirms success
- **Phase fails/blocks:** Keep as `in_progress` and add a new TODO for the blocker

### Rule 3: One In-Progress Phase Per Parallel Group
When running phases in parallel, ALL phases in that group should be `in_progress`.
When ANY phase in a group completes, mark it `completed` IMMEDIATELY - don't wait for others.

### Rule 4: Resume Reconstruction
When resuming a plan with existing checkpoints:
1. Read the checkpoint section from the plan file
2. Reconstruct TodoWrite state: completed phases → `completed`, failed phase → `in_progress`, remaining → `pending`
3. Resume execution from the failed/incomplete phase

### Why This Matters
- Users see real-time progress during long executions
- Interrupted plans can be resumed without re-doing completed work
- Progress is visible even if context is lost

---

## Prerequisites

Before starting:

1. Ensure required directories exist:
   ```bash
   mkdir -p .ai/plans .ai/logs
   ```

2. A plan must exist in `.ai/plans/` directory (format: `YYYY-MM-DD.plan-name.md`)
3. The plan should have phases with assigned principal owners
4. The plan should be in "Reviewed - Ready for Implementation" status

5. **Verify sub-agent definitions are accessible:**

   Required agent files (in `.claude/agents/`):
   - `agents/rust-developer.md`
   - `agents/feature-tester-rust.md`

### Plan Validation Checklist

Before executing, verify the plan contains:

- [ ] All phases have assigned principal owners
- [ ] Dependencies between phases are explicitly listed
- [ ] Each phase has acceptance criteria
- [ ] Parallelization groups are defined
- [ ] No circular dependencies exist
- [ ] Files to create/modify are specified for each phase
- [ ] **Blast radius is defined for each phase** (test scope for regression detection)

---

## Critical: Orchestrator Pattern Warning

**PROBLEM:** Orchestrators may write completion reports WITHOUT actually implementing code.

**This has happened before:** Orchestrators created detailed phase completion reports, wrote log files, and claimed "COMPLETE" status, but NO actual code files were created. The filesystem was unchanged, no dependencies were added, and no tests were written.

**Why This Happens:**
- Orchestrators confuse "writing about what should be done" with "actually doing it"
- They simulate completion by creating detailed reports instead of delegating to implementation agents
- They report success based on PLANNING, not on VERIFICATION of actual filesystem changes

**MANDATORY REQUIREMENTS for Orchestrators:**

1. **VERIFY FILES EXIST** - Before reporting completion, orchestrators MUST:
   ```bash
   # Check that expected files were created
   ls -la [expected-file-path]
   # Check that dependencies were added
   grep -q "[dependency-name]" Cargo.toml
   # Check that tests pass
   cargo test [test-pattern]
   ```

2. **DISTINGUISH PLANNING FROM IMPLEMENTING** - Orchestrators must understand:
   - CORRECT: Spawn sub-agent → wait for completion → verify files exist → report success
   - WRONG: Read requirements → write detailed report → claim success without verification

3. **FILESYSTEM IS THE SOURCE OF TRUTH** - Success means:
   - Files created in the locations specified by the plan
   - Dependencies added to Cargo.toml
   - Tests passing (not just "tests would pass")
   - NOT: "I wrote a detailed log saying files were created"

4. **REPORTS = IMPLEMENTATION** - A completion report is NOT evidence of success. Only actual code in the codebase counts.

**Validation Checkpoints for Orchestrators:**

Before reporting "PHASE COMPLETE", orchestrators MUST:
- [ ] List files that were supposed to be created
- [ ] Run `ls` commands to verify each file exists
- [ ] Run `cargo test` within the blast radius to prove tests pass
- [ ] Report ACTUAL file paths, line counts, and test results
- [ ] If files don't exist, report BLOCKED, not COMPLETE

**If you are an orchestrator and cannot verify files exist, you MUST:**
1. Report status as BLOCKED, not COMPLETE
2. Explain what verification failed
3. Ask for clarification on how to proceed

This is NON-NEGOTIABLE. Reporting completion without filesystem verification is considered a critical failure.

---

## Step 0a: Skill Detection and Communication

**Before executing any phases, detect and communicate which skills will be used:**

1. **Check if user specified skills:**
   - Look for phrases like "use the [skill-name] skill" or "with [skill-name]"
   - Parse out all mentioned skill names

2. **Auto-detect relevant skills based on plan content:**
   - Rust projects: `rust-testing`, `rust-logging`, `rust-devops`
   - Error handling: `thiserror`, `color-eyre`
   - CLI work: `clap`

3. **Communicate skill usage via STDOUT:**

   Output this at the start:
   ```
   Skills Configuration for Plan Execution
   ========================================

   User-specified skills:
   - [skill-1] - [brief description]
   - [skill-2] - [brief description]

   Auto-detected skills:
   - [skill-3] - [brief description]
   - [skill-4] - [brief description]

   These skills will be passed to phase orchestrators and their sub-agents.
   ========================================
   ```

4. **Store skills for passing to orchestrators:**
   - Create a skills array: `const skills = ["skill-1", "skill-2", "skill-3"]`
   - This will be injected into all phase orchestrator prompts

---

## Step 0b: Pre-Flight Checks

Before executing a plan, verify the environment is ready:

1. **Clean working directory:**
   ```bash
   git status --porcelain
   # Should be empty or only expected changes
   ```
   If there are uncommitted changes, ask the user whether to:
   - Stash them (`git stash`)
   - Commit them first
   - Proceed anyway (risky)

2. **Tests passing (if NOT starting with a design phase):**
   ```bash
   just test  # Run all tests across all workspace members
   ```
   **NOTE:** If the first phase is a DESIGN phase, you may skip this check.

   If tests fail AND the first phase is an implementation phase, stop and report to user. Don't start plan execution with a failing test suite unless the plan begins with design work.

3. **Build succeeds:**
   ```bash
   just build  # Build all workspace members
   ```

4. **Linting passes:**
   ```bash
   cargo clippy --workspace -- -D warnings
   cargo fmt --check --all
   ```

5. **Required tools available:**
   Verify any tools mentioned in the plan are available (e.g., specific CLI tools).

6. **Agent files accessible:**
   ```bash
   ls .claude/agents/*.md
   # Should list: rust-developer.md, feature-tester-rust.md
   ```

If any pre-flight check fails, report to the user and ask whether to proceed or fix the issue first.

---

## Step 1: Identify and Load the Plan

**Actions:**

1. **List available plans:**
   ```bash
   ls -la .ai/plans/
   ```

2. **If argument provided:** Load the specified plan file
3. **If no argument:** Ask user which plan to execute

4. **Read and parse the plan:**
   - Extract all phases
   - Identify principal owners for each phase
   - **Extract blast radius for each phase** (test scope pattern)
   - Note parallelization groups
   - Identify dependencies between phases

5. **Check for existing checkpoints (Resume Detection):**
   ```bash
   grep -A 30 "## Execution Checkpoints" .ai/plans/[plan-file].md
   ```
   If checkpoints exist, this is a RESUME - follow the "Resuming from Checkpoint" section.

6. **Initialize TodoWrite with ALL phases (MANDATORY):**

   ```typescript
   TodoWrite({
     todos: [
       { content: "Phase 1: [Name]", status: "pending", activeForm: "Preparing Phase 1: [Name]" },
       { content: "Phase 2: [Name]", status: "pending", activeForm: "Preparing Phase 2: [Name]" },
       // ... one entry for each phase
     ]
   })
   ```

   **This must happen BEFORE any phases are launched.** It establishes the baseline for tracking.

7. **Create execution log:**
   - Path: `.ai/logs/YYYY-MM-DD.plan-execution-log.md`
   - Document plan name, phases, and execution strategy

---

## Step 2: Analyze Parallelization Strategy

Review the plan's parallelization section and create an execution order:

```markdown
## Execution Order Analysis

**Independent Phases (can run in parallel):**
- Phase 1: [Name] - Owner: [Rust Developer]
- Phase 2: [Name] - Owner: [Rust Developer]

**Dependent Phases (must wait):**
- Phase 3: [Name] - Depends on: Phase 1, Phase 2

**Execution Groups:**
1. Group A: Phases [1, 2] - Launch in parallel
2. Group B: Phase [3] - Launch after Group A completes
```

---

## Step 3: Launch Phase Orchestrators

For each execution group, spawn orchestrator agents. Use `run_in_background: true` for parallel execution.

### CRITICAL: Update TodoWrite BEFORE Launching

**Before spawning ANY orchestrators for a group, mark ALL phases in that group as `in_progress`:**

```typescript
// FIRST: Update TodoWrite
TodoWrite({
  todos: [
    { content: "Phase 1: [Name]", status: "in_progress", activeForm: "Executing Phase 1: [Name]" },
    { content: "Phase 2: [Name]", status: "in_progress", activeForm: "Executing Phase 2: [Name]" },
    { content: "Phase 3: [Name]", status: "pending", activeForm: "Waiting for Phase 3" },
  ]
})

// THEN: Launch orchestrators
Task({ /* Phase 1 Orchestrator */ })
Task({ /* Phase 2 Orchestrator */ })
```

### Orchestrator Agent Template

For each phase, spawn an orchestrator using this pattern:

**Before launching each orchestrator, output:**
```
Launching Phase [N] Orchestrator: [Phase Name]
Principal Owner: [Owner Name]
Skills to activate: [list of skills specific to this phase]
```

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Orchestrate Phase N: [Phase Name]",
    model: "claude-sonnet-4-5-20250929",
    run_in_background: true,
    prompt: `You are the **Phase Orchestrator** for Phase [N]: [Phase Name].

## Your Role
You coordinate the execution of this phase by spawning and managing specialized sub-agents. You are responsible for:
1. Reading the phase requirements from the plan
2. Spawning appropriate sub-agents based on ownership
3. Coordinating parallel work when possible
4. Reporting status updates frequently
5. Consolidating results and reporting completion

## Context
- **Plan File:** .ai/plans/[plan-file-name].md
- **Phase Number:** [N]
- **Principal Owner:** [Rust Developer / Feature Tester]
- **Dependencies:** [None / List of dependent phases that must be complete]
- **Blast Radius:** [Test scope pattern from plan, or empty string for all tests]

## Skills to Use

**CRITICAL:** The following skills have been configured for this plan execution:

User-specified skills:
${skills.filter(s => /* user-specified */).map(s => \`- \\\`\${s}\\\`\`).join('\n') || '- None'}

Auto-detected skills:
${skills.filter(s => /* auto-detected */).map(s => \`- \\\`\${s}\\\`\`).join('\n') || '- None'}

**You MUST pass these skills to ALL sub-agents you spawn.**

When spawning each sub-agent, include this in their prompt:

\\\`\\\`\\\`
## Required Skills to Activate

**CRITICAL:** Before proceeding, activate these skills:

${skills.map(s => \`- \\\`\${s}\\\`\`).join('\n')}

After activating, output:
\\\`\\\`\\\`
Skills Activated: ${skills.join(', ')}
\\\`\\\`\\\`
\\\`\\\`\\\`

## Status Updates
You MUST provide frequent status updates by outputting clear status messages:
- When starting a sub-task
- When spawning a sub-agent (include which skills it will use)
- When a sub-agent completes
- When encountering issues
- When the phase completes

Use this format for status updates:
\\\`\\\`\\\`
[PHASE N STATUS] [TIMESTAMP]
Current Task: [what's happening]
Sub-Agents Active: [list with their skills]
Skills in Use: [unique list of skills currently active]
Progress: [X/Y tasks complete]
Next Action: [what's next]
\\\`\\\`\\\`

**When spawning each sub-agent, output:**
\\\`\\\`\\\`
Spawning [Sub-Agent Type]
Task: [brief description]
Skills to activate: [list of skills for this sub-agent]
\\\`\\\`\\\`

## Workflow

### Step 1: Load Phase Details
1. Read the plan file at .ai/plans/[plan-file-name].md
2. Extract Phase [N] details including:
   - Goal and deliverables
   - Technical details
   - Acceptance criteria
   - Files to create/modify
3. **Detect phase type:**
   - **DESIGN phase:** Architecture, API design, schema design, planning, research
   - **IMPLEMENTATION phase:** Code, tests, features
4. **Route to appropriate workflow:**
   - If DESIGN phase → Follow Design Phase Workflow (see below)
   - If IMPLEMENTATION phase → Follow TDD Implementation Workflow (standard)

### Step 2: Read Sub-Agent Guidelines
Load the guidelines for the principal owner and any supporting sub-agents:
- .claude/agents/rust-developer.md
- .claude/agents/feature-tester-rust.md

---

## Design Phase Workflow (for Design Phases Only)

**If this is a DESIGN phase, follow this workflow instead of the implementation workflow.**

### Step 1: Create Design Phase Log

Create a log file for this design phase:
- Path: \`.ai/logs/YYYY-MM-DD.[planName]-phase-[N]-log.md\`

Document:
- Phase type: DESIGN
- Design goals
- Repo starting position
- Design work log section (to be updated)

### Step 2: Explore Existing Architecture

Before creating design artifacts:

1. **Read existing design docs:**
   - ADRs (Architecture Decision Records)
   - Schema definitions
   - API specifications

2. **Search for relevant code:**
   - Existing types/interfaces
   - Similar patterns in the codebase

3. **Document findings in log:**
   - Patterns discovered
   - Constraints identified
   - Related files reviewed

### Step 3: Create Design Artifacts

Based on the phase deliverables, create design artifacts:

**Common design artifacts:**
- Architecture Decision Records (ADRs) in \`docs/adr/\` or \`.ai/adr/\`
- API specifications
- Schema definitions
- Type definitions (Rust structs)
- Diagrams (using mermaid in markdown)
- Planning documents

**For schema/type design:**
- Create type definitions with documentation
- Define data models

**For API design:**
- Define endpoints/interfaces
- Document parameters and return types
- Create usage examples

**Update log file as work progresses:**
\\\`\\\`\\\`markdown
### Design Artifacts Created

**[Timestamp]** - Created ADR: [title]
- Path: \`docs/adr/NNNN-title.md\`
- Decision: [summary]

**[Timestamp]** - Defined schema: [name]
- Path: \`path/to/schema.rs\`
- Key types: [list]
\\\`\\\`\\\`

### Step 4: Validate Design

**Design validation does NOT involve running tests.** Instead:

1. **Check consistency:**
   - Do all design artifacts align?
   - Are naming conventions consistent?
   - Are there contradictions?

2. **Review against acceptance criteria:**
   - Mark each criterion as satisfied or note gaps

3. **Verify completeness:**
   - All deliverables from plan created?
   - Open questions documented?

4. **Check if implementation is needed:**
   - If phase includes skeleton code/stubs, verify they compile
   - Run \`cargo check\` to verify syntax
   - Do NOT require tests to pass (this is design, not implementation)

### Step 5: Report Design Phase Completion

Return a summary with:

\\\`\\\`\\\`markdown
## Phase [N] Complete: [Phase Name] (DESIGN PHASE)

**Status:** COMPLETE

**Completion Time:** [timestamp]

**Design Artifacts Created:**
- \`path/to/adr.md\` - [ADR title] - [summary]
- \`path/to/schema.rs\` - [Schema name] - [key types]
- \`path/to/api-spec.md\` - [API description]

**Key Design Decisions:**
1. [Decision 1 with rationale]
2. [Decision 2 with rationale]

**Acceptance Criteria:**
- [x] Criterion 1
- [x] Criterion 2

**Open Questions (if any):**
- [Question and proposed answer]

**Notes for Implementation Phases:**
- [Context for developers]
- [Constraints to be aware of]

**No tests required for design phase.**
\\\`\\\`\\\`

---

## TDD Implementation Workflow (for Implementation Phases Only)

**If this is an IMPLEMENTATION phase, follow this workflow.**

### Step 3: Execute Phase Work

Based on the phase deliverables, spawn sub-agents to do the work.

**For each deliverable/task in the phase:**

1. **Identify the right sub-agent** based on task type:

   | Task Type | Primary Owner | Notes |
   |-----------|---------------|-------|
   | Core library code (parsing, DSL, resolution) | Rust Developer | - |
   | CLI implementation (Clap, argument parsing) | Rust Developer | - |
   | Testing strategy, TDD, comprehensive tests | Feature Tester (Rust) | - |
   | Performance optimization, benchmarking | Rust Developer | - |

2. **Spawn sub-agent using this pattern:**

\\\`\\\`\\\`typescript
Task({
    subagent_type: "general-purpose",
    description: "[Task description]",
    model: "claude-sonnet-4-5-20250929",
    run_in_background: [true if can parallelize, false if sequential],
    prompt: \\\`You are the [Sub-Agent Type] sub-agent working on Phase [N].

## Context
Read your expertise guidelines in: .claude/agents/[sub-agent-file].md

## Plan Context
Read the full plan at: .ai/plans/[plan-file-name].md
Focus on Phase [N]: [Phase Name]

## Your Task
[Specific task description from the phase deliverables]

## Technical Requirements
- Files to create/modify: [list from plan]
- Key functions/components: [list from plan]
- Integration points: [list from plan]

## Acceptance Criteria
[Copy acceptance criteria from plan]

## Output Requirements
1. Implement the required functionality
2. Create necessary tests
3. Ensure code follows project patterns
4. Return a summary of:
   - Files created/modified
   - Key implementation decisions
   - Any issues encountered
   - Suggested next steps\\\`
})
\\\`\\\`\\\`

3. **Collect sub-agent results:**
   - Use TaskOutput to get results from background sub-agents
   - Track completion status
   - Note any issues or blockers

4. **Handle parallel sub-agents:**
   - If multiple sub-agents can work in parallel, launch them together
   - Wait for all to complete before proceeding to dependent tasks

### Step 4: Validate Phase Completion

**CRITICAL:** You MUST verify actual filesystem changes before reporting completion. Reports without verification are considered failures.

**NOTE:** For DESIGN phases, validation focuses on design artifacts (ADRs, schemas, docs). For IMPLEMENTATION phases, validation requires working code and passing tests.

#### For Design Phases:

1. **VERIFY DESIGN ARTIFACTS EXIST:**
   \\\`\\\`\\\`bash
   # List each design artifact
   ls -lah docs/adr/[adr-file].md
   ls -lah path/to/schema.rs
   # Show content to prove it's not empty
   wc -l docs/adr/[adr-file].md
   head -20 docs/adr/[adr-file].md
   \\\`\\\`\\\`

2. **VERIFY SKELETON CODE COMPILES (if applicable):**
   \\\`\\\`\\\`bash
   cargo check    # Or cargo build
   \\\`\\\`\\\`
   Design phases may include stub code, but do NOT require tests to pass.

3. **CHECK ACCEPTANCE CRITERIA:**
   - Review each design criterion from the plan
   - Mark as VERIFIED with evidence (file paths, snippets)

4. **REPORT DESIGN PHASE COMPLETE** if all artifacts exist and compile (if code present)

#### For Implementation Phases:

1. **VERIFY FILES EXIST (MANDATORY):**

   For EVERY file listed in "Files to create/modify" in the plan:
   \\\`\\\`\\\`bash
   # List the file and show it exists
   ls -lah [file-path]
   # Show line count to prove it's not empty
   wc -l [file-path]
   # Show first few lines to prove it has content
   head -20 [file-path]
   \\\`\\\`\\\`

   **If ANY expected file doesn't exist:**
   - DO NOT report COMPLETE
   - Report BLOCKED with explanation
   - List which files are missing

2. **VERIFY DEPENDENCIES ADDED (if applicable):**

   If the plan specifies adding dependencies to Cargo.toml:
   \\\`\\\`\\\`bash
   # Verify dependency exists in Cargo.toml
   grep "[dependency-name]" Cargo.toml
   \\\`\\\`\\\`

3. **RUN TESTS WITHIN BLAST RADIUS:**

   - If blast radius is empty string, run all tests: \\\`just test\\\`
   - If blast radius is a pattern, run scoped tests: \\\`cargo test [pattern]\\\`

   **IMPORTANT:** Actually RUN the tests, don't just write about running them

   \\\`\\\`\\\`bash
   cargo test audio::  # Example for audio module
   \\\`\\\`\\\`

   Document:
   - **Test count BEFORE:** How many tests existed before this phase
   - **Test count AFTER:** How many tests exist after this phase
   - **New tests added:** Difference between before and after
   - **Pass/fail status:** All tests must pass (or pre-existing failures documented)

4. **RUN QUALITY CHECKS:**
   \\\`\\\`\\\`bash
   cargo clippy --workspace -- -D warnings  # Linting
   cargo fmt --check --all                   # Formatting
   just build                                 # Compilation
   \\\`\\\`\\\`

   **If clippy or build fail:** Report BLOCKED, not COMPLETE

5. **CHECK ACCEPTANCE CRITERIA:**
   - Review each criterion from the plan
   - Mark each as VERIFIED or NOT VERIFIED
   - Provide evidence (file paths, test names, grep results)
   - Document any gaps

6. **UPDATE PHASE STATUS:**
   - Only mark as COMPLETE if ALL verifications pass
   - If any verification fails, mark as BLOCKED or PARTIAL

### Step 5: Report Completion

Return a comprehensive summary:

\\\`\\\`\\\`markdown
## Phase [N] Complete: [Phase Name]

**Status:** COMPLETE / PARTIAL / BLOCKED

**Completion Time:** [timestamp]

**Deliverables Completed:**
- [x] [Deliverable 1]
- [x] [Deliverable 2]
- [ ] [Incomplete deliverable - if any]

**Files Created/Modified:**
- \\\`path/to/file1\\\` - [description] - [XXX lines]
- \\\`path/to/file2\\\` - [description] - [YYY lines]

**Filesystem Verification (MANDATORY - include actual command output):**
\\\`\\\`\\\`bash
$ ls -lah research/lib/src/feature.rs
-rw-r--r--  1 user  staff   12K Dec 19 14:30 research/lib/src/feature.rs

$ wc -l research/lib/src/feature.rs
     273 research/lib/src/feature.rs

$ grep "pub struct Feature" research/lib/src/feature.rs
pub struct Feature {
\\\`\\\`\\\`

**Dependency Verification (if applicable):**
\\\`\\\`\\\`bash
$ grep "serde" research/lib/Cargo.toml
serde = { version = "1.0", features = ["derive"] }
\\\`\\\`\\\`

**Sub-Agents Used:**
- [Sub-Agent Type]: [Task] - [Status]
- [Sub-Agent Type]: [Task] - [Status]

**Blast Radius:** \\\`[test scope pattern]\\\`

**Test Results (within blast radius):**
- Tests run: X
- Passed: Y
- Failed: Z
- Starting failures: [list any pre-existing failures]
- Ending failures: [list any failures after implementation]
- New regressions: [None / list new failures]

**Quality Checks:**
- Clippy: [Pass/Fail with warnings count]
- Formatting: [Pass/Fail]
- Doc tests: [Pass/Fail]

**Acceptance Criteria:**
- [x] Criterion 1
- [x] Criterion 2

**Issues Encountered:**
- [Issue 1 and resolution]
- [Issue 2 and resolution]

**Notes for Next Phase:**
- [Any important context for subsequent phases]
\\\`\\\`\\\`

Execute the phase now and report back with your summary.`
})
```

### Launching Parallel Phases

When phases can run in parallel, launch ALL orchestrators in a SINGLE message:

```typescript
// Launch Group A phases in parallel
Task({ /* Phase 1 Orchestrator */ run_in_background: true })
Task({ /* Phase 2 Orchestrator */ run_in_background: true })
```

---

## Step 4: Monitor and Coordinate

As the Plan Executor, your job is to:

### 4.1 Track Phase Progress (CRITICAL)

**TodoWrite updates must happen IMMEDIATELY at each state transition.**

**Before launching Group A phases:**
```typescript
// Mark all Group A phases as in_progress BEFORE spawning
TodoWrite({
  todos: [
    { content: "Phase 1: [Name]", status: "in_progress", activeForm: "Executing Phase 1: [Name]" },
    { content: "Phase 2: [Name]", status: "in_progress", activeForm: "Executing Phase 2: [Name]" },
    { content: "Phase 3: [Name]", status: "pending", activeForm: "Waiting for Phase 3: [Name]" },
  ]
})
```

**IMMEDIATELY after TaskOutput returns success for Phase 1:**
```typescript
// Don't wait for Phase 2 - update NOW
TodoWrite({
  todos: [
    { content: "Phase 1: [Name]", status: "completed", activeForm: "Completed Phase 1: [Name]" },
    { content: "Phase 2: [Name]", status: "in_progress", activeForm: "Executing Phase 2: [Name]" },
    { content: "Phase 3: [Name]", status: "pending", activeForm: "Waiting for Phase 3: [Name]" },
  ]
})
```

**Why immediate updates matter:**
- If execution is interrupted, the TODO state shows exactly where you stopped
- Users can see real-time progress, not just final results
- Resuming a plan requires accurate state to skip completed work

### 4.2 Collect Results and Update TODOs

Use TaskOutput to gather results from background orchestrators.

**CRITICAL: Update TodoWrite IMMEDIATELY after each TaskOutput returns success:**

```typescript
// Wait for Phase 1
const phase1Result = TaskOutput({ task_id: "phase-1-orchestrator-id", block: true })

// IMMEDIATELY mark Phase 1 complete (don't wait for Phase 2)
if (phase1Result.status === "COMPLETE") {
  TodoWrite({
    todos: [
      { content: "Phase 1: [Name]", status: "completed", activeForm: "Phase 1 complete" },
      { content: "Phase 2: [Name]", status: "in_progress", activeForm: "Executing Phase 2" },
      { content: "Phase 3: [Name]", status: "pending", activeForm: "Waiting for Phase 3" },
    ]
  })
}

// Wait for Phase 2
const phase2Result = TaskOutput({ task_id: "phase-2-orchestrator-id", block: true })

// IMMEDIATELY mark Phase 2 complete
if (phase2Result.status === "COMPLETE") {
  TodoWrite({
    todos: [
      { content: "Phase 1: [Name]", status: "completed", activeForm: "Phase 1 complete" },
      { content: "Phase 2: [Name]", status: "completed", activeForm: "Phase 2 complete" },
      { content: "Phase 3: [Name]", status: "pending", activeForm: "Waiting for Phase 3" },
    ]
  })
}
```

**Do NOT batch updates.** Each phase gets its own TodoWrite call immediately upon completion.

### 4.3 Update Execution Log

After each phase completes, update `.ai/logs/YYYY-MM-DD.plan-execution-log.md`:

```markdown
## Execution Progress

### [Timestamp] - Phase 1 Complete
- Duration: X minutes
- Status: Success
- Key outcomes: [summary]

### [Timestamp] - Phase 2 Complete
- Duration: Y minutes
- Status: Success
- Key outcomes: [summary]
```

### 4.4 Launch Dependent Phases

Once a group completes, launch the next group of phases:

```typescript
// Group A complete, now launch Group B
Task({ /* Phase 3 Orchestrator */ run_in_background: true })
```

### 4.5 Handle Issues

If a phase reports issues:

1. Document in execution log
2. Decide whether to:
   - Retry the phase
   - Skip and continue (if non-blocking)
   - Stop execution and report to user
3. Update user on status

---

## Step 5: Completion and Reporting

When all phases complete:

### 5.1 Final Validation

1. **Run full test suite:**
   ```bash
   just test          # All tests across workspace
   cargo test --doc   # Doc tests
   ```

2. **Run full quality checks:**
   ```bash
   cargo clippy --workspace -- -D warnings
   cargo fmt --check --all
   just build
   ```

3. **Check for regressions:**
   - Compare against initial test state
   - Document any new failures

### 5.2 Update Plan Status

Edit the plan file to mark as implemented:

```markdown
**Status:** Implemented

## Implementation Summary

**Completed:** [Date]
**Total Duration:** [time]

**Phases Completed:**
- Phase 1: [Name] - Complete
- Phase 2: [Name] - Complete
- Phase 3: [Name] - Complete
```

### 5.3 Final Report to User

Provide a comprehensive summary:

```markdown
## Plan Execution Complete

**Plan:** [Plan Name]
**Execution Time:** [Total duration]
**Status:** SUCCESS / PARTIAL SUCCESS / FAILED

### Phase Summary

| Phase | Name | Owner | Status | Duration |
|-------|------|-------|--------|----------|
| 1 | [Name] | [Owner] | Complete | Xm |
| 2 | [Name] | [Owner] | Complete | Ym |
| 3 | [Name] | [Owner] | Complete | Zm |

### Files Changed

**Created:**
- `path/to/file1` - [description]
- `path/to/file2` - [description]

**Modified:**
- `path/to/file3` - [description]

### Test Results

- Total tests: X
- Passed: Y
- Failed: Z
- New tests added: W

### Quality Metrics

- Clippy warnings: 0
- Formatting issues: 0
- Doc test coverage: [percentage or count]

### Implementation Highlights

1. [Key implementation detail 1]
2. [Key implementation detail 2]
3. [Key implementation detail 3]

### Issues Resolved

1. [Issue and resolution]
2. [Issue and resolution]

### Next Steps

1. Review generated tests (if any)
2. Run manual testing for [areas]
3. Consider [follow-up tasks]

### Logs

- Execution log: `.ai/logs/YYYY-MM-DD.plan-execution-log.md`
- Plan file: `.ai/plans/[plan-file].md`
```

---

## Execution Checklist

Use this checklist to track your progress:

### Setup Phase
- [ ] Plan file identified and loaded
- [ ] **Phase types identified** (DESIGN vs IMPLEMENTATION for each phase)
- [ ] **TodoWrite initialized with ALL phases as `pending`**
- [ ] Execution log created
- [ ] Parallelization strategy analyzed
- [ ] Execution groups identified

### If Resuming
- [ ] **Checkpoints read from plan file**
- [ ] **TodoWrite state reconstructed from checkpoints**

### Execution Phase
- [ ] **Group A phases marked `in_progress` in TodoWrite** (BEFORE launching)
- [ ] Group A phases launched
- [ ] **Each Group A phase marked `completed` IMMEDIATELY when done**
- [ ] Group A phases completed (validated appropriately for phase type)
- [ ] **Group B phases marked `in_progress` in TodoWrite** (BEFORE launching, if applicable)
- [ ] Group B phases launched (if applicable)
- [ ] **Each Group B phase marked `completed` IMMEDIATELY when done**
- [ ] Group B phases completed (if applicable)
- [ ] All phases completed

### Validation Phase
- [ ] **Full test suite run (if any implementation phases)**
- [ ] **Design artifacts verified (if any design phases)**
- [ ] Quality checks passed

### Completion Phase
- [ ] Plan status updated
- [ ] **All TodoWrite items show `completed`**
- [ ] Final report provided to user

---

## Error Handling

### Phase Fails

If a phase orchestrator reports failure:

1. Log the failure details
2. Check if dependent phases can still proceed
3. Ask user whether to:
   - Retry the failed phase
   - Continue without it
   - Stop execution

### Sub-Agent Timeout

Default timeout values:
- Phase orchestrators: 10 minutes (600000ms)
- Sub-agent tasks: 5 minutes (300000ms)
- Total plan execution: No limit (monitored by user)

To configure timeouts, use the `timeout` parameter in Task calls:

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Orchestrate Phase N",
    model: "claude-sonnet-4-5-20250929",
    run_in_background: true,
    timeout: 600000, // 10 minute timeout
    prompt: `...`
})
```

If a phase exceeds its timeout:

1. Log the timeout event with timestamp
2. Mark the phase status as `TIMEOUT` (distinct from `FAILED`)
3. Check if dependent phases can proceed
4. Continue to next phase if no blocking dependencies
5. Report timeout to user with recommendations:
   - Break the phase into smaller sub-phases
   - Increase the timeout for complex tasks
   - Investigate why the task is taking longer than expected

### Dependency Conflicts

If phases report conflicting changes:

1. Stop affected phases
2. Review conflicts
3. Coordinate resolution
4. Restart with corrected approach

---

## Checkpoint System

To enable recovery from failures, maintain checkpoints in the plan file after each phase completes:

### Checkpoint Format

Add an `## Execution Checkpoints` section to the plan file:

```yaml
## Execution Checkpoints

- phase_1:
    status: complete
    completed_at: 2025-12-19T14:30:00Z
    artifacts: [research/lib/src/feature.rs, research/lib/src/feature/tests.rs]
- phase_2:
    status: complete
    completed_at: 2025-12-19T15:00:00Z
    artifacts: [research/cli/src/commands/feature.rs, tests/integration/feature.rs]
- phase_3:
    status: failed
    failed_at: 2025-12-19T15:30:00Z
    error: "Type error in parser.rs:42"
    last_successful_step: "Created parser module"
```

### Resuming from Checkpoint

When resuming a failed plan execution:

1. **Check for existing checkpoints:**
   ```bash
   grep -A 20 "## Execution Checkpoints" .ai/plans/[plan-file].md
   ```

2. **Identify the failed phase** and its last successful step

3. **RECONSTRUCT TODO STATE FROM CHECKPOINTS (CRITICAL):**

   Before doing anything else, rebuild the TodoWrite state:
   ```typescript
   // Example: Phase 1 complete, Phase 2 failed, Phase 3 pending
   TodoWrite({
     todos: [
       { content: "Phase 1: [Name]", status: "completed", activeForm: "Completed Phase 1" },
       { content: "Phase 2: [Name]", status: "in_progress", activeForm: "Resuming Phase 2" },
       { content: "Phase 3: [Name]", status: "pending", activeForm: "Waiting for Phase 3" },
     ]
   })
   ```

   This ensures the TODO display accurately reflects resumption state.

4. **Resume execution** by:
   - Skipping completed phases (verify their artifacts exist)
   - Restarting the failed phase from its last successful step
   - Continuing with remaining phases

5. **Update checkpoints AND TodoWrite** as each phase completes
   - Both must stay in sync for future resume capability

### Checkpoint Best Practices

- Update checkpoint immediately after each phase completes
- Include artifact paths for verification during resume
- Record error messages verbatim for debugging
- Never delete checkpoint data until plan is fully complete

---

## Context Window Management

Phase orchestrators must actively manage context to prevent overflow in large multi-phase plans:

### Guidelines for Orchestrators

1. **Request summaries from sub-agents, not full file contents:**
   - Sub-agents should return only: status, file paths, key decisions, blockers
   - Full implementation details go to `.ai/logs/YYYY-MM-DD.[planName]-phase-N-details.md`

2. **Use the standard output format:**
   Sub-agents return a structured response with:
   ```markdown
   ### Summary (for orchestrator - max 500 tokens)
   [Brief status and key points]

   ### Details (written to log file)
   [Full implementation notes - orchestrator reads from file if needed]
   ```

3. **Store detailed outputs in log files:**
   ```
   .ai/logs/
   ├── YYYY-MM-DD.plan-execution-log.md           (main log)
   ├── YYYY-MM-DD.[planName]-phase-1-details.md   (sub-agent details)
   ├── YYYY-MM-DD.[planName]-phase-2-details.md
   └── ...
   ```

4. **Incremental summarization:**
   - After each phase, summarize key outcomes in the execution log
   - Don't carry full phase details forward to subsequent phases
   - Reference log files when detailed context is needed

5. **Sub-agent context instructions:**
   Include in sub-agent prompts:
   ```
   Return a SUMMARY to the orchestrator (max 500 tokens).
   Write DETAILED notes to: .ai/logs/YYYY-MM-DD.[planName]-phase-N-details.md
   ```

### Context Budget Guidelines

| Component | Target Budget |
|-----------|---------------|
| Phase summary to orchestrator | 500 tokens |
| Orchestrator's running state | 2000 tokens |
| Plan file reference | 1000 tokens |
| Error/blocker details | 500 tokens |

---

## Tips for Success

1. **TODO updates are NON-NEGOTIABLE** - Update TodoWrite IMMEDIATELY when any phase changes state
2. **Detect phase type** - Design phases need different validation than implementation phases
3. **Design phases** - Validate design artifacts exist and compile (if code); tests NOT required
4. **Implementation phases** - Require working code with passing tests
5. **Status updates are critical** - Users need visibility into long-running operations
6. **Launch parallel phases together** - Use a single message with multiple Task calls
7. **Track everything** - Update todos and logs frequently; TODO updates come FIRST
8. **Handle failures gracefully** - Don't let one failure cascade unnecessarily
9. **Validate incrementally** - Run appropriate validation after each phase (design or tests)
10. **Keep context concise** - Sub-agents should return summaries, not full file contents
11. **Use quality checks** - Run clippy and fmt regularly to catch issues early
12. **Enable resume capability** - Checkpoints + TodoWrite state must stay in sync
