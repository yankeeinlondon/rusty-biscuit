---
phases: 6
created: 2026-05-25
start_phase: 1
---

# Plan: Comment Quality Rubric and Cleanup

This plan implements the comment quality rubric and performs a cleanup pass on the `claudine` package area as specified in `features/2026-05-25-comment-quality/spec.md`.

## Phase 1: Policy and Reference Documentation

Establish the authoritative rules and provide reference examples for the new comment quality standards.

- [ ] **Task 1.1: Update `CLAUDE.md`**
    - Add "Comment Quality" section immediately after "Rustdoc Convention".
    - Include the 8 anti-patterns and 5 positive criteria as a concise list.
    - Add the authoring-discipline rule.
    - Link to `docs/comment-quality.md`.
- [ ] **Task 1.2: Create `docs/comment-quality.md`**
    - Expand each anti-pattern and positive criterion with real before/after examples from the codebase.
    - Cite file paths for examples to calibrate judgment.
- [ ] **Task 1.3: Seed Topic Doc Links**
    - Update module-level (`//!`) docs in `claudine/lib/src/` for high-traffic areas (composition, mcp, protect, system_prompt, stream) to link to their topic docs in `claudine/docs/topics/`.

## Phase 2: Heuristic Tooling

Create a low-friction script to help identify suspicious comment patterns.

- [ ] **Task 2.1: Implement `scripts/check-comments.sh`**
    - Implement checks for:
        - Long docblocks (>15 lines) on small functions (<10 lines).
        - Literal `## Arguments` headers.
        - Heavy-setup doc examples (>20 lines).
        - Redundant accessor docs (sequential short `///` on simple `pub fn` getters).
    - Ensure output is parseable (file:line:category).
- [ ] **Task 2.2: Add `just` recipe**
    - Add `check-comments` to the root `justfile` to invoke the script.
- [ ] **Task 2.3: Initial Baseline Run**
    - Run `just check-comments` on the current state of `claudine/` to identify cleanup targets.

## Phase 3: Claudine Library Cleanup (High-Density Targets)

Perform the first pass of manual cleanup on modules identified as having high concentration of anti-patterns.

- [ ] **Task 3.1: Cleanup `prompt_reporting`**
    - Target: `claudine/lib/src/prompt_reporting/*`.
    - Focus: HOW-narration, format/color narration, stale comments.
- [ ] **Task 3.2: Cleanup `dispatch`**
    - Target: `claudine/lib/src/dispatch/template.rs` and `loader.rs`.
    - Focus: Redundant accessor docs, tautological examples.
- [ ] **Task 3.3: Cleanup `stream`**
    - Target: `claudine/lib/src/stream/reporting.rs` and `badges.rs`.
    - Focus: Section-marker `//` comments, HOW-narration.
- [ ] **Task 3.4: Cleanup `config`**
    - Target: `claudine/lib/src/config/claudine_config.rs`.
    - Focus: Redundant docs, field doc duplication in `## Arguments`.

## Phase 4: Claudine Library Cleanup (Remaining Sweep)

Complete the cleanup of the library crate.

- [ ] **Task 4.1: Sweep Remaining `claudine/lib/src/`**
    - Review all other `.rs` files in module order.
    - Apply rubric anti-patterns and positive criteria.
- [ ] **Task 4.2: Add `#[allow(missing_docs)]` where necessary**
    - Apply to impl blocks when redundant accessor docs are removed and clippy requires them.
- [ ] **Task 4.3: Verify Library Build and Docs**
    - Run `cargo test -p claudine`.
    - Run `cargo doc -p claudine` and check for intra-doc link warnings.

## Phase 5: Claudine CLI Cleanup

Apply the rubric to the CLI crate.

- [ ] **Task 5.1: Cleanup CLI Core**
    - Target: `claudine/cli/src/main.rs`.
- [ ] **Task 5.2: Cleanup CLI Modules**
    - Target: `claudine/cli/src/argv/`, `completion/`, and `commands/`.
- [ ] **Task 5.3: Verify CLI Build and Docs**
    - Run `cargo test -p claudine-cli`.
    - Run `cargo doc -p claudine-cli`.

## Phase 6: Final Validation

Ensure the codebase meets the acceptance criteria and the tooling reports zero findings.

- [ ] **Task 6.1: Final Heuristic Check**
    - Run `just check-comments` on `claudine/`.
    - Ensure zero findings (or document/fix any remaining outliers).
- [ ] **Task 6.2: Documentation Verification**
    - Confirm `CLAUDE.md` and `docs/comment-quality.md` are correct and links work.
- [ ] **Task 6.3: Behavior Check**
    - Confirm no functional changes were introduced during the comment cleanup.
