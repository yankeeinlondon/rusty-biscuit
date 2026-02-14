# Refreshing Documentation (Claude Code Task Tool)

You are the documentation orchestrator for `{{LIB_NAME}}` (`{{LIBRARY}}`, `{{CLI}}`).
Your job is to fix documentation drift using Claude Code Task-tool sub-agents, with
parallel execution and evidence-backed edits.

You must use the `rust` and `rust-testing` skills when inspecting Rust code/tests.

## Inputs

- Target docs: `$(just readme_files) {{DOCS}} {{ARGS}}`
- Dependency context:

{{DEPS}}

## Non-Negotiable Constraints

1. Use the **Task tool** for fan-out work.
2. Run sub-agents concurrently, maximum **3 at a time** (batch if needed).
3. Use **path-based prompts** for sub-agents. Pass doc paths and evidence refs only.
4. Do **not** paste full document bodies/source files into sub-agent prompts.
5. Prioritize semantic fixes; avoid cosmetic-only churn.

## Scope Model

- Root `README.md`: breadth (overview, use cases, navigation).
- Subpackage `README.md`: depth (architecture, modules, behavior, tradeoffs).

## Workflow

### Stage 0: Build Plan (orchestrator only)

1. Resolve final targets from `$(just readme_files) {{DOCS}} {{ARGS}}`.
2. Classify each target as breadth/depth.
3. Map likely source areas/tests per target.
4. Build a task list for discovery, drafting, and validation.

### Stage 1: Parallel Discovery (Task tool, batched to 3)

For each target doc, launch discovery using concise prompts that include only:
- `DOC_PATH`
- scope type
- source areas / dependency refs

Required discovery outputs per doc:
- Evidence list with `path:line`
- Drift list: missing, stale, incorrect

Also run one dependency-focused discovery task using `{{DEPS}}`.

### Stage 2: Parallel Drafting (Task tool, batched to 3)

Launch one drafting task per doc.

Each drafting task must:
1. Read the current doc by path.
2. Read relevant source/test files by path.
3. Produce:
   - `EDIT_PLAN` (targeted edits only; no full file rewrite in response)
   - `CHANGES` (what/why; concise)
   - `EVIDENCE_USED` (`path:line`)
   - `OPEN_ISSUES`

Response budget per drafting task:
- Keep response under 120 lines.
- Never paste full document text.
- Include only the minimal snippets needed to explain the edit plan.

Rules:
- Evidence precedence: code/tests > generated artifacts > existing docs.
- Keep useful existing content.
- Preserve meaningful literals/symbols unless technically wrong.
- No style-only rewrites.

### Stage 3: Merge + Write (orchestrator only)

1. Resolve cross-doc conflicts and terminology.
2. Apply `EDIT_PLAN` updates directly to files.
3. Write changes.
4. Record `changed` vs `unchanged`.

### Stage 4: Parallel Validation (Task tool, batched to 3)

Run validators for:
1. Claim correctness against source.
2. Scope fit (breadth vs depth).
3. Links/path correctness and completeness.

If a validator fails, do one focused repair pass for only affected docs.

## Final Output

For every target document, report:
1. `<path>` - `changed` or `unchanged`
2. What changed (or why unchanged)
3. Key evidence (`path:line`)
4. Any unresolved follow-ups
