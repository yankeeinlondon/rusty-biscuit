---
phases: 6
created: 2024-05-12
start_phase: 1
---

# Execution Plan: `GitGraph` Component

This plan outlines the steps to promote the inline git graph view from the `worktree` CLI into a reusable, composable component in `biscuit-terminal`.

## Phase 1: Core Component & Typed Builder

Establish the foundational types and the primary construction path (typed builder and raw Mermaid ingestion).

- [ ] Define core types in `biscuit-terminal/lib/src/components/git_graph/mod.rs`:
    - `GitGraph` struct
    - `GitGraphCommit`, `GitGraphBranch`
    - `GitGraphCaps`, `GitGraphAuto`, `GitGraphKind`
    - `VerboseDetail`
- [ ] Implement the identifier sanitizer in `biscuit-terminal/lib/src/components/git_graph/sanitize.rs`.
    - Handle punctuation replacement, leading-digit prefixing, and collision numbering.
- [ ] Implement the typed builder API in `biscuit-terminal/lib/src/components/git_graph/builder.rs`.
    - Delegate Mermaid ID generation to the sanitizer.
- [ ] Implement `GitGraph::from_mermaid` in `mod.rs`.
    - Include best-effort commit counting for width heuristics.
- [ ] Register `git_graph` module in `biscuit-terminal/lib/src/components/mod.rs`.
- [ ] **Validation:** Unit tests in `tests.rs` for `from_mermaid`, sanitizer rules, and basic builder output.

## Phase 2: Auto-Configuration Logic
Port the complex scenario selection and Mermaid construction algorithms from the `worktree` CLI.

- [ ] Implement `GitGraph::auto` in `biscuit-terminal/lib/src/components/git_graph/auto.rs`.
- [ ] Port "Focused Branch" algorithm:
    - Context commits, branch commits, mainline commits, and placeholder `HEAD`.
- [ ] Port "Base Overview" algorithm:
    - Mainline window, branch anchoring, and sorting.
- [ ] Implement cap enforcement and input trimming within `auto()`.
- [ ] Implement `VerboseDetail` population for the focused-branch scenario.
- [ ] **Validation:** Unit tests for `GitGraph::auto` covering both scenarios, over-cap input, and edge cases (detached HEAD, empty mainline).

## Phase 3: Terminal & Browser Rendering

Implement the `Renderable` and `BrowserRenderable` traits to enable consistent output across targets.

- [ ] Implement `Renderable` for `GitGraph` in `mod.rs`.
    - Delegate to `MermaidDiagram`.
    - Implement width policy heuristic (60ch/80ch/120ch/160ch based on commit count).
    - Implement narrow-terminal fallback (return fenced Mermaid block if width < 80).
- [ ] Implement `BrowserRenderable` for `GitGraph` in `biscuit-terminal/lib/src/components/git_graph/browser.rs`.
    - Handle `<svg>` wrapping and CSS variable substitution (`--gitgraph-bg`, `--gitgraph-fg`, etc.).
- [ ] **Validation:** Integration tests asserting on Mermaid instructions string and CSS variable inclusion in browser output.

## Phase 4: Opt-in Git Collection Feature

Add the ergonomic but optional path for callers that want to render directly from a repository path.

- [ ] Add `git` feature to `biscuit-terminal/lib/Cargo.toml` (default disabled).
- [ ] Implement `GitGraph::from_repo` in `biscuit-terminal/lib/src/components/git_graph/collect.rs`.
    - Shell out to `git` via `std::process::Command`.
    - Collect current branch, default branch, recent commits, worktree branches, and merge-base.
- [ ] Define `GitGraphCollectError` using `thiserror`.
- [ ] **Validation:** Feature-gated tests using a temporary git repository to verify collection logic.

## Phase 5: CLI Subcommand & Consumer Migration
Update the `biscuit-terminal` CLI and migrate the `worktree` CLI to use the new shared component.

- [ ] Update `biscuit-terminal/cli/Cargo.toml` to enable `features = ["git"]`.
- [ ] Add `git-graph` subcommand to `biscuit-terminal/cli/src/commands/git_graph.rs`.
- [ ] Migrate `worktree/cli/src/commands/git_graph.rs` to call `GitGraph::auto`.
- [ ] Delete redundant Mermaid emission and formatting code in `worktree`.
- [ ] **Validation:** Smoke test `biscuit-terminal git-graph` and verify `worktree list` (or equivalent) still renders the graph correctly.

## Phase 6: Documentation & Final Polish
Ensure the new component is well-documented and the monorepo remains consistent.

- [ ] Update `biscuit-terminal/lib/README.md` with `GitGraph` usage examples.
- [ ] Update `worktree/cli/README.md` to reflect the migration.
- [ ] Add rustdoc documentation for all public types following the monorepo convention (Summary -> Examples -> Returns -> Errors).
- [ ] **Validation:** Run `cargo test --all-features` and `just lint` across the workspace.
