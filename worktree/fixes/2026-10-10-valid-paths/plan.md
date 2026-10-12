---
total_phases: 5
created: 2026-10-11
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
source_files_during_phase_1:
  - worktree/lib/src/config.rs
  - worktree/lib/src/error.rs
  - worktree/cli/src/commands/create.rs
  - worktree/cli/src/exit.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - Cargo.lock
  - worktree/lib/Cargo.toml
  - worktree/lib/src/config.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/worktree/testing.md
packages:
  - worktree
  - worktree-cli
---

# Plan: Valid base-directory paths

## Work Summary

### Work Required

Close the wrong-OS base-directory incident in the `worktree` package area by
making `wt` accept only absolute paths on the current host, at every source:
the `WT` environment variable, `~/.worktree.json`, and interactive setup.

- **Error surface** (`worktree/lib/src/error.rs`, `worktree/lib/src/config.rs`):
  a public `BaseDirSource` origin enum (`Env`, `ConfigFile(PathBuf)`, `Prompt`)
  and source-aware `WorktreeError` variants — `BaseDirectoryEmpty`,
  `BaseDirectoryForeignPath`, `BaseDirectoryNotAbsolute`,
  `BaseDirectoryNotDirectory`, `BaseDirectoryInvalidEncoding`, and a reshaped
  `BaseDirectoryNotFound { value, origin }` — each naming the value, origin,
  and remedy in plain text.
- **Pure validation** (`config.rs`): public
  `validate_base_dir(raw: &str, origin: BaseDirSource) -> Result<PathBuf, WorktreeError>`
  performing no filesystem access or host discovery, backed by a private
  pure foreign-syntax helper parameterized on `PathStyle` (`Posix`/`Windows`).
  The host's `Path::is_absolute()` is the acceptance authority.
- **Resolver rework** (`config.rs`): `WT` read with `var_os` (unset or exactly
  empty = no override; any other invalid value is a hard error with no
  fallback), JSON `base_dir` validated before filesystem state, metadata
  classification (`NotFound` / `NotDirectory` / I/O with cause), the existing
  `.git` root-only check retained, and the home lookup switched from
  `dirs::home_dir` to biscuit-file's `home_dir()` (enabling its
  `file-reference` feature on the worktree dependency).
- **Writer hardening** (`config.rs`): `save_config` validates syntax before
  serializing or opening its destination; invalid input leaves an existing
  config byte-for-byte unchanged and creates neither a config file nor
  directories.
- **CLI behavior** (`worktree/cli/src/commands/create.rs`, `main.rs`):
  interactive setup gated on `env::is_interactive()`; both suggested and typed
  setup choices validated before any `create_dir_all`/`save_config`; invalid
  choices re-prompt, genuine I/O failures stop, cancel still exits 0; the two
  config-error branches that currently render richly and `return Ok(())` must
  exit 1; WSL drive-path hint (`/mnt/b/wt`) computed by a pure helper and
  shown only after a foreign drive-path rejection with runtime detection
  deferred to that error path.
- **Tests**: all L1 — library table tests for both path styles on every host,
  precedence and environment-guard tests, resolver/writer filesystem tests
  through a config-path-parameterized private helper with an injectable
  metadata probe, the input robustness matrix below, and CLI integration
  tests reproducing the incident in a temporary repository and home.
- **Documentation**: README "Directory Resolution" rewrite, a new
  `worktree/docs/base-directory.md` topic page, worktree-skill
  `cli-contracts.md` updates, dependency documentation, and a drift sweep of
  comments that say "inside a git repo" where the check is root-only.

### Definition of Success

- The incident replay: with `WT` unset and `~/.worktree.json` holding `b:\wt`
  on a POSIX host (or `/home/me/wt` on Windows), `wt create fix/wsl` exits 1,
  prints a stderr diagnostic that names the value, the config file, and a
  host-valid remedy (with the `/mnt/b/wt` hint under WSL), emits no `cd:`
  line, creates no directory or branch, registers no worktree, and leaves the
  config bytes unchanged.
- An invalid nonempty `WT` (whitespace-only, relative, foreign, or
  non-Unicode) always errors; it never falls back to a valid saved config,
  and an unset or exactly empty `WT` still falls back.
- Interactive setup never calls `create_dir_all` or `save_config` before
  syntax validation succeeds; a noninteractive invocation never prompts and
  exits 1 with instructions.
- `just test` and `just lint` pass from `worktree/`; target-specific
  acceptance rides the repository's existing coverage workflow (a pull
  request proves Linux and macOS; push adds Windows; nightly adds WSL2).
- The input robustness matrix below is covered by tests that walk a real
  `save_config`-written fixture with one edit per cell plus a control row,
  asserted through the public resolution result.

## Phase 1 — Rulings and Error Foundations

### Necessary Rules

The specification's required behavior already uses its three open questions'
recommended options as the baseline; these rulings adopt them and settle the
remaining ambiguities. A different author choice requires corresponding
requirement and test changes before implementation continues.

1. **Home-relative references (`~/wt`)** — reject with a full-path hint, in
   every source (`WT`, JSON, prompt). No tilde expansion anywhere in this
   fix; the prompt error for a literal `~/wt` suggests entering the full
   home-directory path.
2. **Repository containment** — keep the root-only check (a `.git` file or
   folder directly under the base directory). Directories merely inside a
   repository remain allowed. Correct the drifted "inside a git repo" prose
   in comments and docs to state the actual root-only check.
3. **Missing saved directory** — hard error with source-aware recovery text;
   no automatic setup restart, no directory recreation, config preserved.
4. **Duplicate JSON keys** — serde_json's last-wins behavior is retained for
   `base_dir` (JSON does not forbid duplicates); validation applies to the
   surviving value. Recorded per the matrix below.
5. **Non-UTF-8 config file bytes** — map to `ConfigInvalidFormat` (the text
   API requires Unicode); real I/O failures keep their `std::io::Error`
   cause. Read bytes, then decode, so the two are distinguishable.
6. **`WT` blankness and encoding** — unset and exactly empty (`OsStr` length
   0) both mean "no override"; a whitespace-only value is
   `BaseDirectoryEmpty`; a non-Unicode value is `BaseDirectoryInvalidEncoding`.
   Blankness is judged after Unicode conversion; environment and JSON values
   are otherwise preserved exactly (trimming only detects blanks), and only
   manually entered prompt text is trimmed.
7. **Exit codes** — every new error reaches exit 1 through `exit.rs`'s
   existing `_ => FAILURE` arm; `Cancelled` stays 0. The two `create.rs`
   branches that custom-render `ConfigInvalidFormat` and
   `ConfigBaseDirIsGitRepo` must exit 1 without rendering the error twice
   (for example by extending `main`'s already-rendered match arms); their
   recovery text gains host-valid example paths and must not promise
   interactive setup to a noninteractive caller.
8. **Dependency boundary** — only the configuration functions in `config.rs`
   switch to `biscuit_file::file_reference::context::home_dir()` (with the
   `file-reference` feature enabled on the worktree dependency, which keeps
   `default-features = false`). `dirs` is retained for `api_preference.rs`
   and any CLI use. Cache and remote-store locations are untouched.
9. **WSL hint placement** — suggestion formatting is a pure helper (in the
   lib's config module) taking the value plus an injected
   `sniff::os::runtime::RuntimeEnvironment`; the CLI calls
   `detect_runtime_environment()` only on the foreign drive-path error branch.
   The hint appears only for drive-absolute values (`B:\wt`, `B:/wt`), is
   explicitly conditional on the drive being mounted, and never fires for
   `b:wt`, `b:`, network shares, or namespace paths. No test needs a real
   WSL mount.
10. **Setup-loop testability** — the setup decision loop is extracted behind
    a thin prompt adapter so scripted-choice tests drive a fake; the
    `inquire` adapter stays thin. No L2 terminal tests are added.
11. **Resolver testability** — the filesystem-reading core is a private
    helper taking the config path and an injectable metadata probe, so I/O
    error classification is tested without brittle permission fixtures. The
    public `resolve_base_dir()` delegates to it with the real home-derived
    path.
12. **`BaseDirSource` definition site** — defined in the config module (not
    `error.rs`) with `Debug`, `Clone`, `PartialEq`, `Eq`, and a plain-text
    `Display`; error variants carry it under the field name `origin`, never
    `source` (thiserror reserves `source` for underlying errors).

### Spikes

None. The specification explicitly rules out a performance spike, benchmark,
extra CI matrix, and L2 terminal test for these short string checks, and
defers WSL detection to the error path so valid resolution gains no
discovery cost. The only technical unknown — whether enabling biscuit-file's
`file-reference` feature alongside `default-features = false` compiles
cleanly — is a compile check folded into Phase 2 Wave 1; both crates pin
`gix = "=0.84.0"`, so no second gix copy is built.

### Tasks

Wave 1 (single task; everything downstream depends on these types):

- [x] **Error surface**
    - Add `BaseDirSource` (`Env`, `ConfigFile(PathBuf)`, `Prompt`) to
      `worktree/lib/src/config.rs` with `Debug`, `Clone`, `PartialEq`, `Eq`,
      and a plain-text `Display` (`WT` / the config path / the prompt).
    - Extend `WorktreeError` in `worktree/lib/src/error.rs`:
      `BaseDirectoryEmpty { origin }`, `BaseDirectoryForeignPath { value,
      origin }`, `BaseDirectoryNotAbsolute { value, origin }`,
      `BaseDirectoryNotDirectory { value, origin }`,
      `BaseDirectoryInvalidEncoding { value: PathBuf, origin }`; change
      `BaseDirectoryNotFound(String)` to `{ value, origin }`.
    - Each variant's `#[error]` display names the value (when available),
      the origin, and a remedy; foreign-path text says the syntax belongs to
      another OS and never claims `b:wt` is absolute on Windows.
    - Update every constructor, match, and test touched by the reshaped
      variants (including `exit.rs`'s failure-list test); retain the
      repository-root variants with origin-identifying messages.
    - Complexity: `BaseDirectoryNotFound` is matched in existing lib and CLI
      tests; grep before editing. No deprecation needed (early-stage repo).

**Checkpoint:** `just test worktree` and `just lint` pass with the new error
surface and no behavior change yet beyond compiled-in variants.

## Phase 2 — Library Validation and Config Hardening

Wave 2 (two independent tasks, parallelizable):

- [x] **Dependency wiring**
    - `worktree/lib/Cargo.toml`: add `features = ["file-reference"]` to the
      existing `biscuit-file` dependency (keeping `default-features =
      false`); add `test-toolkit` as a dev-dependency for `EnvGuard`.
    - Verify the workspace still resolves one `gix` copy (both crates pin
      `=0.84.0`) and `cargo check -p worktree` succeeds.
- [x] **Pure validator**
    - Private `PathStyle` (`Posix`/`Windows`) plus a private pure helper that
      recognizes foreign syntax: for nonabsolute POSIX input, an ASCII letter
      followed by `:` or an initial backslash pair (including `\\?\` and
      `\\?\UNC\` prefixes); for nonabsolute Windows input, a leading `/`.
      Host style comes from `cfg!(windows)`; call it only after the native
      `Path::is_absolute()` check.
    - Public `validate_base_dir(raw, origin)`: empty/whitespace-only →
      `BaseDirectoryEmpty`; host-absolute → `Ok` preserving spelling;
      foreign-syntax → `BaseDirectoryForeignPath`; else
      `BaseDirectoryNotAbsolute`. No filesystem access, CWD joining,
      canonicalization, quote stripping, or variable expansion.
    - Unit tests in `config.rs` covering the spec's full input table for
      **both** styles on **every** host (including `//server/share/wt`
      accepted on POSIX, `b:`/`b:wt` rejected as not-absolute on Windows,
      `\wt` not-absolute on both, and a colon/backslash inside a POSIX
      filename not making the path foreign); plus tests that spaces in
      nonblank absolute values survive and `~`, quotes, and `$VAR` text are
      never expanded.

Wave 3 (single task; reworks the same file as Wave 2's validator):

- [x] **Resolver rework**
    - Read `WT` with `var_os`: absent or exactly empty OsStr → fall through;
      non-Unicode → `BaseDirectoryInvalidEncoding`; otherwise validate
      syntax, then metadata (`fs::metadata`, which follows symlinks —
      symlinks to directories remain allowed): missing →
      `BaseDirectoryNotFound`, existing non-directory →
      `BaseDirectoryNotDirectory`, other I/O → retain cause and name path
      and origin; then the existing `.git`-directly-underneath check (file or
      folder).
    - Config side: read bytes, decode (invalid UTF-8 →
      `ConfigInvalidFormat`, real I/O keeps its cause), parse (malformed or
      missing/non-string field stays `ConfigInvalidFormat`), then validate
      `base_dir` syntax **before** any filesystem state; then metadata and
      `.git` checks as above, with `ConfigFile(path)` as the origin.
    - Extract the filesystem-reading core as a private helper taking the
      config path and an injectable metadata probe; `resolve_base_dir()`
      delegates with the real path.
    - Switch `config_path()` and `considered_dirs()` to
      `biscuit_file::file_reference::context::home_dir()`.
    - Precedence tests (guarded by the existing `serial(wt_env)` group and
      `test_toolkit::EnvGuard` restoring prior values even on panic):
      unset/empty `WT` falls back to config; invalid nonempty `WT` blocks a
      valid config; valid `WT` wins over malformed config. Filesystem tests
      through the private helper: relative/foreign JSON values, missing
      path, existing file, repository roots with `.git` files and folders,
      valid directory — asserting origin and actual config path on failures;
      I/O classification via the injected probe; non-Unicode `WT` value
      rejected with no fallback where the target permits constructing one.

Wave 4 (single task; completes the config module and the matrix):

- [x] **Writer hardening and matrix**
    - `save_config`: `to_str` the path first (unrepresentable →
      `BaseDirectoryInvalidEncoding` with no write), validate syntax before
      serializing or opening the destination, use the actual config path as
      `ConfigFile(path)` in errors, remain a writer that creates no
      directories.
    - Tests: invalid save leaves a missing config absent and an existing
      config byte-for-byte unchanged; save + reload round-trips a valid
      absolute path.
    - Add the robustness-matrix test below: one test per input side walking
      a real `save_config`-written fixture with one edit per cell, asserted
      through the public resolution result, with an unedited control row
      proving the positive path.

**Checkpoint:** `just test` and `just lint` from `worktree/` pass; the
matrix table below is fully covered; `grep` finds no `#[serde(default)]`,
`Option<T>`, `unwrap_or_default()`, or `.ok()` on the `base_dir` read path.

### Input Robustness Matrix

The load-bearing field is the base-directory value, read from two sides:
`base_dir` in `~/.worktree.json` (JSON, the only supported format) and the
`WT` environment variable. Every outcome is asserted through
`resolve_base_dir` (via the private config-path helper) or `save_config`,
never a parser's return value.

| Shape                        | `base_dir` in `~/.worktree.json`                                     | `WT` environment value                                        |
| ---------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------- |
| absent                       | missing key → `ConfigInvalidFormat`; never read as empty              | unset → no override, fall back to config                      |
| explicit null                | `"base_dir": null` → `ConfigInvalidFormat`; not conflated with absent | n/a (env has no null; empty OsStr is spec-defined absence)     |
| wrong type, whole field      | `123`, `{}`, `[]` → `ConfigInvalidFormat`                             | n/a                                                           |
| wrong type, one element      | n/a (scalar string field)                                             | n/a                                                           |
| wrong type, every element    | n/a (scalar string field)                                             | n/a                                                           |
| empty                        | `""` → parses, then `BaseDirectoryEmpty { origin: ConfigFile }`       | exactly empty → no override (spec-defined absence)            |
| whitespace-only              | `"  "` → `BaseDirectoryEmpty`                                         | `"  "` → `BaseDirectoryEmpty { origin: Env }`                  |
| duplicate key                | last-wins per serde_json; surviving value validated (Ruling 4)        | n/a                                                           |
| trailing or invalid content  | valid JSON + garbage → `ConfigInvalidFormat`                          | n/a                                                           |
| non-Unicode                  | invalid UTF-8 file bytes → `ConfigInvalidFormat` (Ruling 5)           | non-UTF-8 value → `BaseDirectoryInvalidEncoding`               |
| relative text                | `wt`, `./wt`, `~/wt` → `BaseDirectoryNotAbsolute`                     | same, `origin: Env`                                           |
| foreign text                 | `b:\wt` on POSIX → `BaseDirectoryForeignPath`                         | same, `origin: Env`                                           |
| control (unedited fixture)   | real `save_config` output, existing absolute dir → `Ok`               | absolute existing dir → `Ok`                                   |

## Phase 3 — CLI Gating, Setup Loop, and Diagnostics

Wave 5 (single task; one coherent rework of the create path):

- [x] **Create command rework**
    - Gate setup on `crate::env::is_interactive()`; when it fails, return
      `BaseDirectoryNotConfigured` whose rendered instructions work for
      scripts (exit 1, no prompt, no `cd:` line).
    - Extract the setup decision loop behind a thin prompt adapter;
      validate **both** suggested and typed choices with
      `validate_base_dir(.., Prompt)` before any `create_dir_all` or
      `save_config`; reject an existing file or repository root before
      saving; when absent, create, then verify directory-and-not-repo-root
      before saving; convert selected candidates losslessly (`to_str`)
      before creation.
    - Invalid choices show the reason (with the full-path hint for `~/wt`)
      and return to selection or text entry; genuine I/O failures stop with
      an error; canceling keeps `Cancelled` and exit 0.
    - Fix the two custom-rendered config branches to exit 1 without double
      rendering (Ruling 7), with host-valid example paths and no interactive
      promise in noninteractive text.
    - Render the new source-aware errors through `Prose` /
      `TerminalRenderable`: user-controlled values escaped, link attributes
      quoted, config errors name `~/.worktree.json` and link the actual
      config path, diagnostics to stderr, no `cd:` line.
    - Wire the WSL hint: pure `wsl_drive_hint(value, RuntimeEnvironment)`
      helper in the lib config module (drive-absolute values only,
      `/mnt/b/wt` spelling conditional on the mount), called from the CLI
      only in the foreign drive-path error branch via
      `sniff::os::runtime::detect_runtime_environment()`; unit-test the
      helper with injected WSL/native results.
    - Unit tests (in-source, non-snapshot; remember shared modules compile
      twice under both `worktree_cli::` and `wt::` paths): scripted-choice
      loop tests — invalid then valid saves only the valid choice; invalid
      then cancel saves nothing; hint never offered for `b:wt`, `b:`,
      network shares, or namespace paths.

Wave 6 (two independent test files, parallelizable):

- [ ] **Incident integration tests**
    - New `worktree/cli/tests/` file: temporary repository plus temporary
      home with **both** `HOME` and `USERPROFILE` pointed at the fixture
      (adopting the home helper makes the lookup env-first); `WT` removed;
      invalid saved `base_dir` (`b:\wt` on POSIX, `/home/me/wt` on Windows);
      `NO_COLOR=1`, closed stdin, archive-safe
      `biscuit_test_harness::bin_exe!("wt")`.
    - Assert: exit 1; stderr names the config and remedy; no `cd:` line;
      config bytes unchanged; no unexpected directory inside the repository;
      no new branch; no added worktree registration.
    - Also cover: invalid `WT` overriding a valid config (exit 1, origin
      `WT`, remedy says editing JSON alone will not override), and a missing
      base directory with source-aware recovery text.
- [ ] **Config error and noninteractive tests**
    - Malformed config and config pointing at a repository root exit 1 after
      their custom rendering (rendered exactly once); user-provided markup
      in values is shown literally; config-link attributes are quoted.
    - Neither source configured plus closed stdin: exit 1, no setup prompt
      output.

**Checkpoint:** `just test` and `just lint` from `worktree/` pass, including
the new integration tests on the development host.

## Phase 4 — Documentation and Drift

Wave 7 (three independent tasks, parallelizable):

- [ ] **User documentation**
    - Rewrite the README "Directory Resolution" section: absolute-path and
      directory rules, empty-`WT` fallback, foreign-path errors, setup retry
      behavior, exit-1 semantics.
    - Create `worktree/docs/base-directory.md` (there is no existing
      configuration topic page): `WT`, `~/.worktree.json`, precedence,
      host-valid examples including WSL, setup, each rejection and its
      recovery; state the repository-root-only check accurately without
      claiming all in-repository paths are blocked; written for a developer
      new to this repository; no reference to this fix's snapshot.
- [ ] **Skill and dependency documentation**
    - Update `.claude/skills/worktree/cli-contracts.md` with validation,
      precedence, exit behavior, home isolation (configuration lookups only;
      cache/store lookups unchanged), and WSL's POSIX path rule; update
      testing notes affected by the new config-home lookup.
    - Update the repository dependency documentation for the enabled
      `biscuit-file` `file-reference` feature and the `test-toolkit`
      dev-dependency.
- [ ] **Comment and doc drift sweep**
    - Amend `create_worktree`'s documentation in
      `worktree/lib/src/worktree.rs` to state explicitly that callers must
      resolve and validate `base` beforehand.
    - Correct prose in setup paths and docs that says "inside a git repo"
      where the check is root-only (Ruling 2); review `///`/`//!` docs of
      every behavior-changing function (resolution, writing, setup,
      create-error handling) in the same change.

**Checkpoint:** every behavior named in the spec is discoverable in `docs/`
and the README; `git diff` of doc-only tasks shows no source changes.

## Phase 5 — Verification and Acceptance

Wave 8 (sequential final validation):

- [ ] **Full local validation**
    - Run `just test` and `just lint` from `worktree/` (implementation-time
      requirement from the spec).
    - Grep the config read path for the matrix smells: `#[serde(default)]`
      on `base_dir`, `Option<T>` where absent/null must differ,
      `filter_map(.. as_str())`, `unwrap_or_default()`, `.ok()` on a
      load-bearing parse — none may remain.
    - Confirm comment/code agreement on every function this fix touched.
- [ ] **Cross-OS acceptance**
    - Target-specific acceptance rides the existing coverage workflow: the
      pull request proves Linux and macOS (macOS also on this development
      host), push to `main` adds native Windows, the nightly schedule adds
      WSL2; the `ci:all-os` label opts in earlier. Native Windows acceptance
      must exercise the table's Windows rows (network-share and
      extended-length syntax, drive-relative rejection) — load the `os`
      skill before claiming a host cannot produce evidence. No new CI
      matrix is added.
- [ ] **Implementation log**
    - Write `worktree/fixes/2026-10-10-valid-paths/implementation-log.md`
      recording what was built and any departure from the spec (docs are
      corrected to match reality; the spec is left saying what was decided).
    - Terminal state: implementation complete, ready for review; do not move
      the fix to `_completed` and do not run `just complete`.

**Final gate:** all checkpoints green, matrix covered by tests, docs
updated, and the incident replay from the Work Summary passes verbatim.
