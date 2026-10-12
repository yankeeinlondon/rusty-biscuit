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
source_files_during_phase_2:
  - Cargo.lock
  - worktree/lib/Cargo.toml
  - worktree/lib/src/config.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/worktree/testing.md
source_files_during_phase_3:
  - worktree/lib/src/config.rs
  - worktree/lib/src/error.rs
  - worktree/cli/src/commands/create.rs
  - worktree/cli/src/commands/mod.rs
  - worktree/cli/src/main.rs
  - worktree/cli/tests/base_dir_fixture/mod.rs
  - worktree/cli/tests/create_base_dir.rs
  - worktree/cli/tests/create_config_errors.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/worktree/testing.md
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

## Phase 2

- dependency wiring: `worktree/lib/Cargo.toml` enables biscuit-file's
  `file-reference` feature (still `default-features = false`) and adds
  `test-toolkit` as a dev-dependency for `EnvGuard`. `cargo tree -p worktree`
  shows one `gix v0.84.0`.
- **departure (spelling only):** `biscuit_file::file_reference::context` is a
  private module; the helper is used through its public re-export
  `biscuit_file::file_reference::home_dir` (same function the plan named).
- pure validator in `lib/src/config.rs`: private `PathStyle { Posix, Windows }`
  and `is_foreign_syntax(raw, style)` (POSIX: ASCII letter + `:`, or leading
  `\\`; Windows: leading `/`), called only after the native
  `Path::is_absolute()`. Public `validate_base_dir(raw, origin)`:
  blank-after-trim → `BaseDirectoryEmpty`; host-absolute → `Ok(PathBuf::from(raw))`
  (spelling preserved, no trimming/expansion/unquoting/joining); foreign →
  `BaseDirectoryForeignPath`; else `BaseDirectoryNotAbsolute`.
- resolver: `resolve_base_dir()` delegates to private
  `resolve_with(wt: Option<OsString>, config_path: Option<&Path>, probe)`.
  The `WT` value is a parameter (read by the public fn with `var_os`) so the
  filesystem and matrix tests need no process-env mutation; precedence tests
  still go through the public fn. The probe is `impl Fn(&Path) ->
  io::Result<bool>` ("is a directory"), real implementation
  `fs::metadata(..).is_dir()` (follows symlinks). Shared `check_base_dir`:
  syntax → probe (`Ok(false)` → `NotDirectory`, `NotFound` → `NotFound`, other
  → `BaseDirectoryUnreadable` keeping the `io::Error`) → `.git` directly
  underneath (file or folder; `ConfigBaseDirIsGitRepo` for the config origin,
  `BaseDirectoryIsGitRepo` otherwise). Config side reads bytes (`NotFound` →
  `BaseDirectoryNotConfigured`, other I/O → `Io`), decodes UTF-8 (failure →
  `ConfigInvalidFormat`), parses, then `check_base_dir`.
- **behavior note:** the config side now checks existence before the `.git`
  check (spec order); previously `.git` was checked first. A missing path
  could never have a `.git` child, so only error precedence for an existing
  directory is unchanged.
- **departure from Ruling 4 (duplicate key):** serde's *derived* struct
  deserializer rejects a repeated field (`duplicate field \`base_dir\``), so a
  duplicate `base_dir` is `ConfigInvalidFormat`, not last-wins. This is
  stricter than the plan stated and never silently picks one of two values;
  the matrix test pins it. The plan's matrix table row is left as decided;
  Phase 4 docs must state the actual behavior.
- writer: `save_config(&Path)` → private `save_config_to(config_path, base_dir)`:
  `to_str` first (non-Unicode → `BaseDirectoryInvalidEncoding`), validate with
  `ConfigFile(config_path)` origin, then serialize and write. No directories
  are created. Saved text is the lossless `to_str` value (was lossy
  `display()`).
- `config_path()` and `considered_dirs()` now use biscuit-file `home_dir()`
  (env-first: `HOME`, `USERPROFILE` on Windows; must be absolute).
- tests (all L1, in-process, `worktree config::tests::*`, 28 on unix):
  - `foreign_syntax_helper_classifies_both_styles_on_every_host` — every spec
    table row, both `PathStyle`s, on every host.
  - `foreign_syntax_helper_never_flags_host_absolute_lookalikes_alone`.
  - `validator_matches_the_spec_table_on_this_host` — native acceptance per
    host column (Windows rows proven only on a Windows run).
  - `validator_errors_carry_the_value_and_every_origin` — empty, whitespace,
    relative, foreign × `Env`/`ConfigFile`/`Prompt`.
  - `validator_preserves_spaces_and_never_expands_text` — trailing space kept,
    leading space rejected as relative, `~/wt`, `$HOME/wt`,
    `%USERPROFILE%\wt`, quoted value never expanded.
  - resolver/filesystem: `config_value_resolves_to_its_directory`,
    `missing_config_file_means_not_configured`,
    `config_value_that_does_not_exist_names_the_config_file`,
    `config_value_naming_a_file_is_not_a_directory`,
    `repository_roots_are_rejected_with_a_git_folder_or_file` (both origins),
    `a_directory_inside_a_repository_is_allowed`,
    `a_symlink_to_a_directory_is_allowed` (unix),
    `unreadable_state_keeps_its_cause_and_is_not_called_missing` (injected
    PermissionDenied, both origins), `probe_results_map_to_not_found_and_not_directory`,
    `syntax_is_checked_before_any_filesystem_probe`,
    `the_incident_value_from_the_config_names_value_file_and_host`.
  - precedence (public `resolve_base_dir`, `serial(wt_env)`, `EnvGuard` on
    `HOME`, `USERPROFILE`, `WT`): `config_lookups_use_the_environment_home`,
    `unset_or_empty_wt_falls_back_to_the_config`,
    `invalid_nonempty_wt_blocks_a_valid_config` (whitespace, relative, `~/wt`,
    foreign, missing), `valid_wt_wins_over_a_malformed_config`,
    `non_unicode_wt_is_rejected_without_fallback` (unix bytes / windows lone
    surrogate).
  - writer: `invalid_save_leaves_a_missing_config_absent` (and nothing else
    created), `invalid_save_leaves_an_existing_config_byte_for_byte_unchanged`,
    `save_does_not_create_the_directory`,
    `non_unicode_save_is_rejected_without_writing` (unix/windows),
    `save_and_reload_round_trips_repeatedly` (read/write/read, bytes stable).
  - matrix: `config_base_dir_robustness_matrix` walks a real
    `save_config_to`-written fixture with one edit per cell (absent, null,
    number/object/array, empty, whitespace, duplicate, trailing garbage,
    non-UTF-8 byte, `wt`, `./wt`, `~/wt`, foreign) between control rows;
    `wt_robustness_matrix` covers control, unset, empty, whitespace, relative ×3,
    foreign, non-Unicode against a valid config fixture.
- smell grep on the non-test part of `config.rs`: no `#[serde(default)]`,
  `unwrap_or_default`, `.ok()`, or `filter_map`; the only `Option`s are the
  home lookup and `WT` (where absent and empty are spec-defined as the same).
- gates: `just test` (worktree/) 1138 passed, 14 skipped (one pre-existing
  `git_graph::tests::old_forks_and_merges_stay_drawn_with_exact_elided_runs`
  took 6.9 s under full-suite load; not touched here). `just lint` clean.
- CLI not yet changed (Phase 3): `create.rs` still compiles against the
  unchanged `save_config(&Path)` signature; the setup prompt still calls
  `create_dir_all` before validating.
- cross-OS: `just cross-check worktree --os windows config::tests` on
  build-win-native → 27 passed (the unix-only symlink test is not compiled
  there). This proves the spec table's Windows column natively
  (`//server/share/wt`, `\\server\share\wt`, `\\?\C:\wt`,
  `\\?\UNC\server\share\wt`, `B:/wt` accepted; `b:wt`, `b:`, `\wt` not
  absolute; `/home/me/wt` foreign), plus the lone-surrogate `WT`/save cases.
  Linux/WSL2 share the POSIX column already proven on macOS; CI covers them.
  Note: `just cross-check` cannot pass a nextest `-E '…'` filterset (the
  parentheses break its `bash -c` quoting); a plain name filter works.
- skill: added the config-home isolation fact to `.claude/skills/worktree/testing.md` (both `HOME` and `USERPROFILE`; cache lookups unchanged). Fuller `cli-contracts.md` updates stay in Phase 4.

## Phase 3

- started: create-command rework (Wave 5), then the two integration test files (Wave 6).
- lib additions in `lib/src/config.rs`:
  - `pub fn wsl_drive_hint(value, RuntimeEnvironment) -> Option<String>`:
    pure; `Some` only under WSL1/WSL2 for drive-absolute values (`B:\wt`,
    `B:/wt`, `D:\`), mapping to `/mnt/<lowercase letter>/<rest with />`.
    The sentence is conditional ("If it is mounted there, use /mnt/b/wt.").
    `None` for `b:wt`, `b:`, `\\server\…`, `//server/…`, `\\?\…`, POSIX paths.
  - `example_base_dir()` moved from a private helper in `error.rs` to a public
    function in `config.rs`, so the CLI's JSON example and the lib remedies
    share one host-valid example path.
- `BaseDirectoryNotConfigured`'s message (lib `error.rs`) now gives script-usable
  instructions: set `WT` to an absolute directory (host example), or create
  `~/.worktree.json` with `{"base_dir": "<absolute directory>"}`; it notes that
  running in an interactive terminal offers setup (conditional, not a promise).
- `cli/src/commands/create.rs`:
  - `run` enters setup only for `BaseDirectoryNotConfigured` **and**
    `crate::env::is_interactive()`; every other resolution error (including
    `ConfigInvalidFormat` / `ConfigBaseDirIsGitRepo`) is returned, so it exits 1
    through `exit.rs`'s `_ => FAILURE` arm. The two `return Ok(())` branches are
    gone.
  - **design choice for Ruling 7 (render once):** rendering moved out of the
    command. `pub fn base_dir_error_markup(&WorktreeError) -> Option<String>`
    (re-exported from `commands`) builds the rich markup for every
    base-directory and config error; `main`'s error handler uses it before the
    plain `error_markup` fallback. So each error is printed once, by `main`, to
    stderr, with no `cd:` line. The private `_with` variant takes the runtime
    detector and the setup candidates as closures for tests.
  - markup: `Prose::escape_text` on the whole library message (not
    `escape_text_outside_code_spans`, so backticks in a user value cannot form
    a code span); a `ConfigFile` origin appends an `Edit ~/.worktree.json` link
    whose href is `Prose::quoted_attr(actual path)`; a `BaseDirectoryForeignPath`
    appends `wsl_drive_hint(value, detect_runtime_environment())`. The detector
    runs only in that arm (unit test injects a panicking detector for the others).
  - invalid-format text: example JSON uses the host example path (backslashes
    doubled on Windows); recovery is "delete the file and run wt create again in
    an interactive terminal". **departure:** the old text interpolated the branch
    (`wt create <branch>`); dropped as unnecessary.
  - repository-root text now says "a directory without .git directly inside it"
    (root-only check, Ruling 2) instead of "Worktrees cannot be stored inside a
    git repo".
  - setup loop: `trait SetupPrompt { select, enter_path, reject }`, thin
    `InquirePrompt` adapter (`Select::raw_prompt` index, so an "Other" label can
    never collide with a candidate), and `choose_base_dir(prompt, candidates,
    save)`. Each choice goes through `prepare_base_dir(raw)`:
    `validate_base_dir(raw, Prompt)` → metadata (existing file →
    `NotDirectory`; other I/O → `BaseDirectoryUnreadable`, stops) → create when
    missing (failure → `WorktreeError::Io` with the path in its text, stops) →
    re-check is-dir → `.git` directly inside → `BaseDirectoryIsGitRepo{Prompt}`.
    Only then is `save` called. Value errors (`Empty`, `ForeignPath`,
    `NotAbsolute`, `NotDirectory`, `IsGitRepo`) are shown via `reject` and the
    user asked again: an invalid suggestion returns to the menu, an invalid
    typed path asks for another path (Esc there cancels → exit 0). Typed text
    is trimmed; suggestions are not.
  - candidates: repository roots are filtered as before, and non-Unicode
    candidate paths are omitted (lossless `to_str`), so a shown choice is saved
    exactly as shown.
- tests (all L1):
  - lib `config::tests::wsl_drive_hint_suggests_the_default_mount_for_drive_absolute_values`,
    `…_is_never_offered_natively_or_for_other_spellings`.
  - CLI unit (`commands::create::tests::*`, run under both `worktree_cli::` and
    `wt::`; no snapshots): `invalid_typed_path_then_valid_one_saves_only_the_valid_one`
    (incident value then a temp dir; only the valid one saved/created; foreign
    value never created relative to cwd), `invalid_typed_path_then_cancel_saves_and_creates_nothing`
    (foreign, `wt`, `./wt`, `~/wt`, blank), `a_home_relative_typed_path_gets_the_full_path_hint`,
    `unusable_suggestions_return_to_the_menu_before_anything_is_saved` (file,
    repo root, then a missing dir that is created then saved),
    `a_path_that_cannot_be_created_stops_setup_without_saving`,
    `setup_candidates_skip_repository_roots`,
    `config_errors_link_the_config_file_and_show_values_literally`,
    `env_and_prompt_errors_have_no_config_link`,
    `foreign_drive_paths_get_the_wsl_hint_only_under_wsl`,
    `runtime_is_detected_only_for_a_foreign_path`,
    `invalid_format_markup_names_the_file_and_offers_no_unconditional_setup`,
    `other_errors_are_left_to_the_plain_renderer`.
  - integration, shared fixture `cli/tests/base_dir_fixture/mod.rs` (temp repo
    + temp home; `HOME` and `USERPROFILE` both set; `WT` removed unless given;
    `NO_COLOR=1`; stdin null; `WT_SHELL_WRAPPER=1` so an absent `cd:` line is
    meaningful; `bin_exe!("wt")`; asserts exit 1, empty stdout, unchanged repo
    entries / branches / `git worktree list --porcelain`):
    - `create_base_dir.rs`: `the_fixture_home_config_is_the_one_wt_reads`
      (control: valid config succeeds), `a_foreign_saved_base_dir_is_refused_without_changing_anything`
      (incident replay: `b:\wt` on POSIX / `/home/me/wt` on Windows; config bytes
      unchanged), `an_invalid_wt_is_refused_even_when_the_config_is_valid`
      (foreign, `wt`, whitespace; no fallback), `a_missing_saved_base_dir_names_the_config_and_the_mount`.
    - `create_config_errors.rs`: malformed config and repository-root config each
      exit 1 with the message rendered exactly once; `markup_in_a_saved_value_is_shown_literally`;
      `the_config_link_survives_a_home_path_with_markup_characters` (unix only:
      Windows file names cannot hold `"<>`; href quoting is also unit-tested);
      `no_configuration_and_no_terminal_exits_1_without_setup` (no prompt text,
      no config file created).
  - all new tests would fail on the pre-Phase-3 code (old branches exited 0,
    setup ran without a terminal check, no hint, unescaped `escape_text_outside_code_spans`).
- gates: `just lint` (worktree/) clean after one clippy fix
  (`cloned_ref_to_slice_refs` in a test). `just test` (worktree/) 1173 passed,
  14 skipped; 15 SLOW (>5 s) reports, all pre-existing list/remove/graph tests
  under full-suite load, none touched here.
- cross-OS: `just cross-check worktree-cli --os windows …` on build-win-native:
  all `commands::create::tests` (both module paths) and all 8 Windows-compiled
  integration tests pass. Linux/WSL share the POSIX code paths proven on macOS;
  CI covers them.
- incident during manual verification (no repository damage): a hand replay of
  the binary inherited this shell's real `WT=/Volumes/coding/wt`, so it created
  a worktree at `/Volumes/coding/wt/repo/fix-wsl` for a throwaway `/tmp` repo.
  Inspected (only that directory, created seconds earlier) and removed it. Added
  the "`env_remove("WT")` for a `wt create` child" fact to the worktree skill's
  `testing.md`.
- left for Phase 4: README/docs (`docs/base-directory.md`), `cli-contracts.md`,
  dependency docs, `create_worktree` doc prerequisite, remaining drift sweep.
