---
phases: 4
created: 2026-05-25
start_phase: 1
---

# Feature / Fix Lifecycle Execution Plan

This plan formalizes the relationship between features/fixes and topic docs, introducing a new lifecycle closure workflow via `just complete`.

## Phase 1: Foundation (Documentation & Schemas)
- [ ] Create `docs/schemas/episodic-entry.schema.json` with the canonical schema for `episodic.jsonl`.
- [ ] Create `docs/feature-lifecycle.md` at the repo root.
  - Detail `blast_radius` semantics.
  - Include examples (new feature, modified feature, fix).
  - Include a walked-through `just complete` transcript.
- [ ] Update `CLAUDE.md` to add the "Feature / Fix Lifecycle" section.
  - Explain the 3 states of `blast_radius`.
  - Detail frontmatter additions (`design_docs`, `commits`).
  - Add pointer to `docs/feature-lifecycle.md`.

## Phase 2: Tooling & Infrastructure
- [ ] Update `sniff` to support branch-based commit listing (`sniff repo commits --since-branch-from=main`).
  - Ensure it produces a structured list suitable for agent consumption.
- [ ] Create a closure prompt template for the claudine agent.
  - Specify instructions for commit filtering, doc identification, diff generation, and summary writing.
- [ ] Create a utility script for robust, idempotent upsert of JSONL entries (to handle `episodic.jsonl` writes safely via temporary files).

## Phase 3: Enhanced Lifecycle Recipe (`just/lifecycle.just`)
- [ ] Implement `just complete-dry-run` recipe.
  - Select dir, gather inputs, invoke claudine, but skip writes and move.
  - Print proposed metadata and JSONL entry.
- [ ] Update `just complete` recipe.
  - **Verification**: Ensure dir is non-empty.
  - **Gather Inputs**: `spec.md`, candidate commits (via sniff), and topic docs.
  - **Agent Invocation**: Call claudine with closure prompt to analyze and propose updates.
  - **User Review**: Prompt user to accept the proposed docs/metadata.
  - **Write Metadata**: Add `design_docs` and `commits` to `spec.md` frontmatter.
  - **JSONL Upsert**: Write the new entry to `{area}/{features|fixes}/episodic.jsonl`.
  - **Move**: Move the directory to `_completed/`.

## Phase 4: Validation & Acceptance
- [ ] Test the full `just complete-dry-run` flow on a dummy feature.
  - Verify no files are modified and the dir is not moved.
- [ ] Test `just complete` on a dummy feature.
  - Ensure `episodic.jsonl` upserts properly.
  - Verify frontmatter is correct and preserved.
  - Verify dir moves to `_completed/`.
- [ ] Verify idempotency by re-running the JSONL upsert logic manually.
- [ ] Test with a spec lacking frontmatter entirely to ensure it's created correctly.
