---
description: Execute a phase of a detailed TDD plan from .ai/plans/
argument-hint: [phase-number]
---

# Execute Phase Using TDD Workflow

You are now in **TDD Execution Mode**. Your goal is to execute a single phase from a plan following the rigorous 5-Step TDD Workflow.

## Prerequisites

Before starting, ensure:

1. A plan exists in `.ai/plans/` directory
2. You know which phase to execute
3. All previous phases are complete (if applicable)

## Step 0a: Skill Detection and Communication

**Before executing the phase, detect and communicate which skills will be used:**

1. **Check if user specified skills:**
   - Look for phrases like "use the [skill-name] skill" or "with [skill-name]"
   - Parse out all mentioned skill names

2. **Auto-detect relevant skills based on project type and phase content:**
   - **Rust projects:** `rust-testing`, `rust-logging`, `rust-devops`
   - **Error handling:** `thiserror`, `color-eyre`
   - **CLI work:** `clap`

3. **Communicate skill usage via STDOUT:**

   Output this at the start:
   ```
   Skills Configuration for Phase [N]
   ==================================

   User-specified skills:
   - [skill-1] - [brief description]
   - [skill-2] - [brief description]

   Auto-detected skills:
   - [skill-3] - [brief description]
   - [skill-4] - [brief description]

   These skills will be activated before phase execution begins.
   ==================================
   ```

4. **Store skills for later use:**
   - Create a skills array: `const skills = ["skill-1", "skill-2", "skill-3"]`
   - Activate all skills before proceeding to Step 1

---

## Step 0b: Activate All Required Skills

**CRITICAL:** Before proceeding with phase execution, activate ALL skills identified in Step 0a.

For each skill in the skills array:
1. Activate the skill
2. Wait for activation confirmation

After all skills are activated, output:
```
All Skills Activated
==================================
Activated skills: [skill-1], [skill-2], [skill-3]

Ready to begin phase execution.
==================================
```

## Step 1: Identify the Phase

Ask the user:

1. **Which plan should we execute?**
   - List available plans in `.ai/plans/`
   - Or ask for the plan filename

2. **Which phase should we execute?**
   - Show available phases from the plan
   - Suggest the next incomplete phase
   - Confirm phase number with the user

3. **Read the plan file:**
   - Use Read tool to load `.ai/plans/[planName].md`
   - Extract the details for the specified phase
   - **Detect phase type:** Is this a design phase or implementation phase?
     - Design phases: Architecture, API design, schema design, planning, research
     - Implementation phases: Code, tests, features
   - **Extract the blast radius** for this phase (test scope pattern)
   - If blast radius is empty string `""`, tests will run against entire test suite

4. **Route to appropriate workflow:**
   - If DESIGN phase → Follow Design Phase Workflow (see below)
   - If IMPLEMENTATION phase → Follow TDD Workflow (standard steps)

---

# Design Phase Workflow

**Use this workflow when the phase involves architecture, API design, schema design, planning, or research.**

## Step 1: Create Log File

**Purpose:** Document the design process and decisions.

**Actions:**

1. **Create log file:**
   - Path: `.ai/logs/YYYY-MM-[planName]-phase[N]-log.md`
   - Create `.ai/logs/` directory if it doesn't exist

2. **Write log file with starting state:**

   ```markdown
   # Phase [N]: [Phase Name] (DESIGN PHASE)

   **Plan:** [planName]
   **Phase:** [N]
   **Started:** [Date and Time]
   **Phase Type:** Design

   ## Phase Overview

   [Copy phase overview from plan]

   ## Design Goals

   [List design goals from the plan]

   ## Repo Starting Position

   **Last local commit:** [git log -1 --format="%H"]
   **Last remote commit:** [git log origin/main -1 --format="%H" 2>/dev/null || echo "N/A"]
   **Branch:** [git branch --show-current]
   **Dirty files:** [git status --short || echo "None"]

   ## Design Work Log

   [This section will be updated as design work progresses]
   ```

3. **Save the log file**

## Step 2: Explore Existing Architecture

**Purpose:** Understand current state before making design decisions.

**Actions:**

1. **Identify relevant files:**
   - Existing types, schemas, APIs
   - Related documentation
   - Similar patterns in the codebase

2. **Read existing documentation:**
   - Architecture Decision Records (ADRs)
   - Design docs
   - API specifications

3. **Document findings in log:**

   ```markdown
   ### Existing Architecture Exploration

   **Files reviewed:**
   - `path/to/file` - [purpose]

   **Patterns discovered:**
   - [Pattern 1]
   - [Pattern 2]

   **Constraints identified:**
   - [Constraint 1]
   - [Constraint 2]
   ```

## Step 3: Complete Design Work

**Purpose:** Create design artifacts (documentation, ADRs, schemas, API specs).

**Actions:**

1. **Create design artifacts based on phase deliverables:**
   - Architecture Decision Records (ADRs)
   - API specifications
   - Schema definitions
   - Type definitions
   - Diagrams (as markdown/mermaid)
   - Planning documents

2. **Follow project conventions:**
   - ADRs typically go in `docs/adr/` or `.ai/adr/`
   - Schemas in appropriate schema directories
   - Type definitions where they'll be used

3. **Update log file as you work:**

   ```markdown
   ### Design Artifacts Created

   **[Timestamp]** - Created ADR: [title]
   - Path: `docs/adr/NNNN-title.md`
   - Decision: [summary]

   **[Timestamp]** - Defined schema: [name]
   - Path: `path/to/schema.rs` or similar
   - Key types: [list]

   **[Timestamp]** - Designed API: [name]
   - Endpoints: [list]
   - Key decisions: [list]
   ```

## Step 4: Validate Design

**Purpose:** Ensure design is complete and coherent.

**Actions:**

1. **Review against acceptance criteria:**
   - Check each criterion from the plan
   - Mark as satisfied or note gaps

2. **Check for consistency:**
   - Do all design artifacts align?
   - Are there contradictions?
   - Are naming conventions consistent?

3. **Verify completeness:**
   - Are all deliverables from the plan created?
   - Are there open questions that need answering?

4. **Check if implementation is needed:**
   - If phase includes skeleton code/stubs, verify they compile
   - Run `cargo check` to verify syntax
   - Do NOT require tests to pass (this is design, not implementation)

5. **Update log file:**

   ```markdown
   ### Design Validation

   **Acceptance Criteria Review:**
   - [x] Criterion 1 - Satisfied
   - [x] Criterion 2 - Satisfied
   - [ ] Criterion 3 - Gap identified: [description]

   **Consistency Check:**
   - All artifacts reviewed for alignment
   - Naming conventions consistent

   **Open Questions:**
   - [Question 1 and proposed answer]
   ```

## Step 5: Close Out Design Phase

**Purpose:** Document completion and prepare for implementation phases.

**Actions:**

1. **Update log file with completion:**

   ```markdown
   ## Phase Completion

   **Completed:** [Date and Time]
   **Duration:** [Time taken]
   **Phase Type:** Design

   ### Design Artifacts Delivered

   **Created:**
   - `path/to/artifact1` - [description]
   - `path/to/artifact2` - [description]

   **Modified:**
   - `path/to/existing-file` - [changes made]

   ### Design Decisions

   **Key decisions made:**
   1. [Decision 1 with rationale]
   2. [Decision 2 with rationale]

   ### Acceptance Criteria

   - [x] Criterion 1
   - [x] Criterion 2

   ### Notes for Implementation Phases

   - [Important context for developers]
   - [Constraints to be aware of]
   - [Suggested implementation order]
   ```

2. **Update plan status:**
   - Read the plan file
   - Mark this phase as complete
   - Update the plan's status section
   - Save the updated plan

3. **Report completion to user:**

   ```text
   Phase [N] Complete: [Phase Name] (DESIGN PHASE)

   **Design artifacts created:**
   - [List of artifacts with paths]

   **Key design decisions:**
   - [Summary of major decisions]

   **Next steps:**
   1. Review design artifacts in [paths]
   2. Run `/execute-phase [N+1]` to continue to next phase

   **Log file:** `.ai/logs/YYYY-MM-[planName]-phase[N]-log.md`
   ```

## Design Phase Checklist

- [ ] Phase identified as DESIGN phase
- [ ] LOG created in `.ai/logs/`
- [ ] Starting position documented
- [ ] Existing architecture explored
- [ ] Design artifacts created (ADRs, schemas, APIs, etc.)
- [ ] Design validated against acceptance criteria
- [ ] Log file updated with completion
- [ ] Plan status updated
- [ ] User notified with summary

---

# TDD Implementation Workflow

**Use this workflow when the phase involves writing code, tests, or features.**

## Step 1.5: EXPLORE EXISTING CODE - MANDATORY

**CRITICAL: Before doing ANYTHING else, understand what code already exists!**

**Purpose:** Prevent architectural misunderstandings by examining existing code structure BEFORE implementation.

**Actions:**

1. **Identify files mentioned in the plan:**

   From the phase description, note ALL files that will be created or modified.

2. **Search for existing related files:**

   ```bash
   # Search for files with similar names
   find src -name "*[keyword]*" -type f

   # Or use Glob
   Glob: src/**/*[keyword]*.rs
   ```

   For example, if implementing "logging", search for:
   - Existing files with "log" in the name
   - Related service stubs
   - Similar utilities

3. **Read existing stubs/interfaces:**

   If files already exist:
   - **Read them COMPLETELY** using the Read tool
   - Understand the existing structure
   - Note TODOs or incomplete sections
   - Check if you're meant to COMPLETE existing code, not create new files

4. **Understand the architecture:**

   Before writing code, understand:
   - What patterns does the existing code use?
   - Are there type definitions you need to follow?
   - Are there existing interfaces or base traits?
   - How do similar features work in the codebase?

5. **Use Grep to find usage patterns:**

   ```bash
   # Find how existing code uses similar features
   Grep: "similar pattern"
   Grep: "use.*types"
   ```

6. **Document findings in log file:**

   ```markdown
   ### Existing Code Exploration

   **Files found:**
   - `research/lib/src/services/logging.rs` - EXISTS as stub (needs completion)
   - `research/lib/src/types/service.rs` - Defines Service type pattern

   **Architecture notes:**
   - Services are composable functions
   - Pattern: trait-based design with generics
   - Error handling: thiserror for library errors

   **Decision:** Complete existing stub, don't create new utility
   ```

7. **Validate plan against reality:**

   Ask yourself:
   - Does the plan match the existing code structure?
   - Am I creating something that already exists?
   - Am I understanding the architecture correctly?
   - Should I complete an existing stub instead of creating new files?

**If you discover a mismatch between the plan and existing code, STOP and inform the user before proceeding.**

**DO NOT SKIP THIS STEP.** Most architectural mistakes happen because this exploration was skipped.

---

## Step 2: SNAPSHOT - Capture Current Test State

**Purpose:** Establish a baseline so you can detect regressions and measure progress within the blast radius.

**Actions:**

1. **Run tests within the blast radius:**

   ```bash
   # If blast radius is empty string, run all tests:
   just test

   # If blast radius is a pattern, run scoped tests:
   cargo test [blast-radius]
   ```

2. **Create XML snapshot:**

   Create a simple XML representation of test results:

   ```xml
   <test-snapshot date="YYYY-MM-DD">
     <blast-radius>[pattern or "all"]</blast-radius>
     <suite name="runtime-tests" total="X" passed="Y" failed="Z" />
     <starting-failures>
       <failure test="module::test_name" />
     </starting-failures>
   </test-snapshot>
   ```

3. **Document starting failures within blast radius** - these are your baseline, don't fix them yet

## Step 3: CREATE LOG - Document Starting Position

**Purpose:** Create a detailed record for debugging and tracking progress.

**Actions:**

1. **Create log file:**
   - Path: `.ai/logs/YYYY-MM-[planName]-phase[N]-log.md`
   - Example: `.ai/logs/2025-12-user-auth-phase1-log.md`
   - Create `.ai/logs/` directory if it doesn't exist

2. **Write log file with starting state:**

   ```markdown
   # Phase [N]: [Phase Name]

   **Plan:** [planName]
   **Phase:** [N]
   **Started:** [Date and Time]
   **Blast Radius:** [test scope pattern or "all"]

   ## Phase Overview

   [Copy phase overview from plan]

   ## Starting Test Position

       <test-snapshot date="YYYY-MM-DD">
         <blast-radius>[pattern or "all"]</blast-radius>
         <suite name="runtime-tests" total="X" passed="Y" failed="Z" />
         <starting-failures>
           <failure test="module::test_name" />
         </starting-failures>
       </test-snapshot>

   ## Repo Starting Position

   **Last local commit:** [git log -1 --format="%H"]
   **Last remote commit:** [git log origin/main -1 --format="%H" 2>/dev/null || echo "N/A"]
   **Branch:** [git branch --show-current]
   **Dirty files:** [git status --short || echo "None"]

   ## Work Log

   [This section will be updated as work progresses]
   ```

3. **Save the log file**

4. **IMPORTANT:** Verify the markdown file has NO linting errors - proper formatting makes logs readable and professional

## Step 4: WRITE TESTS - Create Tests FIRST

**Purpose:** Tests define the contract and expected behavior before any code is written.

**CRITICAL: This is TRUE Test-Driven Development - tests MUST be written BEFORE implementation!**

**Actions:**

1. **Review test requirements from plan:**

   - Happy path tests
   - Edge case tests
   - Error condition tests

2. **Create test files:**

   For **Rust projects**:

   - Create unit tests in `#[cfg(test)] mod tests` blocks within source files
   - Create integration tests in `tests/` directory (each file is a separate crate)
   - Use `#[test]` attribute for test functions
   - Use `use super::*;` to access private functions in unit tests
   - Use `assert_eq!`, `assert_ne!`, `assert!` macros
   - Use `#[should_panic(expected = "message")]` for panic tests
   - Consider property-based tests with proptest for complex invariants
   - Run tests with `cargo test` or `just test`

3. **Write comprehensive tests:**

   Example unit tests:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn it_parses_valid_input() {
           let result = parse("valid input");
           assert!(result.is_ok());
           assert_eq!(result.unwrap().value, "expected");
       }

       #[test]
       fn it_rejects_empty_input() {
           let result = parse("");
           assert!(result.is_err());
       }

       #[test]
       #[should_panic(expected = "invalid state")]
       fn it_panics_on_invalid_state() {
           process_invalid_state();
       }
   }
   ```

   Example integration tests:
   ```rust
   // tests/integration_test.rs
   use research_lib::Feature;

   #[test]
   fn feature_end_to_end() {
       let feature = Feature::new();
       let result = feature.process();
       assert!(result.is_ok());
   }
   ```

4. **Verify tests FAIL initially:**

   For **Rust projects**:
   - Run tests: `cargo test` or `just test`
   - For specific module: `cargo test module_name`

   - Confirm tests fail (no implementation exists yet)
   - This verifies the tests are checking for real functionality, not trivially passing

5. **Think critically about test completeness:**

   - Review each test and ask: **If the functionality were built, would this test be meaningful?**
   - Consider all variants the function/utility/symbol can express:
     - Different input types and combinations
     - Boundary conditions and edge cases
     - Error states and failure modes
     - Return value variations
   - **Think hardest here** - missing variants now means gaps in coverage later
   - Are you testing behavior, not just implementation details?
   - Would these tests catch regressions if someone changed the code?

6. **Update log file with test creation:**

   Add to "Work Log" section:

   ```markdown
   ### Tests Created

   - `research/lib/src/module.rs` - X unit tests in #[cfg(test)] mod
   - `tests/integration_test.rs` - Y integration tests

   **Initial test run:** All tests fail as expected (no implementation yet)
   ```

## Step 4.5: VALIDATE TESTS - Critical Checkpoint

**MANDATORY: Before proceeding to implementation, validate your tests are correct**

**Purpose:** Catch testing pattern mistakes NOW, before they're baked into implementation. This checkpoint prevents hours of rework.

### For Rust Projects

**Actions:**

1. **Open the Rust testing skill:**

   Open `~/.claude/skills/rust-testing/SKILL.md` for testing patterns

2. **Validate test structure:**

   - Unit tests in `#[cfg(test)] mod tests` blocks within source files
   - Integration tests in `tests/` directory
   - `use super::*;` present for accessing private items
   - Descriptive test names: `fn it_returns_error_for_invalid_input()`

3. **Validate test patterns:**

   - Using `assert_eq!`, `assert_ne!`, `assert!` correctly
   - `#[should_panic]` for expected panics
   - Result return type for fallible tests: `fn test() -> Result<(), Box<dyn Error>>`

4. **Check for property tests (if applicable):**

   - Complex logic should have proptest invariants
   - Roundtrip tests for serialization

5. **Run the tests:**

   ```bash
   cargo test            # Standard runner
   cargo test --no-run   # Just compile, verify tests build
   ```

6. **Update log file with validation:**

   ```markdown
   ### Test Validation

   - Completed Rust testing checklist
   - Unit tests in correct location
   - Integration tests in tests/ directory
   - Tests ready for implementation
   ```

**DO NOT PROCEED TO IMPLEMENTATION IF ANY CHECKLIST ITEM FAILS**

Testing mistakes caught here save hours of debugging and rework later. If you're unsure about any pattern, re-read the skill guide sections.

---

## Step 5: IMPLEMENTATION - Build to Pass Tests

**Purpose:** Let tests drive the implementation, ensuring you build exactly what's needed.

**Actions:**

1. **Implement minimal code to pass each test:**
   - Start with one test or small group of related tests
   - Write the simplest code that makes tests pass
   - Don't over-engineer or add features not covered by tests

2. **Follow the plan's implementation details:**
   - Create files specified in the plan
   - Modify files specified in the plan
   - Implement key functions/structs as planned

3. **Iterate rapidly:**
   - Run tests frequently: `cargo test` or `just test`
   - Fix failures immediately
   - Keep the feedback loop tight

4. **Continue until all phase tests pass:**
   - All tests must be green
   - No shortcuts - every test must pass

5. **Refactor with confidence:**
   - Once tests pass, improve code quality
   - Tests act as a safety net
   - Re-run tests after each refactor

6. **Update log file during implementation:**

   Add to "Work Log" section as you go:

   ```markdown
   ### Implementation Progress

   **[Timestamp]** - Created `research/lib/src/feature.rs`
   - Implemented `process_feature()`
   - Tests passing: X/Y

   **[Timestamp]** - Modified `research/lib/src/lib.rs`
   - Added integration with new functionality
   - Tests passing: Y/Y

   **[Timestamp]** - Refactored for better readability
   - All tests still passing
   ```

## Step 6: CLOSE OUT - Verify and Document

**Purpose:** Ensure quality, prevent regressions, and properly document completion.

**Actions:**

1. **Run tests within blast radius:**

   ```bash
   # If blast radius is empty string, run all tests:
   just test        # All tests

   # If blast radius is a pattern, run scoped tests:
   cargo test [blast-radius]
   ```

2. **Check for regressions within blast radius:**

   Compare ending test failures against starting failures:

   - **Capture ending failures:** Run tests and note all failures
   - **Compare against starting failures:** Identify NEW failures
   - **New regressions = ending failures - starting failures**

   If NEW regressions exist:

   - **STOP and think deeply** - understand WHY, not just the error message
   - Add a "Regressions Found" section to log file with test name, failure message, root cause analysis, and resolution
   - Determine root cause:
     - Is your implementation incorrect?
     - Does the existing test need updating? (only if requirements changed)
     - Is there a side effect you didn't anticipate?
   - Fix the root cause, not just the symptom
   - Re-run tests within blast radius to confirm fix

3. **Run quality checks:**

   ```bash
   cargo clippy --workspace -- -D warnings
   cargo fmt --check --all
   just build
   ```

4. **Update log file with completion:**

   Add `## Phase Completion` section:

   ```markdown
   ## Phase Completion

   **Completed:** [Date and Time]
   **Duration:** [Time taken]
   **Blast Radius:** [test scope pattern or "all"]

   ### Final Test Results (within blast radius)

   - Tests passing: X/X

   ### Regression Analysis

   **Starting failures:** [count] tests
   - [list from starting snapshot]

   **Ending failures:** [count] tests
   - [list from final run]

   **New regressions:** [None / list any new failures]

   ### Files Changed

   **Created:**
   - `path/to/new-file.rs`

   **Modified:**
   - `path/to/existing-file.rs`

   ### Quality Checks

   - Clippy: Pass (0 warnings)
   - Formatting: Pass
   - Build: Pass
   ```

   **Verify markdown quality:** Ensure log file has no linting errors

5. **Update plan status:**

   - Read the plan file
   - Mark this phase as complete
   - Update the plan's status section
   - Save the updated plan
   - **Verify markdown quality:** Ensure updated plan has no linting errors

6. **Report completion to user:**

   Provide a clear summary:

   ```text
   Phase [N] Complete: [Phase Name]

   **What was implemented:**
   - [Summary of implementation]

   **Test coverage added:**
   - [Number] tests written
   - All tests passing
   - No regressions

   **Next steps:**
   1. Run `/execute-phase [N+1]` to continue to next phase
   2. Or review the implementation in [paths]

   **Log file:** `.ai/logs/YYYY-MM-[planName]-phase[N]-log.md`
   ```

## Important Reminders

- **Skills first** - Always detect, communicate, and activate skills before phase execution
- **User-specified skills** - Check for skills mentioned by the user and prioritize them
- **Auto-detect skills** - Based on project type and phase content
- **Communicate skills** - Output which skills will be used via STDOUT before starting
- **Detect phase type** - Design phases follow different workflow than implementation phases
- **Design phases** - Create design artifacts (ADRs, schemas, docs); no tests required
- **Implementation phases** - Follow TDD workflow with tests first
- **Tests FIRST** - Always write tests before implementation (implementation phases only)
- **Log everything** - Keep the log file updated throughout
- **Understand failures** - Don't just fix symptoms, understand root causes
- **Blast radius testing** - Run tests within blast radius, not necessarily entire suite
- **Track regressions properly** - Compare ending failures against starting failures; only NEW failures are regressions
- **Rust: Test location** - Unit tests inline with `#[cfg(test)]`, integration tests in `tests/`
- **Rust: Property tests** - Use proptest for complex invariants and roundtrip testing
- **Markdown quality** - ALL markdown files (logs, plan updates) MUST be lint-free; linting errors make documents very hard to read

## Phase Execution Checklist

Use this checklist to ensure you don't miss any steps:

### Common Steps (All Phases)

- [ ] **Skills detected** (user-specified and auto-detected)
- [ ] **Skills communicated via STDOUT**
- [ ] **All skills activated**
- [ ] Plan and phase identified
- [ ] **Phase type detected** (DESIGN vs IMPLEMENTATION)
- [ ] LOG created in `.ai/logs/`
- [ ] Starting position documented

### Design Phase Checklist

- [ ] Existing architecture explored
- [ ] Design artifacts created (ADRs, schemas, API specs, etc.)
- [ ] Design validated against acceptance criteria
- [ ] Design consistency checked
- [ ] Skeleton code compiles (if applicable)
- [ ] Log file updated with completion
- [ ] Plan status updated
- [ ] User notified with summary

### Implementation Phase Checklist

- [ ] Testing skill loaded (rust-testing)
- [ ] **Blast radius extracted from plan**
- [ ] SNAPSHOT captured (baseline test state within blast radius)
- [ ] **Starting failures documented**
- [ ] Tests written
- [ ] Tests initially failed (proving validity)
- [ ] Implementation completed
- [ ] All tests passing
- [ ] **Blast radius tests run**
- [ ] **Ending failures documented**
- [ ] **No NEW regressions** (ending - starting = 0 new failures)
- [ ] Quality checks passed (clippy, fmt, build)
- [ ] Log file updated with completion
- [ ] Plan status updated
- [ ] User notified with summary
