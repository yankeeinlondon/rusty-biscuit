---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-filesystem-watchers/worktree/fixes/2026-10-10-valid-paths/spec.md"
plan: "worktree/fixes/2026-10-10-valid-paths/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - worktree/lib/src/config.rs
  - worktree/lib/src/error.rs
  - worktree/cli/src/commands/create.rs
  - worktree/cli/src/exit.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - worktree
  - worktree-cli
---

# Implementation Log for 2026-10-10-valid-paths (5 phases)

## Phase 1

- started: error surface (`BaseDirSource` + source-aware `WorktreeError` variants)
- added `BaseDirSource { Env, ConfigFile(PathBuf), Prompt }` to `lib/src/config.rs`
  (`Debug, Clone, PartialEq, Eq`; plain-text `Display`: "the WT environment
  variable" / "the config file <path>" / "the setup prompt") so messages read
  "Base directory 'X' from <origin> ...".
- added variants `BaseDirectoryEmpty`, `BaseDirectoryForeignPath`,
  `BaseDirectoryNotAbsolute`, `BaseDirectoryNotDirectory`,
  `BaseDirectoryInvalidEncoding { value: PathBuf, .. }`; reshaped
  `BaseDirectoryNotFound` to `{ value, origin }`. All carry `origin`, never
  `source`.
- **departure from the plan's Phase 1 list (additive):** also added
  `BaseDirectoryUnreadable { value, origin, #[source] cause: io::Error }`,
  because the spec requires "other I/O failures retain their cause and name
  the path and origin" and no listed variant could carry both. Phase 2's
  resolver should construct it for non-NotFound metadata errors.
- **reshaped `BaseDirectoryIsGitRepo(String)` to `{ value, origin }`** so the
  repository-root message identifies `WT` vs the setup prompt (spec: "retain
  the existing repository-root variants, but ensure their messages ... identify
  the origin"). `ConfigBaseDirIsGitRepo { config_path, dir }` keeps its shape
  (create.rs pattern-matches it) and its message now names the config path.
  Both say ".git directly inside", matching the root-only check (Ruling 2).
- remedies are origin-specific private helpers in `error.rs`: `WT` → set or
  unset it, and "Editing ~/.worktree.json alone does not override a nonempty
  WT"; config file → set `base_dir` in the actual path, or remove the file and
  rerun `wt create` in an interactive terminal; prompt → choose an absolute
  directory. Each includes a host-valid example (`/Users/me/wt`, `/home/me/wt`,
  `C:\Users\me\wt`). `BaseDirectoryNotFound` adds the restore-the-mount
  advice. `BaseDirectoryNotAbsolute` adds the full-home-path hint for values
  starting with `~` in every source (Ruling 1). Foreign-path text is
  "uses Windows path syntax on this POSIX host" (or the reverse) and never
  calls the value absolute.
- updated constructors: `resolve_base_dir` (Env / ConfigFile origins; the
  config not-found error now reports the raw `base_dir` text and the config
  path) and the setup prompt's repository-root error (`Prompt`). No other
  behavior change in this phase.
- tests (L1): `worktree error::tests::*` — 9 tests: origin display; the
  incident message `b:\wt` from a config file names value, path, remedy and
  host/foreign style; `b:wt` not called absolute; `WT` remedy text; `~/wt`
  hint from all three origins (and absent for `./wt`); missing dir names config
  + mount recovery; every value variant names value/origin/remedy; the
  unreadable variant keeps its `io::Error` source; config repo-root message.
  `worktree-cli exit::tests::every_other_error_is_a_failure` extended with every
  new/reshaped variant plus `ConfigInvalidFormat`/`ConfigBaseDirIsGitRepo`
  (exit 1 through the `_ => FAILURE` arm).
- gates: `just test` (worktree/) 1113 passed, 14 skipped; `just lint` clean.
  No cross-OS run: the only OS-dependent code is `cfg!`-selected message text,
  and the tests branch on the same `cfg!`; CI covers Linux/macOS on the PR.
- not done here (by plan): the `create.rs` config branches still render and
  `return Ok(())` (Phase 3); comments saying "non-git directory" / "inside a
  git repo" are left for the Phase 4 drift sweep.
