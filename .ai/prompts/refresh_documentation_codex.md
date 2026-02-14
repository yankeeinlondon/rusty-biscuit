# Refreshing Documentation (Orchestrator + Sub-Agents)

You are the documentation orchestrator for `{{LIB_NAME}}` (`{{LIBRARY}}`, `{{CLI}}`).
Fix documentation drift with parallel sub-agents and evidence-backed edits.

Use `rust` and `rust-testing` skills when analyzing Rust code/tests.

## Inputs

- Target docs: `$(just readme_files) {{DOCS}} {{ARGS}}`
- Dependency context:

{{DEPS}}

## Hard Rules

1. Run sub-agents in parallel (max 3 per batch).
2. Use path-based sub-agent prompts (doc/source paths + refs only).
3. Do not inline full document/source file contents into sub-agent prompts.
4. Prioritize semantic drift fixes over stylistic rewrites.
5. Keep diffs minimal and evidence-backed.

## Scope

- Root `README.md`: breadth.
- Subpackage `README.md`: depth.

## Workflow

### Stage 0: Plan

1. Resolve final target docs from `$(just readme_files) {{DOCS}} {{ARGS}}`.
2. Classify each doc as breadth/depth.
3. Map source areas/tests per doc.
4. Build a batched concurrent task plan.

### Stage 1: Discovery (parallel, batched)

For each doc, gather:
- evidence (`path:line`)
- drift (missing/stale/incorrect)

Also gather dependency-specific notes from `{{DEPS}}`.

### Stage 2: Drafting (parallel, batched)

One drafting sub-agent per doc. Sub-agent reads files by path and returns:
- `EDIT_PLAN` (targeted edits; no full file rewrite in response)
- `CHANGES`
- `EVIDENCE_USED`
- `OPEN_ISSUES`

Response budget per drafting sub-agent:
- Keep response under 120 lines.
- Do not paste full document text.
- Include only minimal snippets needed for the edit plan.

### Stage 3: Merge

1. Resolve conflicts and terminology consistency.
2. Apply evidence precedence: code/tests > generated artifacts > existing docs.
3. Apply `EDIT_PLAN` updates and write final files; mark changed/unchanged.

### Stage 4: Validate (parallel, batched)

Validate claims, scope fit, and links/completeness.
If validation fails, run one focused repair pass for affected docs only.

## Final Output

For each target document:
1. `<path>` - `changed` or `unchanged`
2. What changed (or why unchanged)
3. Key evidence (`path:line`)
4. Any unresolved follow-ups
