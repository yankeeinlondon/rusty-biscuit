---
phases: 4
created: 2026-06-02
start_phase: 1
source_files_during_phase_1:
  - sniff/lib/src/remote/types.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/bitbucket.rs
docs_updated_during_phase_1:
  - sniff/docs/repos/repo-tech-design.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/remote/github.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/cli/src/output/remote.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - sniff/cli/src/output/remote.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - sniff
---

# Execution Plan — `sniff repo remote` PR count + CI/CD runs

This plan implements open PR counts in the stats line and a compact CI/CD workflow runs table for the `sniff repo remote` command, focusing on GitHub Actions support first.

## Phase 1: Library Data Model and Trait Update

Update the core remote inspection types and trait to support workflow runs.

- [ ] Add `head_branch` and `event` fields to `CiCdInfo` in `sniff/lib/src/remote/types.rs`.
- [ ] Update `detect_cicd` implementations in `github.rs`, `gitlab.rs`, `gitea.rs`, and `bitbucket.rs` to initialize new fields to `None`.
- [ ] Add `list_workflow_runs` method to `RemoteRepoProvider` trait in `sniff/lib/src/remote/provider.rs` with a default implementation returning an empty `Vec`.
- [ ] Update `RemoteRepoProvider::fetch_report` in `sniff/lib/src/remote/provider.rs` to:
    - Fetch workflow runs via `list_workflow_runs(owner, repo, 5)`.
    - Fall back to `detect_cicd` presence detection only if no runs are found.

## Phase 2: GitHub Provider Implementation

Implement GitHub-specific workflow run fetching using the `schematic-schema` client.

- [ ] Implement `map_workflow_run` helper in `sniff/lib/src/remote/github.rs` to convert `schematic_schema::github::WorkflowRun` to `CiCdInfo`.
- [ ] Implement `list_workflow_runs` for `GitHubRemote` in `sniff/lib/src/remote/github.rs`.
    - Use `ListWorkflowRunsRequest`.
    - Implement anonymous fallback pattern (retry on 401/403/MissingCredential).
- [ ] Add unit tests for `map_workflow_run` in `sniff/lib/src/remote/github.rs`.

## Phase 3: CLI Rendering - Stats and Helpers

Update the remote report output to include PR counts and relative time formatting.

- [ ] Implement `relative_time(iso: &str, now: DateTime<Utc>) -> String` helper in `sniff/cli/src/output/remote.rs`.
- [ ] Update `render_remote_text` in `sniff/cli/src/output/remote.rs`:
    - Relabel issues stat: `◎ N open` → `◎ N issues`.
    - Add PR count: `⇄ N PRs` (using dim `⇄` glyph).
- [ ] Add unit tests for `relative_time` with fixed `now` baseline.

## Phase 4: CLI Rendering - CI/CD Table

Render a rich table for CI/CD runs while maintaining fallback for presence-only detection.

- [ ] Update `render_cicd` in `sniff/cli/src/output/remote.rs`:
    - Detect "run-shaped" entries (where `started_at` is `Some`).
    - Render a `Table` with columns: Conclusion (glyph), Workflow, Branch, Event, Time.
    - Implement conclusion-to-glyph mapping (`success` -> `✓`, `failure` -> `✗`, etc.).
    - Maintain `UnorderedList` fallback for presence-only detection.
- [ ] Add unit tests for CI/CD table rendering in `sniff/cli/src/output/remote.rs`.
- [ ] Final validation: run `cargo test -p sniff -p sniff-cli` and verify output manually if possible.
