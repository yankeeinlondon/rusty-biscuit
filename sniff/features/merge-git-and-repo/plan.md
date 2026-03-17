# Implementation Plan: Merge `sniff git` into `sniff repo`

Derived from `spec.md` and `tech-design.md`. References current code as of 2026-03-16.

## Progress

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1 | **COMPLETE** | All steps 1.1–1.10 done. Build clean (0 warnings). 86 unit tests pass, 92/93 integration tests pass (1 pre-existing failure). Dead accessor methods removed. |
| Phase 2 | **COMPLETE** | All print_* functions converted to render_* returning String. `render_text()` + `emit_text()` added to mod.rs. commands.rs uses render-then-emit pattern. 86 unit tests pass, 98/99 integration tests pass (1 pre-existing failure: terminal width). |
| Phase 3 | **COMPLETE** | `--plain` flag added to Cli. Pre-scan sets NO_COLOR before clap parsing. All text output paths use `emit_text(text, cli.plain)`. Tests added. |
| Phase 4 | **COMPLETE** | AFTER_HELP rewritten with grouped short format. REPO_AFTER_HELP added with categorized examples. `disable_help_subcommand` added to root and RepoSubcommand. Help tests updated. |
| Phase 5 | **COMPLETE** | Deprecation warning emitted on stderr for `sniff git` (text mode only). `OutputFilter::Git` retained for JSON backward compatibility. Tests added for deprecation warning presence/absence. |

---

## Phase 1: Route Layer and Parser Restructure ✓

**Goal:** Introduce `CliRoute`/`RepoAction` normalization layer and reshape `Commands::Repo` to absorb git subcommands. This is the highest-risk phase because it touches parsing and execution routing simultaneously.

### 1.1 Add `RepoAction` and `CliRoute` enums

**File:** `sniff/cli/src/args.rs`

Create the normalized route types that decouple parse-time spelling from runtime semantics. Place these near the top of the file, after imports.

```rust
/// Normalized repo action — decoupled from clap parse shape.
#[derive(Debug, Clone)]
pub enum RepoAction {
    Structure { filter: Option<String>, latest_versions: bool },
    GitStatus { history: usize, refresh_remotes: bool, package: Option<String> },
    Hash { sha: String },
    StagedFiles { package: Option<String> },
    UnstagedFiles { package: Option<String> },
    UntrackedFiles { package: Option<String> },
    Remote { remote: String },
    Deps { filter: Option<String>, ui: bool },
    Packages { filter: Option<String> },
    Package,
    PackageArea,
    DirtyPackages { filter: Option<String> },
    DirtyPackageAreas { filter: Option<String> },
    PackageRoot,
    PackageAreaRoot,
    RepoRoot,
    IsCurrentPackageAreaDirty,
    PackageAreaHasSourceCodeChanges,
}
```

`CliRoute` is optional at this stage — the tech design recommends it but the critical piece is `RepoAction`. If the full `CliRoute` enum is added, it would replace `OutputFilter` as the primary dispatch type in `commands.rs`. **Recommendation:** defer full `CliRoute` to a follow-up; use `RepoAction` as the immediate bridge.

### 1.2 Expand `RepoSubcommand` with git-derived variants

**File:** `sniff/cli/src/args.rs` (lines 331–371)

Add new variants to `RepoSubcommand`:

```rust
#[derive(Subcommand, Debug, Clone)]
pub enum RepoSubcommand {
    /// Show repository structure (default when no subcommand given)
    Structure {
        /// Filter packages by name (or @area); prefix with ! to exclude
        filter: Option<String>,
    },
    /// Show git status, recent commits, and branch info
    #[command(name = "git-status")]
    GitStatus {
        /// Number of recent commits to display
        #[arg(long, default_value_t = DEFAULT_COMMIT_COUNT)]
        history: usize,
        /// Refresh remote-tracking data before reporting
        #[arg(long)]
        refresh_remotes: bool,
        /// Filter to commits and changes within a package
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    /// Show details for a specific commit by SHA
    Hash {
        /// Commit SHA (full or abbreviated)
        #[arg(value_name = "SHA")]
        sha: String,
    },
    /// List files staged for commit
    #[command(name = "staged-files")]
    StagedFiles {
        /// Filter to a specific package
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    /// List modified but unstaged files
    #[command(name = "unstaged-files")]
    UnstagedFiles {
        /// Filter to a specific package
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    /// List untracked files
    #[command(name = "untracked-files")]
    UntrackedFiles {
        /// Filter to a specific package
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    /// Inspect a remote by name, URL, or owner/repo shorthand
    Remote {
        /// Remote name (e.g., "origin"), URL, or owner/repo shorthand
        #[arg(value_name = "REMOTE")]
        remote: String,
    },
    // --- existing variants below (keep as-is) ---
    /// Render an internal dependency diagram
    Deps { ... },
    Packages { ... },
    Package,
    PackageArea,
    DirtyPackages { ... },
    DirtyPackageAreas { ... },
    PackageRoot,
    PackageAreaRoot,
    RepoRoot,
    IsCurrentPackageAreaDirty,
    PackageAreaHasSourceCodeChanges,
}
```

### 1.3 Update `Commands::Repo` — keep `filter` and `latest_versions` at repo level

**File:** `sniff/cli/src/args.rs` (lines 208–218)

No structural change to `Commands::Repo` itself — it keeps:
- `latest_versions: bool`
- `filter: Option<String>` (parse-time sugar for default structure mode)
- `repo_subcommand: Option<RepoSubcommand>`

Add `#[command(disable_help_subcommand = true)]` to `RepoSubcommand`.

### 1.4 Add normalization: `Commands::Repo` → `RepoAction`

**File:** `sniff/cli/src/args.rs`

Add a method on `Commands` (or a free function):

```rust
impl Commands {
    /// Normalize a Repo command into a RepoAction for dispatch.
    pub fn to_repo_action(&self) -> Option<RepoAction> {
        match self {
            Commands::Repo { latest_versions, filter, repo_subcommand } => {
                Some(match repo_subcommand {
                    None => RepoAction::Structure {
                        filter: filter.clone(),
                        latest_versions: *latest_versions,
                    },
                    Some(RepoSubcommand::Structure { filter: sub_filter }) => {
                        RepoAction::Structure {
                            filter: sub_filter.clone().or_else(|| filter.clone()),
                            latest_versions: *latest_versions,
                        }
                    },
                    Some(RepoSubcommand::GitStatus { history, refresh_remotes, package }) => {
                        RepoAction::GitStatus {
                            history: *history,
                            refresh_remotes: *refresh_remotes,
                            package: package.clone(),
                        }
                    },
                    Some(RepoSubcommand::Hash { sha }) => RepoAction::Hash { sha: sha.clone() },
                    Some(RepoSubcommand::StagedFiles { package }) => {
                        RepoAction::StagedFiles { package: package.clone() }
                    },
                    Some(RepoSubcommand::UnstagedFiles { package }) => {
                        RepoAction::UnstagedFiles { package: package.clone() }
                    },
                    Some(RepoSubcommand::UntrackedFiles { package }) => {
                        RepoAction::UntrackedFiles { package: package.clone() }
                    },
                    Some(RepoSubcommand::Remote { remote }) => {
                        RepoAction::Remote { remote: remote.clone() }
                    },
                    Some(RepoSubcommand::Deps { ui, filter: sub_filter }) => {
                        RepoAction::Deps {
                            filter: sub_filter.clone().or_else(|| filter.clone()),
                            ui: *ui,
                        }
                    },
                    Some(RepoSubcommand::Packages { filter: sub_filter }) => {
                        RepoAction::Packages {
                            filter: sub_filter.clone().or_else(|| filter.clone()),
                        }
                    },
                    // ... remaining variants map 1:1 ...
                })
            },
            _ => None,
        }
    }
}
```

### 1.5 Hide `Commands::Git` as compatibility alias

**File:** `sniff/cli/src/args.rs` (lines 181–206)

```rust
/// Deprecated: use `sniff repo` subcommands instead.
#[command(hide = true)]
Git { ... }
```

Keep the existing fields so parsing still works. Remove `#[command(disable_help_flag = true)]` — hidden commands don't need custom help flag handling.

Add a normalization method that maps `Commands::Git` to `RepoAction`:

```rust
pub fn git_to_repo_action(&self) -> Option<RepoAction> {
    match self {
        Commands::Git { history, refresh_remotes, package, remote, git_subcommand, .. } => {
            if let Some(remote_ref) = remote {
                return Some(RepoAction::Remote { remote: remote_ref.clone() });
            }
            match git_subcommand {
                Some(GitSubcommand::Hash { sha }) => Some(RepoAction::Hash { sha: sha.clone() }),
                Some(GitSubcommand::Staged) => Some(RepoAction::StagedFiles { package: package.clone() }),
                Some(GitSubcommand::Unstaged) => Some(RepoAction::UnstagedFiles { package: package.clone() }),
                Some(GitSubcommand::Untracked) => Some(RepoAction::UntrackedFiles { package: package.clone() }),
                None => Some(RepoAction::GitStatus {
                    history: *history,
                    refresh_remotes: *refresh_remotes,
                    package: package.clone(),
                }),
            }
        },
        _ => None,
    }
}
```

### 1.6 Rewrite `commands.rs` to dispatch on `RepoAction`

**File:** `sniff/cli/src/commands.rs`

This is the largest single change. The current flow has 5 separate git-handling blocks scattered across `run()`. Replace them with a single unified path:

1. After parsing, normalize commands to `RepoAction`:
   ```rust
   let repo_action = cli.command.as_ref().and_then(|cmd| {
       cmd.to_repo_action().or_else(|| cmd.git_to_repo_action())
   });
   ```

2. Handle `RepoAction` variants that don't need full detection as early returns:
   - `RepoAction::Remote { remote }` → call existing `handle_remote_url` / `handle_shorthand` / `resolve_remote_name` (lines 92–112 currently)
   - `RepoAction::Hash { sha }` → call existing hash logic (lines 114–141 currently)

3. Handle `RepoAction` variants that need filesystem-only detection:
   - `RepoAction::StagedFiles` / `UnstagedFiles` / `UntrackedFiles` → existing file-list logic (lines 143–190 currently)
   - `RepoAction::GitStatus` → pass through to existing `OutputFilter::Git` path, applying `--package` scoping if present

4. Handle structure-family actions through the existing `OutputFilter::Repo` path, passing `RepoAction` instead of `RepoSubcommand`.

**Key refactoring steps in `commands.rs`:**

- Remove the 3 separate `if let Some(ref cmd) = cli.command && let Some(remote_ref) = cmd.git_remote()` blocks
- Remove the `if let Some(ref cmd) = cli.command && let Some(sha) = cmd.git_hash()` block
- Remove the `if let Some(ref cmd) = cli.command && let Some(sub) = cmd.git_subcommand()` block
- Replace with a single `match repo_action { ... }` dispatch
- Extract `--refresh-remotes` and `--history` from `RepoAction::GitStatus` instead of from `Commands::Git`
- Extract `--package` from `RepoAction::GitStatus` / `StagedFiles` / `UnstagedFiles` / `UntrackedFiles`
- Extract `--latest-versions` from `RepoAction::Structure`

**Preserve:**
- The package-scoping logic (lines 260–320) — move it into a helper function `scope_git_to_package()` and call it for `GitStatus` and the file-list variants when `package.is_some()`
- The dependency enrichment logic — trigger it for `RepoAction::Structure { latest_versions: true, .. }`

### 1.7 Update `output/mod.rs` dispatch

**File:** `sniff/cli/src/output/mod.rs`

The `print_text()` function currently takes `repo_subcommand: Option<&RepoSubcommand>`. Change it to take `repo_action: Option<&RepoAction>` and update the `OutputFilter::Repo` match arm to dispatch on `RepoAction` variants instead of `RepoSubcommand` variants.

Similarly, for `OutputFilter::Git` — that entire arm should be folded into the `RepoAction::GitStatus` handling.

**Candidate approach:** replace the `OutputFilter::Git` and `OutputFilter::Repo` arms with a single `OutputFilter::Repo` arm that dispatches on `RepoAction`. `OutputFilter::Git` can be removed from the enum (or kept as a deprecated alias that maps to `Repo` in JSON filtering).

### 1.8 Remove obsolete accessor methods

**File:** `sniff/cli/src/args.rs`

These methods become dead code after the `RepoAction` migration:
- `git_remote()` (line 560)
- `git_subcommand()` (line 568)
- `git_hash()` (line 578)
- `git_package()` (line 589)
- `repo_subcommand()` (line 498)
- `repo_filter()` (line 538)

Replace with direct access through `to_repo_action()` / `git_to_repo_action()`.

Also update:
- `history()` → extract from `RepoAction::GitStatus` or return default
- `refresh_remotes()` → extract from `RepoAction::GitStatus` or `Commands::Filesystem`
- `latest_versions()` → extract from `RepoAction::Structure` or `Commands::Filesystem`

### 1.9 Update `to_output_filter()`

**File:** `sniff/cli/src/args.rs` (line 387)

Currently `Commands::Git` maps to `OutputFilter::Git`. After the merge:
- `Commands::Repo` maps to `OutputFilter::Repo` (unchanged)
- `Commands::Git` maps to `OutputFilter::Repo` (or better: the `RepoAction` dispatch bypasses `OutputFilter` entirely for repo-family commands)

### 1.10 Update parser tests

**File:** `sniff/cli/src/args.rs` (tests module at bottom)

Update existing tests:
- `git_flags_and_remote_parse` → verify hidden `git` still parses
- Add tests for all new `RepoSubcommand` variants:
  - `repo` → parses as `Structure { filter: None }`
  - `repo biscuit` → parses as `Structure { filter: Some("biscuit") }` via top-level filter
  - `repo structure biscuit` → parses as `Structure { filter: Some("biscuit") }`
  - `repo git-status --history 20` → parses correctly
  - `repo git-status --refresh-remotes --package homelab`
  - `repo hash HEAD`
  - `repo staged-files`
  - `repo unstaged-files`
  - `repo untracked-files`
  - `repo remote origin`
  - `repo remote owner/repo`
- Test normalization: verify `to_repo_action()` produces correct `RepoAction` for each case
- Test `git_to_repo_action()` compatibility alias normalization

**File:** `sniff/cli/tests/cli.rs`

Update integration tests that reference `git` subcommand. These should still pass (hidden alias) but add parallel tests for the new `repo` spellings.

### Phase 1 verification

```bash
just test    # all existing tests pass
just lint    # no warnings on dead code
```

Manual smoke tests:
- `sniff repo` → structure output (same as before)
- `sniff repo git-status` → git status output
- `sniff repo hash HEAD` → commit details
- `sniff repo staged-files` → staged file list
- `sniff repo remote origin` → remote inspection
- `sniff git` → still works (hidden alias), same output as `sniff repo git-status`

---

## Phase 2: Output Refactor — render-first pattern

**Goal:** Convert top-level text renderers from print-now to render-first (return `String`), creating a single text emission boundary in `commands.rs`. This is prerequisite for `--plain`.

### 2.1 Convert `output/filesystem.rs` public print functions

**File:** `sniff/cli/src/output/filesystem.rs`

For each `print_*` function that prints to stdout, create a corresponding `render_*` function that returns `String`. The simplest approach:

1. Rename `print_git_section` → keep it but have it call a new `render_git_section() -> String`
2. Same for: `print_repo_section`, `print_hash_section`, `print_git_file_list`, `print_language_section`, `print_files_section`, `print_docs_section`, `print_filesystem_section`, `print_repo_deps_text`, `print_repo_deps_visual`

**Strategy:** Inside each function, replace `println!()` / `print!()` calls with writes to a `String` buffer. Use `write!` / `writeln!` to a `String` or `fmt::Write`. Some functions use `biscuit_terminal` components that call `display()` — those need to switch to `fallback_render()` or `render()` and append to the buffer.

**Scope limitation:** Functions that return exit codes (`print_current_package_area_dirty`, `print_package_area_has_source_code_changes`) don't produce text output — they call `std::process::exit()`. Leave these as-is. They should not be affected by `--plain`.

**Functions that return plain text already (no escape codes):**
- `print_repo_packages` (CSV output)
- `print_repo_package` / `print_repo_package_area` / roots (single path output)
- `print_repo_root`

These can be trivially converted to return `String`.

### 2.2 Convert `output/programs.rs`

**File:** `sniff/cli/src/output/programs.rs`

- `print_programs_markdown()` → `render_programs_markdown() -> String`

### 2.3 Convert `output/services.rs`

**File:** `sniff/cli/src/output/services.rs`

- `print_services_text()` → `render_services_text() -> String`

### 2.4 Convert `output/remote.rs`

**File:** `sniff/cli/src/output/remote.rs`

- `print_remote_text()` → `render_remote_text() -> String`

### 2.5 Convert `output/hardware.rs`, `output/network.rs`, `output/os.rs`

Same pattern for all remaining text renderers. Each `print_*_section` becomes `render_*_section() -> String`.

### 2.6 Convert `output/topics.rs`

- `print_topics_table()` → `render_topics_table() -> String`

### 2.7 Update `output/mod.rs` — central `render_text()` and `emit_text()`

**File:** `sniff/cli/src/output/mod.rs`

Replace `print_text()` with `render_text() -> String`. This function calls the per-section `render_*` functions and concatenates their output.

Add a thin `emit_text()` helper:

```rust
pub fn emit_text(text: &str, plain: bool) {
    if plain {
        print!("{}", biscuit_terminal::prelude::strip_escape_codes(text));
    } else {
        print!("{text}");
    }
}
```

### 2.8 Update `commands.rs` to use render-then-emit

**File:** `sniff/cli/src/commands.rs`

All text output paths change from:
```rust
output::print_text(...);
```
to:
```rust
let text = output::render_text(...);
output::emit_text(&text, cli.plain);
```

Same for remote, programs, services, topics, hash, file-list outputs.

### Phase 2 verification

```bash
just test    # all tests pass — output is identical
just lint
```

Verify no regressions by comparing output of key commands before/after.

---

## Phase 3: Global `--plain` flag

**Goal:** Add `--plain` to `Cli` and wire it through all text emission paths.

### 3.1 Add `--plain` to `Cli`

**File:** `sniff/cli/src/args.rs`

```rust
pub struct Cli {
    // ... existing fields ...

    /// Strip terminal escape codes from text output
    #[arg(long, global = true)]
    pub plain: bool,
}
```

### 3.2 Wire `plain` through `commands.rs`

**File:** `sniff/cli/src/commands.rs`

Pass `cli.plain` to all `emit_text()` calls. When `cli.json` is true, `plain` is ignored (JSON paths don't go through `emit_text`).

### 3.3 Handle help output with `--plain`

**File:** `sniff/cli/src/commands.rs`

Pre-scan `std::env::args()` for `--plain` before clap parsing. If present, disable clap styles:

```rust
fn prescan_plain() -> bool {
    std::env::args().any(|a| a == "--plain")
}

// In run(), before Cli::parse():
if prescan_plain() {
    // Disable clap ANSI styling
    std::env::set_var("NO_COLOR", "1");
}
```

Or use clap's `Styles::plain()`:
```rust
#[command(styles = if_plain_styles())]
```

The pre-scan approach is simpler and more reliable since it also affects `AFTER_HELP` rendering.

### 3.4 Handle `deps --ui` with `--plain`

**File:** `sniff/cli/src/output/filesystem.rs`

When rendering `deps --ui` (Mermaid visual), check if plain mode is active. If so, use the text fallback path instead of the Mermaid graphical path. The render function should accept a `plain: bool` parameter (or this can be checked at the dispatch level in `commands.rs`/`output/mod.rs`).

### 3.5 Add `--plain` tests

**File:** `sniff/cli/tests/cli.rs`

```rust
#[test]
fn plain_flag_strips_escape_codes() {
    // Run a command that produces styled output
    // Verify no ANSI escape sequences in output
}

#[test]
fn plain_with_json_ignores_plain() {
    // --plain --json should produce normal JSON
}

#[test]
fn plain_help_has_no_styles() {
    // sniff --plain --help should not contain escape codes
}
```

### Phase 3 verification

```bash
just test
just lint
sniff repo --plain           # no escape codes
sniff repo --plain --json    # normal JSON
sniff --plain --help         # no styled output
sniff repo deps --ui --plain # text fallback
```

---

## Phase 4: Help System Redesign

**Goal:** Shorten top-level help, move detail to `repo --help`, remove examples from global help.

### 4.1 Rewrite `AFTER_HELP`

**File:** `sniff/cli/src/args.rs` (lines 705–794)

Replace the long examples-heavy `AFTER_HELP` with a short grouped summary:

```
Commands:
  System:
    sniff os          Show OS information
    sniff hardware    Show hardware information
    sniff network     Show network information
    sniff cpu         Show CPU information
    sniff gpu         Show GPU information
    sniff memory      Show memory information
    sniff storage     Show storage/disk information
    sniff audio-devices  Show audio devices

  Repository & Filesystem:
    sniff repo        Show repository structure (use --help for all repo commands)
    sniff filesystem  Show full filesystem report
    sniff language    Show language detection
    sniff files       Show file associations
    sniff docs        Show markdown documents

  Programs:
    sniff programs    Show all installed programs
    sniff editors     Show editors (supports 'install' subcommand)
    sniff utilities   Show utilities
    sniff agents      Show AI agent CLI tools

  Services:
    sniff services    Show running services

  Discovery:
    sniff topics      Show all subsection topics

Output modes:
  No subcommand: show this help (use --json for full JSON)
  With subcommand: text by default, --json for JSON, --plain for unstyled text
```

### 4.2 Add `repo` command-local help

**File:** `sniff/cli/src/args.rs`

Add `after_help` to `RepoSubcommand` (or to `Commands::Repo`):

```rust
#[command(
    disable_help_subcommand = true,
    after_help = REPO_AFTER_HELP,
)]
pub enum RepoSubcommand { ... }
```

Write `REPO_AFTER_HELP` with examples grouped by category:

```
Structure:
  sniff repo                      Show repository/monorepo structure
  sniff repo biscuit              Filter to packages matching "biscuit"
  sniff repo structure @sniff     Filter to packages in "sniff" area

Git:
  sniff repo git-status           Show git status and recent commits
  sniff repo git-status --history 20  Show more commits
  sniff repo hash HEAD            Show latest commit details
  sniff repo staged-files         List staged files
  sniff repo unstaged-files       List unstaged files
  sniff repo untracked-files      List untracked files
  sniff repo remote origin        Inspect the 'origin' remote

Packages:
  sniff repo packages             List all package names
  sniff repo dirty-packages       Packages with uncommitted changes
  sniff repo package              Package name for current directory

Dependencies:
  sniff repo deps                 Text dependency list
  sniff repo deps --ui            Mermaid dependency diagram
  sniff repo --latest-versions    Check registries for updates
```

### 4.3 Add `disable_help_subcommand` to root command

**File:** `sniff/cli/src/args.rs` (line 12)

```rust
#[command(
    name = "sniff",
    version,
    about,
    after_help = AFTER_HELP,
    help_template = HELP_TEMPLATE,
    disable_help_subcommand = true,
)]
```

### 4.4 Update/refresh snapshot tests

**File:** `sniff/cli/tests/snapshots.rs`

Refresh any snapshots that capture help output or mention `sniff git`.

**File:** `sniff/cli/tests/cli.rs`

Update tests that assert on help content (e.g., tests checking for "git" in help output).

### Phase 4 verification

```bash
just test
sniff --help              # short, grouped, no examples
sniff repo --help         # detailed with examples
sniff repo git-status -h  # subcommand help
```

---

## Phase 5: Deprecation Warning and Cleanup

**Goal:** Add optional deprecation warning for `sniff git` usage, clean up dead code.

### 5.1 Add deprecation warning for `sniff git` (text mode only)

**File:** `sniff/cli/src/commands.rs`

When `Commands::Git` is matched and JSON mode is not active, emit a one-line deprecation notice to stderr:

```rust
if matches!(cli.command, Some(Commands::Git { .. })) && !cli.json {
    eprintln!("note: 'sniff git' is deprecated, use 'sniff repo' subcommands instead");
}
```

### 5.2 Remove `OutputFilter::Git`

**File:** `sniff/cli/src/output/mod.rs`

If all git dispatch now goes through `RepoAction`, `OutputFilter::Git` is dead. Remove it from the enum and all match arms.

Update `apply_filter_to_json()` — `Commands::Git` JSON should map to `OutputFilter::Repo` with git-specific JSON handling, or better: JSON rendering should also be driven by `RepoAction`.

**Risk:** JSON output shape must remain identical for backwards compatibility. `sniff repo git-status --json` must produce `filesystem.git` data, not `filesystem.repo` data. Verify this in tests.

### 5.3 Clean up `Commands` accessor methods

**File:** `sniff/cli/src/args.rs`

Remove methods that are no longer called after the `RepoAction` migration:
- `git_remote()`
- `git_subcommand()`
- `git_hash()`
- `git_package()`

Keep `to_output_filter()` but simplify — `Commands::Git` maps to same filter as `Commands::Repo`.

### 5.4 Remove dead `GitSubcommand` references

If `GitSubcommand` is only used by the hidden `Commands::Git` compatibility alias, it can stay. Do **not** remove it until the compatibility alias is removed in a future release.

### 5.5 Update documentation

- `sniff/cli/README.md` — update command reference
- Any help snapshots in tests
- `CLAUDE.md` if it references `sniff git`

### Phase 5 verification

```bash
just test
just lint    # no dead code warnings
sniff git 2>&1 | head -1  # shows deprecation notice
sniff git --json           # no deprecation, normal JSON
sniff repo git-status      # no deprecation, normal output
```

---

## File Change Summary

| File | Phase | Nature of change |
|---|---|---|
| `sniff/cli/src/args.rs` | 1, 3, 4 | New enums, expanded `RepoSubcommand`, hidden `Git`, `--plain`, help text |
| `sniff/cli/src/commands.rs` | 1, 2, 3, 5 | `RepoAction` dispatch, render-then-emit, `--plain` wiring, deprecation |
| `sniff/cli/src/output/mod.rs` | 1, 2 | `render_text()` replaces `print_text()`, `emit_text()` helper, remove `OutputFilter::Git` |
| `sniff/cli/src/output/filesystem.rs` | 2 | `print_*` → `render_*` (return `String`) |
| `sniff/cli/src/output/programs.rs` | 2 | `print_programs_markdown` → `render_programs_markdown` |
| `sniff/cli/src/output/services.rs` | 2 | `print_services_text` → `render_services_text` |
| `sniff/cli/src/output/remote.rs` | 2 | `print_remote_text` → `render_remote_text` |
| `sniff/cli/src/output/hardware.rs` | 2 | `print_*_section` → `render_*_section` |
| `sniff/cli/src/output/network.rs` | 2 | `print_network_section` → `render_network_section` |
| `sniff/cli/src/output/os.rs` | 2 | `print_os_section` → `render_os_section` |
| `sniff/cli/src/output/topics.rs` | 2 | `print_topics_table` → `render_topics_table` |
| `sniff/cli/tests/cli.rs` | 1, 3, 4 | New parser tests, `--plain` tests, help content tests |
| `sniff/cli/tests/snapshots.rs` | 4 | Refresh help snapshots |

---

## Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| Parser ambiguity between `repo <FILTER>` and `repo <subcommand>` | High | Clap handles this: named subcommands take precedence over positional args. Test edge cases (e.g., `sniff repo structure` where "structure" is both a subcommand and could be a filter string). |
| JSON output shape changes break consumers | High | Keep `OutputFilter::Git` JSON path producing identical output. Add JSON snapshot tests for `repo git-status --json`, `repo hash HEAD --json`, `repo staged-files --json`. |
| render-first refactor introduces output differences | Medium | Run before/after diff on key command outputs. Many functions build strings internally already — the refactor is mostly moving `println!` up the call stack. |
| `--plain` pre-scan conflicts with clap parsing | Low | Pre-scan only checks raw argv for `--plain`; clap still parses normally. `NO_COLOR` env var is well-established convention. |
| Hidden `git` alias confuses shell completions | Low | Hidden commands are excluded from completions by default. |

---

## Implementation Order Rationale

1. **Phase 1 first** — the parser/routing change is the foundation. Everything else depends on repo subcommands being in place.
2. **Phase 2 second** — render-first is prerequisite for `--plain`. Separating it from Phase 1 keeps the parser change and output-pipeline change independent (easier to debug failures).
3. **Phase 3 third** — `--plain` builds on the render-first boundary from Phase 2.
4. **Phase 4 fourth** — help redesign is cosmetic and independent of functional changes. Doing it after phases 1–3 means the help text can reference the final command surface.
5. **Phase 5 last** — cleanup and deprecation are low-risk polish.

Each phase should be a separate commit (or small group of commits) so regressions are easy to bisect.
