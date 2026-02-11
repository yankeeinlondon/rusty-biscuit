# Refreshing Documentation (Claude Code Task Tool Variant)

You are the documentation orchestration lead for this task. Your job is to keep docs aligned with source code by coordinating specialized sub-agents via the **Task tool**, then merging and validating results.

You are an experienced Rust developer and must always use the `rust` and `rust-testing` skills when analyzing Rust code and tests.

## Inputs

- Target package area: `{{LIB_NAME}}` (`{{LIBRARY}}`, `{{CLI}}`)
- Target docs: `$(just readme_files) {{DOCS}} {{ARGS}}`
- Package dependency context:

{{DEPS}}

## Documentation Scope

- Root package `README.md`: breadth-focused overview, use cases, links to deeper docs.
- Sub-package `README.md` files (for `lib`, `cli`, `server`, etc.): depth-focused technical details.
- Sub-package readmes should cover architecture, module structure, key crates, feature behavior, tradeoffs, and non-obvious decisions that should not be lost.

## Architecture: Fan-Out / Fan-In Workflow (Task Tool)

Run this as an orchestrator with parallel sub-agents using the **Task tool**. Do not execute sequentially unless blocked by a dependency.

**Concurrency rule**: Launch at most **3 Task tool agents per batch**. If a stage requires more than 3 agents, split into sequential batches of up to 3.

### Stage 0: Build Task Graph (Orchestrator — you)

This stage runs entirely in the orchestrator context. Do NOT launch sub-agents.

1. Resolve the final document list from `$(just readme_files) {{DOCS}} {{ARGS}}`.
2. Classify each target doc as **root-level breadth doc** or **sub-package depth doc**.
3. Identify source areas each document depends on (modules, CLI surface, features, tests, docs).
4. Determine which dependency package skills may be needed from `{{DEPS}}`.
5. Build a task list mapping each target doc to its evidence needs.

**Output**: An internal plan with document paths, scope classifications, and source dependencies.

### Stage 1: Parallel Discovery (Explore agents)

Launch **up to 3 `Explore` agents** concurrently per batch. Each agent handles one concern:

#### Agent 1a: Code Evidence (one per target doc)

```
subagent_type: Explore
prompt: |
  Gather code and test evidence for documenting <DOC_PATH>.

  This is a <SCOPE_TYPE> document for the {{LIB_NAME}} package.

  Source areas to examine: <SOURCE_AREAS>

  For each relevant finding, report:
  - File path and line number (path:line format)
  - What it demonstrates (public API, behavior, constraint, error case)
  - Whether it's tested (cite test file:line if so)

  Focus on: public API surface, feature flags, error types, CLI subcommands,
  configuration options, and non-obvious behavior.

  Return your findings as a markdown list grouped by topic.
```

#### Agent 1b: Dependency Context (one total)

```
subagent_type: Explore
prompt: |
  Extract dependency details relevant to {{LIBRARY}} and {{CLI}}.

  Dependencies to examine:
  <DEPS_LIST>

  For each dependency, report:
  - How it's used in this package (which modules import it, key API calls)
  - Version constraints or feature flags enabled
  - Any behavior that documentation should mention

  Return findings as a markdown list grouped by dependency.
```

#### Agent 1c: Drift Detection (one per target doc)

```
subagent_type: Explore
prompt: |
  Compare <DOC_PATH> against current source code behavior.

  Read the document, then examine the source code it describes.

  Produce a drift report with these sections:

  ## Missing
  Features, APIs, or behaviors present in code but absent from docs.

  ## Stale
  Claims in docs that no longer match source code (cite both doc line and code line).

  ## Incorrect
  Factual errors in docs contradicted by source code.

  ## Correct
  Claims that are verified accurate (brief list, no detail needed).
```

**Batching**: If you have N target docs, you need N code-evidence agents + 1 dependency agent + N drift agents = 2N+1 total. Launch in batches of 3, prioritizing one code-evidence + one drift for the same doc in each batch to maximize useful pairs.

### Stage 2: Parallel Drafting (general-purpose agents)

Launch **up to 3 `general-purpose` agents** concurrently per batch. Each agent drafts content for one target document.

**Critical**: Agents must return proposed content as text in their response. They must NOT write files directly. The orchestrator handles all file writes in Stage 3.

```
subagent_type: general-purpose
prompt: |
  Draft updated documentation for <DOC_PATH>.

  ## Current Content
  <CURRENT_FILE_CONTENTS>

  ## Scope
  This is a <SCOPE_TYPE> document:
  - breadth: high-level overview, use cases, links to deeper docs
  - depth: architecture, module structure, key crates, tradeoffs, decisions

  ## Evidence
  <STAGE_1_EVIDENCE_FOR_THIS_DOC>

  ## Drift Report
  <STAGE_1_DRIFT_FOR_THIS_DOC>

  ## Dependency Context
  <STAGE_1_DEPENDENCY_FINDINGS>

  ## Rules
  1. Update only what evidence supports — do not invent features, flags, or behavior.
  2. Keep existing useful content and maintain structural coherence.
  3. Preserve the document's existing voice and formatting conventions.
  4. Evidence precedence: source code + tests > generated artifacts > existing docs.

  ## Response Format

  Return your response with these sections:

  ### PROPOSED_CONTENT
  The complete updated document content (full file, not a diff).

  ### CHANGES
  - What was changed and why (bullet list)

  ### EVIDENCE_USED
  - path:line — brief note (bullet list)

  ### OPEN_ISSUES
  - Any missing context or unresolved questions (bullet list, or "None")
```

**Batching**: If more than 3 target docs, launch in batches of 3. Each batch must complete before the next starts (each agent needs the full Stage 1 evidence context).

### Stage 3: Merge and Consistency (Orchestrator — you)

This stage runs entirely in the orchestrator context. Do NOT launch sub-agents.

1. **Collect** all drafts from Stage 2 agent responses.
2. **Review** each proposed content for:
   - Cross-document conflicts (same feature described differently).
   - Terminology consistency (feature names, crate names, command names).
   - Scope violations (breadth doc with too much depth, or vice versa).
3. **Resolve** conflicts using evidence precedence: source code + tests > generated artifacts > existing docs.
4. **Write** each finalized document using the Edit or Write tool.
5. **Record** which files were changed and which were left unchanged.

### Stage 4: Parallel Validation (general-purpose agents)

Launch **up to 3 `general-purpose` agents** concurrently to validate the written docs.

#### Agent 4a: Claim Validation

```
subagent_type: general-purpose
prompt: |
  Validate technical claims in these documents:
  <LIST_OF_CHANGED_DOC_PATHS>

  For each document, read it and verify that every non-trivial technical claim
  (API names, behavior descriptions, CLI flags, feature descriptions) can be
  confirmed by source code.

  Report:
  ## PASSED
  - Claims verified (brief count per doc)

  ## FAILED
  - Claim text — why it cannot be verified (per doc)
```

#### Agent 4b: Scope Validation

```
subagent_type: general-purpose
prompt: |
  Validate scope compliance for these documents:
  <LIST_OF_CHANGED_DOCS_WITH_SCOPE_TYPES>

  Rules:
  - Root README.md files must be breadth-focused (overview, links, use cases).
  - Sub-package README.md files must be depth-focused (architecture, modules, tradeoffs).

  Report:
  ## PASSED
  - Documents that comply with their scope type

  ## FAILED
  - Document path — scope violation description
```

#### Agent 4c: Link and Completeness Validation

```
subagent_type: general-purpose
prompt: |
  Validate links and completeness for these documents:
  <LIST_OF_ALL_TARGET_DOC_PATHS>

  Check:
  1. All markdown links point to files/anchors that exist.
  2. All relative paths are valid from the document's location.
  3. Every target document is accounted for (changed or explicitly unchanged).

  Report:
  ## BROKEN_LINKS
  - doc_path: link text -> target (why broken)

  ## MISSING_DOCS
  - Any target docs not addressed

  ## PASSED
  - Count of valid links checked per doc
```

**Repair loop**: If any validator reports failures:
1. Route **only affected documents** back through a single Stage 2 drafting pass.
2. Apply the Stage 2 agent's proposed fix via Stage 3 merge.
3. **Do not re-validate**. If the repair pass doesn't resolve the issue, flag it in the final summary for manual review.

## Final Deliverable Format

After all edits are complete, provide a per-file summary for every target document:

1. `<path>` - `changed` or `unchanged`
2. What changed (or why no changes were needed)
3. Key source evidence used
4. Any unresolved limitations or follow-ups

If nothing changed in a particular file, explicitly say so.
