# `hug` CLI Architecture

How the `hug` binary (`tree-hugger/cli/`) is wired: the full command and
flag table, the declaration-report modules, `--changed` and package
grouping. User-facing usage lives in
[Using the `hug` CLI](../../../tree-hugger/docs/cli.md) (canonical; the CLI
README is only an overview) and the declaration commands' contract in
[Symbol Reports](../../../tree-hugger/docs/symbol-reports.md). A change to a
command or flag updates `docs/cli.md`, not the README.

## Command and flag reference

| Command | Description |
|---------|-------------|
| `functions` | List functions and methods |
| `types` | List type definitions (struct, enum, class, interface, trait) |
| `symbols` | List all symbols |
| `imports` | List imported symbols |
| `classes` | List classes, structs, interfaces and traits with static/instance member partitioning (on `SymbolInfo::is_static`) |
| `refs <symbol>` / `refs --id <id>` | References to one declaration within its file (`--callers` / `--callees`); text or `--json` |
| `lint` | Run lint and syntax diagnostics |
| `lint --deny <rule>` | Promote rule to error |
| `lint --warn <rule>` | Promote rule to warning |
| `lint --allow <rule>` | Demote rule to info |
| `lint --strict` | Promote all warnings to errors |
| `lint --experimental-semantics` | Enable experimental semantic rules |
| `god-files [DIR]` | Identify oversized source files ripe for refactoring |
| `god-files --high-risk` | Show only high-risk files |
| `completions` | Generate shell completions (Bash, Zsh, Fish, PowerShell) |
| `<declaration cmd> --sort file\|name\|kind` | Order (default `file`: path, then position); only `functions`/`types`/`symbols`/`classes` accept it |
| `--json-full` | Complete per-file indexes (declaration commands only; symbol selectors rejected) |
| `<declaration cmd> --changed[=<ref>]` | Narrow the file selection to Git-reported changed files (staged, unstaged, nonignored untracked, plus `<ref>`-tip vs `HEAD` differences; bare `--changed` = uncommitted only; conflicts with `--prelude`; an empty selection reports empty rather than `NoSourceFiles`; the flag and reference never enter the cache key) |
| `--verbose` | Cache stats line (hits / partial / misses, parses, captures, diagnostics runs) and cache failures on stderr |
| `<declaration cmd> --jsonl` | One self-describing JSON envelope per selected record (`classes` lines add a `members` list) |
| `<declaration cmd> --count` / `--stats` | One integer line / counts by kind, visibility, language + doc coverage (`--stats --json` for the envelope) |
| `<declaration cmd> --format plain\|table` | Unstyled, unwrapped listing / one biscuit-terminal `Table` per file |
| `<declaration cmd> --format tsv\|markdown\|prose\|ctags` | UTF-8 export; ctags uses unsorted format-2 tags with numeric addresses and Hug kind/container fields, every field written raw (Neovim decodes no escapes; a Unix `\` file name stays literal); `ctags::Unsupported` skips controls, `!_` names, paths Neovim expands (`$x`, backtick, `[`, leading `~`) and `UnrepresentablePath` (a name on the scan-root route that is not valid Unicode, decided by `ctags::write_record` from the native record via `lossless_portable_relative`, never by searching for `�`) with a warning and exit 1 |
| `functions\|types\|symbols --tree` | Members nested under their container (by container ID) |
| `<declaration cmd> --signatures full\|short\|none` | Header as written / compact with `… +N` past four fields (default) / names only |
| `<declaration cmd>` selectors | `--file`/`--name`/`--exact`, `--kind`, `--scope`/`--all-scopes`, `--visibility`, `--exported`, `--in`, `--exclude-tests`/`--tests-only`, `--undocumented`, `--deprecated`, `--async`, `--returns`, `--param-type`, `--min-params` (AND across selectors, OR within one; smart-case names) |
| positional directory (`src/`) or `--file <dir>` | selects no files (`NoSourceFiles`): inputs are `ignore` override globs and a directory pattern does not whitelist its files; use `'src/**'` |

## Declaration reports

- **CLI declaration reports** (`cli/src/`: `declarations.rs` drives,
  `selection.rs`, `order.rs`, `report.rs`, `render.rs`, `classes.rs`, `scan.rs`,
  `prelude.rs`; `main.rs` only wires): `Declarations::query(format, &Selection)`
  is each command's fixed kind set ∩ `--kind`, the selectors as `SymbolQuery`
  predicates, and only the capabilities it shows (`classes` and `--tree` add
  containment; `--json`/`--jsonl` add `Flags` and `--stats` adds `Metrics`, all
  in `ReportFormat::capabilities`; no listing runs references, semantics, docs
  or diagnostics); `--json-full` asks `complete_index()` (every capability,
  flags and metrics included). `scan::in_file_order` runs
  `rayon::current_num_threads()` scoped worker threads inside a sliding window
  (`window_size()` = `4 × threads` files past the last *flushed* delivery; a
  panicking job is re-raised when its file is next; no worker outlives the call
  on any exit: a `StopOnDrop` guard, created inside the scope before any worker
  starts, stops the window before the scope joins, so a panicking delivery or
  failed spawn propagates instead of hanging
  (`scan::tests::a_panicking_delivery_is_propagated_without_waiting_on_workers`,
  deadline-bounded)) and delivers results in file order, handing each delivery a
  `scan::Flush` (every window, before waiting on an unarrived head, and last),
  so `--json` is byte-identical across thread counts and cold/warm. The JSON
  envelope is written piece by piece with `errors` always last; records are
  projected (`report::trim_record`: scan-root-relative `file_path` and
  stable-key file segment, no `declaration_text`/source text/attached comments;
  IDs untouched). `tree_hugger::portable_relative` (`lib/src/file/naming.rs`,
  re-exported by `report.rs`; its sibling `lossless_portable_relative` returns
  the same text or `None` when an emitted name was not valid Unicode, through
  one shared private `project`) is the one path projection (entries,
  `errors[].file`, records, the qualified-name prefix; the CLI query passes
  `scan_root`): files outside the root get `../` routes via
  `biscuit_file::PathIdentity`, retried with directories canonicalized (macOS
  `/var` vs `/private/var`), and keep their absolute portable path only when no
  route exists (another Windows drive/share); the key's file segment is parsed
  from the key itself (`{lang}::{file}::{Kind:?}::`), never matched against
  `source.file_path`, because prelude aliases are keyed by the prelude file but
  located at the target. The one renderer (`render::Renderer`, built from
  `RenderOptions`) is `Prose` items in `UnorderedList`s (and a `Table` for
  `--format table`); it wraps only when `RenderOptions::width` is set
  (`main::wrap_width`: stdout is a TTY or `CLICOLOR_FORCE`, width from a
  positive `COLUMNS` else detection; `--plain`/`--format plain`/redirected
  output never wrap), and every source-derived string goes through
  `render::literal` (control chars → `\u{..}`, then `Prose::escape_text`, which
  on its own passes ANSI CSI/OSC through) or `render::neutralized` for table
  text cells; labels come from `identity.keyword` only for
  Type/Variable/Field/Constant, else the kind. The listing, outline and
  `classes` routines write to a `render::Layout`
  (`heading`/`row`/`list`/`note`/`blank`), never to bytes: `TerminalLayout`
  renders each line, `prose::ProseLayout` writes the same markup unrendered
  (`--plain` drops style/link nodes parsed with `InlineProse::to_render_nodes`, never
  a `<` scan), and `markdown::MarkdownLayout` writes CommonMark (headings for
  package/group/file, nested items for rows/sections, hard-break doc lines,
  `**`/`*` kept, spans/links dropped, text nodes CommonMark-escaped);
  `declarations::write_listing`/`write_tree` take a `Presentation`, so Markdown
  and Prose share all buffering, grouping, ordering, ownership and class
  sections (`export_prose_markdown` checks parsed Markdown shapes and both
  Prose-vs-plain comparisons). Only TSV and ctags flatten
  (`export::flatten_selected`: `classes` containers and displayed members
  deduplicated by ID, then sorted once per file as one `--sort` sequence, never
  container blocks; `export_order` pins it). Terminal output streams: a delivery
  continues only by returning the `scan::Flushed` its `Flush` was consumed into
  (`Flush::apply`, or `Flush::nothing_written` for the scan-before-output tally
  and `refs`), so every path, failed files and `imports`/`lint` adapter failures
  included, flushes before the window advances (a closed pipe stops scheduling
  within one window;
  `report::failed_files_still_stop_imports_and_lint_at_a_failing_stdout`,
  `adapter_failures_still_stop_lint_at_a_failing_stdout`); writers map
  `FileOutcome<P>` on the worker into their per-file form (JSON entries/lines
  serialized there, `report::serialized`; one-file listing units pre-rendered
  into bytes by `FileListing::rendered` for the terminal and Prose layouts, not
  Markdown, whose layout carries blank-line state), so delivery only writes;
  `classes` writes per file, and `render::ListingStream` holds only the current
  unit (one file, written on arrival (`units_are_files`); one directory under
  `--group-by-module`; one package under flat `--sort name|kind` or `--prelude`)
  after `render::listing_order` reorders files by package, directory, path.
  Package sections are decided up front from `listing_order`'s package count
  (two or more packages in the scan, declarations or not), so nothing is held
  beyond the current unit; never reintroduce a layout decision that waits on
  later declarations, or a whole-scan accumulator ahead of the renderer.
  `write_json` flushes as asked too (`JsonReport::flush`), so a closed pipe is
  noticed even when no entry fills the buffer. Text `imports`/`lint` (`main.rs`)
  stream the same way: lint's clean-file suppression is decided up front from
  `files.len() > 1`, and an unavailable adapter is warned once, right before the
  first file it affected, after flushing earlier output; `--json` still collects
  one envelope (serialized by reference). `--prelude` analyzes the prelude and
  its targets with an every-kind query first and the scan reuses them via
  `SymbolQueryResult::narrowed` (one parse per file; the reuse map,
  `collect_files` output, `SourceFile::path()` and the CLI roots are all
  `file_identity` spellings, so relative/`..`/symlinked inputs never split reuse
  or the cache); prelude exports are synthetic records
  (`visibility.is_synthetic`/`is_reexported`, own ID from the prelude file).
  Every output branch that writes declaration records (`--json` files,
  `--json-full` `index.symbols`, terminal listings, `classes` containers and
  member sections) orders through `order::compare`/`sort_file_records`:
  extraction order is not source order (C emits a typedef's inner `struct`
  specifier before the alias, Go a receiver parameter before its method). A
  failed file is stderr + `errors` + exit 1; a broken pipe exits 0 quietly.
  User-facing contract: `tree-hugger/docs/symbol-reports.md` (`order.rs` points
  at it); update it with any change to kind sets, envelope, order or
  `--verbose`.
- **Command errors**: `main` only wraps `run() -> Result<(), TreeHuggerError>`;
  an `Err` prints `error: {render::neutralized(Display)}` and exits 1 (never
  the `Debug` dump `fn main() -> Result` would print, and no `source()`
  chain: `Io`'s `Display` already includes it). Clap usage errors keep 2;
  paths that call `process::exit` themselves are unaffected. Tests:
  `cli/tests/command_feedback.rs`.
- **`hug refs`** (`cli/src/refs.rs`, wired early in `main::run_refs`; user
  contract `tree-hugger/docs/references/hug-refs.md`): the first positional is
  the name (`NamePattern::exact`, matched with `SymbolQuery::matches`, so smart
  case and `file#`/qualified names behave as in the selectors), the rest are
  file filters; `--id` compares `record.id` exactly. It is two-phase: every file
  is analyzed for its declarations over every kind and scope (locals included),
  through `scan::Analyzer::analyze_retaining`
  (`SymbolCache::query_retaining`/`SourceFile::query_retaining` give the
  `SourceFile` back, its text moved, never copied), so only a matching file
  keeps its bytes (`refs::Phased::Read`); then only the one declaring file gets
  `Capability::Occurrences`, from those same bytes (resolution is file-local, so
  only the match's file can hold its evidence; the other files' cache entries
  stay declarations-only). A one-file scan asks for Occurrences in phase 1 and
  reuses that result (`refs::Phased::Analyzed`), so its cost and `--verbose`
  counts match a single analysis. `refs::Selection` holds one `MatchedFile`
  (state + first record) per matching file and a lightweight `Candidate` (file
  position, location, label, qualified name, ID) per match; `Phased` is
  deliberately not `Clone`, the unique file's state is moved into the evidence
  phase, and the reported record is looked up there by ID (`refs::tests` pins
  one state per matching file for 40/80 matches). It decides before writing:
  several matches → stderr list (location, label, qualified name, ID) and exit 1
  with empty stdout; none → empty report. Incoming = `Scoped` rows with
  `target.id` == ID, plus a separate `name_only_suggestions` group (`NameOnly`
  whose candidates hold the ID); callees = `Call`s whose `owner.id` is the ID,
  every confidence, with `absent_id` (`import` iff the target name is in the
  record's `relations.dependencies`, which are exactly its bound imports by
  local name). Owner/candidate `qualified_name`s are re-prefixed from the
  result's records for presentation only. `DiscoveryArgs` (`--exclude-files`,
  `--language`, `--no-cache`) is shared with `ScanOptions`; `refs` takes no
  `SelectionArgs`/`ReportArgs`, so clap rejects `--kind`, `--format`, `--jsonl`,
  …; `--json-full` is rejected in `run_refs`. Text reuses `Renderer::{line,
  list, link, declaration}` and `render::label`. Tests:
  `cli/tests/symbols/refs.rs`, `level2_declarations::check_refs`.
- **Output modes** (`cli/src/output.rs`, `stats.rs`):
  `output::resolve(GivenOutput)` is the one place the "exactly one mode" rule
  lives (`--stats` composes with `--json` or a `--format`;
  `--sort`/`--signatures`/`--tree` reject `--stats`; `--tree` is for the
  terminal, Markdown and Prose, and not on `classes`); `main` maps the
  `OutputMode` to `declarations::ReportFormat`. `--sort` is `Option<SortOrder>`
  so "given" is detectable. `declarations::selected_records` is the one
  projection the JSON formats and the table share (classes: containers sorted,
  `displayed_members` trimmed); `write_json_lines`, `write_table`, `write_tree`
  stream per file in path order (not `listing_order`), `write_tally` holds only
  counters (`classes` counts containers). `selected_records` trims records
  (`trim_record`), so their `file_path` is a scan-root-relative *display*
  projection that replaces names that are not valid Unicode: never turn one into
  a file. Writers whose output must lead back to the declaration (ctags, the
  table's OSC 8 Location link) take `native_selected_records` (same records and
  order, untrimmed) and project or link the native path themselves
  (`reports::table_location_links_resolve_against_the_scan_root`,
  `symbols::export_ctags_paths`). `render::Outline` nests by
  `relations.container` ID, falling back to `identity.container_name` (a type
  declared in another file) for a marked `[in …]` heading that is never a record
  (an unlinked member is headed by its in-file owner path when that qualifies
  the name further, `[in other::W]`, so distinct external owners never share a
  heading); it reattaches anything a containment cycle would orphan. Type rows
  (`is_type()` kinds only, never a method or `[in …]` heading) end with `extends
  A; implements B; unclassified bases C` from `relations` as written
  (`Renderer::declared_bases`); under `--signatures full` a list the header
  already writes is dropped by `header_writes(record, list, base)`, an exact
  per-source rule (a Rust type's `implements` comes from impls, Go `extends`
  from interface bodies), never a text search (`struct W<T: Display>` + `impl
  Display for W` must still show it); a new language whose bases are not read
  from the header updates that match (`outline.rs` tests). `stats::Stats` keys
  are the records' serde spellings (`Function`, `Public`, `Rust`); synthetic
  exports count in the maps and apart from doc coverage. Tests:
  `cli/tests/symbols/reports.rs`, `outline.rs` (bases on type rows), goldens
  `symbols.{table,tree,stats,jsonl}`, `symbols-stats.json`,
  `symbols-signatures-full.plain` (`--format plain` shares `symbols.plain`); any
  test forcing color must pin `COLUMNS` (forced color wraps at the detected
  width).
- **CLI filters**: `hug <subcommand> <filter...>` with all-files default scan
  for symbol commands. A pathless scan honors `.gitignore` and skips test-data
  dirs by default (`**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`,
  `**/testdata/**`); an explicit path/glob re-includes a targeted file. `lint`
  scans hide clean files (single-file lint still shows `(no diagnostics)`).
- **Record selectors are one path** (`cli/src/selection.rs`): `SelectionArgs`
  (flattened into the declaration and `classes` args; `TargetArgs` =
  `--file`/`--name`/`--exact`, also on `imports`) resolves to a `Selection`,
  which becomes library predicates; the CLI keeps no name matcher of its own
  (positional names and `--exclude-symbols` are `NamePattern`s;
  `normalize_symbol_glob`/`matches_symbol_filters` are gone). Records are
  filtered by the library after the cache (`SymbolQueryResult::finish`); only
  `--prelude` is applied in `declarations::prepare` (its synthetic exports run
  `query.selects`/`query.matches` there). `symbols` defaults to `Item`+`Member`
  scopes (`SYMBOLS_DEFAULT_SCOPES`); other commands every scope. `classes` uses
  `classes::ClassSelection`: an analysis query with no predicates (container
  kinds ∪ member kinds, capabilities of both filters), identity predicates +
  `--kind` on containers, `--in` as the container's own name, shape predicates
  on members (fields hidden when one is given); `--static-only` conflicts with
  `--instance-only`. `imports` filters `index.imports` with `ImportSelection`
  (name, alias, original's last segment, or the whole original for a qualified
  pattern); positional tokens stay files. `--file` values: globs and unresolved
  values join the walker inputs, a `FileReference` that resolves to a file is an
  explicit target (`scanner::collect_selected_files`: bypasses
  `.gitignore`/fixture exclusions, not `--exclude-files`; never widens the
  scan). A bare `rs` is a name, not a file filter (`is_file_filter_token` needs
  a path, glob or `x.rs`). `--json-full` rejects every record selector
  (`SelectionArgs::given_flags`). Tests: `cli/tests/symbols/filters.rs`; tests
  that need locals from `symbols` pass `--all-scopes`.

## Changed-file declaration selection

```bash
hug functions --changed
hug symbols --changed=main
hug symbols --changed --file 'src/**'
hug --json-full symbols --changed
```

Bare `--changed` uses HEAD for uncommitted-only selection; `=main` adds
local main-tip vs HEAD-tip differences, never merge-base or fetch. It
reports whole files' current on-disk content, even when staging and working
edits cancel. Git-reported regular-file/symlink replacements also select the
current file; default walks still exclude symlinks, while explicit selectors
retain their normal symlink resolution. Keep `ChangedRequest::Omitted` distinct
from explicit HEAD:
only omission tolerates unborn HEAD without resolving a base.

`cli/src/changed.rs` discovers from the invocation directory through sniff,
uses the active worktree root, drops absent candidates, and matches native
`file_identity` values (`ChangedFiles::contains_identity`: scanner output
is already an identity, so membership resolves no path). `main.rs` validates Git
before ordinary file
selection, intersects before analysis/cache lookup, and treats only
`NoSourceFiles` as empty under this flag. Never feed Git paths into explicit
selectors: package scope, ignores, fixtures, forced language and exclusions
must retain their ordinary behavior. Explicit files outside a package must
still belong to the active checkout; guard each changed candidate alias hop and
resolved ancestor against external roots and nested `.git` files/directories
before inserting its canonical identity. Inspect traversal components before
`..` cancels visited directories; resolve aliases before applying subsequent
parents. Ordinary same-checkout parents remain eligible, while an alias through
`inner/../clean.rs` cannot hide a nested marker. Do not discover a second
repository
or scan submodules; ownership I/O failures are errors, not empty reports.

Empty selections succeed in every format (text `(no symbols)`, classes
terminal/plain outlines empty; no JSON Lines; count 0; empty files/indexes and
errors in JSON). Invalid Git state and conflicts anywhere fail before any
report, with exit 1. Truncated indexes produce repository/index context and
empty
stdout before source/cache analysis; absent and valid empty indexes remain
allowed.
`changed::index_corruption` checks every prefix on main and linked
administrative
indexes across commands, base variants, and report modes. Syntax errors and
`--prelude` conflicts exit 2.
Changed flags/references never enter cache keys. The L1 tests in the
`changed` target (`tests/changed/main.rs` declares each module) cover CLI
selection, errors, empty output, public help and repeated warm cache hits;
use `just test --test changed`. User-facing details and the flow diagram
live in [Symbol
Reports](../../../tree-hugger/docs/symbol-reports/changed-files.md).

## Package identity for reports

For package reporting, sniff exposes `detect_repo_structure_or_root_package`,
`PackageNameSource`, and `PackageOwnershipLookup`. Capture one structure catalog
and borrowing lookup; check the deepest owner's `name_source == Manifest`
before treating its display name as declared identity. Root `relative` is empty
(render it as `.`). Use `GitRepo::discover(...).repo_root()` for the active
worktree root, with the scan root outside Git.

`cli/src/grouping.rs::Catalog::capture` runs only for `ReportFormat::Stats`,
before prelude/source analysis and any output. Errors exit 1 with catalog/root
context and repair guidance; absence succeeds. `declarations::tally` creates a
group from each selected native analysis path before settling its result, then
adds selected records to both overall and group counters (classes: containers).
Synthetic export ownership uses the prelude analysis path, never the target
`source.file_path`. `Grouping::finish` sorts by portable relative path then
name,
Unassigned last; groups use native directory keys and stay outside cache
entries.
`stats::write_json` emits `package_root_dir` before overall `stats`, then
`groups` before final `errors` at report version 1. One `as_json` projection
keeps overall/group shapes identical; a null package is Unassigned. Terminal
`write_grouped_terminal` prints All selected declarations then sorted
`name (path)` sections and Unassigned, reusing the existing tables. Headings
use `render::literal` with Prose and no wrapping; plain strips escapes.
The discovery path frame is independent of scan `root_dir`, and empty reports
still include it. Output goldens intentionally include the new sections and
fields; their JSON normalizer redacts both absolute roots. Targeted L1 coverage:
`just test grouping
--test-threads 4`, including bin unit tests and `symbols::grouping_core`.
`symbols::grouping_acceptance` exercises normal CLI membership (Cargo/npm),
shipped lib/CLI manifests, linked-worktree roots, external explicit files,
filters, partial failures, cross-package prelude exports, and warm manifest
renames with unchanged cache bytes and zero parse work. Its shared assertion
checks all grouped counters and ratios against overall stats. Alias cases use
forced-language positional explicit files; outside-root cases use `--file`.
Native Windows spelling coverage is cfg(windows); filesystem non-UTF-8
coverage is cfg(target_os = "linux") because APFS rejects those filenames.
These remain L1 in the declared symbols target, not terminal tests.

User-facing package stats guidance lives in the
[`hug` CLI guide](../../../tree-hugger/docs/cli.md#package-stats) and
[Symbol Reports](../../../tree-hugger/docs/symbol-reports/package-stats.md).
The topic's flow distinguishes file-level group initialization (including
failed/zero-match files) from declaration-level counting. The
[release
notes](../../../tree-hugger/docs/release-notes.md#automatic-package-sections-in-stats)
show the intentional before/after terminal change; additive envelope fields
retain report version 1 independently of symbol-record schema versions.
