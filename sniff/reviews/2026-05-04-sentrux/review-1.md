---
date: 2026-05-04
package_area: sniff
review_tool: sentrux (manual structural analysis)
metrics:
  - modularity
  - acyclicity
  - depth
  - equality
  - redundancy
suggestions: 20
suggestions_critical: 3
suggestions_urgent: 10
---

# Sniff package area structural review (2026-05-04)

> **Note on methodology.** The Sentrux MCP server and CLI were not callable in
> this non-interactive session (permission gates), and no `.sentrux/baseline.json`
> exists for the `sniff` package area at the cited path. The findings below are
> based on a manual structural pass over the `sniff` package area source tree
> (135 Rust files in `sniff/lib` and `sniff/cli`), inspecting module imports,
> file sizes, and re-export surfaces — the same signals Sentrux uses for its
> modularity, acyclicity, depth, equality, and redundancy metrics. Re-run
> `sentrux scan sniff/` once permitted to obtain numeric scores.

## sniff

The `sniff` library crate (134 source files under `lib/src/`) shows clear
acyclicity violations inside the `programs` module, a god-module problem in
`filesystem/repo/detection.rs` and `filesystem/git/detection.rs`, and severe
mass concentration (low equality / high Gini) where the top seven files hold
roughly 12 600 of ~32 000 LOC.

### `critical`: cycle between `programs::types` and `programs::schema`

`programs/types.rs` imports `ProgramError` and `ProgramMetadata` from
`programs::schema`, while `programs/schema.rs` imports `InstallationMethod`
and `SystemPrerequisite` from `programs::types`. This is a true two-node
cycle that defeats Martin's acyclicity principle — neither file can be read,
documented, or stabilised without the other.

**Files**:

- `sniff/lib/src/programs/types.rs:20`
- `sniff/lib/src/programs/schema.rs:15`

**Fix.** Promote the four shared leaf types (`InstallationMethod`,
`SystemPrerequisite`, `ProgramError`, `ProgramMetadata` trait) into a new
`programs::contract` (or `programs::core`) module. Both `types.rs` and
`schema.rs` import from `contract`; nothing imports back.

```rust
// programs/contract.rs (new)
pub enum InstallationMethod { /* … */ }
pub enum SystemPrerequisite { /* … */ }
pub trait ProgramMetadata { /* … */ }
pub enum ProgramError { /* … */ }

// programs/types.rs
use crate::programs::contract::{InstallationMethod, ProgramError, ProgramMetadata, SystemPrerequisite};

// programs/schema.rs
use crate::programs::contract::{InstallationMethod, ProgramError, ProgramMetadata, SystemPrerequisite};
```

### `critical`: cycle between `programs::types` and `programs::enums::metadata`

`programs/enums/metadata.rs` imports `InstallationMethod` from
`programs::types`, while `programs/types.rs` imports `CategoryEnum` from
`programs::enums`. Combined with the previous cycle, the `types`/`schema`/
`enums::metadata` triangle forms a strongly-connected component that
Sentrux's acyclicity score will flag aggressively.

**Files**:

- `sniff/lib/src/programs/types.rs:16`
- `sniff/lib/src/programs/enums/metadata.rs:3-4`

**Fix.** After extracting the leaf contract module above, move
`CategoryEnum` (the `enums` module's public trait) into the same
`programs::contract` module. `types.rs` then depends only on `contract` and
no longer needs to reach into `enums`.

### `urgent`: `filesystem/repo/detection.rs` is a 2783-line god module

A single file owns Cargo, pnpm, npm, yarn, Nx, Turborepo, and Lerna
workspace detection plus dependency parsing for Cargo, npm, Python, and Go.
Sentrux's modularity (Newman 2004) score collapses when one file holds 50+
free functions across that many ecosystems.

**Files**:

- `sniff/lib/src/filesystem/repo/detection.rs` (2783 LOC, 53 functions)

**Fix.** Split per ecosystem under a `repo/` submodule tree:

```text
filesystem/repo/
├── mod.rs                     // public API + RepoInfo composition
├── manifest_index.rs          // ManifestCache + manifest walk
├── cargo.rs                   // detect_cargo_workspace + cargo_*_dependencies
├── npm.rs                     // detect_pnpm/npm/yarn/lerna + npm_*_dependencies
├── python.rs                  // pyproject + requirements parsing
├── go.rs                      // go.mod parsing
├── nx_turbo.rs                // Nx + Turborepo detection
└── package_build.rs           // PackageBuildContext + dedupe/merge helpers
```

Each ecosystem file becomes 300–500 LOC, with a single re-export surface in
`mod.rs`. This is the same pattern `services/windows_scm.rs` already follows
for one init system.

### `urgent`: `filesystem::git::recent_commits` → `blast_radius` → `git` layer inversion

`filesystem/git/recent_commits.rs` imports
`filesystem::blast_radius::{is_documentation_path, is_source_code_path}`,
while `filesystem/blast_radius.rs` imports `filesystem::git::get_commit_files`.
Although the `git` and `blast_radius` symbols are different, this creates a
cross-layer dependency where the higher-level analysis module
(`blast_radius`) is consumed by a lower-level domain module (`git`). Any
future tightening of the cycle (e.g. blast_radius pulling in another git
type) will close it into a true cycle.

**Files**:

- `sniff/lib/src/filesystem/git/recent_commits.rs:8`
- `sniff/lib/src/filesystem/blast_radius.rs:14`

**Fix.** Move the two path-classification predicates out of `blast_radius`
into a leaf module (`filesystem::file_types::path_classification` or simply
`filesystem::path_kind`):

```rust
// filesystem/path_kind.rs (new, leaf module)
pub fn is_documentation_path(p: &Path) -> bool { /* … */ }
pub fn is_source_code_path(p: &Path) -> bool { /* … */ }
```

Both `git::recent_commits` and `blast_radius` then depend on the leaf, and
`blast_radius` continues to depend on `git` only — strict layering preserved.

### `urgent`: `filesystem/git/detection.rs` is a 2005-line god module

A single file holds 16 functions covering branch resolution, status
collection, diff generation, remote tracking, ancestry walks, and worktree
listing. The same modularity argument as `repo/detection.rs` applies, with
the added burden that `recent_commits.rs` (1630 LOC) and `types.rs` (1108
LOC) sit beside it under one `git/` directory.

**Files**:

- `sniff/lib/src/filesystem/git/detection.rs` (2005 LOC)
- `sniff/lib/src/filesystem/git/recent_commits.rs` (1630 LOC)

**Fix.** Split `detection.rs` along `GitRequest` boundaries:

```text
filesystem/git/
├── mod.rs              (already trivial — keep as façade)
├── types.rs            (existing)
├── discovery.rs        // GitRepo::discover, branch + remote resolution
├── status.rs           // dirty/staged/changed-file collection
├── diff.rs             // unified-diff generation, per-file stats
├── remote_refresh.rs   // parallel fetch + ancestry containment
└── recent_commits.rs   // existing, but lighter once status/diff move out
```

### `urgent`: `programs/types.rs` is a 1641-line core type sink

This file is a magnet for every type that doesn't fit elsewhere
(`InstallationMethod`, `SystemPrerequisite`, `CategoryDetector`,
`ExecutableSource`, `ProgramDetector`, `PrereqProbe`). Combined with the
cycles flagged above, it is the highest-coupling node in `programs`.

**Files**: `sniff/lib/src/programs/types.rs` (1641 LOC)

**Fix.** Once the `contract` module exists (per the critical fixes), break
`types.rs` apart further:

- `programs/category_detector.rs` — generic `CategoryDetector<T>`
- `programs/install_method.rs` — `InstallationMethod` and helpers
- `programs/prerequisite.rs` — `SystemPrerequisite`, `PrereqProbe`
- `programs/source.rs` — `ExecutableSource`

Each file ends up < 500 LOC and the public re-export list in `programs/mod.rs`
stays unchanged.

### `urgent`: `programs/installer.rs` (1508) and `programs/install_plan.rs` (1417)

Two adjacent files under `programs/` exceed the 1000-line line that
maintainers can hold in their head. They share input/output types (e.g.
`InstallOptions`) which makes their boundary blurry; `install_interview.rs`
then depends on both.

**Files**:

- `sniff/lib/src/programs/installer.rs` (1508 LOC)
- `sniff/lib/src/programs/install_plan.rs` (1417 LOC)

**Fix.** Group the install-related code into a sibling `programs::install/`
sub-module:

```text
programs/install/
├── mod.rs           // re-exports public surface
├── plan.rs          // build_install_plan, InstallPlanReason
├── command.rs       // get_install_command, get_versioned_install_command
├── execute.rs       // execute_install, InstallResult
├── interview.rs     // existing install_interview moved in
└── options.rs       // shared InstallOptions struct
```

The current flat layout makes Sentrux flag three separate ~1500-LOC peaks;
grouping reduces both depth and concentration.

### `important`: `services/mod.rs` is 1054 lines mixing init detection and per-init listing

Init-system detection (`detect_init`, `detect_init_with_evidence`) and
per-init service listing (`list_systemd_services`, `list_launchd_services`,
`list_openrc_services`, `list_runit_services`, …) sit together in `mod.rs`
even though `windows_scm.rs` already shows the right pattern (one file per
init system).

**Files**: `sniff/lib/src/services/mod.rs` (1054 LOC, 31 items)

**Fix.** Split each init system into its own file so `mod.rs` becomes a
~150-line dispatcher:

```text
services/
├── mod.rs           // HostOs, InitSystem, ServiceManager, ServicesInfo
├── detect.rs        // detect_init + detect_init_with_evidence
├── launchd.rs
├── systemd.rs
├── openrc.rs
├── runit.rs
└── windows_scm.rs   // existing
```

### `important`: 9 near-empty per-category files in `programs/`

`editors.rs` (90% tests), `ai_cli.rs` (75 LOC of one-liner accessors),
`utilities.rs`, `tts_clients.rs`, `terminal_apps.rs`, `headless_audio.rs`,
`pkg_mngrs.rs` (10 lines!), `notification_helpers.rs`, and the deleted-but-
re-exported categories all do essentially the same thing: declare
`pub type InstalledX = CategoryDetector<X>;`. From a Kolmogorov-redundancy
view, nine files exist where one would suffice.

**Files**:

- `sniff/lib/src/programs/{ai_cli,editors,headless_audio,notification_helpers,pkg_mngrs,terminal_apps,tts_clients,utilities}.rs`

**Fix.** Collapse the type aliases into `programs/categories.rs`:

```rust
// programs/categories.rs
use crate::programs::enums::*;
use crate::programs::types::CategoryDetector;

pub type InstalledAiClients         = CategoryDetector<AiCli>;
pub type InstalledEditors           = CategoryDetector<Editor>;
pub type InstalledHeadlessAudio     = CategoryDetector<HeadlessAudio>;
pub type InstalledLanguagePackageManagers = CategoryDetector<LanguagePackageManager>;
pub type InstalledNotificationHelpers = CategoryDetector<NotificationHelper>;
pub type InstalledOsPackageManagers = CategoryDetector<OsPackageManager>;
pub type InstalledTerminalApps      = CategoryDetector<TerminalApp>;
pub type InstalledTtsClients        = CategoryDetector<TtsClient>;
pub type InstalledUtilities         = CategoryDetector<Utility>;
```

The boolean accessors on `InstalledAiClients` (`claude()`, `aider()`, …) are
also redundant: callers already have `is_installed(AiCli::Claude)`. Either
delete them or generate them via a `cfg-tested!` macro to remove ~55 LOC of
hand-rolled forwarders.

### `important`: `package::network` reaches across to `filesystem::repo`

`sniff/lib/src/package/network.rs:501` does
`use crate::filesystem::repo::DependencyEntry;`. The `package` module is
otherwise self-contained, but this single line creates an undeclared
dependency `package → filesystem` that breaks the natural top-level layering
implied by `lib.rs` (`package` is exposed before `filesystem` in re-exports).

**Files**:

- `sniff/lib/src/package/network.rs:501`
- `sniff/lib/src/filesystem/repo/types.rs` (defines `DependencyEntry`)

**Fix.** Move `DependencyEntry` to a shared location used by both modules
(e.g. `sniff::package::dependency::DependencyEntry`) and have
`filesystem::repo` import from there. This makes the dependency direction
`filesystem → package` (filesystem analyses what packages exist), which is
the more natural layering.

### `important`: equality (Gini) — top 7 lib files ≈ 39% of total LOC

Concentration ranking (lib only, excluding tests/benches):

| File | LOC |
|------|----:|
| `filesystem/repo/detection.rs` | 2783 |
| `filesystem/git/detection.rs` | 2005 |
| `programs/types.rs` | 1641 |
| `filesystem/git/recent_commits.rs` | 1630 |
| `programs/installer.rs` | 1508 |
| `programs/install_plan.rs` | 1417 |
| `filesystem/docs.rs` | 1319 |

Together these seven files contribute ~12 300 LOC out of an estimated
~31 500 LOC under `lib/src/` (~39%). Gini-style equality is dominated by
this long tail.

**Fix.** Each of the seven files is addressed by one of the suggestions
above (`detection.rs` splits, `types.rs` split, `installer/install_plan`
grouping, …). Once those splits land, the largest remaining file should be
under ~800 LOC, and equality scores will improve materially without any
behavioural change.

### `important`: dependency depth in `programs::install_interview`

The deepest dependency chain in the crate runs:

```text
lib → programs::install_interview → programs::install_plan
                                  → programs::installer → programs::host_capability
                                                       → programs::types → programs::schema
                                                                        → programs::enums
```

That's six edges from the public root to a leaf — Lakos depth ≥ 6 within a
single namespace. Combined with the type/schema cycle above, this makes the
`programs` module slow to compile-test in isolation.

**Files**: `sniff/lib/src/programs/{install_interview,install_plan,installer,host_capability,types,schema}.rs`

**Fix.** The `programs::contract` extraction (critical fixes 1 and 2)
collapses the bottom three layers into one shared leaf. Re-grouping
`install_*` under `programs::install/` (urgent fix above) further reduces
chain depth from 6 → 3.

### `nice-to-have`: `programs/pkg_mngrs.rs` is a 10-line file

It declares two type aliases. After the categories consolidation above, this
file ceases to exist — but if the consolidation is deferred, inline the two
aliases directly into `programs/mod.rs` to drop a redundant compilation
unit.

**Files**: `sniff/lib/src/programs/pkg_mngrs.rs`

**Fix.** Inline:

```rust
// programs/mod.rs
pub type InstalledLanguagePackageManagers = CategoryDetector<LanguagePackageManager>;
pub type InstalledOsPackageManagers       = CategoryDetector<OsPackageManager>;
```

### `nice-to-have`: `os::package_manager` reaches up into `programs::ExecutableIndex`

`os/package_manager.rs:15` imports `crate::programs::ExecutableIndex`. As
with the `package → filesystem` reach, this inverts the intuitive layering
(`os` is conceptually below `programs`, since program detection asks the OS
what it has).

**Files**: `sniff/lib/src/os/package_manager.rs:15`

**Fix.** Move `ExecutableIndex` into a neutral leaf (e.g. `sniff::executable_index`
at crate root) so both `os` and `programs` import down rather than across.

## sniff-cli

The CLI crate (`sniff/cli/src/`, 22 files) has one true dependency cycle and
two extreme size outliers driving the modularity and equality scores down.

### `critical`: cycle between `commands.rs` and `output/recent_commits.rs`

`commands.rs:19` does `use crate::output::{self, OutputFilter, PathListFormat};`
while `output/recent_commits.rs:7` does
`use crate::commands::{CliPerf, handle_no_results};`. That is a literal
two-file cycle inside the CLI binary — Sentrux acyclicity will flag it,
and rustc only tolerates it because Rust's module graph is decoupled from
the file graph.

**Files**:

- `sniff/cli/src/commands.rs:19,27`
- `sniff/cli/src/output/recent_commits.rs:7`

**Fix.** Move `CliPerf` and `handle_no_results` out of `commands.rs` into a
shared leaf module (`cli/src/perf.rs` or `cli/src/runtime.rs`). Both
`commands.rs` and `output/recent_commits.rs` import from the new leaf:

```rust
// cli/src/perf.rs (new)
pub(crate) struct CliPerf { /* … */ }
impl CliPerf { /* … */ }
pub(crate) fn handle_no_results(/* … */) -> Result<(), Box<dyn std::error::Error>> { /* … */ }

// commands.rs and output/recent_commits.rs
use crate::perf::{CliPerf, handle_no_results};
```

### `urgent`: `cli/src/output/filesystem.rs` is 4571 lines

This single file holds 70 functions covering repo rendering, package
rendering, package-area rendering, dirty-package rendering, dependency
rendering, language rendering, file-list rendering, doc rendering, hash
rendering, and git-section rendering. It is by far the largest file in the
package area.

**Files**: `sniff/cli/src/output/filesystem.rs` (4571 LOC, 70 functions)

**Fix.** The `pub(crate) use filesystem::{ … }` block in `output/mod.rs`
(lines 49–58) already enumerates 22 public symbols — split the file along
those boundaries:

```text
cli/src/output/filesystem/
├── mod.rs                  // re-exports + small render_filesystem_section
├── repo.rs                 // render_repo_*, collect_repo_*
├── packages.rs             // render_*_packages, render_dirty_*
├── package_areas.rs        // render_*_package_areas
├── deps.rs                 // render_repo_deps_text, render_repo_deps_visual
├── language.rs             // render_repo_language, render_language_section
├── files.rs                // render_files_section, render_path_list, PathListFormat
└── docs.rs                 // render_docs_output, render_hash_section
```

Each split file lands at 400–800 LOC.

### `urgent`: `cli/src/args.rs` is 2640 lines

Every clap struct and enum (15 of them, including the top-level `Cli` and
`Commands`) lives in one file. Rust documentation, IDE navigation, and
review diffs all suffer.

**Files**: `sniff/cli/src/args.rs` (2640 LOC, 15 top-level types)

**Fix.** Split clap definitions by command family:

```text
cli/src/args/
├── mod.rs            // pub use of everything below + Cli + Commands enum
├── repo.rs           // RepoSubcommand, RepoAction, RecentCommitActionArg
├── files.rs          // FileListArgs, FilesFilter, FileAssociationArg, BlastRadiusScopeArg
├── docs.rs           // DocsFilter
├── install.rs        // InstallCommandArgs, InstallCommandKind, AllProgramAction
├── services.rs       // ServiceStateArg
└── output.rs         // PackagesFormat (and any output-shape enums)
```

clap derive paths continue to work unchanged because Rust resolves them by
type, not file location.

### `urgent`: `cli/src/commands.rs` is 1721 lines and only 14 functions

That works out to ~123 LOC per function — `run()` itself is almost
certainly the bulk. Sentrux's modularity score reads this as "single
hairball." The file also drives the cycle in `critical:cycle between
commands.rs and output/recent_commits.rs`.

**Files**: `sniff/cli/src/commands.rs` (1721 LOC, 14 functions)

**Fix.** Split per command family, mirroring the args split:

```text
cli/src/commands/
├── mod.rs             // pub async fn run() top-level dispatch
├── repo.rs            // handle_repo_*, handle_pr_command, handle_remote_url
├── files.rs           // handle_file_list_command, path_list_format
├── shorthand.rs       // handle_shorthand
├── completions.rs     // print_completions, *_completions_help
└── enrich.rs          // enrich_result_dependencies, fetch_readme
```

`CliPerf` and `handle_no_results` move to `perf.rs` per the critical fix;
the file `commands.rs` becomes a slim ~150-line dispatcher.

### `urgent`: `cli/src/output/repo_json.rs` is 1474 lines

JSON serialisation logic for repo output sits in one file alongside its
text-rendering counterpart. Two parallel 1400+ LOC files (text vs JSON)
suggest a missing abstraction.

**Files**: `sniff/cli/src/output/repo_json.rs` (1474 LOC)

**Fix.** Either:

1. Co-locate JSON adjacent to its text counterpart inside the new
   `output/filesystem/` tree (`repo.rs` exports `render_repo_text` and
   `repo_json`), so JSON and text live together per concern; or
2. Introduce a single `RepoView` intermediate struct that both renderers
   serialise from, eliminating the parallel hand-rolled JSON construction.

Option 2 is the larger but higher-leverage change: it removes redundancy
between `repo_json.rs` and the text path's per-section structs.

### `urgent`: `cli/src/output/mod.rs` is 1186 lines

`mod.rs` should be a re-export hub; a 1186-LOC `mod.rs` means significant
free-standing logic lives at the module root rather than in a named
sub-file. This works against modularity (no module name to anchor the
behaviour) and against the conventional Rust idiom that `mod.rs` is small.

**Files**: `sniff/cli/src/output/mod.rs` (1186 LOC)

**Fix.** Move free functions out of `mod.rs` into a sibling
`output/render.rs` (or split by function family — `format_bytes`,
`format_uptime`, `format_number`, `relative_path`, `render_*` helpers).
`output/mod.rs` should end up under 200 LOC, holding only `OutputFilter`,
`PathListFormat`, `TextOutput`, and the `pub use` re-exports.

### `important`: equality (Gini) — top 4 CLI files ≈ 79% of CLI LOC

Concentration in the CLI is even more extreme than in the lib:

| File | LOC |
|------|----:|
| `output/filesystem.rs` | 4571 |
| `args.rs` | 2640 |
| `commands.rs` | 1721 |
| `output/repo_json.rs` | 1474 |

Total: 10 406 LOC out of ~13 100 LOC in `cli/src/` (~79%). Five additional
files (output/mod.rs, output/hardware.rs, output/programs.rs, output/remote.rs,
install*.rs) contribute most of the rest. Sentrux equality (Gini) on this
distribution will be at the high-inequality end.

**Fix.** Implementing the four `urgent` splits above (filesystem.rs,
args.rs, commands.rs, repo_json.rs) brings every file under ~800 LOC and
moves the Gini coefficient toward the equality range without touching
behaviour.
