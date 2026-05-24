---
phases: 5
created: 2026-05-23
start_phase: 1
source_files_during_phase_1:
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/git/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - sniff/cli/README.md
  - sniff/lib/README.md
  - sniff/cli/CHANGELOG.md
  - sniff/lib/CHANGELOG.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - sniff
---

## Phase 1 - Library Data Model and Discovery

- [ ] Define the public library shape for listing worktrees in `sniff/lib/src/filesystem/git/worktree.rs`, including a worktree entry type with `name`, `branch`, `path`, `is_current`, and detached-HEAD state.
- [ ] Implement a library function that discovers the repository from a caller-provided base directory and returns all worktrees, including the main worktree.
- [ ] Name the main worktree from its workdir directory basename instead of treating it as anonymous.
- [ ] Mark exactly one entry as current by comparing canonicalized worktree paths against the discovered current repository workdir.
- [ ] Detect detached HEAD separately from branch name so verbose output can render the required fallback text.
- [ ] Sort returned entries alphabetically by worktree name in the library so all CLI formats share the same ordering.
- [ ] Export the new listing API from `sniff/lib/src/filesystem/git/mod.rs`.

Validation checkpoint:

- [ ] Add or update unit tests in `sniff/lib/src/filesystem/git/worktree.rs` covering main-only repos, repos with linked worktrees, current highlighting from the main worktree, current highlighting from a linked worktree, alphabetical sorting, and detached HEAD.
- [ ] Run `cargo test -p sniff-lib filesystem::git::worktree --color=never`.

Parallelizable after the public entry type is defined:

- [ ] Add detached-HEAD fixture setup in tests while the discovery implementation is being completed.
- [ ] Add path canonicalization edge-case tests while the discovery implementation is being completed.

## Phase 2 - CLI Argument Wiring

- [ ] Add a `repo worktrees` subcommand in `sniff/cli/src/args/repo.rs`.
- [ ] Add `--list`, `--csv`, and `--verbose` support for `repo worktrees`, reusing existing CLI conventions where possible.
- [ ] Decide and encode conflicts or precedence between `--list` and `--csv`; prefer matching the existing `FileListArgs` behavior where `--list` wins when both are provided unless the local clap style already enforces conflicts.
- [ ] Add the new action variant to `sniff/cli/src/args/mod.rs` and wire conversion from parsed args to `RepoAction`.
- [ ] Update argument parser tests for `repo worktrees`, `repo worktrees --list`, `repo worktrees --csv`, and `repo worktrees --verbose`.

Validation checkpoint:

- [ ] Run `cargo test -p sniff-cli args:: --color=never`.

Parallelizable after Phase 1 API names are stable:

- [ ] Add parser tests while CLI command execution is being implemented.
- [ ] Update help text and command examples while output rendering is being implemented.

## Phase 3 - CLI Output and JSON Behavior

- [ ] Add the `RepoAction::Worktrees` handler in `sniff/cli/src/commands/mod.rs` that calls the new library API instead of reconstructing git state in the CLI.
- [ ] Implement JSON output for `repo worktrees --json` with enough structured data for each worktree: `name`, `branch`, `path`, `current`, and detached-HEAD status.
- [ ] Implement default text output as a plain list of worktree names, visually marking the current worktree with a stable indicator.
- [ ] Implement `--list` output as one item per line with a `- ` prefix, preserving the current-worktree visual marker.
- [ ] Implement `--csv` output as comma-separated worktree names on a single line, preserving deterministic alphabetical order.
- [ ] Implement `--verbose` text output as `{worktree} (on {branch} branch, located at {path})` with the detached-HEAD fallback required by the spec.
- [ ] Render verbose output using `biscuit-terminal` Renderable components, using `Prose` where practical, so styling, wrapping, hyperlinks, and plain fallback behavior stay consistent.
- [ ] Render verbose paths as OSC8 hyperlinks with a plain fallback.
- [ ] Display paths under the user's home directory with `~` substitution while retaining the actual path as the hyperlink target.
- [ ] Ensure `--plain` strips styling and hyperlink escape codes without changing the user-visible words.

Validation checkpoint:

- [ ] Add CLI integration tests in `sniff/cli/tests/cli.rs` for default, `--list`, `--csv`, `--verbose`, `--json`, `--plain --verbose`, current marker from main worktree, current marker from linked worktree, and detached HEAD.
- [ ] Run `cargo test -p sniff-cli repo_worktrees --color=never`.

Parallelizable after Phase 1 API names are stable:

- [ ] Implement JSON outcome builder tests in `sniff/cli/src/output/repo_json.rs` while text rendering is being implemented.
- [ ] Implement text rendering tests while command dispatch is being implemented.

## Phase 4 - Documentation and Public Behavior Updates

- [ ] Update `sniff/cli/README.md` command examples to include `sniff repo worktrees`.
- [ ] Update the CLI JSON shape table in `sniff/cli/README.md` with the `repo worktrees` JSON response.
- [ ] Update `sniff/lib/README.md` only if the new worktree listing API is public and intended for direct library consumers.
- [ ] Update any relevant feature or changelog notes if this repository tracks unreleased CLI behavior there.
- [ ] Confirm all new documentation uses US English spelling and matches the implemented flags exactly.

Validation checkpoint:

- [ ] Run `rg -n "repo worktrees|worktrees" sniff/cli/README.md sniff/lib/README.md sniff/features/_unscheduled/repo-worktrees`.
- [ ] Run `cargo test -p sniff-cli --doc --color=never` if README-linked doctests or examples were changed.

Parallelizable after Phase 3 behavior is decided:

- [ ] Update README command examples while integration tests are being written.
- [ ] Update JSON documentation while JSON tests are being written.

## Phase 5 - Full Verification and Cleanup

- [ ] Run targeted library tests for git worktree discovery.
- [ ] Run targeted CLI tests for `repo worktrees` and existing `repo worktree` to confirm no regression.
- [ ] Run formatting with the repository's standard Rust formatter.
- [ ] Run the package-area lint or nearest available focused lint command for `sniff`.
- [ ] Manually inspect `sniff repo worktrees`, `sniff repo worktrees --list`, `sniff repo worktrees --csv`, `sniff repo worktrees --verbose`, `sniff repo worktrees --plain --verbose`, and `sniff repo worktrees --json` in a repository with at least one linked worktree.
- [ ] Confirm behavior on main worktree and linked worktree paths, including current marker placement.
- [ ] Review the diff to ensure changes are limited to the library API, CLI args, command handling, output rendering, tests, and docs required by this feature.

Validation checkpoint:

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo test -p sniff-lib --color=never`.
- [ ] Run `cargo test -p sniff-cli --color=never`.
- [ ] Run the local `just` recipe for the sniff package area if one exists and is scoped enough for this change.
