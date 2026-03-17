# Merge `sniff git` Into `sniff repo` Tech Design

This document defines the implementation-ready technical design for the `merge-git-and-repo` feature in `sniff`. It is derived from:

- `sniff/features/merge-git-and-repo/spec.md`
- the current CLI parser in `sniff/cli/src/args.rs`
- the current command execution flow in `sniff/cli/src/commands.rs`
- the current text/JSON renderers in `sniff/cli/src/output/`

The design goal is to unify repository structure and local git inspection under a single `repo` namespace without regressing existing structure workflows, without making the parser ambiguous, and without leaving `--plain` as a one-off hack.

## Overview

Today `sniff` splits repository-facing behavior across two top-level commands:

- `sniff repo` for monorepo/package structure
- `sniff git` for local git status, commit lookup, file state lists, and remote inspection

That split is no longer paying for itself. The implementation already reflects the overlap:

- both commands operate inside the filesystem domain
- both require similar detection setup in `commands.rs`
- `repo` behavior is routed through generic filesystem output
- `git` behavior relies on several early-return special cases

The merged design makes `repo` the only visible repository command family and introduces an explicit `repo structure` subcommand for the old default structure view. `sniff repo` without a child subcommand remains valid and still behaves like structure mode.

The design also adds a global `--plain` switch. Because current text output is printed directly with `println!` in many places, `--plain` is treated as an output-pipeline change, not as a last-minute flag branch.

## Goals

1. Make `repo` the single visible entry point for repository-oriented workflows.
2. Preserve the current `sniff repo` default behavior by making `structure` the implicit default subcommand.
3. Move git-reporting flows into `repo` with names that read clearly beside structural subcommands.
4. Keep JSON behavior stable for existing data shapes where possible.
5. Shorten the top-level help output and move examples to command-local help.
6. Add a global `--plain` mode that strips terminal escape codes from text output and is ignored when `--json` is active.

## Non-Goals

1. Changing `sniff/lib` filesystem detection schemas.
2. Redesigning the remote-provider integration used by `sniff git <remote>`.
3. Changing `sniff filesystem` aggregate reporting beyond plumbing the new repo routes and `--plain`.
4. Reworking package/repo detection algorithms.
5. Replacing existing rich terminal rendering with a new styling system.

## Current Baseline

The current CLI has three important characteristics:

1. `args.rs` models `git` and `repo` as separate top-level `Commands` variants.
2. `commands.rs` handles git behavior in three different ways:
   - remote inspection is handled before detection by inspecting `git_remote()`
   - `git hash` is handled as a direct `git2` lookup
   - `git staged|unstaged|untracked` are handled as dedicated early returns
3. most text renderers print directly to stdout, which means there is no single boundary where ANSI/OSC output can be stripped.

This is workable while `git` is its own top-level command, but it becomes brittle once `repo` must cover:

- structure reporting
- git status reporting
- file-state lists
- commit lookup
- remote inspection
- package/root/dirty queries
- dependency diagrams

The merged command family needs a canonical internal route layer so parse-time shape and execution-time behavior stop being tightly coupled.

## Constraints and Spec Gaps

The spec is directionally clear, but two parser details need explicit design decisions.

### 1. Remote inspection cannot remain a bare positional argument

Current `git` supports:

```sh
sniff git origin
sniff git owner/repo
sniff git https://github.com/...
```

Current `repo` already reserves its first positional argument for a structure filter:

```sh
sniff repo biscuit
sniff repo !biscuit
sniff repo @sniff
```

Those two grammars conflict. `sniff repo origin` cannot mean both:

- "show structure filtered by `origin`"
- "inspect the `origin` remote"

Recommendation:

- make remote inspection explicit as `sniff repo remote <REMOTE>`

This is the only unambiguous design that preserves `sniff repo <FILTER>`.

### 2. `-h` must return to normal help semantics under `repo`

Current `git` disables the standard help flag so `-h` can mean `--history`.
That tradeoff is tolerable on a focused command, but it is the wrong default for a larger merged `repo` surface.

Recommendation:

- `repo` and all visible repo subcommands use normal `-h/--help`
- git history becomes long-only `--history`

This is a user-facing breaking change, but it avoids a much worse steady-state UX.

## Proposed CLI Surface

### Canonical command map

| Current command | New canonical command |
|---|---|
| `sniff repo` | `sniff repo` or `sniff repo structure` |
| `sniff repo biscuit` | `sniff repo biscuit` or `sniff repo structure biscuit` |
| `sniff git` | `sniff repo git-status` |
| `sniff git --refresh-remotes` | `sniff repo git-status --refresh-remotes` |
| `sniff git --history 20` | `sniff repo git-status --history 20` |
| `sniff git --package homelab` | `sniff repo git-status --package homelab` |
| `sniff git hash HEAD` | `sniff repo hash HEAD` |
| `sniff git staged` | `sniff repo staged-files` |
| `sniff git unstaged` | `sniff repo unstaged-files` |
| `sniff git untracked` | `sniff repo untracked-files` |
| `sniff git origin` | `sniff repo remote origin` |
| `sniff git owner/repo` | `sniff repo remote owner/repo` |
| `sniff git https://host/...` | `sniff repo remote https://host/...` |

### Proposed grammar

```sh
sniff repo [FILTER]
sniff repo structure [FILTER] [--latest-versions]
sniff repo git-status [--history N] [--refresh-remotes] [--package PKG]
sniff repo hash <SHA>
sniff repo staged-files [--package PKG]
sniff repo unstaged-files [--package PKG]
sniff repo untracked-files [--package PKG]
sniff repo remote <REMOTE>
sniff repo deps [FILTER] [--ui]
sniff repo packages [FILTER]
sniff repo package
sniff repo package-area
sniff repo dirty-packages [FILTER]
sniff repo dirty-package-areas [FILTER]
sniff repo package-root
sniff repo package-area-root
sniff repo repo-root
sniff repo is-current-package-area-dirty
sniff repo package-area-has-source-code-changes
```

### Behavioral rules

1. `sniff repo` with no child subcommand behaves exactly like today’s `sniff repo`.
2. `sniff repo structure` is the explicit form of the default structure action.
3. `sniff repo <FILTER>` remains valid and maps to structure mode.
4. `sniff repo remote <REMOTE>` is the only form for remote inspection.
5. `sniff repo git-status` is the only form for the old default `sniff git` report.
6. `sniff repo staged-files|unstaged-files|untracked-files` are leaf actions and do not share the top-level structure filter positional.

## Parser Design

### Recommended parse shape

Retain `Commands::Repo` as the top-level parser entry, but expand its child subcommands and move git-derived options to the relevant leaf commands.

Recommended structure:

```rust
pub enum Commands {
    // existing non-repo commands...
    Repo {
        #[arg(long)]
        latest_versions: bool,
        filter: Option<String>,
        #[command(subcommand)]
        repo_subcommand: Option<RepoSubcommand>,
    },
    #[command(hide = true)]
    Git {
        // optional compatibility alias, see migration section
    },
}

pub enum RepoSubcommand {
    Structure {
        filter: Option<String>,
    },
    GitStatus {
        #[arg(long, default_value_t = DEFAULT_COMMIT_COUNT)]
        history: usize,
        #[arg(long)]
        refresh_remotes: bool,
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    Hash {
        sha: String,
    },
    StagedFiles {
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    UnstagedFiles {
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    UntrackedFiles {
        #[arg(short, long, value_name = "PKG")]
        package: Option<String>,
    },
    Remote {
        #[arg(value_name = "REMOTE")]
        remote: String,
    },
    // existing repo structure subcommands...
}
```

### Why git-derived options move to leaf subcommands

This avoids four parser problems:

1. `repo` root keeps standard help semantics.
2. structure-only options do not pollute git-status help.
3. git-status-only options do not appear next to `repo packages`, `repo deps`, and exit-code subcommands.
4. the execution layer no longer has to interpret a single top-level command with partially meaningful fields.

### Structure default normalization

`sniff repo` and `sniff repo <FILTER>` should normalize to:

```rust
RepoAction::Structure { filter }
```

`sniff repo structure <FILTER>` should normalize to the same action.

The top-level `filter` remains parse-time sugar for the default structure route. It should not be inspected by any other repo subcommand.

## Canonical Execution Route

Add a normalized route layer between clap parsing and command execution.

Recommended internal type:

```rust
enum CliRoute {
    FullJson,
    Help,
    Topics,
    Section(OutputFilter),
    Repo(RepoAction),
    Programs(ProgramsRoute),
    Services(ServicesRoute),
}

enum RepoAction {
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

### Why a route layer is required

Right now `Commands::Repo` maps to `OutputFilter::Repo`, while `Commands::Git` maps to `OutputFilter::Git` plus several early returns. That model breaks once `repo` includes both repo-structure and git-status actions.

The route layer fixes that by:

1. separating parse-time spelling from runtime semantics
2. letting `repo git-status` reuse `OutputFilter::Git`
3. letting `repo structure` reuse `OutputFilter::Repo`
4. letting `repo hash`, `repo remote`, and file-list actions stay specialized without overloading `OutputFilter`

## Detection and Execution Matrix

| Repo action | Requires local detection | Detection scope | Extra work |
|---|---|---|---|
| `Structure` | yes | filesystem | optional dependency enrichment |
| `Deps` | yes | filesystem | optional Mermaid render |
| `Packages` | yes | filesystem | none |
| `Package` / `PackageArea` / roots | yes | filesystem | none |
| `DirtyPackages` / `DirtyPackageAreas` | yes | filesystem | needs repo + git |
| `IsCurrentPackageAreaDirty` | yes | filesystem | exit-code path |
| `PackageAreaHasSourceCodeChanges` | yes | filesystem | exit-code path |
| `GitStatus` | yes | filesystem | optional remote refresh, optional package scoping |
| `StagedFiles` / `UnstagedFiles` / `UntrackedFiles` | yes | filesystem | optional package scoping |
| `Hash` | no `SniffResult` | direct `git2` | commit/file lookup |
| `Remote` | no local detection | none | provider fetch |

### Detection recommendation

Keep repo-family local detection at the existing filesystem-only scope:

```rust
config.skip_os().skip_hardware().skip_network()
```

This is not the most granular possible design, but it is the lowest-risk implementation because:

1. current repo and git functionality already assume filesystem-only detection
2. package scoping for git actions depends on repo metadata
3. dirty-package and source-change commands depend on both repo and git data

The merged design should optimize for correctness and code simplification before micro-optimizing detection breadth.

## Output Design

### JSON behavior

JSON should remain action-specific:

| Command | JSON shape |
|---|---|
| `sniff repo --json` | current repo structure JSON (`filesystem.repo`) |
| `sniff repo structure --json` | current repo structure JSON (`filesystem.repo`) |
| `sniff repo git-status --json` | current git JSON (`filesystem.git`) |
| `sniff repo hash <SHA> --json` | current `{ commit, files }` object |
| `sniff repo staged-files --json` | current file list array |
| `sniff repo remote <REMOTE> --json` | current remote report JSON |

This preserves the current data contracts rather than inventing a new wrapper just because the namespace changed.

### Text behavior

The core recommendation is to convert top-level text renderers from print-now functions into render-first functions.

Recommended public output boundary:

```rust
pub fn render_text(...) -> String
pub fn render_programs_text(...) -> String
pub fn render_services_text(...) -> String
pub fn render_remote_text(...) -> String
```

Then `commands.rs` becomes responsible for final emission:

```rust
fn emit_text(text: String, plain: bool) {
    let text = if plain { strip_escape_codes(text) } else { text };
    print!("{text}");
}
```

### Why render-first is the right boundary

`--plain` is global, but current text is emitted from many different places:

- filesystem renderers
- repo renderers
- git renderers
- programs renderers
- services renderers
- remote renderers

Trying to bolt `strip_escape_codes()` onto only the repo path would create inconsistent behavior immediately. A single text emission boundary keeps the flag honest.

### Scope of refactor

This does not require every helper to become a pure function on day one. The important boundary is the top-level command renderer. Helper functions may still build strings incrementally inside each module.

The minimum viable refactor is:

1. replace public `print_*_text` entrypoints with `render_*_text -> String`
2. update `commands.rs` to emit text in one place
3. leave JSON printers unchanged

## `--plain` Design

### CLI contract

Add to `Cli`:

```rust
#[arg(long, global = true)]
pub plain: bool,
```

Behavior:

1. if `--json` is active, `--plain` is ignored
2. if text output is active, all terminal escape codes are removed with `biscuit_terminal::prelude::strip_escape_codes`
3. this applies to normal command output and help output

### Help output handling

Help output is special because clap can exit before normal command execution.

Recommendation:

1. pre-scan raw argv for `--plain`
2. if present, disable clap styling before parsing help output
3. where help text is manually written to a buffer, pass it through the same plain-emission path

This avoids partial compliance where normal command output is plain but `--help` still emits style sequences.

### Commands with graphical output

`sniff repo deps --ui` currently renders Mermaid output directly for the terminal. Stripping graphics escape sequences after the fact will not produce a useful result.

Recommendation:

1. when `--plain` is active, bypass inline graphical rendering
2. use the Mermaid fallback text path instead
3. then apply `strip_escape_codes()` to that fallback output as usual

This keeps `--plain` readable instead of technically correct but empty.

## Help System Redesign

The current top-level help is long because `AFTER_HELP` manually enumerates nearly every command and example. The merged design should make `repo --help` the detailed discovery surface for repository workflows and keep top-level help short.

### Top-level help

Replace the long manual list with a short grouped summary:

1. system sections
2. repo/filesystem discovery
3. program discovery
4. services
5. output mode notes

Top-level help should:

- mention `sniff repo --help` for repository operations
- mention `sniff topics` for discovery
- remove the examples section entirely

### Repo help

`sniff repo --help` becomes the detailed, user-facing help for all merged repo flows. It should list:

- `structure`
- `git-status`
- `hash`
- `staged-files`
- `unstaged-files`
- `untracked-files`
- `remote`
- existing structure/package/dependency actions

### Per-command examples

Move examples to command-local help for the commands that benefit most:

1. `repo`
2. `repo structure`
3. `repo git-status`
4. `repo remote`
5. `repo deps`

### Help subcommand visibility

Disable visible help subcommands to keep help and completion output smaller:

```rust
#[command(disable_help_subcommand = true)]
```

Apply this at least to the root command and the `repo` command family.

## Compatibility and Migration

This feature is user-facing and potentially script-affecting. The design recommends a short compatibility phase.

### Recommendation

Keep `sniff git` as a hidden compatibility alias for one release cycle.

Behavior:

1. `sniff git` stays parsable
2. it is removed from visible help and completions
3. it normalizes internally to the new `RepoAction` routes
4. JSON output remains unchanged

Optional warning behavior:

- text mode may emit a deprecation warning to stderr
- JSON mode should not emit a warning

If the project prefers a hard cut instead, the implementation becomes smaller, but the hidden-alias path is safer and still keeps the public surface clean.

## File-Level Implementation Plan

### `sniff/cli/src/args.rs`

Changes:

1. add global `--plain`
2. add visible `repo structure`
3. add `repo git-status`
4. add `repo staged-files`
5. add `repo unstaged-files`
6. add `repo untracked-files`
7. add `repo remote`
8. move git-only options onto git-related leaf subcommands
9. remove visible `git` command, or keep it hidden as a compatibility alias
10. shorten `AFTER_HELP`
11. remove top-level examples from help text

### `sniff/cli/src/commands.rs`

Changes:

1. add parse normalization into `CliRoute` and `RepoAction`
2. replace `git_*`-specific branches with repo-route branches
3. centralize text emission through an `emit_text` helper
4. keep JSON emission separate
5. handle `--plain` before help emission and after text rendering

### `sniff/cli/src/output/mod.rs`

Changes:

1. replace `print_text` with `render_text -> String`
2. keep `print_json` or rename to `render_json` only if that improves consistency
3. expose route-specific render helpers for repo structure vs git status

### `sniff/cli/src/output/filesystem.rs`

Changes:

1. change repo/git public text entrypoints to return strings
2. keep existing styling markup and rendering logic
3. ensure `deps --ui` has a text fallback path that can be used under `--plain`

### `sniff/cli/src/output/programs.rs`

Changes:

1. replace direct text printing with string rendering for global `--plain`

### `sniff/cli/src/output/services.rs`

Changes:

1. replace direct text printing with string rendering for global `--plain`

### `sniff/cli/src/output/remote.rs`

Changes:

1. replace direct text printing with string rendering for global `--plain`

### Documentation updates

When implementation lands, update:

1. `sniff/cli/README.md`
2. any help snapshots
3. any docs that mention `sniff git`

## Testing Plan

### Parser unit tests

Add or update parser tests for:

1. `repo` defaulting to structure
2. `repo biscuit` parsing as structure with filter
3. `repo structure biscuit`
4. `repo git-status --history 20`
5. `repo git-status --refresh-remotes --package homelab`
6. `repo hash HEAD`
7. `repo staged-files`
8. `repo unstaged-files`
9. `repo untracked-files`
10. `repo remote origin`
11. `repo remote owner/repo`
12. hidden `git` alias normalization, if compatibility mode is kept

### CLI integration tests

Update or add integration coverage for:

1. top-level help no longer showing the old global examples block
2. top-level help pointing users to `repo --help`
3. `repo --help` showing merged repo/git actions
4. `repo git-status --json` containing current git JSON fields
5. `repo hash HEAD --json` returning commit-plus-files JSON
6. `repo staged-files --json` returning a JSON array
7. `repo remote origin --json` matching current remote-report behavior

### `--plain` tests

Add command-level tests proving:

1. `--plain` removes ANSI/OSC sequences from text output
2. `--plain --json` behaves like plain was not supplied
3. `repo deps --ui --plain` produces readable fallback output
4. `--plain --help` does not emit terminal styling sequences

### Snapshot tests

Refresh:

1. global help snapshot
2. repo help snapshot
3. any snapshot that referenced `sniff git`

## Recommended Implementation Order

1. introduce the normalized route layer
2. reshape `repo` subcommands in `args.rs`
3. migrate git execution branches onto repo routes
4. add the `repo remote <REMOTE>` path
5. refactor top-level text renderers to return strings
6. add global `--plain`
7. shorten and rebalance help output
8. add compatibility alias handling if desired
9. update docs and snapshots

This order keeps the risky parser change and the risky output-pipeline change separate, which will make failures easier to reason about during review.

## Final Recommendation

Implement the merge as a parser-and-routing cleanup, not as a set of ad hoc renames.

The key design choices are:

1. `sniff repo` remains the default structure view through normalization to `repo structure`
2. git status becomes `sniff repo git-status`
3. remote inspection becomes explicit as `sniff repo remote <REMOTE>` because the existing repo filter positional makes any other design ambiguous
4. `--plain` is implemented through a central text-emission boundary so it works consistently across repo, programs, services, remote inspection, and help output

That gives `sniff` one coherent repository namespace, keeps the current data contracts stable, and removes several existing special cases from `commands.rs` instead of adding more.
