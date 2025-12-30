---
description: Create a detailed multi-phase plan with sub-agent ownership and parallel reviews for AI-powered research automation
---

# Multi-Phase Planning with Sub-Agent Review (AI Research Edition)

You are the **PLAN ORCHESTRATOR**. Your role is to coordinate specialized sub-agents to create an implementation-ready plan with maximum parallelization opportunities identified.

**CRITICAL:** This command is about **ORCHESTRATION**, not implementation. You coordinate sub-agents who do the detailed work.

**IMPORTANT:** Use the TodoWrite tool to track your progress through these steps.

## Overview

This planning workflow prioritizes **concurrency and orchestration**:

1. **FIRST**: Detect skills and analyze concurrency opportunities
2. Gather requirements (delegated to exploration agents when appropriate)
3. Create initial plan with parallelization groups identified
4. **Launch ALL reviews in parallel** (enforced - validation required)
5. Consolidate feedback and finalize parallelization strategy
6. Produce implementation-ready plan

**Orchestration Pattern**: Main thread → Parallel sub-agents → Consolidation → Output

---

## Step 0a: MANDATORY - Detect and Activate Skills

**🚨 DO THIS FIRST - BEFORE ANY OTHER WORK 🚨**

**Purpose**: Activate domain expertise before planning begins.

**Actions**:

1. **Detect user-specified skills:**
   - Look for phrases like "use the [skill-name] skill" or "with [skill-name]"
   - Parse out all mentioned skill names

2. **Auto-detect relevant skills based on task:**
   - Rust projects: `rust-testing`, `rust-logging`, `rust-devops`
   - AI/LLM work: Any AI/LLM-related skills
   - Error handling: `thiserror`, `color-eyre`
   - CLI work: `clap`
   - Web scraping: `reqwest`, `pulldown-cmark`

3. **Create skills array:**
   ```typescript
   const skills = ["skill-1", "skill-2", "skill-3"]
   ```

4. **Communicate via STDOUT:**
   ```
   📦 Skills Configuration for Planning Phase
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

   User-specified skills:
   • [skill-1] - [brief description]
   • [skill-2] - [brief description]

   Auto-detected skills:
   • [skill-3] - [brief description]
   • [skill-4] - [brief description]

   These skills will be passed to all sub-agents during plan review.
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   ```

5. **Activate all skills NOW** (before proceeding)

6. **Validation checkpoint:**
   ```
   ✅ Skills Activated: [skill-1], [skill-2], [skill-3]
   Ready to proceed to concurrency analysis.
   ```

**⛔ DO NOT PROCEED until skills are activated and validated.**

---

## Step 0b: MANDATORY - Analyze Concurrency Opportunities

**🚨 DO THIS SECOND - BEFORE REQUIREMENTS GATHERING 🚨**

**Purpose**: Identify parallelization strategy UPFRONT, not as an afterthought.

**Actions**:

1. **Understand the request at a high level:**
   - What type of work is this? (new feature, refactor, architecture, testing)
   - How many major areas of work are involved?
   - What are the obvious dependencies?

2. **Identify concurrent work streams:**

   Ask yourself:
   - Can requirements gathering happen in parallel? (exploration agents)
   - Can plan review happen in parallel? (multiple reviewer agents)
   - Can implementation phases run concurrently? (independent modules)

3. **Create initial concurrency map:**
   ```markdown
   ## Concurrency Analysis

   **Planning Phase Parallelization:**
   - Requirements exploration: [Can X, Y, Z be explored concurrently?]
   - Plan reviews: [Rust Developer + Feature Tester in parallel]

   **Implementation Phase Parallelization:**
   - Independent phases: [Phase 1, 2 can run together]
   - Dependent phases: [Phase 3 depends on 1, 2]
   ```

4. **Output concurrency strategy:**
   ```
   🔀 Concurrency Strategy
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

   Planning phase:
   • Requirement exploration: [Sequential / Parallel with N agents]
   • Plan reviews: [Parallel - Rust Dev + Tester]

   Implementation phase groups:
   • Group A: Phases [1, 2] - Parallel
   • Group B: Phase [3] - After Group A

   Estimated speedup: [X%] through parallelization
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   ```

5. **Validation checkpoint:**
   - Have you identified ALL opportunities for parallel work?
   - Are dependencies clearly noted?
   - Is the parallelization strategy clear?

**⛔ DO NOT PROCEED until concurrency strategy is documented.**

## Available Sub-Agents (Principal Owners)

| Sub-Agent | Domain | Assign When |
|-----------|--------|-------------|
| **Rust Developer** | Core library, CLI, async runtime, LLM integrations, API design | Systems programming, API clients, trait design, async workflows, data modeling |
| **Feature Tester (Rust)** | Rust testing strategy, cargo test, proptest | Testing strategy, TDD workflow, property-based tests, mocking LLM providers |

**Note:** Database Expert is NOT used for this project (no database).

---

## Prerequisites

Before starting:

1. **Ensure required directories exist:**

   ```bash
   mkdir -p .ai/plans .ai/logs
   ```

2. **Verify sub-agent definitions are accessible:**

   These commands require sub-agent definitions in one of these locations:
   - `.claude/agents/` (project-level, preferred)
   - `~/.claude/agents/` (user-level, fallback)

   Required agent files:
   - `agents/rust-developer.md`
    - `agents/feature-tester-rust.md`

---

## Step 1: Requirements Gathering

### 1.1 Understand the Task

Ask the user clarifying questions to fully understand what needs to be built:

1. **What is being built?**
   - Feature name and description
   - Primary goal and business value
   - Which workspace member(s) affected: `research/lib`, `research/cli`, `shared`, `tui`

2. **Who are the stakeholders?**
   - End users (researchers, developers)
   - Other systems/integrations (LLM providers: OpenAI, Gemini, ZAI)

3. **What are the constraints?**
   - Performance requirements (API response times, rate limiting)
   - LLM provider compatibility (multi-provider support)
   - Timeline expectations (scope, not duration)

### 1.2 Identify Requirements

Document both functional and non-functional requirements:

**Functional Requirements (FR):**

- What the system should DO
- Research automation workflows
- LLM provider integrations (prompts, responses, streaming)
- Data inputs and outputs (web content → research documents)
- Business rules and logic (content fetching, parsing, summarization)

**Non-Functional Requirements (NFR):**

- Performance (API latency, rate limit handling, concurrent requests)
- Security (API key management, content validation)
- Scalability (batch processing, concurrent research tasks)
- Maintainability (error handling with thiserror, logging with tracing)
- Reliability (error recovery, retry logic, graceful degradation)

### 1.3 Codebase Analysis

Use the Task tool with `subagent_type=Explore` to understand the current codebase:

```
Explore the codebase to understand:
1. Existing architecture and workspace structure (research/, shared/, tui/)
2. Relevant files and Rust modules
3. Testing infrastructure (cargo test setup, integration tests)
4. Build and CI processes (justfile commands)
5. LLM provider integrations (OpenAI, Gemini, ZAI)
6. Shared library capabilities (research-lib dependency on shared)
```

---

## Step 2: Create the Initial Plan

### 2.1 Plan Structure

Create a plan document at `.ai/plans/YYYY-MM-DD.plan-name.md`:

```markdown
# [Plan Name]

**Created:** [Date]
**Status:** Draft - Awaiting Review

## Executive Summary

[2-3 sentence overview of what this plan accomplishes]

## Requirements

### Functional Requirements

| ID | Requirement | Priority | Owner |
|----|-------------|----------|-------|
| FR-1 | [requirement] | High/Med/Low | [sub-agent] |
| FR-2 | [requirement] | High/Med/Low | [sub-agent] |

### Non-Functional Requirements

| ID | Requirement | Target | Owner |
|----|-------------|--------|-------|
| NFR-1 | [requirement] | [metric] | [sub-agent] |
| NFR-2 | [requirement] | [metric] | [sub-agent] |

## Architecture Overview

[High-level architecture description]

### Component Diagram

[ASCII or description of component relationships]

### Data Flow

[How data moves through the system - web content → LLM processing → research output]

## Phases

### Phase 1: [Phase Name]

**Principal Owner:** [Rust Developer/Feature Tester (Rust)]

**Goal:** [What this phase accomplishes]

**Dependencies:** None / [list dependencies]

**Blast Radius:** [Test scope - workspace member or test pattern]

**Deliverables:**
- [Deliverable 1]
- [Deliverable 2]

**Technical Details:**
- Workspace members to create/modify
- Key traits/structs/enums
- LLM provider integration patterns
- Integration points

**Acceptance Criteria:**
- [ ] [Criterion 1]
- [ ] [Criterion 2]

---

### Phase 2: [Phase Name]

[Repeat structure - include Blast Radius field]

---

## Blast Radius Analysis

For each phase, determine the **blast radius** - the scope of tests that should be run to verify both new functionality AND detect unintended downstream effects.

### How to Determine Blast Radius

1. **Identify direct test files:**
   - Tests for modules being created/modified
   - Example: `just test research` for research workspace changes

2. **Identify downstream dependencies:**
   - What workspace members import/depend on the code being changed?
   - What tests cover those dependent modules?

3. **Construct the test command:**
   - Use `just test research` for research workspace changes
   - Use `just test` (full suite) for shared library changes
   - Use `cargo test --lib [module]` for specific module tests

4. **Use full test suite for foundational changes:**
   - If changes affect shared library
   - If changes affect core traits, error types, or provider abstractions
   - If unsure about impact scope

### Blast Radius Examples

| Change Type | Blast Radius |
|-------------|--------------|
| New LLM provider | `cargo test --lib providers` in research/lib |
| Shared library update | `just test` (full monorepo) |
| CLI command addition | `cargo test --bin research` |
| Research workflow | `cargo test --lib workflows` |
| Error type modifications | `just test` (full suite - errors affect everything) |

---

## Cross-Cutting Concerns

### Testing Strategy
- Unit tests: `#[cfg(test)] mod tests` blocks in each module
- Integration tests: `tests/` directory for end-to-end workflows
- Property-based tests: proptest for data transformation roundtrips
- Mocking: mockall for LLM provider isolation in tests
- Wiremock for HTTP mocking

### Security Considerations
- API key management (environment variables, secure storage)
- Content validation (size limits, content-type checks)
- LLM prompt injection prevention
- Rate limiting and quota management

### Performance Considerations
- Async/await patterns for LLM and HTTP requests (tokio runtime)
- Concurrent request handling with rate limiting
- Streaming responses for large LLM outputs
- Caching strategies for API responses
- Retry logic with exponential backoff

### Project-Specific Concerns

**LLM Provider Integration:**
- Provider abstraction (trait-based design for multi-provider support)
- Rate limiting and retry logic per provider
- Response validation and error handling
- Token counting and cost tracking
- Streaming vs batch response handling

**Research Automation:**
- Web content fetching (reqwest, HTML parsing)
- Markdown processing (pulldown-cmark)
- Document summarization workflows
- Research organization and storage

**Workspace Structure:**
- `research/lib`: Core research automation library
- `research/cli`: Command-line interface
- `shared`: Shared utilities and types
- `tui`: Terminal UI (future)
- Dependency flow: cli → lib → shared

**Error Handling:**
- thiserror for library errors (research-lib, shared)
- Structured error types per module
- Error context preservation and reporting
- CLI error presentation

**Observability:**
- tracing for structured logging
- Spans for async operations
- Log levels per module
- Development vs production logging

## Parallelization Opportunities

[Phases that can be executed in parallel]

| Parallel Group | Phases | Reason |
|----------------|--------|--------|
| Group A | Phase 1, Phase 2 | No dependencies |
| Group B | Phase 3 | Depends on Group A |

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| [Risk 1] | High/Med/Low | [Mitigation strategy] |

## Open Questions

- [ ] [Question 1]
- [ ] [Question 2]
```

### 2.2 Assign Principal Owners

For each phase and requirement, assign a principal owner based on:

| Content Type | Primary Owner | Secondary |
|--------------|---------------|-----------|
| Architecture design, API contracts | Rust Developer | Feature Tester (Rust) |
| LLM provider abstractions, trait design | Rust Developer | Feature Tester (Rust) |
| Data structures, type systems | Rust Developer | Feature Tester (Rust) |
| Core library code (async, HTTP, parsing) | Rust Developer | Feature Tester (Rust) |
| CLI implementation (clap, user interaction) | Rust Developer | Feature Tester (Rust) |
| Testing strategy, TDD, mocking | Feature Tester (Rust) | Rust Developer |
| Performance optimization, async patterns | Rust Developer | Feature Tester (Rust) |

---

## Step 3: CRITICAL - Launch ALL Reviews in Parallel

**🚨 ORCHESTRATION CHECKPOINT: This is where you prove you're orchestrating, not implementing 🚨**

**MANDATORY REQUIREMENTS:**
1. **All review agents MUST be launched in a SINGLE message**
2. **All Task calls MUST use `run_in_background: true`**
3. **You MUST use multiple Task tool calls in ONE message**
4. **Validation REQUIRED after launch**

**Why this matters:** This command is about orchestration. If you launch agents sequentially, you're not orchestrating—you're micromanaging. Parallel execution is NON-NEGOTIABLE.

### 3.1 Identify Reviewers

Based on principal owners assigned in Step 2, determine which sub-agents need to review:
- Rust Developer: If any phases assigned to them
- Feature Tester (Rust): If any testing phases assigned

**Minimum**: At least 2 reviewers should run in parallel for most plans.

### 3.2 Review Prompts

For each sub-agent with assigned ownership, create a review task:

**Rust Developer Review:**

Before launching, output:
```
🔍 Launching Rust Developer Review
Skills to activate: [list of skills]
```

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Rust review of [plan-name]",
    model: "claude-sonnet-4-5-20250929",
    run_in_background: true,
    prompt: `You are the Rust Developer sub-agent reviewing a plan.

## First: Activate Required Skills

**CRITICAL:** You MUST activate the following skills before proceeding:

User-specified skills:
${skills.filter(s => /* check if user-specified */).map(s => `- \`${s}\``).join('\n')}

Auto-detected skills for this review:
${skills.filter(s => /* check if auto-detected for Rust */).map(s => `- \`${s}\``).join('\n')}

**Before reading anything else, activate these skills now.**

After activating skills, output:
\`\`\`
✅ Skills Activated for Rust Developer Review:
${skills.map(s => `• ${s}`).join('\n')}
\`\`\`

## Context
Read your expertise guidelines in: .claude/agents/rust-developer.md

## Plan to Review
Read the plan at: .ai/plans/YYYY-MM-DD.plan-name.md

## Your Review Focus
Review ALL sections where Rust Developer is assigned as owner, plus:

1. **Architecture**
   - Is the workspace structure appropriate?
   - Are trait boundaries well-defined for LLM providers?
   - Is the error type design sound (thiserror)?
   - Are async patterns correct (tokio, futures)?

2. **Performance**
   - Are async/await patterns optimized?
   - Is concurrent request handling efficient?
   - Are hot paths identified (API calls, parsing)?
   - Is retry logic with backoff implemented correctly?

3. **Safety**
   - Are ownership patterns correct?
   - Are API keys handled securely?
   - Are lifetimes handled properly?
   - Is error propagation comprehensive?

4. **Testing**
   - Are unit tests planned in \`#[cfg(test)] mod tests\` blocks?
   - Are integration tests planned in \`tests/\` directory?
   - Is mocking planned for LLM providers (mockall/wiremock)?
   - Are doc tests planned for public APIs?
   - Is TDD workflow incorporated?

5. **Observability**
   - Is tracing integrated for async operations?
   - Are spans and log levels appropriate?
   - Is error context preserved (thiserror sources)?

6. **LLM Integration Specifics**
   - Are provider abstractions well-designed (traits)?
   - Is rate limiting per provider implemented?
   - Is response validation comprehensive?
   - Is streaming vs batch handling correct?
   - Is error recovery robust?

7. **Monorepo Structure**
   - Are workspace dependencies correctly specified?
   - Is code duplication minimized via shared library?
   - Are workspace boundaries appropriate?

## Output Format
Return your review as:

### Rust Developer Review

**Overall Assessment:** [Approve / Approve with Changes / Request Revision]

**Strengths:**
- [strength 1]
- [strength 2]

**Concerns:**
- [concern 1 with suggested fix]
- [concern 2 with suggested fix]

**Suggested Changes:**
1. [specific change to plan]
2. [specific change to plan]

**Parallelization Notes:**
- [which Rust phases can run in parallel]
- [dependencies to be aware of]

**Missing Considerations:**
- [anything overlooked]`
})
```


**Feature Tester Review (Rust):**

Before launching, output:
```
🔍 Launching Feature Tester (Rust) Review
Skills to activate: [list of skills]
```

```typescript
Task({
    subagent_type: "general-purpose",
    description: "Rust testing strategy review of [plan-name]",
    model: "claude-sonnet-4-5-20250929",
    run_in_background: true,
    prompt: `You are the Feature Tester (Rust) sub-agent reviewing a plan.

## First: Activate Required Skills

**CRITICAL:** You MUST activate the following skills before proceeding:

User-specified skills:
${skills.filter(s => /* check if user-specified */).map(s => `- \`${s}\``).join('\n')}

Auto-detected skills for this review (minimum \`rust-testing\`):
${skills.filter(s => /* check if relevant to testing work */).map(s => `- \`${s}\``).join('\n')}

**Before reading anything else, activate these skills now.**

After activating skills, output:
\`\`\`
✅ Skills Activated for Feature Tester Review:
${skills.map(s => `• ${s}`).join('\n')}
\`\`\`

## Context
Read your expertise guidelines in: .claude/agents/feature-tester-rust.md

## Plan to Review
Read the plan at: .ai/plans/YYYY-MM-DD.plan-name.md

## Your Review Focus
Review the Rust testing strategy and ensure comprehensive test coverage:

1. **Test Strategy Completeness**
   - Are unit tests planned in \`#[cfg(test)] mod tests\` blocks?
   - Are integration tests planned in \`tests/\` directory?
   - Is TDD workflow incorporated appropriately?
   - Are doc tests planned for public APIs?

2. **Test Coverage**
   - Are happy paths covered?
   - Are edge cases and error conditions addressed?
   - Are property-based tests (proptest) planned for:
     - API response parsing
     - Data transformation roundtrips
     - Provider abstraction invariants
   - Are snapshot tests (insta) considered for LLM output?

3. **Test Organization**
   - Are unit tests in same file as implementation?
   - Are integration tests testing public API only?
   - Are test utilities in \`tests/common/mod.rs\`?
   - Is test data organized (fixture files)?

4. **Acceptance Criteria Testability**
   - Can each acceptance criterion be verified by a test?
   - Are there missing criteria that should be added?
   - Are LLM integration features testable?

5. **Testing Dependencies**
   - Are mockall traits used for LLM provider isolation?
   - Is wiremock used for HTTP mocking?
   - Are external dependencies properly abstracted?
   - Is tokio test runtime configured correctly?

6. **Rust-Specific Testing**
   - Are async tests properly marked with \`#[tokio::test]\`?
   - Are benchmarks (criterion) planned for performance-critical paths?
   - Are compilation tests planned for type constraints?

7. **AI/LLM-Specific Testing**
   - Are LLM provider mocks comprehensive?
   - Is rate limiting tested?
   - Is retry logic tested?
   - Is streaming response handling tested?
   - Are error scenarios from providers tested?

## Output Format
Return your review as:

### Feature Tester (Rust) Review

**Overall Assessment:** [Approve / Approve with Changes / Request Revision]

**Strengths:**
- [strength 1]
- [strength 2]

**Concerns:**
- [concern 1 with suggested fix]
- [concern 2 with suggested fix]

**Suggested Changes:**
1. [specific change to plan]
2. [specific change to plan]

**Test Scenarios to Add:**
- [missing test scenario 1]
- [missing test scenario 2]

**Missing Considerations:**
- [anything overlooked]`
})
```

### 3.3 Launch All Reviews in Parallel (ENFORCED)

**🚨 THIS IS THE CRITICAL ORCHESTRATION MOMENT 🚨**

**MANDATORY PATTERN:**

You MUST launch ALL reviewers in a SINGLE message with MULTIPLE Task tool calls:

```typescript
// ✅ CORRECT - All in ONE message for TRUE parallel execution
Task({
    subagent_type: "general-purpose",
    description: "Rust Developer review",
    run_in_background: true,
    prompt: `[Rust Developer review prompt with skills]`
})
Task({
    subagent_type: "general-purpose",
    description: "Feature Tester review",
    run_in_background: true,
    prompt: `[Feature Tester review prompt with skills]`
})
```

**❌ WRONG - Sequential launches (this is micromanaging, not orchestrating):**

```typescript
// First message
Task({ /* Rust Developer */ })

// Second message (LATER)
Task({ /* Feature Tester */ })
```

**Before launching, output:**
```
🚀 Launching Parallel Reviews
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Reviewers launching NOW (in parallel):
• Rust Developer (skills: [list])
• Feature Tester (Rust) (skills: [list])

Expected completion: [estimated time]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### 3.4 Validation Checkpoint (MANDATORY)

**After launching, IMMEDIATELY validate parallel execution:**

```
✅ Parallel Launch Validation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

☑ All reviewers launched in a SINGLE message
☑ All Task calls used run_in_background: true
☑ [N] reviewers running concurrently
☑ No sequential launches detected

Orchestration pattern: CORRECT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**If you cannot check all boxes above, you failed the orchestration requirement. Do not proceed.**

### 3.5 Collect Review Results

Use TaskOutput to collect results from all background tasks:

```typescript
TaskOutput({ task_id: "rust-review-id", block: true })
TaskOutput({ task_id: "tester-review-id", block: true })
```

**Collect results sequentially** (this is fine—agents ran in parallel, you're just gathering output)

---

## Step 4: Consolidation and Optimization

After all reviews complete, perform a final consolidation pass:

### 4.1 Synthesize Feedback

1. **Aggregate Concerns:** Group similar concerns across reviews
2. **Resolve Conflicts:** If reviewers disagree, determine the best path
3. **Prioritize Changes:** Order suggested changes by impact

### 4.2 Update the Plan

Incorporate review feedback into the plan:

1. Update requirement assignments if suggested
2. Modify phase details based on concerns
3. Add missing considerations identified by reviewers
4. Update acceptance criteria
5. Add project-specific considerations:
   - LLM provider integration patterns
   - Rate limiting and retry strategies
   - Async workflow optimizations
   - Testing with mocks/wiremock

### 4.3 Finalize Parallelization Analysis

Based on all reviews, create the final parallelization strategy:

```markdown
## Implementation Parallelization Strategy

### Parallel Execution Groups

| Group | Phases | Can Start After | Assignees |
|-------|--------|-----------------|-----------|
| A | 1, 2 | Plan approval | Rust Dev, Schema |
| B | 3 | Group A complete | Rust Dev, Tester |

### Parallelization Diagram
```

```text
Timeline:
─────────────────────────────────────────────────────►

Group A: ████████████ (Phase 1 + Phase 2 in parallel)
                     │
Group B:             └──████████████ (Phase 3)
```

### Synchronization Points

1. **After Group A:** Core traits and provider abstractions must be finalized
2. **Final:** Integration testing across workspace members

### 4.4 Update Plan Status

Change the plan status and add the review summary:

```markdown
**Status:** Reviewed - Ready for Implementation

## Review Summary

**Reviews Completed:** [Date]

**Reviewers:**
- Rust Developer: [Approve/Approve with Changes/Request Revision]
- Schema Architect: [Approve/Approve with Changes/Request Revision]
- Feature Tester (Rust): [Approve/Approve with Changes/Request Revision]

**Key Changes from Review:**
1. [Change 1]
2. [Change 2]
3. [Change 3]

**Resolved Concerns:**
- [Concern] → [Resolution]
```

---

## Step 5: Present to User

### 5.1 Summary Report

Present the final plan to the user with:

1. **Executive Summary** - What will be built
2. **Phase Overview** - High-level view of all phases
3. **Owner Assignments** - Who owns what
4. **Parallelization Strategy** - How to maximize efficiency
5. **Key Risks** - Top risks and mitigations
6. **Open Questions** - Items needing user input

After presenting the report to the console, you should speak to the user: "The plan in {{REPO_NAME}} called {{PLAN NAME}} has completed." using TTS.

- TTS
  - if the command `so-you-say` is available in the executable path then use this for TTS: `so-you-say "{{MESSAGE}}"`
  - if a command `say` is available on the host system (as it will be on macOS) then use that: `say "{{MESSAGE}}"`
  - if the host system has an executable called "speak" or "speak-ng" the use it for TTS: `speak "{{MESSAGE}}"` 
- `{{PLAN_NAME}}` should be the plan's file name without the leading date information and excluding the file extension.
- `{{REPO_NAME}}` should be derived the 

### 5.2 Request Approval

Ask the user to:

1. Review the plan at `.ai/plans/YYYY-MM-DD.plan-name.md`
2. Answer any open questions
3. Approve or request changes

---

## Output Artifacts

This command produces:

| Artifact | Location | Purpose |
|----------|----------|---------|
| Plan Document | `.ai/plans/YYYY-MM-DD.plan-name.md` | Complete implementation plan |
| Review Log | Embedded in plan | Sub-agent feedback |

---

## Example Workflow

```text
User: Create a plan for adding Brave Search API integration to research automation

Main Thread:
├── Step 1: Gather requirements
│   ├── Ask clarifying questions
│   ├── Document FR and NFR
│   └── Explore codebase (research/lib, provider abstractions)
│
├── Step 2: Create initial plan
│   ├── Draft plan with phases
│   ├── Assign principal owners:
│   │   ├── Schema Architect: Provider trait, search result types
│   │   ├── Rust Developer: HTTP client, API integration, rate limiting
│   │   └── Feature Tester: Test strategy with wiremock
│   └── Save to .ai/plans/
│
├── Step 3: Parallel reviews (ALL AT ONCE)
│   ├── Rust Developer ──────┐
│   ├── Schema Architect ─────├── Running in parallel
│   └── Feature Tester (Rust) ┘
│
├── Step 4: Consolidation
│   ├── Synthesize feedback
│   ├── Update plan
│   ├── Finalize parallelization:
│   │   ├── Group A: Types + trait (parallel)
│   │   ├── Group B: Implementation (after Group A)
│   │   └── Group C: Integration + tests (after Group B)
│   └── Mark as reviewed
│
└── Step 5: Present to user
    └── Request approval
```

---

## Tips for Success

1. **Be thorough in Step 1** - Good requirements lead to good plans
2. **Assign owners carefully** - Match expertise to tasks
3. **Always run reviews in parallel** - This is the key efficiency gain
4. **Don't skip consolidation** - Cross-cutting concerns emerge in review
5. **Document parallelization clearly** - Implementation teams need this
6. **Keep the plan living** - Update as implementation reveals new information
7. **Consider AI/LLM specifics:**
   - Provider abstraction patterns
   - Rate limiting per provider
   - Retry logic with exponential backoff
   - Streaming vs batch responses
   - Cost tracking and token counting
   - Mock strategies for testing

---

## Next Steps After Planning

Once the plan is approved:

1. **For TDD workflow:** Use `/execute-phase` to implement each phase
2. **For feature workflow:** Use `/add-feature` with the plan as context
3. **For parallel implementation:** Coordinate sub-agents based on parallelization groups
4. **Run tests:** `just test` or `cargo test` per workspace
5. **Verify across workspaces:** `just test` at root level

---

## ⚠️ CRITICAL: Plan Execution Warning

**If you use `/execute-plan` to implement this plan:**

The `/execute-plan` command uses orchestrator agents to coordinate implementation. **These orchestrators have historically failed by writing completion reports WITHOUT actually creating code.**

**To prevent this failure, your plan MUST:**

1. **Be EXTREMELY specific about files to create:**
   - List EXACT file paths for every file
   - Don't say "create provider module" - say "create research/lib/src/providers/brave.rs, research/lib/src/providers/brave/types.rs"

2. **Include verification steps in acceptance criteria:**
   - ✅ GOOD: "File `research/lib/src/providers/brave.rs` exists with `BraveProvider` struct"
   - ❌ BAD: "Core types implemented"

3. **Specify test count expectations:**
   - ✅ GOOD: "Add 5 unit tests for BraveProvider API calls"
   - ❌ BAD: "Tests cover provider integration"

4. **List dependencies explicitly:**
   - ✅ GOOD: "Add `reqwest = { version = \"0.12\", features = [\"json\"] }` to research/lib/Cargo.toml"
   - ❌ BAD: "Add reqwest dependency"

**Why This Matters:**

Orchestrators will verify implementation by:
- Running `ls` on expected files
- Running `grep` on Cargo.toml for dependencies
- Running `cargo test` or `just test` within the blast radius
- Checking line counts with `wc -l`

If your plan is vague, orchestrators may SIMULATE completion instead of VERIFYING it.

**Example of a GOOD Phase:**

```markdown
### Phase 1: Brave Search Provider Types

**Files to create:**
- `research/lib/src/providers/brave.rs` - Module entry point
- `research/lib/src/providers/brave/types.rs` - BraveSearchRequest, BraveSearchResponse structs

**Dependencies to add:**
- None (uses existing reqwest)

**Acceptance Criteria:**
- [ ] File `research/lib/src/providers/brave.rs` exists
- [ ] File `research/lib/src/providers/brave/types.rs` exists with >100 lines
- [ ] `grep "pub struct BraveSearchRequest" research/lib/src/providers/brave/types.rs` succeeds
- [ ] `cargo test --lib providers::brave::types` runs 8+ tests
- [ ] All new tests pass
```

**Example of a BAD Phase:**

```markdown
### Phase 1: Brave Search Provider Types

**Deliverables:**
- Provider types

**Acceptance Criteria:**
- [ ] Types are implemented
```

The BAD example gives orchestrators no way to verify success, so they'll just write a report claiming success.

---
